use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::database::schema::queue_history;

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

impl QueueHistory {
    pub fn create(
        conn: &mut SqliteConnection,
        guild_id: &str,
        user_id: &str,
        url: &str,
        title: Option<&str>,
        duration: Option<i32>,
    ) -> QueryResult<usize> {
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

    pub fn get_recent_for_guild(
        conn: &mut SqliteConnection,
        guild_id: &str,
        limit: i64,
    ) -> QueryResult<Vec<QueueHistory>> {
        queue_history::table
            .filter(queue_history::guild_id.eq(guild_id))
            .order(queue_history::played_at.desc())
            .limit(limit)
            .load::<QueueHistory>(conn)
    }

    pub fn get_recent_for_user(
        conn: &mut SqliteConnection,
        user_id: &str,
        limit: i64,
    ) -> QueryResult<Vec<QueueHistory>> {
        queue_history::table
            .filter(queue_history::user_id.eq(user_id))
            .order(queue_history::played_at.desc())
            .limit(limit)
            .load::<QueueHistory>(conn)
    }

    pub fn cleanup_old_entries(
        conn: &mut SqliteConnection,
        days_to_keep: i32,
    ) -> QueryResult<usize> {
        let cutoff_date =
            chrono::Utc::now().naive_utc() - chrono::Duration::days(days_to_keep as i64);

        diesel::delete(queue_history::table)
            .filter(queue_history::played_at.lt(cutoff_date))
            .execute(conn)
    }
}
