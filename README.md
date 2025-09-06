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
- Run `/play url:<link>` in a text channel
- Use `/next` to skip the current track
- Use `/stop` to stop, clear the queue, and disconnect

### Enhanced Features

- **Rich Embeds**: When playing songs, the bot displays rich embeds with clickable links to the original source
- **Queue Management**: Songs show their position in queue when multiple tracks are queued
- **Auto-disconnect**: The bot automatically disconnects when the queue is empty after a song finishes
- **Next Song Announcements**: When skipping tracks, embeds show the queue status
- **Graceful Shutdown**: The bot responds properly to Ctrl+C (SIGINT) and SIGTERM signals

The bot will join your voice channel, download or reuse a cached MP3 by video ID, and start playback with rich Discord embeds showing song information.

## Troubleshooting

- If playback fails, ensure the URL is supported by yt-dlp.
- If `yt-dlp` fails to download, check your network/proxy and GitHub availability.
- To see where files are cached, look for the "Download cache dir" log line at startup.
- For fewer hiccups on constrained hosts, try:
  - `LYRE_MIX_MODE=mono`
  - `LYRE_BITRATE=64000`
  - `LYRE_PREROLL_MS=5000`
- On Linux/macOS, the downloaded binary is placed in your user cache directory and marked executable.
