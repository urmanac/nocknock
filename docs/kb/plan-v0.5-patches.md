# Plan: the v0.5.1 and v0.5.2 patches

*Planning document, 2026-11-14. Built from a read-only map of the code: six areas, each
re-checked by an independent skeptic, plus a cross-area critique. Nothing here is
implemented yet. File and line anchors are as of `main` at `828d92c`.*

*Every finding behind this plan, with its evidence, the skeptic's verdict, the proposed
design, the exact test specifications and the open questions, is in
[plan-v0.5-patches-findings.md](plan-v0.5-patches-findings.md), generated from the
workflow. Implementers should read the relevant section there before writing code.
Commit both files if the implementation runs anywhere other than this checkout.*

## Two patches, and why

Earlier today I said not to plan 0.5.2. The code map changed that.

| Patch | Theme | Target | What it touches |
|---|---|---|---|
| **v0.5.1** | Honest numbers | **Deployed before 2026-11-18** | Advice text, totals (#94), dry powder (#63), charts (#61), MCP and dashboard wording |
| **v0.5.2** | From zero | After v0.5.1 | Ingestion and config: price source without a position, `positions: []`, NFT ownership check, neutral example config |

**Why the deadline.** The plan `2026-11-reentry` finishes its last tranche around
2026-11-18. After that, the advice falls back to HOLD, and HOLD still prints the false
"0% realized loss" (#95). Snapshot history shows that text going out live at 05:36Z
today, while LP held $0.

> **What "false HOLD" means.** HOLD is the directive NockNock shows when nothing else
> applies. Its text is a fixed template that asserts things NockNock never checks:
>
> - **"0% realized loss."** Never computed. Your Uniswap app shows −$1,002.58 realized.
> - **"N Uniswap position(s) generating yield."** It counts every tracked position, Aero
>   included, as a yielding Uniswap position.
>
> At 05:36Z today it went out as "4 Uniswap position(s) generating yield (+$0.00
> uncollected). 0% realized loss. Maintain posture." Two of those four positions held
> nothing, and LP as a whole held $0.
>
> While a DCA plan is active, the plan's directive replaces HOLD, so you don't see it.
> When the plan completes, HOLD is back every day. The dashboard and the MCP fallback also
> invent a HOLD of their own when the advice field is missing. v0.5.1 builds HOLD from what
> was actually read, and shows "unavailable" when nothing was.

**Why split.** v0.5.2 changes how config loads, which means migrating chudmax's
`config.yaml`. Yesterday's 8-minute outage came from exactly that kind of change, so it
stays out of the advice patch.

**If time runs short.** PR 1 can ship alone as v0.5.1 before the deadline, since it holds
the HOLD fix. PRs 2–5 would then follow as v0.5.2, and the from-zero ingestion would
become v0.5.3. Patches ship continuously, so nothing is lost by splitting further.

## The contract both patches implement

1. **NockNock states facts and paces the user's own plan.** It never sizes a buy outside a
   plan, and never picks a budget, floor, target weight or reserve.
2. **Unset means "no rule", never someone else's number.**
3. **ETH is gas.** It counts toward the total. It is never cash, and never deployable by
   default.
4. **Nothing uncomputed is presented as fact.** No "0% realized loss", no "All-time high",
   no "LIVE P/L".
5. **Gaps in history are shown, never filled.**

---

## v0.5.1 — Honest numbers

### Step 0: triage (AGENTS.md §11 requires an issue before any fix code)

**File these new defects:**

| # | Defect | Severity | Map ref |
|---|---|---|---|
| N1 | The Portfolio Composition chart and Stable Reserve Delta plot each history row's **oldest** counterfactual point, about 100 cycles (~8 h) stale. `slim_historical_payload` keeps element 0 (`main.rs:2037-2051`). | high | 61-2, eth94-05 |
| N2 | The high-water mark is labelled "All-time high" and the drawdown "LIVE P/L"; `high_water_mark_date` is always the snapshot time (`main.rs:6037`, `App.tsx:367-372, 719-753`) | high | ADV-02, eth94-04 |
| N3 | The tax-ledger panel hardcodes 60/40, draws a 100% bar with no harvests, and always shows "✓ TAX RESERVE PROTECTED" (`App.tsx:720-745`, `main.rs:6325-6331`) | high | ADV-03, 63-E |
| N4 | RANGE EXIT advice reports the whole LP book as the exited range's principal and invents "$5.00+ high walls" (`main.rs:6153-6162`) | medium | ADV-08 |
| N5 | MCP: both formatters hardcode "above 100k" and invent HOLD and $0.00 for missing fields. The stdio zero-DB fallback reads the operator's hardcoded wallet (with malformed calldata) and a dead pool's frozen price (`bin/mcp.rs:19, 337-389`). | high | ADV-06/09/10, FZ-09, 63-G |
| N6 | The Aero pool's unclaimable fees (`claimFees()` reverts) show as "$0 earned", and the claim payload reports an RPC outage | medium | ADV-12 |
| N7 | The dashboard shows "NaN NOCK" for a zero stack, renders floor 0 as UNKNOWN, and its fallbacks invent a HOLD directive (`App.tsx:266-267, 382, 419-420`) | medium | ADV-11, FZ-07 |
| N8 | `GET /v1/snapshot/{id}` returns 500 CORRUPTED_DATA for every stored snapshot: the handler parses the stored payload as an envelope (`main.rs:2108` vs `6422`). Verified live. | medium | FZ-10 |
| N9 | Regime reads "normal" when the daily RSI is unknown; a flat series reads RSI 100; a zero sqrtPrice gives a +∞ spot (`main.rs:6204, 3261, 5060-5067`) | low | FZ-11, FZ-12 |
| N10 | The code's default floor (100,000) and the planless lump-sum dip arm ("Deploy $X … toward 120,000") apply the original operator's numbers to every user (`main.rs:50, 6118-6152`) | high | FZ-03/04, ADV-04/05, 63-C |
| N11 | While a plan is active (any status but complete), RANGE EXIT and BATCH HARVEST can never surface: the plan arm returns first (`main.rs:6135`). A five-day plan hides a range exit for five days. | medium | docs-lore verify |
| N12 | The 60/40 harvest ledger books only USDC and NOCK. WETH fees are skipped: about 62% of the v3 WETH/NOCK range's fees today. NOCK is valued at the spot when recorded, not at the block (`main.rs:3800-3806, 4564-4567`). | medium | advice verify |
| N13 | SWEEP TO STABLE tells you to "sweep to the stable stack until momentum cools" (`main.rs:6130`): an instruction to sell NOCK, not a fact. It outranks the plan whenever the daily RSI is 70 or above. | medium | advice verify |
| N14 | A wallet token balance that comes back null is read as a live 0 (`main.rs:4867-4893`). Wallet USDC is the only input to dry powder. | medium | dry-powder verify |
| N15 | The stacked NOCK chart keys LP series by array position (`App.tsx:108-131`). When the tracked list changes, as it did today, older rows draw under the wrong position and some LP NOCK is dropped. | medium | chart verify |

**Update existing issues:**
- **#61:** current line numbers; "gaps are shown, never backfilled"; the 2.5× threshold.
- **#63:** what is still broken at HEAD (A, B, C, D, F, G).
- **#94:** the allocation-base design below.
- **#95:** that the claim is false, not just unverified. The Uniswap app shows realized −$1,002.58.

### PR 1: advice engine (#95, #63, N4, N9, N10, and the from-zero advice arms)

All in `compute_derived_metrics_with_plan` (`main.rs:5965-6200`).

**Text and sizing fixes:**
- **HOLD (#95):**
  - Build the text from the positions: a Rust port of `positionsSummary`, so stale pools
    count as stale.
  - No P/L claims and no "generating yield". Say "LP fees uncollected".
  - Say "no floor configured" when the floor is 0.
- **Floor (N10):**
  - `default_treasury_floor()` returns 0, meaning no floor. Your `config.yaml` keeps
    100,000, so nothing changes for you.
  - DEFENSIVE LOCK fires only when `floor > 0 && total_nock < floor`.
  - Drop "CRITICAL", "locked" and "sovereign foundation" from its text.
  - With an active plan, the action becomes `DEFENSIVE LOCK (Floor Recovery) · <plan
    action>`, followed by the plan's own instruction. A due tranche then still changes the
    action, so it still notifies. Today the lock hides the plan entirely.
- **No-plan arms (#63-C, N10):**
  - The lump-sum `DIP REINVESTMENT SHOT` becomes **`DIP ZONE (No Buy Plan Configured)`**.
    It states facts only (spot, liquid wallet USDC, the LP USDC that is not dry powder),
    sizes nothing, and drops "120,000". The tax-guardrail arm stays.
  - Two new arms for an unconfigured user. They run only when no plan is set, so you will
    never see them:
    - **`NOTHING TO TRACK YET (Empty Wallet)`**: the wallet holds nothing NockNock reads.
    - **`BUILD FROM ZERO (No Buy Plan Configured)`**: USDC but no NOCK; points to the
      user's own `accumulation_plan`.

    Both add one plain fact when the wallet holds no ETH: every action on Base needs gas.
- **Dry powder (#63-A, D, F):**
  - `deployable = wallet_usdc − max(tax_reserve, cash_target)`, the same rule the plan and
    the rebuild already use.
  - The tax guardrail compares against wallet USDC, not wallet plus LP.
  - HOLD carries a factual dry-powder note.
- **RANGE EXIT (N4):** report the exited range's own value and drop the $5.00 wall.
- **BATCH HARVEST:** the split percentages come from `tax_reserve_ratio`.
- **Code defaults become "no rule" in this PR, not in v0.5.2.** Floor 0, target weights
  optional (with a `no_target` status), no default tax ratio, and an empty
  `tracked_wallets` default.
  - **Why move it here:** the newcomer tests can only show NockNock choosing nothing if the
    code chooses nothing. Otherwise a 500-USDC newcomer's "liquid dry powder" is
    silently computed from a default 5% cash weight.
  - **Your deployment doesn't move:** your `config.yaml` sets every one of these keys, and
    the example config still carries them until v0.5.2.
- **SWEEP TO STABLE (N13)** states facts: the daily RSI, and that the plan pauses buys until
  it cools. It instructs no sale.
- **HOLD's label.** "(Dual Growth Engine Active)" is a claim too: out-of-range positions
  earn nothing. The label states only what the positions show. The "HOLD" prefix stays,
  because `apply_lp_rebuild_bias` relies on it to tell when the engine is idle.
- **DEFENSIVE LOCK's text must not claim NockNock keeps floor NOCK out of LP.** Your
  `lp_floor_tolerance_bps` lets some in.
- **Dry-powder relabel, narrow version.** Remove the balance claims ("available",
  "DEPLOYABLE", the always-on ✓) and the fixed percentages. The 60/40 definition stays as
  OPERATOR_GUIDANCE §7.2 has it. Redefining it would need your yes.
- **The design note changes in the same PR.** `design-note-dca-accumulation-plan.md` §8.2
  recorded "with no plan the existing arm stays", so retiring that arm (D5) updates it,
  along with README.md:69, :72 and the `cash_reserve_usdc` doc comment.
- **Tests assert the whole details text**, including the appended LP-bias sentence, and use
  fixtures that hold some ETH, so #94's shift in the totals is caught.
- **Plan precedence (N11):** RANGE EXIT and BATCH HARVEST outrank a plan that has nothing
  due (waiting, pending, paused or blocked). A due tranche still leads. This is a rule
  change, made under your 2026-11-14 mandate to improve the strategy.
- **What happens to the old constants.** The $0.020 line stays: it is the facts-only DIP
  ZONE trigger and the tax-guardrail gate. The "120,000" goal leaves the text, because the
  arm no longer sizes anything toward it. OPERATOR_GUIDANCE §5 says to move magic numbers
  into config rather than delete them. Doing that for both constants is the deferred
  "externalise legacy constants" item (v0.6.0). Until then, the $0.020 trigger stays in
  code.

**Status and math fixes:**
- **LP bias:** with no tracked positions, `lp_rebuild.status = "no_positions"` and no
  bias text.
- **Deviation weights:** guard every division by the total.
- **N9:** regime `"unknown"` when there is no daily RSI; RSI is undefined (not 100) on a
  flat series; the spot must be finite.

**Tests: your from-zero test cases.** These are the test case you asked for: a newcomer with
an empty portfolio asking for shot advice. They run at engine level, so they need no
ingestion changes.

| Test | Scenario | Expected result |
|---|---|---|
| R1 | 500 USDC, no plan | `BUILD FROM ZERO`; never "Deploy $", "120,000", "CRITICAL" or "Standing bias" |
| R2 | Empty wallet | `NOTHING TO TRACK YET` |
| R3 | The user's own plan | `DCA TRANCHE DUE (1 of 5)`, sized only by their 100/5 |
| R4 | After their first fill | `DCA ON SCHEDULE (2 of 5 next)` with their average cost |
| R5 | Their own 100k floor | `DEFENSIVE LOCK · DCA TRANCHE DUE` while the plan stays visible, and the transition notifies |
| R6 | Their own floor, no plan | Points to a plan; sizes nothing |
| R7 | The planless dip zone | States facts only |

R16 and R17 cover the regime and math edges. Four existing tests get updated:
- `defensive_lock_under_100k` and `floor_tolerance…` set the floor explicitly;
- `tax_guardrail_blocks_dip_deployment` and `advice_plan_replaces_lump_sum_dip_shot`
  assert DIP ZONE.

### PR 2: the gas pillar (#94, N2 backend; snapshot spec 2.3.0)

- **Count wallet gas in the total.**
  - The wallet ETH and WETH reads become required: a failed read fails the cycle, as the
    NOCK and USDC reads do.
  - ETH and WETH appear in `wallet_balances`.
  - `total_portfolio_usd` includes wallet gas.
  - New `global_status` fields: `eth_price_usd`, `wallet_gas_eth`, `wallet_gas_usd`,
    `total_eth_amount` and `total_eth_usd` (the pillar is wallet gas plus the WETH inside
    LP), and `allocation_base_usd`.
- **One definition of the portfolio: the full total, gas included.** Targets, weights, the
  high-water mark and history all use it.
  - **Why not a total that excludes wallet gas?** I first proposed that, and a skeptic
    showed it was wrong. That base would include ETH inside LP but not ETH in the wallet.
    Depositing gas into a WETH/NOCK range would then register as a gain, and withdrawing it
    as a drawdown: #94's bug moved into the high-water mark.
  - **The cost** is a one-time step of about +$23 (+1%) when gas starts counting. It is
    noted in the CHANGELOG and marked on the chart.
  - **ETH gets no target and no rebalancing pressure.** The deviation matrix gains a "Gas"
    row with status `no_target` in a neutral colour. The other three weights each drop by
    about 1%.
  - **The dashboard shows ETH as a pillar line, never as a "Goal" pill.**
- **High-water mark query fixes:**
  - `high_water_mark_date` is the date of the peak row (N2), with ties broken by the
    earliest row.
  - An empty table, the sanctioned fresh start, uses `fetch_optional`.
  - A database error no longer silently resets the mark to the current value
    (`.unwrap_or(0.0)` today).
- **ETH/USD gets the same stale-pool guard as the NOCK spot.** Once wallet gas is valued in
  the total, a frozen WETH/USDC pool would be a mock number.
- **LP rebuild never funds an ETH leg from wallet gas.**
  - Positions with an ETH or WETH leg become ineligible, with the reason stated.
  - Their share is redistributed to the USDC/NOCK positions, so the target can still be met
    instead of sitting under target forever.
  - NockNock's own `REBUILD_GAS_BUFFER_ETH` (0.002) constant is removed.
  - A `gas_reserve_eth` setting with your own number would be a new config feature, so it
    waits for v0.6.0, and only if you want it.
- **Every wallet token balance must be present (N14).** A missing USDC or NOCK balance fails
  the cycle instead of reading as 0.
- **The gas exclusion gets its own reason.** Feeding the rebuild 0 ETH would produce "wallet
  holds 0.00000 ETH" while it holds 0.0125. Update the rebuild tests that expect WETH/NOCK
  suggestions (`main.rs:7461-7540`).
- **Snapshot spec bump call sites:** `main.rs:2660`, the stale `'2.0.0'` pinned in
  `freshness.contract.test.ts:194`, and the CHANGELOG line that still lists #94 as
  deferred.
- **Remove the unused $2,500 ETH price default** (`main.rs:509`), which is mock data
  waiting to leak.
- **Make native + stable + ETH = total.**
  - `total_native_usd` stays NOCK-only, and the spec text is corrected to say so.
  - The composition panel stops labelling LP WETH as USDC.
- **Pillar lines in both MCP formatters:** "NOCK $· USDC $· ETH $", plus a "Gas held …
  counted in the total; not cash, not deployable" line.
- **Live effect today:**
  - The total rises by your wallet gas, about $23 on 0.0125 ETH.
  - The targets grow about 1%: the LP target goes from $478.12 to $482.71, and the cash
    target from $119.53 to $120.68.
  - The ETH pillar reads about $67: $23 of wallet gas plus $44 of WETH in the v3 range.

### PR 3: charts (#61, N1)

- **Break the lines at gaps.** Add `frontend/src/timeline.ts` with a `withGapBreaks()`
  helper that inserts a break row when consecutive snapshots are more than **2.5× the sync
  window** apart. Recharts 2.15 already breaks lines and stacked areas at such a row and
  hides the tooltip there. A single missed cycle (476–529 s, seen twice today) stays
  joined.
- **Space the charts by real time.** All four time charts get a numeric time axis and a
  tooltip label formatter.
- **Send the real cadence.** `GET /v1/snapshot/historical` includes `sync_window_seconds`
  (the dashboard hardcodes 300 today).
- **Fix the stale chart (N1).** Build each row's counterfactual from that row's own totals,
  and make `slim_historical_payload` keep the latest point.
  - The "stable" delta uses `total_stable_usd`, and ETH is its own line. Total minus native
    also contains LP WETH today, and would contain wallet gas after #94.
  - The Stable Reserve Delta readout reads the latest snapshot directly, not the last
    history row.
- **Make gaps visible, not just broken lines.**
  - Mark each gap with a shaded area or a caption reading "no snapshots for 19h 20m". On
    its own, a break row would hide that text: the tooltip is suppressed there.
  - Take axis ticks from real rows only.
  - Draw a dot for any real row with gaps on both sides, or it vanishes.
- **One parser everywhere:** a finite number or null. `parseFloat(x || '0')` today draws a
  missing value as a real 0.
- **Key LP series by `position_id`, not array position (N15).**
- **Move the dataset builders out of `App.tsx`** into `timeline.ts` (`buildChartRows`).
  `App.tsx` mounts the app on import, so vitest can't reach the builders where they are.
  Then test them on 0, 1 and 2 rows, zero balances and a gap.

### PR 4: surfaces (N3, N5, N6, N7, N8, first-run errors)

- **MCP (N5):**
  - Take the floor from `nock_floor_target` and print "no floor configured" when it is 0.
  - Print "unavailable" for missing fields, never HOLD or $0.00.
  - State the harvest rule honestly.
  - Split USDC into wallet and inside-LP.
  - The zero-DB fallback states only that the worker is unreachable, and deletes
    `WALLET_ADDR`.
- **Aero fees (N6):** mark the pool's fees "not LP-claimable (`claimFees()` reverts); AERO
  rewards not read (#93)", never $0.
- **Dashboard (N2, N3, N7):**
  - Pure helpers `statValueText`, `floorStatus` and `regimeLabel`.
  - Fallbacks render "—".
  - The high-water mark card says "Highest recorded by this database". The dial says
    "Change from recorded peak, not profit or loss (NockNock does not track cost basis)".
  - Tax dial labels come from the data.
- **Snapshot by id (N8):** parse the stored `SnapshotData` and build the envelope from the
  row.
- **First-run errors:** the 503 `NO_SNAPSHOT` body carries `detail: last_ingest_error`, so
  the dashboard and MCP show the real cause.
- **More MCP honesty:**
  - Both transports label any snapshot "Live API" today. Print the snapshot's time and the
    worker's health instead.
  - The harvest rule text names the precedence: harvest is advised only when no tranche is
    due.
  - Fix `mcp.rs` in place. A formatter module shared by both binaries would mean
    restructuring the crate (there is no `lib.rs`), which is too much for a patch.
- **BATCH HARVEST text says what is actually booked (N12):** USDC and NOCK at the spot when
  recorded; WETH fees not booked yet. Fixing the ledger itself is v0.5.2.
- **Dashboard floor sign and rounding:** "above floor" is printed for a negative delta, and
  small floors are rounded (`App.tsx:265, 336, 344`).

### PR 5: docs

- **Lore:** finalize `docs/lore/2026-11-14-four-pillars-no-hurry.md`. Corrections from
  review are already applied: the plan *paces* buys we make, the remaining plan is $80
  not $100, the "read from the chain" wording is now precise, and plan progress after a
  database wipe can be restored by re-recording buys.
- **OPERATOR_GUIDANCE §7:**
  - Add 7.11 Four pillars, 7.12 Gas is not cash, 7.13 The wallet is the record (a fresh
    start; nothing called a backup), and 7.14 Building in public (the from-zero contract).
  - Reword the AERO item in pillar terms.
- **README §1–2:**
  - Remove the retired portfolio.
  - Say the example numbers are one operator's, not recommendations.
  - List the current directive precedence.
  - Drop the unverifiable "sovereign voting weight" claim.
  - Fix the Quick Start.
- **ONCHAIN_INTEGRATION:** dated "superseded in v0.5.0" notes on its three now-false
  statements.
- **HANDOFF.md:** a banner marking it historical (v0.2.1). AGENTS.md §10 points to the lore
  for philosophy.
- **DEPLOYMENT.md:** the nightly dump is not a backup (per D11).
- **AGENTS.md §9:** test counts.
- **CHANGELOG `[0.5.1]`.**

### Release v0.5.1

1. Merge the PRs with CI green.
2. Tag `v0.5.1`, with your go-ahead.
3. Deploy the tag to chudmax.
4. Verify:
   - `/health` is `ok`;
   - the snapshot version is 2.3.0;
   - the total includes wallet gas;
   - HOLD and DIP text contain no invented figures;
   - `NOCKNOCK_API_URL=http://chudmax:8081 make test-mcp` passes.

**Test counts must go up:** worker from 91 to roughly 115, frontend from 45 to roughly 60.

---

## v0.5.2 — From zero (ingestion and config)

### PR 6: run with no LP

- **Price source without a position:**
  - Add an optional `price_source:` block (v4 `pool_id` with its currencies, or a v3 or
    Aero `pool_address`).
  - `fetch_price_source_pool()` makes pool-level reads only and returns a probe that never
    enters `raw.positions`.
  - The existing stale-price guard applies unchanged.
- **Allow `positions: []`:** delete the fatal "No positions configured" check
  (`main.rs:4805`). A pure `spot_source()` picks the spot: a flagged tracked position
  first, then the block.
- **Check NFT ownership:** `ownerOf` on the v4 POSM or the v3 NPM before reading a
  position. If the wallet doesn't own it, fail with a clear message.
  - **Why it matters:** correctness, not privacy (your wallet being public is fine with
    you). A newcomer who copies the example would see your holdings counted as theirs.
  - **First confirm on chain** that #3163450 and #6336662 are owned by your wallet.
  - **Gauge custody counts as ownership.** A staked NFT is owned by its gauge. Your Aero
    CL NFT #2914490 is, and #91 will track it.
- **Fail clearly at startup:**
  - An empty `tracked_wallets` is an explicit error (today it falls back to your wallet,
    `main.rs:252`).
  - A placeholder Alchemy key is an explicit error at the top of the cycle.
- **Tests:** R8 to R13, and R15.

### PR 7: neutral example config (a migration; order matters)

1. ~~Confirm chudmax's `config.yaml` sets every key itself.~~ **Done:** it sets
   `tracked_wallets`, `positions`, the full `operator_profile`, `market_data` and
   `accumulation_plan`. Names checked only, no values printed.
2. **Clean the example.** By now the code defaults already choose nothing: PR 1 changed
   them, and PR 4 removed `WALLET_ADDR` from `bin/mcp.rs`. What's left is the base layer:
   - It keeps only market-wide infrastructure: token addresses, `eth_price_pool`,
     `market_data`, the sync window, the port and a default `price_source`.
   - Your wallet, positions and profile become commented documentation.
   - Target weights are left unset, so they read `no_target`. Zeros would not work: the
     tolerance rule (`main.rs:6272-6280`) would mark every held bucket over-funded, in red.
3. **Deploy and verify** that chudmax still shows 3 positions, floor 100,000 and targets
   7500/2000/500.
4. **Pin the base layer.** R18 asserts the example carries no operator identity.

### PR 8: the end-to-end newcomer playtest

Run locally with:
- an empty Postgres,
- a freshly generated read-only address,
- `positions: []`,
- a `price_source` block.

Expected: the first cycle succeeds, the advice reads `NOTHING TO TRACK YET`, and nothing in
any view mentions `0xeE11…`. Also check what `alchemy_getTokenBalances` returns for a
never-funded wallet, and fix whatever falls out.

### Also in v0.5.2

- **63-H:** a failed harvest-ledger read silently zeroes the tax reserve. It should show as
  unknown.
- **N12:** book WETH fee income in the harvest ledger, and value NOCK at its block.
- **With no floor,** rebuild NOCK funding is the whole wallet stack, bounded by the user's
  own LP target. With no target set, there are no rebuild suggestions at all.
- **Two example-config traps:** copying its notifications section silently disables
  Pushover (`config.yaml.example:174`), and R18 must load the shipped example (via
  `CARGO_MANIFEST_DIR`), not a synthetic one.
- **Pin the TimescaleDB image by digest** instead of `latest-pg16`.
- **A "Starting from zero" section** in the README and the lore, including the fresh-start
  recovery (re-record buys and harvests from their transaction hashes).
- **Fix DEPLOYMENT.md.** Its documented config path crashes the worker, and the compose and
  cloud images bake `config.yaml` in.
- **Acknowledge the original strategy in the lore.**
  `assets/nock_portfolio_strategy.pdf`, marked confidential but in the public repo, talks
  about lowering cost basis. The lore should say that its "no hurry" position supersedes
  that.

---

## Decisions for you

**Bold** marks my recommendation. I'll proceed with it unless you say otherwise.

| | Question | Options |
|---|---|---|
| D1 | Should LP rebuild suggestions spend wallet ETH on WETH/NOCK legs? | **No.** WETH/NOCK positions are skipped and their share goes to the USDC/NOCK positions; a `gas_reserve_eth` setting can come in v0.6.0 if you want it · keep today's 0.002 ETH buffer |
| D2 | Does the ETH pillar include ETH inside LP (as LP USDC counts as cash)? | **Yes**, with "gas" meaning wallet ETH only · wallet only |
| D3 | What is the high-water mark measured on? | **The full total, gas included**, with a one-time +$23 step that is marked on the chart (changed after review: excluding gas would turn ETH moves into fake gains and drawdowns) · value excluding wallet gas |
| D4 | Default floor for a new user? | **None** (yours stays 100,000 in your config) · keep 100,000 |
| D5 | With no plan, in the dip zone? | **Facts-only `DIP ZONE`** · nothing at all |
| D6 | Default target weights for a new user? (v0.5.2) | **None** (yours stay 7500/2000/500) · keep 80/15/5 |
| D7 | Directive names | **`NOTHING TO TRACK YET (Empty Wallet)`, `BUILD FROM ZERO (No Buy Plan Configured)`, `DIP ZONE (No Buy Plan Configured)`** · your wording |
| D8 | Example `price_source` (v0.5.2) | **The v4 3% USDC/NOCK pool** (today's live source) · the legacy Aerodrome vAMM |
| D9 | Chart gap threshold | **2.5×** (single missed cycles stay joined) · 1.5× (every miss shows) |
| D10 | Snapshot spec version for the new fields and enums | **2.3.0** (additive) |
| D11 | The nightly `pg_dump` timer on chudmax | **Remove the timer, its script and the dumps** (needs your OK: it deletes the dumps) · rename it to "nightly dump" and keep it |
| D12 | ~~The lore publishes your Uniswap loss figures in a public repo~~ | **Settled:** not a concern (you, 2026-11-14: yebyen.eth points at you; anyone can find you) |
| D13 | Is the tax reserve backed by wallet USDC only? | **Yes**, matching the plan and the rebuild |
| D14 | Default `tax_reserve_ratio` for a new user (v0.5.2) | **None** (no virtual reserve until set; yours stays 0.40) · keep 0.40 |

**On D11:** removing the timer takes more than deleting files from the repo.
`deploy.sh` wipes `~/nocknock/src` on every deploy, and the timer's script lives there.
Delete the files alone and the still-enabled timer fails every night. So disable the timer
on chudmax first (`systemctl --user disable --now nocknock-backup.timer`), remove its unit
files, and only then drop the files and the Makefile's `db-backup`/`db-restore` targets
from the repo.

## Open points from the cross-area critique

The critic arrived after this plan was written. Most of its 18 conflicts are areas proposing
different wording for the same string, and the sections above already pick one. Two
points remain open:

- **Default target weights still size things for a newcomer (critic V1, V4).**
  - PR 1 makes the code default floor 0. The default weights stay 8000/1500/500 until the
    targets become optional.
  - A newcomer's "liquid dry powder" and LP target therefore still come from a 5% cash
    weight and a 15% LP weight that they never set.
  - With no floor, LP rebuild funding can draw on their whole NOCK stack, sized from that
    default 15%.
  - **Rule to implement:** rebuild suggestions need an LP target the user set, and dry
    powder subtracts only a cash target the user set. Until targets are optional, the
    newcomer tests should assert the weights the test itself sets, not the defaults.
- **The $0.020 line (critic V2).** It is the original operator's legacy constant, applied
  to everyone. While it remains a trigger, DIP ZONE's text calls it "the fixed $0.020
  line", never a market fact. Moving it into config with no default is the v0.6.0
  "externalise legacy constants" item.

## Out of scope for both patches

- **Directive renames** such as dropping "Dual Growth Engine" (ADV-14): v0.6.0.
- **Recording `observed_through` on deduplicated cycles**, so quiet wallets don't show
  false gaps (61-5): a follow-up issue. The same root cause makes a healthy worker look
  stale on the dashboard while nothing changes.
- **RSI(5M) and the Bollinger SMA(20) use the last N stored rows** regardless of time gaps:
  a follow-up issue.
- **A configurable $5.00 harvest threshold:** #53.
- **Cost basis and average cost against spot:** #98.
- **Aero CL positions (#91) and AERO rewards (#93):** v0.6.0.
- **Mining:** see `docs/kb/plan-l1-mining.md`. Nothing until Kingdon agrees.
