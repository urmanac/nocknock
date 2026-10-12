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
| **`enhancement`** | `#a2eeef` | New feature or architectural expansion | Features deferred to `v0.2.0` |
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

### Phase D: Incoming Bug Registry for Nemotron (`v0.1.3` / `v0.2.0` Triage)

#### 9. Defect: Uniswap V4 Synthetic Yield Discrepancy vs On-Chain Reality
* **Component**: `ingestion-worker/src/main.rs:1370-1376, 1435-1440`
* **Labels**: `bug`, `backend`, `data-integrity`, `patch-0.1.x`
* **Status**: Open / Discovered
* **Defect**: 
  - Operator observation: *"I still don't see any claimable Uniswap V4 yield."*
  - The worker calculates Uniswap V4 yield via `profile.v4_fee_baseline_usdc * v4_scale`, multiplying a static baseline by current liquidity ratio.
  - On Base mainnet, if trading volume in the Uniswap V4 pool has been minimal, no fees have actually accrued on-chain since the last compound. The Uniswap mobile app reflects reality ($0.00 claimable), whereas the dashboard displays synthetic yield (~$0.08).
  - Furthermore, `fee_growth_inside_0_x128` and `fee_growth_inside_1_x128` in `concentrated_liquidity` are hardcoded to `"0"`.
* **Fix**: Query live uncollected fees from Uniswap V4 on Base via `StateView` or position `feeGrowthInside` against `feeGrowthGlobal` instead of baseline scaling. If uncollected fees are 0 on-chain, display $0.00 accurately.

#### 10. Defect: Missing `/v1/snapshot` Route Alias (Returns 404 Not Found)
* **Component**: `ingestion-worker/src/main.rs:501-525`
* **Labels**: `bug`, `backend`, `patch-0.1.x`
* **Status**: Open / Discovered
* **Defect**: Calling `GET /v1/snapshot` or `GET /snapshot` returns HTTP 404 Not Found. Only `/v1/snapshot/latest`, `/v1/snapshot/historical`, and `/v1/snapshot/metadata` are registered. Standard REST tooling, SDKs, and agents naturally probe `/v1/snapshot`.
* **Fix**: Register `/v1/snapshot` and `/snapshot` as aliases for `get_latest_snapshot_handler` in the Axum router.

#### 11. Defect: Instantaneous Single-Point `strategy_counterfactual` Series in Latest Snapshot
* **Component**: `ingestion-worker/src/main.rs:1682-1686`, `frontend/src/App.tsx:395-430`
* **Labels**: `bug`, `backend`, `frontend`, `data-integrity`
* **Status**: Open / Discovered
* **Defect**: `compute_derived_metrics` creates a `strategy_counterfactual` series containing only `vec![now_str]`. Without historical database backfill in memory, the Recharts LineChart cannot render a meaningful trajectory line with a single discrete point.
* **Fix**: Backfill `strategy_counterfactual` from historical database snapshots or cache recent trajectory points in memory across worker ticks.

#### 12. Defect: `structural_channels` Schema Stub
* **Component**: `ingestion-worker/src/main.rs:1681`, `specs/core/snapshot.yaml:282`
* **Labels**: `enhancement`, `backend`
* **Status**: Open / Discovered
* **Defect**: `structural_channels` is permanently returned as `channels: vec![]`, leaving the corresponding visualization panel in the dashboard dormant.
* **Fix**: Implement structural channel calculation (Bollinger/volatility channels around NOCK price) or deprecate panel in favor of the planned `v0.2.0` dual-dial gauges.
