## The situation, plainly

NFT **6336662** (WETH/NOCK, Uniswap v3, 1% tier) currently holds ~**$47, of which ~$46 is WETH**. Its range floor sits **0.65% below the current NOCK price**.

What it does: **while NOCK stays weak (price inside the range), the pool sells that WETH and buys NOCK.** If NOCK **rises just +0.65%**, the price crosses the floor, the position exits range, and it freezes at **100% WETH (~$47)** — the buying stops until someone re-anchors it.

## The operator decision

**Option A — leave it.** If NOCK rallies, the ladder exits by itself and you simply hold the WETH. Zero action, zero gas. If NOCK falls instead, it keeps buying NOCK until the ceiling (74,800 NOCK/ETH) and then also freezes (as NOCK).

**Option B — re-anchor / widen it now.** Re-mint with the floor pushed down (or the band widened) so the WETH→NOCK conversion keeps working through more of NOCK's upside. Costs: one burn + one mint (~2 transactions, gas), and a fresh decision about how wide.

## Why this is time-sensitive

The trigger is a **+0.65% NOCK uptick** — that can happen in minutes in this market (it moved ~12% during today's data pulls). After it happens, Option B is still available, but the conversion that would have happened at the better (lower) prices is gone.

## What nocknock could do afterward (app-side, either way)

- The advice engine should be able to say this sentence in plain words ("your WETH→NOCK ladder froze; it holds X WETH and is no longer converting") — tracked in the advice-language work.

**Operator: comment with A or B (and, for B, the desired floor/width).**
