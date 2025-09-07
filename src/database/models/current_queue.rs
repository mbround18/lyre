use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::database::schema::current_queue;

#[derive(Queryable, Selectable, Serialize, Deserialize, Debug)]
#[diesel(table_name = current_queue)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct CurrentQueue {
    pub id: Option<i32>,
    pub guild_id: String,
    pub url: String,
    pub title: Option<String>,
    pub duration: Option<i32>,
    pub position: i32,
    pub added_by: String,
    pub added_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = current_queue)]
pub struct NewCurrentQueue {
    pub guild_id: String,
    pub url: String,
    pub title: Option<String>,
    pub duration: Option<i32>,
    pub position: i32,
    pub added_by: String,
}

impl CurrentQueue {
    pub fn get_guild_queue(
        conn: &mut SqliteConnection,
        guild_id: &str,
    ) -> QueryResult<Vec<CurrentQueue>> {
        current_queue::table
            .filter(current_queue::guild_id.eq(guild_id))
            .order(current_queue::position.asc())
            .select(CurrentQueue::as_select())
            .load::<CurrentQueue>(conn)
    }

    pub fn get_current_track(
        conn: &mut SqliteConnection,
        guild_id: &str,
    ) -> QueryResult<Option<CurrentQueue>> {
        current_queue::table
            .filter(current_queue::guild_id.eq(guild_id))
            .filter(current_queue::position.eq(0))
            .select(CurrentQueue::as_select())
            .first::<CurrentQueue>(conn)
            .optional()
    }

    pub fn add_to_queue(
        conn: &mut SqliteConnection,
        guild_id: &str,
        url: &str,
        title: Option<&str>,
        duration: Option<i32>,
        added_by: &str,
    ) -> QueryResult<CurrentQueue> {
        // Get the next position
        let next_position = current_queue::table
            .filter(current_queue::guild_id.eq(guild_id))
            .select(current_queue::position)
            .order(current_queue::position.desc())
            .first::<i32>(conn)
            .optional()?
            .map(|pos| pos + 1)
            .unwrap_or(0);

        let new_queue_item = NewCurrentQueue {
            guild_id: guild_id.to_string(),
            url: url.to_string(),
            title: title.map(|s| s.to_string()),
            duration,
            position: next_position,
            added_by: added_by.to_string(),
        };

        diesel::insert_into(current_queue::table)
            .values(&new_queue_item)
            .execute(conn)?;

        // Return the inserted item
        current_queue::table
            .filter(current_queue::guild_id.eq(guild_id))
            .filter(current_queue::position.eq(next_position))
            .select(CurrentQueue::as_select())
            .first::<CurrentQueue>(conn)
    }

    pub fn advance_queue(conn: &mut SqliteConnection, guild_id: &str) -> QueryResult<()> {
        // Remove current track (position 0)
        diesel::delete(current_queue::table)
            .filter(current_queue::guild_id.eq(guild_id))
            .filter(current_queue::position.eq(0))
            .execute(conn)?;

        // Move all other tracks up one position
        diesel::update(current_queue::table)
            .filter(current_queue::guild_id.eq(guild_id))
            .set(current_queue::position.eq(current_queue::position - 1))
            .execute(conn)?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn clear_guild_queue(conn: &mut SqliteConnection, guild_id: &str) -> QueryResult<usize> {
        diesel::delete(current_queue::table)
            .filter(current_queue::guild_id.eq(guild_id))
            .execute(conn)
    }
}
