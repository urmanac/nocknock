# Fix 0.2.0 triage defects: residual mock fallbacks, honest harvest dates, dashboard repairs

## Summary
Result of the 0.2.0-series bug hunt (task 000664): the worker ran live against Base
mainnet via Alchemy for the whole audit; every defect was verified against live data and
fixed in focused commits. 14 defects fixed, 3 enhancements filed for 0.3.0, 5 micro-tests
added. All suites green (worker 33, frontend 25, brain 3). Full assessment:
`final-report.md` in the task folder; issue registry in `nocknock/TRIAGE.md` §Phase E.

## Changes
- **Zero-mock (worker)**: `fetch_eth_price_usd` returns `Option` (no more hardcoded
  $2,500 fallback in claim-fees gas valuation); synthetic Aerodrome 0→999 tick bracket
  removed from `tick_bracket_hypervisor`; MCP formatters (`nocknock-mcp` stdio +
  `/v1/mcp`) show the live gas break-even ("unavailable" when absent) instead of a
  fabricated ~$0.005 and a stale $5.00 trigger; gas break-even no longer clamped at
  $0.05; golden-path handlers return clean 503 on empty wallet config instead of panicking.
- **Tax honesty (worker)**: harvest `confirmed_at` now comes from the on-chain block
  timestamp (`eth_getBlockByNumber`) instead of the recording wall clock, so the Koinly
  export keeps correct dates across pending-broadcast retries.
- **Frontend**: deviation-matrix legend no longer shows the phantom black "Actual %"
  swatch (explicit legend payload; per-status color key below the chart); Floor Health
  keeps its "%" unit; tick-bracket subtitle derives IN/OUT OF RANGE from live `in_range`;
  hardcoded fallback constants (floor 100000, bracket strings, $0.00 price) replaced with
  honest placeholders; 60/40 dials no longer draw a sliver at zero; EXECUTE actions hidden
  while WalletConnect is unconfigured; 409 ALREADY_RECORDED treated as idempotent success;
  API errors surface the backend's human `detail` with HTTP status preserved.
- **Build**: Makefile tmux targets use `$(CURDIR)` instead of a hardcoded macOS path.
- **Tests**: +2 worker (claim-simulation gas honesty, no synthetic bracket), +5 frontend
  (error mapping detail/status, record-harvest classifier). CHANGELOG updated.

## Testing
- `make test-brain` (3/3), `cargo test` in `ingestion-worker` with `SQLX_OFFLINE=true`
  (33/33), `npm run build` + `vitest run` in `frontend` (25/25).
- Live verification against Base mainnet: claim-fees payload simulates both actions with
  real amounts and live gas; Koinly CSV/JSON verified with seeded rows (then removed);
  record-harvest error paths (422/404) honest; MCP stdio output shows live gas $0.0042.
- Browser evidence: `screenshots/01-deviation-matrix-legend-bug-baseline.png` (black
  swatch + fabricated Aerodrome bar) → `04-deviation-matrix-legend-fixed.png` (fixed) →
  `05-after-full-dashboard.png`.

## Screenshots
![Legend bug baseline](https://github.com/urmanac/nocknock/raw/helix-specs/design/tasks/000664_i-was-so-happy-with-your/screenshots/01-deviation-matrix-legend-bug-baseline.png)
![Legend fixed + real V4 bracket](https://github.com/urmanac/nocknock/raw/helix-specs/design/tasks/000664_i-was-so-happy-with-your/screenshots/04-deviation-matrix-legend-fixed.png)
