# Lyre Discord Bot

A minimal Discord music bot using Serenity + Songbird with yt-dlp. It provides slash commands to play audio from links, with caching and a small set of tunables to reduce hiccups.

## Prerequisites

- Rust toolchain (stable)
- A Discord Bot token with the bot invited into your server
- On first run, the bot downloads the latest platform-specific `yt-dlp` from GitHub releases automatically

## Setup

1. Create a bot in the Discord Developer Portal and copy the token.
2. Create a `.env` file:

```dotenv
# Required
DISCORD_TOKEN=your-bot-token-here

# Optional (tuning / behavior)
# Base folder for downloaded/cached MP3s. Relative paths resolve from the current working directory.
# Default: $XDG_CACHE_HOME/lyre/yt-dlp/downloads
# DOWNLOAD_FOLDER=tmp

# Mixing mode: mono reduces bandwidth/CPU, can help with stutter. Default: stereo
# LYRE_MIX_MODE=mono

# Encoder bitrate in bits/sec (16000..192000). Defaults to 96000
# LYRE_BITRATE=64000

# Start tracks muted for N milliseconds, then raise to 0.5 volume (masks initial jitters)
# LYRE_PREROLL_MS=100

# Optional: Path to cookies.txt for authenticated downloads (your own content, private videos)
# COOKIES_FILE=/path/to/cookies.txt

# Optional: Override auto-detected ffmpeg thread count (auto-detects 75% of CPU cores, 2-8 range)
# FFMPEG_THREADS=4
```

3. Build and run:

```bash
# build
cargo build

# run (prefer release for smoother audio)
cargo run --release
```

Notes:

- Global slash commands can take up to an hour to propagate. For faster iteration, you can manually register per-guild using Serenity APIs if desired.
- The bot requires the `GUILD_VOICE_STATES` intent.

## Usage

In any server where the bot is present:

- Join a voice channel
- Run `/play url:<link>` in a text channel to play a single video (playlists are automatically skipped)
- Run `/playlist url:<playlist-link>` to queue an entire playlist (videos over 70 minutes are skipped)
- Use `/next` to skip the current track
- Use `/stop` to stop, clear the queue, and disconnect

### Enhanced Features

- **Rich Embeds**: When playing songs, the bot displays rich embeds with clickable links to the original source
- **Queue Management**: Songs show their position in queue when multiple tracks are queued
- **Auto-disconnect**: The bot automatically disconnects when the queue is empty after a song finishes
- **Next Song Announcements**: When skipping tracks, embeds show the queue status
- **Graceful Shutdown**: The bot responds properly to Ctrl+C (SIGINT) and SIGTERM signals

The bot will join your voice channel, download or reuse a cached MP3 by video ID, and start playback with rich Discord embeds showing song information.

## Authenticated Downloads (Cookies)

To download your own private content or videos requiring authentication:

1. Export cookies from your browser using an extension like "Get cookies.txt LOCALLY" or similar
2. Save the cookies as `cookies.txt`
3. Mount the file in Docker or set `COOKIES_FILE` environment variable:

**Docker Compose:**

```yaml
volumes:
  - ./cookies.txt:/data/cookies/cookies.txt:ro
environment:
  - COOKIES_FILE=/data/cookies/cookies.txt
```

**Local development:**

```bash
export COOKIES_FILE=/path/to/cookies.txt
cargo run --release
```

The bot will automatically use cookies when set, allowing downloads of authenticated content.

## Troubleshooting

- If playback fails, ensure the URL is supported by yt-dlp.
- If `yt-dlp` fails to download, check your network/proxy and GitHub availability.
- To see where files are cached, look for the "Download cache dir" log line at startup.
- For fewer hiccups on constrained hosts, try:
  - `LYRE_MIX_MODE=mono`
  - `LYRE_BITRATE=64000`
  - `LYRE_PREROLL_MS=5000`
- On Linux/macOS, the downloaded binary is placed in your user cache directory and marked executable.
