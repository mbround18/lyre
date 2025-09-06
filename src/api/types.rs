use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct ProbeResp<'a> {
    pub status: &'a str,
}

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct QueueInfo {
    pub guild_id: String,
    pub current_track: Option<TrackInfo>,
    pub queue: Vec<TrackInfo>,
    pub position: usize,
    pub is_playing: bool,
}

#[derive(Serialize)]
pub struct TrackInfo {
    pub title: String,
    pub url: String,
    pub duration: Option<u64>,
    pub position: usize,
}

#[derive(Serialize)]
pub struct GuildInfo {
    pub id: String,
    pub name: String,
    pub connected: bool,
    pub voice_channel: Option<String>,
    pub queue_length: usize,
}

#[derive(Deserialize)]
pub struct PlayRequest {
    pub url: String,
    pub channel_id: Option<String>,
}

#[derive(Deserialize)]
pub struct VolumeRequest {
    pub volume: f32,
}

#[derive(Deserialize)]
pub struct AuthRequest {
    pub access_token: String,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.to_string()),
        }
    }
}
