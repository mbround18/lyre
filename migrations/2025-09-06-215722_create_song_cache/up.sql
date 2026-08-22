CREATE TABLE song_cache (
    url TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    duration INTEGER,
    thumbnail_url TEXT,
    file_path TEXT,
    file_size INTEGER,
    last_accessed TIMESTAMP NOT NULL DEFAULT NOW(),
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);
