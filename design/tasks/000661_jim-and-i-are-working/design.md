# Design: NockNock v0.2.0 Golden Path — Android Claim MVP & Sovereign Cockpit

## Baseline (what already exists, post-0.1.3)

- Worker computes **live** uncollected fees for both LPs — no mock fallbacks (PR #25):
  - Aerodrome T2: `eth_call` simulation of `claimFees()` (`0xd294f093`) from the sovereign wallet to pool `0x85f1aa3a70fedd1c52705c15baed143e675cd626`; returns USDC (6dp) + NOCK (16dp).
  - Uniswap V4 T1: `StateView` `0xa3c0c9b65bad0b08107aa264b0f3db444b867a71` (`getPositionInfo` `0xdacf1d2f`, `getFeeGrowthInside` `0x53e9c1fb`) + zero-dep `U256` math; PositionManager `0x7c5f5a4bbd8fd63184577525326123b519429bdc`, token #3126000, poolId `0x064a5df2…636b13`, ticks 223200/274200.
- Freshness/tiered sync (PR #27), stdio+HTTP MCP (PR #26), `OperatorProfile` externalized config (PR #18), Axum REST + stateless HTTP MCP, React/Vite dashboard with 7 charts, Vitest contract suite.

## Architecture

```mermaid
graph TD
    SUB["Base Mainnet (8453)"] -->|eth_call / receipts| WORKER
    UNI["Uniswap Mobile App<br/>(holds keys, biometrics)"] -->|signs txs| SUB
    AND["Android App<br/>(minimal Kotlin, zero custody)"] -->|"WalletConnect (Reown Sign)"| UNI
    WEB["Web Cockpit<br/>(desktop + lite mobile view)"] -->|"WalletConnect (AppKit/Wagmi)"| UNI
    AND -->|GET snapshot, POST tx-hash| WORKER
    WEB -->|REST + WC payload| WORKER
    NOTIF["Telegram / Pushover / Webhook"] <--directive transitions-- WORKER
    BRAIN["WASM Brain (pure math)<br/>regimes, guardrails, invariants"] --> WORKER
```

Key principle: **the worker is the single source of truth** for on-chain math and calldata; the Android app and web cockpit are thin execution surfaces that fetch state/payloads and dispatch signing. This mirrors the existing zero-custody Golden Path in `HANDOFF.md` §5 and `OPERATOR_GUIDANCE.md` §7 ("No Custom Wallet Development").

## Key Decisions

1. **Android app = thin WalletConnect dapp client.** Kotlin, single-activity, Reown/WalletConnect **Sign** SDK + OkHttp + a tiny JSON parser. It pairs with the Uniswap mobile app (QR or same-device deep link), shows snapshot-derived fee state, and wraps pre-built payloads in `eth_sendTransaction` WC requests. No wallet import, no seed UI, no embedded WASM brain, no direct-RPC fallback in MVP (explicit degraded state instead). Rationale: the operator asked for minimal, and `moon-phase-clock`-style Tier-3 offline autonomy is a later concern. Lives in `android/` at the repo root (matches `ARCHITECTURE.md`'s assumed `android/app/...` layout).

2. **Payload generation lives in the worker, not the clients.** New worker endpoints (sketch below) emit unsigned transactions derived from the latest live snapshot. Both the Android app and web cockpit consume them; ABI/encoder logic exists once, in tested Rust. Clients only: fetch → display → dispatch → report hash.

3. **Two separate claim transactions; no multicall wrapper.** Aerodrome `Pool.claimFees()` is caller-relative (the simulated call is made with `from: <sovereign wallet>`); routing it through Multicall3/Router would change `msg.sender` and misdirect fees. The single "EXECUTE BATCH HARVEST" button queues two txs sequentially through the WC session (two biometric confirms). Simplicity + correctness over one-tx aggregation.

4. **Uniswap V4 fee collection via PositionManager `modifyLiquidities`.** `multicall([DECREASE_LIQUIDITY(tokenId, liquidityDelta=0, 0, 0, deadline), TAKE_PAIR(token0, token1, recipient=sovereign wallet)])` — the standard zero-liquidity collect pattern for V4 posm. Exact action-byte encoding is verified during implementation by `eth_call` simulation first (the repo's established pattern), with the tx blocked if simulation reverts.

5. **Simulate-before-sign, everywhere.** Both payloads are validated with `eth_call` immediately before each WC dispatch. Consistent with the zero-mock invariant: if simulation or signing fails, the UI fails explicitly and nothing is recorded.

6. **Harvest ledger is receipt-verified.** After a WC tx confirms, the client posts the tx hash to `POST /v1/execute/record-harvest`; the worker fetches the receipt from Base, extracts actual amounts transferred, and appends a ledger row (`harvest_events`: timestamp, platform, token amounts, USD at spot, tx hash). Unverifiable hashes are rejected. This keeps 60/40 accounting and Koinly exports tied to on-chain truth only.

7. **60/40 sub-ledger = config-driven derived accounting.** `OperatorProfile` gains `tax_reserve_ratio` (default 0.40) and the split applies to cumulative realized harvests + realized gains tracked in the ledger. Rendered as virtual balances + a tax-health flag in derived metrics (OpenAPI `specs/core/snapshot.yaml` extended). The brain consumes the virtual balances for guardrails (decision 8); it stays pure math.

8. **Guardrails in the brain, signals in the worker.** Worker computes RSI/band-extension series from historical snapshots (structural channels already exist post-PR #25-era) and passes them into the brain; the brain emits regime + directive (`HOLD & ACCRETE`, `BATCH HARVEST SHOT`, `SWEEP TO STABLE / ACCUMULATE DRY POWDER`, cooldown). The worker notifies on directive *transitions* only.

9. **Notifications = three small adapters.** Telegram bot, Pushover, generic JSON POST; configured under `config.yaml` (`[notifications]`); transition-only dispatch with the payload from issue #28 (trigger, gross claimable, gas, net, deep link to lite cockpit).

10. **Web lite cockpit shares everything.** Same REST + payload endpoints; WalletConnect via Reown AppKit/Wagmi in the React app. Mobile-responsive single view (floor health, fees+ROI, harvest button, WC session banner). The full desktop dashboard gets the dual-dial gauges + session banner + expanded canvas per `CHANGELOG [0.2.0]`.

11. **CI: `Swatinem/rust-cache@v2`** on brain/worker jobs (per `OPERATOR_GUIDANCE.md` §10). Android CI job is optional/last — a local debug APK is acceptable for MVP; if CI packaging works, attach as a 5th artifact (gated on Open Question 6).

12. **Deferred to post-0.2.0:** embedded WASM brain in Android + Tier-3 RPC fallback chain, multicall aggregation, multi-tenant runtime (schema/blueprint only now), Urbit desk implementation.

## Worker Endpoint Sketches (additive to existing Axum router)

```
GET  /v1/execute/claim-fees/payload
     -> { chain_id: 8453, from: "<wallet>", estimated_gas, net_after_gas_usd,
          actions: [ { platform: "aerodrome", to: 0x85f1…, data: 0xd294f093, value: "0x0",
                       summary: "Claim Aerodrome fees: X USDC + Y NOCK" },
                     { platform: "uniswap-v4", to: 0x7c5f…, data: <posm multicall>, value: "0x0",
                       summary: "Collect Uniswap V4 fees: …" } ] }

POST /v1/execute/record-harvest  { platform, tx_hash }
     -> verifies receipt on-chain; appends verified harvest_events row (404/409 on failure)

GET  /v1/tax/export?format=koinly
     -> CSV (default) / JSON rows tagged with tx hashes; claims = income, rebalances = transfer

New snapshot fields: tax_reserve_usd, growth_dry_powder_usd, tax_health,
harvest_events[] (recent), pools_registry[] (discovery), directive_transitions[]
```

Schema additions land in `specs/core/snapshot.yaml` + `specs/core/api.yaml` and are covered by the frontend contract tests (existing pattern in `frontend/src/api/freshness.contract.test.ts`).

## Android App (minimal shape)

```
android/
  app/src/main/java/nocknock/cockpit/
    MainActivity.kt        # single screen: fees, floor, harvest button, tx status
    WalletConnect.kt       # Reown Sign pairing + eth_sendTransaction dispatch
    NockApi.kt             # snapshot fetch, payload fetch, record-harvest POST
    ui/                    # 2–3 composables, dark aesthetic tokens from OPERATOR_GUIDANCE §8
  app/src/main/assets/     # (empty for MVP; reserved for future WASM brain)
```

Config: server URL + optional auth token entered once, stored in `SharedPreferences`; Reown project ID compiled in (`BuildConfig`).

## Learnings / Notes for Implementers

- **Zero-mock invariant is enforced in code review and tests**: PR #25 deleted a fallback branch rather than fixing it quietly. Any new "estimated" number must be labeled degraded or fail the cycle.
- The repo uses a **hand-rolled minimal `U256`** (no `alloy`/`ethers` dependency) and raw `eth_call` JSON payloads with `post_rpc_with_retry`. Follow that pattern for the posm multicall encoder instead of adding heavy deps.
- Aerodrome fee claim is **already proven live** by the simulated `eth_call` (issue: matches mobile app to the penny). The real tx uses identical calldata with the wallet as signer.
- Build worker with `SQLX_OFFLINE=true`; brain with `RUSTFLAGS="-C link-arg=--allow-undefined"` targeting `wasm32-wasip1` (see `AGENTS.md` §3–4).
- Pre-flight per PR: `make test-brain` (3), `test-worker` (11), `test-frontend` (20 after PR #27), `test-mcp`, `make dist`. PRs land via `gh pr create/merge` with `unset GITHUB_TOKEN`; `main` is protected.
- UI: dark container tooltips (`#161b22`/`#30363d`/`#f0f6fc`), sky-blue targets (`#38bdf8`), dynamic Y domains only (`OPERATOR_GUIDANCE.md` §8). Sync status pills follow PR #27 tiers (fresh/syncing/stale).
- Frontend CPU regressions were caused by polling-dependency churn (issue #20) — new polling surfaces must follow the `useFreshness` fixed-deps pattern.
