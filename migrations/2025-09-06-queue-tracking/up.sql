-- Create current_queue table to track active queue items
CREATE TABLE current_queue (
    id INTEGER PRIMARY KEY,
    guild_id TEXT NOT NULL,
    url TEXT NOT NULL,
    title TEXT,
    duration INTEGER, -- in seconds
    position INTEGER NOT NULL, -- 0 = currently playing, 1+ = in queue
    added_by TEXT NOT NULL, -- user ID
    added_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(guild_id, position)
);

-- Add current track info to voice_connections
ALTER TABLE voice_connections ADD COLUMN current_track_title TEXT;
ALTER TABLE voice_connections ADD COLUMN is_playing BOOLEAN NOT NULL DEFAULT FALSE;
