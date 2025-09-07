-- Remove current queue tracking
DROP TABLE current_queue;

-- Remove current track info from voice_connections
ALTER TABLE voice_connections DROP COLUMN current_track_title;
ALTER TABLE voice_connections DROP COLUMN is_playing;
