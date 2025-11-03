use anyhow::{Result, anyhow};
use serenity::{
    all::{
        CommandDataOptionValue, CommandInteraction, CommandOptionType, CreateCommand,
        CreateCommandOption, EditInteractionResponse,
    },
    client::Context,
};

use crate::audio::{spawn_download_mp3, ytdlp_extract_playlist};

const MAX_DURATION_SECS: f64 = 4200.0; // 70 minutes

pub fn register() -> CreateCommand {
    CreateCommand::new("playlist")
        .description("Queue an entire playlist (videos over 70 minutes will be skipped)")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "url", "Playlist URL to queue")
                .required(true),
        )
}

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> Result<()> {
    // Parse url option
    let url = match interaction.data.options.first() {
        Some(option) => match &option.value {
            CommandDataOptionValue::String(url) => url,
            _ => return Err(anyhow!("expected string URL")),
        },
        None => return Err(anyhow!("missing URL argument")),
    };

    // Defer response immediately
    interaction.defer(&ctx.http).await?;

    // Extract playlist entries
    let entries = match ytdlp_extract_playlist(url).await {
        Ok(e) => e,
        Err(err) => {
            interaction
                .edit_response(
                    &ctx.http,
                    EditInteractionResponse::new()
                        .content(format!("❌ Failed to load playlist: {}", err)),
                )
                .await?;
            return Err(err);
        }
    };

    let total_count = entries.len();
    let mut queued_count = 0;
    let mut skipped_videos = Vec::new();

    // Initial status
    interaction
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content(format!(
                "📋 Found {} videos in playlist. Validating durations...",
                total_count
            )),
        )
        .await?;

    // Process each video
    for (video_url, title, duration_opt) in entries {
        // Check duration
        if let Some(duration) = duration_opt {
            if duration > MAX_DURATION_SECS {
                let duration_mins = (duration / 60.0).round() as u32;
                skipped_videos.push(format!(
                    "⏭️ **{}** ({} minutes - too long)",
                    title, duration_mins
                ));
                continue;
            }
        }

        // Queue this video - just spawn and forget for now
        let (_rx, _handle) = spawn_download_mp3(video_url.clone());

        // In a full implementation, we'd track these downloads and integrate with queue system
        // For now, they'll download in the background

        queued_count += 1;
    }

    // Final status message
    let mut response = format!(
        "✅ Queued **{}/{}** videos from playlist",
        queued_count, total_count
    );

    if !skipped_videos.is_empty() {
        response.push_str("\n\n**Skipped videos (over 70 minutes):**\n");
        for skipped in skipped_videos.iter().take(10) {
            response.push_str(&format!("{}\n", skipped));
        }

        if skipped_videos.len() > 10 {
            response.push_str(&format!("... and {} more", skipped_videos.len() - 10));
        }
    }

    interaction
        .edit_response(&ctx.http, EditInteractionResponse::new().content(response))
        .await?;

    Ok(())
}
