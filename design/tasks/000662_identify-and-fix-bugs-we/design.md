# Design: Fix Point-Release Bugs: Snapshot Dedup, History Integrity, Zero-Mock

## Context

NockNock is a Base-mainnet portfolio state room: an Axum ingestion worker polls Alchemy RPC
every 300s, stores snapshot envelopes in Postgres (TimescaleDB), and a React/recharts
dashboard renders them. All defects below were verified by reading `main` @ `f621e6f`
(worker `ingestion-worker/src/main.rs`, frontend `frontend/src/App.tsx`,
`frontend/src/hooks/useFreshness.tsx`, `frontend/src/api/client.ts`).

## Key Decisions

### D1 — Canonical state hash excludes wall-clock time (BUG-B1)

`compute_state_hash` currently hashes the full serialized `SnapshotData`, which embeds
`wallet_balances[].updated_at`, `high_water_mark_date`, series `time_points`, and the
fabricated channel/counterfactual timestamps. The hash therefore changes every cycle and
`store_snapshot`'s dedup guard (`state_hash` comparison) is dead code.

**Fix**: build a canonical projection before hashing — serialize `SnapshotData` to
`serde_json::Value`, then blank the volatile time fields at known paths:

- `wallet_balances[*].updated_at`
- `derived_metrics.global_status.high_water_mark_date`
- `derived_metrics.asset_aggregation_series.time_points`
- `derived_metrics.total_value_vs_baseline.time_points`
- `derived_metrics.strategy_counterfactual.time_points`
- `derived_metrics.structural_channels.channels[*].{start_time,break_point}` and
  `*_line[*].time` / `upper_1sd[*].time` etc. (all `LinePoint.time`)

Blanking (not removing) keeps the JSON shape identical so deserialization back into
`SnapshotData` still works. Chosen over "hash only raw on-chain values" because it is a
small, surgical change that keeps `compute_state_hash(&SnapshotData)`'s signature and
existing determinism test meaningful.

**Tests**: (1) hash equality when only timestamps differ; (2) hash inequality when a
balance/fee/price changes. Extends the existing `test_state_hash_determinism`.

### D2 — History queries fetch recent rows, paired, with real timestamps (BUG-B2/B3/B6)

Replace the counterfactual query with a `DESC LIMIT 100` subquery reversed in Rust:

```sql
SELECT generated_at,
       (data->'derived_metrics'->'global_status'->>'total_portfolio_usd')::float8 AS portfolio_usd,
       (data->'derived_metrics'->'global_status'->>'total_native_usd')::float8    AS native_usd
FROM (SELECT generated_at, data FROM snapshots ORDER BY generated_at DESC LIMIT 100) t
ORDER BY generated_at ASC
```

Rows are collected as a `Vec<(DateTime<Utc>, Option<f64>, Option<f64>)>` tuple so both
series are pushed together (fixes misalignment) and the real `generated_at` flows into
`compute_counterfactual_sync` / `compute_structural_channels_sync` instead of the
fabricated `now − i×5min` timestamps. The structural-channels query gets the same
treatment (it already fetches DESC + reverse; add `generated_at` to the SELECT).
`compute_counterfactual_sync` / `compute_structural_channels_sync` signatures gain a
timestamp per point; series value formatting stays unchanged.

### D3 — Fail explicitly when required RPC inputs fail (BUG-B4)

ETH price and gas price are required inputs (they feed `gas_break_even_usd`, efficiency
ratios, and the advice engine). Follow the precedent of issues #11/#16: after retries
exhausted, return `Err(...)` from `fetch_raw_data` so the cycle fails and nothing synthetic
is stored. Remove the `2500.0`, `0.005`, and `0.0043` fallbacks. The Aerodrome reserves and
V4 StateView queries already behave this way; this makes the last two queries consistent.

### D4 — Computed `recommended_action` (BUG-B5)

Replace hardcoded `"claim_now"` with:

```
claim_now          if claimable_usd >= 2 × gas_break_even_usd
claim_recommended  if claimable_usd > 0
accumulate         otherwise
```

The 2× factor mirrors the existing `gas_to_fee_delta_usd` margin logic
(`total_fees_usd − gas_cost × 2`). Frontend already colors/renders `action` dynamically —
no frontend change required beyond accepting the new enum values.

### D5 — Derive bracket prices from ticks (BUG-B8)

Verified relation for the NOCK/USDC V4 pool (USDC 6 dec, NOCK 16 dec), cross-checked
against live values: on-chain ticks are **positive** (+223200 lower, +274200 upper) and the
displayed USDC-per-NOCK prices use the negated exponent:

```
displayed_price(t) = 1.0001^(−t) × 10^(dec_NOCK − dec_USDC) = 1.0001^(−t) × 1e10
1.0001^(−274200) × 1e10 = 0.012355 ≈ "0.01237"  ✓
1.0001^(−223200) × 1e10 = 2.02770  ≈ "2.02783"  ✓
```

Add one helper `tick_to_display_price(tick_onchain: i32) -> f64` used by both
`fetch_raw_data` (concentrated liquidity) and `compute_derived_metrics` (tick bracket
hypervisor). `in_range = spot ≥ lower && spot ≤ upper` from derived prices;
`distance_bps` computed from spot vs nearest bracket edge
(`((edge − spot)/spot × 10000) as i32`) instead of the hardcoded `-200`. Unit tests assert
round-trip against `0.01237`/`2.02783` for the known ticks so a sign/orientation mistake
fails loudly.

### D6 — Stabilize `useFreshness` effects and keep last-known data (BUG-F1)

Two changes in `useFreshness.tsx`:

1. **Ref-stabilize callbacks**: store `onFreshnessChange` in a ref (same pattern as
   `freshnessRef`); remove it from the `refresh` useCallback deps and the main effect deps.
   The polling effect then depends only on `[refresh, pollIntervalMs]` — `refresh` only
   changes when `fetchSnapshot`/`computeFreshness` change, which are stable per mount in
   practice. This stops the teardown/re-run churn (extra foreground fetch + interval reset
   on every render).
2. **Error handling keeps last-known data**: in `refresh`, treat a fetch failure as
   background-style whenever `snapshotRef.current` exists, regardless of foreground flag —
   i.e. only `setError` when there is no snapshot to show. The error screen then appears
   only on cold-start failures; transient blips keep the dashboard up.

In `App.tsx`, also retry the history fetch on the freshness poll cadence (cheap: piggyback
on the existing `snapshot_id`-triggered effect plus a periodic retry) so a single failed
history load isn't swallowed forever (`.catch(() => {})` today).

### D7 — Honest alpha label (BUG-F3)

Minimal honest fix: the stat computes `active − passive` from the **counterfactual series
endpoints** (same numbers the adjacent chart plots) and is relabeled, e.g.
"Stable Reserve Delta (Portfolio vs Native)" with subtitle explaining it equals the USDC
side today. True strategy-vs-passive-hold alpha is a v0.2.0 feature (P/L engine); do not
fake it. Contract-test alpha math helper stays as-is (pure math unchanged).

### D8 — API contract honesty (BUG-A1)

Implement `before`/`after` as simple `WHERE generated_at < $n / > $n` filters on the
historical handler (~10 lines, matches `HistoricalResponse.pagination.next_cursor`
usage), and **remove `Interval` from `api.yaml`** (it was never implemented; bucketing is
v0.2.0 scope). Unsupported `interval` query values, if still sent by old clients, are
harmlessly ignored only after spec removal — during the transition, reject with 400 per
the spec's error contract. Frontend `historicalSnapshots` param typing drops `interval`.

### D9 — Client hygiene (BUG-F2/F5/F6)

- `request()`: spread `...init` **first**, then set `headers` merging `init?.headers` —
  preserves `Content-Type` for callers passing headers.
- Delete dead `fetchIfFresh` and `useFreshnessPoll` (backend ETag flow is exercised by the
  plain-fetch contract tests; nothing imports either).
- Remove the `Math.min(..., 2.5)` cap on the displayed upper bracket.

### D10 — Structural channels panel (BUG-F4)

If channel data exists (≥20 historical points), render a 7th panel: `LineChart` with
center/±1σ/±2σ channel lines from `structural_channels.channels[0]`. Hide the panel
entirely when `channels` is empty (cold start). Fix the stale "7 Real Charts" comment
(panel count becomes dynamic in the copy). Gate on D2 landing first (real timestamps).

### D11 — Deep, bounded history window (BUG-F7, operator-reported)

The operator sees only ~4h of chart timeline: the frontend requests `limit: 50` snapshots
at a 5-min cadence. Fix in two parts:

1. **Frontend**: request `limit: 288` (~24h at 5-min cadence).
2. **Server payload bound**: each stored snapshot embeds a full counterfactual series
   (≤101 points × 2 series) and Bollinger channels (5 lines × ≤31 points) — ~15–20 KB of
   redundant payload per historical row that the charts never read (they only use each
   row's `[0]` current-point values). The historical handler truncates each row's embedded
   `strategy_counterfactual` to its first point and empties `structural_channels.channels`
   before responding, cutting the 288-row response from ~5 MB to ~1 MB. `/snapshot/latest`
   keeps the full series (MCP consumers rely on it; snapshot.yaml declares these fields).
   True `interval` downsampling stays deferred to v0.2.0 if 24h is not enough.

## Learnings for Implementers

- **Rust/worker**: build with `cd ingestion-worker && SQLX_OFFLINE=true cargo build --release`
  (compile-time checked queries; `.sqlx/` cache is committed). Tests: `make test-worker`.
- **Frontend**: Node 22+, `cd frontend && npm test` (vitest; contract tests auto-skip when
  `localhost:8080` is unreachable — no hang risk) and `npm run build` (tsc + vite).
- **Tick orientation gotcha**: the on-chain V4 StateView calls use *positive* ticks
  (223200/274200) while the display math negates them (see D5). Do not "fix" one without
  the other — the live-verified fee test (`test_u256_wrapping_sub_and_mul_div`) and the
  bracket-price round-trip tests pin both.
- **Dedup semantics**: when the state hash is unchanged, `store_snapshot` skips the DB
  insert but the in-memory `latest_snapshot` is still overwritten with a fresh
  `generated_at`/`snapshot_id` — that asymmetry is intentional (issue #10); keep it.
- **Git protocol**: branch off `main`, PR via `unset GITHUB_TOKEN && gh pr create --fill`,
  merge with `gh pr merge <N> --merge`. Never tag without operator authorization.
- `gh` CLI may be absent in agent sandboxes — per operator decision, GitHub issue filing
  is not required; this spec's bug registry is the single source of truth (a post-merge
  Gemini pass may file issues from it with `gh`).

## Implementation Notes

- Sandbox tooling: installed rustup + gcc + pkg-config + libssl-dev to run
  `SQLX_OFFLINE=true cargo test --release` locally — baseline green (11 tests) before any
  change. Frontend baseline: 20 vitest tests + tsc build green.
- Verified tick→display-price math precisely: `1.0001^(−tick) × 1e10` gives 0.0123659
  (hardcoded "0.01237") and 2.02788 (hardcoded "2.02783") — the derived strings differ from
  the old hardcoded ones in the 5th significant digit only; tests assert 0.1% tolerance,
  and the dashboard formats `{:.5}` so the displayed value may shift by ≤0.00005.
  This is expected and honest (derived from ticks, not pasted strings).
- Payload math (BUG-F7): stored snapshots embed ≤101-point counterfactual + 31-point×5
  channel series (~20 KB/row). Historical handler truncates those embedded series; charts
  only read each row's `[0]` values, so no visual change — just bounded payloads.
- **Cargo.lock is gitignored** in `nocknock` — don't stage it when bumping the worker
  version (CI builds the lock fresh).
- The `commit-msg` hook enforces conventional commits in BOTH repos (`nocknock` and
  `helix-specs`); the pre-approved non-conventional spec-commit message from the task
  template gets rejected — prefix with `docs(specs):`.
- Sandbox toolchain install that made local validation possible: `rustup` (minimal
  profile), `sudo apt-get install -y gcc libc6-dev pkg-config libssl-dev make`, then
  `rustup target add wasm32-wasip1` for `make dist`. Baseline before changes: brain 3,
  worker 11, frontend 20 tests green.
- GOTCHA caught by test: in `bracket_distance_bps`, at spot $1.00 the NEAREST bracket
  edge is the LOWER one (0.0124 → 9876 bps away vs 10279 bps to the upper) — my first
  test expectation assumed the upper edge and failed; sign conventions matter.
- Advice-engine policy constants (`$0.020` accumulation boundary, `$5.00` harvest
  threshold, `120,000` target, `$5.00+` wall) were intentionally LEFT hardcoded: they are
  stated strategy policy, not failure fallbacks — externalizing them is an enhancement
  (issue #6 pattern), not a bug fix. The RANGE EXIT threshold, by contrast, WAS derived
  from ticks (`v4_upper_price`) because it duplicates a bracket value.
- Files changed (single feature branch `feature/000662-fix-point-release-bugs`, one
  commit per batch):
  - `ingestion-worker/src/main.rs` — canonical hash, history queries/tuples, tick
    helpers, claim action, fail-explicit RPC, handler cursors + payload slimming, +7 tests
  - `frontend/src/hooks/useFreshness.tsx` — ref-stabilized callbacks, last-known-data
    error policy, removed useFreshnessPoll
  - `frontend/src/App.tsx` — history loader (288 + retry), honest labels, unclipped
    bracket, structural channels panel #7
  - `frontend/src/api/client.ts` — header merge fix, dead code removal, interval typing
  - `specs/core/api.yaml` — Interval parameter removed
  - `CHANGELOG.md` (0.1.4 section), `frontend/package.json`, `ingestion-worker/Cargo.toml`
    (0.1.4)

## Validation Plan

1. `make test-worker` — new unit tests: hash time-invariance, bracket-price round-trip,
   action computation, counterfactual pairing (add a pure function test for the tuple
   collector if extracted).
2. `make test-frontend` — existing 20 tests stay green; alpha relabel doesn't break math
   assertions.
3. `make dist` — packaging sanity.
4. Manual (operator env with live RPC): watch two consecutive ingestion cycles with
   unchanged state → row count in `snapshots` stays flat, `/v1/snapshot/latest` still
   advances its `generated_at`; counterfactual chart shows recent history with real times.
