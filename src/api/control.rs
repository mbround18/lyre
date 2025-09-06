use super::types::{ApiResponse, VolumeRequest};
use crate::auth::{AuthenticatedUser, user_can_control_guild};
use actix_web::{HttpResponse, Result as ActixResult, post, put, web};

#[post("/api/control/{guild_id}/play")]
pub async fn play_pause(
    path: web::Path<String>,
    user: AuthenticatedUser,
) -> ActixResult<HttpResponse> {
    let guild_id = path.into_inner();

    if !user_can_control_guild(&user.guilds, &guild_id) {
        return Ok(HttpResponse::Forbidden()
            .json(ApiResponse::<()>::error("No permission for this guild")));
    }

    // TODO: Implement play/pause functionality

    Ok(HttpResponse::Ok().json(ApiResponse::success("Playback toggled")))
}

#[post("/api/control/{guild_id}/stop")]
pub async fn stop_playback(
    path: web::Path<String>,
    user: AuthenticatedUser,
) -> ActixResult<HttpResponse> {
    let guild_id = path.into_inner();

    if !user_can_control_guild(&user.guilds, &guild_id) {
        return Ok(HttpResponse::Forbidden()
            .json(ApiResponse::<()>::error("No permission for this guild")));
    }

    // TODO: Implement stop functionality

    Ok(HttpResponse::Ok().json(ApiResponse::success("Playback stopped")))
}

#[put("/api/control/{guild_id}/volume")]
pub async fn set_volume(
    path: web::Path<String>,
    req: web::Json<VolumeRequest>,
    user: AuthenticatedUser,
) -> ActixResult<HttpResponse> {
    let guild_id = path.into_inner();

    if !user_can_control_guild(&user.guilds, &guild_id) {
        return Ok(HttpResponse::Forbidden()
            .json(ApiResponse::<()>::error("No permission for this guild")));
    }

    if req.volume < 0.0 || req.volume > 1.0 {
        return Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(
            "Volume must be between 0.0 and 1.0",
        )));
    }

    // TODO: Implement volume control

    Ok(HttpResponse::Ok().json(ApiResponse::success(format!(
        "Volume set to {}",
        req.volume
    ))))
}
