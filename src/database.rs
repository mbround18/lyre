use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use std::env;
use std::path::Path;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub fn establish_connection() -> SqliteConnection {
    dotenvy::dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Extract file path from database URL (remove "sqlite://" prefix if present)
    let db_path = database_url
        .strip_prefix("sqlite://")
        .unwrap_or(&database_url);

    // Create parent directory if it doesn't exist
    if let Some(parent) = Path::new(db_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|e| panic!("Failed to create database directory: {}", e));
        }
    }

    // Create the database file if it doesn't exist
    let is_new_db = !Path::new(db_path).exists();
    if is_new_db {
        tracing::info!("Database file not found, creating: {}", db_path);
        std::fs::File::create(db_path)
            .unwrap_or_else(|e| panic!("Failed to create database file: {}", e));
    }

    let mut connection = SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));

    // Run migrations automatically
    if is_new_db || needs_migration(&mut connection) {
        tracing::info!("Running database migrations...");
        connection
            .run_pending_migrations(MIGRATIONS)
            .unwrap_or_else(|e| panic!("Failed to run migrations: {}", e));
        tracing::info!("Database migrations completed successfully");
    }

    connection
}

fn needs_migration(conn: &mut SqliteConnection) -> bool {
    // Check if we have pending migrations
    match conn.has_pending_migration(MIGRATIONS) {
        Ok(has_pending) => has_pending,
        Err(_) => {
            // If we can't check, assume we need to run migrations
            true
        }
    }
}

#[path = "database/models/mod.rs"]
pub mod models;
pub mod schema;
