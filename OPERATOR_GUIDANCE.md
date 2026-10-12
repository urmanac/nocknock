# NockNock Standing Orders & Operator Guidance (`OPERATOR_GUIDANCE.md`)

> **"Things I've Told You Before"** — A persistent compendium of tribal knowledge, operational rules, domain constraints, and architectural advice accumulated across pair-programming sessions with the Operator.
>
> Every autonomous agent operating on the NockNock repository **must review and obey** these standing orders before planning, testing, or committing code.

---

## 1. Non-Negotiable Branch Protection & Git Protocols

1. **`main` is strictly protected**:
   - Remote GitHub protection rules reject all direct pushes to `main`.
   - Force-pushes (`git push -f`) and branch deletions on `main` are impossible and forbidden.
2. **All changes land via Pull Requests**:
   - Always branch off clean, up-to-date `main`: `git checkout -b <branch-name>`
   - Push branch: `git push -u origin <branch-name>`
   - Create PR: `unset GITHUB_TOKEN && gh pr create --fill`
   - Merge PR: `unset GITHUB_TOKEN && gh pr merge <PR_NUMBER> --merge`
   - Sync local `main`: `git checkout main && git pull origin main`
3. **Release Tag Safety Gate**:
   - Git tags are NOT branch-protected on GitHub; they can be created, pushed, and deleted.
   - **Never tag or push a release without explicit human operator authorization**. Merging a PR or seeing CI turn green on `main` is NOT an automatic sign to tag a release.
   - If a tag is pushed prematurely, delete it immediately from both local and remote:
     ```bash
     git tag -d vX.Y.Z
     git push --delete origin vX.Y.Z
     unset GITHUB_TOKEN && gh release delete vX.Y.Z --yes
     ```

---

## 2. GitHub CLI Environment & Authentication Quirk

- **The Inherited `GITHUB_TOKEN` Quirk**:
  - Subshells and automated runner processes frequently inherit a scoped or restricted `GITHUB_TOKEN` that lacks write/PR/release permissions on `urmanac/nocknock`.
  - **Standing Order**: ALWAYS prepend `unset GITHUB_TOKEN` to every invocation of `gh`:
    ```bash
    unset GITHUB_TOKEN && gh pr create ...
    unset GITHUB_TOKEN && gh pr merge ...
    unset GITHUB_TOKEN && gh run watch ...
    unset GITHUB_TOKEN && gh release ...
    unset GITHUB_TOKEN && gh issue ...
    ```
  - Unsetting the variable forces `gh` to fall back to the host keychain credentials which hold repository admin privileges.

---

## 3. Strict Semantic Versioning & Release Cadence

1. **Features Wait for Minor Releases (`v0.2.0`)**:
   - Never bundle new protocol features (e.g. Ethereum AMM dynamic discovery, Coinbase API Staked ETH connector, WalletConnect 1-click execution, multi-tenant operator profiles) into patch releases.
2. **Patches Ship Continuously (`v0.1.1`, `v0.1.2`, ...)**:
   - Patch releases are strictly reserved for bug fixes, data integrity corrections, performance optimizations, and UI contrast repairs.
   - Release patches indefinitely until every confirmed bug and data-integrity gap is closed.
3. **No Urgency to Release Multiple Times per Day**:
   - "There is no urgency to release multiple times per day. You are free to take your time."
   - Thoroughly test, playtest, and re-evaluate each change before proposing a release tag.

---

## 4. The "Mock Data in Production is a Bug" Invariant

- In a sovereign state room, synthetic numbers, silent mock fallbacks, or static percentages in UI are **high-severity data-integrity bugs**, not cosmetic flaws.
- **Rules of Data Truth**:
  - Never present mock data as live on-chain truth.
  - If an RPC query fails, fail the ingestion cycle explicitly or set `degraded: true`. Never silently substitute magic fallback numbers.
  - Never hardcode static percentages (`+6.3%`, `+5.6%`) or static token amounts in UI legends. Compute all values mathematically from live snapshot data.

---

## 5. Config Externalization (Preserve Magic Numbers)

- When refactoring hardcoded constants or magic numbers (e.g. Uniswap V4 position IDs, fee baselines, liquidity scale factors, floor thresholds), **do not simply delete them**.
- Externalize them into configuration files ([`config.yaml`](file:///Users/yebyen/w/nocknock/ingestion-worker/config.yaml), [`config.yaml.example`](file:///Users/yebyen/w/nocknock/ingestion-worker/config.yaml.example)) and config structs (`AppConfig`).
- Provide sensible defaults so existing deployments continue running while allowing operators to tune parameters without code changes.

---

## 6. Pre-Branching Triage & Documentation Discipline

1. **Pre-Branching Bug Registration**:
   - Before writing fix code, every defect must be formally reported on GitHub and recorded in [`TRIAGE.md`](file:///Users/yebyen/w/nocknock/TRIAGE.md).
   - Reports must include root cause analysis with `file:line` references, on-chain evidence (contract addresses, selectors, transaction hashes), reproduction steps, and label taxonomy (`bug`, `backend`/`frontend`, `data-integrity`, `patch-0.1.x`).
2. **Tribal Knowledge Persistence**:
   - Document all workflow enhancements, environment quirks, and architectural rules in documentation files ([`AGENTS.md`](file:///Users/yebyen/w/nocknock/AGENTS.md), [`TRIAGE.md`](file:///Users/yebyen/w/nocknock/TRIAGE.md), [`RELEASE.md`](file:///Users/yebyen/w/nocknock/RELEASE.md), [`OPERATOR_GUIDANCE.md`](file:///Users/yebyen/w/nocknock/OPERATOR_GUIDANCE.md)).
   - Keep documentation updated with every PR.

---

## 7. On-Chain Reality, Wallet Architecture & Tax Accounting

1. **Single Sovereign Wallet**:
   - Tracked address on Base mainnet: `0xeE114cC97eC5adeB34bA5645e315B497D8487a45`.
   - There is **no separate Tax Escrow cold address** on-chain, and none is planned.
2. **The 60/40 Split Rule as a Virtual Sub-Ledger**:
   - Maintain the 60/40 Split Rule as an internal accounting partition within the single sovereign wallet.
   - 40% of realized fee harvests and capital gains are earmarked for virtual tax escrow buffer.
   - 60% represents liquid growth dry powder for dip accumulation.
3. **No Custom Wallet Development**:
   - NockNock does NOT build its own wallet.
   - Pair with existing mobile wallets (e.g. Uniswap mobile app on Android/iOS) via WalletConnect for biometric confirmation.
4. **Traded Assets & CPA Tie-Out**:
   - The treasury only actively trades NOCK and sometimes ETH/WETH on Base.
   - All transactions tie out to CPA records via Koinly tax reporting.
5. **No Hallucinations / Urbit Lore**:
   - Never hallucinate fake Urbit `@p` (e.g. `~urmanac` is invalid Hoon syntax). Stick strictly to verifiable protocol and mathematical facts.

---

## 8. UI Contrast, Accessibility & The State Room Dark Aesthetic

1. **High Contrast for Key Metrics**:
   - Target and Actual figures must remain clearly legible against dark/gray card backgrounds.
   - Use vibrant sky blue (`#38bdf8`) for Target % and distinct green/amber/red for Actual %; avoid dark text or low-contrast muted tones.
   - Bright axis ticks (`#cbd5e1`/`#e2e8f0`) and clear grid contrast.
2. **Recharts Tooltip Theme**:
   - Tooltips must use the State Room dark container (`#161b22`, border `#30363d`, text `#f0f6fc`), never default stark white.
3. **Dynamic Graph Domains**:
   - Never hardcode static chart Y-axis domains (e.g. `domain={[90000, 115000]}`) that clip data when balances fluctuate. Use dynamic padding while keeping reference lines (100k floor) in view.

---

## 9. Model Context Protocol (MCP) Architecture & Playtesting

1. **Never Release Untested Components**:
   - Do not release binaries or packages (such as `nocknock-mcp`) without installing, activating, and playtesting them in real agent environments. Unevaluated releases cause operational unease.
2. **Transport Mechanisms**:
   - **STDIO**: Standard stdio JSON-RPC for Claude Desktop and Antigravity CLI subprocess invocation. Does NOT need to run in a persistent tmux session.
   - **Stateless HTTP**: Expose `/v1/mcp` on the Axum ingestion worker supporting MCP JSON-RPC over HTTP POST with optional Bearer token authentication matching required server permissions.

---

## 10. Modern Toolchain & CI Hygiene

1. **Eliminate GitHub Actions Deprecations**:
   - Force Node 24 for JavaScript actions: `env: FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: 'true'`.
   - Pin runner images: `runs-on: ubuntu-24.04`.
   - Use modern action versions: `checkout@v7`, `upload-artifact@v7`, `setup-node@v7`, `action-gh-release@v3`.
2. **Build Caching**:
   - `Swatinem/rust-cache@v2` across all Rust jobs cuts CI from minutes to under 40 seconds.
3. **Android JDK**:
   - Use Android Studio bundled JDK: `export PATH="/Applications/Android Studio.app/Contents/jbr/Contents/Home/bin:$PATH"`.
4. **SQLX Offline Mode**:
   - Always build worker with `SQLX_OFFLINE=true` in CI and non-DB container environments. Run `cargo sqlx prepare` when query macros change.

---

## 11. Pre-Flight Verification Routine

Before pushing any PR or proposing a release tag, always execute the full verification battery:
```bash
make test-brain       # 3 tests passing (WASM invariants & hashing)
make test-worker      # 8 tests passing (hex decoding, spot math, advice, V4 scaling, config)
make test-frontend    # 15 tests passing (staleness, contracts, math, dynamic domain)
make dist             # Verifies packaging of dist/ binaries and tarballs
```
