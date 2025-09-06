use super::types::{ApiResponse, GuildInfo};
use crate::auth::{AuthenticatedUser, get_authenticated_user_from_request};
use crate::database::establish_connection;
use crate::database::models::VoiceConnection;
use actix_web::{HttpRequest, HttpResponse, Result as ActixResult, get};

#[get("/api/guilds")]
pub async fn get_guilds(
    req: HttpRequest, 
    _user: AuthenticatedUser,
) -> ActixResult<HttpResponse> {
    // Validate the token and get real user data
    let user = match get_authenticated_user_from_request(&req).await {
        Ok(user) => user,
        Err(e) => {
            return Ok(
                HttpResponse::Unauthorized().json(ApiResponse::<()>::error(&format!(
                    "Authentication failed: {}",
                    e
                ))),
            );
        }
    };

    // Convert user guilds to GuildInfo with connection status
    let guild_infos: Vec<GuildInfo> = user
        .guilds
        .iter()
        .map(|guild| {
            // Check if the bot is connected to this guild's voice channel using the database
            let mut conn = establish_connection();
            let connected = VoiceConnection::is_connected(&mut conn, &guild.id);
            
            GuildInfo {
                id: guild.id.clone(),
                name: guild.name.clone(),
                connected,
                voice_channel: if connected { Some("Connected".to_string()) } else { None },
                queue_length: 0,     // TODO: Get actual queue length from Songbird
            }
        })
        .collect();

    Ok(HttpResponse::Ok().json(ApiResponse::success(guild_infos)))
}
