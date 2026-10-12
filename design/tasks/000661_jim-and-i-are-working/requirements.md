# Requirements: NockNock v0.2.0 Golden Path — Android Claim MVP & Sovereign Cockpit

## Context

NockNock v0.1.3 is ready (PRs #25, #26, #27 merged: live Uniswap V4 StateView fee query, MCP stdio config, tiered freshness + version bump). The operator and Gemini ("Jim") have already planned v0.2.0: GitHub milestone `v0.2.0` with issues #28–#32, plus the `[0.2.0]` section in `CHANGELOG.md` and the "Golden Path to v0.2.0" in `HANDOFF.md`. This spec is the consolidated build plan for that release, adding one new requirement from the operator: **a minimal native Android app** that pairs with the Uniswap mobile app via WalletConnect and executes fee claims — the MVP deliverable.

Standing orders that bind all work: single sovereign wallet (`0xeE114cC97eC5adeB34bA5645e315B497D8487a45`), **no custom wallet development** (zero custody — keys live in the Uniswap mobile app), no separate tax escrow address, 60/40 split as a *virtual* sub-ledger, and the zero-mock invariant (fail explicitly, never show synthetic numbers).

## User Stories

### A. Minimal Android Claim App (operator's MVP requirement)

**A1 — Pair once, no credentials.**
As the operator, I open the NockNock Android app and pair it with my Uniswap mobile app via WalletConnect so the app can propose transactions that I sign with biometrics. The Android app stores no keys, seeds, or credentials.
- AC: Pairing succeeds via on-device deep link or QR — the operator already runs this exact flow from a browser dapp (pair to Uniswap with read-only + suggest-transactions permissions); session persists across app restarts; a different WalletConnect wallet may be paired too ("at least the Uniswap app").
- AC: The app requests only read-only + suggest-transactions permissions; there is no wallet-import, seed, or private-key UI anywhere in the app.

**A2 — See claimable fees without opening the dashboard.**
As the operator, I see uncollected fees for both LPs (Aerodrome Tranche 2 and Uniswap V4 Tranche 1) and net value after estimated Base gas on one screen.
- AC: Values come from the worker API `/v1/snapshot/latest` (server URL configurable, reachable over LAN/Tailscale); if unreachable, the app shows an explicit degraded state, never estimated/mock numbers.
- AC: The app embeds **no secrets** (no Alchemy key, no tokens inside the APK) and persists **no financial state** locally — the worker is the single owner of state.
- AC: Screen shows floor health delta (NOCK vs `nock_floor_target`) so I know whether harvesting is safe to act on.

**A3 — Execute a batch harvest.**
As the operator, I tap "EXECUTE BATCH HARVEST" and the app dispatches the two claim transactions (Aerodrome `claimFees()`, Uniswap V4 fee collect) to the paired Uniswap app for biometric approval.
- AC: Each transaction is pre-simulated (`eth_call`) before dispatch; a simulated failure blocks dispatch with a clear error.
- AC: The Uniswap app signs with one biometric confirmation per transaction (sequential confirms are acceptable — no multicall); the app shows per-tx status and the Base tx hash on completion.
- AC: The app handles the known Uniswap-app broadcast quirk: an approved transaction may be held unbroadcast ("pending limbo") until the operator returns to the Uniswap app. A claim is reported complete only once its receipt is visible on-chain; while awaiting wallet broadcast the app says so plainly.

**A4 — Harvests become ledger truth.**
As the operator, after a claim confirms, the harvest is recorded so the 60/40 sub-ledger and tax exports include it.
- AC: On success the app posts the tx hash to the worker, which verifies the receipt on-chain before recording; unverified hashes are rejected.

### B. Golden Path Payload Engine (worker) — shared by Android app and web cockpit

**B1 — One endpoint builds claim payloads.**
The worker exposes an endpoint that returns ready-to-sign unsigned transactions for Aerodrome and Uniswap V4 fee claims (targets, calldata, gas estimate, net-after-gas USD), derived from the live snapshot.
- AC: Payloads always reflect live on-chain fee state (StateView for V4, simulated `claimFees` for Aerodrome); if a source RPC fails, the endpoint fails explicitly.
- AC: The same payloads power the web cockpit (issue #31) and the Android app — calldata logic exists in exactly one place.

### C. 60/40 Virtual Sub-Ledger & Tax Planner

**C1 — Virtual partition.** As the operator, realized fee harvests (and net capital gains, where measurable) are split 40% virtual tax reserve / 60% growth dry powder inside the single wallet — no on-chain address changes.
- AC: Ratios configurable in `OperatorProfile` (`config.yaml`), defaults 0.40/0.60; dashboard shows the two virtual balances plus a tax-health indicator.

**C2 — Guardrail.** The engine never recommends dip deployments that would breach the tax reserve or the treasury floor; the cockpit blocks/flags such actions.
- AC: Brain advice output includes a guardrail verdict on proposed deployments.

**C3 — Koinly export.** `GET /v1/tax/export?format=koinly` produces a Koinly-compatible export of harvest and swap events tagged with Base tx hashes (fee claims as income; rebalances as non-taxable transfers).
- AC: Every exported row carries an on-chain tx hash; nothing is synthesized.

**C4 — Tie-out suite.** Automated tests prove internal ledger, on-chain balances, and exported records tie out for a harvested-fee scenario.
- AC: `make test-worker` includes the tie-out tests and passes.

**C5 — At-a-glance P/L from local data only.** As the operator, the dashboard and lite cockpit show profit/loss computed from NockNock's own recorded history — high-water marks, recorded spot prices, and receipt-verified harvest ledger — with zero external APIs in 0.2.0.
- AC: Every displayed P/L number traces to recorded snapshots or verified ledger rows; nothing is mocked (zero-mock invariant).
- AC: External cost-basis APIs (Moralis/Alchemy) are explicitly deferred to a future minor release — they are new-feature work under SemVer, not a 0.2.1 patch, and not a mocked shim.

### D. Anti-Top-Buying Guardrails (brain, issue #29)

**D1 — Overextension regime.** When NOCK price is overextended (RSI — Relative Strength Index, a plain 0–100 momentum score; readings roughly 70+ mean "stretched/bought-the-top" territory — and upper-band tests against existing structural channels), the brain suppresses buy/reinvest advice and mandates "SWEEP TO STABLE / ACCUMULATE DRY POWDER" with a cooldown.
- AC: Brain stays pure deterministic math (no RPC/clock); regime inputs are passed in; worker tests cover the regime transitions.

### E. Actionable Notifications (issue #28)

**E1 — Act without watching.** When a tactical directive changes (e.g. `HOLD & ACCRETE` → `BATCH HARVEST SHOT`), the worker sends a notification with claimable amounts, gas, net accretion, and a deep link to the lite mobile cockpit.
- AC: Adapters for Telegram, Pushover, and generic webhook, configured in `config.yaml`; nothing fires without an explicit directive transition; a failed delivery is logged, never silently dropped.

### F. Lite Mobile Web Cockpit (issue #31)

**F1 — Phone-sized golden path.** A mobile-responsive lite view of the dashboard with three elements: floor health, uncollected fees + gas ROI, and an "EXECUTE BATCH HARVEST" button that signs via WalletConnect (Reown AppKit/Wagmi) to the paired Uniswap app — reusing endpoint B1.
- AC: Works on a phone browser; signing goes through the same zero-custody flow as the Android app.

### G. Read-Only Expansion: Multi-AMM Discovery & Coinbase Staked ETH

**G1 — Pool registry.** Discovered/tracked AMM pools (existing NOCK/USDC pools plus ETH-related pools on Base) are registered in the worker, counted in unified NAV, and shown on an expanded canvas.
- AC: Discovery is config/registry-driven and read-only; no trading behavior changes.

**G2 — Coinbase staked ETH.** Read-only tracking of staked ETH / cbETH balances recorded as historical snapshots alongside Base data.
- AC: Uses the minimal viable Coinbase API path (public endpoints where possible); failures degrade explicitly.

### H. Visual Identity & Canvas Overhaul

**H1 — Unmistakable v0.2.0.** Dual-dial gauges (40% tax reserve vs 60% dry powder), WalletConnect session banner, live P/L dial, expanded multi-vault canvas. The dormant `structural_channels` panel is superseded by the dual-dial gauges (per TRIAGE item 12 note).
- AC: Follows the UI contrast rules in `OPERATOR_GUIDANCE.md` (dark container tooltips, sky-blue targets, dynamic Y domains); v0.1.x and v0.2.0 dashboards are visibly distinct.

### I. CI & Release Packaging

**I1 — Faster CI.** `Swatinem/rust-cache@v2` on Rust jobs.
**I2 — Android artifact (optional).** A debug-signed APK attached to GitHub releases when feasible in CI; sideload-only, no store.

## Non-Goals

- No custom wallet/custody code, no key storage anywhere in NockNock (standing order).
- No on-chain changes to the treasury wallet; 60/40 is purely accounting.
- No multicall-aggregation wrapper that changes `msg.sender` semantics for Aerodrome `claimFees()` (see design.md decision 3).
- No embedded WASM brain in the Android app for MVP (Tier-3 offline fallback is deferred — see Open Questions).
- No multi-tenant runtime in 0.2.0 (schema/blueprint only, per CHANGELOG); no Urbit implementation.

## Resolved in Operator Review (2026-10-12)

- **Android app deferred out of 0.2.0** (operator decision at implementation check-in): 0.2.0 executes via the **existing web app + WalletConnect QR pairing** with the Uniswap mobile app (read-only + suggest-transactions permissions); the Android app remains planned future work. Phase 1's payload endpoint serves both surfaces.
- Same-device WalletConnect pairing with the Uniswap app is proven operator practice (arbitrary contract calls included); the pending-broadcast quirk is a designed-for UX case, not a blocker.
- P/L: external cost-basis APIs deferred to a future minor release; 0.2.0 ships local-data-only P/L. No mocked P/L.
- Multiple sequential signatures are acceptable; multicall rejected.

## Open Questions

1. **Coinbase API**: for staked ETH/cbETH balances, is a (free-tier) Coinbase API key available to the operator, or must we stick to public endpoints only?
2. **Koinly format**: CSV (Koinly "custom import") assumed for `/v1/tax/export`; JSON secondary. Confirm CSV-first is right.
3. **Release artifact**: should the debug APK become the 5th release artifact, or stay out of releases for 0.2.0?
4. **Worker exposure for the phone**: the worker currently runs on a local machine (tmux, `:8080`) with no REST auth. We assume LAN or Tailscale reachability with the server URL entered once in the app. Acceptable, or do you want a bearer token (same pattern as the MCP HTTP endpoint) in 0.2.0?
