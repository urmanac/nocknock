# Operator Decision: The WETH→NOCK Ladder on NFT 6336662 (Option A or Option B)

> **An Operator Decision Briefing of Urmanac Sovereign Infrastructure**  
> *Priced from 2026-10-24 data pulls · decision recorded 2026-10-24*

**RESOLVED — Option A (hold, no transaction).** The position stops converting on a +0.65% NOCK uptick and holds ~0.0195 WETH; re-anchoring remains available later if the operator changes his mind.

## The Situation, Plainly

NFT **6336662** (WETH/NOCK, Uniswap v3, 1% tier) currently holds ~**$47, of which ~$46 is WETH**. Its range floor sits **0.65% below the current NOCK price**.

What it does: **while NOCK stays weak (price inside the range), the pool sells that WETH and buys NOCK.** If NOCK **rises just +0.65%**, the price crosses the floor, the position exits range, and it freezes at **100% WETH (~$47)** — the buying stops until someone re-anchors it.

## The Operator Decision

**Option A — leave it.** If NOCK rallies, the ladder exits by itself and you simply hold the WETH. Zero action, zero gas. If NOCK falls instead, it keeps buying NOCK until the ceiling (74,800 NOCK/ETH) and then also freezes (as NOCK).

**Option B — re-anchor / widen it now.** Re-mint with the floor pushed down (or the band widened) so the WETH→NOCK conversion keeps working through more of NOCK's upside. Costs: one burn + one mint (~2 transactions, gas), and a fresh decision about how wide.

## Why This Is Time-Sensitive

The trigger is a **+0.65% NOCK uptick** — that can happen in minutes in this market (NOCK moved ~12% during today's data pulls). After it happens, Option B is still available, but the conversion that would have happened at the better (lower) prices is gone.

## What NockNock Could Do Afterward (App-Side, Either Way)

- The advice engine should be able to say this sentence in plain words ("your WETH→NOCK ladder froze; it holds X WETH and is no longer converting") — tracked in the advice-language work.

---

*Related: [portfolio reality](operator-briefing-portfolio-reality-2026-10-24.md) — where this ladder sits in the four-position book · [position-intents design note](operator-note-position-intents-design-space.md).*
