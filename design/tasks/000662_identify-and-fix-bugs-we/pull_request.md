# Fix point-release bugs: snapshot dedup, history integrity, zero-mock

## Summary

Implements the approved spec for the `v0.1.4` bug batch: 16 verified defects across the
ingestion worker, React dashboard, and API contract. Three high-severity data-integrity
bugs (dead snapshot dedup, stale counterfactual history, silent mock fallbacks) are fixed
at the root, plus the operator-reported narrow chart timeline (~4h → ~24h). Full registry
with `file:line` evidence lives in
`design/tasks/000662_identify-and-fix-bugs-we/requirements.md` on the `helix-specs` branch.

## Changes

**Worker data integrity (`ingestion-worker/src/main.rs`)**
- `compute_state_hash` strips wall-clock fields — dedup was dead code because the hash
  changed every cycle (new DB row every 5 min even in steady state, unbounded growth)
- Counterfactual history now selects the most recent 100 snapshots (was oldest 100) as
  index-aligned `(timestamp, portfolio, native)` tuples with real snapshot timestamps
  (no fabricated 5-min spacing; same for Bollinger channels)
- Bollinger window off-by-one: final window includes the newest sample

**Zero-mock invariant**
- Failed ETH-price / gas-price RPC queries now fail the ingestion cycle instead of
  silently substituting `2500.0` / `0.005` / `0.0043` fallbacks
- `recommended_action` computed from claimable vs gas break-even — no more "CLAIM NOW"
  at $0 claimable
- V4 bracket prices, `in_range`, `distance_bps` derived from on-chain ticks (verified
  0.012366 / 2.02788) replacing duplicated hardcoded strings

**Frontend**
- `useFreshness`: polling interval no longer torn down/recreated (with an extra fetch) on
  every render; transient fetch failures keep last-known data — the error screen appears
  only when nothing has ever loaded; history fetch retries while empty
- Chart timeline ~24h (288 snapshots) with server-side payload bounding (~1 MB not ~5 MB):
  historical responses truncate embedded multi-point series the charts never read
- "Alpha Generated vs passive hold" relabeled to what it measures ("Stable Reserve Delta
  (Total − Native-Only)"); counterfactual panel retitled "Portfolio Composition"
- True upper bracket price (removed hardcoded $2.50 clip); new 7th panel renders the
  Bollinger structural channel when band data exists
- `client.ts` header-merge fix; dead `fetchIfFresh` / `useFreshnessPoll` removed

**API contract**
- `/v1/snapshot/historical` implements `before`/`after` RFC3339 cursors, clamps `limit`,
  returns 400 for the never-implemented `interval` (removed from `specs/core/api.yaml`)

**Release**
- `CHANGELOG.md` 0.1.4 section; version bumps (worker `Cargo.toml`, `package.json`)

## Testing

- `make test-brain`: 3/3 · `make test-worker`: 18/18 (7 new: hash time-invariance,
  hash value-sensitivity, counterfactual alignment/timestamps, Bollinger window, bracket
  price round-trip, bracket distance, claim-action thresholds)
- `make test-frontend`: 20/20 · `npm run build` (tsc + vite): clean
- `make dist`: all 4 artifacts packaged
- `origin/main` merged into the branch; `git merge-base --is-ancestor origin/main HEAD` passes
