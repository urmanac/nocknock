# NockNock Release & Deployment Runbook

This document details the development lifecycle, branch protection rules, testing requirements, and automated release pipeline for **NockNock**.

---

## 1. Branch Protection & Contribution Policy

The repository operates under strict branch protection on the `main` branch:
- **No direct pushes to `main`**: All code must land via a Pull Request.
- **No force pushes or branch deletions**: History on `main` is immutable.
- **PR-driven workflow**: Every feature, chore, or bugfix must be committed to a topic branch, pushed to the remote, and opened as a PR.
- **GitHub CLI elevated credentials**: If using GitHub CLI (`gh`) in local or automated environments where a scoped default token is set in the environment, prepend `unset GITHUB_TOKEN` to ensure the elevated administrative credentials from the keychain are utilized:
  ```bash
  unset GITHUB_TOKEN && gh pr create --fill
  unset GITHUB_TOKEN && gh pr merge <PR-NUMBER> --merge
  ```

---

## 2. Pre-Release Verification Protocol

Before merging to `main` or creating a release tag, verify all test suites locally:

### 2.1 WASM Brain Tests
```bash
make test-brain
```
Ensures pure deterministic logic, invariant preservation, and unit tests pass without host dependencies.

### 2.2 Ingestion Worker & MCP Tests
```bash
make test-worker
```
Runs the Rust test suite covering:
- Parsing Aerodrome `claimFees` return data buffers.
- Dynamic on-chain ETH/USD reserve queries.
- Tactical advice matrix (Batch Harvesting, Defensive Floor Lock, Hold & Accrete).
- Deterministic SHA-256 state hashing.

### 2.3 Frontend Contract & Web Tests
```bash
make test-frontend
```
Executes contract validation tests against the API schema with graceful fallback if the live daemon is offline.

### 2.4 Artifact Packaging Smoke Test
```bash
make dist
```
Builds release binaries and verifies packaging:
- `dist/nocknock_brain.wasm`
- `dist/nocknock-ingestion-worker`
- `dist/nocknock-mcp`
- `dist/nocknock-frontend.tar.gz`

---

## 3. Pull Request & CI Verification

1. **Create Topic Branch**:
   ```bash
   git checkout -b feature/my-new-feature
   ```

2. **Commit Changes**:
   Ensure commits follow conventional commit standards (`feat:`, `fix:`, `chore:`, `docs:`).

3. **Push Topic Branch**:
   ```bash
   git push -u origin feature/my-new-feature
   ```

4. **Open Pull Request**:
   ```bash
   unset GITHUB_TOKEN && gh pr create --title "feat: descriptive title" --body "Summary of changes"
   ```

5. **Observe CI Execution**:
   CI runs on every PR and verifies:
   - `Build & Test WASM Brain` (`wasm32-wasip1`)
   - `Build & Test Ingestion Worker` (Rust Linux x86_64, `SQLX_OFFLINE=true`)
   - `Build & Test Frontend Portal` (Node.js 22, Vite bundle)

6. **Merge PR**:
   Once all checks pass green:
   ```bash
   unset GITHUB_TOKEN && gh pr merge <PR-NUMBER> --merge
   ```

7. **Synchronize Local Main**:
   ```bash
   git checkout main
   git pull origin main
   ```

---

## 4. Semantic Versioning, Patch Cadence & Triage Gate

NockNock adheres strictly to **Semantic Versioning (SemVer 2.0.0)**:
- **Patch Releases (`v0.1.x`)**: Reserved exclusively for bugfixes, UI reactivity fixes, RPC resilience, and data integrity repairs.
  - **Core Tenet**: *Mock data in the production app is a bug.* Any hardcoded placeholder, static string pretending to be live math, or silent mock fallback must be triaged and resolved in a patch release.
  - Patch releases (`v0.1.1`, `v0.1.2`, ...) can and should be shipped indefinitely until every active bug and data anomaly is eliminated.
- **Minor Releases (`v0.2.0`)**: Reserved for backward-compatible architectural features, including:
  - Dynamic operator profile configuration (Issue #6).
  - Multi-tenant tranche allocations and customizable defensive floors.
  - Sovereign Urbit integration (`%nocknock` desk, `sur/nocknock.hoon`).
- **Major Releases (`v1.0.0`)**: Sovereign syndicate consensus and collective liquidity defense protocols.

### 4.1 The Pre-Tag Evaluation Gate
Merging a PR into `main` does **not** automatically trigger a release:
1. Land bugfix PRs onto `main` to unlock CI caching and incremental stability.
2. Review the live triage board (see [`TRIAGE.md`](file:///Users/yebyen/w/nocknock/TRIAGE.md) and `unset GITHUB_TOKEN && gh issue list --label patch-0.1.x`).
3. Re-evaluate readiness with the operator: do not tag until the planned batch of patch fixes is confirmed green and visually validated.

---

## 5. Tagging & Automated GitHub Release

Releases are triggered automatically when an annotated Git tag matching `v*` (e.g. `v0.1.0`, `v0.1.1`) is pushed to `origin`.

### 5.1 Update Changelog
Ensure [`CHANGELOG.md`](file:///Users/yebyen/w/nocknock/CHANGELOG.md) contains a version section corresponding to the target release tag:
```markdown
## [0.1.1] - 2026-10-11
### Fixed
- Detail bugfixes...
```
The automated GitHub Actions release job parses this exact section from `CHANGELOG.md` using `awk` to populate the release notes body automatically.

### 5.2 Create and Push Annotated Tag
Only once human evaluation confirms release readiness from an up-to-date, green `main` branch:
```bash
git tag -a v0.1.1 -m "Release v0.1.1"
git push origin v0.1.1
```

### 5.3 Automated GitHub Actions Pipeline
Once pushed, GitHub Actions triggers the `NockNock CI & Release` workflow:
1. Concurrently builds and tests `nocknock_brain.wasm`, `nocknock-ingestion-worker`, `nocknock-mcp`, and the frontend static bundle.
2. Uploads build artifacts to workflow storage using `actions/upload-artifact@v7`.
3. Triggers the `publish-release` job upon successful completion of all three build jobs.
4. Downloads all assets via `actions/download-artifact@v8`.
5. Extracts release notes for `v0.1.1` from `CHANGELOG.md`.
6. Publishes an official GitHub Release via `softprops/action-gh-release@v3` containing:
   - `nocknock_brain.wasm`
   - `nocknock-ingestion-worker` (Linux x86_64)
   - `nocknock-mcp` (Model Context Protocol binary)
   - `nocknock-frontend.tar.gz` (Static production web bundle)

### 5.4 Verification
Monitor the release workflow in real-time:
```bash
unset GITHUB_TOKEN && gh run list --workflow="NockNock CI & Release"
unset GITHUB_TOKEN && gh release view v0.1.1
```
Verify all four release assets are present and downloadable.

---

## 6. Rollback & Emergency Tag Pruning

If a release tag is pushed erroneously before all checks or changelogs are finalized:
1. Delete the local tag:
   ```bash
   git tag -d v0.1.0
   ```
2. Delete the remote tag (tags are not subject to `main` branch protection):
   ```bash
   git push --delete origin v0.1.0
   ```
3. If a draft or published release was created on GitHub, remove it:
   ```bash
   unset GITHUB_TOKEN && gh release delete v0.1.0 --yes
   ```
