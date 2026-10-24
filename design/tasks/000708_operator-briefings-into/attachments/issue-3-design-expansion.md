## Context (non-urgent design note)

The original nocknock design modeled exactly two positions (one V4 NFT, one Aerodrome pool) with fixed roles. The real portfolio now holds FOUR positions in three shapes (two USDC/NOCK v4 pools at different widths, an ETH/NOCK v4, a WETH/NOCK v3 ladder) — and the operator demonstrably repositions often. The design space is much bigger than "two tranches":

- **Protocol**: v3, v4 (incl. native-ETH-as-address(0), hooks), Aerodrome, Velodrome, …each with its own accounting (NFT ids vs LP shares vs gauges).
- **Network & wallet**: same key across chains; multiple wallets (operator / treasury / hot).
- **Pool & pair**: any pair, any tier — and **multiple positions in the SAME pool** (we hold two USDC/NOCK positions at different widths right now).
- **Range geometry**: tight harvest bands vs wide stability rafts vs ladders/grids (DCA mechanics); single-sided states at the edges.
- **Intent (the deep one)**: yield capture / buy-ladder / sell-ladder / floor support / incentive farming / market-making — one position can serve several. **Advice should speak intent, not tranches.**
- **Accounting**: growth-accrued fees (v4) vs tokensOwed (v3) vs reward contracts; auto-compounding hooks move balances without transactions.
- **Risk shape**: a range is mechanically a short-volatility position; width = how much rebalancing you accept.

## Proposal (after the Uniswap-only ticket lands)

1. **Role tags on positions** (`intent: buy-ladder | yield | floor | sell-ladder | park`) in the config registry; advice engine speaks role-language ("your WETH→NOCK ladder froze; it holds X WETH and stopped converting").
2. **Position-event ledger** (minted / re-ranged / collected / burned, with tx hashes) — repositioning is recurring; current-state-only views rewrite history and the ledger stops lying.
3. **Series keys by role, not NFT id** — so re-anchoring doesn't fracture long-horizon charts.
4. **Adapter registry** — the third protocol costs one adapter file + a config entry.
5. **Multi-wallet / multi-network roll-up** — last; multiplies everything.

Related: #1 (the live reconciliation), spec task spt_01m5nqxqr1ma3qhsqsx6vw5kq7 (in flight).
