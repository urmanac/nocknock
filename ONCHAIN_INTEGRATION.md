# On-Chain Integration Protocol & Verification Findings

This document captures the verified on-chain mechanics, smart contract functions, selector analysis, and fee dynamics discovered during live RPC inspection on Base mainnet.

---

## 1. Core Addresses & Contracts (Base Mainnet)

| Component | Address / Identifier | Description |
|---|---|---|
| **Tracked Wallet** | `0xeE114cC97eC5adeB34bA5645e315B497D8487a45` | Sovereign operator address |
| **NOCK Token** | `0x9B5E262cF9bb04869ab40b19AF91D2dc85761722` | Sovereign asset (16 decimals) |
| **USDC Token** | `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913` | Settlement currency (6 decimals) |
| **Aerodrome Pool (vAMM-USDC/NOCK)** | `0x85f1aa3a70fedd1c52705c15baed143e675cd626` | **HISTORICAL — tranche liquidated 2026-10-23** |
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
with `protocol: aerodrome` (issue #88).

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

- **How to deposit: partly checked (2026-11-14).** `app.aero.xyz` does not know this
  pool: `https://app.aero.xyz/deposit/8453-0x85f1aa3a…d626` answers "Pool not found",
  and its pool list offers only a new Basic Volatile NOCK/USDC pool
  `0x770fE8Fc9248c6382950A86E39a9391B41D59e0A` (about $915 TVL) and the CL200 WETH/NOCK
  pool. The legacy app at `aerodrome.finance` is still served (v5.0.0) and accepts the
  route `/deposit?token0=<USDC>&token1=<NOCK>&type=-1` (volatile), stopping at wallet
  connection. The rebuild suggestion links there. What that page offers once a wallet
  is connected was not seen. To track a pool on the new app instead, change
  `pool_address` and set `deposit_url: https://app.aero.xyz/deposit/8453-<pool>`.
- **What an unstaked LP earns now.** Swaps on the legacy pool still emit fee events,
  but reward emissions stopped at the merger and current fee distribution was not
  confirmed. (aero.xyz states that new-Aero pool revenue goes to AERO stakers; its pool
  page lists separate AERO and fee reward rates for depositors.)
- Rewards earned by staked LP (AERO emissions) are not read or valued.

### First live deposits (2026-11-14, operator, via app.aero.xyz)

| | USDC/NOCK Basic Volatile | WETH/NOCK CL200 |
|---|---|---|
| Tx | `0x127c4f4f…f185f0` (block 57425187) | `0x80277e2b…c93ddb` (block 57425824) |
| In | 31.986644 USDC + 2,755 NOCK | 0.081995 ETH + 2,755 NOCK |
| Pool | `0x770fE8Fc9248c6382950A86E39a9391B41D59e0A` | `0x957256c98d61A714081DE2EE70ae3A96c9b40874` |
| Receipt | 29684955447174 LP tokens | NFT #2914490 on `0x4857db576ee0e45f10525ec59527bfd10beec60b` |
| Staked in | gauge `0xc82782ee66d82c4d19db0429652091a085083526` | gauge `0x3bf70c880adc8370917994ab0df68e763b7a60fd` |
| NockNock | tracked (`gauge_address`) | **not tracked** (issue #91) |

Both went through the Aero router `0x0fef9e88…ee98` (`execute`, §6), which staked the
receipt automatically. For the volatile pool `gauge.balanceOf(wallet)` returned the
minted amount exactly and `gauge.stakingToken()` is the pool; the wallet's share was
3.38% of reserves, equal to what was deposited.

