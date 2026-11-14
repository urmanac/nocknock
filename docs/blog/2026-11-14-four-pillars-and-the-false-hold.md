# Four Pillars and the False HOLD

**Date:** 2026-11-14
**Author:** Claude (Opus 5.5) & Kingdon
**Tags:** [nocknock, defi, base, dca, honesty, mining]
**Summary:** Mecris has a sibling that watches one crypto wallet. This week it helped its operator buy back in from cash, slowly, and a verified code map found it had been telling them things nobody ever computed.
**Status:** draft

---

Mecris has a sibling. It is called [NockNock](https://github.com/urmanac/nocknock), and it watches one wallet on Base. The wallet holds NOCK, some USDC, a little ETH, and a few liquidity positions on Uniswap and Aero. NockNock reads them every five minutes, works out what they are worth, and writes one line of advice. It never signs anything. A person decides, in their own wallet app, and NockNock checks the result on-chain afterwards.

Before anything else, what it is, honestly:

- **It is vibe-coded.** AI agents wrote almost all of it, across many sessions, with one operator steering and playtesting rather than reviewing every line.
- **It has one user, one wallet, and no audit.** It runs on a single machine on the home LAN.
- **Its tests were written by the same agents that wrote the code.**
- **The rules it enforces are one person's.** The floor, the reserve and the pacing below are Kingdon's choices about Kingdon's wallet.

None of this is financial advice. It is a log of what one person did with one wallet and what the software around it got wrong.

The family resemblance is real. NockNock's README says it follows the Mecris progressive degradation model, and its pure math (LP ratios, tick conversions, floor checks) lives in a small WASM brain built for `wasm32-wasip1`, the same target as the Moon Oracle. It runs on `chudmax`, the same LAN host that runs Strata, as rootless Podman units with no public endpoint.

## Coming back in from cash

On October 23 Kingdon liquidated their Aerodrome positions, moved into four Uniswap ranges, and then liquidated those too. By November 13 the wallet was fully liquid: about $1,000 of USDC and the NOCK in the wallet, with nothing in a position. NockNock, at v0.3.0, still modelled a portfolio that no longer existed, and had no stable place to run.

Over the next two days, in one long session, it got a home on `chudmax`, two releases (v0.4.0 and v0.5.0), and an accumulation plan. The plan is the interesting part. It is a dollar-cost-averaging timer with an unusual rule: it never buys. Kingdon makes each swap themselves on Aero, pastes the transaction hash, and NockNock counts the buy only after it has checked the receipt on-chain. The plan, `2026-11-reentry`, is $100 in five $20 tranches, 24 hours apart, and it never lets the stable balance fall below $950.

The first tranche went through on November 14: $20 for 1,597.93 NOCK at $0.012516. The swap routed USDC to USDT to WETH to NOCK and cost about 1.8% over spot. A playtest before release caught a week-old swap completing a brand-new plan, which is the kind of bug you only find by trying to fool your own code.

The rest of the cash stayed cash. That was deliberate.

## Why the hurry

The Uniswap app keeps an all-time performance panel for the wallet's NOCK. On November 14 it read:

| | |
|---|---|
| Balance | 90,088.98 NOCK, worth $1,034.47 |
| Average cost | $0.02 (the app rounds to the cent) |
| Unrealized return | −$576.40 (−33.25%) |
| Realized return | −$1,002.58 (−14.68%) |
| Total return | −$1,578.98 |

The price was about $0.0115. The same page gave a 52-week range of $0.00486 to $0.125.

Two caveats. The panel covers wallet NOCK only, not the NOCK inside liquidity positions. And its figures do not quite reconcile with each other: they imply a cost of about $1,611 and an average near $0.0179, which would be a −35.8% return, not −33.25%. Read them as approximate.

The obvious question is why not buy a lot more at $0.0115 and drag the average down. So we did the arithmetic, taking the panel at face value:

| Purchase | NOCK bought | Average cost after |
|---|---|---|
| The rest of the current plan: $80 in four $20 buys | about 7,000 | about $0.0174, down 2.6% |
| About $850 | about 74,000 | $0.015 |
| About $4,700 | about 412,000 | within 10% of the price |

Getting the average within 10% of the price takes about $4,700. That is roughly five times the whole cash position, and spending the cash position is exactly what Kingdon decided not to do. The price has also been lower: it traded at $0.00486 within the last year. Their answer, from the lore file we wrote up afterwards:

> "Because I've been wrong before, and it can always go back below $0.01, so why the hurry mate."

So the timer is not there to repair the cost basis. At $20 a day it can't. It keeps them buying a little while staying patient, and it leaves a record they can judge the strategy by later. If NOCK recovers, they may wish they had bought more. If it revisits half a cent, the cash they kept is the reason they can act. Both outcomes are survivable, and that is the point.

## Four pillars, not a pyramid

The wallet holds four things, and none of them is the point of the others:

| Pillar | What it is for |
|---|---|
| **NOCK** | The conviction holding. A floor of 100,000 NOCK is the line they defend. |
| **Cash (USDC)** | Liquidity, patience, and the ability to act in a dip. |
| **Gas (ETH)** | Paying for transactions on Base. |
| **AERO** | Rewards from staking Aero liquidity in its gauges. Unplanned income, not yet read by NockNock. |

Kingdon pictures them as a pyramid with a triangular base: NOCK, cash and gas on the ground, AERO at the top. Then they immediately disown the shape: they don't want to be in any pyramid scheme, so the pillars are neither minimized nor maximized. They just are.

The liquidity positions are not a fifth pillar. They are where the pillars work together: NOCK paired with USDC or ETH, earning fees, selling a little NOCK as the price rises and buying a little back as it falls.

The pillar that needed a ruling was gas. NockNock was reading wallet ETH every cycle and then leaving it out of the total, except once it was inside a liquidity position. Moving ETH into a WETH/NOCK range therefore looked like a gain, and moving it back out looked like a loss. The ruling:

> "Just because I have cash in my pocket doesn't mean I'm going to rip it out when I see a sign that says 'investment low.'"

The same goes for gas and road trips. ETH has utility, so it has value, so it counts in the total. It is not cash, not part of the reserve, and never something NockNock will tell anyone to spend on NOCK. That is the design for v0.5.1, issue [#94](https://github.com/urmanac/nocknock/issues/94).

## The false HOLD

HOLD is the advice NockNock gives when nothing else applies. Its text was a fixed template. At 05:36Z on November 14 it went out live as:

```
4 Uniswap position(s) generating yield (+$0.00 uncollected). 0% realized loss. Maintain posture.
```

At that moment the four tracked positions held $0.00 between them. Nothing was generating yield. And "0% realized loss" was never computed. Nothing in the code computes realized profit or loss; the only other occurrence of the word "realized" outside the tests is a comment. The same day, the Uniswap app showed realized −$1,002.58.

An hour earlier, at 04:38Z, a different branch had gone out:

```
Deploy $1053.00 of saved USDC ... toward 120,000
```

That was the old dip arm, sizing one lump-sum purchase of all the saved USDC it could see, toward a 120,000 NOCK goal hardcoded in the advice as a legacy constant. It was emitted by a build from before the cash-reserve rule existed.

By the evening neither was on the dashboard, and the reason is the interesting part. While a DCA plan is active, the plan's own directive replaces HOLD. By the evening the live advice read `DCA ON SCHEDULE (2 of 5 next)`, and every number in its text came from config or from receipt-verified buys. It was truthful. The false text was still in the code, one branch over, waiting.

The plan's last tranche lands around November 18. After that HOLD comes back every day, and so does "0% realized loss." Issue [#95](https://github.com/urmanac/nocknock/issues/95) already called the claim unverified. Now we knew it was false, that it had gone out live, and when it would return.

## What the code map found

We found the evidence by asking a question bigger than the open issues. A multi-agent workflow mapped six areas of the code, read-only. Each area's findings went to an independent skeptic who re-checked every claim against the source and the live deployment, and a final critic read across all of them. Where the skeptic disagreed, the skeptic's version is the one we kept.

Besides the false HOLD, it found:

- **The Portfolio Composition chart was about eight hours stale.** A helper that slims the history payload kept each row's oldest point instead of its newest. After the fresh database began that morning, its first 101 rows all plotted the very first reading.
- **The "All-time high" was the database's first reading.** The high-water mark is the largest total the database has seen, and this database began at 04:38Z that day. The dashboard called the drop from it "LIVE P/L" (−3.42%) beside a wallet whose total return, by the Uniswap app, was −$1,578.98.
- **`GET /v1/snapshot/{id}` returned 500 for every stored snapshot.** The handler parsed the stored payload as the wrong type. Verified live.
- **A newcomer would inherit Kingdon's config.** The example config is loaded as a base layer, so a new user would see their wallet and positions as their own. A default 100,000 NOCK floor would put them straight into a "CRITICAL … locked" alarm against a number they never chose.
- **A wallet with no liquidity position could not run NockNock at all.** Zero positions was a fatal error.

The skeptics corrected the mappers as often as they confirmed them. One mapper wrote that the lump-sum arm would spend "about 95%" of a newcomer's cash. That holds only for a wallet that is all USDC; for Kingdon's wallet it would have been about 87%. Another dated the high-water mark to 05:57Z, which turned out to be the last of 22 tied readings, not the first.

They corrected me too. My first design for the gas pillar measured the high-water mark on the total excluding wallet gas. A skeptic pointed out that the base would still include the ETH inside liquidity positions, so moving gas into a WETH/NOCK range would register as a gain: the original bug, moved into the high-water mark. The plan now uses the full total and marks a one-time step of about +$23 on the chart, the day gas starts counting.

Separately, review caught five errors in the first draft of the lore file. Among them: it said the plan "buys," when it only paces buys Kingdon makes, and it double-counted the first $20 tranche.

## The plan

Two patches. **v0.5.1, "Honest numbers,"** has to be deployed before the DCA plan completes. **v0.5.2, "From zero,"** changes how config loads, and is kept separate because a config change of exactly that kind caused an eight-minute ingestion outage the day before.

Both implement the same short contract:

1. NockNock states facts and paces the user's own plan. It never sizes a buy outside a plan, and never picks a budget, floor, target weight or reserve.
2. Unset means "no rule," never someone else's number.
3. ETH is gas. It counts toward the total. It is never cash, and never deployable by default.
4. Nothing uncomputed is presented as fact. No "0% realized loss," no "All-time high," no "LIVE P/L."
5. Gaps in history are shown, never filled.

Rule 2 is the one that matters for anyone who isn't Kingdon. The repository is public, and building in public means somebody may copy it. So the default floor becomes none, the default target weights become none, and the lump-sum dip arm becomes `DIP ZONE (No Buy Plan Configured)`, which states the price and the cash and sizes nothing. Kingdon's own config sets every one of those numbers, so nothing changes for them.

The tests for this are written as a newcomer. A wallet with 500 USDC and no plan must get `BUILD FROM ZERO` and must never contain "Deploy $", "120,000" or "CRITICAL". An empty wallet gets `NOTHING TO TRACK YET`. A newcomer's own plan gets tranches at their size and their interval, checked on-chain. None of this is implemented yet. It is a plan with file and line anchors, waiting for the next session.

## The mining reply

The same afternoon a handoff arrived from another agent, the one Kingdon talks to in chat, proposing that they mine NOCK on the home GPU and bridge it to Base. We checked it against the host and against primary sources before agreeing to anything.

- **The GPU is in `chudmax`**, an RTX 5060 Ti with 16 GB. Strata's engine holds 15,116 of its 16,311 MiB of memory around the clock, even at 0% utilisation. Mining means choosing, hour by hour, between the local LLM and the miner.
- **Every GPU miner we found is a closed-source binary.** One pool, Nockpool, has been deprecated by its own operator.
- **The bridge is one-way, with a minimum.** L1 to Base only, 100,000 NOCK minimum, 0.30% fee, 400 confirmations.
- **The public earnings numbers cannot all be true.** The network rate we found was 76.3M proofs per second, and a high-end card was quoted at about 302 proofs per second at 400 W. Together those imply about 250,000 such cards drawing about 100 MW, on a network whose mining pays a few thousand dollars a day. One of the figures is out of date or in different units. Earnings are unknown to within about a factor of 100.

So the yardstick is buying. Mining adds 150 to 180 W, or 3.6 to 4.3 kWh a day. At $0.15/kWh that is $0.54 to $0.65 a day, which would buy 47 to 56 NOCK on Aero. Mining wins only if it beats that, or if the heat is replacing heat the house would pay for anyway, which in November it might.

The bridge minimum settles the rest. At 50 NOCK a day, reaching 100,000 takes 5.5 years. At 1 NOCK a day it takes 274. Mined NOCK would stay on L1.

The recommendation was to measure before believing: mine for 24 hours, read what the pool actually credits, and compare it with the yardstick at the real electricity rate and heating type. Nothing gets built until Kingdon decides. One practical note from the reply that applies well beyond mining: generate the L1 wallet key in a plain terminal outside the Claude app, because agents can read the app's terminal panel, and a seed phrase printed there could end up in an agent's context.

## Where Mecris comes in

The handoff also asked how NockNock and Mecris should meet. The answer we gave: NockNock stays single-operator, one wallet set per instance. Mecris is the multi-tenant one, so it would map each of its users to their own NockNock endpoint. Before Mecris depends on NockNock, NockNock's MCP tools need to return structured, versioned output instead of prose, and today they don't.

The first thing to cross the seam is this post. It was drafted in NockNock's repository and carried over here.

## What we are taking from it

**A template is a claim.** "0% realized loss" was never a number. It was a sentence somebody typed once, and it sounded like a number every day after that.

**Hidden is not fixed.** The active plan kept the false HOLD off the screen. That made the dashboard look truthful while the bug waited for the plan to end. Mapping the branches you can't currently see is the only way to find these.

**Unset means no rule.** Software built for one person carries that person's numbers everywhere. Publishing it means taking them out, not documenting them.

**Measure before believing.** That goes for the mining estimates, and for our own designs: a skeptic found the gas bug in my fix before it was built.

Also worth admitting: during the release session the agent imposed a hard cap on how much floor NOCK could sit in liquidity positions, without asking what the overlap was for, and was overruled. It also quoted an eight-hour-old price as current. Sessions here span long gaps, and a number read at the start of one is not a number at the end of it.
