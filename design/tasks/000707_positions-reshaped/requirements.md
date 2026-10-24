# Requirements: Uniswap-Only Portfolio After Aerodrome Liquidation

## Background

On 2026-10-23 the operator liquidated the Aerodrome tranche and repositioned the portfolio
entirely into Uniswap concentrated liquidity (evidence: task screenshot
`screenshots/operator-positions-2026-10-23.png`, taken from the Uniswap app):

| Position | Protocol / fee tier | Range (at screenshot time) | Split | Value | Fees | APR |
|---|---|---|---|---|---|---|
| A: NOCK / ETH | Uniswap v4 · 2% | 0.08091 → 0.1474 ETH | 15.13% ETH / 84.87% NOCK | $89.48 | $0 | 42.69% |
| B: NOCK / USDC | Uniswap v4 · 0.75% | 0.01694 → 0.02339 USDC | 62.82% USDC / 37.18% NOCK | $71.45 | $0 | 111.51% |
| C: WETH / NOCK | Uniswap v3 · 1% | 114,099 → 177,158 NOCK | 99.17% WETH / 0.83% NOCK | $18.13 | $0.000167 | 46.20% |

Totals: liquidity $179.07, fees <$0.01, rewards $0.00, all three in range. Values are
point-in-time and will drift — implementation must re-derive them from RPC, never hardcode.

The app still models the OLD shape (Tranche 1 = Uniswap V4 NFT #3126000, Tranche 2 =
Aerodrome vAMM-USDC/NOCK `0x85f1…626` with `claimFees()`), so the snapshot pipeline would
now fail or fabricate data. Discovery (2026-10-24, Base RPC) verified: the tracked wallet
`0xeE114cC97eC5adeB34bA5645e315B497D8487a45` holds 42 NFTs on the canonical V4 POSM
(`0x7c5f5a4bbd8fd63184577525326123b519429bdc`) but zero mints in the fresh window, and 0 NFTs
on the quoted V3 NPM address — so which wallet(s)/network(s) hold the three new positions must
be confirmed by RPC discovery first, with the operator as the fallback source.

## User Stories

1. **As the operator**, when I open the cockpit, I see one card per live LP position
   (pair name, protocol badge v3/v4, fee tier, range bounds, asset split, USD value,
   in-range status, unclaimed fees) — matching on-chain reality, not the retired
   two-tranche model.
2. **As the operator**, the aggregated totals (portfolio USD, total NOCK, floor delta,
   uncollected fees, deviation matrix, advice text) include ALL tracked positions
   and exclude the liquidated Aerodrome tranche — zero mock or stale numbers.
3. **As the operator**, the Golden Path claim-fees flow builds collect transactions for the
   positions that actually exist (Uniswap), and never proposes an Aerodrome `claimFees()`
   transaction for a position that no longer exists.
4. **As an AI agent (MCP)**, `get_portfolio_state` reflects the Uniswap-only portfolio and
   does not present Aerodrome yields as live truth.
5. **As a developer**, tracked positions are configuration-driven (`config.yaml`), so the
   next repositioning is a config edit, not a code change.

## Acceptance Criteria

1. `specs/core/snapshot.yaml` defines a `positions` list (network, wallet, protocol enum
   including real `uniswap-v3`, nft_id, pool, fee tier, tokens, amounts, unclaimed fees,
   range, in_range) and the platform enums gain `UniswapV3`; the removed old field names
   (`live_tranche_1_*`, `live_tranche_2_*`, `tranche_N_nock` series) carry a CHANGELOG
   migration note; `meta.version` bumps (breaking change).
2. The ingestion worker reads per-position on-chain state for N Uniswap-v4 and M Uniswap-v3
   positions (exact amounts/fees computed from liquidity + slot0, no baseline-scaling
   heuristic) and fails explicitly (no mock fallback) when any required read fails.
3. NOCK/USD spot derives from the designated tracked Uniswap pool (not the liquidated
   Aerodrome pool); ETH/USD moves off the Aerodrome vAMM-WETH/USDC pool to a documented
   replacement with an explicit failure mode; the price provenance tag changes accordingly
   (new rows only).
4. The Aerodrome tranche logic (`claimFees` reads, `aerodrome_lp` allocation bucket,
   Aerodrome pool/reserves reads, `Aerodrome` pool_mechanics entry) is removed; the
   deviation matrix and advice text use the new bucket shape.
5. The frontend renders N position cards with correct pair names, fee tiers, and protocol
   badges, and the cockpit aggregates the new totals; the NOCK-stack chart generalizes
   from three hardcoded tranches to wallet + per-position series.
6. Docs updated: ONCHAIN_INTEGRATION.md §2 marked HISTORICAL (liquidated 2026-10-23) plus a
   new section for the Uniswap-only portfolio with the discovered identifiers;
   OPERATOR_GUIDANCE.md/README touched only where they name the old tranches.
7. Tests: worker suite green (new per-position v4 + v3 decoders, updated advice/tranche
   tests, three-position fixture snapshot); frontend tests updated
   (`freshness.contract.test.ts`, `client.errors.test.ts`, fixture-driven card test).
8. Live validation: a real snapshot fetch against the confirmed wallet shows the three
   positions with values matching the screenshot's magnitudes (± market drift), and the
   cockpit renders three cards.

## Explicitly Out of Scope

- No strategy/auto-rebalancing logic (the operator positions manually).
- No chain additions beyond what Phase-1 discovery confirms.
- No Aerodrome re-integration; no Aerodrome code resurrection "just in case".
- No APR/ROI computation in the cockpit (that is the Uniswap app's display).
- No WASM brain changes (the brain is pure math and does not consume tranches).

## Open Questions

1. **Wallet + network of the three new positions** — discovery must confirm which
   wallet(s) hold the three fresh NFTs (tracked wallet shows zero fresh mints) and which
   network the v3 position is on. Operator confirmation is the fallback source; please
   confirm whether you can name the wallet/network directly to short-circuit discovery.
2. **V3 NPM address in the ticket** — the ticket quotes `0x46A15B0b27311cedF172AB29E4f4766fbE7F4364`
   as "canonical V3 NPM on Base", which does not match the address we expect for Base
   (implementation will verify canonical NPM per network on-chain via the Uniswap
   deployments registry before any read). Please confirm which network the Uniswap app
   used for the WETH/NOCK position.
3. **Deviation-matrix targets** — old targets were treasury 80% / Aerodrome 10% /
   Uniswap-V4 5% / cash 5%. With Aerodrome gone, what should the new targets be?
   Proposed default: treasury / uniswap_lp (aggregate) / cash only, operator-tuned via
   config; please confirm or supply values.
4. **V3 fee claiming** — should the Golden Path also build Uniswap-v3 `collect` actions
   (only meaningful if discovery confirms the position on a reachable network + canonical
   NPM), or stay v4-only for now (proposed default)?
5. **Retiring the v4 baseline-scaling heuristic** — replacing `v4_baseline_*` profile
   anchors with exact on-chain amount math changes headline NOCK totals slightly (they
   become exact). Confirm this is acceptable (it is the zero-mock-correct behavior).
