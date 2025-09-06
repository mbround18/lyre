CREATE TABLE queue_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    url TEXT NOT NULL,
    title TEXT,
    duration INTEGER,
    played_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
