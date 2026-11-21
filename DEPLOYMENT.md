# NockNock Cloud Deployment Guide

This guide details how to deploy the **NockNock Asset State Room** to the cloud or any remote server.

---

## 1. System Overview

```
                      INTERNET / USER
                            │
                            ▼
              ┌───────────────────────────┐
              │    Nginx (Port 80/443)    │  (or Cloud Load Balancer / Fly.io / Caddy)
              │    - Serves SPA assets    │
              │    - Proxies /v1/ to API  │
              └─────────────┬─────────────┘
                            │
               ┌────────────┴────────────┐
               ▼                         ▼
     ┌───────────────────┐     ┌───────────────────────┐
     │  React Dashboard  │     │   Ingestion Worker    │ (Rust Daemon, Port 8080)
     │  (Vite Static)    │     │   - Alchemy JSON-RPC  │
     └───────────────────┘     │   - Dynamic Strategy  │
                               └───────────┬───────────┘
                                           │
                                           ▼
                               ┌───────────────────────┐
                               │  PostgreSQL Database  │ (Port 5432)
                               │  - Snapshots table    │
                               │  - JSONB metrics data │
                               └───────────────────────┘
```

The system consists of three components:
1. **PostgreSQL Database** — Stores immutable historical snapshots (with JSONB payload, state hash, and timestamps).
2. **Ingestion Worker** (`nocknock-ingestion-worker`) — High-performance Rust binary that polls Base mainnet via Alchemy every 5 minutes, computes derived strategic metrics, and exposes the REST API.
3. **Frontend Dashboard** — Static React/Vite web application served via Nginx with reverse proxy to `/v1/`.

---

## 2. Environment Variables & Secrets

Configure these environment variables in your cloud provider:

| Variable | Description | Example / Default | Required |
|:---|:---|:---|:---:|
| `DATABASE_URL` | PostgreSQL connection string | `postgresql://user:pass@host:5432/nocknock` | **Yes** |
| `APP_DATABASE_URL` | Config prefix override for DB | Same as `DATABASE_URL` | Optional |
| `APP_PORT` / `PORT` | Worker HTTP listen port | `8080` | Optional (default 8080) |
| `APP_ALCHEMY_API_KEYS` | Comma-separated Alchemy API keys | `alch_ZE5FcqyTa3z3ZhhqQQbWK` | **Yes** |
| `APP_TRACKED_WALLETS` | Wallet address to track on Base | `0xeE114cC97eC5adeB34bA5645e315B497D8487a45` | **Yes** |
| `APP_SYNC_WINDOW_SECONDS`| Ingestion polling interval | `300` (5 minutes) | Optional |
| `VITE_API_BASE` | Base URL for frontend API calls | `/v1` (via Nginx proxy) or `http://api.domain.com/v1` | Optional |

---

## 3. Deployment Methods

### Option A: One-Click Docker Compose (Simplest for VPS)
Suitable for DigitalOcean Droplet, Hetzner Cloud, Linode, or AWS EC2 ($5–$10/mo).

1. **SSH into your server**:
   ```bash
   ssh root@your-server-ip
   ```

2. **Clone the repository**:
   ```bash
   git clone https://github.com/your-username/nocknock.git /opt/nocknock
   cd /opt/nocknock
   ```

3. **Verify configuration**:
   Ensure `docker-compose.yml` has your desired database credentials and ports.

4. **Launch all services**:
   ```bash
   docker compose up -d --build
   ```

5. **Verify health**:
   ```bash
   curl http://localhost:8080/health
   # Returns: {"status":"ok","ingestion_lag_seconds":...}
   ```
   Open `http://your-server-ip:3000` in your browser.

---

### Option B: Fly.io (Zero-DevOps Serverless Containers)

Fly.io runs containers close to users with built-in HTTPS and automatic TLS.

1. **Install Fly CLI & Authenticate**:
   ```bash
   curl -L https://fly.io/install.sh | sh
   fly auth login
   ```

2. **Create a Managed Postgres Cluster**:
   ```bash
   fly postgres create --name nocknock-db --region iad --initial-cluster-size 1 --vm-size shared-cpu-1x --volume-size 10
   ```

3. **Deploy Ingestion Worker**:
   In `ingestion-worker/`:
   ```bash
   cd ingestion-worker
   fly launch --no-deploy --name nocknock-api
   fly postgres attach nocknock-db
   fly deploy
   ```

4. **Deploy Frontend**:
   In `frontend/`:
   ```bash
   cd ../frontend
   fly launch --name nocknock-web
   fly deploy
   ```

---

### Option C: Railway or Render (GUI-Driven Deployment)

1. **Fork/Push repository** to GitHub.
2. In **Railway** / **Render**:
   - Create a **PostgreSQL Database** service.
   - Create a **Web Service** for `ingestion-worker`:
     - Root directory: `ingestion-worker`
     - Dockerfile: `Dockerfile`
     - Environment variable: `DATABASE_URL` linked to the PostgreSQL service.
     - Port: `8080`
   - Create a **Static Site** or **Web Service** for `frontend`:
     - Root directory: `frontend`
     - Dockerfile: `Dockerfile`
     - Port: `80`

### Option D: Rootless Podman Quadlets (Internal Hosts, No Public Exposure)

For a LAN-only host you own. Services run as systemd user units under your account, start
at boot and survive logout.

1. **One-time host setup** (needs sudo on the host):
   ```bash
   sudo apt install podman
   ```

2. **Deploy from your checkout** (needs `ingestion-worker/config.yaml` with your Alchemy key):
   ```bash
   deploy/podman/deploy.sh <host>
   ```
   The script ships the working tree minus gitignored files, builds both images on the
   host, stores `config.yaml` and a generated database password as Podman secrets,
   installs the quadlets into `~/.config/containers/systemd/` and restarts the stack.
   Re-run it to upgrade; the database volume and password are kept. Upgrades are forward-only: a build older than the newest migration already applied will not start (see AGENTS.md section 13).
   Any change to `config.yaml` reaches the host only through this step, because the
   worker reads the stored secret. That includes a new ladder entry (`role: ladder`,
   #167) and its `deposit_tx`: after deploying a ladder rung, add its entry and mint
   transaction to `config.yaml`, then re-run `deploy.sh <host>`. The receipt is checked
   on the next cycle; the ladder's reading says `verified`, or why not.

3. **Endpoints**: dashboard on `http://<host>:3000`, API and HTTP MCP on
   `http://<host>:8081` (host port 8080 is left free). PostgreSQL is reachable only
   inside the `nocknock` container network.

4. **Operate**:
   ```bash
   systemctl --user status nocknock-db nocknock-worker nocknock-frontend
   journalctl --user -u nocknock-worker -f
   ```

5. **The database is disposable.** Nothing here is a backup, and none is installed. The
   wallet and the chain are the record: after losing the database, start fresh with the
   latest release and your `config.yaml`. To restore an accumulation plan's progress, set
   `accumulation_plan.starts_at` to its real start. From v0.6.0 the worker then finds each
   buy since that time and offers it for confirmation on the dashboard; recording each
   buy's transaction hash again works too. Receipts are re-verified on-chain either way.

6. **Point a local MCP client at the deployment** by setting `NOCKNOCK_API_URL` for the
   stdio server (it defaults to `http://127.0.0.1:8080`):
   ```json
   { "mcpServers": { "nocknock": { "command": "./dist/nocknock-mcp", "env": { "NOCKNOCK_API_URL": "http://<host>:8081" } } } }
   ```

7. **After repositioning liquidity**, update the `positions:` list in `config.yaml` and
   make sure the entry flagged `is_price_source: true` is a pool that still has active
   liquidity; the worker refuses to publish a price from an empty pool.

8. **On your phone** (home-screen install, notifications, optional HTTPS): see
   [section 7](#7-on-your-phone-install-notifications-https-opt-in). A default deploy
   installs none of the HTTPS parts.

---

## 4. Reverse Proxy & SSL (HTTPS) with Caddy

If deploying on a VPS, use Caddy for automated Let's Encrypt SSL:

```caddyfile
# /etc/caddy/Caddyfile
state.yourdomain.com {
    reverse_proxy localhost:3000
}

api.yourdomain.com {
    reverse_proxy localhost:8080
}
```
Reload Caddy:
```bash
sudo systemctl reload caddy
```

This section is for a public server with a real domain. For a LAN-only host, where no
public certificate authority can vouch for the name, see
[section 7.3](#73-https-on-the-lan-opt-in).

---

## 5. Database Maintenance

1. **No backups.** The database is disposable; the wallet and the chain are the record
   (operator decision 2026-11-14, issue #96 closed as not planned).

2. **Deduplication Safeguard**:
   The ingestion worker includes built-in deduplication. If on-chain balances and pool reserves do not change between intervals, `store_snapshot` refreshes the `generated_at` timestamp on the active record rather than inserting redundant rows. Storage footprint remains under 50 MB/year.

---

## 6. Operational Monitoring & Runbook

* **Liveness Probe**: `GET /health` returns HTTP 200 with `status: "ok"` and `ingestion_lag_seconds`.
* **Force Manual Ingest**: `POST /v1/ingest` triggers an immediate Alchemy sync without waiting for the 5-minute cron.
* **Logs Inspection**:
  ```bash
  # Docker Compose
  docker compose logs -f worker

  # Systemd / Bare Metal
  journalctl -u nocknock-worker -f
  ```

---

## 7. On Your Phone: Install, Notifications, HTTPS (opt-in)

For the Podman deployment (Option D). Three separate parts; each is off until you turn
it on. The full guide, with the trade-offs and a precise list of what has and has not
been tested, is
[`docs/kb/how-to-phone-install-notifications-tls.md`](docs/kb/how-to-phone-install-notifications-tls.md).
None of this has been tried on a real phone or on the host yet.

### 7.1 Install on a phone

The dashboard ships a web app manifest, icons and the iOS and Android home-screen tags.
Nothing to configure.

1. On the phone, on the same network, open `http://<host>:3000/?view=lite`.
2. iPhone: Safari, Share, "Add to Home Screen". Android: Chrome, ⋮, "Add to Home screen".
3. Open it from the home screen: it opens the Lite cockpit.

- Android installs a web app only from an HTTPS address. Over plain HTTP it adds a
  bookmark that opens in a browser tab (see 7.3).
- There is no service worker. Nothing is cached, so the app never shows an old snapshot
  as live, and it does not open away from the LAN.
- The Lite cockpit's DIRECTIVE card says whether the page is live. `NOT LIVE · last read
  12m ago` means the numbers below it are that old. If the phone leaves the LAN with the
  app open, the card reads `NOT LIVE · Worker unreachable: …` after the next poll (10 s),
  and it reads the worker again when the app returns to the foreground.

### 7.2 Notifications

The worker pushes a message when the directive changes between two cycles, when an
accumulation plan completes, and when spot reaches the top of the price-source range (a
range exit; pushed even under a floor lock or a due tranche, where the directive does
not change). It does not push when spot falls back into the range unless that changes
the directive, and a failed ingestion cycle pushes nothing. It is off until
`ingestion-worker/config.yaml` has a `notifications:` block. Set one channel or several.

**Telegram**

```yaml
notifications:
  telegram_bot_token: "123456789:AAExampleExample"
  telegram_chat_id: "123456789"
  cockpit_url: "http://<host>:3000/?view=lite"
```

1. In Telegram, message **BotFather**, send `/newbot`, and follow it. The token it
   gives you is `telegram_bot_token`.
2. Open a chat with your new bot and send it any message.
3. `curl -s "https://api.telegram.org/bot<token>/getUpdates"`: in the answer,
   `result[0].message.chat.id` is `telegram_chat_id`.

**Pushover**

```yaml
notifications:
  pushover_token: "your-application-api-token"
  pushover_user: "your-user-key"
  cockpit_url: "http://<host>:3000/?view=lite"
```

1. Create an account at pushover.net and install the Pushover app on the phone.
2. **Your User Key**, on the website's front page after login, is `pushover_user`.
3. **Create an Application/API Token** gives the **API Token/Key**: `pushover_token`.

`pushover_user_key`, the name the example used until v0.5.3, is read when
`pushover_user` is not set. A config with both loads, and `pushover_user` is used.

**Webhook**: `webhook_url: "https://…"` receives `POST {"text": "…"}`.

Then deploy (`deploy/podman/deploy.sh <host>`) and test:

```bash
curl -s http://<host>:8081/v1/notifications/status         # which channels are configured
curl -s -X POST http://<host>:8081/v1/notifications/test   # send one message labelled TEST
```

or press **SEND TEST NOTIFICATION** on the dashboard (last panel) or the Lite cockpit
(bottom card). Each channel is reported as `delivered`, `failed` with the provider's
reason, `incomplete` (half a channel is set) or `not_configured`. `delivered` means the
provider accepted the message. With no channel configured the route says so and sends
nothing. Tokens are never logged or returned. Test sends are limited to one per 30 s.

Keep credentials in `config.yaml` only (gitignored; stored on the host as the Podman
secret `nocknock-config`). `config.yaml.example` is the base layer of every deployment's
config, so nothing real belongs in it.

### 7.3 HTTPS on the LAN (opt-in)

```bash
NOCKNOCK_TLS=1 NOCKNOCK_TLS_NAMES="<host> <host-ip>" deploy/podman/deploy.sh <host>
```

This installs one more rootless quadlet, `nocknock-tls`: a Caddy container with its own
private certificate authority, serving `https://<host>:3443` in front of the dashboard.
No `sudo`, no privileged port, and host port 8080 is refused.

| Variable | Meaning | Default |
|:---|:---|:---|
| `NOCKNOCK_TLS` | `1` installs and restarts the proxy; `remove` stops it and removes its quadlet (its CA volume is kept). Unset: the deploy does not touch it. | unset |
| `NOCKNOCK_TLS_NAMES` | Every name or IPv4 address the phone will type, space separated. The certificate covers only these. | the host argument |
| `NOCKNOCK_TLS_PORT` | Host port, 1024 to 65535, not 8080, 3000 or 8081, no leading zero. | `3443` |

A private CA means **each phone must install and trust its root certificate**:
download it from `https://<host>:3443/nocknock-root-ca.crt` (or fetch it with
`deploy/podman/tls/export-root-ca.sh <host>`), compare its SHA-256 fingerprint with the
one the deploy printed, then install it (iPhone: Settings, Profile Downloaded, then
General, About, Certificate Trust Settings; Android: Settings, Security, Encryption &
credentials, Install a certificate, CA certificate).

Know the cost before doing that. The root you install can vouch for **any** website, and
its private key sits in the `nocknock-tls-data` volume on the host. Install it only on
your own devices and remove it when you stop using this. Ports 3000 and 8081 still serve
plain HTTP.

Two alternatives avoid the private root but need accounts that are yours to open, so
they are described in the guide and not built: a tailnet (Tailscale issues a public
certificate and also reaches the dashboard away from home), or a real domain with a
DNS-01 certificate.
