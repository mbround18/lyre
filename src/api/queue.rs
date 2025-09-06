use super::types::{ApiResponse, PlayRequest, QueueInfo};
use crate::auth::{AuthenticatedUser, user_can_control_guild};
use actix_web::{HttpResponse, Result as ActixResult, delete, get, post, web};

#[get("/api/queue/{guild_id}")]
pub async fn get_queue(
    path: web::Path<String>,
    user: AuthenticatedUser,
) -> ActixResult<HttpResponse> {
    let guild_id = path.into_inner();

    if !user_can_control_guild(&user.guilds, &guild_id) {
        return Ok(HttpResponse::Forbidden()
            .json(ApiResponse::<()>::error("No permission for this guild")));
    }

    // TODO: Get actual queue from Songbird
    let queue_info = QueueInfo {
        guild_id: guild_id.clone(),
        current_track: None,
        queue: vec![],
        position: 0,
        is_playing: false,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(queue_info)))
}

#[post("/api/queue/{guild_id}/add")]
pub async fn add_to_queue(
    path: web::Path<String>,
    req: web::Json<PlayRequest>,
    user: AuthenticatedUser,
) -> ActixResult<HttpResponse> {
    let guild_id = path.into_inner();

    if !user_can_control_guild(&user.guilds, &guild_id) {
        return Ok(HttpResponse::Forbidden()
            .json(ApiResponse::<()>::error("No permission for this guild")));
    }

    // TODO: Implement actual queue addition
    // This would need access to the Songbird manager
    tracing::info!("Adding track {} to queue for guild {}", req.url, guild_id);
    if let Some(channel_id) = &req.channel_id {
        tracing::info!("Using voice channel: {}", channel_id);
    }

    Ok(HttpResponse::Ok().json(ApiResponse::success("Track added to queue")))
}

#[post("/api/queue/{guild_id}/skip")]
pub async fn skip_track(
    path: web::Path<String>,
    user: AuthenticatedUser,
) -> ActixResult<HttpResponse> {
    let guild_id = path.into_inner();

    if !user_can_control_guild(&user.guilds, &guild_id) {
        return Ok(HttpResponse::Forbidden()
            .json(ApiResponse::<()>::error("No permission for this guild")));
    }

    // TODO: Implement actual skip functionality

    Ok(HttpResponse::Ok().json(ApiResponse::success("Track skipped")))
}

#[delete("/api/queue/{guild_id}")]
pub async fn clear_queue(
    path: web::Path<String>,
    user: AuthenticatedUser,
) -> ActixResult<HttpResponse> {
    let guild_id = path.into_inner();

    if !user_can_control_guild(&user.guilds, &guild_id) {
        return Ok(HttpResponse::Forbidden()
            .json(ApiResponse::<()>::error("No permission for this guild")));
    }

    // TODO: Implement actual queue clearing

    Ok(HttpResponse::Ok().json(ApiResponse::success("Queue cleared")))
}
