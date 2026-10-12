# Final Assessment — 0.2.0 Series Bug Hunt and Triage Cycle (task 000664)

## Executive Summary

The 0.1.x triage process was re-run against the v0.2.0 (Golden Path) release with a
**live Alchemy connection**: the worker ran against Base mainnet in this sandbox for the
whole audit, every finding was verified against live on-chain data or live HTTP
responses, and all confirmed defects were fixed on `feature/000664-run-020-series-bug-hunt`
(focused commits, one PR via the Helix platform). **The audit is complete.**

**Result: 14 defects found and fixed, 3 enhancements logged and deferred, 5 micro-tests added.**
All suites green: worker 33, frontend 25, brain 3.

## What Was Verified Live (feature testing ahead of the reviewer's dedicated pass)

- **Koinly tax export** (reviewer priority): seeded ledger rows → CSV columns, `income`
  label, TxHash, Net Worth, RFC3339 dates all correct; empty-ledger header-only CSV;
  422 on unsupported format. The renderer is spec-clean; the only related defect was the
  `confirmed_at` tax-date drift (E11, fixed — now uses the on-chain block timestamp).
- **Claim-fees payload engine**: both actions (Aerodrome `claimFees()`, V4
  `modifyLiquidities` collect) simulate **success** with real live amounts
  (e.g. 0.007720 USDC + 0.3819 NOCK) and live gas ($0.0021/$0.0020, net $0.0359).
- **Record-harvest error paths**: 422 invalid platform, 404 RECEIPT_NOT_FOUND with the
  actionable pending-broadcast guidance — honest.
- **On-chain constants**: NOCK token verified live — `symbol()=NOCK`, `decimals()=16`.
- **Historical API regressions**: `interval` → 400, limit clamp → 200 (0.1.4 fixes intact).
- **RSI/guardrails**: cold-start `rsi_14: None` correct on a fresh DB; Wilder RSI math
  and guardrail logic unit-tested in-repo.
- **WalletConnect degradation**: unconfigured-project hint renders correctly. Live QR
  pairing could not be tested here (no `VITE_WALLETCONNECT_PROJECT_ID` in the sandbox) —
  see "Remaining risks".

## Defects Fixed (14) — registry in `nocknock/TRIAGE.md` §Phase E

| Class | Count | Items |
|---|---|---|
| Data-integrity (zero-mock) | 6 | E1 ($2,500 ETH fallback → honest None), E2 (synthetic Aerodrome 0→999 bracket removed), E7 (MCP fabricated ~$0.005 gas / $5.00 trigger → live gas), E10 (frontend fallback constants → `—`), E11 (`confirmed_at` = block timestamp), E13 (silent $0.05 gas clamp removed) |
| Frontend honesty/UX | 7 | E3 (legend black swatch — the operator's report), E4 (stripped "%" unit), E5 (409 = idempotent success), E6 (error `detail` surfaced), E8 (EXECUTE gated on WalletConnect config), E9 (IN RANGE derived from live state), E10 (also UX) |
| Portability/robustness | 2 | E12 (503 instead of panic on empty wallet config), E14 (Makefile `$(CURDIR)`) |

The operator-reported legend bug (E3) root cause: the `actual` Bar colors per-status via
`<Cell>`s with no series-level `fill`, so Recharts' auto legend fell back to black.
Only 3 actual statuses exist (the worker never emits `fast_running` — the orange branch
was dead code). Fix: explicit legend payload showing only "Target %"; the per-status
color key below the chart documents the real colors. Before/after evidence:
`screenshots/01-…baseline.png` → `04-…fixed.png`.

## Enhancements Deferred (3, filed for the 0.3.0 planning branch)

- **EA — MCP tools for the 0.2.0 Golden Path surface** (ledger/tax-export/claim-payload
  tools absent from `nocknock-mcp`): recommend folding into 0.3.0 tool design.
- **EB — persist/retry failed directive notifications** (no outbox).
- **EC — externalize `v4_position_id`/Aerodrome pool fallbacks** (config validation).

## Remaining Risks / What This Audit Could Not Cover

1. **Live WalletConnect pairing loop** (QR → biometric sign → broadcast → receipt
   record): code-reviewed and unit/contract-covered, but no physical wallet session ran
   in the sandbox. Recommend one end-to-end pairing playtest with the operator's phone
   before trusting the full Golden Path (fits the reviewer's per-feature testing plan).
2. **Record-harvest happy path with a real receipt**: parser is unit-tested with real
   log shapes; a real claim tx would complete the evidence chain.
3. **RSI over time**: needs ~15 ingestion cycles of history to populate on a fresh DB;
   guardrails are unit-tested meanwhile.
4. **sqlx-postgres future-incompat warning**: sqlx 0.7 → 0.9 upgrade remains deferred
   per the dependency policy (needs a live-DB cache regeneration).

## Go/No-Go for 0.3.0 Implementation

**Go.** The bug assessment is complete; nothing here blocks 0.3.0 feature
implementation. Two coordination notes for the 0.3.0 planning branch:
1. Base 0.3.0 implementation branches on `main` **after** this fix branch lands (it
   touches `main.rs`, `App.tsx`, `LiteCockpit.tsx`, `client.ts`, `Makefile`,
   `CHANGELOG.md`, `TRIAGE.md` — rebase collisions otherwise).
2. Consume EA/EB/EC as backlog items rather than re-discovering them.

## Housekeeping for the Operator

- **GitHub issues**: this sandbox has no GitHub credentials, so all 17 items are
  prepared as one-paste bodies in `github-issues/ISSUES.md` (labels included; create
  `patch-0.2.x` `#0e8a16` once). Filing them reproduces the 0.1.x handoff record for
  other agents (Gemini/Antigravity pattern).
- **Release**: `v0.2.1` can ship the fixes on your authorization after CI is green and
  the PR lands (CHANGELOG [Unreleased] section is already populated).
