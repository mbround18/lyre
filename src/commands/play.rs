use anyhow::{Result, anyhow};
use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context as SerenityContext,
    CreateCommand, CreateCommandOption, CreateEmbed, CreateMessage, EditInteractionResponse,
};
use serenity::async_trait;
use songbird::{Event, EventContext, EventHandler as VoiceEventHandler, Songbird};
use std::sync::Arc;

use crate::audio::{DownloadProgress, spawn_download_mp3, ytdlp_extract_title};
use crate::database::establish_connection;
use crate::database::models::{CurrentQueue, QueueHistory, SongCache, VoiceConnection};
use crate::metrics::METRICS;

struct TrackEndNotifier {
    guild_id: serenity::all::GuildId,
    channel_id: serenity::all::ChannelId,
    manager: Arc<Songbird>,
    http: Arc<serenity::http::Http>,
}

#[async_trait]
impl VoiceEventHandler for TrackEndNotifier {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        // Advance the queue in database
        {
            let mut db_conn = establish_connection();
            if let Err(e) = CurrentQueue::advance_queue(&mut db_conn, &self.guild_id.to_string()) {
                tracing::warn!("Failed to advance queue in database: {}", e);
            }
        }

        // Check if queue is empty after this track ends
        if let Some(call_lock) = self.manager.get(self.guild_id) {
            let call = call_lock.lock().await;
            let queue_len = call.queue().len();
            drop(call);

            if queue_len == 0 {
                // Queue is empty, disconnect
                let _ = self.manager.remove(self.guild_id).await;

                // Update database to mark as not playing
                {
                    let mut db_conn = establish_connection();
                    if let Err(e) = VoiceConnection::update_playing_status(
                        &mut db_conn,
                        &self.guild_id.to_string(),
                        false,
                        None,
                    ) {
                        tracing::warn!("Failed to update playing status on disconnect: {}", e);
                    }
                }

                // Send a message to the channel
                let embed = CreateEmbed::new()
                    .title("🎵 Queue Finished")
                    .description(
                        "All songs have finished playing. Disconnected from voice channel.",
                    )
                    .colour(0x808080); // Gray

                let _ = self
                    .channel_id
                    .send_message(&self.http, CreateMessage::new().embeds(vec![embed]))
                    .await;
            } else {
                // Update database with next track info if available
                let mut db_conn = establish_connection();
                if let Ok(Some(next_track)) =
                    CurrentQueue::get_current_track(&mut db_conn, &self.guild_id.to_string())
                    && let Err(e) = VoiceConnection::update_playing_status(
                        &mut db_conn,
                        &self.guild_id.to_string(),
                        true,
                        next_track.title.as_deref(),
                    )
                {
                    tracing::warn!("Failed to update playing status with next track: {}", e);
                }
            }
        }
        None
    }
}

pub fn definition() -> CreateCommand {
    let opt =
        CreateCommandOption::new(CommandOptionType::String, "url", "URL to play").required(true);
    CreateCommand::new("play")
        .description("Queue and play audio from a URL")
        .add_option(opt)
}

pub async fn handle(ctx: &SerenityContext, cmd: &CommandInteraction) -> Result<()> {
    // Log some diagnostic information
    tracing::info!(
        "Processing /play command for user {} in guild {:?}",
        cmd.user.id,
        cmd.guild_id
    );

    let url = match cmd.data.options.first() {
        Some(option) => match &option.value {
            CommandDataOptionValue::String(url) => url,
            _ => return Err(anyhow!("expected string URL")),
        },
        None => return Err(anyhow!("missing URL argument")),
    };

    let guild_id = cmd.guild_id.ok_or_else(|| anyhow!("not in guild"))?;

    // Check bot's permissions first
    let bot_id = ctx.cache.current_user().id;
    {
        let guild = ctx
            .cache
            .guild(guild_id)
            .ok_or_else(|| anyhow!("guild not in cache"))?;

        // Check if bot has necessary permissions
        if let Some(_member) = guild.members.get(&bot_id) {
            tracing::info!("Bot found in guild members");
        }
    }

    // Defer response immediately to give us more time
    cmd.defer(&ctx.http).await?;

    // Get the user's voice channel
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

    // Check if bot has permissions to join the voice channel
    {
        let guild = ctx
            .cache
            .guild(guild_id)
            .ok_or_else(|| anyhow!("guild not in cache"))?;

        if let Some(channel) = guild.channels.get(&channel_id) {
            if let Some(bot_member) = guild.members.get(&bot_id) {
                let bot_permissions = guild.user_permissions_in(channel, bot_member);

                if !bot_permissions.connect() {
                    return Err(anyhow!(
                        "I don't have permission to connect to your voice channel. Please ensure I have the 'Connect' permission."
                    ));
                }

                if !bot_permissions.speak() {
                    return Err(anyhow!(
                        "I don't have permission to speak in your voice channel. Please ensure I have the 'Speak' permission."
                    ));
                }

                tracing::info!(
                    "Bot has necessary permissions to join voice channel {}",
                    channel_id
                );
            } else {
                return Err(anyhow!(
                    "Bot is not a member of this guild. Please re-invite the bot."
                ));
            }
        } else {
            return Err(anyhow!(
                "Voice channel not found in cache. Please try again."
            ));
        }
    }

    let manager = songbird::get(ctx).await.unwrap().clone();
    // Only count a connection if we weren't already connected
    let is_new = manager.get(guild_id).is_none();

    // Check if we're already connected to avoid unnecessary joins
    let call_lock = if let Some(existing_call) = manager.get(guild_id) {
        tracing::info!(
            "Already connected to voice channel in guild {}, reusing connection",
            guild_id
        );
        existing_call
    } else {
        // Retry voice channel joining with exponential backoff
        let mut attempts = 0;
        let max_attempts = 5; // Increased from 3 to 5

        loop {
            tracing::info!(
                "Attempting to join voice channel {} in guild {} (attempt {}/{})",
                channel_id,
                guild_id,
                attempts + 1,
                max_attempts
            );

            match manager.join(guild_id, channel_id).await {
                Ok(call_lock) => {
                    tracing::info!(
                        "Successfully joined voice channel after {} attempt(s)",
                        attempts + 1
                    );

                    // Update database to track voice connection
                    let mut db_conn = establish_connection();
                    if let Err(e) = VoiceConnection::create_or_update(
                        &mut db_conn,
                        &guild_id.to_string(),
                        Some(&channel_id.to_string()),
                    ) {
                        tracing::warn!("Failed to update database with voice connection: {}", e);
                    }

                    break call_lock;
                }
                Err(e) => {
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(anyhow!(
                            "failed to join voice channel after {} attempts: {}. This may be due to network issues, Discord API problems, or insufficient bot permissions.",
                            max_attempts,
                            e
                        ));
                    }

                    let delay_ms = std::cmp::min(5000, 1000 * (2_u64.pow(attempts as u32 - 1))); // Exponential backoff with cap at 5s
                    tracing::warn!(
                        "Voice channel join attempt {} failed: {}. Retrying in {}ms...",
                        attempts,
                        e,
                        delay_ms
                    );

                    // Wait before retrying (exponential backoff with cap)
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                }
            }
        }
    };

    if is_new {
        METRICS.inc_connections();
    } else {
        // Update last activity for existing connection
        let mut db_conn = establish_connection();
        if let Err(e) = VoiceConnection::update_last_activity(&mut db_conn, &guild_id.to_string()) {
            tracing::warn!("Failed to update last activity for voice connection: {}", e);
        }
    }

    // Start download in background and stream progress to the deferred message
    let (mut rx, handle) = spawn_download_mp3(url.to_string());

    // Check song cache first for title and metadata
    let mut db_conn = establish_connection();
    let cached_title = SongCache::find_by_url(&mut db_conn, url)
        .ok()
        .flatten()
        .map(|cached| {
            tracing::info!("Using cached title for {}: {}", url, cached.title);
            // Update last accessed time
            let _ = SongCache::update_last_accessed(&mut db_conn, url);
            cached.title
        });

    // Try to get song title - use cache if available, otherwise extract in parallel
    let title_future = if cached_title.is_some() {
        None // We already have the title
    } else {
        Some(ytdlp_extract_title(url))
    };

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

    // Download finished
    let input_path = handle
        .await
        .map_err(|e| anyhow!("download task panicked: {e}"))??;

    // Create input from the downloaded file path using ffmpeg with specific parameters for consistent playback
    let source = songbird::input::File::new(input_path);

    // Now setup the track with a notifier for when it ends
    let track = {
        let mut call = call_lock.lock().await;
        let track_handle = call.enqueue_input(source.into()).await;

        // Set track event handler
        track_handle
            .add_event(
                Event::Track(songbird::TrackEvent::End),
                TrackEndNotifier {
                    guild_id,
                    channel_id: cmd.channel_id,
                    manager: manager.clone(),
                    http: ctx.http.clone(),
                },
            )
            .map_err(|e| anyhow!("failed to add track event handler: {e}"))?;

        track_handle
    };

    // Get actual title (cached or extracted)
    let title = if let Some(cached_title) = cached_title {
        cached_title
    } else if let Some(future) = title_future {
        future.await.unwrap_or_else(|_| "Unknown".to_string())
    } else {
        "Unknown".to_string()
    };

    // Log to queue history
    let mut db_conn = establish_connection();
    if let Err(e) = QueueHistory::create(
        &mut db_conn,
        &guild_id.to_string(),
        &cmd.user.id.to_string(),
        url,
        Some(&title),
        None,
    ) {
        tracing::warn!("Failed to log queue history: {}", e);
    } else {
        // Increment queue metric on successful queue addition
        METRICS.inc_queue(1);
    }

    // Add to current queue tracking
    if let Err(e) = CurrentQueue::add_to_queue(
        &mut db_conn,
        &guild_id.to_string(),
        url,
        Some(&title),
        None,
        &cmd.user.id.to_string(),
    ) {
        tracing::warn!("Failed to add track to current queue: {}", e);
    }

    // Update voice connection to mark as playing
    if let Err(e) = VoiceConnection::update_playing_status(
        &mut db_conn,
        &guild_id.to_string(),
        true,
        Some(&title),
    ) {
        tracing::warn!("Failed to update playing status: {}", e);
    }

    // Update song cache
    if let Err(e) = SongCache::create_or_update(&mut db_conn, url, &title, None, None, None, None) {
        tracing::warn!("Failed to update song cache: {}", e);
    }

    // Send success message
    let embed = CreateEmbed::new()
        .title("🎵 Now Playing")
        .description(&title)
        .url(url)
        .colour(0x1db954) // Spotify green
        .footer(serenity::all::CreateEmbedFooter::new(format!(
            "Queue position: {} | Duration: Streaming",
            {
                let info = track
                    .get_info()
                    .await
                    .map_err(|e| anyhow!("failed to get track info: {e}"))?;
                format!("{:?}", info.position)
            }
        )));

    cmd.edit_response(
        &ctx.http,
        EditInteractionResponse::new()
            .content("")
            .embeds(vec![embed]),
    )
    .await?;

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
