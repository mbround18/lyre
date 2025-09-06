use super::types::ApiResponse;
use actix_web::{HttpResponse, Result as ActixResult, get, web};

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
            .json(ApiResponse::<()>::error(&format!("OAuth error: {}", error))));
    }

    let code = match &query.code {
        Some(code) => code,
        None => {
            return Ok(HttpResponse::BadRequest()
                .json(ApiResponse::<()>::error("Missing authorization code")));
        }
    };

    // Exchange authorization code for access token
    match exchange_code_for_token(code).await {
        Ok(token_response) => {
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
        localStorage.setItem('discord_token', '{}');
        localStorage.setItem('token_type', '{}');
        window.close();
        // If window.close() doesn't work (popup blockers), redirect back
        setTimeout(() => {{
            window.location.href = '/';
        }}, 2000);
    </script>
</body>
</html>
            "#,
                token_response.access_token, token_response.token_type
            );

            Ok(HttpResponse::Ok().content_type("text/html").body(html))
        }
        Err(e) => Ok(
            HttpResponse::BadRequest().json(ApiResponse::<()>::error(&format!(
                "Failed to exchange code: {}",
                e
            ))),
        ),
    }
}

#[derive(serde::Deserialize)]
struct TokenResponse {
    access_token: String,
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
    let redirect_uri = std::env::var("DISCORD_REDIRECT_URI")
        .unwrap_or_else(|_| "http://localhost:3000/auth/callback".to_string());

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
        return Err(format!("Discord API error: {}", error_text).into());
    }

    let token_response: TokenResponse = response.json().await?;
    Ok(token_response)
}
