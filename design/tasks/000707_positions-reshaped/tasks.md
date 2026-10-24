# Implementation Tasks: Uniswap-Only Portfolio After Aerodrome Liquidation

## Phase 1 — Discovery (RPC, no guessing)
- [ ] Locate the wallet(s) holding the three fresh V4 NFTs via `eth_getLogs` on the canonical V4 POSM (Transfer from 0x0, mint window 2026-10-23 ~21:10–21:20 UTC); fall back to asking the operator
- [ ] Enumerate the tracked wallet's 42 V4 POSM NFTs (Transfer logs → tokenIds) and classify each via `getPositionLiquidity` (0x1efeed33) / `getPoolAndPositionInfo` (0x7ba03aad): active vs dust/empty; record fate of old NFT #3126000
- [ ] Decode the three new v4 positions: poolId, liquidity, ticks (getPoolAndPositionInfo), sqrtPrice/tick (StateView getSlot0), fee tier + currency ordering (POSM poolKeys)
- [ ] Locate the v3 position: verify canonical NPM address per network on-chain (Base expected `0x03a520b32C04BF3bEEf7BEb72E919cf822Ed34f1`; do NOT trust ticket-quoted `0x46A15B…`), find owning wallet, decode `positions(uint256)` (0x99fbab88): fee=1%, ticks, liquidity, tokensOwed; resolve pool via factory `getPool(token0, token1, fee)`
- [ ] Verify the Chainlink ETH/USD aggregator address on Base (and record a fallback pool: canonical Uniswap v3 WETH/USDC 0.05% slot0)
- [ ] If any discovery result is ambiguous, ask the operator before proceeding; then record all identifiers in ONCHAIN_INTEGRATION.md + `config.yaml` `positions:` section

## Phase 2 — Data model & specs
- [ ] `specs/core/snapshot.yaml`: add `Position` schema + `positions` array on `SnapshotData`; platform enums gain real `UniswapV3`; deprecate `ConcentratedLiquidity`; replace `tranche_N_nock` series with `positions_nock`; bump `meta.version` to 2.0.0
- [ ] `specs/core/api.yaml`: update envelope docs; `ClaimFeeAction.platform` + record-harvest `platform` enums gain `uniswap-v3`; remove Aerodrome examples/labels from `/execute/claim-fees/payload`
- [ ] `frontend/src/types/snapshot.ts`: mirror `Position`, series change, enum additions, `uniswap-v3` platform
- [ ] `ingestion-worker/config.yaml.example`: add `positions:` section (discovered ids), drop Aerodrome `tracked_pools`/`lp_tokens`, re-point `market_data.pool_address`, rename `uniswap_v4_lp` → `uniswap_lp`
- [ ] `CHANGELOG.md`: Unreleased entry with migration notes for removed field names

## Phase 2 — Ingestion worker
- [ ] Config structs: parse new `positions:` section (network, protocol, nft_id, pool_id/npm_address, currency ordering, is_price_source); retain serde tolerance for old configs
- [ ] `fetch_raw_data()`: remove Aerodrome getReserves/LP-share/claimFees logic and the `0x88A43…bB9C` ETH/USD source; add per-position v4 reads (PositionLiquidity, PoolAndPositionInfo, StateView getPositionInfo/getFeeGrowthInside, getSlot0) and v3 reads (positions decode, factory getPool, slot0)
- [ ] Implement exact per-position amount math (liquidity + sqrtP + ticks), in_range check, unclaimed fees (v4 fee-growth math; v3 tokensOwed); replace `RawOnChainData.live_tranche_*` with `positions: Vec<RawPosition>`
- [ ] Price pipeline: NOCK/USD spot from the designated `is_price_source` pool slot0; ETH/USD from the verified Chainlink feed with 1h staleness check; rename `PRICE_SOURCE_RPC` to `"uniswap_rpc"`; switch regime RSI SQL to `token_price_series[0]`
- [ ] `compute_derived_metrics()`: aggregate wallet + Σ positions; deviation matrix buckets Treasury / uniswap_lp / Cash; advice engine text de-Aerodromed; RANGE EXIT FLASHPOINT keyed on price-source range; `tick_bracket_hypervisor` emits one bracket per position
- [ ] `pool_mechanics`: emit per tracked Uniswap pool (slot0 tick + sqrtPrice, real fee tier, wallet's amounts as volatile_assets); remove Aerodrome entry
- [ ] `get_claim_fees_payload_handler()`: drop Aerodrome action; emit one V4 collect per tracked position with non-dust fees using per-position currency ordering (v3 collect only if confirmed in Phase 1)
- [ ] `src/bin/mcp.rs`: update tool descriptions and the direct-RPC fallback to stop presenting Aerodrome state as live

## Phase 2 — Frontend
- [ ] `App.tsx`: Positions panel rendering N cards (pair, protocol badge, fee tier, range, split, value, fees, in-range pill); NOCK-stack chart from per-position series + safe haven; iterate N brackets in the hypervisor; de-Aerodrome labels/pills; `SOURCE_LABELS` + `uniswap_rpc`
- [ ] `LiteCockpit.tsx`: relabel "UNCOLLECTED FEES (BOTH LPs)" → "(ALL POSITIONS)"; verify generic action rendering still works with per-position labels
- [ ] `walletconnect.ts`: add non-Base chain support only if Phase 1 confirmed a position on another network

## Phase 2 — Docs
- [ ] `ONCHAIN_INTEGRATION.md`: mark §2 HISTORICAL (liquidated 2026-10-23); add new Uniswap-only portfolio section with discovered identifiers, selectors, and price-source decision
- [ ] `README.md` (Strategic Tranches) + `HANDOFF.md` tranche references: minimal edits to the Uniswap-only shape; verify `OPERATOR_GUIDANCE.md` needs no change

## Phase 2 — Tests & validation
- [ ] Worker: per-position v4 decode tests (StateView + slot0 + poolKeys fixtures); v3 `positions(uint256)` 12-word decode fixture test; amount-math tests against the screenshot's magnitudes; rewrite tranche-based tests (`test_derived_metrics_*`, operator-profile weights) against the new shape; add a three-position fixture snapshot test
- [ ] Frontend: update `freshness.contract.test.ts` schema assertions + add a three-position fixture snapshot driving the position-card rendering; update `client.errors.test.ts` platform usage; keep live-API skip semantics
- [ ] Validate: `make test-worker`, `make test-frontend`, `make validate-specs`, `make build-frontend` all green
- [ ] Live validation: real snapshot fetch against the confirmed wallet shows the three positions matching the screenshot's magnitudes (± drift); cockpit renders three cards; claim payload contains no Aerodrome action
