use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use std::env;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

type PgPool = Pool<ConnectionManager<PgConnection>>;

// Built once on first use and reused for the lifetime of the process, instead of every
// call site opening its own TCP connection to Postgres (previously up to 6 per command).
static POOL: std::sync::LazyLock<PgPool> = std::sync::LazyLock::new(|| {
    dotenvy::dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(&database_url);
    let pool = Pool::builder()
        .build(manager)
        .unwrap_or_else(|e| panic!("Error creating database pool for {database_url}: {e}"));

    // Run migrations once at pool creation instead of checking on every connection.
    let mut connection = pool
        .get()
        .unwrap_or_else(|e| panic!("Error connecting to {database_url}: {e}"));
    if needs_migration(&mut connection) {
        tracing::info!("Running database migrations...");
        connection
            .run_pending_migrations(MIGRATIONS)
            .unwrap_or_else(|e| panic!("Failed to run migrations: {e}"));
        tracing::info!("Database migrations completed successfully");
    }

    pool
});

pub fn establish_connection() -> PooledConnection<ConnectionManager<PgConnection>> {
    POOL.get()
        .unwrap_or_else(|e| panic!("Failed to get pooled database connection: {e}"))
}

fn needs_migration(conn: &mut PgConnection) -> bool {
    // Check if we have pending migrations
    conn.has_pending_migration(MIGRATIONS).unwrap_or(true)
}

#[path = "database/models/mod.rs"]
pub mod models;
pub mod schema;

#[path = "database/listener.rs"]
pub mod listener;
pub use listener::start_listener;
