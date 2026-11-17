# The gas pump

*Lore entry, 2026-11-17. A draft. The operator asked for a story about how this makes
money that is not "NOCK goes up", and named the answer; Claude wrote it up, with the first
lap taken from the chain. Like
[Four pillars, and no hurry](2026-11-14-four-pillars-no-hurry.md), it is one person's
position about one wallet, not a recommendation to anyone.*

---

## Why a story at all

> "We need a story to tell that isn't 'nock goes to the moon.' It isn't rent paid. It's
> not the thing of value being measured. [...] So we need a story that explains how we
> make money that is something other than the price of Nock going up. I think the answer
> is this gas pump."
> — the operator

## The story

We run a gas pump.

It's a small one. On one side there is a token called NOCK, and on the other there is ETH,
which is the gas everything on this chain runs on. The pump has a simple rule: when people
show up wanting NOCK and the price climbs, it hands them NOCK and keeps their gas. Nobody
has to stand there. It does it a little at a time, the whole way up.

Then the pump runs dry. All the NOCK that was in it is gone and what is left is a tank of
gas. At that point we close it and go home. The land lies fallow for a turn.

When the crowd thins out and NOCK is cheap again, we open the pump lower down, with the
gas we kept, and it fills back up with NOCK. Then we wait for the next crowd.

That's the whole business. We don't measure it by what NOCK is worth. We measure it by how
much gas is in the tank after each lap, compared with the lap before.

Somebody has to pump the gas. Some days it feels like we're doing it because that's the
rule and somebody has to. We'd still like to make some money on it.

> "We pump our own gas, or someone does it for us - anyway, I sure hope we make some money
> on this gas. You know? Same old story."
> — the operator

## What the pump is, without the story

A concentrated WETH/NOCK liquidity range. Inside its range it holds both tokens; as NOCK's
price in ETH rises it sells NOCK for WETH, and above the top of the range it holds only
WETH and earns nothing. If the price comes back down into the range it buys NOCK again.
Closing the position at the top and not reopening it is what keeps the gas.

> "If we repeat this transactional circuit over and over again and wait the appropriate
> amount of time before restarting the engine, where the price of [NOCK] is low and when
> it goes high, we've cashed out in Ethereum."
> — the operator

## The first lap

Read from the chain. The position was the Aero concentrated WETH/NOCK range (NFT #2914490),
and its liquidity did not change between the two readings, so the difference is the range
doing its work.

| When (UTC) | WETH in the range | NOCK in the range |
|---|---|---|
| 2026-11-16 04:18 | 0.0559 | 6,946 |
| 2026-11-17 12:18, withdrawn | 0.0997 | 0 |

- The range sold all 6,946 NOCK for about 0.0438 WETH as NOCK rose about 30% in a day.
- It finished selling at the top of its range, about $0.0116 per NOCK with ETH near $1,800.
- The operator claimed 17.65 AERO in gauge rewards (2026-11-17 11:13) and withdrew the
  position an hour later (tx `0x09223b2a…801a`). The WETH went to the wallet.

So the tank went from 0.056 to 0.100 WETH, plus the rewards.

## What it costs

The story is only worth telling with this part in it.

- **The pump sells early.** It was empty at about $0.0116, and NOCK went on to about
  $0.0149 the same day. In dollars, holding would have done better on that leg. In gas,
  the pump did what it was built for.
- **It only pays if there is a next lap.** If NOCK never comes back down, NOCK was swapped
  for ETH once and that is all. If the price falls through the bottom of a reopened range,
  the pump has turned the gas back into NOCK on the way down.
- **Waiting is part of the machine.** An LP left running buys on the way up and sells on
  the way down. The lap only keeps its gas if the pump is closed at the top and stays
  closed for a while.

  > "We should not dilute our gas holdings by 'buying on the way up and selling on the way
  > down' as that's what an LP naturally does, so leaving the land to lie fallow for a turn
  > is definitely a move we would consider."
  > — the operator
- **The fees are small.** Dollars, not a living.
- **Every sale is a taxable event,** and so is every restart.

## What NockNock does and does not do about it

- It reads the range and says what it holds, where its edges are and whether it is in
  range. It does not say when to close a pump or when to reopen one: that timing is the
  operator's.
- ETH is still gas in NockNock's books: counted, not cash, and NockNock never advises
  spending it on NOCK ([`OPERATOR_GUIDANCE.md`](../../OPERATOR_GUIDANCE.md) section 7).
  Reopening a pump with the gas that was kept is the operator's own move.
- It cannot show a lap yet. The dashboard shows the gas pillar now, not what the tank held
  before and after each range. Reporting "what each range did" is planned.

## Read next

[Two trips and a wait](2026-11-17-two-trips-and-a-wait.md): why the waiting decides the
outcome, where on a range to reopen, and the tax bill that comes with every trip.

## Not this

Not a promise, not a yield figure, and not an argument that anyone else should do it. One
lap is one lap.
