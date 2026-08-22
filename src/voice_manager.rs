use anyhow::{Result, anyhow};
use serenity::all::{ChannelId, Context as SerenityContext, GuildId};
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::database::{establish_connection, models::VoiceConnection};

/// Join a voice channel with retry logic
pub async fn join_voice_channel(
    ctx: &SerenityContext,
    guild_id: GuildId,
    channel_id: ChannelId,
) -> Result<()> {
    let manager = songbird::get(ctx).await.unwrap().clone();

    // Check if we're already connected to avoid unnecessary joins
    if let Some(_existing_call) = manager.get(guild_id) {
        info!(
            "Already connected to voice channel in guild {}, reusing connection",
            guild_id
        );
        return Ok(());
    }

    // Retry voice channel joining with exponential backoff
    let mut attempts: u32 = 0;
    let max_attempts = 5;

    loop {
        info!(
            "Attempting to join voice channel {} in guild {} (attempt {}/{})",
            channel_id,
            guild_id,
            attempts + 1,
            max_attempts
        );

        match manager.join(guild_id, channel_id).await {
            Ok(_call_lock) => {
                info!(
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
                    warn!("Failed to update database with voice connection: {}", e);
                }

                return Ok(());
            }
            Err(e) => {
                attempts += 1;
                if attempts >= max_attempts {
                    return Err(anyhow!(
                        "failed to join voice channel after {max_attempts} attempts: {e}. This may be due to network issues, Discord API problems, or insufficient bot permissions."
                    ));
                }

                let delay_ms = std::cmp::min(5000, 1000 * (2_u64.pow(attempts - 1))); // Exponential backoff with cap at 5s
                warn!(
                    "Voice channel join attempt {} failed: {}. Retrying in {}ms...",
                    attempts, e, delay_ms
                );

                // Wait before retrying (exponential backoff with cap)
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
        }
    }
}

/// Background task to process voice channel join requests from the database
pub async fn process_voice_requests(ctx: Arc<SerenityContext>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));

    loop {
        interval.tick().await;

        let requests = {
            let mut db_conn = establish_connection();
            match VoiceConnection::get_pending_joins(&mut db_conn) {
                Ok(requests) => requests,
                Err(e) => {
                    error!("Failed to fetch pending voice requests: {}", e);
                    continue;
                }
            }
        };

        for request in requests {
            if let Some(channel_id_str) = &request.channel_id {
                // Parse IDs
                let guild_id = match request.guild_id.parse::<u64>() {
                    Ok(id) => GuildId::new(id),
                    Err(e) => {
                        error!("Invalid guild ID {}: {}", request.guild_id, e);
                        continue;
                    }
                };

                let channel_id = match channel_id_str.parse::<u64>() {
                    Ok(id) => ChannelId::new(id),
                    Err(e) => {
                        error!("Invalid channel ID {}: {}", channel_id_str, e);
                        continue;
                    }
                };

                // Check if bot is already connected to this specific channel
                let manager = songbird::get(&ctx).await.unwrap().clone();
                let already_connected = if let Some(call_lock) = manager.get(guild_id) {
                    let call = call_lock.lock().await;
                    let current_channel = call.current_channel();
                    drop(call);

                    current_channel.is_some_and(|current| current.0.get() == channel_id.get())
                } else {
                    false
                };

                if already_connected {
                    // Bot is already connected to this channel, skip processing
                    continue;
                }

                // Check if this is a recent request (within last 5 minutes)
                let now = chrono::Utc::now().naive_utc();
                let request_age = now.signed_duration_since(request.connected_at);
                if request_age.num_minutes() > 5 {
                    // This is an old connection record, not a new join request
                    continue;
                }

                // Attempt to join the voice channel
                match join_voice_channel(&ctx, guild_id, channel_id).await {
                    Ok(()) => {
                        info!(
                            "Successfully joined voice channel {} in guild {} via API request",
                            channel_id, guild_id
                        );
                    }
                    Err(e) => {
                        error!(
                            "Failed to join voice channel {} in guild {} via API request: {}",
                            channel_id, guild_id, e
                        );

                        // Remove the failed request from database to avoid infinite retries
                        let mut db_conn = establish_connection();
                        if let Err(db_err) =
                            VoiceConnection::delete(&mut db_conn, &request.guild_id)
                        {
                            error!("Failed to clean up failed voice request: {}", db_err);
                        }
                    }
                }
            }
        }
    }
}

/// Process a queue update notification from `PostgreSQL`
pub async fn process_queue_update(ctx: &SerenityContext, guild_id_str: &str) {
    let guild_id = match guild_id_str.parse::<u64>() {
        Ok(id) => GuildId::new(id),
        Err(_) => return,
    };

    let manager = songbird::get(ctx).await.unwrap().clone();

    // Ensure we are connected to the voice channel
    if manager.get(guild_id).is_none() {
        // If not connected, maybe we shouldn't start playback, or we should connect if channel is known.
        // For now, if we are not connected, we can't play.
        return;
    }

    // Check if we are already playing something
    let is_playing = if let Some(call_lock) = manager.get(guild_id) {
        let call = call_lock.lock().await;
        !call.queue().is_empty()
    } else {
        false
    };

    if is_playing {
        // Already playing, just wait for TrackEndNotifier to advance the queue
        return;
    }

    // Not playing. Fetch the next track from the database.
    let next_track = {
        use crate::database::models::CurrentQueue;
        let mut db_conn = establish_connection();
        match CurrentQueue::get_current_track(&mut db_conn, guild_id_str) {
            Ok(Some(track)) => track,
            Ok(None) => return, // Queue is empty
            Err(e) => {
                error!("Failed to fetch next track for playback: {}", e);
                return;
            }
        }
    };

    // We have a track to play. Start downloading and playing it.
    info!("Starting background playback for: {}", next_track.url);

    // 1. Download
    let (mut rx, download_handle) = crate::audio::spawn_download_mp3(next_track.url.clone());

    // We can optionally process the progress stream here, but since this is background we can just drain it
    while rx.recv().await.is_some() {}

    let input_path = match download_handle.await {
        Ok(Ok(path)) => path,
        Err(e) => {
            error!("Download task panicked: {}", e);
            // Advance queue since this track failed
            let mut db_conn = establish_connection();
            let _ =
                crate::database::models::CurrentQueue::advance_queue(&mut db_conn, guild_id_str);
            return;
        }
        Ok(Err(e)) => {
            error!("Failed to download track: {}", e);
            let mut db_conn = establish_connection();
            let _ =
                crate::database::models::CurrentQueue::advance_queue(&mut db_conn, guild_id_str);
            return;
        }
    };

    // 2. Play
    let source = songbird::input::File::new(input_path);
    if let Some(call_lock) = manager.get(guild_id) {
        let mut call = call_lock.lock().await;
        let track_handle = call.enqueue_input(source.into()).await;
        drop(call);

        // Try to get channel_id from voice connection DB to pass to TrackEndNotifier
        let channel_id = {
            let mut db_conn = establish_connection();
            VoiceConnection::find_by_guild_id(&mut db_conn, guild_id_str)
                .ok()
                .flatten()
                .and_then(|vc| vc.channel_id)
                .and_then(|cid_str| cid_str.parse::<u64>().ok())
                .map(|id| ChannelId::new(id))
        };

        if let Some(cid) = channel_id
            && let Err(e) = track_handle.add_event(
                songbird::Event::Track(songbird::TrackEvent::End),
                crate::commands::play::TrackEndNotifier {
                    guild_id,
                    channel_id: cid,
                    manager: manager.clone(),
                    http: ctx.http.clone(),
                },
            )
        {
            error!("Failed to add track event handler: {}", e);
        }

        // Send a Now Playing message to the channel
        if let Some(cid) = channel_id {
            let embed = serenity::all::CreateEmbed::new()
                .title("🎵 Now Playing")
                .description(next_track.title.as_deref().unwrap_or(&next_track.url))
                .url(&next_track.url)
                .colour(0x001d_b954);

            let _ = cid
                .send_message(
                    &ctx.http,
                    serenity::all::CreateMessage::new().embeds(vec![embed]),
                )
                .await;
        }

        // Update DB voice connection
        let mut db_conn = establish_connection();
        let _ = VoiceConnection::update_playing_status(
            &mut db_conn,
            guild_id_str,
            true,
            next_track.title.as_deref(),
        );
    }
}
