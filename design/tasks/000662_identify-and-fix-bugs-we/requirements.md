# Requirements: Fix Point-Release Bugs: Snapshot Dedup, History Integrity, Zero-Mock

## Background

`v0.1.3` shipped green and `HANDOFF.md` claims "zero open bugs", but a code audit of
`nocknock` @ `f621e6f` found **15 real defects** (3 high-severity data-integrity bugs in the
worker, several zero-mock invariant violations, and frontend resilience bugs). Per
`TRIAGE.md`, features wait for `v0.2.0`; bugs ship as `v0.1.x` patches. This spec plans the
next point-release bug batch (target `v0.1.4`, possibly spilling into `v0.1.5`).

## Bug Registry

All locations verified against `main` @ `f621e6f`. Labels follow the `TRIAGE.md` taxonomy
(`bug`, `data-integrity`, `frontend`/`backend`, `patch-0.1.x`). Each must be filed as a
GitHub issue before branching (pre-branching triage rule).

### Backend — `ingestion-worker/src/main.rs`

| ID | Sev | Location | Defect |
|----|-----|----------|--------|
| BUG-B1 | P0 | `main.rs:1960-1965` (+ volatile inputs at 1550, 1830, 1839, 1847, 1659, 1705) | `compute_state_hash` hashes wall-clock timestamps (`wallet_balances[].updated_at`, `high_water_mark_date`, series `time_points`, fabricated channel/counterfactual timestamps). Hash changes every cycle → `store_snapshot` dedup (1975-1983) never fires → new DB row every 5 min even in steady state. Defeats the issue #10 fix; unbounded table growth; history charts fill with duplicate flat points. |
| BUG-B2 | P0 | `main.rs:1071-1076` | Counterfactual history query uses `ORDER BY generated_at ASC LIMIT 100` — selects the **oldest** 100 rows. As the table grows, the Strategy Counterfactual chart stops reflecting recent history. |
| BUG-B3 | P1 | `main.rs:1084-1091` | `counterfactual_portfolio` and `counterfactual_native` are built with independent `filter_map` pushes — a row missing one field misaligns the two series (index i no longer corresponds). |
| BUG-B4 | P0 | `main.rs:1244, 1270, 1742` | Zero-mock violations: if the ETH-price RPC fails, `live_eth_price_usd` silently falls back to `2500.0`; gas RPC failure falls back to `0.005`; `compute_derived_metrics` falls back `gas_cost` to `0.0043`. Synthetic values flow into `gas_break_even_usd`, efficiency ratios, and advice text. Same class as closed issues #11/#16. |
| BUG-B5 | P1 | `main.rs:1945, 1953` | `recommended_action` hardcoded `"claim_now"` for both fee contracts. Dashboard shows "CLAIM NOW" even at $0 claimable (frontend renders it verbatim at `App.tsx:510-512`). |
| BUG-B6 | P1 | `main.rs:1659, 1705` | Structural-channel and counterfactual points get **fabricated** timestamps (`now − (len−i)×5min`) instead of the actual snapshot `generated_at`. Any ingestion gap/restart distorts both chart timelines. |
| BUG-B7 | P2 | `main.rs:1650` | Bollinger window loop `for i in window..len` excludes the newest price from the final SMA window (off-by-one; should be `window..=len`). |
| BUG-B8 | P2 | `main.rs:1596-1601, 1854-1868` | V4 bracket prices (`"0.01237"`, `"2.02783"`), `in_range`, and `distance_bps: -200` are hardcoded strings/constants duplicated in two places instead of derived from the ticks. |

### Frontend — `frontend/src/`

| ID | Sev | Location | Defect |
|----|-----|----------|--------|
| BUG-F7 | P1 | `App.tsx:37` (limit 50) × 5-min ingestion cadence | **Dashboard timeline is ~4 hours wide** (operator-confirmed): charts fetch only 50 historical snapshots ≈ 50 × 5 min ≈ 4h of data. With dedup fixed (BUG-B1) each row is real state, but 50 rows still caps the visible window at ~4h of active-market data. Fix: request ~24h of history (limit 288) and bound the historical payload (design D11). |
| BUG-F1 | P1 | `hooks/useFreshness.tsx:125-198`, `App.tsx:31` | Main effect deps include the unstable `onFreshnessChange` inline callback → effect teardown/re-run on **every render**: extra foreground fetch per state change, poll interval constantly reset. Worse: foreground `refresh(false)` sets `error` even when a last-known snapshot exists → one transient network blip replaces the entire dashboard with the error screen for up to 10s. |
| BUG-F2 | P2 | `App.tsx:161` | `upper: Math.min(parseFloat(b.upper_price), 2.5)` — hardcoded cap clips the displayed upper bracket if it ever exceeds $2.50. |
| BUG-F3 | P2 | `App.tsx:476-487` | "Alpha Generated ... vs passive hold" computes `total_portfolio_usd − total_native_usd`, which is just the current **USDC stack value** — not strategy alpha vs a passive hold. Misleading stat presented as live truth. |
| BUG-F4 | P3 | `App.tsx:305-516` | Header comment says "7 Real Charts" but only 6 render; backend `structural_channels` (Bollinger bands, fixed in issue #12) is never displayed. |
| BUG-F5 | P2 | `api/client.ts:51-58` | `request()` spreads `...init` **after** building merged `headers`, so any caller-passed `headers` clobbers `Content-Type: application/json` (latent; no caller passes headers yet). |
| BUG-F6 | P3 | `api/client.ts:97-117`, `hooks/useFreshness.tsx:226-311` | Dead code: `fetchIfFresh` is exported but never used (backend ETag path is exercised via plain fetch); `useFreshnessPoll` has zero call sites. |

### API contract — `specs/core/api.yaml` vs worker

| ID | Sev | Location | Defect |
|----|-----|----------|--------|
| BUG-A1 | P2 | `main.rs:700-707`, `api.yaml:91-118` | `/v1/snapshot/historical` declares `Interval`, `Before`, `After` params; the handler reads but ignores `interval` (`_interval`) and has no `before`/`after` support. API advertises parameters it silently drops. |

## User Stories

### US1 — Operator sees a true historical record
As the operator, I want the snapshot store and history charts to reflect real state changes only, so that steady markets produce one row per state, not one row per 5-minute tick.

**Acceptance criteria**
- [ ] With unchanged on-chain state across N ingestion cycles, exactly **one** snapshot row exists (dedup fires) while `/v1/snapshot/latest` still serves a fresh `generated_at` from the in-memory cache.
- [ ] A unit test proves `compute_state_hash` returns the **same** hash when only volatile timestamps differ, and a different hash when balances/reserves/fees change.
- [ ] The Strategy Counterfactual series is built from the **most recent** history and includes the newest snapshots as the table grows.
- [ ] Counterfactual active/passive series are always index-aligned; a row missing one field drops **both** values for that point.
- [ ] Channel and counterfactual points carry the actual snapshot `generated_at` timestamps (no fabricated 5-min spacing).
- [ ] The Bollinger final window includes the newest price.

### US2 — No synthetic numbers ever presented as live truth
As the operator, I want the system to fail loudly when a required RPC query fails, so that no fallback constant is ever displayed as on-chain data.

**Acceptance criteria**
- [ ] If the ETH-price or gas-price RPC fails after retries, the ingestion cycle fails explicitly (snapshot not stored/published) — consistent with issues #11/#16. No `2500.0` / `0.005` / `0.0043` fallbacks remain in the cycle path.
- [ ] `recommended_action` is computed from `claimable_usd` vs `gas_break_even_usd` (claim above ~2× gas → `claim_now`; below → accumulate/wait). Dashboard never shows "CLAIM NOW" at $0 claimable.
- [ ] V4 bracket display prices, `in_range`, and `distance_bps` are derived from the ticks (single source of truth), verified in tests against the known live values (`0.01237` / `2.02783` for ticks ±223200/±274200 on-chain).

### US3 — Dashboard survives transient failures
As the operator, I want the dashboard to keep showing last-known data during brief network blips, so that a 1-second hiccup doesn't blank the state room.

**Acceptance criteria**
- [ ] A transient fetch failure with existing snapshot data does **not** replace the dashboard with the error screen; the status pill/banner reflects the condition instead.
- [ ] Charts render ~24h of history (288 snapshots at the 5-min cadence), not ~4h; the historical response payload stays bounded (embedded series truncated server-side, design D11).
- [ ] The error screen appears only when no snapshot has ever loaded.
- [ ] `useFreshness` effect does not re-run (no extra foreground fetch, no interval reset) on unrelated re-renders; polling cadence stays at the configured interval.
- [ ] The historical-snapshot fetch retries on subsequent polls instead of being silently swallowed forever after one failure.

### US4 — Honest, clean presentation and contract
As the operator, I want chart labels to say what the numbers actually are, and the API to honor its declared contract.

**Acceptance criteria**
- [ ] The "Alpha Generated" stat is relabeled/derived to describe the actual quantity (portfolio-vs-native stable reserve delta), consistent with what the counterfactual chart plots. No stat claims "alpha vs passive hold" while computing a USDC balance.
- [ ] The tick-bracket chart displays the true upper price (no hardcoded $2.50 clip).
- [ ] `/v1/snapshot/historical` either implements `before`/`after` cursor filters or they are removed from `api.yaml` — no silently-ignored parameters. Unsupported `interval` values are rejected, not ignored.
- [ ] `request()` header merge preserves `Content-Type` even when callers pass custom headers.
- [ ] Dead code (`fetchIfFresh`, `useFreshnessPoll`) is removed.
- [ ] Either a structural-channels panel renders when channel data exists, or the "7 charts" comment is corrected — no phantom panel count.

## Non-Goals (deferred to v0.2.0)

- True passive-hold counterfactual math / P/L gauges and cost basis (v0.2.0 "Dual Income Streams").
- WalletConnect, multi-pool discovery, Coinbase staked-ETH tracking, Koinly export, UI overhaul.
- Unauthenticated `/v1/ingest` hardening beyond noting it (self-hosted deployment assumption).

## Release & Process Constraints

- Every defect is documented in this spec's registry (single source of truth on the `helix-specs` branch) with `file:line` RCA, repro, and labels. Per operator decision, GitHub issue filing is **not required** from the implementing agent; the post-merge Gemini pass may file issues from this registry using `gh` from the operator environment.
- All fixes land via PRs to `main` (branch protection; `unset GITHUB_TOKEN` for `gh`).
- `make test-brain`, `make test-worker` (needs `SQLX_OFFLINE=true`), `make test-frontend`, `make dist` must pass before merge.
- Tag `v0.1.4` only after merge + green CI + populated CHANGELOG section, **and only with explicit operator authorization** (OPERATOR_GUIDANCE.md standing orders).

## Open Questions

1. **BUG-B4 remedy**: fail the whole ingestion cycle when the ETH-price/gas RPC fails (strict, consistent with #11/#16), or store the snapshot with a `degraded: true` flag instead? **RESOLVED (approved design)**: fail explicitly.
2. **BUG-F3 remedy**: relabel the stat to what it actually measures (minimal), or hide it until a true counterfactual exists in v0.2.0? **RESOLVED (approved design)**: relabel.
3. **BUG-F4 remedy**: add the missing Bollinger-channels panel now (small chart), or just fix the comment? **RESOLVED (approved design)**: add the panel (hidden when no data).
4. **BUG-A1 remedy**: implement `before`/`after` cursor filters and drop `interval` from the spec, or drop all three? **RESOLVED (approved design)**: implement before/after, drop interval.
5. Single `v0.1.4` release for the whole batch, or split backend (v0.1.4) / frontend (v0.1.5)? **RESOLVED (operator)**: the priority is that the bugs actually get fixed; ship as one batch, single `v0.1.4`.
6. **BUG-F7 (operator-reported)**: dashboard timeline too narrow (~4h). Operator confirmed this is a bug and asked for it on the list. Fix approach: deepen the history window to ~24h and bound the historical payload.
