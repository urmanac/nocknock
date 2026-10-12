feat: v0.2.0 Golden Path - WalletConnect claim execution, 60/40 ledger, tax guardrails

## Summary
Implements the v0.2.0 "Golden Path": the operator executes fee harvests for both
LPs (Aerodrome + Uniswap V4) from the web cockpit by pairing the Uniswap mobile
app over WalletConnect — zero custody, worker-built pre-simulated transactions,
receipt-verified bookkeeping. No mock numbers anywhere: every surface fails
explicitly when on-chain data is unavailable.

- **Golden Path payload engine** (`ingestion-worker`): `GET /v1/execute/claim-fees/payload`
  builds unsigned Aerodrome `claimFees()` and Uniswap V4 PositionManager collect
  transactions (DECREASE_LIQUIDITY with 0 liquidity + TAKE_PAIR) from the live
  snapshot, with pre-dispatch `eth_call` simulation, per-action gas estimates, and
  USD valuation. Encoding verified byte-for-byte against the official
  `@uniswap/v4-sdk` planner output and accepted by live Base mainnet simulation
  (golden-vector tests run in CI). Two sequential transactions, no multicall —
  Aerodrome `claimFees()` is msg.sender-relative and must not be proxied.
- **Receipt-verified harvest ledger**: `harvest_events` table + `POST /v1/execute/record-harvest`
  — verifies the Base receipt and records ERC-20 Transfers (USDC/NOCK) paid to the
  sovereign wallet; idempotent (409 on duplicate); 404 RECEIPT_NOT_FOUND doubles as
  the Uniswap-app pending-broadcast signal.
- **Virtual 60/40 sub-ledger**: `sovereign_ledger` in derived metrics driven by
  `operator_profile.tax_reserve_ratio` (default 0.40); cumulative receipt-verified
  harvests split into a protected virtual tax reserve and growth dry powder. Tax
  guardrail suppresses dip-deployment advice (`SWEEP TO STABLE / ACCUMULATE DRY
  POWDER (Tax Guardrail)`) when the reserve exceeds the stable stack; otherwise the
  deployable amount and protected reserve are surfaced explicitly.
- **Koinly tax export**: `GET /v1/tax/export?format=koinly` (CSV default, JSON
  available) — fee claims tagged `income` with Base tx hashes; tie-out tests prove
  ledger ↔ export ↔ split consistency.
- **Anti-top-buying guardrails**: Wilder RSI(14) over the per-snapshot NOCK spot
  series; `regime` + `rsi_14` in GlobalStatus; `SWEEP TO STABLE / ACCUMULATE DRY
  POWDER (Overextension Cooldown)` directive suppresses buy advice at RSI >= 70.
- **Notifications**: Telegram, Pushover, and generic webhook adapters on
  directive transitions (transition-only dispatch, first boot silent, delivery
  failures logged never fatal, optional cockpit deep link) configured under
  `notifications` in config.yaml.
- **Web cockpit** (`frontend`): WalletConnect pairing via
  `@walletconnect/ethereum-provider` (official WC v2 SDK, QR modal + deep links;
  wrong-wallet proposals refused before signing; requires
  `VITE_WALLETCONNECT_PROJECT_ID` and degrades to an explicit setup hint without
  it), `/?view=lite` mobile cockpit with the EXECUTE BATCH HARVEST flow and
  pending-broadcast handling, desktop WalletConnect session badge, and the
  "Sovereign 60/40 Ledger" panel with dual dials plus live P/L vs high-water mark
  — all from locally recorded data, zero external APIs.
- **Bug fixes found during live verification**: snapshot_id `VARCHAR(26)` schema
  drift (migration-provisioned databases could not persist snapshots; docker
  init-db.sql path used UUID — classic drift) fixed via a new ALTER migration, and
  sqlx migration-version collision (`YYYYMMDD_NNNNNN` file names collide —
  renamed to unique 14-digit versions).
- **Docs**: CHANGELOG 0.2.0 section (landed vs deferred), HANDOFF Golden Path
  status, OPERATOR_GUIDANCE WalletConnect pairing runbook, TRIAGE panel decision,
  ARCHITECTURE code-reuse gap note.

Deferred to 0.2.1 (documented in CHANGELOG and the spec task): Android app (web
QR flow ships instead — operator decision), multicall aggregation (rejected on
msg.sender semantics), multi-pool ingestion loop, Coinbase staked-ETH connector
(needs a CDP key + JWT signing).

## Testing
- `make test-brain` 3/3 · `make test-worker` 31/31 · `make test-frontend` 20/20
  unit + 20/20 live contract · `make test-mcp` ALL PASSED (STDIO + HTTP with live
  on-chain data) · `make dist` packages all 4 release artifacts.
- Live daemon verification with the operator's Alchemy key: real ingestion cycle
  (snapshot persisted in 294 ms), real snapshot served, both claim actions
  simulate "success" (Aerodrome $0.0595, Uniswap V4 $0.2787, net after gas
  $0.3359) — read-only checks only, nothing broadcast, signing stays in the
  operator's wallet app.
- Explicit-failure paths verified live against the running daemon: invalid
  platform/hash (422), no RPC key (503), no snapshot (503), MCP tools degrade to
  "No snapshot available yet".
- Lite cockpit and desktop dashboard verified rendering live on-chain data in a
  browser (screenshots in the spec task's screenshots/ directory).
