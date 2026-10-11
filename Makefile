.PHONY: setup build-brain test-brain lint-brain generate-types validate-specs \
        build-worker test-worker run-worker \
        build-frontend test-frontend dev-frontend \
        test-contract clean all \
        tmux tmux-start tmux-stop tmux-attach tmux-status dev \
        release dist db-backup db-restore

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
build-brain:
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
build-worker:
	cd ingestion-worker && SQLX_OFFLINE=true cargo build --release

test-worker:
	cd ingestion-worker && cargo test

run-worker: build-worker
	cd ingestion-worker && DATABASE_URL="postgresql://postgres:postgres@localhost:5432/nocknock" ./target/release/nocknock-ingestion-worker

# ============================================================
# FRONTEND (React + TypeScript) — calm monitoring portal
# ============================================================
build-frontend:
	cd frontend && npm run build

test-frontend:
	cd frontend && npm test -- --run

dev-frontend:
	cd frontend && npm run dev

# ============================================================
# CONTRACT TESTS — verify backend ↔ frontend contract
# ============================================================
test-contract: build-worker
	@echo "Running frontend contract tests..."
	cd frontend && npm test -- --run

# ============================================================
# TMUX ORCHESTRATION — persistent background execution
# ============================================================
TMUX_SESSION := nocknock

tmux: tmux-start

tmux-start: build-worker
	@echo "Starting NockNock in tmux session '$(TMUX_SESSION)'..."
	@tmux has-session -t $(TMUX_SESSION) 2>/dev/null && tmux kill-session -t $(TMUX_SESSION) || true
	@tmux new-session -d -s $(TMUX_SESSION) -n "worker" \
		"cd /Users/yebyen/w/nocknock/ingestion-worker && DATABASE_URL=\"postgresql://postgres:postgres@localhost:5432/nocknock\" ./target/release/nocknock-ingestion-worker; exec zsh"
	@tmux new-window -t $(TMUX_SESSION):1 -n "frontend" \
		"cd /Users/yebyen/w/nocknock/frontend && npm run dev -- --host 0.0.0.0 --port 3000; exec zsh"
	@tmux select-window -t $(TMUX_SESSION):0
	@echo "=========================================================="
	@echo "NockNock is running in tmux session '$(TMUX_SESSION)'!"
	@echo "  • Ingestion API: http://localhost:8080"
	@echo "  • Frontend Web:  http://localhost:3000"
	@echo ""
	@echo "Commands:"
	@echo "  make tmux-attach  (or 'tmux attach -t $(TMUX_SESSION)')"
	@echo "  make tmux-stop    (or 'tmux kill-session -t $(TMUX_SESSION)')"
	@echo "  make tmux-status  (or 'tmux ls')"
	@echo "=========================================================="

tmux-attach:
	tmux attach -t $(TMUX_SESSION)

tmux-stop:
	@tmux kill-session -t $(TMUX_SESSION) 2>/dev/null && echo "Stopped tmux session '$(TMUX_SESSION)'" || echo "No active '$(TMUX_SESSION)' tmux session found"

tmux-status:
	@tmux ls 2>/dev/null || echo "No tmux sessions running"

# ============================================================
# FULL PIPELINE
# ============================================================
all: build-worker test-worker build-frontend test-frontend

clean:
	cd brain && cargo clean || true
	cd ingestion-worker && cargo clean || true
	cd frontend && rm -rf dist node_modules/.cache || true
	rm -rf /tmp/wit-check

# ============================================================
# DEVELOPMENT SHORTCUTS
# ============================================================
dev: tmux-start

# ============================================================
# RELEASE & PACKAGING MACHINERY
# ============================================================
release: build-brain build-worker build-frontend

dist: release
	@mkdir -p dist
	@cp brain/target/$(RUSTUP_TARGET)/release/nocknock_brain.wasm dist/ 2>/dev/null || cp brain/target/wasm32-wasip1/release/nocknock_brain.wasm dist/ 2>/dev/null || true
	@cp ingestion-worker/target/release/nocknock-ingestion-worker dist/
	@tar -czf dist/nocknock-frontend.tar.gz -C frontend/dist .
	@echo "=========================================================="
	@echo "Release artifacts packaged in dist/:"
	@ls -lh dist/
	@echo "=========================================================="

# ============================================================
# DATABASE OPS & DISASTER RECOVERY
# ============================================================
db-backup:
	@mkdir -p backups
	@BACKUP_FILE="backups/nocknock_$$(date +%Y%m%d_%H%M%S).dump"; \
	pg_dump -h localhost -p 5432 -U postgres -d nocknock -F c -b -f "$$BACKUP_FILE"; \
	echo "✅ Database backup created at: $$BACKUP_FILE"

db-restore:
	@test -n "$(FILE)" || (echo "❌ Usage: make db-restore FILE=backups/nocknock_xxx.dump" && exit 1)
	pg_restore -h localhost -p 5432 -U postgres -d nocknock --clean --if-exists $(FILE)
	@echo "✅ Database restored from $(FILE)"