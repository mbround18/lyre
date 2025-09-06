use actix_files as fs;
use actix_web::{App, HttpServer};
use std::net::Ipv4Addr;

use crate::api::{
    add_to_queue, clear_queue, dashboard_redirect, get_guilds, get_queue, get_song_info, health_metrics,
    livez, oauth_callback, play_pause, readyz, search_songs, set_volume, skip_track, stop_playback,
    validate_auth, get_recent_tracks, get_guild_settings, get_cache_stats, update_guild_settings,
};

pub async fn run_http(bind: Option<String>) -> std::io::Result<()> {
    let bind_addr = bind.unwrap_or_else(|| format!("{}:{}", Ipv4Addr::UNSPECIFIED, 3000));

    HttpServer::new(|| {
        App::new()
            // Health endpoints
            .service(readyz)
            .service(livez)
            .service(health_metrics)
            // Dashboard - serve static files
            .service(fs::Files::new("/static", "./static").show_files_listing())
            .service(dashboard_redirect)
            // OAuth endpoints
            .service(oauth_callback)
            // API endpoints
            .service(validate_auth)
            .service(get_guilds)
            .service(get_queue)
            .service(add_to_queue)
            .service(skip_track)
            .service(clear_queue)
            .service(play_pause)
            .service(stop_playback)
            .service(set_volume)
            .service(search_songs)
            .service(get_song_info)
            // Analytics endpoints
            .service(get_recent_tracks)
            .service(get_guild_settings)
            .service(get_cache_stats)
            .service(update_guild_settings)
    })
    .bind(bind_addr)?
    .workers(1)
    .run()
    .await
}
