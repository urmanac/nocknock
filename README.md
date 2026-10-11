# NockNock — Sovereign Asset State Room & Dual-Growth Engine

> **"Observe and Protect"** — An asynchronous, zero-maintenance asset observation portal and tactical advisory engine for dual-token growth (**NOCK** and **USDC**) on Base network.

---

## 1. Strategic Philosophy & Portfolio Architecture

NockNock monitors and guides a three-tier capital allocation strategy designed to grow the absolute holding of both **NOCK** and **USDC** while maintaining an invariant floor:

```
                            SOVEREIGN CAPITAL FLOW
 
  ┌────────────────────────────────────────────────────────────────────────┐
  │ 1. SOURCING LIQUIDITY                                                 │
  │    ┌───────────────────────────┐      ┌───────────────────────────┐   │
  │    │ Uniswap V4 LP (Tranche 1) │      │  Aerodrome LP (Tranche 2) │   │
  │    │ Range: $0.01237 - $2.028  │      │  Volatile Pool (0.3% fee) │   │
  │    │ Automated DCA Scale-Out   │      │  Continuous Yield Engine  │   │
  │    └─────────────┬─────────────┘      └─────────────┬─────────────┘   │
  └──────────────────┼──────────────────────────────────┼─────────────────┘
                     │                                  │
                     ▼                                  ▼
  ┌────────────────────────────────────────────────────────────────────────┐
  │ 2. HARVESTING & ALLOCATION HUB (60/40 SPLIT RULE)                      │
  │    When claimable fees cross $5.00 threshold:                          │
  │    ├── 40% ➔ Tax Escrow (Cold reserve pre-funding capital gains liability)
  │    └── 60% ➔ Growth Capital / The Vault (Dry powder for market dips)   │
  └────────────────────────────────────────────────────────────────────────┘
                     │
                     ▼
  ┌────────────────────────────────────────────────────────────────────────┐
  │ 3. SOVEREIGN FOUNDATION (Tranche 3)                                    │
  │    Untouchable Treasury Floor: >= 100,000 NOCK                         │
  │    Hard safety floor — never liquidated, staked, or impaired.          │
  └────────────────────────────────────────────────────────────────────────┘
```

### Strategic Tranches

1. **Tranche 1: Uniswap V4 LP (5% Target Allocation)**
   - Contract / NFT ID: `#3126000` on Base (`0x7c5f5a4bbd8fd63184577525326123b519429bdc`)
   - Tick Range: `[-274200, -223200]` ($0.01237 to $2.02783)
   - Function: Automated DCA scale-out engine. As NOCK price appreciates toward $2.028, liquidity gradually shifts into 100% USDC.
2. **Tranche 2: Aerodrome Engine (10% Target Allocation)**
   - Pool: `0x85f1aa3a70fedd1c52705c15baed143e675cd626` (USDC/NOCK 0.3% volatile)
   - Function: High-velocity trading fee accumulator. Yield is harvested and recycled.
3. **Tranche 3: Sovereign Treasury (80% Target Allocation)**
   - Hard Floor Invariant: **100,000 NOCK minimum**.
   - Liquid cold wallet reserves protecting sovereign voting weight and governance posture.
4. **Liquid USDC Buffer (5% Target Allocation)**
   - Dry powder from DCA scale-outs and harvested yield, deployed on market retractions.

---

## 2. Tactical Shot Advice Engine

The backend continuously evaluates on-chain conditions and outputs dynamic recommendations:

| Trigger Condition | Posture | Strategic Action |
|:---|:---|:---|
| `total_nock < 100,000` | **DEFENSIVE LOCK** | Floor breach! All yield locked to restore 100,000 sovereign minimum. |
| `spot_price < $0.020` | **DIP REINVESTMENT** | Spot price entered accumulation zone. Deploy saved USDC to buy discounted NOCK. |
| `spot_price >= $2.028` | **RANGE EXIT FLASHPOINT** | V4 runway is 100% USDC. Take principal and construct single-sided high walls at $5.00+. |
| `claimable_fees >= $5.00` | **BATCH HARVEST** | Gas efficiency optimal (>50x). Execute batch claim and enforce 60/40 Split Rule. |
| Normal Market | **HOLD & ACCRETE** | AMMs generating dual yield; maintain current balanced posture. |

---

## 3. Technical Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         NOCKNOCK SYSTEM ARCHITECTURE                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌──────────────────┐      HTTP/JSON (REST)       ┌──────────────────┐   │
│   │  Ingestion       │ ◄─────────────────────────► │  Frontend        │   │
│   │  Backend Engine  │    /v1/snapshot/latest      │  (React/TS/Vite) │   │
│   │  (Rust / Axum)   │    /v1/snapshot/historical  │  Dashboard Room  │   │
│   └────────┬─────────┘    /v1/snapshot/metadata    └────────┬─────────┘   │
│            │              /health                           │             │
│            │                                                │             │
│            ▼                                                ▼             │
│   ┌──────────────────┐                             ┌──────────────────┐   │
│   │ PostgreSQL 16    │                             │ useFreshness Hook│   │
│   │ TimescaleDB      │                             │ Stale Banner     │   │
│   │ Deduplicated DB  │                             │ React ErrorBound │   │
│   └──────────────────┘                             └──────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

* **Rust Ingestion Worker** (`ingestion-worker/`):
  - Polls Base mainnet via Alchemy JSON-RPC:
    - `eth_gasPrice` for real-time gas break-even economics ($0.0043/tx)
    - `getReserves` and LP `balanceOf` on Aerodrome pool
    - `getPositionLiquidity(3126000)` on Uniswap V4 PositionManager
    - `alchemy_getTokenBalances` for liquid wallet tokens
  - Computes derived metrics, deviation matrix, and shot advice.
  - Generates deterministic SHA-256 state hashes.
  - Implements snapshot deduplication to prevent database bloat.
* **React Frontend Dashboard** (`frontend/`):
  - 6 KPI Stat Cards: Sovereign NOCK Stack, USDC Stack, Net Portfolio, Claimable Yield, Floor Health, High-Water Mark.
  - Tactical Shot Advice Cockpit Banner.
  - 7 Recharts panels with historical time-series curves and 100,000 NOCK Reference Line.
  - Client-side freshness validation (5-minute sync window, zero urgency).
  - Protected with top-level React Error Boundary.

---

## 4. Quick Start

### Prerequisites
- Docker & Docker Compose (or PostgreSQL 16 + Rust 1.80+ + Node 20+)
- Alchemy API Key for Base Mainnet

### Running via Docker Compose

```bash
# Clone the repository
git clone https://github.com/your-username/nocknock.git
cd nocknock

# Start PostgreSQL, Ingestion Worker, and Frontend
docker compose up -d --build

# Open Dashboard
http://localhost:3000

# REST API
http://localhost:8080/v1/snapshot/latest
http://localhost:8080/health
```

### Running in Persistent Background with Tmux (Recommended)

To run both the ingestion worker and frontend dev server in a persistent, detached background session:

```bash
make tmux         # Builds worker & launches both services in 'nocknock' session
make tmux-attach  # Connects directly to session (Ctrl-b d to detach)
make tmux-status  # Checks active session status
make tmux-stop    # Gracefully terminates the session
```

### Running Manually

```bash
# Terminal 1: Database
# Ensure PostgreSQL is running on localhost:5432 with database 'nocknock'

# Terminal 2: Ingestion Worker
cd ingestion-worker
DATABASE_URL="postgresql://postgres:postgres@localhost:5432/nocknock" cargo run --release

# Terminal 3: Frontend Portal
cd frontend
npm install
npm run dev -- --port 3000
```

---

## 5. Verification & Testing

```bash
# 1. Run Frontend Contract Tests (12/12 passing)
cd frontend
npm test -- --run

# 2. Build Frontend Production Bundle
npm run build

# 3. Check Rust Compilation & Offline SQLX Cache
cd ../ingestion-worker
SQLX_OFFLINE=true cargo check

# 4. Trigger Immediate On-Chain Sync
curl -X POST http://localhost:8080/v1/ingest
```

---

## 6. Cloud Deployment

For deploying to production servers, VPS (DigitalOcean / Hetzner), Fly.io, or AWS, refer to the complete **[Deployment Guide](DEPLOYMENT.md)**.

---

## 7. Spec Contracts

Single sources of truth in `specs/core/`:
* `specs/core/snapshot.yaml` — Canonical data model (OpenAPI 3.0)
* `specs/core/api.yaml` — REST API interface specification

---

## 8. Release Machinery & Progressive Degradation

NockNock is engineered following the **Mecris Progressive Degradation Model** (detailed in [ARCHITECTURE.md](ARCHITECTURE.md)), maintaining three tiers of execution:
1. **Full Cloud / Server Mode**: PostgreSQL + Axum Worker + React Portal.
2. **Desktop / MCP Adapter Mode**: Agent-driven MCP server without direct database access.
3. **Edge / Moon Clock Android Model**: Embedded WASM Brain (`nocknock_brain.wasm`) evaluating on-chain math directly on Android handsets with public RPC fallback chains.

### Release Packaging Commands

```bash
# Package all release artifacts into dist/
make dist

# Build individual target artifacts
make build-brain    # Compiles wasm32-wasip1 deterministic brain (134 KB)
make build-worker   # Compiles release ingestion worker and MCP binaries
make build-frontend # Compiles optimized production web bundle

# Run Model Context Protocol (MCP) server for local AI agents
make run-mcp
```

### Model Context Protocol (MCP) Configuration

To connect NockNock to Claude Desktop, Gemini Antigravity, Cursor, or Mecris, add the server to your client configuration:

```json
{
  "mcpServers": {
    "nocknock": {
      "command": "/path/to/nocknock/ingestion-worker/target/release/nocknock-mcp",
      "args": []
    }
  }
}
```

**Exposed MCP Tools:**
- `get_portfolio_state`: Real-time sovereign balances, NOCK floor buffer delta, and tranche allocations.
- `get_shot_advice`: Dynamic tactical directives (Hold & Accrete, Batch Harvest, Dip Reinvestment).
- `simulate_claim_fees`: Simulated gas expenditure vs gross/net claimable yields.

Automated GitHub Actions workflows (`.github/workflows/ci.yml`) test all three tiers on every pull request and publish GitHub Releases with release notes extracted from [CHANGELOG.md](CHANGELOG.md) upon pushing version tags (`git tag v0.1.0`). For release governance and step-by-step instructions, see the **[Release Runbook](RELEASE.md)**.

---

## 9. License

Copyright 2026 Urmanac.

Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0).

See the [LICENSE](LICENSE) file for full license terms.