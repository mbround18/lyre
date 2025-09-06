use super::types::{ApiResponse, AuthRequest};
use crate::auth::{get_user_guilds, validate_discord_token};
use actix_web::{HttpResponse, Result as ActixResult, post, web};

#[post("/api/auth/validate")]
pub async fn validate_auth(req: web::Json<AuthRequest>) -> ActixResult<HttpResponse> {
    match validate_discord_token(&req.access_token).await {
        Ok(user) => match get_user_guilds(&req.access_token).await {
            Ok(guilds) => {
                let response = serde_json::json!({
                    "user": user,
                    "guilds": guilds
                });
                Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
            }
            Err(e) => Ok(
                HttpResponse::BadRequest().json(ApiResponse::<()>::error(&format!(
                    "Failed to get guilds: {}",
                    e
                ))),
            ),
        },
        Err(e) => Ok(HttpResponse::Unauthorized()
            .json(ApiResponse::<()>::error(&format!("Invalid token: {}", e)))),
    }
}
