use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::database::schema::guild_settings;

#[derive(Queryable, Selectable, Serialize, Deserialize, Debug)]
#[diesel(table_name = guild_settings)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct GuildSettings {
    pub guild_id: String,
    pub default_volume: f32,
    pub auto_disconnect_minutes: i32,
    pub max_queue_size: i32,
    pub allowed_roles: Option<String>,   // JSON array
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

impl GuildSettings {
    pub fn create_or_update(
        conn: &mut SqliteConnection,
        guild_id: &str,
    ) -> QueryResult<GuildSettings> {
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

        Self::find_by_guild_id(conn, guild_id)?.ok_or_else(|| diesel::result::Error::NotFound)
    }

    pub fn find_by_guild_id(
        conn: &mut SqliteConnection,
        guild_id: &str,
    ) -> QueryResult<Option<GuildSettings>> {
        guild_settings::table
            .filter(guild_settings::guild_id.eq(guild_id))
            .first::<GuildSettings>(conn)
            .optional()
    }

    pub fn update_volume(
        conn: &mut SqliteConnection,
        guild_id: &str,
        volume: f32,
    ) -> QueryResult<usize> {
        diesel::update(guild_settings::table)
            .filter(guild_settings::guild_id.eq(guild_id))
            .set((
                guild_settings::default_volume.eq(volume),
                guild_settings::updated_at.eq(chrono::Utc::now().naive_utc()),
            ))
            .execute(conn)
    }

    pub fn update_auto_disconnect(
        conn: &mut SqliteConnection,
        guild_id: &str,
        minutes: i32,
    ) -> QueryResult<usize> {
        diesel::update(guild_settings::table)
            .filter(guild_settings::guild_id.eq(guild_id))
            .set((
                guild_settings::auto_disconnect_minutes.eq(minutes),
                guild_settings::updated_at.eq(chrono::Utc::now().naive_utc()),
            ))
            .execute(conn)
    }

    pub fn update_max_queue_size(
        conn: &mut SqliteConnection,
        guild_id: &str,
        size: i32,
    ) -> QueryResult<usize> {
        diesel::update(guild_settings::table)
            .filter(guild_settings::guild_id.eq(guild_id))
            .set((
                guild_settings::max_queue_size.eq(size),
                guild_settings::updated_at.eq(chrono::Utc::now().naive_utc()),
            ))
            .execute(conn)
    }
}
