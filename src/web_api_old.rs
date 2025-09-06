use actix_web::{get, post, delete, put, web, App, HttpResponse, HttpServer, Responder, Result as ActixResult};
use actix_files as fs;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

use crate::auth::{AuthenticatedUser, validate_discord_token, get_user_guilds, user_can_control_guild};
use crate::metrics::{METRICS, MetricsSnapshot};

#[derive(Serialize)]
struct ProbeResp<'a> { 
    status: &'a str 
}

#[derive(Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

#[derive(Serialize)]
struct QueueInfo {
    guild_id: String,
    current_track: Option<TrackInfo>,
    queue: Vec<TrackInfo>,
    position: usize,
    is_playing: bool,
}

#[derive(Serialize)]
struct TrackInfo {
    title: String,
    url: String,
    duration: Option<u64>,
    position: usize,
}

#[derive(Serialize)]
struct GuildInfo {
    id: String,
    name: String,
    connected: bool,
    voice_channel: Option<String>,
    queue_length: usize,
}

#[derive(Deserialize)]
struct PlayRequest {
    url: String,
    channel_id: Option<String>,
}

#[derive(Deserialize)]
struct VolumeRequest {
    volume: f32,
}

#[derive(Deserialize)]
struct AuthRequest {
    access_token: String,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    
    fn error(message: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.to_string()),
        }
    }
}

// Health check endpoints
#[get("/k8s/readyz")]
async fn readyz() -> impl Responder {
    if METRICS.is_ready() { 
        HttpResponse::Ok().json(ProbeResp { status: "ok" }) 
    } else { 
        HttpResponse::ServiceUnavailable().json(ProbeResp { status: "starting" }) 
    }
}

#[get("/k8s/livez")]
async fn livez() -> impl Responder {
    HttpResponse::Ok().json(ProbeResp { status: "ok" })
}

#[get("/k8s/metrics")]
async fn metrics() -> impl Responder {
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

// Authentication endpoint
#[post("/api/auth/validate")]
async fn validate_auth(req: web::Json<AuthRequest>) -> ActixResult<HttpResponse> {
    match validate_discord_token(&req.access_token).await {
        Ok(user) => {
            match get_user_guilds(&req.access_token).await {
                Ok(guilds) => {
                    let response = serde_json::json!({
                        "user": user,
                        "guilds": guilds
                    });
                    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
                }
                Err(e) => Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(&format!("Failed to get guilds: {}", e))))
            }
        }
        Err(e) => Ok(HttpResponse::Unauthorized().json(ApiResponse::<()>::error(&format!("Invalid token: {}", e))))
    }
}

// Guild management endpoints
#[get("/api/guilds")]
async fn get_guilds(user: AuthenticatedUser) -> ActixResult<HttpResponse> {
    // Convert user guilds to GuildInfo with connection status
    let guild_infos: Vec<GuildInfo> = user.guilds.iter().map(|guild| {
        // In a real implementation, we'd check if the bot is connected to this guild
        GuildInfo {
            id: guild.id.clone(),
            name: guild.name.clone(),
            connected: false, // TODO: Check actual connection status
            voice_channel: None, // TODO: Get current voice channel
            queue_length: 0, // TODO: Get actual queue length
        }
    }).collect();
    
    Ok(HttpResponse::Ok().json(ApiResponse::success(guild_infos)))
}

// Queue management endpoints
#[get("/api/queue/{guild_id}")]
async fn get_queue(path: web::Path<String>, user: AuthenticatedUser) -> ActixResult<HttpResponse> {
    let guild_id = path.into_inner();
    
    if !user_can_control_guild(&user.guilds, &guild_id) {
        return Ok(HttpResponse::Forbidden().json(ApiResponse::<()>::error("No permission for this guild")));
    }
    
    // TODO: Get actual queue from Songbird
    let queue_info = QueueInfo {
        guild_id: guild_id.clone(),
        current_track: None,
        queue: vec![],
        position: 0,
        is_playing: false,
    };
    
    Ok(HttpResponse::Ok().json(ApiResponse::success(queue_info)))
}

#[post("/api/queue/{guild_id}/add")]
async fn add_to_queue(
    path: web::Path<String>, 
    _req: web::Json<PlayRequest>,
    user: AuthenticatedUser
) -> ActixResult<HttpResponse> {
    let guild_id = path.into_inner();
    
    if !user_can_control_guild(&user.guilds, &guild_id) {
        return Ok(HttpResponse::Forbidden().json(ApiResponse::<()>::error("No permission for this guild")));
    }
    
    // TODO: Implement actual queue addition
    // This would need access to the Songbird manager
    
    Ok(HttpResponse::Ok().json(ApiResponse::success("Track added to queue")))
}

#[post("/api/queue/{guild_id}/skip")]
async fn skip_track(path: web::Path<String>, user: AuthenticatedUser) -> ActixResult<HttpResponse> {
    let guild_id = path.into_inner();
    
    if !user_can_control_guild(&user.guilds, &guild_id) {
        return Ok(HttpResponse::Forbidden().json(ApiResponse::<()>::error("No permission for this guild")));
    }
    
    // TODO: Implement actual skip functionality
    
    Ok(HttpResponse::Ok().json(ApiResponse::success("Track skipped")))
}

#[delete("/api/queue/{guild_id}")]
async fn clear_queue(path: web::Path<String>, user: AuthenticatedUser) -> ActixResult<HttpResponse> {
    let guild_id = path.into_inner();
    
    if !user_can_control_guild(&user.guilds, &guild_id) {
        return Ok(HttpResponse::Forbidden().json(ApiResponse::<()>::error("No permission for this guild")));
    }
    
    // TODO: Implement actual queue clearing
    
    Ok(HttpResponse::Ok().json(ApiResponse::success("Queue cleared")))
}

// Control endpoints
#[post("/api/control/{guild_id}/play")]
async fn play_pause(path: web::Path<String>, user: AuthenticatedUser) -> ActixResult<HttpResponse> {
    let guild_id = path.into_inner();
    
    if !user_can_control_guild(&user.guilds, &guild_id) {
        return Ok(HttpResponse::Forbidden().json(ApiResponse::<()>::error("No permission for this guild")));
    }
    
    // TODO: Implement play/pause functionality
    
    Ok(HttpResponse::Ok().json(ApiResponse::success("Playback toggled")))
}

#[post("/api/control/{guild_id}/stop")]
async fn stop_playback(path: web::Path<String>, user: AuthenticatedUser) -> ActixResult<HttpResponse> {
    let guild_id = path.into_inner();
    
    if !user_can_control_guild(&user.guilds, &guild_id) {
        return Ok(HttpResponse::Forbidden().json(ApiResponse::<()>::error("No permission for this guild")));
    }
    
    // TODO: Implement stop functionality
    
    Ok(HttpResponse::Ok().json(ApiResponse::success("Playback stopped")))
}

#[put("/api/control/{guild_id}/volume")]
async fn set_volume(
    path: web::Path<String>, 
    req: web::Json<VolumeRequest>,
    user: AuthenticatedUser
) -> ActixResult<HttpResponse> {
    let guild_id = path.into_inner();
    
    if !user_can_control_guild(&user.guilds, &guild_id) {
        return Ok(HttpResponse::Forbidden().json(ApiResponse::<()>::error("No permission for this guild")));
    }
    
    if req.volume < 0.0 || req.volume > 1.0 {
        return Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error("Volume must be between 0.0 and 1.0")));
    }
    
    // TODO: Implement volume control
    
    Ok(HttpResponse::Ok().json(ApiResponse::success(format!("Volume set to {}", req.volume))))
}

// Information endpoints
#[post("/api/search")]
async fn search_songs(_req: web::Json<serde_json::Value>, _user: AuthenticatedUser) -> ActixResult<HttpResponse> {
    // TODO: Implement song search using yt-dlp
    Ok(HttpResponse::Ok().json(ApiResponse::success("Search functionality not yet implemented")))
}

#[get("/api/song/info")]
async fn get_song_info(query: web::Query<std::collections::HashMap<String, String>>, _user: AuthenticatedUser) -> ActixResult<HttpResponse> {
    if let Some(url) = query.get("url") {
        // TODO: Use yt-dlp to get song metadata
        Ok(HttpResponse::Ok().json(ApiResponse::success(format!("Song info for: {}", url))))
    } else {
        Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error("Missing url parameter")))
    }
}

// Dashboard endpoint
#[get("/")]
async fn dashboard() -> ActixResult<HttpResponse> {
    // Redirect to the static dashboard HTML file
    Ok(HttpResponse::Found()
        .append_header(("Location", "/static/dashboard.html"))
        .finish())
}
<!DOCTYPE html>
<html>
<head>
    <title>Lyre Music Bot Dashboard</title>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
        :root {
            --discord-blurple: #5865f2;
            --discord-green: #57f287;
            --discord-red: #ed4245;
            --discord-yellow: #fee75c;
            --bg-primary: #36393f;
            --bg-secondary: #2f3136;
            --bg-tertiary: #202225;
            --text-normal: #dcddde;
            --text-muted: #72767d;
            --border: #40444b;
        }
        
        * { box-sizing: border-box; }
        
        body { 
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; 
            margin: 0; 
            background: var(--bg-tertiary); 
            color: var(--text-normal);
            line-height: 1.6;
        }
        
        .header {
            background: var(--bg-primary);
            padding: 20px 0;
            border-bottom: 1px solid var(--border);
            box-shadow: 0 2px 10px rgba(0,0,0,0.2);
        }
        
        .header-content {
            max-width: 1200px;
            margin: 0 auto;
            padding: 0 20px;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }
        
        .logo {
            display: flex;
            align-items: center;
            gap: 12px;
        }
        
        .logo h1 {
            margin: 0;
            color: var(--discord-blurple);
            font-size: 24px;
            font-weight: 600;
        }
        
        .auth-section {
            display: flex;
            align-items: center;
            gap: 15px;
        }
        
        .container { 
            max-width: 1200px; 
            margin: 0 auto; 
            padding: 30px 20px;
        }
        
        .status-card {
            background: var(--bg-primary);
            padding: 20px;
            border-radius: 8px;
            border: 1px solid var(--border);
            margin-bottom: 30px;
            display: flex;
            align-items: center;
            gap: 15px;
        }
        
        .status-indicator {
            width: 12px;
            height: 12px;
            border-radius: 50%;
            background: var(--discord-green);
            animation: pulse 2s infinite;
        }
        
        @keyframes pulse {
            0% { opacity: 1; }
            50% { opacity: 0.5; }
            100% { opacity: 1; }
        }
        
        .btn {
            padding: 10px 20px;
            border: none;
            border-radius: 6px;
            cursor: pointer;
            font-weight: 500;
            text-decoration: none;
            display: inline-flex;
            align-items: center;
            gap: 8px;
            transition: all 0.2s;
            font-size: 14px;
        }
        
        .btn-discord {
            background: var(--discord-blurple);
            color: white;
        }
        
        .btn-discord:hover {
            background: #4752c4;
            transform: translateY(-1px);
        }
        
        .btn-danger {
            background: var(--discord-red);
            color: white;
        }
        
        .btn-success {
            background: var(--discord-green);
            color: var(--bg-tertiary);
        }
        
        .btn-success:hover {
            background: #4ed376;
        }
        
        .btn:disabled {
            opacity: 0.6;
            cursor: not-allowed;
        }
        
        .api-section {
            background: var(--bg-primary);
            border-radius: 8px;
            border: 1px solid var(--border);
            margin-bottom: 20px;
            overflow: hidden;
        }
        
        .api-section h3 {
            margin: 0;
            padding: 15px 20px;
            background: var(--bg-secondary);
            border-bottom: 1px solid var(--border);
            color: var(--text-normal);
            font-size: 18px;
        }
        
        .endpoint {
            padding: 15px 20px;
            border-bottom: 1px solid var(--border);
            display: flex;
            align-items: center;
            justify-content: space-between;
            transition: background 0.2s;
        }
        
        .endpoint:hover {
            background: var(--bg-secondary);
        }
        
        .endpoint:last-child {
            border-bottom: none;
        }
        
        .endpoint-info {
            display: flex;
            align-items: center;
            gap: 15px;
            flex: 1;
        }
        
        .method {
            padding: 4px 12px;
            border-radius: 4px;
            color: white;
            font-size: 12px;
            font-weight: 600;
            text-transform: uppercase;
            min-width: 60px;
            text-align: center;
        }
        
        .method.get { background: #28a745; }
        .method.post { background: #007bff; }
        .method.put { background: #ffc107; color: #000; }
        .method.delete { background: #dc3545; }
        
        .endpoint-path {
            font-family: 'Consolas', 'Monaco', monospace;
            color: var(--text-normal);
            font-size: 14px;
        }
        
        .endpoint-desc {
            color: var(--text-muted);
            font-size: 13px;
        }
        
        .user-info {
            background: var(--bg-secondary);
            padding: 15px;
            border-radius: 6px;
            margin-bottom: 20px;
            display: none;
        }
        
        .user-info.visible {
            display: block;
        }
        
        .user-avatar {
            width: 40px;
            height: 40px;
            border-radius: 50%;
            margin-right: 12px;
        }
        
        .guild-card {
            background: var(--bg-secondary);
            padding: 12px;
            border-radius: 6px;
            margin: 8px 0;
            display: flex;
            align-items: center;
            justify-content: space-between;
        }
        
        .guild-info {
            display: flex;
            align-items: center;
            gap: 10px;
        }
        
        .guild-icon {
            width: 32px;
            height: 32px;
            border-radius: 50%;
            background: var(--discord-blurple);
            display: flex;
            align-items: center;
            justify-content: center;
            color: white;
            font-weight: bold;
        }
        
        .modal {
            display: none;
            position: fixed;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            background: rgba(0,0,0,0.8);
            z-index: 1000;
            justify-content: center;
            align-items: center;
        }
        
        .modal.visible {
            display: flex;
        }
        
        .modal-content {
            background: var(--bg-primary);
            border-radius: 8px;
            padding: 30px;
            max-width: 500px;
            width: 90%;
            border: 1px solid var(--border);
        }
        
        .form-group {
            margin-bottom: 15px;
        }
        
        .form-group label {
            display: block;
            margin-bottom: 5px;
            color: var(--text-normal);
            font-weight: 500;
        }
        
        .form-group input, .form-group select, .form-group textarea {
            width: 100%;
            padding: 10px;
            border: 1px solid var(--border);
            border-radius: 4px;
            background: var(--bg-secondary);
            color: var(--text-normal);
            font-size: 14px;
        }
        
        .form-group input:focus, .form-group select:focus, .form-group textarea:focus {
            outline: none;
            border-color: var(--discord-blurple);
        }
        
        .response-section {
            margin-top: 20px;
            padding: 15px;
            background: var(--bg-secondary);
            border-radius: 6px;
            border: 1px solid var(--border);
        }
        
        .response-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 10px;
        }
        
        .status-code {
            padding: 4px 8px;
            border-radius: 4px;
            font-size: 12px;
            font-weight: bold;
        }
        
        .status-200 { background: var(--discord-green); color: var(--bg-tertiary); }
        .status-400 { background: var(--discord-yellow); color: var(--bg-tertiary); }
        .status-401 { background: var(--discord-red); color: white; }
        .status-403 { background: var(--discord-red); color: white; }
        
        .response-body {
            background: var(--bg-tertiary);
            padding: 15px;
            border-radius: 4px;
            font-family: 'Consolas', 'Monaco', monospace;
            font-size: 13px;
            overflow-x: auto;
            white-space: pre-wrap;
        }
        
        .hidden { display: none; }
        
        .loading {
            display: inline-block;
            width: 16px;
            height: 16px;
            border: 2px solid var(--border);
            border-radius: 50%;
            border-top-color: var(--discord-blurple);
            animation: spin 1s linear infinite;
        }
        
        @keyframes spin {
            to { transform: rotate(360deg); }
        }
    </style>
</head>
<body>
    <div class="header">
        <div class="header-content">
            <div class="logo">
                <span style="font-size: 24px;">🎵</span>
                <h1>Lyre Music Bot</h1>
            </div>
            <div class="auth-section">
                <div id="login-section">
                    <a href="#" id="discord-login" class="btn btn-discord">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
                            <path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515a.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0a12.64 12.64 0 0 0-.617-1.25a.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057a19.9 19.9 0 0 0 5.993 3.03a.078.078 0 0 0 .084-.028a14.09 14.09 0 0 0 1.226-1.994a.076.076 0 0 0-.041-.106a13.107 13.107 0 0 1-1.872-.892a.077.077 0 0 1-.008-.128a10.2 10.2 0 0 0 .372-.292a.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127a12.299 12.299 0 0 1-1.873.892a.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028a19.839 19.839 0 0 0 6.002-3.03a.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03z"/>
                            <path d="m8.52 15.311c-1.182 0-2.157-1.085-2.157-2.419c0-1.333.956-2.419 2.157-2.419c1.201 0 2.176 1.104 2.157 2.419c0 1.334-.956 2.419-2.157 2.419zm6.979 0c-1.183 0-2.157-1.085-2.157-2.419c0-1.333.955-2.419 2.157-2.419c1.2 0 2.176 1.104 2.157 2.419c0 1.334-.976 2.419-2.157 2.419z"/>
                        </svg>
                        Login with Discord
                    </a>
                </div>
                <div id="user-section" class="hidden">
                    <span id="user-name"></span>
                    <a href="#" id="logout" class="btn btn-danger">Logout</a>
                </div>
            </div>
        </div>
    </div>

    <div class="container">
        <div class="status-card">
            <div class="status-indicator"></div>
            <div>
                <strong>Bot Status:</strong> Online and Ready
                <div style="font-size: 14px; color: var(--text-muted); margin-top: 4px;">
                    Interactive API dashboard with Discord OAuth2 authentication
                </div>
            </div>
        </div>

        <div id="user-info" class="user-info">
            <h3>User Information</h3>
            <div id="user-details"></div>
            <h4>Your Guilds</h4>
            <div id="guild-list"></div>
        </div>

        <div class="api-section">
            <h3>Authentication</h3>
            <div class="endpoint">
                <div class="endpoint-info">
                    <span class="method post">POST</span>
                    <div>
                        <div class="endpoint-path">/api/auth/validate</div>
                        <div class="endpoint-desc">Validate Discord token and get user info</div>
                    </div>
                </div>
                <button class="btn btn-success" onclick="executeAuth()">Execute</button>
            </div>
        </div>

        <div class="api-section">
            <h3>Guild Management</h3>
            <div class="endpoint">
                <div class="endpoint-info">
                    <span class="method get">GET</span>
                    <div>
                        <div class="endpoint-path">/api/guilds</div>
                        <div class="endpoint-desc">List user's guilds with bot status</div>
                    </div>
                </div>
                <button class="btn btn-success" onclick="executeGuilds()" id="guilds-btn" disabled>Execute</button>
            </div>
        </div>

        <div class="api-section">
            <h3>Queue Management</h3>
            <div class="endpoint">
                <div class="endpoint-info">
                    <span class="method get">GET</span>
                    <div>
                        <div class="endpoint-path">/api/queue/{guild_id}</div>
                        <div class="endpoint-desc">Get current queue for guild</div>
                    </div>
                </div>
                <button class="btn btn-success" onclick="openModal('getQueue')" id="get-queue-btn" disabled>Execute</button>
            </div>
            <div class="endpoint">
                <div class="endpoint-info">
                    <span class="method post">POST</span>
                    <div>
                        <div class="endpoint-path">/api/queue/{guild_id}/add</div>
                        <div class="endpoint-desc">Add song to queue</div>
                    </div>
                </div>
                <button class="btn btn-success" onclick="openModal('addQueue')" id="add-queue-btn" disabled>Execute</button>
            </div>
            <div class="endpoint">
                <div class="endpoint-info">
                    <span class="method post">POST</span>
                    <div>
                        <div class="endpoint-path">/api/queue/{guild_id}/skip</div>
                        <div class="endpoint-desc">Skip current track</div>
                    </div>
                </div>
                <button class="btn btn-success" onclick="openModal('skipTrack')" id="skip-btn" disabled>Execute</button>
            </div>
            <div class="endpoint">
                <div class="endpoint-info">
                    <span class="method delete">DELETE</span>
                    <div>
                        <div class="endpoint-path">/api/queue/{guild_id}</div>
                        <div class="endpoint-desc">Clear entire queue</div>
                    </div>
                </div>
                <button class="btn btn-success" onclick="openModal('clearQueue')" id="clear-queue-btn" disabled>Execute</button>
            </div>
        </div>

        <div class="api-section">
            <h3>Playback Control</h3>
            <div class="endpoint">
                <div class="endpoint-info">
                    <span class="method post">POST</span>
                    <div>
                        <div class="endpoint-path">/api/control/{guild_id}/play</div>
                        <div class="endpoint-desc">Play/pause toggle</div>
                    </div>
                </div>
                <button class="btn btn-success" onclick="openModal('playPause')" id="play-btn" disabled>Execute</button>
            </div>
            <div class="endpoint">
                <div class="endpoint-info">
                    <span class="method post">POST</span>
                    <div>
                        <div class="endpoint-path">/api/control/{guild_id}/stop</div>
                        <div class="endpoint-desc">Stop and disconnect</div>
                    </div>
                </div>
                <button class="btn btn-success" onclick="openModal('stopPlayback')" id="stop-btn" disabled>Execute</button>
            </div>
            <div class="endpoint">
                <div class="endpoint-info">
                    <span class="method put">PUT</span>
                    <div>
                        <div class="endpoint-path">/api/control/{guild_id}/volume</div>
                        <div class="endpoint-desc">Set volume (0.0-1.0)</div>
                    </div>
                </div>
                <button class="btn btn-success" onclick="openModal('setVolume')" id="volume-btn" disabled>Execute</button>
            </div>
        </div>

        <div class="api-section">
            <h3>Information</h3>
            <div class="endpoint">
                <div class="endpoint-info">
                    <span class="method get">GET</span>
                    <div>
                        <div class="endpoint-path">/api/song/info?url=...</div>
                        <div class="endpoint-desc">Get song metadata</div>
                    </div>
                </div>
                <button class="btn btn-success" onclick="openModal('songInfo')" id="song-info-btn" disabled>Execute</button>
            </div>
        </div>
    </div>

    <!-- Modal for API calls -->
    <div id="api-modal" class="modal">
        <div class="modal-content">
            <h3 id="modal-title">Execute API Call</h3>
            <div id="modal-form"></div>
            <div style="display: flex; gap: 10px; margin-top: 20px;">
                <button class="btn btn-success" onclick="executeModalCall()" id="execute-btn">
                    <span id="execute-text">Execute</span>
                    <span id="execute-loading" class="loading hidden"></span>
                </button>
                <button class="btn" onclick="closeModal()" style="background: var(--border); color: var(--text-normal);">Cancel</button>
            </div>
            <div id="response-section" class="response-section hidden">
                <div class="response-header">
                    <strong>Response</strong>
                    <span id="status-code" class="status-code"></span>
                </div>
                <div id="response-body" class="response-body"></div>
            </div>
        </div>
    </div>

    <script>
        let currentUser = null;
        let currentToken = null;
        let currentModalType = null;

        // Discord OAuth2 Configuration
        const DISCORD_CLIENT_ID = '191805365212413953'; // Replace with your actual client ID
        const DISCORD_REDIRECT_URI = encodeURIComponent(window.location.origin + '/auth/callback');
        const DISCORD_SCOPES = 'identify guilds';

        document.getElementById('discord-login').addEventListener('click', (e) => {
            e.preventDefault();
            loginWithDiscord();
        });

        document.getElementById('logout').addEventListener('click', (e) => {
            e.preventDefault();
            logout();
        });

        function loginWithDiscord() {
            // For demo purposes, we'll simulate a login
            // In production, this would redirect to Discord OAuth2
            const demoToken = 'demo_' + Math.random().toString(36).substr(2, 9);
            localStorage.setItem('discord_token', demoToken);
            currentToken = demoToken;
            
            // Simulate user data
            currentUser = {
                id: '123456789',
                username: 'DemoUser',
                discriminator: '0000',
                avatar: null,
                global_name: 'Demo User'
            };
            
            updateAuthUI();
            validateToken();
        }

        function logout() {
            localStorage.removeItem('discord_token');
            currentToken = null;
            currentUser = null;
            updateAuthUI();
        }

        function updateAuthUI() {
            const loginSection = document.getElementById('login-section');
            const userSection = document.getElementById('user-section');
            const userInfo = document.getElementById('user-info');
            
            if (currentUser) {
                loginSection.classList.add('hidden');
                userSection.classList.remove('hidden');
                userInfo.classList.add('visible');
                document.getElementById('user-name').textContent = currentUser.global_name || currentUser.username;
                enableButtons();
            } else {
                loginSection.classList.remove('hidden');
                userSection.classList.add('hidden');
                userInfo.classList.remove('visible');
                disableButtons();
            }
        }

        function enableButtons() {
            const buttons = ['guilds-btn', 'get-queue-btn', 'add-queue-btn', 'skip-btn', 'clear-queue-btn', 
                           'play-btn', 'stop-btn', 'volume-btn', 'song-info-btn'];
            buttons.forEach(id => {
                const btn = document.getElementById(id);
                if (btn) btn.disabled = false;
            });
        }

        function disableButtons() {
            const buttons = ['guilds-btn', 'get-queue-btn', 'add-queue-btn', 'skip-btn', 'clear-queue-btn', 
                           'play-btn', 'stop-btn', 'volume-btn', 'song-info-btn'];
            buttons.forEach(id => {
                const btn = document.getElementById(id);
                if (btn) btn.disabled = true;
            });
        }

        async function apiCall(method, endpoint, body = null) {
            const headers = {
                'Content-Type': 'application/json'
            };
            
            if (currentToken) {
                headers['Authorization'] = `Bearer ${currentToken}`;
            }
            
            const config = {
                method,
                headers
            };
            
            if (body) {
                config.body = JSON.stringify(body);
            }
            
            const response = await fetch(endpoint, config);
            const data = await response.json();
            
            return { status: response.status, data };
        }

        async function executeAuth() {
            if (!currentToken) {
                alert('Please login first');
                return;
            }
            
            try {
                const result = await apiCall('POST', '/api/auth/validate', { access_token: currentToken });
                showResponse(result.status, result.data);
            } catch (error) {
                showResponse(500, { error: error.message });
            }
        }

        async function executeGuilds() {
            try {
                const result = await apiCall('GET', '/api/guilds');
                showResponse(result.status, result.data);
                
                if (result.status === 200 && result.data.success) {
                    displayUserGuilds(result.data.data);
                }
            } catch (error) {
                showResponse(500, { error: error.message });
            }
        }

        async function validateToken() {
            if (!currentToken) return;
            
            try {
                const result = await apiCall('POST', '/api/auth/validate', { access_token: currentToken });
                if (result.status === 200 && result.data.success) {
                    const userData = result.data.data;
                    displayUserInfo(userData.user);
                    displayUserGuilds(userData.guilds);
                }
            } catch (error) {
                console.error('Token validation failed:', error);
            }
        }

        function displayUserInfo(user) {
            const userDetails = document.getElementById('user-details');
            userDetails.innerHTML = `
                <div style="display: flex; align-items: center; gap: 12px;">
                    <div class="guild-icon">${user.username[0].toUpperCase()}</div>
                    <div>
                        <strong>${user.global_name || user.username}</strong>
                        <div style="color: var(--text-muted); font-size: 13px;">ID: ${user.id}</div>
                    </div>
                </div>
            `;
        }

        function displayUserGuilds(guilds) {
            const guildList = document.getElementById('guild-list');
            guildList.innerHTML = guilds.map(guild => `
                <div class="guild-card">
                    <div class="guild-info">
                        <div class="guild-icon">${guild.name[0].toUpperCase()}</div>
                        <div>
                            <strong>${guild.name}</strong>
                            <div style="color: var(--text-muted); font-size: 12px;">
                                ${guild.owner ? 'Owner' : 'Member'} • ID: ${guild.id}
                            </div>
                        </div>
                    </div>
                    <div style="color: var(--text-muted); font-size: 12px;">
                        Bot: Not Connected
                    </div>
                </div>
            `).join('');
        }

        function openModal(type) {
            currentModalType = type;
            const modal = document.getElementById('api-modal');
            const title = document.getElementById('modal-title');
            const form = document.getElementById('modal-form');
            
            const configs = {
                getQueue: {
                    title: 'Get Queue',
                    form: '<div class="form-group"><label>Guild ID:</label><input type="text" id="guild-id" placeholder="Enter guild ID" required></div>'
                },
                addQueue: {
                    title: 'Add to Queue',
                    form: `
                        <div class="form-group"><label>Guild ID:</label><input type="text" id="guild-id" placeholder="Enter guild ID" required></div>
                        <div class="form-group"><label>Song URL:</label><input type="url" id="song-url" placeholder="https://www.youtube.com/watch?v=..." required></div>
                        <div class="form-group"><label>Voice Channel ID (optional):</label><input type="text" id="channel-id" placeholder="Voice channel ID"></div>
                    `
                },
                skipTrack: {
                    title: 'Skip Track',
                    form: '<div class="form-group"><label>Guild ID:</label><input type="text" id="guild-id" placeholder="Enter guild ID" required></div>'
                },
                clearQueue: {
                    title: 'Clear Queue',
                    form: '<div class="form-group"><label>Guild ID:</label><input type="text" id="guild-id" placeholder="Enter guild ID" required></div>'
                },
                playPause: {
                    title: 'Play/Pause',
                    form: '<div class="form-group"><label>Guild ID:</label><input type="text" id="guild-id" placeholder="Enter guild ID" required></div>'
                },
                stopPlayback: {
                    title: 'Stop Playback',
                    form: '<div class="form-group"><label>Guild ID:</label><input type="text" id="guild-id" placeholder="Enter guild ID" required></div>'
                },
                setVolume: {
                    title: 'Set Volume',
                    form: `
                        <div class="form-group"><label>Guild ID:</label><input type="text" id="guild-id" placeholder="Enter guild ID" required></div>
                        <div class="form-group"><label>Volume (0.0 - 1.0):</label><input type="number" id="volume" min="0" max="1" step="0.1" value="0.5" required></div>
                    `
                },
                songInfo: {
                    title: 'Get Song Info',
                    form: '<div class="form-group"><label>Song URL:</label><input type="url" id="song-url" placeholder="https://www.youtube.com/watch?v=..." required></div>'
                }
            };
            
            const config = configs[type];
            title.textContent = config.title;
            form.innerHTML = config.form;
            
            // Hide response section
            document.getElementById('response-section').classList.add('hidden');
            
            modal.classList.add('visible');
        }

        function closeModal() {
            document.getElementById('api-modal').classList.remove('visible');
            currentModalType = null;
        }

        async function executeModalCall() {
            if (!currentModalType) return;
            
            const executeBtn = document.getElementById('execute-btn');
            const executeText = document.getElementById('execute-text');
            const executeLoading = document.getElementById('execute-loading');
            
            executeBtn.disabled = true;
            executeText.classList.add('hidden');
            executeLoading.classList.remove('hidden');
            
            try {
                let result;
                const guildId = document.getElementById('guild-id')?.value;
                
                switch (currentModalType) {
                    case 'getQueue':
                        result = await apiCall('GET', `/api/queue/${guildId}`);
                        break;
                    case 'addQueue':
                        const url = document.getElementById('song-url').value;
                        const channelId = document.getElementById('channel-id').value;
                        result = await apiCall('POST', `/api/queue/${guildId}/add`, { 
                            url, 
                            channel_id: channelId || null 
                        });
                        break;
                    case 'skipTrack':
                        result = await apiCall('POST', `/api/queue/${guildId}/skip`);
                        break;
                    case 'clearQueue':
                        result = await apiCall('DELETE', `/api/queue/${guildId}`);
                        break;
                    case 'playPause':
                        result = await apiCall('POST', `/api/control/${guildId}/play`);
                        break;
                    case 'stopPlayback':
                        result = await apiCall('POST', `/api/control/${guildId}/stop`);
                        break;
                    case 'setVolume':
                        const volume = parseFloat(document.getElementById('volume').value);
                        result = await apiCall('PUT', `/api/control/${guildId}/volume`, { volume });
                        break;
                    case 'songInfo':
                        const songUrl = document.getElementById('song-url').value;
                        result = await apiCall('GET', `/api/song/info?url=${encodeURIComponent(songUrl)}`);
                        break;
                }
                
                showModalResponse(result.status, result.data);
            } catch (error) {
                showModalResponse(500, { error: error.message });
            } finally {
                executeBtn.disabled = false;
                executeText.classList.remove('hidden');
                executeLoading.classList.add('hidden');
            }
        }

        function showModalResponse(status, data) {
            const responseSection = document.getElementById('response-section');
            const statusCode = document.getElementById('status-code');
            const responseBody = document.getElementById('response-body');
            
            statusCode.textContent = status;
            statusCode.className = `status-code status-${Math.floor(status / 100) * 100}`;
            responseBody.textContent = JSON.stringify(data, null, 2);
            
            responseSection.classList.remove('hidden');
        }

        function showResponse(status, data) {
            // Create a temporary modal-like display for responses outside modals
            alert(`Status: ${status}\n\n${JSON.stringify(data, null, 2)}`);
        }

        // Initialize
        window.addEventListener('load', () => {
            const savedToken = localStorage.getItem('discord_token');
            if (savedToken) {
                currentToken = savedToken;
                // Simulate user for demo
                if (savedToken.startsWith('demo_')) {
                    currentUser = {
                        id: '123456789',
                        username: 'DemoUser',
                        discriminator: '0000',
                        avatar: null,
                        global_name: 'Demo User'
                    };
                    updateAuthUI();
                    validateToken();
                }
            }
        });

        // Close modal when clicking outside
        document.getElementById('api-modal').addEventListener('click', (e) => {
            if (e.target.id === 'api-modal') {
                closeModal();
            }
        });
    </script>
</body>
</html>
    "#;
    
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

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
