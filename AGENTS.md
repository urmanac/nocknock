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
  `rpc`, `evm`, `calldata`, `contracts`, `notify`, `mcp_text`, `mcp_http`, and `tests`.
  Moved items are `pub(crate)`. `testdata/golden_snapshot.json` pins the snapshot bytes: a
  refactor must leave it untouched; an intended behaviour change regenerates it with
  `NOCKNOCK_BLESS_GOLDEN=1 cargo test test_golden_snapshot` and the diff is reviewed.
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
- **Pinned Modern Action Versions & Runners**:
  - Runners: Pin `runs-on: ubuntu-24.04` to eliminate `ubuntu-latest` migration notice annotations.
  - Checkout: `actions/checkout@v7`
  - Upload Artifacts: `actions/upload-artifact@v7`
  - Download Artifacts: `actions/download-artifact@v8`
  - Setup Node: `actions/setup-node@v7` (with `node-version: 22`)
  - GitHub Release: `softprops/action-gh-release@v3`
- **Release Automation**:
  - The `create-release` job runs automatically on tag pushes (`v*`).
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
make test-worker      # 145 tests as of #97: 137 worker + 8 MCP (decoding, advice, plan, LP sizing, Aero pools, surfaces, harvest ledger)
make test-frontend    # 107 tests as of v0.5.3 (staleness, contract, display helpers, charts, surfaces, ledger)
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
- Other hosts (`netmoom`, `metnoom`) reject the workstation's SSH key; nothing runs there.

