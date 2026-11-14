# Appendix: verified findings behind the v0.5 patch plan

*Generated 2026-11-14 from the `v051-understand` workflow: one read-only mapper per area, then an independent skeptic per area, then a cross-area critic. This is raw material for whoever implements [plan-v0.5-patches.md](plan-v0.5-patches.md). Line numbers are as of `main` at `828d92c`. Where a skeptic marked a finding "partly" or "refuted", the skeptic's note wins.*


## eth-pillar: Issue #94: wallet ETH/WETH as the "gas" pillar (totals, targets, HWM/history, spec, frontend, MCP)

**Summary.** Wallet ETH and WETH are read every cycle (main.rs:4911-4932), but they only feed LP-rebuild funding (6096-6099). total_portfolio_usd = lp_usd + t3_usd + wallet_usdc (6029), so ETH counts only once it sits inside a tracked LP position. ETH/USD comes from the configured v3 WETH/USDC pool, and a failed read fails the cycle (4741-4774). RawOnChainData is never deserialized (the only from_value calls are for SnapshotData/SnapshotEnvelope at 1672/1873/2007/2108). So neither the serde default 2500.0 (375, 509-511) nor the derived Default 0.0 reaches any valuation, gas estimate, snapshot field or displayed number today. The $2,500 constant is still dead mock code and should be deleted.

Live snapshot at 18:11Z (health ok, last_error null): total $2,390.58 = NOCK $1,312.88 + USDC $1,033.71 + $43.99 of LP WETH (0.023918 WETH in v3 #6336662). The implied ETH/USD is about $1,839.20 (range $1,838.95-1,839.45). The snapshot does not expose wallet ETH or WETH at all; wallet_balances lists only NOCK and USDC. Using the reported ~0.0125 ETH of gas (about $22.99), the total becomes about $2,413.57 and the ETH pillar about $66.98. The Aero CL WETH/NOCK position (#91, 0.081995 ETH deposited) is still untracked.

Recommended shape:
- Count wallet gas in total_portfolio_usd.
- Measure the cash target, the LP target, the deviation weights and the high-water mark (HWM) and history against allocation_base_usd = total minus wallet gas. That base is exactly the pre-2.3.0 total, so history stays continuous and today's -3.42% drawdown stays visible. If gas were added to the base, the weights would move to 4304/1744/3856 bps plus 95 bps of untargeted gas, the LP target from $478.12 to $482.71, and P/L vs HWM from -3.42% to -2.49% from the definition change alone.
- Never fund LP deposits from gas by default.
- Make both gas reads required.
- Expose eth_price_usd, wallet_gas_eth, wallet_gas_usd, total_eth_amount, total_eth_usd and allocation_base_usd (snapshot 2.3.0).

Also found:
- high_water_mark_date is always the snapshot time.
- Historical counterfactual points are about 8 hours stale; confirmed against live data.
- The composition panel labels LP WETH as USDC.
- An adjacent hardcoded "0% realized loss" claim.


### eth94-01: Wallet ETH/WETH is read but excluded from total_portfolio_usd; ETH counts only once inside a tracked LP — skeptic: **confirmed**

- **Kind / severity / patch:** defect / high / yes
- **Locations:** `ingestion-worker/src/main.rs:381-386`, `ingestion-worker/src/main.rs:4911-4932`, `ingestion-worker/src/main.rs:4963-4980`, `ingestion-worker/src/main.rs:6029`, `ingestion-worker/src/main.rs:6096-6099`
- **Evidence:** Doc comment at 381-382: "Used only to check that a suggested deposit can be funded — not counted in portfolio totals". 6029: `let total_portfolio_usd = lp_usd + t3_usd + wallet_usdc;`. The balances' only consumer is RebuildFunds.eth (6096-6099). wallet_balances emits only NOCK and USDC (4963-4980), so the snapshot never shows wallet ETH. Both reads are best effort: `.map(|wei| wei as f64 / 1e18)` yields None on failure and the cycle continues. Live 18:11:04Z: total 2390.58 = native 1312.88 + stable 1033.71 + 43.99, the last being LP WETH (0.023918 WETH in #6336662); wallet ~0.0125 ETH (about $22.99) is absent. History (GET /v1/snapshot/historical?limit=400): the 04:38:29Z row has no LP ETH legs. The 15:23:43Z row shows 0.020357 WETH in LP, so ETH moved from wallet to LP entered the tracked total.
- **Impact:** Moving ETH between the wallet and a tracked LP changes the total, the HWM and every history series although nothing was gained or lost. The gas pillar is invisible in the API, the UI and MCP. If gas were added naively with the current best-effort reads, a failed read would silently drop it from the total, a fabricated dip.
- **Proposed fix:** Count wallet gas in the total: total_portfolio_usd = allocation_base_usd + wallet_gas_usd, where wallet_gas_usd = (wallet_eth + wallet_weth) x live_eth_price_usd. Make both reads required, returning Err like the NOCK/USDC read (4891-4893) and the ETH price read (4769-4774), so the gas is never counted as zero. Append WalletBalance rows for ETH (token_address 0x000...0, amount {:.6}) and WETH (WETH_BASE) after NOCK and USDC so indexes 0 and 1 stay put. Update the 381-382 comment. Add the global_status fields listed in proposed_design.

### eth94-02: Target math uses total_portfolio_usd as its base; counting gas there would put rebalancing pressure on ETH — skeptic: **partly**

- **Kind / severity / patch:** design / medium / yes
- **Locations:** `ingestion-worker/src/main.rs:6055`, `ingestion-worker/src/main.rs:6078-6080`, `ingestion-worker/src/main.rs:6284-6291`, `ingestion-worker/src/main.rs:1308-1337`, `ingestion-worker/src/main.rs:1348-1352`, `specs/core/snapshot.yaml:787-789`, `frontend/src/App.tsx:183-189`
- **Evidence:** 6055: `cash_target_usd = total_portfolio_usd * (cash/10000)`. 6078: `lp_rebuild_status(lp_usd, total_portfolio_usd, ...)`. 6080: `lp_target_usd = total_portfolio_usd * (uniswap_lp/10000)`. 6284-6290: `t3_usd / total_portfolio_usd`, `lp_usd / ...`, `wallet_usdc / ...`. The spec says target weights sum to 10000. Live weights are 4345/1760/3893 bps (targets 7500/2000/500). With gas in the denominator they become 4304/1744/3856 plus 95 bps of untargeted gas. The LP target would move from $478.12 to $482.71 and the cash target from $119.53 to $120.68.
- **Impact:** The three targeted buckets could only reach 10000 minus the gas share, so every cycle would read as slightly under-funded until gas is spent. LP and cash targets would rise with the ETH price. Gas would raise how much USDC must stay liquid. All of this contradicts the decision that ETH is not cash, not in the reserve and not under rebalancing pressure.
- **Proposed fix:** Introduce allocation_base_usd = lp_usd + t3_usd + wallet_usdc (identical to today's total) and use it at 6055, 6078, 6080 and 6284-6291, guarding allocation_base_usd > 0. Add no gas row to the deviation matrix: a target of 0 with a positive balance would read over_funded. Change the 1349 wording '% of portfolio' to '% of the portfolio excluding gas'. PlanGuards (6058-6067), deployable_usdc (6050-6056) and the tax guardrail (6051) already use USDC only and need no change beyond the cash target's base.
- **Skeptic's correction:** Code facts and arithmetic are verified: 6055, 6078, 6080 and 6284-6291 use total_portfolio_usd. Live weights are 4345/1760/3893. With $22.99 of gas in the base they become 4304/1744/3856 plus 95 bps; the LP target goes 478.12 -> 482.71 and the cash target 119.53 -> 120.68. The consequence is overstated. There is no ETH bucket, so a gas-inclusive denominator puts no 'rebalancing pressure on ETH'. It lowers all three weights by at most ~41 bps and makes the USDC and LP targets scale with the ETH price. At today's values no status flips: LP 1744 stays within 2000±400, and lp_rebuild's under_target line is 1600. The proposed separate base is still the right fix.

### eth94-03: LP rebuild treats wallet ETH/WETH above a NockNock-chosen 0.002 ETH buffer as deployable — skeptic: **confirmed**

- **Kind / severity / patch:** design / medium / yes
- **Locations:** `ingestion-worker/src/main.rs:708-727`, `ingestion-worker/src/main.rs:945-952`, `ingestion-worker/src/main.rs:6096-6099`, `ingestion-worker/src/main.rs:1348-1349`
- **Evidence:** 727: `const REBUILD_GAS_BUFFER_ETH: f64 = 0.002;`. 6098: `Some(e.unwrap_or(0.0) + w.unwrap_or(0.0) - REBUILD_GAS_BUFFER_ETH)`. 948: reason text "wallet holds {:.5} ETH/WETH after a {} ETH gas buffer". The advice sentence at 1349 says ranges are rebuilt "from NOCK and USDC already held", but WETH/ETH legs are funded from wallet gas. Live lp_rebuild is on_target (gap $57.16), so there is no effect today. An under_target cycle could suggest moving about 0.0105 of the ~0.0125 ETH into WETH/NOCK.
- **Impact:** Contradicts today's decision that gas is not deployable or investable capital. The 0.002 ETH buffer is a floor number chosen by NockNock, which the standing order forbids.
- **Proposed fix:** By default, gas is not a funding source. An ETH/WETH leg goes to lp_rebuild.excluded with the reason 'the ETH leg would come from wallet gas, which NockNock does not deploy (issue #94)', optionally stating the ETH-per-$ mechanics. Remove REBUILD_GAS_BUFFER_ETH. If the operator wants ETH-leg suggestions back, add an operator-set operator_profile.gas_reserve_eth (unset means never; only ETH above it is fundable). This is an open question.

### eth94-04: HWM and history read the total on a basis that changes with #94; the HWM date is always the snapshot time — skeptic: **confirmed**

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `ingestion-worker/src/main.rs:2458-2463`, `ingestion-worker/src/main.rs:2467-2472`, `ingestion-worker/src/main.rs:2510-2523`, `ingestion-worker/src/main.rs:6037`, `ingestion-worker/src/main.rs:6196-6197`, `ingestion-worker/src/main.rs:6233-6235`, `ingestion-worker/src/main.rs:6319-6324`, `frontend/src/App.tsx:367-372`, `frontend/src/App.tsx:727-729`, `specs/core/snapshot.yaml:628-634`
- **Evidence:** 2459: `SELECT COALESCE(MAX((...->>'total_portfolio_usd')::float8), 0.0) FROM snapshots`. The Bollinger SQL (2468) and the counterfactual SQL (2513) read the same field. 6197: `high_water_mark_date: now_str.clone()`, while the spec says 'When the high-water mark was set'. Live: HWM $2,475.27 equals the 04:38:29Z row, the first stored row: the liquid wallet that, per fixtures 6993-7006 and 7379-7388, also held ~0.114 uncounted ETH. The API nevertheless reports high_water_mark_date 2026-11-14T18:11:04Z. The UI labels the HWM 'All-time high' (App.tsx:371).
- **Impact:** Putting gas into total_portfolio_usd would step every series. P/L vs HWM would move from -3.42% to -2.49% from the definition change alone, flattering the drawdown. The HWM date is a fabricated 'now'.
- **Proposed fix:** Measure the HWM and history on allocation_base_usd. The SQL reads COALESCE(gs->>'allocation_base_usd', gs->>'total_portfolio_usd'): pre-2.3.0 totals are exactly that basis, so no restatement or step occurs. Return the max row's generated_at as high_water_mark_date. Use the same COALESCE in the Bollinger SQL (2468) and the counterfactual SQL (2513). Pass allocation_base_usd at 6037, 6233 and 6321. In the frontend, compute P/L as allocation_base_usd ?? total_portfolio_usd. Label the card 'High-water mark (excludes wallet gas) · set <date>' instead of 'All-time high'.

### eth94-05: Historical counterfactual points are ~100 snapshots (~8 h) old: slim_historical_payload keeps the oldest element — skeptic: **confirmed**

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `ingestion-worker/src/main.rs:2043-2047`, `ingestion-worker/src/main.rs:5917-5942`, `frontend/src/App.tsx:168-181`, `frontend/src/App.tsx:695-707`
- **Evidence:** compute_counterfactual_sync writes the history oldest first and appends the current point last (5927-5937). slim_historical_payload does `active_portfolio_series.truncate(1)` (2045-2047), keeping element [0], the oldest. The frontend plots `active_portfolio_series?.[0]` at each row's own time. Live GET /v1/snapshot/historical?limit=3: the row at 18:16:04Z has total $2,390.50, but its counterfactual[0] is 10:20:57Z, $2,474.90 (native $1,441.90).
- **Impact:** The Portfolio Composition chart, and the 'Stable Reserve Delta' summary built from the last row's [0], are about 8 hours stale. The #94 change would show up there about 8 hours late.
- **Proposed fix:** Keep the snapshot's own point instead: for each series, `let n = v.len(); v.drain(..n.saturating_sub(1));`. Rows are slimmed at read time, so all stored history is fixed at once.

### eth94-06: Composition panel shows LP WETH (and after #94 gas) as 'the USDC side'; spec says total_native_usd includes ETH but code is NOCK-only — skeptic: **confirmed**

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `frontend/src/App.tsx:679`, `frontend/src/App.tsx:690-691`, `frontend/src/App.tsx:704`, `ingestion-worker/src/main.rs:6030-6031`, `specs/core/snapshot.yaml:613-618`
- **Evidence:** Panel title 'Total (Native + USDC) vs Native-Only'; its delta is labelled 'the USDC side held alongside LP positions'. Code: `total_native_usd = (lp_nock + t3_nock) * spot_price`, while the spec says 'Value of native tokens (NOCK, ETH, etc.)'. Live: 2,390.58 - 1,312.88 = $1,077.70 is presented as USDC, but total_stable_usd is $1,033.71, so $43.99 of WETH is labelled as cash. After #94 it would be about $66.98.
- **Impact:** ETH is presented as cash, the opposite of the operator's decision.
- **Proposed fix:** Keep total_native_usd NOCK-only and fix the spec text. Add total_eth_usd so that native + stable + eth = total. Retitle the panel 'Portfolio excl. gas vs NOCK-only' and split the delta into USDC (total_stable_usd) and ETH in LP (total_eth_usd - wallet_gas_usd).

### eth94-07: Fabricated $2,500 ETH price survives as a serde default (currently unreachable) — skeptic: **confirmed**

- **Kind / severity / patch:** risk / low / yes
- **Locations:** `ingestion-worker/src/main.rs:360`, `ingestion-worker/src/main.rs:375-376`, `ingestion-worker/src/main.rs:509-511`, `TRIAGE.md:121`
- **Evidence:** `#[serde(default = "default_eth_price")] live_eth_price_usd: f64` and `fn default_eth_price() -> f64 { 2500.0 }`. RawOnChainData is built only by fetch_raw_data (4962), where the price starts at 0.0 and returns Err if the pool read fails (4751-4774); it is never deserialized. The claim path degrades to None (3179-3206, 4270). The derived Default gives 0.0, so in tests ETH legs are worth $0 (leg_usd_price). TRIAGE E1 already removed the same $2,500 fallback from the claim path.
- **Impact:** No production number is affected today, but any future deserialization (cached raw reads, replay tests) would silently value ETH at $2,500.
- **Proposed fix:** Delete default_eth_price and the attribute so the field is required. Guard the new gas math with eth_px > 0: an unknown price makes the gas fields null and leaves the total on the allocation base, never valuing gas at $0.

### eth94-08: ETH/USD is not in the snapshot: the frontend reverse-engineers it from rounded strings; the claim handler ignores the eth_price_pool config — skeptic: **confirmed**

- **Kind / severity / patch:** defect / low / yes
- **Locations:** `frontend/src/App.tsx:191-205`, `ingestion-worker/src/main.rs:5144-5175`, `ingestion-worker/src/main.rs:3181-3190`, `ingestion-worker/src/main.rs:4741`, `ingestion-worker/src/main.rs:4270-4271`
- **Evidence:** App.tsx computes `usd / amt` from reserve_usd ({:.2}) divided by reserve_amount ({:.6}). A 0.000004 WETH dust leg reads "0.01", giving $2,500/ETH. fetch_eth_price_usd hardcodes `"to": DEFAULT_ETH_PRICE_POOL`, while the snapshot cycle uses `state.config.eth_price_pool` (4741), and the claim handler pairs the snapshot's NOCK spot with this separate ETH read.
- **Impact:** ETH brackets get a wrong USD conversion when a leg is dust. An override of eth_price_pool creates two ETH prices. A user with no ETH pool gets no ETH price at all, yet gas still needs valuing.
- **Proposed fix:** Emit global_status.eth_price_usd. The frontend uses it, with the pool-derived fallback only for pre-2.3.0 snapshots. The claim handler reads the snapshot's eth_price_usd, as it already does for NOCK via nock_spot_from_snapshot, or else the config pool.

### eth94-09: MCP portfolio text has no gas or pillar lines, hardcodes 'above 100k' and prints $0.00 for missing fields — skeptic: **confirmed**

- **Kind / severity / patch:** defect / low / yes
- **Locations:** `ingestion-worker/src/main.rs:2136-2157`, `ingestion-worker/src/main.rs:8347-8350`, `ingestion-worker/src/bin/mcp.rs:240-261`
- **Evidence:** `(Floor buffer delta: {delta_prefix}{nock_delta} above 100k)` and `.unwrap_or("0.00")` for total and HWM. The floor is configurable (nock_floor_target). The test at 8349 asserts "+6140 above 100k". bin/mcp.rs duplicates the same formatter.
- **Impact:** AI agents never learn that gas exists or that the total includes it. A configured floor other than 100k is misreported, and missing fields read as $0.00.
- **Proposed fix:** In both formatters, add a 'Pillars: NOCK $native · USDC $stable · ETH $eth' line and a 'Gas held (wallet ETH+WETH): X ETH ($Y), counted in the total; not cash, not deployable' line. Print 'not reported (snapshot predates 2.3.0)' when the fields are absent. Take the floor from nock_floor_target. Label the HWM line 'excludes wallet gas, set <date>'.

### eth94-10: From zero: weights divide by the total without a guard; a gas-only wallet makes the allocation base 0 — skeptic: **confirmed**

- **Kind / severity / patch:** gap / low / yes
- **Locations:** `ingestion-worker/src/main.rs:6284-6291`, `ingestion-worker/src/main.rs:1310-1318`, `frontend/src/App.tsx:656`
- **Evidence:** `((t3_usd / total_portfolio_usd) * 10000.0) as i32`: 0/0 is NaN and the saturating cast turns it into 0, so all three buckets read under_funded by accident. lp_rebuild_status returns "on_target" when total <= 0 (1317). The frontend paints any unrecognised status green (656). After #94 a gas-only wallet has total > 0 but allocation_base 0.
- **Impact:** A brand-new user gets accidental statuses. Neither the advice nor the UI says the obvious fact that a wallet without ETH cannot transact on Base.
- **Proposed fix:** Guard allocation_base_usd > 0 before dividing. Use an explicit status for an empty base (open question) and render unknown statuses in a neutral colour. When wallet_gas_eth == 0, append a plain fact to the advice: 'The wallet holds no ETH on Base: every action (a buy, a deposit, a fee claim) needs a little ETH for gas.' No number is chosen.

### eth94-11: Adjacent (outside #94): HOLD & ACCRETE advice hardcodes '0% realized loss' — skeptic: **confirmed**

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `ingestion-worker/src/main.rs:6180`
- **Evidence:** `... generating yield (+${:.2} uncollected). 0% realized loss. Maintain posture.` NockNock computes no realized P/L anywhere. The operator reports realized about -$1,003 in the Uniswap all-time view (2026-11-14).
- **Impact:** A static claim presented as live, contradicting 'never imply losses are zero'. Any user who falls through to the default arm sees it, including a from-zero user or the operator once the plan completes.
- **Proposed fix:** Remove the clause, or replace it with 'NockNock does not track cost basis or realized P/L; see your wallet app.'

### Proposed design

1. Reads in fetch_raw_data (main.rs:4911-4932)
- eth_getBalance and WETH balanceOf become required. Either one returning None makes the cycle Err ("Failed to read wallet ETH/WETH (gas) balance"), like 4891-4893.
- wallet_balances gains ETH (token_address 0x0000000000000000000000000000000000000000, amount {:.6}, amount_usd {:.2}) and WETH (WETH_BASE). They are appended after NOCK and USDC so tests indexing [0] and [1] keep working.
- RawOnChainData.wallet_eth and wallet_weth stay Option.
- Delete default_eth_price (509-511) and its serde attribute (375).

2. Pure math in compute_derived_metrics_with_plan (5965+)
- eth_px = raw.live_eth_price_usd.
- In the existing loop, lp_eth = sum of the legs where is_eth_token.
- gas_eth = Some(e + w) only if both reads are Some.
- wallet_gas_usd = gas_eth x eth_px, only if eth_px > 0; otherwise None.
- allocation_base = lp_usd + t3_usd + wallet_usdc (the same as today's total).
- total_portfolio_usd = allocation_base + wallet_gas_usd. If gas is None, the total equals the base and every gas field is null. This is reachable only in tests, because production fails the cycle first. Existing fixtures keep their totals.
- total_eth_amount = gas_eth + lp_eth; total_eth_usd = total_eth_amount x eth_px.
- Identity: total_portfolio_usd = total_native_usd + total_stable_usd + total_eth_usd, within $0.02 of rounding.

3. Where gas is excluded
- Cash target (6055) and LP target and status (6078/6080): computed on allocation_base.
- Deviation weights (6284-6291): computed over allocation_base with a > 0 guard. No gas bucket is added, because target weights sum to 10000.
- deployable_usdc (6050-6056), PlanGuards and cash_reserve_usdc (6058-6067, 1157-1169) and the tax guardrail (6051) are already USDC-only and only inherit the new cash-target base.
- LP rebuild funding (6096-6099, 945-952): gas is not deployable by default. ETH/WETH legs are excluded with a reason, and REBUILD_GAS_BUFFER_ETH is removed. An optional operator-set operator_profile.gas_reserve_eth would allow ETH above it.
- Advice text never presents ETH as spendable. A plain "no ETH for gas" fact is added when gas_eth == 0.

4. HWM and history (no step, no restart)
- HWM SQL (2459) becomes: SELECT generated_at, COALESCE((gs->>'allocation_base_usd')::float8, (gs->>'total_portfolio_usd')::float8) AS v ... ORDER BY v DESC NULLS LAST LIMIT 1, where gs = data->'derived_metrics'->'global_status'.
- high_water_mark_usd = max(v, allocation_base). high_water_mark_date = the generated_at of that row, or now if the current value is higher.
- The Bollinger SQL (2468) and the counterfactual SQL (2513) use the same COALESCE.
- Values passed at 6037, 6233 and 6321: allocation_base.
- slim_historical_payload keeps the last counterfactual point.
- Rationale: pre-2.3.0 totals are exactly the allocation base, so history stays continuous; the 04:38Z HWM of $2,475.27 stays valid; today's -3.42% stays visible.

5. New global_status fields (snapshot 2.3.0)
- Fields: eth_price_usd, wallet_gas_eth, wallet_gas_usd, total_eth_amount, total_eth_usd, allocation_base_usd.
- Each is `#[serde(default)] Option<String>`, and null means not measured. This is mandatory: stored rows are deserialized as SnapshotData at 1672/1873/2007/2108 via `.ok()?`, so a required field would silently drop every older row from /historical and from the boot-time snapshot.
- The name wallet_gas_* avoids clashing with the existing gas-cost fields (gas_to_fee_delta_usd, gas_break_even_usd).
- A pillars array can replace the flat fields when AERO (#93) lands.

6. Spec (specs/core/snapshot.yaml)
- info.version 2.2.0 -> 2.3.0, and main.rs:2660 meta.version to match.
- Add the six nullable properties.
- Reword total_portfolio_usd: "all pillars: wallet NOCK + wallet USDC + wallet ETH/WETH gas + tracked LP; includes gas since 2.3.0".
- Reword total_native_usd: "NOCK only".
- Reword high_water_mark_usd and high_water_mark_date: "highest allocation_base_usd ... when it was observed".
- Reword portfolio_value_series: "allocation base (excludes wallet gas)".
- AllocationDeviation: "weights of allocation_base_usd; gas has no target".
- WalletBalance: document the ETH and WETH rows, and that amounts are human units (fixing the existing "raw amount" text).

7. Frontend
- types/snapshot.ts:109-130: add the optional fields.
- App.tsx: add a "Gas (wallet ETH)" StatCard showing `wallet_gas_eth ETH` with subtitle "$wallet_gas_usd · for transactions; counted in total, not cash, not deployable", or "—" when absent. Its slot is near 339-352.
- Portfolio Value subtitle: "NOCK $ · USDC $ · ETH $".
- HWM card: "excludes wallet gas · set <date>".
- P/L at 727-729 uses allocation_base_usd ?? total_portfolio_usd.
- Composition panel (678-707): retitle and split USDC from ETH in LP.
- Deviation panel: base note, and a neutral colour for unknown statuses (656).
- ethUsd (191-205) comes from global.eth_price_usd.
- Add a gas pill to the goal pills (422-441).

8. MCP (main.rs:2136-2157 and bin/mcp.rs:240-261)
- Add the pillars line and the gas line, printing "not reported" when absent.
- Floor from nock_floor_target; HWM labelled.

Live after the change: total ≈ $2,413.57; allocation base $2,390.58; weights 4345/1760/3893 and LP target $478.12 unchanged; HWM $2,475.27 set 04:38:29Z; P/L -3.42%; ETH pillar ≈ $66.98 (gas ≈ $22.99 + LP $43.99). The ETH pillar stays understated until #91 is tracked.

### Tests to add

- worker: test_gas_counted_in_total_not_in_targets. four_position_fixture plus wallet_eth 0.0125, wallet_weth 0, eth 2373.14. Assert total = allocation_base + 0.0125 x 2373.14 (±0.01). Deviation weights, lp_rebuild.target_usd and the cash-target-driven DIP amount must be identical to the same fixture with gas 0.
- worker: test_pillars_partition_total. Assert |native + stable + eth - total| <= 0.02. total_eth_amount must equal wallet gas plus position A's native-ETH and C's WETH legs, and total_eth_usd >= wallet_gas_usd.
- worker: test_moving_eth_wallet_to_lp_is_total_neutral. Move 0.01 ETH from wallet_eth into the v3 WETH leg. total_portfolio_usd must stay unchanged (±0.01), allocation_base must rise by 0.01 x px and wallet_gas_usd must fall by the same.
- worker: test_unread_gas_is_null_never_zero. With wallet_eth None, every gas and ETH field is None (serialized null), allocation_base == total, and nothing renders as "0.00".
- worker: test_gas_balances_required. A pure helper (e.g. require_gas_balances(Option, Option) -> Result<(f64, f64)>) must Err on any None, with a message naming ETH/WETH.
- worker: test_rebuild_never_deploys_gas_by_default. rebuild_fixture (0.114226 ETH) under target. No suggestion may carry an ETH/WETH amount > 0; #6336662 (and #3163432) must appear in excluded with a reason containing "gas"; the USDC/NOCK suggestion must be unchanged. If gas_reserve_eth is adopted, only ETH above the operator's number may be used.
- worker: test_hwm_on_allocation_base_records_when_set. A pure helper over rows [(ts, total, Option<base>)]. Pre-2.3.0 rows use total and new rows use base. high_water_mark_date must be the max row's ts, not now, and adding gas must never raise the HWM.
- worker: test_slim_history_keeps_current_counterfactual_point. Given series [old..., current], after slim_historical_payload element [0] == current, for time_points and both series.
- worker: test_no_fabricated_eth_price_default. serde_json::from_value::<RawOnChainData> without live_eth_price_usd must fail; no 2500 constant may remain.
- worker (from zero): test_from_zero_empty_wallet. No positions, 0 NOCK/USDC/ETH, eth px set. Totals and gas fields must be "0.00" with no NaN-derived statuses. Advice details must include the plain 'no ETH on Base for gas' fact and no instruction to spend ETH.
- worker (from zero): test_from_zero_gas_only_wallet. Only 0.01 ETH. Assert total == wallet_gas_usd, allocation_base "0.00", no lp_rebuild suggestions, no advice text treating ETH as deployable. With a test-supplied accumulation_plan the binding constraint names USDC, never ETH.
- worker (from zero): test_from_zero_first_tranche_usdc_only. USDC 100 + ETH 0.01 + a plan whose budget, tranches and interval are set by the test (never defaults). The tranche must be funded only from wallet USDC, and the cash target must equal allocation_base x cash bps (USDC only).
- worker: test_mcp_state_text_reports_gas_and_pillars. Both formatters must show gas in ETH and USD and the pillars line. Absent fields print 'not reported', never '$0.00'. The floor comes from nock_floor_target; update the 8349 'above 100k' assertion.
- frontend: pillars helper test (new src/pillars.test.ts). pillarsFromGlobal(global) returns NOCK, USDC and ETH rows, with ETH marked untargeted/gas. Missing 2.3.0 fields render '—'. P/L helper uses allocation_base_usd ?? total_portfolio_usd.
- frontend: deviation colour helper. Unknown or empty statuses must render neutral, not the 'on target' green.
- frontend live contract (skips when the backend is unreachable). For version >= 2.3.0: |native + stable + eth - total| < 0.03, |allocation_base + wallet_gas - total| < 0.02, eth_price_usd > 0, and wallet_balances contains ETH and WETH rows.

### Open questions

- LP rebuild and gas: should gas never fund WETH/ETH legs (recommended default)? Or do you want an operator-set operator_profile.gas_reserve_eth (unset = never) so NockNock can still size WETH/NOCK deposits from the ETH above your number?
- HWM basis: the recommendation measures the HWM and P/L against value excluding wallet gas. That keeps all history continuous and today's -3.42% visible. The alternative is an all-pillar HWM that restarts when gas starts counting; P/L would then read about 0% right after the switch, which could look like 'no loss'. Which do you want?
- Should the ETH pillar include ETH inside LP (wallet gas ≈ $22.99 + LP WETH $43.99 ≈ $66.98), with 'gas' meaning only what sits in the wallet? Recommended: yes.
- From zero: what should the deviation matrix say when nothing is allocated yet (allocation base 0)? Options are a new 'empty' status (spec enum plus a neutral colour) or keeping 'under_funded'.
- Is snapshot 2.3.0 (minor) acceptable? total_portfolio_usd values change but the shape does not; specs/README.md:26 reserves a major bump for breaking changes.
- Cross-area from-zero blockers seen while tracing ETH; route to the from-zero area? (a) fetch_raw_data errors without positions or a price-source position (main.rs:4805-4809, 4835-4840), so an empty portfolio gets no snapshot at all. (b) default_tracked_wallets() returns the operator's wallet (main.rs:252-254). (c) bin/mcp.rs:19 hardcodes WALLET_ADDR, and its fallback reads retired pool B (mcp.rs:337). (d) The default floor of 100000 and weights 8000/1500/500 (main.rs:50-64) are numbers NockNock chose.
- Can #94 ship before #91? Until the Aero CL WETH/NOCK position (0.081995 ETH + 2,755 NOCK deposited, ONCHAIN_INTEGRATION.md §7) is tracked, the ETH pillar and the total stay understated.
- Adjacent zero-mock gap: an alchemy_getTokenBalances entry with a null tokenBalance is read as 0 NOCK/USDC (main.rs:4875 unwrap_or("0x0")), while balances_found only needs any one token. Tighten it to a per-token check?

### Skeptic: problems with the proposed design

- HWM/P/L/history on allocation_base is not neutral to moving ETH. allocation_base = lp_usd + t3_usd + wallet_usdc includes LP WETH legs (5979-5981) but not wallet gas. Depositing gas into the v3 WETH/NOCK range therefore raises the P/L and history series (and the HWM once it sets a new max), and unwinding fabricates a drawdown: the eth94-01 defect, moved from the total into the HWM. The 15:23:43Z step (0.020357 WETH, ~$37) is already in history. The design's own tests contradict each other: test_moving_eth_wallet_to_lp_is_total_neutral expects the base to rise by 0.01 x px, while the HWM test says adding gas must never raise the HWM. The '-3.42% stays visible' figure is not like-for-like: the 04:38Z mark excluded ~0.114 wallet ETH, while today's base includes $44 of it. An ETH-free basis computable for every stored row is total_native_usd + total_stable_usd: 2475.27 at 04:38Z vs 2346.59 now, -5.20%. The other option is the all-pillar total with a marked restart. The operator should choose; the open questions frame it only as gas-in vs gas-out.
- Redefining portfolio_value_series, the counterfactual 'active' series and the Bollinger input as excluding wallet gas leaves no time series carrying the gas pillar, while the headline total includes it. The frontend plan also misses call sites. The 'Total Value & NOCK Price vs Baseline' chart's 'Portfolio USD' line (App.tsx:569-583, data at 153) and the P/L sub-line '$current vs HWM' (App.tsx:752) would differ from the 'Portfolio Value (USD)' card (App.tsx:347-352) by the gas amount. Add a new field or series rather than silently changing what an existing 'portfolio' field means.
- The LP-rebuild gas exclusion point is unspecified. Excluded at funding time (eth_f, 945-952), as today's shortfall path does, the WETH/NOCK position's equal share is never filled. lp_rebuild can then sit under_target indefinitely, with 'No tracked position is eligible' (1355-1356) appended to every directive. Excluded at eligibility (793-823), its share is redistributed to the USDC/NOCK positions. Removing REBUILD_GAS_BUFFER_ETH also requires rewriting the reason at 948. Moving the exclusion inside lp_rebuild_suggestions breaks tests that call it directly with eth: Some(10.0) and expect ETH-leg suggestions: 7461-7482 (3 suggestions incl. #3163432/#6336662), 7485-7503, 7506-7515 (#6336662 target 300.00), 7518-7553 and 7556-7572. Doing it at the call site (6096-6099) keeps them valid.
- The proposed from-zero tests would land on DEFENSIVE LOCK. With OperatorProfile::default() (floor 100,000 at main.rs:50/69, also in base-layer config.yaml.example:95), an empty or gas-only wallet takes the first advice arm (6118-6125): 'CRITICAL: Total NOCK stack (0) is below the 100000 floor ... All yield and stable reserves are locked'. The tests need a test-chosen profile, and that default floor is itself a NockNock-chosen number. In production an empty portfolio never reaches compute_derived_metrics (4805-4809, 4835-4840). The pure-function from-zero tests would pass while a real newcomer gets no snapshot.
- HWM SQL rewrite details. Many rows tie at 2475.27 (04:38:29Z, 04:43:31Z, 04:48:30Z, ...), so `ORDER BY v DESC` needs a `generated_at ASC` tiebreak to report when the mark was set. On an empty table (the now-sanctioned fresh start) a row-returning query yields nothing, so use fetch_optional. The existing .unwrap_or(0.0) (2463) silently resets the HWM to the current value on any DB error.
- One cited deserialization site is already broken. 2108 is not a working path: get_snapshot_by_id_handler parses the stored data column as SnapshotEnvelope, but store_snapshot writes SnapshotData (6422), so every row fails. Live GET /v1/snapshot/abe4273e-... returns CORRUPTED_DATA. The serde(default) Option requirement is still correct for 1672, 1873 and 2007.
- test_no_fabricated_eth_price_default does not isolate the field. RawOnChainData's wallet_balances, pool_mechanics, concentrated_liquidity and contract_fees have no serde default, so from_value fails without live_eth_price_usd anyway. A 'no 2500 constant may remain' check would trip on the 2500.0 prev_hwm arguments at 6482, 6494, 6502 and 6514. Simpler and stronger: drop Deserialize from RawOnChainData, which is never deserialized.
- Spec bump call sites are missing. Besides main.rs:2660, frontend/src/api/freshness.contract.test.ts:194 pins meta.version '2.0.0', already stale against live 2.2.0. CHANGELOG.md:21 still lists 'Count ETH in portfolio totals' under Deferred, while #94 is in milestone v0.5.1.
- Patch scope: the v0.5.1 milestone says 'No features'. The optional operator_profile.gas_reserve_eth knob is a new config feature and belongs in a minor release. Defects 05, 06, 08, 09 and 10 each need their own issue before fix code (AGENTS.md §11); 11 is already #95. Also do not render gas as a 'Goal' pill: App.tsx:424/430 label the pills 'Goal 1/Goal 2', and 'neither minimize nor maximize' rules out framing ETH as an accumulation goal.

### Skeptic: missed in this area

- **From zero: a newcomer inherits the operator's Uniswap positions from the base-layer config, and v3/v4 reads never check NFT ownership, so someone else's LP (incl. its WETH) enters their total and ETH pillar** (`ingestion-worker/config.yaml.example:11-12`, `ingestion-worker/config.yaml.example:33-78`, `ingestion-worker/src/main.rs:1637-1639`, `ingestion-worker/src/main.rs:7121-7139`, `ingestion-worker/src/main.rs:5424`, `ingestion-worker/src/main.rs:5758`, `ingestion-worker/src/main.rs:5598`): test_example_config_is_the_base_layer asserts 'positions must come from the example base layer' for a config.yaml without positions. The example ships the operator's wallet and NFTs #3163450 and #6336662. fetch_v4_position and fetch_v3_position use `wallet` only to stamp `wallet: wallet.to_string()` (5424, 5758); there is no ownerOf or owner check. Only the Aero adapter reads balanceOf(wallet) (5598). A newcomer whose config.yaml sets only a key and wallet would count #3163450 ($179.01) and #6336662 ($178.13, 0.023918 WETH) as their own LP, price source and ETH pillar.
- **The HWM is 'since this database began', not all-time; after the now-sanctioned fresh start, P/L vs HWM reads +0.00% in green** (`ingestion-worker/src/main.rs:2458-2463`, `ingestion-worker/src/main.rs:6037`, `frontend/src/App.tsx:371`, `frontend/src/App.tsx:727-752`): prev_hwm = COALESCE(MAX(total) over snapshots, 0.0). On an empty DB, current_hwm = total, so plPct = 0 and the card renders '+0.00%' with plColor #10b981. Live: the DB's first row is 04:38:29Z today (179 rows, has_more false), yet the card says 'All-time high'. The operator's Uniswap view shows realized -$1,002.58 and unrealized -$576.40. The design's 'set <date>' label still does not state the window (since first snapshot) or that cost-basis P/L is untracked (#98).
- **The ETH/USD read has no stale-pool guard, unlike NOCK spot; that matters once wallet gas is valued in the total** (`ingestion-worker/src/main.rs:228-229`, `ingestion-worker/src/main.rs:4741-4774`, `ingestion-worker/src/main.rs:4842-4849`): eth_price_pool is operator-configurable. The read accepts any positive slot0 price and assumes token0=WETH, token1=USDC (4758-4760) without checking in-range liquidity, while the NOCK source is guarded by check_price_source_liquidity (4842-4849). A frozen or mis-specified configured pool would silently misvalue gas and LP WETH. Severity is low with the default canonical 0.05% pool.
- **Adjacent: GET /v1/snapshot/{id} always returns CORRUPTED_DATA** (`ingestion-worker/src/main.rs:2095-2122`, `ingestion-worker/src/main.rs:6422`): store_snapshot stores serde_json::to_value(&envelope.data) (SnapshotData) in the data column. get_snapshot_by_id_handler parses that column with from_value::<SnapshotEnvelope> (2108), which needs meta and data keys. Live: GET http://chudmax:8081/v1/snapshot/abe4273e-101e-4dff-9f64-2f6c88e23ec0 returns {"error":"CORRUPTED_DATA","message":"Failed to parse snapshot data"}. It is a route listed in specs/core/api.yaml:134.


## from-zero: A brand-new user starting NockNock from zero: fresh install, empty database, wallet with no LP, only USDC, or nothing at all

**Summary.** A brand-new user cannot start from zero today. config.yaml.example is the base layer of every install, and it holds the original operator's wallet, LP NFTs, 100,000 NOCK floor and 80/15/5 weights. A user who copies it, or leaves out `positions:`, sees the operator's holdings as their own, because the position readers never check who owns an NFT. A user who honestly writes `positions: []` cannot run at all: zero positions is a fatal error, and the NOCK/USD spot can only come from an owned position flagged as price source.

Fixing that is not enough, because the advice engine then answers with someone else's numbers:
- The default floor shows every new user 'CRITICAL ... All yield and stable reserves are locked' (DEFENSIVE LOCK). It outranks their own DCA plan, so a due tranche never changes the action and never sends a notification.
- With no plan, the dip arm is live (spot is $0.01153, below $0.020). It tells a 500-USDC user 'Deploy $475.00 of saved USDC ... toward 120,000'.
- The LP bias tells a user who has never held LP to 're-establish ranges' against a default 15% target.

The display surfaces make it worse:
- 'Total NOCK Stack: NaN NOCK' for a zero stack.
- A configured floor of 0 renders as UNKNOWN.
- First-run errors stay hidden behind 'No snapshot available yet'.
- The stdio MCP fallback reads the operator's hardcoded wallet (with malformed calldata) and a retired pool with no liquidity.

Verified on chudmax: GET /v1/snapshot/{id} returns 500 CORRUPTED_DATA for a valid id.

The fixes are small, and the operator's deployment stays unchanged if they go in order: ship the code first, then move the operator's identity into chudmax's config.yaml, then clean the example. Every from-zero test pins the same rule: NockNock states facts and points the user to their own plan and floor, and never sizes a buy.


### FZ-01: A new install reads the original operator's wallet and LP NFTs as its own — skeptic: *pending*

- **Kind / severity / patch:** defect / high / yes
- **Locations:** `ingestion-worker/src/main.rs:252-254`, `ingestion-worker/src/main.rs:1618-1639`, `ingestion-worker/config.yaml.example:2`, `ingestion-worker/config.yaml.example:11-12`, `ingestion-worker/config.yaml.example:33-82`, `ingestion-worker/Dockerfile:25-26`, `ingestion-worker/src/main.rs:5299-5318`, `ingestion-worker/src/main.rs:5674-5687`, `ingestion-worker/src/main.rs:5424`, `ingestion-worker/src/main.rs:5758`, `ingestion-worker/src/main.rs:4736`, `ingestion-worker/src/main.rs:2458-2463`, `ingestion-worker/src/main.rs:7122-7150`
- **Evidence:** The example file is the base layer of every config: main.rs:1620 loads `config::File::new(example_path, config::FileFormat::Yaml)`, it is called at main.rs:1639, and the image ships it at Dockerfile:25. Its active keys are the operator's identity: `tracked_wallets: - "0xeE114cC97eC5adeB34bA5645e315B497D8487a45"` (lines 11-12) and positions #3163450 (v4), #6336662 (v3) and the Aero pool (lines 33-82). The code default is the same wallet: `fn default_tracked_wallets() -> Vec<String> { vec!["0xeE114cC9...".to_string()] }` (252-254).

A config.yaml without `positions:` inherits the example's list. test_example_config_is_the_base_layer asserts this on purpose (7136: 'positions must come from the example base layer'); its comment at 7132-7133 says the live config lacked positions on 2026-11-14.

No reader checks ownership. v4 reads `getPositionLiquidity(nft_id)` (5305) and v3 reads `positions(nft_id)` (5680) for any NFT, then both stamp `wallet: wallet.to_string()` (5424, 5758). `&state.config.tracked_wallets[0]` (4736) panics on an empty list inside the spawned scheduler. The panic kills the scheduler, and /health then stays 'starting' with last_error null.
- **Impact:** Following the example's own instruction ('Copy to config.yaml and fill in your Alchemy key', line 2), a new user sees the operator's portfolio as theirs: about 90,089 NOCK, 931 USDC and three LP positions.

A user who sets only their own wallet sees the operator's v4 and v3 LP value and NOCK added to their totals, LP weight, floor delta and rebuild sizing, all labelled with their own address.

A user who later switches wallets keeps the other wallet's history. The high-water mark (2458-2463), the history charts and the micro RSI series are not scoped by wallet.
- **Proposed fix:** Code changes, safe to ship alone because the example still supplies the operator's keys:
(1) Before reading a Uniswap position, eth_call ownerOf(uint256) (selector 0x6352211e) on the POSM 0x7c5f5a4b... (v4) or on npm_address (v3). On a mismatch, fail the cycle with: 'position <id>: NFT #<n> is owned by 0x..., not the tracked wallet 0x...; list your own positions or set positions: []'. On a revert, fail with 'NFT #<n> does not exist'; this also catches an omitted nft_id, which defaults to 0. Put the comparison in a pure check_position_owner(id, nft_id, owner_word: Option<&str>, wallet) so it can be unit-tested.
(2) default_tracked_wallets() returns an empty Vec. fetch_raw_data returns Err('tracked_wallets is empty: set the wallet NockNock should read') instead of indexing [0].

Config change, only after migration: keep the operator's portfolio in the example as commented documentation, and leave tracked_wallets, positions and the operator_profile numbers inactive. The base layer then carries only market-wide defaults. Before that, copy the operator's wallet, positions and profile into chudmax's config.yaml.

Optional: refuse to load a stored latest snapshot whose wallet_balances[0].wallet_address differs from tracked_wallets[0]. The database is disposable, so the remedy is to tell the user to start with an empty one.

### FZ-02: A wallet with no LP cannot run NockNock: zero positions is fatal, and the spot needs an owned price-source position — skeptic: *pending*

- **Kind / severity / patch:** defect / high / yes
- **Locations:** `ingestion-worker/src/main.rs:4805-4809`, `ingestion-worker/src/main.rs:4833-4849`, `ingestion-worker/src/main.rs:4997-5051`, `ingestion-worker/src/main.rs:5055-5068`, `ingestion-worker/src/main.rs:80-134`, `ingestion-worker/src/main.rs:5259-5339`, `ingestion-worker/src/main.rs:5698-5714`, `ingestion-worker/src/main.rs:5589-5597`, `ingestion-worker/src/main.rs:313-318`
- **Evidence:** Two checks make a no-LP wallet unrunnable:
- `if state.config.positions.is_empty() { return Err(anyhow!("No positions configured; add entries under `positions:` in config.yaml (see ONCHAIN_INTEGRATION.md §5)")) }` (4805-4809).
- `nock_spot_from_positions(&raw_positions).ok_or_else(|| anyhow!("No NOCK/USD spot: exactly one configured position must have is_price_source: true and quote NOCK against USDC"))` (4835-4840).

PositionConfig defaults nft_id to 0 and is_price_source to false (88-89, 124-125). There is no pool-only entry.

The readers already make pool-level reads that need no position:
- v4: POSM poolKeys check plus StateView getSlot0(poolId) (5259-5339).
- v3: pool slot0 (5698-5714). Its tokens come only from the per-NFT positions() decode; the pool's own token0()/token1() selectors are used in the Aero reader (5589-5590).
- Aero: token0, token1, getReserves and totalSupply (5589-5597). aerodrome_position_from_reads already treats lp_balance 0 as 'a valid, empty position that still carries the pool price' (test 7247-7254).

The stale-price guard (fetch_pool_in_range_liquidity plus check_price_source_liquidity, 4997-5051) reads only protocol and pool_id.
- **Impact:** With `positions: []`, every cycle fails right after the ETH and gas reads. The user sees:
- /health: {status: 'degraded', last_error: 'No positions configured; add entries under `positions:` ...'}.
- Dashboard: 'Error: No snapshot available yet' with a Retry button.
- Both MCP transports: 'No snapshot available yet'.

The error tells a user with no LP to add positions they do not have. Leaving `positions:` out instead inherits the operator's (FZ-01).
- **Proposed fix:** Add an optional top-level `price_source:` block: protocol uniswap-v4 with pool_id and currency0/currency1, or uniswap-v3 or aerodrome with pool_address.

A new fetch_price_source_pool() makes pool-level reads only:
- v4: the existing poolKeys check plus getSlot0.
- v3: token0() 0x0dfe1681, token1() 0xd21220a7 and slot0() 0x3850c7bd on the pool.
- Aero: token0, token1, getReserves and totalSupply, passed to aerodrome_position_from_reads with lp_balance 0.

It returns a probe RawPosition: liquidity, amounts and fees all 0, is_price_source true, position_id 'price-source:<protocol>:<network>:<pool>'. The probe never enters raw.positions, so LP totals, brackets, contract_fees, claim payloads and rebuild suggestions are untouched.

In fetch_raw_data, delete 4805-4809. Choose the source with a pure spot_source(&raw_positions, probe.as_ref()): a tracked position flagged is_price_source wins, then the block. With neither, return Err('No NOCK/USD price source: set price_source (a NOCK/USDC pool you need not hold) or flag a tracked NOCK/USDC position is_price_source: true'). Run the existing stale guard unchanged on whichever source won.

Rejected alternative, a watch-only entry under `positions:`: its nft_id would default to 0, v4 needs fake ticks, and the entry would become an LP deposit target (URL .../positions/v4/base/0) and a claim action.

Optional: when market_data.pool_address is unset, default the OHLCV pool to the price-source pool instead of the code default 0x91a9..., the retired pool with no liquidity (313-318).

### FZ-03: The default 100,000 NOCK floor puts every new user in DEFENSIVE LOCK, which hides their own plan and silences its notifications — skeptic: *pending*

- **Kind / severity / patch:** defect / high / yes
- **Locations:** `ingestion-worker/src/main.rs:50`, `ingestion-worker/src/main.rs:66-75`, `ingestion-worker/config.yaml.example:95`, `ingestion-worker/src/main.rs:6118-6125`, `ingestion-worker/src/main.rs:6135-6136`, `ingestion-worker/src/main.rs:3743-3751`, `docs/kb/design-note-dca-accumulation-plan.md:315`, `frontend/src/components/AccumulationPlanPanel.tsx:118-122`
- **Evidence:** The floor defaults to someone else's number in two places: `fn default_treasury_floor() -> f64 { 100000.0 }` (50), and the base-layer example sets `treasury_floor_nock: 100000.0` (95).

The first advice arm is `if total_nock < floor`, giving 'DEFENSIVE LOCK (Floor Recovery)' with: 'CRITICAL: Total NOCK stack ({:.0}) is below the {:.0} floor by {:.0} NOCK! All yield and stable reserves are locked to replenish the sovereign foundation.' (6118-6125). The plan arm comes later (6135).

A notification fires only when the action string changes: `if prev == next { return None; }` (3750). The design note keeps DEFENSIVE LOCK primary while 'the plan keeps its own status' (§8.2, line 315).
- **Impact:** Traced for a wallet with 500 USDC, 0 NOCK, 0 ETH and no LP, at spot $0.01153 with the default profile:

- No plan. Action 'DEFENSIVE LOCK (Floor Recovery)'. Details: 'CRITICAL: Total NOCK stack (0) is below the 100000 floor by 100000 NOCK! All yield and stable reserves are locked to replenish the sovereign foundation. Standing bias: LP holds $0.00 against a $75.00 target (15% of portfolio), $75.00 short. ...'

- The user's own plan ($100 over 5 tranches, 24 h apart). The headline is the same. Meanwhile the plan panel says 'Buy $20.00 of NOCK (tranche 1 of 5)' and MCP get_accumulation_plan says '$20.00 due now'. The headline says reserves are locked while the panel says to spend them.

- After the first fill (1,734.6 NOCK): 'CRITICAL: Total NOCK stack (1735) is below the 100000 floor by 98265 NOCK! ...'

The action never changes between tranche_due and waiting, so no tranche notifies until the stack reaches someone else's 100,000.
- **Proposed fix:** (1) No floor unless the user sets one. default_treasury_floor() returns 0.0, meaning 'no floor'. Comment out the example's line with a note that the number is the user's own. The operator keeps treasury_floor_nock: 100000 in chudmax's config.yaml (a migration step). Gate the arm on `floor > 0.0 && total_nock < floor`.

(2) Keep DEFENSIVE LOCK primary, as design note §8.2 decided, but when an active plan exists:
- action = format!("DEFENSIVE LOCK (Floor Recovery) · {plan_action}")
- details = the deficit sentence, then 'Your accumulation plan is the recovery path: {plan_details}'. This replaces 'All yield and stable reserves are locked ...'.
Tranche transitions then change the action, so they notify. With no plan, append: 'NockNock does not size recovery buys: add an accumulation_plan with your own budget, tranches and interval to have them paced.'

(3) Ship together with FZ-04. Once no default floor masks it, the planless dip arm is the first thing a new user sees.

Test fallout: test_derived_metrics_defensive_lock_under_100k (6476-6484) and test_floor_tolerance_lets_lp_hold_a_bounded_overage (7397-7426) rely on OperatorProfile::default() being 100,000. Both must set the floor explicitly.

### FZ-04: With no plan configured, the dip arm sizes a lump-sum buy for the user and cites the original operator's 120,000 goal — skeptic: *pending*

- **Kind / severity / patch:** defect / high / yes
- **Locations:** `ingestion-worker/src/main.rs:6145-6152`, `ingestion-worker/src/main.rs:6050-6056`, `ingestion-worker/src/main.rs:6736-6750`, `ingestion-worker/src/main.rs:7021-7024`, `frontend/src/components/AccumulationPlanPanel.tsx:93-96`, `README.md:69`, `OPERATOR_GUIDANCE.md:110`
- **Evidence:** The arm's text: 'Spot price (${:.5}) has breached the $0.020 accumulation boundary! Deploy ${:.2} of saved USDC (${:.2} available; ${:.2} protected as virtual tax reserve, ${:.2} kept as cash target) to purchase NOCK at discount and compound buffer toward 120,000.' (6149).

The amount is deployable_usdc = min(stable USDC minus tax reserve, wallet USDC minus the 5% cash target) (6050-6056).

Standing orders: plan numbers are 'the operator's call; never invent defaults' (OPERATOR_GUIDANCE.md:110), and allocation stays the operator's decision (line 123). The design note lists the $0.020 line and the 120,000 goal as legacy constants still open (design-note-dca-accumulation-plan.md:407-410).
- **Impact:** Live spot is $0.01153, so this arm is active for any user with no plan whose stack meets their floor, or who sets the floor to 0.

The 500-USDC new user would read: 'Deploy $475.00 of saved USDC ($500.00 available; $0.00 protected as virtual tax reserve, $25.00 kept as cash target) to purchase NOCK at discount and compound buffer toward 120,000.'

That is 95% of their cash in one buy, a size NockNock chose, toward a target that belongs to someone else. The plan panel adds to it: '...instead of advised as one lump sum'.
- **Proposed fix:** Keep the arm's trigger, but replace its action and text:
- action: 'DIP ZONE (No Buy Plan Configured)'
- details: format!("Spot ${:.5} is below the $0.020 accumulation line and the wallet holds ${:.2} of liquid USDC. NockNock does not size buys outside a plan: add an accumulation_plan with your own budget_usdc, tranches and interval_hours and it will pace them.", spot, wallet_usdc)
No 'Deploy $' and no 120,000.

Add the from-zero arm (FZ-06) before this one. Rewrite the assertions at 6741-6750 and 7023. In AccumulationPlanPanel, change 'instead of advised as one lump sum' to 'NockNock does not size buys without one'. Moving $0.020 into config is a separate open question.

### FZ-05: The LP-rebuild bias tells a user with no positions to 're-establish ranges' toward a default 15% target — skeptic: *pending*

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `ingestion-worker/src/main.rs:6078-6115`, `ingestion-worker/src/main.rs:1308-1337`, `ingestion-worker/src/main.rs:1344-1392`, `ingestion-worker/src/main.rs:921-936`, `ingestion-worker/src/main.rs:2185-2188`, `ingestion-worker/src/bin/mcp.rs:217-222`, `frontend/src/components/AccumulationPlanPanel.tsx:68-87`, `frontend/src/components/LpRebuildSteps.tsx:91`, `frontend/src/accumulation.ts:109-111`, `specs/core/snapshot.yaml:530-532`
- **Evidence:** For 500 USDC and no LP, lp_rebuild_status(0, 500, 1500) returns under_target (target $75, tolerance 300 bps). lp_rebuild_suggestions over an empty raw.positions returns no suggestions and no exclusions.

apply_lp_rebuild_bias then appends: 'LP holds $0.00 against a $75.00 target (15% of portfolio), $75.00 short. Re-establishing ranges from NOCK and USDC already held is a reposition, not a purchase ... No tracked position is eligible for a deposit right now.' (1349, 1356).

RebuildFunds.nock = wallet_nock - floor (= -100,000 here) is clamped by factor() and by the text (921-936), so no negative number surfaces. For an empty wallet, lp_rebuild_status returns 'on_target' (1317).
- **Impact:** A user who has never held LP is told to 'rebuild' in every view:
- the advice details;
- the plan panel note ('REBUILD LP TRANCHES · LP holds $0.00 against a $75.00 target (15% of portfolio), $75.00 short...'), followed by 'No tracked position is eligible for a deposit right now.';
- MCP: 'LP rebuild bias: under_target ... Rebuilding ranges is a reposition: no net cash change.'

The 15% is the example's default weight, not the user's.
- **Proposed fix:** When raw.positions is empty:
- set lp_rebuild.status to 'no_positions' (an additive enum value in snapshot.yaml and types/snapshot.ts:74);
- skip lp_rebuild_suggestions;
- add no bias text. apply_lp_rebuild_bias already returns early unless the status is under_target, and lpRebuildActive stays false.

The MCP lines (main.rs:2185-2188, mcp.rs:220) say: 'No LP positions are tracked; NockNock suggests deposits only into positions you list under positions:'.

Nothing else about the rebuild bias changes for tracked positions.

### FZ-06: Empty and USDC-only wallets get 'Maintain posture' or no guidance on how to start, and deviation weights come from 0/0 — skeptic: *pending*

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `ingestion-worker/src/main.rs:6176-6183`, `ingestion-worker/src/main.rs:6284-6291`, `ingestion-worker/src/main.rs:1310-1317`, `ingestion-worker/src/main.rs:4911-4932`, `ingestion-worker/src/main.rs:6096-6099`
- **Evidence:** The HOLD arm says: 'Total NOCK held is {:.0} (+{:.0} above {:.0} hard floor). {} Uniswap position(s) generating yield (+${:.2} uncollected). 0% realized loss. Maintain posture.' (6180; the same text as issue #95).

Deviation weights are `((t3_usd / total_portfolio_usd) * 10000.0) as i32` (6284, 6287, 6290). With a total of 0 that is NaN, which Rust casts to 0. Nothing panics, but the weights rest on that cast; lp_rebuild_status guards the same division (1310-1314).

Wallet ETH and WETH are read (4911-4932) but used only to fund rebuild suggestions (6096-6099).
- **Impact:** Completely empty wallet with floor 0: 'HOLD & ACCRETE (Dual Growth Engine Active)' / 'Total NOCK held is 0 (+0 above 0 hard floor). 0 Uniswap position(s) generating yield (+$0.00 uncollected). 0% realized loss. Maintain posture.' All three deviation rows read 0 bps and 'under_funded' against default targets.

A 500-USDC wallet with no plan gets the lump-sum arm (FZ-04), or DEFENSIVE LOCK under the default floor.

Nothing tells a new user how to start without NockNock choosing numbers for them, or that a wallet with 0 ETH cannot pay gas for any swap.
- **Proposed fix:** Add two arms, both only when no plan is configured.

(a) Empty wallet, right after DEFENSIVE LOCK, when total_portfolio_usd <= 0 && total_nock <= 0:
- action: 'NOTHING TO TRACK YET (Empty Wallet)'
- details: 'The tracked wallet {addr} holds no NOCK, no USDC and no LP that NockNock reads. Once it is funded NockNock reports it here. To have NOCK buys paced, add an accumulation_plan with your own budget, tranches and interval; set operator_profile.treasury_floor_nock only if you want a floor alarm.'

(b) From zero, after the overextension and plan arms and before the tax-guardrail and dip arms, when total_nock <= 0 && wallet_usdc >= PLAN_DUST_USDC:
- action: 'BUILD FROM ZERO (No Buy Plan Configured)'
- details: 'The tracked wallet holds ${usdc} of USDC and no NOCK. NockNock does not choose amounts for you: add an accumulation_plan with your own budget_usdc, tranches and interval_hours (optionally cash_reserve_usdc and max_price_usd) and it will pace each NOCK buy and verify it on-chain.'

Both append ' The wallet holds no ETH; every swap on Base pays its gas in ETH.' when wallet_eth and wallet_weth both read 0. That states a fact with no amount, consistent with gas as a pillar (#94).

Guard the deviation divisions with `if total_portfolio_usd > 0.0 { ... } else { 0 }`, as lp_rebuild_status does. Fix the HOLD text under #95.

### FZ-07: The dashboard shows 'NaN NOCK' for a zero stack, renders 'no floor' as UNKNOWN, and keeps static fallback text — skeptic: *pending*

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `frontend/src/App.tsx:266-267`, `frontend/src/App.tsx:334`, `frontend/src/App.tsx:845`, `frontend/src/App.tsx:262-264`, `frontend/src/App.tsx:336`, `frontend/src/App.tsx:360-366`, `frontend/src/App.tsx:391`, `frontend/src/App.tsx:426`, `frontend/src/App.tsx:382`, `frontend/src/App.tsx:419-420`, `frontend/src/App.tsx:700`, `frontend/src/App.tsx:726`, `frontend/src/App.tsx:745`, `frontend/src/components/LiteCockpit.tsx:247-255`
- **Evidence:** `const totalNockVal = nockAmountRaw > 0 ? nockAmountRaw.toLocaleString() : '—';` (267) is passed to StatCard as its value (334). StatCard runs `parseFloat(value.replace(/,/g, ''))` on any truthy string (845), and parseFloat('—') is NaN.

`const floorKnown = floorTarget > 0` (263) treats a configured 0 the same as a missing field.

Two fallbacks are static claims: `global?.shot_advice_action || 'HOLD & ACCRETE (Dual Growth Engine Active)'` (382), and 'AMMs are generating dual yield. Maintain posture.' (420).

`passiveUsd > 0 ? ... : '0.0'` (700) and the dial's `pct={100 - taxPct}` (745) turn 0/0 into numbers.
- **Impact:** For a wallet with 0 NOCK, the first header card reads 'Total NOCK Stack: NaN NOCK', and Goal 1 reads '— NOCK total'.

Under the default floor it also shows '-100,000 above 100k floor' and 'CRITICAL: -100,000 Below Floor'.

With a floor of 0 (FZ-03), every floor element reads 'floor target unavailable', 'floor status unknown' or '— NOCK Hard Floor UNKNOWN', and the Lite cockpit shows 'target 0 NOCK'.

A USDC-only wallet shows '+$500.00 USD (+0.0% of native value)'. With no harvests, the dial claims '100% of realized harvests'.
- **Proposed fix:** Add pure, unit-tested helpers in positions.ts:
- statValueText(value, prefix, suffix): '—' unless the value parses to a finite number, formatted with 'en-US'. '0' renders as '0 NOCK'.
- floorStatus(gs): returns state 'none' (target '0'), 'safe', 'below' or 'unknown' (field missing), together with the numbers.
- regimeLabel(): see FZ-11.

In App, pass global?.total_nock_amount straight to StatCard. When the floor state is 'none', show 'No floor set' in the stat card, the banner, the chart title (with no ReferenceLine) and the Lite cockpit. Static fallbacks render '—'. Omit the '% of native value' when native value is 0. The dials say 'no harvests yet' when the cumulative total is 0.

### FZ-08: First-run failures are visible only in /health, and the placeholder key produces a misleading first error — skeptic: *pending*

- **Kind / severity / patch:** gap / medium / yes
- **Locations:** `ingestion-worker/src/main.rs:1911-1919`, `ingestion-worker/src/main.rs:2411-2413`, `ingestion-worker/src/main.rs:4733-4774`, `ingestion-worker/src/main.rs:3909-3916`, `frontend/src/api/client.ts:122-128`, `frontend/src/App.tsx:77-85`, `frontend/src/components/LiteCockpit.tsx:261-262`, `README.md:118-135`
- **Evidence:** With no snapshot, /v1/snapshot/latest returns 503 `{"error": "NO_SNAPSHOT", "message": "No snapshot available yet"}` (1915-1918). It does not include the stored last_ingest_error. MCP returns a bare 'No snapshot available yet' (2412). client.ts already prefers `err.detail` when one is present (127).

fetch_raw_data uses the Alchemy key without checking it (4734-4735). With 'YOUR_ALCHEMY_API_KEY', the first read to fail is the ETH price: 'Failed to query live ETH/USD spot price from the v3 WETH/USDC pool (...) slot0' (4770-4773). The record and claim handlers do check for the placeholder (3910, 4220).

The README Quick Start is only `docker compose up -d --build`: no step for the wallet, the key or a price source.
- **Impact:** A new user whose first cycle fails (no price source, placeholder key, or an NFT they do not own) sees 'Error: No snapshot available yet' indefinitely. MCP says the same. The Lite cockpit says 'Worker unreachable: No snapshot available yet; ingestion cycle has not completed' even though the worker is up. The reason exists only in /health.last_error, and with the placeholder key it points at an ETH pool rather than the key.
- **Proposed fix:** - Add 'detail': last_ingest_error to the 503 body through a pure no_snapshot_body(last_error) helper. The dashboard then shows the real cause through the existing client.ts mapping.
- Append 'Last ingestion error: ...' to the MCP no-snapshot text.
- At the top of fetch_raw_data, return Err('No Alchemy key configured (alchemy_api_keys)') for an empty or placeholder key, and an explicit Err for an empty tracked_wallets, mirroring the checks at 3910 and 3914.
- On a 503, the Lite cockpit shows the error detail instead of 'Worker unreachable'.
- Docs: a from-zero quick start covering the wallet, the key, price_source, positions: [], and the user's own plan and floor.

### FZ-09: MCP: the stdio fallback reads the operator's wallet and a pool with no liquidity; both transports hardcode 'above 100k' and invent fallback values — skeptic: *pending*

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `ingestion-worker/src/bin/mcp.rs:19`, `ingestion-worker/src/bin/mcp.rs:333-389`, `ingestion-worker/src/bin/mcp.rs:255`, `ingestion-worker/src/bin/mcp.rs:247`, `ingestion-worker/src/bin/mcp.rs:265-266`, `ingestion-worker/src/bin/mcp.rs:229`, `ingestion-worker/src/main.rs:2143`, `ingestion-worker/src/main.rs:2151`, `ingestion-worker/src/main.rs:2161-2162`, `ingestion-worker/src/main.rs:2247-2298`, `ingestion-worker/config.yaml.example:35-36`
- **Evidence:** `const WALLET_ADDR: &str = "0xeE114cC97eC5adeB34bA5645e315B497D8487a45";` (mcp.rs:19).

The fallback balance call is `format!("0x70a082310000000000000000{}", WALLET_ADDR...)` (355). It pads with 16 zero hex digits where 24 are needed, so the argument is only 28 bytes.

The fallback spot comes from `NOCK_USDC_POOL_ID = 0x91a9ef0a...f99c` (337), with no liquidity check. That is the retired 0.75% pool; the example says it 'has had no in-range liquidity since the 2026-11 liquidation' (config.yaml.example:35-36).

Both formatters print `(Floor buffer delta: {delta_prefix}{nock_delta} above 100k)` (main.rs:2151, mcp.rs:255). Missing fields default to 'HOLD', 'Accreting' and '0.00' (main.rs:2143, 2161-2162).
- **Impact:** When the worker is unreachable, any user's MCP client gets a 'NOCK Spot Price' read from a frozen pool and presented as live.

The balance line most likely reads 'Unavailable' today only because the calldata is malformed: a Solidity ABI decoder rejects a short argument. Fixing the padding alone would publish the operator's balance to every user.

A new user with floor 0 reads 'Floor buffer delta: +0 above 100k'. With no positions, simulate_claim_fees reports 'Estimated Gas: unavailable' although the worker has the live gas cost.
- **Proposed fix:** - Turn the zero-DB fallback into a pure zero_db_fallback_text(tool, api_base) that returns: 'The NockNock worker at {api_base} is unreachable (set NOCKNOCK_API_URL). Nothing is reported that the worker has not read.' Delete WALLET_ADDR, WALLET_PADDED and both RPC reads.
- In both formatters, read nock_floor_target: '0' prints 'no floor configured'; otherwise print '{delta} vs your {target} NOCK floor'. Missing fields print 'unavailable'.
- format_claim_simulation_text says 'No tracked positions: nothing to claim.' when contract_fees is empty.
- Align mcp.rs:229 with main.rs:2223-2237, so a waiting plan no longer shows '$0.00 due now'.

### FZ-10: GET /v1/snapshot/{id} returns 500 CORRUPTED_DATA for every stored snapshot — skeptic: *pending*

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `ingestion-worker/src/main.rs:2087-2134`, `ingestion-worker/src/main.rs:6422`, `specs/core/api.yaml:134-153`, `frontend/src/api/client.ts:157`
- **Evidence:** The handler parses the data column as a whole envelope: `serde_json::from_value::<SnapshotEnvelope>(row.data)` (2108). But store_snapshot writes only the payload: `let data_json = serde_json::to_value(&envelope.data)?;` (6422).

Verified live: GET http://chudmax:8081/v1/snapshot/b99b049c-8297-4692-99d1-12d7579cffab, the latest id listed by /v1/snapshot/metadata, returned 500 {"error":"CORRUPTED_DATA","message":"Failed to parse snapshot data"}. api.yaml promises a 200 with a SnapshotEnvelope.
- **Impact:** The snapshot-by-id endpoint is unusable for every user, including one on a fresh database, and it reports healthy data as corrupted. client.ts exposes snapshotById; no UI calls it today.
- **Proposed fix:** Deserialize SnapshotData from row.data, and build the envelope from the row's snapshot_id, generated_at, state_hash and version, as get_historical_snapshots_handler already does (2004-2018). Factor this into envelope_from_stored(...) so a unit test can round-trip sample_snapshot_data().

### FZ-13: The default target weights (80/15/5) are the original operator's allocation, applied to every new user — skeptic: *pending*

- **Kind / severity / patch:** design / medium / no
- **Locations:** `ingestion-worker/src/main.rs:52-64`, `ingestion-worker/config.yaml.example:112-115`, `ingestion-worker/src/main.rs:6055-6056`, `ingestion-worker/src/main.rs:1148-1156`, `ingestion-worker/src/main.rs:6078-6101`, `ingestion-worker/src/main.rs:6270-6316`, `OPERATOR_GUIDANCE.md:73-75`, `OPERATOR_GUIDANCE.md:123`
- **Evidence:** default_target_weights() returns treasury 8000, uniswap_lp 1500 and cash 500 (59-63), and the base-layer example sets the same values (113-115). These weights drive three things:
- the cash target, which caps plan tranches and the dip arm (6055-6056, 1148-1156);
- the LP target, which sizes rebuild deposits (6078-6101);
- the deviation matrix, which labels every bucket against them (6270-6316).

OPERATOR_GUIDANCE §5 asks for 'sensible defaults so existing deployments continue running'. Standing order 9 keeps allocation 'the operator's decision' (line 123).
- **Impact:** Once a new user lists their first position, NockNock sizes LP deposits at 15% of their portfolio, and with no floor it may put all of their NOCK in. It also labels their holdings under- or over-funded against someone else's 80/15/5.

With FZ-05 in place, this does not affect a user with no positions. The live operator uses 7500/2000/500 from their own config.
- **Proposed fix:** This needs an operator decision. Recommended:
- weights default to 0, meaning 'no target', and are commented out in the example;
- deviation rows report status 'no_target' when the target is 0;
- lp_rebuild_status already returns on_target when the target is 0 or less; rename that to no_target so it is honest;
- with a cash target of 0, the plan relies on the user's own cash_reserve_usdc.
Migration: confirm that chudmax's config.yaml sets all three weights; cash may currently be inherited from the example.

### FZ-11: Regime reads 'normal' when the daily RSI is unknown (fresh database), and the UI cannot show anything else — skeptic: *pending*

- **Kind / severity / patch:** defect / low / yes
- **Locations:** `ingestion-worker/src/main.rs:6204`, `ingestion-worker/src/main.rs:2606-2610`, `ingestion-worker/src/main.rs:1114-1118`, `frontend/src/App.tsx:395-405`, `frontend/src/types/snapshot.ts:125`, `specs/core/snapshot.yaml:659-662`
- **Evidence:** `regime: if regime.overextended { "overextended".to_string() } else { "normal".to_string() }` (6204), where overextended is `rsi_1d.map_or(false, |r| r >= RSI_OVERBOUGHT)` (2609). The badge shows 'NORMAL' for anything other than 'overextended' (App.tsx:404).
- **Impact:** On a fresh database where the GeckoTerminal and DEXScreener backfill fails, or before 15 daily closes exist, the dashboard shows 'REGIME: NORMAL' next to 'RSI(14D): —'. The plan's overextension pause is then silently inactive: it fails open, and nothing says so.
- **Proposed fix:** - Emit regime 'unknown' when rsi_1d is None, as an additive enum value in snapshot.yaml and types/snapshot.ts.
- Add a pure frontend regimeLabel() that maps 'unknown' to 'UNKNOWN (no daily RSI yet)'.
- Optional: when a plan with pause_when_overextended is due and the RSI is unknown, plan_advice notes 'RSI guard unavailable: no daily price history yet'.

### FZ-12: Latent math edge cases: a zero sqrtPrice gives a +inf spot, and a flat price series reads as RSI 100 (overextended) — skeptic: *pending*

- **Kind / severity / patch:** defect / low / yes
- **Locations:** `ingestion-worker/src/main.rs:5058-5067`, `ingestion-worker/src/main.rs:3260-3262`
- **Evidence:** For a USDC/NOCK source, `10f64.powi(src.token1.decimals - src.token0.decimals) / raw` is +inf when raw is 0, and `if price > 0.0 { Some(price) }` accepts it (5060, 5067).

`let rsi = |g, l| if l == 0.0 { 100.0 } else { ... }` (3261) maps no movement at all (g = l = 0) to 100.
- **Impact:** A mistyped v4 price_source pool id (an uninitialized pool with sqrtPrice 0) is stopped today only by the zero-liquidity guard, which runs after the spot has been computed.

A frozen or flat daily series, for example from a stale OHLCV pool, reads as RSI 100. That pauses every plan and shows SWEEP TO STABLE.
- **Proposed fix:** Require `price.is_finite() && price > 0.0` in nock_spot_from_positions. In compute_rsi_series, return None when avg_gain and avg_loss are both 0: the RSI is undefined there, not overbought.

### FZ-14: A fresh start on an empty database resets plan progress and the tax reserve; nothing documents how to recover — skeptic: *pending*

- **Kind / severity / patch:** risk / low / no
- **Locations:** `ingestion-worker/src/main.rs:2731-2746`, `ingestion-worker/src/main.rs:3982-4003`, `ingestion-worker/src/main.rs:2487-2505`, `ingestion-worker/src/main.rs:6049`, `ingestion-worker/src/main.rs:2458-2463`
- **Evidence:** The plan anchor is `starts_at.unwrap_or(registered_at)` (2732), and registered_at is the first time the new database sees the plan. Earlier buys are rejected with BEFORE_PLAN_START: 'To count earlier buys, set accumulation_plan.starts_at to an earlier time.' (3995-3999).

The tax reserve is SUM(harvest_events) times the ratio (2487-2505, 6049). The high-water mark is the MAX over stored snapshots (2458-2463).
- **Impact:** Issue #96 is dropped and the database is disposable. After a wipe:
- a plan in progress starts again at 'DCA TRANCHE DUE (1 of N)' and would re-advise tranches already bought;
- the virtual tax reserve and its guardrail drop to 0;
- the high-water mark and the charts restart.
The wallet itself is unaffected.
- **Proposed fix:** Documentation only, as a 'Starting fresh' section in the lore or runbook. After starting on an empty database, either give the plan a new id, or set starts_at to the original start and re-record each buy's transaction hash; receipts are re-verified on-chain. Re-record harvest hashes to restore the tax reserve, and accept that the high-water mark and history restart. No code change is needed: the BEFORE_PLAN_START message already names the remedy.

### Proposed design

The from-zero contract: NockNock reads a wallet and reports what it holds. Every number that sizes an action comes from the user's own config: floor, weights, budget, tranches, interval and reserve. An unset number means 'no rule', never someone else's value.

1) Config layering (FZ-01, FZ-03, FZ-13)
config.yaml.example stays the base layer, but its active keys are limited to market-wide infrastructure: token addresses, eth_price_pool, market_data.pool_address, sync window, port, and possibly a default price_source (operator's choice). The operator's wallet, positions and profile stay in it as commented documentation. Code defaults change too: tracked_wallets defaults to empty (with an explicit error) and treasury_floor_nock to 0 (no floor).
Migration order:
  (a) Ship the code changes. The example still carries the operator's values, so chudmax is unchanged.
  (b) Copy tracked_wallets, positions and operator_profile into chudmax's config.yaml and deploy. Verify: /health is ok, the snapshot has 3 positions, nock_floor_target is '100000', the deviation targets are 7500/2000/500, and make test-mcp passes.
  (c) Clean the example.
Add an ownerOf guard to the Uniswap readers (selector 0x6352211e on the POSM or NPM) behind a pure check_position_owner().

2) NOCK/USD spot without a position (FZ-02)
New config block:
  price_source:
    protocol: "uniswap-v4"
    pool_id: "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13"
    currency0: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"   # USDC
    currency1: "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722"   # NOCK
  # or protocol: "aerodrome" | "uniswap-v3" with pool_address
Code:
- `struct PriceSourceConfig { protocol, network (default "base"), pool_id, currency0, currency1, pool_address }`, added to AppConfig as `#[serde(default)] price_source: Option<PriceSourceConfig>`.
- `async fn fetch_price_source_pool(..) -> Result<RawPosition>` makes pool-level reads only:
  - v4: the existing poolKeys check plus StateView getSlot0.
  - v3: token0 0x0dfe1681, token1 0xd21220a7 and slot0 0x3850c7bd on the pool.
  - Aero: token0, token1, getReserves and totalSupply, then aerodrome_position_from_reads with lp_balance 0.
  It returns a probe: liquidity, amounts and fees all 0, is_price_source true, id 'price-source:<protocol>:<network>:<pool>'.
- fetch_raw_data: delete the positions.is_empty() return. Read the probe only when no tracked position is flagged. Then `let src = spot_source(&raw_positions, probe.as_ref())?;` (a pure function), and `spot = nock_spot_from_positions(std::slice::from_ref(src))`, with is_finite added (FZ-12).
- The stale guard is unchanged: fetch_pool_in_range_liquidity(src) followed by check_price_source_liquidity(&src.position_id, &src.pool_id, l).
- The probe never enters raw.positions.

3) Advice precedence in compute_derived_metrics_with_plan (FZ-03 to FZ-06)
  1. DEFENSIVE LOCK, only when floor > 0 && total_nock < floor. With an active plan the action is 'DEFENSIVE LOCK (Floor Recovery) · <plan action>' and the details carry the plan's instruction, so tranche transitions notify. Without a plan, the details point to configuring one.
  2. 'NOTHING TO TRACK YET (Empty Wallet)': no plan configured and nothing held.
  3. Overextension cooldown (unchanged).
  4. Active plan (unchanged).
  5. Tax guardrail (unchanged).
  6. 'BUILD FROM ZERO (No Buy Plan Configured)': no plan, no NOCK, liquid USDC of at least $1.
  7. 'DIP ZONE (No Buy Plan Configured)': the legacy dip arm with no amount and no 120,000.
  8. Range exit, batch harvest, HOLD. HOLD text is fixed per #95, and says 'no floor configured' when the floor is 0.
The LP bias reports status 'no_positions' when nothing is tracked; otherwise it is unchanged. The deviation divisions are guarded against a total of 0. The two from-zero arms add a gas fact when wallet ETH and WETH both read 0.

4) Surfaces
- The 503 NO_SNAPSHOT response carries detail = last_ingest_error, and the MCP no-snapshot text includes it.
- fetch_raw_data rejects an empty or placeholder key and empty tracked_wallets up front.
- The MCP formatters use nock_floor_target and say 'unavailable' instead of inventing values.
- The stdio MCP fallback reports nothing it has not read through the worker.
- GET /v1/snapshot/{id} parses SnapshotData.
- Regime reports 'unknown' when there is no daily RSI.
- Frontend helpers statValueText, floorStatus and regimeLabel separate 'none' from 'unknown'.
- Snapshot spec: additive enum values 'no_positions' and 'unknown'.

5) Docs and lore
Add a 'Starting from zero' section to the README quick start and docs/kb. It covers:
- what NockNock reads, and what it never decides;
- the minimal config: wallet, key, price_source, positions: [], and an optional floor and plan, all with the user's own numbers;
- that a wallet with 0 ETH cannot act on Base;
- the fresh-start recovery for plans and harvests (FZ-14).

6) Ship order
FZ-03 together with FZ-04, because removing the default floor exposes the dip arm. FZ-05 together with FZ-02, because a user with no LP only exists once FZ-02 lands.

Local playtest: an empty Postgres, a freshly generated read-only wallet address, positions: [], and a price_source block. Expect the first cycle to succeed, the advice to read 'NOTHING TO TRACK YET (Empty Wallet)', and nothing in any view to refer to 0xeE11...

### Tests to add

- Rust test helpers (tests module, main.rs). Every number in them is a test input that stands for the user's own choice, never a default:
- from_zero_raw(usdc: f64, nock: f64) -> RawOnChainData: starts from RawOnChainData::default() and sets spot_price_nock_usd 0.01153, live_eth_price_usd 3000.0, live_gas_cost_usd 0.004, wallet_eth Some(0.0), wallet_weth Some(0.0), positions empty. wallet_balances are [NOCK format!("{:.4}", nock), USDC format!("{:.4}", usdc)], with wallet_address '0x00000000000000000000000000000000000000aa', token addresses NOCK_BASE and USDC_BASE.
- own_plan() -> AccumulationPlanConfig: id 'my-first-plan', budget_usdc 100.0, tranches 5, interval_hours 24.0, venue 'Aero', starts_at None, max_price_usd None, pause_when_overextended true, respect_cash_target true, cash_reserve_usdc None.
- zero_metrics(raw, profile, ctx): compute_derived_metrics_with_plan(raw, 0.0, profile, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default(), ctx).
- R1 test_from_zero_usdc_only_without_plan_never_sizes_a_buy. Setup: from_zero_raw(500.0, 0.0), OperatorProfile::default() (floor 0 after FZ-03), PlanContext::none().
Expect in global_status:
- total_portfolio_usd '500.00', total_native_usd '0.00', total_stable_usd '500.00';
- total_nock_amount '0', nock_floor_target '0', high_water_mark_usd '500.00';
- shot_advice_action == 'BUILD FROM ZERO (No Buy Plan Configured)';
- details contain '$500.00', 'accumulation_plan' and 'no ETH';
- details contain none of 'Deploy $', 'Buy $', '120,000', 'DEFENSIVE LOCK', 'CRITICAL', 'Standing bias', 'Re-establishing'.
Also expect: accumulation_plan is None; lp_rebuild.status == 'no_positions' with empty suggestions and excluded; deviation actual_weight_bps == [0, 0, 10000]; brackets, positions_nock and velocity_gas_sandbox.contracts are all empty.
- R2 test_from_zero_empty_wallet_is_an_explicit_empty_state. Setup: from_zero_raw(0.0, 0.0), default profile, no plan.
Expect:
- action == 'NOTHING TO TRACK YET (Empty Wallet)';
- details contain '0x00000000000000000000000000000000000000aa', 'holds no NOCK, no USDC and no LP', 'accumulation_plan' and 'no ETH';
- details contain none of 'Maintain posture', 'generating yield', 'realized loss', 'Deploy $';
- total_portfolio_usd '0.00' and high_water_mark_usd '0.00';
- deviation actual_weight_bps == [0, 0, 0];
- lp_rebuild.status == 'no_positions'.
- R3 test_from_zero_users_own_plan_leads_and_uses_only_their_numbers. Setup: from_zero_raw(500.0, 0.0), default profile, PlanContext { cfg: Some(own_plan()), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-15T12:00:00Z") }.
Expect:
- action == 'DCA TRANCHE DUE (1 of 5)';
- details start with 'Buy $20.00 of NOCK on Aero (spot $0.01153).';
- details contain 'Plan "my-first-plan": $0.00 of $100.00 deployed over 0 verified buy(s).';
- plan status 'tranche_due', tranche_usdc '20.00', nominal_tranche_usdc '20.00', budget_usdc '100.00', binding_constraint None;
- details contain neither 'DEFENSIVE LOCK' nor 'Standing bias'.
The $20.00 is the user's own 100/5; no other figure sizes the buy.
- R4 test_from_zero_after_first_fill_waits_on_the_users_interval. Setup: from_zero_raw(480.0, 1734.6054); fills PlanFills { count: 1, spent_usdc: 20.0, acquired_nock: 1734.6054, last_at: Some(t("2026-11-15T12:00:00Z")), first_price: Some(0.01153) }; now t("2026-11-15T18:00:00Z"); own_plan(); default profile.
Expect:
- action == 'DCA ON SCHEDULE (2 of 5 next)';
- details equal exactly: 'Nothing to buy yet: the next $20.00 tranche is due at 2026-11-16T12:00:00+00:00. Plan "my-first-plan": $20.00 of $100.00 deployed over 1 verified buy(s), average cost $0.011530.';
- total_nock_amount '1735' and total_portfolio_usd '500.00';
- plan: tranche_index 2, fills 1, avg_price_usd Some('0.011530'), lump_sum_nock Some('8673.0269'), remaining_usdc '80.00', next_due_at Some('2026-11-16T12:00:00+00:00').
- R5 test_from_zero_users_own_floor_keeps_the_plan_visible_and_notifying. Profile: OperatorProfile { treasury_floor_nock: 100_000.0, ..OperatorProfile::default() } (the user's own goal).
Due case, with R3's inputs:
- action == 'DEFENSIVE LOCK (Floor Recovery) · DCA TRANCHE DUE (1 of 5)';
- details contain 'is below the 100000 floor by 100000 NOCK' and 'Buy $20.00 of NOCK on Aero (spot $0.01153)', and do not contain 'stable reserves are locked';
- nock_floor_target '100000', nock_floor_delta '-100000';
- plan status 'tranche_due'.
Waiting case, with R4's inputs:
- action == 'DEFENSIVE LOCK (Floor Recovery) · DCA ON SCHEDULE (2 of 5 next)';
- details contain 'below the 100000 floor by 98265 NOCK'.
Notification: directive_transition_notification(Some(&due_action), &waiting_action, &waiting_details, None).is_some().
- R6 test_from_zero_users_own_floor_without_plan_points_to_a_plan. Setup: floor 100_000.0, from_zero_raw(500.0, 0.0), PlanContext::none().
Expect:
- action == 'DEFENSIVE LOCK (Floor Recovery)';
- details contain 'below the 100000 floor by 100000 NOCK' and 'accumulation_plan';
- details contain none of 'Deploy $', 'Buy $', '120,000'.
- R7 test_planless_dip_zone_states_facts_only. This replaces the assertions at main.rs:6741-6750 and 7021-7024. Setup: liquid_wallet_fixture() (115,747.8 NOCK, 1,053 USDC, spot 0.01229), profile floor 100_000.0 set explicitly, PlanContext::none().
Expect:
- action == 'DIP ZONE (No Buy Plan Configured)';
- details contain '$0.01229', '$1053.00' and 'accumulation_plan';
- details contain neither 'Deploy $' nor '120,000'.
In test_tax_guardrail_blocks_dip_deployment, the 5000-USDC case asserts the same instead of 'Deploy ${advised}'.
- R8 test_price_source_pool_is_never_a_position (pure). Build the probe with aerodrome_position_from_reads(&aero_cfg(true), "", "0x85f1aa3a70fedd1c52705c15baed143e675cd626", token_info(USDC_BASE).unwrap(), token_info(NOCK_BASE).unwrap(), 369_321_720_000, 318_394_460_000_000_000_000_000, 342_542_824_101_829_799, 0, 0, 0).unwrap().
Expect:
- probe amount0, amount1 and liquidity are (0.0, 0.0, 0);
- spot_source(&[], Some(&probe)) returns the probe, and nock_spot_from_positions(&[probe]) is within 0.0001 of 0.0116;
- spot_source(&[four_position_fixture().positions[1].clone()], Some(&probe)) returns the flagged position;
- spot_source(&[four_position_fixture().positions[0].clone()], None) is Err with a message containing 'price_source';
- check_price_source_liquidity(&probe.position_id, &probe.pool_id, 0) is Err containing 'Stale NOCK/USD spot';
- zero_metrics over from_zero_raw(500.0, 0.0) with spot taken from the probe gives empty brackets, positions_nock and velocity contracts, and active_protocol_series == ['0.00'].
- R9 test_price_source_block_parses_and_positions_may_be_empty. serde_yaml::from_str::<AppConfig>("positions: []\nprice_source:\n  protocol: uniswap-v4\n  pool_id: \"0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13\"\n  currency0: \"0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913\"\n  currency1: \"0x9B5E262cF9bb04869ab40b19AF91D2dc85761722\"\n").
Expect: positions is empty; price_source protocol == 'uniswap-v4' and network == 'base'. A config without the block parses with price_source None.
- R10 test_position_owner_must_be_the_tracked_wallet (pure check_position_owner). Use owner word '000000000000000000000000ee114cc97ec5adeb34ba5645e315b497d8487a45'.
- With wallet '0x00000000000000000000000000000000000000aa': Err containing 'owned by 0xee114cc97ec5adeb34ba5645e315b497d8487a45' and 'not the tracked wallet'.
- With wallet '0xeE114cC97eC5adeB34bA5645e315B497D8487a45': Ok(()).
- With owner None (ownerOf reverted): Err containing 'NFT #3163450 does not exist'.
- R11 test_new_user_defaults_choose_nothing: serde_yaml::from_str::<AppConfig>("port: 8080\n") gives an empty tracked_wallets and accumulation_plan None, and OperatorProfile::default().treasury_floor_nock == 0.0.
- R12 test_no_snapshot_body_carries_the_last_ingest_error: no_snapshot_body(Some("No NOCK/USD price source: x")) == json!({"error":"NO_SNAPSHOT","message":"No snapshot available yet","detail":"No NOCK/USD price source: x"}). no_snapshot_body(None) has no 'detail' key.
- R13 test_snapshot_by_id_round_trips_stored_data. Let stored = serde_json::to_value(&sample_snapshot_data()).
- serde_json::from_value::<SnapshotEnvelope>(stored.clone()) is Err; this documents the current bug.
- envelope_from_stored(id, generated_at, hash, "2.2.0", stored.clone()) is Some, and its data serializes back equal to stored.
- R14 test_mcp_formatters_from_zero (main.rs). Fixture: json!({"derived_metrics":{"global_status":{"total_portfolio_usd":"500.00","total_nock_amount":"0","nock_floor_delta":"0","nock_floor_target":"0","total_usdc_amount":"500.00","high_water_mark_usd":"500.00","shot_advice_action":"BUILD FROM ZERO (No Buy Plan Configured)","total_fees_claimable_usd":"0.00","gas_to_fee_delta_usd":"0.00"},"accumulation_plan":null,"lp_rebuild":{"status":"no_positions","actual_usd":"0.00","target_usd":"75.00","gap_usd":"75.00","suggestions":[],"excluded":[]}},"contract_fees":[]}).
Expect:
- format_portfolio_state_text contains 'Sovereign NOCK Stack: 0 NOCK' and 'no floor configured', and does not contain '100k'.
- With target '100000' and delta '-100000' it contains 'below' and '100000', and not 'above 100k'.
- format_accumulation_plan_text contains 'No accumulation_plan is configured' and 'No LP positions are tracked', and not 'Rebuilding ranges'.
- format_claim_simulation_text contains 'No tracked positions: nothing to claim'.
- format_shot_advice_text(&json!({})) contains 'unavailable' and neither 'HOLD' nor 'Accreting'.
Apply the same assertions to mcp.rs format_portfolio_state and format_accumulation_plan, through a new #[cfg(test)] module in mcp.rs or a shared formatter module.
- R15 test_zero_db_fallback_reports_nothing_unread (mcp.rs): zero_db_fallback_text("get_portfolio_state", "http://127.0.0.1:8080") contains 'unreachable' and 'NOCKNOCK_API_URL'. Case-insensitively it does not contain '0xee114cc9', and it does not contain 'Spot Price'.
- R16 test_regime_unknown_without_daily_rsi. regime is 'unknown' with MarketRegime::default(); 'normal' with { rsi_5m: None, rsi_1d: Some(34.2), overextended: false }; 'overextended' with { rsi_1d: Some(82.0), overextended: true }. Update the frontend union and regimeLabel to match.
- R17 test_degenerate_math_edges:
- A USDC/NOCK RawPosition with sqrt_price_x96 U256(0, 0) and is_price_source true gives nock_spot_from_positions(..) == None.
- compute_rsi_series(&[0.0115; 20], 14) contains no Some(v) with v >= 70.0; the proposed behaviour is all None.
- With rsi_1d None, zero_metrics never produces an action containing 'Overextension Cooldown'.
- R18 test_example_config_has_no_operator_identity. Land this only after example cleanup step (c). Call load_app_config("config.yaml.example", Some(<tmp>/config.yaml containing alchemy_api_keys: ["k"] and tracked_wallets: ["0x00000000000000000000000000000000000000aa"]), false) from the crate root, cargo test's working directory.
Expect: positions is empty, tracked_wallets == ['0x00000000000000000000000000000000000000aa'], operator_profile.treasury_floor_nock == 0.0, accumulation_plan is None.
Keep test_example_config_is_the_base_layer: it uses a synthetic example file.
- Existing tests to update for FZ-03 and FZ-04:
- test_derived_metrics_defensive_lock_under_100k (main.rs:6476) and test_floor_tolerance_lets_lp_hold_a_bounded_overage (7397): set treasury_floor_nock 100_000.0 explicitly, or add an operator_profile_2026_11() helper and use it in metrics_with_plan.
- test_tax_guardrail_blocks_dip_deployment (6741-6750) and test_advice_plan_replaces_lump_sum_dip_shot (7023): assert 'DIP ZONE (No Buy Plan Configured)' with no 'Deploy $'.
- V1 (vitest, new 'from zero' describe block in frontend/src/api/client.errors.test.ts or src/fromZero.test.ts). statValueText formats with 'en-US':
- statValueText('0', '', ' NOCK') === '0 NOCK'
- statValueText('113863', '', ' NOCK') === '113,863 NOCK'
- statValueText('—', '', ' NOCK') === '—'
- statValueText(undefined, '$', '') === '—'
None of the results contains 'NaN'.
- V2 floorStatus:
- { nock_floor_target: '0', nock_floor_delta: '0' } deep-equals { state: 'none', target: 0, delta: null }.
- { nock_floor_target: '100000', nock_floor_delta: '-100000' } deep-equals { state: 'below', target: 100000, delta: -100000 }.
- { nock_floor_target: '100000', nock_floor_delta: '13863' } deep-equals { state: 'safe', target: 100000, delta: 13863 }.
- {} and { nock_floor_target: 'n/a' } both deep-equal { state: 'unknown', target: null, delta: null }.
- V3 lpRebuildActive({ status: 'no_positions', target_usd: '75.00', actual_usd: '0.00', gap_usd: '75.00', target_weight_bps: 1500, suggestions: [], excluded: [] }) === false. The existing under_target case stays true.
- V4 regimeLabel('unknown') === 'UNKNOWN (no daily RSI yet)'; regimeLabel('normal') === 'NORMAL'; regimeLabel('overextended') === 'OVEREXTENDED'; regimeLabel(undefined) === '—'.
- V5 walletNockUnits({ wallet_nock_series: ['0.0000'] }, 0, 0.01153) === 0: a zero stack plots as 0, not as a gap. Also pin positionsSummary([]) === 'No LP positions tracked.' in the same block.
- V6 nativeDeltaPct(500, 0) === null, which renders no '% of native value'. harvestSharePct(0, 0) === null, which renders 'no harvests yet'. harvestSharePct(40, 100) === 40.

### Open questions

- Does chudmax's config.yaml set tracked_wallets, positions, operator_profile.treasury_floor_nock and all three target weights itself, or does it inherit some from config.yaml.example? The #86 test comment says positions were inherited on 2026-11-14. The example cleanup has to wait until the operator's config sets them all. After deploying, verify: 3 positions, nock_floor_target '100000', deviation targets 7500/2000/500.
- Should the example ship an active default price_source? It is market data, like eth_price_pool. If yes, which pool: the v4 3% USDC/NOCK pool 0x064a...6b13 (today's live price source, on a proven code path), or the deeper legacy Aerodrome vAMM 0x85f1...d626 (already the OHLCV pool)?
- DEFENSIVE LOCK with an active plan: is it acceptable to append the plan state to the action, for example 'DEFENSIVE LOCK (Floor Recovery) · DCA TRANCHE DUE (1 of 5)'? DEFENSIVE LOCK stays primary, as design note §8.2 requires, and tranche transitions start notifying again.
- Default target weights for a new user: none, or the original 80/15/5 (FZ-13)? The answer also decides whether a default 5% cash target can shrink a new user's tranches.
- For the dip arm with no plan, keep $0.020 as a facts-only trigger, or move it into config with no default? With no default, a user without a plan only ever sees the from-zero and HOLD arms.
- Directive wording: are 'BUILD FROM ZERO (No Buy Plan Configured)', 'NOTHING TO TRACK YET (Empty Wallet)' and 'DIP ZONE (No Buy Plan Configured)' right? The operator voices the lore; the tests pin whatever is chosen.
- Which snapshot spec version for the additive enum values (lp_rebuild.status 'no_positions', regime 'unknown'): 2.2.1 inside the v0.5.x patch line, or 2.3.0?
- Before shipping the ownership guard, confirm that ownerOf(3163450) on the v4 POSM and ownerOf(6336662) on the v3 NPM both return the tracked wallet. If an NFT sits in a staker or another contract, the guard needs a configured exception.
- Playtest: does alchemy_getTokenBalances return a parseable zero ('0x000...0') for a wallet that has never been funded? If it returns '0x', an empty wallet fails the cycle at main.rs:4891-4893 with 'Failed to query live token balances'. Run it locally: empty Postgres, a freshly generated read-only address, positions: [] and a price_source block.
- Gas note: is it acceptable to state 'the wallet holds no ETH; every swap on Base pays its gas in ETH' in from-zero advice? It is a fact with no amount, consistent with #94's gas pillar and with no rebalancing pressure on ETH. Should 'DCA TRANCHE DUE' advice say the same when ETH reads 0?

*Skeptic verification for this area was still running when this appendix was generated.*


## chart-gaps: Issue #61: the chart timeline collapses downtime gaps (time-series charts in frontend/src/App.tsx, GET /v1/snapshot/historical)

**Summary.** How history loads today: the dashboard calls GET /v1/snapshot/historical?limit=288 (client.ts:143-153, App.tsx:48-56). The worker returns rows newest first (main.rs:1997) and the dashboard reverses them (App.tsx:52). History reloads whenever the state hash of the latest snapshot changes (App.tsx:58-60, useFreshness.tsx:140-145), and retries every 60 s while it is empty (App.tsx:63-67). Each stored row becomes one chart row in four datasets (App.tsx:135-181, 249-258). None of these datasets ever contains a break row, and all four charts use categorical x-axes (App.tsx:513 stacked AreaChart, 572 ComposedChart, 682 LineChart, 800 LineChart). So any stretch without stored snapshots is drawn as one ordinary step, joined by a curve.

The installed Recharts 2.15.4 already breaks Lines and stacked Areas at rows where the series value is null or missing, and it hides the tooltip there. The smallest correct fix has two parts, and either one alone is incomplete:
1. A pure helper in frontend/src/timeline.ts that inserts a gap row with no series values when two consecutive rows are more than 2.5x the sync window apart.
2. A numeric time x-axis, with tick and tooltip label formatters.

Live data from chudmax: 175 rows, 2026-11-14T04:38:29.570516Z to 18:06:05.086105Z. There is no gap of several hours today. The only real gaps are single missed cycles: 05:43:58.370673Z to 05:52:47.460738Z (529.090 s) and 16:59:37.000680Z to 17:07:33.049608Z (476.049 s). The normal timing jitter shows that the issue's rule of "more than 2x the sync window" would break the line on some missed cycles and not others. The dashboard does not know the real sync cadence: it hardcodes 300 s (App.tsx:39), and only /health reports the real value.

While mapping the datasets I found a separate high-severity defect, finding 61-2. The Portfolio Composition chart (total vs native-only) plots each row's OLDEST counterfactual point, so its line and the Stable Reserve Delta readout lag by about 100 cycles (about 8 h). Live example: the 18:06:05 row shows $2,474.90 against its own $2,390.53. The file:line references in issue #61 are out of date. Current locations: datasets App.tsx:135-181, XAxis 513/572/682/800, ensure_price_history main.rs:3591, store_snapshot main.rs:6404. This work was read-only and I changed no files. The AGENTS.md change and the untracked docs/lore/ in the working tree are not mine.


### 61-1: Lines and stacked areas are drawn straight through intervals with no stored snapshots — skeptic: **confirmed**

- **Kind / severity / patch:** defect / high / yes
- **Locations:** `frontend/src/App.tsx:135-142`, `frontend/src/App.tsx:148-163`, `frontend/src/App.tsx:168-181`, `frontend/src/App.tsx:249-258`, `frontend/src/App.tsx:539-558`, `frontend/src/App.tsx:581-583`, `frontend/src/App.tsx:690-691`, `frontend/src/App.tsx:809-813`, `ingestion-worker/src/main.rs:6404-6442`
- **Evidence:** Every dataset maps stored rows one to one: history.map(...) at App.tsx:136, 149 and 169, and channel.center_line.map(...) at 250. Missing values are turned into 0 with parseFloat(... || '0') (App.tsx:123, 127, 153-155, 173-174), so no row ever carries a break. Nothing is written while the worker is down: store_snapshot only inserts when the state hash is new (main.rs:6404-6442), and there are no tombstone rows. Recharts then joins each pair of defined points with one monotone segment, however much time lies between them.

Live GET http://chudmax:8081/v1/snapshot/historical?limit=1000 returned 175 rows, 2026-11-14T04:38:29.570516Z to 18:06:05.086105Z, has_more=false. There is no gap of several hours yet. The real gaps are single missed cycles: 2026-11-14T05:43:58.370673Z to 05:52:47.460738Z (529.090 s), and 16:59:37.000680Z to 17:07:33.049608Z (476.049 s). In both cases the row after the gap is out of the previous 300 s rhythm, which points to a restart. The 19.3 h gap in the issue came from the pre-chudmax workstation database. It will come back with any outage, or on any new user's laptop that goes to sleep.

The installed Recharts 2.15.4 can draw breaks:
- Line.js:497 sets y to null when the value is nil.
- Area.js:482 marks a break point with `isBreakPoint = value[1] == null || hasStack && getValueByDataKey(entry, dataKey) == null`.
- Curve.js:42-44 and :97 pass a defined() check to d3.
- connectNulls defaults to false (Line.js:451, Area.js:409).
- **Impact:** A 19 h outage is drawn as a smooth move from the last value before the outage to the first value after it. Values that were never observed are shown as if they had been. This breaks the repo rule that no fabricated or interpolated numbers are shown as live data.
- **Proposed fix:** Add withGapBreaks in frontend/src/timeline.ts and wrap all four datasets with it. The gap row it inserts has no series keys, so every Line and every stacked Area breaks there. Recharts handles that row safely:
- Stacking counts it as 0 (ChartUtils.js:791-792) but does not draw it.
- Numeric y-domains ignore it (ChartUtils.js:79-84).
- Dots skip it (Dot.js:24).
- Animation keeps it null (DataUtils.js:98-106).
- The tooltip hides there, because filterNull defaults to true (Tooltip.js:72-75, :114) and the box is hidden when nothing is left (TooltipBoundingBox.js:144).

Set connectNulls={false} explicitly on every Area and Line. Label the gap neutrally, for example 'no snapshots for 19h 20m', never 'worker offline' (see 61-5).

### 61-2: The Portfolio Composition chart plots each history row's oldest counterfactual point, about 100 cycles (about 8 h) out of date — skeptic: **confirmed**

- **Kind / severity / patch:** defect / high / yes
- **Locations:** `frontend/src/App.tsx:168-176`, `frontend/src/App.tsx:695-706`, `ingestion-worker/src/main.rs:2037-2051`, `ingestion-worker/src/main.rs:2510-2523`, `ingestion-worker/src/main.rs:5917-5943`, `ingestion-worker/src/main.rs:6319-6324`
- **Evidence:** How the series is built and trimmed:
- compute_counterfactual_sync builds the series oldest first from the 100 most recent stored rows (ORDER BY generated_at ASC, main.rs:2510-2523) and adds the current point last (main.rs:5933-5936).
- slim_historical_payload uses truncate(1), which keeps the OLDEST point (main.rs:2045-2047). Its own comment says it keeps the current point (main.rs:2037-2042).
- The frontend reads [0] of each history row (App.tsx:173-174). That read has been there since commit d5f7b3f.

Live example: the newest row, 2026-11-14T18:06:05.086105Z, carries time_points[0]=2026-11-14T10:10:57.466343+00:00 with active 2474.90 and passive 1441.90. The same row's global_status says total_portfolio_usd 2390.53 and total_native_usd 1312.88. The Total Value chart reads a one-point series and correctly shows 2390.53.

After the fresh start, the first 101 rows (04:38:29 to 12:20:57.764168Z) all plotted the very first row's 2475.27. The row at 14:00:57.601534Z plots 2475.27 while its own total is 2379.06.
- **Impact:** - The 'Total Portfolio' line lags real time by about 8 h.
- After every fresh database it stays flat at the first value for about 8 h.
- The 'Stable Reserve Delta' readout, presented as current, uses the last history row (App.tsx:695-706). For the 18:06:05 row it shows +$1,033.00 (+71.6%), but that row's own totals give +$1,077.65 (+82.1%). This is stale data shown as live.
- **Proposed fix:** Not covered by any open issue, so file it on GitHub before writing the fix (AGENTS.md section 11).
- Frontend: build the counterfactual rows in history mode from each row's own dm.global_status.total_portfolio_usd and total_native_usd, using a pure counterfactualPoint(dm) helper that returns null when a value is missing. These are the same numbers as the series' current point, because main.rs:6319-6324 passes these totals into compute_counterfactual_sync.
- Backend: make slim_historical_payload keep the LAST point (remove all but the final element), so the payload matches its comment. Add a Rust test.

### 61-3: All four time-series charts use a categorical x-axis, so spacing ignores elapsed time — skeptic: **confirmed**

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `frontend/src/App.tsx:513`, `frontend/src/App.tsx:572`, `frontend/src/App.tsx:682`, `frontend/src/App.tsx:800`, `frontend/src/App.tsx:524-529`, `frontend/src/App.tsx:576-580`, `frontend/src/App.tsx:685-689`, `frontend/src/App.tsx:803-808`, `frontend/src/App.tsx:1062-1068`
- **Evidence:** `<XAxis dataKey="time" .../>` sets no type, so Recharts uses its default type 'category' (XAxis.js:84) with allowDuplicatedCategory true (:92). Every row gets one equal slot, keyed by a formatted label string (formatSnapshotTime, App.tsx:1062-1068).

The charts:
- 513: stacked AreaChart (the wallet NOCK Area plus one Area per position, stackId 1).
- 572: ComposedChart with 3 Lines on two y-axes.
- 682: LineChart with 2 Lines.
- 800: LineChart with 5 Bollinger Lines, built from the latest snapshot's channel points with real times.

Live: the 529 s gap after 05:43:58.370673Z takes one slot, the same as a normal 300 s step. Three manually triggered rows at 05:38:52.168138Z, 05:38:54.342099Z and 05:38:56.881262Z, only 2.2 s and 2.5 s apart, each take a full slot with the same '05:38' label.
- **Impact:** Elapsed time cannot be read from the x-axis: outages shrink to one step, and bursts of manual ingests are stretched out. Even with the gap row from 61-1, a 19h 20m outage would only take one extra slot; the duration would appear only in the label.
- **Proposed fix:** Give every row a field t (epoch milliseconds). Change the four axes to:
`dataKey="t" type="number" scale="time" domain={['dataMin','dataMax']} tickFormatter={(t) => formatSnapshotTime(t, spansMultipleDays)}`
and widen formatSnapshotTime to accept string or number.

Add a labelFormatter to the four Tooltips; otherwise their headers show raw epoch milliseconds.

Notes on how Recharts behaves with this axis:
- With scale='time' it computes no rounded ticks (ChartUtils.js:868-871). It uses one tick per data row instead (generateCategoricalChart.js:311-313, 363-365; ChartUtils.js:520-528), and hides overlapping labels (default interval='preserveEnd'). Optionally pass ticks taken from real rows only.
- Stacking is done per row, so stacked Areas work on a numeric axis.
- Each point's x is scale(t) (ChartUtils.js:912-913), so every gap row needs a real number for t. Use the midpoint of the gap.

### 61-4: The issue's 'more than 2x the sync window' rule falls right on the span of one missed cycle, so it fires about half the time — skeptic: **confirmed**

- **Kind / severity / patch:** design / medium / yes
- **Locations:** `ingestion-worker/src/main.rs:1705-1716`, `ingestion-worker/src/main.rs:2637`, `frontend/src/timeline.ts (new)`
- **Evidence:** The scheduler fires every sync_window_seconds (main.rs:1705-1716), but generated_at is taken after the RPC fetch (Utc::now() at main.rs:2637), so the intervals jitter.

Live chudmax: 150 of 174 intervals are 297.954-302.077 s. One skipped or deduplicated cycle leaves a span of two normal intervals. Measured on consecutive pairs, those spans run from 598.058 to 602.068 s, and 71 of 138 exceed 600 s.
- **Impact:** With a strict 600 s cut, about half of the single missed cycles break the line and the other half do not. The same kind of event would look different depending on a second or two of jitter.
- **Proposed fix:** Use 2.5x the cadence (750 s at 300 s). A break then means at least two cycles in a row left no row (a gap of about 898 s or more), and a single missed or deduplicated cycle never breaks the line.

If the operator wants every missed cycle shown, use 1.5x (450 s) instead; that would break at both live restart gaps (529 s and 476 s). In either case, avoid whole-number multiples of the cadence.

### 61-5: Row spacing cannot tell downtime apart from deduplicated cycles, so quiet or empty portfolios would show false gaps — skeptic: **confirmed**

- **Kind / severity / patch:** risk / medium / no
- **Locations:** `ingestion-worker/src/main.rs:6412-6419`, `ingestion-worker/src/main.rs:2671-2674`, `ingestion-worker/src/main.rs:6362-6402`
- **Evidence:** store_snapshot returns early without inserting a row when the new state hash equals the latest stored hash (main.rs:6412-6419). The in-memory latest snapshot still moves forward (main.rs:2674). Wall-clock fields are removed before hashing (main.rs:6362-6402).

Deduplication is not firing for the operator today. Consecutive rows at 2026-11-14T04:53:40.926301Z and 04:58:40.767110Z have identical slimmed payloads but different hashes (fb8ae5183c6c18a7 vs e5824c3bb71b96e7). The likely reason is that the embedded 100-row counterfactual window and 50-row channel window change every cycle.

The NOCK spot price stayed at 0.01229 for 109 stored rows, from 04:38:29.570516Z to 13:00:57.423538Z (8h 22m).
- **Impact:** Take a new user with an empty portfolio. Once those windows fill with identical zeros, an unchanged spot price will likely produce the same hash every cycle. A flat-price stretch like the one above would then be stored as a single row and drawn as 'no snapshots for 8h 22m', even though the worker observed every cycle. The operator's portfolio does not show this today.
- **Proposed fix:** For 0.5.1: keep the gap label literal ('no snapshots for ...', never 'offline') and pin the wording with a test.

Follow-up (outside this patch): when deduplicating, record an observed_through time on the latest row instead of returning early. This needs a new nullable column and an UPDATE. Expose it in the historical metadata, so the frontend measures a gap from the previous row's observed_through to the next row's generated_at.

### 61-6: The dashboard does not know the worker's real sync cadence — skeptic: **confirmed**

- **Kind / severity / patch:** gap / low / yes
- **Locations:** `frontend/src/App.tsx:39`, `frontend/src/hooks/useFreshness.tsx:4`, `frontend/src/api/client.ts:103-111`, `ingestion-worker/src/main.rs:2026-2033`, `ingestion-worker/src/main.rs:1851`, `specs/core/api.yaml:427-436`, `specs/core/snapshot.yaml:53`
- **Evidence:** App.tsx:39 passes syncWindowSeconds: 300, and useFreshness also defaults to 300 (useFreshness.tsx:4).
- SnapshotMeta has no cadence field (snapshot.yaml:53; client.ts:9-14), and neither does the historical response (main.rs:2026-2033; api.yaml:427-436).
- Only /health reports sync_window_seconds (main.rs:1851; api.yaml:417-419), and the dashboard never calls api.health (its only caller is client.errors.test.ts:42).
- The cadence is configurable (main.rs:220-221, default 300 at 255-257), and the test config uses 60 (main.rs:8251). Live /health reports 300 today.
- **Impact:** A gap threshold built on the hardcoded 300 is wrong for any other cadence.
- At 900 s, every interval would exceed 750 s and every segment would break. Areas need two defined points in a row, so the charts would go empty.
- At 60 s, up to 12 missed cycles in a row would not be flagged.
- The stale banner already has the same hidden problem.
- **Proposed fix:** Add `"sync_window_seconds": state.config.sync_window_seconds` to the historical response body. Declare it as optional in the HistoricalResponse schema in api.yaml and in the HistoricalResponse type in client.ts. Store it next to the history in App and pass it to both gapThresholdMs and useFreshness. Fall back to 300 when it is missing.

Frontend-only alternative: call api.health() once on load.

### 61-7: The issue's backend item 1 (backfill snapshots across downtime) should not be built — skeptic: **confirmed**

- **Kind / severity / patch:** design / low / no
- **Locations:** `ingestion-worker/src/main.rs:3591`, `ingestion-worker/src/main.rs:1696`, `ingestion-worker/src/main.rs:2452`
- **Evidence:** ensure_price_history (main.rs:3591, run at boot at 1696 and every cycle at 2452) backfills third-party market price candles and records where they came from. That is observed market data.

Portfolio snapshots (balances, positions, fees) for a period the worker did not observe could only be rebuilt by reading archive state at historical blocks. Anything else is interpolation.
- **Impact:** Made-up rows would show unobserved values as on-chain truth, against the 'mock data is a bug' rule (AGENTS.md section 11). The 2026-11-14 decision on #96 also treats the database as disposable, with the chain as the record.
- **Proposed fix:** Re-scope the backend half of #61: add the cadence field (61-6) now, and the observed_through record (61-5) later. Say in the issue that gaps are shown, never filled. Rebuilding history from archive blocks, if ever wanted, is a separate feature.

### Proposed design

Plan for 0.5.1: a frontend fix, plus one optional backend field and a one-line backend fix.

1. New pure module frontend/src/timeline.ts. It has no React or locale code.
- `DEFAULT_SYNC_WINDOW_SECONDS = 300`, matching default_sync_window in main.rs:255-257.
- `GAP_CADENCE_MULTIPLIER = 2.5`.
- `gapThresholdMs(syncWindowSeconds?: number | null)`: returns s * 2.5 * 1000, where s is the given cadence if it is a finite number above 0, otherwise 300. The default result is 750000.
- `formatGapDuration(ms)`: rounds to whole minutes and returns '15m', '1h 0m', '19h 20m' or '1d 1h'.
- `withGapBreaks<T extends { t: number }>(rows, thresholdMs)` keeps the rows in order and returns the same objects. Between two consecutive rows, if dt = rows[i].t - rows[i-1].t is a finite number above thresholdMs, it inserts a gap row: `{ gap: true, t: start + Math.floor(dt / 2), time: 'no snapshots for ' + formatGapDuration(dt), gapStart, gapEnd, gapMs: dt }`. It never adds a gap row at the start or end. A NaN, zero or negative dt never produces one. The gap row deliberately has no series keys, so every Recharts dataKey reads undefined there.
- `counterfactualPoint(dm)`: returns { active, passive } from dm.global_status.total_portfolio_usd and total_native_usd, or null when a value is missing. Fixes 61-2.

2. Changes in App.tsx.
- Give every row a field t = Date.parse(...). Use generated_at for history rows, the time_points string in the fallback path, and LinePoint.time for the channel. This applies to assetSeries (135-142), valueVsBaseline (148-163), counterfactual (168-181, now using counterfactualPoint) and channelData (249-258), in both the history path and the latest-snapshot fallback path.
- Wrap each of those arrays in `withGapBreaks(rows, gapThresholdMs(syncWindowSeconds))`.
- The Stable Reserve Delta readout (696-698) must pick the last row that is not a gap row. Otherwise strict tsc (tsconfig strict: true) fails on the union type.

3. Axes and tooltips.
- Replace the four `<XAxis dataKey="time" .../>` (513, 572, 682, 800) with `dataKey="t" type="number" scale="time" domain={['dataMin','dataMax']} tickFormatter={(t) => formatSnapshotTime(t, spansMultipleDays)}`, and widen formatSnapshotTime (App.tsx:1062) to accept string or number.
- Add a labelFormatter to the four Tooltips (524-529, 576-580, 685-689, 803-808).
- Set connectNulls={false} explicitly on every Area and Line (539-558, 581-583, 690-691, 809-813). It is already the default in 2.15.4 (Line.js:451, Area.js:409), but setting it records the intent.
- Optional: a ReferenceArea from gapStart to gapEnd for each gap row, labelled with its time text, to shade the hole; and ticks taken from real rows only, so no tick label lands on a gap midpoint.

4. Cadence. Add sync_window_seconds to the historical response body (main.rs:2026-2033), to api.yaml HistoricalResponse (427-436) and to the HistoricalResponse type in client.ts (103-111). Pass it to gapThresholdMs and useFreshness, replacing the hardcoded 300 at App.tsx:39, and fall back to 300 when it is missing. A frontend-only alternative is to call api.health() once on load.

5. Backend.
- Make slim_historical_payload (main.rs:2043-2047) keep the last, current counterfactual point.
- Never backfill snapshots.
- Follow-up, outside this patch: an observed_through record for deduplicated cycles (61-5).

Why both parts are needed, checked against the installed Recharts 2.15.4 and the actual chart types:
- The gap row alone, on the current categorical axis, breaks every Line and every stacked Area: Area.js:482 marks a break for each stacked series whose own value is null; the baseline goes null too, and the d3 defined() check splits the polygon (Curve.js:42-44, 97). But a 19h 20m outage still takes just one extra slot, and the duration shows only in the label.
- The numeric axis alone spaces points correctly, but d3 still joins the two points at the edges of the outage with one long monotone segment, so the false continuity stays.
- Together they give gaps of the right width, with nothing drawn and the tooltip hidden inside them.
- Effects on charts that need checking are all safe:
  - Stacked Areas: the stack counts the gap row as 0 but does not draw it, and the stacked y-domain already starts at the 0 baseline.
  - Line charts (including the ComposedChart with two y-axes): numeric domains drop undefined values (ChartUtils.js:79-84), dots skip the row (Dot.js:24), and animation keeps it null (DataUtils.js:98-106).
  - Tooltips: in the middle of a gap the tooltip snaps to the gap row and hides itself (Tooltip.js:72-75 filterNull, TooltipBoundingBox.js:144).
- History still loads by row count (288 rows). After an outage, the numeric axis will honestly span more than 24 h.

### Tests to add

- frontend/src/timeline.test.ts (picked up by the default `vitest run` include). gapThresholdMs: gapThresholdMs(300) === 750000; gapThresholdMs(60) === 150000; gapThresholdMs(undefined), gapThresholdMs(null), gapThresholdMs(0), gapThresholdMs(-300) and gapThresholdMs(NaN) all === 750000 (falls back to the worker default of 300 s).
- withGapBreaks leaves a regular cadence untouched (live chudmax rows). Input rows have t = 1794678064995 (2026-11-14T17:41:04.995794Z), 1794678365096 (17:46:05.096807Z) and 1794678664986 (17:51:04.986024Z); threshold 750000. Expected: the output toEqual(rows), and out[0] toBe(rows[0]).
- withGapBreaks does not break for one missed cycle (live restart). Rows t = 1794635038370 (2026-11-14T05:43:58.370673Z) and 1794635567460 (05:52:47.460738Z), so dt = 529090; threshold 750000. Expected: output length 2.
- No flip-flop at the one-missed-cycle span. [{t:0},{t:598058}] and [{t:0},{t:602068}] both give length 2 at threshold 750000. For contrast, a strict 2x threshold of 600000 gives 2 and 3 respectively; these are the observed live bounds of one skipped cycle.
- The threshold is exclusive: [{t:0},{t:750000}] gives length 2; [{t:0},{t:750001}] gives length 3.
- Issue #61 downtime, 12:38 AM to 7:58 PM EST. before = {t:1794548280000, portfolio:2475.27} (2026-11-13T05:38:00.000Z); after = {t:1794617880000, portfolio:2390.53} (2026-11-14T00:58:00.000Z); threshold 750000. Expected: toEqual([before, {gap:true, t:1794583080000, time:'no snapshots for 19h 20m', gapStart:1794548280000, gapEnd:1794617880000, gapMs:69600000}, after]), and ('portfolio' in out[1]) === false.
- Two gaps, none at the start or end. Rows t = [0, 300000, 3900000, 4200000, 90000000]. Expected: out.map(r => 'gap' in r ? 'gap' : r.t) equals [0, 300000, 'gap', 3900000, 4200000, 'gap', 90000000], and out[out.length - 1] toBe(rows[4]), because the Stable Reserve Delta readout reads the last row.
- Fresh start with an empty database (from-zero user). withGapBreaks([], 750000) returns []. A single row [{t:1794680164649}] (2026-11-14T18:16:04.649290819Z) returns that same one row with no gap row.
- Bad input never creates a gap: [{t:NaN},{t:10000000}] gives length 2; out-of-order [{t:10000000},{t:0}] gives length 2.
- Configured cadence: withGapBreaks([{t:0},{t:180000}], gapThresholdMs(60)) gives 3 rows, with out[1] toEqual {gap:true, t:90000, time:'no snapshots for 3m', gapStart:0, gapEnd:180000, gapMs:180000}.
- Quiet or empty portfolio wording, documenting finding 61-5. Rows t = 1794631109570 (2026-11-14T04:38:29.570516Z) and 1794661257423 (13:00:57.423538Z). Expected out[1] toEqual {gap:true, t:1794646183496, time:'no snapshots for 8h 22m', gapStart:1794631109570, gapEnd:1794661257423, gapMs:30147853}, and out[1].time does not contain 'offline'.
- formatGapDuration: 900000 gives '15m'; 3600000 gives '1h 0m'; 69600000 gives '19h 20m'; 30147853 gives '8h 22m'; 90000000 gives '1d 1h'.
- counterfactualPoint (finding 61-2), using the live row from 2026-11-14T18:06:05.086105Z. Input: {global_status:{total_portfolio_usd:'2390.53', total_native_usd:'1312.88'}, strategy_counterfactual:{time_points:['2026-11-14T10:10:57.466343+00:00'], active_portfolio_series:['2474.90'], passive_baseline_series:['1441.90']}}. Expected: {active:2390.53, passive:1312.88}. counterfactualPoint({}) returns {active:null, passive:null}.
- Rust test in ingestion-worker/src/main.rs: slim_historical_payload keeps the current point. Start from sample_snapshot_data() and set strategy_counterfactual time_points ['2026-11-14T10:10:57.466343+00:00', '2026-11-14T18:06:05.086105+00:00'], active ['2474.90', '2390.53'], passive ['1441.90', '1312.88']. After slim_historical_payload, expect time_points ['2026-11-14T18:06:05.086105+00:00'], active ['2390.53'], passive ['1312.88'], and structural_channels.channels empty.
- Rust test, only if finding 61-6 is done through the historical response: move the JSON body into a pure function and assert it includes "sync_window_seconds": 300 alongside snapshots and pagination.

### Open questions

- Gap threshold: should it be 2.5x the cadence (only two or more empty cycles in a row break the line, so the deploy restarts at 05:43:58 and 16:59:37 stay joined) or 1.5x (every missed cycle breaks the line, including each deploy restart)?
- Should the observed_through record for deduplicated cycles (61-5) go into 0.5.1, or into a follow-up issue? It only matters for quiet or empty portfolios, such as a new user starting from zero.
- Should the dashboard load the last 24 h of history (after = now minus 24 h, with a higher limit) instead of the last 288 rows? Then the numeric time axis would always cover one day, even after an outage.
- 61-2 (the out-of-date composition chart and Stable Reserve Delta) is not covered by any open issue. Should it be filed separately (AGENTS.md section 11) and shipped in 0.5.1 alongside #61? Should #61's out-of-date line references and its backend backfill item be updated at the same time?

### Skeptic: problems with the proposed design

- The gap text never reaches the user. With dataKey='t' and a tickFormatter, the gap row's `time` field ('no snapshots for 19h 20m') goes unused. The tooltip hides on gap rows (Tooltip.js:72-77; TooltipBoundingBox.js:144), and the gap row still gets an axis tick (categoricalDomain, generateCategoricalChart.js:364) labelled with the formatted midpoint, a time when nothing was observed. The ReferenceArea or caption, and ticks taken from real rows only, must be required rather than optional; otherwise the 61-5 wording test pins text that no one sees.
- The counterfactualPoint contract contradicts itself. The design says it returns null when a value is missing; the test expects counterfactualPoint({}) to return {active:null, passive:null}. With the object-of-nulls form, the Stable Reserve Delta readout (App.tsx:696-700) computes null - null = 0 and prints '+$0.00 USD (+0.0% ...)', a made-up zero. The readout is presented as current, so it should read the latest snapshot's `global` (App.tsx:90) directly, not the last history row.
- The 61-2 fix (active = total_portfolio_usd, passive = total_native_usd) builds in a mislabel and conflicts with today's #94 decision. Total - Native equals wallet USDC + LP USDC + every non-USDC LP leg (main.rs:6029-6031). Live row 18:06:05.086105Z: 2390.53 - 1312.88 = 1077.65, while total_stable_usd = 1033.71. The roughly $44 difference is the v3 position's 0.023918 WETH, and App.tsx:704 calls it 'the USDC side'. Once #94 adds wallet ETH/WETH to total_portfolio_usd, the 'Stable Reserve' would also take in gas ETH, which the operator ruled is not cash and not reserve. Use total_stable_usd (already in global_status) for the stable side, or chart the NOCK, USDC and ETH pillars separately, and update the proposed test expectations.
- The design keeps parseFloat(x || '0') in assetSeries, valueVsBaseline and channelData (App.tsx:123, 127, 153-155, 160-162, 252-256). A value missing from a real row is still drawn as a real 0 instead of breaking like a gap row; a missing spot would also pull the right axis down to $0. Only counterfactualPoint returns null. Use one Number.isFinite-to-null parser everywhere.
- Isolated observations disappear. A real row with gap rows on both sides (the new user's laptop that wakes for one cycle, which 61-1 itself cites, or a worker in a crash loop) has no defined neighbour. The stacked Areas and every Line without dots (highWaterMark, passive, all five Bollinger lines) draw nothing for it, because d3 needs two consecutive defined points (Curve.js:97, 107). Observed data would be hidden. Render a dot for rows whose neighbours are gaps.
- Only the pure helpers get tests. App.tsx mounts the app at import (createRoot, App.tsx:1097-1109), so the inline dataset builders the design changes stay out of vitest's reach. That covers the t assignment, the fallback path a from-zero user hits with 0-1 rows (hasHistory, App.tsx:98), the counterfactual rows, the readout row selection, and the position keying. Move the builders into timeline.ts (for example buildChartRows(history, latest, syncWindowSeconds)) and test them with from-zero fixtures: 0, 1 and 2 rows, and zero balances.
- Wrapping assetSeries in withGapBreaks leaves the index-keyed per-position series unchanged (App.tsx:108-111, 126-129), so the stacked chart keeps misattributing LP NOCK whenever the tracked position list changes (first missed item).

### Skeptic: missed in this area

- **The stacked NOCK chart keys LP series by array index. History rows from before a position-list change are drawn under the wrong positions, and some LP NOCK is dropped.** (`frontend/src/App.tsx:108-111`, `frontend/src/App.tsx:126-131`, `frontend/src/App.tsx:547-558`): The Area definitions take the LATEST snapshot's positions_nock order: pos0 = v4 3163450, pos1 = v3 6336662, pos2 = Aero 0x770f... But each history row sets row[`pos${j}`] from its own list.

Live: 24 of the 180 rows (15:23:43.021216Z to 17:07:33.049608Z) have 4-6 positions, with the retired v4 3163431/3163432 at indexes 0-1.

Row 16:14:59.513104Z holds [3163431: 0, 3163432: 0, 3163450: 9,363.08, 6336662: 10,958.96, aero 0x85f1: 0] and wallet 95,598.98. It is drawn as:
- v4 3163450 = 0
- v3 = 0
- Aero 0x770f = 9,363
- the v3's 10,959 NOCK (index 3) has no Area at all.

The stacked top is 104,962 against the row's own total_nock_amount of 115,921, drawn against the floor line.

Fix: key the series by position_id; build Area definitions from the union of ids in the loaded history, labelling retired ones; leave absent ids undefined so they break. File it on GitHub first (AGENTS.md section 11).
- **'Stable Reserve Delta' and 'Total (Native + USDC)' already count the v3 LP's WETH leg as USDC** (`frontend/src/App.tsx:679`, `frontend/src/App.tsx:695-706`, `ingestion-worker/src/main.rs:6029-6031`): total_portfolio_usd = lp_usd + t3_usd + wallet_usdc, and total_native_usd = (lp_nock + t3_nock) * spot, so Total - Native includes every non-USDC LP leg.

Live latest (18:36:04Z): 2390.56 - 1312.98 = 1077.58, while total_stable_usd = 1033.61. The difference matches the v3 position's 0.023918 WETH: 178.17 - 11,637.84 * 0.01153 = 43.99.

The chart title and readout say 'USDC'. Under the 2026-11-14 #94 decision (ETH is gas, not cash, not reserve), this needs explicit stable and ETH fields, not a subtraction.
- **From-zero readouts show values that were never measured** (`frontend/src/App.tsx:700`, `frontend/src/App.tsx:704`, `frontend/src/App.tsx:266-267`): `deltaPct = passiveUsd > 0 ? ... : '0.0'` means a new user who funds USDC before buying any NOCK sees '+$500.00 USD (+0.0% of native value)'. The percentage is undefined, not 0.0%.

`nockAmountRaw > 0 ? ... : '—'` shows a real, observed 0 NOCK balance as unknown.

Neither case is covered by the from-zero tests the operator asked for today.
- **Under deduplication the dashboard shows a healthy worker as stale** (`ingestion-worker/src/main.rs:1890-1898`, `ingestion-worker/src/main.rs:2674`, `frontend/src/api/client.ts:141`, `frontend/src/hooks/useFreshness.tsx:146`): The ETag is the state_hash. If-None-Match equal to it returns 304 (verified live: 304 for etag 2f24b0a9...).

On a deduplicated cycle the in-memory envelope gets a new generated_at but keeps the same state_hash. The browser's no-cache revalidation gets 304 and hands useFreshness the cached body with the first-seen generated_at. After 390 s the StaleBanner reports stale data while /health says ok.

This is latent for the operator, whose hash changes every cycle, but it hits exactly the quiet or from-zero portfolios described in 61-5.

Fix: put generated_at or snapshot_id in the ETag, or measure freshness from /health.
- **RSI(5M) and the Bollinger SMA(20) use the last N stored rows regardless of time gaps** (`ingestion-worker/src/main.rs:2465-2484`, `ingestion-worker/src/main.rs:2548-2578`, `frontend/src/App.tsx:408`, `frontend/src/App.tsx:796-819`): The channels use the 50 most recent stored rows, and rsi_5m uses the 50 most recent stored spots, with no check on spacing. After an outage or a long dedup stretch, consecutive '5-minute' closes can be hours apart, while the chip still says RSI(5M).

Breaking the drawn channel at the gap does not fix the band values after it, because their 20-row windows straddle the outage.

Low severity: document it, or restart the windows at gaps. Not needed for the 0.5.1 patch.


## dry-powder: Issue #63: dry powder computed from LP-paired USDC

**Summary.** #63 is only partly fixed at HEAD. Its headline repro (advising $178.40 when only $37.44 is liquid) no longer happens with the default 5% cash weight. v0.5.0 caps the dip spend at wallet USDC minus the cash target (main.rs:6053-6056).

These parts are still defects:
(A) Sizing. The tax-reserve term subtracts the reserve from wallet + LP USDC (main.rs:6050). Whenever the reserve is larger than the cash target, the advised spend uses up the wallet USDC that should back the reserve. Worked case: $171.75 advised instead of $100.00. With the issue's own numbers plus a $20 reserve and cash weight 0, it advises the whole $37.44 wallet.
(B) Wording. The DIP REINVESTMENT text prints wallet + LP USDC as '$X available' (6149-6150). On today's live wallet with no plan it would read '$1033.71 available' while the wallet holds $930.78. It also cites a fixed 'toward 120,000' that comes from no config value.
(F) Guardrail. The tax guardrail counts LP USDC as covering the reserve (6051, 6141-6142). That is a defect by the issue's own reasoning and by the plan and rebuild code, which keep the reserve in wallet USDC only (1142, 6086/6095). Once (A) is fixed it can no longer cause an over-spend, so it is low severity.
#63 item 3 is half done: dust-sized advice is suppressed by the $1 floor, but nothing says the dry powder is used up. That state falls through to HOLD text with fixed claims ('Dual Growth Engine Active', 'Uniswap position(s) generating yield', '0% realized loss'). #63 item 4 is not done: the MCP portfolio text does not split wallet and LP USDC, and it hardcodes '100k' as the floor.

growth_dry_powder_usd is not computed from LP USDC. It is 60% of all harvests ever recorded, an accounting figure, yet the spec and dashboard call it 'Liquid dry powder available' and 'DEPLOYABLE'. The 40%/60% labels are fixed text even though the ratio is configurable, and '✓ TAX RESERVE PROTECTED' always shows.

One finding goes beyond the issue text. With no plan configured, the dip arm advises spending wallet USDC down to the 5% cash target. That contradicts OPERATOR_GUIDANCE §7.6-7.7 ('not permission to spend down to it'; buys are paced by the plan, never invent defaults) and today's 'cash in my pocket' position. It also picks a lump-sum budget for every from-zero user. The recommended fix retires that arm and reports liquid dry powder as a fact in HOLD.

Not flagged, by design:
- cash_reserve_usdc counting wallet + LP USDC (operator decision)
- the honest totals: total_stable_usd, 'Total USDC Stack', 'Stable Stack'
- the deviation matrix's 'Liquid USDC' bucket, which already uses wallet USDC only

I built and ran nothing. Expected values were worked out by hand and in Python using the engine's floating-point order of operations; live figures come from GET http://chudmax:8081/v1/snapshot/latest (2026-11-14 18:11Z).


### 63-A-sizing-tax-term: Dip-arm sizing still lets LP USDC stand in for the tax reserve — skeptic: **confirmed**

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `ingestion-worker/src/main.rs:6049-6056`, `ingestion-worker/src/main.rs:6031`, `ingestion-worker/src/main.rs:1141-1147`, `ingestion-worker/src/main.rs:6083-6095`, `CHANGELOG.md:105-106`
- **Evidence:** main.rs:6050 `let deployable_usdc = (total_stable_usd - tax_reserve_usd).max(0.0);` (total_stable_usd = lp_usdc + wallet_usdc, 6031) is limited only by `.min((wallet_usdc - cash_target_usd).max(0.0))` (6056). Nothing keeps the reserve itself in wallet USDC.
Worked case, no plan, spot $0.01831: LP USDC $140.96 (20.00 + 120.96), wallet $300, 110,000 wallet NOCK + 6,000 LP NOCK, harvest ledger $500 (reserve $200), cash target $128.25 (5% of $2,564.92). HEAD advises 'Deploy $171.75', leaving $128.25 in the wallet against a $200 reserve. With the issue's own numbers ($37.44 wallet, $140.96 LP), a $20 reserve and cash weight 0, HEAD advises 'Deploy $37.44': the whole wallet.
Everywhere else the reserve is wallet-only. plan_directive caps at `guards.wallet_usdc - guards.tax_reserve_usd` (1142). The LP rebuild keeps `cash_target_usd.max(tax_reserve_usd + plan_remaining)` in wallet USDC (6086, 6095). CHANGELOG.md:105-106 notes a range converts its USDC to NOCK on a dip, which is exactly when this arm fires.
- **Impact:** No live impact today: the operator's plan turns this arm off and the harvest ledger is empty. But any user without a plan who has harvest history can be told to spend the USDC that backs the virtual tax reserve. The 'protected' reserve then sits in LP USDC, which a falling price turns into NOCK.
- **Proposed fix:** Replace 6050 and 6056 with `let deployable_usdc = (wallet_usdc - tax_reserve_usd.max(cash_target_usd)).max(0.0);`. This is the same rule plan_directive and the rebuild's liquid floor use: reserve and cash target overlap, and the larger one stays in the wallet. Expected results: $100.00 in the first example, $17.44 in the second.

### 63-B-available-label: DIP REINVESTMENT text calls wallet + LP USDC 'available' and cites a fixed 120,000 target — skeptic: **confirmed**

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `ingestion-worker/src/main.rs:6145-6152`
- **Evidence:** 6149-6150: 'Deploy ${:.2} of saved USDC (${:.2} available; ${:.2} protected as virtual tax reserve, ${:.2} kept as cash target) to purchase NOCK at discount and compound buffer toward 120,000.', with total_stable_usd as the 'available' figure.
Live (chudmax, 18:11Z): wallet USDC 930.7834; LP USDC 102.92 (v4 3% 71.04 + Aero 31.88); total_stable_usd 1033.71; spot 0.01153; cash weight 500 bps. Without the accumulation_plan section this arm would print 'Deploy $811.25 of saved USDC ($1033.71 available; $0.00 protected as virtual tax reserve, $119.53 kept as cash target) ...'.
120,000 is not a config value; the floor is profile.treasury_floor_nock.
- **Impact:** LP working capital is shown as liquid cash. That is the core #63 data-integrity complaint, still present in the text for every user without a plan. The 120,000 target is a number picked for the user.
- **Proposed fix:** Print wallet USDC ('$W in the wallet'). Add a separate LP note: ' The $Y of USDC inside LP is working capital, not dry powder: it stays in its ranges.' Drop 'and compound buffer toward 120,000'. If 63-C retires the arm, this text goes with it.

### 63-C-noplan-lump-sum: With no plan, the dip arm sizes one lump-sum buy down to the cash target (breaks standing orders; affects from-zero users) — skeptic: **partly**

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `ingestion-worker/src/main.rs:6145-6152`, `ingestion-worker/src/main.rs:6071-6073`, `OPERATOR_GUIDANCE.md:109-114`, `README.md:55-56`
- **Evidence:** With no accumulation_plan and spot < $0.020, the arm says 'Deploy $X', where X is all wallet USDC above the cash target and reserve.
OPERATOR_GUIDANCE.md:113: 'The default 5% cash target is a long-run weight, not permission to spend down to it'. :110: purchases are paced by the accumulation_plan, 'never invent defaults'.
The plan path already turned this arm off for the same reason (6071-6072: 'a completed plan must not be followed by "deploy everything that is left"').
From zero: a new user with treasury_floor_nock 0, $500 wallet USDC and an empty Aero price-source position gets 'Deploy $475.00 of saved USDC ($500.00 available; $0.00 protected as virtual tax reserve, $25.00 kept as cash target) ... toward 120,000'. That budget comes from a 5% default the user never set.
The operator said today that cash in pocket is not ripped out at an 'investment low' sign; the DCA timer is the mechanism. README.md:55-56 still says dry powder is 'deployed on market retractions'.
- **Impact:** Every user without a plan is told to deploy about 95% of liquid USDC at once. That includes every from-zero user, and the operator if the plan section is ever removed: $811.25 of today's $930.78, below the $950 stable reserve, which exists only inside the plan section.
- **Proposed fix:** Delete the arm (6145-6152) and carry the facts in the HOLD dip note (63-D): 'Liquid dry powder: $X (wallet USDC $W less the larger of the $C cash target and the $T virtual tax reserve). ... NockNock does not size a lump-sum buy or an outside deposit: budget, tranches and interval are your decision, set in accumulation_plan.' HOLD is on apply_lp_rebuild_bias's idle list, so a fundable LP rebuild still leads.
Alternative: an informational 'DIP ZONE (No Accumulation Plan)' arm placed just before HOLD, after BATCH HARVEST, and added to the idle list. This keeps a notification when spot crosses $0.020. Confirm with the operator (open question 1).
- **Skeptic's correction:** The sizing is as described and conflicts with OPERATOR_GUIDANCE.md:110 and :113. Corrections: (1) Keeping the arm was a recorded decision. docs/kb/design-note-dca-accumulation-plan.md:147-148 says 'With no plan configured the existing arm stays as it is', with only the cash-target cap added, and :321 records it as shipped. Two same-day documents conflict, so this needs an operator ruling; it is not a plain defect. (2) 'Every from-zero user' is overstated. The default floor (main.rs:50, 100,000) and config.yaml.example:95 (the base layer since #86) send any newcomer holding under 100k NOCK to DEFENSIVE LOCK first. The $475 example needs treasury_floor_nock 0. (3) 'About 95% of liquid USDC' holds only for USDC-only portfolios, because the target is 5% of the whole portfolio. The operator's case is $811.25 of $930.78, about 87%. The from-zero figures ($475.00 deploy, $500.00 available, $25.00 target) recompute correctly.

### 63-D-depleted-fallthrough: When the dip zone has no dry powder, advice falls through to HOLD text with fixed claims and no explanation — skeptic: **confirmed**

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `ingestion-worker/src/main.rs:6176-6183`, `frontend/src/positions.ts:21-25`
- **Evidence:** #63 item 3 asked for an explicit 'dry powder depleted' state. At HEAD the arm is skipped when deployable < $1 (correct). The cycle then lands on 'HOLD & ACCRETE (Dual Growth Engine Active)' with '{n} Uniswap position(s) generating yield (+$F uncollected). 0% realized loss. Maintain posture.' (6178-6180).
Nothing says the dip boundary was crossed, or that LP USDC is not dry powder. '0% realized loss' is literal text; NockNock computes no realized P/L, and the dashboard's copy of it was removed as a bug in 0.5.0 (positions.ts:23). 'Uniswap' miscounts Aero positions.
The issue's own state ($37.44 wallet, $140.96 LP, spot $0.01831) gets this text at HEAD.
From zero (floor 0, empty wallet, empty Aero price source): 'HOLD & ACCRETE (Dual Growth Engine Active)' / 'Total NOCK held is 0 (+0 above 0 hard floor). 1 Uniswap position(s) generating yield (+$0.00 uncollected). 0% realized loss. Maintain posture.'
- **Impact:** Fixed claims are presented as live state on the dashboard banner and in MCP get_shot_advice, and a user with no dry powder is never told why nothing is advised.
- **Proposed fix:** Label: keep '(Dual Growth Engine Active)' only when lp_usd > 0, else '(No Active LP)'.
Details: 'Total NOCK held is {n} (+{d} above {f} hard floor). {k} tracked LP position(s) holding ${lp_usd} (+${fees} uncollected fees).{dip_note} Maintain posture.'
dip_note (no plan, 0 < spot < 0.020): ' Spot ($s) is below the $0.020 accumulation boundary. Liquid dry powder: {$X or none} (wallet USDC $W less the larger of the $C cash target and the $T virtual tax reserve).{lp_note} NockNock does not size a lump-sum buy or an outside deposit: budget, tranches and interval are your decision, set in accumulation_plan.'
This adds no 'CAPITAL INFUSION' directive.

### 63-E-growth-dry-powder-labels: growth_dry_powder_usd shown as liquid and deployable; 40/60 labels fixed; reserve always shown 'PROTECTED' — skeptic: **confirmed**

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `ingestion-worker/src/main.rs:6325-6331`, `specs/core/snapshot.yaml:446-451`, `frontend/src/App.tsx:720`, `frontend/src/App.tsx:744-745`, `frontend/src/App.tsx:754-756`, `frontend/src/components/LiteCockpit.tsx:274`, `frontend/src/components/LiteCockpit.tsx:322`, `ingestion-worker/src/main.rs:6045-6046`, `ingestion-worker/src/main.rs:6141`, `ingestion-worker/src/main.rs:6172`, `ingestion-worker/config.yaml.example:103-106`, `ingestion-worker/config.yaml.example:139-140`
- **Evidence:** The figure is `ledger.total_usd * (1.0 - profile.tax_reserve_ratio)` (6328): 60% of all receipt-verified harvests ever recorded. It is never reduced when the money is spent or redeployed, it includes NOCK fee harvests at their USD value, and the engine never sizes anything from it.
It is not computed from LP USDC, so it is not the #63 root cause. But it is labeled 'Liquid dry powder available for tactical dip accumulation' (snapshot.yaml:451) and '60% GROWTH DRY POWDER (DEPLOYABLE)' (App.tsx:745).
App.tsx:755 always renders '✓ TAX RESERVE PROTECTED — never deployable', even while the backend shows the Tax Guardrail.
The 40%/60% labels (App.tsx:744-745, LiteCockpit.tsx:274) and the backend '60/40 buffer' (6141) and '60/40 Split Rule (40% ..., 60% ...)' (6172) ignore the configurable tax_reserve_ratio.
The comment at 6045-6046 says dip deployments 'may only consume growth dry powder'; no code enforces that.
config.yaml.example:139-140 describes cash_reserve_usdc as 'liquid USDC ... (keep dry powder for a dip)', but by design it counts LP USDC too.
- **Impact:** All values read $0.00 today (no harvests, and the database is disposable), so there is no live harm. With harvest history, the dial could show '$300 DEPLOYABLE' while the wallet holds $10 and the advice says nothing is spendable. AGENTS.md §11 treats fixed percentages in the UI as high-severity data-integrity bugs.
- **Proposed fix:** No schema change.
Spec description: 'Growth share of cumulative receipt-verified harvests; an accounting figure, not a balance (never reduced by spending; NOCK harvests counted at USD value). Liquid dry powder is wallet USDC above the cash target and tax reserve.'
Dashboard: derive dial labels from tax_reserve_usd / cumulative_harvested_usd: '{p}% TAX RESERVE (VIRTUAL)' and '{100-p}% GROWTH SHARE OF HARVESTS', with no % when the cumulative total is 0. Replace the ✓ line with a coverage line computed from data.wallet_balances USDC vs tax_reserve_usd.
LiteCockpit: drop the fixed percentages.
Backend text: take the ratio from profile.tax_reserve_ratio.
Fix the 6045 comment and the config example wording ('stable balance (wallet USDC + USDC inside LP)'). The cash_reserve_usdc behaviour itself is unchanged.

### 63-F-tax-guardrail-lp: Tax guardrail counts LP USDC as covering the tax reserve — skeptic: **partly**

- **Kind / severity / patch:** defect / low / yes
- **Locations:** `ingestion-worker/src/main.rs:6051`, `ingestion-worker/src/main.rs:6137-6144`
- **Evidence:** The check is `let tax_guardrail_blocked = tax_reserve_usd > 0.0 && total_stable_usd < tax_reserve_usd;`, and the text reads '... exceeds the stable stack (${total_stable_usd})'.
Worked case, no plan: wallet $10, LP USDC $140.96, reserve $16, spot $0.01831. HEAD does not block (150.96 >= 16) and falls through to HOLD with no warning. The plan path blocks the same state: 'nothing spendable: wallet USDC $10.00 less the $16.00 virtual tax reserve' (1141-1147).
#63 remediation item 2 asks for `liquid_wallet_usdc < tax_reserve_usd`. By the issue's reasoning this is a defect. Once 63-A is fixed it cannot cause an over-spend (deployable is 0 when wallet < reserve), so what remains is a missing warning.
- **Impact:** The operator is never told that the tax reserve is not backed by liquid USDC.
- **Proposed fix:** Change the check to `wallet_usdc < tax_reserve_usd`.
New text: 'Spot price ($s) is at accumulation territory, but the virtual tax reserve ($T) exceeds liquid wallet USDC ($W).{lp_note} Deployment suppressed: harvest fees and rebuild the reserve in wallet USDC before buying dips.' This keeps 'Deployment suppressed', which the existing assertion at 6724 checks.
Part 2 of test_tax_guardrail_blocks_dip_deployment (6726-6734) flips and must be updated (see tests).
- **Skeptic's correction:** The check (6051) and its text (6141-6142) use the LP-inclusive total_stable_usd, as stated. In the worked case (wallet $10, LP $140.96, reserve $16, spot 0.01831), HEAD does not block: 150.96 >= 16, deployable is 0 because the cash target is $113.75, and the cycle falls to HOLD with no warning. One detail is wrong. The plan path does block, but not with the quoted reason. With the default respect_cash_target: true (main.rs:175-177, 1148), the cash cap (10 - 113.75) is smaller than the tax cap (10 - 16), so the reason reads 'nothing spendable: wallet USDC $10.00 less the $113.75 cash target'. The quoted tax-reserve text appears only with respect_cash_target false. The rest holds: after 63-A the guardrail only warns, and Part 2 of the existing test (6729-6734) flips.

### 63-G-mcp-stable-split: MCP portfolio text does not split wallet vs LP USDC and hardcodes the 100k floor — skeptic: **confirmed**

- **Kind / severity / patch:** defect / low / yes
- **Locations:** `ingestion-worker/src/main.rs:2136-2157`, `ingestion-worker/src/bin/mcp.rs:240-261`, `ingestion-worker/src/main.rs:8347-8350`
- **Evidence:** Both formatters print 'Floor buffer delta: {delta_prefix}{nock_delta} above 100k' and 'Stable Stack: {total_usdc} USDC'. #63 item 4 (show liquid dry powder and LP collateral separately in get_portfolio_state) is not done.
'Stable Stack' is an honest total. But the literal '100k' ignores global_status.nock_floor_target, and test_mcp_text_formatters asserts '+6140 above 100k'.
Live: 'Stable Stack: 1033.71 USDC', with 930.78 in the wallet and 102.92 inside LP.
- **Impact:** Agents reading MCP see a single stable figure, and the floor text is wrong for any user whose floor is not 100,000.
- **Proposed fix:** Take wallet USDC from data.wallet_balances and LP USDC from the USDC legs of data.positions; print 'unknown' for either when absent. Example: 'Stable Stack: 349.68 USDC (liquid in wallet: 300.00; inside LP, not spendable: 49.68)' and '(Floor buffer delta: +6140 vs the 100000 NOCK floor)'. Make the same change in mcp.rs, which is a separate binary with no shared code.

### 63-H-harvest-ledger-silent-zero: A failed harvest-ledger read silently sets the tax reserve to zero — skeptic: **confirmed**

- **Kind / severity / patch:** risk / low / no
- **Locations:** `ingestion-worker/src/main.rs:2486-2505`, `ingestion-worker/src/main.rs:1094-1098`
- **Evidence:** On a query error the code logs a warning and uses `LedgerAggregate::default()` (2501-2504). tax_reserve_usd becomes 0, and sovereign_ledger shows 0.00, which looks the same as a genuinely empty ledger. The accumulation ledger handles this explicitly with `ledger_unavailable`, which blocks the plan (1094-1098); the harvest ledger has no equivalent.
- **Impact:** After 63-A, the reserve is what limits dry powder, so a failed read would report dry powder as if no reserve existed. This should be rare: a fresh database correctly returns 0 via COALESCE.
- **Proposed fix:** Follow-up, not part of the minimal patch. Pass a `harvest_ledger_unavailable` flag into compute_derived_metrics. When it is set, report dry powder as unknown and replace the guardrail and dip text with a plain reason, the same way the plan's Blocked state works.

### Proposed design

Before branching (AGENTS.md §11): comment on #63 with the HEAD analysis.
- The lines it cites (3549/3564/3594) are now main.rs:6029-6056 and 6137-6152.
- The headline repro is fixed by the v0.5.0 cash-target cap (6053-6056).
- Name what remains: 63-A, B, D, F and G.
- File 63-C and 63-E as companion issues with the evidence above.

PART A: minimal #63 patch (main.rs, compute_derived_metrics_with_plan)

(1) Replace lines 6045-6056 with:
// Liquid dry powder (issue #63): only wallet USDC is spendable. USDC inside LP is working capital (a USDC/NOCK range converts it to NOCK as price falls), so it neither covers the tax reserve nor counts as dry powder: the rule plan_directive's tax cap and the LP rebuild's liquid floor already use.
let tax_reserve_usd = ledger.total_usd * profile.tax_reserve_ratio;
let tax_guardrail_blocked = tax_reserve_usd > 0.0 && wallet_usdc < tax_reserve_usd;
// Cash target (issue #79) and tax reserve overlap: the larger one stays in the wallet.
let cash_target_usd = total_portfolio_usd * (profile.target_weights_bps.cash as f64 / 10000.0);
let deployable_usdc = (wallet_usdc - tax_reserve_usd.max(cash_target_usd)).max(0.0);
let lp_usdc_note = if lp_usdc >= 0.005 { format!(" The ${:.2} of USDC inside LP is working capital, not dry powder: it stays in its ranges.", lp_usdc) } else { String::new() };

(2) Guardrail details (6141-6142), arguments spot_price, tax_reserve_usd, wallet_usdc, lp_usdc_note:
"Spot price (${:.5}) is at accumulation territory, but the virtual tax reserve (${:.2}) exceeds liquid wallet USDC (${:.2}).{} Deployment suppressed: harvest fees and rebuild the reserve in wallet USDC before buying dips."

(3) DIP details (6149-6150), only if Part B is declined. Arguments: spot_price, deployable_usdc, wallet_usdc, tax_reserve_usd, cash_target_usd, lp_usdc_note:
"Spot price (${:.5}) has breached the $0.020 accumulation boundary! Deploy ${:.2} of liquid wallet USDC (${:.2} in the wallet; ${:.2} protected as virtual tax reserve, ${:.2} kept as cash target) to purchase NOCK at discount.{}"

(4) HOLD arm (6176-6183):
let dip_note = if !plan_configured && spot_price > 0.0 && spot_price < 0.020 {
  let dry = if deployable_usdc >= PLAN_DUST_USDC { format!("${:.2}", deployable_usdc) } else { "none".to_string() };
  format!(" Spot (${:.5}) is below the $0.020 accumulation boundary. Liquid dry powder: {} (wallet USDC ${:.2} less the larger of the ${:.2} cash target and the ${:.2} virtual tax reserve).{} NockNock does not size a lump-sum buy or an outside deposit: budget, tranches and interval are your decision, set in accumulation_plan.", spot_price, dry, wallet_usdc, cash_target_usd, tax_reserve_usd, lp_usdc_note)
} else { String::new() };
action = if lp_usd > 0.0 { "HOLD & ACCRETE (Dual Growth Engine Active)" } else { "HOLD & ACCRETE (No Active LP)" };
details = format!("Total NOCK held is {:.0} (+{:.0} above {:.0} hard floor). {} tracked LP position(s) holding ${:.2} (+${:.2} uncollected fees).{} Maintain posture.", total_nock, nock_floor_delta, floor, raw.positions.len(), lp_usd, total_fees_usd, dip_note);

(5) BATCH HARVEST (6172): replace the fixed 60/40 sentence with "Harvest fees and apply the split rule ({:.0}% to the virtual tax reserve, {:.0}% growth)." Arguments: profile.tax_reserve_ratio*100.0 and (1.0-profile.tax_reserve_ratio)*100.0.

(6) MCP:
- In format_portfolio_state_text (main.rs:2136-2157) and mcp.rs format_portfolio_state (240-261), take the floor from global_status.nock_floor_target.
- Print the stable line as 'Stable Stack: {total} USDC (liquid in wallet: {w}; inside LP, not spendable: {l})'.
- w comes from the data.wallet_balances USDC amount; l is the sum of the USDC legs in data.positions; either is 'unknown' when absent.

(7) Labels only, no schema change:
- snapshot.yaml descriptions for growth_dry_powder_usd and tax_reserve_usd.
- App.tsx: dial labels derived from tax_reserve_usd / cumulative_harvested_usd, and a computed wallet-coverage line in place of the always-on ✓. Put this in a pure helper such as frontend/src/ledger.ts so it can be tested.
- LiteCockpit.tsx:274 and :322 without fixed percentages.
- config.yaml.example wording at :105 and :139-140.
- The comment at 6045-6046.

(8) CHANGELOG [0.5.1]: 'Fixed — Dry powder is wallet USDC only (Issue #63)'. Cover: the sizing, the label, the guardrail, the depleted note, removal of the fixed claims, and the MCP split.

PART B: standing-order and from-zero fix (recommended; confirm with the operator)
- Delete the no-plan DIP arm (6145-6152). deployable_usdc is then only reported, through the HOLD dip note, and never advised as a purchase. Users with a plan are unaffected.
- HOLD is on apply_lp_rebuild_bias's idle list, so a fundable LP rebuild leads, as the standing bias requires.
- Side effect: users without a plan no longer get a directive change, or a notification, when spot crosses $0.020. If the operator wants that signal, add an informational 'DIP ZONE (No Accumulation Plan)' arm just before HOLD and add it to the idle list. It must not go above BATCH HARVEST, or it would hide a due harvest.

NOT CHANGED
- cash_reserve_usdc counting wallet + LP USDC (operator decision).
- total_stable_usd, total_usdc_amount, 'Total USDC Stack' and 'Stable Stack' as honest totals.
- The deviation matrix's 'Liquid USDC' bucket, which already uses wallet USDC only.
- No new configurable minimum threshold: the $1 dust floor already suppresses dust advice, and a $5/$10 default would be a number chosen for the user.
- No 'CAPITAL INFUSION' directive.

Wallet ETH/WETH (#94, gas) is never part of wallet_usdc, so this design keeps it out of dry powder. The test expectations assume wallet_eth and wallet_weth are None (RawOnChainData::default()).

### Tests to add

- Update test_tax_guardrail_blocks_dip_deployment (main.rs:6709-6751). In four_position_fixture, LP USDC is 44.80 + 129.04 = $173.84, not the '≈140' in the comment at 6715. Part 1 (wallet $10, ledger $500, reserve $200): keep the three asserts and add details.contains('the virtual tax reserve ($200.00) exceeds liquid wallet USDC ($10.00).'). Part 2 (ledger $40, reserve $16) now blocks: replace 6733-6734 with action.contains('Tax Guardrail') and details.contains('exceeds liquid wallet USDC ($10.00)'). Then set wallet_balances[1].amount = '50.0' (cash target $104.81) and assert !action.contains('Tax Guardrail'), !action.contains('DIP REINVESTMENT SHOT'), and details.contains('Liquid dry powder: none (wallet USDC $50.00 less the larger of the $104.81 cash target and the $16.00 virtual tax reserve)'). Part 3 (wallet $5000, total $7046.28): with Part A only it is unchanged ('Deploy $4647.69 ...'). With Part B: action.starts_with('HOLD & ACCRETE'), details.contains('Liquid dry powder: $4647.69 (wallet USDC $5000.00 less the larger of the $352.31 cash target and the $16.00 virtual tax reserve)'), built from total_portfolio_usd as the test does now, and !details.contains('Deploy $').
- New fixture dip_fixture(wallet_usdc: &str), built on RawOnChainData::default(). positions: fixture_position_v4_usdc() with amount0 = 20.0, amount1 = 1000.0, is_price_source = false; and fixture_position_v4_usdc_wide() with amount0 = 120.96, amount1 = 5000.0. spot_price_nock_usd = 0.01831; live_gas_cost_usd = 0.005. wallet_balances: NOCK '110000.0' (NOCK_BASE) and USDC wallet_usdc (USDC_BASE). This gives LP USDC $140.96, LP value $250.82 and total NOCK 116,000, and leaves pool_liquidity empty so LP rebuild suggestions are empty.
- New test_dip_dry_powder_is_wallet_usdc_only_issue_63.
(a) Default profile, wallet '300.0', ledger total_usd 500 (reserve $200; cash target $128.25 of $2564.92). Parts A+B: action == 'HOLD & ACCRETE (Dual Growth Engine Active)'; details.contains('Liquid dry powder: $100.00 (wallet USDC $300.00 less the larger of the $128.25 cash target and the $200.00 virtual tax reserve). The $140.96 of USDC inside LP is working capital, not dry powder: it stays in its ranges.'); !contains('440.96'). Part A only: action == 'DIP REINVESTMENT SHOT (Deploy USDC)' and details.starts_with('Spot price ($0.01831) has breached the $0.020 accumulation boundary! Deploy $100.00 of liquid wallet USDC ($300.00 in the wallet; $200.00 protected as virtual tax reserve, $128.25 kept as cash target) to purchase NOCK at discount. The $140.96 of USDC inside LP is working capital, not dry powder: it stays in its ranges.'). HEAD gives 'Deploy $171.75 of saved USDC ($440.96 available; ...'.
(b) profile.target_weights_bps.cash = 0, wallet '37.44', ledger 50 (reserve $20). Parts A+B: contains('Liquid dry powder: $17.44 (wallet USDC $37.44 less the larger of the $0.00 cash target and the $20.00 virtual tax reserve).'). Part A only: contains('Deploy $17.44 of liquid wallet USDC ($37.44 in the wallet; $20.00 protected as virtual tax reserve, $0.00 kept as cash target)'). HEAD gives 'Deploy $37.44 of saved USDC ($178.40 available; ...'. Both: !contains('178.40').
(c) The issue's exact state: default profile, wallet '37.44', empty ledger. action == 'HOLD & ACCRETE (Dual Growth Engine Active)'; details.starts_with('Total NOCK held is 116000 (+16000 above 100000 hard floor). 2 tracked LP position(s) holding $250.82 (+$0.00 uncollected fees). Spot ($0.01831) is below the $0.020 accumulation boundary. Liquid dry powder: none (wallet USDC $37.44 less the larger of the $115.12 cash target and the $0.00 virtual tax reserve). The $140.96 of USDC inside LP is working capital, not dry powder: it stays in its ranges. NockNock does not size a lump-sum buy or an outside deposit: budget, tranches and interval are your decision, set in accumulation_plan. Maintain posture.'); !contains('0% realized loss'); !contains('Uniswap position').
(d) Default profile, wallet '10.0', ledger 40 (reserve $16). action == 'SWEEP TO STABLE / ACCUMULATE DRY POWDER (Tax Guardrail)'; details.starts_with('Spot price ($0.01831) is at accumulation territory, but the virtual tax reserve ($16.00) exceeds liquid wallet USDC ($10.00). The $140.96 of USDC inside LP is working capital, not dry powder: it stays in its ranges. Deployment suppressed: harvest fees and rebuild the reserve in wallet USDC before buying dips.'). HEAD does not block and falls through to HOLD.
- New fixture from_zero_fixture(usdc: &str, nock: &str): positions = [aerodrome_position_from_reads(&aero_cfg(true), "0xw", "0xpool", token_info(USDC_BASE).unwrap(), token_info(NOCK_BASE).unwrap(), 369_321_720_000, 318_394_460_000_000_000_000_000, 342_542_824_101_829_799, 0, 0, 0).unwrap()]. The user tracks a pool for its price and holds no LP. spot = nock_spot_from_positions(&positions) = $0.01160. wallet_balances: NOCK nock and USDC usdc. Call compute_derived_metrics directly with OperatorProfile { treasury_floor_nock: 0.0, ..Default::default() } (the user's own floor; with the default 100,000 the result is DEFENSIVE LOCK), no plan, LedgerAggregate::default().
- New test_from_zero_usdc_only_user_is_not_told_to_deploy with from_zero_fixture('500.0', '0.0'). Parts A+B: action == 'HOLD & ACCRETE (No Active LP)'; details.starts_with('Total NOCK held is 0 (+0 above 0 hard floor). 1 tracked LP position(s) holding $0.00 (+$0.00 uncollected fees). Spot ($0.01160) is below the $0.020 accumulation boundary. Liquid dry powder: $475.00 (wallet USDC $500.00 less the larger of the $25.00 cash target and the $0.00 virtual tax reserve). NockNock does not size a lump-sum buy or an outside deposit: budget, tranches and interval are your decision, set in accumulation_plan. Maintain posture.'); !contains('Deploy $'); !contains('120,000'); !contains('inside LP'). Part A only: 'DIP REINVESTMENT SHOT (Deploy USDC)' with details.starts_with('Spot price ($0.01160) has breached the $0.020 accumulation boundary! Deploy $475.00 of liquid wallet USDC ($500.00 in the wallet; $0.00 protected as virtual tax reserve, $25.00 kept as cash target) to purchase NOCK at discount.'). HEAD gives '... $500.00 available ... toward 120,000.'
- New test_from_zero_empty_portfolio_has_no_static_claims with from_zero_fixture('0.0', '0.0'). action == 'HOLD & ACCRETE (No Active LP)'. details must equal exactly: 'Total NOCK held is 0 (+0 above 0 hard floor). 1 tracked LP position(s) holding $0.00 (+$0.00 uncollected fees). Spot ($0.01160) is below the $0.020 accumulation boundary. Liquid dry powder: none (wallet USDC $0.00 less the larger of the $0.00 cash target and the $0.00 virtual tax reserve). NockNock does not size a lump-sum buy or an outside deposit: budget, tranches and interval are your decision, set in accumulation_plan. Maintain posture.' No LP-rebuild text is appended, because lp_rebuild is on_target at a $0 total. sovereign_ledger fields are all '0.00'. HEAD gives '(Dual Growth Engine Active)' and '1 Uniswap position(s) generating yield (+$0.00 uncollected). 0% realized loss.'
- Part B only: update test_advice_plan_replaces_lump_sum_dip_shot (main.rs:7021-7024). liquid_wallet_fixture with no plan: action == 'HOLD & ACCRETE (No Active LP)'; details.contains('Liquid dry powder: $929.22 (wallet USDC $1053.00 less the larger of the $123.78 cash target and the $0.00 virtual tax reserve).'); !contains('Deploy $'). The plan half of the test is unchanged.
- test_derived_metrics_batch_harvest_above_5_dollars: assert details.contains('(40% to the virtual tax reserve, 60% growth)'). With profile.tax_reserve_ratio = 0.25, assert '(25% to the virtual tax reserve, 75% growth)'.
- Update test_mcp_text_formatters (main.rs:8309-8360). Add global_status.nock_floor_target '100000', wallet_balances [{token_symbol 'USDC', amount '300.0000'}] and positions [{token0 {symbol 'USDC'}, token1 {symbol 'NOCK'}, amount0 '49.680000', amount1 '1000.0'}]. Replace contains('+6140 above 100k') with contains('+6140 vs the 100000 NOCK floor'), and add contains('Stable Stack: 349.68 USDC (liquid in wallet: 300.00; inside LP, not spendable: 49.68)'). A copy without wallet_balances and positions must contain 'liquid in wallet: unknown; inside LP, not spendable: unknown'.
- Frontend ledger.test.ts, for the new pure helper ledgerDisplay(ledger, walletUsdc | null):
- {cumulative '500.00', tax '200.00', growth '300.00'} with wallet 10 gives '40% TAX RESERVE (VIRTUAL)', '60% GROWTH SHARE OF HARVESTS' and '✗ Wallet USDC $10.00 does not cover the $200.00 tax reserve (USDC inside LP does not count)'.
- {'100.00', '33.00', '67.00'} with wallet 50 gives '33% TAX RESERVE (VIRTUAL)', '67% GROWTH SHARE OF HARVESTS' and '✓ Wallet USDC $50.00 covers the $33.00 tax reserve'.
- An empty ledger gives 'TAX RESERVE (VIRTUAL)' with no % and 'No harvest income recorded, so there is no tax reserve to cover'.
- An unread wallet gives 'Tax reserve coverage unknown: wallet USDC was not read'.
- Optional positive from-zero path: from_zero_fixture('500.0', '0.0') with the floor-0 profile, through compute_derived_metrics_with_plan. Do not use metrics_with_plan, which applies the default 100k floor. Plan: plan_cfg() with id 'from-zero', budget_usdc 100.0, tranches 5. Expect action == 'DCA TRANCHE DUE (1 of 5)' and details.starts_with('Buy $20.00 of NOCK on Aero (spot $0.01160). Plan "from-zero": $0.00 of $100.00 deployed over 0 verified buy(s).'). Only the user's own numbers are used.

### Open questions

- Part B (63-C). With no accumulation_plan, the dip arm advises spending wallet USDC down to the 5% cash target. OPERATOR_GUIDANCE §7.7 says that target is 'not permission to spend down to it', and today you said cash in pocket isn't ripped out at an 'investment low' sign. Options: retire the arm and report liquid dry powder as a fact in HOLD, which means no directive change or notification at the $0.020 crossing; or keep a non-sizing 'DIP ZONE (No Accumulation Plan)' directive just before HOLD. #63's original remediation asked for a sized 'Deploy Liquid Dry Powder' directive, which these 2026-11-14 standing orders appear to supersede. OK to supersede it?
- Should the tax reserve stay wallet-only? plan_directive and the LP rebuild already treat it that way, and this patch makes the dip text and guardrail match. cash_reserve_usdc is different: by your 2026-11-14 decision it counts LP USDC.
- #96 was dropped and the database is disposable. A fresh start resets cumulative_harvested_usd, so the tax reserve restarts at $0, even for harvests already realized this tax year. Is that acceptable (Koinly being the tax record), or should the reserve be rebuildable from chain?
- 'compound buffer toward 120,000' and the $0.020 accumulation boundary are your own constants, written into advice text that a new user also sees. I propose deleting the 120,000 clause now and leaving the boundary to the deferred 'externalise the legacy advice constants' item. Agreed?
- This overlaps another area (from-zero). With the default config, a new user holding no NOCK hits DEFENSIVE LOCK ('below the 100000 floor ... All yield and stable reserves are locked') before any dry-powder logic runs, so the from-zero tests here set treasury_floor_nock: 0 explicitly. The from-zero work needs to decide whether the floor, the 5%/15% weights and the 0.40 ratio may be applied to a new user by default at all.

### Skeptic: problems with the proposed design

- Part A step (4) depends on Part B. The HOLD dip_note says 'NockNock does not size a lump-sum buy or an outside deposit', and the new guardrail text says 'rebuild the reserve in wallet USDC before buying dips'. If Part B is declined, the DIP arm (main.rs:6145-6152) still sizes a lump sum whenever liquid dry powder is $1 or more. The minimal patch would then state the opposite of what the engine does one branch earlier. Tests (c), both from-zero tests and the Part 2 'Liquid dry powder: none' assertion all contain that sentence. Make the sentence conditional on Part B, or move the dip_note into Part B.
- Part B overturns a recorded design decision without updating the record. docs/kb/design-note-dca-accumulation-plan.md:147-148 says 'With no plan configured the existing arm stays as it is, except that it should also respect the cash target', and the §8.2 row at :321 says 'Shipped for the no-plan path too'. The change list also misses: README.md:69 (advice table 'DIP REINVESTMENT ... Deploy saved USDC to buy discounted NOCK'), README.md:72 ('AMMs generating dual yield'), and the cash_reserve_usdc doc comment at main.rs:178-181 ('would take liquid USDC below this amount'). That comment has the same stale wording as config.yaml.example:139-140, which the design does fix.
- The growth_dry_powder_usd relabel ('an accounting figure, not a balance') contradicts the operator's own guidance: OPERATOR_GUIDANCE.md:95-98 ('60% represents liquid growth dry powder for dip accumulation') and config.yaml.example:103-105. The design changes only config.yaml.example:105. Redefining the operator's 60/40 rule needs the guidance updated in the same PR and an explicit yes from the operator. The narrower fix removes only the balance claims ('available', 'DEPLOYABLE', the always-on ✓) and the fixed percentages.
- The from-zero tests do not prove the 'never choose numbers for the user' goal. (1) They assert with starts_with, so the sentence apply_lp_rebuild_bias appends (main.rs:1348-1391) is never checked. For the $500 USDC-only user it reads 'LP holds $0.00 against a $75.00 target (15% of portfolio), $75.00 short. Re-establishing ranges from NOCK and USDC already held is a reposition, not a purchase'. That is a default 15% target, and false for someone holding 0 NOCK. (2) 'Liquid dry powder: $475.00' is itself $500 minus a default 5% cash weight (main.rs:54) the user never set. (3) The tests set treasury_floor_nock: 0, which bypasses the shipped base-layer floor (config.yaml.example:95, 100,000). So the real fresh start (example config, empty database, new wallet) is untested, and it lands on DEFENSIVE LOCK. Assert the full details string, or pin these default-derived parts explicitly.
- Keeping '(Dual Growth Engine Active)' whenever lp_usd > 0 is still a fixed claim. Out-of-range positions hold value but earn no fees, and frontend positionsSummary (positions.ts:26-45) already separates in range, out of range and stale pool. Derive the label from in_range positions, which compute_derived_metrics has, or drop it. Otherwise 63-D is only half fixed.
- The coupling with #94 is not addressed. cash_target_usd = total_portfolio_usd x cash weight (main.rs:6055), and total_portfolio_usd (6029) is where #94 will add wallet ETH/WETH. Once gas is counted, the cash target, the LP target (6080) and 'liquid dry powder' all move with the gas balance. The deviation matrix's three buckets (6283-6315) would also stop summing to the total. The design should state which base the cash target uses after #94. Every proposed test uses RawOnChainData::default() (wallet_eth and wallet_weth None), so none would catch the shift.

### Skeptic: missed in this area

- **MCP direct-RPC fallback reads NOCK spot from the retired dead pool and presents it as live** (`ingestion-worker/src/bin/mcp.rs:333-337`, `ingestion-worker/src/bin/mcp.rs:363-375`, `ingestion-worker/src/bin/mcp.rs:391-421`, `CHANGELOG.md:37-39`, `ingestion-worker/src/main.rs:4842-4849`, `ingestion-worker/src/main.rs:8461-8468`): mcp.rs:337 `const NOCK_USDC_POOL_ID: &str = "0x91a9ef0a...f99c"` is the pool of v4 #3163431 (fixture main.rs:8463-8468). CHANGELOG.md:37-39 retired that position as 'on a dead pool'; the live price source is #3163450. The worker refuses a price-source pool with zero in-range liquidity (main.rs:4842-4849, check_price_source_liquidity at 5041-5051). The fallback has no such check: it prints 'NOCK Spot Price: $...' and a wallet NOCK USD value from that slot0 in get_portfolio_state and get_shot_advice. #63 item 4 names this fallback; 63-G covered only the snapshot-API fallback.
- **A failed wallet USDC read is published as a live zero, and wallet USDC becomes the only input to dry powder** (`ingestion-worker/src/main.rs:4867-4893`, `ingestion-worker/src/main.rs:4972-4979`): main.rs:4875 `b.get("tokenBalance").and_then(|h| h.as_str()).unwrap_or("0x0")` turns a null or absent USDC tokenBalance into 0 and sets balances_found. balances_found is also set when only the NOCK entry parses (4869-4891), so a USDC-only failure does not fail the cycle. wallet_balances always carries a USDC row (4972-4979), so 63-G's 'unknown' path never fires on live data. After 63-A/F, a failed read would show 'Liquid dry powder: none'. With any harvest history it would also trip the tax guardrail as if the wallet were empty, and plan_directive would report 'nothing spendable'.
- **LP rebuild counts wallet ETH/WETH above 0.002 ETH as deployable LP capital, contrary to today's #94 decision** (`ingestion-worker/src/main.rs:6096-6099`, `ingestion-worker/src/main.rs:726-727`, `ingestion-worker/src/main.rs:948`): RebuildFunds.eth = wallet_eth + wallet_weth - REBUILD_GAS_BUFFER_ETH (0.002), and it sizes the ETH legs of suggested WETH/NOCK and ETH/NOCK deposits. The 2026-11-14 decision says ETH is gas: valuable, but not deployable or investable capital. The report says the design keeps ETH 'out of dry powder', which is true only for the dip path. In the same function, about 0.0105 of the operator's ~0.0125 ETH counts as fundable LP capital whenever LP is under target. It is latent today because lp_rebuild is on_target.
- **Dashboard fallbacks still show the fixed HOLD directive and 'dual yield' claims** (`frontend/src/App.tsx:381-382`, `frontend/src/App.tsx:418-420`, `specs/core/snapshot.yaml:654-658`): `{global?.shot_advice_action || 'HOLD & ACCRETE (Dual Growth Engine Active)'}` and the details fallback '... AMMs are generating dual yield. Maintain posture.' render whenever a returned snapshot lacks global_status or has an empty action. These are the same fixed claims 63-D removes from the backend, and the spec examples repeat them. Impact is limited: with no snapshot at all, the API returns 503 NO_SNAPSHOT (main.rs:1916) and the app shows its error screen.


## advice-text: Issue #95 and a truthfulness audit of user-facing advice strings: the worker advice engine, MCP text on both transports, notifications, the dashboard and the Lite cockpit

**Summary.** The live advice on chudmax is truthful. Snapshot 73d13bdb (2026-11-14T18:06:05Z) has action 'DCA ON SCHEDULE (2 of 5 next)' and details 'Nothing to buy yet: the next $20.00 tranche is due at 2026-11-15T06:02:25+00:00. Plan "2026-11-reentry": $20.00 of $100.00 deployed over 1 verified buy(s), average cost $0.012516. The plan never spends the stable balance (wallet + LP USDC) below $950.00.' Every number in it comes from config or from receipt-verified fills.

The false strings sit in branches that the active plan hides, and in static dashboard and MCP text. Snapshot history shows two of them went out live today:
- **05:36:10Z, the #95 HOLD text:** '4 Uniswap position(s) generating yield (+$0.00 uncollected). 0% realized loss.' At that moment LP held $0.00.
- **04:38:29Z, the lump-sum DIP shot:** 'Deploy $1053.00 of saved USDC ... toward 120,000'.

HOLD comes back as soon as the plan completes, around 2026-11-18 at one buy a day.

Other untrue claims found:
- **High-water mark:** 'All-time high' labels a peak set this morning, the drawdown from it is labelled 'LIVE P/L', and high_water_mark_date is always the snapshot time.
- **Tax ledger:** it hardcodes 60/40 although the ratio is configurable, and draws a full bar captioned '100% of realized harvests' when there are no harvests.
- **MCP text:** it hardcodes 'above 100k' and states a 2x-gas claim rule that the directive engine does not use.
- **MCP stdio fallback:** it prints a dead pool's frozen price as spot.
- **Aero pool:** its fees cannot be claimed (claimFees() reverts), but they show as '$0 earned', and the live claim payload reports an RPC outage that leaves gas and net blank.

A brand-new user gets one of two outcomes. With no positions there is no snapshot at all. Otherwise the advice is built on the operator's positions and numbers, which a partial config.yaml inherits from config.yaml.example. That means either a CRITICAL 'locked' alarm against a 100,000 floor they never set, or a lump-sum Deploy of all cash above a 5% default, toward 120,000 NOCK. 13 of the 14 findings are patch scope; the flavour renames are not.


### ADV-01: HOLD advice asserts '0% realized loss' and calls every tracked position a yielding Uniswap position (#95) — skeptic: **confirmed**

- **Kind / severity / patch:** defect / high / yes
- **Locations:** `ingestion-worker/src/main.rs:6176-6183`, `ingestion-worker/src/main.rs:1393-1401`, `ingestion-worker/src/main.rs:2647-2654`, `ingestion-worker/src/main.rs:3753`, `specs/core/snapshot.yaml:653-658`, `frontend/src/positions.ts:20-46`
- **Evidence:** main.rs:6180 formats '... {} Uniswap position(s) generating yield (+${:.2} uncollected). 0% realized loss. Maintain posture.' with raw.positions.len() and total_fees_usd. Nothing computes realized P/L: the only other 'realized' in non-test code is a comment at main.rs:6046. raw.positions now includes the Aero pool. chudmax history (GET /v1/snapshot/historical) shows the text live at 2026-11-14T05:36:10Z: 'LP holds $0.00 against a $371.29 target ... Meanwhile: HOLD & ACCRETE (Dual Growth Engine Active) — Total NOCK held is 115748 (+15748 above 100000 hard floor). 4 Uniswap position(s) generating yield (+$0.00 uncollected). 0% realized loss. Maintain posture.' Four empty positions were described as yielding. That day the Uniswap web app showed realized −$1,002.58 and unrealized −$576.40 on wallet NOCK.
- **Impact:** Once the DCA plan completes, HOLD becomes the live directive and would read '3 Uniswap position(s) generating yield (+$0.30 uncollected). 0% realized loss.' Tranche 2 is due 2026-11-15T06:02Z, so at one buy per 24 h that is about 2026-11-18. The text reaches the dashboard banner and both MCP transports (format_shot_advice_text passes it through). It is also pushed to Telegram, Pushover and the webhook on the directive change, and it rides inside REBUILD LP TRANCHES as the 'Meanwhile:' clause.
- **Proposed fix:** Build the details from the positions, porting frontend positionsSummary to a Rust helper over raw.positions and raw.pool_liquidity (a pool reading 0 counts as stale). Template: 'Total NOCK held is {total:.0} (+{delta:.0} above the {floor:.0} floor). {n} LP position(s) tracked: {r} ranged ({i} in range[, {o} out of range][, {s} on a stale pool])[, {f} full range]. ${fees:.2} of LP fees uncollected. Maintain posture.' It contains no P/L figure and never says 'generating yield': an empty or out-of-range range earns nothing, and the Aero pool's yield is AERO rewards, which are not read (#93). Today it would read: 'Total NOCK held is 113863 (+13863 above the 100000 floor). 3 LP positions tracked: 2 ranged (2 in range), 1 full range. $0.30 of LP fees uncollected. Maintain posture.' Keep the action string: its 'HOLD & ACCRETE' prefix drives idle detection (main.rs:1393) and transition notifications. Update the snapshot.yaml examples. Extend test_derived_metrics_hold_and_accrete (main.rs:6499-6506): the details must contain 'LP position' and must contain none of 'realized', '0%', 'Uniswap position' or 'generating yield'.

### ADV-02: 'All-time high', 'LIVE P/L' and high_water_mark_date misstate the portfolio's performance — skeptic: **partly**

- **Kind / severity / patch:** defect / high / yes
- **Locations:** `frontend/src/App.tsx:367-372`, `frontend/src/App.tsx:719-753`, `ingestion-worker/src/main.rs:2458-2463`, `ingestion-worker/src/main.rs:6037`, `ingestion-worker/src/main.rs:6196-6197`, `specs/core/snapshot.yaml:628-634`, `frontend/src/components/AccumulationPlanPanel.tsx:168-176`
- **Evidence:** The high-water mark is MAX(total_portfolio_usd) over the snapshots table (main.rs:2458-2463). chudmax's first visible snapshot is 2026-11-14T04:38:29Z: the history endpoint returns 0 rows before it and 177 after. The $2,475.27 peak was set at 05:57:48Z today, yet the card subtitle says 'All-time high' (App.tsx:371) and the spec says 'All-time high portfolio value'. high_water_mark_date is now_str.clone() on every cycle (main.rs:6197). Its live value is 18:06:05Z, not 05:57:48Z, while the spec says 'When the high-water mark was set'. App.tsx:747 labels ((current / hwm) - 1) * 100 'LIVE P/L vs HIGH-WATER MARK'; live that is −3.42%. The Uniswap app shows a total return of about −$1,578.98 on wallet NOCK alone. NockNock tracks no cost basis (#98).
- **Impact:** A deep loss position reads as a 3% dip, which is the misreading the operator wants to avoid while building in public. The database is disposable (#96 dropped), so every fresh start resets 'All-time' to day one. The plan panel's 'AVERAGE COST $0.01252' sits nearby and is easy to mistake for the wallet's cost basis (about $0.018-0.02 in the Uniswap app).
- **Proposed fix:** (1) In main.rs, return the generated_at of the peak row along with prev_hwm and emit it as high_water_mark_date; use now only when this cycle sets a new peak. (2) Change the App.tsx:371 subtitle to 'Highest recorded since {date}' or 'Highest recorded by this database'. Change spec line 630 to 'Highest total_portfolio_usd recorded in this database; resets with it'. (3) Retitle the App.tsx:747 dial 'CHANGE FROM RECORDED PEAK' and add the line 'Not profit or loss: NockNock does not track cost basis (#98).' (4) Relabel AccumulationPlanPanel.tsx:169 'PLAN AVERAGE COST', with the caption 'this plan's verified buys only; not the wallet's cost basis'.
- **Skeptic's correction:** The code claims hold:
- HWM is MAX over snapshots (2458-2463).
- high_water_mark_date = now_str every cycle (6197); live it reads 18:31:04Z.
- 'All-time high' (App.tsx:371).
- 'LIVE P/L vs HIGH-WATER MARK' (747) = 2390.65/2475.27-1 = -3.42%.
- Spec lines 630 and 634.
One detail is wrong: $2,475.27 was first recorded by the very first snapshot at 04:38:29Z and held identically through 05:57:48Z (22 tied rows). 05:57:48Z is the last tie, not when the peak was set. The 'peak' is simply the fresh database's first reading.

### ADV-03: Tax-ledger text hardcodes 60/40, draws a fabricated 100% bar, and calls a virtual reserve a 'protected' escrow — skeptic: **confirmed**

- **Kind / severity / patch:** defect / high / yes
- **Locations:** `ingestion-worker/src/main.rs:6141`, `ingestion-worker/src/main.rs:6170-6173`, `ingestion-worker/src/main.rs:6325-6331`, `frontend/src/App.tsx:720`, `frontend/src/App.tsx:726-745`, `frontend/src/App.tsx:754-756`, `frontend/src/components/LiteCockpit.tsx:274`, `frontend/src/components/LiteCockpit.tsx:322`, `specs/core/snapshot.yaml:448`, `ingestion-worker/config.yaml.example:103-106`
- **Evidence:** The split comes from config, ledger.total_usd * profile.tax_reserve_ratio (main.rs:6327), and test_sovereign_ledger_60_40_split shows that 0.25 yields 25/75 (main.rs:6635-6642). Hardcoded ratios appear anyway:
- main.rs:6172: 'apply 60/40 Split Rule (40% to Tax Escrow, 60% Growth Capital)'.
- main.rs:6141: 'rebuild the 60/40 buffer'.
- App.tsx:744-745: '40% TAX RESERVE (PROTECTED)' and '60% GROWTH DRY POWDER (DEPLOYABLE)'.
- LiteCockpit.tsx:274 and 322: '40% tax reserve ... 60% dry powder' and '60/40 ledger'.
With no harvests (live harvest_count 0), taxPct is 0, so the growth dial gets pct={100 - taxPct}: a full bar captioned '100% of realized harvests' for $0.00. App.tsx:755 always shows '✓ TAX RESERVE PROTECTED — never deployable', including the tax_guardrail_blocked case (main.rs:6051) where the stable balance does not cover the reserve. The reserve is bookkeeping ('no on-chain address change', main.rs:1408), not an escrow, and the ledger applies the split on record-harvest; 'apply ... rule' tells the user to do it. 'Harvest fees and rebuild the buffer' can also widen the deficit: a harvest adds ratio × its USD value to the reserve, but only its USDC part to the stable balance. Live v3 fees are 9.2194 NOCK plus 0.000093 WETH.
- **Impact:** Every surface misreports any ratio other than 0.40. The live dashboard draws a static 100% bar, which AGENTS.md classes as a high-severity data-integrity bug.
- **Proposed fix:** Backend, ratio from the profile. BATCH HARVEST: 'Uncollected LP fees (${fees:.2}) have reached the fixed $5.00 harvest threshold ({eff} estimated gas). Harvest, then record the transaction: NockNock books {r:.0}% of its USD value to the virtual tax reserve and {100-r:.0}% to growth dry powder. Bookkeeping only; nothing moves on-chain.' Tax guardrail, if that arm survives ADV-04: 'The virtual tax reserve (${tax:.2}) exceeds the stable balance (${stable:.2}), so no buy is advised until the stable balance covers it.' Add an optional sovereign_ledger.tax_reserve_ratio field so the UI can read 'TAX RESERVE ({r}%)' and 'GROWTH DRY POWDER ({100-r}%)'. Retitle the panel 'Sovereign Ledger (Virtual Tax Reserve vs Growth Dry Powder)'. With zero harvests, show 'no harvests recorded' and empty bars. Replace the static badge with a computed line: 'Advice never counts the ${tax} reserve as spendable' when the stable balance covers it, otherwise 'Stable balance ${stable} is below the ${tax} reserve'. In LiteCockpit, write 'tax reserve $X · dry powder $Y' and 'recorded in the harvest ledger'.

### ADV-04: With no plan, DIP REINVESTMENT advice sizes a lump-sum buy from invented constants ($0.020 boundary, 'toward 120,000') — skeptic: **confirmed**

- **Kind / severity / patch:** defect / high / yes
- **Locations:** `ingestion-worker/src/main.rs:6049-6056`, `ingestion-worker/src/main.rs:6137-6152`, `frontend/src/components/AccumulationPlanPanel.tsx:89-98`, `ingestion-worker/config.yaml.example:122-126`
- **Evidence:** main.rs:6149: 'Spot price (${:.5}) has breached the $0.020 accumulation boundary! Deploy ${:.2} of saved USDC (${:.2} available; ...) to purchase NOCK at discount and compound buffer toward 120,000.' 0.020 and 120,000 exist only as literals; no config key holds them. The deploy figure is all wallet cash above the cash target, deployable_usdc.min((wallet_usdc - cash_target_usd).max(0.0)), and the cash target defaults to 5% (main.rs:54). The 'available' figure is total_stable_usd, which includes USDC inside LP (#63, milestone v0.5.1): $1,033.71 live against $930.78 in the wallet. The deployment emitted it at 2026-11-14T04:38:29Z, on a build that predates the cash-target clause: 'Deploy $1053.00 of saved USDC ($1053.00 available; $0.00 protected as virtual tax reserve) to purchase NOCK at discount and compound buffer toward 120,000.' The plan panel says as much: '... instead of advised as one lump sum' (AccumulationPlanPanel.tsx:95).
- **Impact:** The operator's plan suppresses this arm (plan_configured), so it is dormant on chudmax. A newcomer with no plan, with spot at $0.0115, would have NockNock pick the budget (all cash above a 5% default), the price trigger and a 120,000 NOCK goal. These are numbers the standing order says it must never choose.
- **Proposed fix:** Delete both no-plan dip arms (main.rs:6137-6152) and their literals. With no plan, size nothing and say once: 'No accumulation plan is configured, so NockNock does not size NOCK buys. Budget, tranche count and interval are yours to choose; once set in config.yaml, NockNock paces each buy and verifies it on-chain.' Change AccumulationPlanPanel.tsx:93-96 to: 'No accumulation plan is configured, so NockNock does not size NOCK buys. To have buys paced, add accumulation_plan (budget_usdc, tranches, interval_hours) to config.yaml; the numbers are yours to choose.' If the operator wants to keep a no-plan signal for 0.5.1, the minimum is wallet USDC as 'available' (#63), no 'toward 120,000', no 'at discount', and the boundary taken from config.

### ADV-05: DEFENSIVE LOCK says funds are 'locked' and fires on a 100,000 NOCK floor that NockNock chose itself — skeptic: **partly**

- **Kind / severity / patch:** defect / high / yes
- **Locations:** `ingestion-worker/src/main.rs:6118-6125`, `ingestion-worker/src/main.rs:50`, `ingestion-worker/src/main.rs:66-75`, `ingestion-worker/src/main.rs:1458-1460`, `ingestion-worker/config.yaml.example:94-95`, `frontend/src/App.tsx:364`, `frontend/src/App.tsx:426`, `frontend/src/components/LiteCockpit.tsx:147-148`, `frontend/src/components/LiteCockpit.tsx:249-254`
- **Evidence:** main.rs:6122: 'CRITICAL: Total NOCK stack ({:.0}) is below the {:.0} floor by {:.0} NOCK! All yield and stable reserves are locked to replenish the sovereign foundation.' NockNock never signs or holds anything, so nothing is locked, and other panels keep advising: the plan status is still computed and the velocity panel still says CLAIM_NOW. Two defaults supply the floor:
- fn default_treasury_floor() -> f64 { 100000.0 } (main.rs:50) and the example base layer (config.yaml.example:95) give every new user the operator's floor.
- default_floor_target_string() puts '100000' back into any snapshot that lacks the field (main.rs:1458-1460), including the latest snapshot reloaded at boot (main.rs:1672).
For an empty portfolio total_nock = 0 < 100000, so the first directive a newcomer sees is CRITICAL.
- **Impact:** For a new user, this advice amounts to putting all cash into NOCK toward a floor they never chose. With the inherited positions (ADV-07), a stranger's stack would be about 21,010 NOCK, all of it the operator's LP, and the text would read 'below the 100000 floor by 78990 NOCK'.
- **Proposed fix:** Details: 'Total NOCK ({total:.0}) is {deficit:.0} below your {floor:.0} floor. NockNock will not suggest putting floor NOCK into LP. Whether and how to rebuild the stack is your decision; an accumulation plan paces buys at amounts you set.' No 'CRITICAL', 'locked' or 'sovereign foundation'. Make treasury_floor_nock an Option<f64> with no default. When it is unset, skip the floor branch and the floor clauses, leave nock_floor_target absent or empty, and have default_floor_target_string() return ''. Change App.tsx:364 to '{abs} below floor' and App.tsx:426 to 'Below {floor}, NockNock stops suggesting LP deposits of NOCK.' LiteCockpit must render 'no floor set': today parseInt('') gives 'target NaN NOCK'. Keep the action label, which is flavour and is pinned by tests and notifications.
- **Skeptic's correction:** Confirmed:
- the 'locked' text (6122);
- the defaults (50, 69);
- default_floor_target_string (1458-1460), reused for the boot reload (1672);
- the stranger arithmetic: v4 9,363.08 + v3 11,637.84 + 9.22 fee NOCK = 21,010; Aero is balanceOf-gated, so it reads 0 for a stranger.
The title is wrong for the operator: their config.yaml sets treasury_floor_nock explicitly (key present), so the code default only reaches newcomers. LiteCockpit would also show NaN for nock_floor_delta (line 147), not only for the target (148).

### ADV-06: nocknock-mcp fallbacks print a dead pool's frozen price, the operator's wallet, a static ACCRUE directive and '$0.00 due now' — skeptic: **confirmed**

- **Kind / severity / patch:** defect / high / yes
- **Locations:** `ingestion-worker/src/bin/mcp.rs:19`, `ingestion-worker/src/bin/mcp.rs:337`, `ingestion-worker/src/bin/mcp.rs:402`, `ingestion-worker/src/bin/mcp.rs:405-421`, `ingestion-worker/src/bin/mcp.rs:229`, `ingestion-worker/src/bin/mcp.rs:255`, `ingestion-worker/src/bin/mcp.rs:259`, `ingestion-worker/src/bin/mcp.rs:90`, `ingestion-worker/src/bin/mcp.rs:108`
- **Evidence:** Zero-DB mode:
- Reads spot from NOCK_USDC_POOL_ID 0x91a9ef0a…f99c (mcp.rs:337). That is the v4 0.75% pool, which config.yaml.example:35-36 records as having had no in-range liquidity since the 2026-11 liquidation. The worker fails its own cycle on exactly this condition (main.rs:4842-4849: 'Publishing that as "live" spot is a data-integrity bug').
- Reads the hardcoded operator wallet (mcp.rs:19), whoever runs it.
- Prints advice = 'ACCRUE (...)' as the 'Recommended Action' (mcp.rs:402).
- Says 'LP Positions: Uniswap-only portfolio' (mcp.rs:410), though an Aero pool is tracked.
- Says 'Status: Sovereign RPC Connected directly to Base mainnet.' even when both reads failed (mcp.rs:412).
The snapshot-API fallback still prints 'Next tranche: {} of {} — ${} due now' with tranche_usdc, which reads '$0.00 due now' while waiting (mcp.rs:229). v0.5.0 fixed that only in main.rs:2223-2237. The same fallback calls itself 'Tier 1 Connected (Verified Persistent Snapshot)' (mcp.rs:259) and hardcodes 'above 100k' (mcp.rs:255).
- **Impact:** If chudmax is unreachable, an AI agent is handed a frozen price as live spot and a static directive. Anyone other than the operator also gets the operator's balance. AGENTS.md classes silent mock fallbacks as high severity.
- **Proposed fix:** Zero-DB mode:
- 'NOCK Spot Price: unavailable in zero-DB mode', unless a configured pool passes the same in-range-liquidity check.
- Take the wallet from an environment variable; otherwise print 'Wallet balance: unavailable (no wallet configured)'.
- 'Recommended Action: unavailable — the advice engine runs in the worker, which is unreachable'.
- 'LP positions: not read in zero-DB mode'.
- Report each read's status separately.
Snapshot fallback: move the main.rs formatters into a module shared by both binaries, so the 'when due'/'due now' and floor text cannot drift again. The mode line becomes 'snapshot API fallback (worker MCP endpoint unreachable)'. Tool descriptions say 'tracked LP positions' instead of 'Uniswap'.

### ADV-07: For a new user, the example base layer supplies the operator's wallet, LP positions and limits — skeptic: **confirmed**

- **Kind / severity / patch:** risk / high / yes
- **Locations:** `ingestion-worker/src/main.rs:1618-1631`, `ingestion-worker/src/main.rs:252-254`, `ingestion-worker/config.yaml.example:11-12`, `ingestion-worker/config.yaml.example:33-88`, `ingestion-worker/config.yaml.example:91-116`, `ingestion-worker/src/main.rs:4805-4840`, `ingestion-worker/src/main.rs:5230-5260`, `ingestion-worker/src/main.rs:7122-7152`
- **Evidence:** config.yaml.example is loaded first and config.yaml overrides it key by key (main.rs:1618-1631; CHANGELOG 0.5.0, #86). Arrays are replaced only when the user's file sets them, so a partial config.yaml inherits:
- the operator's wallet (example:11-12, plus default_tracked_wallets() at main.rs:252-254);
- the operator's three positions (example:33-88);
- operator_profile: floor 100000, tax ratio 0.40, weights.
The position readers take an NFT id and never check ownership; there is no ownerOf anywhere in main.rs. The operator's v4 #3163450 and v3 #6336662 would therefore read as the newcomer's LP and feed their advice. An empty positions list fails every cycle instead ('No positions configured', main.rs:4805-4809), because spot requires a tracked price-source position (main.rs:4835-4840). test_example_config_is_the_base_layer pins the inheritance, including the floor (main.rs:7141).
- **Impact:** The from-zero test case cannot pass today. A newcomer gets either no snapshot or advice computed from someone else's book, and fixing the advice strings alone does not reach them.
- **Proposed fix:** Keep only protocol constants in the example (token addresses, eth_price_pool, market_data pool). Turn operator decisions (tracked_wallets, positions, operator_profile numbers, plan) into commented samples. Remove the wallet default. Check ownerOf on the v3 NPM and v4 POSM, and exclude or flag positions the wallet does not own. Allow positions: [] with a separately configured price-source pool. Coordinate with the live config first (open question 1). Replace the floor assertion at main.rs:7141 with one asserting that a partial config inherits no operator decisions.

### ADV-08: RANGE EXIT advice reports the whole LP book as the exited range's principal and invents a $5.00+ wall — skeptic: **confirmed**

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `ingestion-worker/src/main.rs:6041-6043`, `ingestion-worker/src/main.rs:6153-6162`
- **Evidence:** main.rs:6157: 'Spot ${:.4} has reached the top of the price-source range (${:.4})! Take principal (${:.2}) and rebuild single-sided high walls at $5.00+.' The principal is lp_usd (main.rs:6160), the sum over every position: $420.90 live, against $179.01 for the price-source range. $5.00 is a literal about 120x the live range top of $0.04106. Above its top a USDC/NOCK range holds only USDC, while a range above spot can only be funded with NOCK, so 'take principal and rebuild ... at $5.00+' cannot be followed as written.
- **Impact:** Dormant while the plan is active; reachable after it completes, or for other users. It combines a wrong number with an invented range.
- **Proposed fix:** 'Spot ${spot:.4} is at or above the top of the price-source range (${top:.4}). That range ({label}) has sold all its NOCK: it holds ${pos_usd:.2}, all USDC, and earns no fees unless the price falls back into it. Whether to leave it, withdraw it or open a new range is your decision; NockNock does not choose range bounds.' Use the price-source position's value. The '(Build High Wall)' parenthetical is an instruction tied to the deleted number, so change it to '(Above Range)' in the same patch and keep the 'RANGE EXIT' prefix.

### ADV-09: MCP portfolio text hardcodes 'above 100k' and invents zeros and HOLD for missing fields — skeptic: **confirmed**

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `ingestion-worker/src/main.rs:2136-2157`, `ingestion-worker/src/bin/mcp.rs:240-261`, `ingestion-worker/src/main.rs:8347-8350`
- **Evidence:** main.rs:2151, 'Floor buffer delta: {delta_prefix}{nock_delta} above 100k', ignores nock_floor_target, which is configurable (the test at 6544 uses 150000). A negative delta renders as '-30000 above 100k'. The fallbacks unwrap_or('0.00'), unwrap_or('0') and unwrap_or('HOLD') (main.rs:2138-2143, mcp.rs:242-247) print made-up values if a field is missing. The formatter renders the live snapshot as '113863 NOCK (Floor buffer delta: +13863 above 100k)', correct only because the operator's floor happens to be 100,000. test_mcp_text_formatters pins the string at main.rs:8349 ('+6140 above 100k').
- **Impact:** Any other floor, or no floor at all, is misreported to AI agents on both transports.
- **Proposed fix:** '• NOCK stack: {total} NOCK ({delta:+} vs the {floor} floor)'. When the delta is negative, '{abs} below the {floor} floor'; when the target is absent, '(no floor configured)'. Missing fields print 'n/a'. Update the 8349 assertion and add a 150000 case and a no-floor case.

### ADV-10: MCP shot advice states a 2x-gas claim rule the directive does not use, and 'net' fees are computed three different ways — skeptic: **partly**

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `ingestion-worker/src/main.rs:2159-2175`, `ingestion-worker/src/bin/mcp.rs:263-279`, `ingestion-worker/src/main.rs:6163`, `ingestion-worker/src/main.rs:5948-5956`, `ingestion-worker/src/main.rs:6194-6195`, `ingestion-worker/src/main.rs:2279-2288`, `frontend/src/App.tsx:353-358`, `specs/core/snapshot.yaml:624`
- **Evidence:** The text ends 'Execution rule: claim advised when claimable yield ≥ 2× live gas break-even.' (main.rs:2173), but BATCH HARVEST fires at a fixed total_fees_usd >= 5.00 (main.rs:6163). Live, fees are $0.30, which is ≥ 2 × $0.0037, so the stated rule says claim. The Recommended Action above it is DCA ON SCHEDULE, and the directive would wait for $5.00. 'Net Gas-to-Fee Profitability: +${gas_delta}' prints (fees − 2×gas).max(0.0) (main.rs:6195), always with a '+'. The same 'net' is computed three ways: fees − 2× heuristic gas on the dashboard (App.tsx:353-357), gross − the largest single-claim gas in the MCP claim simulation (main.rs:2279-2288), and gross − Σ estimateGas in the claim payload (main.rs:4372-4416). total_fees_claimable_usd is documented as 'Fees worth claiming after gas' but is simply set to total fees (main.rs:6194).
- **Impact:** An agent reading the MCP output is told a claim is advised when the engine says it is not. The per-position CLAIM_NOW for 3 cents also works against the operator's Koinly transaction budget (#53).
- **Proposed fix:** '• Harvest rules: the directive advises BATCH HARVEST once uncollected LP fees reach $5.00 (fixed); per position, claim_now means fees ≥ 2× estimated claim gas.' and '• Fees less 2× estimated gas: ${signed}', with no clamp and no forced '+'. Use one net definition everywhere: gross − Σ per-claim gas over claimable positions. Fix the spec description. Making the $5.00 configurable is #53, a feature.
- **Skeptic's correction:** Confirmed:
- the rule text (2173) does not match the fixed $5.00 arm (6163);
- the clamp and forced '+' (6195 and 2172);
- three net definitions (App.tsx:353-357; 2279-2288, max single gas; 4372-4416, Σ estimateGas);
- claimable = total fees (6194).
The stated consequence is wrong. Live, with the plan active, plan_advice (6135) outranks the $5.00 arm (6163). So the directive does not 'wait for $5.00': no harvest directive fires at any fee level until the plan completes.

### ADV-11: Dashboard fallbacks invent a HOLD directive and show a known zero as unknown — skeptic: **confirmed**

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `frontend/src/App.tsx:382`, `frontend/src/App.tsx:419-420`, `frontend/src/App.tsx:266-267`, `frontend/src/App.tsx:362`, `frontend/src/App.tsx:344`, `frontend/src/App.tsx:432`
- **Evidence:** {global?.shot_advice_action || 'HOLD & ACCRETE (Dual Growth Engine Active)'} (App.tsx:382), and a details fallback ending 'AMMs are generating dual yield. Maintain posture.' (419-420). Both are a directive and a yield claim that nothing computed, shown whenever the snapshot's strings are empty; GlobalStatus deserializes missing advice as '' (main.rs:1443-1446). nockAmountRaw > 0 ? … : '—' (267) shows a real 0 NOCK as unknown, and Floor Health goes blank (362). The '0.00' fallbacks for fees and USDC (344, 432) make up zeros.
- **Impact:** If the from-zero fix leaves the action empty for 'nothing configured', the dashboard would quietly show HOLD & ACCRETE. An empty portfolio looks like missing data rather than zero.
- **Proposed fix:** Action fallback 'Advice unavailable'; details fallback 'No advice text in this snapshot.' When the field is present, render 0 as '0' and '0.0%'; use '—' only when it is absent. Drop the '0.00' fallbacks.

### ADV-12: The Aero pool's unclaimable fees appear as '$0 earned' and as an RPC outage that blanks gas and net — skeptic: **confirmed**

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `ingestion-worker/src/main.rs:5611-5635`, `ingestion-worker/src/main.rs:5948-5956`, `frontend/src/App.tsx:786-791`, `frontend/src/App.tsx:438`, `frontend/src/components/LiteCockpit.tsx:260-271`, `ingestion-worker/src/main.rs:4184-4202`, `ingestion-worker/src/main.rs:4334-4356`, `ingestion-worker/src/main.rs:4372-4409`
- **Evidence:** claimFees() reverts on the Aero pool and the result reads as zero (main.rs:5614-5631). Its velocity entry is claimable 0.00, efficiency 0.0, 'accumulate', rendered as 'ACCUMULATE ($0 earned / gas $0.0037 — 0x ROI)' (App.tsx:789). Staked LP earns AERO, which is not read (#93). App.tsx:438 nonetheless says 'Unclaimed fees live-tracked on-chain.', and the Lite card says 'UNCOLLECTED FEES (ALL POSITIONS)'. The claim payload still builds an Aero claim. Its reverting eth_call is reported as 'RPC unavailable for collect simulation' (main.rs:4200), and its failed estimateGas sets gas_ok=false (main.rs:4400-4402), wiping the totals. Live GET /v1/execute/claim-fees/payload: v4 gas $0.0014, v3 gas $0.0027, aerodrome simulation 'unavailable' with detail 'RPC unavailable for collect simulation', estimated_gas_usd null, net_after_gas_usd null. The Lite cockpit therefore shows 'est. gas $— · net after gas $—'.
- **Impact:** The phone cockpit shows a false outage message and no gas or net figures, and a reward stream NockNock never reads is shown as zero.
- **Proposed fix:** Show such pools as not LP-claimable rather than zero. Velocity text: 'no LP-claimable fees (claimFees() reverts on this pool); AERO rewards not read (#93)', with a recommended_action such as not_claimable. App.tsx:438: 'Uniswap LP fees are read on-chain; rewards on staked Aero LP are not read yet.' Lite label: 'UNCOLLECTED LP FEES'. In the payload, either skip the Aero action when the snapshot reads zero LP-claimable fees, or simulate with eth_call_allow_revert and report 'claimFees() reverts on this pool: nothing to collect'. Total the gas over actions whose simulation succeeded.

### ADV-13: LP-rebuild text says it moves only NOCK and USDC, but it funds ETH legs from gas; composition labels count LP WETH as stable — skeptic: **confirmed**

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `ingestion-worker/src/main.rs:6096-6099`, `ingestion-worker/src/main.rs:727`, `ingestion-worker/src/main.rs:945-952`, `ingestion-worker/src/main.rs:1348-1354`, `ingestion-worker/src/main.rs:2186`, `frontend/src/components/AccumulationPlanPanel.tsx:81-84`, `ingestion-worker/src/main.rs:938-943`, `ingestion-worker/src/main.rs:980`, `frontend/src/App.tsx:690-691`, `frontend/src/App.tsx:704`, `ingestion-worker/src/main.rs:6030`, `specs/core/snapshot.yaml:615`
- **Evidence:** eth: Some(e + w - REBUILD_GAS_BUFFER_ETH) (main.rs:6096-6099, buffer 0.002 ETH at main.rs:727) funds WETH/NOCK deposits from wallet ETH. The advice says 'Re-establishing ranges from NOCK and USDC already held is a reposition, not a purchase: the stable total is unchanged' (main.rs:1349, repeated in the panel and the MCP text). Under today's #94 decision ETH and WETH are gas, counted but not deployable, and a WETH/NOCK range below spot buys NOCK with that ETH as the price falls. Two reason strings are also wrong:
- main.rs:941 says 'after the cash target, tax reserve and remaining DCA tranches', but the code keeps only the larger of the cash target and the tax reserve plus the plan remainder (main.rs:6086).
- main.rs:980 says 'amount below $1', but the cutoff is $5 (LP_SUGGESTION_MIN_USD).
'Stable Reserve Delta (Total − Native-Only) … the USDC side' (App.tsx:704) is $1,077.65 live against $1,033.71 of USDC. The $43.94 difference is the v3 range's 0.0239 WETH, because total_native_usd counts NOCK only (main.rs:6030) while the spec says 'NOCK, ETH, etc.'
- **Impact:** Rebuild advice can tell the operator to deploy gas money while saying it moves only NOCK and USDC. The 2026-11-14 deposit of 0.082 ETH described in #94 is the precedent.
- **Proposed fix:** Together with #94: give the rebuild zero ETH funding and exclude those positions with the reason 'needs ETH/WETH, which is gas, not investable capital (operator decision 2026-11-14)', so the sentence stays true. Reason strings: 'only $X of wallet USDC is free after keeping the larger of the cash target ($A) and the tax reserve plus unspent plan budget ($B)', and 'amount below the $5 minimum'. Composition label: 'Non-NOCK value (USDC + ETH in LP)' until #94 adds a gas line. Spec line 615: 'USD value of NOCK held (wallet + LP)'.

### ADV-14: Flavour text that contradicts the four-pillar framing (rename later, not in the patch) — skeptic: **confirmed**

- **Kind / severity / patch:** design / low / no
- **Locations:** `ingestion-worker/src/main.rs:6178`, `frontend/src/App.tsx:286`, `frontend/src/App.tsx:424-433`, `ingestion-worker/src/main.rs:6309`, `ingestion-worker/src/main.rs:2155`, `ingestion-worker/src/main.rs:6126-6134`, `frontend/src/App.tsx:796-818`, `frontend/src/App.tsx:789`, `ingestion-worker/src/main.rs:6163`, `specs/core/snapshot.yaml:653`
- **Evidence:** '(Dual Growth Engine Active)', the 'Dual-Growth Engine' badge, and 'Goal 1 (NOCK Accretion)' / 'Goal 2 (USDC Accumulation)' frame two growth targets. The operator's position is four pillars, each neither maximized nor minimized. Other items:
- 'Liquid USDC (Escrow/Vault)': no escrow or vault exists.
- 'Tier 1 Connected (Verified Persistent Snapshot)'.
- 'Regression channel ... band squeezes precede volatility expansion': the series is SMA ± σ (main.rs:5861-5915), and the second clause is a market prediction.
- 'x ROI' is fees ÷ gas.
- 'SWEEP TO STABLE' is ambiguous: it could mean harvested fees, or a NOCK→USDC sale NockNock should not choose.
- '$5.00 threshold' is accurate but is a code constant (#53).
- **Impact:** Tone and framing only; none of these is a wrong computed number.
- **Proposed fix:** Defer to a minor release. When renaming, keep or migrate the prefixes that code relies on: apply_lp_rebuild_bias idle detection (main.rs:1393-1397), notification transitions, and the tests at 6503, 7053, 7091 and 8353. Suggested names: 'HOLD', 'LP / range status', 'Liquid USDC', 'Mode: worker snapshot', 'Bollinger bands (20-period SMA ± 1σ/2σ)'.

### Proposed design

**Principle for v0.5.1:** every figure in advice comes from config or the chain. A figure with no source is left out, never defaulted. Text says what NockNock read or suggests, never what funds 'are' (locked, protected, escrowed, earned).

**Design**
1. **#95.** Port positionsSummary to a Rust lp_summary helper (over raw.positions and raw.pool_liquidity) and use it in the HOLD text. Drop every P/L claim until cost basis exists (#98).
2. **No-plan buy arms.** Delete the DIP REINVESTMENT and Tax Guardrail arms (main.rs:6137-6152) with their $0.020, 120,000 and 'at discount' literals. With no plan, size nothing and say the numbers are the user's.
3. **Operator decisions become optional.** treasury_floor_nock and target_weights_bps become Option; whether tax_reserve_ratio follows is open question 3. When one is unset, skip what depends on it: the floor directive, the cash-target cap, LP target and rebuild sizing, and the deviation status. nock_floor_target is then absent, and default_floor_target_string() returns ''. Tests that build OperatorProfile::default() assume a 100,000 floor (lines 6481, 6493, 6501, 6513, 6616, 6716, 7011 via metrics_with_plan, 7411, 8399, 8411, 8747, 8778); give them an explicit test profile rather than keeping a production default.
4. **Text built from config.**
   - Harvest and ledger text use the configured tax ratio; add an optional sovereign_ledger.tax_reserve_ratio field.
   - MCP text uses the configured floor.
   - RANGE EXIT uses the price-source range's own value; delete '$5.00+'.
   - The $5.00 BATCH HARVEST threshold stays a named constant whose text says 'fixed' until #53.
5. **High-water mark.** Emit the real peak date. The UI says 'highest recorded since …' and 'change from recorded peak'.
6. **One MCP formatting module,** shared by the worker and by nocknock-mcp's snapshot fallback. Zero-DB mode prints 'unavailable' for anything it cannot read live: no wallet default, no dead-pool price, no static directive.
7. **Aero pools whose fees cannot be claimed** are shown as 'not LP-claimable; rewards not read (#93)', never as $0 earned. Claim-payload totals cover only actions that can execute.
8. **With #94,** wallet ETH and WETH never fund LP rebuilds, and the composition labels name the WETH leg.
9. **From-zero config.** First confirm the live config sets its own wallet, positions and limits. Then reduce config.yaml.example to protocol constants and check position ownership with ownerOf.
10. **From-zero advice text, with no invented numbers.**
    - When the wallet and LP are empty, the action is 'NOTHING TRACKED YET' and the details read: 'The wallet holds 0 NOCK and $0.00 USDC, and no LP positions are tracked. NockNock does not choose amounts: set your own accumulation_plan (budget_usdc, tranches, interval_hours) and, if you want one, treasury_floor_nock; NockNock then paces each buy and verifies it on-chain.'
    - With cash but no plan: 'The wallet holds {nock} NOCK and ${usdc} USDC; NOCK is ${spot}. No accumulation plan is configured, so NockNock does not size a buy.'
11. **Flavour renames wait for 0.6.0.** Keep the action prefixes that code keys on.

**Existing tests that assert these strings and must change** (all in main.rs unless noted)
- 6476: DEFENSIVE LOCK, relies on the default floor.
- 6499: HOLD; add the #95 assertions.
- 6544: floor equality at 6565; type change.
- 6594: default ratio 0.40, if that default goes.
- 6710: Tax Guardrail at 6722-6724; DIP sizing including 'Deploy $' at 6741-6750.
- 7017: legacy 'DIP REINVESTMENT SHOT' at 7023.
- 7122: positions and floor inherited from the example, 7137-7142.
- 7556: ETH funding, 7565-7570.
- 7597: the primary directive under PlanContext::none changes.
- 8310: '+6140 above 100k' at 8349.
- 8363: 'Net Accretion if Harvested: $0.0404' at 8377, if net becomes Σ gas.

**Recheck, probably unchanged:** 6825 (6832-6834), 7037 (7046, 7053-7055, 7061-7062), 7067, 7080 (7091), 7362 (7374), 7397 (7425), 7675 (7700, 7708), 7729 (literal inputs), and 8724/8764 (8754, 8784-8785).

**Elsewhere**
- **Frontend:** no test covers the App.tsx, LiteCockpit or AccumulationPlanPanel strings. client.errors.test.ts:239-263 covers positionsSummary only, and freshness.contract.test.ts:78-89 checks only that the ledger amounts add up.
- **scripts/playtest-mcp.py** (74-82, 120-129) pins only the MCP header strings.

### Tests to add

- main.rs test_hold_advice_states_no_pnl_and_names_lp_positions (#95): four-position fixture plus an Aero full-range position. HOLD details contain 'LP position' and the in-range and full-range counts, and contain none of 'realized', '0%', 'Uniswap position' or 'generating yield'.
- main.rs test_hold_advice_replays_the_2026_11_14_empty_book: four tracked positions with zero liquidity (the 05:36Z state). No position is described as yielding, and fees read $0.00.
- main.rs test_from_zero_empty_portfolio_invents_no_numbers: 0 NOCK, 0 USDC, no positions, and no floor, weights or plan configured. No DEFENSIVE LOCK; details contain none of 'CRITICAL', 'locked', '100000', '120,000', '$0.020', '$5.00', 'Deploy $' or 'realized'. nock_floor_target is empty, there are no LP suggestions, and no deviation status is computed against default weights.
- main.rs test_from_zero_cash_only_without_plan_sizes_no_buy: $500 USDC, 0 NOCK, spot $0.0115, no plan. No 'DIP REINVESTMENT' and no buy amount; the text says budget, tranches and interval are the user's to set.
- main.rs test_from_zero_with_user_plan_uses_only_user_numbers: the same wallet with a plan of budget 100, 5 tranches, interval 24 gives 'DCA TRANCHE DUE (1 of 5)' and 'Buy $20.00 of NOCK', and no number that does not come from config or the chain.
- main.rs test_floor_breach_text_never_claims_funds_are_locked: a user floor of 50,000 with 0 NOCK. The text names 50000 and the deficit, and contains none of 'locked', 'CRITICAL' or 'sovereign foundation'.
- main.rs test_old_snapshot_without_floor_target_does_not_invent_100000: deserializing a GlobalStatus that lacks nock_floor_target yields an empty target.
- main.rs test_range_exit_uses_price_source_value_and_no_invented_targets: when the price-source range has exited, details carry that range's USD value (not the LP total) and no '$5.00'.
- main.rs test_harvest_and_ledger_text_follow_configured_tax_ratio: with ratio 0.25 the BATCH HARVEST text says 25% and 75%, never '60/40' or '40%'.
- main.rs test_high_water_mark_date_is_when_the_peak_was_set: previous peak 2475.27 at T0 and current value 2390.53 keep the date at T0; a new peak sets it to now.
- main.rs test_mcp_portfolio_text_uses_configured_floor: target 150000 with delta -30000 renders '30000 below the 150000 floor', and an absent target renders 'no floor configured'. Replaces the '+6140 above 100k' assertion at 8349.
- main.rs test_mcp_shot_advice_names_the_directive_harvest_threshold: the text names the $5.00 directive threshold, and a negative fees-less-gas figure is never printed with '+$'.
- main.rs test_claim_payload_totals_ignore_non_claimable_pools (pure helper): one reverting Aero claim does not null estimated_gas_usd or the net for the executable actions, and its detail is not 'RPC unavailable'.
- main.rs test_lp_rebuild_never_funds_from_gas (with #94): wallet ETH 0.0125 and an eligible WETH/NOCK range. The range is excluded with a gas reason, and the rebuild sentence still names only NOCK and USDC.
- main.rs test_partial_config_inherits_no_operator_decisions: a config.yaml with only a key and a wallet takes no positions, floor, weights or plan from the example (the inverse of 7137-7142).
- bin/mcp.rs unit tests: while waiting, the snapshot-fallback plan text says '$X when due', never 'due now'. Zero-DB mode prints no wallet balance without a configured wallet, no spot when the pool has no in-range liquidity, and no static 'ACCRUE' directive.
- frontend vitest:
- an empty shot_advice_action renders 'Advice unavailable', not HOLD & ACCRETE;
- total_nock_amount '0' renders 0;
- ledger labels use the snapshot's ratio, with no 100% bar when there are zero harvests;
- the HWM subtitle never says 'All-time';
- the Aero velocity row says rewards are not read;
- LiteCockpit renders 'no floor set', not NaN, for an empty nock_floor_target.

### Open questions

- Does the live config.yaml on chudmax set tracked_wallets, positions, treasury_floor_nock, target_weights_bps and tax_reserve_ratio itself, or does it rely on config.yaml.example? The #86 test describes an operator config with only a key and a port. Check which keys are present without printing values before removing operator numbers from the example or from the code defaults.
- With no plan configured, should NockNock say nothing about buying, or show an informational 'NO ACCUMULATION PLAN' directive? Should the old $0.020 trigger survive only as the plan's max_price_usd?
- Is tax_reserve_ratio 0.40 acceptable as a documented default, or should it be explicit like the floor, since it is an allocation number under the standing order?
- For #94: is all ETH and WETH off-limits for LP rebuilds, or may WETH already set aside for LP fund WETH/NOCK deposits?
- Keep the fixed $5.00 BATCH HARVEST threshold and the per-position 2x-gas claim_now until #53, or suppress claim_now below $5.00 now so the two rules agree and 3-cent claims stop being flagged?
- Should the HOLD text mention that AERO rewards are not read (#93), or leave that to the positions panel?
- Should a snapshot be possible with no positions, using a price-source pool independent of owned NFTs, so the from-zero case works end to end? If so, is that v0.5.1 or a v0.6.0 feature?

### Skeptic: problems with the proposed design

- **ADV-07's test plan targets the wrong test.** It says to replace the assertion at main.rs:7141, but test_example_config_is_the_base_layer (7122-7150) writes a synthetic config.yaml.example to a temp dir and never reads the shipped file. Stripping operator numbers from the real example is therefore untested; a new test must load ingestion-worker/config.yaml.example (via CARGO_MANIFEST_DIR) through load_app_config. Replacing 7141 only loses nested-merge coverage. The operator's own config sets every operator key (key presence checked), so the change is safe for chudmax. default_tracked_wallets (main.rs:252-254) and mcp.rs:19 must also change.
- **ADV-13's list of tests to update is incomplete.** Excluding ETH-legged positions inside lp_rebuild_suggestions breaks unit tests that expect WETH/NOCK or NOCK/ETH suggestions with eth Some(10.0):
- 7461-7477: s.len()==3, including 3163432 and 6336662;
- 7485-7495 and 7506-7513: sg(&s, 6336662);
- 7518-7540: every suggestion halved.
If the change is instead done by passing eth Some(0.0) at main.rs:6096, the exclusion reason at 948 reads 'wallet holds 0.00000 ETH/WETH after a 0.002 ETH gas buffer'. That is false while the wallet holds about 0.0125 ETH; a separate gas reason is required.
- **ADV-03's proposed BATCH HARVEST text would itself be untrue.** It says NockNock books r% of the harvest's 'USD value'. parse_harvest_transfers (main.rs:3800-3806) records only USDC and NOCK; WETH fees from the v3 WETH/NOCK range are skipped (about 62% of that position's live fees). NOCK is valued at the latest snapshot spot at recording time (4564-4567), not at the block.
- **ADV-05's replacement text is false whenever lp_floor_tolerance_bps > 0.** The text says 'NockNock will not suggest putting floor NOCK into LP', and the App.tsx:426 replacement says 'Below {floor}, NockNock stops suggesting LP deposits of NOCK'. Rebuild funding allows wallet_nock - floor + tolerance (main.rs:6087-6093), and test 7397 asserts LP NOCK may exceed what sits above the floor. The operator's config sets lp_floor_tolerance_bps.
- **ADV-10's proposed MCP rule text is false while a plan is active, which is live now.** The text says 'directive advises BATCH HARVEST once fees reach $5.00'. The plan, floor, cooldown and range-exit arms (6118-6162) all outrank the $5.00 arm (6163). The text must name that precedence, or the engine must change.
- **ADV-02 fix:** MAX(total_portfolio_usd) has long ties: 2475.27 held from 04:38:29Z to 05:57:48Z. The query must take the earliest tie (ORDER BY value DESC, generated_at ASC); the report's own example picks the last tie. 'Highest recorded since {date}' is wrong if {date} is the peak date: 'since' should name the database's first snapshot.
- **Optional floor (design item 3).** nock_floor_delta also becomes empty, so LiteCockpit.tsx:147 shows 'NaN NOCK' in red, not only the target at 148. With floor None, rebuild NOCK funding (main.rs:6093) has no bound. The design must say whether rebuild sizing is skipped or the whole wallet stack becomes LP-eligible; the latter would let ranges sell a newcomer's entire NOCK.
- **ADV-14 marks SWEEP TO STABLE as flavour, but its details instruct the user.** They say 'sweep to the stable stack until momentum cools' (main.rs:6130), which is a NOCK→USDC allocation push. It is live-reachable for the operator: with pause_when_overextended defaulting to true, it outranks the active plan whenever daily RSI ≥ 70. Under the standing order and the 'neither minimize nor maximize any pillar' decision, the instruction clause belongs in the patch.
- **ADV-06's shared formatter module needs crate restructuring.** Cargo.toml has only two [[bin]] targets and there is no src/lib.rs, so 'a module shared by both binaries' means adding a lib target or #[path] includes. For a patch, fixing mcp.rs:229, 255 and 259 in place is smaller.

### Skeptic: missed in this area

- **nocknock-mcp zero-DB mode sends malformed balanceOf calldata** (`ingestion-worker/src/bin/mcp.rs:355`, `ingestion-worker/src/bin/mcp.rs:412`, `ingestion-worker/src/main.rs:4924`): mcp.rs:355 builds format!("0x70a082310000000000000000{}", WALLET_ADDR...). That is 16 hex pad characters plus 40 for the address, a 28-byte argument; main.rs:4924 correctly pads with 24 zeros. A token with Solidity's calldata-size check reverts, so the wallet balance always reads 'Unavailable'. Otherwise it reads balanceOf of a shifted address. Either way, 'Status: Sovereign RPC Connected directly to Base mainnet.' is printed unconditionally (412).
- **MCP on both transports labels a stale snapshot 'Live API' with no timestamp or health** (`ingestion-worker/src/main.rs:2401-2412`, `ingestion-worker/src/main.rs:2148`, `ingestion-worker/src/main.rs:2155`, `ingestion-worker/src/main.rs:1659-1688`, `ingestion-worker/src/main.rs:2671-2674`, `ingestion-worker/src/bin/mcp.rs:252`, `ingestion-worker/src/bin/mcp.rs:259`): tools/call formats state.latest_snapshot, which is loaded from the database at boot and replaced only after a successful cycle. The text is headed '(Live API)' and 'Tier 1 Connected (Verified Persistent Snapshot)' but never includes generated_at, even though /health already computes 'degraded' (1590-1595). During failing cycles, or right after boot on an old database, agents get hours-old figures presented as live.
- **The floor directive outranks an active DCA plan, which defeats the from-zero journey** (`ingestion-worker/src/main.rs:6118`, `ingestion-worker/src/main.rs:6135`, `ingestion-worker/src/main.rs:1393-1397`, `ingestion-worker/src/main.rs:3749-3752`): total_nock < floor is checked before active_plan, so a user below their own floor never gets 'DCA TRANCHE DUE' as the directive or as a notification, which fires only on an action change. Meanwhile the plan panel says TRANCHE DUE. Every from-zero user who sets a floor and a plan is stuck on DEFENSIVE LOCK. The report's from-zero tests never combine a floor with a plan. The operator would hit the same in a rally: the live rebuild text says 90,098 NOCK would remain if every range sold out.
- **The 60/40 harvest ledger drops WETH fee income and values NOCK at recording-time spot** (`ingestion-worker/src/main.rs:3800-3806`, `ingestion-worker/src/main.rs:4564-4567`, `frontend/src/App.tsx:738`, `frontend/src/components/LiteCockpit.tsx:322`): parse_harvest_transfers skips any token other than USDC or NOCK. Live v3 #6336662 fees are 0.000093 WETH + 9.2194 NOCK = $0.278, of which about $0.17 (62%) is WETH and would never reach the ledger. NOCK usd_value uses the latest snapshot spot when the hash is recorded, not at confirmed_at. So '% of realized harvests' and 'Verified receipt recorded in the 60/40 ledger' understate income, and so does the tax reserve built from it.
- **The dashboard repeats the 'above floor' sign bug and rounds small floors** (`frontend/src/App.tsx:336`, `frontend/src/App.tsx:265`, `frontend/src/App.tsx:344`): The Total NOCK Stack subtitle renders `${delta} above ${floorTargetShort} floor` for negative deltas ('-30,000 above 150k floor'), the dashboard twin of ADV-09; ADV-05 lists only lines 364 and 426. floorTargetShort = (floor/1000).toFixed(0)+'k' turns a newcomer floor of 1,500 into '2k' across the cards and the advice fallback. Line 344 also shows all fee value ($0.30, about 92% NOCK and WETH) as '+ USD fees accrued' under the USDC stack.
- **Copying the example's notifications section silently disables Pushover** (`ingestion-worker/config.yaml.example:174`, `ingestion-worker/src/main.rs:3736`, `ingestion-worker/src/main.rs:2807`): The example documents `pushover_user_key`, but NotificationConfig reads `pushover_user` (no serde alias). The unknown key is ignored and `if let (Some(token), Some(user))` never matches, so directive texts are never pushed and nothing is logged. A newcomer following the example would never receive DCA TRANCHE DUE pushes.


## docs-lore: documentation: lore doc and stale statements

**Summary.** Reviewed the requested docs, deploy/podman/*, the example config, the live chudmax snapshot (read-only GETs), issues #93-#96, and the in-progress branch fix/v0.5.1. That branch already rewrites AGENTS.md §13 (HEAD said 'Treat it as production') and has a lore draft at docs/lore/2026-11-14-four-pillars-no-hurry.md (FROM-ZERO section still a placeholder at :148). The biggest problems are in the from-zero path, where docs and behaviour disagree. (1) config.yaml.example is the base layer and ships the operator's wallet, positions, 100,000 floor, 60/40 ratio and 80/15/5 targets. v3/v4 positions are read without an ownership check, so a newcomer who only adds a key inherits someone else's LP and the operator's floor. (2) An empty-LP wallet cannot ingest at all. (3) The cash reserve exists only inside an accumulation plan, so with no plan the engine sizes a lump-sum 'DIP REINVESTMENT SHOT' toward a hardcoded 120,000 NOCK. Policy conflicts. 'Nothing is called a backup' is false in the repo (units, script, Makefile, DEPLOYMENT.md, CHANGELOG). 'Plan progress restarts after a wipe' is avoidable: fills re-record from the chain once starts_at is set. 'ETH is never spent' conflicts with LP rebuild drawing on wallet ETH above a 0.002 ETH buffer. Stale facts: README is a retired portfolio with the operator's numbers presented as the strategy. ONCHAIN_INTEGRATION has three statements that became false in v0.5.0. Today's decisions are not yet in OPERATOR_GUIDANCE §7, and HANDOFF is a v0.2.1 dossier that AGENTS §10 still cites for vision. The lore draft also has wording errors: it says the plan 'buys', double-counts the first $20 fill, overstates 'read from the chain', and misplaces config.yaml. proposed_design keeps the draft's filename and gives an outline, the FROM-ZERO content, and a glossary compiled from what the docs already say.


### DOC-01: Example config (base layer) hands a newcomer the operator's wallet, positions, floor and targets; docs only say 'fill in your Alchemy key' — skeptic: **confirmed**

- **Kind / severity / patch:** defect / high / yes
- **Locations:** `ingestion-worker/config.yaml.example:2`, `ingestion-worker/config.yaml.example:11-12`, `ingestion-worker/config.yaml.example:33-78`, `ingestion-worker/config.yaml.example:95`, `ingestion-worker/config.yaml.example:106`, `ingestion-worker/config.yaml.example:112-115`, `ingestion-worker/src/main.rs:1637-1639`, `ingestion-worker/src/main.rs:7136-7142`, `ingestion-worker/src/main.rs:5424`, `ingestion-worker/src/main.rs:5758`, `ingestion-worker/src/main.rs:6118-6124`, `README.md:118-167`, `docs/lore/2026-11-14-four-pillars-no-hurry.md:156-157`
- **Evidence:** config.yaml.example:2 'Copy to config.yaml and fill in your Alchemy key'. main.rs:1637 'config.yaml.example is the base layer, config.yaml overrides it'. The test at main.rs:7136-7142 asserts a partial config inherits positions and treasury_floor_nock 100000 ('Nested sections merge key by key'). The v4/v3 readers use the wallet only as a label (main.rs:5424, 5758); there is no ownerOf check. README Quick Start has no config step.
- **Impact:** A newcomer who sets only key and wallet inherits the operator's positions. The operator's Uniswap LP (~$357 live) counts as theirs: fabricated holdings, which is a mock-data bug. They also inherit the 100,000 NOCK floor (immediate DEFENSIVE LOCK: 'stable reserves are locked to replenish the sovereign foundation'), tax_reserve_ratio 0.40 and 80/15/5 targets. This contradicts the 'never choose a user's numbers' standing order and the lore's 'Treat them as an example, not a template.'
- **Proposed fix:** Docs now: README Quick Start and the lore FROM-ZERO section list the keys a new user must set or explicitly zero (tracked_wallets, positions, operator_profile.treasury_floor_nock, tax_reserve_ratio, target_weights_bps). Say plainly that the example values are one operator's and load as the base layer. Reword config.yaml.example:2. Code (v0.5.1 from-zero): ship neutral example values (no positions, no floor, no targets) or keep operator values out of the base layer. Verify NFT ownership against the tracked wallet and reject or flag foreign positions.

### DOC-02: No doc explains running with zero LP; ingestion refuses an empty positions list and spot needs a position — skeptic: **confirmed**

- **Kind / severity / patch:** gap / high / yes
- **Locations:** `ingestion-worker/src/main.rs:4805-4809`, `ingestion-worker/src/main.rs:4835-4840`, `ingestion-worker/src/main.rs:5002-5004`, `DEPLOYMENT.md:182-184`, `CHANGELOG.md:39-40`
- **Evidence:** main.rs:4807 'No positions configured; add entries under `positions:` in config.yaml'. Spot comes only from a position flagged is_price_source (main.rs:4835-4840). CHANGELOG.md:39 'A volatile pool can serve as the NOCK/USD price source (spot = reserve ratio)'.
- **Impact:** The from-zero user (empty wallet, no LP) gets no snapshot and a degraded /health. The only workaround is undocumented: an `aerodrome` entry for a public NOCK/USDC pool holding zero LP, flagged is_price_source. It works because the reader takes balanceOf(wallet)=0 and prices from reserves.
- **Proposed fix:** Docs now: document the price-source-only entry for wallets without LP, and state that its zero balance is real data, not a placeholder. Code (from-zero): allow an empty positions list with a separate price-source pool setting, producing a snapshot of wallet balances only.

### DOC-03: The cash reserve exists only inside an accumulation plan; with no plan the engine sizes a lump-sum DIP REINVESTMENT SHOT — skeptic: **partly**

- **Kind / severity / patch:** defect / high / yes
- **Locations:** `docs/lore/2026-11-14-four-pillars-no-hurry.md:30`, `OPERATOR_GUIDANCE.md:113`, `README.md:69`, `ingestion-worker/src/main.rs:183`, `ingestion-worker/src/main.rs:6053-6056`, `ingestion-worker/src/main.rs:6145-6151`
- **Evidence:** Lore:30 'A reserve the engine never advises spending below.' cash_reserve_usdc is a field of AccumulationPlanConfig (main.rs:183). main.rs:6149 'Deploy ${:.2} of saved USDC ... compound buffer toward 120,000.'
- **Impact:** With no plan configured and spot below $0.020 (spot is ~$0.0115 now), the engine advises a one-shot deployment of wallet USDC down to 5% of the portfolio, toward a 120,000 NOCK goal nobody chose. This hits any newcomer, and the operator too if the plan section is ever removed. The lore's no-hurry promise and the 'never choose numbers' order hold only while a plan exists.
- **Proposed fix:** Docs: state that the reserve binds only while an accumulation_plan is configured, and fix the README §2 dip row. Code (from-zero): replace the lump-sum arm with 'set up a plan: budget, tranches and interval are yours', or lift cash_reserve_usdc into operator_profile so it applies without a plan. Drop the 120,000 constant.
- **Skeptic's correction:** The core claim holds. cash_reserve_usdc exists only on AccumulationPlanConfig (main.rs:183). The no-plan arm (6145-6151) sizes deployable = min(stable − tax reserve, wallet USDC − 5% cash target) and says 'toward 120,000'. For the operator without a plan today that reads 'Deploy $811.25' (930.78 − 0.05 × 2,390.52). The consequence is overstated, though. DEFENSIVE LOCK (6118-6127) runs first, so a newcomer below the inherited or default 100,000 floor sees the lock, not the lump sum. That holds even with a plan configured, so DCA advice is also unreachable for them. The lump-sum shot reaches a newcomer only after their floor is met (for example set to 0, as DOC-01 suggests) and wallet USDC exceeds the cash target by at least $1 (PLAN_DUST_USDC). OPERATOR_GUIDANCE.md:113 is itself accurate ('Plans carry…'). The problem is the lore's unqualified line 30.

### DOC-04: 'Nothing is called a backup' is false in the repo; the backup script's rationale contradicts 'database is disposable'; #96 still OPEN — skeptic: **partly**

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `docs/lore/2026-11-14-four-pillars-no-hurry.md:129`, `deploy/podman/nocknock-backup.sh:2-7`, `deploy/podman/nocknock-backup.sh:24`, `deploy/podman/nocknock-backup.sh:35`, `deploy/podman/nocknock-backup.service:2`, `deploy/podman/nocknock-backup.timer:2`, `deploy/podman/nocknock-backup.timer:6`, `deploy/podman/deploy.sh:44-51`, `DEPLOYMENT.md:169-174`, `DEPLOYMENT.md:209-215`, `Makefile:175-187`, `CHANGELOG.md:52-54`, `AGENTS.md:217-221`
- **Evidence:** nocknock-backup.sh:4 'The database holds records that exist nowhere else in this form'. DEPLOYMENT.md:172 'copy them elsewhere if the host itself is at risk.' AGENTS.md:221 (working tree) says #96 'closed as not planned', but gh shows #96 OPEN.
- **Impact:** The lore draft and the AGENTS.md edit state a policy the repo contradicts. The contradictions are in unit names and descriptions, script comments, Makefile targets ('DISASTER RECOVERY', db-backup/db-restore) and DEPLOYMENT.md. DEPLOYMENT.md still recommends the off-host copy that #96 proposed and the operator dropped.
- **Proposed fix:** The operator chooses: (a) delete nocknock-backup.* and deploy.sh:44-51, or (b) rename to nocknock-dump.* and `make db-dump`, with the script header 'convenience dump; the wallet and chain are the record'. Either way: replace DEPLOYMENT.md:169-174 with one 'database is disposable' line, retitle §5 at :209, and add an [Unreleased] CHANGELOG note (leave the released 0.5.0 and 0.1.0 entries). Close #96 as not planned before AGENTS.md:221 is committed. Keep lore:129 only once the names agree.
- **Skeptic's correction:** The backup wording is confirmed everywhere cited: nocknock-backup.sh:2, 4-7, 24 and 35; service:2; timer:2 and 6; deploy.sh:44-51; DEPLOYMENT.md:169-174 (including :172 'copy them elsewhere') and 209-215; Makefile:175-186 ('DISASTER RECOVERY', db-backup and db-restore); CHANGELOG.md:52-54, plus :481 (0.1.0, 'Disaster recovery database tooling'). One claim is wrong: #96 is now CLOSED, stateReason NOT_PLANNED, at 2026-11-14T18:13:06Z, with a comment recording the operator's decision. So AGENTS.md:221 is accurate and the 'close #96 first' step is moot. The script header also concedes that the ledgers 'can be re-derived from the chain' (:6). There are two more collisions with lore:129: AGENTS.md:217 says 'Back it up before editing', and the gitignored repo-root backups/ folder holds config copies and an Oct-11 db-backup dump.

### DOC-05: 'Plan progress restarts' after a wipe is avoidable: fills re-record from the chain once starts_at is set — skeptic: **confirmed**

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `AGENTS.md:202-204`, `docs/lore/2026-11-14-four-pillars-no-hurry.md:129-131`, `ingestion-worker/src/main.rs:2731-2733`, `ingestion-worker/src/main.rs:3991-4001`, `ONCHAIN_INTEGRATION.md:184-186`
- **Evidence:** AGENTS.md:203 (working tree) 'the accumulation plan's progress restarts with it'. main.rs:3997 'To count earlier buys, set accumulation_plan.starts_at to an earlier time.'
- **Impact:** On an empty database the plan anchor becomes 'first sight', so old fills are rejected with BEFORE_PLAN_START unless starts_at is set. If progress just restarts, the plan re-advises its whole $100 budget although $20 is already spent (live: fills 1, spent_usdc 20.00): an unintended overspend. Re-recording from transaction hashes is exactly what 'the wallet is the record' means.
- **Proposed fix:** Document the fresh-start step. Set accumulation_plan.starts_at to the plan's real start (at or before 2026-11-14T06:02:25Z for 2026-11-reentry). Then POST /v1/execute/record-accumulation once per fill hash from the wallet history (fill 1: 0x5d0c02d9…3f0e, ONCHAIN §6). If the operator prefers a clean restart, say to lower budget_usdc by the amount spent or start a new plan id. Harvests re-record the same way (none recorded yet).

### DOC-06: Lore says ETH is never spent on NOCK, but LP rebuild funds WETH/NOCK deposits from wallet ETH above a 0.002 ETH buffer — skeptic: **confirmed**

- **Kind / severity / patch:** design / medium / yes
- **Locations:** `docs/lore/2026-11-14-four-pillars-no-hurry.md:31`, `docs/lore/2026-11-14-four-pillars-no-hurry.md:46-47`, `CHANGELOG.md:94-96`, `ingestion-worker/src/main.rs:727`, `ingestion-worker/src/main.rs:6096-6099`, `docs/kb/design-note-dca-accumulation-plan.md:258-259`, `docs/kb/design-note-dca-accumulation-plan.md:419-421`
- **Evidence:** Lore:47 'never in the cash reserve, and never tells anyone to spend it on NOCK.' CHANGELOG.md:95 'keep an ETH gas buffer'. main.rs:6098 computes funding as wallet ETH+WETH minus REBUILD_GAS_BUFFER_ETH (0.002).
- **Impact:** The documented gas rule and the engine disagree. LP rebuild suggestions size WETH/NOCK deposits from wallet ETH, and a range converts ETH into NOCK as the price falls. The design note still lists Q6 ('count ETH / funding source') as open.
- **Proposed fix:** Get the operator's decision (see open questions), then align code and docs. Either drop wallet ETH from rebuild funding (WETH/NOCK suggestions only by explicit opt-in), or state in the lore that moving gas into a WETH/NOCK range is allowed and counts in the gas pillar, just as LP USDC counts as cash. Mark design-note Q6 answered in §8.4.

### DOC-07: README, the public front page, shows a retired portfolio and presents the operator's numbers as 'the strategy' — skeptic: **confirmed**

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `README.md:3`, `README.md:9`, `README.md:17-35`, `README.md:39-58`, `README.md:62-72`, `README.md:104`, `README.md:121`, `README.md:128`, `README.md:174`, `README.md:193`, `README.md:242-245`
- **Evidence:** README.md:51 'USDC/NOCK — Uniswap v4, 0.75% fee (NFT #3163431; NOCK/USD price source)'. README.md:35 'Hard safety floor — never liquidated, staked, or impaired.' README.md:27 'Tax Escrow (Cold reserve…)'.
- **Impact:** It lists retired #3163431 and #3163432 (the price source is now #3163450) and calls Aero retired, though it has been tracked again since 0.5.0. It shows 80/15/5 targets, but the live config is 75/20/5, and these are the operator's own numbers anyway. It contradicts the LP floor tolerance (OPERATOR_GUIDANCE:120), the staked Aero LP, and 'no tax cold address' (OPERATOR_GUIDANCE:94). The advice table predates DCA: range exit at $2.028, and no rows for DCA, rebuild or sweep. Quick Start is stale too: placeholder clone URL, no config step, Node 20+ instead of 22+, '12/12' tests (45 now), and only 3 of the 4 MCP tools. There is no 'not a recommendation / vibe-coded' note, although the GitHub description says 'Vibe code (docs may contain hallucinations)'.
- **Proposed fix:** Shorten §1-§2. Link the lore doc first and say that positions, floor and targets come from your own config.yaml, and the operator's values are not recommendations. List the current precedence: DEFENSIVE LOCK → SWEEP TO STABLE → DCA directives → no-plan dip/tax guardrail → RANGE EXIT at the top of the price-source range → BATCH HARVEST at ≥ $5 → HOLD, with the REBUILD LP bias. Fix Quick Start: urmanac/nocknock URL, a config step, Node 22+, test counts per AGENTS §9, and add get_accumulation_plan. Add a one-line disclaimer matching the repo description.

### DOC-08: ONCHAIN_INTEGRATION.md states three things that became false in v0.5.0 — skeptic: **confirmed**

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `ONCHAIN_INTEGRATION.md:29-31`, `ONCHAIN_INTEGRATION.md:114`, `ONCHAIN_INTEGRATION.md:172-174`, `ONCHAIN_INTEGRATION.md:14`
- **Evidence:** :30 'the worker no longer reads or claims anything on Aerodrome, and no Aerodrome transactions are built.' :114 'B stays the single NOCK/USD source.' :173 'NockNock holds no Aero LP'.
- **Impact:** main.rs:4819-4821 reads aerodrome positions, and main.rs:4260 and 4332-4342 build claimFees() actions into the claim payload. The price source is position D #3163450 (config.yaml.example:40-52 and the live snapshot). The operator holds staked Aero LP, which §7 of the same doc records. Readers who follow the lore's links hit these contradictions.
- **Proposed fix:** Add dated 'Superseded in v0.5.0' notes at :29-31, at :114 (the price source moved to D; B is retired), and at :172-174 (Aero LP tracked since 0.5.0, see §7). Label the legacy vAMM row at :14 as 'not a position; still the OHLCV history source (market_data.pool_address)'.

### DOC-09: Today's decisions are missing from the canonical standing orders, and older wording keeps the 'dual growth' frame — skeptic: **confirmed**

- **Kind / severity / patch:** gap / medium / yes
- **Locations:** `OPERATOR_GUIDANCE.md:90-126`, `OPERATOR_GUIDANCE.md:113-114`, `OPERATOR_GUIDANCE.md:119`, `OPERATOR_GUIDANCE.md:104`, `README.md:3`, `HANDOFF.md:3`, `CHANGELOG.md:21`, `docs/kb/design-note-dca-accumulation-plan.md:419-421`, `ingestion-worker/src/main.rs:6178`
- **Evidence:** OPERATOR_GUIDANCE:119 'AERO rewards are unplanned income... do not assume the operator wants to accumulate AERO.' CHANGELOG.md:21 'Count ETH in portfolio totals'. README.md:3 'dual-token growth (NOCK and USDC)'.
- **Impact:** Agents must obey OPERATOR_GUIDANCE §7 (line 5), but it has no entry for any of today's decisions: four pillars; ETH as gas (counted, never cash or reserve, not deployable); a disposable database with the wallet as the record; infrastructure, not production; and 'never choose a newcomer's numbers'. The lore doc would be the only home for these, while README, HANDOFF and the HOLD label 'Dual Growth Engine Active' keep the two-pillar frame.
- **Proposed fix:** Add OPERATOR_GUIDANCE §7.11 'Four pillars' (2026-11-14, with the quote) and §7.12 'Gas is not cash': the reserve counts wallet and LP USDC, never ETH (also amend :114). Add §7.13 'The wallet is the record' (disposable database, fresh-start steps, no backups) and §7.14 'Building in public' (from-zero rules). Reword :119 in pillar terms (neither minimized nor maximized; policy pending #93). Change CHANGELOG [Unreleased]:21 to 'Count ETH as the gas pillar (#94)'. Mark design-note Q6 answered. Rename the HOLD label as part of the #95 fix.

### DOC-10: Deviation-matrix targets currently read as 'buy ~$750 of NOCK'; the lore's 'no hurry' must explain this, or the operator must revisit the targets — skeptic: **confirmed**

- **Kind / severity / patch:** design / medium / no
- **Locations:** `docs/lore/2026-11-14-four-pillars-no-hurry.md:17-36`, `docs/kb/session-2026-11-13-chudmax-dca-aerodrome.md:57-58`, `docs/kb/session-2026-11-13-chudmax-dca-aerodrome.md:139-140`, `ingestion-worker/src/main.rs:6283-6314`, `README.md:45-56`
- **Evidence:** Live matrix: Sovereign Treasury 4345 vs 7500 bps 'under_funded'; Liquid USDC 3893 vs 500 bps 'over_funded'. kb:140 'decide target weights that reflect the cash-heavy posture.'
- **Impact:** The dashboard groups holdings by where they sit (Treasury, LP, Liquid USDC) and compares them with the operator's own targets. That signals a large NOCK purchase, the opposite of the lore's position, and it has no line for ETH or AERO. In a public repo, readers will take an 'under_funded' label as advice.
- **Proposed fix:** In the lore, explain pillars (what you own) versus matrix buckets (where it sits). Say the labels measure against the operator's own targets and are not instructions, and that 'cash over-funded is welcome' (kb:57-58). The operator decides whether to revise the targets; NockNock does not. Code follow-up (#94/#93): show ETH and AERO lines without targets.

### DOC-11: Lore draft wording errors: the plan 'buys', the first fill is double-counted, 'read from the chain' is overstated, config is misplaced — skeptic: **confirmed**

- **Kind / severity / patch:** defect / medium / yes
- **Locations:** `docs/lore/2026-11-14-four-pillars-no-hurry.md:105`, `docs/lore/2026-11-14-four-pillars-no-hurry.md:87`, `docs/lore/2026-11-14-four-pillars-no-hurry.md:119-120`, `docs/lore/2026-11-14-four-pillars-no-hurry.md:132-134`, `docs/lore/2026-11-14-four-pillars-no-hurry.md:29`
- **Evidence:** :105 'The accumulation plan buys NOCK in small slices on a timer'. :119 'Everything NockNock shows is read from the chain'. :134 'after a wipe you would list them again by hand.'
- **Impact:** (1) Contradicts config.yaml.example:125 ('The plan only PACES: you make each swap yourself') and the rule that NockNock never signs; a newcomer may think it trades. (2) Row :87 counts $100 → 8,700 NOCK, but $20 is already spent and inside the 90,089 balance (live: spent 20.00, remaining 80.00); the right figures are ~$80 → ~7,000 NOCK → average ~$0.0174. (3) Price history and RSI come from GeckoTerminal/DexScreener backfill (live data_provenance: 190 geckoterminal points), and the high-water mark and charts exist only in the database. 'Not a vault' also collides with the legacy 'The Vault' (README:28; bucket 'Liquid USDC (Escrow/Vault)'). (4) A database wipe does not touch config.yaml, which lives on the workstation and as a Podman secret (AGENTS.md:214-217). (5) 'The line we defend' (:29) omits the deliberate LP overlap bounded by lp_floor_tolerance_bps (OPERATOR_GUIDANCE:120).
- **Proposed fix:** (1) Say 'paces NOCK buys that we make ourselves on Aero'. (2) Recompute the row from the remaining $80. (3) Say 'balances, positions, fees and recorded buys are read from the chain; price history is backfilled from public OHLCV feeds; the high-water mark and charts live only in the database', and use 'not the record' instead of 'vault'. (4) Say 'if config.yaml itself is lost, you list positions again by hand'. (5) Add one sentence on the tolerated overlap.

### DOC-12: HANDOFF.md is a v0.2.1 dossier, but AGENTS.md §10 points agents to it for the strategic vision — skeptic: **confirmed**

- **Kind / severity / patch:** defect / low / yes
- **Locations:** `HANDOFF.md:3`, `HANDOFF.md:7-8`, `HANDOFF.md:31`, `HANDOFF.md:112-113`, `AGENTS.md:161-164`
- **Evidence:** HANDOFF.md:7 'Current Active Version: `v0.2.1` (Shipped)'; :8 'zero open bugs'. AGENTS.md:161 'Consult HANDOFF.md for the strategic vision'.
- **Impact:** Agents looking for the vision land in October's world: dual growth, four Uniswap positions, 10/16 tests. The new lore doc would compete with it.
- **Proposed fix:** Add a banner at the top of HANDOFF: 'Historical, 2026-10-12 (v0.2.1). Current philosophy: docs/lore/; standing orders: OPERATOR_GUIDANCE §7.' Point AGENTS.md §10 to the lore doc for philosophy, and keep HANDOFF only for the Urbit vision.

### DOC-13: Minor: 'mock data in production' phrasing, and two different LP floor tolerances recorded for the operator — skeptic: **confirmed**

- **Kind / severity / patch:** defect / low / yes
- **Locations:** `OPERATOR_GUIDANCE.md:61`, `RELEASE.md:103`, `TRIAGE.md:3`, `docs/kb/design-note-dca-accumulation-plan.md:383`, `docs/kb/session-2026-11-13-chudmax-dca-aerodrome.md:86`
- **Evidence:** OPERATOR_GUIDANCE:61 'The "Mock Data in Production is a Bug" Invariant'. The design note at :383 says '(operator's config: 500, i.e. up to 5,000 NOCK)', while kb:86 says 'floor tolerance 10%'.
- **Impact:** Now that chudmax is declared 'not production', the qualifier invites reading mock data as acceptable there; AGENTS.md:173 already says simply 'Mock Data is a Bug'. One of the two tolerance figures is stale, so a lore doc quoting either may be wrong.
- **Proposed fix:** Drop 'in Production' / 'in the production app' from all three headings. Check lp_floor_tolerance_bps in the operator's config.yaml and annotate whichever doc is stale; the lore doc should not quote the tolerance number.

### Proposed design

FILENAME
Keep docs/lore/2026-11-14-four-pillars-no-hurry.md. It is the draft already on fix/v0.5.1, and AGENTS.md:205 in the working tree links to it. Add docs/lore/README.md as a short index of dated lore entries plus the living glossary below, so dated entries never redefine terms. Link both from the first screen of README.md and from OPERATOR_GUIDANCE §7. New §7.11-7.14 there hold the enforceable rules; the lore doc holds the reasons.

OUTLINE (dated entry). Keep the draft's voice and quotes.
0. Header. Date. 'One person's position about one wallet, not a recommendation.' 'Vibe-coded; docs may contain hallucinations' (the same words as the GitHub repo description). Every number is dated and sourced, per the kb lesson: re-read the snapshot before quoting.
1. The short version (keep).
2. Four pillars, not a pyramid (keep the quote).
   - Per pillar: what it is for; how it is counted today versus from v0.5.1 (#94 ETH, #93 AERO); neither minimized nor maximized; no rebalancing pressure. LP is where the pillars work together, not a fifth pillar.
   - Add: how the dashboard's three buckets relate to the pillars. Sovereign Treasury is wallet NOCK, LP is all tracked positions, Liquid USDC is wallet USDC. The under/over-funded labels measure against the operator's own targets and are not instructions; 'cash over-funded is welcome'.
3. Cash and gas are not road-trip money (keep the quote). Precise rules:
   - The reserve is wallet USDC plus LP USDC, never ETH.
   - Today it binds only while an accumulation plan is configured.
   - Whether LP suggestions may use wallet ETH follows the operator's decision.
4. Where we stand (keep: dated panel figures with caveats).
   - NockNock's high-water mark is the highest total it has recorded, not a cost basis.
   - The plan's average cost covers plan fills only.
5. Why not buy more now (keep). Fix the plan row to the remaining $80. Add that NockNock will never size a catch-up purchase.
6. What the DCA timer is for:
   - It paces buys we make on Aero; it never buys.
   - DCA here is about timing, not cost (design note §2).
   - Missed windows never stack, and a pause defers rather than consumes a tranche.
   - The lump-sum counterfactual can show DCA losing, and that is the point of measuring it.
7. The wallet is the record.
   - What comes from the chain, what comes from public OHLCV feeds, and what exists only in the database.
   - Fresh start: latest release + empty database + config.yaml (your key and numbers) + the wallet.
   - Set starts_at and re-record plan fills by transaction hash, or lower the budget if you would rather restart. Harvests re-record the same way.
   - Charts and the high-water mark restart; price history backfills.
   - Taxes come from the wallet via Koinly. NockNock's ledger is a second record, not a compliance system (kb:72-73).
   - Nothing is called a backup, once the units are renamed or removed.
8. Infrastructure, not production (keep).
9. Building in public: the newcomer. This fills the FROM-ZERO placeholder; content below.
10. What this is, honestly (keep). Add: it builds unsigned claim payloads but never signs. Do not repeat README:47's governance claim unless it is verified (OPERATOR_GUIDANCE §7.10).
11. Words we use (link docs/lore/README.md).
12. Sources: OPERATOR_GUIDANCE §7; design note §2, §4.3, §8; kb session log; ONCHAIN_INTEGRATION §6-7; issues #93, #94, #95 and #96 (closed as not planned); CHANGELOG 0.5.0.

FROM-ZERO SECTION (a newcomer with an empty wallet asking for shot advice)
(a) What NockNock will and won't do. It reads, paces and explains. It never signs or holds keys, and it never chooses your floor, targets, tax ratio, reserve, budget, tranche count or interval.
(b) Minimal config: your tracked_wallets, an alchemy_api_keys entry, and a price source. Until v0.5.1 the price source is an aerodrome entry for a public NOCK/USDC pool you hold no LP in, with is_price_source: true. Also give explicit values, or zeros, for every operator_profile key, because config.yaml.example is a base layer holding one operator's numbers.
(c) Day one: an empty portfolio. No DEFENSIVE LOCK unless you set a floor, no lump-sum shot, and a prompt to decide a plan.
(d) Building from zero. The order and the numbers are both yours:
   - Gas: a little ETH on Base. NockNock shows the live cost of a transaction.
   - Cash: USDC.
   - A plan: budget, number of buys, interval.
   - Record each buy by transaction hash.
   - Optionally, a floor once you hold NOCK.
   - Optionally, LP later (targets and positions).
   - AERO only if you stake on Aero.
(e) Taxes: use a wallet-sync tool.

GLOSSARY (docs/lore/README.md), drawn from what the docs already say
- **Pillar**: NOCK, cash (USDC), gas (ETH) and AERO (2026-11-14). LP is not a pillar.
- **Tranche** has three senses; always qualify it.
  1. The original tiers: T1 Uniswap v4 LP, T2 Aerodrome LP, T3 Sovereign Treasury (README:17-35, ONCHAIN §4). Retired.
  2. LP tranche: a tracked LP position, as in 'REBUILD LP TRANCHES' (OPERATOR_GUIDANCE §7.5 and §7.8).
  3. DCA tranche: one paced buy in an accumulation plan (config.yaml.example:124).
- **Floor**: treasury_floor_nock (the operator's is 100,000 NOCK).
  - It counts wallet NOCK, LP NOCK and uncollected fee NOCK (main.rs:6035). Below it the directive is DEFENSIVE LOCK.
  - LP may deliberately overlap it, bounded by lp_floor_tolerance_bps (OPERATOR_GUIDANCE §7.8).
  - Do not confuse it with the cash reserve (accumulation_plan.cash_reserve_usdc; the operator's is $950, wallet plus LP USDC, held at any price) or with the cash target (target_weights_bps.cash, a long-run weight, not permission to spend; OPERATOR_GUIDANCE:113).
- **Treasury**: 'Sovereign Treasury' is the wallet-NOCK bucket (T3 and the deviation matrix). OPERATOR_GUIDANCE:104 also uses 'the treasury' for all holdings.
- **Sovereign**: the operator's single self-custodied wallet (OPERATOR_GUIDANCE §7.1) and their sole authority to sign. Also appears in 'sovereign state room', the 'Sovereign NOCK Stack' card, and the Urbit 'sovereign ships' (HANDOFF §6, vision only).
- **Shot / shot advice**: the engine's single current directive, an action plus details (README §2; MCP get_shot_advice). The current set:
  - DEFENSIVE LOCK
  - SWEEP TO STABLE (overextension, or the tax guardrail)
  - DCA TRANCHE DUE / DCA ON SCHEDULE
  - DIP REINVESTMENT SHOT (the no-plan legacy arm)
  - RANGE EXIT FLASHPOINT
  - BATCH HARVEST SHOT
  - HOLD & ACCRETE
  - plus the REBUILD LP TRANCHES bias, which leads when the engine is otherwise idle and never displaces a due tranche or a safety directive.
- **Harvest**: claiming accrued LP fees (BATCH HARVEST SHOT at ≥ $5.00 claimable), counted only from a verified receipt (record-harvest). None are recorded yet.
- **60/40 Split Rule**: a virtual partition inside the single wallet. 40% of realized harvests (and measurable gains) is earmarked as tax reserve and 60% is growth dry powder; there is no tax cold address (OPERATOR_GUIDANCE §7.1-7.2; tax_reserve_ratio 0.40). 'Tax Escrow' and 'The Vault' are legacy names for the two partitions.
- **High wall**: single-sided NOCK liquidity far above spot, built from principal when price exits the top of the price-source range (RANGE EXIT FLASHPOINT). The '$5.00+' figure is a legacy hardcoded constant (design note §8.6).
- **Dry powder**: liquid USDC held for dips.
- **Raft / wide raft**: the wide-range v4 USDC/NOCK 3% position #3163450, now the price source.
- **State room, cockpit, Lite Cockpit, Golden Path**: the dashboard; its advice banner; the /?view=lite phone view; and the v0.2.0 flow in which an unsigned payload goes over WalletConnect, the Uniswap app signs, and a receipt-verified ledger records it.
- **Cash-neutral ('No Net Cash Change')**: moving USDC into LP keeps it in the stable balance (OPERATOR_GUIDANCE §7.5).
- **Deviation matrix**: actual versus operator-set target weight per bucket. Its labels are measurements.
- **High-water mark**: the highest total that this database has recorded. It resets on a fresh start and is not a cost basis.
- **Degraded**: the /health status when the last cycle failed or the data is stale.
- **Observe and Protect**: the motto. The state room is calm and asynchronous, with zero urgency (README:3, :113).

PHILOSOPHY ALREADY IN THE DOCS (the lore doc must stay consistent with these)
- Zero custody: 'the schedule is a prompt, not a bot' (design note §3).
- Mock data is a bug.
- The operator's numbers are the operator's. Agents may improve the soundness of the strategy, not the allocation (OPERATOR_GUIDANCE §7.9).
- Protect cash: 'There might be a dip!' The reserve is held at any price and only the operator lowers it.
- The LP rebuild bias is cash-neutral.
- 'We need to trade more': the LP overlap with the floor is deliberate.
- Do not fill out-of-range positions. Link the Aero app, never aerodrome.finance.
- A full-range Aero position is the long-term core (kb:63-65).
- DCA is about timing, not cost. Missed windows never stack.
- The honest scoreboard may show DCA losing.
- No urgency to release (OPERATOR_GUIDANCE:56).
- No hallucinated facts (OPERATOR_GUIDANCE §7.10).

WHAT A FRESH-START USER NEEDS THAT NO DOC EXPLAINS TODAY
- Which example-config keys to override, and that those values are one operator's.
- How to price NOCK without any LP.
- What the engine will say for an empty, cash-only or gas-only wallet.
- That the reserve only exists inside a plan.
- How to rebuild plan and harvest records from transaction hashes (starts_at).
- That the high-water mark, charts and notifications reset.
- That a WalletConnect project id is needed for pairing (it appears only in OPERATOR_GUIDANCE §7.3).
- That the database holds nothing the wallet cannot reproduce, except the operator's own numbers, which live in config.yaml.

### Tests to add

- Config layering, newcomer: a config.yaml with only tracked_wallets and alchemy_api_keys. Assert exactly which operator values load from config.yaml.example today (positions, treasury_floor_nock 100000, targets, tax_reserve_ratio). After the from-zero fix, assert none are inherited, or that a 'numbers not set' status is surfaced.
- Position ownership: a v3 or v4 position whose NFT owner is not the tracked wallet is rejected or flagged, never counted in totals (today main.rs:5424 and 5758 only label it).
- Empty wallet with a price-source-only aerodrome entry holding zero LP: the snapshot builds; totals are 0 with no NaN; deviation statuses are not 'under_funded' (the NaN-to-0 cast makes them so today); advice is not HOLD's 'generating yield' text.
- Floor unset or 0 with zero NOCK: no DEFENSIVE LOCK, and no advice text mentions 100,000 or 120,000.
- Cash-only wallet, no plan, spot below $0.020: advice must not contain 'Deploy $X of saved USDC' and must point to configuring an accumulation plan (fails today at main.rs:6145-6151).
- Cash-only wallet with a plan whose cash_reserve_usdc is at least the balance: the plan reports blocked and names the cash reserve as the binding constraint.
- Gas-only wallet (after #94): ETH counts in total_portfolio_usd but never in total_stable_usd, the reserve or the deployable amount. LP rebuild funding follows the operator's ETH decision (assert wallet ETH is excluded if that option is chosen).
- Fresh-start recovery: empty database plus a plan with starts_at at or before the first fill. Re-recording the fill hashes reproduces fills, spent_usdc, avg_price_usd and next_due_at. Without starts_at, the request returns 422 BEFORE_PLAN_START with the starts_at hint.
- HOLD text with zero positions or only aerodrome positions contains neither 'Uniswap position(s)' nor '0% realized loss' (#95).
- Frontend: an empty-portfolio snapshot renders without NaN% and shows 'no target set' when targets are unset.
- MCP: get_shot_advice and get_portfolio_state on an empty-wallet snapshot return the from-zero guidance, not HOLD or DEFENSIVE LOCK boilerplate.
- Optional docs check in CI: once the operator chooses to rename or remove the units, fail if README, DEPLOYMENT, OPERATOR_GUIDANCE or deploy/podman use 'backup' wording.

### Open questions

- Should nocknock-backup.* and the Makefile db-backup/db-restore targets be removed, or kept and renamed as a 'nightly dump' convenience (which is what the AGENTS.md working-tree edit keeps)? Issue #96 is still OPEN; close it as not planned before AGENTS.md:221 is committed?
- After a database wipe, should the plan's progress be restored by re-recording its fills (set starts_at), or deliberately restarted (lower budget_usdc or start a new plan id)?
- ETH: may LP rebuild suggestions use wallet ETH above the 0.002 ETH buffer for WETH/NOCK positions? Does ETH inside LP count toward the gas pillar, the way LP USDC counts toward cash? Is there a gas floor the operator wants to set themselves?
- AERO (#93): should claimed AERO be held as its own pillar (which 'neither minimize nor maximize' suggests), or converted? Is it OK to rewrite OPERATOR_GUIDANCE:119 in pillar terms?
- Deviation targets (the live config's 75/20/5): keep them and treat the labels as measurements, or revise them for the cash-heavy, no-hurry posture? Should ETH and AERO get dashboard lines without targets?
- Newcomer defaults: ship a neutral config.yaml.example (no positions, floor or targets), or keep the operator's values clearly labelled as an example? Retire the no-plan lump-sum dip arm outright?
- README.md:47 says NOCK reserves protect 'sovereign voting weight and governance posture'. Is that a verifiable property of NOCK? If not, drop it rather than carry it into the lore (OPERATOR_GUIDANCE §7.10).
- Which lp_floor_tolerance_bps is current? The design note says 500; the kb session log says 10%.
- The lore draft publishes the Uniswap app's realized and unrealized loss figures and describes chudmax as a home-LAN machine in a public repo. Confirm the operator is comfortable with both.

### Skeptic: problems with the proposed design

- DOC-01, DOC-07 and FROM-ZERO (b): the operator's numbers also live in code. main.rs:50-64 sets the serde defaults (floor 100000, ratio 0.40, targets 8000/1500/500), main.rs:252-254 defaults tracked_wallets to 0xeE114c…, and mcp.rs:19 hardcodes the same wallet. A neutral config.yaml.example would still give any omitted key the operator's values, so the README promise that 'positions, floor and targets come from your own config.yaml' stays false. The change also collides with test_example_config_is_the_base_layer (main.rs:7122-7148, the #86 fix), which asserts that positions and the 100000 floor are inherited. Changing OperatorProfile::default() moves 12 test call sites (for example 6716 'floor 100k; ratio 0.40' and the shared helper metrics_with_plan at 7009-7013). Those tests need an explicit operator fixture, not dropped assertions (AGENTS §9: counts only go up).
- 'Verify NFT ownership … reject foreign positions': a plain ownerOf == wallet check would reject staked NFTs, because their owner is the gauge. The operator's Aero CL200 NFT #2914490 sits in gauge 0x3bf70c88… (ONCHAIN_INTEGRATION.md:290-295), and it is exactly the position #91 will track. The check must accept gauge custody, and it should flag the position or fail explicitly rather than silently drop it.
- Documenting the price-source-only aerodrome entry (DOC-02, FROM-ZERO b) needs guards. The entry is always in range and sits in a live pool, so lp_rebuild_suggestions treats it as eligible (main.rs:803-822). A newcomer whose floor is met would get deposit suggestions sized from the code-default 15% LP target, a number nobody chose. The entry also counts as a 'Uniswap position' in the HOLD text (6180) and adds a claimFees action (4331-4342). The doc must add rebuild_weight_bps: 0 (or uniswap_lp: 0). It should also point at the deep legacy vAMM 0x85f1… (about $735k TVL, ONCHAIN:252) rather than the thin 0x770f… pool (about $1.8k, :255).
- 'Give explicit values, or zeros, for every operator_profile key' backfires for targets. calc_status uses a tolerance of max(target × 20%, 100 bps) (main.rs:6272-6280), so zero targets mark every held bucket over_funded, and App.tsx:656 paints that red. The from-zero path needs a real 'no target set' state (an Option), not zeros.
- 'Drop the 120,000 constant' and retiring the no-plan arm conflict with existing orders and tests. OPERATOR_GUIDANCE §5 says to externalize magic numbers rather than delete them, and kb:139-140 and CHANGELOG:21 record 'externalise the legacy advice constants'. The $0.020 line also gates the tax-guardrail arm (6137). Tests at main.rs:6733-6745 and 7016-7024 assert that the legacy DIP arm fires, so they must be rewritten deliberately. Lifting cash_reserve_usdc into operator_profile only helps once it has a value, and NockNock may not choose that value.
- DOC-04 options (a) and (b) are missing a step on the host. deploy.sh only copies and enables units (:46-51) and runs rm -rf ~/nocknock/src on every deploy (:24). Deleting or renaming nocknock-backup.* in the repo therefore leaves the timer already enabled on chudmax pointing at an ExecStart script that no longer exists (service:8, %h/nocknock/src/…/nocknock-backup.sh). It would fail every night, or run beside a renamed twin. The fix needs `systemctl --user disable --now nocknock-backup.timer` plus removal of the unit files (user units, so no sudo). 'Close #96 first' is already done.
- Precedence and glossary: any active plan (any status except complete) returns at main.rs:6135, before the RANGE EXIT, BATCH HARVEST and HOLD arms. No harvest or range-exit directive can surface while a plan runs. The proposed README precedence and the glossary line 'BATCH HARVEST SHOT at ≥ $5.00' must say so, or this should be triaged as a defect. Also, REBUILD LP is promoted only over HOLD and DCA ON SCHEDULE, PENDING, PAUSED or BLOCKED (1393-1397), never over TRANCHE DUE or the safety arms.
- Outline §5 ('NockNock will never size a catch-up purchase') and FROM-ZERO (c) ('No DEFENSIVE LOCK unless you set a floor, no lump-sum shot') describe behaviour that does not exist yet (main.rs:50, 6118-6127, 6145-6151). Today the default floor even blocks a newcomer's DCA advice. The lore must merge together with or after those fixes, or describe them as planned. Otherwise the lore itself becomes a static claim presented as fact, which the mock-data invariant forbids.
- Glossary and sources errors:
- Tranche sense 1 is called retired, but the Aerodrome full-range tranche was restored in v0.5.0 (ONCHAIN_INTEGRATION.md:235; CHANGELOG.md:25).
- The vocabulary's origin, assets/nock_portfolio_strategy.pdf, is not cited.
- 'The WalletConnect id appears only in OPERATOR_GUIDANCE §7.3' is false: it also appears in HANDOFF.md:185, frontend/.env.example:2, deploy.sh:21 and LiteCockpit.tsx:101.
- The sources list omits #91 (the untracked CL position) and #98 (cost-basis charting).

### Skeptic: missed in this area

- **The MCP stdio zero-DB fallback reports the operator's wallet and a frozen price from the retired v4 0.75% pool** (`ingestion-worker/src/bin/mcp.rs:19`, `ingestion-worker/src/bin/mcp.rs:183-211`, `ingestion-worker/src/bin/mcp.rs:333-345`, `ingestion-worker/src/bin/mcp.rs:405-427`, `ingestion-worker/config.yaml.example:35-36`, `docs/kb/design-note-dca-accumulation-plan.md:389-392`, `README.md:209`): If the worker at NOCKNOCK_API_URL (default 127.0.0.1:8080) does not answer within 2 s (mcp.rs:183, 194), handle_direct_rpc_fallback takes over:
- It reads NOCK balanceOf for the hardcoded WALLET_ADDR 0xeE114c… (mcp.rs:19).
- It takes spot from slot0 of pool 0x91a9ef0a…f99c (mcp.rs:337). That is the retired #3163431 pool, which config.yaml.example:35-36 says has had 'no in-range liquidity since the 2026-11 liquidation'. The design note records its price frozen at $0.0169 while the market was $0.0123.
- It prints both as 'NOCK Spot Price' and 'Wallet NOCK Balance' (mcp.rs:408-409).

README:209 presents this as a deployment tier. A newcomer, or the operator while the worker is down, therefore gets the operator's holdings and a stale price. The worker's stale-price guard (main.rs:4845-4849) does not apply here. This is a mock-data and from-zero defect, in patch scope.
- **The lore draft overclaims what NockNock counts: an untracked CL position, ETH counted 'now', and 'no cost basis at all'** (`docs/lore/2026-11-14-four-pillars-no-hurry.md:29`, `docs/lore/2026-11-14-four-pillars-no-hurry.md:46`, `docs/lore/2026-11-14-four-pillars-no-hurry.md:67-68`, `docs/lore/2026-11-14-four-pillars-no-hurry.md:114-115`, `ONCHAIN_INTEGRATION.md:288-295`, `docs/kb/session-2026-11-13-chudmax-dca-aerodrome.md:83-84`, `ingestion-worker/src/main.rs:6029`): - lore:29 says NOCK is 'Counted in full'. But the Aero CL200 WETH/NOCK NFT #2914490 (2,755 NOCK + 0.082 ETH, about $180) is 'not tracked' (ONCHAIN:295), and kb:84 says the 'true total is ~$180 higher'. #91 is milestoned for v0.6.0.
- lore:46 states in the present tense that 'NockNock counts ETH in the portfolio total'. main.rs:6029 sums only lp_usd + t3_usd + wallet_usdc, and #94 is open. The line is true only if the lore ships with the #94 fix.
- lore:68 says NockNock 'does not compute cost basis at all', yet the live advice prints the plan's 'average cost $0.012516'.
- lore:114-115 ('NockNock can't do it yet') should link #98, which was opened at 18:13Z.
- **DEPLOYMENT.md's documented config path crashes the worker, and compose and cloud images bake config.yaml in** (`DEPLOYMENT.md:1`, `DEPLOYMENT.md:46-54`, `DEPLOYMENT.md:60-138`, `ingestion-worker/src/main.rs:1627-1628`, `ingestion-worker/Dockerfile:25-26`, `deploy/podman/deploy.sh:6-7`, `README.md:124-140`): - DEPLOYMENT.md:51-52 make APP_ALCHEMY_API_KEYS ('Comma-separated') and APP_TRACKED_WALLETS 'Required', and :52 uses the operator's wallet as the example value.
- load_app_config adds the Environment source without try_parsing or list_separator (main.rs:1627-1628). config 0.14.1 therefore yields a String, and seq deserialization goes through deserialize_any → visit_string, which the Vec<String> fields reject. The worker fails at startup.
- Options A–C and the README Quick Start never mention config.yaml.
- Dockerfile:26 runs `COPY config.yaml* /app/` with no .dockerignore. Any local config.yaml, including the Alchemy key, gets baked into compose, Fly and Railway images. That is the opposite of what deploy.sh:6-7 promises.
- The title 'Cloud Deployment Guide' and README:193 'production servers' also clash with 'infrastructure, not production'.

This is the path a fresh-start user would follow.
- **The lore's origin document is a 'CONFIDENTIAL' PDF in the public repo, and it tells the reader to lower cost basis** (`assets/nock_portfolio_strategy.pdf`, `SESSION_LOG.md:173`, `docs/lore/2026-11-14-four-pillars-no-hurry.md:112`): git ls-files shows assets/nock_portfolio_strategy.pdf tracked in a PUBLIC repo. Every page carries the footer 'CONFIDENTIAL - STRATEGIC ASSET ALLOCATION', and SESSION_LOG.md:173 calls it the 'original confidential PDF'. It is where Tranche 1-3, Sovereign Protected Treasury, the 60/40 Escrow/Vault split, the $2.02 transition and '$5.00+' high walls come from. Its Market Retraction row says to 'Re-inject capital back into Aerodrome to organically lower cost basis'. That is the opposite of lore:112, which says the timer is 'not there to repair the cost basis'. The lore and glossary should cite the PDF as the historical origin they supersede. The operator should decide whether a file marked confidential stays public.
- **high_water_mark_date is always the snapshot's own time, not when the peak was set** (`ingestion-worker/src/main.rs:6197`, `specs/core/snapshot.yaml:631-634`): main.rs:6197 sets high_water_mark_date: now_str.clone() on every cycle, while the spec describes the field as 'When the high-water mark was set'. Live, high_water_mark_usd is 2475.27, which is not the current 2390.52 total, yet it is dated 2026-11-14T18:26:04Z, the snapshot's own generated_at. That is a fabricated value served by the API: a mock-data defect in patch scope. It also undercuts the lore outline, which uses the high-water mark as a reference point.


## Cross-area critique

*The critic was still running when this appendix was generated.*

