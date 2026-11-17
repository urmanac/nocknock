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
- **AERO is read, never steered.** Since v0.6.0 NockNock reads the wallet's AERO and the
  AERO each staked Aero position has earned and not claimed, and states the amounts. It
  sets no target for AERO and gives no advice to claim, sell, hold or accumulate it.
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
- `positions`: the LP positions to track (Uniswap v4, Uniswap v3, Aero volatile pools and,
  since v0.6.0, Aero concentrated "CL" positions with `protocol: aerodrome-cl`, staked or
  not), one of which is the NOCK/USD price source (`is_price_source: true`). A position
  whose `protocol` is none of these is a configuration error, never a guess.
  Discovery-verified identifiers for the original operator's positions are in
  [`ONCHAIN_INTEGRATION.md`](ONCHAIN_INTEGRATION.md) §5; Aero CL is §8. An Aero CL position
  is counted only after you add it to `positions:` (example in `config.yaml.example`):
  until then it is not tracked and the totals leave it out.
  When you withdraw a tracked NFT position in the wallet app its NFT is burned. NockNock
  then reports the entry as **closed** (#144): listed apart from the live positions,
  counted in no total, with one notification, while ingestion goes on. Remove the entry
  when you like. Only proof counts (two reads at one current block that both give the
  position manager's own "no such token" answer,
  [`ONCHAIN_INTEGRATION.md`](ONCHAIN_INTEGRATION.md) §13); a node that does not answer,
  or one that is more than two minutes behind the chain, still fails the cycle, and a
  mistyped or missing `nft_id` reads as closed too.
  If the closed entry is the price source, a Uniswap v4 entry's pool keeps supplying the
  price until you flag another position; a Uniswap v3 or Aero CL one fails the cycle and
  asks you to flag another.
- AERO, the fourth pillar (since v0.6.0, [`ONCHAIN_INTEGRATION.md`](ONCHAIN_INTEGRATION.md)
  §9), needs no switch. Every cycle reads the wallet's AERO (counted in the total, like
  gas), the unclaimed AERO of each staked Aero position (an `aerodrome` position needs
  its `gauge_address`; an `aerodrome-cl` position's gauge is read from its pool) and
  AERO/USD from `aero_price_pool` (an AERO/USDC pool; the shipped default is verified and
  refused if it has no in-range liquidity). Unclaimed AERO is shown beside uncollected LP
  fees and is not in the total until you claim it. A staked position's unclaimed AERO is
  reported as that position's collectible earnings (#142): the amount, its USD value and
  the gauge it is collected from, in the Aero app. LP fees, AERO rewards and their sum
  are three labelled figures, never one merged number, and the harvest rule keeps
  counting LP fees only. Anything that cannot be read says so
  with the reason; it is never shown as 0. AERO is the token the Aero gauges pay
  (`0x3f5bF3Ed…a4Ee`), not the older "Aerodrome" token with the same symbol: a legacy
  gauge that pays the older token is stated as not read, never shown as AERO.
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
- `cost_basis.opening`: optional, and yours alone to fill in: what the NOCK you already
  held cost you, as of an instant you name, with a note on where the figures came from.
  Without it NockNock reports the average cost of your plan's verified buys and nothing
  more (see "Cost basis" below).

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
| 7 | **HOLD & ACCRETE** | Nothing above applies | What the LP positions are doing. No profit or loss is claimed: cost basis is reported separately, as a measurement (below), and never drives a directive. |

Alongside the directive, the **LP rebuild bias** stands: while LP holds less than your
LP target, NockNock suggests deposits into the positions you track. Rebuilding ranges is
a reposition, with no net cash change. With no LP target configured it suggests nothing.

### Cost basis: what NockNock knows, and what only you know

NockNock reports what NOCK cost in two layers, and keeps them apart.

**Plan buys (always on).** Every buy the accumulation plan paced and verified on-chain is
in the accumulation ledger with the USDC that left the wallet and the NOCK that entered
it. Their running average cost is exact. The dashboard panel **Average cost vs spot**
charts it against the stored NOCK price, marks each buy, lists what each buy did to the
average, and states what that NOCK is worth at the current spot against what it cost.
That difference is a measurement: a valuation of the NOCK those buys acquired. It does
not say the NOCK is still held, because NockNock does not track whether it was later
sold or moved into LP. NockNock sizes no buy from it and never suggests averaging down
([why](docs/lore/2026-11-14-four-pillars-no-hurry.md)).

**Whole stack (only if you supply it).** NockNock cannot work out what the rest of your
NOCK cost. The chain does not say what you paid before the plan, what you sold, or what
moved through LP ranges. If you want a whole-stack average, enter your own opening
figures in `config.yaml`:

```yaml
cost_basis:
  opening:
    nock_amount: <NOCK you held at as_of, as a number>
    cost_usd: <what that NOCK cost you in total, in USD, as a number>
    as_of: <the instant you read those figures, written like "YYYY-MM-DDTHH:MM:SSZ">
    source: <where the figures came from, in your own words>
```

- Blended average = (`cost_usd` + USDC of verified buys confirmed after `as_of`) /
  (`nock_amount` + NOCK received in those buys). It is shown with your `source` note and
  the instant, as your figure. NockNock neither reads nor checks the opening numbers.
- Buys confirmed at or before `as_of` are taken to be inside your figures and are not
  added again. That is why `as_of` must be a full timestamp: a date alone does not say
  whether a buy made that day is already counted.
- **Not tracked after `as_of`:** sells, NOCK that LP ranges sell or buy back, fees earned
  in NOCK, transfers in or out, buys you did not record, realized gains and losses, and
  gas. The panel and the MCP text show the NOCK NockNock reads now (to the nearest
  whole NOCK) beside the NOCK the basis covers, so the difference is visible.
- Unset, the whole-stack basis reads "not configured". A half-filled block, a zero or
  negative amount, or an `as_of` in the future is a configuration error, shown on the
  dashboard, in the snapshot and in MCP. It is never completed with a default, and it
  does not stop the worker.

Where the numbers are: `derived_metrics.cost_basis` in the snapshot, the series behind
the chart at `GET /v1/cost-basis/series` (fills with their running averages, and stored
spot; gaps in the price history stay gaps, while consecutive backfilled candles are one
captioned stretch, not a gap each), and two lines in the `get_portfolio_state`
MCP text. After a database loss the plan buys come back when you record their
transaction hashes again; the opening basis lives in `config.yaml`; the stored price
history restarts from backfilled candles, so older stretches of the spot line are coarser.
A backfilled candle is drawn at its open time with its open price (the price at that
instant), and a buy is matched to the worker's own stored read when one lies within 60
minutes of it, otherwise to the nearest backfilled point in that window.

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
  - 7 KPI Stat Cards: Sovereign NOCK Stack, USDC Stack, Net Portfolio, Claimable Yield (LP fees and AERO rewards as two labelled figures and their total; the LP-fee net of gas, as before, for a wallet with no staked Aero position), Floor Health, High-Water Mark, AERO (wallet).
  - Position cards: a staked Aero position shows its gauge reward as its collectible earnings, with the LP-fee reason beneath; the velocity chart draws rewards as their own series beside LP fees.
  - Pillar line: NOCK · USDC · ETH · AERO, which add up to the total, with what is held as gas and as AERO and what AERO is unclaimed in each tracked gauge.
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
does not own), the dashboard and `/v1/snapshot/latest` show the last ingestion error. An
NFT that does not exist at all (withdrawn and burned, or a mistyped `nft_id`) is listed
under "Closed positions" and does not fail the cycle (#144), with three exceptions that
still do: the entry is a Uniswap v3 or Aero CL price source (`is_price_source: true`),
every configured position is closed (for example, it is your only entry), or its
position manager is not one of the three whose answers were verified.

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

# 4. Trigger Immediate On-Chain Sync (waits for a cycle already running, then runs one)
curl -X POST http://localhost:8080/v1/ingest
```

---

## 6. Cloud Deployment

For deploying to production servers, VPS (DigitalOcean / Hetzner), Fly.io, or AWS, refer to the complete **[Deployment Guide](DEPLOYMENT.md)**.

### On your phone (opt-in)

Three separate parts, each off until you turn it on. Steps, config blocks and the phone
checklist are in [DEPLOYMENT.md section 7](DEPLOYMENT.md#7-on-your-phone-install-notifications-https-opt-in);
the trade-offs are in [`docs/kb/how-to-phone-install-notifications-tls.md`](docs/kb/how-to-phone-install-notifications-tls.md).

* **Install**: open `http://<host>:3000/?view=lite` on the phone and use "Add to Home
  Screen". It opens the Lite cockpit, which shows the directive, the plan and whether
  the page is live. There is no service worker: nothing is cached, so an old snapshot is
  never shown as live.
* **Notifications**: add a `notifications:` block (Telegram, Pushover or a webhook) to
  `config.yaml`. The worker pushes when the directive changes, when a plan completes
  and when spot reaches the top of the price-source range.
  `POST /v1/notifications/test`, or SEND TEST NOTIFICATION on the dashboard, sends one
  message labelled TEST and reports each channel.
* **HTTPS on the LAN**: `NOCKNOCK_TLS=1 deploy/podman/deploy.sh <host>` adds a Caddy
  proxy on `https://<host>:3443` with a private CA. Each phone must install that CA's
  root certificate, which is a real trade-off: read the guide first.

None of the three has been tried on a real phone yet.

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
- `get_portfolio_state`: Balances, the four pillars (NOCK, USDC, ETH, AERO), gas held, AERO held and unclaimed (each staked position's reward with the gauge it is collected from), collectible earnings (LP fees, AERO rewards and their sum, none of it in the total), the NOCK floor buffer (if you set a floor), the average cost of your plan's verified buys against spot (and the whole-stack average if you configured an opening basis), and the directive.
- `get_shot_advice`: The current directive, its details, uncollected LP fees, collectible AERO rewards and the total collectible as three labelled lines, and the harvest rule (which counts LP fees only).
- `simulate_claim_fees`: Gas versus uncollected LP fees across the tracked LP positions, then collectible AERO rewards and the total collectible. A staked Aero position's row states its reward (amount, USD value, the gauge it is collected from, in the Aero app) and then why its LP fees are not LP-claimable. A gas estimate for collecting a reward is printed only for a gauge whose claim call is verified on-chain ([`ONCHAIN_INTEGRATION.md`](ONCHAIN_INTEGRATION.md) §9); otherwise the row says the gas is not estimated. These are amounts, not a suggestion to claim, and no transaction is built for a reward.
- `get_accumulation_plan`: Your DCA plan's status, next tranche, verified buys, swaps found on-chain that await your confirmation (listed only: MCP has no write tool), the LP rebuild bias and, while a tranche is due, read-only route quotes for that amount on Aero and Uniswap (a comparison, not a recommendation).

Every answer ends with the snapshot's generation time and the worker's health.

Automated GitHub Actions workflows (`.github/workflows/ci.yml`) test all three tiers on every pull request and publish GitHub Releases with release notes extracted from [CHANGELOG.md](CHANGELOG.md) upon pushing version tags (`git tag v0.1.0`). For release governance and step-by-step instructions, see the **[Release Runbook](RELEASE.md)**. For the strategic roadmap and future development priorities, refer to the **[Hand-Off Document](HANDOFF.md)**.

---

## 9. License

Copyright 2026 Urmanac.

Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0).

See the [LICENSE](LICENSE) file for full license terms.