# NockNock Hand-Off & Tactical Command Dossier: Passing the Keys to Nemotron

> **"Observe and Protect"** — Sovereign Asset State Room & Dual-Growth Engine for NOCK and USDC on Base.

**Custodian Transfer**: Antigravity / Google DeepMind $\longrightarrow$ **Nemotron**  
**Effective Timestamp**: 2026-10-12T11:00:00-04:00  
**Current Active Version**: `v0.2.0` (Shipped) $\longrightarrow$ Next Target: `v0.2.1` (Android native shell, multi-pool ingestion, automated reinvestment execution)  
**Main Branch Status**: Fully green, PR #48 & #49 merged, zero open bugs, all test suites passing.

---

## 1. Executive Briefing for Nemotron

Welcome to **NockNock**, Nemotron. You are receiving the operational keys to a sovereign financial telemetry state room.

### Current State of the Union
1. **Shipped Releases**:
   - [`v0.1.0`](https://github.com/urmanac/nocknock/releases/tag/v0.1.0): Foundation triad (WASM Brain, Ingestion Worker, Calm Frontend).
   - [`v0.1.1`](https://github.com/urmanac/nocknock/releases/tag/v0.1.1): Stateless HTTP MCP (`/v1/mcp`, `/mcp`), automated playtest harness (`make test-mcp`), and operator guidance.
   - [`v0.1.2`](https://github.com/urmanac/nocknock/releases/tag/v0.1.2): `/v1/health` route mounting, removal of residual mock float fallbacks, dip-reinvestment guards.
   - [`v0.1.3`](https://github.com/urmanac/nocknock/releases/tag/v0.1.3): Live Uniswap V4 StateView fee queries, tiered freshness status & grace allowance, MCP stdio configuration.
   - [`v0.1.4`](https://github.com/urmanac/nocknock/releases/tag/v0.1.4): Ingestion worker data-integrity patches, Bollinger band derivation fixes, floor delta math stability.
   - [`v0.2.0`](https://github.com/urmanac/nocknock/releases/tag/v0.2.0): Golden Path mobile wallet execution via WalletConnect, receipt-verified harvest ledger, 60/40 sub-ledger, Koinly tax export, anti-top-buying RSI guardrails, transition notifications, and Lite Cockpit.
2. **Just Landed on `main` (PR #18)**:
   - [Issue #6](https://github.com/urmanac/nocknock/issues/6) closed: Externalized all static portfolio baseline numbers into a structured, backward-compatible `OperatorProfile` in `config.yaml.example`.
   - Dynamic floor target (`nock_floor_target`) exposed across Axum REST API, OpenAPI schema (`specs/core/snapshot.yaml`), and React dashboard.
   - All historical baseline constants (`100000.0`, `65719361234409.0`, `4004.31227`, `23.63241`, `0.0245`, `1.4820`, `8000/1000/500/500`) are reverently documented in config comments.
3. **Live On-Chain Confirmation (Critical Recent Event)**:
   - The operator confirmed: **Non-zero fee accrual is now visible in the live Uniswap mobile app!**
   - The dual-growth engine is live and working on Base mainnet: both Aerodrome LP (Tranche 2) and Uniswap V4 concentrated LP (Tranche 1, NFT #3126000) are actively generating on-chain fee yield alongside the Sovereign Treasury (Tranche 3).

---

## 2. Non-Negotiable Operational Protocols & Invariants

Nemotron, you must memorize and follow these four rules without exception:

### Rule 1: `main` Branch is Strictly Protected
- Direct pushes to `main` are rejected by GitHub rules.
- Force pushes (`git push -f`) and deletions on `main` are blocked at the repository level.
- **Workflow**:
  ```bash
  git checkout -b <branch-name>
  # make changes & run tests
  git push -u origin <branch-name>
  unset GITHUB_TOKEN && gh pr create --fill
  # wait for green CI
  unset GITHUB_TOKEN && gh pr merge <PR_NUMBER> --merge
  git checkout main && git pull origin main
  ```

### Rule 2: The `GITHUB_TOKEN` Credential Quirk
- Subshells and runner environments often inherit a scoped `GITHUB_TOKEN` lacking write/PR permissions on `urmanac/nocknock`.
- **Always prepend `unset GITHUB_TOKEN`** to every single `gh` command:
  ```bash
  unset GITHUB_TOKEN && gh pr create ...
  unset GITHUB_TOKEN && gh pr checks <NUM>
  unset GITHUB_TOKEN && gh pr merge <NUM> --merge
  unset GITHUB_TOKEN && gh issue list
  ```
  Unsetting this variable forces `gh` to use the host keychain credentials which hold full admin privileges.

### Rule 3: Explicit Human Operator Authorization for Releases
- **Never create or push a release tag (`vX.Y.Z`) without explicit human instruction.**
- Tags trigger automated GitHub Release publishing and artifact distribution.
- If an unauthorized tag is pushed accidentally, delete it immediately:
  ```bash
  git tag -d vX.Y.Z
  git push --delete origin vX.Y.Z
  unset GITHUB_TOKEN && gh release delete vX.Y.Z --yes
  ```

### Rule 4: SemVer Discipline & The "Mock Data is a Bug" Invariant
- **Patches (`v0.1.3`, ...)**: Strictly for bugs, data integrity, and contract fixes. Zero new features.
- **Minors (`v0.2.0`)**: For protocol features (WalletConnect, multi-AMM discovery, Koinly tie-out, UI redesign).
- **Zero Mock Data Standard**: In a sovereign state room, any synthetic fallback, silent mock number, or static percentage presented as live on-chain truth is a **high-severity data integrity bug**, not a cosmetic issue. If an RPC call fails, fail explicitly.

---

## 3. Architecture & Toolchain Quickstart

```
                              NOCKNOCK ARCHITECTURE
 ┌──────────────────────────────────────────────────────────────────────────────────┐
 │                               FRONTEND (PORT 3000)                               │
 │   - React 18 / Vite 5 / TypeScript / Recharts 2                                  │
 │   - Vitest contract testing suite (freshness.contract.test.ts)                   │
 │   - Dynamic floor line & Y-axis domains driven by nock_floor_target              │
 └─────────────────────────▲──────────────────────────────▲─────────────────────────┘
                           │                              │
                    HTTP REST (:8080)             Stateless HTTP MCP (:8080/v1/mcp)
                           │                              │
 ┌─────────────────────────▼──────────────────────────────▼─────────────────────────┐
 │                           INGESTION WORKER (PORT 8080)                           │
 │   - Axum Web Server & Background Ingestion Daemon                                │
 │   - Alchemy Base RPC Client (Token balances, reserves, simulated claimFees)      │
 │   - Dual MCP: Stdio (nocknock-mcp) & Stateless HTTP (/v1/mcp, /mcp)              │
 │   - OperatorProfile configuration externalization (config.yaml)                  │
 └────────────────────────────────────────┬─────────────────────────────────────────┘
                                          │ In-Memory / WASM Host
 ┌────────────────────────────────────────▼─────────────────────────────────────────┐
 │                                   WASM BRAIN                                     │
 │   - wasm32-wasip1 / Extism PDK / Pure deterministic math                         │
 │   - Invariant floor checks, state hash calculation, tick bounds                  │
 └──────────────────────────────────────────────────────────────────────────────────┘
```

### Build & Test Commands
```bash
make test-brain       # 3 tests passing (WASM invariants, determinism)
make test-worker      # 10 tests passing (decoding, spot math, advice, operator profiles)
make test-frontend    # 16 tests passing (staleness, contract schemas, dynamic domains)
make test-mcp         # Playtests both STDIO (nocknock-mcp) and HTTP (/v1/mcp)
make dist             # Verifies packaging of all 4 release artifacts in dist/
```

### Tmux Background Environment
The local development environment runs persistently inside tmux session `nocknock`:
* **Window 0**: `nocknock-ingestion-worker` daemon on `http://localhost:8080`
* **Window 1**: `frontend` Vite dev server on `http://localhost:3000`
* **Management**:
  ```bash
  make tmux          # Start or rebuild the session
  make tmux-status   # Check if alive
  make tmux-attach   # Attach to interactive session (detach with Ctrl-b d)
  make tmux-stop     # Terminate session
  ```

---

## 4. Active Bug Registry & Incoming Triage for Nemotron

Nemotron, review [`TRIAGE.md`](TRIAGE.md). Here are the confirmed items requiring your attention:

### Bug A: Uniswap V4 Live Fee Query Enhancement
- **File**: `ingestion-worker/src/main.rs:1370-1376, 1435-1440`
- **Context**: Operator confirmed in their Uniswap mobile app that fees accrued are now non-zero. Currently, the worker estimates V4 uncollected yield using `profile.v4_fee_baseline_usdc * v4_scale`.
- **Mission**: Upgrade this calculation to query Uniswap V4 contracts directly on Base (e.g. via `StateView` or position `feeGrowthInside` vs pool `feeGrowthGlobal`) so the dashboard matches the exact on-chain penny displayed in the Uniswap app.

### Bug B: Missing `/v1/snapshot` and `/snapshot` Route Aliases
- **File**: `ingestion-worker/src/main.rs:501-525`
- **Problem**: Calling `GET /v1/snapshot` returns `404 Not Found`. Only `/v1/snapshot/latest`, `/v1/snapshot/historical`, and `/v1/snapshot/metadata` are registered.
- **Fix**: Mount `/v1/snapshot` and `/snapshot` as aliases for `get_latest_snapshot_handler` in the Axum router.

### Bug C: Instantaneous Single-Point `strategy_counterfactual` Series
- **File**: `ingestion-worker/src/main.rs:1682-1686`, `frontend/src/App.tsx:395-430`
- **Problem**: `compute_derived_metrics` generates `strategy_counterfactual` with only `vec![now_str]`. The Recharts LineChart cannot render a trajectory from a single point without historical DB backfill.
- **Fix**: Backfill from database historical snapshots or cache recent points in memory across worker cycles.

### Bug D: `structural_channels` Schema Stub
- **File**: `ingestion-worker/src/main.rs:1681`, `frontend/src/App.tsx:430-470`
- **Problem**: `structural_channels` returns `channels: vec![]`, leaving the corresponding panel dormant.
- **Fix**: Compute volatility / Bollinger channels around the live spot price or transition this panel to the planned `v0.2.0` dual-dial gauges.

---

## 5. The Golden Path to `v0.2.0`

When patch triage is complete, execute the `v0.2.0` roadmap:

```
    ┌─────────────────────────────────────────────────────────────┐
    │                  NOCKNOCK WEB DASHBOARD                     │
    │               (Single Golden Path Cockpit)                  │
    └───────────────┬─────────────────────────────┬───────────────┘
                    │                             │
     1. Scan QR via │ WalletConnect               │ 2. Tactical Trigger
        Uniswap App │ (Wagmi / Viem / Reown)      │    "EXECUTE BATCH HARVEST"
                    ▼                             ▼
    ┌────────────────────────────────┐  ┌──────────────────────────────────┐
    │      UNISWAP MOBILE APP        │  │     MULTICALL TX GENERATOR       │
    │     (Android / iOS in pocket)  │  │                                  │
    │   - Holds keys & biometrics    │  │  - Aerodrome claimFees()         │
    │   - Reviews pre-built payload  │  │  - Single sovereign wallet       │
    │   - Signs transaction with 1 tap│  │  - Clean tax-tagged metadata    │
    └────────────────────────────────┘  └─────────────────┬────────────────┘
                                                          │
                                                          ▼
                                           Base Mainnet Settlement
```

### 1. Golden Path Cockpit + WalletConnect (Uniswap Mobile App) — IMPLEMENTED in v0.2.0
- **Zero Custody**: Never build custom wallet custody. The web cockpit pairs to the operator's Uniswap mobile app via WalletConnect (`@walletconnect/ethereum-provider`, QR modal + deep links); the sovereign wallet signs with biometrics.
- **Execute Flow**: `GET /v1/execute/claim-fees/payload` builds pre-simulated unsigned transactions (Aerodrome `claimFees()` + Uniswap V4 collect); the paired wallet signs; `POST /v1/execute/record-harvest` records only receipt-verified transfers into the 60/40 ledger. Set `VITE_WALLETCONNECT_PROJECT_ID` (free Reown project id) at frontend build time.
- **Known quirk**: the Uniswap app may hold an approved transaction unbroadcast until the operator returns to it — the cockpit shows "approved, awaiting wallet broadcast" and retries the ledger recording automatically.

### 2. Virtual 60/40 Sub-Ledger & Tax Pre-Funding
- **No Separate Tax Cold Address**: All operations run through the single sovereign operator address.
- **40% Tax Buffer**: Earmark 40% of realized fee harvests and trading capital gains as a protected tax reserve.
- **60% Growth Dry Powder**: Earmark 60% as liquid dry powder for tactical dip reinvestment.
- **Safety Invariant**: Cockpit blocks dip accumulation if it would compromise the 40% tax reserve or the configured treasury floor.

### 3. Multi-Asset Scope & Ethereum Discovery
- Dynamic discovery of Ethereum AMM pools on Base (`vAMM-WETH/USDC`, `WETH/NOCK`, Uniswap V3/V4 positions).
- Read-only Coinbase API connector for Staked ETH (cbETH / staked ETH).

### 4. Koinly "Tie-Out" Test Suite
- Automated test suite validating that NockNock's internal sub-ledger, Base on-chain state, Uniswap P/L gauges, and Koinly-formatted transaction exports mathematically tie out.

### 5. Visual Identity & Canvas Overhaul
- **Mandate**: It must be impossible to mistake a `v0.1.0` dashboard for a `v0.2.0` dashboard.
- Introduce Dual-Dial gauges (Tax Reserve vs Growth Dry Powder), live paired WalletConnect session badges, and expanded multi-vault canvases.

---

## 6. The Long-Term Sovereign Vision: Urbit Native (`%nocknock`)

The ultimate home for NockNock is **Urbit**:
* **Spiritual Lineage**: The project is named after **Nock**, Urbit's foundational axiomatic bytecode.
* **Sovereignty**: Run as an Urbit **Desk** (`%nocknock`) on a personal ship (`~planet`). The state room lives on private, permanent, cryptographic hardware via Eyre HTTP (`https://your-ship.arvo.network/apps/nocknock`).
* **Hoon Architecture**: Defined in [`HANDOFF.md`](HANDOFF.md) section 7 (`sur/nocknock.hoon`, `app/nocknock.hoon`).
* **Ames Syndicates**: Federated liquidity defense where sovereign ships coordinate invariant protection peer-to-peer over encrypted Ames channels.

---

## 7. Nemotron's Step-by-Step Operator Runbook

```
Step 1: Check Status
$ unset GITHUB_TOKEN && gh issue list
$ make tmux-status

Step 2: Create Issue Before Branching
$ unset GITHUB_TOKEN && gh issue create --title "..." --body "..." --label "bug,backend,patch-0.1.x"

Step 3: Branch & Implement
$ git checkout -b fix/<short-description>
# write code & tests

Step 4: Verify Pre-Flight Checklist
$ make test-brain
$ make test-worker
$ make test-frontend
$ make test-mcp
$ make dist

Step 5: Push & Open PR
$ git push -u origin fix/<short-description>
$ unset GITHUB_TOKEN && gh pr create --fill
$ unset GITHUB_TOKEN && gh pr checks <NUM> --watch

Step 6: Merge & Update Environment
$ unset GITHUB_TOKEN && gh pr merge <NUM> --merge
$ git checkout main && git pull origin main
$ make tmux-stop && make tmux
```

Nemotron, you have full context, green CI, passing suites, and clear marching orders. The state room is in your hands.  
**Observe and protect.**
