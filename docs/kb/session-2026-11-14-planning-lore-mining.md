# Session 2026-11-14 (afternoon): planning the patches, lore, and the mining reply

*Claude: Fable 5.2 to start, then Opus 5.5 at maximum thinking. A planning session: no
code changed. It follows the v0.5.0 session in
[session-2026-11-13-chudmax-dca-aerodrome.md](session-2026-11-13-chudmax-dca-aerodrome.md).*

## What the operator asked for

- Plan what comes after v0.5.0, because there was weekly capacity to use before Monday.
- Write a message introducing NockNock to the Chat agent, which still thought the week was
  about Mecris. It was to be honest, including the term "vibe-coded".
- Answer two v0.5.1 questions, which the operator did:
  - **ETH (#94)** is "gas": a pillar with value and utility, counted in the total, but
    neither cash nor investable.
  - **Backups (#96)** are not wanted: "We're not in production, and we're not calling
    anything a backup." The database is disposable and the wallet is the record.
- Write lore documentation presenting the position: four pillars, an underwater cost
  basis, and no hurry to average down.
- Consider a test case for a newcomer with an empty portfolio asking for shot advice: "we
  are building in public."
- Respond to the Chat agent's handoff on L1 mining and the Mecris boundary.
- In maximum-thinking mode, plan rather than build.

## What was done

**GitHub:**
- **Milestones:** v0.5.1 (#61, #63, #94, #95), v0.6.0 (#97, #91, #93), v0.7.0 (#89).
- **Closed as shipped in v0.5.0:** #79, #81, #83, #88.
- **#96** closed as not planned, by operator decision.
- **#98** filed: cost basis, and a chart of average cost against spot.

**Lore:** [docs/lore/2026-11-14-four-pillars-no-hurry.md](../lore/2026-11-14-four-pillars-no-hurry.md),
covering:
- the four pillars (NOCK, cash, gas, AERO);
- the Uniswap app's cost-basis figures, with caveats;
- the arithmetic of averaging down. At $0.0115, pulling the average within 10% of the
  price takes about $4,700 of NOCK, roughly five times the cash pillar.

**AGENTS.md §13:** chudmax is infrastructure, not production, and the nightly `pg_dump` is
not a backup.

**A verified code map** (a multi-agent workflow): six read-only areas, each re-checked by an
independent skeptic, plus a cross-area critique. It found much more than the open issues:
- **The false HOLD went out live.** At 05:36Z, HOLD read "4 Uniswap position(s) generating
  yield … 0% realized loss" while LP held $0.
- **So did a lump-sum buy order.** At 04:38Z, the planless dip arm read "Deploy $1053.00 of
  saved USDC … toward 120,000".
- **The Portfolio Composition chart is about 8 hours stale.**
- **`GET /v1/snapshot/{id}` returns 500 for every snapshot.**
- **A newcomer inherits the operator's config.** They would see the operator's wallet and
  positions as their own, a default 100,000 floor would put them in "CRITICAL … locked",
  and the planless dip arm would size a lump-sum buy of 95% of their cash.
- **A wallet with no LP cannot run NockNock at all.**

**The plan:** [docs/kb/plan-v0.5-patches.md](plan-v0.5-patches.md).
- **v0.5.1, "Honest numbers":** must deploy before the DCA plan completes around
  2026-11-18, when the false HOLD would return.
- **v0.5.2, "From zero":** ingestion and config changes plus a chudmax config migration,
  kept separate because that kind of change caused yesterday's outage.
- **14 decisions**, each with a recommendation.

**The mining reply:** [docs/kb/plan-l1-mining.md](plan-l1-mining.md).
- **Bridge rules verified:** 100,000 NOCK minimum, 0.30% fee, one direction only.
- **The GPU box is chudmax**, and Strata holds 15.1 of its 16.3 GB.
- **Nockpool is deprecated**, and the GPU miners are binary-only.
- **The public earnings numbers don't reconcile:** measure for 24 hours instead.
- **The yardstick is buying:** at $0.15/kWh, the electricity alone would buy about 50 NOCK
  a day on Aero.
- **Proposed placement:** mining telemetry in v0.7.0, notifications moved to v0.8.0.
  Nothing built until the operator agrees.

## State at the end

- **No code changed.** Branch `fix/v0.5.1` holds uncommitted docs only: the lore, AGENTS.md
  §13, the two plan files and this log. Nothing is committed or pushed.
- **chudmax is unchanged.** It still runs v0.5.0 (the `3d58f84` tree) and is healthy.
- **Live plan:** `2026-11-reentry` has fill 1 of 5 recorded; tranche 2 is due
  2026-11-15T06:02Z.

## Mistakes and corrections

- **My first #94 design was wrong.** I proposed measuring the high-water mark on value
  excluding wallet ETH. A skeptic showed that moving ETH into or out of a WETH/NOCK range
  would then register as a fake gain or drawdown. The plan now uses the full total, with a
  one-time step marked on the chart.
- **Review caught five errors in the lore draft:**
  - it double-counted the plan's first $20 buy;
  - it said the plan "buys", when it only paces the operator's buys;
  - it overstated what is read from the chain;
  - it said a database wipe loses `config.yaml`;
  - it claimed NockNock counts all NOCK, when the Aero CL position is untracked.
- **I reversed my own advice.** Earlier in the day I said not to plan 0.5.2. The map
  reversed that.
- **The first workflow launch failed** on a `${…}` inside a JavaScript template string.
- **One request was blocked, correctly.** A direct call to the NockBlocks explorer's API sits
  behind a Cloudflare bot check, and the auto-mode classifier blocked it. I used public
  pages instead and didn't work around it.
