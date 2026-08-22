CREATE TABLE voice_connections (
    guild_id TEXT PRIMARY KEY NOT NULL,
    connected_at TIMESTAMP NOT NULL DEFAULT NOW(),
    channel_id TEXT,
    last_activity TIMESTAMP NOT NULL DEFAULT NOW()
);
