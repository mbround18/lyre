use super::types::ApiResponse;
use actix_web::{HttpResponse, Result as ActixResult, get};

/// Development-only endpoint to generate a test token
/// WARNING: This should only be used in development!
#[get("/api/dev/test-token")]
pub async fn get_test_token() -> ActixResult<HttpResponse> {
    // Only allow in development
    if cfg!(debug_assertions) {
        // Generate a simple test token that the demo auth will accept
        let test_token = format!("demo_{}", chrono::Utc::now().timestamp());

        Ok(
            HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                "access_token": test_token,
                "token_type": "Bearer",
                "note": "This is a development test token. Use the OAuth flow in production."
            }))),
        )
    } else {
        Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error("Not available in production")))
    }
}
