# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
- **Verified on Base** (ONCHAIN_INTEGRATION.md §8): tests run on real receipts. The
  plan's three buys qualify; four Uniswap fee claims and two Aero deposits that returned
  a NOCK refund do not.

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
