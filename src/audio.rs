use std::{path::PathBuf, process::Stdio};

use anyhow::{Context as AnyhowContext, Result, anyhow};
use once_cell::sync::Lazy;
use reqwest::header::{ACCEPT, USER_AGENT};
use serde::Deserialize;
use tokio::{
    fs,
    io::{AsyncBufReadExt, BufReader},
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
        return Err(anyhow!(
            "yt-dlp --print id failed with status: {}",
            out.status
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
        return Err(anyhow!(
            "yt-dlp --print title failed with status: {}",
            out.status
        ));
    }
    let title = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if title.is_empty() {
        return Err(anyhow!("empty title from yt-dlp"));
    }
    Ok(title)
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
    pub percent: u8,
}

pub fn spawn_download_mp3(
    url: String,
) -> (
    mpsc::UnboundedReceiver<DownloadProgress>,
    JoinHandle<Result<PathBuf>>,
) {
    let (tx, rx) = mpsc::unbounded_channel();
    let handle = tokio::spawn(async move {
        let ytdlp = ensure_yt_dlp().await?;
        let base = download_base_dir()?;
        fs::create_dir_all(&base).await?;
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
            let _ = tx.send(DownloadProgress { percent: 100 });
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

        let mut cmd = TokioCommand::new(&ytdlp);
        cmd.arg("-f")
            .arg("bestaudio/best")
            .arg("-x")
            .arg("--audio-format")
            .arg("mp3")
            .arg("--audio-quality")
            .arg("0") // Best quality
            .arg("--postprocessor-args")
            .arg("ffmpeg:-ar 48000 -ac 2") // Force 48kHz stereo (Discord's preferred format)
            .arg("--no-playlist")
            .arg("--newline")
            .arg("-o")
            .arg(dir.join("%(id)s.%(ext)s").to_string_lossy().to_string())
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().context("spawning yt-dlp")?;

        if let Some(stderr) = child.stderr.take() {
            let mut reader = BufReader::new(stderr).lines();
            let mut last_sent = 255u8; // impossible value to force first update
            while let Some(Ok(line)) = reader.next_line().await.transpose() {
                if let Some(pct) = parse_percent(&line)
                    && pct != last_sent
                {
                    let _ = tx.send(DownloadProgress { percent: pct });
                    last_sent = pct;
                }
            }
        }

        let status = child.wait().await.context("waiting for yt-dlp")?;
        if !status.success() {
            return Err(anyhow!("yt-dlp failed with status: {status}"));
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
