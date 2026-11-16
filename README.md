# NockNock — Sovereign Asset State Room

> **"Observe and Protect"** — a read-only observation portal and advisory engine for a
> single wallet on Base that holds NOCK, cash (USDC), gas (ETH) and AERO. NockNock reads
> the wallet and the chain, states facts, and paces the plan you configure. It never
> signs or sends a transaction.

*Vibe-coded: AI agents wrote almost all of this, and the docs may contain hallucinations.
The example numbers in this repository are one operator's, about one wallet. They are not
recommendations.*

---

## 1. Philosophy: four pillars, and no hurry

The reasoning behind NockNock is written up in
[`docs/lore/2026-11-14-four-pillars-no-hurry.md`](docs/lore/2026-11-14-four-pillars-no-hurry.md).
In short:

- **Four pillars, not a pyramid.** The wallet holds NOCK, cash (USDC), gas (ETH) and AERO.
  NockNock neither minimizes nor maximizes any of them. LP positions are where the pillars
  work together, not a fifth pillar.
- **Gas is not cash.** ETH pays for transactions. It is never counted toward a cash
  reserve, and NockNock suggests no amount of it to hold
  ([`OPERATOR_GUIDANCE.md`](OPERATOR_GUIDANCE.md) §7.12).
- **The wallet is the record.** Balances, positions, fees and recorded buys are read from
  the chain. NockNock's database (high-water mark, charts) is a convenience: if it is
  lost, start fresh.
- **Your numbers are yours.** A buying plan's budget, tranches and interval, a NOCK floor,
  target weights, a cash reserve, a tax-reserve ratio: NockNock never chooses them. Leave
  one unset and there is no rule.

### What is tracked

Everything comes from `ingestion-worker/config.yaml`:

- `tracked_wallets`: the wallet to read.
- `positions`: the LP positions to track (Uniswap v4, Uniswap v3, Aero volatile pools),
  one of which is the NOCK/USD price source (`is_price_source: true`). Discovery-verified
  identifiers for the original operator's positions are in
  [`ONCHAIN_INTEGRATION.md`](ONCHAIN_INTEGRATION.md) §5.
- `operator_profile`: an optional NOCK floor, target weights, tax-reserve ratio and LP
  floor tolerance.
- `accumulation_plan`: an optional dollar-cost-averaging plan that paces NOCK buys you
  make yourself. After a buy, NockNock finds the swap in the wallet's on-chain history and
  offers it on the dashboard: CONFIRM counts it (its receipt is verified again first),
  NOT PART OF THE PLAN leaves it out. A detected swap is never counted on its own, and
  recording a buy by its transaction hash works whether or not detection ran.
- `route_quotes.enabled` (default `true`): while a tranche of the plan is due, NockNock
  asks Aero's and Uniswap's quoter contracts what that exact amount returns in NOCK and
  shows both. Read-only, a comparison and not a recommendation; set it to `false` to
  stop the quoter calls.

The shipped `config.yaml.example` is loaded as the base layer and still carries the
original operator's wallet, positions and limits. Set or clear every one of those in your
own `config.yaml`; they are not defaults meant for you.

---

## 2. The directive engine

Each ingestion cycle produces one directive. The first rule that applies wins:

| # | Directive | When | What it says |
|:--|:---|:---|:---|
| 1 | **DEFENSIVE LOCK (Floor Recovery)** | You set `treasury_floor_nock` and total NOCK (wallet plus LP) is below it | How far below, and your plan's next step if you have one. Nothing is "locked"; NockNock does not size recovery buys. |
| 2 | **OVEREXTENDED** | Daily RSI(14) is 70 or more (and the plan pauses when overextended) | No buy and no sale is advised while it stays there. A range exit or a due harvest still outranks it. |
| 3 | **DCA TRANCHE DUE** | Your accumulation plan has a tranche due | Buy that tranche's size of NOCK yourself. NockNock then finds the swap on-chain and offers it for you to confirm (or record its transaction hash); either way the receipt is verified on-chain before it counts. |
| 4 | **RANGE EXIT / BATCH HARVEST** | The price-source range has been exited, or uncollected LP fees reach the fixed $5.00 threshold | Facts about the exited range, or that fees are worth collecting. Harvest is advised only when no tranche is due. |
| 5 | **Waiting plan** (DCA ON SCHEDULE, pending, paused, blocked) | A plan exists but nothing is due | When the next tranche is due and its size. |
| 6 | **Planless arms** (nothing to track yet, build from zero, tax reserve short, dip zone) | No plan is configured | Facts only: what the wallet holds and where the price is. No buy is sized; the numbers are yours to set. |
| 7 | **HOLD & ACCRETE** | Nothing above applies | What the LP positions are doing. No profit or loss is claimed (NockNock does not track cost basis). |

Alongside the directive, the **LP rebuild bias** stands: while LP holds less than your
LP target, NockNock suggests deposits into the positions you track. Rebuilding ranges is
a reposition, with no net cash change. With no LP target configured it suggests nothing.

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
- Docker & Docker Compose (or PostgreSQL 16 + Rust 1.80+ + Node 22+)
- Alchemy API Key for Base Mainnet

### Running via Docker Compose

```bash
# Clone the repository
git clone https://github.com/urmanac/nocknock.git
cd nocknock

# Write your own config: your Alchemy key, your wallet, your positions (one of them
# the NOCK/USD price source), and only the limits and plan you choose. The example's
# wallet, positions, floor and targets are one operator's: replace or clear them.
cp ingestion-worker/config.yaml.example ingestion-worker/config.yaml
$EDITOR ingestion-worker/config.yaml

# Start PostgreSQL, Ingestion Worker, and Frontend
docker compose up -d --build

# Open Dashboard
http://localhost:3000

# REST API
http://localhost:8080/v1/snapshot/latest
http://localhost:8080/health
```

If the first ingestion cycle fails (a placeholder key, no price source, an NFT the wallet
does not own), the dashboard and `/v1/snapshot/latest` show the last ingestion error.

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
# 1. Run Frontend Tests (counts in AGENTS.md §9)
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
- `get_portfolio_state`: Balances, the NOCK floor buffer (if you set a floor), and the directive.
- `get_shot_advice`: The current directive, its details, and the harvest rule.
- `simulate_claim_fees`: Gas versus uncollected LP fees across the tracked LP positions.
- `get_accumulation_plan`: Your DCA plan's status, next tranche, verified buys, swaps found on-chain that await your confirmation (listed only: MCP has no write tool), the LP rebuild bias and, while a tranche is due, read-only route quotes for that amount on Aero and Uniswap (a comparison, not a recommendation).

Every answer ends with the snapshot's generation time and the worker's health.

Automated GitHub Actions workflows (`.github/workflows/ci.yml`) test all three tiers on every pull request and publish GitHub Releases with release notes extracted from [CHANGELOG.md](CHANGELOG.md) upon pushing version tags (`git tag v0.1.0`). For release governance and step-by-step instructions, see the **[Release Runbook](RELEASE.md)**. For the strategic roadmap and future development priorities, refer to the **[Hand-Off Document](HANDOFF.md)**.

---

## 9. License

Copyright 2026 Urmanac.

Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0).

See the [LICENSE](LICENSE) file for full license terms.