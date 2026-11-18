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

> **Confirmed in v0.7.0 (noted 2026-11-16):** Aero now publishes a contract list
> (`app.aero.xyz/docs/security/`) that names `0x0fef9e88…` "Metarouter" and the factory
> below "CLFactory", and the `fee()` unit of these pools was measured: millionths, so
> the three percentages at the end of this section are right. See §9.

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
total. Since #142 what it earns instead, the gauge reward, is reported as its collectible
earnings beside that reason (§9, "The claim call, proven").

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

### Legacy Aerodrome gauges answer the same call in the other token

Read at **block 58093329** (`0x3766f11`, 2026-11-16T06:04:01Z, hash `0x0e07395b…2ebdf4`).

| | Legacy basic gauge | Legacy CL gauge |
|---|---|---|
| Gauge | `0x4cbb6188c6101cacd4eb1126cc14882bd93dc3db` | `0xf33a96b5932d9e9b9a0eda447abd8c9d48d2e0c8` |
| How it was found | Voter `0x16613524e02ad97eDfeF371bC883F2F5d6C480A5` `gauges(address)` `0xb9a09fd5` for the legacy vAMM USDC/NOCK pool `0x85f1aa3a…d626` | `gauge()` `0xa6f19c84` on the legacy Slipstream CL100 WETH/USDC pool `0xb2cc224c…dc59` (§8) |
| `stakingToken()` `0x72f702f3` | `0x85f1aa3a70fedd1c52705c15baed143e675cd626` (that pool) | not read |
| `rewardToken()` `0xf7c618c1` | `0x000…940181a94a35a4569e4529a3cdfb74e38fd98631`: the **older** token | the same older token |
| `earned(address)` `0x008cc262` for the wallet | **answers**: one word, 0 | — |
| `earned(address,uint256[])` `0xd942be03` for the wallet with `[]` | — | reverts |
| `rewardRate()` `0x7b0a47ee` | `0x13b343813f9f3e` (still emitting) | not read |

At the same block `rewardToken()` on the two Aero-app gauges above still reverts
(`{"code":3,"message":"execution reverted"}`).

A legacy basic gauge therefore answers the read NockNock uses, in a token that is not
NockNock's AERO. An `aerodrome` position on the legacy pool with that gauge as its
`gauge_address` would otherwise have its figure published as "unclaimed AERO" and priced
from the AERO pool. So each cycle calls `rewardToken()` on the gauge before `earned`
(`gauge_pays_aero`):

- it reverts: the Aero app's gauge, shown below to pay `0x3f5bF3Ed…a4Ee`. `earned` is read;
- it returns `0x3f5bF3Ed…a4Ee`: `earned` is read (no such gauge was found; allowed for);
- it returns any other token: nothing is read as AERO. The position says "unclaimed AERO
  not read (gauge … pays 0x9401…8631 (its rewardToken()), not AERO 0x3f5b…a4ee)", with
  no amount and no USD value, and the unclaimed total is null;
- the read fails or is not an address: not read, with that reason.

The legacy CL gauge could not be misread in any case, since the array form of `earned`
reverts there; the same check runs for it.

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
the wallet at the pinned block both return empty data without reverting. On 2026-11-17
the tracked wallet itself claimed from the CL gauge with the second of them; what each
call pays is proven in "The claim call, proven" below.

### The claim call, proven (issue #142, read 2026-11-17)

Issue #142 reports a staked position's gauge reward as its collectible earnings and may
state what collecting it costs in gas, **only** for a claim call proven to pay what
`earned` reports. Everything below was read with `eth_call`, `eth_getLogs`,
`eth_getTransactionByHash`, `eth_getTransactionReceipt`, `eth_estimateGas` and
`eth_simulateV1` through `https://base-rpc.publicnode.com`, and read again through
`https://mainnet.base.org` with the same answers. Nothing was sent. (Base produced a
block about every 0.2 s on this day: 542,070 blocks between 2026-11-16T05:11:50Z and
2026-11-17T11:18:44Z.) The raw values are the fixtures in
`ingestion-worker/src/tests_rewards.rs`.

**CL gauge `0x3bf70c88…60fd`: proven by the tracked wallet's own claim.**

| | |
|---|---|
| Tx | `0x2d2b609b74142a5e56951850c646963d3553adac0c5280b70847f0bf0ed7a16e` |
| Block | 58618046 (`0x37e70be`, 2026-11-17T11:13:04Z, hash `0x453956f0…1c38`), status success |
| From → to | the wallet `0xeE114cC9…7a45` → the gauge, directly |
| Input | `0x09902477 ‖ wallet ‖ wallet ‖ 0x60 ‖ 1 ‖ 0x2c78ba`: two address words, then a one-element array holding token id 2914490 |
| `earned(wallet, [2914490])` at block 58618045 | `0xf4ffa950a248bebf` = 17.654015228100787903 AERO |
| AERO `Transfer` to the wallet (log `0x13`) | from the zero address, `0xf4ffa950a248bebf`: exactly that amount |
| Gauge event (log `0x10`) | `EmissionsClaimed(address,uint256,address,uint256)`, topic `0x7b6bd720815adbeaaaed4fc759e1a535705901f33709fbb33e62ee9a89427bb1`, with the wallet, token id `0x2c78ba`, the wallet, and the same amount |
| `earned(wallet, [2914490])` at block 58618046 | 0 |
| `AERO.balanceOf(wallet)` | 0 at block 58618045, `0xf4ffa950a248bebf` at block 58618046 |
| Further mint (log `0x14`) | `0x0c3ffbaa6e83a323` (0.882700761405039395 AERO, 5.0000% of the claim) to `0x9beb95cc…1156`, on top of the wallet's amount |
| Gas | `gasUsed` 418,528 (`0x662e0`) at `effectiveGasPrice` 9,000,000 wei, `l1Fee` 1,258,065,811 wei: 3,768,010,065,811 wei in all, about $0.0067 at ETH $1,790.24. The L1 data fee is 0.03% of it |

`eth_simulateV1` of the same call from the wallet on the state of block 58618045 returns
status 1, `gasUsed` 418,528 and the same thirteen logs: the simulation reproduces the
real transaction, on both nodes. That is what makes the simulation below evidence.

On the same state, the two-address form `0x8ed86282 ‖ wallet ‖ wallet` sent to the **CL**
gauge also succeeds (`gasUsed` 419,916) and emits the same logs, including
`EmissionsClaimed` for token id 2914490 and the same AERO mint: with one NFT staked, it
pays the same. NockNock estimates the form the real transaction used.

**Basic gauge `0xc82782ee…3526`: proven by a real claim of another account, and for the
tracked wallet by simulation.**

- The real claim recorded above (tx `0x189f6ddf…b8f8`, block 57635701), re-read on
  2026-11-17: from `0x7c12cd5b…386d` to the gauge, input
  `0x8ed86282 ‖ claimer ‖ claimer`, `gasUsed` 349,146, AERO minted to the claimer
  `0x12df9e0749351725`, which is `earned(claimer)` one block earlier.
- For the tracked wallet, at block 58619904 (`0x37e7800`, 2026-11-17T11:19:16Z, hash
  `0x5f3cff08…d457`): `earned(wallet)` = `0x02ecc3bc217fa18b` (0.210758495362851211
  AERO). `eth_simulateV1` of `0x8ed86282 ‖ wallet ‖ wallet` from the wallet to the gauge
  on that state: status 1, `gasUsed` 384,013, an AERO `Transfer` from the zero address to
  the wallet of `0x02ecc3bc217fa18b` (exactly `earned`), the gauge's
  `EmissionsClaimed(address,address,uint256)` (topic `0x41a8f9b7…7a968`) with the wallet
  twice and the same amount, and the further 5% (`0x25702fce7994e0`) to
  `0x9beb95cc…1156`. The same on both nodes. The wallet has not sent this transaction.

**`eth_estimateGas` from the wallet**, 2026-11-17 near block 58619693:
`0x8ed86282(wallet, wallet)` on the basic gauge 390,860 (`0x5f6cc`);
`0x09902477(wallet, wallet, [2914490])` on the CL gauge 363,527 (`0x58c07`), with nothing
left to collect there (it was 367,022 for `0x8ed86282` on the CL gauge).

**What NockNock does with this** (`ingestion-worker/src/rewards.rs`):

- It reports the reward as collectible earnings on every surface: the token, the amount,
  its USD value at the cycle's AERO price, the gauge it is collected from and how
  ("the Aero app"). `positions[].collectible_reward`, the `reward` of the fee and
  velocity rows, `global_status.collectible_rewards_usd` and `total_collectible_usd`.
- It still builds **no transaction** for a reward. The claim payload lists rewards in a
  `rewards` array whose rows have no `to` and no `data`; no action claims AERO.
- When the claim simulation is asked for, the worker runs a read-only `eth_estimateGas`
  from the wallet for the call above and prints its units and USD cost (units × the live
  gas price × the cycle's ETH/USD). It does so only for these two gauge addresses with
  their own call. For any other gauge the row reads "gas not estimated (claim call not
  verified for this gauge)". A zero reward reads "0.000000 AERO ($0.00), nothing to
  collect in gauge 0x…" with no gas statement (its claim payload row says "nothing to
  collect, gas not estimated"). The claim payload uses the `eth_gasPrice` it already
  read for its LP actions; the HTTP MCP reads it once. The calldata is used for that
  estimate and nowhere else.
- Unclaimed AERO stays out of `total_portfolio_usd`. `total_fees_claimable_usd`, BATCH
  HARVEST, the notifier and every threshold keep reading LP fees only.

**Still not verified:**

- **The names.** `claimEmissions` and `EmissionsClaimed` come from a public signature
  database. The selectors and topics are what the transactions and logs carry.
- **The basic gauge for the tracked wallet in a real transaction.** The proof is another
  account's real claim plus a simulation for the wallet, not a transaction of the wallet.
- **Any other gauge.** Only these two addresses were exercised. Another Aero gauge may
  be the same code behind the same proxy; that was not read, so no gas figure is shown
  for it.
- **What an estimate promises.** `eth_estimateGas` at request time says the call does not
  revert from the wallet and what it would cost then. It is not a second proof of what
  the call pays, and gas at the moment of a real claim can differ. The USD figure is L2
  execution gas only (the L1 data fee was 0.03% of the operator's claim).
- **Penalties and deferral** (see "Not verified" below). In the operator's claim `earned`
  was paid in full.
- **The further 5% mint** to `0x9beb95cc…1156`: seen in every claim examined, in addition
  to the claimer's amount. What it is was not investigated.
- **Which interface sent the operator's claim.** The transaction is a direct call from
  the wallet to the gauge; the chain does not record the page it was signed from.

### A claim from several gauges in one transaction

Tx `0xe32b6e476ceff9040c1e22b5a23e9ba2a8c4e5cff8221939e871e9ec84bb3a67`, block 58077008
(`0x3762f50`), status success, sent by `0x0cf406ec…382b` to
`0x091607c3355f5875400851efc6347a839d3b25f6` (a contract: `eth_getCode` returns code). Its
receipt has 256 logs, among them
fourteen AERO `Transfer`s from the zero address: seven mints to `0x091607c3…25f6`, one per
CL gauge claimed, each followed by the further 5% to `0x9beb95cc…1156`.

| Log index | AERO minted to `0x091607c3…25f6` (raw) |
|---|---:|
| `0x1d` | 472646160276911224 |
| `0x41` | 1605459784569903980 |
| `0x65` | 1506849352525191393 |
| `0x8a` | 523941515033061636 |
| `0xaf` | 242379159878096194 |
| `0xd4` | 1270601896224565304 |
| `0xf9` | 443269540123302401 |
| **Sum** | **6065147408631032132** (6.065147408631032132 AERO) |

The same receipt pays that account USDC seven times from seven different pools (for
example `0x0d8bfc3d` from `0xf668e219…c8ec` and `0x1c459d9a` from `0x2836e062…4e75`).

The harvest ledger's unique key is `(tx_hash, token_address)`, so such a receipt cannot be
stored as one row per `Transfer`: the second insert would conflict with the first and be
dropped. `merge_harvest_transfers` sums every `Transfer` of one token to the wallet into
one row before the insert (amounts added as integers; the first sender is kept). The
operator's two gauges claimed in one transaction would be two AERO mints and one ledger
row of their sum. The recipient here is a contract; whether the Aero app batches a plain
wallet's claims into one transaction was not established.

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
  never a partial sum. An `aerodrome` position with no `gauge_address` says so and makes
  the total null as well. A gauge whose `rewardToken()` names another token is not read
  as AERO (see "Legacy Aerodrome gauges" above).
- **The price** is required only while the wallet holds AERO (its value is in the total,
  so a failed read fails the cycle). While the wallet holds none, a failed read leaves
  `aero_price_usd` null with the reason in `aero_note`, the amounts stay, and the cycle
  goes on.
- **A claim** recorded with `POST /v1/execute/record-harvest` books the AERO `Transfer`
  to the wallet, valued from the price pool at the claim's block, or with no USD value
  when that read fails. The Koinly export describes the row as an AERO reward claim.
  Several AERO `Transfer`s to the wallet in one transaction are summed into one ledger
  row (see "A claim from several gauges in one transaction" below). A row recorded with
  no USD value is filled from the same block's price when the transaction is recorded
  again; a valued row is never changed.
- **No advice.** NockNock builds no claim transaction for AERO, sets no target, and no
  directive reads an AERO figure.
- **Collectible earnings (#142).** The unclaimed AERO of a staked position is reported as
  that position's collectible earnings, with the gauge it is collected from, and the
  totals state LP fees, AERO rewards and their sum as three labelled figures. This is
  reporting only: see "The claim call, proven" above for what is and is not estimated.

### Not verified

- **Penalties and deferral.** The basic gauge has `isPenaltyActive(address)` `0x9af3a17c`
  (false for the wallet), `depositBlock(address)` `0x202019a6` (57425187, the deposit) and
  both gauges have `deferredEmissions(address)` `0xba135be9` and
  `deferredReferralEmissions(address)` `0xa9d4ee99` (0 for the wallet); the CL gauge has
  `forfeitRollover()`. What these do to a claim was not investigated. In the three claims
  observed (the third is the tracked wallet's own, 2026-11-17), `earned` was paid in full. If a penalty can apply, `earned` would overstate
  what a claim pays; NockNock labels the figure "unclaimed AERO" and does not promise it.
- **How the Aero app claims.** The three claims examined that were sent to a gauge
  directly (two by other accounts, and the tracked wallet's own on 2026-11-17) paid the
  claimer directly. If the app
  routes a claim through a contract that receives the AERO and forwards it, the wallet's
  `Transfer` would come from that contract rather than the zero address. The ledger books
  any AERO `Transfer` to the wallet in a recorded transaction, whatever its sender, so it
  would still be booked; a recorded transaction that merely buys AERO would be booked too.
- **Other gauges.** Of the Aero app's gauges only these two were read. Another basic
  gauge must answer `earned(address)` and another CL gauge `earned(address,uint256[])`;
  one that does not is reported as "not read", never as 0. A gauge that names a reward
  token other than AERO is not read either (the two legacy gauges above). A gauge with no
  `rewardToken()` is taken to be an Aero-app gauge and to pay AERO, which was shown for
  these two only: a contract of another kind that reverts on `rewardToken()` and answers
  `earned(address)` with a word would be read as AERO. `gauge_address` is the operator's
  to set.
- **AERO inside a position.** AERO is not in the position token registry, so a pool with
  an AERO leg cannot be tracked (the cycle fails and names the token).
- **The names** `EmissionsClaimed`, `claimEmissions` and the others above come from a
  public signature database; the topic hashes were recomputed from those signatures and
  match the logs. No verified source for these contracts was read.


---

## 10. Fill Detection for the Accumulation Plan (v0.6.0, issue #118) — Verified Record

The worker looks for the plan's buys itself: each cycle it lists the NOCK transfers into
the tracked wallet back to the plan anchor, fetches the receipt of every transaction it
has not seen before, and applies the checks of `record-accumulation` (one shared function,
`fill_verify.rs`). What passes is offered for confirmation; nothing is recorded until the
operator confirms, and confirming verifies the receipt again.

Detection adds **no contract address, no function selector and no event topic**. It
reuses the NOCK and USDC token addresses (§1) and the ERC-20 `Transfer` topic already in
`ledger.rs`. It adds one RPC method, `alchemy_getAssetTransfers`, and calls
`eth_getTransactionReceipt` and `eth_getBlockByNumber`, which the worker already used.

### What was read, and by whom

Read on 2026-11-15/16 (the evening of 2026-11-15 in the operator's timezone), Base mainnet.

| What | Call | Block | Raw result |
|---|---|---|---|
| `Transfer` topic | `keccak256("Transfer(address,address,uint256)")`, recomputed | n/a | `0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef`; it is `topics[0]` of every log below |
| Transfer history (Alchemy, the operator's plan, read-only) | `alchemy_getAssetTransfers`: ERC-20, `toAddress` = wallet, `contractAddresses` = [NOCK], `withMetadata`, `order: desc`, `maxCount`, `pageKey` paging | newest 58043697, oldest 53006877 | 20 transfers and a `pageKey` (`e73927e8-c0bf-4c78-a0eb-5279ce640947`); saved verbatim as `ingestion-worker/testdata/fills/alchemy_getAssetTransfers_nock_in.json` |
| The same transfers from logs (public RPC `base-rpc.publicnode.com`) | `eth_getLogs` address = NOCK, topics `[Transfer, null, 0x…ee114cc9…7a45]`, blocks `0x374eef8`..`0x375b248` (57,995,000..58,045,000) | head was 58062703 (`0x375f76f`) | 3 logs: tx `0x78376f3c…` block 58040708 logIndex 10 `data` `0xf713117c2d57cfbb` (1780.359297696005 NOCK); tx `0x929cb3e5…` block 58043556 logIndex 11 `data` `0x59970c4b41c2685` (40.34776317640147 NOCK); tx `0x3127a1f4…` block 58043697 logIndex 13 `data` `0xfd114621e11ee71` (113.97145923452884 NOCK). Same hashes, blocks, log indexes and amounts as the first three Alchemy entries. Saved as `eth_getLogs_nock_in_57995000_58045000.json` |
| `eth_getLogs` range limits | same call over 200,000 blocks | n/a | publicnode: error `-32701 exceed maximum block range: 50000`. On the operator's Alchemy plan the 200,000-block span succeeded (read by the orchestrating session; not repeated here, no key) |
| Receipts | `eth_getTransactionReceipt` for all 20 hashes (public RPC) | see table | For the six receipts also read through Alchemy, `status`, `blockNumber` and every log (`address`, `topics`, `data`, `logIndex`) are identical between the two providers |
| Block times | `eth_getBlockByNumber(n, false)` (public RPC) | 58040708, 57605753, 57228848, 53725121, 57425187 | `timestamp` `0x6afa73b5`, `0x6af91fe6`, `0x6af7f971`, `0x6aed4827`, `0x6af892d5` = 2026-11-16T03:08:37Z, 2026-11-15T02:58:46Z, 2026-11-14T06:02:25Z, 2026-11-06T03:23:19Z, 2026-11-14T16:56:53Z: equal to Alchemy's `metadata.blockTimestamp` for the same transfers |

### The 20 most recent NOCK transfers into the wallet, classified

Net flows are the sum of the USDC and NOCK `Transfer` logs touching the wallet in each
receipt (what `parse_accumulation_swap` computes). A purchase is net USDC out and net NOCK in.

| Transaction | Block | Block time | Receipt `to` | Net USDC | Net NOCK | Verdict |
|---|---:|---|---|---:|---:|---|
| `0x3127a1f4…e49c9e` | 58043697 | 2026-11-16T03:18:35Z | `0x03a520b3…` (v3 NPM) | +0.000000 | +113.9715 | not a purchase (v3 fee claim; also +0.000678 WETH) |
| `0x929cb3e5…cc4fd0` | 58043556 | 2026-11-16T03:18:06Z | `0x7c5f5a4b…` (v4 POSM) | +0.573656 | +40.3478 | not a purchase (v4 fee claim) |
| `0x78376f3c…6e31b9` | 58040708 | 2026-11-16T03:08:37Z | `0xcaf22ce3…7c67` | −20.000000 | +1780.3593 | **purchase** (plan buy 3, $0.01123369) |
| `0x4644edd9…4c61e8` | 57605753 | 2026-11-15T02:58:46Z | `0x0fef9e88…` (Aero router) | −20.000000 | +1720.2181 | **purchase** (plan buy 2, $0.01162643) |
| `0x1d566001…a374b1` | 57504113 | 2026-11-14T21:19:58Z | `0x03a520b3…` | +0.000000 | +33.7768 | not a purchase (v3 fee claim) |
| `0x8f0b3cbb…c78228` | 57503975 | 2026-11-14T21:19:30Z | `0x7c5f5a4b…` | +0.025169 | +18.8069 | not a purchase (v4 fee claim) |
| `0x80277e2b…c93ddb` | 57425824 | 2026-11-14T16:59:00Z | `0x0fef9e88…` | +0.000000 | −2755.0000 | not a purchase (CL200 deposit, §7) |
| `0x127c4f4f…f185f0` | 57425187 | 2026-11-14T16:56:53Z | `0x0fef9e88…` | −31.986644 | −2755.0000 | not a purchase (volatile-pool deposit, §7) |
| `0x5d0c02d9…2a3f0e` | 57228848 | 2026-11-14T06:02:25Z | `0x0fef9e88…` | −20.000000 | +1597.9340 | **purchase** (plan buy 1, $0.01251616, §6) |
| `0xa8cd4501…9ce4dc` | 53725121 | 2026-11-06T03:23:19Z | `0x0fef9e88…` | −105.471820 | +7370.1998 | **purchase**, before the plan ($0.01431058) |
| `0x56efde30…4f893b` | 53223698 | 2026-10-29T01:19:03Z | `0x7c5f5a4b…` | +0.000000 | +33415.7370 | not a purchase |
| `0x7f0daf07…c32124` | 53223681 | 2026-10-29T01:18:29Z | `0x7c5f5a4b…` | +0.328365 | +7693.1080 | not a purchase |
| `0x5673582f…7b4cac` | 53223656 | 2026-10-29T01:17:39Z | `0x7c5f5a4b…` | +103.253128 | +5257.3122 | not a purchase |
| `0xea9680ac…ea57ed` | 53223635 | 2026-10-29T01:16:57Z | `0x03a520b3…` | +0.000000 | +1444.3263 | not a purchase |
| `0x889c865f…52191c` | 53008782 | 2026-10-24T01:55:11Z | `0x7c5f5a4b…` | +0.000000 | +191.1711 | not a purchase |
| `0xfdbcebc2…d55c5b` | 53008778 | 2026-10-24T01:55:03Z | `0x7c5f5a4b…` | +5.376593 | +156.1783 | not a purchase |
| `0x6fc8f541…0c91da` | 53006987 | 2026-10-24T00:55:21Z | `0xcf77a3ba…` | +78.788719 | +3822.4952 | not a purchase |
| `0x8132c691…97f44d` | 53006951 | 2026-10-24T00:54:09Z | `0x7c5f5a4b…` | +60.372284 | +11600.9806 | not a purchase |
| `0x2a7b564a…02eae1` | 53006900 | 2026-10-24T00:52:27Z | `0x7c5f5a4b…` | +0.013038 | +5.4668 | not a purchase |
| `0xb27b6f44…5de3f9` | 53006877 | 2026-10-24T00:51:41Z | `0x85f1aa3a…` | +0.024665 | +1.4752 | not a purchase |

With the plan anchored on 2026-11-14 the worker examines the first nine rows and offers
exactly the three plan buys. Ten of these receipts are checked in under
`ingestion-worker/testdata/fills/` and drive `src/tests_fills.rs`: the three plan buys, the
four fee claims after the anchor, the two deposits and the pre-plan purchase.

### What the real data showed

- **The two Aero deposits appear in the transfer list.** The router `0x0fef9e88…` sent
  the wallet a NOCK refund in each (0.0000459601 NOCK and 0.0000000000000152 NOCK). #118
  named this as the possible false positive: a deposit that pays USDC and receives some
  NOCK. Neither qualifies, because the wallet's net NOCK flow is outward (−2,755). A
  deposit that paid USDC and returned more NOCK than it took would qualify; none has been
  observed. This is the reason a detected swap is offered and not recorded.
- **A purchase from before the plan exists** (`0xa8cd4501…`, 2026-11-06, $105.47182 for
  7,370.2 NOCK). The scan stops at the plan anchor, so it is never fetched or offered. Set
  `accumulation_plan.starts_at` earlier to reach it.
- **Plan buy 2 came 20 h 56 min after buy 1.** Against a 24-hour interval that is before
  its tranche was due, so the offer is labelled `early` (judged against the buy before it,
  whether that buy is recorded or still on offer). Buy 3 came 24 h 10 min after buy 2.
- **Plan buy 3 did not go through the Aero router of §6.** Its receipt's `to` is
  `0xcaf22ce31298cf2bf1d152862f80216478ad7c67`, and both the USDC and the NOCK moved
  directly between the wallet and the legacy vAMM `0x85f1aa3a…d626`. Which app sent it was
  not established. Verification is contract-agnostic, so it qualifies like the others, and
  NockNock stores nothing about the router.

### Cost over the reference spot

Each recorded fill and each offer carries `execution_cost_bps` =
(effective price / reference spot − 1) × 10,000. The reference is the stored NOCK/USD price
nearest the fill's block time, within 60 minutes (`nock_price_near`, the v0.5.3 lookup the
harvest ledger uses), kept with its source and its own timestamp. Inside the window the
worker's own read nearest the fill is used; a backfilled candle is used only when there is
none, and it counts as its **open** (the table stores a candle at its open time with its
close in `price_usd`, which is the price one candle period later; issue #129). While the worker is
running that is a price-source pool read at most a few minutes from the fill; after
downtime it may be a backfilled hourly candle, and the source says so. No stored price in
the window means no reference and no figure. The reference is not the price in the fill's
own block, and it is not a quote: cost against a quote belongs to #115.

For scale, plan buy 1 at $0.01251616 against the engine spot noted at the time ($0.01229,
§6) is +184 bps.

### Not verified

- **The worker's own `alchemy_getAssetTransfers` request.** This branch had no Alchemy
  key. The method and its parameters were exercised on the operator's plan by the
  orchestrating session and the response saved, but the request that produced it was not
  saved. The worker sends `fromBlock: "0x0"`, `toBlock: "latest"`, `excludeZeroValue: true`
  and `maxCount: "0x64"` with the parameters in the table above; the first deployed cycle
  is the first time that exact request runs. If Alchemy rejects it, the plan status reads
  `unavailable` with the reason and recording by hash is unaffected.
- **Following a `pageKey`.** The saved response has one; the second page was not fetched.
  The paging test splits the real response in two.
- **Compute-unit cost** of one transfer-history call per cycle on the operator's plan.


---

## 11. Route Quotes for a Due Tranche (v0.7.0, issue #115) — Verified Record

When the accumulation plan says a tranche is due, the worker asks each venue's own
quoter contract what that exact USDC amount returns in NOCK, along a fixed table of
routes, all at one block (`route_quotes.rs`). Every call is an `eth_call`; nothing is
signed, built or sent. This section records what was read to choose the quoters, pools
and selectors. Each address below answered a live call with a sane result before the
code relied on it; none is taken from memory or from a web page alone.

Read on 2026-11-16 between 05:17 and 05:41 UTC, Base mainnet, through the public RPC
`https://base-rpc.publicnode.com` (no key). Selectors were computed with
`python3 -I /tmp/nn-97/keccak.py '<signature>'`. The requests and their verbatim answers
are checked in under `ingestion-worker/testdata/quotes/` and drive `src/tests_quotes.rs`.

Base now produces a block every 200 ms (811,860 blocks between plan buys 1 and 3,
162,372 seconds apart), so "the quote block" is a much finer instant than the 2-second
blocks earlier sections assumed.

### Quoters

| Venue | Contract | Address | Identity check (at block 58081910) | Quote call |
|---|---|---|---|---|
| Aero | Metaquoter | `0x1dDfeC5058180C207F55BB77D95AcDd0d8227868` | `factoryRegistry()` `0x3bf0c9fb` → `0x7b2a2c12dc5ea2a31455952a05031c3e4bb28f3e` | `quoteExactInput(address[],address,uint256)` `0x76278b93` |
| Uniswap v3 | QuoterV2 | `0x3d4e44Eb1374240CE5F1B871ab261CD16335B76a` | `factory()` `0xc45a0155` → `0x33128a8f…FDfD` (the v3 factory of §1); `WETH9()` `0x4aa4a4fc` → WETH | `quoteExactInput(bytes,uint256)` `0xcdca1753` |
| Uniswap v4 | V4Quoter | `0x0d5e0f971ed27fbff6c2837bf31316121532048d` | `poolManager()` `0xdc4c90d3` → `0x498581ff…2b2b` (the PoolManager of §1) | `quoteExactInputSingle(((address,address,uint24,int24,address),bool,uint128,bytes))` `0xaa9d21cb`; `quoteExactInput((address,(address,uint24,int24,address,bytes)[],uint128))` `0xca253dc9` |

Where the Aero address came from: Aero's own contract list
(`https://app.aero.xyz/docs/security/`, "Metaquoter (All chains)") and the file
`deployment-addresses/cl-base.json` of `Dromos-Labs/metadex-slipstream-public`
(`metaquoter`), whose `IMetaquoter.sol` gives the signature. The same list names
`0x0fEf9e88A7b76BCDBF451780C6774FE32965Ee98` "Metarouter", `0x9CB220581b91b0C83223012FE69688b92aD07a76`
"CLFactory", `0x8E6a9FCd1d638CC760Ffba397605Fe3355dE76D2` "VolatilePoolFactory" and
`0x7949DE8d67E4F6b8509Bdbb43FF0503aF1247a7A` "CustomFeeModule (volatile pools)". That
settles what §6 could only infer: the router of the plan's buys is Aero's. The
legacy Slipstream quoter `0x254cF9E1…15b0` answers `factory()` with `0x5e7bb104…809a`,
a different factory, so it cannot quote these pools and is not used.

The Metaquoter takes the pools of a route in order and the input token; it handles
concentrated and basic pools in one call. All three quoters return the amount out as
their first word.

**Replay check (the proof that the Aero quoter matches execution).** The Metaquoter was
asked for 20 USDC along the route of plan buy 1 (§6: `0xd448…aaf7`, `0x2dbe…cf7b`,
`0x9572…0874`) at block **57228847**, the block before that buy (57228848). It returned
`0x…ddc205801b5ceaca` = 15,979,340,475,636,968,138 raw = 1,597.9340475636968138 NOCK.
The buy's receipt shows the wallet received 15,979,340,475,636,968,138 raw NOCK: the
same number, to the wei. The public RPC served that block (about two days old), so no
approximation was needed. Fixture: `replay_fill1_block_57228847.json`.

### Pools on the routes

| Pool | Address or pool id | Resolved by | Read (blocks 58079466 to 58081910) |
|---|---|---|---|
| Aero basic volatile USDC/NOCK | `0x770fE8Fc9248c6382950A86E39a9391B41D59e0A` | VolatilePoolFactory `getPool(address,address)` `0x531aa03e` (this factory has no stable flag); `isPool` `0x5b16ebb7` → true | `token0` USDC, `token1` NOCK; `getReserves()` 911.552758 USDC / 81,388.8687 NOCK |
| Aero concentrated WETH/NOCK (tick spacing 200) | `0x957256c98d61A714081DE2EE70ae3A96c9b40874` | CLFactory `getPool(address,address,int24)` `0x28af8d0b` | `fee()` 9700; holds 0.2335 WETH and 131,330 NOCK |
| Aero concentrated WETH/USDC (50) | `0x4a7af46bc8abc00048c6c7920a1488daf568366c` | same | `fee()` 799 at block 58081910 (1282 a few minutes earlier: it moves); holds 1,396,977 USDC and 4,048.4 WETH |
| Aero concentrated WETH/USDC (100) | `0x2a430bebd2082d8e05d56d64dede8944279bbc24` | same | `fee()` 885; holds 893,364 USDC and 1,897.3 WETH |
| Aero concentrated USDC/USDT (1) | `0xd448990883e3bf9cdd8ebca368a3ecf9d629aaf7` | `factory()` is the CLFactory; `tickSpacing()` 1 | `fee()` 8 |
| Aero concentrated WETH/USDT (100) | `0x2dbe0e264c7c246e05c6614794899937f264cf7b` | `factory()` is the CLFactory; `tickSpacing()` 100 | `fee()` 468 |
| Uniswap v3 WETH/USDC 0.05% | `0xd0b53d9277642d899df5c87a3966a349a798f224` | v3 factory `getPool(WETH, USDC, 500)` (§1; also the worker's ETH price pool) | |
| Uniswap v3 WETH/NOCK 1% | `0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a` | v3 factory `getPool(WETH, NOCK, 10000)` | holds 0.4996 WETH and 388,929 NOCK |
| Uniswap v4 USDC/NOCK 3% | `0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13` | keccak256 of the pool key (USDC, NOCK, 30000, 300, no hooks) | StateView `getLiquidity` 127,997,653,824,339,161; tick 275113 |
| Uniswap v4 ETH/NOCK 2% | `0xd316152008e12105594c32686b9d0401fd2c60ca4805c84f280069d218b771a8` | keccak256 of (native ETH, NOCK, 20000, 400, no hooks) | `getLiquidity` 11,491,937,738,508,897,075 |
| Uniswap v4 ETH/USDC 0.05% | `0x96d4b53a38337a5733179751781178a2613306063c511b78cd02684739288c0a` | keccak256 of (native ETH, USDC, 500, 10, no hooks) | `getLiquidity` 40,914,903,574,237,929 |

The three v4 pool ids were recomputed from the keys the worker sends and then read
through StateView; two of them are the ids §5 already recorded for positions D and A.

**USDT** `0xfde4c96c8593536e31f229ea8f37b2ada2699bb2`: `token1()` of both USDT pools,
`decimals()` 6, `symbol()` "USDT". It is used only as a hop inside one Metaquoter call.
It is not added to the position-token registry, so a tracked position holding USDT is
still refused.

**Fee units.**
- Aero concentrated pools: `fee()` is in millionths. Checked with
  `quoteExactInputSingleV3(address,address,uint256,uint160)` `0x0a674142` for 1e12 wei
  WETH on the WETH/NOCK pool: the answer equals amount × slot0 price × (1 − 9700/1e6) to
  1.5 parts in 10 million (and is 33 times too large if the unit were basis points). So
  9700 is 0.97%, as §6 guessed. These fees are dynamic; the worker reads `fee()` at the
  quote block.
- Aero basic pool: the pool's own `fee()` and `stable()` revert. The factory's
  `defaultFee()` `0x5a6c72d0` returns 30 and the fee module's `customFee(address)`
  `0x4d419abc` returns 0 for this pool; `ZERO_FEE_INDICATOR()` `0x38c55d46` is 420 and
  `MAX_FEE()` 300. The pool's `getAmountOut(uint256,address)` `0xf140a35a` for 20 USDC is
  17,422,508,413,591,799,568 raw, exactly x·y=k on the reserves above with 30 basis
  points taken from the input, and exactly what the Metaquoter returns for that pool. So
  the fee in force is 0.30%. The rule for a non-zero custom fee (it replaces the default;
  420 means none) is the legacy Aerodrome factory's and was not exercised here.

### The comparison at block 58081910 (2026-11-16T05:25:57Z), 20 USDC in

| Route | Quoter | NOCK out | USDC per NOCK |
|---|---|---:|---:|
| Aero: USDC → NOCK (basic pool) | Metaquoter | 1,742.2508413591799568 | 0.01147940 |
| Aero: USDC → WETH (50) → NOCK | Metaquoter | 1,772.5259725360589554 | 0.01128333 |
| Aero: USDC → WETH (100) → NOCK | Metaquoter | 1,772.2004408122825130 | 0.01128541 |
| **Aero: USDC → USDT → WETH → NOCK** | Metaquoter | **1,773.0768618618146014** | 0.01127983 |
| Uniswap: USDC → NOCK (v4 3%) | V4Quoter | 1,716.8660568388805278 | 0.01164913 |
| **Uniswap: USDC → WETH → NOCK (v3)** | QuoterV2 | **1,772.2085413951150619** | 0.01128535 |
| Uniswap: USDC → ETH → NOCK (v4) | V4Quoter | 1,692.7635343644656443 | 0.01181500 |

Aero's best route returned 0.05% more NOCK than Uniswap's. The reference spot at that
block (the v4 3% pool's own price, the worker's `is_price_source`) was $0.01128675, so
both best routes were slightly *below* it (−0.06% and −0.01%), while the direct swap in
that 3% pool cost +3.21%. A cost over spot can be negative: the reference is one pool's
mid price, not the market. The Aero app's choice for buy 1 (through USDT) is still the
best Aero route.

The worker's own code, run against the same public RPC, produced the same shape at
block 58086356 (Aero 1,779.3262 NOCK, Uniswap 1,778.5967, Aero ahead by 0.04%).

### What a failing quote looks like

| Case | Call | Answer |
|---|---|---|
| A pool with no liquidity | QuoterV2 on the v3 USDC/NOCK 1% pool `0x7e096e98c5653bacf5352a2a3c7019757f6cf68d` (`liquidity()` = 0) | error 3 `execution reverted: Unexpected error` |
| A pool key never initialized | V4Quoter, USDC/NOCK fee 100 spacing 1 | error 3 `execution reverted`, data `0x6190b2b0…` |
| Input token not in the pool | Metaquoter, USDC into the WETH/NOCK pool | error 3 `execution reverted: TI` |
| A pool run dry part-way (5,000,000 USDC) | QuoterV2, USDC → WETH → NOCK | **a result**: 379,723 NOCK for the part that swapped, with the second pool's post-swap sqrt price at `0x1000276a4` (the minimum plus one) |
| The same, v4 | V4Quoter, USDC/NOCK 3% | error 3 `execution reverted`, data wrapping `0x7a5ed734` and the pool id |
| The same, Aero | Metaquoter, USDC → WETH (50) → NOCK | a result: 131,005.94 NOCK, post-swap price far down but not at the bound (the pool holds full-range liquidity, so it fills any size at a collapsing price) |

The fourth row matters: QuoterV2 does not signal a partial fill. It was observed to
return a partial amount with the post-swap price pinned at the range's bound, and no
error. The worker therefore reads the post-swap prices in the answer and treats one
pinned at the range's end as "unavailable", never as a quote. The Aero pools on these
routes hold full-range liquidity and fill any size at a collapsing price (last row), so
the Metaquoter was never seen to answer for a pool with no liquidity left; its answer
has the same shape and the same price-bound check is applied to it.

The retired v4 USDC/NOCK 0.75% pool (`0x91a9ef0a…f99c`, zero in-range liquidity) answers
1,035.83 NOCK for 20 USDC by crossing empty ticks to reach some liquidity: a real quote
40% worse than the others. It is not in the route table.

### Swap links

- **Aero**: `https://app.aero.xyz/swap`. The path is the app's own "Trade" entry
  (read from the app's route table in its public bundle). Aero's documentation
  (`app.aero.xyz/docs`, Trading) documents no query parameters, so nothing is prefilled.
- **Uniswap**: `https://app.uniswap.org/swap`. Uniswap's documentation
  (`developers.uniswap.org/docs/trading/custom-interface-links`) documents
  `inputCurrency`, `outputCurrency`, `value` and `field`, but no chain parameter.
  Without one a link cannot say the addresses are on Base, so nothing is prefilled
  here either.

### Not verified

- **The Aero Metaquoter on a concentrated pool with no liquidity left.** No such pool
  exists on these routes to test against: the CLFactory resolves one USDC/NOCK or
  WETH/NOCK concentrated pool (`0x9572…0874`), and it and the hop pools fill any size.
  Whether the Metaquoter then reverts or returns a partial amount with the price at the
  bound was not observed. The worker handles both: a revert reads "unavailable", and so
  does a post-swap price at the bound.
- **The worker's calls through Alchemy.** This branch had no key. The whole read path
  ran against the public RPC only. Each quote is pinned to the latest block number; if
  a node behind Alchemy's balancer has not seen that block yet the call is an RPC error
  (not a revert) and is retried twice before the route reads "unavailable".
- **Compute-unit cost** on the operator's plan: one block header, seven quoter calls and
  up to three fee reads per cycle while a tranche is due, and per refresh. The fee
  reads run together, once each, inside two seconds; one that does not answer in time
  leaves that hop's fee "not read" and the quotes stand.
- **Wallet-specific fees on Aero.** The Metaquoter is called without a `from`. Its
  interface speaks of a pool's "total contextual fee" and Aero deploys a
  DiscountRegistry; if the operator's wallet has a discount the real swap returns
  slightly more than the quote. The replay above matched exactly, so none applied then.
- **The newer V4Quoter.** Uniswap's SDK is moving to a later V4Quoter deployment on
  Base; the one used here is the original, and `msgSender()` reverts on it. It answers
  the quote calls correctly today.
- **What the apps' routers will do.** A route table is not the router. The Uniswap app
  can mix v3 and v4 pools or split an order; the Aero app can pick any path. The quotes
  are for the named routes only.
- **Gas.** No quote includes the swap's gas (the V4Quoter and QuoterV2 return an
  estimate; it is not shown). Gas is ETH, a different pillar.
- **The legacy Aerodrome vAMM** `0x85f1aa3a…d626` (plan buy 3 went through it, §10) is
  deep, but `app.aero.xyz` does not list it. It is not quoted.
## 12. Accumulation Fills Behind the Cost-Basis Fixtures (#98) — Read 2026-11-16

The cost basis (issue #98) adds **no contract address, function selector or event topic**.
It reads the `accumulation_events` table, whose rows the record handler already wrote
from receipt logs (`parse_accumulation_swap` in `ledger.rs`). Its unit-test fixtures
(`ingestion-worker/src/tests_cost_basis.rs`, `frontend/src/costBasis.test.ts`) are the
operator's three real plan buys, re-read from Base here so the expected averages rest on
chain data rather than on the ledger's own copy.

RPC: `https://base-rpc.publicnode.com`, read-only. Chain head at the time:
`eth_blockNumber` → `0x375f6e7` (58062567).

### Scaling assumptions, re-read

| Call | To | Data | Raw result | Meaning |
|---|---|---|---|---|
| `decimals()` `0x313ce567` | NOCK `0x9B5E262cF9bb04869ab40b19AF91D2dc85761722` | `0x313ce567` | `0x…0010` | 16 |
| `decimals()` `0x313ce567` | USDC `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913` | `0x313ce567` | `0x…0006` | 6 |

Both at `latest` (head 58062567). `keccak256("Transfer(address,address,uint256)")` was
recomputed as `0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef`, the
topic `ledger.rs` already matches.

### The three receipts (`eth_getTransactionReceipt`, then `eth_getBlockByNumber`)

Each receipt has `status 0x1`, `from` = the wallet
`0xeE114cC97eC5adeB34bA5645e315B497D8487a45`, exactly one USDC `Transfer` out of the
wallet and exactly one NOCK `Transfer` into it.

| Tx | Block | Block `timestamp` | USDC out of wallet (raw `data`) | NOCK into wallet (raw `data`) | `to` |
|---|---|---|---|---|---|
| `0x5d0c02d9929f49f71526e0c8d91d1bd99ec19f116befc285d67f6bf9412a3f0e` | 57228848 (`0x3693e30`) | `0x6af7f971` = 2026-11-14T06:02:25Z | `0x…01312d00` = 20000000 | `0x…ddc205801b5ceaca` = 15979340475636968138 | `0x0fef9e88…ee98` (Aero router, §6) |
| `0x4644edd99bb2770f369862357eb5647a68a7157ca9e91ecccfce79e6964c61e8` | 57605753 (`0x36efe79`) | `0x6af91fe6` = 2026-11-15T02:58:46Z | `0x…01312d00` = 20000000 | `0x…eeba6ce707ab3fb6` = 17202181466216939446 | `0x0fef9e88…ee98` (Aero router, §6) |
| `0x78376f3cc9e80f2eb124a945a452df40556649e3d9983be526b2bd97586e31b9` | 58040708 (`0x375a184`) | `0x6afa73b5` = 2026-11-16T03:08:37Z | `0x…01312d00` = 20000000 | `0x…f713117c2d57cfbb` = 17803592976960049083 | `0xcaf22ce3…7c67` (not identified; acceptance is contract-agnostic) |

### What follows from them

| After buy | USDC spent | NOCK received | Price paid | Running average cost |
|---|---|---|---|---|
| 1 | 20 | 1,597.9340475636968138 | $0.01251616 | $0.01251616 |
| 2 | 20 | 1,720.2181466216939446 | $0.01162643 | $0.01205490 |
| 3 | 20 | 1,780.3592976960049083 | $0.01123369 | $0.01176814 |

Total: $60 for 5,098.5114918813956667 NOCK. These are the figures the tests assert, and
they match what the deployment's `GET /v1/accumulation/events` returned on the same day
(plan `2026-11-reentry`).

### Not verified

- **The price at each buy's block.** The series pairs each buy with the nearest *stored*
  spot within 60 minutes (`market_price_history`), not with a pool read at the block. No
  historical pool read was added.
- **The third buy's router** (`0xcaf22ce31298cf2bf1d152862f80216478ad7c67`). Its code
  and name were not looked up. The record handler accepts a buy on the wallet's own
  USDC and NOCK Transfer logs, whatever contract routed it.
- **The SQL.** The two new queries (the ledger read and the price-history read) are
  plain named-column SELECTs. They were not run against a PostgreSQL instance in this
  session: no local server was available and the deployment's database was not touched.

---

## 13. Closed Positions: How a Position Manager Answers for a Token Id That Does Not Exist (issue #144) — Verified Record

A configured position whose NFT no longer exists is reported as **closed** instead of
failing the cycle (`ingestion-worker/src/closed.rs`; the allow-list is in
`adapters/mod.rs`). "Closed" is stated only on proof, so this section records exactly
what each contract answers. Everything below was read with `eth_call` on 2026-11-17
through `https://base-rpc.publicnode.com`, and again through `https://mainnet.base.org`,
which returned the same bytes for every call (code, message and `data`). Reads marked
"fixture block" are pinned to **block 58646285** (`0x37edf0d`, timestamp 1794919632 =
2026-11-17T12:47:12Z, hash `0xc6a1d6c5…5cf1b1`); they are the contents of
`ingestion-worker/testdata/closed/cycle_fixture.json`, recorded by running the worker's
own cycle reads through a recording proxy (`test_record_closed_cycle_fixture`). All 48
`eth_call` answers in that file, and the block's header, were re-read on
`mainnet.base.org` at the same block and were identical.

### What happened on 2026-11-17

The operator withdrew the Aero CL position NFT #2914490 (§8) in the Aero app:

| Fact | Value |
|---|---|
| Transaction | `0x09223b2a78bf925d591f10a04687200ab7cd657c63b74313b9582bcc976d801a`, status 1 |
| Block | 58637717 (`0x37ebd95`), timestamp 1794917919 = 2026-11-17T12:18:39Z |
| From / to | the tracked wallet `0xeE114cC9…7a45` / `0x0fef9e88a7b76bcdbf451780c6774fe32965ee98` |
| NFT `Transfer` logs on the position manager | gauge `0x3bf70c88…60fd` → `0x0fef9e88…ee98`, then `0x0fef9e88…ee98` → the zero address (burn), token id `0x2c78ba` = 2914490 |
| WETH paid out | `Transfer` pool → wallet of `0x0162179a5493fdb7` = 0.099668093366959 WETH |
| `ownerOf(2914490)` at block 58637716 | `0x…3bf70c880adc8370917994ab0df68e763b7a60fd` (the gauge: a live, staked position) |
| `ownerOf(2914490)` at block 58637717 | reverts (below) |

From the next ingestion cycle on, every cycle failed with `Aero CL positions() read
failed (NFT #2914490 …)` until the config entry was commented out by hand.

### Aero CL (Slipstream) position manager `0x4857db576ee0e45f10525ec59527bfd10beec60b`

| Call | Token id | Answer | Raw revert data |
|---|---|---|---|
| `ownerOf(uint256)` `0x6352211e` | 2914490 (burned) | JSON-RPC error code 3, `Error("ERC721: owner query for nonexistent token")` | `0x08c379a0` + offset `0x20` + length `0x29` + `4552433732313a206f776e657220717565727920666f72206e6f6e6578697374656e7420746f6b656e` (zero-padded to 64 bytes) |
| `positions(uint256)` `0x99fbab88` | 2914490 (burned) | JSON-RPC error code 3, `Error("ID")` | `0x08c379a0` + offset `0x20` + length `0x02` + `4944` (zero-padded to 32 bytes) |
| both | 99999999999 and 0 (never minted) | the same two answers, byte for byte | the same |

### Uniswap v3 NonfungiblePositionManager `0x03a520b32C04BF3bEEf7BEb72E919cf822Ed34f1`

A burned id was found from the manager's `Transfer` logs to the zero address: #6604113,
burned in block 58632803 (tx `0x64e76755359e2d478ed5ca5b76f996fcc8d8e59911024352dca0b83b72cf68f3`,
another account's). One block earlier `ownerOf(6604113)` returned an owner and
`positions(6604113)` returned 12 words.

| Call | Token id | Answer | Raw revert data |
|---|---|---|---|
| `ownerOf(uint256)` | 6604113 (burned) | error code 3, `Error("ERC721: owner query for nonexistent token")` | as for the Aero CL manager |
| `positions(uint256)` | 6604113 (burned) | error code 3, `Error("Invalid token ID")` | `0x08c379a0` + offset `0x20` + length `0x10` + `496e76616c696420746f6b656e204944` (zero-padded to 32 bytes) |
| both | 99999999999 (never minted) | the same two answers | the same |

### Uniswap v4 PositionManager `0x7c5f5a4bbd8fd63184577525326123b519429bdc`

A burned id: #3229273, burned in block 58641149 (tx
`0x7af0d60ce11c1ee02dc0f764a5e1ec402f1f9dd378c849c088aef6576bc63819`, another account's).
**This contract does not revert on its own position reads for an id that does not
exist**, so the worker cannot tell a burned NFT from an empty one by the reads it made
before #144:

| Call | #3229273 one block before its burn | #3229273 after (burned), 99999999999 and 0 (never minted) | #3163431 and #3163432 (the wallet's own, empty, still held) | #3163450 (the wallet's live position) |
|---|---|---|---|---|
| `ownerOf(uint256)` `0x6352211e` | an owner | error code 3, `Error("NOT_MINTED")`: `0x08c379a0` + offset `0x20` + length `0x0a` + `4e4f545f4d494e544544` | the tracked wallet | the tracked wallet |
| `getPoolAndPositionInfo(uint256)` `0x7ba03aad` | pool key and packed info | **six zero words (192 zero bytes), a successful call** | pool key and packed info (non-zero) | pool key and packed info |
| `getPositionLiquidity(uint256)` `0x1efeed33` | `0xc1de40e675` | **0, a successful call** | **0** | `0xe3402a72bc7b` |

So before #144 a burned v4 NFT did not stall ingestion: it read as a live position
holding nothing. The StateView reads the adapter makes next (`getSlot0`, `getPositionInfo`,
`getFeeGrowthInside`) are keyed by the configured pool id and ticks, not by whether the
NFT exists.

The pool id of #3163431 used by the fixture, `0x91a9ef0a…f9f99c`, is keccak256 of the
pool key its `getPoolAndPositionInfo` returns (USDC, NOCK, fee 7500, tick spacing 75, no
hooks); its first 25 bytes are the pool id packed in the position info word, whose
ticks are 267825 and 271050. The same derivation reproduces the known pool id of
#3163450.

### What counts as proof

| Protocol | Position manager (the only one on the allow-list) | Closed when BOTH, in the same cycle, at the proof block |
|---|---|---|
| `aerodrome-cl` | `0x4857db576ee0e45f10525ec59527bfd10beec60b` | `positions(id)` reverts with exactly `Error("ID")` AND `ownerOf(id)` reverts with exactly `Error("ERC721: owner query for nonexistent token")` |
| `uniswap-v3` | `0x03a520b32C04BF3bEEf7BEb72E919cf822Ed34f1` | `positions(id)` reverts with exactly `Error("Invalid token ID")` AND `ownerOf(id)` reverts with exactly `Error("ERC721: owner query for nonexistent token")` |
| `uniswap-v4` | `0x7c5f5a4bbd8fd63184577525326123b519429bdc` (fixed in the worker) | `getPositionLiquidity(id)` is 0 and `ownerOf(id)` reverts with `Error("NOT_MINTED")` at `latest`, THEN at the proof block `ownerOf(id)` reverts with exactly `Error("NOT_MINTED")` AND `getPoolAndPositionInfo(id)` returns exactly six zero words. Liquidity above 0 is itself proof the NFT exists (a burn clears it), so that path makes no extra read; an owner at `latest` means the NFT exists and is empty, and no proof is read. |
| `aerodrome` | none: the position is an LP-token balance, there is no NFT | never closed. A balance of zero in the pool and in the gauge is a live read of an empty position, and the gauge's `earned` is still read (AERO earned before an unstake stays claimable). |

**The proof block.** A token id that was never minted answers exactly like a burned
one (below), so a node whose state is older than an NFT's mint gives both "nonexistent"
answers for a position that is live. The proof is therefore not read at `latest`. When
the first read at `latest` gives the manager's "nonexistent" answer, the worker reads
the node's latest block header once (`eth_getBlockByNumber("latest", false)`) and makes
BOTH proving reads at that block's number (`closed::fetch_proof_block`). The block must
be current: its timestamp must be within 120 seconds of the worker's clock, or within
`sync_window_seconds` if that is shorter (`closed::ProofClock`), in either direction.
A node that is further behind fails the cycle ("the node's latest block N is S seconds
old … a node that is behind the chain cannot prove that a token id does not exist"), as
does a header that cannot be read, and a proving read answered by a backend that does
not hold that block (a JSON-RPC error, not a revert). Because the limit is never longer
than the cycle interval, a proof block is newer than every earlier cycle: a position
that an earlier cycle read as live cannot be reported closed by a node that has not yet
seen its mint. This rests on the host's clock being right to within the limit; a clock
that is off by more fails the cycle for a closed entry, in both directions.

The reason is decoded strictly from the revert `data` of the JSON-RPC error
(`rpc::decode_revert_reason`): selector `0x08c379a0`, offset `0x20`, a length that fits
the data exactly, zero padding, valid UTF-8. A length word larger than the data is
refused before any arithmetic on it. The node's `message` text is not used.
Everything else fails the cycle as before: a transport failure, an HTTP error status
(whatever its body carries), an unparseable body, a JSON-RPC error that is not an
execution revert (each retried three times first), a revert with any other reason, with
empty data, with no `data` field, with a custom error or a panic, a position manager
that is not on the list, and one read saying "nonexistent" while the other does not.

A token id that was never minted answers exactly like a burned one on all three
managers, so the worker cannot tell a withdrawn position from a mistyped `nft_id`, and
every text says both: "withdrawn and burned, or a mistyped nft_id".

A `uniswap-v4` or `uniswap-v3` entry with no `nft_id` reads as token id 0, which was
never minted on either manager (the v4 answers for id 0 at the fixture block are in the
fixture: liquidity 0, `Error("NOT_MINTED")`, six zero words). It is therefore closed
like any other id that does not exist. Before #144 a v4 entry without `nft_id` read as
a live position holding nothing and could carry `is_price_source`; as the only
configured entry it now fails the cycle with "No live positions: every configured
position is closed". A setup that only watches a price uses an `aerodrome` entry, which
has no NFT and is never closed.

### The price source, when its position is closed

- **Uniswap v4**: the pool goes on supplying the NOCK/USD spot. The price of a v4
  position was always two reads that never involved the NFT: `poolKeys(bytes25)` on the
  PositionManager (the pool key must be the config's `currency0`/`currency1`, in order)
  and StateView `getSlot0(poolId)`; the stale-pool guard `getLiquidity(poolId)` still
  runs. Verified at the fixture block: with the configured price-source entry pointed
  at the nonexistent id #3229273, the spot is the same number as with the live NFT
  (0.014934577766643168 USD per NOCK).
- **Uniswap v3 and Aero CL**: the cycle fails with a configuration error naming the fix
  (flag another position as `is_price_source`). The tokens of the pool, and for Aero CL
  that `pool_address` is the NFT's pool at all, are read from the NFT, which is gone.

### The gauge of the withdrawn Aero CL position

At block 58637716 (before the withdrawal) `earned(wallet, [2914490])` on gauge
`0x3bf70c88…60fd` returned 0 (why it was zero was not looked into; the operator had
collected this gauge's reward earlier that day, §9). After the withdrawal it reverts with `Error("NA")` (`0x08c379a0` … `4e41`) and
`stakedContains(wallet, 2914490)` returns false. The wallet's AERO balance is the same
at blocks 58637716 and 58637717 (`0xf4ffa950a248bebf`). So there is nothing to read for
a closed Aero CL position, and the worker does not call its gauge.

### Not verified

- **Alchemy's response for a reverting `eth_call`.** The deployment reads through
  Alchemy; this record was read through two public nodes, which both send JSON-RPC
  error code 3 with the revert bytes in `error.data`. The worker requires that `data`
  field. If a provider reports the revert without it, the position is not reported as
  closed and the cycle fails with "reverted with no revert data from the node", which
  is the behaviour before #144 with a more precise message. The first withdrawal on the
  deployment after this change will show which it is.
- **The legacy Slipstream position manager** `0x827922686190790b37229fd06084350E74485b72`.
  A never-minted id (99999999999) reverts there with the same two reasons as on the
  current manager (fixture block). A burned id was not read on it, so it is not on the
  allow-list: a missing NFT there fails the cycle and says the manager is not verified.
- **Any other chain, and any other deployment of these contracts.**
- **The burn itself.** The worker proves that the id does not exist now; it does not
  look for the `Transfer` to the zero address, so it does not know when, in which
  transaction, or whether the id ever existed. `first_seen_closed_at` is the time of the
  first cycle that recorded it as closed.
- **That a token id is never minted again.** All three managers mint increasing ids, so
  a burned id should stay nonexistent; this was not checked in their code. If an id did
  exist again the position would simply read as live again.
- **A node that is behind the chain by less than the proof-block limit.** The proof
  block must be at most 120 seconds old (above), so what remains is an NFT added to the
  config and deployed within that limit of its own mint, read through a node that has
  not seen the mint yet: it would be listed as closed for that cycle and as live on the
  next, with no notification (one is sent only for a position the previous snapshot
  held as live). The limit itself was not tuned against the deployment's provider: if
  its `latest` is routinely older than that, a closed entry fails the cycle and the
  message says how old the block was.
- **The host's clock.** The freshness of the proof block is judged against it. A clock
  that is wrong by more than the limit fails the cycle while a closed entry is
  configured; one that is slow by less than the limit loosens the check by that much.
- **StateView reads for a burned v4 id in its own pool.** The adapter never reaches
  them for a closed NFT, and they were not read for #3229273.

---

## 14. Principal in a Harvest Receipt (issue #156) — Verified Record

Read on Base on 2026-11-18. `record-harvest` takes principal out of a "remove liquidity"
transaction before booking it (`ingestion-worker/src/harvest_principal.rs`).

**Event topics** (keccak256 computed with viem, then found in live logs):

| Event | Topic | Seen in |
|---|---|---|
| `DecreaseLiquidity(uint256 indexed,uint128,uint256,uint256)` | `0x26f6a048…77b4` | Uniswap v3 NPM `0x03a520b3…34f1` and Aero CL position manager `0x4857db57…c60b` |
| `Collect(uint256 indexed,address,uint256,uint256)` | `0x40d0efd1…8f01` | both managers |
| pool `Burn(address indexed,int24 indexed,int24 indexed,uint128,uint256,uint256)` | `0x0c396cd9…982c` | v3 and Slipstream pools |
| Aero basic pool `Burn(address indexed,address indexed,uint256,uint256)` | `0x5d624aa9…6fa2` | refused, not split |
| v4 `ModifyLiquidity(bytes32 indexed,address indexed,int24,int24,int256,bytes32)` | `0xf208f491…d5ec` | refused when `liquidityDelta` < 0 |

**The operator's withdrawal of v3 WETH/NOCK #6336662.** Tx `0x06515a23413aff3f56dab78c4989229352e80fc48d70f503f94158a70e78b778`,
block 58831000, sent by the wallet to the NPM. `positions(6336662)` at block 58830999
gives token0 WETH, token1 NOCK, and liquidity 18841285041665077684, which is the whole of the
liquidity the transaction decreased.

| | WETH | NOCK |
|---|---|---|
| `DecreaseLiquidity`: principal | 0.102325535439356415 | 905.2303512140725855 |
| `Collect`: principal + fees | 0.103325038812771257 | 1006.8107081028485540 |
| Booked as fees | 0.000999503373414842 (paid as ETH) | 101.5803568887759685 |

The NPM collects to itself, unwraps the WETH (WETH `Withdrawal`, `src` = NPM) and sends
ETH, then transfers the NOCK to the wallet. The receipt and the `positions()` answer are
the fixtures in `ingestion-worker/testdata/harvest/`.

**Aero CL.** Tx `0x7ba5de12ae2fa2c42d53718e15a49ab85832dc578e25673502444a5333fd39cc`
(block 59100004, a third party's withdrawal) carries the same three topics from the Aero CL
position manager and its pool, and burns the NFT in the same transaction. That is why a
decreased position's tokens are read at the block before the transaction.

**Not read:**
- **A Uniswap v4 or Aero basic liquidity removal.** These are refused, not split.
- **The legacy Aero CL position manager `0x82792268…5b72`.** It is not listed, so a Burn
  it owns is refused.
- **A partial decrease collected in a later transaction.** If the position still holds
  liquidity, the later collect is booked in full. A collect from a position with no
  liquidity left is refused instead.

**Collect recipients.** In the withdrawal above, the NPM's `Collect` names the NPM itself as
recipient. It then unwraps the WETH and sweeps the NOCK to the wallet. A decrease counts as
collected towards the wallet when its position's `Collect` pays at least the principal to
one of these:
- the wallet;
- the position manager, or the zero address, which the v3 NPM reads as itself;
- the contract the wallet called, such as a router.
