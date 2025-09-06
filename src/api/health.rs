use super::types::ProbeResp;
use crate::metrics::{METRICS, MetricsSnapshot};
use actix_web::{HttpResponse, Responder, get};

#[get("/k8s/readyz")]
pub async fn readyz() -> impl Responder {
    if METRICS.is_ready() {
        HttpResponse::Ok().json(ProbeResp { status: "ok" })
    } else {
        HttpResponse::ServiceUnavailable().json(ProbeResp { status: "starting" })
    }
}

#[get("/k8s/livez")]
pub async fn livez() -> impl Responder {
    HttpResponse::Ok().json(ProbeResp { status: "ok" })
}

#[get("/k8s/metrics")]
pub async fn health_metrics() -> impl Responder {
    let m: MetricsSnapshot = METRICS.snapshot();
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
