use diesel::prelude::*;
use diesel::pg::PgConnection;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use std::env;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub fn establish_connection() -> PgConnection {
    dotenvy::dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let mut connection = PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));

    // Run migrations automatically
    if needs_migration(&mut connection) {
        tracing::info!("Running database migrations...");
        connection
            .run_pending_migrations(MIGRATIONS)
            .unwrap_or_else(|e| panic!("Failed to run migrations: {}", e));
        tracing::info!("Database migrations completed successfully");
    }

    connection
}

fn needs_migration(conn: &mut PgConnection) -> bool {
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

#[path = "database/listener.rs"]
pub mod listener;
pub use listener::start_listener;
