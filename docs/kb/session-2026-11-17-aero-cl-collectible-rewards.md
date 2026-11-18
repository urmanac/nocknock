# Session 2026-11-17: the Aero CL position goes live, and gauge rewards become collectible earnings

*Claude: Fable 5.2 with Ultracode workflows. It follows
[session-2026-11-15-overnight-stack.md](session-2026-11-15-overnight-stack.md), the day
after v0.8.0 was released and deployed.*

## What the operator reported and asked for

- Fill detection worked on a real buy: tranche 4 was found and confirmed with one press.
- NOCK had risen almost 30% in a day; the WETH/NOCK ranges were at or past their tops.
- Add the Aero CL block to the config, and check that the assistant's own MCP connection
  to NockNock is configured.
- The WETH in the out-of-range position stays where it is: "it might count as a reserve
  tank", ready to buy if the price dips back into the range. No rule was set for it.
- "Can we make sure this tranche reports its earnings as collectible too?" (issue #142).

## What was done

- **Config:** the `aero-cl200-weth-nock` block was added to `positions:` in the operator's
  `config.yaml` at their request (backup kept outside the repo), and the v0.8.0 tag was
  redeployed to chudmax. The position read $178.43, out of range and all WETH, with
  17.65 AERO ($7.75) unclaimed: the same as the Aero app.
- **MCP from the workstation:** `.mcp.json` runs `./dist/nocknock-mcp` with no
  environment, so it read `127.0.0.1:8080`. `NOCKNOCK_API_URL` is now set in the untracked
  `.claude/settings.local.json`. `make dist` had also overwritten the binary in place,
  which macOS then kills on launch; filed as #140.
- **The operator's AERO claim:** at 2026-11-17T11:13:04Z the wallet claimed 17.654015 AERO
  from the CL gauge (tx `0x2d2b609b…a16e`). It was found on-chain and recorded in the
  harvest ledger on chudmax at $7.75 (the AERO price pool at that block).
- **#142, merged as pull request #143 at 2026-11-17T12:19:34Z:** a staked Aero position's gauge reward is reported as its
  collectible earnings on the snapshot (spec 2.7.0), the claim simulation, both MCP
  transports, the dashboard and the Lite cockpit. LP fees, AERO rewards and their total
  are three labelled figures. Reporting only: the total, the directive, the harvest
  threshold and the notifier do not read rewards, and no claim transaction is built.

## How #142 was built and checked

- One implementing agent, two independent reviewers (on-chain and numbers; product and
  every surface), then a fix pass. Neither reviewer found a blocker or a major issue; six
  minor findings were fixed.
- The operator's own claim proved the CL gauge's claim call (`0x09902477`, wallet, wallet,
  [NFT id]) and that it pays exactly what `earned` reported one block earlier. The basic
  gauge's call (`0x8ed86282`) is proven by another account's real claim plus a simulation
  for this wallet. A gas estimate is shown only for those two gauge addresses.
- Run by me at the branch tip: 343 worker tests + 24 MCP, 299 frontend tests, the frontend
  build, `make test-brain`, a release build, and a local run against the live chain through
  the operator's Alchemy plan on a scratch database. The claim simulation printed a live
  gas estimate for the basic gauge reward; the dashboard and the Lite cockpit (375 px)
  rendered the reward lines with no console errors.

## Not verified, and open questions for the operator

- When this log was first written nothing had been deployed from the branch and chudmax
  ran v0.8.0. That changed the same day: see "After the merge" below.
- The basic gauge's claim call has not been sent by the operator's wallet itself.
- Whether gauge rewards should count toward BATCH HARVEST: no rule is set, and today they
  do not.
- With the CL reward claimed and the position out of range, that position's collectible
  earnings read 0.000000 AERO until it earns again.

## After the merge: the withdrawal, and issue #144

*Added with the fix for #144. An earlier version of this log, and `SESSION_LOG.md`, still
called #142 an open pull request.*

- #142 was merged to `main` as pull request #143 at 2026-11-17T12:19:34Z (merge commit
  `1db3c11`) and deployed to chudmax that day. The deployment's snapshot of 12:24:59Z
  carries spec version 2.7.0, which only that code emits. It is not a tagged release.
- At 12:18:39Z, while that was going on, the operator withdrew the Aero CL position in
  the Aero app: transaction `0x09223b2a…801a`, Base block 58637717. NFT #2914490 was
  unstaked from the gauge and burned, and 0.099668 WETH returned to the wallet.
- From the next cycle (12:20:13Z) every ingestion cycle failed with `Aero CL positions()
  read failed (NFT #2914490 …)`; `/health` read `degraded` and the snapshot stayed at
  12:15:14Z. It was noticed a few minutes later because the deploy was being verified.
  The `aero-cl200-weth-nock` entry was commented out of the operator's config and the
  stack redeployed; the three remaining positions have been read since.
- That is issue #144 (opened 12:25:50Z): an ordinary action in the wallet stopped all
  reporting. The fix reports such a position as closed and keeps ingesting; it is
  described in `CHANGELOG.md` under Unreleased and in `ONCHAIN_INTEGRATION.md` §13. At
  the time of writing it is a local branch (`fix/144-closed-positions`): not pushed, not
  merged and not deployed.

## Filed today

#140 (`make dist` overwrites binaries in place), #142 (this work), #144 (a withdrawn
position stalls ingestion). #130 to #137 from the
v0.8.0 stack review remain open.

## Later: #144 merged, the burn-down, gas-side ranges, v0.8.1

*Added with the v0.8.1 release PR.*

- **#144** merged as #149 and deployed. The first outside pull request (#141, fixing #140)
  arrived and was merged by the operator after the CI audit in
  `docs/kb/ci-security-audit-2026-11-17.md`. Workflow hardening for #146 was merged as #147,
  and two lore notes as #148.
- **The v0.8.1 burn-down,** #130 to #137 in #150, was merged and deployed late on 2026-11-17.
  - CONFIRM and RECORD BUY wait at most 8 s for the refresh they start, because a single
    cycle would otherwise make them look stuck.
  - The cross-site guard ignores ports, so another port on the same host (Strata on :8080)
    and DNS rebinding are not stopped. Both are left for the v0.9.0 access decision.
- **Gas-side ranges.** That evening the operator opened two ranges with no NOCK leg:
  Uniswap v3 ETH/USDC 0.05% #6607225 and Aero CL WETH/USDC #3449558, staked. They hold
  about $160 of USDC and idle gas ETH, to compare which venue pays better.
  - A scratch run showed they would be counted as NOCK LP, drawn on the NOCK price chart
    and recommended for claims below gas.
  - That was filed as #151 to #153. An audit (29 agents) found the affected places, and
    an adversarial review (22 agents) checked the fix.
  - Merged as #154, then deployed with both positions added to the operator's config.
  - chudmax then read a $3,058 total across 5 positions, with NOCK LP $130 short of its
    target and REBUILD LP TRANCHES leading.
- **Not built:** a side-by-side view of what the Uniswap and Aero ranges earn. Today the
  Uniswap range earns LP fees and the Aero range earns AERO, each in its own velocity row.

## v0.8.2: the harvest ledger books only fees (2026-11-18)

- The operator withdrew v3 WETH/NOCK #6336662 (tx `0x06515a23…b778`, block 58831000), and
  its empty config entry was removed. Recording the withdrawal's fees showed that
  `record-harvest` booked every inflow: the returned principal (#156) and, found while
  testing, a NOCK buy (#157) would have been booked as fee income. Nothing had been
  misbooked: the withdrawal was held back until the fix.
- PR #158 separates principal read from `DecreaseLiquidity` and `positions()` at the block
  before, and refuses what it cannot split or what is not a claim. It was verified on a
  scratch database against the real withdrawal (fees $3.40: 0.000999503 ETH and 101.58
  NOCK) and the real buy (refused), and through two review rounds that replayed 17 of the
  wallet's transactions.
- Released as v0.8.2 at the operator's word, deployed to chudmax, and the withdrawal's
  fees recorded there.

