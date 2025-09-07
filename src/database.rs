use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use std::env;

pub fn establish_connection() -> SqliteConnection {
    dotenvy::dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub mod models;
pub mod schema;
