# Operator Briefing: Portfolio Reality After the Aerodrome Liquidation (2026-10-24)

> **An Operator Briefing of Urmanac Sovereign Infrastructure**  
> *Verified on-chain 2026-10-24 ~03:10 UTC · Uniswap v3 + v4 on Base*

## What Happened

On 2026-10-23 the operator liquidated the Aerodrome tranche and re-opened the portfolio as concentrated liquidity on Uniswap (Base). The app still models the old shape — it reads a dead Aerodrome pool and the retired V4 NFT #3126000, so its numbers are fiction.

## The Real Portfolio (Verified On-Chain 2026-10-24 ~03:10 UTC)

All positions live on the tracked wallet `0xeE114cC97eC5adeB34bA5645e315B497D8487a45` (Base). NOCK ≈ $0.0206, ETH ≈ $2,368.

| NFT | Pool / tier | Holding | Value |
|---|---|---|---|
| 3163431 | USDC/NOCK v4 · 0.75% | 43.0 USDC + 1,363 NOCK | $71.11 |
| 3163432 | ETH/NOCK v4 · 2% | 0.0100 ETH + 6,372 NOCK | $131.41 |
| 3163450 | USDC/NOCK v4 · 3% (wide) | 134.4 USDC + 3,904 NOCK | $214.93 |
| 6336662 | WETH/NOCK v3 · 1% | 0.0195 WETH + 37.6 NOCK | $46.97 |

- Wallet free: ~104,000 NOCK ($2,144) + 0.0027 ETH ($6.40) + 5.38 USDC
- Unclaimed fees: ~0.67 NOCK (dust)
- **True total ≈ $2,620** — the wallet alone shows only ~$2,156, because LP holdings are invisible to token balances.
- All four positions were in range at last read. Fee tiers, ticks, and poolIds are recorded in the spec task record (`spt_01m5nqxqr1ma3qhsqsx6vw5kq7`).

## The Fix (In Flight)

Spec task `spt_01m5nqxqr1ma3qhsqsx6vw5kq7` ("Uniswap-Only Portfolio After Aerodrome Liquidation") is in implementation:

- Config-driven position registry (so the next repositioning is a config edit).
- Per-position v4+v3 reads; ETH/USD via Chainlink (stale-checked).
- The cockpit renders one card per live position; Aerodrome logic removed.

The PR will land here; after merge, `docker compose up -d --build` shows the live reconciliation above.

## Acceptance

- [ ] Cockpit totals reconcile to wallet + positions + fees (no Aerodrome rows)
- [ ] One card per live position with pair, protocol badge, fee tier, range, split, value, in-range pill
- [ ] Golden Path never proposes an Aerodrome `claimFees()` transaction

---

*Related: [the WETH→NOCK ladder decision](operator-decision-weth-nock-ladder-2026-10-24.md) on NFT 6336662 · [position-intents design note](operator-note-position-intents-design-space.md).*
