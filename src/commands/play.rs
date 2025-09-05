use anyhow::{Result, anyhow};
use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context as SerenityContext,
    CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage, EditInteractionResponse,
};
use songbird::{driver::Bitrate, input::Input, tracks::TrackHandle};

use crate::audio::{DownloadProgress, spawn_download_mp3};

pub fn definition() -> CreateCommand {
    let opt =
        CreateCommandOption::new(CommandOptionType::String, "url", "URL to play").required(true);
    CreateCommand::new("play")
        .description("Queue and play audio from a URL")
        .add_option(opt)
}

pub async fn handle(ctx: &SerenityContext, cmd: &CommandInteraction) -> Result<()> {
    let url = cmd
        .data
        .options
        .iter()
        .find(|o| o.name == "url")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.as_str()),
            _ => None,
        })
        .ok_or_else(|| anyhow!("missing url"))?;

    cmd.create_response(
        &ctx.http,
        CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new()),
    )
    .await
    .ok();

    let guild_id = cmd.guild_id.ok_or_else(|| anyhow!("not in a guild"))?;
    let channel_id = {
        let guild = ctx
            .cache
            .guild(guild_id)
            .ok_or_else(|| anyhow!("guild not in cache"))?;
        guild
            .voice_states
            .get(&cmd.user.id)
            .and_then(|vs| vs.channel_id)
            .ok_or_else(|| anyhow!("you must be in a voice channel"))?
    };

    let manager = songbird::get(ctx).await.unwrap().clone();
    let call_lock = manager.join(guild_id, channel_id).await?;

    // Start download in background and stream progress to the deferred message
    let (mut rx, handle) = spawn_download_mp3(url.to_string());

    // Progress loop: update message periodically while downloading
    while let Some(DownloadProgress { percent }) = rx.recv().await {
        let bar = text_bar(percent);
        let _ = cmd
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .content(format!("Downloading… {} {}%", bar, percent)),
            )
            .await;
    }

    // Wait for completion and enqueue
    let mp3 = handle.await??;
    let source: Input = songbird::input::File::new(mp3).into();

    let mut call = call_lock.lock().await;
    // Lower, fixed bitrate can reduce CPU usage and packet size, helping avoid stutter on busy hosts.
    let br = std::env::var("LYRE_BITRATE")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|v| *v >= 16_000 && *v <= 192_000)
        .unwrap_or(96_000);
    call.set_bitrate(Bitrate::BitsPerSecond(br as i32));
    let handle: TrackHandle = call.enqueue_input(source).await;
    // Determine queue position after enqueue. If >1, this track is queued behind current/others.
    let position = call.queue().len();
    let target_volume = 0.5f32;
    if position > 1 {
        let _ = handle.set_volume(target_volume);
        drop(call);
        let _ = cmd
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().content(format!("Queued, in position {}", position)),
            )
            .await;
        return Ok(());
    } else {
        // First in queue: apply optional preroll buffer to mask initial jitters.
        let preroll_ms = std::env::var("LYRE_PREROLL_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|ms| *ms > 0 && *ms <= 30000)
            .unwrap_or(0);
        if preroll_ms > 0 {
            let _ = handle.set_volume(0.0);
        } else {
            let _ = handle.set_volume(target_volume);
        }
        drop(call);

        if preroll_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(preroll_ms)).await;
            let _ = handle.set_volume(target_volume);
        }
    }

    let _ = cmd
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content(format!(
                "Playing at {} kbps{}!",
                br / 1000,
                if std::env::var("LYRE_MIX_MODE").ok().as_deref() == Some("mono") {
                    " (mono)"
                } else {
                    ""
                }
            )),
        )
        .await;
    Ok(())
}

fn text_bar(percent: u8) -> String {
    // 20-wide bar
    let total = 20u8;
    let filled = ((percent as u16 * total as u16) / 100) as u8;
    let mut s = String::with_capacity((total as usize) + 2);
    s.push('[');
    for i in 0..total {
        if i < filled {
            s.push('█');
        } else {
            s.push(' ');
        }
    }
    s.push(']');
    s
}
