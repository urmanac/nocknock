feat: v0.2.0 Golden Path - WalletConnect claim execution, 60/40 ledger, tax guardrails

## Summary
Implements the v0.2.0 "Golden Path": the operator can now execute fee harvests
for both LPs (Aerodrome + Uniswap V4) from the web cockpit by pairing the
Uniswap mobile app over WalletConnect — zero custody, worker-built pre-simulated
transactions, receipt-verified bookkeeping. No mock numbers anywhere: every
surface fails explicitly when on-chain data is unavailable.

## Changes
- **Golden Path payload engine** (`ingestion-worker`): `GET /v1/execute/claim-fees/payload`
  builds unsigned Aerodrome `claimFees()` and Uniswap V4 PositionManager collect
  transactions (DECREASE_LIQUIDITY 0 + TAKE_PAIR) from the live snapshot with
  pre-dispatch `eth_call` simulation, gas estimates, and USD valuation. Encoding
  verified byte-for-byte against the official `@uniswap/v4-sdk` planner output and
  accepted by live Base mainnet simulation (golden-vector tests).
- **Receipt-verified harvest ledger**: `harvest_events` table + `POST /v1/execute/record-harvest`
  (verifies Base receipts, idempotent, 404 = pending-broadcast signal).
- **Virtual 60/40 sub-ledger**: `sovereign_ledger` in derived metrics driven by
  `operator_profile.tax_reserve_ratio` (default 0.40); tax-reserve guardrail
  suppresses dip-deployment advice (`SWEEP TO STABLE / ACCUMULATE DRY POWDER`)
  when the reserve exceeds the stable stack.
- **Koinly tax export**: `GET /v1/tax/export?format=koinly` (CSV/JSON) with tie-out tests.
- **Anti-top-buying guardrails**: Wilder RSI(14) regime detection; overextension
  directive + `regime`/`rsi_14` in GlobalStatus.
- **Notifications**: Telegram/Pushover/webhook adapters on directive transitions
  (transition-only, failures logged, optional cockpit deep link).
- **Web cockpit**: `@walletconnect/ethereum-provider` pairing (QR + deep links,
  wrong-wallet refusal), `/?view=lite` mobile cockpit with the EXECUTE BATCH
  HARVEST flow and pending-broadcast handling, desktop session badge,
  Sovereign 60/40 Ledger panel with dual dials + live P/L vs HWM.
- **Bug fixes found during live verification**: snapshot_id `VARCHAR(26)` schema
  drift (migration-provisioned DBs could not persist snapshots) and sqlx
  migration-version collision; migrations renamed to unique 14-digit versions.
- **Docs**: CHANGELOG 0.2.0 section, HANDOFF Golden Path status, OPERATOR_GUIDANCE
  pairing runbook, TRIAGE/ARCHITECTURE notes.

## Deferred to 0.2.1 (documented)
Android app (web QR flow ships instead), multicall aggregation (rejected:
Aerodrome `claimFees()` is msg.sender-relative), multi-pool ingestion loop,
Coinbase staked-ETH connector (needs CDP key + JWT signing).

## Testing
- `make test-brain` 3/3 · `make test-worker` 31/31 · `make test-frontend` 20/20
  unit + 20/20 live contract · `make test-mcp` ALL PASSED with live on-chain data ·
  `make dist` all 4 artifacts packaged.
- Live daemon verification with the operator's Alchemy key: real ingestion,
  real snapshot, both claim actions simulate "success" (Aerodrome $0.0595, V4
  $0.2787, net after gas $0.3359); payload/record-harvest/export rejection paths
  return explicit errors (422/404/503).
- Screenshots: lite cockpit + desktop dashboard + 60/40 ledger panel in the
  spec task's screenshots/ directory.

## Operator follow-ups
1. Create a free Reown project id (cloud.reown.com) and build the frontend with
   `VITE_WALLETCONNECT_PROJECT_ID=<id>` to enable wallet pairing.
2. Playtest the full loop on the phone: PAIR → batch harvest → thumbprint in the
   Uniswap app (return to the app to broadcast) → ledger + Koinly row appear.
3. Configure `notifications:` in config.yaml (Telegram/Pushover/webhook) to get
   directive-transition alerts.
