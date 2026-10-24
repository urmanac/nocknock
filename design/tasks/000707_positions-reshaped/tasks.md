# Implementation Tasks: Uniswap-Only Portfolio After Aerodrome Liquidation

## Phase 1 — Discovery (RPC, no guessing) — DONE (results in design.md)
- [x] Locate the wallet(s) holding the three fresh positions via `eth_getLogs` (POSN mints joined to ModifyLiquidity by tx) — both v4 mints went to the tracked wallet; ticket premise corrected
- [x] Enumerate the tracked wallet's 42 V4 POSN NFTs and classify: 2 active (3163431, 3163432) + 40 dust (incl. old #3126000, liquidity=0)
- [x] Decode the three new v4 positions: poolIds, ticks, fee tiers, currency ordering via POSM poolKeys + StateView getSlot0
- [x] Locate the v3 position: Base canonical NPM 0x03a520b3…, tokenId 6336662 (WETH/NOCK 1%, ticks [70400,74800]), pool via factory getPool
- [x] Verify ETH/USD source: both Chainlink candidates empty → adopted approved fallback: canonical v3 WETH/USDC 0.05% pool 0xd0b53d92… (factory-resolved, price-verified $2,373)
- [x] Record all identifiers in design.md + ONCHAIN_INTEGRATION.md (Phase-2 docs task)

## Phase 2 — Data model & specs — DONE
- [x] `specs/core/snapshot.yaml`: add `Position` schema + `positions` array on `SnapshotData`; platform enums gain real `UniswapV3`; deprecate `ConcentratedLiquidity`; replace `tranche_N_nock` series with `positions_nock`; bump `meta.version` to 2.0.0
- [x] `specs/core/api.yaml`: update envelope docs; `ClaimFeeAction.platform` + record-harvest `platform` enums gain `uniswap-v3`; remove Aerodrome examples/labels from `/execute/claim-fees/payload`
- [x] `frontend/src/types/snapshot.ts`: mirror `Position`, series change, enum additions, `uniswap-v3` platform
- [x] `ingestion-worker/config.yaml.example`: add `positions:` section (discovered ids), drop Aerodrome `tracked_pools`/`lp_tokens`, re-point `market_data.pool_address`, rename `uniswap_v4_lp` → `uniswap_lp`
- [x] `CHANGELOG.md`: Unreleased entry with migration notes for removed field names

## Phase 2 — Ingestion worker — DONE (live-validated 2026-10-24)
- [x] Config structs: parse new `positions:` section (network, protocol, nft_id, pool_id/npm_address, currency ordering, is_price_source); retain serde tolerance for old configs
- [x] `fetch_raw_data()`: remove Aerodrome getReserves/LP-share/claimFees logic and the `0x88A43…bB9C` ETH/USD source; add per-position v4 reads (PositionLiquidity, poolKeys validation, StateView getPositionInfo/getFeeGrowthInside, getSlot0) and v3 reads (positions decode, slot0, feeGrowthGlobal + tick outsides)
- [x] Implement exact per-position amount math (liquidity + sqrtP + ticks), in_range check, unclaimed fees (v4 fee-growth math; v3 inside-delta + tokensOwed); replace `RawOnChainData.live_tranche_*` with `positions: Vec<RawPosition>`
- [x] Price pipeline: NOCK/USD spot from the designated `is_price_source` pool slot0; ETH/USD from the verified canonical v3 WETH/USDC pool; rename `PRICE_SOURCE_RPC` to `"uniswap_rpc"`; switch regime RSI SQL to `token_price_series[0]`
- [x] `compute_derived_metrics()`: aggregate wallet + Σ positions; deviation matrix buckets Treasury / uniswap_lp / Cash; advice engine de-Aerodromed; RANGE EXIT keyed on price-source range; `tick_bracket_hypervisor` emits one bracket per position
- [x] `pool_mechanics`: emit per tracked Uniswap pool (slot0 tick + sqrtPrice, real fee tier, wallet's amounts as volatile_assets); remove Aerodrome entry
- [x] `get_claim_fees_payload_handler()`: drop Aerodrome action; emit one V4 collect per tracked position with per-position currency ordering (live-validated: both simulate success on Base)
- [x] `src/bin/mcp.rs`: update tool descriptions and the direct-RPC fallback to stop presenting Aerodrome state as live
- [x] Live-run bugfixes found during validation: v3 fee-delta used the wrong TickInfo word (fixed via NPM collect staticcall ground truth: 0.165434 NOCK exact match); lower/upper price orientation; adaptive bracket price precision; meta.version bumped

## Phase 2 — Frontend
- [~] `App.tsx`: Positions panel rendering N cards (pair, protocol badge, fee tier, range, split, value, fees, in-range pill); NOCK-stack chart from per-position series + safe haven; iterate N brackets in the hypervisor; de-Aerodrome labels/pills; `SOURCE_LABELS` + `uniswap_rpc`
- [ ] `LiteCockpit.tsx`: relabel "UNCOLLECTED FEES (BOTH LPs)" → "(ALL POSITIONS)"; verify generic action rendering still works with per-position labels
- [ ] `walletconnect.ts`: no change needed — Phase 1 confirmed all positions on Base

## Phase 2 — Docs
- [ ] `ONCHAIN_INTEGRATION.md`: mark §2 HISTORICAL (liquidated 2026-10-23); add new Uniswap-only portfolio section with discovered identifiers, selectors, and price-source decision
- [ ] `README.md` (Strategic Tranches) + `HANDOFF.md` tranche references: minimal edits to the Uniswap-only shape; verify `OPERATOR_GUIDANCE.md` needs no change

## Phase 2 — Tests & validation
- [ ] Worker: per-position v4 decode tests (StateView + slot0 + poolKeys fixtures); v3 `positions(uint256)` 12-word decode fixture test; amount-math tests against the screenshot's magnitudes; rewrite tranche-based tests (`test_derived_metrics_*`, operator-profile weights) against the new shape; add a three-position fixture snapshot test
- [ ] Frontend: update `freshness.contract.test.ts` schema assertions + add a three-position fixture snapshot driving the position-card rendering; update `client.errors.test.ts` platform usage; keep live-API skip semantics
- [ ] Validate: `make test-worker`, `make test-frontend`, `make validate-specs`, `make build-frontend` all green
- [ ] Live validation: real snapshot fetch against the confirmed wallet shows the three positions matching the screenshot's magnitudes (± drift); cockpit renders three cards; claim payload contains no Aerodrome action
