# NockNock Issue Triage & Bug Classification Runbook (`TRIAGE.md`)

> **Core Principle**: "Mock data in the production app is a bug." — Sovereign State Room Directive

This document defines the issue triage protocol, label architecture, bug severity taxonomy, and active bug registry for the **NockNock** project across the `v0.1.x` patch release cycle.

---

## 1. Issue Triage Philosophy & Invariants

1. **Strict Semantic Versioning (SemVer)**:
   - **Patch Releases (`v0.1.1`, `v0.1.2`, ...)**: Strictly reserved for bug fixes, data integrity corrections, performance optimizations, and UX contrast/readability repairs. Zero new features.
   - **Minor Releases (`v0.2.0`, ...)**: New protocol features (Ethereum AMM dynamic discovery, Coinbase API Staked ETH connector, WalletConnect 1-click execution, parameterized operator profiles).
2. **The "Zero Mock Data" Standard**:
   - In a sovereign state room, any synthetic fallback, hardcoded static percentage, or unverified number presented as live on-chain truth is classified as a **high-severity data-integrity bug**, not a cosmetic issue.
   - If an RPC query fails, the system must either fail explicitly, flag the snapshot as `degraded: true`, or present a clean empty/loading state. It must never silently substitute arbitrary constants.
3. **Pre-Branching Triage Rule**:
   - Before branching for code changes, every identified issue must be formally reported on GitHub with:
     - Exact source code locations (`file:line`).
     - On-chain contract evidence, selectors, and transaction hashes (if applicable).
     - Concrete reproduction steps.
     - Actionable remediation plan.

---

## 2. GitHub Issue Label Taxonomy

| Label | Color | Description | Usage |
|---|---|---|---|
| **`bug`** | `#d73a4a` | Something is broken or behaving incorrectly | All confirmed defects |
| **`data-integrity`** | `#b60205` | Mock data, silent fallbacks, or mathematical errors | Calculations, mock strings, corrupted state |
| **`frontend`** | `#1d76db` | UI components, React hooks, and visualization canvas | `frontend/src/**` |
| **`backend`** | `#5319e7` | Ingestion daemon, Axum REST API, and MCP adapter | `ingestion-worker/src/**` |
| **`patch-0.1.x`** | `#0e8a16` | Targeted for resolution in the `v0.1.x` patch series | Fixes required prior to `v0.2.0` |
| **`patch-0.2.x`** | `#0e8a16` | Targeted for resolution in the `v0.2.x` patch series | Fixes required prior to `v0.3.0` |
| **`enhancement`** | `#a2eeef` | New feature or architectural expansion | Features deferred to the next minor release |
| **`triage`** | `#d4c5f9` | Needs investigation or reproduction | Initial incoming state |

---

## 3. Active Issue Inventory

### Phase A: Frontend Stability & Dynamic Reactivity (`v0.1.1` Patch Target)

#### 1. [Issue #7: `useFreshness` fails to poll new snapshots and becomes permanently stale](https://github.com/urmanac/nocknock/issues/7)
* **Component**: `frontend/src/hooks/useFreshness.tsx:128-150`
* **Labels**: `bug`, `frontend`, `patch-0.1.x`
* **Status**: Resolved & Merged (PR #13)
* **Defect**: The polling timer interval re-evaluates freshness only on the snapshot captured in the initial mount closure. It never invokes `fetchSnapshot()` or `refresh()`. After 300 seconds, the dashboard permanently displays `STALE`.
* **Fix**: Store the active snapshot in a ref and trigger background `refresh()` using `fetchIfFresh` (ETag conditional GET) on every poll interval.

#### 2. [Issue #8: Remove hardcoded mock constants, static labels, and unreactive colors](https://github.com/urmanac/nocknock/issues/8)
* **Component**: `frontend/src/App.tsx:205, 226, 266, 320, 328, 336, 343, 423`
* **Labels**: `bug`, `frontend`, `data-integrity`, `patch-0.1.x`
* **Status**: Resolved & Merged (PR #13)
* **Defect**:
  - AreaChart legend names contain static token balances in parentheses (`(96,005 NOCK)`, `(6,329 NOCK)`, `(4,004 NOCK)`).
  - Alpha text in Strategy Counterfactual hardcodes `(+5.6% above passive hold)`.
  - Safety Buffer text hardcodes `(+6.3%)`.
  - Floor Health card highlight color remains green (`#2ecc71`) even when below the 100k floor.
  - Fallback strings default to test values (`'354.19'`, `'3062'`, `'2898'`).
* **Fix**: Compute all percentage deltas dynamically, remove static quantities from legend names, and make alert colors reactive.

#### 3. [Issue #9: Sovereign NOCK Stack chart clips data outside static 90k–115k YAxis domain](https://github.com/urmanac/nocknock/issues/9)
* **Component**: `frontend/src/App.tsx:302`
* **Labels**: `bug`, `frontend`, `patch-0.1.x`
* **Status**: Resolved & Merged (PR #13)
* **Defect**: Hardcoded `domain={[90000, 115000]}` clips chart data if total stack dips below 90k or grows above 115k.
* **Fix**: Use dynamic domain padding relative to `dataMin` and `dataMax` while keeping the 100k floor line visible.

---

### Phase B: Backend Data Integrity & History (`v0.1.1` / `v0.1.x` Patch Target)

#### 4. [Issue #10: Snapshot deduplication overwrites `generated_at` and destroys historical time-series](https://github.com/urmanac/nocknock/issues/10)
* **Component**: `ingestion-worker/src/main.rs:1385-1406`
* **Labels**: `bug`, `backend`, `data-integrity`, `patch-0.1.x`
* **Status**: Resolved (Branch `fix/backend-yield-and-history-integrity`)
* **Defect**: When `state_hash` is unchanged, `store_snapshot` updates `generated_at` on the previous row. This erases the duration for which that state existed and prevents the accumulation of historical rows in steady markets.
* **Fix**: Preserve historical timestamps by skipping row mutation in DB when state hash is unchanged, while serving fresh timestamps from in-memory cache.

#### 5. [Issue #11: Prevent silent mock fallbacks when primary RPC queries fail](https://github.com/urmanac/nocknock/issues/11)
* **Component**: `ingestion-worker/src/main.rs:920-1005`
* **Labels**: `bug`, `backend`, `data-integrity`, `patch-0.1.x`
* **Status**: Resolved (Branch `fix/backend-yield-and-history-integrity`)
* **Defect**: If Alchemy RPC calls fail, local variables silently fall back to `96004.52`, `0.2316`, `81297590130616`, generating synthetic snapshots with fresh timestamps.
* **Fix**: Fail the ingestion cycle explicitly rather than publishing synthetic numbers to database or frontend.

#### 6. [Issue #5: Dynamic on-chain fee calculation for Uniswap V4 uncollected yield](https://github.com/urmanac/nocknock/issues/5)
* **Component**: `ingestion-worker/src/main.rs:1043-1048`
* **Labels**: `bug`, `backend`, `data-integrity`, `patch-0.1.x`
* **Status**: Resolved & Merged (PR #14)
* **Defect**: Uniswap V4 uncollected fees are hardcoded to `0.0` following the auto-compounding transaction (`0x6e1b...ddf2`), hiding newly accreting yield from the dashboard.
* **Fix**: Dynamically calculate accreting V4 fees scaled by position liquidity ratio and config baselines, displaying accurate uncollected yield.

#### 7. [Issue #16: Mount /v1/health route and eliminate residual fallback constants in derived metrics](https://github.com/urmanac/nocknock/issues/16)
* **Component**: `ingestion-worker/src/main.rs:418, 1390-1450`, `frontend/src/api/freshness.contract.test.ts:54`
* **Labels**: `bug`, `backend`, `frontend`, `data-integrity`, `patch-0.1.x`
* **Status**: Resolved (Branch `fix/v1-health-and-derived-fallbacks`)
* **Defect**: `/v1/health` returned 404 due to root-only `/health` registration; `compute_derived_metrics` contained residual fallback floats (`0.0268`, `96004.52`, etc.) violating zero-mock invariant.
* **Fix**: Mount `/v1/health` alongside `/health`, eliminate all synthetic constants, and add contract tests.

---

#### 8. [Issue #6: Externalize static portfolio baseline numbers into operator profile config](https://github.com/urmanac/nocknock/issues/6)
* **Component**: `ingestion-worker/src/main.rs:30-60, 1165-1195`, `ingestion-worker/config.yaml.example`, `frontend/src/App.tsx`, `specs/core/snapshot.yaml`
* **Labels**: `enhancement`, `backend`, `frontend`
* **Status**: Resolved & Merged (PR #18)
* **Scope**: Migrated 100k treasury floor, V4 liquidity scale baselines, and target weights into `config.yaml` as part of the multi-tenant operator profile architecture. Retained historical baseline numbers reverently in configuration docs.

---

### Phase E: 0.2.x Triage Cycle (task 000664 — live-data audit of the Golden Path release)

> Filed from the 0.2.0 bug hunt (live Alchemy stack; ready-to-file issue bodies in
> `helix-specs/design/tasks/000664_i-was-so-happy-with-your/github-issues/ISSUES.md`).
> Fixes land as focused commits on `feature/000664-run-020-series-bug-hunt`.

| # | Defect | Component | Labels | Status |
|---|---|---|---|---|
| E1 | ETH price lookup falls back to hardcoded $2,500 in claim-fees gas valuation | `ingestion-worker/src/main.rs:1591-1619, 1996` | `bug`, `backend`, `data-integrity`, `patch-0.2.x` | Fixed (commit on 000664) |
| E2 | Tick Bracket chart draws fabricated Aerodrome 0→999 placeholder bracket (axis stretched, V4 bracket hidden) | `main.rs:3042-3048`, `frontend/src/App.tsx` | `bug`, `backend`, `frontend`, `data-integrity`, `patch-0.2.x` | Fixed (commit on 000664) |
| E3 | Deviation matrix legend shows black swatch for "Actual %" (per-Cell colors, no series fill) | `frontend/src/App.tsx:488-496` | `bug`, `frontend`, `patch-0.2.x` | Fixed (commit on 000664) |
| E4 | StatCard parseFloat strips "%" from Floor Health value (shows bare `106.3`) | `frontend/src/App.tsx:296, 665` | `bug`, `frontend`, `patch-0.2.x` | Fixed (commit on 000664) |
| E5 | 409 ALREADY_RECORDED (idempotent retry) shown as failure in Lite Cockpit; client drops HTTP status | `frontend/src/api/client.ts`, `LiteCockpit.tsx:148-170` | `bug`, `frontend`, `backend`, `patch-0.2.x` | Fixed (commit on 000664) |
| E6 | API client prefers machine error codes over human `detail` guidance | `frontend/src/api/client.ts:108` | `bug`, `frontend`, `patch-0.2.x` | Fixed (commit on 000664) |
| E7 | MCP formatters fabricate "~$0.005" gas + "$5.00" threshold text (zero-mock violations) | `main.rs:970-999`, `bin/mcp.rs:235-372` | `bug`, `backend`, `data-integrity`, `patch-0.2.x` | Fixed (commit on 000664) |
| E8 | Lite Cockpit renders EXECUTE buttons while WalletConnect is unconfigured | `LiteCockpit.tsx:259-305` | `bug`, `frontend`, `patch-0.2.x` | Fixed (commit on 000664) |
| E9 | Tick bracket subtitle hardcodes "(IN RANGE)" ignoring live `in_range` | `frontend/src/App.tsx:470` | `bug`, `frontend`, `patch-0.2.x` | Fixed (commit on 000664) |
| E10 | Residual mock fallback constants in dashboard text (floor 100000, bracket strings, $0.00) | `frontend/src/App.tsx:202, 285, 347` | `bug`, `frontend`, `data-integrity`, `patch-0.2.x` | Fixed (commit on 000664) |
| E11 | Harvest `confirmed_at` uses recording time instead of block timestamp (tax-date drift) | `main.rs:2172` | `bug`, `backend`, `data-integrity`, `patch-0.2.x` | Fixed (commit on 000664) |
| E12 | Unguarded `tracked_wallets[0]` / `tracked_pools.first()` indexing panics on empty config | `main.rs:1850, 2106` | `bug`, `backend`, `patch-0.2.x` | Fixed (commit on 000664) |
| E13 | Gas cost silently clamped to ≤$0.05; undocumented `+0.002` constant | `main.rs:2398-2403` | `bug`, `backend`, `data-integrity`, `patch-0.2.x` | Fixed (commit on 000664) |
| E14 | Makefile tmux targets hardcode macOS path (broken everywhere else) | `Makefile` | `bug`, `backend`, `patch-0.2.x` | Fixed (commit on 000664) |
| EA | MCP tools missing for all 0.2.0 Golden Path features (ledger, tax export, payload) | `bin/mcp.rs` | `enhancement`, `backend` | Deferred to 0.3.0 planning |
| EB | No persist/retry for failed directive-transition notifications | `main.rs:1288-1301` | `enhancement`, `backend` | Deferred (low) |
| EC | Externalize `v4_position_id` / Aerodrome pool fallbacks (config externalization) | `main.rs:1876-1881, 1935, 2493` | `enhancement`, `backend` | Deferred (low) |

---

### Phase D: Patch 0.1.3 Resolution & Triage

#### 9. [Issue #21: Live Uniswap V4 Fee Query via StateView](https://github.com/urmanac/nocknock/issues/21)
* **Component**: `ingestion-worker/src/main.rs:1176-1225, 1360-1435`
* **Labels**: `bug`, `backend`, `data-integrity`, `patch-0.1.x`
* **Status**: Resolved (Branch `fix/live-uniswap-v4-stateview-query`)
* **Defect**: PR #24 introduced a silent mock fallback when `0x1efeed33` (`getPositionLiquidity`) returned 64 hex chars instead of 288, masking the live query failure and falling back to static baseline numbers (~$0.06 vs ~$0.26 live in mobile app).
* **Fix**: Implemented live on-chain queries against official Base Uniswap V4 `StateView` contract (`0xa3c0c9b65bad0b08107aa264b0f3db444b867a71`):
  1. `getPositionInfo(poolId, owner, tickLower, tickUpper, salt)` (`0xdacf1d2f`) to extract `liquidity`, `feeGrowthInside0LastX128`, and `feeGrowthInside1LastX128`.
  2. `getFeeGrowthInside(poolId, tickLower, tickUpper)` (`0x53e9c1fb`) to extract current `feeGrowthInside0X128` and `feeGrowthInside1X128`.
  3. Built zero-dependency `U256` math (`wrapping_sub` and `mul_div_q128`) calculating exact uncollected fee tokens (matching live on-chain $0.120033 USDC + 5.4468 NOCK). Zero mock fallbacks.

#### 10. [Issue #22: Missing /v1/snapshot and /snapshot Route Aliases](https://github.com/urmanac/nocknock/issues/22)
* **Component**: `ingestion-worker/src/main.rs:504-513`
* **Labels**: `bug`, `backend`, `patch-0.1.x`
* **Status**: Resolved & Merged (commit `e9e7778`)
* **Fix**: Mounted `/v1/snapshot` and `/snapshot` as aliases for `get_latest_snapshot_handler` in the Axum router. Verified both return HTTP 200.

#### 11. Defect: Instantaneous Single-Point `strategy_counterfactual` Series in Latest Snapshot
* **Component**: `ingestion-worker/src/main.rs:1040-1065, 1720-1760`
* **Labels**: `bug`, `backend`, `frontend`, `data-integrity`
* **Status**: Resolved & Merged (commit `e9e7778`)
* **Fix**: Backfilled `strategy_counterfactual` series from historical database snapshots, returning full trajectory curve.

#### 12. Defect: `structural_channels` Schema Stub
* **Component**: `ingestion-worker/src/main.rs:1680-1720`
* **Labels**: `enhancement`, `backend`
* **Status**: Resolved & Merged (commit `e9e7778`); 0.1.4 additionally revived the panel with live SMA channels (kept — operator values it); v0.2.0 supersedes the deprecation idea by ADDING the Sovereign 60/40 Ledger panel instead of removing this one
* **Fix**: Implemented Bollinger Band regression channels (20-period SMA with 1σ and 2σ bands) calculated across historical price snapshots.

#### 13. [Issue #20: Frontend High CPU Usage in useFreshness Hook](https://github.com/urmanac/nocknock/issues/20)
* **Component**: `frontend/src/hooks/useFreshness.tsx`
* **Labels**: `bug`, `frontend`, `patch-0.1.x`
* **Status**: Resolved & Merged (PR #23)
* **Fix**: Removed dynamic freshness state dependencies from polling effect dependency array, added stateHash comparison refs, and eliminated continuous re-render churn. CPU dropped from ~117% to near-zero idle.

#### 14. Defect: False-Alarm Stale Warnings During Scheduled Ingestion Cycles
* **Component**: `frontend/src/hooks/useFreshness.tsx`, `frontend/src/App.tsx`, `ingestion-worker/src/main.rs:1880-1920`
* **Labels**: `bug`, `frontend`, `backend`, `patch-0.1.x`
* **Status**: Resolved & Merged (PR #27)
* **Defect**: Stale threshold was strictly equal to the 300s sync cycle interval, while ingestion RPC calls and transactions take a non-zero duration (~1s). At second 301, the UI flashed a stale warning banner during normal scheduled execution. Additionally, `/v1/snapshot/latest` sent `max-age=300`, causing browsers to trap stale snapshots.
* **Fix**: Measured live ingestion cycle duration (`sync_duration_ms`) and exposed on `/health` and `/v1/health`. Added 90s grace allowance with 3-tier sync status (`LIVE ON-CHAIN` green $\to$ `SYNC IN PROGRESS` cyan $\to$ `SYNC DELAYED` amber). Switched snapshot cache header to `no-cache, must-revalidate` and added `{ cache: 'no-cache' }` fetch headers.

#### 15. [Issue #61: Chart timeline collapses multi-hour downtime gaps into adjacent ticks without backfill or gap indicators](https://github.com/urmanac/nocknock/issues/61)
* **Component**: `frontend/src/App.tsx:105-136, 142-157, 162-175, 420`, `ingestion-worker/src/main.rs:2157-2240, 3850-3879`
* **Labels**: `bug`, `frontend`, `backend`, `data-integrity`
* **Status**: Open (Triaged)
* **Defect**: When the engine is offline across multi-hour spans (e.g. 12:38 AM to 7:58 PM), no snapshots are stored. On restart, the backend backfills only price candles for RSI, leaving the snapshot timeline un-backfilled. The frontend renders historical snapshots on an ordinal Recharts `<XAxis dataKey="time" />`, placing pre-downtime and post-downtime points directly adjacent to each other with standard 5-minute spacing, flattening a ~19.3-hour outage and drawing a deceptive continuous line across the gap.
* **Remediation**:
  1. Frontend: Detect timestamp deltas $> 2 \times \text{sync_window_seconds}$ and insert null data points with `connectNulls={false}` to visually break the line across downtime intervals, or switch to a continuous time scale with shaded downtime markers.
  2. Backend: Detect historical gaps on boot and provide telemetry gap metadata in `/v1/snapshot/historical`.

