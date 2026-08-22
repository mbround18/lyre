use super::types::ApiResponse;
use actix_web::{HttpResponse, Result as ActixResult, get, web};
use serde::Serialize;

fn redirect_uri() -> String {
    std::env::var("DISCORD_REDIRECT_URI")
        .unwrap_or_else(|_| "http://localhost:3000/auth/callback".to_string())
}

#[derive(Serialize)]
pub struct OAuthConfig {
    client_id: String,
    redirect_uri: String,
}

/// Public (unauthenticated) config the frontend needs to build the Discord authorize URL.
/// Only the client ID is exposed here - `DISCORD_CLIENT_SECRET` never leaves the server.
#[get("/api/oauth/config")]
pub async fn get_oauth_config() -> ActixResult<HttpResponse> {
    let Ok(client_id) = std::env::var("DISCORD_CLIENT_ID") else {
        return Ok(
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                "DISCORD_CLIENT_ID is not configured",
            )),
        );
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(OAuthConfig {
        client_id,
        redirect_uri: redirect_uri(),
    })))
}

#[derive(serde::Deserialize)]
pub struct OAuthCallback {
    code: Option<String>,
    error: Option<String>,
    #[allow(dead_code)]
    state: Option<String>,
}

#[get("/auth/callback")]
pub async fn oauth_callback(query: web::Query<OAuthCallback>) -> ActixResult<HttpResponse> {
    if let Some(error) = &query.error {
        return Ok(HttpResponse::BadRequest()
            .json(ApiResponse::<()>::error(&format!("OAuth error: {error}"))));
    }

    let Some(code) = &query.code else {
        return Ok(
            HttpResponse::BadRequest().json(ApiResponse::<()>::error("Missing authorization code"))
        );
    };

    // Exchange authorization code for access token
    match exchange_code_for_token(code).await {
        Ok(token_response) => {
            // Prefer postMessage back to the window that opened this popup - it works
            // regardless of whether the dashboard is same-origin (prod, served from
            // /static/app) or on a different origin (dev, Vite on :5173), unlike a
            // `storage` event which only fires across same-origin windows.
            let html = format!(
                r#"
<!DOCTYPE html>
<html>
<head>
    <title>Authentication Success</title>
    <style>
        body {{ font-family: Arial, sans-serif; text-align: center; padding: 50px; }}
        .success {{ color: #28a745; }}
    </style>
</head>
<body>
    <h1 class="success">Authentication Successful!</h1>
    <p>You can now close this window and return to the dashboard.</p>
    <script>
        const token = {token};
        if (window.opener) {{
            window.opener.postMessage({{ type: 'lyre-oauth', token }}, '*');
            window.close();
        }} else {{
            localStorage.setItem('lyre_token', token);
            window.location.href = '/static/app/';
        }}
    </script>
</body>
</html>
            "#,
                token = serde_json::to_string(&token_response.access_token)
                    .unwrap_or_else(|_| "null".to_string())
            );

            Ok(HttpResponse::Ok().content_type("text/html").body(html))
        }
        Err(e) => Ok(
            HttpResponse::BadRequest().json(ApiResponse::<()>::error(&format!(
                "Failed to exchange code: {e}"
            ))),
        ),
    }
}

#[derive(serde::Deserialize)]
struct TokenResponse {
    access_token: String,
    #[allow(dead_code)]
    token_type: String,
    #[allow(dead_code)]
    expires_in: u64,
    #[allow(dead_code)]
    refresh_token: Option<String>,
    #[allow(dead_code)]
    scope: String,
}

async fn exchange_code_for_token(code: &str) -> Result<TokenResponse, Box<dyn std::error::Error>> {
    let client_id = std::env::var("DISCORD_CLIENT_ID")
        .map_err(|_| "DISCORD_CLIENT_ID environment variable not set")?;
    let client_secret = std::env::var("DISCORD_CLIENT_SECRET")
        .map_err(|_| "DISCORD_CLIENT_SECRET environment variable not set")?;
    let redirect_uri = redirect_uri();

    let params = [
        ("client_id", client_id.as_str()),
        ("client_secret", client_secret.as_str()),
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri.as_str()),
    ];

    let client = reqwest::Client::new();
    let response = client
        .post("https://discord.com/api/oauth2/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("Discord API error: {error_text}").into());
    }

    let token_response: TokenResponse = response.json().await?;
    Ok(token_response)
}
