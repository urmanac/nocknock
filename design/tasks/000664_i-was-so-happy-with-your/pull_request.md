# Fix 14 defects from the 0.2.0 series bug hunt and triage cycle

## Summary
Result of re-running the proven 0.1.x find → log → fix triage process against the
v0.2.0 (Golden Path) release, this time with a live Alchemy connection: the worker ran
against Base mainnet for the whole audit, every finding was verified against live
on-chain data or live HTTP responses, and all confirmed defects were fixed.
Registry: `TRIAGE.md` §Phase E · full assessment: `final-report.md` in the task folder.

- **Zero-mock (worker)**: `fetch_eth_price_usd` no longer falls back to a hardcoded
  $2,500 in claim-fees gas valuation (returns `Option`, degrades honestly); the
  synthetic Aerodrome 0→999 tick bracket is removed from `tick_bracket_hypervisor` (it
  stretched the chart axis and hid the real V4 bracket); MCP formatters (`nocknock-mcp`
  stdio + `/v1/mcp`) surface the live gas break-even ("unavailable" when absent)
  instead of a fabricated ~$0.005 and a stale $5.00 trigger; gas break-even is no
  longer silently clamped at $0.05; golden-path handlers return a clean 503 on empty
  wallet config instead of panicking.
- **Tax honesty (worker)**: harvest `confirmed_at` now comes from the on-chain block
  timestamp instead of the recording wall clock, keeping Koinly export dates correct
  across pending-broadcast retries.
- **Frontend**: deviation-matrix legend no longer shows the phantom black "Actual %"
  swatch (explicit legend payload; the per-status color key below the chart documents
  the real cell colors); Floor Health keeps its "%" unit; the tick-bracket subtitle
  derives IN/OUT OF RANGE from live `in_range`; hardcoded fallback constants (floor
  100000, bracket strings, $0.00 price) replaced with honest placeholders; 60/40 dials
  no longer draw a phantom sliver at zero; EXECUTE actions are hidden while
  WalletConnect is unconfigured; 409 ALREADY_RECORDED is treated as idempotent success;
  API errors surface the backend's human `detail` with the HTTP status preserved.
- **Build**: Makefile tmux targets use `$(CURDIR)` instead of a hardcoded macOS path.
- **Enhancements filed for 0.3.0** (not implemented here): MCP tools for the 0.2.0
  Golden Path surface, notification retry/outbox, config externalization of
  `v4_position_id`/pool fallbacks.

## Testing
- Suites: brain 3/3 (`make test-brain`), worker 33/33 (`cargo test`,
  `SQLX_OFFLINE=true`), frontend 25/25 (`npm run build` + `vitest run`) — +2 worker and
  +5 frontend micro-tests tied to the fixed defects.
- Live Base mainnet verification: claim-fees payload simulates both actions with real
  amounts and live gas ($0.0021/$0.0020, net $0.0359); Koinly CSV/JSON verified with
  seeded rows (correct columns, `income` label, TxHash, Net Worth) then removed;
  record-harvest error paths (422/404) honest; historical API regressions intact
  (`interval` → 400, limit clamp); NOCK token verified on-chain (`symbol()=NOCK`,
  `decimals()=16`); MCP stdio output shows live gas $0.0042.
- Browser evidence:
  ![Legend bug baseline](https://github.com/urmanac/nocknock/raw/helix-specs/design/tasks/000664_i-was-so-happy-with-your/screenshots/01-deviation-matrix-legend-bug-baseline.png)
  ![Legend fixed + real V4 bracket](https://github.com/urmanac/nocknock/raw/helix-specs/design/tasks/000664_i-was-so-happy-with-your/screenshots/04-deviation-matrix-legend-fixed.png)

Known limitation: live WalletConnect QR pairing could not be exercised in this sandbox
(no project ID) — the code is reviewed and contract-covered; one end-to-end pairing
playtest with a real wallet is recommended next.
