# Deployment — Hetzner Server

This file describes how to build, deploy, and operate FartCloud on the
production server. Read it entirely before making changes that affect deployment.

---

## Server

| Property       | Value                          |
|----------------|-------------------------------|
| IP             | `46.225.85.7`                 |
| SSH user       | `admin`                       |
| SSH key        | `~/.ssh/hetzner`              |
| Apps directory | `/opt/apps/`                  |
| Static files   | `/var/www/fartcloud/`         |
| OS             | Ubuntu 24.04 LTS              |

The server runs Docker + Docker Compose. Caddy handles TLS and reverse-proxying.
The infrastructure repo is at `c:\DevOps\HetznerManagement` (or the org's ops repo).

---

## App type

**Type**: `wasm`

This is a Rust/Macroquad game compiled to WebAssembly, served as static files.

| Property       | Value                                                  |
|----------------|--------------------------------------------------------|
| Build tool     | `cargo build --release --target wasm32-unknown-unknown` |
| Output         | `dist/` (`.wasm` + `assets/` + `index.html`)           |
| Runs as        | Static files served by Caddy                            |
| Framework      | Macroquad 0.4                                           |
| Language       | Rust → WASM                                             |

---

## CI/CD — GitHub Actions

The workflow file is at `.github/workflows/deploy.yml`.

**Trigger**: push to `main` branch.

**Required GitHub secrets** (set in repo Settings → Secrets → Actions):

| Secret              | Value                                      |
|---------------------|--------------------------------------------|
| `HETZNER_SSH_KEY`   | Private key content of `~/.ssh/hetzner`    |
| `HETZNER_SSH_HOST`  | `46.225.85.7`                              |
| `HETZNER_SSH_USER`  | `admin`                                    |

### Deploy flow

1. `cargo build --release --target wasm32-unknown-unknown`
2. Prepare `dist/`: copy `.wasm`, `assets/`, `web/index.html`
3. Optional: `wasm-opt -Oz` for size optimization
4. `rsync -avz --delete dist/ admin@46.225.85.7:/var/www/fartcloud/`
5. Caddy serves the files immediately (no restart needed)

---

## Reverse proxy (Caddy)

Caddy config lives in the ops repo: `HetznerManagement/docker/Caddyfile`.

This app is served at:
- **URL**: `http://46.225.85.7/fartcloud/`
- **Caddy block type**: `handle_path` + `file_server` (static WASM app, path-based)

To add or modify the routing, edit `HetznerManagement/docker/Caddyfile` then run:
```bash
make docker-restart   # from HetznerManagement/
```

---

## Adding this app to the server (first deploy)

1. Create the directory on the server:
   ```bash
   ssh -i ~/.ssh/hetzner admin@46.225.85.7 \
     "sudo mkdir -p /var/www/fartcloud && sudo chown admin:admin /var/www/fartcloud"
   ```

2. Add a Caddy block inside the existing `:80` block in `HetznerManagement/docker/Caddyfile`:
   ```caddy
   # Inside :80 { ... }
   redir /fartcloud /fartcloud/ permanent
   handle_path /fartcloud/* {
       root * /var/www/fartcloud
       encode gzip
       file_server
       @wasm path *.wasm
       header @wasm Content-Type "application/wasm"
       header @wasm Cache-Control "public, max-age=31536000, immutable"
       @assets path *.js *.css *.ogg *.png
       header @assets Cache-Control "public, max-age=31536000, immutable"
       @html path *.html
       header @html Cache-Control "no-cache"
       try_files {path} /index.html
   }
   ```

3. Sync and reload Caddy:
   ```bash
   make docker-restart   # from HetznerManagement/
   ```

4. Push to `main` → GitHub Actions deploys automatically.

---

## Platform API integration

FartCloud connects to an external platform via REST API for authentication,
game config, leaderboard, and score submission. See `API_SPEC.md` for full docs.

The platform URL is configured via:
1. **URL query parameter** `?api=URL` (preferred — set by CORE when embedding or redirecting)
2. **Meta tag** `<meta name="platform-api-url">` in `web/index.html` (fallback)

If neither is set, the game runs in **anonymous mode** (fully playable, no API calls).

**Integration URLs:**
- Redirect: `http://46.225.85.7/fartcloud/?token=JWT&api=https://platform-url.com`
- iframe: `<iframe src="http://46.225.85.7/fartcloud/?token=JWT&api=https://platform-url.com">`

### Modes

| Mode | Condition | Behavior |
|------|-----------|----------|
| **Anonymous** | No `PLATFORM_API_URL` or no token | Game playable, scores in sessionStorage, no leaderboard |
| **Connected** | `PLATFORM_API_URL` set + valid token | Config override, score submission, leaderboard from platform |

### API Endpoints (see `API_SPEC.md`)

| Endpoint | Method | Direction |
|----------|--------|-----------|
| `/api/game/auth/validate` | GET | Validate auth token |
| `/api/game/auth/login` | POST | Login (placeholder) |
| `/api/game/config?gameId=fartcloud` | GET | Config override (partial) |
| `/api/game/scores?gameId=fartcloud` | POST | Submit score |
| `/api/game/leaderboard?gameId=fartcloud` | GET | Get leaderboard |

---

## Monitoring

| Dashboard  | URL                            | Notes                        |
|------------|-------------------------------|------------------------------|
| Portainer  | `http://46.225.85.7:9000`     | Containers, CPU/RAM, logs    |
| Netdata    | `http://46.225.85.7:19999`    | System metrics, real-time    |

---

## Environment variables / secrets

| Variable              | Where set                        | Notes                          |
|-----------------------|----------------------------------|--------------------------------|
| `PLATFORM_API_URL`    | URL `?api=` param or meta tag    | Platform API base URL (empty = anonymous mode) |
| `HETZNER_SSH_KEY`     | GitHub Actions secret            | SSH private key for deploy     |
| `HETZNER_SSH_HOST`    | GitHub Actions secret            | Server IP                      |
| `HETZNER_SSH_USER`    | GitHub Actions secret            | SSH user                       |

No server-side secrets needed — this is a fully static client-side app.
The platform API handles all server-side logic (auth, scores, config).

---

## Local development

```bash
# Build and run natively (desktop, no WASM)
cargo run

# Build WASM for testing
cargo build --release --target wasm32-unknown-unknown
mkdir -p dist
cp target/wasm32-unknown-unknown/release/fartcloud.wasm dist/
cp -r assets dist/
cp web/index.html dist/
# Serve dist/ with any static file server (e.g. python3 -m http.server -d dist 8080)
```

The app does not need Docker locally. In anonymous mode, no external service is needed.

---

## Project structure

```
fartcloud/
├── .github/workflows/deploy.yml  # CI/CD: build WASM → rsync to Hetzner
├── src/main.rs                   # Entire game (Rust/Macroquad, single file)
├── Cargo.toml                    # Rust dependencies
├── web/index.html                # HTML template with JS bridge (platform API + keyboard)
├── assets/
│   ├── config.json               # Game parameters (local defaults)
│   ├── sounds/                   # Sound effects (.ogg)
│   └── sprites/                  # PNG sprites
├── API_SPEC.md                   # Platform API specification
├── AGENTS.md                     # This file (deployment & ops)
└── SETUP.md                      # Project setup guide
```

---

## Ops helper commands (run from HetznerManagement/)

```bash
make ssh                          # SSH into the server
make docker-ps                    # Container status
make docker-logs-svc SVC=<name>  # Logs for this service
make docker-pull                  # Pull latest images + restart
make docker-restart               # Reload Caddy config
```
