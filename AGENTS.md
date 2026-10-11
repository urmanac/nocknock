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
- **Pinned Modern Action Versions**:
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
make test-brain       # 3 tests passing (WASM invariants)
make test-worker      # 6 tests passing (decoding, spot math, advice)
make test-frontend    # 12 tests passing (staleness & contract)
make dist             # Verifies packaging of dist/ binaries and tarballs
```
