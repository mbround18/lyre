use super::types::ApiResponse;
use crate::auth::AuthenticatedUser;
use actix_web::{HttpResponse, Result as ActixResult, get, post, web};

#[post("/api/search")]
pub async fn search_songs(
    _req: web::Json<serde_json::Value>,
    _user: AuthenticatedUser,
) -> ActixResult<HttpResponse> {
    // TODO: Implement song search using yt-dlp
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        "Search functionality not yet implemented",
    )))
}

#[get("/api/song/info")]
pub async fn get_song_info(
    query: web::Query<std::collections::HashMap<String, String>>,
    _user: AuthenticatedUser,
) -> ActixResult<HttpResponse> {
    if let Some(url) = query.get("url") {
        // TODO: Use yt-dlp to get song metadata
        Ok(HttpResponse::Ok().json(ApiResponse::success(format!("Song info for: {}", url))))
    } else {
        Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error("Missing url parameter")))
    }
}
