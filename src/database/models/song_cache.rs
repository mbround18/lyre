use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::database::schema::song_cache;

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

impl SongCache {
    pub fn create_or_update(
        conn: &mut SqliteConnection,
        url: &str,
        title: &str,
        duration: Option<i32>,
        thumbnail_url: Option<&str>,
        file_path: Option<&str>,
        file_size: Option<i32>,
    ) -> QueryResult<usize> {
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
                song_cache::last_accessed.eq(chrono::Utc::now().naive_utc()),
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

    pub fn cleanup_old_entries(
        conn: &mut SqliteConnection,
        days_to_keep: i32,
    ) -> QueryResult<usize> {
        let cutoff_date =
            chrono::Utc::now().naive_utc() - chrono::Duration::days(days_to_keep as i64);

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
