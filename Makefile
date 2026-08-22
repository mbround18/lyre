.DEFAULT_GOAL := help

ENV_FILE := .env
DEV_DB_PORT := 5433
DEV_HTTP_PORT := 3100
DEV_DATABASE_URL := postgres://postgres:postgres@localhost:$(DEV_DB_PORT)/lyre
DEV_HTTP_BIND := 127.0.0.1:$(DEV_HTTP_PORT)
DEV_REDIRECT_URI := http://localhost:$(DEV_HTTP_PORT)/auth/callback

.PHONY: help dev down install install-backend install-frontend db-up db-down db-wait check-token check-port

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}'

$(ENV_FILE):
	cp .env.example $(ENV_FILE)
	@echo "Created $(ENV_FILE) from .env.example - set DISCORD_TOKEN (or DISCORD_BOT_TOKEN/BOT_TOKEN/DOCKER_TOKEN) before running 'make dev'"

install: install-backend install-frontend ## Install Rust + frontend dependencies

install-backend: ## Fetch Rust dependencies
	cargo fetch

install-frontend: frontend/node_modules ## Install frontend dependencies

frontend/node_modules: frontend/package.json frontend/pnpm-lock.yaml
	cd frontend && pnpm install
	@touch frontend/node_modules

db-up: ## Start the local Postgres container
	docker compose up -d db

db-down: ## Stop the local Postgres container
	docker compose stop db

db-wait: db-up ## Start Postgres and wait until it accepts connections
	@echo "Waiting for Postgres..."
	@until docker compose exec -T db pg_isready -U postgres >/dev/null 2>&1; do sleep 1; done
	@echo "Postgres is ready"

check-token: $(ENV_FILE)
	@grep -qE '^(DISCORD_TOKEN|DISCORD_BOT_TOKEN|BOT_TOKEN|DOCKER_TOKEN)=\S+' $(ENV_FILE) || \
		(echo "Set DISCORD_TOKEN (or DISCORD_BOT_TOKEN/BOT_TOKEN/DOCKER_TOKEN) in $(ENV_FILE) before running 'make dev'" >&2 && exit 1)

# Fails loudly up front instead of letting `cargo run` silently die in the background
# (which previously left the frontend dev server proxying into whatever else already
# owned the port, e.g. Grafana on 3000 - a confusing 404 instead of a clear error).
check-port:
	@(exec 3<>/dev/tcp/127.0.0.1/$(DEV_HTTP_PORT)) 2>/dev/null && \
		(echo "Port $(DEV_HTTP_PORT) is already in use - set DEV_HTTP_PORT=<port> (e.g. 'make dev DEV_HTTP_PORT=3101') to use a different one" >&2 && exit 1) || true

dev: $(ENV_FILE) install db-wait check-token check-port ## Run Postgres + backend + frontend together for local dev
	@echo "Starting backend (http://$(DEV_HTTP_BIND)) and frontend dev server - Ctrl+C to stop"
	@echo "Discord OAuth redirect URI for this run: $(DEV_REDIRECT_URI)"
	@echo "  (must be added under OAuth2 > Redirects in the Discord Developer Portal for your app, once)"
	@trap 'kill 0' EXIT INT TERM; \
	(DATABASE_URL=$(DEV_DATABASE_URL) LYRE_HTTP_BIND=$(DEV_HTTP_BIND) DISCORD_REDIRECT_URI=$(DEV_REDIRECT_URI) cargo run) & \
	(cd frontend && VITE_API_PROXY_TARGET=http://$(DEV_HTTP_BIND) pnpm dev) & \
	wait

down: db-down ## Stop dev background services (Postgres)
