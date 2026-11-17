# Two trips and a wait

*Lore entry, 2026-11-17. A draft, and the companion to [The gas pump](2026-11-17-the-gas-pump.md).
It came out of a conversation about when to reopen a range. The operator's position is in
their own words where possible; the mechanics were written up by Claude. It is one person's
position about one wallet. It is not tax advice and not a recommendation: what is owed, and
how a transaction is treated, is for the operator and their accountant.*

---

## We are always playing

> "There are a couple of 'better plays' and 'worse plays' and we do not need to engage in
> play in order to be caught in a play. So, we should consider that we are always playing,
> and attempt to play it better with a soft touch when needed."
> — the operator

Staying out of a pool is a position too. Holding gas instead of NOCK is a bet, the same as
holding NOCK instead of gas.

## Two trips

- **Up.** A range is loaded with NOCK while the price is low. The price climbs, the pump
  sells NOCK for gas, and it is closed at the top. The tank is fuller.
- **Down.** A range is loaded with gas while the price is high. The price falls, the pump
  buys NOCK back, and it ends holding NOCK again.

Run both in the same range and you are back where you began, plus fees. A range sells on
the way up at the same prices it buys back at on the way down. That is a fine story and a
small one.

## The wait is where it is won or lost

Neither trip decides the outcome. What happens between them does.

| | NOCK rises | NOCK falls |
|---|---|---|
| **In the range** | Sells NOCK for gas, earns fees. Gives up the rest of the rally. | Buys NOCK with gas, earns fees. Catches it the whole way down. |
| **Out, holding gas** | Nothing lost in gas. The NOCK rally goes on without you. | The gas is intact and buys more NOCK than was sold. The best case for the circuit. |

The gain of the whole circuit comes from the bottom-right square: closing at the top, then
reopening after the price has come down. The cost is the bottom-left: it was closed, and
the price kept going. No story removes that risk. The fallow turn is the bet.

## Where on the range you reopen decides which trip you are loading

You cannot add only NOCK to a range that is in range: the pool sets the mix. Near the top
of a range the mix is mostly gas, so adding there is mostly a standing order to buy NOCK
with gas as the price falls. Near the bottom the mix is mostly NOCK, so adding there loads
the pump to sell NOCK for gas as the price rises.

So reopening at the top is not wrong. It loads the other trip. On 2026-11-17 the
operator's remaining WETH/NOCK range stood 84% of the way to its top and held 81% gas and
19% NOCK by value; a dollar added there would have gone in at that mix.

## The soft touch

> "Sometimes it's easy to suggest the correct soft touch, and the risk of being wrong is
> extraordinarily low."
> — the operator

The low-regret ones describe something that is already true, and say what follows in both
directions. They are not forecasts.

- This range is out of range and has earned nothing for N hours.
- This range is a few percent from its edge, and past it will hold only gas.
- The uncollected fees are worth many times the gas to collect them.
- Your position is the only liquidity in range in this pool.

"The price might move soon" is a forecast. Forecasts are where being wrong is expensive,
and NockNock does not make them.

## The tax bill is part of every trip

> "The advice is 'you will owe taxes' and 'each leg is a taxable event' - this means we
> need to book real profit (eg. USDC portfolio balance increase) every time (within
> tolerances) we've added a taxable event that might incur some liability."
> — the operator

What follows from that, as the operator put it:

- **Gas is the first leg, not the finish.** "I think the 'ETH as profit' is the first leg
  of a potentially profitable journey, but it usually does not end with 'then we dispose
  of the ETH' in the immediate short term, unless it was for USDC rather than NOCK."
- **Turning gas straight back into NOCK is how the trouble starts.** Each trip is another
  taxable event. After many of them, some gaining and some losing, a person can owe tax
  and have no dollars to pay it with.
- **So profit has to show up as dollars.** A trip that creates a liability should leave
  the USDC balance higher, within a tolerance, not only the gas tank.

> "One would rather have the money in the bank than a story about some losses and an
> audit."
> — the operator

Two mechanical points that go with it:

- Reopening near the price you closed at does not by itself reduce what is owed. The tax
  follows the sales the range made, and their gain depends on what that NOCK cost.
  Turning WETH back into NOCK disposes of the WETH, so what matters there is whether ETH's
  dollar price has moved since it was received.
- Trips measured in days are all short-term. Fewer, larger trips mean fewer events.

## Where this left things on 2026-11-17

> "So, standing pat at this point is what I would probably recommend!"
> — the operator

The Aero range had been closed at its top with the gas kept in the wallet. The remaining
Uniswap WETH/NOCK range was left as it was. Nothing was reopened.

## What NockNock does and does not do about it

- It keeps a virtual tax reserve: a share of each recorded harvest, set by the operator,
  is booked against the USDC balance. That covers fees and rewards that were claimed.
- It does **not** yet book the sales a range makes. A trip that turns NOCK into gas is a
  taxable event the ledger does not see, and nothing checks that dollars were set aside
  for it. Closing that gap is planned with the range and ledger work.
- It does not say when to close or reopen a range, and it never advises spending gas on
  NOCK ([`OPERATOR_GUIDANCE.md`](../../OPERATOR_GUIDANCE.md) section 7). It can state what a
  posture means in both directions; the move is the operator's.
