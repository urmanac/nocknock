package main

import (
	"context"
	"encoding/json"
	"flag"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/extism/go-pdk"
	"github.com/gin-gonic/gin"
	"github.com/google/uuid"
	"github.com/robfig/cron/v3"
	"github.com/jackc/pgx/v5/pgxpool"
)

// Config holds all configuration for the ingestion worker
type Config struct {
	DatabaseURL      string        `yaml:"database_url"`
	AlchemyAPIKeys   []string      `yaml:"alchemy_api_keys"`
	TrackedWallets   []string      `yaml:"tracked_wallets"`
	TrackedPools     []string      `yaml:"tracked_pools"`
	TokenAddresses   []string      `yaml:"token_addresses"`
	TrackedRanges    []string      `yaml:"tracked_ranges"`
	FeeContracts     []string      `yaml:"fee_contracts"`
	LPTokens         []string      `yaml:"lp_tokens"`
	SyncWindowSec    int           `yaml:"sync_window_seconds"`
	Schedule         string        `yaml:"schedule"`          // cron expression
	Port             int           `yaml:"port"`              // API server port
	WASMPath         string        `yaml:"wasm_path"`         // path to brain.wasm
	TestMode         bool          `yaml:"test_mode"`         // for contract tests
	DevMode          bool          `yaml:"dev_mode"`          // extra logging
}

var defaultConfig = Config{
	SyncWindowSec: 300,
	Schedule:      "*/5 * * * *", // every 5 minutes
	Port:          8080,
	WASMPath:      "../../brain/target/wasm32-wasip1/release/nocknock_brain.wasm",
}

// SnapshotEnvelope mirrors the API contract
type SnapshotEnvelope struct {
	Meta SnapshotMeta   `json:"meta"`
	Data SnapshotData   `json:"data"`
}

type SnapshotMeta struct {
	GeneratedAt string `json:"generated_at"`
	StateHash   string `json:"state_hash"`
	Version     string `json:"version"`
	SnapshotID  string `json:"snapshot_id"`
}

type SnapshotData struct {
	WalletBalances       []json.RawMessage `json:"wallet_balances"`
	PoolMechanics        []json.RawMessage `json:"pool_mechanics"`
	ConcentratedLiquidity []json.RawMessage `json:"concentrated_liquidity"`
	ContractFees         []json.RawMessage `json:"contract_fees"`
	DerivedMetrics       json.RawMessage   `json:"derived_metrics"`
}

// Worker holds the WASM plugin and database connection
type Worker struct {
	config   Config
	plugin   *pdk.Plugin
	db       *pgxpool.Pool
	latest   *SnapshotEnvelope
	latestMu sync.RWMutex
}

func main() {
	var (
		configPath = flag.String("config", "config.yaml", "Path to config file")
		testMode   = flag.Bool("test-mode", false, "Run in test mode (no cron, fixed port)")
		devMode    = flag.Bool("dev", false, "Development mode")
		port       = flag.Int("port", 0, "Override port")
	)
	flag.Parse()

	cfg := loadConfig(*configPath)
	if *testMode {
		cfg.TestMode = true
	}
	if *devMode {
		cfg.DevMode = true
	}
	if *port > 0 {
		cfg.Port = *port
	}

	// Initialize WASM plugin
	plugin, err := pdk.NewPlugin(cfg.WASMPath, pdk.WithConfig(map[string]string{
		"alchemy_api_keys": join(cfg.AlchemyAPIKeys, ","),
		"tracked_wallets":  join(cfg.TrackedWallets, ","),
		"tracked_pools":    join(cfg.TrackedPools, ","),
		"tracked_ranges":   join(cfg.TrackedRanges, ","),
		"fee_contracts":    join(cfg.FeeContracts, ","),
	}))
	if err != nil {
		log.Fatalf("Failed to load WASM plugin: %v", err)
	}
	defer plugin.Close()

	// Initialize database
	db, err := pgxpool.New(context.Background(), cfg.DatabaseURL)
	if err != nil {
		log.Fatalf("Failed to connect to database: %v", err)
	}
	defer db.Close()

	// Run migrations
	if err := runMigrations(db); err != nil {
		log.Fatalf("Migration failed: %v", err)
	}

	worker := &Worker{
		config: cfg,
		plugin: plugin,
		db:     db,
	}

	// Load latest snapshot from DB
	if err := worker.loadLatestSnapshot(context.Background()); err != nil {
		log.Printf("Warning: could not load latest snapshot: %v", err)
	}

	// Start cron scheduler (unless test mode)
	var cronScheduler *cron.Cron
	if !cfg.TestMode {
		cronScheduler = cron.New()
		_, err = cronScheduler.AddFunc(cfg.Schedule, func() {
			if err := worker.runIngestion(context.Background()); err != nil {
				log.Printf("Ingestion run failed: %v", err)
			}
		})
		if err != nil {
			log.Fatalf("Failed to add cron job: %v", err)
		}
		cronScheduler.Start()
		log.Printf("Ingestion scheduler started: %s", cfg.Schedule)
	}

	// Start HTTP server
	router := worker.setupRouter()
	server := &http.Server{
		Addr:    fmt.Sprintf(":%d", cfg.Port),
		Handler: router,
	}

	go func() {
		log.Printf("API server listening on :%d", cfg.Port)
		if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			log.Fatalf("Server failed: %v", err)
		}
	}()

	// Wait for shutdown signal
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)
	<-sigCh

	log.Println("Shutting down...")
	if cronScheduler != nil {
		cronScheduler.Stop()
	}
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	server.Shutdown(ctx)
}

func loadConfig(path string) Config {
	cfg := defaultConfig
	if _, err := os.Stat(path); err == nil {
		data, err := os.ReadFile(path)
		if err != nil {
			log.Printf("Warning: could not read config: %v", err)
			return cfg
		}
		if err := yaml.Unmarshal(data, &cfg); err != nil {
			log.Printf("Warning: could not parse config: %v", err)
		}
	}
	// Env var overrides
	if v := os.Getenv("DATABASE_URL"); v != "" {
		cfg.DatabaseURL = v
	}
	if v := os.Getenv("PORT"); v != "" {
		fmt.Sscanf(v, "%d", &cfg.Port)
	}
	return cfg
}

func join(slice []string, sep string) string {
	if len(slice) == 0 {
		return ""
	}
	result := slice[0]
	for _, s := range slice[1:] {
		result += sep + s
	}
	return result
}

// runIngestion executes one ingestion cycle via WASM
func (w *Worker) runIngestion(ctx context.Context) error {
	now := time.Now().UTC().Format(time.RFC3339Nano)

	input := map[string]interface{}{
		"config": map[string]interface{}{
			"alchemy_api_keys":      w.config.AlchemyAPIKeys,
			"tracked_wallets":       w.config.TrackedWallets,
			"tracked_pools":         w.config.TrackedPools,
			"token_addresses":       w.config.TokenAddresses,
			"tracked_ranges":        w.config.TrackedRanges,
			"fee_contracts":         w.config.FeeContracts,
			"lp_tokens":             w.config.LPTokens,
			"sync_window_seconds":   w.config.SyncWindowSec,
		},
		"now": now,
	}

	inputJSON, _ := json.Marshal(input)

	// Call WASM compute-snapshot
	outputPtr, err := w.plugin.Call("compute-snapshot", inputJSON)
	if err != nil {
		return fmt.Errorf("WASM call failed: %w", err)
	}

	outputJSON := pdk.ReadMemory(outputPtr)
	pdk.FreeMemory(outputPtr)

	var envelope SnapshotEnvelope
	if err := json.Unmarshal(outputJSON, &envelope); err != nil {
		return fmt.Errorf("failed to parse WASM output: %w", err)
	}

	// Store in database
	if err := w.storeSnapshot(ctx, &envelope); err != nil {
		return fmt.Errorf("failed to store snapshot: %w", err)
	}

	// Update in-memory latest
	w.latestMu.Lock()
	w.latest = &envelope
	w.latestMu.Unlock()

	log.Printf("Ingestion complete: snapshot_id=%s, hash=%s", envelope.Meta.SnapshotID, envelope.Meta.StateHash[:16])
	return nil
}

func (w *Worker) storeSnapshot(ctx context.Context, env *SnapshotEnvelope) error {
	dataJSON, _ := json.Marshal(env.Data)
	_, err := w.db.Exec(ctx, `
		INSERT INTO snapshots (snapshot_id, generated_at, state_hash, version, data)
		VALUES ($1, $2, $3, $4, $5)
		ON CONFLICT (snapshot_id) DO UPDATE SET
			generated_at = EXCLUDED.generated_at,
			state_hash = EXCLUDED.state_hash,
			version = EXCLUDED.version,
			data = EXCLUDED.data
	`, env.Meta.SnapshotID, env.Meta.GeneratedAt, env.Meta.StateHash, env.Meta.Version, dataJSON)
	return err
}

func (w *Worker) loadLatestSnapshot(ctx context.Context) error {
	row := w.db.QueryRow(ctx, `
		SELECT snapshot_id, generated_at, state_hash, version, data
		FROM snapshots
		ORDER BY generated_at DESC
		LIMIT 1
	`)

	var env SnapshotEnvelope
	var dataJSON []byte
	if err := row.Scan(&env.Meta.SnapshotID, &env.Meta.GeneratedAt, &env.Meta.StateHash, &env.Meta.Version, &dataJSON); err != nil {
		return err
	}
	if err := json.Unmarshal(dataJSON, &env.Data); err != nil {
		return err
	}

	w.latestMu.Lock()
	w.latest = &env
	w.latestMu.Unlock()
	return nil
}

func (w *Worker) setupRouter() *gin.Engine {
	if !w.config.DevMode {
		gin.SetMode(gin.ReleaseMode)
	}
	r := gin.New()
	r.Use(gin.Recovery())
	if w.config.DevMode {
		r.Use(gin.Logger())
	}

	// Health endpoint
	r.GET("/health", func(c *gin.Context) {
		w.latestMu.RLock()
		latest := w.latest
		w.latestMu.RUnlock()

		status := "ok"
		var lastSync string
		var lagSeconds int64
		if latest != nil {
			lastSync = latest.Meta.GeneratedAt
			if t, err := time.Parse(time.RFC3339Nano, lastSync); err == nil {
				lagSeconds = int64(time.Since(t).Seconds())
			}
		} else {
			status = "degraded"
		}

		c.JSON(http.StatusOK, gin.H{
			"status":              status,
			"last_sync":           lastSync,
			"ingestion_lag_seconds": lagSeconds,
		})
	})

	// Latest snapshot
	r.GET("/v1/snapshot/latest", func(c *gin.Context) {
		w.latestMu.RLock()
		latest := w.latest
		w.latestMu.RUnlock()

		if latest == nil {
			c.JSON(http.StatusServiceUnavailable, gin.H{
				"error":   "NO_SNAPSHOT",
				"message": "No snapshot available yet",
			})
			return
		}

		// Conditional GET support
		if etag := c.GetHeader("If-None-Match"); etag != "" && etag == latest.Meta.StateHash {
			c.Status(http.StatusNotModified)
			return
		}

		c.Header("ETag", latest.Meta.StateHash)
		c.Header("Last-Modified", latest.Meta.GeneratedAt)
		c.Header("Cache-Control", fmt.Sprintf("public, max-age=%d, stale-while-revalidate=%d", w.config.SyncWindowSec, w.config.SyncWindowSec*2))
		c.JSON(http.StatusOK, latest)
	})

	// Historical snapshots
	r.GET("/v1/snapshot/historical", func(c *gin.Context) {
		interval := c.DefaultQuery("interval", "1d")
		limit := c.DefaultQuery("limit", "100")
		before := c.Query("before")
		after := c.Query("after")

		// Build query based on interval
		// For simplicity, just return recent snapshots
		query := `
			SELECT snapshot_id, generated_at, state_hash, version, data
			FROM snapshots
			WHERE 1=1
		`
		args := []interface{}{}
		argIdx := 1

		if before != "" {
			query += fmt.Sprintf(" AND generated_at < $%d", argIdx)
			args = append(args, before)
			argIdx++
		}
		if after != "" {
			query += fmt.Sprintf(" AND generated_at >= $%d", argIdx)
			args = append(args, after)
			argIdx++
		}

		query += fmt.Sprintf(" ORDER BY generated_at DESC LIMIT $%d", argIdx)
		args = append(args, limit)

		rows, err := w.db.Query(ctx, query, args...)
		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": "QUERY_FAILED"})
			return
		}
		defer rows.Close()

		var snapshots []SnapshotEnvelope
		for rows.Next() {
			var env SnapshotEnvelope
			var dataJSON []byte
			if err := rows.Scan(&env.Meta.SnapshotID, &env.Meta.GeneratedAt, &env.Meta.StateHash, &env.Meta.Version, &dataJSON); err != nil {
				continue
			}
			json.Unmarshal(dataJSON, &env.Data)
			snapshots = append(snapshots, env)
		}

		c.JSON(http.StatusOK, gin.H{
			"snapshots": snapshots,
			"pagination": gin.H{
				"limit":      limit,
				"returned":   len(snapshots),
				"has_more":   len(snapshots) == 100, // simplify
			},
		})
	})

	// Metadata only (for freshness polling)
	r.GET("/v1/snapshot/metadata", func(c *gin.Context) {
		limit := c.DefaultQuery("limit", "20")
		rows, err := w.db.Query(ctx, `
			SELECT snapshot_id, generated_at, state_hash
			FROM snapshots
			ORDER BY generated_at DESC
			LIMIT $1
		`, limit)
		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": "QUERY_FAILED"})
			return
		}
		defer rows.Close()

		type MetaEntry struct {
			SnapshotID  string `json:"snapshot_id"`
			GeneratedAt string `json:"generated_at"`
			StateHash   string `json:"state_hash"`
			AgeSeconds  int64  `json:"age_seconds"`
		}

		var entries []MetaEntry
		for rows.Next() {
			var e MetaEntry
			if err := rows.Scan(&e.SnapshotID, &e.GeneratedAt, &e.StateHash); err != nil {
				continue
			}
			if t, err := time.Parse(time.RFC3339Nano, e.GeneratedAt); err == nil {
				e.AgeSeconds = int64(time.Since(t).Seconds())
			}
			entries = append(entries, e)
		}

		c.JSON(http.StatusOK, gin.H{"snapshots": entries})
	})

	// Specific snapshot by ID
	r.GET("/v1/snapshot/:id", func(c *gin.Context) {
		id := c.Param("id")
		row := w.db.QueryRow(ctx, `
			SELECT snapshot_id, generated_at, state_hash, version, data
			FROM snapshots
			WHERE snapshot_id = $1
		`, id)

		var env SnapshotEnvelope
		var dataJSON []byte
		if err := row.Scan(&env.Meta.SnapshotID, &env.Meta.GeneratedAt, &env.Meta.StateHash, &env.Meta.Version, &dataJSON); err != nil {
			c.JSON(http.StatusNotFound, gin.H{"error": "NOT_FOUND", "message": "Snapshot not found"})
			return
		}
		json.Unmarshal(dataJSON, &env.Data)
		c.JSON(http.StatusOK, env)
	})

	return r
}

func runMigrations(db *pgxpool.Pool) error {
	_, err := db.Exec(context.Background(), `
		CREATE TABLE IF NOT EXISTS snapshots (
			snapshot_id   UUID PRIMARY KEY,
			generated_at  TIMESTAMPTZ NOT NULL,
			state_hash    CHAR(64) NOT NULL,
			version       VARCHAR(20) NOT NULL,
			data          JSONB NOT NULL,
			created_at    TIMESTAMPTZ DEFAULT NOW()
		);
		CREATE INDEX IF NOT EXISTS idx_snapshots_generated_at ON snapshots(generated_at DESC);
	`)
	return err
}