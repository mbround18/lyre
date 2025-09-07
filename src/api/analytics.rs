use actix_web::{HttpRequest, HttpResponse, Result as ActixResult, get, put, web};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use super::types::ApiResponse;
use crate::auth::AuthenticatedUser;
use crate::database::establish_connection;
use crate::database::models::{GuildSettings, QueueHistory, SongCache};

#[derive(Serialize)]
pub struct RecentTrack {
    pub url: String,
    pub title: Option<String>,
    pub user_id: String,
    pub played_at: String,
    pub duration: Option<i32>,
}

#[derive(Deserialize)]
pub struct RecentTracksQuery {
    pub guild_id: String,
    pub limit: Option<i64>,
}

#[get("/api/recent-tracks")]
pub async fn get_recent_tracks(
    _req: HttpRequest,
    _user: AuthenticatedUser,
    query: web::Query<RecentTracksQuery>,
) -> ActixResult<HttpResponse> {
    let mut conn = establish_connection();
    let limit = query.limit.unwrap_or(10).min(50); // Cap at 50 tracks

    match QueueHistory::get_recent_for_guild(&mut conn, &query.guild_id, limit) {
        Ok(history) => {
            let tracks: Vec<RecentTrack> = history
                .into_iter()
                .map(|h| RecentTrack {
                    url: h.url,
                    title: h.title,
                    user_id: h.user_id,
                    played_at: h.played_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                    duration: h.duration,
                })
                .collect();

            Ok(HttpResponse::Ok().json(ApiResponse::success(tracks)))
        }
        Err(e) => {
            tracing::error!("Failed to get recent tracks: {}", e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to get recent tracks")))
        }
    }
}

#[derive(Serialize)]
pub struct GuildSettingsResponse {
    pub guild_id: String,
    pub default_volume: f32,
    pub auto_disconnect_minutes: i32,
    pub max_queue_size: i32,
    pub allowed_roles: Vec<String>,
    pub blocked_domains: Vec<String>,
}

#[derive(Deserialize)]
pub struct GuildSettingsQuery {
    pub guild_id: String,
}

#[get("/api/guild-settings")]
pub async fn get_guild_settings(
    _req: HttpRequest,
    _user: AuthenticatedUser,
    query: web::Query<GuildSettingsQuery>,
) -> ActixResult<HttpResponse> {
    let mut conn = establish_connection();

    match GuildSettings::find_by_guild_id(&mut conn, &query.guild_id) {
        Ok(Some(settings)) => {
            let allowed_roles: Vec<String> = settings
                .allowed_roles
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();

            let blocked_domains: Vec<String> = settings
                .blocked_domains
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();

            let response = GuildSettingsResponse {
                guild_id: settings.guild_id,
                default_volume: settings.default_volume,
                auto_disconnect_minutes: settings.auto_disconnect_minutes,
                max_queue_size: settings.max_queue_size,
                allowed_roles,
                blocked_domains,
            };

            Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
        }
        Ok(None) => {
            // Create default settings if none exist
            match GuildSettings::create_or_update(&mut conn, &query.guild_id) {
                Ok(settings) => {
                    let response = GuildSettingsResponse {
                        guild_id: settings.guild_id,
                        default_volume: settings.default_volume,
                        auto_disconnect_minutes: settings.auto_disconnect_minutes,
                        max_queue_size: settings.max_queue_size,
                        allowed_roles: vec![],
                        blocked_domains: vec![],
                    };
                    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
                }
                Err(e) => {
                    tracing::error!("Failed to create guild settings: {}", e);
                    Ok(HttpResponse::InternalServerError()
                        .json(ApiResponse::<()>::error("Failed to get guild settings")))
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to get guild settings: {}", e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to get guild settings")))
        }
    }
}

#[derive(Serialize)]
pub struct CacheStats {
    pub total_songs: i64,
    pub total_size_bytes: i64,
    pub total_size_mb: f64,
}

#[get("/api/cache-stats")]
pub async fn get_cache_stats(
    _req: HttpRequest,
    _user: AuthenticatedUser,
) -> ActixResult<HttpResponse> {
    let mut conn = establish_connection();

    match SongCache::get_cache_size(&mut conn) {
        Ok(total_size) => {
            // Get count of cached songs
            use crate::database::schema::song_cache;
            use diesel::dsl::count;

            let total_songs = song_cache::table
                .select(count(song_cache::url))
                .first::<i64>(&mut conn)
                .unwrap_or(0);

            let stats = CacheStats {
                total_songs,
                total_size_bytes: total_size,
                total_size_mb: total_size as f64 / 1_048_576.0, // Convert to MB
            };

            Ok(HttpResponse::Ok().json(ApiResponse::success(stats)))
        }
        Err(e) => {
            tracing::error!("Failed to get cache stats: {}", e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to get cache stats")))
        }
    }
}

#[derive(Deserialize)]
pub struct UpdateGuildSettingsRequest {
    pub guild_id: String,
    pub default_volume: Option<f32>,
    pub auto_disconnect_minutes: Option<i32>,
    pub max_queue_size: Option<i32>,
}

#[put("/api/guild-settings")]
pub async fn update_guild_settings(
    _req: HttpRequest,
    _user: AuthenticatedUser,
    body: web::Json<UpdateGuildSettingsRequest>,
) -> ActixResult<HttpResponse> {
    let mut conn = establish_connection();
    let req = body.into_inner();

    // Ensure guild settings exist first
    if GuildSettings::find_by_guild_id(&mut conn, &req.guild_id).is_err()
        && let Err(e) = GuildSettings::create_or_update(&mut conn, &req.guild_id)
    {
        tracing::error!("Failed to create guild settings: {}", e);
        return Ok(HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("Failed to create guild settings")));
    }

    // Update individual settings if provided
    if let Some(volume) = req.default_volume {
        if !(0.0..=1.0).contains(&volume) {
            return Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(
                "Volume must be between 0.0 and 1.0",
            )));
        }
        if let Err(e) = GuildSettings::update_volume(&mut conn, &req.guild_id, volume) {
            tracing::error!("Failed to update volume: {}", e);
            return Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to update volume")));
        }
    }

    if let Some(minutes) = req.auto_disconnect_minutes {
        if !(1..=60).contains(&minutes) {
            return Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(
                "Auto-disconnect must be between 1 and 60 minutes",
            )));
        }
        if let Err(e) = GuildSettings::update_auto_disconnect(&mut conn, &req.guild_id, minutes) {
            tracing::error!("Failed to update auto-disconnect: {}", e);
            return Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to update auto-disconnect")));
        }
    }

    if let Some(size) = req.max_queue_size {
        if !(1..=100).contains(&size) {
            return Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(
                "Max queue size must be between 1 and 100",
            )));
        }
        if let Err(e) = GuildSettings::update_max_queue_size(&mut conn, &req.guild_id, size) {
            tracing::error!("Failed to update max queue size: {}", e);
            return Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to update max queue size")));
        }
    }

    // Return updated settings
    match GuildSettings::find_by_guild_id(&mut conn, &req.guild_id) {
        Ok(Some(settings)) => {
            let response = GuildSettingsResponse {
                guild_id: settings.guild_id,
                default_volume: settings.default_volume,
                auto_disconnect_minutes: settings.auto_disconnect_minutes,
                max_queue_size: settings.max_queue_size,
                allowed_roles: vec![],   // TODO: Parse JSON if needed
                blocked_domains: vec![], // TODO: Parse JSON if needed
            };
            Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
        }
        Ok(None) => {
            Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error("Guild settings not found")))
        }
        Err(e) => {
            tracing::error!("Failed to get updated guild settings: {}", e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to get updated settings")))
        }
    }
}
