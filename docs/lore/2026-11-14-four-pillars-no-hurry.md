# Four pillars, and no hurry

*Lore entry, 2026-11-14. The operator's position in their own words where possible,
written up by Claude after a long session that re-entered the market from cash. It
explains the reasoning behind the numbers in NockNock. It is one person's position about
one wallet, not a recommendation to anyone.*

---

## The short version

We hold four things: **NOCK, cash (USDC), gas (ETH) and AERO**. None of them is the
point of the others. We are down on NOCK, we know by how much, and we are not in a hurry
to buy our way out of it. NockNock watches the wallet and keeps us honest; the wallet is
the record, not NockNock.

## Four pillars, not a pyramid

Picture a pyramid with a triangular base. The three corners on the ground are NOCK, cash
and gas, and the fourth corner, at the top, is AERO.

> "We don't want to be in any pyramid scheme, so we neither minimize nor maximize the
> importance of the pillars. They just are. If they grow, that's good. We try to keep
> them in balance so that we can say we have a diversified shit-coin portfolio."
> — the operator

| Pillar | What it is for | How NockNock treats it |
|---|---|---|
| **NOCK** | The conviction holding. A floor of 100,000 NOCK is the line we defend. | Wallet NOCK plus the NOCK inside tracked LP positions. One Aero WETH/NOCK concentrated position (about 2,755 NOCK) is not tracked yet ([#91](https://github.com/urmanac/nocknock/issues/91)), so the count reads low. |
| **Cash (USDC)** | Liquidity, patience, and the ability to act in a dip. | Wallet USDC plus the USDC inside tracked LP. A reserve the engine never advises spending below, and USDC inside LP counts toward it. |
| **Gas (ETH)** | Paying for transactions on Base. Cheap there, so a little goes a long way. | Has value, so it counts in the total: wallet ETH and the ETH inside LP alike. Not cash, not money to invest, and never the source NockNock sizes a deposit from (from v0.5.1). |
| **AERO** | Rewards from staking Aero LP in its gauges. Unplanned income. | Not yet read or valued ([#93](https://github.com/urmanac/nocknock/issues/93)). We are not trying to accumulate it, and we are not refusing it either. |

The LP positions are not a fifth pillar. They are where the pillars work together: NOCK
paired with USDC or ETH, earning fees, selling a little NOCK as the price rises and
buying a little back as it falls. We let LP hold slightly more NOCK than sits above the
floor, on purpose. Ranges sell gradually, so only a very large rally could take the stack
below the floor. When it does propose deposits, NockNock states the worst case for the
floor alongside them.

The dashboard's deviation panel measures something different: where the value sits
(treasury, LP, cash) against the target weights the operator set. Its labels are
measurements, not instructions. "Cash over-funded" is welcome here, and "treasury under
target" is not a signal to buy.

### Cash and gas are not road-trip money

> "Just because I have cash in my pocket doesn't mean I'm going to rip it out when I see
> a sign that says 'investment low.' Similarly, just because I have some gas in the bank
> doesn't mean I'm going to go on a huge road trip and burn it all right away."

A little extra gas is fine to keep around, the same as a little extra cash. It has
utility, so it has value. It is still not investable capital: its price can fall like
anything else's. So from v0.5.1 NockNock counts ETH in the portfolio total but never in
the cash reserve, and never tells anyone to spend it on NOCK. (Until then, wallet ETH is
not in the total at all: [#94](https://github.com/urmanac/nocknock/issues/94).) Moving some gas into a WETH/NOCK
range is the operator's own call, as it was on 2026-11-14. Once there, it still counts as
gas, the way USDC inside LP still counts as cash.

## Where we stand

On 2026-11-14 the Uniswap web app's all-time performance panel for this wallet's NOCK
showed:

| | |
|---|---|
| Balance | 90,088.98 NOCK, worth $1,034.47 |
| Average cost | $0.02 (the app rounds to the cent) |
| Unrealized return | −$576.40 (−33.25%) |
| Realized return | −$1,002.58 (−14.68%) |
| Total return | −$1,578.98 |

The same page showed a 52-week range of **$0.00486 to $0.125**, about $10,000 of volume
that day, and $203,600 of total value locked. The price that day was about $0.0115.

Two caveats:

- **The panel only covers wallet NOCK.** The NOCK inside LP positions is not included.
  NockNock can't fill the gap: it tracks the average cost of the buys its plan paces, but
  not the cost basis of the whole stack ([#98](https://github.com/urmanac/nocknock/issues/98)).
- **The panel's figures don't reconcile exactly.** Its own numbers imply a cost of about
  $1,611 and an average near $0.0179, but a −35.8% return rather than the −33.25% shown.
  Read them as approximate.

**We are in unrealized-loss territory, and we have realized losses too.** Earlier versions
of NockNock printed "0% realized loss" in their HOLD advice. That was never computed, and
it was false. v0.5.1 removes it ([#95](https://github.com/urmanac/nocknock/issues/95)).

## Why not buy more now?

The price is well below our average cost, so the obvious question is why we aren't buying
much more NOCK to bring the average down.

**First, the arithmetic.** Take the panel at face value: about 90,000 NOCK at an average
near $0.0179, against a price of $0.0115.

| Purchase | NOCK bought | Average cost after |
|---|---|---|
| The rest of the current plan: $80 in four $20 buys | about 7,000 | about $0.0174, down 2.6% |
| About $850 | about 74,000 | $0.015 |
| About $4,700 | about 412,000 | within 10% of the price |

The plan's first $20 buy, made earlier that day, is already inside the panel's figures.

The $4,700 purchase is roughly five times the whole cash pillar, and spending the cash
pillar is exactly what we decided not to do. No purchase of that size is scheduled.

**Second, the price has been lower.** It traded at $0.00486 within the last year.

> "Because I've been wrong before, and it can always go back below $0.01, so why the
> hurry mate."

So we accept being underwater for a while. If NOCK recovers, we may wish we had bought
more at $0.0115. If it revisits $0.005, the cash we kept will be the reason we can act.
Both outcomes are survivable, and that is the point.

## What the DCA timer is for

The accumulation plan paces NOCK buys that we make ourselves on Aero: currently $100 in
five $20 tranches, 24 hours apart. NockNock says when the next one is due, checks each buy
against its on-chain receipt, and never buys anything itself. The plan holds a cash
reserve at any price, and it pauses when the market is overextended.

> "That's what the DCA strategy timer is for — to encourage a little light growth,
> especially in a way where we're buying low and selling high."

The timer is not there to repair the cost basis, and at this size it can't. It keeps us
buying a little while staying patient, and it leaves a record we can judge the strategy
by later. Charting the average cost against the price over time would make that judgment
easier; NockNock can't do it yet.

## The wallet is the record

Balances, positions, fees and the receipts of buys we record are read from the chain.
Price history is backfilled from public market feeds. The high-water mark and the charts
live only in NockNock's database, which is a convenience, not the record.

> "We can always do that, even when our history is wiped out and we are getting a
> fresh, new start with an empty database and the latest release from urmanac/nocknock —
> we just need our wallet. That's completely out of scope for nocknock to manage. It's
> the Uniswap wallet!"

What that means in practice:

- **Nothing in NockNock is called a backup.** If the database is lost, start fresh. The
  high-water mark and charts restart. Plan progress can be restored, because every buy is
  still on-chain: set the plan's `starts_at` to its real start and record each buy's
  transaction hash again, and NockNock re-verifies each one. Or simply start a new plan.
- **Keep `config.yaml`.** It lives outside the database (on the workstation, deployed as a
  secret) and lists the positions to track, the plan, and the limits, so a database wipe
  doesn't touch it. If `config.yaml` itself were lost, you would list the positions again
  by hand: NockNock does not discover them from the wallet yet.
- **The deployment is infrastructure, not production.** It runs on `chudmax`, a machine
  on the home LAN, in rootless Podman, with no public endpoint. There is a pile of spare
  boards that may run Kubernetes one day. Until then it stays simple.

## Building in public, so: the newcomer

We did not start from zero. We came to NockNock already holding NOCK, liquidated, and
came back in. That is not the path a newcomer takes.

> "It's not meant to be our focus... But we're building in public, which means we are
> always at risk of serving as an example to someone else. And so we do always need to
> consider them, as well."

So the rule for a newcomer is the rule for us, stated plainly: **NockNock reads your
wallet and states facts. Every number that sizes an action is yours.** That covers the
budget, tranches and interval of a buying plan, a floor, target weights, and a cash
reserve. Leave one unset and there is no rule, never someone else's number filled in.

What a newcomer will see, once v0.5.1 and v0.5.2 land:

- **An empty wallet:** NockNock says there is nothing to track yet. No alarms, no
  "maintain posture".
- **Only USDC:** NockNock says so, and explains that it paces buys only inside an
  accumulation plan whose numbers you choose. It never tells you to put most of your cash
  in at once, whatever the price.
- **Your own plan:** each tranche at your size and your interval, each buy checked on-chain.
- **Your own floor:** if you set one and are below it, NockNock says by how much and keeps
  your plan's next step in view.
- **No ETH:** NockNock tells you once that every action on Base pays its gas in ETH, and
  suggests no amount.

Until those patches ship, NockNock can't run for a wallet with no LP position at all, and
its example config carries one operator's wallet and numbers. Both are being fixed; see
`docs/kb/plan-v0.5-patches.md`.

## What this is, honestly

- **NockNock is vibe-coded.** AI agents wrote almost all of it, across many sessions,
  with one operator steering and playtesting rather than reviewing every line.
- **It has one user, one wallet, and no audit.** It runs only on that operator's LAN.
- **Its tests were written by the same agents that wrote the code.**
- **The rules it enforces are one person's.** The 100,000 NOCK floor, the reserve and the
  pacing are their choices. Treat them as an example, not a template.
- **It never signs anything.** It reads, sizes and explains. A person decides, in their
  own wallet app, and NockNock checks the result on-chain afterwards.
