# Implementation Tasks: Run 0.2.0 Series Bug Hunt and Triage Cycle

## Phase 1 — Bring-up & Setup
- [~] Provision `patch-0.2.x` GitHub label (green `#0e8a16`). NOTE: this sandbox has no GitHub token/`gh` — prepare ready-to-file issue bodies under this task's `github-issues/` folder instead; label + issue creation happens operator-side (or via an agent with credentials).
- [x] Build and start the live stack: worker built (`SQLX_OFFLINE=true cargo build --release`; NOTE: `cargo` lives at `~/.cargo/bin`, not on the non-interactive PATH) and running on :8080 with live Alchemy data; Vite on :3000 (`./node_modules/.bin/vite --port 3000 --host` after `npm install`). Verified `/health`, `/v1/snapshot/latest`, `/v1/execute/claim-fees/payload` (both actions simulate success live), `/v1/tax/export` (header-only CSV on empty ledger; 422 on bad format). On-chain verified NOCK token: `symbol()=NOCK`, `decimals()=16` — `NOCK_BASE` + 1e16 scaling correct.
- [x] Capture baseline screenshots into `screenshots/` — `01-deviation-matrix-legend-bug-baseline.png` (black "Actual %" swatch visible; Aerodrome 0→999 placeholder bar also visible) and `02-baseline-full-dashboard.png`.

## Phase 2 — Audit & Triage (log before fixing)
- [x] Audit WalletConnect bridge + session badge (`frontend/src/walletconnect.ts`, header badge in `App.tsx`) — zero-custody checks correct (account/sovereign match enforced, unconfigured degradation honest); live pairing untestable here (no project id) — hint state verified.
- [x] Audit claim-fees payload engine (`ingestion-worker/src/main.rs:1843-2067`): calldata golden-vector verified, live sim success on both actions; found E1 ($2,500 fallback), E12 (unguarded indexing), E13 (gas clamp).
- [x] Audit record-harvest ledger endpoint (`main.rs:2068-2254`): receipt parsing/idempotency/valuation verified live (422/404 paths honest); found E11 (NOW() vs block timestamp).
- [x] Audit Koinly tax export (`main.rs:2255-2330`): CSV+JSON verified with seeded rows (correct columns, `income` label, TxHash, Net Worth); empty-ledger header + 422 bad-format verified. No defects in the renderer itself.
- [x] Audit 60/40 sovereign ledger derivation + panel (`main.rs` ~L3114, `App.tsx:539-586`): split math tested in-repo; found dial `Math.max(2,…)` sliver + HWM=0 display notes (bundled into E10 fix set).
- [x] Audit RSI regime + tax-reserve/overextension guardrails in worker — Wilder RSI correct, guardrails sound, cold-start `rsi_14: None` verified live. No defects.
- [x] Audit directive-transition notifications — failure isolation correct; found EB (no retry/outbox) as enhancement.
- [x] Audit Lite Cockpit + view switcher (`frontend/src/components/LiteCockpit.tsx`, header pills): live-verified; found E8 (EXECUTE gating), E5 (409-as-failure).
- [x] Regression-sweep pre-0.2.0 surfaces: freshness hook, client headers, historical cursors/400/limit-clamp live-verified — 0.1.x fixes intact. MCP parity gap → EA.
- [x] Contract/docs drift audit: `specs/core/api.yaml` covers all new routes; found E14 (Makefile macOS paths).
- [x] Re-check zero-mock invariant end-to-end: found E1, E7 (MCP fabrications), E10 (frontend fallback constants), E2 (placeholder bracket).
- [x] Write ready-to-file issue bodies → `github-issues/ISSUES.md` (14 defects + 3 enhancements); registry → nocknock `TRIAGE.md` Phase E.

## Phase 3 — Fixes (iterate per patch)
- [x] Update `TRIAGE.md`: add `patch-0.2.x` label row and a new 0.2.x active-inventory section listing every finding.
- [x] Fix the deviation-matrix legend: remove the black "Actual %" swatch (explicit legend payload showing only "Target %"); manual status legend kept (3 real colors; `fast_running` orange confirmed dead code). Verified: `04-deviation-matrix-legend-fixed.png`.
- [x] Fix remaining confirmed defects E1, E2, E4-E14 as focused commits on this feature branch (Helix opens the single PR; see design.md PR-strategy adaptation). Committed `b33c4c1`; branch pushed.
- [x] Add bounded micro-tests tied to fixed defects: +2 worker (claim-simulation gas honesty, no synthetic bracket), +5 frontend (error detail/status mapping, record-harvest classifier). Suites green: worker 33, frontend 25, brain 3.
- [x] Update `CHANGELOG.md` ([Unreleased] → Fixed — 0.2.x Triage Cycle); patch releases (`v0.2.1`, ...) remain operator-authorized only.

## Phase 4 — Close-out
- [x] Final assessment: `TRIAGE.md` 0.2.x inventory statuses finalized (14 fixed, EA/EB/EC deferred as enhancements).
- [x] Write operator summary (`final-report.md`: counts by component/severity, residual risks incl. live WalletConnect pairing, audit-complete statement, 0.3.0 go recommendation).
- [x] Verify no 0.3.0 feature work leaked into any commit from this task (fixes + tests + docs only).
- [x] Write `pull_request.md` PR description with before/after screenshot evidence; feature branch `feature/000664-run-020-series-bug-hunt` pushed (commit `b33c4c1`).
