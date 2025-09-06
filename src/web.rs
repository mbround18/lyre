use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use serde::Serialize;
use std::net::Ipv4Addr;

use crate::metrics::{METRICS, MetricsSnapshot};

#[derive(Serialize)]
struct ProbeResp<'a> { status: &'a str }

#[get("/k8s/readyz")]
async fn readyz() -> impl Responder {
    if METRICS.is_ready() { HttpResponse::Ok().json(ProbeResp { status: "ok" }) }
    else { HttpResponse::ServiceUnavailable().json(ProbeResp { status: "starting" }) }
}

#[get("/k8s/livez")]
async fn livez() -> impl Responder {
    HttpResponse::Ok().json(ProbeResp { status: "ok" })
}

#[get("/k8s/metrics")]
async fn metrics() -> impl Responder {
    let m: MetricsSnapshot = METRICS.snapshot();
    // Prometheus-like text exposition (simple)
    let body = format!(
        concat!(
            "# HELP lyre_uptime_seconds Seconds since process start\n",
            "# TYPE lyre_uptime_seconds counter\n",
            "lyre_uptime_seconds {}\n",
            "# HELP lyre_ready 1 if ready, 0 otherwise\n",
            "# TYPE lyre_ready gauge\n",
            "lyre_ready {}\n",
            "# HELP lyre_active_voice_calls Number of active voice calls\n",
            "# TYPE lyre_active_voice_calls gauge\n",
            "lyre_active_voice_calls {}\n",
            "# HELP lyre_connected_guilds Number of connected guilds (approx)\n",
            "# TYPE lyre_connected_guilds gauge\n",
            "lyre_connected_guilds {}\n",
            "# HELP lyre_total_queue_len Total tracks enqueued across calls (approx)\n",
            "# TYPE lyre_total_queue_len gauge\n",
            "lyre_total_queue_len {}\n",
            "# HELP lyre_downloads_bytes Total size of downloads folder in bytes\n",
            "# TYPE lyre_downloads_bytes gauge\n",
            "lyre_downloads_bytes {}\n",
            "# HELP lyre_downloads_files Total files in downloads folder\n",
            "# TYPE lyre_downloads_files gauge\n",
            "lyre_downloads_files {}\n"
        ),
        m.uptime_secs,
        if m.ready { 1 } else { 0 },
        m.active_voice_calls,
        m.connected_guilds,
        m.total_queue_len,
        m.downloads_bytes,
        m.downloads_files,
    );
    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4")
        .body(body)
}

pub async fn run_http(bind: Option<String>) -> std::io::Result<()> {
    let bind_addr = bind.unwrap_or_else(|| format!("{}:{}", Ipv4Addr::UNSPECIFIED, 3000));
    HttpServer::new(|| {
        App::new()
            .service(readyz)
            .service(livez)
            .service(metrics)
    })
    .bind(bind_addr)?
    .workers(1) // lightweight
    .run()
    .await
}
