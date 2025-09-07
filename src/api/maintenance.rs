use actix_web::{HttpRequest, HttpResponse, Result as ActixResult, delete, get, web};
use serde::{Deserialize, Serialize};

use super::types::ApiResponse;
use crate::auth::AuthenticatedUser;
use crate::database::establish_connection;
use crate::database::models::{QueueHistory, SongCache, VoiceConnection};

#[derive(Serialize)]
pub struct MaintenanceStats {
    pub connected_guilds: usize,
    pub cleanup_summary: CleanupSummary,
}

#[derive(Serialize)]
pub struct CleanupSummary {
    pub old_queue_entries_removed: usize,
    pub old_cache_entries_removed: usize,
}

#[derive(Deserialize)]
pub struct CleanupQuery {
    pub days_to_keep: Option<i32>,
}

#[get("/api/maintenance/stats")]
pub async fn get_maintenance_stats(
    _req: HttpRequest,
    _user: AuthenticatedUser,
) -> ActixResult<HttpResponse> {
    let mut conn = establish_connection();

    match VoiceConnection::get_all_connected(&mut conn) {
        Ok(connections) => {
            let stats = MaintenanceStats {
                connected_guilds: connections.len(),
                cleanup_summary: CleanupSummary {
                    old_queue_entries_removed: 0,
                    old_cache_entries_removed: 0,
                },
            };
            Ok(HttpResponse::Ok().json(ApiResponse::success(stats)))
        }
        Err(e) => {
            tracing::error!("Failed to get maintenance stats: {}", e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to get maintenance stats")))
        }
    }
}

#[delete("/api/maintenance/cleanup")]
pub async fn cleanup_old_data(
    _req: HttpRequest,
    _user: AuthenticatedUser,
    query: web::Query<CleanupQuery>,
) -> ActixResult<HttpResponse> {
    let mut conn = establish_connection();
    let days_to_keep = query.days_to_keep.unwrap_or(30);

    let queue_cleanup = QueueHistory::cleanup_old_entries(&mut conn, days_to_keep).unwrap_or(0);

    let cache_cleanup = SongCache::cleanup_old_entries(&mut conn, days_to_keep).unwrap_or(0);

    let summary = CleanupSummary {
        old_queue_entries_removed: queue_cleanup,
        old_cache_entries_removed: cache_cleanup,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(summary)))
}

#[derive(Deserialize)]
pub struct UserHistoryQuery {
    pub user_id: String,
    pub limit: Option<i64>,
}

#[get("/api/maintenance/user-history")]
pub async fn get_user_history(
    _req: HttpRequest,
    _user: AuthenticatedUser,
    query: web::Query<UserHistoryQuery>,
) -> ActixResult<HttpResponse> {
    let mut conn = establish_connection();
    let limit = query.limit.unwrap_or(10).min(50);

    match QueueHistory::get_recent_for_user(&mut conn, &query.user_id, limit) {
        Ok(history) => Ok(HttpResponse::Ok().json(ApiResponse::success(history))),
        Err(e) => {
            tracing::error!("Failed to get user history: {}", e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to get user history")))
        }
    }
}
