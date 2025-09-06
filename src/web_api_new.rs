use actix_web::{web, App, HttpServer};
use actix_files as fs;
use std::net::Ipv4Addr;

mod api;
use api::*;

pub async fn run_http(bind: Option<String>) -> std::io::Result<()> {
    let bind_addr = bind.unwrap_or_else(|| format!("{}:{}", Ipv4Addr::UNSPECIFIED, 3000));
    
    HttpServer::new(|| {
        App::new()
            // Health endpoints
            .service(readyz)
            .service(livez)
            .service(metrics)
            // Dashboard - serve static files
            .service(fs::Files::new("/static", "./static").show_files_listing())
            .service(dashboard)
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
    })
    .bind(bind_addr)?
    .workers(1)
    .run()
    .await
}
