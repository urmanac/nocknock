# On-Chain Integration Protocol & Verification Findings

This document captures the verified on-chain mechanics, smart contract functions, selector analysis, and fee dynamics discovered during live RPC inspection on Base mainnet.

---

## 1. Core Addresses & Contracts (Base Mainnet)

| Component | Address / Identifier | Description |
|---|---|---|
| **Tracked Wallet** | `0xeE114cC97eC5adeB34bA5645e315B497D8487a45` | Sovereign operator address |
| **NOCK Token** | `0x9B5E262cF9bb04869ab40b19AF91D2dc85761722` | Sovereign asset (16 decimals) |
| **USDC Token** | `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913` | Settlement currency (6 decimals) |
| **Aerodrome Pool (vAMM-USDC/NOCK)** | `0x85f1aa3a70fedd1c52705c15baed143e675cd626` | **HISTORICAL — tranche liquidated 2026-10-23.** Not a position; still the OHLCV history source (`market_data.pool_address`) |
| **Aerodrome Implementation** | `0xa4e46b4f701c62e14df11b48dce76a7d793cd6d7` | (historical) Implementation behind EIP-1167 proxy |
| **Uniswap V4 Position Manager** | `0x7c5f5a4bbd8fd63184577525326123b519429bdc` | Position NFT manager (all tracked v4 positions) |
| **Uniswap V4 StateView** | `0xa3c0c9b65bad0b08107aa264b0f3db444b867a71` | Read-only pool/position state |
| **Uniswap V4 Pool Manager** | `0x498581ff718922c3f8e6a244956af099b2652b2b` | Core singleton pool state |
| **Uniswap V4 NFT #3126000** | liquidity = 0 (2026-10-24 read) | **RETIRED** — old Tranche 1 position, now dust |
| **Uniswap V3 NPM (Base canonical)** | `0x03a520b32C04BF3bEEf7BEb72E919cf822Ed34f1` | v3 position manager (verified on-chain) |
| **Uniswap V3 Factory (Base)** | `0x33128a8fC17869897dcE68Ed026d694621f6FDfD` | Pool resolution via `getPool(token0, token1, fee)` |
| **ETH/USD price pool (v3 WETH/USDC 0.05%)** | `0xd0b53d9277642d899df5c87a3966a349a798f224` | Factory-resolved + price-verified ($2,373 at discovery) |

---

## 2. Aerodrome Fee Harvesting Mechanics — **HISTORICAL (tranche liquidated 2026-10-23)**

> **The Aerodrome tranche was liquidated by the operator on 2026-10-23** and the
> portfolio repositioned entirely into Uniswap concentrated liquidity (see §5). The
> mechanics below are preserved as historical verification record only — the worker no
> longer reads or claims anything on Aerodrome, and no Aerodrome transactions are built.
>
> **Superseded in v0.5.0 (noted 2026-11-14):** the worker reads `aerodrome` positions
> again (Aero volatile pools, including LP staked in a gauge), and the claim payload
> builds a `claimFees()` action for each. On the new Aero pools `claimFees()` reverts;
> since v0.5.2 that is reported as "not LP-claimable", not as an RPC outage. See §7.

### Verification & Selector Analysis
* Aerodrome pool `0x85f1aa3a70fedd1c52705c15baed143e675cd626` is an EIP-1167 minimal proxy pointing to `0xa4e46b4f701c62e14df11b48dce76a7d793cd6d7`.
* The fee collection function is:
  ```solidity
  function claimFees() external returns (uint256 claimed0, uint256 claimed1);
  ```
  Selector: **`0xd294f093`** (`keccak256("claimFees()")[0:4]`).

### Live Real-Time Query
When called as a read-only `eth_call` with `from: <wallet_address>` and `to: <pool_address>`, the contract computes and returns exact uncollected fees in the return bytes:
* Offset `0x00..0x20` (first 32 bytes): `claimed0` (USDC, 6 decimals)
* Offset `0x20..0x40` (second 32 bytes): `claimed1` (NOCK, 16 decimals)

**Verified live return:**
* `c0 = 0x3e8dc` = `256,220` raw units = **`0.25622 USDC`**
* `c1 = 0x134d13b2ef42a01` = `86,924,344,947,911,169` raw units = **`8.69243 NOCK`**

### Fee Indices & Claim Event
* Prior claim transaction: `0xac733f8b66b98cd56afc82a30302a88ec5479cad0761293e2377a38f6e03d946`
* Block timestamp: `1791601001` (`2026-10-10 02:56:41 UTC`)
* Event emitted: `Claim(address indexed sender, address indexed recipient, uint256 amount0, uint256 amount1)`
  (topic: `0x865ca08d59f5cb456e85cd2f7ef63664ea4f73327414e9d8152c4158b0e94645`)

---

## 3. Uniswap V4 Position & Auto-Compounding Mechanics

### Position Manager Selectors
* `getPositionLiquidity(uint256)`: `0x1efeed33`
* `getPoolAndPositionInfo(uint256)`: `0x7ba03aad`
* `modifyLiquidities(bytes,uint256)`: `0xdd46508f`

### Auto-Compounding Event Analysis
* Transaction hash: `0x6e1b1b97d64b28eaa3b7e51e56f0d9366f783785dfd24c53c17b03e983b2ddf2`
* Action sequence decoded: `Actions [0, 18, 18]`
  * `Action 0`: `INCREASE_LIQUIDITY` (added liquidity to NFT `#3126000`)
  * `Action 18`: `SETTLE_PAIR` / `TAKE_PAIR`
* **Result**: All previously accrued trading fees (`0.470 USDC` and `5.27 NOCK`) were automatically compounded into the position's active liquidity.
* Active liquidity increased from `65,720,426,387,913` to **`81,297,590,130,616`** (+23.7%).
* Unclaimed fees on V4 were reset to zero at the moment of modification and are now accreting afresh.

---

## 4. Derived Valuation Architecture — **HISTORICAL (two-tranche model, retired v2.0.0)**

> Superseded by the per-position model in §5. Kept for the historical record of how the
> pre-repositioning tranches were valued.

| Tranche | Formula / Source | Live Value |
|---|---|---|
| **Tranche 1 (Uniswap V4)** | `v4_liq / 65719361234409.0 * base` | ~4,953.5 NOCK + ~29.23 USDC |
| **Tranche 2 (Aerodrome)** | `user_lp / total_lp * reserves` | ~5,810 NOCK + ~156.64 USDC |
| **Tranche 3 (Treasury)** | `alchemy_getTokenBalances` | ~95,165.5 NOCK + ~172.79 USDC |
| **Claimable Fees (Aero)** | `eth_call(from: wallet, claimFees())` | **0.2562 USDC + 8.6924 NOCK** |
| **Claimable Fees (V4)** | Accreting since add-liquidity block | 0.00 USDC + 0.00 NOCK |
| **Total Sovereign NOCK** | T1 + T2 + T3 + Aero Fee NOCK + V4 Fee NOCK | **~105,938 NOCK** (+5,938 above floor) |

---

## 5. Uniswap-Only Portfolio (v2.0.0) — Verified Discovery Record

Phase-1 RPC discovery (2026-10-24, Base mainnet). All identifiers below were read from
chain responses (mint logs joined to PoolManager `ModifyLiquidity` events by tx hash,
POSM `poolKeys`, StateView reads, NPM decodes) — none are guessed. The tracked wallet
holds 43 V4 POSM NFTs (3 active + 40 zero-liquidity dust) and 60 Base V3 NPM NFTs
(1 active + 59 dust); old V4 NFT #3126000 is owned but liquidity = 0.

| | Position A | Position B | Position C | Position D |
|---|---|---|---|---|
| protocol | uniswap-v4 | uniswap-v4 | uniswap-v3 | uniswap-v4 |
| wallet | `0xeE114cC97eC5adeB34bA5645e315B497D8487a45` | same | same | same |
| NFT id | **3163432** | **3163431** | **6336662** | **3163450** |
| pool | v4 poolId `0xd316152008e12105594c32686b9d0401fd2c60ca4805c84f280069d218b771a8` | v4 poolId `0x91a9ef0a8dd231b99fa53a2a145a9e8bca7897e0d85852107abb997916f9f99c` | pool `0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a` | v4 poolId `0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13` |
| fee tier | 2% (tickSpacing 400) | 0.75% (tickSpacing 75) | 1% | 3.00% (fee 30000, tickSpacing 300) |
| tokens | currency0 = native ETH (`0x0`, 18 dec) / currency1 = NOCK (16) | currency0 = USDC (6) / currency1 = NOCK (16) | token0 = WETH (18) / token1 = NOCK (16) | currency0 = USDC (6) / currency1 = NOCK (16) |
| ticks | [65200, 71200] | [267825, 271050] | [70400, 74800] | [262200, 282300] (wide raft) |
| v4 mint tx | `0x0a001ce91d08089c4a4b8529548de5220ffb1415653ee5b9c20658b97d214319` (2026-10-24 01:14:47 UTC) | `0x2613657172d6bec2c2d9585e5ce05203ec1f4f2958696f2935952ac2d95ba221` (2026-10-24 01:12:37 UTC) | (pre-existing NFT) | `0x6cdabada1ff062b6d3b4ba87bfe04158dd8b14a91e93cabe11dc3e35b587e89e` (block 53008241); operator compounded post-mint (+143.7e12 add, +1.2e12, −7.7e12 partial remove → L = 185,454,734,478,733 at 02:37 UTC) |

Position D (added by the 2026-10-24 change request): identified via the mint's
`ModifyLiquidity` event (poolId + ticks) and cross-checked against POSM
`getPoolAndPositionInfo` (fee 30000 = 3.00%, tickSpacing 300, packed poolId/ticks) and
`ownerOf` (tracked wallet). NOT a price source — B stays the single NOCK/USD source.

> **Superseded in v0.5.0 (noted 2026-11-14):** B (#3163431) is retired and its pool has
> had no in-range liquidity since the 2026-11 liquidation. Position D (#3163450) is now
> the NOCK/USD price source (`is_price_source: true` in the config).

### Per-position read pipeline (all reads fail the cycle explicitly — zero-mock)

- **v4:** POSM `getPositionLiquidity(uint256)` (`0x1efeed33`) → liquidity;
  StateView `getSlot0(bytes32)` (`0xc815641c`) → sqrtPriceX96 + tick;
  POSM `poolKeys(bytes25)` (`0x86b6be7d`, first 25 bytes of the poolId) → canonical
  currency ordering + fee tier (cross-validated against config);
  StateView `getPositionInfo(bytes32,address,int24,int24,bytes32)` (`0xdacf1d2f`) →
  feeGrowthInside{0,1}Last; StateView `getFeeGrowthInside(bytes32,int24,int24)`
  (`0x53e9c1fb`) → current inside growth. Uncollected =
  `(inside_now − inside_last) × L / 2^128` (Q128 wrap-safe math as in v0.1.0).
- **v3:** NPM `positions(uint256)` (`0x99fbab88`) → 12-word decode (token0, token1,
  fee, tickLower, tickUpper, liquidity, feeGrowthInside{0,1}Last, tokensOwed{0,1});
  pool.slot0(); pool.feeGrowthGlobal{0,1}X128() (`0xf3058399` / `0x46141319`);
  pool.ticks(tick) → word [2] = feeGrowthOutside0, word [3] = feeGrowthOutside1.
  Uncollected = `(global − outside_l − outside_u − inside_last) × L / 2^128 + tokensOwed`.
  **Ground-truth validated**: an NPM `collect((uint256,address,uint128,uint128))`
  staticcall (`0xfc6f7865`) returned exactly the same amounts as the decode+delta math.
- **Amounts:** concentrated-liquidity math from `L`, `sqrtPriceX96` and tick bounds
  (in-range splits both tokens; out-of-range is single-sided).
- **Display prices:** `1.0001^(−tick) × 10^(dec1−dec0)` (token0-per-token1 human
  units); the frontend inverts sub-cent orientations for readability.

### Price sources (and their failure modes)

- **NOCK/USD:** slot0 of the designated `is_price_source` position (the v4 NOCK/USDC
  pool) → `USDC per NOCK = 1.0001^(−tick) × 1e10`. Failure: StateView read failure or
  zero price fails the whole cycle explicitly (no fabricated spot).
- **ETH/USD:** canonical Uniswap v3 WETH/USDC 0.05% pool slot0
  (`0xd0b53d92…f224`), resolved on-chain via the verified v3 factory `getPool` and
  sanity-checked (≈ $2,373 at discovery; cross-checked against third-party quotes).
  The design originally preferred a Chainlink aggregator, but both candidate feed
  addresses returned empty `eth_call` responses, so the pre-approved fallback was
  adopted as primary. Failure: read failure or zero price fails the cycle explicitly.
- **Provenance:** contemporaneous price rows now carry source `uniswap_rpc`
  (historical rows keep `aerodrome_rpc`); the RSI regime SQL reads each snapshot's
  stored `token_price_series[0]`, so the time series is continuous across the change.

### Discovery method (for the next repositioning)

1. `eth_getLogs` on the V4 POSM for ERC-721 `Transfer` from `0x0` over the mint window;
   join each mint to its PoolManager `ModifyLiquidity` log (same tx, sender = POSM)
   to get the poolId. PoolManager event topics verified on Base: Swap `0x40e9cecb…`,
   ModifyLiquidity `0xf208f491…`, ERC-6909 Transfer `0x1b3d7edb…`.
2. `poolKeys(bytes25)` per poolId → currencies + fee; `getPositionLiquidity` per NFT →
   active vs dust classification.
3. For v3: check the canonical NPM per network (Base `0x03a520b3…`; the address quoted
   in the original ticket, `0x46A15B…`, was the wrong contract), decode
   `positions(uint256)`, resolve the pool via factory `getPool`.
4. Update `config.yaml positions:` (identifiers + ticks for v4) and re-verify with the
   magnitude cross-check against the operator's UI before trusting the pipeline.

---

## 6. Aero Transition (Aerodrome → aero.xyz) — Observed Record, 2026-11-14

Aerodrome and Velodrome merged into **Aero** (front end `aero.xyz`), live since
2026-10-21; legacy contracts stopped emitting rewards at launch. NockNock holds no
Aero LP, so this affects only **where NOCK buys are executed** (the accumulation plan)
and one third-party history feed.

> **Superseded in v0.5.0 (noted 2026-11-14):** the operator holds staked Aero LP, and
> NockNock tracks Aero positions again (see §7). The AERO that staked LP earns is read
> since v0.6.0 (#93, §9).

> **Verification status.** Everything below was read from Base mainnet. No official
> Aero contract list was found, so the *identity* of these contracts is inferred:
> `0x0fef9e88…` is labelled the registered Aero router by a third-party analytics
> repository only. Treat the addresses as observed, not endorsed, and re-verify
> before building any transaction against them.

### Evidence transaction

`0x5d0c02d9929f49f71526e0c8d91d1bd99ec19f116befc285d67f6bf9412a3f0e` — block 57228848,
2026-11-14T06:02:25Z, status success, sent by the sovereign wallet from aero.xyz.
First tranche of plan `2026-11-reentry`.

| Item | Value |
|---|---|
| Router (`to`) | `0x0fef9e88a7b76bcdbf451780c6774fe32965ee98` |
| Selector | `0x3593564c` — `execute(bytes,bytes[],uint256)` (universal-router style) |
| Pool factory (all three hops) | `0x9cb220581b91b0c83223012fe69688b92ad07a76` |
| Wallet net flows | −20.000000 USDC, +1597.934048 NOCK |
| Effective price | $0.01251616 per NOCK (engine spot $0.01229, +1.8%) |

Route (concentrated pools; `fee()` and `tickSpacing()` as returned on-chain, units per
that factory's convention and not independently confirmed):

| Hop | Pool | Pair | `fee()` | `tickSpacing()` |
|---|---|---|---:|---:|
| 1 | `0xd448990883e3bf9cdd8ebca368a3ecf9d629aaf7` | USDC / USDT | 8 | 1 |
| 2 | `0x2dbe0e264c7c246e05c6614794899937f264cf7b` | WETH / USDT | 435 | 100 |
| 3 | `0x957256c98d61a714081de2ee70ae3a96c9b40874` | WETH / NOCK | 9700 | 200 |

If `fee()` is in hundredths of a basis point (the Uniswap-v3 convention these pools
otherwise follow), the hops charge 0.0008%, 0.0435% and 0.97%: the WETH/NOCK hop
accounts for most of the 1.8% paid over spot.

*Confirmed 2026-11-16 for the WETH/NOCK pool (§8, "Swap fee units"): `fee()` is in
millionths, so 9700 is 0.97%. The other two hops were not re-measured.*

### Legacy Aerodrome pool

The NOCK/USDC vAMM `0x85f1aa3a70fedd1c52705c15baed143e675cd626` (factory
`0x420dd381b31aef6683db6b902084cb0ffece40da`) was **not** on the route. At the same
time it still held about 379,809 USDC and 30.96M NOCK (implied $0.012268) and emitted
18 logs in the preceding ~9,000 blocks, so it is alive but no longer where aero.xyz
sends a NOCK buy. The legacy router address still has code; whether it still executes
swaps was not tested.

### Consequences for NockNock

- **Receipt verification is unaffected.** `record-accumulation` nets USDC/NOCK Transfer
  logs touching the wallet and never inspects the router, so multi-hop and
  universal-router trades record correctly (the transaction above did).
- **Execution cost is higher than first estimated.** The design note assumed the direct
  vAMM (~0.3% fee plus negligible impact). The observed route cost ~1.8% over spot.
- **Phase 2 cannot use the original encoding.** `Router.swapExactTokensForTokens` and
  `pool.getAmountOut` on the vAMM do not describe how Aero routes. See the design note
  §8.5.
- **History feed dependency.** `market_data.pool_address` is pinned to the legacy vAMM
  for OHLCV/RSI history because it was still the highest-volume NOCK/USDC market. If
  that pool goes quiet the feed degrades fail-open (warnings, widening gaps); re-point
  it then. The live spot is unaffected: it comes from the Uniswap v4 3% pool.

---

## 7. Aerodrome Tranche Restored (v0.5.0) — Read Path and What Is Unverified

The operator intends to hold a full-range NOCK/USDC position again as the long-term
core (it cannot go out of range, so it never forces a re-range). The worker tracks it
with `protocol: aerodrome` (issue #88). The measurements below were taken on the legacy
pool, which is what the adapter was first written against; the pool actually tracked
is the Aero app's (see "Which pool is tracked" and "First live deposits").

### Pool state, read 2026-11-14 (block 57409285)

Legacy vAMM `0x85f1aa3a70fedd1c52705c15baed143e675cd626`:

| Read | Value |
|---|---|
| `getReserves()` | 369,321.72 USDC / 31,839,446 NOCK (implied $0.01160) |
| `totalSupply()` | 342542824101829799 |
| Activity | 61 swaps in the preceding ~20,000 blocks (~11 h), each emitting a fee event |
| GeckoTerminal 24h | $111,742 volume, 506 transactions, $735,609 TVL |
| Wallet LP balance | 0 |

For comparison, the new Aero volatile NOCK/USDC pool `0x770fe8fc…` held about $1,809.

### Read path (same selectors §2 verified against this pool)

- `token0()` `0x0dfe1681`, `token1()` `0xd21220a7`, `getReserves()` `0x0902f1ac`,
  `totalSupply()` `0x18160ddd`, `balanceOf(wallet)` `0x70a08231`.
- Position amounts = `balanceOf / totalSupply` of each reserve.
- Unclaimed fees = `claimFees()` `0xd294f093` as an `eth_call` from the wallet (skipped
  when the wallet holds no LP).
- Spot = reserve1/reserve0, expressed as a synthetic sqrtPriceX96 so the shared pricing
  and sizing code is unchanged. The position's ticks are Uniswap's MIN/MAX (full range).

### Not verified

- **Which pool is tracked (settled 2026-11-14).** `app.aero.xyz` does not know the
  legacy pool: `https://app.aero.xyz/deposit/8453-0x85f1aa3a…d626` answers "Pool not
  found". Its pool list offers a Basic Volatile NOCK/USDC pool
  `0x770fE8Fc9248c6382950A86E39a9391B41D59e0A` and the CL200 WETH/NOCK pool. The operator
  uses the Aero app, so the shipped config tracks `0x770fE8Fc…` and the legacy pool is
  no longer a tracked position (it remains the OHLCV history source). Deposit links are
  `https://app.aero.xyz/deposit/8453-<pool>`. The retired aerodrome.finance app still
  resolves but must not be linked.
- **New-pool differences.** On `0x770fE8Fc…`, `claimFees()` and `stable()` revert;
  `token0`, `token1`, `getReserves`, `totalSupply` and `balanceOf` behave as on the
  legacy pool. The reader treats the `claimFees()` revert as "no LP-claimable fees".
- **What an unstaked LP earns now.** Swaps on the legacy pool still emit fee events,
  but reward emissions stopped at the merger and current fee distribution was not
  confirmed. (aero.xyz states that new-Aero pool revenue goes to AERO stakers; its pool
  page lists separate AERO and fee reward rates for depositors.)
- Rewards earned by staked LP (AERO emissions) were not read until v0.6.0. They are now
  read from the gauge with `earned(address)` and valued (issue #93, §9).

### First live deposits (2026-11-14, operator, via app.aero.xyz)

| | USDC/NOCK Basic Volatile | WETH/NOCK CL200 |
|---|---|---|
| Tx | `0x127c4f4f…f185f0` (block 57425187) | `0x80277e2b…c93ddb` (block 57425824) |
| In | 31.986644 USDC + 2,755 NOCK | 0.081995 ETH + 2,755 NOCK |
| Pool | `0x770fE8Fc9248c6382950A86E39a9391B41D59e0A` | `0x957256c98d61A714081DE2EE70ae3A96c9b40874` |
| Receipt | 29684955447174 LP tokens | NFT #2914490 on `0x4857db576ee0e45f10525ec59527bfd10beec60b` |
| Staked in | gauge `0xc82782ee66d82c4d19db0429652091a085083526` | gauge `0x3bf70c880adc8370917994ab0df68e763b7a60fd` |
| NockNock | tracked (`gauge_address`) | tracked since v0.6.0 with `protocol: aerodrome-cl` (issue #91, §8); not tracked before |

Both went through the Aero router `0x0fef9e88…ee98` (`execute`, §6), which staked the
receipt automatically. For the volatile pool `gauge.balanceOf(wallet)` returned the
minted amount exactly and `gauge.stakingToken()` is the pool; the wallet's share was
3.38% of reserves, equal to what was deposited.

---

## 8. Aero Concentrated-Liquidity (Slipstream) Positions (v0.6.0, issue #91) — Verified Record

The worker tracks an Aero CL position with `protocol: aerodrome-cl`
(`ingestion-worker/src/adapters/aero_cl.rs`). Everything below was read with `eth_call`
through `https://base-rpc.publicnode.com` on 2026-11-16 (workstation date 2026-11-15),
pinned to **block 58061668** (`0x375f364`, 2026-11-16T04:18:29Z, hash `0xb3cb8ebf…4a92f9`)
unless another block is named. Selectors were computed from the signatures shown and each
was checked against the response it produces. The raw responses are the fixtures in
`ingestion-worker/src/tests_aero_cl.rs`.

### Contracts

| Role | Address | How it was tied to the others |
|---|---|---|
| Position manager (NFT "Slipstream V3 Deposit") | `0x4857db576ee0e45f10525ec59527bfd10beec60b` | `pool.nft()` and `gauge.nft()` both return it |
| Pool, WETH/NOCK CL200 | `0x957256c98d61A714081DE2EE70ae3A96c9b40874` | `factory.getPool(WETH, NOCK, 200)` and `gauge.pool()` return it |
| Pool factory | `0x9cb220581b91b0c83223012fe69688b92ad07a76` | `pool.factory()` and position manager `factory()`; the factory of §6's route |
| Gauge of that pool | `0x3bf70c880adc8370917994ab0df68e763b7a60fd` | `pool.gauge()` returns it |
| Gauge implementation | `0x89e91a960154b1ed6c093039a8f0725ab67d14e0` | the gauge's code is a 45-byte EIP-1167 proxy to it |
| Gauge factory / voter | `0xf437f62053acb121665b8fdbfea8f214fb8e589d` / `0x382409fad8503fbcdbb27a4211551fa03e884078` | `gauge.gaugeFactory()` / `gauge.voter()` |

### Selectors used by the adapter

| Call | Selector | On | Used for |
|---|---|---|---|
| `positions(uint256)` | `0x99fbab88` | position manager | tokens, tick spacing, ticks, liquidity |
| `ownerOf(uint256)` | `0x6352211e` | position manager | who holds the NFT |
| `collect((uint256,address,uint128,uint128))` | `0xfc6f7865` | position manager | fees of an UNSTAKED position (simulated from the wallet) |
| `slot0()` | `0x3850c7bd` | pool | sqrt price and tick |
| `liquidity()` | `0x1a686502` | pool | in-range liquidity (pool liveness) |
| `nft()` | `0x47ccca02` | pool | the configured pool belongs to the NFT's position manager (may revert: see "Whose pool it is") |
| `token0()` / `token1()` | `0x0dfe1681` / `0xd21220a7` | pool | the configured pool is the NFT's pool |
| `tickSpacing()` | `0xd0c93a7c` | pool | same check |
| `fee()` | `0xddca3f43` | pool | current swap fee (dynamic) |
| `gauge()` | `0xa6f19c84` | pool | the only contract the NFT may be staked in |
| `stakedContains(address,uint256)` | `0xc69deec5` | gauge | proof that the wallet staked the NFT |

### Layouts (they differ from Uniswap v3)

`positions(2914490)` returns 12 words. Word 4 is the pool's **tick spacing**, where
Uniswap v3 has the fee tier:

| Word | Field | Value |
|---:|---|---|
| 0, 1 | nonce, operator | 0, zero address |
| 2 | token0 | `0x4200…0006` (WETH) |
| 3 | token1 | `0x9b5e…1722` (NOCK) |
| 4 | tickSpacing | 200 |
| 5 | tickLower | 73400 |
| 6 | tickUpper | 74800 |
| 7 | liquidity | 57859113596792372851 (`0x322f4dd43fbc87e73`) |
| 8, 9 | feeGrowthInside0/1LastX128 | `0x574a55555810582e165fb0bd53ef`, `0x212d3d750e1bdf94505a584b88be1f5` |
| 10, 11 | tokensOwed0, tokensOwed1 | 0, 0 |

Pool `slot0()` returns **6 words** (Uniswap v3 returns 7; there is no `feeProtocol`):
sqrtPriceX96 `0x2871f7214287269e35ad48582c`, tick 74002, observationIndex 207,
observationCardinality 900, observationCardinalityNext 900, unlocked 1.

Pool `ticks(int24)` (`0xf30dba93`) returns **10 words**: liquidityGross, liquidityNet,
stakedLiquidityNet, feeGrowthOutside0X128, feeGrowthOutside1X128, rewardGrowthOutsideX128,
tickCumulativeOutside, secondsPerLiquidityOutsideX128, secondsOutside, initialized. Fee
growth is at words 3 and 4, not 2 and 3 as in Uniswap v3, so the v3 fee-growth maths must
not be pointed at this pool. The adapter does not read `ticks()` at all.

Negative ticks arrive sign-extended over the whole word (`ff…fe98` is -360; seen on NFT
#3117536 below and on the ETH/USD price pool's own `slot0`, tick -201250 at this block).
`parse_hex_word_i32` used to reject such a word; it and the calldata encoder
(`int24_calldata_word`) were corrected in the same change.

### Whose pool it is (block 58074373)

`pool_address` comes from config, so the adapter proves it is the NFT's pool before it
values anything at that pool's price. Read at **block 58074373** (`0x3762505`,
2026-11-16T05:00:50Z, hash `0x381fd9a2…ee2150`):

| Pool | `token0` / `token1` | `tickSpacing()` | `nft()` |
|---|---|---:|---|
| Aero CL200 WETH/NOCK `0x957256c9…0874` | WETH / NOCK | 200 | `0x4857db57…c60b` (the position manager) |
| Uniswap v3 WETH/NOCK 1% `0x71a4a6f6…ad9a` | WETH / NOCK | 200 | **reverts** (error code 3, "execution reverted") |
| Aero CL100 WETH/USDC, current deployment `0x2a430beb…bc24` | WETH / USDC | 100 | `0x4857db57…c60b` |
| Aero CL100 WETH/USDC, legacy deployment `0xb2cc224c…dc59` | WETH / USDC | 100 | `0x82792268…5b72` (the legacy position manager) |

- The Uniswap v3 pool of the same pair has the same tokens **and the same tick spacing**
  as the CL200 pool, so tokens and spacing alone would accept it as `pool_address`.
- `getPool(address,address,int24)` (`0x28af8d0b`) for (WETH, USDC, 100) returns
  `0x2a430beb…bc24` on the current factory `0x9cb22058…7a76` and `0xb2cc224c…dc59` on the
  legacy factory `0x5e7bb104…809a` (that pool's `factory()` returns it): two live pools
  with one identity, each with its own position manager. No NOCK pair has a legacy pool
  at this block: the legacy factory's `getPool` returns the zero address for WETH/NOCK
  and USDC/NOCK at tick spacings 100, 200 and 2000, while the current factory returns
  `0x957256c9…0874` for (WETH, NOCK, 200).

So the adapter reads `pool.nft()` first and requires it to equal `npm_address`. A revert
(a Uniswap v3 pool, or an address that is not a pool) and a different position manager are
both "Configuration error: position …: pool_address … is not …", raised before any amount
is computed. A node that does not answer is a failed read, not a configuration error.
Then the pool's tokens and tick spacing must be the NFT's.

### Amounts at block 58061668

From liquidity, ticks 73400..74800 and the pool's sqrt price, with the maths shared with
the Uniswap v3 adapter (`evm::concentrated_amounts_raw`):

- **0.055908 WETH + 6,945.83 NOCK**, in range (tick 74002).
- Valued at the snapshot's own prices at that block (ETH/USD $1,820.91 from the
  `eth_price_pool` slot0, NOCK/USD $0.0112868 from the v4 3% pool): **$180.20**.
- The deposit (block 57425824, 2026-11-14T16:59:00Z; `gauge.depositBlock(2914490)` returns
  that block) was 0.081995 ETH + 2,755 NOCK. The liquidity read today reproduces both
  deposit amounts at a pool tick of 73641 (0.0819946 WETH and 2,755 NOCK), so the
  liquidity is unchanged since the deposit; NOCK has since fallen against ETH, so the
  range holds more NOCK and less WETH.
- The position is 20.1% of the pool's in-range liquidity (`liquidity()` =
  287515221646699784807).

### Ownership: the NFT is staked, and how the wallet is proved to be the staker

- `ownerOf(2914490)` = the gauge `0x3bf70c88…60fd`, not the wallet.
  `positionManager.balanceOf(wallet)` = 0.
- `gauge.stakedContains(wallet, 2914490)` = **true**. `stakedLength(wallet)` (`0xae775c32`)
  = 1, `stakedByIndex(wallet, 0)` (`0x38463937`) = 2914490, `stakedValues(wallet)`
  (`0x4b937763`) = `[2914490]`.
- The same question about an account that did not stake it
  (`stakedContains(0x4857db57…, 2914490)`) and about another id
  (`stakedContains(wallet, 2914491)`) = false.
- `gauge.withdraw(2914490)` (`0x2e1a7d4d`) simulated **from the wallet** succeeds; simulated
  from another account it reverts with `NA`. So the per-depositor stake set is what gives
  the right to take the NFT back, and `stakedContains` is the read that proves it.

The adapter counts the position only when `ownerOf` is the wallet, or `ownerOf` is
`pool.gauge()` **and** `stakedContains(wallet, tokenId)` is true. Otherwise the cycle fails
and says who holds the NFT. A `gauge_address` in config is optional and must equal
`pool.gauge()`.

This gauge is not the older Slipstream `CLGauge`: `earned(address,uint256)`
(`0x3e491d47`), `rewards(uint256)` and `rewardToken()` revert on it. Its selectors, read
from the implementation's bytecode, include `earned(address,uint256[])` `0xd942be03`,
`claimEmissions(address,address,uint256[])` `0x09902477`, `deferredEmissions(address)`
`0xba135be9`, `depositBlock(uint256)` `0x32590654`, `pendingFees()` `0x224438d1`,
`collectFees()` `0xc8796572`, `depositFor(uint256,address)` `0x36efd16f` and
`withdrawFrom(uint256,address)` `0xb1cbf888` (names resolved through the public signature
database; only the ones called above were exercised).

### Fees: what a staked LP can and cannot claim

Measured, not assumed:

- **Swap fees on staked liquidity go to the gauge.** At this block `pool.stakedLiquidity()`
  (`0x3ab04b20`) equals `pool.liquidity()`: all in-range liquidity is staked. Across the
  swap in block 58060732 (tx `0xd42d28d1…518137`, 9765162112164125 wei WETH in),
  `pool.gaugeFees()` (`0x293833ba`) token0 rose from `0xdf2a2eb83ab2` to `0x135506233e2eb`:
  94722072487993 wei, which is exactly `ceil(amountIn × 9700 / 1e6)`. The whole fee went to
  the gauge's account. `gauge.pendingFees()` reports the same balance.
- **Nothing is collectable for the position.** `collect` for NFT #2914490 simulated from
  the wallet reverts (the gauge holds the NFT); simulated from the gauge, its owner, it
  returns (0, 0). `positions().tokensOwed0/1` are 0. Five other contract-held NFTs on the
  same position manager (#3116708, #2890904, #3089776, #3096968, #3110680; three of the
  holders are 45-byte proxies like this gauge, none was confirmed to be a gauge) also
  returned (0, 0) when `collect` was simulated from the holder.
- **The staker earns AERO instead.** `gauge.earned(wallet, [2914490])` returned
  `0xa08e3e2faa16f716` (11.569 at 18 decimals) at this block. When #91 shipped NockNock
  did not read this value; since #93 it does, every cycle (§9).

So a staked position reports no fee figure. Its fee fields say
"not LP-claimable (staked: swap fees on staked liquidity go to the gauge)" in
`positions[].fees_note`, the claim payload (`not_claimable`) and, followed by the
position's unclaimed AERO (§9), in `contract_fees[].amount`, the velocity row's `note`
and the MCP text. It is never shown as $0 earned, and it adds nothing to the claimable
total.

**An unstaked position** (the wallet holds the NFT) can collect. Verified on NFT #3117536
at **block 58063800** (`0x375fbb8`), then held by the EOA `0xe3c74593…a88a` (USDC / an
unregistered token, tick spacing 10, ticks -360..-270; burned a few hundred blocks later):
`positions()` showed tokensOwed 0/0, while `collect` simulated from the owner returned
(129192, 599) raw, and from another account reverted. The adapter therefore takes an
unstaked position's fees from the `collect` simulation, not from `tokensOwed`. One other
EOA-held NFT (#86038) reverted on the same simulation for a reason not investigated. The
simulation is a required read, like the Uniswap adapters' fee reads: if it reverts for a
tracked position the **cycle fails** with "collect() for NFT #… simulated from the tracked
wallet … reverted; its claimable fees cannot be read", and nothing is published with that
fee as 0. (The first version of the adapter carried such a position as "fees unavailable"
with zero amounts; the zeros reached the claim payload and the fee totals, so it was
removed. Whether to degrade instead is the operator's call; it would need a stated
"unread" marker on every total that includes fees.)

### Swap fee units

`pool.fee()` = 9700 and the measurement above gives fee = amountIn × 9700 / 1,000,000, so
the unit is millionths (the Uniswap v3 "pips"): 9700 is **0.97%**, reported as
`fee_tier_bps` 97. The fee is dynamic on these pools; the worker reads it every cycle.
`pool.unstakedFee()` (`0xb64cc67b`) = 50000; its meaning was not measured and it is not
used.

### What NockNock does with the position

- Its NOCK counts toward the NOCK total and the floor; its WETH toward the ETH pillar
  (never cash, never investable).
- It is a ranged position: in/out of range comes from the pool tick against its ticks
  (upper bound exclusive), and it appears in the tick bracket panel like the Uniswap ones.
- It is never offered an LP rebuild deposit: its WETH leg would have to come from gas
  (issue #94). No Aero CL position is sized for a rebuild, whatever its pair
  (`lp_rebuild.excluded[]` gives the reason): the app's deposit link starts a new position
  (see "Not verified"), a different NFT that NockNock would not track, so a suggestion to
  "add to NFT #N" would send capital out of the totals and the floor reading.

### Not verified

- **Adding liquidity to an existing CL NFT.** `https://app.aero.xyz/deposit/8453-0x957256c9…0874`
  does open the WETH/NOCK CL200 pool (observed 2026-11-16 in a browser with no wallet
  connected: "DEPOSIT WETH / NOCK 0.97% Base CL200", then "Select deposit type: Custom:
  set your own price range"). That is the start of a **new** deposit; nothing on the page
  refers to an existing NFT. Whether the app, or the contracts (`increaseLiquidity` on
  the position manager, `increaseStakedLiquidity` on the older gauges), can add to an
  NFT that is staked in this gauge was not tested. Until it is, no rebuild suggestion is
  made for an Aero CL position and no deposit link is shown for one.
- **AERO**: read since #93; the token, the gauge reads, the price source and what is
  still unverified about the claim path are in §9.
- **Other CL pools.** Only the WETH/NOCK CL200 pool and its gauge were exercised. The
  adapter reads the gauge from the pool, so another pool works only if its gauge answers
  `stakedContains(address,uint256)`; one that does not fails the cycle rather than being
  counted.
- **`unstakedFee()`** and how fees are split when only part of the in-range liquidity is
  staked.

---

## 9. AERO, the Fourth Pillar (v0.6.0, issue #93) — Verified Record

The worker reads the wallet's AERO, the AERO each staked Aero position has earned and
not claimed, and an AERO/USD price (`ingestion-worker/src/aero_rewards.rs`). Everything
below was read with `eth_call`, `eth_getLogs` and `eth_getTransactionReceipt` through
`https://base-rpc.publicnode.com` on 2026-11-16, pinned to **block 58077673**
(`0x37631e9`, 2026-11-16T05:11:50Z, hash `0xcaef3e28…aecd40`) unless another block is
named. Selectors and topics were computed from the signatures shown and each was checked
against the response it produces. The raw responses are the fixtures in
`ingestion-worker/src/tests_aero_rewards.rs`.

### Which token is "AERO": there are two

| | The token the Aero gauges pay | The older token |
|---|---|---|
| Address | `0x3f5bF3EdAc59d650D8fDf9657a71F2f904A3a4Ee` | `0x940181a94A35A4569E4529A3CDfB74e38FD98631` |
| `symbol()` `0x95d89b41` | "AERO" | "AERO" |
| `name()` `0x06fdde03` | "Aero" | "Aerodrome" |
| `decimals()` `0x313ce567` | 18 | 18 |
| `totalSupply()` `0x18160ddd` | 2,142,866,660 (raw `0x06ec89aa690b7aba6e31c247`) | 2,008,099,184 (raw `0x067d0f8ae7f0f34ed947e31e`) |
| `balanceOf(wallet)` `0x70a08231` | 0 | 0 |
| Paid by the two gauges below | **yes** (see "A claim pays what `earned` said") | no: no `Transfer` of it out of either gauge in blocks 57,000,000 to 58,077,673, and `balanceOf` of each gauge is 0 |

Issue #93 and the work order named `0x940181a9…8631`. That is **not** what the
operator's gauges pay, so NockNock's AERO is `0x3f5bF3Ed…a4Ee` (`AERO_BASE`). The older
token is not read, priced or ledgered: a wallet that held it would not see it in
NockNock. Whether the two are convertible was not investigated.

### The gauges and how each reports unclaimed AERO

| | Basic (volatile pool) gauge | CL gauge |
|---|---|---|
| Gauge | `0xc82782ee66d82c4d19db0429652091a085083526` | `0x3bf70c880adc8370917994ab0df68e763b7a60fd` |
| Its pool | `stakingToken()` `0x72f702f3` = `0x770fE8Fc…9e0A` (USDC/NOCK) | `pool.gauge()` returns it (§8; WETH/NOCK CL200) |
| Code | 45-byte EIP-1167 proxy to `0xbce3a68f9c22d18ddfff308eb77ce32c31853b03` | 45-byte proxy to `0x89e91a960154b1ed6c093039a8f0725ab67d14e0` |
| `voter()` `0x46c96aac` | `0x382409fad8503fbcdbb27a4211551fa03e884078` | the same |
| `gaugeFactory()` `0x0d52333c` | `0x9fbcf2df0dc553ce40ef63963bff465b76792b7e` | `0xf437f62053acb121665b8fdbfea8f214fb8e589d` |
| `rewardToken()` `0xf7c618c1` | **reverts** | **reverts** |
| The read NockNock uses | `earned(address)` `0x008cc262` | `earned(address,uint256[])` `0xd942be03` |
| For the wallet | `0x01adf89b6483d30d` = 0.121026111298851597 AERO | with `[2914490]`: `0xa573ba928fdaf5d2` = 11.922077777221318098 AERO |
| A zero answer | `earned(0x…dead)` (no stake) = 0 | `earned(wallet, [])` = 0 |
| What reverts | — | `earned(wallet, [2914491])` (an id the wallet has not staked): "execution reverted: NA". `earned(address,uint256)` `0x3e491d47`, the older Slipstream gauge's form: reverts |
| Staked | `balanceOf(wallet)` = 29684955447174 LP (the wallet's own `pool.balanceOf` is 0) | `stakedContains(wallet, 2914490)` = true |

Neither gauge names its reward token, so the token was established from what a claim
pays (next section). The CL calldata is the account word, the array offset `0x40`, the
length 1 and the token id:
`0xd942be03 ‖ wallet ‖ 0x40 ‖ 1 ‖ 0x2c78ba`.

The wallet's unclaimed AERO at this block was therefore **12.043104 AERO**, worth
**$5.03** at the price below. Its wallet AERO was 0: it has not claimed.

### A claim pays what `earned` said (two real claims by other accounts)

| | Basic gauge | CL gauge |
|---|---|---|
| Tx | `0x189f6ddfbb0e278ed37b2049d8eb5deeeb78f618a1b20c213c47f42511f9b8f8` | `0x7216bc7edd6d0a98bfbb57d6c8fb817eb328bf0f34556c75b1c5717472c79760` |
| Block | 57635701 (`0x36f7375`, 2026-11-15T04:38:35Z) | 57427872 |
| Claimer | `0x7c12cd5b…386d`, calling the gauge directly | `0xa8373266…dbdb` (a contract), NFT #2895206, in a larger transaction |
| `earned` one block before | `0x12df9e0749351725` | `0x1ae6d93dff2d0957` |
| AERO `Transfer` to the claimer | from the **zero address**, `0x12df9e0749351725` (1.359979366619354917) | from the zero address, `0x1ae6d93dff2d0957` (1.938475549926951255) |
| `earned` in the claim's block | 0 | reverts "NA" (the NFT was withdrawn in the same transaction) |
| Gauge event | `EmissionsClaimed(address,address,uint256)`, topic `0x41a8f9b7…7a968` | `EmissionsClaimed(address,uint256,address,uint256)`, topic `0x7b6bd720…27bb1` |

So in both gauge types `earned` is exactly what a claim then mints, and the AERO arrives
as a **mint**: an ERC-20 `Transfer` of `0x3f5bF3Ed…a4Ee` from `0x0000…0000` to the
recipient. The same receipts also show, and NockNock ignores:

- a token `0x6cdda0e730744dc6ff67654b35350adad05ecee7` (symbol "xAERO") minted to and
  burned from `0x8c37f30f…22c3` for the same amount;
- a further mint of AERO to `0x9beb95cc…1156`, 5% of the claim (`0x00f194b39075dac1`
  beside `0x12df9e07…1725`). It is in addition to the claimer's amount, not taken from it.

The claim functions, by selector lookup only and never called by NockNock, are
`claimEmissions(address,address)` `0x8ed86282` on the basic gauge and
`claimEmissions(address,address,uint256[])` `0x09902477` on the CL gauge. Simulated from
the wallet at the pinned block both return empty data without reverting.

### AERO/USD price source

AERO/USDC pools found through the factories at the pinned block (`getPool` on the Aero
CL factory `0x9cb22058…7a76` with `0x28af8d0b`, and on the Uniswap v3 factory
`0x33128a8f…fdfd` with `0x1698ee82`):

| Pool | `slot0` price (USDC per AERO) | `liquidity()` | USDC / AERO held |
|---|---:|---:|---|
| **Aero CL200 `0xd0acf293547457fb4fe9ee32ca93e89497626816`** | **0.417608** | 718149000491862264365 | 3,330,360 / 4,172,956 |
| Aero CL100 `0x422df9aa…a1cf` | 0.415519 | 16573887176669224717 | 94,336 / 981,725 |
| Uniswap v3 0.30% `0x229cfa9f…eb3d` | 0.416275 | 150128334157482 | 1.84 / 383.72 |
| Aero CL1 `0xb27460d0…f863` | 0.704442 | **0** | dust |
| Uniswap v3 0.01% `0xed66bcda…5473` | 0.639400 | **0** | dust |
| Uniswap v3 0.05% `0x32ed1579…6c9e` | 0.803276 | **0** | dust |
| Uniswap v3 1% `0xcc500ceb…7367` | 0.750975 | **0** | dust |

Four of the seven have no in-range liquidity and show a frozen price 53% to 92% above the
market. This is the stale-pool case the NOCK and ETH price guards exist for, so the same
guard applies: a pool whose `liquidity()` is 0 gives no price.

- **Default `aero_price_pool`**: the Aero CL200 pool `0xd0acf293…6816`, the deepest.
  `token0()` = AERO `0x3f5bf3ed…a4ee`, `token1()` = USDC, `tickSpacing()` = 200, `nft()` =
  the position manager of §8, `factory()` = `0x9cb22058…7a76`, `fee()` = 10500 pips.
  `slot0()` returns six words (a Slipstream pool); word 0 is sqrtPriceX96
  `0xad784fbb267f75a7626`, tick -285057. Price = (sqrtPriceX96 / 2^96)² × 10^(18−6) =
  **$0.417608**.
- **Cross-check at the same block**: the Aero CL200 AERO/WETH pool `0xc85d5cb7…c9b3`
  (sqrtPriceX96 `0x03e0fd7ad1a3064d1658cef5`, 0.000229578 WETH per AERO) times ETH/USD
  from `eth_price_pool` ($1,811.33) gives $0.415842, within 0.5% of the pool above.
- **At a past block**: the same pool at block 57635701 read sqrtPriceX96
  `0xb1d397eb6115a61877a`, $0.438847. That is the read the harvest ledger makes for a
  claim in that block, and it needs an archive node (Alchemy serves one).

Every cycle the worker reads the pool's `token0()`, `token1()`, `slot0()` and
`liquidity()`. The tokens must be AERO and USDC, in either order; otherwise it is
"Configuration error: aero_price_pool … is not an AERO/USDC pool" and no price is used.
Zero liquidity is "Stale AERO/USD price: price pool … has zero in-range liquidity".

### What NockNock does with it

- **Wallet AERO** is a required read (`AERO.balanceOf(wallet)`), like NOCK, USDC and gas:
  a failed read fails the cycle. It is counted in `total_portfolio_usd` at the price
  above, appears as the last `wallet_balances` row and as the "AERO (wallet)" row of the
  deviation matrix (status `no_target`), and is never cash, never dry powder and never
  the source of a suggested deposit.
- **Unclaimed AERO** is read for every tracked Aero position that has a gauge: an
  `aerodrome` position with `gauge_address`, and an `aerodrome-cl` position the cycle
  proved staked. It is **not** in the total, exactly as uncollected LP fees are not; it is
  stated beside them (`positions[].unclaimed_aero`, `global_status.unclaimed_aero_amount`)
  and counts once claimed. The read is optional: a revert or a failed read is stated as
  "unclaimed AERO not read (…)" on that position and makes the total null, never 0 and
  never a partial sum. An `aerodrome` position with no `gauge_address` says so.
- **The price** is required only while the wallet holds AERO (its value is in the total,
  so a failed read fails the cycle). While the wallet holds none, a failed read leaves
  `aero_price_usd` null with the reason in `aero_note`, the amounts stay, and the cycle
  goes on.
- **A claim** recorded with `POST /v1/execute/record-harvest` books the AERO `Transfer`
  to the wallet, valued from the price pool at the claim's block, or with no USD value
  when that read fails. The Koinly export describes the row as an AERO reward claim.
- **No advice.** NockNock builds no claim transaction for AERO, sets no target, and no
  directive reads an AERO figure.

### Not verified

- **Penalties and deferral.** The basic gauge has `isPenaltyActive(address)` `0x9af3a17c`
  (false for the wallet), `depositBlock(address)` `0x202019a6` (57425187, the deposit) and
  both gauges have `deferredEmissions(address)` `0xba135be9` and
  `deferredReferralEmissions(address)` `0xa9d4ee99` (0 for the wallet); the CL gauge has
  `forfeitRollover()`. What these do to a claim was not investigated. In the two claims
  observed, `earned` was paid in full. If a penalty can apply, `earned` would overstate
  what a claim pays; NockNock labels the figure "unclaimed AERO" and does not promise it.
- **How the Aero app claims.** Both observed claims paid the claimer directly. If the app
  routes a claim through a contract that receives the AERO and forwards it, the wallet's
  `Transfer` would come from that contract rather than the zero address. The ledger books
  any AERO `Transfer` to the wallet in a recorded transaction, whatever its sender, so it
  would still be booked; a recorded transaction that merely buys AERO would be booked too.
- **Other gauges.** Only these two gauges were read. Another basic gauge must answer
  `earned(address)` and another CL gauge `earned(address,uint256[])`; one that does not is
  reported as "not read", never as 0.
- **AERO inside a position.** AERO is not in the position token registry, so a pool with
  an AERO leg cannot be tracked (the cycle fails and names the token).
- **The names** `EmissionsClaimed`, `claimEmissions` and the others above come from a
  public signature database; the topic hashes were recomputed from those signatures and
  match the logs. No verified source for these contracts was read.

