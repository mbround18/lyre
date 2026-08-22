# Graph Report - lyre  (2026-08-21)

## Corpus Check
- Corpus is ~19,591 words - fits in a single context window. You may not need a graph.

## Summary
- 680 nodes · 1010 edges · 44 communities (42 shown, 2 thin omitted)
- Extraction: 95% EXTRACTED · 5% INFERRED · 0% AMBIGUOUS · INFERRED: 46 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- API Auth & Control Routes
- DB Pool & Analytics API
- Frontend Lint Config
- Queue Advance & Autoplay
- Docker Compose Stack
- Frontend Package Deps
- Audio Download & Cache
- Frontend TS App Config
- Frontend UI Dependencies
- Legacy Static Dashboard
- Frontend Vite Entry
- Voice Connections Model
- Frontend TS Node Config
- Auth Middleware
- DB Listener & Migrations
- Play Command Handler
- Frontend Component Aliases
- Voice Manager App State
- Bot Bridge Commands
- OAuth Flow
- Current Queue Model
- Discord Event Handling
- Guild Settings Model
- Queue History Model
- Song Cache Model
- Frontend API Client
- Rust CI Workflow
- HTTP Server Entrypoint
- Frontend Discord OAuth
- Scratch File
- Dashboard Redirect Route
- Dev Auth Test Route
- Frontend TS Root Config
- Lyre Package Root

## God Nodes (most connected - your core abstractions)
1. `establish_connection()` - 22 edges
2. `AuthenticatedUser` - 21 edges
3. `compilerOptions` - 19 edges
4. `compilerOptions` - 15 edges
5. `VoiceConnection` - 15 edges
6. `Lyre Music Bot Dashboard (static)` - 15 edges
7. `Metrics` - 14 edges
8. `AppState` - 14 edges
9. `spawn_download_mp3()` - 13 edges
10. `get_authenticated_user_from_extensions()` - 13 edges

## Surprising Connections (you probably didn't know these)
- `Lyre Music Bot Dashboard (static)` --semantically_similar_to--> `Lyre Dashboard index.html`  [INFERRED] [semantically similar]
  static/dashboard.html → frontend/index.html
- `GET /api/song/info` --conceptually_related_to--> `yt-dlp`  [INFERRED]
  static/dashboard.html → README.md
- `Authenticated Downloads (Cookies)` --references--> `lyre service (mbround18/lyre image)`  [EXTRACTED]
  README.md → compose.yaml
- `join_voice_channel()` --calls--> `establish_connection()`  [INFERRED]
  src/api/control.rs → src/database.rs
- `get_guilds()` --calls--> `get_authenticated_user_from_extensions()`  [INFERRED]
  src/api/guilds.rs → src/auth.rs

## Import Cycles
- None detected.

## Hyperedges (group relationships)
- **Queue Management API Endpoints** — static_dashboard_html_get_queue_endpoint, static_dashboard_html_add_queue_endpoint, static_dashboard_html_skip_queue_endpoint, static_dashboard_html_clear_queue_endpoint [EXTRACTED 0.90]
- **Playback Control API Endpoints** — static_dashboard_html_play_endpoint, static_dashboard_html_stop_endpoint, static_dashboard_html_join_endpoint, static_dashboard_html_volume_endpoint [EXTRACTED 0.90]
- **Docker Deployment Stack (compose + Dockerfile + lyre + db)** — compose_yaml_stack, compose_yaml_lyre_service, compose_yaml_db_service, compose_yaml_dockerfile [EXTRACTED 0.90]

## Communities (44 total, 2 thin omitted)

### Community 0 - "API Auth & Control Routes"
Cohesion: 0.07
Nodes (54): ActixResult, HttpResponse, Json, validate_auth(), join_voice_channel(), JoinRequest, next_track(), ActixResult (+46 more)

### Community 1 - "DB Pool & Analytics API"
Cohesion: 0.06
Nodes (53): ConnectionManager, FromRequest, PooledConnection, CacheStats, get_cache_stats(), get_guild_settings(), get_recent_tracks(), GuildSettingsQuery (+45 more)

### Community 2 - "Frontend Lint Config"
Cohesion: 0.05
Nodes (18): plugins, rules, react/only-export-components, react/rules-of-hooks, $schema, App(), Badge(), badgeVariants (+10 more)

### Community 3 - "Queue Advance & Autoplay"
Cohesion: 0.05
Nodes (29): AtomicBool, AtomicU64, AtomicUsize, Instant, definition(), handle(), CommandInteraction, CreateCommand (+21 more)

### Community 4 - "Docker Compose Stack"
Cohesion: 0.08
Nodes (29): DATABASE_URL env var, db service (postgres:18-alpine), Dockerfile, LYRE_HTTP_BIND env var, lyre service (mbround18/lyre image), Docker Compose Stack, Authenticated Downloads (Cookies), Auto-disconnect Feature (+21 more)

### Community 5 - "Frontend Package Deps"
Cohesion: 0.07
Nodes (28): devDependencies, oxlint, tailwindcss, @tailwindcss/vite, @types/node, @types/react, @types/react-dom, typescript (+20 more)

### Community 6 - "Audio Download & Cache"
Cohesion: 0.16
Nodes (27): JoinHandle, PathBuf, cache_dir(), download_base_dir(), DownloadProgress, ensure_yt_dlp(), get_ffmpeg_threads(), get_or_fetch_metadata() (+19 more)

### Community 7 - "Frontend TS App Config"
Cohesion: 0.08
Nodes (24): compilerOptions, allowArbitraryExtensions, allowImportingTsExtensions, erasableSyntaxOnly, jsx, lib, module, moduleDetection (+16 more)

### Community 8 - "Frontend UI Dependencies"
Cohesion: 0.09
Nodes (23): class-variance-authority, clsx, dependencies, class-variance-authority, clsx, lucide-react, next-themes, radix-ui (+15 more)

### Community 9 - "Legacy Static Dashboard"
Cohesion: 0.18
Nodes (20): allUserGuilds, apiCall(), disableButtons(), DISCORD_REDIRECT_URI, displayUserGuilds(), displayUserInfo(), enableButtons(), executeAuth() (+12 more)

### Community 10 - "Frontend Vite Entry"
Cohesion: 0.10
Nodes (21): src/main.tsx entry script, Lyre Dashboard index.html, Oxlint Configuration, React Compiler, React + TypeScript + Vite Template, @vitejs/plugin-react (Oxc), @vitejs/plugin-react-swc (SWC), POST /api/queue/{guild_id}/add (+13 more)

### Community 11 - "Voice Connections Model"
Cohesion: 0.27
Nodes (9): NewVoiceConnection, NaiveDateTime, Option, PgConnection, QueryResult, Self, String, Vec (+1 more)

### Community 12 - "Frontend TS Node Config"
Cohesion: 0.10
Nodes (19): compilerOptions, allowImportingTsExtensions, erasableSyntaxOnly, lib, module, moduleDetection, noEmit, noFallthroughCasesInSwitch (+11 more)

### Community 13 - "Auth Middleware"
Cohesion: 0.14
Nodes (17): Rc, S, Service, ServiceRequest, AuthMiddleware, AuthMiddlewareService, AuthMiddlewareService<S>, extract_token_from_request() (+9 more)

### Community 14 - "DB Listener & Migrations"
Cohesion: 0.13
Nodes (16): Arc, Context, start_listener(), needs_migration(), PgConnection, read_discord_token(), Result, String (+8 more)

### Community 15 - "Play Command Handler"
Cohesion: 0.12
Nodes (16): Event, EventContext, Http, definition(), handle(), Arc, ChannelId, CommandInteraction (+8 more)

### Community 16 - "Frontend Component Aliases"
Cohesion: 0.11
Nodes (17): aliases, components, hooks, lib, ui, utils, iconLibrary, rsc (+9 more)

### Community 17 - "Voice Manager App State"
Cohesion: 0.21
Nodes (10): Default, AppState, Arc, GuildId, HashMap, Option, RwLock, Self (+2 more)

### Community 18 - "Bot Bridge Commands"
Cohesion: 0.20
Nodes (12): BotCommandReceiver, BotCommandSender, Sender, BotCommand, BotResponse, Arc, HashMap, Result (+4 more)

### Community 19 - "OAuth Flow"
Cohesion: 0.22
Nodes (15): exchange_code_for_token(), get_oauth_config(), oauth_callback(), OAuthCallback, OAuthConfig, redirect_uri(), ActixResult, Box (+7 more)

### Community 20 - "Current Queue Model"
Cohesion: 0.28
Nodes (9): CurrentQueue, NewCurrentQueue, NaiveDateTime, Option, PgConnection, QueryResult, Self, String (+1 more)

### Community 21 - "Discord Event Handling"
Cohesion: 0.16
Nodes (10): EventHandler, Interaction, Payload, Ready, Future, HttpRequest, Handler, main() (+2 more)

### Community 22 - "Guild Settings Model"
Cohesion: 0.30
Nodes (8): GuildSettings, NewGuildSettings, NaiveDateTime, Option, PgConnection, QueryResult, Self, String

### Community 23 - "Queue History Model"
Cohesion: 0.27
Nodes (9): NewQueueHistory, QueueHistory, NaiveDateTime, Option, PgConnection, QueryResult, Self, String (+1 more)

### Community 24 - "Song Cache Model"
Cohesion: 0.29
Nodes (8): NewSongCache, NaiveDateTime, Option, PgConnection, QueryResult, Self, String, SongCache

### Community 25 - "Frontend API Client"
Cohesion: 0.17
Nodes (9): api, ApiError, ApiResponse, getToken(), Guild, OAuthConfig, QueueInfo, request() (+1 more)

### Community 26 - "Rust CI Workflow"
Cohesion: 0.40
Nodes (5): cargo build --verbose step, cargo clippy -D warnings step, cargo fmt --check step, cargo test --verbose step, Rust CI Workflow

### Community 27 - "HTTP Server Entrypoint"
Cohesion: 0.40
Nodes (4): Option, Result, String, run_http()

### Community 28 - "Frontend Discord OAuth"
Cohesion: 0.67
Nodes (3): isOAuthMessage(), loginWithDiscord(), OAuthMessage

### Community 29 - "Scratch File"
Cohesion: 0.50
Nodes (3): main(), Error, Result

### Community 30 - "Dashboard Redirect Route"
Cohesion: 0.50
Nodes (3): dashboard_redirect(), ActixResult, HttpResponse

### Community 31 - "Dev Auth Test Route"
Cohesion: 0.50
Nodes (3): get_test_token(), ActixResult, HttpResponse

## Knowledge Gaps
- **130 isolated node(s):** `lyre`, `$schema`, `typescript`, `oxc`, `react/rules-of-hooks` (+125 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **2 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `establish_connection()` connect `DB Pool & Analytics API` to `API Auth & Control Routes`, `Queue Advance & Autoplay`, `Audio Download & Cache`, `DB Listener & Migrations`, `Play Command Handler`, `Discord Event Handling`?**
  _High betweenness centrality (0.069) - this node is a cross-community bridge._
- **Why does `AuthenticatedUser` connect `DB Pool & Analytics API` to `API Auth & Control Routes`, `Auth Middleware`, `Discord Event Handling`?**
  _High betweenness centrality (0.037) - this node is a cross-community bridge._
- **Why does `get_or_fetch_metadata()` connect `Audio Download & Cache` to `DB Pool & Analytics API`?**
  _High betweenness centrality (0.024) - this node is a cross-community bridge._
- **Are the 18 inferred relationships involving `establish_connection()` (e.g. with `get_cache_stats()` and `get_guild_settings()`) actually correct?**
  _`establish_connection()` has 18 INFERRED edges - model-reasoned connections that need verification._
- **What connects `lyre`, `$schema`, `typescript` to the rest of the system?**
  _130 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `API Auth & Control Routes` be split into smaller, more focused modules?**
  _Cohesion score 0.07049180327868852 - nodes in this community are weakly interconnected._
- **Should `DB Pool & Analytics API` be split into smaller, more focused modules?**
  _Cohesion score 0.06327683615819209 - nodes in this community are weakly interconnected._