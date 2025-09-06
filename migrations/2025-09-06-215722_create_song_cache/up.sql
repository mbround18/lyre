CREATE TABLE song_cache (
    url TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    duration INTEGER,
    thumbnail_url TEXT,
    file_path TEXT,
    file_size INTEGER,
    last_accessed DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
