use diesel::prelude::*;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::database::schema::{voice_connections, queue_history, guild_settings, song_cache};

// Voice Connection Models
#[derive(Queryable, Selectable, Serialize, Deserialize, Debug)]
#[diesel(table_name = voice_connections)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct VoiceConnection {
    pub guild_id: String,
    pub connected_at: NaiveDateTime,
    pub channel_id: Option<String>,
    pub last_activity: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = voice_connections)]
pub struct NewVoiceConnection {
    pub guild_id: String,
    pub channel_id: Option<String>,
}

// Queue History Models
#[derive(Queryable, Selectable, Serialize, Deserialize, Debug)]
#[diesel(table_name = queue_history)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct QueueHistory {
    pub id: Option<i32>,
    pub guild_id: String,
    pub user_id: String,
    pub url: String,
    pub title: Option<String>,
    pub duration: Option<i32>,
    pub played_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = queue_history)]
pub struct NewQueueHistory {
    pub guild_id: String,
    pub user_id: String,
    pub url: String,
    pub title: Option<String>,
    pub duration: Option<i32>,
}

// Guild Settings Models
#[derive(Queryable, Selectable, Serialize, Deserialize, Debug)]
#[diesel(table_name = guild_settings)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct GuildSettings {
    pub guild_id: String,
    pub default_volume: f32,
    pub auto_disconnect_minutes: i32,
    pub max_queue_size: i32,
    pub allowed_roles: Option<String>, // JSON array
    pub blocked_domains: Option<String>, // JSON array
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = guild_settings)]
pub struct NewGuildSettings {
    pub guild_id: String,
    pub default_volume: Option<f32>,
    pub auto_disconnect_minutes: Option<i32>,
    pub max_queue_size: Option<i32>,
    pub allowed_roles: Option<String>,
    pub blocked_domains: Option<String>,
}

// Song Cache Models
#[derive(Queryable, Selectable, Serialize, Deserialize, Debug)]
#[diesel(table_name = song_cache)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SongCache {
    pub url: String,
    pub title: String,
    pub duration: Option<i32>,
    pub thumbnail_url: Option<String>,
    pub file_path: Option<String>,
    pub file_size: Option<i32>,
    pub last_accessed: NaiveDateTime,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = song_cache)]
pub struct NewSongCache {
    pub url: String,
    pub title: String,
    pub duration: Option<i32>,
    pub thumbnail_url: Option<String>,
    pub file_path: Option<String>,
    pub file_size: Option<i32>,
}

// Voice Connection Implementation
impl VoiceConnection {
    pub fn create(conn: &mut SqliteConnection, guild_id: &str, channel_id: Option<&str>) -> QueryResult<VoiceConnection> {
        let new_connection = NewVoiceConnection {
            guild_id: guild_id.to_string(),
            channel_id: channel_id.map(|s| s.to_string()),
        };

        diesel::insert_into(voice_connections::table)
            .values(&new_connection)
            .execute(conn)?;
            
        // For SQLite, we need to query back the inserted record
        Self::find_by_guild_id(conn, guild_id)?
            .ok_or_else(|| diesel::result::Error::NotFound)
    }

    pub fn find_by_guild_id(conn: &mut SqliteConnection, guild_id: &str) -> QueryResult<Option<VoiceConnection>> {
        voice_connections::table
            .filter(voice_connections::guild_id.eq(guild_id))
            .first::<VoiceConnection>(conn)
            .optional()
    }

    pub fn update_last_activity(conn: &mut SqliteConnection, guild_id: &str) -> QueryResult<usize> {
        diesel::update(voice_connections::table)
            .filter(voice_connections::guild_id.eq(guild_id))
            .set(voice_connections::last_activity.eq(chrono::Utc::now().naive_utc()))
            .execute(conn)
    }

    pub fn disconnect(conn: &mut SqliteConnection, guild_id: &str) -> QueryResult<usize> {
        diesel::delete(voice_connections::table)
            .filter(voice_connections::guild_id.eq(guild_id))
            .execute(conn)
    }

    pub fn get_all_connected(conn: &mut SqliteConnection) -> QueryResult<Vec<VoiceConnection>> {
        voice_connections::table
            .load::<VoiceConnection>(conn)
    }

    pub fn is_connected(conn: &mut SqliteConnection, guild_id: &str) -> bool {
        Self::find_by_guild_id(conn, guild_id)
            .map(|result| result.is_some())
            .unwrap_or(false)
    }
}

// Queue History Implementation
impl QueueHistory {
    pub fn create(conn: &mut SqliteConnection, guild_id: &str, user_id: &str, url: &str, title: Option<&str>, duration: Option<i32>) -> QueryResult<usize> {
        let new_history = NewQueueHistory {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            url: url.to_string(),
            title: title.map(|s| s.to_string()),
            duration,
        };

        diesel::insert_into(queue_history::table)
            .values(&new_history)
            .execute(conn)
    }

    pub fn get_recent_for_guild(conn: &mut SqliteConnection, guild_id: &str, limit: i64) -> QueryResult<Vec<QueueHistory>> {
        queue_history::table
            .filter(queue_history::guild_id.eq(guild_id))
            .order(queue_history::played_at.desc())
            .limit(limit)
            .load::<QueueHistory>(conn)
    }

    pub fn get_recent_for_user(conn: &mut SqliteConnection, user_id: &str, limit: i64) -> QueryResult<Vec<QueueHistory>> {
        queue_history::table
            .filter(queue_history::user_id.eq(user_id))
            .order(queue_history::played_at.desc())
            .limit(limit)
            .load::<QueueHistory>(conn)
    }

    pub fn cleanup_old_entries(conn: &mut SqliteConnection, days_to_keep: i32) -> QueryResult<usize> {
        let cutoff_date = chrono::Utc::now().naive_utc() - chrono::Duration::days(days_to_keep as i64);
        
        diesel::delete(queue_history::table)
            .filter(queue_history::played_at.lt(cutoff_date))
            .execute(conn)
    }
}

// Guild Settings Implementation
impl GuildSettings {
    pub fn create_or_update(conn: &mut SqliteConnection, guild_id: &str) -> QueryResult<GuildSettings> {
        let new_settings = NewGuildSettings {
            guild_id: guild_id.to_string(),
            default_volume: None,
            auto_disconnect_minutes: None,
            max_queue_size: None,
            allowed_roles: None,
            blocked_domains: None,
        };

        diesel::insert_into(guild_settings::table)
            .values(&new_settings)
            .on_conflict(guild_settings::guild_id)
            .do_update()
            .set(guild_settings::updated_at.eq(chrono::Utc::now().naive_utc()))
            .execute(conn)?;

        Self::find_by_guild_id(conn, guild_id)?
            .ok_or_else(|| diesel::result::Error::NotFound)
    }

    pub fn find_by_guild_id(conn: &mut SqliteConnection, guild_id: &str) -> QueryResult<Option<GuildSettings>> {
        guild_settings::table
            .filter(guild_settings::guild_id.eq(guild_id))
            .first::<GuildSettings>(conn)
            .optional()
    }

    pub fn update_volume(conn: &mut SqliteConnection, guild_id: &str, volume: f32) -> QueryResult<usize> {
        diesel::update(guild_settings::table)
            .filter(guild_settings::guild_id.eq(guild_id))
            .set((
                guild_settings::default_volume.eq(volume),
                guild_settings::updated_at.eq(chrono::Utc::now().naive_utc())
            ))
            .execute(conn)
    }

    pub fn update_auto_disconnect(conn: &mut SqliteConnection, guild_id: &str, minutes: i32) -> QueryResult<usize> {
        diesel::update(guild_settings::table)
            .filter(guild_settings::guild_id.eq(guild_id))
            .set((
                guild_settings::auto_disconnect_minutes.eq(minutes),
                guild_settings::updated_at.eq(chrono::Utc::now().naive_utc())
            ))
            .execute(conn)
    }

    pub fn update_max_queue_size(conn: &mut SqliteConnection, guild_id: &str, size: i32) -> QueryResult<usize> {
        diesel::update(guild_settings::table)
            .filter(guild_settings::guild_id.eq(guild_id))
            .set((
                guild_settings::max_queue_size.eq(size),
                guild_settings::updated_at.eq(chrono::Utc::now().naive_utc())
            ))
            .execute(conn)
    }
}

// Song Cache Implementation
impl SongCache {
    pub fn create_or_update(conn: &mut SqliteConnection, url: &str, title: &str, duration: Option<i32>, thumbnail_url: Option<&str>, file_path: Option<&str>, file_size: Option<i32>) -> QueryResult<usize> {
        let new_cache = NewSongCache {
            url: url.to_string(),
            title: title.to_string(),
            duration,
            thumbnail_url: thumbnail_url.map(|s| s.to_string()),
            file_path: file_path.map(|s| s.to_string()),
            file_size,
        };

        diesel::insert_into(song_cache::table)
            .values(&new_cache)
            .on_conflict(song_cache::url)
            .do_update()
            .set((
                song_cache::title.eq(&new_cache.title),
                song_cache::duration.eq(&new_cache.duration),
                song_cache::thumbnail_url.eq(&new_cache.thumbnail_url),
                song_cache::file_path.eq(&new_cache.file_path),
                song_cache::file_size.eq(&new_cache.file_size),
                song_cache::last_accessed.eq(chrono::Utc::now().naive_utc())
            ))
            .execute(conn)
    }

    pub fn find_by_url(conn: &mut SqliteConnection, url: &str) -> QueryResult<Option<SongCache>> {
        song_cache::table
            .filter(song_cache::url.eq(url))
            .first::<SongCache>(conn)
            .optional()
    }

    pub fn update_last_accessed(conn: &mut SqliteConnection, url: &str) -> QueryResult<usize> {
        diesel::update(song_cache::table)
            .filter(song_cache::url.eq(url))
            .set(song_cache::last_accessed.eq(chrono::Utc::now().naive_utc()))
            .execute(conn)
    }

    pub fn cleanup_old_entries(conn: &mut SqliteConnection, days_to_keep: i32) -> QueryResult<usize> {
        let cutoff_date = chrono::Utc::now().naive_utc() - chrono::Duration::days(days_to_keep as i64);
        
        diesel::delete(song_cache::table)
            .filter(song_cache::last_accessed.lt(cutoff_date))
            .execute(conn)
    }

    pub fn get_cache_size(conn: &mut SqliteConnection) -> QueryResult<i64> {
        use diesel::dsl::sum;
        
        song_cache::table
            .select(sum(song_cache::file_size))
            .first::<Option<i64>>(conn)
            .map(|result| result.unwrap_or(0))
    }
}
