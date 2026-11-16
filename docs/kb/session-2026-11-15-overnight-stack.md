# Session 2026-11-14 (evening) → 2026-11-16: v0.5.2, v0.5.3, and the v0.6.0–v0.8.0 stack

*Claude: Opus 5.5, then Fable 5.2 with Ultracode workflows. It follows the planning session in
[session-2026-11-14-planning-lore-mining.md](session-2026-11-14-planning-lore-mining.md).
The last part ran overnight without the operator, by their instruction.*

## What the operator asked for

- Release v0.5.2, then start on #102 and #111 and release that as v0.5.3.
- Plan the long-term shape: what belongs in v0.6.0, v0.7.0 and beyond. Decided: v0.6.0 is
  "the whole wallet" (the numbers have to be right first), v0.7.0 is "guided buys", v0.8.0
  is "on your phone".
- Overnight: do the heavy lifting for v0.6.0 to v0.8.0 and submit pull requests for all of
  it, stacked, without stopping for review, so the three can be released as one minor
  release in the morning if they are in order.

## What was released

- **v0.5.2** (PR #116) and **v0.5.3** (PR #117, harvest ledger: #102, #111) were merged,
  tagged with the operator's authorization, deployed to chudmax and verified there.
- chudmax runs **v0.5.3** at the end of this session. Nothing from the stack below was
  deployed.

## What was filed

- Milestones v0.6.0 (#97, #91, #93, #118), v0.7.0 (#115, #98), v0.8.0 (#89).
- New issues: #118 (fill detection), #119 (from-zero ingestion, unscheduled). #115 was
  rewritten to cover route quotes only.

## The stack (nothing merged, nothing tagged, nothing deployed)

Seven pull requests, each based on the one before it:

| PR | Branch | Issue | Milestone | Tests at its tip (worker + MCP / frontend) |
|---|---|---|---|---|
| #120 | `refactor/97-split-main` | #97 | v0.6.0 | 137 + 8 / 107 |
| #121 | `feat/91-aero-cl` | #91 (+ #127, #128) | v0.6.0 | 165 + 10 / 119 |
| #122 | `feat/93-aero-rewards` | #93 | v0.6.0 | 194 + 13 / 137 |
| #123 | `feat/118-fill-detection` | #118 (+ #129) | v0.6.0, spec 2.5.0 | 225 + 16 / 170 |
| #124 | `feat/115-route-quotes` | #115 | v0.7.0 | 260 + 18 / 201 |
| #125 | `feat/98-cost-basis` | #98 | v0.7.0, spec 2.6.0 | 290 + 19 / 222 |
| #126 | `feat/89-phone` | #89 | v0.8.0 | 323 + 19 / 256 |

How it was built:

- #97 was done first as 22 move-only extraction steps behind a golden snapshot test.
- The six features were built in four isolated worktrees by workflow agents. Each got an
  implementation pass, two independent reviews (on-chain correctness; product and
  surfaces) and a fix pass.
- I then stacked the branches in the planned order with merge commits, resolved the
  conflicts by keeping both sides, bumped the snapshot spec at the v0.6.0 tip (2.5.0) and
  the v0.7.0 tip (2.6.0), regenerated the golden snapshots and reviewed their diffs, and
  updated the test counts at every tip.

## The review of the whole stack, and what it changed

After stacking, six reviewers looked at the tip from different angles (merge integrity,
cross-feature behaviour, deploy and upgrade safety, security, spec and surface
consistency, and an independent recomputation of the numbers from raw chain reads). A
second agent then tried to refute every serious finding. Ten findings survived; they were
the same three defects seen from several angles, and all three are fixed in the stack:

- **#127, blocker.** A network error from `eth_call_allow_revert` carried the request URL,
  which holds the Alchemy key, into text that is published. The root line is already in
  v0.5.3 (it can reach `/health`); the stack would have made it persistent through the
  AERO notes stored in the snapshot. Fixed on the #91 branch. chudmax showed
  `last_error: null` when checked, and v0.5.3 snapshots have no note fields.
- **#129, major.** The stored-price lookup returned a backfilled candle's close labelled
  with its open time, so a buy's "cost over spot" could be measured against a price from
  an hour later, and could disagree with the cost-basis panel for the same buy. Fixed on
  the #118 branch. On the scratch database the two figures now agree exactly for all
  three real buys.
- **#128, minor.** `deploy.sh` run from a checkout holding nested git worktrees would
  have shipped them whole (about 16 GB on this workstation). Fixed on the #91 branch.

Also fixed: `specs/core/snapshot.yaml` had stopped parsing as YAML on the #91 branch (I
found this while the review ran; a test now loads both spec files). Deploys being
forward-only is now stated in AGENTS.md section 13 and DEPLOYMENT.md.

Each of the three fixes has a test that I confirmed fails on the old code. The minor
findings were filed as issues rather than patched overnight; they are listed in the
issues opened on 2026-11-16: #130 (a failed ledger read shows $0.00 progress), #131
(range bounds to three significant digits), #132 (plan venue default), #133 (LAN API
hardening), #134 (overlapping ingest cycles), #135 (notice order in the directive), #136
(history payload and quote-table growth).

## What was verified, and how

- **Every tip**: worker tests, frontend tests and the frontend build were run at each of
  the six branch tips after stacking. Counts are in the table; zero compiler warnings.
- **Stack tip**: `make test-brain` (3), `make dist` (four artifacts, run again after the review
  fixes), the four ignored tests (three scratch-database, one live-RPC) and the
  database-gated cost-basis tests. The scratch-database tests need an empty, migrated
  database whose name ends in `_scratch`.
- **End to end against the live chain**, through the operator's Alchemy plan, with the
  stack-tip worker on a throwaway PostgreSQL and a private copy of the operator's config
  (copied to a 0700 temp directory, deleted afterwards; the real file was not edited):
  - Unchanged config: the same wallet as chudmax's v0.5.3 ($2,388.70 against $2,388.59).
    Migrations 10, 11 and 12 applied on a database that was at the v0.5.3 schema.
  - With the Aero CL block added to the copy: position `aerodrome-cl:base:2914490` read
    $179.62, in range, staked in gauge `0x3bf70c88…60fd`; total $2,568.35.
  - AERO: wallet 0; unclaimed 0.125 (basic gauge) and 12.47 (CL gauge), about $5.26.
  - Fill detection: the worker's own `alchemy_getAssetTransfers` request works. With
    `starts_at: 2026-11-14` in the copy it offered exactly the three plan buys. Confirming
    them in the dashboard rebuilt the plan to $60 of $100 at $0.011768, tranche 4 due
    2026-11-17T03:08:37Z, the same as chudmax. This is also the database-loss recovery
    path, now without typing hashes.
  - Route quotes: for a $20 tranche, Aero 1,784.13 NOCK against Uniswap 1,783.13.
  - Cost basis: $0.011768 over 5,098.51 NOCK for $60.
  - Harvest ledger: recording the two real fee claims gave the same rows as chudmax
    (NOCK values differ by under a cent: the scratch database had only hourly prices).
  - MCP playtest passed on both transports. The dashboard and the Lite cockpit (375 px)
    rendered with no console errors.
- **The AERO token**: I re-checked the agents' finding from claim receipt
  `0x189f6ddf…b8f8`: the gauges mint `0x3f5bF3Ed…a4Ee` ("Aero"), not `0x940181a9…8631`
  ("Aerodrome").

## What was not verified

- **GitHub CI did not run on #121 to #126.** The workflows trigger only for pull requests
  into `main`. CI ran on #120 only. A workflow change that would have fixed this could
  not be pushed (see below); it is saved outside the repo and described in the summary.
- Nothing on a real phone; no Telegram or Pushover message; the Caddy TLS proxy has never
  run.
- The whole-stack cost basis with real figures; an AERO claim by the operator's wallet; a
  buy recorded after a quote.
- The migrations on chudmax's own database.

## Things the operator should know

- **The SSH key was not loaded overnight**, so `git push` over SSH failed. I pushed over
  HTTPS with the `gh` keyring credential for those commands only (no git config changed).
- **The `gh` token has no `workflow` scope**, so a push touching `.github/workflows` is
  rejected. I did not work around it.
- **To track the Aero CL position**, add the block in #121's description to `positions:`
  in `config.yaml`. Expect the total and the directive to step once.
- **Decisions taken on the operator's behalf** are listed in each pull request under
  "Decisions taken for you". None of them chooses an allocation number.
- **Going back is not free.** Once a build with migrations 10 to 12 has run against a
  database, v0.5.3 will not start on it (see AGENTS.md section 13).
- `Cargo.toml` still says 0.5.0. Stale PR #59 is still open in milestone v0.4.0.
