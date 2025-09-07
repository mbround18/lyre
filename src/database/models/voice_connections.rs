use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::database::schema::voice_connections;

#[derive(Queryable, Selectable, Serialize, Deserialize, Debug)]
#[diesel(table_name = voice_connections)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct VoiceConnection {
    pub guild_id: String,
    pub connected_at: NaiveDateTime,
    pub channel_id: Option<String>,
    pub last_activity: NaiveDateTime,
    pub current_track_title: Option<String>,
    pub is_playing: bool,
}

#[derive(Insertable)]
#[diesel(table_name = voice_connections)]
pub struct NewVoiceConnection {
    pub guild_id: String,
    pub channel_id: Option<String>,
}

impl VoiceConnection {
    pub fn create(
        conn: &mut SqliteConnection,
        guild_id: &str,
        channel_id: Option<&str>,
    ) -> QueryResult<VoiceConnection> {
        let new_connection = NewVoiceConnection {
            guild_id: guild_id.to_string(),
            channel_id: channel_id.map(|s| s.to_string()),
        };

        diesel::insert_into(voice_connections::table)
            .values(&new_connection)
            .execute(conn)?;

        // For SQLite, we need to query back the inserted record
        Self::find_by_guild_id(conn, guild_id)?.ok_or_else(|| diesel::result::Error::NotFound)
    }

    pub fn create_or_update(
        conn: &mut SqliteConnection,
        guild_id: &str,
        channel_id: Option<&str>,
    ) -> QueryResult<VoiceConnection> {
        // First try to update if exists
        if let Some(_existing) = Self::find_by_guild_id(conn, guild_id)? {
            diesel::update(voice_connections::table)
                .filter(voice_connections::guild_id.eq(guild_id))
                .set((
                    voice_connections::channel_id.eq(channel_id),
                    voice_connections::last_activity.eq(chrono::Utc::now().naive_utc()),
                ))
                .execute(conn)?;

            // Return updated record
            Self::find_by_guild_id(conn, guild_id)?.ok_or_else(|| diesel::result::Error::NotFound)
        } else {
            // Create new if doesn't exist
            Self::create(conn, guild_id, channel_id)
        }
    }

    pub fn find_by_guild_id(
        conn: &mut SqliteConnection,
        guild_id: &str,
    ) -> QueryResult<Option<VoiceConnection>> {
        voice_connections::table
            .filter(voice_connections::guild_id.eq(guild_id))
            .select(VoiceConnection::as_select())
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
            .select(VoiceConnection::as_select())
            .load::<VoiceConnection>(conn)
    }

    pub fn is_connected(conn: &mut SqliteConnection, guild_id: &str) -> bool {
        Self::find_by_guild_id(conn, guild_id)
            .map(|result| result.is_some())
            .unwrap_or(false)
    }

    pub fn clear_all_connections(conn: &mut SqliteConnection) -> QueryResult<usize> {
        diesel::delete(voice_connections::table).execute(conn)
    }

    /// Get voice connections that have a channel_id set but may need to be joined
    /// This is used to process API requests for joining voice channels
    pub fn get_pending_joins(conn: &mut SqliteConnection) -> QueryResult<Vec<VoiceConnection>> {
        voice_connections::table
            .filter(voice_connections::channel_id.is_not_null())
            .select(VoiceConnection::as_select())
            .load::<VoiceConnection>(conn)
    }

    /// Delete a voice connection record
    pub fn delete(conn: &mut SqliteConnection, guild_id: &str) -> QueryResult<usize> {
        diesel::delete(voice_connections::table)
            .filter(voice_connections::guild_id.eq(guild_id))
            .execute(conn)
    }

    /// Update playing status and current track
    pub fn update_playing_status(
        conn: &mut SqliteConnection,
        guild_id: &str,
        is_playing: bool,
        current_track_title: Option<&str>,
    ) -> QueryResult<usize> {
        diesel::update(voice_connections::table)
            .filter(voice_connections::guild_id.eq(guild_id))
            .set((
                voice_connections::is_playing.eq(is_playing),
                voice_connections::current_track_title.eq(current_track_title),
                voice_connections::last_activity.eq(chrono::Utc::now().naive_utc()),
            ))
            .execute(conn)
    }
}
