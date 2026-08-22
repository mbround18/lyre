use anyhow::{Result, anyhow};
use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context as SerenityContext,
    CreateCommand, CreateCommandOption, CreateEmbed, CreateMessage, EditInteractionResponse,
};
use serenity::async_trait;
use songbird::{Event, EventContext, EventHandler as VoiceEventHandler, Songbird};
use std::sync::Arc;


use crate::database::establish_connection;
use crate::database::models::{CurrentQueue, QueueHistory, SongCache, VoiceConnection};
use crate::metrics::METRICS;

pub struct TrackEndNotifier {
    pub guild_id: serenity::all::GuildId,
    pub channel_id: serenity::all::ChannelId,
    pub manager: Arc<Songbird>,
    pub http: Arc<serenity::http::Http>,
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
        let db_queue_empty = {
            let mut db_conn = establish_connection();
            match CurrentQueue::get_current_track(&mut db_conn, &self.guild_id.to_string()) {
                Ok(Some(_)) => false,
                _ => true,
            }
        };

        if db_queue_empty {
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
                .description("All songs have finished playing. Disconnected from voice channel.")
                .colour(0x808080); // Gray

            let _ = self
                .channel_id
                .send_message(&self.http, CreateMessage::new().embeds(vec![embed]))
                .await;
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
    let _call_lock = if let Some(existing_call) = manager.get(guild_id) {
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

    // Try to get song title - use cache if available, otherwise extract
    let title = if let Some(t) = cached_title {
        t
    } else {
        crate::audio::get_or_fetch_metadata(url)
            .await
            .map(|m| m.title)
            .unwrap_or_else(|_| "Unknown".to_string())
    };

    // Log to queue history
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
        METRICS.inc_queue(1);
    }

    // Add to current queue tracking
    // This will trigger pg_notify, and the background listener will handle playback
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

    // Update song cache
    if let Err(e) = SongCache::create_or_update(&mut db_conn, url, &title, None, None, None, None) {
        tracing::warn!("Failed to update song cache: {}", e);
    }

    // Send success message
    let embed = CreateEmbed::new()
        .title("🎵 Added to Queue")
        .description(&title)
        .url(url)
        .colour(0x1db954); // Spotify green

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
