# NockNock — Asset State Room

> **"Observe and Protect"** — An asynchronous, low-friction portfolio monitoring system that eliminates active maintenance overhead.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         NOCKNOCK SYSTEM ARCHITECTURE                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌──────────────────┐      HTTP/JSON (REST)       ┌──────────────────┐   │
│   │  Ingestion       │ ◄─────────────────────────► │  Frontend        │   │
│   │  Backend Engine  │    /v1/snapshot/latest      │  (React/TS)      │   │
│   │  (Async Worker)  │    /v1/snapshot/historical  │  Calm Monitoring │   │
│   └────────┬─────────┘    /v1/snapshot/metadata    └────────┬─────────┘   │
│           │                                                │              │
│           │  ┌──────────────┐                               │              │
│           └► │  WASM Brain  │  (pure logic, no I/O)         │              │
│              │  (Rust)      │  compute-snapshot             │              │
│              │              │  derive-metrics               │              │
│              │              │  check-freshness              │              │
│              └──────────────┘                               │              │
│                     ▲                                       │              │
│                     │ host imports                          │              │
│              ┌──────┴──────┐                                 │              │
│              │  Host Env   │                                 │              │
│              │  (Go)       │                                 │              │
│              │  • HTTP     │                                 │              │
│              │  • Storage  │                                 │              │
│              │  • Logger   │                                 │              │
│              └─────────────┘                                 │              │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Two completely isolated modules communicating exclusively via a stable network API contract:**

1. **Ingestion Backend Engine** — Silent, detached operational monitor
   - Periodic Web3 data fetching (Alchemy premium APIs)
   - Pure WASM logic for computations (deterministic, testable)
   - Immutable snapshot storage with SHA-256 state hashes
   - REST API with sync metadata on every payload

2. **Unified Presentation Front-End** — Calm monitoring portal
   - Freshness validation (client-side, no push)
   - Non-obtrusive stale data indicator
   - Storyboard visual layout (7 dashboard panels)
   - Decoupled — any consumer gets identical data

## Spec-Driven Development

All contracts live in `specs/core/`:

| File | Purpose |
|------|---------|
| `ingestion.wit` | WASM Interface Types (pure logic contract) |
| `snapshot.yaml` | Canonical data models (OpenAPI 3.0 components) |
| `api.yaml` | REST API contract (OpenAPI 3.0) |

These are the **single source of truth**. Code is generated from them:

```bash
# Generate TypeScript API client for frontend
make generate-types

# Validate all specs
make validate-specs
```

## Quick Start

### Prerequisites
- Docker & Docker Compose
- Rust (for WASM brain): `rustup target add wasm32-wasip1`
- Go 1.23+ (for ingestion worker)
- Node 20+ (for frontend)

### Local Development

```bash
# 1. Build WASM brain
make build-brain

# 2. Start infrastructure (PostgreSQL + worker + frontend)
docker-compose up -d

# 3. Frontend at http://localhost:3000
#    API at http://localhost:8080/v1
```

### Manual Development (without Docker)

```bash
# Terminal 1: Start PostgreSQL
docker run -d --name nocknock-db -e POSTGRES_PASSWORD=postgres -p 5432:5432 timescale/timescaledb:latest-pg16

# Terminal 2: Run ingestion worker (dev mode)
cd ingestion-worker
cp config.yaml.example config.yaml
# Edit config.yaml with your Alchemy keys
go run ./cmd/worker -dev -config config.yaml

# Terminal 3: Run frontend
cd frontend
npm install
npm run dev
```

## Project Structure

```
nocknock/
├── specs/
│   └── core/
│       ├── ingestion.wit       # WASM logic contract
│       ├── snapshot.yaml       # Data models (OpenAPI components)
│       └── api.yaml            # REST API contract
├── brain/                      # Rust WASM (pure logic)
│   ├── src/
│   │   ├── lib.rs              # Exports ingestion.wit functions
│   │   └── *.rs                # Implementation modules
│   └── Cargo.toml
├── ingestion-worker/           # Go host runtime
│   ├── cmd/worker/main.go      # Scheduler + HTTP server + WASM host
│   ├── go.mod
│   └── Dockerfile
├── frontend/                   # React + TypeScript
│   ├── src/
│   │   ├── api/                # Generated API client
│   │   ├── hooks/useFreshness.ts  # Core freshness contract
│   │   ├── components/         # Storyboard panels
│   │   └── types/              # TypeScript types
│   ├── package.json
│   └── vite.config.ts
├── docker-compose.yml
├── init-db.sql
└── Makefile
```

## Key Contracts

### Snapshot Envelope (every API response)
```json
{
  "meta": {
    "generated_at": "2026-10-09T17:23:29.123Z",
    "state_hash": "a1b2c3d4e5f6...",
    "version": "1.0.0",
    "snapshot_id": "01ARZ3NDEKTSV4RRFFQ69G5FAV"
  },
  "data": { /* SnapshotData */ }
}
```

### Freshness Validation (Frontend)
```typescript
const { freshness } = useFreshness(fetchSnapshot, {
  syncWindowSeconds: 300,  // Must match backend config
  pollIntervalMs: 10_000,
});

// freshness.isFresh → boolean
// freshness.ageSeconds → number
// freshness.stateHash → string
```

### Stale Data Banner (Non-Obtrusive)
- Shows when `age > syncWindowSeconds`
- Fixed position, low visual priority
- Dismissible but auto-reappears on next stale check
- **No blinking, no countdowns, no modals**

## Storyboard Visual Layout (Frontend Panels)

| Panel | Data Source | Purpose |
|-------|-------------|---------|
| **Global Status Bar** | `global_status` | Spot metrics, balances, fee deltas, high-water mark |
| **Asset Aggregation** | `asset_aggregation_series` | Stacked area: safe-haven vs active protocol |
| **Total Value vs Baseline** | `total_value_vs_baseline` | Dual-axis: portfolio value vs token price + HW mark |
| **Tick Bracket Hypervisor** | `tick_bracket_hypervisor` | Cross-section: price vs hard-bounded range brackets |
| **Portfolio Deviation** | `portfolio_deviation_matrix` | Target vs actual weights, over/under-funded anchors |
| **Structural Channels** | `structural_channels` | Regression corridors, ghosted on break |
| **Strategy Counterfactual** | `strategy_counterfactual` | Active vs passive (cash-out) growth curves |
| **Velocity Gas Sandbox** | `velocity_gas_sandbox` | Claimable fees vs gas break-even per contract |

## Testing

```bash
# Brain unit tests (pure logic)
make test-brain

# Worker integration tests
make test-worker

# Frontend unit tests
make test-frontend

# Contract tests (requires running worker)
make test-contract

# All tests
make all
```

## Design Principles (from `useless` & `time-tracker`)

1. **Pure logic in WASM** — `ingestion.wit` defines the brain. No HTTP, no DB, no time.
2. **Deterministic envelopes** — Every response has `meta.state_hash` = SHA-256 of canonicalized data.
3. **Decimal strings, never floats** — All amounts, USD values, prices use `string` with `description: "decimal string"`.
4. **Freshness is a client concern** — Backend exposes timestamps; frontend validates locally.
5. **Versioned schemas** — `meta.version` follows semver. Breaking changes = new major version.

## Environment Variables

| Variable | Service | Description |
|----------|---------|-------------|
| `DATABASE_URL` | Worker | PostgreSQL connection string |
| `PORT` | Worker | API server port (default 8080) |
| `VITE_API_BASE` | Frontend | API base URL (default `/v1`) |
| `ALCHEMY_API_KEY_*` | Worker | Premium Alchemy keys (optional, can use config.yaml) |

## License

ISC