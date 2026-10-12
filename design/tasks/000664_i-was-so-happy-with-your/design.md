# Design: Run 0.2.0 Series Bug Hunt and Triage Cycle

## Approach

Reuse the proven 0.1.x triage loop (`TRIAGE.md`), extended for `patch-0.2.x`:

```mermaid
graph TD
    A[Bring up live stack] --> B[Module-by-module audit]
    B --> C{Defect found?}
    C -- Yes --> D[GitHub issue w/ RCA + labels]
    C -- No --> E[Next module]
    D --> F[Fix branch + PR + micro-test]
    F --> G[Merge to main, close issue]
    G --> H[TRIAGE.md inventory update]
    E --> B
    H --> I{All modules audited?}
    I -- No --> B
    I -- Yes --> J[Final assessment report]
```

- **Audit first, fix second.** Every defect is filed on GitHub before any fix branch is cut (pre-branching triage rule).
- **Live verification.** The sandbox has Postgres ready and the Alchemy key wired into `ingestion-worker/config.yaml` — run the real worker and verify findings against live on-chain snapshots instead of guessing.
- **Issue = handoff artifact.** Issues are written so a cold agent (Gemini/Antigravity pattern) could implement the fix: exact `file:line`, repro, evidence, remediation plan.

## Confirmed Known Issue — Deviation Matrix Legend

Root cause (verified by code reading AND live browser evidence, `screenshots/01-*.png`):

- `App.tsx:488` — `<Bar dataKey="actual" name="Actual %">` has **no series-level `fill`**; colors come only from per-data `<Cell>` children (`App.tsx:490-494`). Recharts' default `<Legend>` (`App.tsx:496`) derives the swatch color from the series-level fill, which is unset → renders **black**.
- The worker's `calc_status` (`main.rs:3052-3061`) only ever emits `on_target` / `under_funded` / `over_funded` — **never `fast_running`** — so the orange `#f97316` branch in the Cell mapping is dead code and only 3 actual colors exist (matches the operator's observation). Fix: remove the "Actual %" entry from the auto legend; the manual status legend below the chart already covers the 3 real colors.

## Implementation Notes (audit results, live-verified)

- **Bug haul (14 defects, 3 enhancements)** — full ready-to-file bodies in `github-issues/ISSUES.md`; registry in nocknock `TRIAGE.md` §Phase E. Headliners: `fetch_eth_price_usd` still hardcodes the $2,500 fallback (main.rs:1618, zero-mock violation in claim-fees gas valuation); Tick Bracket chart draws a fabricated Aerodrome 0→999 placeholder bracket (main.rs:3042-3048) that stretches the axis and hides the real V4 bracket; MCP text formatters fabricate "~$0.005" gas + "$5.00" thresholds (main.rs:992-996, bin/mcp.rs); StatCard parseFloat strips "%" from Floor Health (shows "106.3"); Lite Cockpit shows EXECUTE buttons while WalletConnect is unconfigured; ALREADY_RECORDED (409) rendered as failure on idempotent retry.
- **Live test results**: claim-fees payload simulates both actions successfully with real amounts; Koinly CSV/JSON verified with seeded rows (correct columns, `income` label, TxHash, Net Worth) — reviewer's Koinly priority covered; record-harvest error paths (422/404) honest; historical `interval` 400 + limit clamp intact; NOCK token on-chain `symbol()=NOCK, decimals()=16`.
- **Not bugs (verified)**: bracket strings `0.01237/2.02783` ARE the tick-derived values at `{:.5}` (test asserts the round-trip — the doc comment's "was hardcoded" wording is misleading, not the code); `nock_floor_delta` has no commas so `parseInt` is safe; ledger aggregate refresh is per-ingestion-cycle (5-min staleness is by design).
- **PR strategy adaptation**: Helix opens ONE PR for `feature/000664-run-020-series-bug-hunt`, so per-defect branches/PRs are replaced by focused per-defect commits on this feature branch (the TRIAGE.md one-defect-one-PR protocol applies to direct-to-GitHub flows; here the platform owns PR creation).
- **Gotchas**: `cargo` at `~/.cargo/bin` (not on non-interactive PATH); `make tmux` unusable on Linux (macOS paths in Makefile — logged as defect); Vite needs `npm install` then `./node_modules/.bin/vite`; Alchemy key reads from `ingestion-worker/config.yaml` (`awk '/alchemy_api_keys:/ {getline; print $2}'`).

## Audit Surface Map

| Area | Files | Focus |
|---|---|---|
| WalletConnect bridge | `frontend/src/walletconnect.ts` (92 ln), header session badge in `App.tsx` | Session reconnection, chain/account mismatch rejection, unconfigured-project degradation, `eth_sendTransaction` params (hex value, gas absence) |
| Claim-fees payload engine | `ingestion-worker/src/main.rs:1843-2067` | Calldata correctness vs @uniswap/v4-sdk planner & Aerodrome ABI, `eth_call` pre-simulation error paths, zero-claim handling, `msg.sender` invariant |
| Record-harvest ledger | `main.rs:2068-2254`, `harvest_events` table | Receipt parsing (ERC-20 Transfer logs), idempotency (`ON CONFLICT (tx_hash, token_address)`), USD valuation (spot price at record time), pending-broadcast retry semantics |
| Koinly tax export | `main.rs:2255-2330` | CSV spec conformance, `income` tagging, empty-ledger header, `format` param validation |
| 60/40 sovereign ledger | `main.rs` `sovereign_ledger` derivation (~L3114), `App.tsx:539-586` | Split math vs `tax_reserve_ratio`, first-boot zero-division (`hwm=0`, `total=0` → `0.00%` display instead of "—"), dial pct floor (`Math.max(2, …)` shows 2% bar at zero) |
| RSI regime + guardrails | worker regime/directive logic | Wilder RSI correctness over history series, RSI≥70 suppression, tax-guardrail condition |
| Transition notifications | worker notify module | Duplicate-transition suppression, silent first-boot, webhook failures don't break ingestion |
| Lite Cockpit + view switcher | `frontend/src/components/LiteCockpit.tsx` (386 ln), `App.tsx` header | Live data (zero-mock), execution button states, mobile layout, deep-link pairing |
| Regression: snapshot/history/freshness | `useFreshness.tsx`, `client.ts`, worker snapshot+historical handlers | 0.1.x fixes still hold (dedup, cursors, 400 on `interval`, no-cache headers) |
| Regression: MCP server | `ingestion-worker/src/bin/mcp.rs` (377 ln) | Parity with new endpoints (`claim-fees`, `record-harvest`, `tax/export`), stdio robustness |
| Contract & docs drift | `specs/core/api.yaml`, `README.md`, `CHANGELOG.md`, `.mcp.json`, `Makefile` | Claims vs implementation. Known: `Makefile` tmux targets hardcode macOS path `/Users/yebyen/w/nocknock/...` — broken on Linux; audit for more |

Severity classification reuses `TRIAGE.md`: anything that presents non-live numbers as on-chain truth is `data-integrity` high-severity.

## Environment (Linux sandbox)

- `make tmux` **cannot** be used as-is (macOS path in Makefile — log it, don't fix silently). Start servers directly:
  - Worker: `cd ingestion-worker && SQLX_OFFLINE=true cargo build --release` then `DATABASE_URL="postgresql://postgres:postgres@localhost:5432/nocknock" ./target/release/nocknock-ingestion-worker` (port 8080; reads Alchemy key from wired `config.yaml`).
  - Frontend: `cd frontend && npm run dev` (port 3000).
- Browser evidence: `list_windows` → `focus_window` → `save_screenshot` into this task's `screenshots/` folder for issue attachments.

## Triage Protocol

- New GitHub label `patch-0.2.x` (green, mirrors `patch-0.1.x` `#0e8a16`).
- Issue body template mirrors 0.1.x entries: **Component / Labels / Defect / Evidence / Fix**.
- All `gh` invocations prefixed `unset GITHUB_TOKEN`.
- Fix branches: `fix/<short-slug>-issue-N` off `main`; one defect (or one tightly-coupled cluster) per PR; PRs merged with `gh pr merge <N> --merge`; issues closed only after merge.

## Coordination with the 0.3.0 Planning Branch

- A concurrent branch is planning 0.3.0 features. To avoid rebase collisions:
  - This task only touches **defect surfaces** (bug fixes), never feature work.
  - Anything feature-shaped → `enhancement` issue for the 0.3.0 branch to consume.
  - 0.3.0 feature implementation starts only after this task's final assessment lands (operator gates it).

## Micro-Test Policy

- One test per non-trivially-visual fixed defect, added in the same PR as its fix (vitest for frontend logic, Rust `#[test]` in the existing `mod tests` for worker math/parsing).
- Hard cap ~6 new tests; visual/UI fixes are verified by screenshot, not snapshot tests.

## Patterns & Learnings (for implementing agents)

- This repo runs a **single-file worker** (`ingestion-worker/src/main.rs`, ~4,100 lines) with unit tests in a bottom `mod tests` — search before adding helpers; most utilities already exist there.
- Frontend is one `App.tsx` (~844 lines) + small modules; styling is inline `styles` objects; charts are Recharts 2.15 (legend icon colors come from series-level `fill`, NOT per-`Cell` fills — the root of the known legend bug).
- Frontend tests live in `frontend/src/api/freshness.contract.test.ts` and gracefully skip live API tests when `localhost:8080` is unreachable (500ms AbortController in `beforeAll`) — keep that pattern.
- Worker builds require `SQLX_OFFLINE=true` (query cache in `ingestion-worker/.sqlx/`).
- WASM brain is deterministic-only: never introduce clocks/network there; needs `-C link-arg=--allow-undefined`.
- GitHub credentials: always `unset GITHUB_TOKEN` before `gh`.
- Patch history shows the recurring defect classes in this codebase: silent mock fallbacks, hardcoded constants, stale-closure polling bugs, and history/series truncation — grep for these first.
