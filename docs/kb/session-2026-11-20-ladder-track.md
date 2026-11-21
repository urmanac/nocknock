# Session 2026-11-20 → 2026-11-21: the ladder track, v0.9.0

Agent: Claude (Fable 5.2, then Opus 5.5 after the Fable limit). Release: v0.9.0.

## What shipped

- #165 the floor-crossing price; #166 the ladder simulator; #167 `role: ladder`;
  #168 the ladder suggestion, then the operator's three-deposit "bottle" (cap, near 15%
  down to $0.0110, deep 85% down to the $0.01 bottom), measured by liquidity and frozen by
  deposits (PR #176).
- Positions found on-chain: the wallet's Uniswap v4 NFTs are found from its transfers and
  a self-minted position on a suggested rung is tracked by itself (migration
  `20261121000013`, forward-only).
- #169 ladder maintenance with a restraint model, call-to-action tiers as
  `derived_metrics.events[]`, and the ladder shape chart (PR #178).
- PR #177: release builds on macOS 27.2.1 (`[profile.release.build-override] strip = "none"`).

## Operator decisions

- The bottle was deployed by the operator on 2026-11-21 and tracked automatically.
- Reserve tier: the reserve event is at the top whenever wallet USDC alone is below
  `cash_reserve_usdc`.
- Restraint values: 24 h out of range, 24 h cooldown, 2% hysteresis, 30-day gas payback,
  0.5 fill fraction, recovery 10%, then lowered to 5% the same day once the page showed
  that 10% could not fire on a 9.42%-wide deep rung.
- UX: finished calls to action become quiet lines; ladder rows stay one line; the
  deployed ranges are drawn on a log price axis with area proportional to capital, "so
  that we can approximate the bottle shape and judge whether it looks bottle-y".
- The gas-side ETH/USDC ranges are measured against the reserve shortfall, never steered;
  the operator pulls them when they judge the time is right.
- Moved to v0.10.0: #170 (AERO and ETH ladders), wallet-signed deposit buttons, and the
  phone/push notification channel.

## Lessons

- Uniswap's app drops a prefilled amount from a deposit link; the amount is printed beside
  the link instead.
- The scratch-database tests share one database: run them with `--test-threads=1`.
- A merge before CI has started reads as green: wait for the checks to appear first.
- chudmax runs merged `main` (abc8515) at the end of the session, then the v0.9.0 tag.
