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

## 4. The "Mock Data is a Bug" Invariant

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
   - **v0.2.0 pairing runbook**: open the dashboard, click PAIR in the header badge (or open `/?view=lite`), scan the QR / accept the deep link in the Uniswap app, and grant read-only + suggest-transactions permissions. Payloads come from `GET /v1/execute/claim-fees/payload` (simulate-first); the wallet signs with your thumbprint. If a transaction shows approved but nothing lands, the Uniswap app is holding it unbroadcast — open the Uniswap app to broadcast; the cockpit records the receipt automatically once it lands (up to ~10 min). Requires `VITE_WALLETCONNECT_PROJECT_ID` at frontend build time (free Reown project id).
4. **Traded Assets & CPA Tie-Out**:
   - The treasury only actively trades NOCK and sometimes ETH/WETH on Base.
   - All transactions tie out to CPA records via Koinly tax reporting.
   - AERO claimed from a gauge is booked by the harvest ledger when the claim's transaction is recorded (v0.6.0, #93): valued at the claim's block from the AERO price pool, or with no USD value when that price cannot be read. The export describes it as an AERO reward claim and keeps the `income` label; a different label is the operator's and their CPA's call. With a `tax_reserve_ratio` set, a priced AERO row counts toward the virtual tax reserve like any other harvest row.
5. **LP Rebuild Bias (standing order, 2026-11-14)**:
   - "Rebuilding the tranches has basically no net cash position change so we should always act with a bias towards doing that."
   - When LP is under its target weight, advice must surface `REBUILD LP TRANCHES`: as the primary directive whenever the engine would otherwise be idle, and appended to every other directive. It must never displace a due DCA tranche or a safety directive.
6. **NOCK Buys Are Dollar-Cost Averaged**:
   - Purchases are paced by the `accumulation_plan` (budget, tranches and cadence are the operator's call; never invent defaults). Where to buy is the operator's call too: `accumulation_plan.venue` is an optional label with no default, shown as "your plan's venue label"; unset, the advice names no venue (v0.8.1, #132). Only receipt-verified buys the operator has confirmed advance the plan.
   - **A cycle that cannot read the ledger (v0.8.1, #130)** reports the plan's progress as unknown (`DCA BLOCKED (ledger unreadable)`, `progress_unknown: true`, amounts "unknown"), never as $0.00 or "1 of N". That cycle pushes no change of the plan's own part of the directive; a change outside the plan (a floor lock, say) is still pushed. The next good cycle is compared with what the operator was last told, so the return to a good read is not news.
   - **Recording a buy (v0.6.0, issue #118)**: NockNock reads the wallet's incoming NOCK transfers each cycle and offers every swap that passes the recording rules in the Accumulation Plan panel. CONFIRM records it (the worker fetches and verifies the receipt again; there is no second write path). NOT PART OF THE PLAN leaves it uncounted; it stays listed and can be restored. A detected swap is never recorded automatically, and NockNock still signs and sends nothing. Pasting the transaction hash (`POST /v1/execute/record-accumulation`) works whether or not detection ran (it needs the plan, the RPC key and a readable receipt, as before), and is the fallback when the panel says it could not check. Swaps from before the plan anchor are not offered. `fill_detection.enabled: false` turns the check off.
   - **Where a due tranche fills better (v0.7.0, issue #115)**: while a tranche is due, NockNock asks Aero's and Uniswap's own quoter contracts what that exact tranche amount returns in NOCK, at one block, and shows the two venues side by side in the Accumulation Plan panel and in `get_accumulation_plan`: NOCK out, price, cost over the snapshot's reference spot, the route with each pool and fee, and the quote's block and time. It states which venue returns more NOCK as a fact and links each venue's own swap page (`app.aero.xyz/swap`, `app.uniswap.org/swap`) with nothing prefilled. It is a comparison of execution routes for a buy the plan already sized, not a recommendation to buy: NockNock quotes no other amount, builds no transaction, and the wallet app's own router may pick a different route. A venue that cannot be quoted reads "unavailable" with the reason, never an estimate. REFRESH QUOTES (`POST /v1/accumulation/quotes/refresh`) reads again at the latest block, at most once every 10 seconds; inside that window it answers with the quotes already held (v0.8.1, #133). Each recorded buy then shows its cost over the best quote read for its tranche within the 60 minutes before the buy's block, with that quote's block, or "no quote on record". Once the plan has a recorded buy, stored quotes read before it that no buy can use any more are deleted (each swap still on offer or dismissed keeps the quote read just before it); a buy below the latest recorded one that fill detection never offered, recorded by hash later, reads "no quote on record". Quotes read after the latest recorded buy (or before the first) are kept for 24 hours, plus the newest one and the one before each swap on offer or dismissed, so a tranche that stays due for days does not grow the table without bound; a buy fill detection never offered, recorded by hash more than 24 hours after it, reads "no quote on record". `route_quotes.enabled: false` turns the quoter calls off.
7. **Protect the Cash Position (standing order, 2026-11-14)**:
   - "Let's not dilute the cash position overtly — we have lots of cash, we want to keep it. There might be a dip!"
   - Plans carry a `cash_reserve_usdc` floor the engine never advises spending below, at any price. The default 5% cash target is a long-run weight, not permission to spend down to it. Only the operator lowers the reserve.
   - The reserve counts USDC inside LP positions as well as wallet USDC (operator decision, 2026-11-14), consistent with the LP-rebuild rule above. Since #151 only USDC inside NOCK-pair ranges counts: USDC in a gas-side ETH/USDC range turns into WETH as ETH falls, so it is no NOCK reserve. The USDC pillar (`total_stable_usd`) still counts every USDC held.
8. **LP Rebuild Rules (standing orders, 2026-11-14)**:
   - Split the LP target equally across eligible tracked positions unless config sets weights.
   - The LP target is NOCK LP (#151). A tracked range with no NOCK leg (the operator's gas-side WETH/USDC ranges, opened 2026-11-17) is measured, never steered: it is in valuation and every total, and appears as a no-target "ETH/USDC LP (measured)" row ("Non-NOCK LP (measured)" unless every such range is an ETH-or-WETH / USDC pair), but it never fills or silences the NOCK LP target, is never a rebuild template or exclusion, never makes the engine label "active", is not in plan headroom or `positions_nock`, and is not drawn on the NOCK bracket chart. One predicate decides: token0 or token1 is NOCK (`is_nock_pair` / `hasNockLeg`).
   - "We should not generally aim to fill positions that are out of range." Out-of-range positions are skipped unless config makes an explicit exception.
   - Never suggest a deposit into a pool with no in-range liquidity.
   - Aero tranches live at app.aero.xyz ("We shouldn't refer folks to aerodrome.finance anymore"). Link there, never to the retired Aerodrome app. Staking is left at the Aero app's default. AERO is the fourth pillar (7.11): neither minimized nor maximized. Since v0.6.0 NockNock reads it (#93): the wallet's AERO, and the AERO each staked position has earned in its gauge and not claimed. A value it cannot read is "not read" or "not reported" with the reason, never zero. It states amounts and must not advise claiming, selling, holding or accumulating AERO; what to do with claimed AERO is the operator's decision and no policy for it is built in. A staked position's unclaimed AERO is its earnings, so it is reported as that position's collectible earnings (#142; operator, 2026-11-17: "can we make sure this tranche reports its earnings as collectible too?"): the amount, its USD value and the gauge it is collected from, in the Aero app. "Collectible" states a fact; words such as "should", "worth claiming" or "claim now" must never attach to an AERO reward, and the LP-fee harvest words (claim_now, claim_recommended, accumulate) never label one. LP fees, AERO rewards and their sum are three labelled figures, never one merged number. BATCH HARVEST and every other threshold count LP fees only: no rule for rewards has been set, and setting one is the operator's decision. NockNock builds no transaction for a reward, and prints a gas estimate for collecting one only where the claim call is verified on-chain ([`ONCHAIN_INTEGRATION.md`](ONCHAIN_INTEGRATION.md) §9).
   - Aero concentrated (CL) positions are tracked since v0.6.0 with `protocol: aerodrome-cl` (#91), once listed under `positions:`; the WETH/NOCK CL200 position (NFT #2914490) was invisible before that. A CL position is ranged: it can go out of range. Staked in its gauge, it earns no swap fees for the LP (they go to the gauge), so NockNock shows "not LP-claimable" and never $0 earned. Its WETH is gas-pillar ETH, so it is never offered a rebuild deposit; no Aero CL position is, whatever its pair, because the Aero app's deposit link starts a new position (another NFT, which NockNock would not track) and adding to an existing NFT is not verified. A position the wallet neither holds nor staked fails the cycle; it is never counted on someone else's behalf. So does a `pool_address` that is not the NFT's own pool, and an unstaked position whose fees cannot be read (its `collect` simulation reverts): an unread fee is never published as 0.
   - A tracked NFT position that the operator withdraws is reported as **closed**, and ingestion goes on (#144; operator, 2026-11-17: "fix it so 'closed' is understood"). Withdrawing in the wallet app burns the NFT; on 2026-11-17 that happened to NFT #2914490 and every cycle failed until its config entry was commented out. Rules: (a) only proof counts. A position is closed when two reads of the same cycle, made at one current block (at most 120 seconds old by the host's clock), both give the position manager's own answer for a token id that does not exist ([`ONCHAIN_INTEGRATION.md`](ONCHAIN_INTEGRATION.md) §13). A node that does not answer, an HTTP error, a rate limit, any other revert, one read disagreeing with the other, or a node that is behind the chain (an id that is not minted yet answers like a burned one) fails the cycle as before: wrongly calling a live position closed would silently remove real money from the total, which is worse than a stalled dashboard. (b) A closed position is never shown as a live position holding $0, its last known value is never shown as current, and nothing of it is in any total, pillar, LP weight, rebuild suggestion, claim action or reward figure. It is listed apart, with the fact, "not counted in any total", and that its config entry can be removed, on the dashboard, the Lite cockpit and all four MCP tools. (c) A mistyped `nft_id` reads exactly like a burned one, so the text always says both. An NFT that still exists with no liquidity is not closed. (d) If the closed entry is the price source, no price is guessed: a Uniswap v4 entry's pool keeps supplying it (the same reads as before) until another position is flagged, and no text may say that entry "can be removed" (removing it first leaves no price source); a Uniswap v3 or Aero CL entry fails the cycle and asks for another `is_price_source`. (e) One notification when a live position closes; never a directive about it. Finding positions on-chain instead of listing NFT ids in config was raised by the operator the same day ("as a long-term strategy it seems fraught, especially if the price begins to move permanently outside of the original range") and is not built: until it is, a new position must still be added to `positions:` by hand.
   - LP may hold slightly more NOCK than sits above the hard floor. Operator, 2026-11-14: "Because the LP sells a fraction every time the price moves, by putting in just a little bit more than what we have allowed as a trading limit we will actually be quite unlikely to sell it all unless there is a super rally, in which case we'll still have most of the hundred thousand... We need to trade more." The overage is bounded by `operator_profile.lp_floor_tolerance_bps`, and the engine must state the worst case rather than hide it. Do not re-impose a hard cap at the floor.
9. **Agents May Improve the Strategy (2026-11-14)**:
   - "I am counting on you to make the strategy good — the strategy that we wrote may not be good. You are allowed to suggest adjustments, and even build them into the code."
   - This covers the soundness of the rules: internal consistency, guardrails, measurement, and removing ways the engine can contradict its own invariants. State each adjustment and its reason plainly. It does not extend to choosing the operator's allocation (how much to hold, deploy or reserve): those numbers stay the operator's decision.
10. **No Hallucinations / Urbit Lore**:
   - Never hallucinate fake Urbit `@p` (e.g. `~urmanac` is invalid Hoon syntax). Stick strictly to verifiable protocol and mathematical facts.
11. **Four Pillars (standing order, 2026-11-14)**:
   - The wallet holds four things: NOCK, cash (USDC), gas (ETH) and AERO. "We neither minimize nor maximize the importance of the pillars. They just are. If they grow, that's good."
   - The four add up to the total: NOCK (wallet plus LP), USDC (wallet plus LP), ETH (wallet gas plus LP WETH) and wallet AERO (since v0.6.0, #93). AERO has no target and no deviation label other than `no_target`. AERO still unclaimed in a gauge is not in the total, exactly as uncollected LP fees are not; it is shown beside them, as collectible earnings (#142), and counts once it is claimed. AERO here is the token the Aero gauges pay, `0x3f5bF3EdAc59d650D8fDf9657a71F2f904A3a4Ee`; the older "Aerodrome" token that shares the symbol is not read ([`ONCHAIN_INTEGRATION.md`](ONCHAIN_INTEGRATION.md) §9).
   - No pillar is the point of the others, and NockNock puts no rebalancing pressure on any of them. LP positions are where the pillars work together, not a fifth pillar. Reasons: [`docs/lore/2026-11-14-four-pillars-no-hurry.md`](docs/lore/2026-11-14-four-pillars-no-hurry.md).
   - The deviation matrix's under/over-funded labels measure against the operator's own targets. They are not instructions.
12. **Gas Is Not Cash (standing order, 2026-11-14)**:
   - ETH (and WETH held as gas) pays for transactions. It is counted as the gas pillar, never as cash: the cash reserve is wallet USDC plus USDC inside NOCK-pair LP (#151), never ETH.
   - NockNock never advises spending gas on NOCK, and wallet ETH/WETH does not fund LP rebuild suggestions (#94).
   - NockNock suggests no amount of ETH to hold. With no ETH it says once that every action on Base pays gas in ETH.
13. **The Wallet Is the Record (operator decision, 2026-11-14)**:
   - Balances, positions, fees and recorded buys are read from the chain. NockNock's database (high-water mark, charts, ledgers) is disposable: a convenience, not the record.
   - There are no backups (issue #96 closed as not planned). Do not add backup tooling, and do not call any dump a backup.
   - Recovery is a fresh start: an empty database, the latest release, the wallet and `config.yaml`. Plan progress can be restored by setting `accumulation_plan.starts_at` to the plan's real start: from v0.6.0 NockNock then finds every buy since that time and offers it again for confirmation, or each buy's transaction hash can be recorded by hand (`POST /v1/execute/record-accumulation`). Either way NockNock re-verifies each receipt on-chain. Harvests re-record the same way. The high-water mark and charts restart. If `config.yaml` itself is lost, the positions are listed again by hand.
14. **Building in Public (standing order, 2026-11-14)**:
   - "We're building in public, which means we are always at risk of serving as an example to someone else." The rules for a newcomer are the rules for the operator.
   - NockNock reads the wallet and states facts. It never chooses a user's numbers: plan budget, tranches and interval, NOCK floor, target weights, cash reserve, tax-reserve ratio. Unset means no rule, never someone else's number filled in.
   - The operator's own numbers in docs and examples are labelled as one operator's, never as recommendations or defaults.

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
   - **STDIO**: Standard stdio JSON-RPC for Claude Desktop, Antigravity CLI, and Pi Harness (`pi-mcp-adapter`) subprocess invocation. Configured via repository `.mcp.json` or global `~/.gemini/config/mcp_config.json` / `~/.config/mcp/mcp.json`. Does NOT need to run in a persistent tmux session.
   - **Stateless HTTP**: Expose `/v1/mcp` on the Axum ingestion worker supporting MCP JSON-RPC over HTTP POST with optional Bearer token authentication matching required server permissions.
3. **Agent Integration Profiles**:
   - **Antigravity CLI**: Configured in `~/.gemini/config/mcp_config.json` pointing to `dist/nocknock-mcp`.
   - **Pi Harness**: Uses `npm:pi-mcp-adapter` (Pi 0.99+) with `.mcp.json` and user-global `~/.config/mcp/mcp.json` (`projectServers: "allow"`). Verify with `pi-mcp-adapter doctor`.

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
make test-worker      # 426 tests as of v0.8.1 (399 worker + 27 MCP)
make test-frontend    # 338 tests as of v0.8.1
make dist             # Verifies packaging of dist/ binaries and tarballs
```
