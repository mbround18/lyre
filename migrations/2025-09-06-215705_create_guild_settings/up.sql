CREATE TABLE guild_settings (
    guild_id TEXT PRIMARY KEY NOT NULL,
    default_volume REAL NOT NULL DEFAULT 0.5,
    auto_disconnect_minutes INTEGER NOT NULL DEFAULT 5,
    max_queue_size INTEGER NOT NULL DEFAULT 50,
    allowed_roles TEXT,
    blocked_domains TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);
