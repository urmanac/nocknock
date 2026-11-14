# Design Note: Dollar-Cost-Averaged NOCK Accumulation ("Accumulation Plan")

> **Status: Phase 1 IMPLEMENTED (issue #79)** · drafted 2026-11-14 against v0.4.0
> Target: v0.5.0 (new protocol feature, so a minor release per the SemVer cadence rule)
>
> Sections 1–4 are the original design. **Section 8 records what Phase 1 actually
> shipped and where it differs**, including the operator's LP-rebuild standing order.

## 1. The Problem

After the 2026-11 liquidation the wallet is fully liquid: 115,747.8 NOCK, 1,053 USDC,
0.114 ETH, no LP. The advice engine's answer to "what do I do with the USDC" is one line:

> DIP REINVESTMENT SHOT: Deploy $1053.00 of saved USDC … to purchase NOCK at discount

That branch (`ingestion-worker/src/main.rs`, the `spot_price < 0.020` arm of the Dynamic
Shot Advice Engine) has three gaps:

1. **It is all-or-nothing.** `deployable_usdc = total_stable_usd − tax_reserve_usd`. There
   is no notion of a tranche, a schedule, or "you already bought today".
2. **It ignores the cash target.** `target_weights_bps.cash` (5%) appears in the deviation
   matrix but never limits what the advice tells you to spend.
3. **It has no memory of buys.** Harvests are receipt-verified into `harvest_events`;
   purchases are recorded nowhere, so the engine cannot know a plan is in progress and
   cannot report an average cost.

The operator wants NOCK buys dollar-cost averaged, executed on Aerodrome.

## 2. What DCA Is Buying You Here (and What It Is Not)

Measured at Base block 57202723 on the Aerodrome NOCK/USDC vAMM
(`0x85f1aa3a70fedd1c52705c15baed143e675cd626`, reserves ≈ 379,719 USDC / 30.97M NOCK,
0.3% fee):

| Buy size | Price impact (xy=k) | Pool fee | Gas (live estimate) |
|---:|---:|---:|---:|
| $1,053 in one swap | ≈ 0.28% | 0.30% | ≈ $0.004 |
| $105 tranche | ≈ 0.03% | 0.30% | ≈ $0.004 |

So splitting the buy saves about a quarter of a percent of slippage in total, and gas is
negligible on Base at any sensible tranche size. **Slippage is not the reason to DCA
here; timing is.** NOCK moved ~12% inside a single afternoon of data pulls on 2026-10-24.
The feature is about spreading entry price over time, so the design optimises for pacing
and discipline, not execution cost.

## 3. Principles (inherited, not new)

- **Zero custody.** NockNock never holds keys. Every tranche is an unsigned payload the
  operator signs in the Uniswap mobile app over WalletConnect, exactly like claim-fees.
- **The schedule is a prompt, not a bot.** Nothing executes without a signature. A
  "tranche due" is an advice state and a notification, never an automatic transaction.
- **Zero mock data.** Plan progress is derived only from receipt-verified on-chain
  transfers. No synthetic fills, no assumed prices. If the pool quote fails, the payload
  endpoint fails explicitly.
- **Pure decision logic.** The "is a tranche due, and how big" decision is a pure function
  with the clock passed in as an argument, unit-tested like
  `directive_transition_notification`.
- **Guardrails outrank the schedule.** The floor lock, the overextension cooldown and the
  tax guardrail keep their existing precedence; the plan can only spend what they allow.

## 4. Design

### 4.1 Configuration

New optional section. Absent section = today's behaviour, so existing configs keep working.

```yaml
accumulation_plan:
  id: "2026-11-reentry"            # operator-chosen label; keys the ledger
  budget_usdc: 0                   # total to deploy over the plan      ← operator decision
  tranches: 0                      # number of equal buys               ← operator decision
  interval_hours: 0                # minimum spacing between buys       ← operator decision
  starts_at: "2026-11-15T00:00:00Z"
  venue:
    protocol: "aerodrome-v2"
    pool: "0x85f1aa3a70fedd1c52705c15baed143e675cd626"   # NOCK/USDC vAMM
  max_slippage_bps: 50             # amountOutMin = quote × (1 − this)
  max_price_usd: null              # optional ceiling: skip a tranche above this spot
  pause_when_overextended: true    # daily RSI(14) ≥ 70 pauses the plan
  respect_cash_target: true        # never spend below target_weights_bps.cash
```

The three zeroed fields are deliberately left blank in this draft: budget, count and
cadence are the operator's allocation decision, not an engineering default.

### 4.2 Budget ceiling

Each cycle the engine computes what the plan is allowed to spend, taking the smallest of:

```
remaining_plan   = budget_usdc − Σ receipt-verified spend for this plan id
deployable       = total_stable_usd − tax_reserve_usd            (existing guardrail)
above_cash_floor = total_stable_usd − cash_target_usd            (new, if respect_cash_target)
```

`tranche_usdc = min(budget_usdc / tranches, remaining_plan, deployable, above_cash_floor)`.
If that is below a dust threshold (say $1) the plan reports `blocked` with the binding
constraint named in plain words.

For orientation only, the configured targets applied to today's wallet ($2,475 total):
cash 5% = $124, so at most $929 is above the cash floor; closing the treasury gap to 80%
would take about $558 of NOCK buys; the 15% LP target is $371. These are the config's
existing numbers, shown so the budget field can be chosen with them in view.

### 4.3 Decision function

```rust
enum PlanDirective {
    NotConfigured,
    Pending   { starts_at },
    TrancheDue{ index, of, usdc },
    Waiting   { index, of, next_due_at },
    Paused    { reason },            // overextended | above max_price | floor lock
    Blocked   { reason },            // tax guardrail | cash floor | no stable balance
    Complete  { spent_usdc, acquired_nock, avg_price },
}

fn plan_directive(plan, fills: &[Fill], market: &Market, guards: &Guards, now) -> PlanDirective
```

Rules:

- A tranche is due when `now ≥ last_fill_at + interval_hours` (or `now ≥ starts_at` for the
  first one).
- **Missed windows do not stack.** If the operator is away for three intervals, exactly one
  tranche is due on return and the plan simply finishes later. Catch-up buying would
  recreate the lump sum the plan exists to avoid.
- A pause defers; it does not consume a tranche.
- `index` is derived from the count of verified fills, never from wall-clock arithmetic,
  so a failed or unsigned transaction can never advance the plan.

### 4.4 Advice engine integration

When a plan is configured and not complete, the dip-reinvestment arm is replaced by the
plan's directive. Precedence becomes:

1. `DEFENSIVE LOCK` (unchanged)
2. `SWEEP TO STABLE` overextension cooldown (unchanged; plan shows `Paused`)
3. Tax guardrail (unchanged; plan shows `Blocked`)
4. **`DCA TRANCHE DUE (3 of 10)`**: "Buy $X of NOCK on Aerodrome. $Y remains in the plan;
   average cost so far $Z."
5. **`DCA ON SCHEDULE`**: "Tranche 3 of 10 is next, due in 14h. Nothing to do."
6. Range exit / batch harvest / hold (unchanged)

The `spot < $0.020` trigger no longer gates buying while a plan is active: the plan's own
`max_price_usd` replaces it, so a plan does not silently stall if price drifts above an
old hardcoded line. With no plan configured the existing arm stays as it is, except that
it should also respect the cash target (a small fix worth shipping regardless).

`directive_transition_notification` already fires on directive changes, so
`ON SCHEDULE → TRANCHE DUE` produces a Telegram/Pushover/webhook ping with the cockpit
deep link for free.

### 4.5 Execution (Golden Path extension)

`GET /v1/execute/accumulate/payload`, mirroring `/v1/execute/claim-fees/payload`:

1. Refuse unless the directive is `TrancheDue` (returns the directive otherwise).
2. Quote: `pool.getAmountOut(usdc_in, USDC)` on the Aerodrome pool.
3. Build up to two unsigned transactions:
   - `USDC.approve(router, usdc_in)`: exact amount, only if the current allowance is short.
     Never an unlimited approval.
   - `Router.swapExactTokensForTokens(usdc_in, amountOutMin, [route], wallet, deadline)`
     with `amountOutMin = quote × (1 − max_slippage_bps)`, `deadline = now + 10 min`,
     recipient hard-set to the tracked wallet.
4. Simulate both with `eth_call` / `eth_estimateGas` and return `simulation: success |
   failed | unavailable` per action, as claim-fees does.

Router and factory addresses, the `Route` struct encoding and the selectors must be
RPC-verified against Base mainnet and recorded in ONCHAIN_INTEGRATION.md before any
payload ships, the same discipline as the v4 collect encoding. This draft does not assert
them.

The cockpit and Lite Cockpit get a "BUY TRANCHE" button that appears only when a tranche
is due, showing amount in, minimum out, implied price and price impact before the wallet
prompt.

### 4.6 Recording fills

New table, same shape and rules as `harvest_events`:

```sql
CREATE TABLE accumulation_events (
    id BIGSERIAL PRIMARY KEY,
    plan_id TEXT NOT NULL,
    tx_hash TEXT NOT NULL UNIQUE,
    venue TEXT NOT NULL,
    usdc_spent_raw TEXT NOT NULL,
    nock_received_raw TEXT NOT NULL,
    usdc_spent TEXT NOT NULL,
    nock_received TEXT NOT NULL,
    effective_price_usd TEXT NOT NULL,
    block_number BIGINT NOT NULL,
    confirmed_at TIMESTAMPTZ NOT NULL,     -- on-chain block time, not wall clock
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

`POST /v1/execute/record-accumulation { tx_hash }` fetches the receipt and accepts it only
if it contains a USDC `Transfer` **from** the tracked wallet and a NOCK `Transfer` **to**
it; amounts come from the logs, never from the request. Idempotent on `tx_hash`, with the
same "approved but not yet broadcast" retry handling as record-harvest.

Because acceptance depends only on the receipt, a buy made by hand in the Aerodrome UI
can be recorded into the plan by submitting its hash. That keeps the operator free to
execute wherever is convenient while the plan stays truthful.

### 4.7 Snapshot, API and MCP surface

Additive, so snapshot version 2.0.0 → 2.1.0:

```
derived_metrics.accumulation_plan {
  plan_id, status, tranche_index, tranches_total, tranche_usdc,
  next_due_at, spent_usdc, remaining_usdc, acquired_nock, avg_price_usd,
  lump_sum_nock            // NOCK the whole budget would have bought at the plan's first fill price
  binding_constraint       // when paused or blocked
}
```

`lump_sum_nock` gives an honest scoreboard: DCA versus buying everything at the start,
both priced from real fills, in the spirit of `strategy_counterfactual`. It can and
sometimes will show DCA losing; that is the point of measuring it.

MCP: new `get_accumulation_plan` tool; `get_shot_advice` text picks up the new directives
automatically.

### 4.8 Tax

Each fill establishes cost basis. Per the Issue #56 stance, Koinly gets these from its own
Base wallet sync; NockNock does not export them as CSV. The ledger exists for the engine
and for tie-out, and the Koinly tie-out suite should assert that Σ `accumulation_events`
matches the wallet's on-chain USDC→NOCK swaps.

## 5. Phasing

| Phase | Delivers | Touches chain? |
|---|---|---|
| **1. Pacing** | config, decision function, advice text, cash-target fix, snapshot fields, dashboard card, notifications, `record-accumulation` | No (read-only receipts) |
| **2. Execution** | `accumulate/payload`, verified Aerodrome encoding, BUY TRANCHE button | Builds unsigned txs |
| **3. Price-aware** | optional value averaging (larger tranche when spot is below its N-day mean), per-plan analytics | No |

Phase 1 alone is enough to start: the engine paces and remembers, the operator swaps in
the Aerodrome UI and pastes the transaction hash. Phase 2 removes the copy-paste.

## 6. Open Questions for the Operator

1. **Budget, tranche count, cadence.** The three blank fields in §4.1.
2. **Overextension.** Pause the plan while daily RSI ≥ 70 (drafted), or keep buying on
   schedule regardless, on the view that DCA should not try to time?
3. **Price ceiling.** Is there a spot price above which you would rather stop accumulating?
   Today's hardcoded line is $0.020.
4. **Hand-executed buys.** Should a USDC→NOCK swap the worker notices on-chain be attached
   to the active plan automatically, or only when you submit its hash (drafted)?
5. **LP re-establishment.** The LP target is 15% and currently 0%. Is rebuilding LP part of
   this plan (a single-sided USDC range below spot is itself a price-triggered buy ladder,
   per the position-intents note), or a separate decision after accumulation?
6. **ETH.** The wallet's 0.114 ETH is not counted in portfolio totals today. Should it be,
   and is any of it in scope as a funding source?
7. **One plan or several.** Is one active plan at a time enough for v0.5.0?

## 7. Out of Scope (unchanged)

- Unattended execution, session keys or any form of delegated signing.
- Venue routing or aggregation. The operator named Aerodrome; one venue, one pool.
- Sell-side averaging and take-profit ladders (same machinery, later).
- Multi-wallet plans.

## 8. Phase 1 As Built

### 8.1 Operator decision: LP rebuild is a standing bias, not part of the plan

Open question 5 is resolved (operator, 2026-11-14): *"rebuilding the tranches has
basically no net cash position change so we should always act with a bias towards doing
that."* The engine's own accounting agrees: `total_stable_usd` is wallet USDC plus USDC
inside positions, so moving funds into ranges changes where the stable balance sits, not
how large it is.

Implemented as `derived_metrics.lp_rebuild` plus `apply_lp_rebuild_bias`:

- LP is `under_target` when its weight is below target by more than the deviation
  matrix's own tolerance (20% of the target weight, minimum 100 bps).
- While under target, **`REBUILD LP TRANCHES (No Net Cash Change)` becomes the primary
  directive whenever the engine would otherwise say "nothing to do"** (hold, DCA on
  schedule, pending, paused, blocked). The displaced status is kept in the details.
- It is appended as a "Standing bias" sentence to every other directive.
- It never outranks a due tranche or a safety directive (floor lock, overextension
  cooldown, tax guardrail). If it did, a rebuild that is never done would hold the
  primary slot forever and the `TRANCHE DUE` notification could never fire.

The rebuild is not budgeted by the plan. The two interact only through the wallet: USDC
moved into LP is no longer liquid, so later tranches are sized against what remains and
name the limiting guardrail if they shrink.

### 8.1a Operator decision: protect the cash position

Operator, 2026-11-14: *"Let's not dilute the cash position overtly — we have lots of
cash, we want to keep it. There might be a dip!"* The default targets (cash 5%) would
have let a plan spend about $929 of the $1,053 on hand, and the example plan ($500 over
five daily buys) nearly half.

Resolved with `accumulation_plan.cash_reserve_usdc`: a floor on liquid USDC that the
plan can never draw below, whatever the budget, the targets or the price. It is a hard
cap in `plan_directive`, alongside the tax reserve and cash target, and the smallest
limit wins. The operator chose **$950 protected, held at any price** (no automatic
release on a dip; the reserve is lowered by hand when the operator decides to use it).
First plan: $100 over five buys, 24 hours apart.

### 8.2 Differences from the draft

| Draft | As built | Why |
|---|---|---|
| `venue` object with protocol and pool | `venue` is a label (default "Aerodrome") | Phase 1 builds no transactions, so the pool address has no consumer yet. Phase 2 reintroduces it with verified addresses. |
| `max_slippage_bps` | omitted | Same reason: only meaningful once the worker quotes and builds the swap. |
| Floor lock listed as a pause reason | Floor lock does not pause the plan | Below the floor, buying NOCK is the remedy. `DEFENSIVE LOCK` stays the primary directive and the plan keeps its own status. |
| Plan completes after N tranches | Plan completes when the budget is spent | Guardrails can shrink tranches, so a plan may need more buys than planned; `tranches_total` grows to match rather than stranding budget. |
| Deployable = stable − tax reserve | Deployable uses **wallet** USDC | USDC inside LP positions cannot be spent on a swap. |
| Ledger read failure unspecified | Plan reports `blocked` | Treating an unreadable ledger as "no buys yet" would re-advise a tranche already bought. |
| `confirmed_at` falls back to wall clock (as harvests do) | No fallback: recording fails | The fill time sets when the next tranche is due; a wrong time would shift the schedule. |
| Any verified USDC→NOCK swap can be recorded | Only swaps confirmed at or after the plan's anchor | Found in the chudmax playtest: a week-old swap was accepted and instantly completed a new plan. The anchor is `starts_at` when set, otherwise when the worker first saw the plan id (`accumulation_plans` registry). |
| Cash-target fix "worth shipping regardless" | Shipped for the no-plan path too | The legacy `DIP REINVESTMENT SHOT` now leaves the cash target behind and says so. |
| `spot < $0.020` gate | Dropped while a plan is configured | The plan's own `max_price_usd` replaces it. A configured plan, even a completed one, also retires the lump-sum arm so completion is not followed by "deploy everything left". |

### 8.3 Surface

- Config: optional `accumulation_plan` (see `config.yaml.example`); venue label defaults
  to "Aero".
- Ledger: `accumulation_events` (migration `20261114000005`).
- API: `POST /v1/execute/record-accumulation { tx_hash }`, `GET /v1/accumulation/events`.
- Snapshot 2.1.0 (additive): `derived_metrics.accumulation_plan`, `derived_metrics.lp_rebuild`.
- MCP: `get_accumulation_plan` (HTTP and stdio). The stdio server also honours
  `NOCKNOCK_API_URL` so it can read a remote deployment instead of `127.0.0.1:8080`.
- Dashboard: "Accumulation Plan" panel with a RECORD BUY field.

### 8.5 Aero transition (Issue #81) — Phase 2 must be redesigned

Sections 2 and 4.5 were written assuming Aerodrome's classic router and the direct
NOCK/USDC vAMM. Aerodrome became Aero (aero.xyz) on 2026-10-21, and the first plan
tranche shows what that means in practice (full record: ONCHAIN_INTEGRATION.md §6):

- **Cost.** §2 estimated ~0.3% fee plus ~0.03% impact per $105 tranche. The first real
  $20 tranche paid about **1.8% over spot**, because aero.xyz routed USDC → USDT → WETH
  → NOCK through three concentrated pools and the WETH/NOCK hop alone charges ~0.97%.
  The §2 conclusion still holds (DCA here is about timing, not cost), but each tranche
  is roughly six times more expensive to execute than that table says.
- **Encoding.** The trade went through a universal-router style
  `execute(bytes,bytes[],uint256)` on `0x0fef9e88…`, not `swapExactTokensForTokens`.
  Phase 2 as drafted in §4.5 would build the wrong transaction.
- **Quoting.** `pool.getAmountOut` on the vAMM no longer predicts what the operator
  receives, since the route does not touch that pool.

What Phase 2 needs before any payload is built: an official Aero contract list (none
found yet; the router identity is so far third-party-labelled only), the command
encoding for `execute`, and a quoting source that matches Aero's routing. An option
worth weighing instead of reimplementing routing: keep Phase 1's model (the operator
swaps in the app, the worker verifies the receipt), which is already
router-agnostic, and spend Phase 2 on **measuring** execution cost per fill against
spot at the fill's block so the state room shows what each venue actually costs.

The legacy vAMM still holds deep NOCK/USDC liquidity at a better implied price than
the routed fill. Whether it is still safely tradable through the legacy router is
untested and is the operator's call, not something the engine should assume.

### 8.6 LP rebuild suggestions (Issue #83) and strategy-consistency fixes

`REBUILD LP TRANCHES` reported a gap but gave no way to act on it. The engine now sizes
a deposit per eligible tracked position (`lp_rebuild_suggestions`, pure) and links to
the position in the Uniswap app. Operator rules: equal split unless weighted; do not
fill out-of-range positions by default; USDC inside LP counts toward the cash reserve.

Building it surfaced three places where the strategy contradicted itself. The operator
authorised fixing these in code (OPERATOR_GUIDANCE §7.9):

1. **LP could breach the hard floor.** `total_nock` counts NOCK inside LP toward the
   100,000 floor, but a range sells its NOCK as price rises. An equal split of the LP
   target would have placed about 18,900 NOCK in ranges with only 17,346 above the
   floor, so a rally could sell the stack below the floor and trip `DEFENSIVE LOCK`.
   First fix: a hard cap at the NOCK above the floor (observed live: both
   suggestions scaled from $185.62 to $170.09, placing exactly 17,346 NOCK).
   **The operator overruled the hard cap the same day**: the overlap is deliberate.
   Ranges sell gradually, so only a large rally sells everything, and the point is to
   trade more and earn fees. Final rule: LP may hold the NOCK above the floor plus
   `lp_floor_tolerance_bps` of the floor (operator's config: 500, i.e. up to 5,000
   NOCK). The engine no longer forbids the exposure; it bounds and reports it. Each
   suggestion shows the price multiple at which it has sold all its NOCK, and
   `worst_case_nock` shows what remains if every range sells out. Those multiples
   differ a lot between ranges: on 2026-11-14 the wide USDC/NOCK range needed a
   ~3.6x rise to sell out, the narrow WETH/NOCK range only ~1.44x (against ETH).
2. **"In range" was reported for a dead pool.** `in_range` compares a position's ticks
   with its own pool's tick. NFT #3163431's pool has no in-range liquidity, so its tick
   is frozen at $0.0169 while the market is at $0.0123; it read "in range" and would
   have been a rebuild target. Fix: positions carry `pool_active`, dead pools are never
   suggested, and the dashboard shows STALE POOL.
3. **The cash reserve and the LP rule disagreed.** The reserve counted wallet USDC only,
   while the LP rule treats USDC in a range as still cash. A USDC-bearing deposit would
   have blocked the DCA plan. Fix (operator's choice): the reserve is measured on wallet
   plus LP USDC. Consequence stated in the advice and tests: if a range converts its
   USDC to NOCK on a dip, the stable balance falls and the plan stops spending.

Open strategy questions this leaves, none decided:

- **Dip behaviour of LP USDC.** With USDC in a range counted as reserve, a falling price
  spends the reserve automatically (the range buys NOCK). That is a price-triggered buy
  ladder, which may be exactly what "there might be a dip" wants, or may not be.
- **Execution cost is unmeasured.** The first tranche paid ~1.8% over spot. The ledger
  stores the fill price but not spot at the fill's block, so cost per venue cannot yet
  be tracked over time.
- **Legacy advice constants.** The no-plan arm still hardcodes the $0.020 accumulation
  line, a 120,000 NOCK compounding goal and "$5.00+ high walls"; standing orders say
  such numbers belong in config.
- **Deposits are suggested, not built.** The unsigned add-liquidity transaction (v4 POSM
  `modifyLiquidities`, v3 NPM `increaseLiquidity`) needs its encoding verified by
  simulation before the cockpit can offer it for signing.

### 8.4 Still open

Question 1 is answered for the first plan ($100 / 5 / 24h, $950 reserve). Question 5
is answered by 8.1 and 8.6. Questions
2 (keep or drop the RSI pause), 3 (price
ceiling), 4 (auto-attach hand-executed buys), 6 (count ETH) and 7 (multiple plans) are
unchanged. Phases 2 and 3 are not started.

