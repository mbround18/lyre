use super::types::{ApiResponse, PlayRequest, QueueInfo, TrackInfo};
use crate::auth::{get_authenticated_user_from_extensions, user_can_control_guild};
use crate::database::{
    establish_connection,
    models::{CurrentQueue, VoiceConnection},
};
use actix_web::{HttpRequest, HttpResponse, Result as ActixResult, delete, get, post, web};

#[get("/api/queue/{guild_id}")]
pub async fn get_queue(path: web::Path<String>, req: HttpRequest) -> ActixResult<HttpResponse> {
    let guild_id = path.into_inner();

    let user = match get_authenticated_user_from_extensions(&req) {
        Ok(user) => user,
        Err(_) => {
            return Ok(HttpResponse::Unauthorized()
                .json(ApiResponse::<()>::error("Authentication failed")));
        }
    };

    if !user_can_control_guild(&user.guilds, &guild_id) {
        return Ok(HttpResponse::Forbidden()
            .json(ApiResponse::<()>::error("No permission for this guild")));
    }

    // Get actual queue from database
    let mut db_conn = establish_connection();

    let queue_items = CurrentQueue::get_guild_queue(&mut db_conn, &guild_id).unwrap_or_default();

    let voice_connection =
        VoiceConnection::find_by_guild_id(&mut db_conn, &guild_id).unwrap_or(None);

    let current_track = queue_items.first().map(|item| TrackInfo {
        title: item.title.clone().unwrap_or_else(|| "Unknown".to_string()),
        url: item.url.clone(),
        duration: item.duration.map(|d| d as u64),
        position: item.position as usize,
    });

    let queue: Vec<TrackInfo> = queue_items
        .iter()
        .skip(1)
        .enumerate()
        .map(|(idx, item)| TrackInfo {
            title: item.title.clone().unwrap_or_else(|| "Unknown".to_string()),
            url: item.url.clone(),
            duration: item.duration.map(|d| d as u64),
            position: idx + 1,
        })
        .collect();

    let is_playing = voice_connection.map(|vc| vc.is_playing).unwrap_or(false);

    let queue_info = QueueInfo {
        guild_id: guild_id.clone(),
        current_track,
        queue,
        position: 0,
        is_playing,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(queue_info)))
}

#[post("/api/queue/{guild_id}/add")]
pub async fn add_to_queue(
    path: web::Path<String>,
    req_body: web::Json<PlayRequest>,
    req: HttpRequest,
) -> ActixResult<HttpResponse> {
    let guild_id = path.into_inner();

    let user = match get_authenticated_user_from_extensions(&req) {
        Ok(user) => user,
        Err(_) => {
            return Ok(HttpResponse::Unauthorized()
                .json(ApiResponse::<()>::error("Authentication failed")));
        }
    };

    if !user_can_control_guild(&user.guilds, &guild_id) {
        return Ok(HttpResponse::Forbidden()
            .json(ApiResponse::<()>::error("No permission for this guild")));
    }

    // TODO: Implement actual queue addition
    // This would need access to the Songbird manager
    tracing::info!(
        "Adding track {} to queue for guild {}",
        req_body.url,
        guild_id
    );
    if let Some(channel_id) = &req_body.channel_id {
        tracing::info!("Using voice channel: {}", channel_id);
    }

    Ok(HttpResponse::Ok().json(ApiResponse::success("Track added to queue")))
}

#[post("/api/queue/{guild_id}/skip")]
pub async fn skip_track(path: web::Path<String>, req: HttpRequest) -> ActixResult<HttpResponse> {
    let guild_id = path.into_inner();

    let user = match get_authenticated_user_from_extensions(&req) {
        Ok(user) => user,
        Err(_) => {
            return Ok(HttpResponse::Unauthorized()
                .json(ApiResponse::<()>::error("Authentication failed")));
        }
    };

    if !user_can_control_guild(&user.guilds, &guild_id) {
        return Ok(HttpResponse::Forbidden()
            .json(ApiResponse::<()>::error("No permission for this guild")));
    }

    // TODO: Implement actual skip functionality

    Ok(HttpResponse::Ok().json(ApiResponse::success("Track skipped")))
}

#[delete("/api/queue/{guild_id}")]
pub async fn clear_queue(path: web::Path<String>, req: HttpRequest) -> ActixResult<HttpResponse> {
    let guild_id = path.into_inner();

    let user = match get_authenticated_user_from_extensions(&req) {
        Ok(user) => user,
        Err(_) => {
            return Ok(HttpResponse::Unauthorized()
                .json(ApiResponse::<()>::error("Authentication failed")));
        }
    };

    if !user_can_control_guild(&user.guilds, &guild_id) {
        return Ok(HttpResponse::Forbidden()
            .json(ApiResponse::<()>::error("No permission for this guild")));
    }

    // TODO: Implement actual queue clearing

    Ok(HttpResponse::Ok().json(ApiResponse::success("Queue cleared")))
}
