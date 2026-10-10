# NockNock Specifications

Single source of truth for the **Ingestion Backend Engine** ↔ **Unified Presentation Front-End** contract.

## Spec Files

| File | Purpose | Consumers |
|------|---------|-----------|
| `core/ingestion.wit` | WASM Interface Types for pure ingestion logic | Rust WASM brain, Go/TS hosts, test mocks |
| `core/snapshot.yaml` | Canonical data models (OpenAPI 3.0 components) | Backend serialization, Frontend TypeScript types, API docs |
| `core/api.yaml` | REST API contract (OpenAPI 3.0) | Backend server, Frontend API client, Integration tests |

## Design Principles (mirroring `useless` & `time-tracker`)

1. **Pure logic in WASM** — `ingestion.wit` defines the brain: `compute-snapshot`, `check-freshness`, `derive-metrics`. No HTTP, no DB, no time. Host provides `http`, `storage`, `logger` imports.

2. **Deterministic envelopes** — Every API response wraps data in `SnapshotEnvelope { meta, data }`. `meta.state_hash` = SHA-256 of canonicalized `data` (sorted keys, fixed field order). Enables:
   - Frontend freshness validation without re-fetching
   - Conditional requests (`If-None-Match: <state_hash>`)
   - Exact cache invalidation

3. **Decimal strings, never floats** — All on-chain amounts, USD values, prices use `type: string` with `description: "decimal string"`. Prevents precision loss across WASM/JSON boundaries.

4. **Freshness is a client concern** — Backend exposes `generated_at` + `state_hash`. Frontend compares `now - generated_at` against `SYNC_WINDOW_SECONDS` (default 300s). Stale → non-obtrusive banner. No push, no websockets, no "act now".

5. **Versioned schemas** — `meta.version` follows semver. Breaking changes = new major version + new API path (`/v2/`).

## Code Generation

```bash
# Generate TypeScript types for frontend
npm install -g @hey-api/openapi-ts
openapi-ts -i specs/core/api.yaml -o frontend/src/api --input-type=openapi

# Generate Rust types for backend
cargo install openapi-generator-cli
openapi-generator-cli generate -i specs/core/api.yaml -g rust-server -o backend/generated

# Validate WIT
wit-bindgen rust specs/core/ingestion.wit --out-dir brain/src/bindings
```

## Validation

```bash
# Validate OpenAPI specs
npm install -g @redocly/openapi-cli
redocly lint specs/core/api.yaml specs/core/snapshot.yaml

# Validate WIT
wit-bindgen rust specs/core/ingestion.wit --out-dir /tmp/check
```

## Contract Testing

The `time-tracker` pattern: integration tests spin up the backend, hit real endpoints, assert:

```typescript
// Freshness contract test
const res = await fetch('/v1/snapshot/latest');
const envelope = await res.json();
const ageSec = (Date.now() - new Date(envelope.meta.generated_at).getTime()) / 1000;
expect(ageSec).toBeLessThan(SYNC_WINDOW_SECONDS); // 300s default

// Hash determinism test
const res1 = await fetch('/v1/snapshot/latest');
const res2 = await fetch('/v1/snapshot/latest');
expect(res1.headers.get('ETag')).toBe(res2.headers.get('ETag')); // same hash = same data
```

## Adding New Metrics

1. Add schema to `snapshot.yaml` under `DerivedMetrics` → new component
2. Update `ingestion.wit` `derive-metrics` output shape
3. Regenerate types (`openapi-ts`, `wit-bindgen`)
4. Implement in WASM brain (`brain/src/derive_metrics.rs`)
5. Frontend consumes new field — no API version bump (additive)

## Directory Structure (target)

```
nocknock/
├── specs/
│   ├── core/
│   │   ├── ingestion.wit       # WASM logic contract
│   │   ├── snapshot.yaml       # Data models (OpenAPI components)
│   │   └── api.yaml            # REST API contract
│   └── README.md
├── brain/                      # Rust WASM (pure logic)
│   ├── src/
│   │   ├── lib.rs              # Exports ingestion.wit functions
│   │   ├── compute_snapshot.rs
│   │   ├── derive_metrics.rs
│   │   └── freshness.rs
│   ├── Cargo.toml
│   └── wit/
│       └── deps/               # wit-deps (logger, http, storage)
├── ingestion-worker/           # Host runtime (Go/Rust/Node)
│   ├── main.go                 # Scheduler → calls WASM → stores → serves API
│   ├── storage/                # TimescaleDB/PostgreSQL/SQLite
│   └── api/                    # Axum/Actix/Spin HTTP server
├── frontend/                   # React/TypeScript (calm monitoring portal)
│   ├── src/
│   │   ├── api/                # Generated from api.yaml
│   │   ├── components/         # Storyboard visual layout
│   │   └── hooks/
│   │       └── useFreshness.ts # Stale-data banner logic
│   └── package.json
└── Makefile                    # Build, test, lint, generate
```