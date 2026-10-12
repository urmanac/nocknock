# Implementation Tasks: Run 0.2.0 Series Bug Hunt and Triage Cycle

## Phase 1 — Bring-up & Setup
- [~] Provision `patch-0.2.x` GitHub label (green `#0e8a16`). NOTE: this sandbox has no GitHub token/`gh` — prepare ready-to-file issue bodies under this task's `github-issues/` folder instead; label + issue creation happens operator-side (or via an agent with credentials).
- [ ] Build and start the live stack: `SQLX_OFFLINE=true cargo build --release` in `ingestion-worker/`, run worker with `DATABASE_URL=postgresql://postgres:postgres@localhost:5432/nocknock`, start Vite dev server; verify `/health` and a live Alchemy snapshot on :8080 and dashboard on :3000.
- [ ] Capture baseline screenshots (desktop + `?view=lite`) including the deviation-matrix legend black square, into this task's `screenshots/` folder.

## Phase 2 — Audit & Triage (log before fixing)
- [ ] Audit WalletConnect bridge + session badge (`frontend/src/walletconnect.ts`, header badge in `App.tsx`) against live pairing; file issues for confirmed defects.
- [ ] Audit claim-fees payload engine (`ingestion-worker/src/main.rs:1843-2067`): calldata vs planner specs, pre-simulation failure paths, zero-claim handling; file issues.
- [ ] Audit record-harvest ledger endpoint (`main.rs:2068-2254`): receipt parsing, idempotency, USD valuation, retry semantics; file issues.
- [ ] Audit Koinly tax export (`main.rs:2255-2330`): CSV conformance, `income` tag, param validation; file issues.
- [ ] Audit 60/40 sovereign ledger derivation + panel (`main.rs` ~L3114, `App.tsx:539-586`): split math, zero-division/first-boot display, dial pct floor; file issues.
- [ ] Audit RSI regime + tax-reserve/overextension guardrails in worker; file issues.
- [ ] Audit directive-transition notifications (dup suppression, first-boot silence, webhook failure isolation); file issues.
- [ ] Audit Lite Cockpit + view switcher (`frontend/src/components/LiteCockpit.tsx`, header pills) for zero-mock and mobile correctness; file issues.
- [ ] Regression-sweep pre-0.2.0 surfaces: snapshot dedup, historical API cursors, freshness hook, client headers, MCP server parity with new endpoints; file issues.
- [ ] Contract/docs drift audit: `specs/core/api.yaml` vs handlers, README/CHANGELOG claims, Makefile portability (macOS-only `tmux` target); file issues.
- [ ] Re-check zero-mock invariant end-to-end (grep for residual fallback constants in worker + frontend); file issues.

## Phase 3 — Fixes (iterate per patch)
- [ ] Update `TRIAGE.md`: add `patch-0.2.x` label row and a new 0.2.x active-inventory section listing every finding.
- [ ] Fix the deviation-matrix legend: remove/repair the black "Actual %" swatch and extend the manual status legend to all four actual colors (incl. `fast_running` orange `#f97316`); verify with browser screenshot; PR referencing its issue.
- [ ] Fix each remaining confirmed defect as its own short-lived `fix/<slug>-issue-N` branch → PR → merge → close issue (respecting SemVer: bugs only, no features).
- [ ] Add bounded micro-tests (≤6 total) tied to fixed defects in the same PRs; keep existing suites green (`make test-brain`, `make test-worker`, `make test-frontend`).
- [ ] Update `CHANGELOG.md` per fix batch; cut patch releases (`v0.2.1`, ...) only with explicit operator authorization after CI green.

## Phase 4 — Close-out
- [ ] Final assessment: `TRIAGE.md` 0.2.x inventory statuses finalized (fixed / open / deferred-as-enhancement).
- [ ] Write operator summary (counts by component/severity, residual risks, audit-complete statement) to inform the 0.3.0 go/no-go decision.
- [ ] Verify no 0.3.0 feature work leaked into any PR from this task.
