CREATE TABLE guild_settings (
    guild_id TEXT PRIMARY KEY NOT NULL,
    default_volume REAL NOT NULL DEFAULT 0.5,
    auto_disconnect_minutes INTEGER NOT NULL DEFAULT 5,
    max_queue_size INTEGER NOT NULL DEFAULT 50,
    allowed_roles TEXT, -- JSON array of role IDs
    blocked_domains TEXT, -- JSON array of blocked domains
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
