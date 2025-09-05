use anyhow::Result;
use serenity::{
    all::{
        Command as AppCommand, Context as SerenityContext, GatewayIntents, Interaction,
        Permissions, Ready,
    },
    async_trait,
};
use songbird::{Config as VoiceConfig, driver::MixMode, serenity::SerenityInit};
use tracing::{error, info};

mod audio;
mod commands;
mod env;

struct Handler;

#[async_trait]
impl serenity::prelude::EventHandler for Handler {
    async fn ready(&self, ctx: SerenityContext, ready: Ready) {
        info!("Logged in as {}", ready.user.name);

        // Log an invite URL with minimal required voice permissions
        let perms = Permissions::CONNECT | Permissions::SPEAK;
        if let Ok(app) = ctx.http.get_current_application_info().await {
            let invite = format!(
                "https://discord.com/api/oauth2/authorize?client_id={}&permissions={}&scope=bot%20applications.commands",
                app.id,
                perms.bits()
            );
            info!(
                "Invite this bot: {} (app_id={}, user_id={})",
                invite, app.id, ready.user.id
            );
        }

        // Register global slash commands
        for def in [
            commands::play::definition(),
            commands::next::definition(),
            commands::stop::definition(),
        ] {
            if let Err(e) = AppCommand::create_global_command(&ctx.http, def).await {
                error!("failed to register global command: {e:?}");
            }
        }
    }

    async fn interaction_create(&self, ctx: SerenityContext, interaction: Interaction) {
        if let Interaction::Command(cmd) = interaction {
            match cmd.data.name.as_str() {
                "play" => {
                    if let Err(why) = commands::play::handle(&ctx, &cmd).await {
                        error!("/play failed: {why:?}");
                    }
                }
                "next" => {
                    if let Err(why) = commands::next::handle(&ctx, &cmd).await {
                        error!("/next failed: {why:?}");
                    }
                }
                "stop" => {
                    if let Err(why) = commands::stop::handle(&ctx, &cmd).await {
                        error!("/stop failed: {why:?}");
                    }
                }
                _ => {}
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let token = env::read_discord_token()?;

    let intents = GatewayIntents::non_privileged() | GatewayIntents::GUILD_VOICE_STATES;
    // Tune Songbird to reduce chance of audio hiccups under load.
    // - preallocated_tracks: avoid runtime allocations when queueing
    // - use_softclip(false): small (~3%) perf win; safe since we set volume <= 1.0 and play one track at a time
    // Keep stereo mixing by default to preserve quality.
    let voice_cfg = {
        let mix = match std::env::var("LYRE_MIX_MODE").as_deref() {
            Ok("mono") => MixMode::Mono,
            _ => MixMode::Stereo,
        };
        VoiceConfig::default()
            .preallocated_tracks(2)
            .use_softclip(false)
            .mix_mode(mix)
    };

    let mut client = serenity::Client::builder(token, intents)
        .event_handler(Handler)
        .register_songbird_from_config(voice_cfg)
        .await?;

    // Log an invite URL early (before gateway READY), so it's visible immediately
    let app = client.http.get_current_application_info().await?;
    let perms = Permissions::CONNECT | Permissions::SPEAK;
    let invite = format!(
        "https://discord.com/api/oauth2/authorize?client_id={}&permissions={}&scope=bot%20applications.commands",
        app.id,
        perms.bits()
    );
    info!("Invite this bot: {} (app_id={})", invite, app.id);
    println!("Invite this bot: {}", invite);

    if let Ok(dir) = audio::resolved_download_base_dir() {
        info!("Download cache dir: {}", dir.display());
    }
    info!("Commands: /play url:<link>, /next, /stop");
    info!(
        "Tunables: LYRE_MIX_MODE=mono|stereo, LYRE_BITRATE=16000..192000, LYRE_PREROLL_MS=0..30000, DOWNLOAD_FOLDER=path"
    );

    if let Err(why) = client.start_autosharded().await {
        error!("Client error: {why:?}");
    }
    Ok(())
}
