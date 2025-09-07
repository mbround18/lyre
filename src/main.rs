use anyhow::Result;
use serenity::{
    all::{
        Command as AppCommand, Context as SerenityContext, GatewayIntents, Interaction,
        Permissions, Ready,
    },
    async_trait,
};
use songbird::{Config as VoiceConfig, driver::MixMode, serenity::SerenityInit};
use std::sync::Arc;
use tracing::{error, info};

mod api;
mod audio;
mod auth;
mod bot_bridge;
mod commands;
mod database;
mod env;
mod metrics;
mod middleware;
mod voice_manager;
mod web_api;

struct Handler;

#[async_trait]
impl serenity::prelude::EventHandler for Handler {
    async fn ready(&self, ctx: SerenityContext, ready: Ready) {
        info!("Logged in as {}", ready.user.name);

        // Clear any stale voice connection records from database
        // When the bot restarts, it's not actually connected to any voice channels
        {
            use crate::database::{establish_connection, models::VoiceConnection};
            let mut db_conn = establish_connection();
            match VoiceConnection::clear_all_connections(&mut db_conn) {
                Ok(cleared) => {
                    if cleared > 0 {
                        info!(
                            "Cleared {} stale voice connection records from database",
                            cleared
                        );
                    }
                }
                Err(e) => {
                    error!("Failed to clear voice connection records: {}", e);
                }
            }
        }

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
            println!("Invite this bot: {}", invite);
        }

        if let Ok(dir) = crate::audio::resolved_download_base_dir() {
            info!("Download cache dir: {}", dir.display());
        }
        info!("Commands: /play url:<link>, /next, /stop");
        info!(
            "Tunables: LYRE_MIX_MODE=mono|stereo, LYRE_BITRATE=16000..192000, LYRE_PREROLL_MS=0..30000, DOWNLOAD_FOLDER=path"
        );

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

        // Mark ready for probes once we've registered commands
        metrics::METRICS.set_ready(true);

        // Start background task to process voice channel join requests from API
        let ctx_clone = ctx.clone();
        tokio::spawn(async move {
            voice_manager::process_voice_requests(Arc::new(ctx_clone)).await;
        });
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

    // Start background metrics scanners
    metrics::spawn_download_size_scanner();

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
            // Increase gateway timeout to handle slow connections (60 seconds for very slow networks)
            .gateway_timeout(Some(std::time::Duration::from_secs(60)))
    };

    let mut client = serenity::Client::builder(token, intents)
        .event_handler(Handler)
        .register_songbird_from_config(voice_cfg)
        .await?;

    // Initial startup info will be logged in the ready event handler

    // Run the HTTP server and Discord client concurrently with signal handling
    let http_bind = std::env::var("LYRE_HTTP_BIND").ok();
    let http_task = tokio::task::spawn_blocking(move || {
        // Run a dedicated Actix system on this blocking thread
        actix_web::rt::System::new().block_on(web_api::run_http(http_bind))
    });

    let discord_task = tokio::spawn(async move {
        if let Err(why) = client.start_autosharded().await {
            error!("Client error: {why:?}");
        }
    });

    // Set up signal handling
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

    tokio::select! {
        _ = http_task => {
            info!("HTTP server terminated");
        }
        _ = discord_task => {
            info!("Discord client terminated");
        }
        _ = sigterm.recv() => {
            info!("Received SIGTERM, shutting down gracefully");
        }
        _ = sigint.recv() => {
            info!("Received SIGINT (Ctrl+C), shutting down gracefully");
        }
    }

    info!("Shutdown complete");
    Ok(())
}
