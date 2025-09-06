use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serenity::model::id::GuildId;
use songbird::Songbird;

/// Shared state for tracking voice connections across the application
#[derive(Debug, Clone)]
pub struct AppState {
    /// Tracks which guilds the bot is currently connected to
    pub voice_connections: Arc<RwLock<HashMap<GuildId, bool>>>,
    /// Reference to the Songbird manager for checking actual connection status
    pub songbird: Option<Arc<Songbird>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            voice_connections: Arc::new(RwLock::new(HashMap::new())),
            songbird: None,
        }
    }

    pub fn with_songbird(songbird: Arc<Songbird>) -> Self {
        Self {
            voice_connections: Arc::new(RwLock::new(HashMap::new())),
            songbird: Some(songbird),
        }
    }

    /// Mark a guild as connected to voice
    pub fn set_voice_connected(&self, guild_id: GuildId, connected: bool) {
        if let Ok(mut connections) = self.voice_connections.write() {
            if connected {
                connections.insert(guild_id, true);
            } else {
                connections.remove(&guild_id);
            }
        }
    }

    /// Check if the bot is connected to voice in a guild
    /// This checks the Songbird manager directly if available
    pub fn is_voice_connected(&self, guild_id: GuildId) -> bool {
        // First check with Songbird manager if available
        if let Some(ref songbird) = self.songbird {
            return songbird.get(guild_id).is_some();
        }
        
        // Fallback to our manual tracking
        self.voice_connections
            .read()
            .map(|connections| connections.contains_key(&guild_id))
            .unwrap_or(false)
    }

    /// Get all guilds the bot is connected to
    pub fn get_connected_guilds(&self) -> Vec<GuildId> {
        // Use Songbird manager if available for accurate data
        if let Some(ref songbird) = self.songbird {
            return songbird.connection_info().keys().copied().collect();
        }
        
        // Fallback to our manual tracking
        self.voice_connections
            .read()
            .map(|connections| connections.keys().copied().collect())
            .unwrap_or_default()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
