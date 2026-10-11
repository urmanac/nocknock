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
* **Defect**: The polling timer interval re-evaluates freshness only on the snapshot captured in the initial mount closure. It never invokes `fetchSnapshot()` or `refresh()`. After 300 seconds, the dashboard permanently displays `STALE`.
* **Fix**: Store the active snapshot in a ref and trigger background `refresh()` using `fetchIfFresh` (ETag conditional GET) on every poll interval.

#### 2. [Issue #8: Remove hardcoded mock constants, static labels, and unreactive colors](https://github.com/urmanac/nocknock/issues/8)
* **Component**: `frontend/src/App.tsx:205, 226, 266, 320, 328, 336, 343, 423`
* **Labels**: `bug`, `frontend`, `data-integrity`, `patch-0.1.x`
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
* **Defect**: Hardcoded `domain={[90000, 115000]}` clips chart data if total stack dips below 90k or grows above 115k.
* **Fix**: Use dynamic domain padding relative to `dataMin` and `dataMax` while keeping the 100k floor line visible.

---

### Phase B: Backend Data Integrity & History (`v0.1.2` Patch Target)

#### 4. [Issue #10: Snapshot deduplication overwrites `generated_at` and destroys historical time-series](https://github.com/urmanac/nocknock/issues/10)
* **Component**: `ingestion-worker/src/main.rs:1385-1406`
* **Labels**: `bug`, `backend`, `data-integrity`, `patch-0.1.x`
* **Defect**: When `state_hash` is unchanged, `store_snapshot` updates `generated_at` on the previous row. This erases the duration for which that state existed and prevents the accumulation of historical rows in steady markets.
* **Fix**: Maintain original snapshot start timestamps and record periodic discrete snapshots at regular interval boundaries.

#### 5. [Issue #11: Prevent silent mock fallbacks when primary RPC queries fail](https://github.com/urmanac/nocknock/issues/11)
* **Component**: `ingestion-worker/src/main.rs:920-1005`
* **Labels**: `bug`, `backend`, `data-integrity`, `patch-0.1.x`
* **Defect**: If Alchemy RPC calls fail, local variables silently fall back to `96004.52`, `0.2316`, `81297590130616`, generating synthetic snapshots with fresh timestamps.
* **Fix**: Fail the ingestion cycle or mark snapshot metadata as `degraded: true` rather than publishing synthetic numbers.

#### 6. [Issue #5: Dynamic on-chain fee calculation for Uniswap V4 uncollected yield](https://github.com/urmanac/nocknock/issues/5)
* **Component**: `ingestion-worker/src/main.rs:1043-1048`
* **Labels**: `bug`, `backend`, `data-integrity`, `patch-0.1.x`
* **Defect**: Uniswap V4 uncollected fees are hardcoded to `0.0` following the auto-compounding transaction (`0x6e1b...ddf2`), hiding newly accreting yield from the dashboard.
* **Fix**: Compute live uncollected fees via `feeGrowthInsideLastX128` delta or simulation, and display an auto-compounded milestone badge (`0.470 USDC` + `5.27 NOCK`) recognizing past earnings.

---

### Phase C: Major Features & Profile Parameterization (`v0.2.0` Minor Target)

#### 7. [Issue #6: Externalize static portfolio baseline numbers into operator profile config](https://github.com/urmanac/nocknock/issues/6)
* **Component**: `ingestion-worker/src/main.rs:30-60, 1165-1195`, `specs/core/snapshot.yaml`
* **Labels**: `enhancement`, `backend`
* **Scope**: Migrate 100k treasury floor, V4 liquidity scale factors, and target weights into `config.yaml` as part of the multi-tenant operator profile architecture.
