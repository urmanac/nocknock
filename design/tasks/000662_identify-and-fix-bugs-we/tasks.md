# Implementation Tasks: Fix Point-Release Bugs: Snapshot Dedup, History Integrity, Zero-Mock

## Batch 0 — Triage documentation

- [x] Document the full bug registry in this spec (single source of truth on `helix-specs`); GitHub issue filing deferred to the post-merge Gemini pass per operator decision (see requirements.md "Release & Process Constraints")
- [x] Add operator-reported BUG-F7 (dashboard timeline ~4h wide) to the registry with fix design D11

## Batch 1 — Worker data integrity (BUG-B1, B2, B3, B6, B7)

- [~] Add `canonical_state_json` time-field stripping + rewrite `compute_state_hash` to use it (design D1); extend `test_state_hash_determinism` with time-invariance and value-sensitivity cases
- [ ] Rewrite counterfactual history query as DESC-100 subquery reversed, collecting `(generated_at, portfolio_usd, native_usd)` tuples; drop independent series pushes (design D2)
- [ ] Add `generated_at` to the structural-channels history query; pass real timestamps into `compute_structural_channels_sync`
- [ ] Update `compute_counterfactual_sync` / `compute_structural_channels_sync` signatures to consume real timestamps; delete fabricated `now − i×5min` logic
- [ ] Fix Bollinger loop off-by-one (`window..len` → `window..=len`)
- [ ] `cd ingestion-worker && SQLX_OFFLINE=true cargo test` green; open PR, merge

## Batch 2 — Worker zero-mock & derived honesty (BUG-B4, B5, B8)

- [ ] Remove `live_eth_price_usd = 2500.0`, `live_gas_cost_usd = 0.005`, and `gas_cost = 0.0043` fallbacks; fail the cycle explicitly when those RPC queries exhaust retries (design D3)
- [ ] Compute `recommended_action` per fee contract from claimable vs gas (claim_now / claim_recommended / accumulate, design D4)
- [ ] Add `tick_to_display_price` helper; derive bracket prices, `in_range`, and `distance_bps` from ticks in both `fetch_raw_data` and `compute_derived_metrics`; delete hardcoded `"0.01237"`/`"2.02783"`/`-200` (design D5)
- [ ] Add round-trip unit test: known ticks ±223200/±274200 → `0.01237`/`2.02783`; action-threshold unit test
- [ ] `SQLX_OFFLINE=true cargo test` green; open PR, merge

## Batch 3 — Frontend resilience (BUG-F1)

- [ ] Store `onFreshnessChange` in a ref; remove it from `refresh` deps and the main polling effect deps (design D6)
- [ ] In `refresh`, only `setError` when no snapshot is available; keep last-known data on transient failures
- [ ] Retry the historical-snapshot fetch on poll cadence instead of swallowing errors forever after one failure
- [ ] `cd frontend && npm test` and `npm run build` green; open PR, merge

## Batch 4 — Frontend presentation & hygiene (BUG-F2, F3, F4, F5, F6)

- [ ] Fix `request()` header merge order in `client.ts`; delete dead `fetchIfFresh` and `useFreshnessPoll`
- [ ] Remove hardcoded $2.50 upper-bracket clip in `App.tsx` tickData mapping
- [ ] Relabel "Alpha Generated" to the actual quantity (stable reserve delta), computing from counterfactual endpoints (design D7)
- [ ] Add structural-channels panel when `structural_channels.channels` is non-empty (or fix the "7 Real Charts" comment if panel declined — see Open Questions)
- [ ] Vitest + tsc build green; open PR, merge

## Batch 5 — API contract (BUG-A1)

- [ ] Implement `before`/`after` filters on `get_historical_snapshots_handler`; reject unsupported `interval` with 400
- [ ] Remove `Interval` parameter from `specs/core/api.yaml`; update frontend `historicalSnapshots` typing
- [ ] `SQLX_OFFLINE=true cargo test` + frontend tests green; open PR, merge

## Batch 6 — Release (v0.1.4)

- [ ] Update `CHANGELOG.md` with a populated `0.1.4` section; bump versions (`frontend/package.json`, worker `Cargo.toml`/Cargo.lock)
- [ ] Run full pre-flight gate: `make test-brain && make test-worker && make test-frontend && make dist`
- [ ] Confirm all PRs merged and CI green on `main`; pull latest `main`
- [ ] Request operator authorization, then tag `v0.1.4` and push tag (release workflow attaches artifacts); update `TRIAGE.md` registry statuses
