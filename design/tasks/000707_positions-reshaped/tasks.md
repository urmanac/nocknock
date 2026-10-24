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

## Phase 2 — Frontend — DONE
- [x] `App.tsx`: Positions panel rendering N cards (pair, protocol badge, fee tier, range, split, value, fees, in-range pill); NOCK-stack chart from per-position series + safe haven; iterate N brackets in the hypervisor; de-Aerodrome labels/pills; `SOURCE_LABELS` + `uniswap_rpc`
- [x] `LiteCockpit.tsx`: relabel "UNCOLLECTED FEES (BOTH LPs)" → "(ALL POSITIONS)" (L260); generic action rendering verified with per-position labels
- [x] `walletconnect.ts`: no change needed — Phase 1 confirmed all positions on Base (verified: zero Aerodrome references)

## Phase 2 — Docs — DONE
- [x] `ONCHAIN_INTEGRATION.md`: §1 address table updated; §2 + §4 marked HISTORICAL (liquidated 2026-10-23); new §5 records the discovered identifiers, selectors, v3 fee-math ground truth, and the ETH/USD price-source decision
- [x] `README.md` (portfolio section) + `HANDOFF.md` briefing line: minimal edits to the Uniswap-only shape; `OPERATOR_GUIDANCE.md` verified to contain zero Aerodrome/tranche references — no change needed
- [x] `CHANGELOG.md`: Unreleased entry with migration notes (included in specs slice above)

## Phase 2 — Tests & validation — DONE
- [x] Worker: per-position v4 decode tests (StateView + slot0 + poolKeys fixtures); v3 `positions(uint256)` 12-word decode fixture test; amount-math tests against the screenshot's magnitudes; tranche-based tests rewritten against the new shape; three-position fixture snapshot tests — **47/47 green**
- [x] Frontend: `freshness.contract.test.ts` asserts v2.0.0 positions schema (live-API guarded) + three-position fixture drives card-rendering helpers; `client.errors.test.ts` verified to contain zero platform/source references — nothing to update; live-API skip semantics kept — **28/28 green**
- [x] Validate: `make test-worker` ✓, `make test-frontend` ✓, `make validate-specs` ✓ (13 pre-existing warnings only), `make build-frontend` ✓ (via `make dist`) — all green
- [x] Live validation: real snapshot fetch against `0xeE11…7a45` returned the three positions matching the screenshot's magnitudes (B $71.11 vs $71.45); both v4 collect payload entries simulate success on Base; cockpit renders three cards (`screenshots/02-cockpit-three-positions.png`)

## Post-review fixes (PR proposal rejected — addressed, 2026-10-24)
- [x] Claim payload: v3 collect action for NFT #6336662 added — canonical Base NPM `0x03a520b3…`, selector `collect((uint256,address,uint128,uint128))` = 0xfc6f7865 (Phase-1 ground truth), type(uint128).max maxes; live eth_call verified: returns (0, 1654339369283004) = 0.165434 NOCK = the snapshot's tokens_owed_1 exactly; golden-vector test added; payload emits 3 actions, all sim=success (gross $0.04, net +$0.0348)
- [x] Hypervisor axis: ETH-denominated brackets converted to USD per NOCK via the worker's own live ETH/USD (derived client-side from pool_mechanics volatile_assets reserve_usd/reserve_amount — same reads the worker valued with); shared axis + spot ReferenceLine; footer discloses the conversion rate
- [x] Data integrity: worker now emits `distance_bps: null` for cross-denomination rows (was a meaningless -9993/-9996 cross-unit number); schema field is `nullable: true` (v2.0.0 unreleased); TS type `number | null`
- [x] Tests: worker 48/48 (new v3 collect golden vector), frontend 28/28, `make validate-specs` green; live payload = 3 actions all sim=success

## Change request (2nd review, 2026-10-24): FOURTH live position — DONE
- [x] Discovery via RPC: ownerOf(3163450) = tracked wallet; poolId + raw ticks [262200, 282300] from the mint's ModifyLiquidity event (tx 0x6cdabada…, block 53008241); getPoolAndPositionInfo cross-check (fee 30000 = 3.00%, tickSpacing 300, packed poolId/ticks); PositionLiquidity active — recorded in ONCHAIN_INTEGRATION §5
- [x] Config: position D in `config.yaml.example` + live config (USDC/NOCK, 300bps, wide raft, NOT price source — B stays the single source)
- [x] Fixtures/tests: worker `four_position_fixture` (real chain reads; D liquidity 185,454,734,478,733 after the operator's post-mint compounding), count assertions 3→4, config-parse YAML extended, tax-guardrail input adjusted (D's 129-USDC leg); frontend fixture + label/range assertions for D — worker 48/48, frontend 28/28
- [x] Live validation: snapshot ingests 4 positions — B $71.11, A $150.51, **D $206.33**, C $47.21, total **$475.16 ≈ the operator's ~$477** ✓; claim payload = 4 v4/v3 actions all sim=success; cockpit renders FOUR cards (screenshot `05-cockpit-four-positions.png`); portfolio $2,632.22 (new HWM), floor 115.7% protected
- [ ] Re-propose PR (approved on sight per reviewer)

Note: the operator's "fee 3000 (3.00%)" — the raw fee value is 30000 (0x7530) = 3.00%; the percentage was right. D's first read (02:33 UTC, L=48.3e12 → ~$54) predated the operator's +143.7e12 add; the live pipeline picked up the compounded state immediately — zero code change needed for repositions (the config-registry design working as intended).

## Post-implementation fixes
- [x] Drive-by: `specs/core/snapshot.yaml` was missing the colon in `openapi: 3.0.3` (pre-existing on main — redocly treated the file as invalid); `specs/core/api.yaml` had `ClaimFeesPayload`/`ClaimFeeAction` at the `components:` root instead of `components.schemas:` (unresolved-ref errors)
- [x] Drive-by: Makefile's `cargo install wit-bindgen` targets the library crate (binary lives in `wit-bindgen-cli`); installed `wit-bindgen-cli v0.63.0` in-sandbox to run the WIT half of `validate-specs` (Makefile left as-is — out of scope)
- [x] `make test-brain` ✓ 3/3, `make dist` ✓ (all 4 artifacts packaged)
- [x] Release-build dead-code warning fixed: `UNISWAP_V3_NPM_BASE` scoped to the tests module (live v3 reads take `npm_address` from config and fail explicitly when missing)
