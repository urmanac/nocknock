# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **WETH/NOCK bets (#181).** A standing card that suggests a small two-sided WETH/NOCK
  range in the Aero CL200 pool around the price, staked for AERO, only while no bet is in
  range. Every number is the operator's own setting, `operator_profile.nock_bets`
  (`pool`, `bet_usd`, `width_ticks` or `width_pct`, `gas_floor_eth`), with no defaults:
  the card names each one that is missing.
  - The bet pool is read every cycle with no position in it (tokens, spacing, fee, gauge,
    position manager, factory and the factory's own `getPool`, price, liquidity).
  - The range is the pool's tick plus and minus the width, rounded to the pool's spacing;
    the WETH and NOCK amounts are the exact mint for `bet_usd` at the pool's price, printed
    beside the deposit link (`https://app.aero.xyz/deposit/8453-<pool>?range=custom&tickLow=A&tickHigh=B`,
    with `native=true` when wallet WETH is short and ETH covers it). The Aero app takes no
    amount in a link.
  - Gates, in order: the settings; the gas floor (wallet ETH and WETH above it must cover
    the ETH side; short of it, the card measures how much WETH would have to come out of
    the ETH/USDC ranges first); total NOCK held at or above the floor; no bet in range.
  - The floor-crossing price with the hypothetical bet; the AERO each bet has earned.
  - A top-of-page event while a bet is suggested; quiet lines for bets in range and bets
    out of range (cashed out). Dashboard card, phone card and the `get_shot_advice`
    section in both MCP transports.
  - **`role: bet`**: counted in every total, the NOCK floor and the floor-crossing price;
    left out of NOCK LP, the LP target and the rebuild; its own measured deviation row.
  - **Bets are found through the gauge.** A staked deposit mints the NFT to the Aero
    router and moves it to the gauge in one transaction, so it never reaches the wallet.
    Each cycle reads `gauge.stakedValues(wallet)`; a new NFT is tracked by itself only when
    its deposit transaction was sent by the wallet, minted it to the router, moved it to
    the bet pool's gauge and minted the range in the bet pool. Anything else staked there
    is offered with one tap (TRACK AS A BET, NOT NOW). Rows live in `tracked_positions`
    with protocol `aerodrome-cl`: **no migration**.
  - OPERATOR_GUIDANCE §12 is amended for bets only: ETH above the gas floor may fund a bet.
- Snapshot spec 2.12.0 (additive): `derived_metrics.bet_suggestion` (left out of the state
  hash) and `Position.role: "bet"`. With `nock_bets` unset no byte of the snapshot changes.

## [0.9.1] - 2026-11-21

A display patch for v0.9.0's ladder cards and the ladder shape chart.

### Fixed

- **Position cards no longer run into each other.** A card's badge rows now wrap, and a
  card can no longer grow past its grid column, so "LADDER · CAP" and "● LADDER ABOVE
  RANGE (ALL USDC)" stay inside their card at any width.
- **The ladder shape chart keeps a readable size on a wide screen.** It scaled with the
  window, so its labels grew to about three times their size; it now stops at 800 px wide.

No worker, protocol, snapshot or database change: v0.9.1 runs on a v0.9.0 database and the
reverse.

## [0.9.0] - 2026-11-21

The ladder track: NockNock now measures, suggests, tracks and maintains a mostly-USDC
ladder of Uniswap v4 ranges below the price, finds the positions you deposit on-chain by
itself, and shows the ladder's shape. Only real calls to action reach the top of the page.
NockNock still builds no transaction and signs nothing.

- **The floor-crossing price (#165)**: how far NOCK can rise before the ranges sell the
  stack below the floor.
- **The ladder simulator (#166)**: a read-only what-if, `GET /v1/simulate/ladder` and the
  `simulate_ladder` MCP tool.
- **Ladders as their own role (#167)**: `role: ladder`, outside the LP target and the
  rebuild; counted in the totals and the reserve.
- **The ladder suggestion and the bottle (#168)**: with `operator_profile.ladder_rungs`, a
  numbered deposit per rung, measured by liquidity, frozen once deposits start.
- **Positions found on-chain**: the wallet's Uniswap v4 position NFTs are found from its
  transfers; one it minted on a suggested rung is tracked by itself, anything else is
  offered with one tap. No hash, NFT id or `config.yaml` edit.
- **Ladder maintenance, call-to-action tiers and the ladder shape chart (#169)**: a
  restraint model for moving a near rung and pulling a deep one, `derived_metrics.events[]`
  in two tiers, and a chart of the deployed NOCK ranges whose area is their capital.

Snapshot spec 2.8.3 -> 2.11.0, every step additive.

Before you upgrade:

- **This release adds a database migration, and from it on a deploy is forward-only.**
  `20261121000013_tracked_positions` adds `tracked_positions`, `position_candidates` and
  `position_detection_state`. A build older than v0.9.0 refuses to start on that database
  (`VersionMissing`), and once a position is tracked from the chain an older build would
  publish a total without it. To go back anyway, first copy each row's `config_entry` from
  `GET /v1/positions/found` into `positions:` in `config.yaml`, then follow DEPLOYMENT.md
  section 5.
- **Every new setting is optional and has no default**: `operator_profile.ladder_bottom_usd`,
  `ladder_rungs` and `ladder_maintenance` (see `config.yaml.example`). Without them the
  ladder cards say which settings they need, and nothing else changes.

### Added

- **Ladder maintenance, with a "keep it in your pants" restraint model (#169).**
  `derived_metrics.ladder_maintenance` (snapshot 2.11.0, additive) states per tracked
  ladder rung what its role's rule says this cycle, gated by
  `operator_profile.ladder_maintenance`: `min_hours_out_of_range`, `cooldown_hours`,
  `hysteresis_pct`, `max_gas_payback_days`, `deep_pull_fill_fraction` and
  `deep_pull_recovery_pct`. None has a default: a rule stays off, and says which
  settings turn it on, until they are all set.
  - **near**: out of range above (all USDC) past every gate, a move back to just below
    the price is stated, keeping its width in ticks and its USDC, with the gas (two
    transactions), the pool's measured in-range fees for the moved range, the payback in
    days in range and the fees earned so far. Once stated it stands until the price is
    back in its range (hysteresis).
  - **deep**: after it converted at least the fill fraction and the price recovered the
    recovery percent above the lowest price seen while it held NOCK, a pull is stated
    ("pull the reserve at the right time"). It is never moved or chased down. Fully
    converted at its bottom it is reported, and no lower ladder is ever suggested.
  - **cap**: out of range above it sold its NOCK by design; stated, never refilled.
  The clocks are carried from the previous snapshot (no table, no migration); a gap past
  the /health window restarts them. Each suggestion states what the operator would sign;
  NockNock builds and signs nothing.
- **The ladder shape (#169; operator, 2026-11-21: "so that we can approximate the bottle
  shape and judge whether it looks bottle-y").** `derived_metrics.ladder_profile` (snapshot
  2.11.0, additive) and a chart in the dashboard's ladder section and on the phone: every
  deployed USDC/NOCK range as a bar on a logarithmic price axis, its height
  capital / ln(high/low) so its area is its capital (a rung's capital is its value as all
  USDC at its top; any other range's is its value now), overlapping ranges stacked into the
  depth profile, the price as a vertical line, the ladder bottom dashed with the reserve
  kept below it noted. Wider ranges are clipped to the ladder zone with where they
  continue; full-range pools are stated, not drawn; a rung of unknown capital is an
  outline. Both MCP transports print it as a short table.
- **Only real calls to action at the top of the page (#169; operator, 2026-11-21: "it is
  swamped by the 'call to action' that is no longer a call to action").** A ladder with
  nothing left to deposit is one quiet line ("Ladder: all 3 rungs deployed") in the ladder
  section, its card behind a collapsed details; positions found on-chain with nothing to
  decide are one quiet line ("3 positions tracked from the chain · checked at block N"),
  the rows and buttons behind a collapsed details. An offer or an incomplete check stays at
  the top. Same on the phone.
- **Compact ladder rows.** Each rung is one summary line (label, rung, a short badge, what
  it holds in plain numbers, its range at 4 significant figures); the rest is behind a
  per-row details. Rows wrap and long text breaks, so nothing leaks out of the card.
- **Calls to action as events, in two tiers (#169).** `derived_metrics.events[]`
  (snapshot 2.11.0, additive) lists every call to action with an id, a tier (top |
  quiet), what changed, its numbers, since, a deep link and what the operator would sign,
  shaped for the v0.10 push. The dashboard shows the top tier under the title row and the
  Lite cockpit above the directive; quiet entries stay where they are. Both MCP
  transports print `• Top of page`, `• Quiet` and `• Ladder maintenance` lines in
  get_shot_advice from one shared `events_text.rs`.

- **Positions found on-chain: deposit in the Uniswap app, and NockNock tracks it
  (#145 for Uniswap v4).** No transaction hash, no NFT id and no `config.yaml` edit. At
  the start of every cycle, before the position reads, the worker reads the wallet's
  Uniswap v4 position NFTs from the chain and decides for each:
  - **Tracked by itself** only when the wallet minted it in a whitelisted pool and its
    pool and ticks are exactly a rung of the current ladder suggestion. It is tracked as
    `role: ladder` with that rung (unset while the suggestion names none, never a guessed
    one) and its mint transaction as the #167 baseline, checked by the same
    `parse_ladder_deposit` as a config ladder. It is counted in the cycle that finds it.
  - **Found, not counted** for anything else the wallet holds with liquidity in a
    whitelisted pool: stated with what it holds, its value at the cycle's prices and why
    it was not tracked by itself, with one tap to track it as a ladder rung, track it as
    LP, or set it aside. It is never tracked silently, and the ladder suggestion waits
    for that tap instead of measuring a second ladder beside it.
  - **Never listed**: a pool with a token outside the whitelist (dust), a hooked pool, an
    empty NFT, an NFT the wallet does not own. They are counted by reason.
  Tracked positions live in the database and are appended after `config.yaml`'s each
  cycle; `config.yaml` wins a collision. A database-tracked position is never a price
  source or a rebuild template. A table that cannot be read fails the cycle rather than
  publish a total without them. An NFT that left the wallet stops being counted and is
  named; a burned one is reported closed by the #144 proof; an emptied one reads as empty
  and no longer stands for a deployed rung, so the suggestion measures again.
  `derived_metrics.position_detection` (snapshot 2.9.0, additive) states whether new
  positions were looked for, how far back the transfers were read, and what is missing
  when the check is partial or could not run. The scan reads the position manager's
  ERC-721 `Transfer` logs: one request where the RPC allows the whole range, resumable
  45,000-block chunks where it does not, and the RPC's transfer index while the logs do
  not cover the history. `balanceOf` on the position manager cross-checks it. After a
  database loss the positions are found again from the chain; one-tap choices are lost
  and offered again. New: `GET /v1/positions/found` and `POST /v1/positions/track`,
  `untrack`, `dismiss` and `restore`, each behind the cross-site write guard and taking a
  JSON body; a "Found on-chain" block under the ladder card on the dashboard and in the
  phone cockpit, with one-tap buttons and no typed input; the same statement in both MCP
  transports (`get_portfolio_state`, and `get_shot_advice` when it bears on the totals).
  The ladder card's after-deposit sentence is now the automatic path. Migration
  `20261121000013` adds three tables; from it on a deploy is forward-only
  (AGENTS.md section 13, DEPLOYMENT.md section 5). NockNock still builds no transaction
  for a ladder and never signs or sends.

- **The floor-crossing price (#165).** `derived_metrics.floor_breach` (snapshot spec
  2.9.0, additive) measures how far NOCK can rise before the deployed ranges sell the
  stack below the floor. Tracked NOCK at a multiple of spot is wallet NOCK, uncollected
  NOCK fees and every NOCK range at that price (concentrated ranges from their own
  liquidity and ticks, full-range pools by x*y=k), so at spot it is the Floor Health
  reading. It reports both the hard floor and the tolerated level (floor less
  `lp_floor_tolerance_bps`), the NOCK left if every range sells out (wallet plus fees),
  the range that sells out first and the one that sells the most, and each range's
  sell-out multiple. WETH/NOCK ranges hold ETH/USD constant, as stated. It is
  unavailable with a reason naming the position when one cannot be evaluated (no
  spot, a zero sqrtPrice, or a range holding NOCK on a frozen pool), never "safe". The
  dashboard shows it under the Floor Health tile and both MCP transports state it in
  `get_portfolio_state`. A measurement, never an instruction. On the live book of
  2026-11-21: the 100,000 floor is crossed at NOCK ≥ $0.025405 (+96.3%), and the
  tolerated 90,000 is never crossed (90,201 NOCK would remain).

- **The ladder simulator (#166).** A read-only what-if for a USDC/NOCK range that is
  mostly USDC and sits below spot: `GET /v1/simulate/ladder`, the `simulate_ladder`
  MCP tool in both transports (the first tool that takes arguments; the stdio
  transport forwards them and `isError`, and its fallback formats the worker's own
  answer) and a Ladder Simulator panel on the dashboard. It works only from the latest
  stored snapshot: no RPC read, nothing signed or sent, no amount chosen. For the
  inputs (USDC, optional NOCK, bounds in USD, in % or `high=spot`) it states the
  liquidity and the NOCK the range needs, NOCK bought and USDC left down a price grid,
  the full-conversion price and average buy price, the top (all USDC), the stable
  balance at the ladder's bottom against the $950 standing reserve (counting the
  existing NOCK ranges' USDC converting on the way down), the reading below the bottom
  and at the top as separate measured lines, and whether its NOCK fits the floor plus
  tolerance budget, with its effect on the floor-crossing price (#165) before and after.
  Bounds snap to a tracked pool's tick spacing or a spacing the caller states; on a
  snapped grid `high=spot` is the usable tick at or below spot (no NOCK), and both
  ticks next to spot are listed with their NOCK need. Snapshot spec 2.9.0, additive:
  `positions[].tick_spacing` for Uniswap v4 and `global_status.lp_nock_budget`,
  `lp_nock_budget_note` and `lp_floor_tolerance_bps`. On the #166 fixture (spot
  $0.012944): $900 from -30% to +2% needs 4,194 NOCK and averages $0.01083; $741 from
  $0.01 to spot needs none, holds 65,130 NOCK at the bottom at an average $0.01138, and
  leaves the stable balance there $25.99 above the reserve.

- **A deployed ladder is tracked as `role: ladder` (#167).** A position entry may set
  `role: ladder`, with an optional `rung: near|deep` and an optional `deposit_tx` (no
  defaults; unset `role` keeps its pre-0.9.0 meaning, and any other value fails every
  cycle as a configuration error, as do a ladder that is the price source, an Aero basic
  pool, a ladder that sets a rebuild key or `deposit_url`, and, after the read, a ladder
  that is not USDC/NOCK or is full range, including a full-range NFT on the pool's usable ticks). A bound at the usable extremes is stated as open-ended, never as a price. A ladder counts in every total, in the floor-crossing price
  and in the standing reserve, whose stable balance now includes its USDC
  (`global_status.reserve_ladder_usdc` names that part, and the reserve, plan-headroom
  and working-capital sentences say "NOCK LP and ladders"). It is not NOCK LP: no LP
  target, no NOCK LP deviation row (a no_target "NOCK ladders (measured)" row is
  appended), no rebuild template or exclusion, no engine or HOLD label. Each ladder gets
  a reading in `data.ladders[]` (snapshot spec 2.9.0, additive): its state by what it
  holds (`holds_only_usdc`, `in_range`, `holds_only_nock`, `empty`, `pool_stale`), its
  bounds against spot, what it holds against its own all-USDC top, and what it bought
  since its deposit only from the `deposit_tx` receipt, checked against the position
  (sent by the wallet, one deposit into that NFT, no swap, amounts on the range's curve);
  otherwise null with the reason (`no_baseline`, `refused`, `unverified`,
  `liquidity_changed` once a ladder is added to or partly withdrawn). Verified net buys of
  a ladder deposited after `cost_basis.opening.as_of` enter the whole-stack average cost,
  and the ladder's own average is on its reading. The ladder simulator (#166) counts a
  tracked ladder as an existing range. The dashboard shows a Ladders card and a LADDER
  tag, the phone cockpit one line per ladder, and both MCP transports the same ladder
  lines. A burned ladder's closed label says it was a ladder. With no ladder configured
  no snapshot byte changes. On the golden fixture (the recorded mint of NFT #3240369,
  1,221.85 USDC + 247.58 NOCK): with the pool fallen into it, it holds 5,551.37 NOCK and
  has net bought 5,303.79 NOCK for $84.35 since its deposit, an average of $0.01590283.

- **Ladders of rungs, the fee yield and the operator's ladder shape (extends #166).**
  Snapshot spec 2.10.0, additive. A one-rung body is unchanged apart from the new
  `fee_estimate` and, only when `operator_profile.ladder_bottom_usd` is set, the gate's
  price and rule.
  - *A ladder of rungs.* `GET /v1/simulate/ladder` takes up to four rungs (`cap`,
    `near`, `deep`; a role is required and never assumed) in one pool, through the
    indexed `r1_`..`r4_` parameters and a `rungs` array in both MCP transports. The
    flat form is unchanged. The rungs share one reserve gate and one NOCK budget, and
    every rung enters the floor-crossing figure. Overlaps are listed. A near rung states
    its round trip: back at the price now it holds what it started with, so its gain is
    its fees. A deep rung states the NOCK it keeps if pulled at fill. Trade-off tables
    (the split, the near rung's depth and the deep rung's top) are measurements in the
    order of the quantity varied, never ranked and with no row marked.
  - *The reserve gate.* It is read at the lower of the deepest rung's bottom and the
    new `operator_profile.ladder_bottom_usd` (no default), so a ladder that stops above
    the bottom cannot recruit the reserve kept for prices below it. With the key unset
    the gate is the deepest bottom, and the body says so.
  - *The fee estimate.* Fees per day while in range are measured from our own tracked
    range, not forecast. `derived_metrics.fee_yield` holds the fees per unit of
    liquidity, measured from stored snapshot rows:
    - one measuring range per USDC/NOCK pool (not full range, not staked, not a
      ladder; the widest when there are several);
    - skipped intervals counted with their seconds;
    - true UTC days;
    - spans on the tick grid;
    - measured at most hourly, carried forward and priced at the current pool price.

    `pool_mechanics[].active_liquidity_raw` holds the active liquidity behind the
    diluted figure. Both are left out of the state hash, so an unchanged cycle still
    stores no row. A rung's estimate is the measuring range's average fee per unit of
    liquidity times the rung's liquidity: an assumption, stated. It is always shown
    with the in-range share of stored history and the replay upper bounds, the
    measurement inside the rung (a per-day figure only from 20 h of intervals) and
    every day's figure. Stored market volume, which belongs to another pool, is never
    read. On the recorded chudmax history (2026-11-14 to 11-21, 1,836 intervals over
    6.33 days) the rate is $6.08 per 1e15 liquidity per day at the current price.
  - *The operator's shape.* `operator_profile.ladder_rungs` (no default) records it
    from the top down. It may start with a `cap`: the tick band containing spot,
    holding the LP NOCK budget and the USDC its curve pairs with it, never more USDC
    than the near rung's. The near and deep shares divide the rest. It is validated and
    stated in `global_status.ladder_rungs`, or named as a configuration error in
    `ladder_rungs_note`. `shape=configured` sizes it to the shared reserve gate's most.
    `derived_metrics.ladder_shape` carries that result in the snapshot, for a card with
    no inputs.
  - *Rung vocabulary.* `cap | near | deep`, for tracked ladders (#167) too.
  - *The shape is planned against what is deployed.* Tracked ladder rungs in the pool
    are part of the shape, not existing ranges. The gated total counts their USDC as if
    it were still in the wallet (a near or deep rung's USDC at its top, a cap's USDC now
    with its NOCK counted back into the budget). Each is matched to a configured rung by
    role, then ticks, and each shape rung states its target, what is deployed, what is
    left and a status (`to_deposit`, `deployed`, `over`, `passed`, `none`). Depositing
    one rung leaves the others' targets unchanged. Only the remainder is minted, on the
    matched rung's own ticks. An unmatched tracked ladder is listed and still counts.
  - *Below a rung, it is passed.* When the price is below a rung's range the rung is
    `passed` ("a USDC-only deposit there is not possible"), the rungs below are still
    drawn from the usable tick at or below spot, and with every rung passed the status
    is `price_at_or_below_bottom`. `shape=configured` never answers a parameter error
    for a valid setting.
  - *Exact mints.* A snapped rung states `mint`: the liquidity its USDC and NOCK buy,
    rounded down with Uniswap's integer formulas, and the amounts a mint of it pulls,
    rounded up, never more than the amounts stated.
  - *Stored rows stay small.* The fee measurement's by-day and band detail and the
    shape's grid and full estimates are kept in the worker's memory and served from the
    latest snapshot; a stored row keeps the summary and says what it leaves out.
    Intervals in which the measuring range was staked are skipped and their seconds
    stated.
  - *Dashboard.* No form is added (operator, 2026-11-20). The typed client and a
    fee-yield helper (a quiet line and the by-day spread) are ready for the card.

- **The ladder suggestion (#168).** A card at the top of the dashboard and under the
  directive on the phone, with no input of any kind, states whether a NOCK ladder fits
  your own rules this cycle. `derived_metrics.ladder_suggestion` (snapshot spec 2.9.0,
  additive) is a structured top-of-page event (id, tier, what changed, the numbers,
  the deep link, what you would sign). It is ready when no ladder is tracked (#167),
  the new setting `operator_profile.ladder_bottom_usd` is set (no default; positive and
  finite, otherwise reported as invalid) and the standing reserve allows at least
  $1.00. The range runs from the usable tick at or below spot (so it holds 0 NOCK) down
  to your bottom, snapped, as a new position in the one tracked USDC/NOCK concentrated
  pool that can hold it (frozen, hooked, dynamic-fee or unread pools are listed with
  the reason; NockNock does not choose between several). The USDC is the most that
  leaves the stable balance at the $950 reserve at the lower of the snapped bottom and
  your setting, capped by wallet USDC, and the card names which limit binds; the plan
  budget, the tax reserve and the cash target are stated lines, not gates. A price at
  or below your bottom is its own status (`price_at_or_below_bottom`). The card sits
  at the top of the dashboard, above the stat tiles, and shows a plain headline, a
  picture on a price axis (spot, each deposit's bar and amount, the bottom line and the
  reserve's zone), the outcomes in plain words (a fall inside the range, a fall to the
  bottom, a rise selling nothing; exact, from the ticks), and numbered deposits with
  their range in USD per NOCK and NOCK per USDC and a link to the Uniswap web app's
  new-position page that prefills the pool and the range (the format is the app's own
  and undocumented, so the card says so once and prints the figures beside it). The
  amount is printed beside the link ("Amount to enter in the app: $… USDC") because
  the app was observed, on 2026-11-21, to drop an amount passed in the link. Each
  deposit carries the pool, ticks and the amounts in raw units for the transaction
  builder that follows. How a minted rung becomes tracked is one isolated field, today
  a one-sentence fallback. Every other status is one line saying which condition
  fails; with no bottom set it lists candidate bottoms as a measurement, none picked.
  It shows only on a fresh snapshot, never changes the directive and never pushes.
  Both MCP transports print it, with the stated lines, in `get_shot_advice`. The v4
  adapter now stamps `positions[].fee_raw` and `hooks` from the poolKeys answer it
  already reads. A non-numeric `ladder_bottom_usd` stops the worker at startup like
  every numeric key. On the live book of 2026-11-21: up to $765.86 USDC and 0 NOCK in
  the USDC/NOCK 3% v4 pool, ticks 273900 to 276300 ($0.01274 down to $0.01002; 78.4750
  to 99.7600 NOCK per USDC), about 67,763 NOCK at an average of $0.01130 if fully
  converted. The Ladder Simulator panel is no longer rendered on the dashboard
  (operator: no forms); the what-if stays in `GET /v1/simulate/ladder` and the
  `simulate_ladder` tool.

- **The ladder card shows your shape: the three-deposit "bottle" (#168 with the rungs
  of #166).** With `operator_profile.ladder_rungs` set, the ladder suggestion is built
  from `derived_metrics.ladder_shape` instead of one rung. With no shape set, the card
  is the single rung above, byte for byte.
  - *One numbered deposit per rung still to deposit,* numbered as in your shape:
    "Deposit 1 of 3 (cap): about 86.79 NOCK + $1.22 USDC, in range now", then the near
    and deep rungs, USDC only. Each states its exact ticks, its bounds in USD per NOCK
    and in NOCK per USDC (from the ticks), the exact liquidity and raw amounts of its
    mint (for the transaction builder that follows) and its own Uniswap link, which
    prefills the pool and the range and never an amount. The amount is printed beside
    the link, cut (never rounded up) to what is typed.
  - *The cap states both amounts and its NOCK limit.* The app works out one side from
    whichever field is typed, so typing the USDC amount can pull more NOCK than the LP
    NOCK budget. The cap's row says "the NOCK side must not exceed 86.7891 NOCK", names
    the NOCK field as the one that keeps the limit, and gives the NOCK amount for it.
  - *The card follows the plan as rungs are deposited.* A tracked ladder no longer hides
    it. A deployed rung is shown as done with what it holds and is not offered again; a
    passed rung states its plain reason; a rung over its target says so. The headline
    counts only what is left ("Ladder: 2 of 3 deposits left, up to $116.08 USDC and
    about 87 NOCK"), and with every rung deployed the card says the ladder is deployed
    (new status `ladder_deployed`). How it keeps up while the price moves is the next
    entry.
  - *What the deposits leave in the wallet,* in the numbers and on the card: "After
    these deposits your wallet holds about $779 USDC; the $950 reserve counts the $224
    of USDC inside your existing NOCK ranges. At $0.01 with every rung converted, the
    stable balance reads $950.02: the wallet's $778.87 plus $171.15 still inside those
    ranges." (the test fixture's figures).
  - *The picture* draws one bar per rung on the price axis: the cap across the spot
    marker, a deployed rung hatched and muted, each label a dollar amount, placed beside
    a bar too narrow for it so it stays readable on a phone.
  - *The outcomes, in plain words:* what a fall to the near rung's bottom buys, what a
    fall to the ladder's bottom buys in total and at what average, and what a rise
    does (it sells the cap's NOCK, about 87, for about $1.13 of USDC, and nothing else).
  - *One muted fee line per rung,* from the shape's fee estimate, never without its
    sample size, the in-range share of stored history and the replay bound; the reason
    when it is not measured. No rung is ranked. The estimates are left out of the state
    hash and of stored rows, like the measurement they come from.
  - Both MCP transports list the same deposits in `get_shot_advice`, and the phone
    cockpit shows the same headline, outcomes and deposits. Nothing on the card takes
    typed input, and it still never changes the directive or sends a push.

- **The ladder card keeps up deposit by deposit while the price moves (review of the
  bottle, 2026-11-21).** The operator follows the card in the Uniswap app over minutes or
  hours; before this, one tick of price after the first deposit could hide every
  remaining deposit behind "no USDC is free", a deployed cap out of its band read "over
  its target" for good, and a rung minted through the link was offered again.
  - *Measured by liquidity.* Each rung's target is a liquidity on its own ticks, and a
    liquidity does not change with the price. A tracked rung is deployed when it holds
    at least its target's liquidity, wherever the price is; a tracked cap is the cap.
    What is left is the remaining liquidity on the same ticks, at the price now.
  - *Frozen by the first deposit.* The first near or deep rung deposited fixes the plan
    (the earliest by deposit block when the blocks are known, otherwise the one implying
    the most); the total is no longer re-divided each cycle. The plan is worked out from
    the tracked rungs, not stored, so the database stays disposable.
  - *Never all or nothing.* Once the plan is frozen the cap takes what the reserve
    leaves beside the near and deep rungs, and says when it holds less than the budget.
    When they alone need more than the reserve allows, each deposit left is cut pro
    rata and the shortfall is stated in dollars (`fit`). `no_headroom` now means nothing
    above the $1.00 minimum fits, and its headline says so ("Ladder: 1 of 3 deposits
    left, but your reserve leaves under $1.00 for them").
  - *Nothing under a dollar.* A remainder under the $1.00 minimum is never offered, so
    a harvest or a cent of rounding never brings a deployed ladder back to the top of
    the page.
  - *Found on-chain.* A position found on-chain and not yet tracked on exactly a rung's
    pool and ticks counts for that rung and reads "found on-chain, being tracked"; one
    another address minted, or worth less than the minimum, never does. One that
    matches no rung is named on the card and not counted until one tap. Position
    detection already matches a new mint against the shape's cap, near and deep rungs
    and tracks it with that rung's role.
  - *The cap's text* states both limits, which one binds now and the field that keeps
    it: the USDC field, with "the cap holds less than the budget", when its USDC limit
    binds. *Passed rungs* give their own reason (the price at or below the rung's
    bottom, or the cap's band covering what is left of it). *Fee lines* state the
    diluted figure beside the undiluted one. A remainder that needs NOCK the budget
    cannot give is held back with the figures that differ. The headline, outcomes and
    wallet line add the amounts the card says to enter, cut to cents.
  - *Second review:* only a position the wallet itself minted (its receipt read) counts
    for a rung; a stranger's position minted or transferred to the wallet never freezes
    the plan, never reads "found on-chain, being tracked" and never holds the
    single-rung card back. A withdrawal reads "partly withdrawn" or "withdrawn", never
    "done", is never offered again and never shrinks the plan (each rung stands for what
    its verified mint deposited). With near and deep in and no room for the cap, the
    card reads "Ladder deployed: 2 of 3 deposits are done, and the reserve leaves no
    room for the cap". A near rung minted a band ago (its top follows the price) is
    matched by its bottom tick, on the card and by detection. Detection re-reads at most
    40 untracked NFTs a cycle, in turn, inside a 40-second budget, so dust cannot time it
    out. After a restart both MCP transports say the stored card is not available until
    the first cycle.
  - *Third review:* a rung burned or sent away keeps standing for its deposit:
    `tracked_positions` remembers each ladder rung's mint liquidity and the most a cycle
    read it holding (migration `20261121000013` amended in place: it has never been
    applied), and a rung whose deposit is not known never freezes the plan. Once a rung
    is pulled the cap is not offered. The re-read rotation resumes where a time cut
    stopped it (`position_detection_state.refresh_cursor`), and of two mints of one rung
    in a cycle only the exact one is tracked.
  - `operator_profile.ladder_rungs` set without `operator_profile.ladder_bottom_usd` is
    now `needs_setting` (OPERATOR_GUIDANCE section 7, `config.yaml.example`).
  - The stored snapshot row keeps the card's rungs, ticks, pools and mints and leaves out
    its links, texts, limits, picture, gates and outcomes (`detail_note`; a stored ready
    card reads shown false with the reason), so the card no longer adds about 18.7 KB to
    every row. `derived_metrics.ladder_shape` gains `fit`, `headroom_usdc`,
    `rest_rule` and, per rung, `target_liquidity`, `tracked_liquidity`,
    `remaining_liquidity`, `found` and `note`; statuses `found`, `below_minimum`, `held`
    and `no_room` (snapshot spec 2.10.0). `shape=configured` no longer answers
    `SHAPE_NO_ROOM`.

### Changed

- **Call-to-action audit (#169).** Top of the page: DEFENSIVE LOCK, DCA TRANCHE DUE and
  DCA PLAN INVALID, a plan buy to confirm, a position found on-chain, the shown ladder
  suggestion, the ladder's maintenance, and wallet USDC below the standing reserve. Quiet:
  every other directive, collectable LP fees (still pushed at the $5.00 BATCH HARVEST
  rule), AERO rewards, closed positions, the floor-crossing price and a cap that sold its
  NOCK. The pushes are unchanged.
- **Wallet USDC below the reserve is at the top (operator's choice, 2026-11-21: "Top when
  wallet < $950").** No exceptions. The event states the wallet / NOCK LP / ladder split of
  the stable balance the gate counts, and one measured line on the gas-side ETH/USDC ranges
  (still outside the gate, #151 and #162): their USDC and WETH now, wallet USDC if they
  were withdrawn now, and the ETH price at which their USDC side alone would cover the
  wallet's shortfall, with the WETH still in them there (operator, 2026-11-21: "we can plan
  to pull the ETH LPs some time after it would cover the $950 usdc reserve").
- **Ladder maintenance, after review (#169).** A deep pull whose
  `deep_pull_recovery_pct` is at or above the rung's width can never fire (the rung sells
  all its NOCK back first): that is now a top-of-page statement naming the setting and the
  width, and no number is picked. A stated pull says how much of the NOCK the rung held at
  most it has sold back since the low. A near move from a pool tick on a spacing multiple
  opens out of range. A standing near move survives a pool_stale cycle, unknown reads and a
  dip to within one spacing above the top; only a real return into the range clears it. A
  completed near move (minted by the wallet with USDC only on the moved ticks) is tracked as
  the near rung by itself, and the move states the stretch to the deep rung's top that no
  rung would cover. Every event's deep link lands on an element of the page.

## [0.8.3] - 2026-11-20

A patch release: the protected cash reserve is a standing rule, and dry powder honours it.

- **The $950 reserve no longer lapses with the plan (#162).** It is now
  `operator_profile.cash_reserve_usdc`, and dry powder is never more than the headroom
  above it. Snapshot spec 2.8.3.
- **The tick bracket chart draws the ETH/USDC ranges (#160).** This is a display-only
  feature in a patch release, an exception the operator chose (2026-11-20): it reads
  numbers the snapshot already carried, with no worker, protocol or spec change.

Before you upgrade:

- **To keep a standing reserve, add it under `operator_profile`**, for example
  `cash_reserve_usdc: 950` (see `config.yaml.example`). A plan's own `cash_reserve_usdc`
  still works, but it ends with the plan. Without the setting, dry powder is computed as
  before.
- **No database migration.** v0.8.3 runs on a v0.8.2 database and the reverse.

### Fixed

- **The protected cash reserve no longer lapses with the plan, and dry powder honours it
  (#162).** The operator's $950 reserve was configured as `accumulation_plan.cash_reserve_usdc`,
  which only the plan's buy guard read. Once plan `2026-11-reentry` completed, nothing
  protected it, and the advice reported "Liquid dry powder: $1360.77" (wallet USDC less
  the cash target) without naming it. The reserve is now a standing setting,
  `operator_profile.cash_reserve_usdc`, measured as before on wallet USDC plus USDC
  inside NOCK LP (gas-side ETH/USDC ranges are left out, #151).
  - Dry powder never exceeds the headroom above it. The advice names the reserve and the
    headroom ("$818.18 above it"), and says when the stable balance is below it.
  - A plan never draws below either reserve, the plan's own or the standing one. The
    plan panel names the one that binds.
  - The snapshot (spec 2.8.3) adds `cash_reserve_usdc`, `reserve_stable_usdc`,
    `reserve_headroom_usdc` and `dry_powder_usdc` to `global_status`. The Total USDC
    Stack tile and `get_portfolio_state` (both MCP transports) state the reserve.
  - Moving USDC into a NOCK range does not use the reserve up; buying NOCK does.
  - Unset means no rule. A reserve that is not zero or positive is named, and no dry
    powder is reported until it is fixed.

### Added

- **The tick bracket chart draws the ETH/USDC ranges (#160).** The gas-side WETH/USDC
  ranges, left out of the NOCK chart since #152, get their own chart in the same panel,
  in USD per ETH. Each range is a bar from its lower to its upper edge, green in range
  and amber out of range. The ETH spot is a line, and each range has a caption line
  with its span and its distance to the nearer edge. A WETH/USDC bracket (WETH per USDC)
  is inverted; a full-range one is named, not drawn. They stay measured, never steered:
  no target or directive changes. Frontend only: no worker or snapshot change.

## [0.8.2] - 2026-11-18

A patch release: the harvest ledger books only fees.

- **Returned liquidity is left out of harvest income (#156).** A withdrawal's principal is
  separated from its fees, or the transaction is refused with nothing recorded.
- **A swap is refused as a harvest (#157).** A claim only pays the wallet.
- **The cockpit classifies a record-harvest failure by its HTTP status.**

Before you upgrade:

- **Nothing new is required in `config.yaml`.**
- **No database migration.** v0.8.2 runs on a v0.8.1 database and the reverse.
- **`record-harvest` responses gain `principal_excluded`,** and it may now refuse with
  `PRINCIPAL_NOT_SEPARABLE`, `PRINCIPAL_EXCEEDS_INFLOW`, `PRINCIPAL_ONLY` or `NOT_A_CLAIM`
  (422), or `POSITION_UNREADABLE` (503).

### Fixed

- **Returned liquidity is no longer booked as fee income (#156).** `POST /v1/execute/record-harvest`
  booked everything a transaction paid the wallet. A "remove liquidity" transaction
  (`decreaseLiquidity` + `collect` in one multicall, as the Uniswap and Aero apps send it)
  pays back the position's principal with its fees, so the principal would have been
  booked as harvest income, which overstates income, the tax reserve and the Koinly
  export. Now each `DecreaseLiquidity` from the Uniswap v3 NPM or the Aero CL position
  manager is read as principal: the position's tokens come from `positions(tokenId)` at
  the block before the transaction (the NFT is often burned in it), and the amounts are
  taken out of what the wallet received. Only the fees are booked, and the response
  lists the principal it left out in `principal_excluded`. Principal that cannot be read
  from the receipt is refused with nothing recorded:
  - a Uniswap v4 liquidity removal (`PRINCIPAL_NOT_SEPARABLE`);
  - an Aero basic pool `Burn` (`PRINCIPAL_NOT_SEPARABLE`);
  - a concentrated pool `Burn` for another owner (`PRINCIPAL_NOT_SEPARABLE`);
  - a decrease not collected in the same transaction, or collected to anyone but the
    wallet, the position manager or the contract the wallet called (`PRINCIPAL_NOT_SEPARABLE`);
  - a collect from a position that held no liquidity just before the transaction, whose
    payout is principal from an earlier decrease (`PRINCIPAL_NOT_SEPARABLE`);
  - principal larger than what the wallet received (`PRINCIPAL_EXCEEDS_INFLOW`), including
    principal in a token NockNock does not book, which a zap may have swapped into one it does.

  A transaction that returns only principal records nothing (`PRINCIPAL_ONLY`).
  - The fixture is the real withdrawal of v3 WETH/NOCK #6336662
    (`0x06515a23…b778`). Of its 0.103325 ETH and 1,006.81 NOCK, the fees were
    0.000999503 WETH and 101.58 NOCK.
  - Known limit: part of a position's liquidity decreased in one transaction and
    collected in a later one, while the position still holds liquidity, is booked in
    full by the later one.
  - Nothing had been misbooked: that withdrawal was held back until this fix.
- **A swap is no longer booked as fee income (#157).** `record-harvest` also booked a
  purchase: the operator's $20 NOCK buy (`0x9092eede…0315`) recorded against a scratch
  worker became a $20.21 "harvest" of 1,247.58 NOCK. A fee or reward claim only pays the
  wallet, so a transaction in which the wallet pays out an ERC-20, or which sends ETH, is
  refused with `NOT_A_CLAIM` and nothing is recorded. A position NFT leaving the wallet
  (burned on withdrawal, or staked) is not a payment and does not count. A NOCK buy is
  recorded with `record-accumulation`, as before.
- **The cockpit no longer reads "404" in a refusal as "awaiting broadcast" (#156).** A
  record-harvest failure is classified by its HTTP status. Refusal details now carry
  addresses and raw amounts, and the digits "404" inside one would have retried a final
  refusal for ten minutes.

## [0.8.1] - 2026-11-18

A patch release: no new feature, every change makes a number or a surface truer.

- **Collectible gauge rewards (#142).** A staked position's unclaimed AERO is reported as what
  it earns, not as $0 of fees. Snapshot spec 2.7.0.
- **A withdrawn position is reported as closed (#144),** proven on-chain, and no longer
  stalls every ingestion cycle. Snapshot spec 2.8.0.
- **The v0.8.1 burn-down (#130-#137):** one ingestion cycle at a time, LAN API hardening,
  "unknown" instead of $0.00 for an unread ledger, and five smaller fixes. Snapshot spec 2.8.1.
- **Positions with no NOCK leg (#151-#153).** Gas-side ranges such as WETH/USDC are measured,
  never steered: they count in the totals but not as NOCK LP, and they are not drawn on
  the NOCK price chart. No claim is recommended below its gas. Snapshot spec 2.8.2.
- **CI hardened for outside contributions (#146)** and **`make dist` replaces binaries
  atomically (#140),** the project's first outside contribution (#141, from Görkem Taha).
- **Lore:** "The gas pump" and "Two trips and a wait" (docs/lore, 2026-11-17).

Before you upgrade:

- **Nothing new is required in `config.yaml`.** An existing config loads unchanged. A
  position with no NOCK leg can now be tracked like any other (see `config.yaml.example`).
- **No database migration.** v0.8.1 runs on a v0.8.0 database and the reverse, so going
  back to v0.8.0 needs no repair.
- **Expect the directive to move** when you first track a position with no NOCK leg. Its
  value raises the portfolio total, and with it the NOCK LP target.

### Fixed — `make dist` replaces packaged executables atomically (#140)
Contributed in #141. Each executable is staged in `dist/` and renamed into place, so
macOS no longer kills a packaged binary (such as the MCP server a client is running)
that was overwritten in place. Temporary files are removed if installation fails.

### Docs — lore
`docs/lore/2026-11-17-the-gas-pump.md` and `docs/lore/2026-11-17-two-trips-and-a-wait.md`.

### Fixed — v0.8.1 burn-down (#130-#137)
Eight minor findings of the 2026-11-16 adversarial review, each with a test that fails
without its fix. Snapshot spec 2.8.1: additive (`accumulation_plan.progress_unknown`) and
one widening (`accumulation_plan.venue` nullable). No new config key is required.

- **#134 One ingestion cycle at a time.** The scheduled loop and `POST /v1/ingest` share
  one lock held for the whole cycle, so a stale cycle can no longer push a directive that
  a newer one already replaced, or push a transition twice. The endpoint runs its cycle
  in a task of its own: a client that disconnects (or nginx's 60 s timeout) no longer
  drops a cycle between the store and its notification. Requests that arrive while a
  cycle runs wait for it and then share one new cycle. A request joined to a cycle that
  never finished is told so, never an older cycle's outcome.
- **#133 LAN API hardening.** A POST is refused with 403 `CROSS_SITE_REFUSED` when its
  `Sec-Fetch-Site` is `cross-site`, or its `Origin` host is not the request's `Host` host
  (ports are not compared, because nginx forwards the host without the port: the
  dashboard on `:3000` and behind the `:3443` TLS proxy keeps working). Requests without
  `Origin` (curl, scripts, MCP clients) are unaffected. `POST
  /v1/accumulation/quotes/refresh` reads the chain at most once every 10 seconds; inside
  the window it answers with the quotes held (200), or 429 `REFRESH_COOLDOWN` with
  `Retry-After` when it holds none. Stored `accumulation_quotes` rows that no buy can use
  any more are deleted after each stored quote, and rows read since the latest recorded
  buy are kept for 24 hours (plus the newest one and the one before each swap on offer or
  dismissed), so a tranche due for days no longer grows the table without bound (this
  also covers the table growth noted in #136). The Vite dev proxy passes the browser's
  `Host` through. No authentication was added, and the guard does not stop DNS
  rebinding or a page served on another port of the same host (that needs a Host
  allow-list).
- **#130 An unread ledger is unknown progress.** A cycle that cannot read the plan
  ledger reads `DCA BLOCKED (ledger unreadable)` with no "$0.00 of $X" sentence and no
  "(1 of N)"; the plan status carries `progress_unknown: true` and its amounts read
  "unknown"; the dashboard panel, the Lite view and both MCP transports say unknown; the
  LP rebuild keeps the whole budget liquid meanwhile. That cycle pushes no change of the
  plan's own part of the directive (a floor lock or another change outside the plan is
  still pushed), and the next good cycle is compared with what the operator was last
  told, so a plan completed just before the outage is announced once. The raw JSON keeps
  `fills`, `tranche_index` and `tranches_total` numeric; with `progress_unknown: true`
  they are not progress.
- **#131 Range bounds keep their digits.** Sub-cent `lower_price`/`upper_price` carry six
  significant digits (`0.00000564468`, was `0.00000564`): the Aero CL card reads 154,015 -
  177,158 NOCK per WETH, as the ticks and the Aero app do (was 154,083 - 177,305), and the
  USD bracket matches the ticks.
- **#135 The swap-on-offer notice leads every directive.** Under REBUILD LP TRANCHES and a
  floor lock too, it now comes first and once. The read-only route-quote pointer sits
  right after the due-tranche sentence, before the progress sentence and any LP text, so
  "this amount" is the tranche. Which directive is chosen is unchanged.
- **#132 No default venue.** `accumulation_plan.venue` is optional with no default
  (it was "Aero"). Unset, the advice reads "Buy $X of NOCK (spot $Y)." with no venue, the
  panel and MCP show no venue label and recorded buys store none (`venue: null` in
  `GET /v1/accumulation/events`). Set, it reads as the operator's label ("your plan's
  venue label: Aero"). Configs that set it load unchanged.
- **#136 Lighter history rows.** `/v1/snapshot/historical` rows carry no
  `accumulation_plan.route_quotes` and empty `fill_detection.offers`/`dismissed`
  (detection's status and check times stay); the latest snapshot keeps them in full.
- **#137 Average-cost chart gaps.** Consecutive backfilled candles one candle period apart
  are no longer gaps: a run of hourly candles is one lighter region with one caption
  ("hourly third-party candles"), only real gaps break the line and count in the footer,
  and gap labels are thinned so none overprints another. Each candle is measured against
  its own period, so daily and hourly candles in one range are each read at their own
  cadence, and a worker read more than one bucket rule beyond the last candle's own
  period is a gap.
- **CONFIRM and RECORD BUY wait at most 8 seconds for the refresh** they ask for. Since
  one cycle runs at a time (#134) a requested cycle can wait for the one in flight; the
  recording has already succeeded by then, and the next refresh shows it.

### Fixed — positions with no NOCK leg (#151-#153)
The operator opened two gas-side WETH/USDC ranges (Uniswap v3 #6607225 and Aero CL
#3449558, staked). Until now every tracked position had a NOCK leg, and the NOCK-strategy
figures counted every position. A range with no NOCK leg is now measured, never steered:
valuation and the totals keep counting it, because it is held. One predicate decides,
`is_nock_pair` in the worker and `hasNockLeg` in the frontend (token0 or token1 is NOCK).
Snapshot spec 2.8.2: additive (`TickBracket.position_id`). No config key changes.

- **#151 Not NOCK LP.** The LP rebuild judges NOCK-pair value only: gas-side LP no longer
  fills the NOCK LP target, silences the rebuild bias or shrinks its suggestions, and a
  WETH/USDC range is neither a rebuild template nor an exclusion. The deviation matrix
  row the LP target is measured against is "NOCK LP (NOCK-pair positions)" (was "LP (all
  tracked positions)"); a no-target "ETH/USDC LP (measured)" row is appended, like Gas
  and AERO, only when such ranges hold value. The engine label and LP summary count NOCK
  ranges ("+2 ETH/USDC ranges, measured only (2 in range)" is said after them); "No LP
  Tracked", "LP Empty" and "No Ranged LP Confirmed In Range" read "No NOCK LP Tracked",
  "NOCK LP Empty" and "No Ranged NOCK LP Confirmed In Range". One rule names those ranges
  on every surface (summary, matrix row, chart caption): "ETH/USDC" when every one is an
  ETH-or-WETH / USDC pair, otherwise "non-NOCK" (row "Non-NOCK LP (measured)"). The NOCK
  stack chart no longer draws a flat-zero "(no longer tracked)" band for a retired id
  that never held NOCK in history. The plan's
  protected cash reserve and the working-capital note count wallet USDC plus USDC inside
  NOCK-pair ranges only; `total_stable_usd` (the USDC pillar) still counts every USDC
  held. `positions_nock` and the NOCK stack chart list NOCK-pair positions only. Goal 1
  counts NOCK-leg positions. The bias text reads "NOCK LP holds"; both MCP transports say
  "NOCK LP rebuild bias", state an over_target overage and never "$0.00 short". The ETH
  pillar is labelled "ETH (gas + LP WETH)", which it always was; the gas line stays
  wallet ETH + WETH.
- **#152 The NOCK bracket chart draws NOCK ranges.** Each tick bracket names its position
  (`position_id`); the dashboard pairs them by id (by index for older snapshots), leaves
  out ranges with no NOCK leg (their brackets are WETH per USDC, which drew as bands at
  about $0.61-$1.22 "per NOCK") and says how many are not drawn. The worker still sends
  one bracket per position, in order, so older dashboards keep working.
- **#153 No claim below the gas cost.** With gas known, fees worth less than one claim's
  gas read `accumulate` (was `claim_recommended`), from the gas cost up to twice it
  `claim_recommended`, twice or more `claim_now`. With gas unknown a positive amount
  still reads `claim_recommended`. BATCH HARVEST now counts only the fees of positions
  whose own row says `claim_now` or `claim_recommended`: many positions each below their
  claim's gas no longer add up to a harvest. Its gas efficiency is those fees over the gas
  of every claim they take ("3.8x gas efficiency over 3 claims", one decimal below 10x so it is never rounded up; was the total over one
  claim's gas), and the text names the uncollected total when some fees are left out.

### Security — CI hardened for outside contributions (#146)
- Every action in `ci.yml` is pinned to a full commit SHA; the workflow declares read-only
  permissions at the top; the release job takes the tag name from the environment and
  refuses anything that is not a plain version tag.
- Repository settings, changed by the operator: every pull request from an outside
  contributor waits for approval before CI runs, every time; workflows may no longer
  approve pull request reviews.
- The audit, the decisions and the rules for reviewing an outside pull request are in
  `docs/kb/ci-security-audit-2026-11-17.md` and AGENTS.md section 14.

### Fixed — a withdrawn position reports as closed instead of stalling ingestion (#144)
On 2026-11-17 the operator withdrew the Aero CL position in the Aero app (NFT #2914490,
burned in Base block 58637717). From the next cycle every ingestion cycle failed, and
the dashboard, the MCP tools and the notifier went stale until the config entry was
commented out by hand. An ordinary action in the wallet must not stop all reporting.

- **Closed, on proof only.** A configured position whose NFT does not exist is reported
  as closed and the cycle goes on. It is proof when two reads of the same cycle, made at
  one current block, both give the position manager's own answer for a token id that
  does not exist, decoded strictly from the raw revert data, on a position manager whose
  answers were read on-chain (`ONCHAIN_INTEGRATION.md` §13):
  - `aerodrome-cl` (`0x4857db57…c60b`): `positions()` reverts `Error("ID")` and
    `ownerOf()` reverts `Error("ERC721: owner query for nonexistent token")`;
  - `uniswap-v3` (`0x03a520b3…34f1`): `positions()` reverts `Error("Invalid token ID")`
    and `ownerOf()` reverts the same ERC721 reason;
  - `uniswap-v4`: `ownerOf()` reverts `Error("NOT_MINTED")` and `getPoolAndPositionInfo()`
    returns six zero words. The v4 PositionManager answers a liquidity of 0 for a burned
    id, exactly as for an NFT that exists and is empty, so until now **a burned v4 NFT
    read as a live position holding nothing**. Zero liquidity is now followed by an
    existence check.
  Anything else fails the cycle exactly as before: a transport failure, an HTTP error, a
  rate limit, a JSON-RPC error that is not an execution revert, a revert with another
  reason, with empty data or with none, a position manager that is not on the list, and
  one read saying "nonexistent" while the other does not. Wrongly calling a live
  position closed would silently take real money out of the total.
- **The proof is read at one current block.** A token id that was never minted answers
  exactly like a burned one, so a node whose state is older than an NFT's mint would
  "prove" a live position closed. When the first read at `latest` says "nonexistent",
  the worker reads the node's latest block header and makes both proving reads at that
  block number. The block's time must be within 120 seconds of the worker's clock (or
  within `sync_window_seconds`, if shorter). A node that is further behind, a header
  that cannot be read, and a proving read answered by a backend that does not hold that
  block all fail the cycle. A position that an earlier cycle read as live can therefore
  not be reported closed by a lagging node. The check relies on the host's clock.
- **A mistyped `nft_id` reads the same.** A token id that was never minted answers
  exactly like a burned one, so every text says "withdrawn and burned, or a mistyped
  nft_id". An NFT that exists with no liquidity is not closed: it stays a live position
  holding nothing. An Aero basic pool (`aerodrome`) has no NFT and is never closed; its
  gauge reward is read as before.
- **Changed for a Uniswap v4 entry with no `nft_id`.** It reads as token id 0, which
  does not exist, so it is closed like any other missing NFT. Before, such an entry read
  as a live position holding nothing and could be the price source. Beside live
  positions it still supplies the price (as a closed price-source entry, below). As the
  ONLY configured entry it used to complete a cycle and now fails it with "No live
  positions: every configured position is closed". Give a v4 entry the id of an NFT
  that exists; to watch a price without a position, use an `aerodrome` entry.
- **Listed apart, counted nowhere.** New `data.closed_positions[]` (config id, protocol,
  NFT id, pool, position manager, a label, the reason, the two reads with their raw
  answers, and when this worker first recorded it as closed). A closed position is never
  in `positions[]`, `pool_mechanics[]`, `contract_fees[]`, the velocity rows or the tick
  brackets, which stay index-aligned, and nothing of it enters a total, a pillar, an LP
  weight, a rebuild suggestion, a claim action or a reward figure: with the withdrawn
  entry still configured, every other byte of the snapshot is what it is without the
  entry. `global_status` gains `closed_positions_count` and `closed_positions_note`. All
  three are absent when nothing is closed, so such a snapshot keeps its bytes and its
  state hash. Snapshot spec 2.8.0.
- **The price source.** If the closed entry is flagged `is_price_source`, no price is
  guessed. A Uniswap v4 entry's pool goes on supplying the NOCK/USD spot through the
  same two reads as before (`poolKeys` and StateView `getSlot0`, neither of which needs
  the NFT; the stale-pool guard still runs), and every surface says to flag another
  position before removing the entry. No text says that this entry "can be removed":
  not the notification, and not `closed_positions_note`, which names only the other
  closed entries as removable. A Uniswap v3 or Aero CL entry fails the cycle with
  a configuration error that names the fix, because its pool was identified through the
  NFT. A live flagged position always comes first.
- **All positions closed.** The cycle fails, as it does with no positions configured
  (#119 is not changed here).
- **One notification** when a position that the previous snapshot held as live is closed
  in this one. Not on later cycles, not after a restart, and not for an entry that was
  never a tracked position. The directive is not changed by it.
- **Surfaces.** A "Closed positions" note under the tracked positions on the dashboard,
  a card in the Lite cockpit, and a block in all four MCP tools (`get_portfolio_state`,
  `get_shot_advice`, `get_accumulation_plan`, `simulate_claim_fees`) on both transports
  (one shared text source): the label, that the NFT no longer exists, "not counted in
  any total", and that its entry can be removed from `config.yaml`. The directive and
  the rebuild lines themselves are unchanged; without the block each text is what it is
  with no closed position. No amount and no value is printed for a closed position.
  The claim payload builds no action for one. A snapshot without the new fields renders
  exactly as before.
- **`/health`**: a cycle with a closed position is a successful cycle (`ok`,
  `last_error` null).
- **RPC layer.** `eth_call_outcome` keeps the revert data of a reverting call, and
  `decode_revert_reason` decodes `Error(string)`, `Panic(uint256)`, a custom error's
  selector and an empty revert. A declared string length larger than the data is
  refused before any arithmetic, so no revert payload can panic the ingestion task.
  `latest_block_header` reads the block for a proof. Error text still never contains
  the RPC URL.
- **No new config key.** Existing configs load unchanged. Nothing was looked up about
  the burn itself (transaction or time): the worker proves that the id does not exist
  now.
- **Not verified:** the response shape of the deployment's RPC provider for a reverting
  call (the record was read through two public nodes, which agree byte for byte); if a
  provider sends no revert data the cycle fails as before, with a message that says so.

### Changed — gauge rewards reported as collectible earnings (#142)
A staked Aero position's earnings are its gauge reward, and NockNock reported them as if
the position earned nothing collectible: a trailing note after "not LP-claimable", a $0
bar in the velocity chart, and totals that left the AERO out. Reporting only: nothing
here signs, sizes or advises.

- **Each reward is stated as collectible earnings.** `positions[].collectible_reward`
  carries the token ("AERO"), the amount, its USD value at the cycle's AERO price, the
  gauge it is collected from and how ("the Aero app", `https://app.aero.xyz`), with
  `status` `read` or `not_read`. It is the same read as `unclaimed_aero`,
  `unclaimed_aero_usd` and `unclaimed_aero_note`, which keep their names and values.
- **Totals side by side, never merged.** New `global_status` fields
  `collectible_rewards_usd`, `collectible_rewards_note` and `total_collectible_usd`.
  `total_fees_claimable_usd` keeps meaning LP fees only; the total is the printed LP-fee
  figure plus the printed rewards figure, so the three add up to the cent. The rewards
  figure is the tracked gauges' summed amount at the cycle's AERO price, the same figure
  as `unclaimed_aero_usd`, so one amount of AERO never prints with two dollar values; it
  is not the sum of the rounded per-position `usd` values and can differ from that sum
  by a cent per position. When a gauge was not read, or a non-zero reward is unpriced,
  both new figures are null and the note says which: never a partial sum, never 0.
- **Fee and velocity rows.** `contract_fees[]` and `velocity_gas_sandbox.contracts[]`
  gain `earnings_kind` (`gauge_reward` when the reward is what the position earns),
  `reward` (the same object) and `lp_fees_note` (the reason alone). `recommended_action`
  stays `not_claimable` and `claimable_usd` stays `0.00` for such a row: those are about
  LP fees. A row with a fee action (claim_now, claim_recommended, accumulate) carries no
  reward, so no harvest word ever labels AERO. `contract_fees[].amount` and the velocity
  `note` are unchanged.
- **What does not change.** Unclaimed AERO stays out of `total_portfolio_usd`. BATCH
  HARVEST, the notifier and every threshold keep reading LP fees only; whether rewards
  should count toward the harvest rule is the operator's to set. The directive is the
  same for any reward size (tested from 0 to 1,000,000 AERO, on both sides of the $5.00
  threshold). No text advises claiming, selling, holding or accumulating AERO.
- **MCP, both transports, one source file** (`reward_text.rs`). `get_shot_advice` keeps
  "Uncollected Claimable Fees" (LP fees) and adds "Collectible AERO rewards" and "Total
  collectible (LP fees + AERO rewards)". `get_portfolio_state` adds a "Collectible
  earnings" line and says on each position's AERO row which gauge it is collected from.
  `simulate_claim_fees` prints "Collectible LP fees" (was "Total Gross Claimable"),
  "Collectible AERO rewards" and "Total collectible", and a staked position's row now
  reads "reward 17.654015 AERO ($7.75), collectible from gauge 0x… in the Aero app | …;
  LP fees not LP-claimable (…)" where it read "not LP-claimable (…); unclaimed AERO …".
- **A read zero is "nothing to collect".** A gauge that answers 0 (the operator's CL
  gauge after the claim of 2026-11-17) reads "0.000000 AERO ($0.00), nothing to collect
  in gauge 0x…" on every surface, never as an amount "collectible … in the Aero app",
  and its row carries no gas statement. When every gauge answers 0 the rewards line
  reads "$0.00 USD (0.000000 AERO in tracked gauges: nothing to collect)".
- **Gas for collecting a reward, only where the claim call is proven.** On the worker's
  HTTP MCP endpoint and in the claim payload, a reward's row carries a read-only
  `eth_estimateGas` from the wallet for the gauge's claim call, bounded to 1.5 seconds.
  It is made only for the two gauges whose call was proven on-chain to pay what `earned`
  reports (ONCHAIN_INTEGRATION.md §9): the CL gauge by the tracked wallet's own claim of
  2026-11-17 (tx `0x2d2b609b…a16e`, 17.654015228100787903 AERO, exactly `earned` one
  block before), the basic gauge by another account's real claim and a simulation for
  the wallet. Any other gauge reads "gas not estimated (claim call not verified for this
  gauge)", a zero reward's payload row "nothing to collect, gas not estimated", and text
  built from a stored snapshot alone (the stdio binary's fallback, the golden files)
  says no live estimate was made. The USD figure is L2 execution gas at the live gas
  price. The claim payload prices its reward rows at the `eth_gasPrice` it already read
  for its LP actions: one read and one price per payload.
- **No transaction is built for a reward.** `GET /v1/execute/claim-fees/payload` gains
  `collectible_lp_fees_usd`, `collectible_rewards_usd`, `collectible_rewards_note`,
  `total_collectible_usd` and a `rewards` array whose rows have no `to` and no `data`.
  A `not_claimable` action's `simulation_detail` now ends with the position's reward and
  where it is collected. No action claims AERO, and the Lite cockpit sends none.
- **Dashboard.** The Claimable Yield tile shows the total with LP fees and AERO rewards
  as two labelled figures (it keeps the LP-fee net of gas in its subtitle). A wallet
  with no position that can earn a reward (Uniswap only, or from zero) keeps the tile it
  had: "Claimable Yield (Net)" with the LP-fee net of gas, and "AERO rewards none" in
  the subtitle. When the rewards are not totalled, the tile and the Lite cockpit say so
  in short ("not totalled (1 gauge reward not read)"); the full reason, with its gauge
  address, stays on that position's own line, and both wrap inside their card. A staked
  position's card shows "Collectible earnings: … AERO ($…), collectible from gauge … in
  the Aero app" with the LP-fee reason beneath. The velocity chart draws rewards as
  their own series and legend entry, in the AERO colour, beside LP fees; its text rows
  read "…: reward collectible in gauge 0x3bf7…60fd: 17.654015 AERO ($7.75); LP fees not
  LP-claimable (…)". The Lite cockpit's uncollected-fees card lists the reward lines and
  the split totals. None of it has a button.
- **Older snapshots.** Every new field is optional or nullable. A snapshot that states
  no collectible rewards (one from before 2.7.0, which the worker serves from its
  database until its first cycle after the upgrade) renders exactly as before on the
  dashboard, the Lite cockpit and all three MCP texts: no reward line is added, so
  nothing reads "not reported" beside the unclaimed AERO that snapshot already shows.
  The one difference is the claim text's renamed "Collectible LP fees" label.
- **Snapshot spec 2.7.0.** `SNAPSHOT_SPEC_VERSION`, `specs/core/snapshot.yaml` and the
  frontend contract test move from 2.6.0 to 2.7.0; `specs/core/api.yaml` documents the
  claim payload's new fields. New fields are tagged "(#142)".
- **Golden files.** `golden_snapshot.json` and `golden_snapshot_aero_cl.json` change by
  three null `global_status` fields, `lp_fees_note` on the Aero rows and the renamed
  "Collectible LP fees" label (their fixtures read no AERO, so they state no rewards
  and gain no reward line). `golden_snapshot_aero_rewards.json` also gains the reward
  objects, `earnings_kind`, the reward lines and the reward wording of the claim text.
  No existing number moved in any of them. New `golden_snapshot_rewards.json`: the
  cycle one block before the operator's claim and the cycle in its block.
- **No new config key.** An existing `config.yaml` loads unchanged.
- New worker modules `rewards` (the reward object, totals, the proven claim calls and
  the gas estimate) and `reward_text` (the shared MCP text); new frontend helpers
  `rewards.ts` and `components/CollectibleEarnings.tsx`.
- **Not exercised:** the worker against a live RPC or database. The live
  `eth_estimateGas` path was run only against a refused port (to pin that no RPC URL
  reaches its error text); the calls it sends were made by hand on 2026-11-17 and
  answered. The dashboard and the Lite cockpit were looked at in a browser against a
  mock API serving the deployment's real snapshot of 2026-11-17 with the new fields
  added, not against a worker.

## [0.8.0] - 2026-11-16

One minor release carrying three planned milestones, built as a stack of seven pull
requests (#120 to #126), reviewed as a whole and merged in order:

- **The whole wallet** (planned as v0.6.0): the worker split into modules (#97), Aero
  concentrated positions (#91), AERO as the fourth pillar (#93), and plan buys detected
  on-chain and offered for confirmation (#118). Snapshot spec 2.5.0.
- **Guided buys** (planned as v0.7.0): read-only route quotes for a due tranche (#115),
  and average cost against spot (#98). Snapshot spec 2.6.0.
- **On your phone** (planned as v0.8.0): install to the home screen, a notification test,
  and an opt-in TLS proxy (#89).

Before you upgrade:

- **Nothing new is required in `config.yaml`.** An existing config loads unchanged. To
  track an Aero concentrated position, add an `aerodrome-cl` entry to `positions:` (see
  `config.yaml.example`); the total and the directive step once when it is first counted.
- **Upgrades are forward-only.** Three additive migrations run at start. An older build
  will not start on a database they have run on (AGENTS.md section 13).
- **Not exercised before release:** anything on a real phone, any Telegram or Pushover
  send, and the TLS proxy, which has never run. They are opt-in.
- Field descriptions tagged "(v0.6.0)" or "(v0.7.0)" in the specs and source name the
  milestone the work was planned under; all of it ships in 0.8.0.

### Added — phone install, notification test, opt-in TLS (#89)
- **Install on a phone.** The dashboard ships a web app manifest (start URL
  `/?view=lite`, standalone), icons, and the iOS and Android home-screen tags. The icons
  are a crop of the roundel in `assets/image_6cc4c3c4.jpg` (the repo has no logo);
  `frontend/scripts/make-icons.sh` regenerates them. **No service worker ships**: nothing
  is cached, so an installed copy can never show an old snapshot as live. The cost is
  that it does not open away from the LAN.
- **The Lite cockpit shows the directive and its own freshness.** A DIRECTIVE card (action,
  details, and `LIVE ON-CHAIN · read 42s ago` or `NOT LIVE · last read 12m ago`) and, when
  a plan is configured, the accumulation plan panel with RECORD BUY. Before this the Lite
  view showed neither, so a "DCA TRANCHE DUE" push landed on a page without the tranche
  and with no sign of how old its numbers were. The card reads `NOT LIVE · Worker
  unreachable: … · last read 50s ago` from the first poll that fails, however recent the
  last read (a phone that left the LAN with the app open), and reads the worker again
  when the app returns to the foreground.
- **Notification status and test routes.** `GET /v1/notifications/status` states which of
  Telegram, Pushover and the webhook are configured. `POST /v1/notifications/test` sends
  one message labelled TEST through every configured channel and reports each channel:
  `delivered`, `failed` with the provider's reason, `incomplete` or `not_configured`. With
  no channel configured it says so and sends nothing. Test sends are limited to one per
  30 s. The dashboard and the Lite cockpit have a SEND TEST NOTIFICATION control.
- **A plan completing notifies.** When the last buy was recorded while LP was under target,
  the action read `REBUILD LP TRANCHES` before and after, so nothing was pushed. The
  completion now carries its own sentence, with the plan's own figures, placed before
  the directive's details so Pushover's 1024-character cut keeps it. A cycle that could
  not read the accumulation ledger no longer makes the next cycle announce an already
  completed plan again.
- **A range exit notifies whatever the directive reads.** A floor lock and a due tranche
  outrank `RANGE EXIT FLASHPOINT`, so under either the action did not change when spot
  reached the top of the price-source range and nothing was pushed. The exit now carries
  its own sentence (`Range exit: spot $… is at or above the top of the price-source
  range ($…)`), from the same rule the directive uses. The return into the range is
  pushed only when it changes the directive, and the last reading is held in memory, so
  an exit across a worker restart is pushed only if the directive changes with it.
- **Opt-in TLS proxy for the Podman deployment.** `NOCKNOCK_TLS=1 deploy/podman/deploy.sh
  <host>` adds the `nocknock-tls` quadlet: Caddy with a private CA on an unprivileged port
  (default 3443; 8080, 3000 and 8081 are refused, and so is a leading zero such as
  `08080`), no `sudo`.
  `NOCKNOCK_TLS=remove` takes it out. With the variable unset
  the deploy script's remote commands are identical to before. Each phone must install
  the CA's root certificate; `docs/kb/how-to-phone-install-notifications-tls.md` states
  what that costs and describes the two alternatives that were not built (a tailnet; a
  real domain with DNS-01).
- **Docs**: `DEPLOYMENT.md` §7, a README section, and the guide above with a phone
  checklist and a list of what was and was not tested. Nothing here has run on a real
  phone, against a real provider, or on the host.

### Fixed — notifier (#89)
- **No token in the logs.** A failed Telegram request logged reqwest's error, which
  includes the request URL, and Telegram's URL contains the bot token. Errors now drop
  the URL and every detail is redacted before it is logged or returned.
- **`pushover_user_key` is accepted.** The example config documented that name while the
  worker read only `pushover_user`, so a copied example left Pushover off without a word.
  Both spellings work; the example now uses `pushover_user`. A config that carries both
  loads: `pushover_user` is used and the status route says `pushover_user_key` is ignored.
- **Half a channel is reported.** A Telegram token without a chat id (or a Pushover token
  without a user key) was skipped silently. It is now `incomplete`, with the missing key
  named, in the status route and in the log.
- **The example `cockpit_url` opens the Lite view.** It ended in `/lite`, which nginx
  serves as the full dashboard; the Lite view is `/?view=lite`. The status route warns
  when it sees the old form.
- **The push follows the store.** It went out before the snapshot was stored, so a failed
  store repeated it every cycle. It is now sent once the snapshot is on record.
- **Provider requests time out after 10 s.** The worker's HTTP client has no timeout, and
  a hung provider held the ingestion cycle open.
- **Long messages fit the provider.** A directive whose details list LP rebuild steps can
  pass Pushover's 1024-character limit. Messages are now cut to each provider's limit with
  a "…", keeping the cockpit link.
### Changed — snapshot spec 2.6.0 (guided buys: #115, #98)
- `SNAPSHOT_SPEC_VERSION`, `specs/core/snapshot.yaml` and the frontend contract test move
  from 2.5.0 to 2.6.0. `accumulation_plan.route_quotes` (#115) and
  `derived_metrics.cost_basis` (#98) are optional or nullable, so an older reader still
  parses a 2.6.0 snapshot.
- The three golden snapshots were regenerated at this point: each case gains
  `cost_basis: null`, the line "Cost basis: not reported by this snapshot" in the
  `get_portfolio_state` text (the fixtures carry no accumulation ledger), and a new state
  hash. Nothing else differs.

### Added — cost basis and the average-cost chart (#98)
- **Plan buys: average cost against spot.** The snapshot gains `derived_metrics.cost_basis`.
  Its `plan_buys` layer is the running average cost of every receipt-verified buy in the
  accumulation ledger (per plan and across plans), what that NOCK is worth at the current
  spot, and the signed difference. It needs no operator input and reads
  `accumulation_events` only; fill recording, receipt verification and plan pacing are
  untouched.
- **Whole stack: only from an operator-entered opening basis.** New optional config
  `cost_basis.opening` (`nock_amount`, `cost_usd`, `as_of`, `source`). NockNock cannot
  derive the stack's history from the chain and never fills these in. Unset, the
  `whole_stack` layer is `not_configured` and carries no number. Set, the blended average
  is (opening cost + verified buys after `as_of`) / (opening NOCK + NOCK bought since),
  labelled with the source note and the instant, with statements of exactly what is and
  is not counted: sells, LP rebalancing, fees, transfers and unrecorded buys after
  `as_of` are not tracked.
- **Config errors are stated, not defaulted.** A half-filled block, a zero or negative
  amount or cost, a non-numeric value, a missing source, an `as_of` in the future, or an
  `as_of` that is a date without a time and offset makes `whole_stack.status` `invalid`
  with a reason naming every problem. The worker keeps running and plan buys still
  report; the error is also logged once at startup.
- **`GET /v1/cost-basis/series`**: every verified fill with the running averages after it
  (its plan's, the ledger's, and the blended one when configured), the stored spot
  that stands for it (within 60 minutes, else null), and the stored spot history over
  the range, thinned to at most one stored observation per bucket. Nothing is
  interpolated: a bucket with no observation has no point. `?days=` sets the range
  (1 to 365).
- **Every spot point is the price at its own time.** The price history stores a
  third-party candle at its open time with its close as the price, which is the price an
  hour (or a day) later. The series uses the candle's open instead, so a backfilled
  point is never a later price shown early; a candle without a usable open yields no
  point. A buy is matched to the worker's own read when one lies within the 60 minutes,
  and to a backfilled point only when there is none.
- **Dashboard panel "Average cost vs spot"** (`AverageCostPanel`, helper `costBasis.ts`):
  the average-cost step line, the spot line with its gaps shaded and never bridged, a
  marker on each buy, a table of what each buy did to the average, and the difference at
  spot stated as a fact. No advice: nothing suggests buying, selling or averaging down.
  The difference is described as a valuation of the NOCK the buys acquired; the panel
  does not claim that NOCK is still held, because sells and LP activity are not tracked.
- **MCP**: `get_portfolio_state` gains two cost-basis lines. Both transports compile them
  from one file (`src/cost_basis_text.rs`), so they cannot drift, and both say so in the
  tool's `tools/list` description. The whole-stack line values "the NOCK this basis
  covers" and puts the NOCK read now beside it.
- **A failed ledger read is `unavailable`, never "no buys".** It does not fail the cycle,
  `plan_buys.fills` is null (not 0) in that state, and both plan-buys tiles on the
  dashboard say "unavailable" with the reason.
- `whole_stack.held_nock` is a whole number of NOCK, the precision the snapshot's own
  total has.
- `/v1/snapshot/historical` rows keep the cost-basis figures and drop the repeated
  `includes`/`excludes` prose, so the history payload stays bounded.
- New modules `cost_basis` and `cost_basis_text`; tests in `tests_cost_basis.rs`, built
  from the operator's three real plan buys re-read from Base (ONCHAIN_INTEGRATION.md §12).
  No migration. The golden snapshot gains `"cost_basis": null` and one portfolio-text
  line per fixture (its pure path does not read the ledger); no other byte moved.
- Specs: `CostBasis`, `PlanBuysBasis`, `WholeStackBasis` in `snapshot.yaml`; the series
  endpoint and `CostBasisSpotPoint` in `api.yaml`. The spec version is not bumped here.

### Added — route quotes for a due tranche (#115)
- **Both venues, quoted for the exact tranche.** When the plan's status is `tranche_due`
  the worker asks each venue's own quoter contract what that tranche amount returns in
  NOCK, along a built-in table of verified routes, all at one block: four Aero routes
  through the Aero Metaquoter (the basic pool directly, through WETH by either
  USDC/WETH pool, and the three-hop route through USDT the Aero app used for the plan's
  first buy) and three Uniswap routes (the v4 USDC/NOCK 3% pool directly, through WETH on
  v3, through native ETH on v4). Each venue shows its best route: NOCK out, effective
  price, cost over the snapshot's reference spot in percent, every hop with its pool and
  fee, and the block and block time of the quote.
- **A fact, not advice.** The comparison names the venue that returns more NOCK and by
  how much ("Aero returns more NOCK by 0.05% for $20.00 USDC at block 58081910 …"). With
  one quote or none, no venue is named. It compares execution routes for a buy the
  operator's plan already sized; it is not a recommendation to buy.
- **Only the plan's own amount.** Quotes are requested only while a tranche is due and
  only for that tranche's amount (after guardrails), keyed on the status field. No plan,
  no due tranche or `route_quotes.enabled: false` means no call. Nothing is signed, built
  or sent: every call is an `eth_call`, and there is no swap calldata, approval, slippage
  or minimum-out value in the code.
- **Unavailable is never a number.** A quote that reverts, gets no answer, or comes from
  a pool that ran dry part-way reads "unavailable" with its reason, for that route and
  venue only. Uniswap QuoterV2 was observed to answer a dry pool with a partial amount
  and the post-swap price pinned at the range's bound, and no error; the worker detects
  that from the price. The Aero pools on these routes hold full-range liquidity and
  fill any size at a collapsing price, so what the Aero Metaquoter answers for a dry
  concentrated pool was not observed: the same price-bound check is applied to it. Nothing is derived from
  the spot price and an older quote is never shown as current. A failed quote never
  fails the cycle or hides the due tranche.
- **Snapshot.** `accumulation_plan.route_quotes` (v0.7.0): `status` (`ok`, `partial`,
  `unavailable`), `amount_usdc`, `block_number`, `quoted_at`, `reference_spot_usd`,
  `venues[]` (with `hops[]` and every candidate in `routes_tried[]`), `more_nock_venue`,
  `more_nock_by_pct`, `comparison`. Null unless a tranche is due. The whole object is
  left out of the state hash, so quotes never force a new snapshot row; the golden
  snapshot file is unchanged for that reason, and the payload's shape is pinned by its
  own fixture instead. While a tranche is due with quoting on, the advice details end
  with one fixed sentence pointing to the quotes; it does not depend on what the
  quoters answered. A snapshot read back from the database at start-up carries no
  quotes until the first cycle reads new ones.
- **Dashboard.** The Accumulation Plan panel shows the two venues side by side while a
  tranche is due, marks the one the worker named, and links each venue's own swap page
  (`app.aero.xyz/swap`, `app.uniswap.org/swap`) labelled as a swap page. Nothing is
  prefilled: neither app documents a link format that also fixes the chain. The panel
  says that quotes are estimates as of their block and that the wallet app's own router
  may pick a different route. REFRESH QUOTES reads again at the latest block. The panel
  reads the quotes from `GET /v1/accumulation/quotes/latest` every 30 seconds, because
  the snapshot's ETag is the state hash and the quotes are not part of it. NOCK amounts
  are cut to four decimals, not rounded, as in the comparison sentence and the MCP
  text. Two candidate routes with the same tokens are told apart by their route id.
- **Cost over the quote, per fill.** When a buy is recorded or confirmed, the worker
  attaches the best stored quote read for its tranche before the buy's block (and not
  before the plan's previous buy) and stores `quote_cost_bps` = fill price / quote price
  − 1, beside the cost over spot from #118. The quote must have been read for that
  tranche (the plan keeps quoting a tranche after its buy lands, until the buy is
  recorded; those quotes never serve the next buy) and within 60 minutes before the
  buy. The recorded-buys table shows it with the quote's block; without such a quote it
  reads "no quote on record", never a later quote, another tranche's or an old one. The gap between quote and fill includes price movement, and the table says so.
- **MCP.** `get_accumulation_plan` prints the quotes, the comparison and the caveat on
  both transports from one shared source file (`quote_text.rs`). MCP gains no tool.
- **New REST endpoints**, unauthenticated like the existing routes:
  `POST /v1/accumulation/quotes/refresh` (no body; 409 `NO_TRANCHE_DUE`, `NO_PLAN` or
  `QUOTES_DISABLED`; 503 `NO_SNAPSHOT`) and `GET /v1/accumulation/quotes/latest` (the
  quotes the worker holds in memory; it makes no call and is never cached). A refresh
  measures cost against the cycle's own full-precision spot; without one it shows no
  cost over spot. `GET /v1/accumulation/events` and the record
  response gain the eight `quote_*` fields.
- **Storage.** New table `accumulation_quotes` and eight nullable `quote_*` columns on
  `accumulation_events` (migration `20261116000012`, additive). One quote row is kept
  per cycle while a tranche is due. Disposable like the rest of the database: after a
  loss, buys recorded later show "no quote on record".
- **Config.** `route_quotes.enabled` (default `true`). There is nothing else to set: the
  amount is the plan's and the routes are the verified table.
- **Verified on Base** (ONCHAIN_INTEGRATION.md §11): every quoter, pool and selector
  answered a live read-only call before the code relied on it, and the tests run on those
  answers. Replayed at the block before the plan's first buy, the Aero quoter returns
  the NOCK that buy received, to the wei. The fee unit of Aero concentrated pools
  (millionths) and the Aero router's identity, both open since v0.5.0, are now confirmed.
### Changed — snapshot spec 2.5.0 (the whole wallet: #91, #93, #118)
- `SNAPSHOT_SPEC_VERSION`, `specs/core/snapshot.yaml` and the frontend contract test move
  from 2.4.0 to 2.5.0. Every field added by #91, #93 and #118 is optional or nullable, so a
  2.4.0 reader still parses a 2.5.0 snapshot.
- The golden snapshot was regenerated at this point: against the #93 golden the only
  differences are `accumulation_plan.fill_detection: null` in the plan case and that
  case's state hash.

### Fixed — found by the review of the whole stack (#127, #128, #129)
- **The RPC URL can no longer reach an error message (#127).** reqwest appends the request
  URL to a network error, and the RPC URL holds the Alchemy key. That text was returned to
  callers that publish it: `/health` `last_error` already in v0.5.3, and with this release
  the Aero CL adapter and the AERO notes stored in the snapshot. All three sites in
  `rpc.rs` strip the URL; a test calls the real function against a refused port and fails
  if the URL or key appears.
- **A stored candle counts as its open, never its close (#129).** A backfilled candle is
  stored at its open time with its close in `price_usd`. The lookup behind harvest values
  and each fill's cost over spot returned that close labelled with the open time: a price
  from an hour (or a day) later. It now reads a candle as its open, prefers the worker's
  own read inside the 60-minute window, and gives no price for a candle with no positive
  open. This is the rule the cost-basis series uses, so one buy shows one spot figure.
  Rows already written are not rewritten.
- **`deploy.sh` does not ship nested git worktrees (#128).** git lists a nested worktree as
  one directory and tar archived it recursively: about 16 GB from a checkout holding
  agent worktrees. tar is now told not to recurse, and `.claude/` is ignored.
- **`specs/core/snapshot.yaml` parses as YAML again.** A description added with #91 held a
  `: ` in a plain scalar. A test now loads both spec files.
- **Deploys are forward-only**, now stated in AGENTS.md section 13 and DEPLOYMENT.md: a
  build older than the newest applied migration does not start.

### Added — fill detection for the accumulation plan (#118)
- **The worker finds the plan's buys on-chain.** Each ingestion cycle, while a plan is
  active, it lists the NOCK transfers into the tracked wallet back to the plan anchor
  (`alchemy_getAssetTransfers`), and checks every transaction it has not seen before
  against its receipt and block header. Cost: one transfer-history call per cycle, plus
  one receipt (and one block header for a purchase) per new transaction.
- **A detected swap is offered, never recorded.** The Accumulation Plan panel shows each
  one with its full hash, amounts, price and block time. CONFIRM calls the existing
  `POST /v1/execute/record-accumulation` with the offered hash, so the receipt is fetched
  and verified again: there is no second write path. NOT PART OF THE PLAN leaves it
  uncounted; it stays listed and can be restored. The manual hash field is unchanged and
  does not depend on detection. Open offers change no plan number: fills, spend, average
  cost and cadence still count ledger rows only. When a tranche is due the panel's hint
  follows the detection state (it promises to find the swap only when detection is
  working) and, with a swap on offer, says to answer it before buying again.
- **One set of rules.** The record handler's checks are now one pure function
  (`fill_verify.rs`: receipt, wallet, block time, plan anchor) that the handler and the
  detector both call. The handler's order of checks, status codes, error codes and
  messages are unchanged.
- **Honest failure.** Detection never fails a cycle. `accumulation_plan.fill_detection`
  (snapshot, v0.6.0) carries `status`: `ok`, `partial` (read, but incompletely, with what
  is missing), `unavailable` (could not check, with the reason) or `not_run` (with the
  reason). "Could not check" is never shown as "none found". A receipt that could not be
  read is not classified; it is tried again next cycle. `last_checked_at` is left out of
  the state hash.
- **Bounds.** The scan goes back to the plan anchor and no further; swaps from before it
  are not offered. A transaction already in the ledger (any plan id) is never offered. A
  swap made less than one plan interval after the buy before it (a recorded fill, or an
  older swap still on offer) is offered with an `early` label; the label does not depend
  on the order in which offers are confirmed. A stalled transfer-history call fails with
  its own reason in about 20 seconds (two 10-second attempts), inside the 60-second cap
  on the whole check.
- **Execution cost per fill.** Each recorded fill and each offer carries
  `execution_cost_bps` over the reference spot at its block time: the stored NOCK/USD
  price nearest the block, within 60 minutes (the v0.5.3 lookup the harvest ledger uses),
  kept with its source. No stored price near the block means no figure, never a
  substitute. Every surface prints the reference's own time next to the figure (the
  dashboard also says how many minutes it is from the swap), because a price move between
  the two reads as cost. A fill confirmed from an offer keeps the figure the offer
  showed. Fills recorded before this release get theirs on the next cycle when a
  stored price exists. `GET /v1/accumulation/events` returns the three new fields and the
  panel lists the plan's fills with them.
- **Advice and MCP.** While a swap is on offer, the advice text leads with it in every
  directive (also when an overextension, a range exit or a harvest outranks the plan, and
  after the plan is complete); the action title does not change, so no extra notification
  fires. `get_accumulation_plan`
  lists offers and the detection state on both transports, from one shared source file.
  MCP gains no write tool.
- **New REST endpoints**, unauthenticated like the existing routes, touching only the
  candidates table: `GET /v1/accumulation/candidates`,
  `POST /v1/accumulation/candidates/dismiss`, `POST /v1/accumulation/candidates/restore`
  (503 `NO_TRACKED_WALLET` when no wallet is configured).
- **Storage.** New table `accumulation_candidates` (migration `20261116000010`, with a
  `fills_checked_at` column on `accumulation_plans`) so a transaction is classified once;
  new nullable columns `reference_spot_usd`, `reference_spot_source`, `execution_cost_bps`
  on `accumulation_events` (migration `20261116000011`). Both additive. The table is
  disposable like the rest of the database: after a loss the next cycle rebuilds it and
  offers every buy since the anchor again.
- **Config.** `fill_detection.enabled` (default `true`) turns the check off. There are no
  thresholds: detection filters nothing by size or price.
- **Verified on Base** (ONCHAIN_INTEGRATION.md §10): tests run on real receipts. The
  plan's three buys qualify; four Uniswap fee claims and two Aero deposits that returned
  a NOCK refund do not.
### Added — AERO rewards, the fourth pillar (#93)
- **AERO is read every cycle.** The wallet's AERO balance, the AERO each staked Aero
  position has earned in its gauge and not claimed, and an AERO/USD price. NockNock
  states the amounts. It sets no target for AERO, puts no rebalancing pressure on it,
  builds no claim transaction for it and gives no advice to claim, sell, hold or
  accumulate it; no directive reads an AERO figure. A wallet holding only gas and AERO
  is still "NOTHING TO TRACK YET", exactly as it would be without the AERO.
- **Which AERO.** Two tokens on Base carry the symbol. The Aero gauges pay
  `0x3f5bF3EdAc59d650D8fDf9657a71F2f904A3a4Ee` (name "Aero", 18 decimals): in two real
  claims, one per gauge type, that token was minted to the claimer for exactly what
  `earned` reported one block earlier. That is NockNock's AERO. The older
  `0x940181a9…8631` (name "Aerodrome"), which issue #93 named, is not what these gauges
  pay and is not read (ONCHAIN_INTEGRATION.md §9).
- **How unclaimed AERO is read.** A basic (volatile pool) gauge answers `earned(address)`;
  a CL gauge answers `earned(address,uint256[])` for the token ids the wallet staked.
  Neither has `rewardToken()`, and the older `earned(address,uint256)` reverts on the CL
  gauge. An `aerodrome` position is read through its `gauge_address`; an `aerodrome-cl`
  position through the gauge the cycle proved it is staked in.
- **A gauge that pays another token is not read as AERO.** A legacy Aerodrome gauge
  (for example `0x4cbb6188…c3db`, the gauge of the legacy vAMM USDC/NOCK pool) answers
  the same `earned(address)` but pays the older token `0x940181a9…8631`, and it does
  have `rewardToken()`. So each cycle asks the gauge `rewardToken()` before `earned`: a
  revert is the Aero app's gauge and is read; an answer other than NockNock's AERO is
  stated on the position ("gauge … pays …, not AERO …") with no amount and no USD value.
- **Price.** New optional config key `aero_price_pool`, an AERO/USDC pool whose `slot0`
  prices AERO. Default: the Aero CL200 AERO/USDC pool
  `0xd0acf293547457fb4fe9ee32ca93e89497626816`, the deepest found. Each cycle the pool's
  tokens must be AERO and USDC and it must have in-range liquidity, or no price is used:
  four other AERO/USDC pools on Base had none and showed $0.64 to $0.80 against a live
  $0.418.
- **Required and optional reads, as #94 decided for gas.** The wallet balance is required
  (a failed read fails the cycle). The price is required while the wallet holds AERO,
  because its value is in the total; while the wallet holds none, a failed price read
  cannot fail the cycle and leaves AERO unpriced with the reason. A gauge read that
  reverts or fails is stated on that position ("unclaimed AERO not read (…)") and makes
  the unclaimed total null: never 0, never a partial sum. An `aerodrome` position with
  no `gauge_address` says its AERO is not read, and it too makes the unclaimed total
  null, because that position's AERO is part of what the wallet has earned.
- **One total.** `total_portfolio_usd` now includes wallet AERO, and the cash and LP
  targets, deviation weights, the high-water mark and history all use that one total.
  **One-time step:** on the first cycle after the upgrade the total, and so the high-water
  mark, rises by the value of any AERO already in the wallet (nothing for a wallet that
  holds none, which is the operator's wallet on 2026-11-16). AERO still unclaimed in a
  gauge is **not** in the total, exactly as uncollected LP fees are not: the total steps
  up by its value when it is claimed.
- **Pillars.** The pillar line is NOCK · USDC · ETH · AERO everywhere (dashboard, both MCP
  transports) and the four add up to the total. The deviation matrix gains a fifth row,
  "AERO (wallet)", with status `no_target`.
- **Snapshot** (spec version unchanged until the v0.6.0 branches are stacked): new
  `global_status` fields `aero_price_usd`, `wallet_aero_amount`, `wallet_aero_usd`,
  `unclaimed_aero_amount`, `unclaimed_aero_usd` and `aero_note` (null when not read);
  optional `positions[].unclaimed_aero`, `unclaimed_aero_usd` and `unclaimed_aero_note`; a
  fifth `wallet_balances` row, AERO. A snapshot without them renders "not reported" or
  "—", never 0.
- **The "AERO rewards not read (#93)" text is gone.** A position whose fees are not
  LP-claimable now says why and then states its AERO: "not LP-claimable (claimFees()
  reverts); unclaimed AERO 0.121026 ($0.05)" in `contract_fees[].amount`, the velocity
  row's `note` and the MCP claim text. `positions[].fees_note` and the claim payload's
  `simulation_detail` carry the reason alone. A velocity row whose fees ARE claimable
  (CLAIM_NOW, ACCUMULATE) says nothing about AERO, so a fee action never sits beside an
  AERO amount; that position's AERO is on its card and in the pillar lines.
- **Harvest ledger.** `POST /v1/execute/record-harvest` books an AERO `Transfer` to the
  wallet in the recorded transaction (a gauge claim mints it, so the sender is the zero
  address), valued from `aero_price_pool` read at the claim's block. When that read fails
  or the pool had no in-range liquidity at that block the row has no USD value, never a
  substitute. A receipt that pays only AERO is accepted. The Koinly export describes an
  AERO row as "NockNock AERO reward claim (…)" and keeps the `income` label. With a
  `tax_reserve_ratio` set, a priced AERO row counts toward the virtual tax reserve like
  any other harvest row.
- **Harvest ledger: one row per transaction and token, summed.** A claim from several
  gauges in one transaction mints AERO to the wallet once per gauge. The ledger's key is
  (transaction, token), so only the first mint was booked and the rest were reported as
  already present. All Transfers of one token to the wallet in a recorded transaction
  are now summed into its one row. This applies to USDC, NOCK and WETH as well, which
  had the same limit. Rows recorded before this change are not rewritten.
- **Harvest ledger: a row recorded without a price can be priced later.** If the price
  read at the claim's block failed, the row was stored with no USD value and stayed that
  way. Recording the same transaction again now fills the missing value from that
  block's price (response field `priced`). A row that has a value is never changed.
- **Dashboard.** A seventh stat card (AERO in the wallet), AERO lines under the pillar
  line (held, unclaimed per position, price, and the reason for anything not read), an
  unclaimed-AERO line on each staked position's card, and an AERO card in the Lite
  cockpit. None of them has a button. The composition readout no longer counts wallet
  AERO as "other legs": it states it ("AERO $0.57 (wallet; not cash)"), so the panel's
  Total equals the parts shown.
- **Golden files.** `testdata/golden_snapshot.json` and `golden_snapshot_aero_cl.json`
  change by the new null fields, the AERO deviation row, the pillar line and the shortened
  reasons; no number moved. A new `testdata/golden_snapshot_aero_rewards.json` is built
  from reads at Base block 58077673: the operator's two staked positions with 12.043104
  AERO ($5.03) unclaimed, and the same cycle with the amount of a real claim in the wallet.
- New worker modules `aero_rewards` (reads, pillar, ledger pricing) and `aero_text` (the
  AERO lines of the MCP text, one source file compiled into both the worker and
  `nocknock-mcp`).

### Added — Aero concentrated positions (#91)
- **New position protocol `aerodrome-cl`.** An Aero concentrated-liquidity ("CL",
  Slipstream) position is read from its NFT: tokens, tick spacing, ticks and liquidity from
  the position manager, price, tick and the current swap fee from the pool. Amounts use the
  same maths as Uniswap v3. Its NOCK counts toward the NOCK total and the floor, its WETH
  toward the ETH pillar, and its range shows in the tick bracket panel, now with which side
  an out-of-range position left on. Nothing is tracked until the position is added to
  `positions:`; `config.yaml.example` carries a commented entry.
- **Ownership is checked every cycle.** The Aero app stakes the NFT in the pool's gauge, so
  `ownerOf` is the gauge. The position is counted only if the wallet holds the NFT or the
  gauge lists it among the wallet's stakes (`stakedContains`); otherwise the cycle fails and
  names the holder. A staked position says so (`positions[].staked_in_gauge`).
- **Fees are stated truthfully.** Swap fees on staked liquidity go to the gauge, measured
  on-chain (ONCHAIN_INTEGRATION.md §8), so a staked position shows "not LP-claimable
  (staked: swap fees on staked liquidity go to the gauge); AERO rewards not read (#93)" on
  the card, in `contract_fees`, the velocity panel, the claim payload and the MCP text,
  never $0 earned. An unstaked position's fees are what `collect` simulated from the wallet
  returns; that read is required like the Uniswap fee reads, so if the simulation reverts
  the cycle fails and says so, and no unread fee is published as 0. With nothing
  LP-claimable in the book, the MCP claim text says "Estimated Gas: n/a (nothing
  LP-claimable)" where it used to blame an unavailable gas estimate (this also changes
  the text for a book holding only an Aero basic pool whose `claimFees()` reverts).
- **The configured pool must be the NFT's pool.** The pool's `nft()` must be the
  position's `npm_address`, and its tokens and tick spacing the NFT's; anything else is a
  configuration error before any amount is computed. Tokens and spacing alone do not
  identify it: the Uniswap v3 WETH/NOCK 1% pool also has tick spacing 200.
- **No rebuild deposit is sized for an Aero CL position**, whatever its pair
  (`lp_rebuild.excluded[]` states why): the Aero app's deposit link starts a new position,
  a different NFT that NockNock would not track, and adding to an existing NFT is not
  verified. A WETH pair keeps the gas reason it already had.
- The tick bracket panel's new range line says "empty" for a ranged position that holds
  nothing, on any protocol, where it would have said "in range, holding …".
- **An unknown `protocol` is a configuration error.** It is reported before any chain read
  ("Configuration error: position '…' has unknown protocol '…'") and by the claim payload.
  Labels, deposit links, the fee contract, the pool liveness read and the claim payload used
  to treat any protocol that was not `uniswap-v3` or `aerodrome` as Uniswap v4.
- **Negative ticks decode.** Tick words are read as sign-extended `int24` and encoded the
  same way in calldata; a negative tick used to fail the read (latent for the tracked
  pairs, whose ticks are positive).
- Snapshot (spec version unchanged until the v0.6.0 branches are stacked): optional
  `positions[].tick_spacing`, `staked_in_gauge`, `fees_note` and velocity `contracts[].note`,
  present only for positions that have them. `record-harvest` and the claim payload accept
  platform `aerodrome-cl`. `testdata/golden_snapshot.json` is unchanged; the new protocol
  has its own `testdata/golden_snapshot_aero_cl.json`, built from reads at Base block
  58061668.

### Changed — worker split into modules (#97)
- **Move-only refactor.** `ingestion-worker/src/main.rs` (10,853 lines) is now 1,322 lines
  plus 22 modules: `evm`, `calldata`, `contracts`, `rpc`, `adapters/{uniswap_v3,uniswap_v4,
  aero_basic}`, `valuation`, `onchain`, `lp_rebuild`, `advice`, `plan`, `market`, `ledger`,
  `ingest`, `api`, `claims`, `notify`, `mcp_text`, `mcp_http` and `tests`. Functions and
  constants moved verbatim and gained `pub(crate)`; types stay in `main.rs`. No behaviour
  change.
- **Golden snapshot test** (`testdata/golden_snapshot.json`): pins the state hash, the
  canonical snapshot and the four MCP texts for three fixtures, byte for byte. It was added
  before anything moved and is unchanged by the split.

## [0.5.3] - 2026-11-15

### Fixed — harvest ledger (#102, #111; snapshot spec 2.4.0)
- **WETH fees are booked (#111).** The harvest ledger read only USDC and NOCK Transfers, so
  most of the v3 WETH/NOCK range's fees were never recorded. It now books WETH paid to the
  wallet, and the native ETH the Uniswap app pays out when it collects to the position
  manager and unwraps: a WETH `Withdrawal` by the v3 NonfungiblePositionManager or the v4
  PositionManager, in a transaction the wallet sent, is booked as "ETH". Unwraps by anyone
  else (a router, say) are not fee payouts and are ignored.
- **Each harvest is valued at its claim's block, not when it was recorded (#111).** NOCK
  uses the stored price nearest the block time (within 60 minutes); ETH and WETH use the
  `eth_price_pool` read at that block. A late recording no longer changes the value. When
  no price is found the row has no USD value and is left out of the total, never valued at
  a substitute. New `harvest_events` columns `price_usd` and `price_source` record the
  price used and where it came from (migration `20261115000007`, additive).
- **The tax-ledger panel's labels come from the data (#102).** The hardcoded "40% TAX
  RESERVE (PROTECTED)", "60% GROWTH DRY POWDER (DEPLOYABLE)" and the always-on "✓ TAX
  RESERVE PROTECTED" are gone. The panel is "Harvest Ledger (Virtual Tax Reserve)"; its
  dials name the configured ratio (or say none is configured), and with no harvests they
  read "no harvests yet" instead of drawing a full bar. It says plainly that the split is
  bookkeeping: nothing is set aside on-chain. The Lite cockpit line follows the same rules.
- **`sovereign_ledger`** gains `tax_reserve_ratio` and `unpriced_count`; the spec text no
  longer calls the reserve "protected" or the rest "available dry powder".
- **BATCH HARVEST text** says the ledger books USDC, NOCK and WETH/ETH, each valued at the
  claim's block, instead of "WETH fees are not booked yet".
- **The claim payload's ETH price** reads the configured `eth_price_pool` instead of always
  the default pool.

## [0.5.2] - 2026-11-14

### Fixed — wallet ETH is the gas pillar (#94; snapshot spec 2.3.0)
- **Gas counts in the total.** Wallet ETH and WETH are read every cycle, and both reads are
  now required: a failed read fails the cycle, as NOCK and USDC do. They are valued at the
  live ETH/USD and included in `total_portfolio_usd`. ETH is gas: never cash, never in the
  cash reserve or deployable dry powder, with no target and no rebalancing pressure.
- **One definition of the portfolio.** Targets, deviation weights, the high-water mark and
  history all use the full total, gas included. **One-time step:** when this ships the
  total rises once by the wallet gas (about +$23 on 0.0125 ETH, about +1%), and the LP and
  cash targets grow by the same ~1%. It is a definition change, not a gain. A total that
  excluded wallet gas was rejected: moving ETH between the wallet and a WETH range would
  then read as a gain or a drawdown.
- **Three pillars.** New `global_status` fields `eth_price_usd`, `wallet_gas_eth`,
  `wallet_gas_usd`, `total_eth_amount` and `total_eth_usd` (wallet gas plus the WETH inside
  LP). NOCK + USDC + ETH = total. `total_native_usd` stays NOCK-only (the spec text that
  said it included ETH is corrected). `wallet_balances` gains ETH and WETH rows after NOCK
  and USDC.
- **Every wallet token balance must be present.** A missing or null `tokenBalance` fails
  the cycle instead of reading as 0.
- **Deviation matrix** gains a "Gas (wallet ETH+WETH)" row with status `no_target`.
- **High-water mark date** is the time of the earliest stored snapshot at the peak, not the
  snapshot time. An empty table starts a new mark, and a database error fails the cycle
  instead of silently resetting the mark to the current value.
- **ETH/USD stale-pool guard.** A price pool with no in-range liquidity fails the cycle, as
  the NOCK spot's does.
- **LP rebuild never funds an ETH leg from wallet gas.** Positions with an ETH or WETH leg
  are excluded ("needs ETH/WETH, which is gas, not investable capital") and their share goes
  to the other eligible positions. The 0.002 ETH `REBUILD_GAS_BUFFER_ETH` is removed.
- **Removed** the unused $2,500 ETH price serde default.
- **MCP** state text adds `Pillars: NOCK $x · USDC $y · ETH $z` and a "Gas held" line, or
  "not reported (snapshot predates 2.3.0)".
- **Dashboard** shows the pillar line and the gas held (never as a "Goal"), ETH in the
  composition readout, the snapshot's own ETH/USD for ETH brackets, and when the
  high-water mark was set.

### Fixed — surfaces (v0.5.2)
- **`GET /v1/snapshot/{id}`** returned 500 `CORRUPTED_DATA` for every stored row: the
  handler parsed the `data` column as a whole envelope, but `store_snapshot` writes only
  the payload. It now parses `SnapshotData` and builds the envelope from the row
  (`envelope_from_stored`).
- **First-run errors are visible.** The 503 `NO_SNAPSHOT` body (latest snapshot and the
  claim payload) carries the last ingestion error as `detail`, and MCP appends it to "No
  snapshot available yet". The dashboard and the Lite cockpit show that cause instead of
  "Worker unreachable"; "unreachable" is now said only when the worker did not answer.
- **Aero pools whose `claimFees()` reverts** read "not LP-claimable (claimFees()
  reverts); AERO rewards not read (#93)" in the velocity panel, the MCP claim simulation
  and the Lite cockpit, never "$0 earned". In the claim payload the action reports
  `simulation: "not_claimable"` instead of "RPC unavailable", gets no gas estimate, and
  no longer nulls `estimated_gas_usd` and `net_after_gas_usd` for the actions that can
  execute.
- **MCP honesty (both transports).** "Live API" and "Tier 1 Connected" are gone; every
  answer ends with the snapshot's generation time and the worker's health. The harvest
  rule says BATCH HARVEST is advised only when no DCA tranche is due. Tool descriptions say
  "LP" instead of "Uniswap" for all positions.
- Specs: `recommended_action` lists the values the worker emits (`claim_now`,
  `claim_recommended`, `accumulate`, `not_claimable`); `simulation` adds `not_claimable`.

### Docs
- README §1–2 rewritten: the retired portfolio and the operator's numbers presented as
  "the strategy" are gone; example numbers are one operator's, not recommendations; the
  current directive precedence and the LP rebuild bias; link to the four-pillars lore.
  Quick Start: `urmanac/nocknock`, a config step, Node 22+.
- OPERATOR_GUIDANCE §7.11–7.14: four pillars, gas is not cash, the wallet is the record,
  building in public; the AERO item reworded in pillar terms.
- ONCHAIN_INTEGRATION: dated "superseded in v0.5.0" notes; HANDOFF.md marked historical;
  AGENTS.md §10 points to the lore; design note §8.2 records that the no-plan arm was
  retired in v0.5.1; "in production" dropped from the mock-data invariant headings.

### Deferred to Future Releases
- [ ] **Koinly On-Chain Transaction Classifier (Issue #56, v0.4.0)**: reconcile on-chain fee-claim harvests directly with Koinly's Base wallet sync as ordinary income — explicitly NOT manual CSV download buttons/tooling (duplicate transactions and burns the annual 1,000-transaction budget on the $99 tier; Koinly misclassifying claims as "Liquidity Removals" corrupts LP cost basis).
- [ ] **Native Android App (APK)**: Standalone native Android client (currently served via responsive mobile web cockpit).
- [ ] **Multi-Pool Dynamic Ingestion**: Discovering and tracking secondary AMM pools on Base.
- [ ] **Coinbase Staked-ETH Connector**: CDP API integration to monitor staked ETH balances.
- [ ] **Urbit Native (%nocknock Desk)**: Hoon data structures and Gall agent.
- [ ] **Phone install, notifications and TLS (Epic #89)** for the internal deployment.
- [ ] **Accumulation plan Phase 2**: redesign for Aero's router, or keep operator-executed swaps and measure execution cost per fill against spot.
- [ ] **Unsigned add-liquidity transactions** for LP rebuild suggestions (encoding to be verified by simulation).
- [ ] **Aero concentrated (CL) positions (Issue #91)**, starting with WETH/NOCK CL200.
- [ ] **AERO reward accounting (Issue #93)**: read, value and ledger rewards from staked Aero positions.
- [ ] **Externalise the legacy advice constants.**

## [0.5.1] - 2026-11-14

Patch: "honest numbers", part 1. Plan: `docs/kb/plan-v0.5-patches.md`.

### Fixed — advice states only what was read (#95, #63)
- **HOLD** is built from the tracked positions (in range, out of range, stale pool, full
  range). Removed the never-computed "0% realized loss", "generating yield" and
  "Uniswap position(s)".
- **No default floor.** DEFENSIVE LOCK fires only below a floor the operator set, without
  "CRITICAL" or "locked". With an active plan the action reads
  `DEFENSIVE LOCK (Floor Recovery) · <plan action>`, so tranches still notify.
- **Defaults choose nothing.** Target weights and the tax ratio default to "no rule"
  (`no_target`), and an empty `tracked_wallets` is an explicit error. Configs that set
  these keys, like the operator's, are unchanged.
- **No planless lump-sum buy.** The old DIP REINVESTMENT arm became three facts-only
  directives: `NOTHING TO TRACK YET`, `BUILD FROM ZERO` and `DIP ZONE` (each "No Buy Plan
  Configured" where relevant).
- **Dry powder** is wallet USDC less the larger of the tax reserve and the cash target. The
  tax guardrail compares wallet USDC only.
- **No sell instructions.** SWEEP TO STABLE became `OVEREXTENDED (Daily RSI ≥ 70, No Buy
  Advised)`. RANGE EXIT reports the range's own value, with no invented $5 wall.
- **A plan with nothing due no longer hides RANGE EXIT or BATCH HARVEST.**
- **BATCH HARVEST** percentages come from `tax_reserve_ratio`, and the text says WETH fees
  are not booked yet.
- **Regime** reads `unknown` with no daily RSI. The RSI is undefined on a flat series, and
  the spot must be finite.
- **MCP** stops inventing values ("above 100k", HOLD, $0.00), and the stdio fallback no
  longer reads a hardcoded wallet or a dead pool.
- **Dashboard** shows no invented HOLD fallback and no "NaN NOCK", and shows "no floor
  configured" for a floor of 0.

### Fixed — charts (#61)
- **Downtime gaps** are shaded and captioned, on a real time axis.
- **The composition chart** uses each row's own totals. It was about 8 hours stale.
- **LP series are keyed by `position_id`.**

### Removed
- **The nightly `pg_dump` timer and backup tooling** (#96, not planned): the database is
  disposable, and the wallet is the record.

### Notes
- **Snapshot spec:** additive enum values (`no_positions`, `no_target`, regime `unknown`).
- **Tests:** worker 91 → 119, frontend 45 → 84.

## [0.5.0] - 2026-11-14

### Added — Aerodrome Full-Range Tranche (Issue #88)
- **`protocol: aerodrome` positions.** A volatile x*y=k pool is tracked as a full-range
  position: the wallet's LP-token share of the reserves, with fees from `claimFees()`
  simulated from the wallet. It counts in totals, the deviation matrix, charts, fee
  reporting, the claim payload and LP rebuild suggestions. It is always in range and has
  no sell-out multiple (it sells NOCK gradually as price rises and never empties).
  v0.4.0 had removed Aerodrome support, so such a position was invisible. A pool whose
  `claimFees()` reverts (the new Aero pools) reads as having no LP-claimable fees
  instead of failing the cycle. `gauge_address` adds LP tokens staked in the pool's
  gauge to the wallet's balance.
- **Shipped config.** Tracks the Aero Basic Volatile NOCK/USDC pool listed on
  app.aero.xyz (`0x770fE8Fc…9e0A`) with its gauge. The legacy Aerodrome NOCK/USDC pool
  is still the deepest market but the Aero app does not list it, so it is not tracked as
  a position. The two empty Uniswap v4 positions (#3163431 on a dead pool, #3163432 out
  of range) are retired from the shipped position list. A volatile pool can serve as the NOCK/USD price source (spot = reserve
  ratio); the shipped config keeps the Uniswap v4 3% pool as the source.
- **Deposit links** go to the Aero app (`https://app.aero.xyz/deposit/<chain>-<pool>`),
  never to the retired aerodrome.finance. Display names say "Aero"; the `aerodrome`
  protocol key is kept for ledger continuity.
- Positions and brackets carry `full_range`; the dashboard shows FULL RANGE, draws no
  bracket for it, and no longer assumes the price source is the first position.
- Suggestion buttons name their real destination ("Open position in Uniswap web app",
  "Open deposit on aero.xyz"); the label was hardcoded to "Open in Uniswap"
  for every venue. A suggestion capped by the aggregate LP gap reports the share being
  filled, not "$128 of $162 wanted".

### Added — Operations
- **Nightly database backup** for the Podman deployment (`nocknock-backup.timer`,
  03:30, newest 14 kept in `~/nocknock/backups`). The database now holds the
  accumulation ledger.
- `make test-mcp` honours `NOCKNOCK_API_URL`, so both MCP transports can be playtested
  against a remote deployment.

### Fixed — Pre-0.5.0 Triage
- **NOCK stack chart units (Issue #85).** The floor chart plotted wallet NOCK from a USD
  series, drawing a 95,599-NOCK stack at 1,095 against the 100,000 floor. The worker
  emits `wallet_nock_series` (token units) and the chart uses it; older snapshots are
  recovered from their own USD value and spot, and an unknowable point is a gap.
- **Example config layering (Issue #86).** `config.yaml.example` is now really the base
  layer. It was silently skipped (unknown `.example` extension), so a partial
  `config.yaml` failed ingestion.
- **Honest `/health` (Issue #87).** `degraded` when the last ingestion cycle failed or
  the data is older than the sync window plus grace, with `last_error`. It used to
  report `ok` whenever any snapshot existed.
- The dashboard's positions summary is computed from the positions. It used to hardcode
  "0% realized losses", call every position a "concentrated Uniswap position" and count
  a dead pool as in range.
- The claim-fees payload says "Nothing to collect" for an empty position instead of
  "RPC unavailable", and the plan's MCP text no longer says "$0.00 due now" while waiting.

### Added — LP Rebuild Suggestions: a Call to Action for REBUILD LP TRANCHES (Issue #83)
- **Concrete deposits.** While LP is under target, `lp_rebuild.suggestions` sizes one
  deposit per eligible tracked position (amounts of each token at the pool's current
  price) with a link to that position in the Uniswap app; the advice text carries the
  same steps. The operator makes the deposit; the worker then reads it on-chain.
- **Rules.** The LP target is split equally across eligible positions, or by
  `positions[].rebuild_weight_bps`. Out-of-range positions are not filled unless
  `rebuild_when_out_of_range: true` (operator rule: "we should not generally aim to fill
  positions that are out of range").
- **Dead pools are never suggested**, and positions gain `pool_active`. A pool with no
  in-range liquidity has a frozen price, so `in_range` judged against its own tick is
  meaningless; the dashboard now shows STALE POOL instead of a green "in range" pill.
- **LP exposure of floor NOCK is bounded and reported.** A range sells its NOCK as
  price rises. Suggestions may place the NOCK above `treasury_floor_nock` plus
  `operator_profile.lp_floor_tolerance_bps` of the floor (default 0). The operator
  accepts a small overage on purpose: ranges sell gradually, so only a large rally
  sells everything, and more NOCK in range means more trading and fees. Each suggestion
  states the price multiple at which it would have sold all its NOCK, and
  `lp_rebuild.worst_case_nock` states what would remain if every range sold out.
- **Funding is checked, never assumed.** Suggestions leave liquid USDC for the cash
  target, tax reserve and remaining DCA tranches, and keep an ETH gas buffer. A short
  source scales every position that uses it by the same factor (independent of config
  order) and is named; an unread balance or pool state withholds the suggestion.
- **Snapshot 2.2.0 (additive)**: `lp_rebuild.suggestions`, `lp_rebuild.excluded`,
  `positions[].pool_active`.

### Changed — Protected Cash Reserve Counts USDC Inside LP
- `accumulation_plan.cash_reserve_usdc` is now measured on the stable balance (wallet
  USDC plus USDC inside LP positions), per the operator's rule that rebuilding LP is
  cash-neutral. Moving USDC into a range no longer consumes reserve headroom; buying NOCK
  does. If a range converts its USDC to NOCK on a dip the stable balance falls and the
  plan stops spending.

### Added — Accumulation Plan: Dollar-Cost-Averaged NOCK Buys, Phase 1 (Issue #79)
- **Optional `accumulation_plan` config** (budget, tranches, interval, optional price
  ceiling). While configured, the advice engine paces buys as `DCA TRANCHE DUE (n of N)` /
  `DCA ON SCHEDULE` instead of advising one lump-sum deployment. The plan only paces:
  nothing is signed or sent. Missed windows never stack; a pause defers without consuming
  a tranche; guardrails can shrink a tranche (naming the limit) but never enlarge it.
- **Protected cash reserve** (`accumulation_plan.cash_reserve_usdc`): the plan never
  advises a buy that would take liquid USDC below it, at any price, and says so in the
  advice. It binds independently of the 5% cash target, so an oversized budget or the
  default targets cannot dilute the cash position past it. Only the operator lowers it.
- **Accumulation ledger** (`accumulation_events`) and
  `POST /v1/execute/record-accumulation { tx_hash }`: a buy counts only after the worker
  verifies on-chain that the transaction moved USDC out of, and NOCK into, the sovereign
  wallet. Amounts and the fill time come from the receipt and its block, never from the
  request. A buy confirmed before the plan began (`starts_at`, or when the worker first
  saw the plan) is rejected, so an old hash cannot advance a new plan.
  `GET /v1/accumulation/events` lists verified fills.
- **Standing LP-rebuild bias** (operator standing order, 2026-11-14): rebuilding LP
  tranches leaves the net cash position unchanged, so when LP is under target
  `REBUILD LP TRANCHES (No Net Cash Change)` leads whenever the engine would otherwise be
  idle and is appended to every other directive. It never displaces a due tranche or a
  safety directive.
- **Snapshot 2.1.0 (additive)**: `derived_metrics.accumulation_plan` (status, next
  tranche, spend, average cost, lump-sum counterfactual) and `derived_metrics.lp_rebuild`.
- **MCP**: new `get_accumulation_plan` tool (HTTP and stdio). The stdio server reads
  `NOCKNOCK_API_URL` so it can target a remote deployment.
- **Dashboard**: "Accumulation Plan" panel with progress, average cost versus the
  lump-sum counterfactual, the LP-rebuild note and a RECORD BUY field.

### Changed
- **Aerodrome is now Aero (Issue #81).** Aerodrome and Velodrome merged into Aero
  (aero.xyz) on 2026-10-21. The plan's default venue label is "Aero"; wording that
  described current behaviour as Aerodrome is corrected (historical references stay).
  The observed Aero router, pool factory and the three-hop route of the first plan
  tranche are recorded in ONCHAIN_INTEGRATION.md §6, marked unverified against official
  documentation. Receipt verification is contract-agnostic and needed no change.
- **Advised spend respects the cash target.** `DIP REINVESTMENT SHOT` (the no-plan path)
  no longer advises spending liquid USDC below `target_weights_bps.cash`, and is skipped
  when nothing is spendable. Previously the 5% cash target was displayed but never applied.

### Known Limitations in 0.5.0
- **Aero concentrated (CL) pools are not tracked (Issue #91).** The operator holds a
  WETH/NOCK CL200 position (about $180 at deposit, NFT #2914490); it is invisible to
  NockNock, so portfolio and LP totals understate by that amount and the LP weight reads
  lower than it is.
- **AERO rewards are not read, valued or ledgered (Issue #93).** Aero positions are
  staked by the Aero app and earn AERO.
- Aero volatile positions are read only through a configured `gauge_address` (plus any
  LP held directly in the wallet). The first live read matched the deposit exactly.
- ETH is not counted in portfolio totals, so ETH moved into a position leaves the
  tracked total without appearing anywhere.
- Suggested LP deposits are sized, not built: the operator makes them in the venue's app.
- A rebuild suggestion smaller than $5 is not shown; the position is listed as not
  fundable, and the rebuild note then annotates the directive instead of leading it.
- Notification delivery for `DCA TRANCHE DUE` has not been exercised on the on-prem
  deployment (Epic #89).
- 17 frontend dependency advisories remain with no upstream fix (unchanged from 0.4.0).

## [0.4.0] - 2026-11-13

### Security — Dependency Advisories (Issue #75)
- **Ingestion worker: 16 → 0 actionable advisories** (`cargo audit`).
  - `sqlx` 0.7.4 → 0.8.6 (RUSTSEC-2024-0363, binary protocol misinterpretation), which also
    drops the whole legacy TLS chain it pulled in (`rustls` 0.21, `rustls-webpki` 0.101,
    `webpki-roots` 0.25).
  - Lockfile refresh for the TLS stack that authenticates the Alchemy Base RPC endpoint:
    `rustls` 0.23.45 → 0.23.46 (RUSTSEC-2026-0346/0347/0348/0349), `rustls-webpki` →
    0.103.17 (RUSTSEC-2026-0382, distrusted intermediates), `webpki-roots` → 1.0.12
    (RUSTSEC-2026-0381).
  - `rsa` (RUSTSEC-2023-0071) and `libsqlite3-sys` (RUSTSEC-2026-0459) remain in
    `Cargo.lock` only as sqlx's unused MySQL/SQLite backends; neither is compiled
    (`cargo tree -i` prints nothing). Recorded with justification in
    `ingestion-worker/.cargo/audit.toml`.
- **`ingestion-worker/Cargo.lock` is now tracked.** It was gitignored while the Dockerfile
  copies it, so a clean checkout could not build the image and dependency versions were
  neither reproducible nor auditable.
- **Frontend: 27 → 17 advisories, both criticals cleared** (`npm audit`): `vitest` 2 → 5
  (GHSA-5gmw-xhrv-c9v3, GHSA-5xrq-8626-4rwp), `vite` 5 → 8 (GHSA-4w7w-66w2-5vf9),
  `openapi-typescript` 6 → 7 (`undici`, `@fastify/busboy`), `@typescript-eslint` 7 → 8,
  `@vitejs/plugin-react` 4 → 6, plus `npm audit fix` for `follow-redirects`.
- **Known remaining, no upstream fix**: `zod` < 4.6.6 (GHSA-mrhx-c7xc-m3w5, memory
  exhaustion on oversized input) inside `@walletconnect/ethereum-provider` 2.26.0 →
  `@reown/appkit`, which accounts for 11 of the 17; and `minimatch` (moderate, lint-time
  only) inside `@typescript-eslint`. The only offered remedy for the first is downgrading
  WalletConnect to 2.19.4; forcing `zod` 4 under a library written for `zod` 3 was not done
  because the wallet-pairing path cannot be exercised without a paired phone.

### Fixed — Stale Spot From an Illiquid Price-Source Pool (Issue #76)
- **Zero-mock guard**: each cycle the worker reads the price-source pool's in-range
  liquidity (StateView `getLiquidity(bytes32)` for v4, `liquidity()` for v3) and fails the
  cycle explicitly when it is zero. Previously a pool emptied by the operator's own
  withdrawal kept reporting its frozen tick as live spot ($0.01695 vs a $0.0123 market,
  overstating the portfolio by about $540 and skewing daily RSI from 36.5 to 56.6).
- **`config.yaml.example`**: `is_price_source` moved from the v4 0.75% pool to the v4 3%
  pool; `market_data.pool_address` pinned to the Aerodrome NOCK/USDC vAMM (deepest market)
  for OHLCV/RSI history.

### Added — Rootless Podman Deployment for Internal Hosts
- `deploy/podman/`: quadlet units (database, worker, frontend, network, volume) and
  `deploy.sh [host]`, which ships the working tree minus gitignored files, builds both
  images on the host and (re)starts the stack as systemd user services. `config.yaml`
  (Alchemy key) and a generated database password live in Podman secrets, never in an
  image; the database is not published to the host. See DEPLOYMENT.md, Option D.
- Dockerfiles use fully qualified base images (`docker.io/library/...`) so Podman builds
  without unqualified-search registries; the frontend builds on Node 22 and accepts
  `VITE_WALLETCONNECT_PROJECT_ID` as a build argument.

### Changed — Uniswap-Only Portfolio (Aerodrome tranche liquidated 2026-10-23)
- **Snapshot model v2.0.0 (breaking)**: `SnapshotData.positions` is an N-capable list
  (network, wallet, protocol incl. real `uniswap-v3`, nft_id, pool, fee tier, tokens,
  amounts, unclaimed fees, range, in_range) replacing the retired two-tranche shape.
  Removed fields (migration notes):
  - `asset_aggregation_series.tranche_1_nock` / `tranche_2_nock` / `tranche_3_nock` →
    `positions_nock: [{ position_id, series }]`; `safe_haven_series` (wallet NOCK) and
    `active_protocol_series` (LP USD) keep their semantics.
  - `concentrated_liquidity` is deprecated and now emitted as an empty array (schema
    retained so historical rows stay readable); per-position range/fee detail moved to
    `positions`.
  - Worker inputs `live_tranche_1_*` / `live_tranche_2_*` / `live_aero_fee_*` /
    `live_uni_fee_*` replaced by per-position reads; the `v4_baseline_*` scaling
    heuristic is gone (amounts are now exact on-chain math).
  - `meta.version` → `2.0.0`; `PoolMechanic.platform` gains real `UniswapV3`;
    `ClaimFeeAction.platform` / record-harvest gain `uniswap-v3`.
- **Ingestion pipeline**: per-position reads for N Uniswap-v4 + M Uniswap-v3 positions
  (POSM `getPositionLiquidity`/`poolKeys`, StateView slot0/positionInfo/feeGrowth,
  v3 NPM `positions(uint256)` decode + pool fee-growth math — validated against an
  NPM `collect` staticcall ground truth). NOCK/USD spot comes from the designated
  price-source position pool (v4 NOCK/USDC); ETH/USD moved off the liquidated
  Aerodrome pool to the canonical v3 WETH/USDC 0.05% pool (`0xd0b53d92…f224`,
  factory-resolved and price-verified); price provenance tag `aerodrome_rpc` →
  `uniswap_rpc` for new rows. Both price reads fail the cycle explicitly (zero-mock).
- **Config**: new `positions:` registry + `eth_price_pool` (discovery-verified
  identifiers in `config.yaml.example`); `target_weights_bps.uniswap_v4_lp` →
  `uniswap_lp` (legacy keys still parse but are ignored); `v4_position_id` and the
  `v4_baseline_*` profile anchors retired.
- **Golden Path**: claim-fees payload builds one pre-simulated Uniswap-v4 collect per
  tracked position (no Aerodrome `claimFees()` anymore); token-aware USD valuation.
- **Frontend**: one card per tracked position (pair, protocol badge, fee tier,
  readable range, split, value, unclaimed fees, in-range pill); stacked NOCK chart
  built from per-position series; tick hypervisor renders one bracket per position.
- **Docs**: ONCHAIN_INTEGRATION.md §2/§4 marked HISTORICAL with the full Uniswap-only
  discovery record in §5; README portfolio section rewritten.

## [0.3.0] - 2026-10-13

### Added — Market Price Backfill & Multi-Timeframe Regime Engine
- **Historical Price Backfill Engine (`market_price_history`)**:
  - New provenance-tagged price table: every row records `source` (`aerodrome_rpc` live RPC spot vs `geckoterminal`/`dexscreener` gap fills), `is_backfilled`, and `backfilled_at` — third-party retro data can never masquerade as contemporaneous on-chain observation.
  - Contemporaneous snapshots record the live Aerodrome spot price each cycle (`is_backfilled: false`), even when the full snapshot is deduplicated by `state_hash`.
  - On boot and at the start of every cycle, gap detection (newest point older than 15 minutes) backfills completed hourly candles from GeckoTerminal (limit 168) and extends daily depth to at least 14 days (limit 30) — idempotent `ON CONFLICT DO NOTHING` upserts; the provider's in-progress candle is never stored.
  - DEXScreener fallback seeds at most a single point stamped at its observation time (never backdated into the gap); provider outages log warnings and never block the ingestion loop or on-chain polling.
  - Optional `market_data` config section (base URLs, pool address, gap/depth thresholds, lookbacks); existing config files keep working unchanged.
- **Multi-Timeframe Regime Engine**:
  - The `Overextension Cooldown` directive now keys on the **daily RSI(14)** (`rsi_14_1d`), computed over one close per UTC day spanning the multi-day lookback — a 4-hour intraday bounce no longer fakes a multi-day top.
  - The short-term series remains exposed as `rsi_14` (RSI(14) over 5-minute snapshot closes) with unchanged semantics; `regime: normal|overextended` reflects the daily series.
  - `GET /v1/snapshot` + `/v1/snapshot/latest` expose top-level `data_provenance: { last_backfill_at, backfilled_points_count, sources }` (computed live; excluded from `state_hash`/ETag).
- **Frontend Observability (Tactical Shot Advice banner)**:
  - `REGIME: NORMAL` (green) / `OVEREXTENDED` (amber) badge sourced from the daily RSI.
  - `RSI(14D)` and `RSI(5M)` readouts with honest `—` when a series lacks enough samples.
  - Subtle `✓ N backfilled candles (GeckoTerminal)` provenance indicator fed by `data_provenance.sources` (feed named from data, never hardcoded).

## [0.2.1] - 2026-10-12

### Fixed — 0.2.x Triage Cycle (task 000664, live-data audit)
- **Zero-mock**: ETH/USD price lookup in the claim-fees gas valuation no longer falls back to a hardcoded $2,500; when the price RPC fails, gas USD degrades to "unavailable" instead of a fabricated valuation.
- **Zero-mock**: removed the synthetic Aerodrome tick bracket (0.00000 → 999.00000) from `tick_bracket_hypervisor`; the chart now shows only the live tick-derived V4 bracket (Tranche 2 liquidity remains visible in the stack/deviation panels).
- **Zero-mock**: MCP formatters (`nocknock-mcp` stdio + `/v1/mcp` HTTP) surface the live gas break-even from the snapshot ("unavailable" when absent) instead of a hardcoded ~$0.005 gas and a stale $5.00 execution trigger.
- **Tax honesty**: harvest `confirmed_at` now uses the on-chain block timestamp instead of the recording wall clock, keeping Koinly export dates correct across pending-broadcast retries.
- Gas break-even no longer silently clamped at $0.05 (documented $0.002 L1-data buffer retained).
- Dashboard: deviation matrix legend no longer shows a phantom black swatch for "Actual %" (the per-status color key below the chart documents the real cell colors); Floor Health keeps its "%" unit; tick-bracket subtitle derives IN/OUT OF RANGE from live state; residual hardcoded fallbacks (floor 100000, bracket strings, $0.00 price) replaced with honest placeholders; 60/40 dials no longer draw a phantom sliver at zero.
- Lite Cockpit: EXECUTE actions hidden while WalletConnect is unconfigured; an idempotent record-harvest retry (409 ALREADY_RECORDED) now completes as recorded instead of showing a failure; API errors surface the backend's human `detail` with the HTTP status preserved.
- Golden-path handlers return a clean 503 when no tracked wallet is configured instead of panicking on the empty config index.
- Makefile tmux targets use `$(CURDIR)` instead of a hardcoded macOS path.

### Tests
- Worker: 33 unit tests (+2: claim-simulation gas honesty, no synthetic bracket).
- Frontend: 25 tests (+5: error mapping detail/status, record-harvest classifier).

## [0.2.0] - 2026-10-12

### Added — Golden Path: Web Cockpit & Mobile Wallet Execution
- **WalletConnect v2 Integration (`@walletconnect/ethereum-provider`)**:
  - Instant QR pairing modal and deep links connecting the cockpit to the Uniswap mobile app on Android/iOS.
  - Active session auto-reconnects across refreshes with zero private key custody; mismatched wallet addresses are strictly rejected before signing.
  - Requires `VITE_WALLETCONNECT_PROJECT_ID` (Reown project ID), gracefully displaying setup hints when unconfigured.
- **Sequential Harvest Execution Engine (`GET /v1/execute/claim-fees/payload`)**:
  - Dynamically constructs unsigned Aerodrome `claimFees()` and Uniswap V4 PositionManager collect transactions (`modifyLiquidities` decrease 0 + `takePair`) from live on-chain snapshots.
  - Calldata verified against `@uniswap/v4-sdk` planner specifications and pre-simulated on Base via Alchemy `eth_call`.
  - Dispatched directly via WalletConnect for biometric confirmation (Fingerprint / Face ID); no transaction aggregation or proxying (preserving Aerodrome `msg.sender` invariant).
- **Receipt-Verified Harvest Ledger (`POST /v1/execute/record-harvest`)**:
  - Queries Base transaction receipts, extracts ERC-20 Transfers for USDC and NOCK paid to the sovereign wallet, and records verified harvest events idempotently.
  - Resilient broadcast retry handling absorbs Uniswap mobile app delayed broadcast latency without false errors.
- **Virtual 60/40 Sub-Ledger (`sovereign_ledger`)**:
  - Maintains internal accounting allocation within the single sovereign wallet driven by `tax_reserve_ratio` (default 40%).
  - Splits verified harvest revenues into a strictly protected Virtual Tax Reserve Buffer (40%) and Liquid Growth Dry Powder (60%).
  - Tax guardrail suppresses buy recommendations (`SWEEP TO STABLE / ACCUMULATE DRY POWDER (Tax Guardrail)`) when tax reserves exceed stable cash reserves.
- **Koinly-Ready Tax Export Engine (`GET /v1/tax/export?format=koinly`)**:
  - Generates standardized CSV exports adhering strictly to Koinly custom import specifications.
  - Fee harvests tagged unambiguously as `income` with verified Base transaction hashes, ensuring CPAs and tax software distinguish yield income from capital dispositions.
- **Anti-Top-Buying Guardrail & Regime Detection**:
  - Wilder RSI(14) calculated over historical NOCK spot price series; surfaces `regime` and `rsi_14` in GlobalStatus.
  - Overextension Cooldown suppresses FOMO buy directives when RSI ≥ 70 (`SWEEP TO STABLE / ACCUMULATE DRY POWDER (Overextension Cooldown)`).
- **Directive-Transition Notifications**:
  - Dispatches transition alerts via Telegram, Pushover, and generic HTTP webhooks when active directives shift, with silent first-boot and deep links to cockpit.
- **Lite Mobile Cockpit & 1-Click View Switcher**:
  - Dedicated mobile-friendly cockpit at `/?view=lite` rendering floor health, uncollected fee valuation, and Golden Path execution buttons.
  - 1-click view switcher pills in header (`⚡ LITE COCKPIT` ↔ `📊 FULL STATE ROOM`).
- **Sovereign 60/40 Ledger Panel**:
  - Dual visual gauges on desktop dashboard plotting protected tax reserves vs deployable dry powder, alongside live P/L vs high-water mark.

## [0.1.4] - 2026-10-12

### Fixed — Worker Data Integrity
- **Snapshot deduplication actually fires** (`fix(worker): canonical state hash`): the
  state hash embedded wall-clock timestamps (`updated_at`, `high_water_mark_date`, series
  `time_points`, fabricated channel timestamps), so it changed every ingestion cycle and
  `store_snapshot` inserted a new row every 5 minutes even with unchanged on-chain state —
  growing the table unboundedly and filling history charts with duplicate flat points.
  The hash now strips volatile time fields; identical state no longer inserts rows while
  `/v1/snapshot/latest` still serves fresh timestamps from the in-memory cache.
- **Strategy Counterfactual uses recent history**: the backfill query selected the
  OLDEST 100 snapshots (`ORDER BY generated_at ASC LIMIT 100`), so the chart stopped
  reflecting recent history as the table grew. It now selects the most recent 100,
  oldest-first, as paired `(timestamp, portfolio, native)` tuples so the two series can
  never misalign, and carries real snapshot timestamps instead of fabricated 5-minute
  spacing (structural channels likewise).
- **Bollinger window off-by-one**: the final SMA window now includes the newest sample.

### Fixed — Zero-Mock Invariant
- Failing ETH-price or gas-price RPC queries now **fail the ingestion cycle** instead of
  silently substituting hardcoded `2500.0` USD / `0.005` USD / `0.0043` USD fallbacks into
  snapshots (gas break-even, efficiency ratios, advice text).
- `recommended_action` is computed from claimable yield vs gas break-even
  (`claim_now` at ≥2× gas, else `claim_recommended` / `accumulate`) — the dashboard no
  longer shows "CLAIM NOW" at $0 claimable.
- Uniswap V4 bracket display prices, `in_range`, and `distance_bps` are derived from the
  on-chain ticks (verified `0.012366` / `2.02788`) instead of duplicated hardcoded
  strings; the RANGE EXIT advice threshold uses the same derived bound.

### Fixed — Frontend
- **Dashboard survives transient fetch failures**: `useFreshness` no longer tears down and
  recreates its polling interval (with an extra foreground fetch) on every render, and a
  failed fetch keeps last-known data — the full-screen error state appears only when no
  snapshot has ever loaded. History fetch retries every 60s while empty.
- **Deep, bounded chart timeline** (~24h instead of ~4h): charts request 288 snapshots;
  historical responses truncate embedded multi-point series the charts never read,
  bounding the payload to ~1 MB.
- **Honest stat labels**: the "Alpha Generated vs passive hold" stat is relabeled to what
  it actually measures — "Stable Reserve Delta (Total − Native-Only)"; the counterfactual
  panel is titled "Portfolio Composition: Total (Native + USDC) vs Native-Only".
- The tick-bracket chart shows the true upper price (removed hardcoded $2.50 clip).
- New 7th panel renders the backend Bollinger structural channel (SMA ± 1σ/2σ bands)
  when band data exists.

### Fixed — API Contract
- `/v1/snapshot/historical` implements `before`/`after` RFC3339 cursor filters, clamps
  `limit` to 1–1000, and returns 400 for the never-implemented `interval` parameter
  (removed from `specs/core/api.yaml`).
- `client.ts` `request()` no longer lets caller headers clobber `Content-Type`.
- Removed dead code: `fetchIfFresh`, `useFreshnessPoll`.

### Tests
- Worker: 18 unit tests (was 11) — hash time-invariance/value-sensitivity, counterfactual
  alignment & real timestamps, Bollinger window, bracket-price round-trip, bracket
  distance, claim-action thresholds.
- Frontend: 20 contract/logic tests green; tsc build clean.

## [0.1.3] - 2026-10-12

### Fixed
- **Uniswap V4 Live Fee Querying via StateView (`0xa3c0...7a71`)**:
  - Implemented live on-chain queries against official Base Uniswap V4 `StateView` contract using `getPositionInfo` (`0xdacf1d2f`) and `getFeeGrowthInside` (`0x53e9c1fb`).
  - Added deterministic zero-dependency `U256` math (`wrapping_sub` and `mul_div_q128`) to calculate accurate uncollected fees matching on-chain balances and the Uniswap mobile app.
  - Eliminated silent mock fallbacks and static baseline numbers, preserving the zero-mock invariant. Uncollected fees disappear and accrete dynamically upon fee collections.
- **Tiered Freshness Status & Grace Allowance**:
  - Added sync duration measurement (`sync_duration_ms`) via `Instant::now()` exposed on `/health` and `/v1/health` alongside `sync_window_seconds`.
  - Introduced 3-tier sync status in `useFreshness` and App header (`LIVE ON-CHAIN` green, `SYNC IN PROGRESS` cyan, `SYNC DELAYED` amber) with a 90-second grace allowance (`syncGraceSeconds = 90`) on top of the 300s window.
  - Eliminated false-alarm "Data is five minutes old" / "Data is six minutes old" warnings during normal scheduled background sync cycles.
  - Switched `/v1/snapshot/latest` HTTP caching to `Cache-Control: no-cache, must-revalidate` and added `{ cache: 'no-cache' }` fetch headers to prevent browsers from trapping stale snapshots.
- **Contract & Route Aliases**:
  - Mounted `/v1/snapshot` and `/snapshot` route aliases in Axum router alongside `/v1/snapshot/latest`.
  - Added contract tests in `freshness.contract.test.ts` validating tiered sync status, grace allowance thresholds, and `/v1/health` JSON payload contract.

### Added
- **Model Context Protocol (MCP) Stdio Configuration**:
  - Added repository `.mcp.json` configuring `nocknock-mcp` in stdio mode for Gemini Antigravity, Claude Desktop, and Pi Harness (`pi-mcp-adapter`).
  - Documented MCP configuration, environment variables, and verification commands in `OPERATOR_GUIDANCE.md`.

## [0.1.2] - 2026-10-11

### Fixed
- **API Routing Contract & Health Probe**:
  - Mounted `/v1/health` in Axum router alongside root `/health`, adhering to `specs/core/api.yaml` OpenAPI server path (`/v1`) and frontend `api.health()` client expectations ([#16](https://github.com/urmanac/nocknock/issues/16)).
  - Eliminated `.replace('/v1', '')` workaround in `frontend/src/api/freshness.contract.test.ts`.
  - Added dedicated contract test validating `/v1/health` JSON schema response structure.
- **Zero-Mock Data Invariant in Derived Metrics**:
  - Eliminated hardcoded fallback floats (`649976.0`, `24237793.0`, `4004.31`, `23.63`, `5655.59`, `156.64`, `96004.52`, `0.23`, `0.0268`) in `compute_derived_metrics` ([#16](https://github.com/urmanac/nocknock/issues/16)).
  - Ensured dip reinvestment advice requires positive spot price (`spot_price > 0.0 && spot_price < 0.020`) to prevent uninitialized zero values from masquerading as market dips.

## [0.1.1] - 2026-10-11

### Fixed
- **Freshness Polling Hook & Stale Closure**:
  - Resolved stale snapshot reference in `useFreshness` polling interval with `snapshotRef` and continuous background conditional GET polling ([#7](https://github.com/urmanac/nocknock/issues/7)).
- **Dashboard Metric Calculation & Contrast**:
  - Re-engineered Portfolio Deviation Matrix with high-contrast Sky Blue (`#38bdf8`) Target % and vivid Emerald/Amber/Red Actual % on dark background; bright axis tick labels (`#cbd5e1`/`#e2e8f0`) ([#8](https://github.com/urmanac/nocknock/issues/8)).
  - Replaced hardcoded static percentages (`+6.3%`, `+5.6%`) with live mathematical derivations.
- **Dynamic Chart YAxis Bounds**:
  - Fixed clipping in Sovereign NOCK Stack AreaChart by deriving dynamic domain bounds framing the 100k floor ([#9](https://github.com/urmanac/nocknock/issues/9)).
- **Uniswap V4 Uncollected Yield**:
  - Dynamically calculates accreting V4 fees scaled by position liquidity ratio and config baselines ([#5](https://github.com/urmanac/nocknock/issues/5)).
- **Historical Snapshot Timestamp Integrity**:
  - Preserved original `generated_at` timestamps in PostgreSQL by skipping row mutation when state hash is unchanged ([#10](https://github.com/urmanac/nocknock/issues/10)).
- **Elimination of Silent RPC Mock Fallbacks**:
  - Strict validation on Alchemy RPC calls for pool reserves, token balances, total supply, and V4 liquidity to fail explicitly rather than substituting synthetic constants ([#11](https://github.com/urmanac/nocknock/issues/11)).
- **Canvas Tooltip Date Formatting**:
  - Resolved `Invalid Date` display in the Total Value vs Baseline dual-axis chart and Strategy Counterfactual chart.
  - Eliminated redundant `new Date()` wrapping on pre-formatted snapshot time strings.
- **Chart Tooltip Theme & Visual Contrast**:
  - Aligned all Recharts tooltips with the State Room dark aesthetic (`#161b22` container, `#30363d` border, `#8b949e` label header).
  - Eliminated default stark white tooltip cards that clashed with cockpit canvas panels.

- **Model Context Protocol (MCP) Verification & Stateless HTTP**:
  - Exposed `/v1/mcp` and `/mcp` endpoints on the Axum ingestion worker delivering stateless HTTP MCP services compliant with the 2024-11-05 MCP specification.
  - Added optional Bearer token authentication via `mcp_bearer_token` configuration.
  - Added comprehensive automated test suite (`scripts/playtest-mcp.py` / `make test-mcp`) validating handshake, tool discovery, and tool execution across both STDIO and HTTP transports.
  - Dynamically bound `nocknock-mcp` version to `env!("CARGO_PKG_VERSION")`.
  - Removed silent mock spot price fallback in edge direct-RPC mode, ensuring explicit failure states when Base RPC queries fail.
- **Operator Guidance & Invariants Compilation**:
  - Published [`OPERATOR_GUIDANCE.md`](OPERATOR_GUIDANCE.md) documenting standing orders, branch protection protocols, zero-mock-data rules, and 60/40 virtual sub-ledger accounting.

### Changed
- **CI Build Performance**:
  - Added `Swatinem/rust-cache@v2` across `build-brain` and `build-worker` GitHub Actions jobs, dramatically accelerating offline SQLX compilation times.

## [0.1.0] - 2026-10-11

### Added
- **Tactical Dual-Growth Engine**:
  - Live on-chain tracking for sovereign asset NOCK (16 decimals) and USDC (6 decimals) on Base.
  - Three distinct strategic tranches:
    - Tranche 1: Uniswap V4 Concentrated LP (`0.01237 - 2.02783`) scaled dynamically via position NFT `#3126000`.
    - Tranche 2: Aerodrome Volatile vAMM LP (`vAMM-USDC/NOCK`) proportional share calculation.
    - Tranche 3: Sovereign Protected Treasury with 100,000 NOCK hard floor invariant.
- **Dynamic On-Chain Fee Harvesting & Pricing**:
  - Direct read-only simulation (`eth_call`) of `claimFees()` (selector `0xd294f093`) on Aerodrome pool (`0x85f1aa3a70fedd1c52705c15baed143e675cd626`).
  - Auto-compounding detection for Uniswap V4 modifying liquidity deltas.
  - Dynamic on-chain ETH/USD spot pricing queried directly from Aerodrome `vAMM-WETH/USDC` (`0x88A43bbDF9D098eEC7bCEda4e2494615dfD9bB9C`).
  - Dynamic `tick_current` derivation from spot price ratios.
  - Live Base gas price query with claim efficiency and ROI calculations.
  - Alchemy RPC resilience helper with exponential backoff and randomized jitter (`post_rpc_with_retry`).
- **Backend Ingestion Worker**:
  - Rust Axum HTTP service with PostgreSQL persistent storage.
  - Persistent High-Water Mark tracking with automatic ratchet upward.
  - SHA-256 state hashing and snapshot deduplication.
  - Offline SQLX query caching (`SQLX_OFFLINE=true`) for reproducible container builds.
  - Graceful Unix signal handling (`SIGTERM`/`SIGINT`) for clean worker shutdown.
  - Robust configuration loading with fallback to `config.yaml.example` and environment variables.
  - 6 unit tests covering fee hex decoding, ETH spot parsing, tactical advice decisions, and state hash determinism.
- **Model Context Protocol (MCP) Adapter (`nocknock-mcp`)**:
  - Dedicated CLI binary communicating over stdio JSON-RPC 2.0.
  - Tools for AI agents: `get_portfolio_state`, `get_shot_advice`, `simulate_claim_fees`.
  - Zero-DB progressive degradation with direct Base decentralized RPC fallback.
- **Monitoring Portal (Frontend)**:
  - 6 KPI stat cards with dual-goal metrics and floor health buffer.
  - Tactical Shot Advice cockpit banner with 5-tier dynamic advice matrix.
  - 7 visual data canvas panels built with Recharts.
  - React `ErrorBoundary` wrapper and `useFreshness` polling hook.
- **Progressive Degradation Architecture**:
  - Extensible WASM Brain (`wasm32-wasip1`) compiled for edge evaluation and Android app embedding.
  - Architectural blueprint for MCP adapter and direct Alchemy/Base RPC fallbacks.
- **Operations & Orchestration**:
  - Persistent background execution via `make tmux` orchestrating worker and frontend.
  - Disaster recovery database tooling via `make db-backup` and `make db-restore`.
  - Multi-stage Dockerfiles and Docker Compose configuration.
  - Complete automated CI and release packaging pipeline for GitHub Actions (`make dist`).
- **Licensing**:
  - Licensed under Apache 2.0, Copyright 2026 Urmanac.
