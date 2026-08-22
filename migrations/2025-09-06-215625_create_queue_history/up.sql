CREATE TABLE queue_history (
    id SERIAL PRIMARY KEY,
    guild_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    url TEXT NOT NULL,
    title TEXT,
    duration INTEGER,
    played_at TIMESTAMP NOT NULL DEFAULT NOW()
);
