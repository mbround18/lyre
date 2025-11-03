use std::{path::PathBuf, process::Stdio};

use anyhow::{Context as AnyhowContext, Result, anyhow};
use once_cell::sync::Lazy;
use reqwest::header::{ACCEPT, USER_AGENT};
use serde::Deserialize;
use tokio::{
    fs,
    io::{AsyncBufReadExt, BufReader, copy as tokio_copy},
    process::Command as TokioCommand,
    sync::mpsc,
    task::JoinHandle,
};

static HTTP: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .user_agent("lyre-bot/0.1 (+https://github.com/)")
        .build()
        .expect("client")
});

const GITHUB_RELEASES_API: &str = "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest";

#[derive(Debug, Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct ReleaseInfo {
    assets: Vec<ReleaseAsset>,
    #[allow(dead_code)]
    tag_name: String,
}

fn cache_dir() -> Result<PathBuf> {
    let base = dirs::cache_dir().ok_or_else(|| anyhow!("no cache dir available on this system"))?;
    Ok(base.join("lyre").join("yt-dlp"))
}

fn platform_asset_name() -> &'static str {
    if cfg!(target_os = "windows") {
        if cfg!(target_arch = "x86_64") {
            "yt-dlp.exe"
        } else {
            "yt-dlp_x86.exe"
        }
    } else if cfg!(target_os = "linux") {
        "yt-dlp_linux"
    } else if cfg!(target_os = "macos") {
        "yt-dlp_macos"
    } else {
        "yt-dlp"
    }
}

async fn ensure_yt_dlp() -> Result<PathBuf> {
    if let Ok(p) = which::which("yt-dlp") {
        return Ok(p);
    }

    let dir = cache_dir()?;
    fs::create_dir_all(&dir).await.ok();

    let local = dir.join(if cfg!(target_os = "windows") {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    });
    if fs::try_exists(&local).await.unwrap_or(false) {
        return Ok(local);
    }

    let resp = HTTP
        .get(GITHUB_RELEASES_API)
        .header(ACCEPT, "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?;
    let rel: ReleaseInfo = resp.json().await?;

    let wanted = platform_asset_name();
    let asset = rel
        .assets
        .into_iter()
        .find(|a| a.name == wanted)
        .ok_or_else(|| anyhow!("no suitable yt-dlp asset for this platform: {}", wanted))?;

    let bytes = HTTP
        .get(asset.browser_download_url)
        .header(USER_AGENT, "lyre-bot/0.1")
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    fs::write(&local, &bytes).await?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&local).await?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&local, perms).await?;
    }
    Ok(local)
}

async fn ytdlp_extract_id(ytdlp: &PathBuf, url: &str) -> Result<String> {
    let out = TokioCommand::new(ytdlp)
        .arg("--print")
        .arg("id")
        .arg("--skip-download")
        .arg("-q")
        .arg(url)
        .stdin(Stdio::null())
        .output()
        .await
        .context("running yt-dlp to extract id")?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(anyhow!(
            "yt-dlp --print id failed with status: {}. Error: {}",
            out.status,
            stderr.trim()
        ));
    }
    let id = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if id.is_empty() {
        return Err(anyhow!("empty id from yt-dlp"));
    }
    Ok(id)
}

pub async fn ytdlp_extract_title(url: &str) -> Result<String> {
    let ytdlp = ensure_yt_dlp().await?;
    let out = TokioCommand::new(&ytdlp)
        .arg("--print")
        .arg("title")
        .arg("--skip-download")
        .arg("-q")
        .arg(url)
        .stdin(Stdio::null())
        .output()
        .await
        .context("running yt-dlp to extract title")?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(anyhow!(
            "yt-dlp --print title failed with status: {}. Error: {}",
            out.status,
            stderr.trim()
        ));
    }
    let title = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if title.is_empty() {
        return Err(anyhow!("empty title from yt-dlp"));
    }
    Ok(title)
}

async fn ytdlp_extract_duration(ytdlp: &PathBuf, url: &str) -> Result<Option<f64>> {
    let out = TokioCommand::new(ytdlp)
        .arg("--print")
        .arg("duration")
        .arg("--skip-download")
        .arg("-q")
        .arg(url)
        .stdin(Stdio::null())
        .output()
        .await
        .context("running yt-dlp to extract duration")?;

    if !out.status.success() {
        return Ok(None); // Duration might not be available for all videos
    }

    let duration_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if duration_str.is_empty() || duration_str == "NA" {
        return Ok(None);
    }

    Ok(duration_str.parse::<f64>().ok())
}

fn get_ffmpeg_threads() -> String {
    // Check env var first
    if let Ok(threads) = std::env::var("FFMPEG_THREADS") {
        return threads;
    }

    // Auto-detect based on CPU count
    let cpu_count = num_cpus::get();
    // Use 75% of available CPUs for ffmpeg, minimum 2, maximum 8
    let optimal = ((cpu_count as f32 * 0.75).ceil() as usize).clamp(2, 8);
    optimal.to_string()
}

fn download_base_dir() -> Result<PathBuf> {
    if let Ok(dir) = std::env::var("DOWNLOAD_FOLDER") {
        let p = PathBuf::from(dir);
        if p.is_absolute() {
            Ok(p)
        } else {
            Ok(std::env::current_dir()?.join(p))
        }
    } else {
        Ok(cache_dir()?.join("downloads"))
    }
}

// Public helper so other modules (e.g., main) can log where downloads are cached.
pub fn resolved_download_base_dir() -> Result<PathBuf> {
    download_base_dir()
}

// removed blocking download_mp3 in favor of spawn_download_mp3 used by /play

#[derive(Clone, Debug)]
pub struct DownloadProgress {
    /// One of: queued, downloading, converting, done, failed
    pub status: String,
    /// Optional percent for downloading stage
    pub percent: Option<u8>,
}

pub fn spawn_download_mp3(
    url: String,
) -> (
    mpsc::UnboundedReceiver<DownloadProgress>,
    JoinHandle<Result<PathBuf>>,
) {
    let (tx, rx) = mpsc::unbounded_channel();
    let handle = tokio::spawn(async move {
        // initial queued status
        let _ = tx.send(DownloadProgress {
            status: "queued".into(),
            percent: None,
        });
        let ytdlp = ensure_yt_dlp().await?;
        let base = download_base_dir()?;
        fs::create_dir_all(&base).await?;

        // Check duration before downloading - reject if > 1h10m (4200 seconds)
        const MAX_DURATION_SECS: f64 = 4200.0; // 70 minutes
        if let Ok(Some(duration)) = ytdlp_extract_duration(&ytdlp, &url).await {
            if duration > MAX_DURATION_SECS {
                let duration_mins = (duration / 60.0).round() as u32;
                return Err(anyhow!(
                    "Video is too long ({} minutes). Maximum allowed duration is 70 minutes (1h10m).",
                    duration_mins
                ));
            }
            tracing::info!(
                "Video duration: {:.1} seconds ({:.1} minutes)",
                duration,
                duration / 60.0
            );
        } else {
            tracing::warn!("Could not determine video duration, proceeding anyway");
        }

        // Resolve a stable video ID for caching; fall back to a timestamp if it fails.
        let vid = match ytdlp_extract_id(&ytdlp, &url).await {
            Ok(v) => v,
            Err(_) => format!(
                "ts-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            ),
        };
        let cached = base.join(format!("{}.mp3", vid));
        if fs::try_exists(&cached).await.unwrap_or(false) {
            let _ = tx.send(DownloadProgress {
                status: "done".into(),
                percent: Some(100),
            });
            return Ok(cached);
        }
        // Create a unique subdirectory for this download to avoid cross-task collisions.
        let unique = {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            format!("job-{}", now)
        };
        let dir = base.join(unique);
        fs::create_dir_all(&dir).await?;

        // We'll stream yt-dlp stdout into ffmpeg stdin to start conversion immediately
        let mut ytdlp_cmd = TokioCommand::new(&ytdlp);
        ytdlp_cmd
            .arg("-f")
            .arg("bestaudio/best")
            .arg("--external-downloader")
            .arg("aria2c")
            .arg("--external-downloader-args")
            .arg("aria2c:-x 16 -s 16 -k 1M") // 16 connections, 16 splits, 1MB chunk size
            .arg("--no-playlist");

        // Add cookies if COOKIES_FILE is set
        if let Ok(cookies_path) = std::env::var("COOKIES_FILE") {
            if std::path::Path::new(&cookies_path).exists() {
                ytdlp_cmd.arg("--cookies").arg(cookies_path);
                tracing::info!("Using cookies file for authentication");
            }
        }

        ytdlp_cmd
            .arg("--newline")
            .arg("-o")
            .arg("-") // stream to stdout
            .arg(&url)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        tracing::info!("Starting yt-dlp download for: {}", url);
        let mut ytdlp_child = ytdlp_cmd.spawn().context("spawning yt-dlp")?;
        tracing::info!("yt-dlp process spawned successfully");

        // Start ffmpeg to read from stdin (pipe:0) and write mp3 to final cached path
        let ffmpeg_threads = get_ffmpeg_threads();
        tracing::info!("Using {} ffmpeg threads", ffmpeg_threads);

        let mut ffmpeg_cmd = TokioCommand::new("ffmpeg");
        // -y overwrite, -hide_banner suppress, -loglevel info for progress on stderr
        ffmpeg_cmd
            .arg("-y")
            .arg("-hide_banner")
            .arg("-loglevel")
            .arg("info")
            .arg("-threads")
            .arg(&ffmpeg_threads)
            .arg("-i")
            .arg("pipe:0")
            .arg("-ar")
            .arg("48000")
            .arg("-ac")
            .arg("2")
            .arg("-f")
            .arg("mp3")
            .arg(cached.to_string_lossy().to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        tracing::info!("Starting ffmpeg process for conversion");
        let mut ffmpeg_child = ffmpeg_cmd.spawn().context("spawning ffmpeg")?;
        tracing::info!("ffmpeg process spawned successfully");

        // Pipe data from yt-dlp stdout -> ffmpeg stdin
        if let (Some(mut ytdlp_out), Some(mut ffmpeg_in)) =
            (ytdlp_child.stdout.take(), ffmpeg_child.stdin.take())
        {
            // spawn a copy task; we don't await here because we want to also parse progress concurrently
            tracing::info!("Setting up streaming pipe from yt-dlp to ffmpeg");
            let copy_handle = tokio::spawn(async move {
                match tokio_copy(&mut ytdlp_out, &mut ffmpeg_in).await {
                    Ok(bytes) => {
                        tracing::info!("Streamed {} bytes from yt-dlp to ffmpeg", bytes);
                        Ok(bytes)
                    }
                    Err(e) => {
                        tracing::error!("Error streaming data: {}", e);
                        Err(e)
                    }
                }
            });

            // parse yt-dlp stderr for download progress
            let mut reader = BufReader::new(ytdlp_child.stderr.take().unwrap()).lines();
            let mut last_sent = 255u8;
            let mut error_lines = Vec::new();
            let mut downloading_started = false;
            
            while let Some(Ok(line)) = reader.next_line().await.transpose() {
                // Log all yt-dlp output for debugging
                tracing::debug!("yt-dlp: {}", line);
                
                if let Some(pct) = parse_percent(&line) {
                    if !downloading_started {
                        tracing::info!("Download started");
                        downloading_started = true;
                    }
                    if pct != last_sent {
                        tracing::info!("Download progress: {}%", pct);
                        let _ = tx.send(DownloadProgress {
                            status: "downloading".into(),
                            percent: Some(pct),
                        });
                        last_sent = pct;
                    }
                } else if line.contains("ERROR") || line.contains("error") {
                    tracing::warn!("yt-dlp error: {}", line);
                    error_lines.push(line);
                } else if line.contains("[download]") {
                    // Log download-related lines even if we can't parse percentage
                    tracing::info!("yt-dlp download: {}", line);
                }
            }

            // Wait for yt-dlp to finish
            tracing::info!("Waiting for yt-dlp to finish downloading...");
            let ytdlp_status = ytdlp_child.wait().await.context("waiting for yt-dlp")?;
            if !ytdlp_status.success() {
                let error_msg = if error_lines.is_empty() {
                    format!("yt-dlp failed with status: {ytdlp_status}")
                } else {
                    format!(
                        "yt-dlp failed with status: {ytdlp_status}. Errors: {}",
                        error_lines.join("; ")
                    )
                };
                return Err(anyhow!(error_msg));
            }
            tracing::info!("yt-dlp download completed successfully");

            // Ensure copy completes (ffmpeg will continue until stdin closed)
            tracing::info!("Waiting for data stream to ffmpeg...");
            let _ = copy_handle.await;
            tracing::info!("Data stream to ffmpeg completed");

            // Now ffmpeg is converting the streamed input; notify converting
            tracing::info!("Starting conversion...");
            let _ = tx.send(DownloadProgress {
                status: "converting".into(),
                percent: None,
            });

            // Optionally parse ffmpeg stderr for progress here (left as future improvement)
            let ff_err = ffmpeg_child.stderr.take();
            if let Some(fe) = ff_err {
                // Drain ffmpeg stderr but do not block on parsing for now
                let mut _buf = BufReader::new(fe).lines();
                // spawn a task to drain
                tokio::spawn(async move {
                    while let Some(Ok(_)) = _buf.next_line().await.transpose() {
                        // ignore for now
                    }
                });
            }

            tracing::info!("Waiting for ffmpeg to finish conversion...");
            let ff_status = ffmpeg_child.wait().await.context("waiting for ffmpeg")?;
            if !ff_status.success() {
                return Err(anyhow!(format!("ffmpeg failed with status: {ff_status}")));
            }
            tracing::info!("ffmpeg conversion completed successfully");

            // conversion finished
            let _ = tx.send(DownloadProgress {
                status: "done".into(),
                percent: Some(100),
            });
        } else {
            return Err(anyhow!("failed to setup streaming pipes"));
        }

        // Find produced mp3 in the unique dir
        let mut entries = fs::read_dir(&dir).await?;
        let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
        while let Some(e) = entries.next_entry().await? {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("mp3") {
                let meta = e.metadata().await?;
                let mtime = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                if newest.as_ref().map(|(_, t)| mtime > *t).unwrap_or(true) {
                    newest = Some((p, mtime));
                }
            }
        }
        let (p, _) = newest.ok_or_else(|| anyhow!("no mp3 produced"))?;
        // Move/copy into cache location, handling races and cross-device moves.
        let final_path = if fs::try_exists(&cached).await.unwrap_or(false)
            || fs::rename(&p, &cached).await.is_ok()
        {
            cached.clone()
        } else if fs::copy(&p, &cached).await.is_ok() {
            let _ = fs::remove_file(&p).await;
            cached.clone()
        } else {
            p.clone()
        };
        let _ = fs::remove_dir_all(&dir).await;
        Ok(final_path)
    });

    (rx, handle)
}

/// Extract playlist entries with titles and URLs
pub async fn ytdlp_extract_playlist(url: &str) -> Result<Vec<(String, String, Option<f64>)>> {
    let ytdlp = ensure_yt_dlp().await?;

    // Get playlist entries (URL and title)
    let out = TokioCommand::new(&ytdlp)
        .arg("--flat-playlist")
        .arg("--print")
        .arg("%(url)s|||%(title)s|||%(duration)s")
        .arg("--skip-download")
        .arg("-q")
        .arg(url)
        .stdin(Stdio::null())
        .output()
        .await
        .context("running yt-dlp to extract playlist")?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(anyhow!(
            "yt-dlp playlist extraction failed: {}",
            stderr.trim()
        ));
    }

    let output = String::from_utf8_lossy(&out.stdout);
    let mut entries = Vec::new();

    for line in output.lines() {
        let parts: Vec<&str> = line.split("|||").collect();
        if parts.len() >= 3 {
            let video_url = parts[0].trim().to_string();
            let title = parts[1].trim().to_string();
            let duration = parts[2].trim().parse::<f64>().ok();

            if !video_url.is_empty() && !title.is_empty() {
                entries.push((video_url, title, duration));
            }
        }
    }

    if entries.is_empty() {
        return Err(anyhow!("No videos found in playlist"));
    }

    Ok(entries)
}

fn parse_percent(line: &str) -> Option<u8> {
    // Try to find a pattern like "[download]   42.3%" and parse percent
    if let Some(idx) = line.find('%') {
        let start = line[..idx].rfind(|c: char| !(c.is_ascii_digit() || c == '.'))? + 1;
        let num = &line[start..idx];
        if let Ok(val) = num.parse::<f32>() {
            let pct = val.round().clamp(0.0, 100.0) as u8;
            return Some(pct);
        }
    }
    None
}
