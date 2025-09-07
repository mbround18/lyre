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
    let mut attempts = 0;
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
                        "failed to join voice channel after {} attempts: {}. This may be due to network issues, Discord API problems, or insufficient bot permissions.",
                        max_attempts,
                        e
                    ));
                }

                let delay_ms = std::cmp::min(5000, 1000 * (2_u64.pow(attempts as u32 - 1))); // Exponential backoff with cap at 5s
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

                    if let Some(current) = current_channel {
                        current.0.get() == channel_id.get()
                    } else {
                        false
                    }
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
