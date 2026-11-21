# NockNock Agent Orchestration Guide (`AGENTS.md`)

This guide captures the operational rules, environment quirks, toolchain configurations, and tribal knowledge required for autonomous agents to develop, test, and release **NockNock**.

---

## 1. Non-Negotiable Branch Protection & Git Protocol

- **`main` is protected**:
  - Direct pushes to `main` are rejected by GitHub remote rules.
  - Force pushes (`git push -f`) and deletions on `main` are strictly forbidden for everyone (including repository administrators).
- **All changes MUST land via Pull Requests**:
  - Always branch off `main`: `git checkout -b <branch-name>`
  - Push branch: `git push -u origin <branch-name>`
  - Create PR: `unset GITHUB_TOKEN && gh pr create --fill`
  - Merge PR: `unset GITHUB_TOKEN && gh pr merge <PR_NUMBER> --merge`
  - Pull merged `main`: `git checkout main && git pull origin main`
- **Release Tags**:
  - Tags are NOT protected by branch rules. They can be created, pushed, and deleted.
  - **Never tag or push a release until**:
    1. The feature/fix is merged to `main`.
    2. All CI checks on `main` have completed green.
    3. The version section in `CHANGELOG.md` is populated.
  - If a premature tag is pushed, immediately delete it:
    ```bash
    git tag -d vX.Y.Z
    git push --delete origin vX.Y.Z
    unset GITHUB_TOKEN && gh release delete vX.Y.Z --yes
    ```

---

## 2. GitHub CLI & Credential Management

- **The `GITHUB_TOKEN` Environment Quirk**:
  - Subshells and AI runner processes often inherit a restricted or scoped `GITHUB_TOKEN` that lacks write/PR permissions on `urmanac/nocknock`.
  - **Crucial Rule**: ALWAYS prepend `unset GITHUB_TOKEN` to all `gh` commands:
    ```bash
    unset GITHUB_TOKEN && gh pr create ...
    unset GITHUB_TOKEN && gh pr merge ...
    unset GITHUB_TOKEN && gh run watch <RUN_ID>
    unset GITHUB_TOKEN && gh release view ...
    ```
  - Unsetting the variable forces `gh` to fall back to the host keychain credentials which hold repository admin rights.

---

## 3. WASM Brain Toolchain & WASIP1 Linker

- **Target**: `wasm32-wasip1` using stable Rust.
- **Linker Flag Requirement**:
  - Extism / WIT bindings import host runtime symbols (e.g. `logger:log-debug`).
  - Without the `--allow-undefined` linker flag, `rust-lld` fails with undefined external symbol errors.
  - **Always ensure**:
    - `brain/.cargo/config.toml` contains:
      ```toml
      [target.wasm32-wasip1]
      rustflags = ["-C", "link-arg=--allow-undefined"]
      ```
    - When invoking `cargo build` in CI or makefiles:
      ```bash
      RUSTFLAGS="-C link-arg=--allow-undefined" cargo build --target wasm32-wasip1 --release
      ```
- **Invariant**: The Brain is pure deterministic mathematical logic (LP ratios, tick conversions, floor checks). It has zero dependencies on network, databases, or system clocks.

---

## 4. Ingestion Worker & SQLX Offline Mode

- **SQLX Offline Compilation**:
  - The worker uses compile-time checked queries via `sqlx`.
  - In CI or developer environments without a live PostgreSQL database, cargo builds will fail unless `SQLX_OFFLINE=true` is set.
  - The repo includes the query cache in `ingestion-worker/.sqlx/`.
  - **Always build with**:
    ```bash
    cd ingestion-worker && SQLX_OFFLINE=true cargo build --release
    ```
- **Module layout (since #97)**: `src/main.rs` holds the config schema, every shared type and
  startup. Functions live in modules: `adapters/` (one file per protocol; `adapters/mod.rs`
  holds every function that branches on the protocol string), `onchain` (one cycle's reads),
  `valuation`, `advice`, `lp_rebuild`, `plan`, `ledger`, `claims`, `market`, `ingest`, `api`,
  `aero_rewards`, `aero_text`, `harvest_principal`, `rewards`, `reward_text`, `closed`, `closed_text`, `fill_verify`, `fill_detection`, `fill_text`, `route_quotes`, `quote_text`, `cost_basis`, `cost_basis_text`, `rpc`, `evm`, `calldata`, `contracts`, `notify`, `mcp_text`, `reserve_text` (shared with the stdio MCP), `floor_breach` (the floor-crossing price, #165; `floor_breach_with` adds hypothetical ranges), `floor_breach_text` (shared with the stdio MCP), `ladder` (the read-only ladder simulator, #166: `GET /v1/simulate/ladder` and the `simulate_ladder` tool, from the stored snapshot only), `ladder_text` (its text, tool schema and arguments, shared with the stdio MCP), `ladder_track` (a deployed ladder tracked as `role: ladder`, #167: config checks, the deposit-receipt baseline and `data.ladders[]`; `contracts::is_nock_lp` is NOCK LP without ladders), `ladder_track_text` (its MCP lines, shared with the stdio MCP), `ladder_suggest` (the ladder suggestion, #168: `derived_metrics.ladder_suggestion` from the assembled snapshot through the simulator, a top-of-page event that never changes the directive; with `operator_profile.ladder_rungs` set, `shape_suggestion` builds it from `derived_metrics.ladder_shape` instead, one numbered deposit per rung still to deposit, and a tracked ladder no longer hides it; tests in `tests_ladder_suggest.rs` and `tests_ladder_bottle.rs`, goldens `testdata/ladder_suggest/golden_ladder_suggest.json` for one rung and `golden_ladder_bottle.json` for a shape), `ladder_suggest_text` (its get_shot_advice section, shared with the stdio MCP), `fee_yield` (fees per unit of liquidity measured on our own tracked ranges from stored snapshots, 2.10.0: at most one bounded query an hour, carried forward; `derived_metrics.fee_yield` and `pool_mechanics[].active_liquidity_raw` are left out of the state hash; the ladder simulator's rungs, `shape=configured` and `derived_metrics.ladder_shape` are in `ladder`; tests in `tests_fee_yield.rs` and `tests_ladder_rungs.rs`), `mint_math` (exact integer mint amounts: TickMath, LiquidityAmounts and SqrtPriceMath on a small big integer, no crate), `tracked` (positions tracked from the database, v0.9.0: the store and its SQL, the merge after `config.yaml`'s positions, the owner check; `PositionStore::Mem` is the same rules in memory for tests), `position_detect` (finding the wallet's Uniswap v4 position NFTs on-chain before the position reads: the transfer scan, the decision to track or offer, `derived_metrics.position_detection` and the `/v1/positions/*` routes), `position_detect_text` (its MCP lines, shared with the stdio MCP), `mcp_http`, and `tests` (each feature since #91 adds its own `tests_<topic>.rs`; the v0.8.1 burn-down's are in `tests_burndown.rs`).
  The HTTP router is `api::api_router`, with the cross-site write guard (#133); every
  ingestion cycle runs through `ingest::CycleGate`, one at a time (#134).
  `closed` (#144) decides when a configured position's NFT provably does not exist; the
  allow-list of contract answers is in `adapters/mod.rs`. The proof is read at one current
  block (`closed::fetch_proof_block`, `ProofClock`): never loosen that into reads at
  `latest`, because an id that is not minted yet answers like a burned one.
  `testdata/closed/cycle_fixture.json` is one whole cycle's real RPC answers, re-recorded with
  `NOCKNOCK_LIVE_RPC_URL=https://base-rpc.publicnode.com cargo test test_record_closed_cycle_fixture -- --ignored`
  (add `NOCKNOCK_RECORD_BLOCK=58646285` to add reads at the block it already holds; that
  keeps its gas price, so the golden file does not move).
  Moved items are `pub(crate)`. `testdata/golden_snapshot.json` pins the snapshot bytes: a
  refactor must leave it untouched; an intended behaviour change regenerates it with
  `NOCKNOCK_BLESS_GOLDEN=1 cargo test test_golden_snapshot` and the diff is reviewed.
  `tracked` and `position_detect` (v0.9.0) use runtime `sqlx::query`, so the `.sqlx` cache
  does not change with them. Their SQL is run for real by two ignored tests against a
  scratch PostgreSQL whose name ends in `_scratch` (the migrations are applied to it):
  `NOCKNOCK_SCRATCH_DATABASE_URL=postgres://…/nocknock_scratch cargo test scratch_database -- --ignored`.
  CI has no PostgreSQL: run it before a release that touches a query or a migration.
  `testdata/position_detect/chain_fixture.json` holds recorded chain answers for the scan
  (ONCHAIN_INTEGRATION.md §15); a position is tracked by itself only on an exact pool and
  tick match with a suggested rung: never loosen that, or a stranger's dust mint on a
  rung's ticks marks it deployed.
- **Binaries Produced**:
  - `nocknock-ingestion-worker`: Background ingestion daemon and Axum REST API (port 8080).
  - `nocknock-mcp`: Stdio Model Context Protocol (MCP) server for Claude Desktop, Gemini Antigravity, and AI agents.

---

## 5. Frontend & Contract Testing

- **Contract Tests (`frontend/src/api/freshness.contract.test.ts`)**:
  - Vitest runs both unit staleness tests and live API schema validation.
  - In CI containers without the backend daemon running, tests MUST NOT hang.
  - Uses an `AbortController` (500ms timeout) in `beforeAll` to detect whether `localhost:8080` is reachable. If unreachable, live network tests are gracefully skipped while contract assertions and timestamp math tests run.
- **Node Toolchain**:
  - Configured for Node 22+ (local runtime is Node 26).
  - Build command: `cd frontend && npm run build` (tsc + vite).

---

## 6. GitHub Actions Workflows (Mecris / Moon Phase Alignment)

- **Node 20 Deprecation Notice**:
  - Runner images warn on Node 20 actions. Eliminate notices with the top-level env:
    ```yaml
    env:
      FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: 'true'
    ```
- **Pinned Action Versions & Runners**:
  - Runners: Pin `runs-on: ubuntu-24.04` to eliminate `ubuntu-latest` migration notice annotations.
  - Every `uses:` is pinned to a **full commit SHA**, with the version it was tagged as in a
    trailing comment (since 2026-11-17, #146). Never add or bump an action by tag or branch:
    resolve the tag to its commit (`git ls-remote https://github.com/<owner>/<repo> 'refs/tags/<tag>^{}'`)
    and pin that. Versions in use:
    - Checkout: `actions/checkout` v7
    - Upload Artifacts: `actions/upload-artifact` v7
    - Download Artifacts: `actions/download-artifact` v8
    - Setup Node: `actions/setup-node` v7 (with `node-version: 22`)
    - Rust toolchain: `dtolnay/rust-toolchain` (pinned commit of its `stable` branch, `toolchain: stable`)
    - Rust cache: `Swatinem/rust-cache` v2
    - GitHub Release: `softprops/action-gh-release` v3
  - `ci.yml` declares `permissions: contents: read` at the top; only `create-release` asks for more.
- **Release Automation**:
  - The `create-release` job runs automatically on tag pushes (`v*`). The tag must be a plain
    version (`vX.Y.Z`); any other tag name stops the job.
  - Parses `CHANGELOG.md` via `awk` to extract version-specific release notes.
  - Attaches 4 artifacts:
    1. `nocknock_brain.wasm`
    2. `nocknock-ingestion-worker` (Linux x86_64)
    3. `nocknock-mcp` (Linux x86_64)
    4. `nocknock-frontend.tar.gz`

---

## 7. Local Tmux Background Orchestration

- The project uses persistent tmux sessions for local running:
  - `make tmux` starts session `nocknock`:
    - Window 0: `ingestion-worker` daemon on `:8080`
    - Window 1: `frontend` Vite dev server on `:3000`
  - `make tmux-status`: Verify session is alive.
  - `make tmux-attach`: Attach to interactive session (detach with `Ctrl-b d`).
  - `make tmux-stop`: Gracefully terminate both processes.

---

## 8. Android Toolchain Rule

- When working with Android subprojects (e.g. `moon-phase-clock` edge integration):
  - Always use the Android Studio bundled JDK:
    ```bash
    export PATH="/Applications/Android Studio.app/Contents/jbr/Contents/Home/bin:$PATH"
    ```

---

## 9. Pre-Flight Verification Checklist

Before pushing any PR or tag:
```bash
make test-brain       # 3 tests (WASM invariants)
make test-worker      # 468 tests as of v0.8.3: 440 worker + 28 MCP (decoding, advice, plan, LP sizing, Aero pools, surfaces, harvest ledger, collectible rewards, closed positions, burn-down #130-#137, gas-side ranges #151-#153, harvest principal #156-#157, standing reserve #162)
make test-frontend    # 357 tests as of v0.8.3 (staleness, contract, display helpers, charts, surfaces, ledger, collectible rewards, closed positions, burn-down, NOCK-leg brackets, ETH/USDC brackets, reserve)
make dist             # Verifies packaging of dist/ binaries and tarballs
NOCKNOCK_API_URL=http://chudmax:8081 make test-mcp   # both MCP transports against the deployment
```
Counts only go up; a lower number means tests were lost. Run the suite and let it finish
before committing: never chain test, commit and push so that a failure cannot stop the push.

---

## 10. Long-Term Architectural Vision & Roadmaps

- For the philosophy behind the numbers (four pillars, gas is not cash, the wallet is the
  record, building in public), read the dated lore in [`docs/lore/`](docs/lore/), starting
  with [`2026-11-14-four-pillars-no-hurry.md`](docs/lore/2026-11-14-four-pillars-no-hurry.md).
  The enforceable rules are in [`OPERATOR_GUIDANCE.md`](OPERATOR_GUIDANCE.md) §7.
- [`HANDOFF.md`](HANDOFF.md) is a historical dossier (v0.2.1, 2026-10-12). Use it only for
  the long-range ideas it records:
  - Multi-tenant parameterization (custom operator profiles, flexible floors, dynamic tranches).
  - Urbit native architecture (`%nocknock` desk, `sur/nocknock.hoon`, `app/nocknock.hoon`).
  - Ames P2P sovereign syndicates and collective liquidity defense protocols.

---

## 11. Issue Triage, Label Taxonomy & Patch Cadence

- **SemVer Cadence Rule**:
  - **Features wait for minor versions (`v0.2.0`)**: Do not bundle new protocol features into patch releases.
  - **Patches (`v0.1.1`, `v0.1.2`, ...) ship continuously**: Release patches indefinitely until all confirmed defects and data-integrity gaps are closed.
- **The "Mock Data is a Bug" Invariant**:
  - In a sovereign state room, synthetic numbers, silent mock fallbacks, or static percentages in UI are **high-severity data-integrity bugs**, not cosmetic issues.
  - Never present mock data as live on-chain truth. If an RPC query fails, fail explicitly or set `degraded: true`.
- **Pre-Branching Triage Rule**:
  - Before writing fix code, every defect must be reported on GitHub with:
    - Root cause analysis with `file:line` references.
    - Reproduction steps and on-chain evidence (contract addresses, selectors, transaction hashes).
    - Labeling: `bug`, component (`frontend` or `backend`), severity (`data-integrity`), and target milestone (`patch-0.1.x`).
  - Refer to [`TRIAGE.md`](TRIAGE.md) for the active bug registry and triage taxonomy.

---

## 12. Operator Guidance & Standing Orders Reference

- Comprehensive operator guidance is compiled in [`OPERATOR_GUIDANCE.md`](OPERATOR_GUIDANCE.md).
- Agents must review `OPERATOR_GUIDANCE.md` for standing orders regarding:
  - Branch protection and explicit operator authorization before tagging releases.
  - The `unset GITHUB_TOKEN` elevated credential protocol.
  - Zero mock data invariants and configuration externalization.
  - On-chain reality: single sovereign wallet, 60/40 virtual sub-ledger, no custom wallet development.
  - Model Context Protocol (MCP) installation, activation, and playtesting requirements.

---

## 13. On-Prem Deployment: `chudmax` Is Infrastructure

Since 2026-11-13 NockNock runs continuously on the internal host `chudmax` (x86_64,
Ubuntu, LAN only). It is infrastructure, not production: the operator reads it to decide
what to do, so keep it running and honest, but its database is disposable. The wallet
and the chain are the record. If the database is lost, the recovery is a fresh start
(empty database, latest release, the wallet and `config.yaml`). Plan progress can be
restored by setting `accumulation_plan.starts_at` to the plan's real start and re-recording
each buy's transaction hash (receipts are re-verified on-chain); the high-water mark and
charts restart. See
[`docs/lore/2026-11-14-four-pillars-no-hurry.md`](docs/lore/2026-11-14-four-pillars-no-hurry.md).

- **Shape**: rootless Podman quadlets as systemd user units (`nocknock-db`,
  `nocknock-worker`, `nocknock-frontend`), lingering enabled. Dashboard
  `http://chudmax:3000`, API and HTTP MCP `http://chudmax:8081`. PostgreSQL is not
  published. Host port 8080 belongs to Strata: never take it.
- **Deploy**: `deploy/podman/deploy.sh chudmax` from a clean checkout of the commit you
  mean to run. It ships the working tree (minus gitignored files), builds on the host,
  and restarts. After a release, deploy the tagged commit so the host matches the tag.
- **Deploys are forward-only**: sqlx refuses to start a build that does not embed a
  migration the database already has (`Failed to run migrations: VersionMissing(<version>)`,
  then a restart every 5 seconds). So do not playtest a branch that carries a newer
  migration on chudmax and then redeploy an older commit or tag. To go back anyway: redeploy
  the newer build, or delete the newer rows from `_sqlx_migrations` in the `nocknock-db`
  container (the migrations are additive, so older code runs on the newer schema), or start
  from an empty database. `deploy.sh` does not ship nested git worktrees (#128).
  The newest is `20261121000013_tracked_positions` (v0.9.0: `tracked_positions`,
  `position_candidates`, `position_detection_state`). It adds a second reason not to go
  back: a build older than v0.9.0 does not read `tracked_positions`, so once a position is
  tracked from the chain an older build publishes a total WITHOUT it and says nothing.
  Before running one, copy each row's `config_entry` from `GET /v1/positions/found` into
  `positions:` in `config.yaml` (DEPLOYMENT.md section 5).
- **Config and secrets**: the operator's `ingestion-worker/config.yaml` on the
  workstation is the source of truth (gitignored; it holds the Alchemy key, the
  accumulation plan and the operator's limits). `deploy.sh` stores it as the Podman
  secret `nocknock-config`. Back it up before editing; never commit it.
- **No backups**: the nightly `pg_dump` timer was removed on 2026-11-14 (operator
  decision, issue #96 closed as not planned). The one dump it left in
  `~/nocknock/backups` on chudmax is untouched. Do not reinstall backup tooling.
- **Privileges**: `sudo` on chudmax needs the operator's password. Ask them to run it.
- **Verify after every deploy**: `/health` is `ok` with `last_error: null`; the snapshot
  version and spot look right; `NOCKNOCK_API_URL=http://chudmax:8081 make test-mcp`.
- **Playtests on this host** may use throwaway accumulation plans, but label them
  plainly, remove their ledger rows afterwards, and restore the operator's config.
- **Do not** leave it running an unmerged branch at the end of a session without saying
  so in the session log.
- **Opt-in TLS proxy (#89)**: `NOCKNOCK_TLS=1 deploy/podman/deploy.sh chudmax` adds the
  `nocknock-tls` quadlet (Caddy, private CA, `https://chudmax:3443`). A deploy without
  that variable never installs, restarts or removes it. Its `nocknock-tls-data` volume
  holds the CA's root key: do not delete it casually (every phone would need the new
  root) and never copy the key off the host. See `DEPLOYMENT.md` §7.
- Other hosts (`netmoom`, `metnoom`) reject the workstation's SSH key; nothing runs there.

---

## 14. Outside Contributions

The first pull request from outside arrived on 2026-11-17 (#141). The audit it prompted is
[`docs/kb/ci-security-audit-2026-11-17.md`](docs/kb/ci-security-audit-2026-11-17.md); read
it before touching an outside pull request.

- **Every outside pull request waits for approval before CI runs, every time** (repository
  setting `all_external_contributors`). Approving the run, commenting on the pull request
  and merging it are the operator's decisions: an agent does none of them without the
  operator's word for that pull request, and an agent's read does not count as a review.
- **Read before running.** Read the whole diff before the CI run is approved and again
  before merging. Never run a fork's code on the workstation or on chudmax before reading
  it. Read every changed line of `Makefile`, `deploy/`, `.github/`, lockfiles, `build.rs`
  and scripts; a lockfile change needs a stated reason.
- **Verify, do not trust.** A pull request's description is a claim. Reproduce the defect
  and test the fix.
- **Be kind and curious.** The project is AI-forward and does not reject a contribution for
  being automated. Say what the evidence shows about who sent it, and ask how they found
  the project.
