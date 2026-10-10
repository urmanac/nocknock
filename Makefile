.PHONY: setup build-brain test-brain lint-brain generate-types validate-specs \
        build-worker test-worker run-worker \
        build-frontend test-frontend dev-frontend \
        test-contract clean all

# ============================================================
# TOOLCHAIN
# ============================================================
RUSTUP_TARGET := wasm32-wasip1
WIT_BINDGEN := wit-bindgen
OPENAPI_TS := npx openapi-typescript
REDOCLY := npx @redocly/cli

# ============================================================
# SETUP
# ============================================================
setup:
	rustup target add $(RUSTUP_TARGET)
	cargo install $(WIT_BINDGEN) --locked || true
	npm install -g openapi-ts @redocly/cli || true

# ============================================================
# SPEC VALIDATION & CODE GENERATION
# ============================================================
validate-specs:
	@echo "Validating OpenAPI specs..."
	$(REDOCLY) lint specs/core/api.yaml specs/core/snapshot.yaml
	@echo "Validating WIT..."
	$(WIT_BINDGEN) rust specs/core/ingestion.wit --out-dir /tmp/wit-check

generate-types: validate-specs
	@echo "Generating Rust WASM bindings..."
	$(WIT_BINDGEN) rust specs/core/ingestion.wit --out-dir brain/src/bindings
	@echo "Generating TypeScript API client (requires frontend deps)..."
	cd frontend && npx openapi-typescript ../specs/core/api.yaml -o src/api/schema.ts || true

# ============================================================
# BRAIN (Rust WASM) — pure logic, no deps on HTTP/DB/time
# ============================================================
build-brain: validate-specs
	cd brain && cargo build --target $(RUSTUP_TARGET) --release

test-brain:
	cd brain && cargo test

lint-brain:
	cd brain && cargo clippy --target $(RUSTUP_TARGET) -- -D warnings

format-brain:
	cd brain && cargo fmt

# ============================================================
# INGESTION WORKER (Host runtime) — scheduler + storage + API
# ============================================================
build-worker: build-brain
	cd ingestion-worker && go build -o bin/worker ./cmd/worker

test-worker:
	cd ingestion-worker && go test ./...

run-worker: build-worker
	cd ingestion-worker && ./bin/worker

# ============================================================
# FRONTEND (React + TypeScript) — calm monitoring portal
# ============================================================
build-frontend: generate-types
	cd frontend && npm run build

test-frontend:
	cd frontend && npm run test

dev-frontend: generate-types
	cd frontend && npm run dev

# ============================================================
# CONTRACT TESTS — verify backend ↔ frontend contract
# ============================================================
test-contract: build-worker build-frontend
	@echo "Starting test worker on :8080..."
	cd ingestion-worker && ./bin/worker --test-mode --port 8080 & \
	WORKER_PID=$$!; \
	sleep 2; \
	cd frontend && npm run test:contract; \
	kill $$WORKER_PID

# ============================================================
# FULL PIPELINE
# ============================================================
all: validate-specs build-brain test-brain build-worker test-worker build-frontend test-frontend

clean:
	cd brain && cargo clean || true
	cd ingestion-worker && rm -rf bin || true
	cd frontend && rm -rf dist node_modules/.cache || true
	rm -rf /tmp/wit-check

# ============================================================
# DEVELOPMENT SHORTCUTS
# ============================================================
dev: generate-types
	@echo "Starting ingestion worker (dev)..."
	cd ingestion-worker && go run ./cmd/worker --dev & \
	WORKER_PID=$$!; \
	echo "Worker PID: $$WORKER_PID"; \
	echo "Starting frontend dev server..."; \
	cd frontend && npm run dev; \
	kill $$WORKER_PID