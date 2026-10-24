# Design: Uniswap-Only Portfolio After Aerodrome Liquidation

## Current State (verified in code)

- `ingestion-worker/src/main.rs` (5,145 lines) — `fetch_raw_data()` (L2979) hardcodes:
  Aerodrome `getReserves` (NOCK/USDC pool `0x85f1…626`), LP `balanceOf`/`totalSupply` share
  math, `claimFees()` (`0xd294f093`) simulation, ETH/USD from Aerodrome
  `vAMM-WETH/USDC` (`0x88A43…bB9C`), and ONE V4 position (config `v4_position_id`,
  default 3126000) via StateView `getPositionInfo` (`0xdacf1d2f`) +
  `getFeeGrowthInside` (`0x53e9c1fb`). V4 tranche 1 amounts come from
  `operator_profile.v4_baseline_*` scaled by liquidity ratio (`live_t1_nock = baseline × v4_scale`).
- `RawOnChainData` carries `live_tranche_1_*` / `live_tranche_2_*` / `live_aero_fee_*` /
  `live_uni_fee_*`; `AssetAggregationSeries` carries `tranche_1_nock/2/3` series (L409);
  `compute_derived_metrics()` (L3495+) aggregates exactly three tranches and emits advice
  text naming Aerodrome; tick brackets use hardcoded `V4_TICK_LOWER/UPPER_ONCHAIN`
  constants (L2955).
- `get_claim_fees_payload_handler` (L2440) always builds Action 1 = Aerodrome
  `claimFees()` + Action 2 = V4 collect for NFT #3126000 (currency ordering hardcoded
  USDC/NOCK).
- `specs/core/snapshot.yaml` / `specs/core/api.yaml` / `frontend/src/types/snapshot.ts`
  mirror the two-tranche shape; `PoolMechanic.platform` enum lacks a real `UniswapV3`.
- Regime RSI SQL (`run_ingestion_cycle` L1339) reads `pool_mechanics[0]` reserve ratio —
  i.e. the Aerodrome pool; price point insert tags `PRICE_SOURCE_RPC = "aerodrome_rpc"`.
- Frontend: `App.tsx` stacks `tranche_1/2/3_nock` and labels "Aerodrome"; `LiteCockpit.tsx`
  shows aggregate fees/ledger; `walletconnect.ts` is platform-agnostic (dispatches
  worker-built calldata); MCP `src/bin/mcp.rs` direct-RPC fallback simulates Aerodrome
  `claimFees` and its tool descriptions name Aerodrome.
- Brain (`brain/`) is pure math over LP ratios/ticks and consumes none of this — untouched.

## Phase 1 — Discovery (RPC, no guessing) — COMPLETE 2026-10-24

**Result: all three positions belong to the tracked wallet `0xeE114cC97eC5adeB34bA5645e315B497D8487a45` on Base. The ticket's premise ("zero fresh mints to this wallet", "v3 NPM has 0 NFTs") was based on the WRONG v3 NPM address and a stale v4 mint check.** The screenshot's "9:16:33 PM 2026-10-23" is US Eastern (EDT) = 2026-10-24 01:16 UTC; the v4 mints landed 01:12:37 and 01:14:47 UTC ("1m/3m ago" ✓).

| | Position A | Position B | Position C |
|---|---|---|---|
| protocol | uniswap-v4 | uniswap-v4 | uniswap-v3 |
| NFT id | **3163432** | **3163431** | **6336662** |
| pool | v4 poolId `0xd316152008e12105594c32686b9d0401fd2c60ca4805c84f280069d218b771a8` | v4 poolId `0x91a9ef0a8dd231b99fa53a2a145a9e8bca7897e0d85852107abb997916f9f99c` | pool `0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a` |
| fee tier | 2% (20000, tickSpacing 400) | 0.75% (7500, tickSpacing 75) | 1% (10000) |
| tokens | currency0 = native ETH (`0x0`, 18 dec), currency1 = NOCK (16) | currency0 = USDC (6), currency1 = NOCK (16) | token0 = WETH (`0x4200…06`, 18), token1 = NOCK (16) |
| ticks | [65200, 71200] | [267825, 271050] | [70400, 74800] |
| in_range at check | ✓ (tick 70370) | ✓ (tick 269008) | ✓ (tick 70433) |
| v4 mint tx | `0x0a001ce91d08089c4a4b8529548de5220ffb1415653ee5b9c20658b97d214319` | `0x2613657172d6bec2c2d9585e5ce05203ec1f4f2958696f2935952ac2d95ba221` | (pre-existing NFT) |
| value at check | ~$156 (position compounded +75% since the $89.48 screenshot; split still 15.2% ETH / 84.8% NOCK) | ~$71.3 (screenshot $71.45 ✓) | ~$18.10 (screenshot $18.13 ✓) |
| unclaimed fees | 0 | 0 | 0.00799 NOCK = **$0.000166** (screenshot $0.000167 ✓) |

Other discovery facts:
- Old V4 NFT **#3126000 still owned but liquidity = 0** (dead). The wallet's 42 V4 NFTs = **2 active + 40 zero-liquidity dust**; 59 of 60 V3 NFTs (Base NPM) are dust too.
- NOCK spot (pool B slot0): **$0.020783**; third-party (DEXScreener/GeckoTerminal) agree ≈ $0.0206–0.0209.
- ETH/USD: **$2,370.85 from the canonical v3 WETH/USDC 0.05% pool `0xd0b53d9277642d899df5c87a3966a349a798f224`** (resolved via the verified Base v3 factory `0x33128a8fC17869897dcE68Ed026d694621f6FDfD` `getPool(WETH, USDC, 500)`; code verified; price sanity-checked). Both Chainlink aggregator address candidates returned empty `eth_call` → the approved fallback source is adopted as primary.
- Tracked wallet live balances: NOCK 110,648.08, USDC 134.41, WETH 0, ETH 0.0173.
- Pool state (for fixtures): pool B slot0 sqrtPriceX96 = 54957167047204907031948508154524863, tick 269008; pool A sqrtPriceX96 = 2672287759573224343549847773129, tick 70370; v3 pool slot0 tick 70433, fgGlobal0 = 88964247217657351611252137835053562972, fgGlobal1 = 9695026829229832421930202654646707105191.
- RPC quirks learned (validated by a pure-Python keccak implementation self-tested against known vectors): PoolManager event topics on Base — Swap `0x40e9cecb…`, ModifyLiquidity `0xf208f491…` (sender=POSM), ERC-6909 Transfer `0x1b3d7edb…`; POSM `poolKeys(bytes25)` selector `0x86b6be7d` returns the full PoolKey for poolId-prefix; v3 `positions(uint256)` 12-word layout confirmed; this pool's `ticks(int24)` returns an 8-word TickInfo (extended vs canonical 6-word) but words 0–3 follow the canonical layout (liqGross, liqNet, feeGrowthOutside0, feeGrowthOutside1).

Phase-1 discovery tasks are done; identifiers above feed `config.yaml positions:`. The
method used (kept for future re-discovery):

1. **Find the wallet(s) holding the new positions.**
   - `eth_getLogs` on the canonical V4 POSM for ERC-721 `Transfer` (topic0
     `0xddf252ad…`) with `from = 0x0` over the mint window → tokenIds + recipients;
     join each mint to its `ModifyLiquidity` log (same tx, `sender = POSM`) to get the
     poolId without trusting any pre-known pool list.
   - **Actual outcome:** both v4 mints went to the tracked wallet (EDT screenshot time
     resolves the "zero fresh mints" contradiction).
2. **Enumerate + classify the tracked wallet's 42 V4 POSM NFTs.**
   - `balanceOf` → enumerate owned tokenIds (Transfer logs to wallet); per NFT call
     `getPositionLiquidity(uint256)` (`0x1efeed33`) and `getPoolAndPositionInfo(uint256)`
     (`0x7ba03aad`) — classify active-liquidity vs dust/empty (liquidity == 0).
   - Includes resolving the fate of old NFT #3126000 (burnt? dust?).
3. **Decode each position.**
   - v4: StateView `getSlot0(poolId)` → sqrtPriceX96 + current tick; POSM
     `poolKeys(bytes25 poolId)` (`0x86b6be7d`) → currency0/currency1/fee/tickSpacing
     (verified against the known old pool); StateView `getPositionInfo` +
     `getFeeGrowthInside` for uncollected fees.
   - v3: Base canonical NPM `0x03a520b32C04BF3bEEf7BEb72E919cf822Ed34f1` (the
     ticket-quoted `0x46A15B…` was the wrong address); `positions(uint256)`
     (`0x99fbab88`) 12-word decode; pool via factory `getPool(token0, token1, fee)`;
     `slot0()` + `feeGrowthGlobal{0,1}X128()` + `ticks(int24)` for fees.
4. **Record** identifiers (table above). NOCK (16 dec), USDC (6 dec), WETH (18 dec).

## Phase 2 — Model Update

### 2.1 Data model (`specs/core/snapshot.yaml`, `frontend/src/types/snapshot.ts`)

- New `Position` schema (array field `positions` on `SnapshotData`):
  `position_id` (`"{protocol}:{network}:{nft_id}"`), `network` (`"base"` | …), `wallet`,
  `protocol` enum `uniswap-v4 | uniswap-v3 | aerodrome` (aerodrome kept for historical
  rows), `nft_id`, `pool_id` (v4 bytes32 or v3 pool address), `fee_tier_bps` (200 / 75 /
  100), `token0/token1` (address, symbol, decimals), `amount0/amount1` (current position
  amounts, decimal strings), `amount_usd`, `tokens_owed_0/1` (unclaimed fees),
  `fees_usd`, `lower_tick/upper_tick`, `lower_price/upper_price`, `in_range`,
  `liquidity_amount`, `is_price_source` (bool).
- `PoolMechanic.platform` enum gains real `UniswapV3`.
- `ConcentratedLiquidity` marked DEPRECATED in the schema (kept only so historical rows
  remain readable); the worker stops emitting it.
- `AssetAggregationSeries`: drop `tranche_1_nock/2/3`; add
  `positions_nock: [{ position_id, series: string[] }]` (wallet NOCK stays
  `safe_haven_series`; `active_protocol_series` = Σ position USD).
- `api.yaml`: `ClaimFeeAction.platform` enum and `/execute/record-harvest` `platform`
  enum gain `uniswap-v3` (only wired if v3 claiming is confirmed); doc comments updated
  to Uniswap-only portfolio.
- Migration note in CHANGELOG: old field names removed (`live_tranche_1_*`,
  `live_tranche_2_*`, `tranche_N_nock` series, `ConcentratedLiquidity` emission);
  `meta.version` → `2.0.0`.

### 2.2 Ingestion pipeline (`ingestion-worker/src/main.rs`)

- New config section (drives everything; replaces hardcoded ids/ticks/pool):

  ```yaml
  positions:
    - id: "uniswap-v4-nock-eth"
      protocol: "uniswap-v4"        # network: base
      nft_id: <from discovery>
      pool_id: "0x…"                # v4 poolId
      currency0: "0x…"              # per-pool ordering from poolKeys
      currency1: "0x…"
      is_price_source: false
    - id: "uniswap-v4-nock-usdc"
      protocol: "uniswap-v4"
      nft_id: <from discovery>
      pool_id: "0x…"
      currency0: "0x…"
      currency1: "0x…"
      is_price_source: true         # designated NOCK/USD spot pool
    - id: "uniswap-v3-weth-nock"
      protocol: "uniswap-v3"
      network: <from discovery>
      nft_id: <from discovery>
      npm_address: "0x…"            # canonical per network
  ```

- **Per-position reads.** v4: `getPositionLiquidity` + `getPoolAndPositionInfo` + StateView
  `getPositionInfo`/`getFeeGrowthInside` (existing selectors, now per position) + StateView
  `getSlot0(poolId)` for sqrtPrice/tick. v3: NPM `positions(uint256)` decode +
  factory `getPool` + `slot0()`.
- **Exact amount math** replaces the `v4_baseline_*` scaling heuristic: current token
  amounts from `liquidity L`, `sqrtP`, tick bounds (standard Uniswap formulas; the repo
  already has Q96/Q128 helpers `mul_div_q128` and tick-to-price conversion). In-range =
  `tickLower <= currentTick <= tickUpper`. Uncollected fees: v4 via fee-growth-delta math
  (existing), v3 directly from `tokensOwed0/1`. `RawOnChainData` replaces
  `live_tranche_*/live_uni_fee_*` with `positions: Vec<RawPosition>` (per-position amounts
  + fees) and drops Aerodrome fields.
- **Price sources.**
  - NOCK/USD: slot0 of the designated `is_price_source` pool (v4 NOCK/USDC) — direct
    USDC quote, no oracle chain. Renames `PRICE_SOURCE_RPC` value `"aerodrome_rpc"` →
    `"uniswap_rpc"` (new rows only; historical rows keep the old tag;
    `App.tsx SOURCE_LABELS` gains the label).
  - ETH/USD: replace Aerodrome `vAMM-WETH/USDC getReserves` with the **Chainlink ETH/USD
    aggregator on Base** (`AggregatorV3Interface.latestRoundData`, `0xfeaf968c`), chosen
    over the v4 NOCK/ETH pool (which only quotes NOCK/ETH — circular for ETH/USD) and
    over a single AMM pool because it is manipulation-resistant and oracle-grade.
    Staleness check: reject `updatedAt` older than 1h. Failure mode: failed/stale read →
    the cycle fails explicitly (zero-mock invariant, same as today). Exact aggregator
    address verified on-chain during implementation and recorded in
    ONCHAIN_INTEGRATION.md; documented fallback = Uniswap v3 canonical WETH/USDC 0.05%
    pool `slot0`.
  - `market_data.pool_address` re-pointed to the tracked NOCK pool for GeckoTerminal/
    DEXScreener OHLCV; if those providers do not list the new pool, backfill degrades
    fail-open (warnings only, RPC spot remains the primary close) — same as today.
- **Regime RSI SQL** switches from `pool_mechanics[0]` reserve ratio to
  `derived_metrics.total_value_vs_baseline.token_price_series[0]` — old and new snapshot
  rows both carry it, so the series stays continuous.
- **pool_mechanics**: emit one entry per tracked pool (platform `UniswapV4`/`UniswapV3`,
  slot0 tick + sqrt_price_x96, real fee_tier_bps, `volatile_assets` = the tracked wallet's
  amounts in that pool); Aerodrome entry removed. Semantics note in schema: these are the
  tracked wallet's amounts, not pool-wide reserves.
- **Derived metrics**: totals = wallet + Σ positions; deviation matrix buckets become
  Treasury / Uniswap LP (aggregate) / Cash (`TargetWeightsBps.uniswap_v4_lp` →
  `uniswap_lp`; old config key silently ignored by serde — documented); advice engine:
  tranche/Aerodrome wording removed, "RANGE EXIT FLASHPOINT" keyed on the
  price-source position's range instead of hardcoded ticks.
- **Claim-fees payload**: Aerodrome action deleted; one V4 collect per tracked v4 position
  with non-dust unclaimed fees, currency ordering taken from per-position config
  (`build_v4_collect_calldata` unchanged); v3 collect gated on Phase-1 confirmation
  (default: skipped, see Open Questions).
- **MCP** (`src/bin/mcp.rs`): tool descriptions updated (Uniswap-only); direct-RPC
  fallback stops presenting Aerodrome claimFees/getReserves as live state.

### 2.3 Frontend (`App.tsx`, `LiteCockpit.tsx`, `walletconnect.ts`, types)

- `types/snapshot.ts`: add `Position`; `AssetAggregationSeries` per-position series;
  platform enum + `UniswapV3`; `ClaimFeeAction.platform` + `uniswap-v3`.
- `App.tsx`: new **Positions panel** rendering N cards — pair name, protocol badge
  (v3/v4), fee tier ("2% / 0.75% / 1%"), range bounds, asset split %, USD value,
  unclaimed fees, in-range pill. NOCK-stack chart builds one area per position series +
  safe haven (no hardcoded tranche keys). "Tick Bracket Hypervisor" iterates N brackets;
  labels/pills lose Aerodrome wording; `SOURCE_LABELS` + `uniswap_rpc`.
- `LiteCockpit.tsx`: "UNCOLLECTED FEES (BOTH LPs)" → "(ALL POSITIONS)"; action rows already
  render generically from the payload (labels come from the worker).
- `walletconnect.ts`: no change unless Phase 1 puts a position on a non-Base chain
  (then add the chain to the wallet-switch list).

### 2.4 Docs

- `ONCHAIN_INTEGRATION.md`: §2 (Aerodrome fee mechanics) marked **HISTORICAL — tranche
  liquidated 2026-10-23**; new section for the Uniswap-only portfolio: discovered
  identifiers (wallets, NFT ids, poolIds, fee tiers, ordering), per-position read
  selectors, price-source decision + failure mode.
- `README.md` Strategic Tranches section + `HANDOFF.md` tranche references: updated to the
  Uniswap-only shape (minimal edits). `OPERATOR_GUIDANCE.md`: no tranche references found —
  no change expected (verify during implementation).
- `CHANGELOG.md`: Unreleased entry with the breaking-change migration notes.

## Key Decisions & Rationale

1. **Config-driven `positions:` registry** — repositioning is operator behavior that will
   recur; a config edit must be enough. Old constants (`v4_position_id`,
   `V4_TICK_*`, Aerodrome pool address) are retired.
2. **Exact on-chain amount math over baseline scaling** — `v4_baseline_*` anchors are a
   stale-able approximation and contradict the "mock data is a bug" invariant; per-position
   liquidity + slot0 math is exact and self-updating.
3. **Chainlink ETH/USD (verified on-chain) as the ETH/USD source** — the only ticket-approved
   option that actually yields ETH/USD (the new v4 NOCK/ETH pool quotes NOCK/ETH only);
   manipulation-resistant; explicit stale/failed-round behavior preserves zero-mock.
4. **Continuous price series via `token_price_series`** — switching the RSI spot source to a
   field every historical row already contains avoids a discontinuity when the Aerodrome
   pool disappears from `pool_mechanics`.
5. **Positions list instead of parallel tranche fields** — one N-capable schema replaces
   `live_tranche_1_*/2_*`, the hardcoded bracket, and the single hardcoded claim action.

## Learnings / Gotchas (for implementing agents)

- NOCK has 16 decimals (not 18) — every raw→human conversion divides by 1e16; USDC 1e6.
- On-chain tick orientation for the NOCK/USDC v4 pool is negated relative to display price
  (`tick_to_display_price`, L2965) — keep per-pool token-decimals ordering in mind; new
  pools may order currencies differently (USDC first vs NOCK first). Use `poolKeys`.
- `store_snapshot` dedups by `state_hash`; `compute_state_hash` is canonicalized — adding
  fields is safe, but the RSI/counterfactual SQL reads JSON paths that must keep existing
  names where history is consumed.
- Old snapshot rows keep the `aerodrome_rpc` provenance tag and the Aerodrome
  `pool_mechanics` entry — queries and UI labels must tolerate both shapes.
- The tracked wallet's 42 V4 NFTs include dust; never emit a zero-liquidity position as a
  "live" card — classify and skip (record in discovery notes).
- Build/test protocol: `SQLX_OFFLINE=true` for worker builds; `unset GITHUB_TOKEN` before
  any `gh` command; brain untouched (no tranche coupling).

## Implementation Notes (post-implementation, 2026-10-24)

- **Pre-existing spec-lint blockers fixed (drive-by, on main):** `specs/core/snapshot.yaml`
  line 1 lacked the colon (`openapi 3.0.3` → `openapi: 3.0.3`) — redocly parsed it as garbage
  and any spec change was unlintable; `specs/core/api.yaml` had `ClaimFeesPayload` /
  `ClaimFeeAction` at the `components:` root instead of `components.schemas:` (unresolved-ref
  errors). Both fixed; `make validate-specs` now passes with only pre-existing warnings
  (info-license, localhost server, missing 4XX responses, unused envelope components).
- **`wit-bindgen` gotcha:** the Makefile's `cargo install wit-bindgen` targets the library
  crate ("no binaries"); the CLI is `cargo install wit-bindgen-cli` (v0.63.0 installs a
  `wit-bindgen` executable). Installed in-sandbox; Makefile left unchanged (out of scope).
- **Rust toolchain in fresh sandboxes:** `rustup target add wasm32-wasip1` is required before
  `make test-brain`/`make dist` (brain builds, not the worker).
- **`client.errors.test.ts` needed no change:** it contains zero platform/source/tranche
  references (it tests HTTP error mapping only); the ticket's assumption was wrong.
- **Grep include-paths must carry the repo root prefix** (`nocknock/frontend/src/**`, not
  `frontend/src/**`) or the search silently matches nothing — cost a few false negatives.
- **Release-build hygiene:** `UNISWAP_V3_NPM_BASE` was referenced only from `#[cfg(test)]`
  fixtures and warned dead-code in `--release`; moved into the tests module. Live v3 reads
  take `npm_address` from config and fail explicitly when missing (zero-guess invariant).
- **Final validation record:** worker 47/47, frontend 28/28, brain 3/3, `make dist` packages
  all 4 artifacts, live snapshot fetch returned the 3 positions at screenshot magnitudes
  (B $71.11 vs $71.45), both v4 collect sims succeed, cockpit renders 3 cards
  (screenshot `02-cockpit-three-positions.png`).

## Post-review fixes (2026-10-24, after first PR proposal was rejected)

The rejection carried no written feedback; the two imperfections flagged during the
live inspection were treated as the feedback and fixed:

1. **v3 collect added to the claim payload.** The payload was v4-only while the velocity
   sandbox labeled the v3 "CLAIM_RECOMMENDED" — promising an action the payload didn't
   carry. The approved spec permitted v3 collect once discovery confirmed a reachable
   canonical NPM (it did: Base `0x03a520b3…Ed34f1`). Implementation: `collect((uint256,
   address,uint128,uint128))` selector **0xfc6f7865** (keccak-verified; the same
   staticcall used as Phase-1 fee-math ground truth), `type(uint128).max` maxes, expected
   amounts from the snapshot's `tokens_owed_*`. Live eth_call ground truth: returns
   (0, 1654339369283004) = 0.165434 NOCK — exactly the snapshot value. The per-action
   expected/valuation/simulation tail was extracted into `finish_claim_action`, shared
   by both the v4 and v3 loops; NO_CLAIMABLE_POSITIONS guard now requires BOTH loops empty.
2. **Hypervisor shared-USD axis.** Bracket prices are token0-per-NOCK; ETH-pool rows
   (~8e-6 ETH/NOCK) were invisible on an axis scaled for the USDC pool (0.024). Fix:
   derive the worker's live ETH/USD client-side from `pool_mechanics` volatile_assets
   (`reserve_usd/reserve_amount` — same reads the worker valued with), convert ETH rows
   to USD/NOCK, add a spot ReferenceLine, disclose the conversion in the footer.
   Gotcha: `pool_mechanics` is a SIBLING of `derived_metrics` on the snapshot data —
   `metrics.pool_mechanics` is undefined (first attempt silently no-opped).
3. **Honest cross-unit distances.** The worker's `distance_bps` for ETH-pool rows was a
   meaningless cross-unit number (-9993/-9996 sentinels). Now `Option<i32>`: Some only
   for USD-denominated (USDC) brackets, else null; schema `nullable: true`; TS
   `number | null`. Frontend computes honest USD-normalized distances itself.

**Post-fix validation:** worker 48/48 (new `test_v3_collect_calldata_golden_vector`),
frontend 28/28, validate-specs green; live payload = 3 actions (2× v4 + 1× v3), all
simulation=success, gross $0.04, net +$0.0348; hypervisor renders all 3 brackets on the
shared axis (screenshot `04-hypervisor-shared-axis.png`).

**Range-edge reading for the operator (arithmetic-verified, spot $0.02062 ≈ 115,300
NOCK per ETH, NOCK-per-ETH lens matching the cards):** the ETH pools bracket NOCK/ETH
from opposite sides. C's band [114,155–177,305 NOCK/WETH] — spot hugs its strength
floor: C exits on ~1% further NOCK **strengthening** vs ETH (its weak-side cushion is
+54%). A's band [67,843–123,609 NOCK/ETH] — spot sits 7.2% below its weak ceiling: A
exits on ~7% NOCK **weakening** vs ETH (strength-side cushion −41%). B (USDC) is
mid-range: exits at −17.8% (weak) or +13.4% (strong). Net: a NOCK rally takes C out of
range first; a NOCK dip takes A out first, then B.
