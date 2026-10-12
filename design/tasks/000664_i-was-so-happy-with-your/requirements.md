# Requirements: Run 0.2.0 Series Bug Hunt and Triage Cycle

## Background

The `v0.1.x` patch series (0.1.1 → 0.1.4) used a proven workflow: systematic code audit → GitHub issue triage (file:line RCA, repro steps, labels) → continuous patch fixes via short-lived PR branches → final assessment. Issue logging doubled as an agent-to-agent handoff medium (issues were passed to the Gemini/Antigravity agent, which implemented fixes from them).

`v0.2.0` (Golden Path) is now shipped on `main`. It introduced a large new surface: WalletConnect execution, claim-fees payload engine, receipt-verified harvest ledger, 60/40 sovereign ledger, Koinly tax export, RSI regime guardrails, directive-transition notifications, and the Lite Cockpit. This task repeats the 0.1.x process against that surface, using the now-live Alchemy connection for real on-chain verification.

## User Stories

### US1 — Systematic re-run of the bug-triage process
As the **operator**, I want a full audit of the v0.2.0 codebase (new Golden Path surface plus regression checks on pre-0.2.0 surfaces) so that every defect is found and logged before any 0.3.0 feature implementation begins.

**Acceptance Criteria**
- [ ] Every 0.2.0 feature area is audited (see design.md surface map): WalletConnect bridge, claim-fees payload engine, record-harvest ledger, Koinly export, 60/40 sovereign ledger, RSI/overextension + tax-reserve guardrails, transition notifications, Lite Cockpit, view switcher.
- [ ] Regression sweep of pre-0.2.0 surfaces (snapshot ingestion, derived metrics, historical API, freshness hook, MCP server) is performed — the worker `main.rs` is ~4,100 lines and much of it predates 0.2.0.
- [ ] Zero-mock invariant is explicitly re-checked: no silent fallbacks, no synthetic numbers presented as live on-chain truth.
- [ ] Live Alchemy data is used to verify actual behavior (start worker + frontend locally against the wired `config.yaml` Alchemy key and Postgres).

### US2 — GitHub issues as the triage record and agent handoff
As the **operator**, I want every confirmed defect logged on GitHub using the `TRIAGE.md` taxonomy so that another agent (e.g. Gemini/Antigravity) can implement fixes cold, exactly as worked in the 0.1.x series.

**Acceptance Criteria**
- [ ] Each confirmed defect has a GitHub issue with: root cause (`file:line`), reproduction steps, on-chain/HTTP evidence where applicable, and a remediation plan.
- [ ] Labels applied per taxonomy: `bug` (or `enhancement` for feature-shaped findings), `frontend`/`backend`, `data-integrity` where relevant, `triage`, plus a new `patch-0.2.x` label.
- [ ] `TRIAGE.md` gains a `patch-0.2.x` label row and a new active-inventory section for the 0.2.x cycle.
- [ ] GitHub commands follow the `unset GITHUB_TOKEN` elevated-credential protocol.

### US3 — Fix the deviation-matrix legend UX bug
As the **operator**, I want the "Portfolio Deviation Matrix (Target vs Actual)" legend fixed: the "Actual %" legend swatch renders black even though actual bars use status colors (green/amber/red/orange), none of which is black.

**Acceptance Criteria**
- [ ] No black swatch appears anywhere in that panel's legend.
- [ ] Every color the `actual` bars can use has a matching, correctly-named legend entry (the current manual legend omits the orange `fast_running` color).
- [ ] Verified visually against live data in the browser (desktop and `?view=lite` unaffected).

### US4 — Fixes ship as continuous 0.2.x patches, no feature creep
As the **operator**, I want confirmed bugs fixed in small PRs landing on `main` and released as `v0.2.1`, `v0.2.2`, ... strictly per SemVer cadence, with zero 0.3.0 features bundled.

**Acceptance Criteria**
- [ ] Each fix is a short-lived branch off `main`, lands via PR (never direct push to `main`).
- [ ] Each fix PR references its GitHub issue; issues are closed only when the fix is merged.
- [ ] Feature-shaped findings are logged as `enhancement` issues and deferred to the 0.3.0 planning branch — not implemented here.
- [ ] Pre-flight checks pass before every PR: `make test-brain`, `make test-worker`, `make test-frontend`.
- [ ] Release tags are only cut with explicit operator authorization (per `OPERATOR_GUIDANCE.md`), and only after CI is green and `CHANGELOG.md` is populated.

### US5 — Final bug assessment report
As the **operator**, I want a closing assessment of everything found (logged, fixed, deferred) so I can decide when to green-light 0.3.0 implementation without colliding with bug-fix work.

**Acceptance Criteria**
- [ ] `TRIAGE.md` 0.2.x section lists every finding with status (fixed / open / deferred-as-enhancement).
- [ ] A concise summary is provided: counts by component and severity, risk areas that remain, and an explicit statement that the audit is complete.
- [ ] No implementation of 0.3.0 features occurs in this task.

### US6 — Micro-tests where they earn their keep
As the **operator**, I want a few targeted micro-tests added for fixed defects so regressions stay fixed — without over-engineering the suite.

**Acceptance Criteria**
- [ ] At most ~4–6 new tests total across frontend (vitest) and worker (Rust unit tests in `mod tests`), each tied to a fixed defect.
- [ ] Existing suites stay green: 18 worker tests, 20 frontend contract/logic tests, 3 brain tests.

## Non-Goals

- No new protocol features (those belong to the concurrent 0.3.0 planning branch).
- No native Android app, multi-pool ingestion, Coinbase connector, or Urbit desk work.
- No framework/dependency major upgrades (React 19, Recharts 3, sqlx 0.9, etc.).
- No redesign of the dashboard beyond the specific legend fix in US3.

## Open Questions

1. **Who implements the fixes?** Assumed: this agent runs the find → log → fix loop directly (you said "go forth, find bugs, and iterate"). If you'd rather hand the logged issues to the Gemini/Antigravity agent as in 0.1.0, say so at review time — the spec works either way, only the fix-phase owner changes.
2. **Tag cadence.** Per `OPERATOR_GUIDANCE.md`, release tags need explicit authorization. Assumed: fix PRs merge continuously, but tags (`v0.2.1`, ...) are cut only when you authorize — likely batched rather than one per merged fix. Confirm preference.
3. **Docs/contract drift.** Should mismatches between docs/specs and implementation (e.g. `specs/core/api.yaml` vs actual handlers, README claims vs reality) be logged as bugs too? The 0.1.x cycle treated `api.yaml` drift as in-scope. Assumed: yes, in scope.
4. **Legend fix shape.** Simplest correct fix is to drop the black "Actual %" swatch from the auto-generated chart legend and extend the manual status legend below it to all four colors (incl. `fast_running` orange). Alternative: render per-status swatches inside the chart legend itself. Assumed: the first (simpler) option unless you prefer the second.
5. **Live-polling load.** The sandbox has the Alchemy key wired and Postgres ready; repeated ingestion cycles will hit Alchemy at the default 5-minute cadence. Any objection to running the worker continuously here for testing, or should we trigger cycles manually to limit RPC usage?
