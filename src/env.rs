use anyhow::{Result, anyhow};

pub fn read_discord_token() -> Result<String> {
    const CANDIDATES: &[&str] = &[
        "DISCORD_TOKEN",
        "DISCORD_BOT_TOKEN",
        "BOT_TOKEN",
        "DOCKER_TOKEN",
    ];
    for key in CANDIDATES {
        if let Ok(val) = std::env::var(key)
            && !val.is_empty()
        {
            return Ok(val);
        }
    }
    Err(anyhow!(
        "Set one of DISCORD_TOKEN, DISCORD_BOT_TOKEN, BOT_TOKEN, or DOCKER_TOKEN in environment"
    ))
}
