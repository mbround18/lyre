use actix_web::{
    Error as ActixError, FromRequest, HttpMessage, HttpRequest, dev::Payload,
    error::ErrorUnauthorized,
};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::future::{Ready, ready};

const DISCORD_API_BASE: &str = "https://discord.com/api/v10";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    pub discriminator: String,
    pub avatar: Option<String>,
    pub global_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserGuild {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub owner: bool,
    pub permissions: String,
}

pub struct AuthenticatedUser {
    #[allow(dead_code)]
    pub user: DiscordUser,
    pub guilds: Vec<UserGuild>,
}

impl FromRequest for AuthenticatedUser {
    type Error = ActixError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let auth_header = req.headers().get("Authorization");

        if let Some(auth_value) = auth_header
            && let Ok(auth_str) = auth_value.to_str()
            && let Some(token) = auth_str.strip_prefix("Bearer ")
        {
            // For demo purposes, accept any token that starts with "demo_"
            if token.starts_with("demo_") {
                let user = DiscordUser {
                    id: "123456789".to_string(),
                    username: "demouser".to_string(),
                    discriminator: "0000".to_string(),
                    avatar: None,
                    global_name: Some("Demo User".to_string()),
                };

                let guilds = vec![UserGuild {
                    id: "987654321".to_string(),
                    name: "Demo Server".to_string(),
                    icon: None,
                    owner: true,
                    permissions: "8".to_string(), // Administrator
                }];

                return ready(Ok(AuthenticatedUser { user, guilds }));
            }

            // Store the token in the request extensions so endpoints can validate it
            req.extensions_mut().insert(token.to_string());

            // Return a placeholder that indicates we have a token
            // Individual endpoints will need to validate the token themselves
            let user = DiscordUser {
                id: "needs_validation".to_string(),
                username: "token_present".to_string(),
                discriminator: "0000".to_string(),
                avatar: None,
                global_name: Some("Token Present".to_string()),
            };

            let guilds = vec![];

            return ready(Ok(AuthenticatedUser { user, guilds }));
        }

        ready(Err(ErrorUnauthorized(
            "Missing or invalid Authorization header",
        )))
    }
}

// Helper function to validate and get authenticated user data from request
pub async fn get_authenticated_user_from_request(req: &HttpRequest) -> Result<AuthenticatedUser> {
    if let Some(token) = req.extensions().get::<String>() {
        if token.starts_with("demo_") {
            let user = DiscordUser {
                id: "123456789".to_string(),
                username: "demouser".to_string(),
                discriminator: "0000".to_string(),
                avatar: None,
                global_name: Some("Demo User".to_string()),
            };

            let guilds = vec![UserGuild {
                id: "987654321".to_string(),
                name: "Demo Server".to_string(),
                icon: None,
                owner: true,
                permissions: "8".to_string(),
            }];

            return Ok(AuthenticatedUser { user, guilds });
        }

        // Validate real Discord token
        match validate_discord_token(token).await {
            Ok(user) => match get_user_guilds(token).await {
                Ok(guilds) => Ok(AuthenticatedUser { user, guilds }),
                Err(e) => Err(anyhow!("Failed to get user guilds: {}", e)),
            },
            Err(e) => Err(anyhow!("Invalid token: {}", e)),
        }
    } else {
        Err(anyhow!("No token found in request"))
    }
}

/// Validate a Discord access token by calling Discord's API
pub async fn validate_discord_token(access_token: &str) -> Result<DiscordUser> {
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/users/@me", DISCORD_API_BASE))
        .header("Authorization", format!("Bearer {}", access_token))
        .header("User-Agent", "lyre-bot/0.1")
        .send()
        .await
        .map_err(|e| anyhow!("Failed to call Discord API: {}", e))?;

    if !response.status().is_success() {
        return Err(anyhow!("Discord API returned error: {}", response.status()));
    }

    let user: DiscordUser = response
        .json()
        .await
        .map_err(|e| anyhow!("Failed to parse Discord user response: {}", e))?;

    Ok(user)
}

/// Get user's guilds from Discord API
pub async fn get_user_guilds(access_token: &str) -> Result<Vec<UserGuild>> {
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/users/@me/guilds", DISCORD_API_BASE))
        .header("Authorization", format!("Bearer {}", access_token))
        .header("User-Agent", "lyre-bot/0.1")
        .send()
        .await
        .map_err(|e| anyhow!("Failed to call Discord API: {}", e))?;

    if !response.status().is_success() {
        return Err(anyhow!("Discord API returned error: {}", response.status()));
    }

    let guilds: Vec<UserGuild> = response
        .json()
        .await
        .map_err(|e| anyhow!("Failed to parse Discord guilds response: {}", e))?;

    Ok(guilds)
}

/// Check if user has permission to control bot in a specific guild
pub fn user_can_control_guild(user_guilds: &[UserGuild], guild_id: &str) -> bool {
    user_guilds.iter().any(|guild| {
        guild.id == guild_id
            && (
                guild.owner || has_permission(&guild.permissions, 0x8) || // Administrator
            has_permission(&guild.permissions, 0x20) || // Manage Guild
            has_permission(&guild.permissions, 0x100000)
                // Use Voice Activity
            )
    })
}

fn has_permission(permissions_str: &str, permission_bit: u64) -> bool {
    if let Ok(permissions) = permissions_str.parse::<u64>() {
        (permissions & permission_bit) != 0
    } else {
        false
    }
}
