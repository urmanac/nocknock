# Ready-to-File GitHub Issues — 0.2.x Triage Cycle

No GitHub API credentials exist in this sandbox (no `gh`, no token), so issues are
prepared here for one-paste filing. Labels to create once: `patch-0.2.x` (`#0e8a16`).
Common labels: `bug` + `backend`/`frontend` (+ `data-integrity` where noted) + `patch-0.2.x`.
Enhancements: `enhancement` + component label + `patch-0.2.x` (or `0.3.0` where deferred).

---

## Issue 1 — `fix(worker)`: ETH price lookup still falls back to hardcoded $2,500
**Labels:** `bug` `backend` `data-integrity` `patch-0.2.x`
**Component:** `ingestion-worker/src/main.rs:1591-1619` (`fetch_eth_price_usd`), call site `main.rs:1996`

**Defect.** `fetch_eth_price_usd` returns the constant `2500.0` when the Aerodrome
vAMM-WETH/USDC reserves call fails. The comment calls it a "documented conservative peg",
but presenting a fabricated ETH price as live truth violates the zero-mock invariant
(fixing this class was the 0.1.4 release's headline). Blast radius: the Golden Path
`GET /v1/execute/claim-fees/payload` — `gas_usd`, `estimated_gas_usd`, and
`net_after_gas_usd` are all silently wrong whenever the price RPC fails, which distorts
the claim/no-claim decision for real money.

**Fix.** Return `Option<f64>`; when `None`, mark gas as unavailable (the handler already
degrades honestly when gas estimation fails) instead of valuing gas at a fictional price.

**Repro.** Temporarily point `alchemy_api_keys` at an invalid key, call
`/v1/execute/claim-fees/payload`: `estimated_gas_usd` is computed from $2,500 ETH.

---

## Issue 2 — `fix(backend+frontend)`: Tick Bracket chart draws a fabricated Aerodrome 0→999 bracket
**Labels:** `bug` `backend` `frontend` `data-integrity` `patch-0.2.x`
**Component:** `ingestion-worker/src/main.rs:3042-3048` (placeholder bracket), `frontend/src/App.tsx:171-179,453-466`

**Defect.** `compute_derived_metrics` appends a synthetic Aerodrome bracket
(`lower_price: "0.00000"`, `upper_price: "999.00000"`, `in_range: true`,
`distance_bps: 0`). The Tick Bracket Hypervisor chart renders it as if it were live
on-chain truth: a full-width amber bar from $0 to ~$1,000, stretching the X-axis so the
real V4 bracket (0.0124–2.028) is visually crushed to a sliver. Screenshot evidence:
`01-deviation-matrix-legend-bug-baseline.png` (top-left panel).

**Fix.** Do not emit the placeholder bracket; the frontend renders the real V4 bracket
(brackets stay index-labeled: `[0]` = V4). Aerodrome liquidity remains visible in the
NOCK-stack and deviation panels, which already report Tranche 2.

---

## Issue 3 — `fix(frontend)`: Deviation matrix legend shows a black swatch for "Actual %"
**Labels:** `bug` `frontend` `patch-0.2.x`
**Component:** `frontend/src/App.tsx:488-496`

**Defect.** The `actual` Bar colors its cells per-status (green/amber/red) but has no
series-level `fill`, so Recharts' default Legend renders the "Actual %" swatch black —
a color the bars can never use (reported by the operator; see screenshot).

**Fix.** Filter the "Actual %" entry out of the Legend payload; the manual status legend
below the chart already maps the three real colors. (The `fast_running` orange branch in
the Cell mapping is dead code — the backend never emits that status.)

---

## Issue 4 — `fix(frontend)`: StatCard silently strips "%" from Floor Health value
**Labels:** `bug` `frontend` `patch-0.2.x`
**Component:** `frontend/src/App.tsx:296` (value `"...%"`) + `App.tsx:665` (StatCard parseFloat)

**Defect.** StatCard runs every value through `parseFloat(value.replace(/,/g,''))`, so the
Floor Health card's `"106.3%"` renders as bare `106.3` with the unit dropped (live-verified).
A percent value displayed without its unit is misleading in a state room.

**Fix.** Pass the numeric value and `suffix="%"` (StatCard already supports `suffix`).

---

## Issue 5 — `fix(frontend)`: Idempotent harvest re-record (409 ALREADY_RECORDED) shown as failure
**Labels:** `bug` `frontend` `backend` `patch-0.2.x`
**Component:** `frontend/src/api/client.ts:97-117`, `frontend/src/components/LiteCockpit.tsx:148-170`, worker `main.rs:2209-2215`

**Defect.** When a record-harvest response is lost and the client retries, the worker
correctly replies 409 `ALREADY_RECORDED` — but the Lite Cockpit treats it as a failure
("RETRY" state) even though the harvest IS in the ledger. The client also discards the
HTTP status, so the component can't distinguish "already recorded" from real errors.

**Fix.** Attach `status` to errors thrown by `request()`; in `recordWithPendingHandling`,
treat 409 as success (state `recorded`).

---

## Issue 6 — `fix(frontend)`: API client surfaces machine error codes, drops actionable detail
**Labels:** `bug` `frontend` `patch-0.2.x`
**Component:** `frontend/src/api/client.ts:108`

**Defect.** `request()` prefers `err.error` (the machine code, e.g. `RECEIPT_NOT_FOUND`)
over `err.detail` (the human guidance: "…may still be pending broadcast in the wallet…").
Users see code strings instead of the remediation text the backend carefully provides.

**Fix.** Prefer `detail`, then `message`, then `error`.

---

## Issue 7 — `fix(backend)`: MCP text formatters fabricate gas estimates and stale thresholds
**Labels:** `bug` `backend` `data-integrity` `patch-0.2.x`
**Component:** `ingestion-worker/src/main.rs:970-999` (`format_claim_simulation_text`), `ingestion-worker/src/bin/mcp.rs:235-263,355-372`, per-fee fallback `main.rs:980`

**Defect.** MCP-facing claim simulation text hardcodes "Estimated Base Gas: ~$0.005 USD",
computes "Net Accretion" by subtracting that constant, defaults missing per-fee gas to
"0.005", and prints a stale "Execution Trigger: >= $5.00" threshold — all contradicting
the live engine (claim_now at ≥2× gas, dynamic gas from `gas_break_even_usd`). Zero-mock
violations served to MCP clients (Claude Desktop, Antigravity, Mecris).

**Fix.** Use the snapshot's real gas figures (`contract_fees[].gas_break_even_usd`,
`global_status.gas_to_fee_delta_usd`); print "gas estimate unavailable" when absent; drop
the fabricated $5.00 trigger text.

---

## Issue 8 — `fix(frontend)`: Lite Cockpit renders EXECUTE buttons while WalletConnect is unconfigured
**Labels:** `bug` `frontend` `patch-0.2.x`
**Component:** `frontend/src/components/LiteCockpit.tsx:259-305`

**Defect.** The "not configured" hint renders, but the EXECUTE buttons below it render
regardless (live-verified). Clicking one can only fail with "No paired wallet session".
Buttons must be suppressed or disabled whenever pairing is unavailable.

**Fix.** Gate the action rows on `walletConnectConfigured()`.

---

## Issue 9 — `fix(frontend)`: Tick bracket subtitle hardcodes "(IN RANGE)"
**Labels:** `bug` `frontend` `patch-0.2.x`
**Component:** `frontend/src/App.tsx:470`

**Defect.** The V4 range subtitle always appends "(IN RANGE)" in green regardless of the
bracket's live `in_range` field. If spot exits the bracket, the dashboard still claims
IN RANGE — dishonest status display.

**Fix.** Derive the label/color from `brackets[0].in_range` (green "IN RANGE" / amber
"OUT OF RANGE").

---

## Issue 10 — `fix(frontend)`: Residual mock fallback constants in dashboard text
**Labels:** `bug` `frontend` `data-integrity` `patch-0.2.x`
**Component:** `frontend/src/App.tsx:202, 285, 347`

**Defect.** Hardcoded fallbacks presented as live truth when fields are missing:
floor target `: 100000` (L202), bracket prices `'0.01237'`/`'2.02783'` (L347), and
`'0.00'` NOCK price subtitle (L285). Mostly unreachable today, but they violate the
zero-mock invariant and will silently mask missing data later.

**Fix.** Render `—`/loading placeholders when the fields are absent.

---

## Issue 11 — `fix(worker)`: Harvest `confirmed_at` uses recording time, not block timestamp
**Labels:** `bug` `backend` `data-integrity` `patch-0.2.x`
**Component:** `ingestion-worker/src/main.rs:2172` (NOW() in insert)

**Defect.** `record_harvest_handler` writes `NOW()` as `confirmed_at`. For the
pending-broadcast retry path the recording can happen long after confirmation, giving the
Koinly export a wrong tax date. The receipt already carries `blockNumber`; the block
timestamp is one `eth_getBlockByNumber` away.

**Fix.** Fetch the block timestamp and store it as `confirmed_at` (fall back to `NOW()`
only if the block query fails, and say so in the response).

---

## Issue 12 — `fix(worker)`: `tracked_wallets[0]` / `tracked_pools.first()` unguarded indexing
**Labels:** `bug` `backend` `patch-0.2.x`
**Component:** `ingestion-worker/src/main.rs:1850, 1876-1881, 2106`

**Defect.** Golden-path handlers index `config.tracked_wallets[0]` directly; an empty
list panics the handler (request dies with a connection error instead of a clean 5xx).
Config validation at startup doesn't guarantee a non-empty wallet list.

**Fix.** Return a clear `SERVICE_UNAVAILABLE` ("no tracked wallet configured") or validate
non-empty at boot.

---

## Issue 13 — `fix(worker)`: Gas cost silently clamped to ≤$0.05
**Labels:** `bug` `backend` `data-integrity` `patch-0.2.x`
**Component:** `ingestion-worker/src/main.rs:2398-2403`

**Defect.** `usd_gas_cost.clamp(0.002, 0.05)` caps gas break-even at $0.05; during fee
spikes real gas above the cap is silently understated, biasing toward "claim now". The
`+0.002` add is an undocumented constant (plausibly an L1-data buffer — document or drop).

**Fix.** Keep a small documented floor if needed, remove the silent upper clamp, and
document the buffer constant.

---

## Issue 14 — `fix(build)`: Makefile tmux targets hardcode a macOS path
**Labels:** `bug` `backend` `patch-0.2.x`
**Component:** `Makefile` (`tmux-start` et al., `/Users/yebyen/w/nocknock/...`)

**Defect.** `make tmux` cannot work on any machine but the original author's (hardcoded
absolute macOS path). Broken on Linux/CI sandboxes (encountered during this audit).

**Fix.** Use `$(CURDIR)` / `$(realpath ...)` instead of absolute hardcoded paths.

---

## Enhancement A — `feat(backend)`: MCP tools for the 0.2.0 Golden Path surface
**Labels:** `enhancement` `backend` `patch-0.2.x`
**Component:** `ingestion-worker/src/bin/mcp.rs`

The stdio MCP server exposes only the three 0.1.x-era tools. Harvest ledger inspection,
tax export, and (optionally) claim-payload retrieval are missing — MCP agents can't see
any 0.2.0 feature. Defer detailed tool design to the 0.3.0 planning branch; filing so the
gap is tracked.

## Enhancement B — `feat(backend)`: Persist/retry failed directive-transition notifications
**Labels:** `enhancement` `backend` `patch-0.2.x`
**Component:** `ingestion-worker/src/main.rs:1288-1301, 1375-1400`

Dispatch failures are logged and dropped; a directive transition during a Telegram/Pushover
outage is never re-delivered (no outbox/pending queue). Low priority; silent-first-boot and
failure-isolation behavior are otherwise correct.

## Enhancement C — `feat(worker)`: Externalize remaining hardcoded chain constants
**Labels:** `enhancement` `backend` `patch-0.2.x`
**Component:** `main.rs:1876-1881` (Aerodrome pool fallback), `main.rs:1935, 2493` (`v4_position_id.unwrap_or(3126000)`)

`v4_position_id` and the Aerodrome pool silently fall back to hardcoded values when config
omits them. They are correct for this operator but the pattern contradicts the
config-externalization direction (0.1.x issue #6). Require or validate at boot instead.
