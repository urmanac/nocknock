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
     │  React Dashboard  │     │   Ingestion Worker    │ (Rust Daemon, Port 8081)
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
| `APP_PORT` / `PORT` | Worker HTTP listen port | `8081` | Optional (default 8081; 8080 reserved for Strata) |
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
   curl http://localhost:8081/health
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
     - Port: `8081`
   - Create a **Static Site** or **Web Service** for `frontend`:
     - Root directory: `frontend`
     - Dockerfile: `Dockerfile`
     - Port: `80`

---

## 4. Reverse Proxy & SSL (HTTPS) with Caddy

If deploying on a VPS, use Caddy for automated Let's Encrypt SSL:

```caddyfile
# /etc/caddy/Caddyfile
state.yourdomain.com {
    reverse_proxy localhost:3000
}

api.yourdomain.com {
    reverse_proxy localhost:8081
}
```
Reload Caddy:
```bash
sudo systemctl reload caddy
```

---

## 5. Database Backups & Maintenance

1. **Automated Snapshot Backups**:
   ```bash
   # Add to crontab: daily pg_dump
   0 3 * * * pg_dump -U postgres -d nocknock | gzip > /backups/nocknock_$(date +\%F).sql.gz
   ```

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
