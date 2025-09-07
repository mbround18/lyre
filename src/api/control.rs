use super::types::{ApiResponse, VolumeRequest};
use crate::auth::{get_authenticated_user_from_extensions, user_can_control_guild};
use actix_web::{
    Error, HttpRequest, HttpResponse, Responder, Result as ActixResult, error::ErrorUnauthorized,
    post, put, web,
};

#[post("/api/control/{guild_id}/play")]
pub async fn next_track(
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<impl Responder, Error> {
    let guild_id = path.into_inner();

    // Get authenticated user from middleware
    let user = get_authenticated_user_from_extensions(&req)
        .map_err(|e| ErrorUnauthorized(format!("Authentication required: {}", e)))?;

    if !user_can_control_guild(&user.guilds, &guild_id) {
        return Err(ErrorUnauthorized("No permission for this guild"));
    }

    // TODO: Implement next track functionality

    Ok(HttpResponse::Ok().json(ApiResponse::success("Next track requested")))
}

#[post("/api/control/{guild_id}/stop")]
pub async fn stop_playback(path: web::Path<String>, req: HttpRequest) -> ActixResult<HttpResponse> {
    let guild_id = path.into_inner();

    // Get authenticated user from middleware
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

    // TODO: Implement stop functionality

    Ok(HttpResponse::Ok().json(ApiResponse::success("Playback stopped")))
}

#[put("/api/control/{guild_id}/volume")]
pub async fn set_volume(
    path: web::Path<String>,
    req_body: web::Json<VolumeRequest>,
    req: HttpRequest,
) -> ActixResult<HttpResponse> {
    let guild_id = path.into_inner();

    // Get authenticated user from middleware
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

    if req_body.volume < 0.0 || req_body.volume > 1.0 {
        return Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(
            "Volume must be between 0.0 and 1.0",
        )));
    }

    // TODO: Implement volume control

    Ok(HttpResponse::Ok().json(ApiResponse::success(format!(
        "Volume set to {}",
        req_body.volume
    ))))
}

#[derive(serde::Deserialize)]
pub struct JoinRequest {
    pub channel_id: String,
}

#[post("/api/control/{guild_id}/join")]
pub async fn join_voice_channel(
    path: web::Path<String>,
    req_body: web::Json<JoinRequest>,
    req: HttpRequest,
) -> ActixResult<HttpResponse> {
    let guild_id = path.into_inner();

    // Get authenticated user from middleware
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

    // Validate channel ID format (Discord snowflake)
    if req_body.channel_id.is_empty() || !req_body.channel_id.chars().all(char::is_numeric) {
        return Ok(
            HttpResponse::BadRequest().json(ApiResponse::<()>::error("Invalid channel ID format"))
        );
    }

    // Update database to track the request (even if we can't join immediately)
    {
        use crate::database::{establish_connection, models::VoiceConnection};
        let mut db_conn = establish_connection();
        if let Err(e) =
            VoiceConnection::create_or_update(&mut db_conn, &guild_id, Some(&req_body.channel_id))
        {
            tracing::warn!(
                "Failed to update database with voice connection request: {}",
                e
            );
        }
    }

    tracing::info!(
        "API request to join voice channel {} in guild {} (user: {})",
        req_body.channel_id,
        guild_id,
        user.user.id
    );

    // Bot will process the join request via background task
    Ok(
        HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
            "message": "Voice channel join request received",
            "guild_id": guild_id,
            "channel_id": req_body.channel_id,
            "status": "The bot will join the voice channel within a few seconds"
        }))),
    )
}
