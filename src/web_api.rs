use actix_files as fs;
use actix_web::{App, HttpServer, middleware::Logger};
use std::net::Ipv4Addr;

use crate::middleware::AuthMiddleware;

use crate::api::{
    add_to_queue, cleanup_old_data, clear_queue, dashboard_redirect, get_cache_stats,
    get_guild_settings, get_guilds, get_maintenance_stats, get_queue, get_recent_tracks,
    get_song_info, get_test_token, get_user_history, health_metrics, join_voice_channel, livez,
    next_track, oauth_callback, readyz, search_songs, set_volume, skip_track, stop_playback,
    update_guild_settings, validate_auth,
};

pub async fn run_http(bind: Option<String>) -> std::io::Result<()> {
    let bind_addr = bind.unwrap_or_else(|| format!("{}:{}", Ipv4Addr::UNSPECIFIED, 3000));

    HttpServer::new(|| {
        App::new()
            // Add authentication middleware
            .wrap(AuthMiddleware)
            // Add request logging
            .wrap(Logger::default())
            // Health endpoints (no auth required)
            .service(livez)
            .service(readyz)
            .service(health_metrics)
            // Dashboard - serve static files
            .service(fs::Files::new("/static", "./static").show_files_listing())
            .service(dashboard_redirect)
            // OAuth endpoints
            .service(oauth_callback)
            // Development endpoints (debug builds only)
            .service(get_test_token)
            // API endpoints
            .service(validate_auth)
            .service(get_guilds)
            .service(get_queue)
            .service(add_to_queue)
            .service(skip_track)
            .service(clear_queue)
            .service(next_track)
            .service(stop_playback)
            .service(set_volume)
            .service(join_voice_channel)
            .service(search_songs)
            .service(get_song_info)
            // Analytics endpoints
            .service(get_recent_tracks)
            .service(get_guild_settings)
            .service(get_cache_stats)
            .service(update_guild_settings)
            // Maintenance endpoints
            .service(get_maintenance_stats)
            .service(cleanup_old_data)
            .service(get_user_history)
    })
    .bind(bind_addr)?
    .workers(1)
    .run()
    .await
}
