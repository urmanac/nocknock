# Session Log: 2026-11-13 → 2026-11-14

**Agent:** Claude (Opus 5.5 for the first exchange, then Fable 5.2), in Claude Code
**Operator:** Kingdon
**Releases cut:** v0.4.0, v0.5.0
**Outcome:** NockNock has a standing on-prem deployment, understands the current
portfolio again, paces NOCK buys, and tells the operator which LP deposits to make.

---

## Where things stood at the start

- `main` was v0.3.0 plus docs. It modelled a portfolio that no longer existed: the
  operator had liquidated Aerodrome on 2026-10-23, repositioned into four Uniswap
  ranges (PR #74, unmerged), and then liquidated those too. The wallet was fully liquid.
- There was no stable deployment. The local MCP server answered in "zero-DB" mode.
- The operator had about $1,000 of USDC to put to work and wanted guidance.

## What was done, in order

1. **On-prem deployment on `chudmax`** (rootless Podman quadlets, LAN only; PR #77).
   Found along the way: `Cargo.lock` was gitignored while the Dockerfile copies it, and
   base image names had to be fully qualified for Podman.
2. **v0.4.0** (PR #78, containing #72, #74, #77): security updates (worker advisories
   16 → 0 actionable, frontend criticals cleared), the Uniswap-only portfolio model, a
   guard against publishing a frozen price from a pool with no in-range liquidity
   (issue #76), and the deploy tooling. `sqlx` 0.7 no longer compiled on current stable
   Rust, which is why the PRs had to land together.
3. **Accumulation plan, Phase 1** (issue #79, PR #80): dollar-cost-averaged NOCK buys.
   The plan paces and remembers; the operator swaps and records the transaction hash;
   only receipt-verified buys count. A chudmax playtest caught a real flaw (a week-old
   swap completing a brand-new plan) before it shipped.
4. **Protected cash reserve** (same PR): the plan never spends the stable balance below
   a floor the operator sets.
5. **First real tranche**: $20 → 1,597.93 NOCK at $0.012516, tx `0x5d0c02d9…3f0e`,
   recorded as fill 1 of 5.
6. **Aero transition** (issue #81, PR #82): Aerodrome and Velodrome merged into Aero
   (aero.xyz) on 2026-10-21. The tranche routed USDC → USDT → WETH → NOCK through a
   universal-router style contract and cost about 1.8% over spot.
7. **LP rebuild suggestions** (issue #83, PR #84): `REBUILD LP TRANCHES` gained a call
   to action (amounts per position, link to the position in the Uniswap app). The
   operator rebuilt both Uniswap ranges from it; the handoff worked.
8. **Pre-0.5.0 fixes and the Aerodrome tranche** (issues #85–#88, PR #90): the NOCK
   stack chart was plotting dollars as NOCK; the example config was never loaded as a
   base layer; `/health` said "ok" while ingestion failed; Aerodrome full-range
   positions are tracked again and included in guidance; nightly database backups.
9. **v0.5.0** released and deployed to chudmax.

## Operator decisions and standing orders recorded this session

All are in `OPERATOR_GUIDANCE.md` §7.

- Rebuilding LP is cash-neutral: always carry a bias toward doing it.
- NOCK buys are dollar-cost averaged; budget, tranches and cadence are the operator's.
- Protect the cash position: $950 of stable balance is never spent by the plan. USDC
  inside LP counts toward it. "Cash over-funded" on the deviation panel is welcome.
- Do not fill out-of-range positions by default.
- The overlap between the 100,000 NOCK floor and LP is deliberate: ranges sell
  gradually, the aim is to trade more and earn fees. Exposure is bounded by a tolerance
  and reported, not forbidden. (The agent first imposed a hard cap and was overruled.)
- A full-range Aerodrome position is the intended long-term core: it cannot go out of
  range, so it never forces a re-range.
- Agents may improve the soundness of the strategy and build that into code, saying what
  changed and why. Choosing the operator's allocation stays with the operator.
- NockNock's ledger is a second record so nothing on-chain is a surprise at tax time.
  It is not a compliance system.

## State at the end of the session (2026-11-14 ~17:00 UTC)

| | |
|---|---|
| Wallet | 95,599 NOCK, 962.77 USDC, plus some ETH (not counted in totals, not re-read at session end) |
| Uniswap v4 USDC/NOCK 3% #3163450 | ~$180, in range |
| Uniswap v3 WETH/NOCK 1% #6336662 | ~$179, in range |
| Aerodrome NOCK/USDC (full range) | tracked, not yet funded |
| Total | ~$2,430; 115,922 NOCK including LP |
| Plan `2026-11-reentry` | $20 of $100 deployed, tranche 2 of 5 due 2026-11-15T06:02Z |
| Limits | LP target 20%, floor tolerance 10%, cash reserve $950 |
| Live directive | `REBUILD LP TRANCHES`: add ~$64 USDC + ~5,500 NOCK to Aerodrome |

## Infrastructure now in place

- `chudmax` runs the stack as systemd user units (see AGENTS.md §13). Dashboard on
  port 3000, API and MCP on 8081, PostgreSQL private to the container network.
- Nightly `pg_dump` at 03:30 into `~/nocknock/backups` on chudmax, 14 kept.
- The operator's config is `ingestion-worker/config.yaml` on the workstation
  (gitignored); `deploy/podman/deploy.sh chudmax` ships it as a Podman secret.

## Not verified, stated plainly

- The Aerodrome read with a non-zero LP balance. It works against the live pool at a
  zero balance and in unit tests with real reserves.
- The legacy Aerodrome deposit page past wallet connection (aero.xyz does not list the
  pool; aerodrome.finance still serves its deposit route), and what an unstaked LP earns
  there now. A gauge-staked balance is not read.
- WalletConnect pairing and a real fee claim on the Vite 8 frontend build.
- Notification delivery: no notifier is configured on chudmax.
- No official Aero contract list was found; router identity is observed on-chain plus
  one third-party label.
- 17 frontend advisories remain with no upstream fix (`zod` inside WalletConnect,
  `minimatch` inside the linter).

## Mistakes worth remembering

- Deployed an operator config in the pre-v0.4.0 shape; ingestion was down ~8 minutes.
  This exposed the config-layering and `/health` bugs, both since fixed.
- Imposed a hard "no floor NOCK in LP" cap without asking what the overlap was for.
- Chained test, commit and push in one command and pushed a commit with a failing test
  (fixed in the next commit; `main` never saw it red).
- Quoted an eight-hour-old price as current. Sessions here span long gaps: re-read the
  snapshot before quoting numbers.

## Open work

- **Epic #89**: phone install flow, notifications, TLS for the internal deployment.
- **Issue #91**: Aero concentrated (CL) positions. The operator wants exposure to both
  USDC/NOCK and WETH/NOCK on Aero; the CL200 WETH/NOCK pool cannot be read yet, so a
  deposit there is invisible. The Basic Volatile USDC/NOCK pool on aero.xyz is tracked.
- **Phase 2** of the accumulation plan needs redesign for Aero's router; an alternative
  is to keep the operator-swaps model and measure execution cost per fill instead.
- Build the unsigned add-liquidity transaction for LP suggestions (encoding must be
  verified by simulation first).
- Count ETH in portfolio totals; externalise the legacy advice constants ($0.020 line,
  120,000 NOCK goal); decide target weights that reflect the cash-heavy posture.
- Hosts `netmoom` and `metnoom` (RK1 boards) reject the workstation's SSH key; nothing
  is deployed there.
- The NOCK mining plan lives elsewhere (the operator suspects their Google AI Mode chat
  history) and was deliberately deferred.
