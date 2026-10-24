use anyhow::Result;
use axum::{
    extract::Extension,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, TimeZone, Utc};
use config::Config;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

// ============================================================
// Configuration
// ============================================================

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OperatorProfile {
    #[serde(default = "default_treasury_floor")]
    pub treasury_floor_nock: f64,
    #[serde(default = "default_tax_reserve_ratio")]
    pub tax_reserve_ratio: f64,
    #[serde(default = "default_target_weights")]
    pub target_weights_bps: TargetWeightsBps,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TargetWeightsBps {
    #[serde(default = "default_treasury_bps")]
    pub treasury: i32,
    #[serde(default = "default_uniswap_lp_bps")]
    pub uniswap_lp: i32,
    #[serde(default = "default_cash_bps")]
    pub cash: i32,
}

fn default_treasury_floor() -> f64 { 100000.0 }
fn default_tax_reserve_ratio() -> f64 { 0.40 }
fn default_treasury_bps() -> i32 { 8000 }
fn default_uniswap_lp_bps() -> i32 { 1500 }
fn default_cash_bps() -> i32 { 500 }
fn default_target_weights() -> TargetWeightsBps {
    // v2.0.0: the Aerodrome tranche was liquidated (2026-10-23); the LP target is now
    // the aggregate across all tracked Uniswap positions. Legacy configs carrying
    // `aerodrome_lp` / `uniswap_v4_lp` keys still parse (unknown fields are ignored).
    TargetWeightsBps {
        treasury: 8000,
        uniswap_lp: 1500,
        cash: 500,
    }
}

impl Default for OperatorProfile {
    fn default() -> Self {
        Self {
            treasury_floor_nock: default_treasury_floor(),
            tax_reserve_ratio: default_tax_reserve_ratio(),
            target_weights_bps: default_target_weights(),
        }
    }
}

/// One operator-tracked LP position (v2.0.0). Identifiers come from the Phase-1
/// RPC discovery (see ONCHAIN_INTEGRATION.md §5) and are NEVER guessed at runtime.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PositionConfig {
    pub id: String,
    /// "uniswap-v4" | "uniswap-v3"
    pub protocol: String,
    #[serde(default = "default_network")]
    pub network: String,
    pub nft_id: u64,
    /// v4: bytes32 poolId. Unused for v3 (pool comes from the NPM decode).
    #[serde(default)]
    pub pool_id: Option<String>,
    /// v3 only: the canonical NonfungiblePositionManager for the network.
    #[serde(default)]
    pub npm_address: Option<String>,
    /// v3 only: pool contract address (pre-resolved via factory getPool).
    #[serde(default)]
    pub pool_address: Option<String>,
    /// v4 only: currency0/currency1 per the pool's canonical ordering (validated
    /// at runtime against POSM poolKeys).
    #[serde(default)]
    pub currency0: Option<String>,
    #[serde(default)]
    pub currency1: Option<String>,
    /// v4 only: immutable tick bounds from the mint (re-run discovery to update).
    #[serde(default)]
    pub tick_lower: Option<i32>,
    #[serde(default)]
    pub tick_upper: Option<i32>,
    /// Designates the position whose pool supplies the NOCK/USD spot.
    #[serde(default)]
    pub is_price_source: bool,
}

fn default_network() -> String {
    "base".to_string()
}

fn default_eth_price_pool() -> Option<String> {
    Some(DEFAULT_ETH_PRICE_POOL.to_string())
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct AppConfig {
    #[serde(default = "default_db_url")]
    database_url: String,
    #[serde(default)]
    alchemy_api_keys: Vec<String>,
    #[serde(default = "default_tracked_wallets")]
    tracked_wallets: Vec<String>,
    #[serde(default)]
    tracked_pools: Vec<String>,
    #[serde(default)]
    token_addresses: Vec<String>,
    #[serde(default)]
    tracked_ranges: Vec<String>,
    #[serde(default)]
    fee_contracts: Vec<String>,
    #[serde(default)]
    lp_tokens: Vec<String>,
    #[serde(default = "default_sync_window")]
    sync_window_seconds: u64,
    #[serde(default = "default_port")]
    port: u16,
    /// Tracked LP positions (v2.0.0) — replaces the single hardcoded v4_position_id.
    #[serde(default)]
    positions: Vec<PositionConfig>,
    /// Verified default (canonical v3 WETH/USDC 0.05% pool on Base).
    #[serde(default = "default_eth_price_pool")]
    eth_price_pool: Option<String>,
    #[serde(default)]
    mcp_bearer_token: Option<String>,
    #[serde(default)]
    operator_profile: OperatorProfile,
    #[serde(default)]
    notifications: NotificationConfig,
    #[serde(default)]
    market_data: MarketDataConfig,
}

impl AppConfig {
    pub fn profile(&self) -> OperatorProfile {
        self.operator_profile.clone()
    }
}

fn default_db_url() -> String {
    "postgresql://postgres:postgres@localhost:5432/nocknock".to_string()
}
fn default_tracked_wallets() -> Vec<String> {
    vec!["0xeE114cC97eC5adeB34bA5645e315B497D8487a45".to_string()]
}
fn default_sync_window() -> u64 {
    300
}
fn default_port() -> u16 {
    8080
}

/// Market-data backfill configuration (v0.3.0). The whole section is optional;
/// every field defaults so existing config.yaml files keep working unchanged.
#[derive(Deserialize, Debug, Clone)]
struct MarketDataConfig {
    /// Master switch for the historical price backfill engine
    #[serde(default = "default_backfill_enabled")]
    backfill_enabled: bool,
    #[serde(default = "default_geckoterminal_base_url")]
    geckoterminal_base_url: String,
    #[serde(default = "default_dexscreener_base_url")]
    dexscreener_base_url: String,
    /// Tracked Aerodrome pool used for third-party OHLCV lookups
    #[serde(default = "default_tracked_pool")]
    pool_address: String,
    /// Newest stored price point older than this triggers an hourly backfill
    #[serde(default = "default_gap_threshold_minutes")]
    gap_threshold_minutes: i64,
    /// Stored history must span at least this many days for the daily RSI
    #[serde(default = "default_min_history_days")]
    min_history_days: i64,
    #[serde(default = "default_hourly_lookback")]
    hourly_lookback: u32,
    #[serde(default = "default_daily_lookback")]
    daily_lookback: u32,
}

impl Default for MarketDataConfig {
    fn default() -> Self {
        Self {
            backfill_enabled: default_backfill_enabled(),
            geckoterminal_base_url: default_geckoterminal_base_url(),
            dexscreener_base_url: default_dexscreener_base_url(),
            pool_address: default_tracked_pool(),
            gap_threshold_minutes: default_gap_threshold_minutes(),
            min_history_days: default_min_history_days(),
            hourly_lookback: default_hourly_lookback(),
            daily_lookback: default_daily_lookback(),
        }
    }
}

fn default_backfill_enabled() -> bool {
    true
}
fn default_geckoterminal_base_url() -> String {
    "https://api.geckoterminal.com/api/v2".to_string()
}
fn default_dexscreener_base_url() -> String {
    "https://api.dexscreener.com/latest".to_string()
}
fn default_tracked_pool() -> String {
    // v2.0.0: tracked Uniswap v4 NOCK/USDC pool (0.75% fee) on Base — the operator's
    // active venue and the NOCK/USD price source. Third-party OHLCV coverage is
    // re-pointed here; providers without coverage degrade fail-open (warnings only).
    "0x91a9ef0a8dd231b99fa53a2a145a9e8bca7897e0d85852107abb997916f9f99c".to_string()
}
fn default_gap_threshold_minutes() -> i64 {
    15
}
fn default_min_history_days() -> i64 {
    14
}
fn default_hourly_lookback() -> u32 {
    168
}
fn default_daily_lookback() -> u32 {
    30
}

// ============================================================
// Data Models (matching spec)
// ============================================================

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SnapshotEnvelope {
    meta: SnapshotMeta,
    data: SnapshotData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SnapshotMeta {
    generated_at: DateTime<Utc>,
    state_hash: String,
    version: String,
    snapshot_id: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SnapshotData {
    wallet_balances: Vec<WalletBalance>,
    positions: Vec<Position>,
    pool_mechanics: Vec<PoolMechanic>,
    concentrated_liquidity: Vec<ConcentratedLiquidity>,
    contract_fees: Vec<ContractFee>,
    derived_metrics: DerivedMetrics,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct RawOnChainData {
    wallet_balances: Vec<WalletBalance>,
    pool_mechanics: Vec<PoolMechanic>,
    concentrated_liquidity: Vec<ConcentratedLiquidity>,
    contract_fees: Vec<ContractFee>,
    /// Per-position on-chain reads (v2.0.0) — replaces live_tranche_1_*/2_* and
    /// the Aerodrome LP-share/claimFees inputs.
    #[serde(default)]
    positions: Vec<RawPosition>,
    /// Live NOCK/USD spot from the designated price-source position pool.
    #[serde(default)]
    spot_price_nock_usd: f64,
    #[serde(default)]
    live_gas_cost_usd: f64,
    #[serde(default = "default_eth_price")]
    live_eth_price_usd: f64,
}

/// Token descriptor for position legs (symbol/decimals come from the repo's verified
/// token registry — NOCK 16, USDC 6, WETH/native-ETH 18).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct PositionTokenInfo {
    address: String,
    symbol: String,
    decimals: i32,
}

/// A fully-priced tracked position as emitted in snapshots (v2.0.0).
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Position {
    position_id: String,
    network: String,
    wallet: String,
    protocol: String,
    nft_id: u64,
    pool_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    npm_address: Option<String>,
    fee_tier_bps: u32,
    token0: PositionTokenInfo,
    token1: PositionTokenInfo,
    amount0: String,
    amount1: String,
    amount_usd: String,
    tokens_owed_0: String,
    tokens_owed_1: String,
    fees_usd: String,
    lower_tick: i32,
    upper_tick: i32,
    lower_price: String,
    upper_price: String,
    liquidity_amount: String,
    in_range: bool,
    is_price_source: bool,
}

/// Per-position NOCK series entry for the asset aggregation chart (v2.0.0;
/// replaces the retired tranche_1/2/3_nock series).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct PositionNockSeries {
    position_id: String,
    series: Vec<String>,
}

/// Raw per-position on-chain state (human-unit amounts; USD pricing happens later).
#[derive(Serialize, Deserialize, Debug, Clone)]
struct RawPosition {
    position_id: String,
    network: String,
    protocol: String,
    wallet: String,
    nft_id: u64,
    pool_id: String,
    npm_address: Option<String>,
    fee_tier_bps: u32,
    token0: PositionTokenInfo,
    token1: PositionTokenInfo,
    liquidity: u128,
    sqrt_price_x96: U256,
    tick: i32,
    tick_lower: i32,
    tick_upper: i32,
    /// Current position amounts in HUMAN token units
    amount0: f64,
    amount1: f64,
    /// Unclaimed fees in HUMAN token units
    fee0: f64,
    fee1: f64,
    in_range: bool,
    is_price_source: bool,
}

fn default_eth_price() -> f64 {
    2500.0
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct WalletBalance {
    wallet_address: String,
    token_symbol: String,
    token_address: String,
    amount: String,
    amount_usd: String,
    updated_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PoolMechanic {
    pool_id: String,
    platform: String,
    pool_type: String,
    volatile_assets: Vec<PoolAsset>,
    liquidity_usd: String,
    fee_accrued_usd: String,
    fee_tier_bps: u32,
    tick_current: Option<i32>,
    sqrt_price_x96: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PoolAsset {
    token_address: String,
    token_symbol: String,
    reserve_amount: String,
    reserve_usd: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ConcentratedLiquidity {
    range_id: String,
    pool_id: String,
    lower_tick: i32,
    upper_tick: i32,
    lower_price: String,
    upper_price: String,
    liquidity_amount: String,
    tokens_owed_0: String,
    tokens_owed_1: String,
    in_range: bool,
    fee_growth_inside_0_x128: String,
    fee_growth_inside_1_x128: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ContractFee {
    contract_address: String,
    fee_token_address: String,
    fee_token_symbol: String,
    amount: String,
    amount_usd: String,
    claimable: bool,
    gas_break_even_usd: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct DerivedMetrics {
    global_status: GlobalStatus,
    asset_aggregation_series: AssetAggregationSeries,
    total_value_vs_baseline: TotalValueVsBaseline,
    tick_bracket_hypervisor: TickBracketHypervisor,
    portfolio_deviation_matrix: PortfolioDeviationMatrix,
    structural_channels: StructuralChannels,
    strategy_counterfactual: StrategyCounterfactual,
    velocity_gas_sandbox: VelocityGasSandbox,
    #[serde(default)]
    sovereign_ledger: SovereignLedger,
}

/// Virtual 60/40 sub-ledger over receipt-verified harvests (no on-chain address change).
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct SovereignLedger {
    cumulative_harvested_usd: String,
    tax_reserve_usd: String,
    growth_dry_powder_usd: String,
    harvest_count: i64,
    last_harvest_at: Option<String>,
}

#[derive(Debug, Default, Clone, Copy)]
struct LedgerAggregate {
    total_usd: f64,
    harvest_count: i64,
    last_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct GlobalStatus {
    total_portfolio_usd: String,
    total_native_usd: String,
    total_stable_usd: String,
    total_fees_accrued_usd: String,
    total_fees_claimable_usd: String,
    gas_to_fee_delta_usd: String,
    high_water_mark_usd: String,
    high_water_mark_date: String,
    #[serde(default)]
    total_nock_amount: String,
    #[serde(default)]
    nock_floor_delta: String,
    #[serde(default = "default_floor_target_string")]
    nock_floor_target: String,
    #[serde(default)]
    total_usdc_amount: String,
    #[serde(default)]
    shot_advice_action: String,
    #[serde(default)]
    shot_advice_details: String,
    #[serde(default)]
    regime: String,
    /// Wilder RSI(14) over the 5-minute snapshot series (short-term momentum)
    #[serde(default)]
    rsi_14: Option<String>,
    /// Wilder RSI(14) over daily closes — the industry-standard "daily RSI";
    /// drives `regime` and the Overextension Cooldown
    #[serde(default)]
    rsi_14_1d: Option<String>,
}

fn default_floor_target_string() -> String {
    "100000".to_string()
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct AssetAggregationSeries {
    time_points: Vec<String>,
    safe_haven_series: Vec<String>,
    active_protocol_series: Vec<String>,
    #[serde(default)]
    positions_nock: Vec<PositionNockSeries>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct TotalValueVsBaseline {
    time_points: Vec<String>,
    portfolio_value_series: Vec<String>,
    token_price_series: Vec<String>,
    high_water_mark_series: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct TickBracketHypervisor {
    current_price: String,
    brackets: Vec<TickBracket>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct TickBracket {
    lower_price: String,
    upper_price: String,
    liquidity_usd: String,
    in_range: bool,
    /// Signed bps from spot to the nearest edge — only when the bracket is denominated
    /// like the spot (USD/NOCK). Cross-denomination rows (ETH-pools) emit null rather
    /// than a meaningless cross-unit number (zero-mock).
    distance_bps: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct PortfolioDeviationMatrix {
    allocations: Vec<AllocationDeviation>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AllocationDeviation {
    bucket_name: String,
    target_weight_bps: i32,
    actual_weight_bps: i32,
    deviation_bps: i32,
    status: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct StructuralChannels {
    channels: Vec<RegressionChannel>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RegressionChannel {
    start_time: String,
    end_time: Option<String>,
    center_line: Vec<LinePoint>,
    upper_1sd: Vec<LinePoint>,
    lower_1sd: Vec<LinePoint>,
    upper_2sd: Vec<LinePoint>,
    lower_2sd: Vec<LinePoint>,
    is_active: bool,
    break_point: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LinePoint {
    time: String,
    value: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct StrategyCounterfactual {
    time_points: Vec<String>,
    active_portfolio_series: Vec<String>,
    passive_baseline_series: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct VelocityGasSandbox {
    contracts: Vec<FeeCollectionIndex>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct FeeCollectionIndex {
    contract_address: String,
    contract_name: String,
    claimable_usd: String,
    gas_break_even_usd: String,
    efficiency_ratio: f64,
    recommended_action: String,
}

// ============================================================
// App State
// ============================================================

struct AppState {
    db: PgPool,
    http_client: Client,
    config: Arc<AppConfig>,
    latest_snapshot: Mutex<Option<SnapshotEnvelope>>,
    last_sync_duration_ms: std::sync::atomic::AtomicU64,
}

// ============================================================
// Main
// ============================================================

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Load config (falls back gracefully to config.yaml.example or env vars)
    let config = Config::builder()
        .add_source(config::File::with_name("config.yaml.example").required(false))
        .add_source(config::File::with_name("config").required(false))
        .add_source(config::Environment::with_prefix("APP").separator("__"))
        .add_source(config::Environment::default())
        .build()?
        .try_deserialize::<AppConfig>()?;

    // Initialize database
    let db = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect to PostgreSQL");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("Failed to run migrations");

    // Initialize HTTP client
    let http_client = Client::builder()
        .user_agent("nocknock-ingestion-worker/0.1.0")
        .build()?;

    // Load existing latest snapshot from database on boot
    use sqlx::Row;
    let initial_latest = sqlx::query(
        "SELECT snapshot_id, generated_at, state_hash, version, data FROM snapshots ORDER BY generated_at DESC LIMIT 1"
    )
    .fetch_optional(&db)
    .await
    .ok()
    .flatten()
    .and_then(|r| {
        let snapshot_id: uuid::Uuid = r.try_get("snapshot_id").ok()?;
        let generated_at: DateTime<Utc> = r.try_get("generated_at").ok()?;
        let state_hash: String = r.try_get("state_hash").ok()?;
        let version: String = r.try_get("version").ok()?;
        let data_val: serde_json::Value = r.try_get("data").ok()?;
        let data: SnapshotData = serde_json::from_value(data_val).ok()?;
        Some(SnapshotEnvelope {
            meta: SnapshotMeta {
                generated_at,
                state_hash,
                version,
                snapshot_id,
            },
            data,
        })
    });

    let state = Arc::new(AppState {
        db: db.clone(),
        http_client,
        config: Arc::new(config),
        latest_snapshot: Mutex::new(initial_latest),
        last_sync_duration_ms: std::sync::atomic::AtomicU64::new(0),
    });

    // Historical price backfill (v0.3.0): heal multi-hour/multi-day gaps left by
    // laptop sleep or downtime before the first cycle computes the daily RSI.
    // Fail-open by design; never blocks startup.
    ensure_price_history(&state).await;

    // Start ingestion scheduler
    let state_clone = state.clone();
    tokio::spawn(async move {
        // Run immediately on boot to guarantee freshness
        if let Err(e) = run_ingestion_cycle(&state_clone).await {
            eprintln!("Initial ingestion error: {e}");
        }
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_secs(
                state_clone.config.sync_window_seconds as u64,
            ),
        );
        interval.tick().await; // consume first tick since we just ran
        loop {
            interval.tick().await;
            if let Err(e) = run_ingestion_cycle(&state_clone).await {
                eprintln!("Ingestion error: {e}");
            }
        }
    });

    // Start API server
    let app = axum::Router::new()
        .route("/health", axum::routing::get(health_handler))
        .route("/v1/health", axum::routing::get(health_handler))
        .route(
            "/v1/snapshot/latest",
            axum::routing::get(get_latest_snapshot_handler),
        )
        .route(
            "/v1/snapshot",
            axum::routing::get(get_latest_snapshot_handler),
        )
        .route(
            "/snapshot",
            axum::routing::get(get_latest_snapshot_handler),
        )
        .route(
            "/v1/snapshot/historical",
            axum::routing::get(get_historical_snapshots_handler),
        )
        .route(
            "/v1/snapshot/metadata",
            axum::routing::get(get_snapshot_metadata_handler),
        )
        .route(
            "/v1/snapshot/:id",
            axum::routing::get(get_snapshot_by_id_handler),
        )
        .route(
            "/v1/ingest",
            axum::routing::post(trigger_ingest_handler),
        )
        .route(
            "/v1/mcp",
            axum::routing::post(mcp_http_handler),
        )
        .route(
            "/mcp",
            axum::routing::post(mcp_http_handler),
        )
        .route(
            "/v1/execute/claim-fees/payload",
            axum::routing::get(get_claim_fees_payload_handler),
        )
        .route(
            "/v1/execute/record-harvest",
            axum::routing::post(record_harvest_handler),
        )
        .route(
            "/v1/tax/export",
            axum::routing::get(tax_export_handler),
        )
        .layer(Extension(state.clone()));

    let port = state.config.port;
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await?;
    println!(
        "🚀 NockNock ingestion worker listening on http://{}",
        listener.local_addr()?
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    println!("👋 Ingestion worker shut down cleanly.");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => println!("Received Ctrl+C, shutting down gracefully..."),
        _ = terminate => println!("Received SIGTERM, shutting down gracefully..."),
    }
}

// ============================================================
// Handlers
// ============================================================

async fn health_handler(Extension(state): Extension<Arc<AppState>>) -> impl IntoResponse {
    let latest = state.latest_snapshot.lock().await;
    let latest = &*latest;
    let status: &str = if latest.is_some() { "ok" } else { "starting" };
    let last_sync: Option<DateTime<Utc>> = latest.as_ref().map(|s| s.meta.generated_at);
    let ingestion_lag_seconds: i64 = last_sync
        .map(|ts| {
            Utc::now().signed_duration_since(ts).num_seconds().max(0)
        })
        .unwrap_or(0);

    let sync_duration_ms = state.last_sync_duration_ms.load(std::sync::atomic::Ordering::Relaxed);

    Json(json!({
        "status": status,
        "last_sync": last_sync.map(|dt| dt.to_rfc3339()),
        "ingestion_lag_seconds": ingestion_lag_seconds,
        "sync_duration_ms": sync_duration_ms,
        "sync_window_seconds": state.config.sync_window_seconds,
    }))
}

async fn get_latest_snapshot_handler(
    Extension(state): Extension<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let mut latest = state.latest_snapshot.lock().await;

    if latest.is_none() {
        use sqlx::Row;
        if let Ok(Some(r)) = sqlx::query(
            "SELECT snapshot_id, generated_at, state_hash, version, data FROM snapshots ORDER BY generated_at DESC LIMIT 1"
        ).fetch_optional(&state.db).await {
            if let (Ok(sid), Ok(gen), Ok(sh), Ok(ver), Ok(dv)) = (
                r.try_get::<uuid::Uuid, _>("snapshot_id"),
                r.try_get::<DateTime<Utc>, _>("generated_at"),
                r.try_get::<String, _>("state_hash"),
                r.try_get::<String, _>("version"),
                r.try_get::<serde_json::Value, _>("data"),
            ) {
                if let Ok(data) = serde_json::from_value::<SnapshotData>(dv) {
                    *latest = Some(SnapshotEnvelope {
                        meta: SnapshotMeta {
                            generated_at: gen,
                            state_hash: sh,
                            version: ver,
                            snapshot_id: sid,
                        },
                        data,
                    });
                }
            }
        }
    }

    if let Some(envelope) = &*latest {
        // Check If-None-Match for 304
        if let Some(if_none_match) = headers.get("if-none-match") {
            if if_none_match == envelope.meta.state_hash.as_str() {
                return (StatusCode::NOT_MODIFIED, HeaderMap::new(), Json(serde_json::Value::Null));
            }
        }

        let mut headers = HeaderMap::new();
        if let Ok(value) = envelope.meta.state_hash.parse() {
            headers.insert("etag", value);
        }
        if let Ok(value) = envelope.meta.generated_at.to_rfc3339().parse() {
            headers.insert("last-modified", value);
        }
        let cache_control = "no-cache, must-revalidate";
        if let Ok(value) = cache_control.parse() {
            headers.insert("cache-control", value);
        }

        let mut body = serde_json::to_value(envelope).unwrap_or_else(|_| json!({}));
        body["data_provenance"] = data_provenance(&state.db).await;
        (StatusCode::OK, headers, Json(body))
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            HeaderMap::new(),
            Json(json!({
                "error": "NO_SNAPSHOT",
                "message": "No snapshot available yet"
            })),
        )
    }
}

async fn trigger_ingest_handler(
    Extension(state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    match run_ingestion_cycle(&state).await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "status": "triggered",
                "message": "Ingestion cycle completed successfully"
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "status": "error",
                "message": format!("Ingestion failed: {}", e)
            })),
        ),
    }
}

async fn get_historical_snapshots_handler(
    Extension(state): Extension<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Response {
    // `interval` was declared in api.yaml but never implemented (now removed from the
    // spec); reject it explicitly instead of silently ignoring declared parameters.
    if params.contains_key("interval") {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "INVALID_PARAMETER",
                "message": "interval is not supported; use before/after RFC3339 cursors with limit"
            })),
        )
            .into_response();
    }

    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(100)
        .clamp(1, 1000);

    let parse_cursor = |key: &str| -> Option<DateTime<Utc>> {
        params
            .get(key)
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&Utc))
    };
    let before = parse_cursor("before");
    let after = parse_cursor("after");

    let base =
        "SELECT snapshot_id, generated_at, state_hash, version, data FROM snapshots".to_string();
    let rows = if let Some(before) = before {
        sqlx::query(&format!(
            "{base} WHERE generated_at < $1 ORDER BY generated_at DESC LIMIT $2"
        ))
        .bind(before)
        .bind(limit)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default()
    } else if let Some(after) = after {
        sqlx::query(&format!(
            "{base} WHERE generated_at > $1 ORDER BY generated_at DESC LIMIT $2"
        ))
        .bind(after)
        .bind(limit)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default()
    } else {
        sqlx::query(&format!("{base} ORDER BY generated_at DESC LIMIT $1"))
            .bind(limit)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default()
    };

    let snapshots: Vec<SnapshotEnvelope> = rows
        .into_iter()
        .filter_map(|r| {
            let data: SnapshotData = serde_json::from_value(r.try_get("data").ok()?).ok()?;
            let envelope = SnapshotEnvelope {
                meta: SnapshotMeta {
                    generated_at: r.try_get("generated_at").ok()?,
                    state_hash: r.try_get("state_hash").ok()?,
                    version: r.try_get("version").ok()?,
                    snapshot_id: r.try_get("snapshot_id").ok()?,
                },
                data,
            };
            Some(envelope)
        })
        .map(|mut envelope| {
            slim_historical_payload(&mut envelope.data);
            envelope
        })
        .collect();

    let returned = snapshots.len();
    Json(json!({
        "snapshots": snapshots,
        "pagination": {
            "limit": limit,
            "returned": returned,
            "has_more": returned == limit as usize,
        }
    }))
    .into_response()
}

/// Bound the historical response payload: charts only read each stored snapshot's
/// scalar / current-point values (the `[0]` element of embedded series), never its
/// multi-point series. Truncating the embedded counterfactual to its current point and
/// dropping the Bollinger channel points keeps a 288-row response ~1 MB instead of
/// ~5 MB. `/snapshot/latest` still serves the full embedded series (MCP consumers and
/// the cold-start chart fallback rely on it; snapshot.yaml declares these fields).
fn slim_historical_payload(data: &mut SnapshotData) {
    let dm = &mut data.derived_metrics;
    dm.strategy_counterfactual.time_points.truncate(1);
    dm.strategy_counterfactual.active_portfolio_series.truncate(1);
    dm.strategy_counterfactual.passive_baseline_series.truncate(1);
    // Channels are recomputed by the worker each cycle from history; historical rows
    // never need to carry them
    dm.structural_channels.channels.clear();
}

async fn get_snapshot_metadata_handler(
    Extension(state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    let rows = sqlx::query!(
        r#"
        SELECT snapshot_id, generated_at, state_hash
        FROM snapshots
        ORDER BY generated_at DESC
        LIMIT 20
        "#
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let snapshots: Vec<_> = rows
        .into_iter()
        .map(|r| {
            let generated_at = r.generated_at;
            let age_seconds = Utc::now().signed_duration_since(generated_at).num_seconds().max(0)
                as u64;

            json!({
                "snapshot_id": r.snapshot_id,
                "generated_at": r.generated_at.to_rfc3339(),
                "state_hash": r.state_hash,
                "age_seconds": age_seconds
            })
        })
        .collect();

    Json(json!({ "snapshots": snapshots }))
}

async fn get_snapshot_by_id_handler(
    Extension(state): Extension<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let snapshot_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, HeaderMap::new(), Json(json!({"error": "INVALID_UUID"}))),
    };
    let row = sqlx::query!(
        r#"
        SELECT snapshot_id, generated_at, state_hash, version, data
        FROM snapshots
        WHERE snapshot_id = $1
        "#,
        snapshot_id
    )
    .fetch_optional(&state.db)
    .await
    .unwrap_or_else(|_| None);

    if let Some(row) = row {
        if let Ok(envelope) = serde_json::from_value::<SnapshotEnvelope>(row.data) {
            (
                StatusCode::OK,
                HeaderMap::new(),
                Json(json!(envelope))
            )
        } else {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                HeaderMap::new(),
                Json(json!({
                    "error": "CORRUPTED_DATA",
                    "message": "Failed to parse snapshot data"
                })),
            )
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            HeaderMap::new(),
            Json(json!({
                "error": "NOT_FOUND",
                "message": "Snapshot not found"
            })),
        )
    }
}

pub fn format_portfolio_state_text(data: &serde_json::Value) -> String {
    let global = data.pointer("/derived_metrics/global_status");
    let total_usd = global.and_then(|g| g.get("total_portfolio_usd")).and_then(|v| v.as_str()).unwrap_or("0.00");
    let total_nock = global.and_then(|g| g.get("total_nock_amount")).and_then(|v| v.as_str()).unwrap_or("0");
    let nock_delta = global.and_then(|g| g.get("nock_floor_delta")).and_then(|v| v.as_str()).unwrap_or("0");
    let total_usdc = global.and_then(|g| g.get("total_usdc_amount")).and_then(|v| v.as_str()).unwrap_or("0.00");
    let hwm = global.and_then(|g| g.get("high_water_mark_usd")).and_then(|v| v.as_str()).unwrap_or("0.00");
    let advice = global.and_then(|g| g.get("shot_advice_action")).and_then(|v| v.as_str()).unwrap_or("HOLD");

    let delta_prefix = if nock_delta.starts_with('-') { "" } else { "+" };

    format!(
        "📈 NockNock Sovereign Portfolio State (Live API)\n\
         =========================================\n\
         • Total Value: ${total_usd} USD\n\
         • Sovereign NOCK Stack: {total_nock} NOCK (Floor buffer delta: {delta_prefix}{nock_delta} above 100k)\n\
         • Stable Stack: {total_usdc} USDC\n\
         • High-Water Mark: ${hwm} USD\n\
         • Tactical Directive: {advice}\n\
         • Mode: Tier 1 Connected (Verified Persistent Snapshot)"
    )
}

pub fn format_shot_advice_text(data: &serde_json::Value) -> String {
    let global = data.pointer("/derived_metrics/global_status");
    let action = global.and_then(|g| g.get("shot_advice_action")).and_then(|v| v.as_str()).unwrap_or("HOLD");
    let details = global.and_then(|g| g.get("shot_advice_details")).and_then(|v| v.as_str()).unwrap_or("Accreting");
    let claimable_usd = global.and_then(|g| g.get("total_fees_claimable_usd")).and_then(|v| v.as_str()).unwrap_or("0.00");
    let gas_delta = global.and_then(|g| g.get("gas_to_fee_delta_usd")).and_then(|v| v.as_str()).unwrap_or("0.00");

    format!(
        "🎯 Tactical Shot Advice Engine\n\
         ===============================\n\
         • Recommended Action: {action}\n\
         • Decision Matrix Details: {details}\n\
         • Uncollected Claimable Fees: ${claimable_usd} USD\n\
         • Net Gas-to-Fee Profitability: +${gas_delta} USD\n\
         • Execution rule: claim advised when claimable yield ≥ 2× live gas break-even."
    )
}

pub fn format_claim_simulation_text(data: &serde_json::Value) -> String {
    let fees = data.get("contract_fees").and_then(|f| f.as_array());
    let mut details = String::new();
    let mut total_claimable_usd = 0.0f64;
    let mut gas_usd: Option<f64> = None;

    if let Some(list) = fees {
        for fee in list {
            let symbol = fee.get("fee_token_symbol").and_then(|s| s.as_str()).unwrap_or("Pool");
            let amount = fee.get("amount").and_then(|a| a.as_str()).unwrap_or("0");
            let amount_usd = fee.get("amount_usd").and_then(|a| a.as_str()).unwrap_or("0.00");
            let gas_str = fee.get("gas_break_even_usd").and_then(|g| g.as_str()).unwrap_or_default();
            match gas_str.parse::<f64>() {
                Ok(g) => {
                    // Per-contract break-evens come from the live gas query; take the max
                    // as the representative claim cost. Never fabricate a constant.
                    gas_usd = Some(match gas_usd {
                        Some(acc) => acc.max(g),
                        None => g,
                    });
                    details.push_str(&format!("  - {symbol}: {amount} (~${amount_usd} USD | Gas Est: ${:.4})\n", g));
                }
                Err(_) => {
                    details.push_str(&format!("  - {symbol}: {amount} (~${amount_usd} USD | Gas Est: unavailable)\n"));
                }
            }
            if let Ok(usd_val) = amount_usd.parse::<f64>() {
                total_claimable_usd += usd_val;
            }
        }
    }

    let (gas_line, net_line) = match gas_usd {
        Some(g) => (
            format!("~${:.4} USD", g),
            format!("${:.4} USD", (total_claimable_usd - g).max(0.0)),
        ),
        None => (
            "unavailable".to_string(),
            "unknown (live gas estimate unavailable)".to_string(),
        ),
    };
    format!(
        "⚡ Fee Claim Simulation\n\
         =======================\n\
         • Total Gross Claimable: ${:.4} USD\n\
         • Estimated Gas: {}\n\
         • Net Accretion if Harvested: {}\n\
         Pool Breakdown:\n{}",
        total_claimable_usd, gas_line, net_line, details
    )
}

async fn mcp_http_handler(
    Extension(state): Extension<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // Validate Bearer token if configured
    if let Some(ref required_token) = state.config.mcp_bearer_token {
        let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
        let provided_token = auth_header.and_then(|h| {
            h.strip_prefix("Bearer ")
                .or_else(|| h.strip_prefix("bearer "))
        });
        if provided_token != Some(required_token.as_str()) {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "jsonrpc": "2.0",
                    "id": payload.get("id"),
                    "error": {
                        "code": -32001,
                        "message": "Unauthorized: Invalid or missing Bearer token"
                    }
                })),
            );
        }
    }

    let method = payload.get("method").and_then(|m| m.as_str()).unwrap_or_default();
    let id = payload.get("id").cloned();

    let response = match method {
        "initialize" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "nocknock-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {
                    "tools": {}
                }
            }
        }),
        "ping" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {}
        }),
        "tools/list" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": [
                    {
                        "name": "get_portfolio_state",
                        "description": "Returns current sovereign portfolio balances, NOCK floor buffer delta, native token balance, and active tranche allocations.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {},
                            "required": []
                        }
                    },
                    {
                        "name": "get_shot_advice",
                        "description": "Returns the active tactical directive (e.g. HOLD & ACCRETE, BATCH HARVEST SHOT, DEFENSIVE LOCK) and break-even gas economics.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {},
                            "required": []
                        }
                    },
                    {
                        "name": "simulate_claim_fees",
                        "description": "Simulates gas expenditure vs uncollected yield across Aerodrome and Uniswap V4 positions.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {},
                            "required": []
                        }
                    }
                ]
            }
        }),
        "tools/call" => {
            let tool_name = payload
                .get("params")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or_default();

            let latest = state.latest_snapshot.lock().await;
            let result_text = if let Some(ref env) = *latest {
                let data_val = serde_json::to_value(&env.data).unwrap_or_default();
                match tool_name {
                    "get_portfolio_state" => format_portfolio_state_text(&data_val),
                    "get_shot_advice" => format_shot_advice_text(&data_val),
                    "simulate_claim_fees" => format_claim_simulation_text(&data_val),
                    _ => format!("Unknown tool: {tool_name}"),
                }
            } else {
                "No snapshot available yet".to_string()
            };

            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [
                        {
                            "type": "text",
                            "text": result_text
                        }
                    ]
                }
            })
        }
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32601,
                "message": format!("Method not found: {method}")
            }
        }),
    };

    (StatusCode::OK, Json(response))
}

// ============================================================
// Ingestion Logic
// ============================================================

async fn run_ingestion_cycle(state: &Arc<AppState>) -> Result<()> {
    let start_time = std::time::Instant::now();
    let now = Utc::now().to_rfc3339();
    tracing::info!("Starting ingestion cycle at {now}");

    // Heal price-history gaps first (fail-open) so the daily RSI below sees
    // backfilled candles for any period the worker was offline.
    ensure_price_history(state).await;

    // Fetch raw data from Alchemy (parallelized on-chain calls)
    let raw_data = fetch_raw_data(state).await?;

    // Query persistent high-water mark from snapshots table
    let prev_hwm: f64 = sqlx::query_scalar::<_, f64>(
        "SELECT COALESCE(MAX((data->'derived_metrics'->'global_status'->>'total_portfolio_usd')::float8), 0.0) FROM snapshots"
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0.0);

    // Compute structural channels from historical data — most recent 50 snapshots,
    // oldest first, with their real snapshot timestamps
    let historical_rows = sqlx::query(
        r#"SELECT generated_at, (data->'derived_metrics'->'global_status'->>'total_portfolio_usd')::float8 as portfolio_usd FROM snapshots ORDER BY generated_at DESC LIMIT 50"#
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let historical_prices: Vec<(DateTime<Utc>, f64)> = historical_rows
        .iter()
        .rev()
        .filter_map(|r| {
            let ts: DateTime<Utc> = r.try_get("generated_at").ok()?;
            let v: f64 = r.try_get("portfolio_usd").ok()?;
            Some((ts, v))
        })
        .collect();

    let structural_channels = compute_structural_channels_sync(&historical_prices);

    // Sovereign ledger aggregate: receipt-verified harvests feed the virtual 60/40 sub-ledger
    let ledger_agg = match sqlx::query(
        "SELECT COALESCE(SUM(usd_value::numeric)::float8, 0) AS total_usd, \
                COUNT(DISTINCT tx_hash) AS harvests, \
                MAX(recorded_at) AS last_at \
         FROM harvest_events",
    )
    .fetch_one(&state.db)
    .await
    {
        Ok(r) => LedgerAggregate {
            total_usd: r.try_get::<f64, _>("total_usd").unwrap_or(0.0),
            harvest_count: r.try_get::<i64, _>("harvests").unwrap_or(0),
            last_at: r.try_get::<Option<DateTime<Utc>>, _>("last_at").ok().flatten(),
        },
        Err(e) => {
            tracing::warn!("Harvest ledger aggregate query failed: {e}");
            LedgerAggregate::default()
        }
    };

    // Fetch counterfactual historical data: the MOST RECENT 100 snapshots, collected as
    // paired (timestamp, portfolio, native) tuples so the two series stay index-aligned,
    // oldest first, carrying real snapshot timestamps (no fabricated spacing).
    let counterfactual_rows = sqlx::query(
        r#"
        SELECT generated_at,
               (data->'derived_metrics'->'global_status'->>'total_portfolio_usd')::float8 as portfolio_usd,
               (data->'derived_metrics'->'global_status'->>'total_native_usd')::float8 as native_usd
        FROM (
            SELECT generated_at, data FROM snapshots ORDER BY generated_at DESC LIMIT 100
        ) recent
        ORDER BY generated_at ASC
        "#
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let mut counterfactual_history: Vec<(DateTime<Utc>, f64, f64)> = Vec::new();
    for row in counterfactual_rows {
        if let (Ok(ts), Ok(Some(p)), Ok(Some(n))) = (
            row.try_get::<DateTime<Utc>, _>("generated_at"),
            row.try_get::<Option<f64>, _>("portfolio_usd"),
            row.try_get::<Option<f64>, _>("native_usd"),
        ) {
            counterfactual_history.push((ts, p, n));
        }
    }

    // Compute derived metrics
    let profile = state.config.profile();
    // Market regime — two RSI(14) series per industry convention (the period is
    // 14 bars of the underlying timeframe, not 14 days):
    //   • Micro: RSI(14) over the 5-minute snapshot series (short-term momentum pulse).
    //   • Macro: daily RSI(14) over one close per UTC day spanning the multi-day
    //     lookback, backfilled from third-party OHLCV where snapshots are missing.
    //     The Overextension Cooldown keys on the daily series so a 4-hour bounce
    //     cannot fake a multi-day top (issue #29).
    // Historical micro closes: the NOCK spot stored in each past snapshot
    // (total_value_vs_baseline.token_price_series[0] — every row since v0.1.0 carries
    // it, so the series stays continuous across the v2.0.0 pool-shape change).
    let regime_rows = sqlx::query(
        r#"SELECT (data->'derived_metrics'->'total_value_vs_baseline'->'token_price_series'->>0)::float8 AS spot
           FROM snapshots ORDER BY generated_at DESC LIMIT 50"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    let mut micro_closes: Vec<f64> = regime_rows
        .iter()
        .rev()
        .filter_map(|r| {
            let spot: f64 = r.try_get("spot").ok()?;
            if spot > 0.0 { Some(spot) } else { None }
        })
        .collect();
    // Live NOCK/USD spot: computed upstream from the price-source position pool.
    let live_spot = raw_data.spot_price_nock_usd;
    if live_spot > 0.0 {
        micro_closes.push(live_spot);
        // Contemporaneous price record: the RPC-observed spot is stored before any
        // snapshot dedup so the daily series always includes the freshest close.
        if let Err(e) =
            insert_price_point(&state.db, Utc::now(), live_spot, PRICE_SOURCE_RPC, false).await
        {
            tracing::warn!("Contemporaneous price insert failed (non-fatal): {e}");
        }
    }
    let rsi_5m = compute_rsi_series(&micro_closes, RSI_PERIOD)
        .iter()
        .rev()
        .find_map(|x| *x);

    // Macro (daily) series: one close per UTC day from market_price_history —
    // daily candle closes where backfilled, else the day's last spot.
    let macro_points: Vec<(DateTime<Utc>, Option<f64>, f64)> = sqlx::query(
        "SELECT timestamp, close_usd, price_usd FROM market_price_history \
         WHERE timestamp >= NOW() - make_interval(days => $1) ORDER BY timestamp ASC",
    )
    .bind(state.config.market_data.daily_lookback as i32)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
    .iter()
    .filter_map(|r| {
        let ts: DateTime<Utc> = r.try_get("timestamp").ok()?;
        let close_usd: Option<f64> = r.try_get("close_usd").ok()?;
        let price_usd: f64 = r.try_get("price_usd").ok()?;
        Some((ts, close_usd, price_usd))
    })
    .collect();
    let mut macro_closes = daily_closes_from_points(&macro_points);
    if live_spot > 0.0 {
        macro_closes.push(live_spot); // the live spot closes today's partial day
    }
    let rsi_1d = compute_rsi_series(&macro_closes, RSI_PERIOD)
        .iter()
        .rev()
        .find_map(|x| *x);
    let regime = MarketRegime {
        rsi_5m,
        rsi_1d,
        overextended: rsi_1d.map_or(false, |r| r >= RSI_OVERBOUGHT),
    };

    let derived_metrics = compute_derived_metrics(&raw_data, prev_hwm, &profile, structural_channels,
        &counterfactual_history, &ledger_agg, &regime);
    // Capture directive strings before the metrics move into the snapshot envelope
    let next_directive_action = derived_metrics.global_status.shot_advice_action.clone();
    let next_directive_details = derived_metrics.global_status.shot_advice_details.clone();

    // Build snapshot data
    let snapshot_data = SnapshotData {
        wallet_balances: raw_data.wallet_balances,
        positions: build_position_snapshots(&raw_data.positions, raw_data.spot_price_nock_usd, raw_data.live_eth_price_usd),
        pool_mechanics: raw_data.pool_mechanics,
        concentrated_liquidity: raw_data.concentrated_liquidity,
        contract_fees: raw_data.contract_fees,
        derived_metrics,
    };

    // Compute deterministic state hash
    let state_hash = compute_state_hash(&snapshot_data);

    // Build envelope
    let now = Utc::now();
    // Directive-transition notifications (issue #28): fire only when the tactical
    // directive changes between cycles. First boot stays silent; delivery failures
    // are logged, never fatal.
    let prev_directive = state
        .latest_snapshot
        .lock()
        .await
        .as_ref()
        .map(|s| s.data.derived_metrics.global_status.shot_advice_action.clone());
    if let Some(text) = directive_transition_notification(
        prev_directive.as_deref(),
        &next_directive_action,
        &next_directive_details,
        state.config.notifications.cockpit_url.as_deref(),
    ) {
        dispatch_notifications(state, &text).await;
    }

    let envelope = SnapshotEnvelope {
        meta: SnapshotMeta {
            generated_at: now,
            state_hash: state_hash.clone(),
            version: "2.0.0".to_string(),
            snapshot_id: Uuid::new_v4(),
        },
        data: snapshot_data,
    };

    // Extract values for logging before moving
    let snapshot_id = envelope.meta.snapshot_id;
    let state_hash = envelope.meta.state_hash.clone();

    // Store in database with deduplication
    store_snapshot(&state.db, &envelope).await?;

    // Update in-memory cache
    *state.latest_snapshot.lock().await = Some(envelope);

    let duration_ms = start_time.elapsed().as_millis() as u64;
    state.last_sync_duration_ms.store(duration_ms, std::sync::atomic::Ordering::Relaxed);

    tracing::info!(
        "Ingestion complete in {duration_ms}ms: snapshot_id={}, hash={}",
        snapshot_id,
        &state_hash[..16]
    );

    Ok(())
}

async fn post_rpc_with_retry(
    client: &Client,
    url: &str,
    payload: &serde_json::Value,
    max_retries: usize,
) -> Option<serde_json::Value> {
    for attempt in 0..=max_retries {
        match client.post(url).json(payload).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    if let Ok(json_val) = resp.json::<serde_json::Value>().await {
                        if let Some(err) = json_val.get("error") {
                            let msg = err.get("message").and_then(|m| m.as_str()).unwrap_or("RPC error");
                            tracing::warn!("RPC returned error (attempt {}/{}): {}", attempt + 1, max_retries + 1, msg);
                        } else {
                            return Some(json_val);
                        }
                    }
                } else if resp.status().as_u16() == 429 {
                    tracing::warn!("Rate limited (429) on RPC (attempt {}/{})", attempt + 1, max_retries + 1);
                } else {
                    tracing::warn!("HTTP status {} on RPC (attempt {}/{})", resp.status(), attempt + 1, max_retries + 1);
                }
            }
            Err(e) => {
                tracing::warn!("Network error calling RPC (attempt {}/{}): {}", attempt + 1, max_retries + 1, e);
            }
        }

        if attempt < max_retries {
            let base_delay_ms = 100u64 * (1 << attempt);
            let jitter_ms = (Utc::now().timestamp_subsec_millis() as u64) % 75;
            tokio::time::sleep(tokio::time::Duration::from_millis(base_delay_ms + jitter_ms)).await;
        }
    }
    None
}

/// Fire-and-forget directive-transition notifications (issue #28). Delivery
/// failures are logged and never fatal to the ingestion cycle.
async fn dispatch_notifications(state: &Arc<AppState>, text: &str) {
    let cfg = &state.config.notifications;
    if let (Some(token), Some(chat)) = (&cfg.telegram_bot_token, &cfg.telegram_chat_id) {
        if !token.is_empty() && !chat.is_empty() {
            let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
            let payload = json!({ "chat_id": chat, "text": text });
            match state.http_client.post(&url).json(&payload).send().await {
                Ok(r) if r.status().is_success() => {
                    tracing::info!("Telegram notification delivered")
                }
                Ok(r) => tracing::warn!("Telegram notification failed: HTTP {}", r.status()),
                Err(e) => tracing::error!("Telegram notification error: {e}"),
            }
        }
    }
    if let (Some(token), Some(user)) = (&cfg.pushover_token, &cfg.pushover_user) {
        if !token.is_empty() && !user.is_empty() {
            let payload = json!({ "token": token, "user": user, "message": text });
            match state
                .http_client
                .post("https://api.pushover.net/1/messages.json")
                .json(&payload)
                .send()
                .await
            {
                Ok(r) if r.status().is_success() => {
                    tracing::info!("Pushover notification delivered")
                }
                Ok(r) => tracing::warn!("Pushover notification failed: HTTP {}", r.status()),
                Err(e) => tracing::error!("Pushover notification error: {e}"),
            }
        }
    }
    if let Some(url) = &cfg.webhook_url {
        if !url.is_empty() {
            let payload = json!({ "text": text });
            match state.http_client.post(url).json(&payload).send().await {
                Ok(r) if r.status().is_success() => {
                    tracing::info!("Webhook notification delivered")
                }
                Ok(r) => tracing::warn!("Webhook notification failed: HTTP {}", r.status()),
                Err(e) => tracing::error!("Webhook notification error: {e}"),
            }
        }
    }
}

/// Represents an unsigned 256-bit integer as (hi: u128, lo: u128)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct U256(pub u128, pub u128);

impl serde::Serialize for U256 {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("0x{}", self.to_hex_be()))
    }
}

impl<'de> serde::Deserialize<'de> for U256 {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        U256::from_hex_be(&s).map_err(serde::de::Error::custom)
    }
}

impl U256 {
    pub fn from_hex_be(hex_str: &str) -> Result<Self> {
        let clean = hex_str.trim_start_matches("0x");
        if clean.len() > 64 {
            return Err(anyhow::anyhow!("Hex string too long for U256: {}", clean.len()));
        }
        let padded = format!("{:0>64}", clean);
        let hi = u128::from_str_radix(&padded[0..32], 16)?;
        let lo = u128::from_str_radix(&padded[32..64], 16)?;
        Ok(U256(hi, lo))
    }

    /// Wrapping subtraction modulo 2^256
    pub fn wrapping_sub(&self, other: &Self) -> Self {
        let (lo, borrow) = self.1.overflowing_sub(other.1);
        let hi = self.0.wrapping_sub(other.0).wrapping_sub(borrow as u128);
        U256(hi, lo)
    }

    /// Computes (self * liquidity) >> 128 where liquidity is u128
    pub fn mul_div_q128(&self, liquidity: u128) -> u128 {
        let a = self.1 >> 64;
        let b = self.1 & 0xFFFF_FFFF_FFFF_FFFF;
        let c = liquidity >> 64;
        let d = liquidity & 0xFFFF_FFFF_FFFF_FFFF;

        let bd = b * d;
        let ad = a * d;
        let bc = b * c;
        let ac = a * c;

        let mid = (bd >> 64) + (ad & 0xFFFF_FFFF_FFFF_FFFF) + (bc & 0xFFFF_FFFF_FFFF_FFFF);
        let hi_part = ac + (ad >> 64) + (bc >> 64) + (mid >> 64);

        self.0.wrapping_mul(liquidity).wrapping_add(hi_part)
    }

    pub fn to_hex_be(&self) -> String {
        format!("{:032x}{:032x}", self.0, self.1)
    }

    /// Lossy f64 value (magnitude only; used for human-unit pricing math).
    pub fn to_f64(&self) -> f64 {
        self.0 as f64 * 18446744073709551616.0 * 18446744073709551616.0 + self.1 as f64
    }
}

// ============================================================
// Uniswap v3/v4 per-position reads (v2.0.0)
// ============================================================
// Addresses and selectors verified live on Base mainnet 2026-10-24 (Phase-1
// discovery; see ONCHAIN_INTEGRATION.md §5). Selectors were recomputed from
// canonical signatures and cross-checked against the responses they produce.

const UNISWAP_V4_STATE_VIEW_BASE: &str = "0xa3c0c9b65bad0b08107aa264b0f3db444b867a71";
/// Canonical Uniswap v3 WETH/USDC 0.05% pool on Base — the ETH/USD spot source.
/// Resolved via factory getPool(WETH, USDC, 500) and price-verified ($2,373 at
/// discovery time); replaces the liquidated Aerodrome vAMM-WETH/USDC feed.
const DEFAULT_ETH_PRICE_POOL: &str = "0xd0b53d9277642d899df5c87a3966a349a798f224";

const V4_GET_POSITION_LIQUIDITY: &str = "0x1efeed33"; // getPositionLiquidity(uint256)
const V4_POOL_KEYS: &str = "0x86b6be7d"; // poolKeys(bytes25)
// (getPoolAndPositionInfo(uint256) = 0x7ba03aad exists on the POSM; the pipeline
// reads ticks/liquidity via StateView + config instead.)
const V4_STATE_VIEW_GET_SLOT0: &str = "0xc815641c"; // getSlot0(bytes32)
const V4_STATE_VIEW_GET_POSITION_INFO: &str = "0xdacf1d2f"; // getPositionInfo(bytes32,address,int24,int24,bytes32)
const V4_STATE_VIEW_GET_FEE_GROWTH_INSIDE: &str = "0x53e9c1fb"; // getFeeGrowthInside(bytes32,int24,int24)
const V3_POSITIONS: &str = "0x99fbab88"; // positions(uint256)
const V3_SLOT0: &str = "0x3850c7bd"; // slot0()
const V3_FEE_GROWTH_GLOBAL0: &str = "0xf3058399"; // feeGrowthGlobal0X128()
const V3_FEE_GROWTH_GLOBAL1: &str = "0x46141319"; // feeGrowthGlobal1X128()
const V3_TICKS: &str = "0xf30dba93"; // ticks(int24)

const WETH_BASE: &str = "0x4200000000000000000000000000000000000006";
const NATIVE_ETH: &str = "0x0000000000000000000000000000000000000000";

/// Verified token registry for position legs. Unknown tokens fail explicitly —
/// a position leg we cannot symbolize/decimalize is never emitted with guesses.
fn token_info(address: &str) -> Result<PositionTokenInfo> {
    let a = address.trim_start_matches("0x").to_lowercase();
    match a.as_str() {
        "9b5e262cf9bb04869ab40b19af91d2dc85761722" => Ok(PositionTokenInfo {
            address: NOCK_BASE.to_string(),
            symbol: "NOCK".to_string(),
            decimals: 16,
        }),
        "833589fcd6edb6e08f4c7c32d4f71b54bda02913" => Ok(PositionTokenInfo {
            address: USDC_BASE.to_string(),
            symbol: "USDC".to_string(),
            decimals: 6,
        }),
        "4200000000000000000000000000000000000006" => Ok(PositionTokenInfo {
            address: WETH_BASE.to_string(),
            symbol: "WETH".to_string(),
            decimals: 18,
        }),
        "0000000000000000000000000000000000000000" => Ok(PositionTokenInfo {
            address: NATIVE_ETH.to_string(),
            symbol: "ETH".to_string(),
            decimals: 18,
        }),
        other => Err(anyhow::anyhow!(
            "Unregistered token 0x{} in a tracked position; add it to the token registry",
            other
        )),
    }
}

fn is_nock_token(t: &PositionTokenInfo) -> bool {
    t.symbol == "NOCK"
}

fn is_usdc_token(t: &PositionTokenInfo) -> bool {
    t.symbol == "USDC"
}

fn is_eth_token(t: &PositionTokenInfo) -> bool {
    t.symbol == "WETH" || t.symbol == "ETH"
}

/// Uniswap sqrt-price → raw price (token1_raw per token0_raw).
fn sqrt_price_x96_to_raw(sqrt_price_x96: U256) -> f64 {
    let s = sqrt_price_x96.to_f64() / 79228162514264337593543950336.0; // 2^96
    s * s
}

/// sqrt price at a tick boundary: 1.0001^(tick/2).
fn tick_sqrt_price(tick: i32) -> f64 {
    1.0001f64.powf(tick as f64 / 2.0)
}

/// Current position amounts from liquidity + slot0 + tick bounds (raw token units).
/// In-range splits across both tokens; out-of-range positions are single-sided.
fn concentrated_amounts_raw(liquidity: u128, sqrt_price_x96: U256, tick_lower: i32, tick_upper: i32) -> (f64, f64) {
    let l = liquidity as f64;
    let s = sqrt_price_x96_to_raw(sqrt_price_x96).sqrt();
    let sa = tick_sqrt_price(tick_lower);
    let sb = tick_sqrt_price(tick_upper);
    if s <= sa {
        // Entirely token0: L * (1/sa - 1/sb)
        (l * (1.0 / sa - 1.0 / sb), 0.0)
    } else if s >= sb {
        // Entirely token1: L * (sb - sa)
        (0.0, l * (sb - sa))
    } else {
        let amount0 = l * (sb - s) / (s * sb);
        let amount1 = l * (s - sa);
        (amount0, amount1)
    }
}

/// Human display price for a tick in token0-per-token1 units:
/// 1.0001^(-tick) * 10^(dec1 - dec0). For USDC(6)/NOCK(16) this is USDC per NOCK
/// (the historical bracket convention); for ETH(18)/NOCK(16) it is ETH per NOCK.
fn tick_to_display_price_dec(tick: i32, dec0: i32, dec1: i32) -> f64 {
    1.0001f64.powi(-tick) * 10f64.powi(dec1 - dec0)
}

fn parse_hex_word_u128(hex_word: &str) -> Result<u128> {
    u128::from_str_radix(hex_word.trim_start_matches("0x"), 16)
        .map_err(|e| anyhow::anyhow!("bad hex word: {e}"))
}

fn parse_hex_word_i32(hex_word: &str) -> Result<i32> {
    let clean = hex_word.trim_start_matches("0x");
    let v = u32::from_str_radix(clean, 16).map_err(|e| anyhow::anyhow!("bad hex word: {e}"))?;
    Ok(v as i32)
}

/// Adaptive-precision price formatting: 5 decimals for readable values, 8 for
/// sub-cent ones (e.g. ETH-per-NOCK ≈ 0.0000081) so brackets never collapse to
/// "0.00001".
fn fmt_price(v: f64) -> String {
    if v != 0.0 && v.abs() < 0.001 {
        format!("{:.8}", v)
    } else {
        format!("{:.5}", v)
    }
}

// ============================================================
// Golden Path: claim-fees payload engine (v0.2.0)
// ============================================================
// Encoding verified against Base mainnet via eth_call simulation on 2026-10-12
// (helix-specs task 000661 verification/v4collect_sim.py; golden vectors in
// verification/sdk_full.txt match @uniswap/v4-sdk@2.4.1 V4PositionPlanner output
// byte-for-byte). Do not "fix" these constants without re-running that simulation.

const UNISWAP_V4_POSM_BASE: &str = "0x7c5f5a4bbd8fd63184577525326123b519429bdc";
const POSM_MODIFY_LIQUIDITIES_SELECTOR: &str = "dd46508f"; // modifyLiquidities(bytes,uint256)
const V4_ACTION_DECREASE_LIQUIDITY: u8 = 0x01; // liquidity 0 => collect fees only
const V4_ACTION_TAKE_PAIR: u8 = 0x11; // unified Actions enum as deployed on Base
const UNISWAP_V3_COLLECT_SELECTOR: &str = "fc6f7865"; // collect((uint256,address,uint128,uint128)) —
    // Phase-1 ground truth: the NPM collect staticcall used to validate the v3 fee math

fn word_addr(addr: &str) -> String {
    format!("000000000000000000000000{}", addr.trim_start_matches("0x").to_lowercase())
}

fn word_u256(v: u64) -> String {
    format!("{:064x}", v)
}

/// ABI-encode a dynamic `bytes` member: 32-byte length word + data padded to 32-byte multiples.
fn abi_encode_bytes(b_hex: &str) -> String {
    let clean = b_hex.trim_start_matches("0x");
    let pad_pairs = ((64 - (clean.len() % 64)) % 64) / 2;
    format!("{:064x}{}{}", clean.len() / 2, clean, "00".repeat(pad_pairs))
}

/// ABI-encode a `bytes[]` array at its own position: [length, offsets..., contents...].
/// Offsets are in bytes, relative to the start of the array (after the length word).
fn abi_encode_bytes_array(items: &[String]) -> String {
    let n = items.len();
    let mut offsets = String::new();
    let mut contents = String::new();
    let mut acc = 0usize;
    for it in items {
        offsets.push_str(&word_u256((n * 32 + acc) as u64));
        acc += 32 + ((it.len() / 2 + 31) / 32) * 32;
    }
    for it in items {
        contents.push_str(&abi_encode_bytes(it));
    }
    format!("{:064x}{}{}", n, offsets, contents)
}

/// Build the V4 PositionManager `unlockData` for a fee collect:
/// strict abi.encode(bytes actions, bytes[] params) where actions = [DECREASE_LIQUIDITY,
/// TAKE_PAIR] and params are flat member encodings. Byte-for-byte identical to
/// @uniswap/v4-sdk V4PositionPlanner.finalize() for the same inputs (golden vector test).
fn build_v4_collect_unlock_data(token_id: u64, currency0: &str, currency1: &str, recipient: &str) -> String {
    let actions_hex = format!("{:02x}{:02x}", V4_ACTION_DECREASE_LIQUIDITY, V4_ACTION_TAKE_PAIR);
    // DECREASE_LIQUIDITY flat params: [tokenId u256, liquidity u256 (=0 collects fees),
    // amount0Min u128, amount1Min u128, hookData offset (head-relative, incl. offset word
    // => 5*32 = 0xa0), hookData length (=0)]
    let decrease = format!(
        "{}{}{}{}{}{}",
        word_u256(token_id),
        word_u256(0),
        word_u256(0),
        word_u256(0),
        word_u256(0xa0),
        word_u256(0)
    );
    // TAKE_PAIR flat params: [currency0, currency1, recipient] — all static, 3 words
    let take_pair = format!(
        "{}{}{}",
        word_addr(currency0),
        word_addr(currency1),
        word_addr(recipient)
    );
    let params = vec![decrease, take_pair];
    let actions_len_padded_bytes = ((actions_hex.len() / 2 + 31) / 32) * 32;
    let params_offset = 0x40 + 32 + actions_len_padded_bytes;
    format!(
        "0x{}{}{}{}",
        word_u256(0x40),
        word_u256(params_offset as u64),
        abi_encode_bytes(&actions_hex),
        abi_encode_bytes_array(&params)
    )
}

/// Full calldata for PositionManager.modifyLiquidities(bytes,uint256):
/// [selector, 0x40 (offset to unlockData), deadline, bytesLen, unlockData...]
fn build_v4_collect_calldata(token_id: u64, currency0: &str, currency1: &str, recipient: &str, deadline: u64) -> String {
    format!(
        "0x{}{}{}{}",
        POSM_MODIFY_LIQUIDITIES_SELECTOR,
        word_u256(0x40),
        word_u256(deadline),
        abi_encode_bytes(&build_v4_collect_unlock_data(token_id, currency0, currency1, recipient))
    )
}

/// Canonical Uniswap v3 NPM collect: [selector, tokenId, recipient, amount0Max,
/// amount1Max] with type(uint128).max maxes — sweeps all accrued fees + tokensOwed
/// to the recipient. Only valid on the canonical NPM (Phase-1-verified on Base).
fn build_v3_collect_calldata(token_id: u64, recipient: &str) -> String {
    format!(
        "0x{}{}{}{}{}",
        UNISWAP_V3_COLLECT_SELECTOR,
        word_u256(token_id),
        word_addr(recipient),
        format!("{:064x}", u128::MAX),
        format!("{:064x}", u128::MAX),
    )
}

/// Single-action variant: unlockData with only DECREASE_LIQUIDITY (collects fees to
/// the posm ledger without sweeping them out). Live-verified companion to the full
/// collect; useful for isolating decode vs take failures. Kept (test-exercised via
/// golden vector) even though production paths use the full collect variant.
#[allow(dead_code)]
fn build_v4_collect_unlock_data_decrease_only(token_id: u64) -> String {
    let actions_hex = format!("{:02x}", V4_ACTION_DECREASE_LIQUIDITY);
    let decrease = format!(
        "{}{}{}{}{}{}",
        word_u256(token_id),
        word_u256(0),
        word_u256(0),
        word_u256(0),
        word_u256(0xa0),
        word_u256(0)
    );
    let params = vec![decrease];
    let actions_len_padded_bytes = ((actions_hex.len() / 2 + 31) / 32) * 32;
    let params_offset = 0x40 + 32 + actions_len_padded_bytes;
    format!(
        "0x{}{}{}{}",
        word_u256(0x40),
        word_u256(params_offset as u64),
        abi_encode_bytes(&actions_hex),
        abi_encode_bytes_array(&params)
    )
}

/// Live ETH/USD spot from the canonical Uniswap v3 WETH/USDC 0.05% pool slot0
/// (`eth_price_pool` config, default DEFAULT_ETH_PRICE_POOL — factory-resolved and
/// price-verified at Phase-1 discovery). token0 = WETH (18 dec), token1 = USDC (6 dec).
/// Zero-mock: returns None when the RPC query fails — callers must degrade honestly
/// instead of valuing gas at a fabricated price.
async fn fetch_eth_price_usd(http_client: &Client, alchemy_url: &str) -> Option<f64> {
    let payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": DEFAULT_ETH_PRICE_POOL,
            "data": V3_SLOT0
        }, "latest"],
        "id": 99
    });
    if let Some(res_json) = post_rpc_with_retry(http_client, alchemy_url, &payload, 2).await {
        if let Some(hex_str) = res_json.get("result").and_then(|r| r.as_str()) {
            let clean = hex_str.trim_start_matches("0x");
            if clean.len() >= 64 {
                if let Ok(sqrt_price) = U256::from_hex_be(&clean[..64]) {
                    let raw = sqrt_price_x96_to_raw(sqrt_price); // USDC_raw per WETH_raw
                    let usdc_per_eth = raw * 1e18 / 1e6;
                    if usdc_per_eth > 0.0 {
                        return Some(usdc_per_eth);
                    }
                }
            }
        }
    }
    None
}

const USDC_BASE: &str = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";
const NOCK_BASE: &str = "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722";
/// keccak256("Transfer(address,address,uint256)") — verified against the repo keccak tooling.
const TRANSFER_EVENT_TOPIC: &str =
    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ParsedHarvestTransfer {
    token_address: String,
    token_symbol: String,
    from_address: String,
    recipient: String,
    amount_raw: String,
    amount_human: String,
    decimals: i32,
}

/// Exact decimal formatting: raw integer amount -> human string, no float rounding.
/// "120033" @6dp -> "0.120033"; "54468217823004916" @16dp -> "5.4468217823004916".
fn format_units(amount: u128, decimals: i32) -> String {
    let digits = amount.to_string();
    let d = decimals.max(0) as usize;
    let s = if digits.len() <= d {
        format!("0.{}{}", "0".repeat(d - digits.len()), digits)
    } else {
        format!("{}.{}", &digits[..digits.len() - d], &digits[digits.len() - d..])
    };
    if s.contains('.') {
        let trimmed = s.trim_end_matches('0').trim_end_matches('.');
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    s
}

/// Wilder's RSI over a close-price series. Returns one entry per input index;
/// `None` until `period` samples exist. Pure math (no I/O) — the anti-top-buying
/// tripwire per issue #29 ("check the RSI and buy when oversold").
fn compute_rsi_series(closes: &[f64], period: usize) -> Vec<Option<f64>> {
    let mut out = vec![None; closes.len()];
    if period == 0 || closes.len() <= period {
        return out;
    }
    let mut gain_sum = 0.0f64;
    let mut loss_sum = 0.0f64;
    for w in 1..=period {
        let diff = closes[w] - closes[w - 1];
        if diff > 0.0 { gain_sum += diff; } else { loss_sum -= diff; }
    }
    let mut avg_gain = gain_sum / period as f64;
    let mut avg_loss = loss_sum / period as f64;
    let rsi = |g: f64, l: f64| {
        if l == 0.0 { 100.0 } else { 100.0 - 100.0 / (1.0 + g / l) }
    };
    out[period] = Some(rsi(avg_gain, avg_loss));
    for i in period + 1..closes.len() {
        let diff = closes[i] - closes[i - 1];
        let (g, l) = if diff > 0.0 { (diff, 0.0) } else { (0.0, -diff) };
        avg_gain = (avg_gain * (period as f64 - 1.0) + g) / period as f64;
        avg_loss = (avg_loss * (period as f64 - 1.0) + l) / period as f64;
        out[i] = Some(rsi(avg_gain, avg_loss));
    }
    out
}

/// Market regime inputs consumed by the advice engine (computed by the worker
/// from historical prices; the advice engine itself stays pure math).
///
/// Industry convention: RSI(14) means a 14-period lookback — 14 bars of the
/// underlying timeframe, not 14 days. The two series below are therefore named
/// by timeframe: `rsi_5m` is the short-term momentum pulse, `rsi_1d` is the
/// daily RSI(14) that spans the multi-day trend.
#[derive(Debug, Default, Clone, Copy)]
struct MarketRegime {
    /// Latest Wilder RSI(14) over the 5-minute snapshot closes (micro)
    rsi_5m: Option<f64>,
    /// Latest Wilder RSI(14) over daily closes (macro); drives `overextended`
    rsi_1d: Option<f64>,
    /// Overextension: daily RSI(14) >= 70 (overbought exhaustion). At 70+, buy/deploy
    /// advice is suppressed in favor of sweeping gains to stables.
    overextended: bool,
}

const RSI_OVERBOUGHT: f64 = 70.0;
const RSI_PERIOD: usize = 14;

/// Provenance tags for market_price_history rows. Historical rows keep
/// "aerodrome_rpc" (the retired feed); live spots now come from the tracked
/// Uniswap pools.
const PRICE_SOURCE_RPC: &str = "uniswap_rpc";
const PRICE_SOURCE_GECKOTERMINAL: &str = "geckoterminal";
const PRICE_SOURCE_DEXSCREENER: &str = "dexscreener";

// ============================================================
// Market Price History & Backfill Engine (v0.3.0)
// ============================================================

/// A third-party OHLCV candle keyed by its open time (UTC).
#[derive(Debug, Clone, Copy, PartialEq)]
struct PriceCandle {
    timestamp: DateTime<Utc>,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: Option<f64>,
}

/// Parse a GeckoTerminal OHLCV response. `data.attributes.ohlcv_list` entries are
/// [open_time_unix_secs, open, high, low, close, volume], newest-first; malformed
/// entries are skipped and the result is normalized to ascending time. Pure —
/// unit-testable against recorded fixtures.
fn parse_geckoterminal_ohlcv(body: &serde_json::Value) -> Vec<PriceCandle> {
    let mut out = Vec::new();
    if let Some(list) = body
        .get("data")
        .and_then(|d| d.get("attributes"))
        .and_then(|a| a.get("ohlcv_list"))
        .and_then(|l| l.as_array())
    {
        for entry in list {
            let Some(row) = entry.as_array() else { continue };
            if row.len() < 5 {
                continue;
            }
            let (Some(secs), Some(open), Some(high), Some(low), Some(close)) = (
                row[0].as_f64(),
                row[1].as_f64(),
                row[2].as_f64(),
                row[3].as_f64(),
                row[4].as_f64(),
            ) else {
                continue;
            };
            let Some(ts) = DateTime::<Utc>::from_timestamp(secs as i64, 0) else {
                continue;
            };
            if !(open.is_finite() && high.is_finite() && low.is_finite() && close.is_finite())
                || open <= 0.0
            {
                continue;
            }
            out.push(PriceCandle {
                timestamp: ts,
                open,
                high,
                low,
                close,
                volume: row.get(5).and_then(|v| v.as_f64()),
            });
        }
    }
    out.sort_by_key(|c| c.timestamp);
    out.dedup_by_key(|c| c.timestamp);
    out
}

/// Parse DEXScreener's current pair price (fallback source; exposes no history).
fn parse_dexscreener_price(body: &serde_json::Value) -> Option<f64> {
    body.get("pairs")?
        .get(0)?
        .get("priceUsd")?
        .as_str()?
        .parse::<f64>()
        .ok()
        .filter(|p| p.is_finite() && *p > 0.0)
}

/// What the gap detector wants backfilled, decided purely from stored history
/// bounds and wall-clock time (no I/O — unit-testable).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct BackfillPlan {
    /// Hourly candles are missing between the newest stored point and now
    backfill_hourly: bool,
    /// Daily history is shallower than the multi-day RSI lookback
    backfill_daily: bool,
    /// A provider single-point fallback seed is allowed if OHLCV fetches fail
    seed_fallback_point: bool,
}

/// Decide the backfill actions for this cycle. A gap exists when the table is
/// empty or the newest point is older than `gap_threshold_minutes`; depth is
/// insufficient when the earliest point is newer than `now - min_history_days`.
fn plan_backfill(
    last_point: Option<DateTime<Utc>>,
    earliest_point: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    gap_threshold_minutes: i64,
    min_history_days: i64,
) -> BackfillPlan {
    let gap = match last_point {
        None => true,
        Some(last) => {
            now.signed_duration_since(last) > chrono::Duration::minutes(gap_threshold_minutes)
        }
    };
    let depth_insufficient = match earliest_point {
        None => true,
        Some(earliest) => earliest > now - chrono::Duration::days(min_history_days),
    };
    BackfillPlan {
        backfill_hourly: gap,
        backfill_daily: depth_insufficient,
        seed_fallback_point: gap,
    }
}

/// Collapse raw price points (timestamp, candle_close, spot_price) into one close
/// per UTC day. Daily candle closes are authoritative; days without a candle take
/// the last arriving spot (points are ordered ascending). Pure — unit-testable.
fn daily_closes_from_points(points: &[(DateTime<Utc>, Option<f64>, f64)]) -> Vec<f64> {
    use std::collections::BTreeMap;
    // day -> (has_candle_close, close)
    let mut days: BTreeMap<chrono::NaiveDate, (bool, f64)> = BTreeMap::new();
    for &(ts, close_usd, price_usd) in points {
        let day = ts.date_naive();
        match close_usd {
            Some(c) => {
                days.insert(day, (true, c));
            }
            None => {
                let entry = days.entry(day).or_insert((false, price_usd));
                if !entry.0 {
                    entry.1 = price_usd;
                }
            }
        }
    }
    days.into_values().map(|(_, close)| close).collect()
}

/// Latest and earliest stored price points (both None when the table is empty).
async fn price_history_bounds(
    db: &PgPool,
) -> Result<(Option<DateTime<Utc>>, Option<DateTime<Utc>>)> {
    let row = sqlx::query(
        "SELECT MAX(timestamp) AS last_ts, MIN(timestamp) AS earliest_ts FROM market_price_history",
    )
    .fetch_one(db)
    .await?;
    let last: Option<DateTime<Utc>> = row.try_get("last_ts").unwrap_or(None);
    let earliest: Option<DateTime<Utc>> = row.try_get("earliest_ts").unwrap_or(None);
    Ok((last, earliest))
}

/// Insert a single price point with explicit provenance. `ON CONFLICT DO NOTHING`
/// keeps every write idempotent: the first row at a timestamp wins and is never
/// rewritten by a later backfill.
async fn insert_price_point(
    db: &PgPool,
    ts: DateTime<Utc>,
    price_usd: f64,
    source: &str,
    is_backfilled: bool,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO market_price_history (timestamp, price_usd, source, is_backfilled) \
         VALUES ($1, $2, $3, $4) ON CONFLICT (timestamp) DO NOTHING",
    )
    .bind(ts)
    .bind(price_usd)
    .bind(source)
    .bind(is_backfilled)
    .execute(db)
    .await?;
    Ok(())
}

/// Idempotent candle upsert. Only completed candles are accepted: an open time
/// less than one candle period old is the provider's in-progress candle and is
/// skipped so a partial close never gets frozen into history. Returns rows added.
async fn upsert_price_candles(
    db: &PgPool,
    candles: &[PriceCandle],
    source: &str,
    candle_period: chrono::Duration,
    now: DateTime<Utc>,
) -> Result<u64> {
    let mut added = 0u64;
    for c in candles {
        if now.signed_duration_since(c.timestamp) < candle_period {
            continue;
        }
        let result = sqlx::query(
            "INSERT INTO market_price_history \
             (timestamp, price_usd, open_usd, high_usd, low_usd, close_usd, volume_usd, source, is_backfilled, backfilled_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, true, $9) ON CONFLICT (timestamp) DO NOTHING",
        )
        .bind(c.timestamp)
        .bind(c.close)
        .bind(c.open)
        .bind(c.high)
        .bind(c.low)
        .bind(c.close)
        .bind(c.volume)
        .bind(source)
        .bind(now)
        .execute(db)
        .await?;
        added += result.rows_affected();
    }
    Ok(added)
}

/// GET a GeckoTerminal OHLCV series (unauthenticated public endpoint). Returns
/// None on any network/HTTP/parse failure — the caller degrades instead of failing.
async fn fetch_geckoterminal_ohlcv(
    state: &Arc<AppState>,
    timeframe: &str,
    limit: u32,
) -> Option<Vec<PriceCandle>> {
    let url = format!(
        "{}/networks/base/pools/{}/ohlcv/{}?limit={}",
        state.config.market_data.geckoterminal_base_url.trim_end_matches('/'),
        state.config.market_data.pool_address,
        timeframe,
        limit
    );
    let resp = match state
        .http_client
        .get(&url)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("GeckoTerminal {timeframe} OHLCV request failed: {e}");
            return None;
        }
    };
    if !resp.status().is_success() {
        tracing::warn!("GeckoTerminal {timeframe} OHLCV returned HTTP {}", resp.status());
        return None;
    }
    match resp.json::<serde_json::Value>().await {
        Ok(body) => Some(parse_geckoterminal_ohlcv(&body)),
        Err(e) => {
            tracing::warn!("GeckoTerminal {timeframe} OHLCV body parse failed: {e}");
            None
        }
    }
}

/// GET DEXScreener's current pair price (fallback; no historical candles).
async fn fetch_dexscreener_price(state: &Arc<AppState>) -> Option<f64> {
    let url = format!(
        "{}/dex/pairs/base/{}",
        state.config.market_data.dexscreener_base_url.trim_end_matches('/'),
        state.config.market_data.pool_address
    );
    let resp = match state
        .http_client
        .get(&url)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("DEXScreener request failed: {e}");
            return None;
        }
    };
    if !resp.status().is_success() {
        tracing::warn!("DEXScreener returned HTTP {}", resp.status());
        return None;
    }
    match resp.json::<serde_json::Value>().await {
        Ok(body) => parse_dexscreener_price(&body),
        Err(e) => {
            tracing::warn!("DEXScreener body parse failed: {e}");
            None
        }
    }
}

/// Gap detection + backfill (v0.3.0). Heals multi-hour/multi-day gaps in
/// `market_price_history` (laptop sleep, worker downtime) from free public OHLCV
/// feeds, and extends daily depth for the multi-day RSI. Strictly fail-open: any
/// provider or DB failure logs a warning and returns — the ingestion loop and
/// on-chain polling are never blocked. Idempotent: reruns upsert nothing new.
async fn ensure_price_history(state: &Arc<AppState>) {
    if !state.config.market_data.backfill_enabled {
        return;
    }
    let md = &state.config.market_data;
    let (last_point, earliest_point) = match price_history_bounds(&state.db).await {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!("Price-history bounds query failed (backfill skipped): {e}");
            return;
        }
    };
    let now = Utc::now();
    let plan = plan_backfill(
        last_point,
        earliest_point,
        now,
        md.gap_threshold_minutes,
        md.min_history_days,
    );
    if plan == BackfillPlan::default() {
        return;
    }
    tracing::info!(
        "Price backfill: hourly={} daily={} (last={:?}, earliest={:?})",
        plan.backfill_hourly,
        plan.backfill_daily,
        last_point,
        earliest_point
    );

    let mut provider_ok = true;
    if plan.backfill_daily {
        let candles = fetch_geckoterminal_ohlcv(state, "day", md.daily_lookback).await;
        match candles {
            Some(c) => match upsert_price_candles(
                &state.db,
                &c,
                PRICE_SOURCE_GECKOTERMINAL,
                chrono::Duration::days(1),
                now,
            )
            .await
            {
                Ok(n) => tracing::info!("Backfilled {n} daily candle(s)"),
                Err(e) => tracing::warn!("Daily candle upsert failed (non-fatal): {e}"),
            },
            None => provider_ok = false,
        }
    }
    if plan.backfill_hourly {
        let candles = fetch_geckoterminal_ohlcv(state, "hour", md.hourly_lookback).await;
        match candles {
            Some(c) => match upsert_price_candles(
                &state.db,
                &c,
                PRICE_SOURCE_GECKOTERMINAL,
                chrono::Duration::hours(1),
                now,
            )
            .await
            {
                Ok(n) => tracing::info!("Backfilled {n} hourly candle(s)"),
                Err(e) => tracing::warn!("Hourly candle upsert failed (non-fatal): {e}"),
            },
            None => provider_ok = false,
        }
    }

    // DEXScreener fallback: it exposes only the current price, so it may seed a
    // single point stamped at observation time — never backdated into the gap.
    if !provider_ok && plan.seed_fallback_point {
        match fetch_dexscreener_price(state).await {
            Some(price) => {
                tracing::warn!(
                    "GeckoTerminal unavailable; seeding current DEXScreener price (degraded, single point)"
                );
                if let Err(e) =
                    insert_price_point(&state.db, now, price, PRICE_SOURCE_DEXSCREENER, true).await
                {
                    tracing::warn!("DEXScreener seed insert failed (non-fatal): {e}");
                }
            }
            None => {
                tracing::warn!(
                    "Both GeckoTerminal and DEXScreener unavailable; price backfill degraded this cycle"
                )
            }
        }
    }
}

/// Backfill telemetry for /v1/snapshot: how much of the stored price series came
/// from third-party gap fills and when the last one landed. Computed live per
/// request; never persisted into snapshots (keeps state_hash/ETag stable). Fail-open.
async fn data_provenance(db: &PgPool) -> serde_json::Value {
    match sqlx::query(
        "SELECT MAX(backfilled_at) AS last_backfill_at, COUNT(*) AS points \
         FROM market_price_history WHERE is_backfilled",
    )
    .fetch_one(db)
    .await
    {
        Ok(r) => {
            let last: Option<DateTime<Utc>> = r.try_get("last_backfill_at").unwrap_or(None);
            let points: i64 = r.try_get("points").unwrap_or(0);
            // Per-source breakdown so consumers can name the feed instead of
            // hardcoding one (geckoterminal is the norm; dexscreener is the fallback).
            let mut sources = serde_json::Map::new();
            if let Ok(rows) = sqlx::query(
                "SELECT source, COUNT(*) AS points FROM market_price_history \
                 WHERE is_backfilled GROUP BY source ORDER BY points DESC",
            )
            .fetch_all(db)
            .await
            {
                for row in rows {
                    let src: String = row.try_get("source").unwrap_or_default();
                    let n: i64 = row.try_get("points").unwrap_or(0);
                    sources.insert(src, json!(n));
                }
            }
            json!({
                "last_backfill_at": last.map(|t| t.to_rfc3339()),
                "backfilled_points_count": points,
                "sources": sources,
            })
        }
        Err(e) => {
            tracing::warn!("Provenance query failed (non-fatal): {e}");
            json!({
                "last_backfill_at": Option::<String>::None,
                "backfilled_points_count": 0,
                "sources": {},
            })
        }
    }
}

/// Notification config for directive-transition dispatch (issue #28).
#[derive(Deserialize, Debug, Clone, Default)]
struct NotificationConfig {
    telegram_bot_token: Option<String>,
    telegram_chat_id: Option<String>,
    pushover_token: Option<String>,
    pushover_user: Option<String>,
    webhook_url: Option<String>,
    cockpit_url: Option<String>,
}

/// Pure transition decision: notify only when a directive CHANGES between cycles
/// (never on first boot, never on repeats). Returns the message text.
fn directive_transition_notification(
    prev: Option<&str>,
    next: &str,
    details: &str,
    cockpit_url: Option<&str>,
) -> Option<String> {
    let prev = prev?;
    if prev == next {
        return None;
    }
    let mut text = format!("🛡️ NockNock directive change: {}\n{}", next, details);
    if let Some(url) = cockpit_url {
        if !url.is_empty() {
            text.push_str(&format!("\n→ {}", url));
        }
    }
    Some(text)
}

/// Extract ERC-20 Transfer logs that paid the sovereign wallet in a transaction
/// receipt. Logs for tokens we do not track (anything other than USDC/NOCK on
/// Base) are ignored with a warning rather than guessed at.
fn parse_harvest_transfers(receipt: &serde_json::Value, wallet: &str) -> Vec<ParsedHarvestTransfer> {
    let mut out = Vec::new();
    let wallet_padded = format!(
        "000000000000000000000000{}",
        wallet.trim_start_matches("0x").to_lowercase()
    );
    let Some(logs) = receipt.get("logs").and_then(|l| l.as_array()) else {
        return out;
    };
    for log in logs {
        let Some(topics) = log.get("topics").and_then(|t| t.as_array()) else {
            continue;
        };
        if topics.len() < 3 {
            continue;
        }
        let (Some(t0), Some(from_word), Some(to_word)) = (
            topics[0].as_str(),
            topics[1].as_str(),
            topics[2].as_str(),
        ) else {
            continue;
        };
        if !t0.eq_ignore_ascii_case(TRANSFER_EVENT_TOPIC) {
            continue;
        }
        if !to_word
            .trim_start_matches("0x")
            .eq_ignore_ascii_case(&wallet_padded)
        {
            continue;
        }
        let Some(token_addr) = log.get("address").and_then(|a| a.as_str()) else {
            continue;
        };
        let (symbol, decimals): (&str, i32) = if token_addr.eq_ignore_ascii_case(USDC_BASE) {
            ("USDC", 6)
        } else if token_addr.eq_ignore_ascii_case(NOCK_BASE) {
            ("NOCK", 16)
        } else {
            tracing::warn!("Harvest receipt contains a Transfer of an untracked token {}; skipping", token_addr);
            continue;
        };
        let Some(data) = log.get("data").and_then(|d| d.as_str()) else {
            continue;
        };
        let Ok(amount) = u128::from_str_radix(data.trim_start_matches("0x"), 16) else {
            continue;
        };
        out.push(ParsedHarvestTransfer {
            token_address: token_addr.to_string(),
            token_symbol: symbol.to_string(),
            from_address: format!("0x{}", from_word.trim_start_matches("0x")[24..].to_lowercase()),
            recipient: wallet.to_string(),
            amount_raw: amount.to_string(),
            amount_human: format_units(amount, decimals),
            decimals,
        });
    }
    out
}

fn is_valid_tx_hash(s: &str) -> bool {
    let clean = s.trim_start_matches("0x");
    clean.len() == 64 && clean.chars().all(|c| c.is_ascii_hexdigit())
}

/// NOCK spot price from the latest snapshot (stored in token_price_series by every
/// cycle since v0.1.0). Works for historical Aerodrome-derived rows and current
/// Uniswap-derived rows alike.
fn nock_spot_from_snapshot(data: &SnapshotData) -> Option<f64> {
    data.derived_metrics
        .total_value_vs_baseline
        .token_price_series
        .first()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|v| *v > 0.0)
}

#[derive(Serialize, Debug, Clone)]
struct ClaimFeeAction {
    platform: String,
    label: String,
    to: String,
    data: String,
    value: String,
    expected_amount_0: Option<String>,
    expected_amount_1: Option<String>,
    expected_usd: Option<String>,
    gas_estimate: Option<String>,
    gas_usd: Option<String>,
    /// "success" | "failed" | "unavailable" — clients must only dispatch "success"
    simulation: String,
    simulation_detail: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
struct ClaimFeesPayload {
    chain_id: u64,
    from: String,
    generated_at: String,
    gross_usd: Option<String>,
    estimated_gas_usd: Option<String>,
    net_after_gas_usd: Option<String>,
    actions: Vec<ClaimFeeAction>,
}

/// Fill expected amounts + token-aware USD valuation from the matching snapshot
/// position and simulate the collect via eth_call. Shared by the v4 and v3 action
/// builders so both payload halves behave identically (zero-mock: degrades to None).
async fn finish_claim_action(
    http_client: &Client,
    alchemy_url: &str,
    wallet: &str,
    snapshot: &SnapshotEnvelope,
    action: &mut ClaimFeeAction,
    pos_id: &str,
    sim_id: i32,
    nock_spot: Option<f64>,
    eth_price: Option<f64>,
) {
    let price_for = |symbol: &str| -> Option<f64> {
        match symbol {
            "USDC" => Some(1.0),
            "NOCK" => nock_spot,
            "WETH" | "ETH" => eth_price,
            _ => None,
        }
    };
    if let Some(p) = snapshot.data.positions.iter().find(|p| p.position_id == pos_id) {
        action.expected_amount_0 = Some(p.tokens_owed_0.clone());
        action.expected_amount_1 = Some(p.tokens_owed_1.clone());
        if let (Ok(v0), Ok(v1)) = (
            p.tokens_owed_0.parse::<f64>(),
            p.tokens_owed_1.parse::<f64>(),
        ) {
            match (price_for(&p.token0.symbol), price_for(&p.token1.symbol)) {
                (Some(p0), Some(p1)) => {
                    action.expected_usd = Some(format!("{:.4}", v0 * p0 + v1 * p1));
                }
                _ => {
                    action.simulation_detail = Some(
                        "USD valuation unavailable (missing live NOCK or ETH price)".to_string(),
                    );
                }
            }
        }
    }
    let sim_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "from": wallet, "to": action.to, "data": action.data }, "latest"],
        "id": sim_id
    });
    match post_rpc_with_retry(http_client, alchemy_url, &sim_payload, 2).await {
        Some(res_json) => {
            if res_json.get("result").and_then(|r| r.as_str()).is_some() {
                action.simulation = "success".to_string();
            } else {
                action.simulation = "failed".to_string();
                action.simulation_detail = Some("collect eth_call reverted".to_string());
            }
        }
        None => {
            action.simulation_detail = Some("RPC unavailable for collect simulation".to_string());
        }
    }
}

/// GET /v1/execute/claim-fees/payload — unsigned fee-claim transactions for the
/// sovereign wallet, derived from live on-chain state. Clients (Android app, web
/// cockpit) dispatch these to the paired wallet for signing; they never build
/// calldata themselves.
async fn get_claim_fees_payload_handler(Extension(state): Extension<Arc<AppState>>) -> impl IntoResponse {
    let wallet = match state.config.tracked_wallets.first().cloned() {
        Some(w) => w,
        None => {
            return (StatusCode::SERVICE_UNAVAILABLE, Json(json!({
                "error": "NO_TRACKED_WALLET",
                "detail": "No tracked wallet configured; cannot build claim transactions"
            }))).into_response();
        }
    };
    let alchemy_key = state.config.alchemy_api_keys.first().cloned().unwrap_or_default();
    if alchemy_key.is_empty() || alchemy_key == "YOUR_ALCHEMY_API_KEY" {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(json!({
            "error": "No Alchemy RPC key configured; cannot simulate or estimate claim transactions"
        }))).into_response();
    }
    let alchemy_url = format!("https://base-mainnet.g.alchemy.com/v2/{}", alchemy_key);

    // Live snapshot is the source of truth for claimable amounts (zero-mock)
    let snapshot = state.latest_snapshot.lock().await.clone();
    let snapshot = match snapshot {
        Some(s) => s,
        None => {
            return (StatusCode::SERVICE_UNAVAILABLE, Json(json!({
                "error": "No snapshot available yet; ingestion cycle has not completed"
            }))).into_response()
        }
    };
    let gross_usd = snapshot.data.derived_metrics.global_status.total_fees_claimable_usd.clone();

    let deadline = (Utc::now().timestamp() as u64) + 600;
    let mut actions: Vec<ClaimFeeAction> = Vec::new();

    // --- Per-position Uniswap v4 collect actions ---
    // (The Aerodrome tranche was liquidated 2026-10-23; no Aerodrome actions are built.)
    let v4_configs: Vec<&PositionConfig> = state
        .config
        .positions
        .iter()
        .filter(|p| p.protocol == "uniswap-v4")
        .collect();
    let v3_configs: Vec<&PositionConfig> = state
        .config
        .positions
        .iter()
        .filter(|p| p.protocol == "uniswap-v3")
        .collect();
    if v4_configs.is_empty() && v3_configs.is_empty() {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(json!({
            "error": "NO_CLAIMABLE_POSITIONS",
            "detail": "No Uniswap positions configured; cannot build claim transactions"
        }))).into_response();
    }

    // Live prices for token-aware USD valuation (degrade to None, never fabricate).
    let eth_price = fetch_eth_price_usd(&state.http_client, &alchemy_url).await;
    let nock_spot = nock_spot_from_snapshot(&snapshot.data);

    for (idx, pos) in v4_configs.iter().enumerate() {
        let (Some(c0), Some(c1)) = (pos.currency0.as_deref(), pos.currency1.as_deref()) else {
            return (StatusCode::SERVICE_UNAVAILABLE, Json(json!({
                "error": "NO_POSITION_CONFIG",
                "detail": format!("Position {} is missing currency0/currency1 in config", pos.id)
            }))).into_response();
        };
        let v4_data = build_v4_collect_calldata(pos.nft_id, c0, c1, &wallet, deadline);
        let mut action = ClaimFeeAction {
            platform: "uniswap-v4".to_string(),
            label: format!("Collect Uniswap V4 LP fees ({} #{})", pos.id, pos.nft_id),
            to: UNISWAP_V4_POSM_BASE.to_string(),
            data: v4_data,
            value: "0x0".to_string(),
            expected_amount_0: None,
            expected_amount_1: None,
            expected_usd: None,
            gas_estimate: None,
            gas_usd: None,
            simulation: "unavailable".to_string(),
            simulation_detail: None,
        };
        let pos_id = format!("uniswap-v4:{}:{}", pos.network, pos.nft_id);
        finish_claim_action(&state.http_client, &alchemy_url, &wallet, &snapshot,
            &mut action, &pos_id, 20 + idx as i32, nock_spot, eth_price).await;
        actions.push(action);
    }

    // --- Per-position Uniswap v3 collect actions ---
    // v3 fees accrue in the canonical NPM (tokensOwed); collect((uint256,address,
    // uint128,uint128)) with type(uint128).max maxes sweeps them to the wallet. Emitted
    // only for positions whose config carries the canonical npm_address (Phase-1
    // verified on Base); positions without one stay read-only (advice: ACCUMULATE).
    for (idx, pos) in v3_configs.iter().enumerate() {
        let Some(npm) = pos.npm_address.as_deref() else {
            continue;
        };
        let v3_data = build_v3_collect_calldata(pos.nft_id, &wallet);
        let mut action = ClaimFeeAction {
            platform: "uniswap-v3".to_string(),
            label: format!("Collect Uniswap V3 LP fees ({} #{})", pos.id, pos.nft_id),
            to: npm.to_string(),
            data: v3_data,
            value: "0x0".to_string(),
            expected_amount_0: None,
            expected_amount_1: None,
            expected_usd: None,
            gas_estimate: None,
            gas_usd: None,
            simulation: "unavailable".to_string(),
            simulation_detail: None,
        };
        let pos_id = format!("uniswap-v3:{}:{}", pos.network, pos.nft_id);
        finish_claim_action(&state.http_client, &alchemy_url, &wallet, &snapshot,
            &mut action, &pos_id, 20 + (v4_configs.len() + idx) as i32, nock_spot, eth_price).await;
        actions.push(action);
    }

    // --- Gas estimation per action (live; degrades to None, never fabricated) ---
    let gas_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_gasPrice",
        "params": [],
        "id": 22
    });
    let gas_price_wei: Option<u128> = match post_rpc_with_retry(&state.http_client, &alchemy_url, &gas_payload, 2).await {
        Some(res_json) => res_json
            .get("result")
            .and_then(|r| r.as_str())
            .and_then(|s| u128::from_str_radix(s.trim_start_matches("0x"), 16).ok()),
        None => None,
    };
    let mut total_gas_usd = 0.0f64;
    let mut gas_ok = !actions.is_empty();
    for (idx, action) in actions.iter_mut().enumerate() {
        let est_payload = json!({
            "jsonrpc": "2.0",
            "method": "eth_estimateGas",
            "params": [{ "from": wallet, "to": action.to, "data": action.data, "value": action.value }, "latest"],
            "id": 30 + idx
        });
        match post_rpc_with_retry(&state.http_client, &alchemy_url, &est_payload, 2).await {
            Some(res_json) => match res_json.get("result").and_then(|r| r.as_str()).and_then(|s| u128::from_str_radix(s.trim_start_matches("0x"), 16).ok()) {
                Some(units) => {
                    action.gas_estimate = Some(format!("0x{:x}", units));
                    // Zero-mock: gas USD requires BOTH the live gas price and the live
                    // ETH/USD spot; if either is unavailable the USD valuation degrades
                    // to None instead of being valued at a fabricated price.
                    if let (Some(gp), Some(ep)) = (gas_price_wei, eth_price) {
                        let usd = (units as f64) * (gp as f64) / 1e18 * ep;
                        action.gas_usd = Some(format!("{:.4}", usd));
                        total_gas_usd += usd;
                    }
                }
                None => {
                    gas_ok = false;
                    action.simulation_detail = Some(action.simulation_detail.clone().unwrap_or_default()
                        + "; gas estimation failed");
                }
            },
            None => {
                gas_ok = false;
            }
        }
    }
    let estimated_gas_usd = if gas_ok && gas_price_wei.is_some() && eth_price.is_some() {
        Some(format!("{:.4}", total_gas_usd))
    } else {
        None
    };
    let net_after_gas_usd = match (&gross_usd, &estimated_gas_usd) {
        (g, Some(e)) if !g.is_empty() => {
            let gross: f64 = g.parse().unwrap_or(0.0);
            Some(format!("{:.4}", gross - e.parse::<f64>().unwrap_or(0.0)))
        }
        _ => None,
    };

    let payload = ClaimFeesPayload {
        chain_id: 8453,
        from: wallet,
        generated_at: Utc::now().to_rfc3339(),
        gross_usd: Some(gross_usd),
        estimated_gas_usd,
        net_after_gas_usd,
        actions,
    };
    Json(payload).into_response()
}
#[derive(Deserialize, Debug)]
struct RecordHarvestRequest {
    platform: String,
    tx_hash: String,
}

/// POST /v1/execute/record-harvest — receipt-verified harvest ledger entry.
/// Clients call this after a claim transaction is confirmed by the wallet.
/// "Receipt not found" also covers the Uniswap-app pending-broadcast quirk
/// (approved but not yet broadcast): the client should surface that state and retry.
async fn record_harvest_handler(
    Extension(state): Extension<Arc<AppState>>,
    Json(req): Json<RecordHarvestRequest>,
) -> impl IntoResponse {
    if !matches!(req.platform.as_str(), "aerodrome" | "uniswap-v4" | "uniswap-v3") {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "error": "INVALID_PLATFORM", "detail": "platform must be 'aerodrome' (historical), 'uniswap-v4' or 'uniswap-v3'" })),
        )
            .into_response();
    }
    if !is_valid_tx_hash(&req.tx_hash) {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "error": "INVALID_TX_HASH", "detail": "tx_hash must be a 32-byte hex string" })),
        )
            .into_response();
    }
    let alchemy_key = state
        .config
        .alchemy_api_keys
        .first()
        .cloned()
        .unwrap_or_default();
    if alchemy_key.is_empty() || alchemy_key == "YOUR_ALCHEMY_API_KEY" {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "NO_RPC", "detail": "No Alchemy RPC key configured; cannot verify receipts" })),
        )
            .into_response();
    }
    let alchemy_url = format!("https://base-mainnet.g.alchemy.com/v2/{}", alchemy_key);
    let wallet = match state.config.tracked_wallets.first().cloned() {
        Some(w) => w,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "NO_TRACKED_WALLET", "detail": "No tracked wallet configured; cannot verify receipts" })),
            )
                .into_response();
        }
    };

    let receipt_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_getTransactionReceipt",
        "params": [if req.tx_hash.starts_with("0x") { req.tx_hash.clone() } else { format!("0x{}", req.tx_hash) }],
        "id": 40
    });
    let receipt = match post_rpc_with_retry(&state.http_client, &alchemy_url, &receipt_payload, 2).await {
        Some(res) => res.get("result").cloned().unwrap_or(serde_json::Value::Null),
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "RPC_UNAVAILABLE", "detail": "Could not query receipt; not recorded" })),
            )
                .into_response()
        }
    };
    if receipt.is_null() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "RECEIPT_NOT_FOUND",
                "detail": "Transaction not found on-chain. If the wallet shows it approved, it may still be pending broadcast in the wallet (Uniswap app quirk) - surface that state and retry once it broadcasts."
            })),
        )
            .into_response();
    }
    if receipt.get("status").and_then(|s| s.as_str()) != Some("0x1") {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "error": "TX_REVERTED", "detail": "Receipt status is not success; nothing recorded" })),
        )
            .into_response();
    }
    let transfers = parse_harvest_transfers(&receipt, &wallet);
    if transfers.is_empty() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "error": "NO_TRACKED_TRANSFERS", "detail": "Receipt contains no Transfer of tracked tokens (USDC/NOCK) to the sovereign wallet" })),
        )
            .into_response();
    }

    // USD valuation from the latest snapshot spot; explicit null when unavailable.
    let spot = state
        .latest_snapshot
        .lock()
        .await
        .clone()
        .and_then(|s| nock_spot_from_snapshot(&s.data));
    let block_number: i64 = receipt
        .get("blockNumber")
        .and_then(|b| b.as_str())
        .and_then(|s| i64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        .unwrap_or(0);
    // Tax-honest confirmed_at: the block timestamp (when the tx actually landed), not
    // the wall clock at recording time — retries after the pending-broadcast quirk can
    // record long after confirmation, which would misdate the Koinly export. Falls back
    // to now only if the block query fails.
    let confirmed_at: DateTime<Utc> = if block_number > 0 {
        let block_payload = json!({
            "jsonrpc": "2.0",
            "id": 41,
            "method": "eth_getBlockByNumber",
            "params": [format!("0x{block_number:x}"), false]
        });
        post_rpc_with_retry(&state.http_client, &alchemy_url, &block_payload, 2)
            .await
            .and_then(|res| res.get("result").cloned())
            .and_then(|blk| {
                blk.get("timestamp")
                    .and_then(|t| t.as_str())
                    .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            })
            .and_then(|ts| Utc.timestamp_opt(ts as i64, 0).single())
            .unwrap_or_else(Utc::now)
    } else {
        Utc::now()
    };

    let mut recorded = Vec::new();
    let mut skipped = 0usize;
    for t in &transfers {
        let usd_value: Option<String> = match t.token_symbol.as_str() {
            "USDC" => Some(t.amount_human.clone()),
            "NOCK" => spot.map(|s| format!("{:.4}", t.amount_human.parse::<f64>().unwrap_or(0.0) * s)),
            _ => None,
        };
        let res = sqlx::query(
            "INSERT INTO harvest_events (platform, tx_hash, token_address, token_symbol, from_address, recipient, amount_raw, amount_human, decimals, usd_value, block_number, confirmed_at)              VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)              ON CONFLICT (tx_hash, token_address) DO NOTHING",
        )
        .bind(&req.platform)
        .bind(&req.tx_hash)
        .bind(&t.token_address)
        .bind(&t.token_symbol)
        .bind(&t.from_address)
        .bind(&t.recipient)
        .bind(&t.amount_raw)
        .bind(&t.amount_human)
        .bind(t.decimals)
        .bind(&usd_value)
        .bind(block_number)
        .bind(confirmed_at)
        .execute(&state.db)
        .await;
        match res {
            Ok(r) if r.rows_affected() > 0 => {
                recorded.push(json!({
                    "platform": req.platform,
                    "tx_hash": req.tx_hash,
                    "token_symbol": t.token_symbol,
                    "amount_human": t.amount_human,
                    "usd_value": usd_value,
                    "block_number": block_number,
                }));
            }
            Ok(_) => skipped += 1,
            Err(e) => {
                tracing::error!("Failed to insert harvest event: {e}");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "DB_ERROR", "detail": "Ledger insert failed; nothing partially recorded beyond this row" })),
                )
                    .into_response();
            }
        }
    }
    if recorded.is_empty() {
        return (
            StatusCode::CONFLICT,
            Json(json!({ "error": "ALREADY_RECORDED", "detail": "All transfers from this transaction are already in the ledger", "skipped": skipped })),
        )
            .into_response();
    }
    Json(json!({ "recorded": recorded, "already_present": skipped })).into_response()
}

#[derive(Serialize, Debug, Clone)]
struct HarvestExportRow {
    platform: String,
    tx_hash: String,
    token_symbol: String,
    amount_human: String,
    usd_value: Option<String>,
    confirmed_at: String,
}

/// Koinly custom-CSV rendering of harvest ledger rows: fee claims are tagged
/// `income` (per CHANGELOG: Income / Staking Reward) and carry their Base tx hash.
/// Rebalances/liquidity transfers are not yet ledgered — when they are, they export
/// with the (non-taxable) `transfer` label; documented gap, not silently omitted.
fn format_koinly_csv(rows: &[HarvestExportRow]) -> String {
    let header = "Date, Sent Amount, Sent Currency, Received Amount, Received Currency, Fee Amount, Fee Currency, Net Worth Amount, Net Worth Currency, Label, Description, TxHash";
    let mut out = String::from(header);
    out.push('\n');
    for r in rows {
        let (net_amount, net_currency) = match &r.usd_value {
            Some(usd) => (usd.clone(), "USD".to_string()),
            None => (String::new(), String::new()),
        };
        let description = format!("NockNock fee harvest ({})", r.platform);
        out.push_str(&format!(
            "{}, , , {}, {}, , , {}, {}, income, \"{}\", {}\n",
            r.confirmed_at,
            r.amount_human,
            r.token_symbol,
            net_amount,
            net_currency,
            description,
            r.tx_hash
        ));
    }
    out
}

/// GET /v1/tax/export?format=koinly — Koinly-compatible CSV (default) or JSON export
/// of the receipt-verified harvest ledger. Empty ledger exports a header-only CSV.
async fn tax_export_handler(
    Extension(state): Extension<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let format = params.get("format").cloned().unwrap_or_else(|| "koinly".to_string());
    let rows = match sqlx::query(
        "SELECT platform, tx_hash, token_symbol, amount_human, usd_value, confirmed_at \
         FROM harvest_events ORDER BY confirmed_at ASC, id ASC",
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(rows) => {
            let mut out = Vec::new();
            for r in rows {
                let confirmed_at: DateTime<Utc> = r.try_get("confirmed_at").unwrap_or_else(|_| Utc::now());
                out.push(HarvestExportRow {
                    platform: r.try_get("platform").unwrap_or_default(),
                    tx_hash: r.try_get("tx_hash").unwrap_or_default(),
                    token_symbol: r.try_get("token_symbol").unwrap_or_default(),
                    amount_human: r.try_get("amount_human").unwrap_or_default(),
                    usd_value: r.try_get::<Option<String>, _>("usd_value").ok().flatten(),
                    confirmed_at: confirmed_at.to_rfc3339(),
                });
            }
            out
        }
        Err(e) => {
            tracing::error!("Tax export query failed: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "DB_ERROR", "detail": "Tax export query failed" })),
            )
                .into_response();
        }
    };
    match format.as_str() {
        "json" => Json(json!({ "rows": rows })).into_response(),
        "koinly" | "csv" => {
            (
                StatusCode::OK,
                [
                    ("content-type", "text/csv; charset=utf-8"),
                    (
                        "content-disposition",
                        "attachment; filename=\"nocknock-harvests-koinly.csv\"",
                    ),
                ],
                format_koinly_csv(&rows),
            )
                .into_response()
        }
        _ => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "error": "UNSUPPORTED_FORMAT", "detail": "format must be 'koinly' (default) or 'json'" })),
        )
            .into_response(),
    }
}

// (The retired V4_TICK_LOWER_ONCHAIN / V4_TICK_UPPER_ONCHAIN constants and the
// single-position tick_to_display_price() were removed in v2.0.0: tick bounds now
// come from the per-position config/discovery, and display prices from
// tick_to_display_price_dec() with the pool's own token decimals.)

/// Signed basis-point distance from spot to the nearest bracket edge
/// (positive when spot is below the edge, negative when above).
fn bracket_distance_bps(spot: f64, lower: f64, upper: f64) -> i32 {
    if spot <= 0.0 || lower <= 0.0 || upper <= 0.0 {
        return 0;
    }
    let nearest_edge = if (spot - lower).abs() <= (spot - upper).abs() { lower } else { upper };
    (((nearest_edge - spot) / spot) * 10000.0).round() as i32
}

async fn fetch_raw_data(state: &Arc<AppState>) -> Result<RawOnChainData> {
    let alchemy_key = state.config.alchemy_api_keys.first().cloned().unwrap_or_default();
    let alchemy_url = format!("https://base-mainnet.g.alchemy.com/v2/{}", alchemy_key);
    let wallet_addr = &state.config.tracked_wallets[0];

    // 0. Fetch live ETH/USD spot price from the canonical Uniswap v3 WETH/USDC pool slot0.
    //    (The former Aerodrome vAMM-WETH/USDC source is retired along with the liquidated
    //    Aerodrome tranche.) Zero-mock invariant: a failed query fails the cycle.
    let eth_price_pool = state.config.eth_price_pool.clone().unwrap_or_else(|| DEFAULT_ETH_PRICE_POOL.to_string());
    let eth_slot0_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": eth_price_pool,
            "data": V3_SLOT0
        }, "latest"],
        "id": 99
    });
    let mut live_eth_price_usd = 0.0f64;
    let mut eth_price_found = false;
    if let Some(res_json) = post_rpc_with_retry(&state.http_client, &alchemy_url, &eth_slot0_payload, 2).await {
        if let Some(hex_str) = res_json.get("result").and_then(|r| r.as_str()) {
            let clean = hex_str.trim_start_matches("0x");
            if clean.len() >= 64 {
                if let Ok(sqrt_price) = U256::from_hex_be(&clean[..64]) {
                    // token0 = WETH (18 dec), token1 = USDC (6 dec):
                    // raw price = USDC_raw per WETH_raw -> human USDC per ETH
                    let usdc_per_eth = sqrt_price_x96_to_raw(sqrt_price) * 1e18 / 1e6;
                    if usdc_per_eth > 0.0 {
                        live_eth_price_usd = usdc_per_eth;
                        eth_price_found = true;
                    }
                }
            }
        }
    }
    if !eth_price_found {
        return Err(anyhow::anyhow!(
            "Failed to query live ETH/USD spot price from the v3 WETH/USDC pool ({}) slot0",
            eth_price_pool
        ));
    }

    // 1. Fetch live gas price (eth_gasPrice) — required input, fail explicitly on error
    let gas_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_gasPrice",
        "params": [],
        "id": 10
    });
    let mut live_gas_cost_usd = 0.0f64;
    let mut gas_price_found = false;
    if let Some(res_json) = post_rpc_with_retry(&state.http_client, &alchemy_url, &gas_payload, 2).await {
        if let Some(hex_str) = res_json.get("result").and_then(|r| r.as_str()) {
            let clean = hex_str.trim_start_matches("0x");
            if let Ok(wei) = u128::from_str_radix(clean, 16) {
                let eth_gas_cost = (wei as f64 / 1e18) * 150_000.0;
                // The +$0.002 add covers Base L1 data-fee volatility (documented buffer,
                // not a mock price). No upper clamp: a silent cap understated real gas
                // during fee spikes and biased the claim decision toward "claim now".
                let usd_gas_cost = (eth_gas_cost * live_eth_price_usd) + 0.002;
                live_gas_cost_usd = usd_gas_cost.max(0.002);
                gas_price_found = true;
            }
        }
    }
    if !gas_price_found {
        return Err(anyhow::anyhow!("Failed to query live gas price from RPC"));
    }

    // 2. Per-position reads (v2.0.0): every configured Uniswap position is read live;
    //    there are no Aerodrome LP-share reads anymore (tranche liquidated 2026-10-23).
    if state.config.positions.is_empty() {
        return Err(anyhow::anyhow!(
            "No positions configured; add entries under `positions:` in config.yaml (see ONCHAIN_INTEGRATION.md §5)"
        ));
    }
    let mut raw_positions: Vec<RawPosition> = Vec::new();
    for pos in &state.config.positions {
        let raw = match pos.protocol.as_str() {
            "uniswap-v4" => {
                fetch_v4_position(&state.http_client, &alchemy_url, pos, wallet_addr).await?
            }
            "uniswap-v3" => {
                fetch_v3_position(&state.http_client, &alchemy_url, pos, wallet_addr).await?
            }
            other => {
                return Err(anyhow::anyhow!(
                    "Position {} has unsupported protocol '{}' (expected uniswap-v4 or uniswap-v3)",
                    pos.id,
                    other
                ));
            }
        };
        raw_positions.push(raw);
    }

    // 3. NOCK/USD spot from the designated price-source position pool (zero-mock:
    //    fail the cycle when missing or unparseable).
    let spot_price = nock_spot_from_positions(&raw_positions).ok_or_else(|| {
        anyhow::anyhow!(
            "No NOCK/USD spot: exactly one configured position must have is_price_source: true \
             and quote NOCK against USDC"
        )
    })?;

    // 4. Fetch live token balances for tracked wallet

    // 5. Fetch live token balances for tracked wallet
    let balances_payload = json!({
        "jsonrpc": "2.0",
        "method": "alchemy_getTokenBalances",
        "params": [
            wallet_addr,
            [
                "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913", // USDC
                "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722"  // NOCK
            ]
        ],
        "id": 2
    });

    let mut nock_balance = 0.0f64;
    let mut usdc_balance = 0.0f64;
    let mut balances_found = false;

    if let Some(res_json) = post_rpc_with_retry(&state.http_client, &alchemy_url, &balances_payload, 2).await {
        if let Some(balances) = res_json.get("result").and_then(|r| r.get("tokenBalances")).and_then(|t| t.as_array()) {
            for b in balances {
                let addr = b.get("contractAddress").and_then(|a| a.as_str()).unwrap_or_default().to_lowercase();
                let hex_bal = b.get("tokenBalance").and_then(|h| h.as_str()).unwrap_or("0x0").trim_start_matches("0x");
                if addr == "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913" {
                    if let Ok(val) = u128::from_str_radix(hex_bal, 16) {
                        usdc_balance = val as f64 / 1e6;
                        balances_found = true;
                    }
                } else if addr == "0x9b5e262cf9bb04869ab40b19af91d2dc85761722" {
                    if let Ok(val) = u128::from_str_radix(hex_bal, 16) {
                        nock_balance = val as f64 / 1e16;
                        balances_found = true;
                    }
                }
            }
        }
    }

    if !balances_found {
        return Err(anyhow::anyhow!("Failed to query live token balances from Alchemy RPC"));
    }

    let nock_balance_usd = nock_balance * spot_price;
    let now_str = Utc::now().to_rfc3339();

    // 5. Snapshot-level builders (sync, unit-testable): USD pricing, pool mechanics,
    //    and per-position contract fees.
    let positions = build_position_snapshots(&raw_positions, spot_price, live_eth_price_usd);
    let pool_mechanics = build_pool_mechanics(&raw_positions, spot_price, live_eth_price_usd);
    let contract_fees = build_contract_fees(&positions, live_gas_cost_usd);

    tracing::info!(
        "Live position reads: {} position(s), NOCK spot ${:.6}, ETH/USD ${:.2}",
        positions.len(),
        spot_price,
        live_eth_price_usd
    );

    Ok(RawOnChainData {
        wallet_balances: vec![
            WalletBalance {
                wallet_address: wallet_addr.clone(),
                token_symbol: "NOCK".to_string(),
                token_address: "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722".to_string(),
                amount: format!("{:.4}", nock_balance),
                amount_usd: format!("{:.2}", nock_balance_usd),
                updated_at: now_str.clone(),
            },
            WalletBalance {
                wallet_address: wallet_addr.clone(),
                token_symbol: "USDC".to_string(),
                token_address: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".to_string(),
                amount: format!("{:.4}", usdc_balance),
                amount_usd: format!("{:.2}", usdc_balance),
                updated_at: now_str.clone(),
            },
        ],
        pool_mechanics,
        concentrated_liquidity: vec![], // DEPRECATED since v2.0.0 — see `positions`
        contract_fees,
        positions: raw_positions,
        spot_price_nock_usd: spot_price,
        live_gas_cost_usd,
        live_eth_price_usd,
    })
}

/// NOCK/USD spot from the designated price-source position. The pool must quote
/// NOCK against USDC (either ordering); the conversion follows the pool's decimals.
fn nock_spot_from_positions(positions: &[RawPosition]) -> Option<f64> {
    let src = positions.iter().find(|p| p.is_price_source)?;
    let raw = sqrt_price_x96_to_raw(src.sqrt_price_x96); // token1_raw per token0_raw
    let price = if is_usdc_token(&src.token0) && is_nock_token(&src.token1) {
        // raw = NOCK_raw per USDC_raw -> USDC per NOCK = raw^-1 * 10^(dec1-dec0)
        10f64.powi(src.token1.decimals - src.token0.decimals) / raw
    } else if is_nock_token(&src.token0) && is_usdc_token(&src.token1) {
        // raw = USDC_raw per NOCK_raw -> USDC per NOCK = raw * 10^(dec1-dec0)... dec0=16, dec1=6
        raw * 10f64.powi(src.token1.decimals - src.token0.decimals)
    } else {
        return None;
    };
    if price > 0.0 { Some(price) } else { None }
}

/// USD price of one human unit of a position leg.
fn leg_usd_price(token: &PositionTokenInfo, nock_usd: f64, eth_usd: f64) -> f64 {
    if is_usdc_token(token) {
        1.0
    } else if is_nock_token(token) {
        nock_usd
    } else if is_eth_token(token) {
        eth_usd
    } else {
        0.0
    }
}

/// Build the snapshot `positions` array from raw reads + live prices.
fn build_position_snapshots(raw_positions: &[RawPosition], nock_usd: f64, eth_usd: f64) -> Vec<Position> {
    raw_positions
        .iter()
        .map(|p| {
            let p0 = leg_usd_price(&p.token0, nock_usd, eth_usd);
            let p1 = leg_usd_price(&p.token1, nock_usd, eth_usd);
            let amount_usd = p.amount0 * p0 + p.amount1 * p1;
            let fees_usd = p.fee0 * p0 + p.fee1 * p1;
            Position {
                position_id: p.position_id.clone(),
                network: p.network.clone(),
                wallet: p.wallet.clone(),
                protocol: p.protocol.clone(),
                nft_id: p.nft_id,
                pool_id: p.pool_id.clone(),
                npm_address: p.npm_address.clone(),
                fee_tier_bps: p.fee_tier_bps,
                token0: p.token0.clone(),
                token1: p.token1.clone(),
                amount0: format!("{:.6}", p.amount0),
                amount1: format!("{:.6}", p.amount1),
                amount_usd: format!("{:.2}", amount_usd),
                tokens_owed_0: format!("{:.6}", p.fee0),
                tokens_owed_1: format!("{:.6}", p.fee1),
                fees_usd: format!("{:.6}", fees_usd),
                lower_tick: p.tick_lower,
                upper_tick: p.tick_upper,
                // Display orientation: bigger tick -> lower price (token0-per-token1).
                lower_price: format!(
                    "{}",
                    fmt_price(tick_to_display_price_dec(p.tick_upper, p.token0.decimals, p.token1.decimals))
                ),
                upper_price: format!(
                    "{}",
                    fmt_price(tick_to_display_price_dec(p.tick_lower, p.token0.decimals, p.token1.decimals))
                ),
                liquidity_amount: p.liquidity.to_string(),
                in_range: p.in_range,
                is_price_source: p.is_price_source,
            }
        })
        .collect()
}

/// One PoolMechanic per tracked position's pool. volatile_assets carry the tracked
/// wallet's amounts in that pool (NOT pool-wide reserves).
fn build_pool_mechanics(raw_positions: &[RawPosition], nock_usd: f64, eth_usd: f64) -> Vec<PoolMechanic> {
    raw_positions
        .iter()
        .map(|p| {
            let platform = match p.protocol.as_str() {
                "uniswap-v4" => "UniswapV4",
                "uniswap-v3" => "UniswapV3",
                "aerodrome" => "Aerodrome",
                _ => "Other",
            };
            let p0 = leg_usd_price(&p.token0, nock_usd, eth_usd);
            let p1 = leg_usd_price(&p.token1, nock_usd, eth_usd);
            PoolMechanic {
                pool_id: p.pool_id.clone(),
                platform: platform.to_string(),
                pool_type: "concentrated".to_string(),
                volatile_assets: vec![
                    PoolAsset {
                        token_address: p.token0.address.clone(),
                        token_symbol: p.token0.symbol.clone(),
                        reserve_amount: format!("{:.6}", p.amount0),
                        reserve_usd: format!("{:.2}", p.amount0 * p0),
                    },
                    PoolAsset {
                        token_address: p.token1.address.clone(),
                        token_symbol: p.token1.symbol.clone(),
                        reserve_amount: format!("{:.6}", p.amount1),
                        reserve_usd: format!("{:.2}", p.amount1 * p1),
                    },
                ],
                liquidity_usd: format!("{:.2}", p.amount0 * p0 + p.amount1 * p1),
                fee_accrued_usd: format!("{:.6}", p.fee0 * p0 + p.fee1 * p1),
                fee_tier_bps: p.fee_tier_bps,
                tick_current: Some(p.tick),
                sqrt_price_x96: Some(format!("{:032x}{:032x}", p.sqrt_price_x96.0, p.sqrt_price_x96.1)),
            }
        })
        .collect()
}

/// One ContractFee per tracked position with its unclaimed fees.
fn build_contract_fees(positions: &[Position], live_gas_cost_usd: f64) -> Vec<ContractFee> {
    positions
        .iter()
        .map(|p| {
            let label = match p.protocol.as_str() {
                "uniswap-v4" => "Uniswap V4".to_string(),
                "uniswap-v3" => "Uniswap V3".to_string(),
                "aerodrome" => "Aerodrome".to_string(),
                _ => p.protocol.clone(),
            };
            ContractFee {
                contract_address: if p.protocol == "uniswap-v3" {
                    p.npm_address.clone().unwrap_or_else(|| p.pool_id.clone())
                } else {
                    UNISWAP_V4_POSM_BASE.to_string()
                },
                fee_token_address: p.token0.address.clone(),
                fee_token_symbol: format!("{}+{} ({} #{})", p.token0.symbol, p.token1.symbol, label, p.nft_id),
                amount: format!(
                    "{:.p0$} {} + {:.p1$} {}",
                    p.tokens_owed_0.parse::<f64>().unwrap_or(0.0),
                    p.token0.symbol,
                    p.tokens_owed_1.parse::<f64>().unwrap_or(0.0),
                    p.token1.symbol,
                    p0 = (p.token0.decimals as usize).min(8),
                    p1 = (p.token1.decimals as usize).min(8),
                ),
                amount_usd: p.fees_usd.clone(),
                claimable: p.fees_usd.parse::<f64>().unwrap_or(0.0) > 0.0,
                gas_break_even_usd: format!("{:.4}", live_gas_cost_usd),
            }
        })
        .collect()
}

// ------------------------------------------------------------------
// Async per-position readers
// ------------------------------------------------------------------

async fn fetch_v4_position(
    http_client: &Client,
    alchemy_url: &str,
    pos: &PositionConfig,
    wallet: &str,
) -> Result<RawPosition> {
    let pool_id = pos
        .pool_id
        .as_deref()
        .map(|s| s.trim_start_matches("0x").to_lowercase())
        .filter(|s| s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()))
        .ok_or_else(|| anyhow::anyhow!("position {} missing/invalid v4 pool_id (re-run discovery)", pos.id))?;
    let currency0 = pos
        .currency0
        .as_deref()
        .map(|s| s.trim_start_matches("0x").to_lowercase())
        .ok_or_else(|| anyhow::anyhow!("position {} missing currency0 in config", pos.id))?;
    let currency1 = pos
        .currency1
        .as_deref()
        .map(|s| s.trim_start_matches("0x").to_lowercase())
        .ok_or_else(|| anyhow::anyhow!("position {} missing currency1 in config", pos.id))?;
    let tick_lower = pos
        .tick_lower
        .ok_or_else(|| anyhow::anyhow!("position {} missing tick_lower in config", pos.id))?;
    let tick_upper = pos
        .tick_upper
        .ok_or_else(|| anyhow::anyhow!("position {} missing tick_upper in config", pos.id))?;

    // 0. poolKeys(bytes25) -> (currency0, currency1, fee, tickSpacing, hooks);
    //    validates the config's declared currency ordering and supplies the fee tier.
    let pk_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": UNISWAP_V4_POSM_BASE,
            "data": format!("{}{}{}", V4_POOL_KEYS, &pool_id[..50], "0".repeat(14))
        }, "latest"],
        "id": 50
    });
    let mut fee_tier_bps = 0u32;
    if let Some(res_json) = post_rpc_with_retry(http_client, alchemy_url, &pk_payload, 2).await {
        if let Some(hex_str) = res_json.get("result").and_then(|r| r.as_str()) {
            let clean = hex_str.trim_start_matches("0x");
            if clean.len() >= 160 {
                let c0 = clean[24..64].to_lowercase();
                let c1 = clean[88..128].to_lowercase();
                let fee = parse_hex_word_u128(&clean[128..192]).unwrap_or(0) as u64;
                if c0 != currency0 || c1 != currency1 {
                    return Err(anyhow::anyhow!(
                        "position {}: poolKeys currency ordering (0x{}, 0x{}) does not match config ({}, {}); re-run discovery",
                        pos.id,
                        c0,
                        c1,
                        pos.currency0.as_deref().unwrap_or(""),
                        pos.currency1.as_deref().unwrap_or("")
                    ));
                }
                fee_tier_bps = (fee / 100) as u32; // fee is in hundredths of a basis point
            }
        }
    }
    if fee_tier_bps == 0 {
        return Err(anyhow::anyhow!(
            "position {}: failed to read poolKeys fee tier from POSM",
            pos.id
        ));
    }

    // 1. getPositionLiquidity(uint256)
    let liq_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": UNISWAP_V4_POSM_BASE,
            "data": format!("{}{:064x}", V4_GET_POSITION_LIQUIDITY, pos.nft_id)
        }, "latest"],
        "id": 51
    });
    let liquidity = post_rpc_with_retry(http_client, alchemy_url, &liq_payload, 2)
        .await
        .and_then(|r| r.get("result").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .and_then(|s| u128::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "position {}: failed to query getPositionLiquidity from POSM",
                pos.id
            )
        })?;

    // 2. StateView getSlot0(bytes32) -> (sqrtPriceX96, tick, ...)
    let slot0_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": UNISWAP_V4_STATE_VIEW_BASE,
            "data": format!("{}{}", V4_STATE_VIEW_GET_SLOT0, pool_id)
        }, "latest"],
        "id": 52
    });
    let slot0_hex = post_rpc_with_retry(http_client, alchemy_url, &slot0_payload, 2)
        .await
        .and_then(|r| r.get("result").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .ok_or_else(|| anyhow::anyhow!("position {}: StateView getSlot0 query failed", pos.id))?;
    let clean = slot0_hex.trim_start_matches("0x");
    if clean.len() < 128 {
        return Err(anyhow::anyhow!("position {}: StateView getSlot0 returned short output", pos.id));
    }
    let sqrt_price_x96 = U256::from_hex_be(&clean[..64])?;
    let tick = parse_hex_word_i32(&clean[64..128])?;

    // 3. Uncollected fees: getPositionInfo(poolId, posm, tickLower, tickUpper, tokenId)
    //    + getFeeGrowthInside(poolId, tickLower, tickUpper), existing verified selectors.
    let pos_info_data = format!(
        "{}{}{}{}{}{:064x}",
        V4_STATE_VIEW_GET_POSITION_INFO,
        pool_id,
        word_addr(UNISWAP_V4_POSM_BASE),
        format!("{:064x}", tick_lower as u32),
        format!("{:064x}", tick_upper as u32),
        pos.nft_id
    );
    let pos_info_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": UNISWAP_V4_STATE_VIEW_BASE,
            "data": pos_info_data
        }, "latest"],
        "id": 53
    });
    let fg_data = format!(
        "{}{}{}{}",
        V4_STATE_VIEW_GET_FEE_GROWTH_INSIDE,
        pool_id,
        format!("{:064x}", tick_lower as u32),
        format!("{:064x}", tick_upper as u32)
    );
    let fg_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": UNISWAP_V4_STATE_VIEW_BASE,
            "data": fg_data
        }, "latest"],
        "id": 54
    });
    let (fg0_last, fg1_last, fg0_curr, fg1_curr) = match (
        post_rpc_with_retry(http_client, alchemy_url, &pos_info_payload, 2).await,
        post_rpc_with_retry(http_client, alchemy_url, &fg_payload, 2).await,
    ) {
        (Some(res_pos), Some(res_fg)) => {
            let hex_pos = res_pos.get("result").and_then(|r| r.as_str()).unwrap_or("");
            let hex_fg = res_fg.get("result").and_then(|r| r.as_str()).unwrap_or("");
            let cp = hex_pos.trim_start_matches("0x");
            let cf = hex_fg.trim_start_matches("0x");
            if cp.len() < 192 || cf.len() < 128 {
                return Err(anyhow::anyhow!(
                    "position {}: StateView getPositionInfo/getFeeGrowthInside returned short output",
                    pos.id
                ));
            }
            (
                U256::from_hex_be(&cp[64..128])?,
                U256::from_hex_be(&cp[128..192])?,
                U256::from_hex_be(&cf[0..64])?,
                U256::from_hex_be(&cf[64..128])?,
            )
        }
        _ => {
            return Err(anyhow::anyhow!(
                "position {}: failed to query StateView fee growth",
                pos.id
            ));
        }
    };
    let fee0_raw = fg0_curr.wrapping_sub(&fg0_last).mul_div_q128(liquidity);
    let fee1_raw = fg1_curr.wrapping_sub(&fg1_last).mul_div_q128(liquidity);

    let token0 = token_info(&format!("0x{}", currency0))?;
    let token1 = token_info(&format!("0x{}", currency1))?;
    let (amount0_raw, amount1_raw) = concentrated_amounts_raw(liquidity, sqrt_price_x96, tick_lower, tick_upper);
    let dec0 = 10f64.powi(token0.decimals);
    let dec1 = 10f64.powi(token1.decimals);

    tracing::info!(
        "v4 position {} (NFT #{}): liquidity={}, tick={}, in_range={}, fees raw {}/{}",
        pos.id, pos.nft_id, liquidity, tick, tick_lower <= tick && tick <= tick_upper, fee0_raw, fee1_raw
    );

    Ok(RawPosition {
        position_id: format!("uniswap-v4:{}:{}", pos.network, pos.nft_id),
        network: pos.network.clone(),
        protocol: "uniswap-v4".to_string(),
        wallet: wallet.to_string(),
        nft_id: pos.nft_id,
        pool_id: format!("0x{}", pool_id),
        npm_address: None,
        fee_tier_bps,
        token0,
        token1,
        liquidity,
        sqrt_price_x96,
        tick,
        tick_lower,
        tick_upper,
        amount0: amount0_raw / dec0,
        amount1: amount1_raw / dec1,
        fee0: fee0_raw as f64 / dec0,
        fee1: fee1_raw as f64 / dec1,
        in_range: tick_lower <= tick && tick <= tick_upper,
        is_price_source: pos.is_price_source,
    })
}

async fn fetch_v3_position(
    http_client: &Client,
    alchemy_url: &str,
    pos: &PositionConfig,
    wallet: &str,
) -> Result<RawPosition> {
    let npm = pos
        .npm_address
        .clone()
        .ok_or_else(|| anyhow::anyhow!("position {} missing npm_address in config", pos.id))?;
    let pool = pos
        .pool_address
        .clone()
        .ok_or_else(|| anyhow::anyhow!(
            "position {} missing pool_address in config (resolve via factory getPool at discovery)",
            pos.id
        ))?;

    // 1. positions(uint256) -> 12-word decode (self-describing: tokens/fee/ticks/liquidity)
    let positions_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": npm,
            "data": format!("{}{:064x}", V3_POSITIONS, pos.nft_id)
        }, "latest"],
        "id": 60
    });
    let res_hex = post_rpc_with_retry(http_client, alchemy_url, &positions_payload, 2)
        .await
        .and_then(|r| r.get("result").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .ok_or_else(|| anyhow::anyhow!("position {}: NPM positions() query failed", pos.id))?;
    let decoded = decode_v3_positions_return(&res_hex)
        .map_err(|e| anyhow::anyhow!("position {}: {}", pos.id, e))?;
    if decoded.token0 == NATIVE_ETH || decoded.token1 == NATIVE_ETH {
        return Err(anyhow::anyhow!(
            "position {}: NFT #{} decoded with a zero token address",
            pos.id,
            pos.nft_id
        ));
    }

    // 2. pool.slot0() -> (sqrtPriceX96, tick, ...)
    let slot0_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "to": pool, "data": V3_SLOT0 }, "latest"],
        "id": 61
    });
    let slot0_hex = post_rpc_with_retry(http_client, alchemy_url, &slot0_payload, 2)
        .await
        .and_then(|r| r.get("result").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .ok_or_else(|| anyhow::anyhow!("position {}: pool slot0 query failed", pos.id))?;
    let clean = slot0_hex.trim_start_matches("0x");
    if clean.len() < 128 {
        return Err(anyhow::anyhow!("position {}: pool slot0 returned short output", pos.id));
    }
    let sqrt_price_x96 = U256::from_hex_be(&clean[..64])?;
    let tick = parse_hex_word_i32(&clean[64..128])?;

    // 3. v3 uncollected fees: inside = global - outside(lower) - outside(upper);
    //    owed = (inside - insideLast) * L / 2^128 + tokensOwed. TickInfo word layout:
    //    [0] liquidityGross, [1] liquidityNet, [2] feeGrowthOutside0, [3] feeGrowthOutside1
    //    (this pool returns an 8-word struct; words 4+ are fork extras we ignore).
    let fg0_g = post_rpc_word(http_client, alchemy_url, pool.as_str(), V3_FEE_GROWTH_GLOBAL0, 0, 62).await?;
    let fg1_g = post_rpc_word(http_client, alchemy_url, pool.as_str(), V3_FEE_GROWTH_GLOBAL1, 0, 63).await?;
    // TickInfo words: [2] feeGrowthOutside0, [3] feeGrowthOutside1 — BOTH bounds, BOTH legs.
    let tl_lower_tick_data = format!("{}{:064x}", V3_TICKS, decoded.tick_lower as u32);
    let tu_upper_tick_data = format!("{}{:064x}", V3_TICKS, decoded.tick_upper as u32);
    let fgo0_l = post_rpc_word(http_client, alchemy_url, pool.as_str(), &tl_lower_tick_data, 2, 64).await?;
    let fgo1_l = post_rpc_word(http_client, alchemy_url, pool.as_str(), &tl_lower_tick_data, 3, 66).await?;
    let fgo0_u = post_rpc_word(http_client, alchemy_url, pool.as_str(), &tu_upper_tick_data, 2, 67).await?;
    let fgo1_u = post_rpc_word(http_client, alchemy_url, pool.as_str(), &tu_upper_tick_data, 3, 68).await?;
    let inside0 = fg0_g.wrapping_sub(&fgo0_l).wrapping_sub(&fgo0_u);
    let inside1 = fg1_g.wrapping_sub(&fgo1_l).wrapping_sub(&fgo1_u);
    // Uncollected = (inside - insideLast) * L / 2^128 + tokensOwed (the settled part).
    let fee0_raw = inside0
        .wrapping_sub(&decoded.fee_growth_inside0_last)
        .mul_div_q128(decoded.liquidity)
        .saturating_add(decoded.tokens_owed0);
    let fee1_raw = inside1
        .wrapping_sub(&decoded.fee_growth_inside1_last)
        .mul_div_q128(decoded.liquidity)
        .saturating_add(decoded.tokens_owed1);

    let token0 = token_info(&decoded.token0)?;
    let token1 = token_info(&decoded.token1)?;
    let (amount0_raw, amount1_raw) =
        concentrated_amounts_raw(decoded.liquidity, sqrt_price_x96, decoded.tick_lower, decoded.tick_upper);
    let dec0 = 10f64.powi(token0.decimals);
    let dec1 = 10f64.powi(token1.decimals);

    tracing::info!(
        "v3 position {} (NFT #{}): fee={}bps, liquidity={}, tick={}, in_range={}, fees raw {}/{}",
        pos.id, pos.nft_id, decoded.fee, decoded.liquidity, tick,
        decoded.tick_lower <= tick && tick <= decoded.tick_upper, fee0_raw, fee1_raw
    );

    Ok(RawPosition {
        position_id: format!("uniswap-v3:{}:{}", pos.network, pos.nft_id),
        network: pos.network.clone(),
        protocol: "uniswap-v3".to_string(),
        wallet: wallet.to_string(),
        nft_id: pos.nft_id,
        pool_id: pool.clone(),
        npm_address: Some(npm.clone()),
        fee_tier_bps: (decoded.fee / 100) as u32,
        token0,
        token1,
        liquidity: decoded.liquidity,
        sqrt_price_x96,
        tick,
        tick_lower: decoded.tick_lower,
        tick_upper: decoded.tick_upper,
        amount0: amount0_raw / dec0,
        amount1: amount1_raw / dec1,
        fee0: fee0_raw as f64 / dec0,
        fee1: fee1_raw as f64 / dec1,
        in_range: decoded.tick_lower <= tick && tick <= decoded.tick_upper,
        is_price_source: pos.is_price_source,
    })
}

/// Single-word (or selected-word) eth_call helper for v3 pool reads.
async fn post_rpc_word(
    http_client: &Client,
    alchemy_url: &str,
    to: &str,
    data: &str,
    word_index: usize,
    id: u64,
) -> Result<U256> {
    let payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "to": to, "data": if data.starts_with("0x") { data.to_string() } else { format!("0x{}", data) } }, "latest"],
        "id": id
    });
    let res = post_rpc_with_retry(http_client, alchemy_url, &payload, 2)
        .await
        .ok_or_else(|| anyhow::anyhow!("eth_call to {} failed", to))?;
    let hex_str = res
        .get("result")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("eth_call to {} returned no result", to))?;
    let clean = hex_str.trim_start_matches("0x");
    let start = word_index * 64;
    if clean.len() < start + 64 {
        return Err(anyhow::anyhow!(
            "eth_call to {} returned short output ({} chars, need word {})",
            to,
            clean.len(),
            word_index
        ));
    }
    U256::from_hex_be(&clean[start..start + 64])
}

/// Decoded `positions(uint256)` return (12 words).
#[derive(Debug, Clone)]
struct V3PositionDecoded {
    token0: String,
    token1: String,
    /// Fee in Uniswap units (hundredths of a basis point; 10000 = 1%)
    fee: u64,
    tick_lower: i32,
    tick_upper: i32,
    liquidity: u128,
    fee_growth_inside0_last: U256,
    fee_growth_inside1_last: U256,
    tokens_owed0: u128,
    tokens_owed1: u128,
}

/// Parse the 384-byte `positions(uint256)` return:
/// (nonce, operator, token0, token1, fee, tickLower, tickUpper, liquidity,
///  feeGrowthInside0Last, feeGrowthInside1Last, tokensOwed0, tokensOwed1).
fn decode_v3_positions_return(hex_str: &str) -> Result<V3PositionDecoded> {
    let clean = hex_str.trim_start_matches("0x");
    if clean.len() < 12 * 64 {
        return Err(anyhow::anyhow!(
            "NPM positions() returned {} chars, expected >= 768",
            clean.len()
        ));
    }
    let word = |i: usize| -> &str { &clean[i * 64..(i + 1) * 64] };
    let addr = |i: usize| -> String { format!("0x{}", &word(i)[24..]) };
    let signed_tick = |i: usize| -> Result<i32> {
        let v = u32::from_str_radix(word(i), 16).map_err(|e| anyhow::anyhow!("bad tick word: {e}"))?;
        Ok(v as i32)
    };
    Ok(V3PositionDecoded {
        token0: addr(2),
        token1: addr(3),
        fee: u64::from_str_radix(word(4), 16).map_err(|e| anyhow::anyhow!("bad fee word: {e}"))?,
        tick_lower: signed_tick(5)?,
        tick_upper: signed_tick(6)?,
        liquidity: u128::from_str_radix(word(7), 16).map_err(|e| anyhow::anyhow!("bad liquidity word: {e}"))?,
        fee_growth_inside0_last: U256::from_hex_be(word(8))?,
        fee_growth_inside1_last: U256::from_hex_be(word(9))?,
        tokens_owed0: u128::from_str_radix(word(10), 16).unwrap_or(0),
        tokens_owed1: u128::from_str_radix(word(11), 16).unwrap_or(0),
    })
}

fn compute_structural_channels_sync(history: &[(DateTime<Utc>, f64)]) -> StructuralChannels {
    let mut channels = Vec::new();

    if history.len() >= 20 {
        let window = 20;
        let mut center_line = Vec::new();
        let mut upper_1sd = Vec::new();
        let mut lower_1sd = Vec::new();
        let mut upper_2sd = Vec::new();
        let mut lower_2sd = Vec::new();

        // `i` is the exclusive end of the window; iterating through len+1 means the final
        // window `[len-window..len]` includes the newest sample (off-by-one otherwise).
        for i in window..=history.len() {
            let window_slice = &history[i-window..i];
            let sum: f64 = window_slice.iter().map(|(_, v)| v).sum();
            let mean = sum / window as f64;
            let variance: f64 = window_slice.iter()
                .map(|(_, v)| (v - mean).powi(2))
                .sum::<f64>() / window as f64;
            let std_dev = variance.sqrt();

            // Label the point with the timestamp of the newest sample inside the window
            let ts = window_slice[window_slice.len() - 1].0;
            center_line.push(LinePoint { time: ts.to_rfc3339(), value: format!("{:.2}", mean) });
            upper_1sd.push(LinePoint { time: ts.to_rfc3339(), value: format!("{:.2}", mean + std_dev) });
            lower_1sd.push(LinePoint { time: ts.to_rfc3339(), value: format!("{:.2}", mean - std_dev) });
            upper_2sd.push(LinePoint { time: ts.to_rfc3339(), value: format!("{:.2}", mean + 2.0 * std_dev) });
            lower_2sd.push(LinePoint { time: ts.to_rfc3339(), value: format!("{:.2}", mean - 2.0 * std_dev) });
        }

        if !center_line.is_empty() {
            let last_idx = center_line.len() - 1;
            let break_point = if last_idx > 0 {
                Some(center_line[last_idx].time.clone())
            } else {
                None
            };

            channels.push(RegressionChannel {
                start_time: center_line.first().map(|p| p.time.clone()).unwrap_or_else(|| Utc::now().to_rfc3339()),
                end_time: None,
                center_line,
                upper_1sd,
                lower_1sd,
                upper_2sd,
                lower_2sd,
                is_active: true,
                break_point,
            });
        }
    }

    StructuralChannels { channels }
}

fn compute_counterfactual_sync(
    history: &[(DateTime<Utc>, f64, f64)],
    current_portfolio: f64,
    current_native: f64,
    now_str: &str,
) -> StrategyCounterfactual {
    let mut time_points = Vec::new();
    let mut active_series = Vec::new();
    let mut passive_series = Vec::new();

    for (ts, portfolio, native) in history {
        time_points.push(ts.to_rfc3339());
        active_series.push(format!("{:.2}", portfolio));
        passive_series.push(format!("{:.2}", native));
    }

    // Always include current point
    time_points.push(now_str.to_string());
    active_series.push(format!("{:.2}", current_portfolio));
    passive_series.push(format!("{:.2}", current_native));

    StrategyCounterfactual {
        time_points,
        active_portfolio_series: active_series,
        passive_baseline_series: passive_series,
    }
}

/// Claim recommendation computed from live claimable yield vs gas break-even.
/// The 2x factor mirrors the gas_to_fee margin used elsewhere; gas <= 0 means gas is
/// unknown (not free), so only a moderate recommendation is possible.
fn recommended_claim_action(claimable_usd: f64, gas_break_even_usd: f64) -> &'static str {
    if gas_break_even_usd > 0.0 && claimable_usd >= 2.0 * gas_break_even_usd {
        "claim_now"
    } else if claimable_usd > 0.0 {
        "claim_recommended"
    } else {
        "accumulate"
    }
}

fn compute_derived_metrics(raw: &RawOnChainData, prev_hwm: f64, profile: &OperatorProfile, structural_channels: StructuralChannels, counterfactual_history: &[(DateTime<Utc>, f64, f64)], ledger: &LedgerAggregate, regime: &MarketRegime) -> DerivedMetrics {
    // 1. Spot price: live NOCK/USD from the designated price-source position pool
    //    (zero-mock: the cycle fails upstream when the read fails).
    let spot_price = raw.spot_price_nock_usd;

    // 2. LP aggregates across all tracked positions (v2.0.0; replaces tranche 1/2 math).
    let mut lp_nock = 0.0f64;
    let mut lp_usdc = 0.0f64;
    let mut lp_usd = 0.0f64;
    let mut lp_fee_nock = 0.0f64;
    let mut lp_fee_usd = 0.0f64;
    // Per-position (address, label, fee_usd) tuples for the velocity panel.
    let mut fee_entries: Vec<(String, String, f64)> = Vec::new();
    for p in &raw.positions {
        let p0 = leg_usd_price(&p.token0, spot_price, raw.live_eth_price_usd);
        let p1 = leg_usd_price(&p.token1, spot_price, raw.live_eth_price_usd);
        lp_usd += p.amount0 * p0 + p.amount1 * p1;
        if is_usdc_token(&p.token0) { lp_usdc += p.amount0; }
        if is_usdc_token(&p.token1) { lp_usdc += p.amount1; }
        if is_nock_token(&p.token0) { lp_nock += p.amount0; }
        if is_nock_token(&p.token1) { lp_nock += p.amount1; }
        let fee_usd = p.fee0 * p0 + p.fee1 * p1;
        lp_fee_usd += fee_usd;
        if is_nock_token(&p.token0) { lp_fee_nock += p.fee0; }
        if is_nock_token(&p.token1) { lp_fee_nock += p.fee1; }
        fee_entries.push((
            if p.protocol == "uniswap-v3" {
                p.npm_address.clone().unwrap_or_else(|| p.pool_id.clone())
            } else {
                UNISWAP_V4_POSM_BASE.to_string()
            },
            format!(
                "{} ({}/{} {:.2}%, #{})",
                if p.protocol == "uniswap-v4" { "Uniswap V4" } else { "Uniswap V3" },
                p.token0.symbol,
                p.token1.symbol,
                p.fee_tier_bps as f64 / 100.0,
                p.nft_id
            ),
            fee_usd,
        ));
    }

    let total_fees_usd = lp_fee_usd;
    // Zero-mock invariant: no fabricated gas fallback. A zero value means the gas query
    // was unavailable (only reachable in unit tests via default RawOnChainData) — derived
    // efficiency/action values then treat gas as unknown, never as free.
    let gas_cost = raw.live_gas_cost_usd;

    // 3. Sovereign Treasury (wallet) and totals
    let wallet_nock = raw.wallet_balances.iter()
        .find(|w| w.token_symbol == "NOCK")
        .and_then(|w| w.amount.parse().ok())
        .unwrap_or(0.0);
    let t3_nock = wallet_nock;
    let t3_usd = t3_nock * spot_price;

    let wallet_usdc: f64 = raw.wallet_balances.iter()
        .find(|w| w.token_symbol == "USDC")
        .and_then(|w| w.amount.parse().ok())
        .unwrap_or(0.0);

    let total_portfolio_usd = lp_usd + t3_usd + wallet_usdc;
    let total_native_usd = (lp_nock + t3_nock) * spot_price;
    let total_stable_usd = lp_usdc + wallet_usdc;

    let floor = profile.treasury_floor_nock;
    let now_str = Utc::now().to_rfc3339();
    let total_nock = lp_nock + t3_nock + lp_fee_nock;
    let nock_floor_delta = total_nock - floor;
    let current_hwm = prev_hwm.max(total_portfolio_usd);

    // Range top of the price-source position (display units) — feeds the RANGE EXIT
    // branch; replaces the retired hardcoded V4 tick constants.
    let range_top_price = raw.positions.iter().find(|p| p.is_price_source).map(|p| {
        tick_to_display_price_dec(p.tick_lower, p.token0.decimals, p.token1.decimals)
    });

    // Tax-reserve guardrail (virtual 60/40 sub-ledger): dip deployments may only
    // consume growth dry powder; the tax reserve share of realized harvests is
    // protected. If the wallet's stable stack cannot cover the reserve, deployment
    // advice is suppressed entirely in favor of rebuilding the buffer.
    let tax_reserve_usd = ledger.total_usd * profile.tax_reserve_ratio;
    let deployable_usdc = (total_stable_usd - tax_reserve_usd).max(0.0);
    let tax_guardrail_blocked = tax_reserve_usd > 0.0 && total_stable_usd < tax_reserve_usd;

    // Dynamic Shot Advice Engine
    let (advice_action, advice_details) = if total_nock < floor {
        (
            "DEFENSIVE LOCK (Floor Recovery)".to_string(),
            format!(
                "CRITICAL: Total NOCK stack ({:.0}) is below the {:.0} floor by {:.0} NOCK! All yield and stable reserves are locked to replenish the sovereign foundation.",
                total_nock, floor, floor - total_nock
            ),
        )
    } else if regime.overextended {
        (
            "SWEEP TO STABLE / ACCUMULATE DRY POWDER (Overextension Cooldown)".to_string(),
            format!(
                "NOCK spot (${:.5}) is overextended (daily RSI(14) {:.0} >= 70). Buying tops is how gains evaporate: suppress deployment, harvest if fees warrant it, and sweep to the stable stack until momentum cools.",
                spot_price,
                regime.rsi_1d.unwrap_or(0.0)
            ),
        )
    } else if spot_price > 0.0 && spot_price < 0.020 && tax_guardrail_blocked {
        (
            "SWEEP TO STABLE / ACCUMULATE DRY POWDER (Tax Guardrail)".to_string(),
            format!(
                "Spot price (${:.5}) is at accumulation territory, but the virtual tax reserve (${:.2}) exceeds the stable stack (${:.2}). Deployment suppressed: harvest fees and rebuild the 60/40 buffer before buying dips.",
                spot_price, tax_reserve_usd, total_stable_usd
            ),
        )
    } else if spot_price > 0.0 && spot_price < 0.020 {
        (
            "DIP REINVESTMENT SHOT (Deploy USDC)".to_string(),
            format!(
                "Spot price (${:.5}) has breached the $0.020 accumulation boundary! Deploy ${:.2} of saved USDC (${:.2} available; ${:.2} protected as virtual tax reserve) to purchase NOCK at discount and compound buffer toward 120,000.",
                spot_price, deployable_usdc, total_stable_usd, tax_reserve_usd
            ),
        )
    } else if spot_price > 0.0 && range_top_price.map_or(false, |top| top > 0.0 && spot_price >= top) {
        (
            "RANGE EXIT FLASHPOINT (Build High Wall)".to_string(),
            format!(
                "Spot ${:.4} has reached the top of the price-source range (${:.4})! Take principal (${:.2}) and rebuild single-sided high walls at $5.00+.",
                spot_price,
                range_top_price.unwrap_or(0.0),
                lp_usd
            ),
        )
    } else if total_fees_usd >= 5.00 {
        let gas_efficiency = if gas_cost > 0.0 {
            format!("{:.0}x", total_fees_usd / gas_cost)
        } else {
            "unknown".to_string()
        };
        (
            "BATCH HARVEST SHOT (Execute Claim)".to_string(),
            format!(
                "Accumulated yield (${:.2}) exceeds $5.00 threshold at {} gas efficiency. Harvest fees and apply 60/40 Split Rule (40% to Tax Escrow, 60% Growth Capital).",
                total_fees_usd, gas_efficiency
            ),
        )
    } else {
        (
            "HOLD & ACCRETE (Dual Growth Engine Active)".to_string(),
            format!(
                "Total NOCK held is {:.0} (+{:.0} above {:.0} hard floor). {} Uniswap position(s) generating yield (+${:.2} uncollected). 0% realized loss. Maintain posture.",
                total_nock, nock_floor_delta, floor, raw.positions.len(), total_fees_usd
            ),
        )
    };

    DerivedMetrics {
        global_status: GlobalStatus {
            total_portfolio_usd: format!("{:.2}", total_portfolio_usd),
            total_native_usd: format!("{:.2}", total_native_usd),
            total_stable_usd: format!("{:.2}", total_stable_usd),
            total_fees_accrued_usd: format!("{:.2}", total_fees_usd),
            total_fees_claimable_usd: format!("{:.2}", total_fees_usd),
            gas_to_fee_delta_usd: format!("{:.2}", (total_fees_usd - (gas_cost * 2.0)).max(0.0)),
            high_water_mark_usd: format!("{:.2}", current_hwm),
            high_water_mark_date: now_str.clone(),
            total_nock_amount: format!("{:.0}", total_nock),
            nock_floor_delta: format!("{:.0}", nock_floor_delta),
            nock_floor_target: format!("{:.0}", floor),
            total_usdc_amount: format!("{:.2}", total_stable_usd),
            shot_advice_action: advice_action,
            shot_advice_details: advice_details,
            regime: if regime.overextended { "overextended".to_string() } else { "normal".to_string() },
            rsi_14: regime.rsi_5m.map(|r| format!("{:.1}", r)),
            rsi_14_1d: regime.rsi_1d.map(|r| format!("{:.1}", r)),
        },
        asset_aggregation_series: AssetAggregationSeries {
            time_points: vec![now_str.clone()],
            safe_haven_series: vec![format!("{:.2}", t3_usd)],
            active_protocol_series: vec![format!("{:.2}", lp_usd)],
            positions_nock: raw
                .positions
                .iter()
                .map(|p| {
                    let nock_amt = if is_nock_token(&p.token0) {
                        p.amount0
                    } else if is_nock_token(&p.token1) {
                        p.amount1
                    } else {
                        0.0
                    };
                    PositionNockSeries {
                        position_id: p.position_id.clone(),
                        series: vec![format!("{:.2}", nock_amt)],
                    }
                })
                .collect(),
        },
        total_value_vs_baseline: TotalValueVsBaseline {
            time_points: vec![now_str.clone()],
            portfolio_value_series: vec![format!("{:.2}", total_portfolio_usd)],
            token_price_series: vec![format!("{:.5}", spot_price)],
            high_water_mark_series: vec![format!("{:.2}", current_hwm)],
        },
        tick_bracket_hypervisor: TickBracketHypervisor {
            current_price: format!("{:.5}", spot_price),
            // Zero-mock: one bracket per tracked position, straight from its live ticks.
            brackets: raw
                .positions
                .iter()
                .map(|p| {
                    let p0 = leg_usd_price(&p.token0, spot_price, raw.live_eth_price_usd);
                    let p1 = leg_usd_price(&p.token1, spot_price, raw.live_eth_price_usd);
                    let pos_usd = p.amount0 * p0 + p.amount1 * p1;
                    // Display orientation: bigger tick -> lower price (token0-per-token1).
                    let lower_disp = tick_to_display_price_dec(p.tick_upper, p.token0.decimals, p.token1.decimals);
                    let upper_disp = tick_to_display_price_dec(p.tick_lower, p.token0.decimals, p.token1.decimals);
                    TickBracket {
                        lower_price: fmt_price(lower_disp),
                        upper_price: fmt_price(upper_disp),
                        liquidity_usd: format!("{:.2}", pos_usd),
                        in_range: p.in_range,
                        // Spot is USD/NOCK; brackets are token0-per-NOCK. Only USD-
                        // denominated token0 (USDC) yields an honest distance.
                        distance_bps: if p.token0.symbol == "USDC" {
                            Some(bracket_distance_bps(spot_price, lower_disp, upper_disp))
                        } else {
                            None
                        },
                    }
                })
                .collect(),
        },
        portfolio_deviation_matrix: PortfolioDeviationMatrix {
            allocations: {
                let calc_status = |deviation_bps: i32, target_weight_bps: i32| -> String {
                    let tol = (target_weight_bps as f64 * 0.20).max(100.0) as i32;
                    if deviation_bps > tol {
                        "over_funded".to_string()
                    } else if deviation_bps < -tol {
                        "under_funded".to_string()
                    } else {
                        "on_target".to_string()
                    }
                };

                let w = &profile.target_weights_bps;
                let t3_weight = ((t3_usd / total_portfolio_usd) * 10000.0) as i32;
                let t3_dev = t3_weight - w.treasury;

                let lp_weight = ((lp_usd / total_portfolio_usd) * 10000.0) as i32;
                let lp_dev = lp_weight - w.uniswap_lp;

                let usdc_weight = ((wallet_usdc / total_portfolio_usd) * 10000.0) as i32;
                let usdc_dev = usdc_weight - w.cash;

                vec![
                    AllocationDeviation {
                        bucket_name: "Sovereign Treasury".to_string(),
                        target_weight_bps: w.treasury,
                        actual_weight_bps: t3_weight,
                        deviation_bps: t3_dev,
                        status: calc_status(t3_dev, w.treasury),
                    },
                    AllocationDeviation {
                        bucket_name: "Uniswap LP (all tracked positions)".to_string(),
                        target_weight_bps: w.uniswap_lp,
                        actual_weight_bps: lp_weight,
                        deviation_bps: lp_dev,
                        status: calc_status(lp_dev, w.uniswap_lp),
                    },
                    AllocationDeviation {
                        bucket_name: "Liquid USDC (Escrow/Vault)".to_string(),
                        target_weight_bps: w.cash,
                        actual_weight_bps: usdc_weight,
                        deviation_bps: usdc_dev,
                        status: calc_status(usdc_dev, w.cash),
                    },
                ]
            },
        },
        structural_channels,
        strategy_counterfactual: compute_counterfactual_sync(
            counterfactual_history,
            total_portfolio_usd,
            total_native_usd,
            &now_str,
        ),
        sovereign_ledger: SovereignLedger {
            cumulative_harvested_usd: format!("{:.2}", ledger.total_usd),
            tax_reserve_usd: format!("{:.2}", ledger.total_usd * profile.tax_reserve_ratio),
            growth_dry_powder_usd: format!("{:.2}", ledger.total_usd * (1.0 - profile.tax_reserve_ratio)),
            harvest_count: ledger.harvest_count,
            last_harvest_at: ledger.last_at.map(|dt| dt.to_rfc3339()),
        },
        velocity_gas_sandbox: VelocityGasSandbox {
            contracts: fee_entries
                .iter()
                .map(|(addr, name, fee_usd)| FeeCollectionIndex {
                    contract_address: addr.clone(),
                    contract_name: name.clone(),
                    claimable_usd: format!("{:.2}", fee_usd),
                    gas_break_even_usd: format!("{:.4}", gas_cost),
                    efficiency_ratio: if gas_cost > 0.0 {
                        (fee_usd / gas_cost * 100.0).round() / 100.0
                    } else {
                        0.0
                    },
                    recommended_action: recommended_claim_action(*fee_usd, gas_cost).to_string(),
                })
                .collect(),
        },
    }
}

fn compute_state_hash(data: &SnapshotData) -> String {
    // Canonical JSON: sorted keys, no whitespace, wall-clock fields stripped
    let canonical = canonical_state_json(data);
    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_vec(&canonical).unwrap());
    format!("{:x}", hasher.finalize())
}

/// Canonical hash input: the snapshot payload with every wall-clock field removed.
///
/// Timestamps (`updated_at`, `high_water_mark_date`, series `time_points`, channel
/// `start_time`/`end_time`/`break_point`, `LinePoint.time`) are produced fresh on every
/// ingestion cycle and carry no state information. Including them made the hash change
/// every cycle, defeating snapshot deduplication (`store_snapshot` would insert a new
/// row every cycle even when on-chain state was unchanged, growing the table unboundedly
/// and filling history charts with duplicate points).
fn canonical_state_json(data: &SnapshotData) -> serde_json::Value {
    let mut value = serde_json::to_value(data).expect("SnapshotData is JSON-serializable");
    strip_volatile_time_fields(&mut value);
    value
}

fn strip_volatile_time_fields(value: &mut serde_json::Value) {
    const VOLATILE_KEYS: [&str; 7] = [
        "updated_at",
        "high_water_mark_date",
        "time_points",
        "time",
        "start_time",
        "end_time",
        "break_point",
    ];
    match value {
        serde_json::Value::Object(map) => {
            for key in VOLATILE_KEYS {
                map.remove(key);
            }
            for (_, v) in map.iter_mut() {
                strip_volatile_time_fields(v);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                strip_volatile_time_fields(item);
            }
        }
        _ => {}
    }
}

async fn store_snapshot(db: &PgPool, envelope: &SnapshotEnvelope) -> Result<()> {
    // Deduplicate: check if most recent snapshot has identical state hash
    let latest = sqlx::query!(
        "SELECT snapshot_id, state_hash FROM snapshots ORDER BY generated_at DESC LIMIT 1"
    )
    .fetch_optional(db)
    .await?;

    if let Some(ref r) = latest {
        if r.state_hash.trim() == envelope.meta.state_hash.trim() {
            tracing::info!(
                "State hash unchanged ({}), preserving historical snapshot timestamp",
                &r.state_hash[..16]
            );
            return Ok(());
        }
    }

    let data_json = serde_json::to_value(&envelope.data)?;
    sqlx::query!(
        r#"
        INSERT INTO snapshots (snapshot_id, generated_at, state_hash, version, data)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (snapshot_id) DO UPDATE SET
            generated_at = EXCLUDED.generated_at,
            state_hash = EXCLUDED.state_hash,
            version = EXCLUDED.version,
            data = EXCLUDED.data
        "#,
        envelope.meta.snapshot_id,
        envelope.meta.generated_at,
        envelope.meta.state_hash,
        envelope.meta.version,
        data_json
    )
    .execute(db)
    .await?;
    Ok(())
}

// ============================================================
// Unit Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Canonical Uniswap v3 NonfungiblePositionManager on Base — used by fixtures to
    // mirror the npm_address configured for the live v3 position (ONCHAIN_INTEGRATION.md §5).
    const UNISWAP_V3_NPM_BASE: &str = "0x03a520b32C04BF3bEEf7BEb72E919cf822Ed34f1";

    #[test]
    fn test_claim_fees_hex_parsing() {
        // Simulated return from 0xd294f093 (claimFees)
        // c0 = 256220 (0.25622 USDC with 6 decimals) -> 0x3e8dc
        // c1 = 86924344947911169 (8.69243 NOCK with 16 decimals) -> 0x134d13b2ef42a01
        let hex_str = "0x000000000000000000000000000000000000000000000000000000000003e8dc0000000000000000000000000000000000000000000000000134d13b2ef42a01";
        let clean = hex_str.trim_start_matches("0x");
        assert!(clean.len() >= 128);

        let c0 = u128::from_str_radix(&clean[..64], 16).unwrap();
        let c1 = u128::from_str_radix(&clean[64..128], 16).unwrap();

        let usdc = c0 as f64 / 1e6;
        let nock = c1 as f64 / 1e16;

        assert!((usdc - 0.25622).abs() < 1e-4);
        assert!((nock - 8.69243).abs() < 1e-4);
    }

    #[test]
    fn test_derived_metrics_defensive_lock_under_100k() {
        let mut raw = four_position_fixture();
        // 80,000 wallet NOCK + ~15,296 LP NOCK < 100,000 floor
        raw.wallet_balances[0].amount = "80000.0".to_string();
        raw.wallet_balances[0].amount_usd = "1664.0".to_string();
        let profile = OperatorProfile::default();
        let metrics = compute_derived_metrics(&raw, 2500.0, &profile, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        assert!(metrics.global_status.shot_advice_action.contains("DEFENSIVE LOCK"));
    }

    #[test]
    fn test_derived_metrics_batch_harvest_above_5_dollars() {
        let mut raw = four_position_fixture();
        // Uncollected USDC fees on the NOCK/USDC position push the aggregate over $5.
        raw.positions[1].fee0 = 6.0;
        raw.live_gas_cost_usd = 0.005;

        let profile = OperatorProfile::default();
        let metrics = compute_derived_metrics(&raw, 2500.0, &profile, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        assert!(metrics.global_status.shot_advice_action.contains("BATCH HARVEST SHOT"));
    }

    #[test]
    fn test_derived_metrics_hold_and_accrete() {
        let raw = four_position_fixture();
        let profile = OperatorProfile::default();
        let metrics = compute_derived_metrics(&raw, 2500.0, &profile, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        assert!(metrics.global_status.shot_advice_action.contains("HOLD & ACCRETE"));
        // Advice text is Uniswap-only now: no Aerodrome wording survives.
        assert!(!metrics.global_status.shot_advice_details.contains("Aerodrome"));
    }

    #[test]
    fn test_derived_metrics_four_position_fixture_matches_screenshot() {
        // Full aggregation over the discovery fixture: totals, brackets, series and
        // the deviation buckets must all reflect N positions, not the old tranches.
        let raw = four_position_fixture();
        let profile = OperatorProfile::default();
        let metrics = compute_derived_metrics(&raw, 2500.0, &profile, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());

        // NOCK spot from the price-source pool matches the screenshot-era value.
        assert!((raw.spot_price_nock_usd - 0.0208).abs() < 0.0002);

        // Position values (USD): B ≈ 71, C ≈ 18, A ≈ 156 (post-compounding),
        // D ≈ 207 (wide raft, largest holding after the operator's post-mint adds).
        let total_lp_usd = metrics.asset_aggregation_series.active_protocol_series[0].parse::<f64>().unwrap();
        assert!(total_lp_usd > 430.0 && total_lp_usd < 470.0, "lp total {}", total_lp_usd);

        // One bracket per position, all in range.
        assert_eq!(metrics.tick_bracket_hypervisor.brackets.len(), 4);
        assert!(metrics.tick_bracket_hypervisor.brackets.iter().all(|b| b.in_range));

        // Per-position NOCK series entries carry the position ids.
        assert_eq!(metrics.asset_aggregation_series.positions_nock.len(), 4);
        assert!(metrics.asset_aggregation_series.positions_nock.iter().any(|s| s.position_id == "uniswap-v3:base:6336662"));
        assert!(metrics.asset_aggregation_series.positions_nock.iter().any(|s| s.position_id == "uniswap-v4:base:3163450"));

        // Deviation matrix: Treasury / Uniswap LP / Cash only (no Aerodrome bucket).
        let names: Vec<&str> = metrics.portfolio_deviation_matrix.allocations.iter().map(|a| a.bucket_name.as_str()).collect();
        assert!(names.iter().any(|n| n.contains("Uniswap LP")));
        assert!(!names.iter().any(|n| n.contains("Aerodrome")));

        // v3 unclaimed fees surface in contract_fees with the NPM address.
        let v3_fee = metrics.velocity_gas_sandbox.contracts.iter().find(|c| c.contract_name.contains("Uniswap V3"));
        assert!(v3_fee.is_some());
    }

    #[test]
    fn test_operator_profile_custom_treasury_floor_and_weights() {
        // Legacy config keys (v4_baseline_*, aerodrome_lp, uniswap_v4_lp) still parse
        // but are ignored; the LP target now comes from `uniswap_lp`.
        let yaml = r#"
database_url: "postgresql://localhost/test"
operator_profile:
  treasury_floor_nock: 150000.0
  v4_baseline_liquidity: 65719361234409.0
  v4_baseline_nock: 4004.31
  v4_baseline_usdc: 23.63
  v4_fee_baseline_usdc: 0.05
  v4_fee_baseline_nock: 2.5
  tax_reserve_ratio: 0.33
  target_weights_bps:
    treasury: 7000
    aerodrome_lp: 1500
    uniswap_v4_lp: 1000
    cash: 500
"#;
        let cfg: AppConfig = serde_yaml::from_str(yaml).expect("Failed to deserialize yaml");
        let profile = cfg.profile();
        assert_eq!(profile.treasury_floor_nock, 150000.0);
        assert_eq!(profile.tax_reserve_ratio, 0.33);
        assert_eq!(profile.target_weights_bps.treasury, 7000);
        // aerodrome_lp/uniswap_v4_lp are ignored; uniswap_lp falls back to its default.
        assert_eq!(profile.target_weights_bps.uniswap_lp, 1500);
        assert_eq!(profile.target_weights_bps.cash, 500);

        let mut raw = RawOnChainData::default();
        raw.wallet_balances = vec![
            WalletBalance {
                wallet_address: "0x123".to_string(),
                token_symbol: "NOCK".to_string(),
                token_address: "0xNOCK".to_string(),
                amount: "120000.0".to_string(),
                amount_usd: "3120.0".to_string(),
                updated_at: "now".to_string(),
            },
        ];
        // 120,000 is above default 100k floor, BUT below custom 150k floor!
        let metrics = compute_derived_metrics(&raw, 3000.0, &profile, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        assert_eq!(metrics.global_status.nock_floor_target, "150000");
        assert_eq!(metrics.global_status.nock_floor_delta, "-30000");
        assert!(metrics.global_status.shot_advice_action.contains("DEFENSIVE LOCK"));
        assert_eq!(metrics.portfolio_deviation_matrix.allocations[0].target_weight_bps, 7000);
        assert_eq!(metrics.portfolio_deviation_matrix.allocations[1].target_weight_bps, 1500);
        assert_eq!(metrics.portfolio_deviation_matrix.allocations[2].target_weight_bps, 500);
    }

    #[test]
    fn test_operator_profile_default_tax_reserve_ratio() {
        // Backward compatibility: configs without the field still parse, default 60/40 split.
        let yaml = r#"
database_url: "postgresql://localhost/test"
"#;
        let cfg: AppConfig = serde_yaml::from_str(yaml).expect("Failed to deserialize yaml");
        assert_eq!(cfg.profile().tax_reserve_ratio, 0.40);
    }

    #[test]
    fn test_sovereign_ledger_60_40_split() {
        let raw = RawOnChainData {
            wallet_balances: vec![WalletBalance {
                wallet_address: "0x123".to_string(),
                token_symbol: "NOCK".to_string(),
                token_address: "0xNOCK".to_string(),
                amount: "110000.0".to_string(),
                amount_usd: "2860.0".to_string(),
                updated_at: "now".to_string(),
            }],
            ..Default::default()
        };
        let mut profile = OperatorProfile::default();
        profile.tax_reserve_ratio = 0.40;
        let ledger = LedgerAggregate {
            total_usd: 100.0,
            harvest_count: 2,
            last_at: Some(Utc::now()),
        };
        let metrics = compute_derived_metrics(
            &raw, 0.0, &profile,
            StructuralChannels { channels: vec![] },
            &[], &ledger, &MarketRegime::default(),
        );
        assert_eq!(metrics.sovereign_ledger.cumulative_harvested_usd, "100.00");
        assert_eq!(metrics.sovereign_ledger.tax_reserve_usd, "40.00");
        assert_eq!(metrics.sovereign_ledger.growth_dry_powder_usd, "60.00");
        assert_eq!(metrics.sovereign_ledger.harvest_count, 2);
        assert!(metrics.sovereign_ledger.last_harvest_at.is_some());

        // Custom ratio honored
        profile.tax_reserve_ratio = 0.25;
        let metrics = compute_derived_metrics(
            &raw, 0.0, &profile,
            StructuralChannels { channels: vec![] },
            &[], &ledger, &MarketRegime::default(),
        );
        assert_eq!(metrics.sovereign_ledger.tax_reserve_usd, "25.00");
        assert_eq!(metrics.sovereign_ledger.growth_dry_powder_usd, "75.00");
    }

    #[test]
    fn test_koinly_csv_format() {
        let rows = vec![
            HarvestExportRow {
                platform: "aerodrome".to_string(),
                tx_hash: "0xabc123".to_string(),
                token_symbol: "USDC".to_string(),
                amount_human: "0.119983".to_string(),
                usd_value: Some("0.119983".to_string()),
                confirmed_at: "2026-10-12T18:31:21+00:00".to_string(),
            },
            HarvestExportRow {
                platform: "uniswap-v4".to_string(),
                tx_hash: "0xdef456".to_string(),
                token_symbol: "NOCK".to_string(),
                amount_human: "5.4468217823004916".to_string(),
                usd_value: Some("0.2612".to_string()),
                confirmed_at: "2026-10-12T18:35:00+00:00".to_string(),
            },
        ];
        let csv = format_koinly_csv(&rows);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("Date, Sent Amount, Sent Currency, Received Amount"));
        assert!(lines[0].ends_with("Label, Description, TxHash"));
        assert!(lines[1].contains(", , , 0.119983, USDC, , , 0.119983, USD, income,"));
        assert!(lines[1].contains("\"NockNock fee harvest (aerodrome)\""));
        assert!(lines[1].ends_with("0xabc123"));
        assert!(lines[2].contains("5.4468217823004916, NOCK"));
        assert!(lines[2].contains("income"));
        assert!(lines[2].ends_with("0xdef456"));
    }

    #[test]
    fn test_tax_tie_out_split_equals_export_total() {
        // Koinly tie-out: the CSV rows must carry exactly the ledger amounts that the
        // 60/40 sub-ledger split was computed from — every harvest dollar accounted
        // for, nothing invented.
        let ledger_rows = vec![
            ("0xabc123", "USDC", "0.119983", 119983u128, 6i32),
            ("0xabc123", "NOCK", "5.4468217823004916", 54468217823004916, 16),
        ];
        let total_usd: f64 = 0.119983 + 5.4468217823004916 * 0.048; // NOCK spot example
        let profile_tax_ratio = 0.40f64;
        let tax_reserve = total_usd * profile_tax_ratio;
        let dry_powder = total_usd * (1.0 - profile_tax_ratio);

        // Ledger aggregate must equal the sum of the exported receipt-verified rows
        let exported_total: f64 = ledger_rows
            .iter()
            .map(|(_, sym, human, raw, dec)| {
                let units = format_units(*raw, *dec);
                assert_eq!(&units, human);
                match *sym {
                    "USDC" => units.parse::<f64>().unwrap(),
                    "NOCK" => units.parse::<f64>().unwrap() * 0.048,
                    _ => 0.0,
                }
            })
            .sum();
        assert!((exported_total - total_usd).abs() < 1e-9);
        assert!((tax_reserve + dry_powder - total_usd).abs() < 1e-9);
    }

    #[test]
    fn test_tax_guardrail_blocks_dip_deployment() {
        // Overdrawn tax reserve: the wallet's stable stack cannot cover the virtual
        // 40% tax reserve, so dip-deployment advice must be suppressed entirely.
        let mut raw = four_position_fixture();
        raw.spot_price_nock_usd = 0.015; // dip zone (< 0.020)
        raw.wallet_balances[1].amount = "10.0".to_string(); // LP USDC legs (B+D ≈ 140) + wallet 10
        let profile = OperatorProfile::default(); // floor 100k; ratio 0.40
        let ledger = LedgerAggregate { total_usd: 500.0, harvest_count: 3, last_at: None };
        // tax reserve = 500 * 0.40 = 200 > stable stack (~150) -> blocked
        let metrics = compute_derived_metrics(
            &raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[], &ledger, &MarketRegime::default(),
        );
        assert!(metrics.global_status.shot_advice_action.contains("SWEEP TO STABLE"));
        assert!(metrics.global_status.shot_advice_action.contains("Tax Guardrail"));
        assert!(metrics.global_status.shot_advice_details.contains("Deployment suppressed"));

        // Small harvest ledger: reserve 40 * 0.40 = 16 <= stable stack -> DIP advice fires
        let small = LedgerAggregate { total_usd: 40.0, harvest_count: 1, last_at: None };
        let metrics = compute_derived_metrics(
            &raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[], &small, &MarketRegime::default(),
        );
        assert!(metrics.global_status.shot_advice_action.contains("DIP REINVESTMENT SHOT"));
        assert!(metrics.global_status.shot_advice_details.contains("protected as virtual tax reserve"));
    }

    #[test]
    fn test_rsi_series_known_values() {
        // Monotonic gains -> RSI pinned near 100 (overbought)
        let closes: Vec<f64> = (1..=20).map(|i| 100.0 + i as f64).collect();
        let rsi = compute_rsi_series(&closes, 14);
        assert!(rsi[..14].iter().all(|x| x.is_none()));
        assert!((rsi[14]).is_some());
        let last = rsi[19].unwrap();
        assert!(last > 95.0, "all-gain series should push RSI near 100, got {last}");
        // Monotonic losses -> RSI near 0 (oversold)
        let closes: Vec<f64> = (0..20).map(|i| 100.0 - i as f64).collect();
        let rsi = compute_rsi_series(&closes, 14);
        let last = rsi[19].unwrap();
        assert!(last < 5.0, "all-loss series should push RSI near 0, got {last}");
        // Too few samples -> all None
        assert!(compute_rsi_series(&closes[..5], 14).iter().all(|x| x.is_none()));
    }

    #[test]
    fn test_directive_transition_notification() {
        // First boot (no previous directive): never notify
        assert!(directive_transition_notification(None, "A", "d", None).is_none());
        // Unchanged directive: never notify
        assert!(directive_transition_notification(Some("A"), "A", "d", None).is_none());
        // Changed: notify with action + details
        let text = directive_transition_notification(
            Some("HOLD & ACCRETE"),
            "BATCH HARVEST SHOT",
            "Accumulated yield ($5.20) exceeds threshold",
            Some("https://cockpit.example/lite"),
        )
        .unwrap();
        assert!(text.contains("directive change: BATCH HARVEST SHOT"));
        assert!(text.contains("Accumulated yield"));
        assert!(text.contains("https://cockpit.example/lite"));
        // Changed without cockpit URL: no trailing link
        let text = directive_transition_notification(Some("A"), "B", "d", None).unwrap();
        assert!(!text.contains("→"));
    }

    #[test]
    fn test_state_hash_determinism() {
        let data = sample_snapshot_data();

        let hash1 = compute_state_hash(&data);
        let hash2 = compute_state_hash(&data);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn test_state_hash_ignores_volatile_timestamps() {
        // Two snapshots whose ONLY differences are wall-clock fields (produced fresh each
        // cycle) must hash identically — otherwise snapshot deduplication never fires.
        let mut a = sample_snapshot_data();
        let b = sample_snapshot_data();

        a.wallet_balances[0].updated_at = "2027-01-01T00:00:00Z".to_string();
        a.derived_metrics.global_status.high_water_mark_date = "2027-01-01T00:00:00Z".to_string();
        a.derived_metrics.asset_aggregation_series.time_points = vec!["2027-01-01T00:00:00Z".to_string()];
        a.derived_metrics.total_value_vs_baseline.time_points = vec!["2027-01-01T00:00:00Z".to_string()];
        a.derived_metrics.strategy_counterfactual.time_points = vec!["2027-01-01T00:00:00Z".to_string(); 4];
        a.derived_metrics.structural_channels.channels[0].start_time = "2027-01-01T00:00:00Z".to_string();
        a.derived_metrics.structural_channels.channels[0].break_point = Some("2027-01-01T00:00:00Z".to_string());
        for p in a.derived_metrics.structural_channels.channels[0].center_line.iter_mut() {
            p.time = "2027-01-01T00:00:00Z".to_string();
        }

        assert_eq!(compute_state_hash(&a), compute_state_hash(&b));
    }

    #[test]
    fn test_state_hash_changes_with_state() {
        // Real state changes must change the hash so dedup keeps genuinely new snapshots.
        let mut a = sample_snapshot_data();
        let b = sample_snapshot_data();
        a.wallet_balances[0].amount = "96004.52".to_string();
        a.derived_metrics.global_status.total_nock_amount = "106141".to_string();

        assert_ne!(compute_state_hash(&a), compute_state_hash(&b));
    }

    #[test]
    fn test_counterfactual_series_use_real_timestamps_and_stay_aligned() {
        let history = vec![
            (Utc::now() - chrono::Duration::minutes(30), 2900.0f64, 2600.0f64),
            (Utc::now() - chrono::Duration::minutes(25), 2910.0, 2602.0),
            (Utc::now() - chrono::Duration::minutes(20), 2925.0, 2603.5),
        ];
        let now_str = Utc::now().to_rfc3339();
        let cf = compute_counterfactual_sync(&history, 2950.0, 2605.0, &now_str);

        assert_eq!(cf.time_points.len(), 4);
        assert_eq!(cf.active_portfolio_series.len(), 4);
        assert_eq!(cf.passive_baseline_series.len(), 4);

        // Real snapshot timestamps, in chronological order — no fabricated spacing
        assert_eq!(cf.time_points[0], history[0].0.to_rfc3339());
        assert_eq!(cf.time_points[2], history[2].0.to_rfc3339());
        assert_ne!(cf.time_points[0], cf.time_points[1]);

        // Paired series values align with the same history rows
        assert_eq!(cf.active_portfolio_series[1], "2910.00");
        assert_eq!(cf.passive_baseline_series[1], "2602.00");

        // Current point appended last
        assert_eq!(cf.time_points[3], now_str);
        assert_eq!(cf.active_portfolio_series[3], "2950.00");
        assert_eq!(cf.passive_baseline_series[3], "2605.00");
    }

    #[test]
    fn test_structural_channels_final_window_includes_newest_sample() {
        // 21 samples: the final 20-sample window must include the newest price.
        let base_ts = Utc::now() - chrono::Duration::minutes(60);
        let mut history: Vec<(DateTime<Utc>, f64)> = (0..20)
            .map(|i| (base_ts + chrono::Duration::minutes(i as i64), 100.0))
            .collect();
        let newest_ts = base_ts + chrono::Duration::minutes(20);
        history.push((newest_ts, 200.0)); // outlier newest sample

        let channels = compute_structural_channels_sync(&history);
        let ch = &channels.channels[0];
        let last_center: f64 = ch.center_line.last().unwrap().value.parse().unwrap();

        // Window of the last 20 samples = nineteen 100s + one 200 → mean 105.00
        assert!((last_center - 105.0).abs() < 0.01);
        // Point is labeled with the timestamp of the newest sample in the window
        assert_eq!(ch.center_line.last().unwrap().time, newest_ts.to_rfc3339());
    }

    #[test]
    fn test_tick_to_display_price_matches_live_values() {
        // Derived bracket prices must round-trip against the previously hardcoded (and
        // live-verified) display values for the NOCK/USDC V4 ticks (USDC per NOCK).
        let lower_display = tick_to_display_price_dec(274200, 6, 16);
        let upper_display = tick_to_display_price_dec(223200, 6, 16);

        assert!((lower_display - 0.01237).abs() / 0.01237 < 0.001);
        assert!((upper_display - 2.02783).abs() / 2.02783 < 0.001);
        assert!(lower_display < upper_display);

        // The negated-exponent orientation: larger on-chain tick → lower displayed price
        assert!(tick_to_display_price_dec(274200, 6, 16) < tick_to_display_price_dec(223200, 6, 16));

        // Discovery cross-check: the ETH(18)/NOCK(16) pool's screenshot range
        // (0.08091 → 0.1474 in the operator's UI scale) corresponds to ticks
        // [65200, 71200] — spot 70370 sits between the decoded bounds.
        let lower_eth = tick_to_display_price_dec(71200, 18, 16);
        let upper_eth = tick_to_display_price_dec(65200, 18, 16);
        assert!(lower_eth < upper_eth);
        assert!(upper_eth / lower_eth > 1.5);
    }

    #[test]
    fn test_bracket_distance_bps() {
        // Spot inside the range: distance is to the nearest edge, sign says which side
        let lower = tick_to_display_price_dec(274200, 6, 16); // ~0.012366
        let upper = tick_to_display_price_dec(223200, 6, 16); // ~2.027880
        let d = bracket_distance_bps(1.0, lower, upper);
        // Nearest edge is the lower (~0.0124, 9876 bps away vs 10279 to the upper):
        // (0.012366 - 1.0)/1.0 * 10000 ≈ -9876
        assert!((d as f64 + 9876.0).abs() < 5.0);

        // Spot above range → distance to upper edge is negative
        assert!(bracket_distance_bps(2.5, lower, upper) < 0);
        // Degenerate inputs stay zero instead of panicking on division
        assert_eq!(bracket_distance_bps(0.0, lower, upper), 0);
    }

    #[test]
    fn test_recommended_claim_action_thresholds() {
        // Claimable well above 2x gas → claim now
        assert_eq!(recommended_claim_action(0.12, 0.005), "claim_now");
        // Claimable positive but under the 2x gas margin → moderate recommendation
        assert_eq!(recommended_claim_action(0.008, 0.005), "claim_recommended");
        // Nothing claimable → accumulate
        assert_eq!(recommended_claim_action(0.0, 0.005), "accumulate");
        // Gas unknown (0) is NOT free gas — never claim_now
        assert_eq!(recommended_claim_action(5.0, 0.0), "claim_recommended");
        assert_eq!(recommended_claim_action(0.0, 0.0), "accumulate");
    }

    fn sample_snapshot_data() -> SnapshotData {
        SnapshotData {
            wallet_balances: vec![WalletBalance {
                wallet_address: "0x123".to_string(),
                token_symbol: "NOCK".to_string(),
                token_address: "0xNOCK".to_string(),
                amount: "96004.52".to_string(),
                amount_usd: "2496.12".to_string(),
                updated_at: "2026-10-11T00:00:00Z".to_string(),
            }],
            pool_mechanics: vec![],
            positions: vec![],
            concentrated_liquidity: vec![],
            contract_fees: vec![],
            derived_metrics: DerivedMetrics {
                global_status: GlobalStatus {
                    total_portfolio_usd: "100.00".to_string(),
                    total_native_usd: "50.00".to_string(),
                    total_stable_usd: "50.00".to_string(),
                    total_fees_accrued_usd: "1.00".to_string(),
                    total_fees_claimable_usd: "1.00".to_string(),
                    gas_to_fee_delta_usd: "0.99".to_string(),
                    high_water_mark_usd: "100.00".to_string(),
                    high_water_mark_date: "2026-10-11T00:00:00Z".to_string(),
                    total_nock_amount: "105000".to_string(),
                    nock_floor_delta: "5000".to_string(),
                    nock_floor_target: "100000".to_string(),
                    total_usdc_amount: "50.00".to_string(),
                    shot_advice_action: "HOLD".to_string(),
                    shot_advice_details: "Steady".to_string(),
                    regime: "normal".to_string(),
                    rsi_14: None,
                    rsi_14_1d: None,
                },
                sovereign_ledger: Default::default(),
                asset_aggregation_series: AssetAggregationSeries {
                    time_points: vec!["2026-10-11T00:00:00Z".to_string()],
                    safe_haven_series: vec!["48.00".to_string()],
                    active_protocol_series: vec!["2.00".to_string()],
                    positions_nock: vec![],
                },
                tick_bracket_hypervisor: TickBracketHypervisor {
                    brackets: vec![],
                    current_price: "0.026".to_string(),
                },
                velocity_gas_sandbox: VelocityGasSandbox {
                    contracts: vec![],
                },
                portfolio_deviation_matrix: PortfolioDeviationMatrix {
                    allocations: vec![],
                },
                strategy_counterfactual: StrategyCounterfactual {
                    time_points: vec![
                        "2026-10-11T00:00:00Z".to_string(),
                        "2026-10-11T00:05:00Z".to_string(),
                        "2026-10-11T00:10:00Z".to_string(),
                    ],
                    active_portfolio_series: vec!["99.00".to_string(), "99.50".to_string(), "100.00".to_string()],
                    passive_baseline_series: vec!["49.00".to_string(), "49.50".to_string(), "50.00".to_string()],
                },
                structural_channels: StructuralChannels {
                    channels: vec![RegressionChannel {
                        start_time: "2026-10-11T00:00:00Z".to_string(),
                        end_time: None,
                        center_line: vec![
                            LinePoint { time: "2026-10-11T00:00:00Z".to_string(), value: "99.00".to_string() },
                            LinePoint { time: "2026-10-11T00:05:00Z".to_string(), value: "100.00".to_string() },
                        ],
                        upper_1sd: vec![],
                        lower_1sd: vec![],
                        upper_2sd: vec![],
                        lower_2sd: vec![],
                        is_active: true,
                        break_point: None,
                    }],
                },
                total_value_vs_baseline: TotalValueVsBaseline {
                    time_points: vec!["2026-10-11T00:00:00Z".to_string()],
                    portfolio_value_series: vec!["100.00".to_string()],
                    high_water_mark_series: vec!["100.00".to_string()],
                    token_price_series: vec!["0.02600".to_string()],
                },
            },
        }
    }

    #[test]
    fn test_eth_usd_spot_parsing() {
        // Sample return from Aerodrome vAMM-WETH/USDC getReserves (0x0902f1ac)
        // r0 = 0x997534da7f7a6b1a4 = 176929380488719577508 (176.929 WETH with 18 decimals)
        // r1 = 0x689640625e = 449198383710 (449,198.38 USDC with 6 decimals)
        let hex_str = "0x00000000000000000000000000000000000000000000000997534da7f7a6b1a4000000000000000000000000000000000000000000000000000000689640625e";
        let clean = hex_str.trim_start_matches("0x");
        assert!(clean.len() >= 128);

        let r0 = u128::from_str_radix(&clean[..64], 16).unwrap();
        let r1 = u128::from_str_radix(&clean[64..128], 16).unwrap();

        let weth = r0 as f64 / 1e18;
        let usdc = r1 as f64 / 1e6;
        assert!(weth > 0.0);
        let eth_price = usdc / weth;

        // Verify ETH price falls within expected ~$2538 range
        assert!((eth_price - 2538.86).abs() < 1.0);
    }

    #[test]
    fn test_u256_wrapping_sub_and_mul_div() {
        // Live on-chain feeGrowthInside values for Uniswap V4 NOCK/USDC pool #3126000
        let fg0_curr = U256::from_hex_be("00000000000000000000002af933ab46390e82c57ffd6903").unwrap();
        let fg0_last = U256::from_hex_be("000000000000000000000024a1cdc1fa3d9c5ed6c43492cf").unwrap();
        let fg1_curr = U256::from_hex_be("000000000000087947b99b01f17bd63c97cbe99129a02a0e").unwrap();
        let fg1_last = U256::from_hex_be("00000000000005db4b66cee311145e59a7723d5ce16bb18f").unwrap();
        let liquidity = 81297590130616u128;

        let delta0 = fg0_curr.wrapping_sub(&fg0_last);
        let delta1 = fg1_curr.wrapping_sub(&fg1_last);

        let fee0_raw = delta0.mul_div_q128(liquidity);
        let fee1_raw = delta1.mul_div_q128(liquidity);

        // Verify exact fee token amounts match on-chain Uniswap mobile UI
        assert_eq!(fee0_raw, 120033); // 0.120033 USDC (6 decimals)
        assert_eq!(fee1_raw, 54468217823004916); // 5.446822 NOCK (16 decimals)

        let usdc = fee0_raw as f64 / 1e6;
        let nock = fee1_raw as f64 / 1e16;
        assert!((usdc - 0.120033).abs() < 1e-6);
        assert!((nock - 5.446822).abs() < 1e-6);
    }

    #[test]
    fn test_v4_collect_unlock_data_golden_vector() {
        // Golden vector: byte-for-byte identical to @uniswap/v4-sdk@2.4.1
        // V4PositionPlanner.finalize() for addDecrease(3126000, 0, 0, 0, no hookData)
        // + addTakePair(USDC, NOCK, sovereign wallet). Accepted by live Base mainnet
        // eth_call simulation against the deployed PositionManager on 2026-10-12
        // (audit trail: helix-specs task 000661 verification/v4collect_sim.py).
        let unlock = build_v4_collect_unlock_data(
            3126000,
            "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
            "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722",
            "0xeE114cC97eC5adeB34bA5645e315B497D8487a45",
        );
        assert_eq!(unlock, "0x000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000002011100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000012000000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000002fb2f000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000060000000000000000000000000833589fcd6edb6e08f4c7c32d4f71b54bda029130000000000000000000000009b5e262cf9bb04869ab40b19af91d2dc85761722000000000000000000000000ee114cc97ec5adeb34ba5645e315b497d8487a45");
    }

    #[test]
    fn test_v4_collect_calldata_golden_vector() {
        // Full outer calldata accepted by the live eth_call simulation (selector
        // modifyLiquidities(bytes,uint256) = 0xdd46508f; offset 0x40; deadline
        // 0xFFFFFFFFFFFF; bytes length word; unlockData). Reference bytes:
        // helix-specs task 000661 verification/verified_calldata.txt.
        let data = build_v4_collect_calldata(
            3126000,
            "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
            "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722",
            "0xeE114cC97eC5adeB34bA5645e315B497D8487a45",
            0xFFFFFFFFFFFF,
        );
        assert_eq!(data, "0xdd46508f00000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000ffffffffffff0000000000000000000000000000000000000000000000000000000000000240000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000002011100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000012000000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000002fb2f000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000060000000000000000000000000833589fcd6edb6e08f4c7c32d4f71b54bda029130000000000000000000000009b5e262cf9bb04869ab40b19af91d2dc85761722000000000000000000000000ee114cc97ec5adeb34ba5645e315b497d8487a45");
    }

    #[test]
    fn test_v3_collect_calldata_golden_vector() {
        // collect((uint256,address,uint128,uint128)) on the canonical Base NPM
        // 0x03a520b3…Ed34f1 for tokenId 6336662 (WETH/NOCK 1%). Live-verified via
        // eth_call 2026-10-24: returns (0, 1654339369283004) = 0.165434 NOCK —
        // exactly the snapshot's tokens_owed_1 for uniswap-v3:base:6336662.
        let data = build_v3_collect_calldata(
            6336662,
            "0xeE114cC97eC5adeB34bA5645e315B497D8487a45",
        );
        assert_eq!(data, "0xfc6f7865000000000000000000000000000000000000000000000000000000000060b096000000000000000000000000ee114cc97ec5adeb34ba5645e315b497d8487a4500000000000000000000000000000000ffffffffffffffffffffffffffffffff00000000000000000000000000000000ffffffffffffffffffffffffffffffff");
    }

    #[test]
    fn test_v4_collect_unlock_data_decrease_only_golden_vector() {
        // Single-action variant (DECREASE only, actions byte 0x01), also live-verified.
        let unlock = build_v4_collect_unlock_data_decrease_only(3126000);
        assert_eq!(unlock, "0x00000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000000101000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000002fb2f000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000000");
    }

    #[test]
    fn test_format_units_exact_decimal() {
        assert_eq!(format_units(120033, 6), "0.120033");
        assert_eq!(format_units(54468217823004916, 16), "5.4468217823004916");
        assert_eq!(format_units(1, 6), "0.000001");
        assert_eq!(format_units(1000000, 6), "1");
        assert_eq!(format_units(0, 16), "0");
        assert_eq!(format_units(4000000, 6), "4");
    }

    #[test]
    fn test_is_valid_tx_hash() {
        assert!(is_valid_tx_hash(
            "0xdd46508fdd46508fdd46508fdd46508fdd46508fdd46508fdd46508fdd46508f"
        ));
        assert!(is_valid_tx_hash(
            "dd46508fdd46508fdd46508fdd46508fdd46508fdd46508fdd46508fdd46508f"
        ));
        assert!(!is_valid_tx_hash("0x1234"));
        assert!(!is_valid_tx_hash(""));
        assert!(!is_valid_tx_hash(
            "zz46508fdd46508fdd46508fdd46508fdd46508fdd46508fdd46508fdd46508f"
        ));
    }

    #[test]
    fn test_parse_harvest_transfers_receipt_parsing() {
        let wallet = "0xeE114cC97eC5adeB34bA5645e315B497D8487a45";
        let from = "000000000000000000000000360e68faccca8ca495c1b759fd9eee466db9fb32";
        let wallet_word = format!("000000000000000000000000{}", wallet.trim_start_matches("0x").to_lowercase());
        let other_word = "0000000000000000000000001111111111111111111111111111111111111111";
        let receipt = json!({
            "status": "0x1",
            "blockNumber": "0x1234",
            "logs": [
                // USDC Transfer -> wallet (keep)
                { "address": USDC_BASE, "topics": [
                    TRANSFER_EVENT_TOPIC, from, wallet_word ], "data": "0x000000000000000000000000000000000000000000000000000000000001d4af" },
                // NOCK Transfer -> wallet (keep)
                { "address": NOCK_BASE, "topics": [
                    TRANSFER_EVENT_TOPIC, from, wallet_word ], "data": "0x0000000000000000000000000000000000000000000000000c1828e36e388f4" },
                // USDC Transfer -> someone else (skip)
                { "address": USDC_BASE, "topics": [
                    TRANSFER_EVENT_TOPIC, from, other_word ], "data": "0x1" },
                // Untracked token -> wallet (skip)
                { "address": "0xdead000000000000000000000000000000000001", "topics": [
                    TRANSFER_EVENT_TOPIC, from, wallet_word ], "data": "0x2" },
                // wrong topic (skip)
                { "address": USDC_BASE, "topics": [
                    "0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925", from, wallet_word ], "data": "0x3" }
            ]
        });
        let transfers = parse_harvest_transfers(&receipt, wallet);
        assert_eq!(transfers.len(), 2);
        // 0x1d4af = 119983 raw USDC
        assert_eq!(transfers[0].token_symbol, "USDC");
        assert_eq!(transfers[0].amount_raw, "119983");
        assert_eq!(transfers[0].amount_human, "0.119983");
        assert_eq!(transfers[0].decimals, 6);
        // 0xc1e3fd41c20874 = 54468217823004916 raw NOCK
        assert_eq!(transfers[1].token_symbol, "NOCK");
        assert_eq!(transfers[1].amount_raw, "54468217823004916");
        assert_eq!(transfers[1].amount_human, "5.4468217823004916");
        assert_eq!(transfers[1].decimals, 16);
    }

    #[test]
    fn test_concentrated_amounts_match_discovery_magnitudes() {
        // Golden anchors from Phase-1 discovery (live chain reads on 2026-10-24).
        // Position B (v4 NOCK/USDC 0.75%): screenshot-era value ≈ $71.45 with the
        // split 62.82% USDC / 37.18% NOCK.
        let b = fixture_position_v4_usdc();
        let (a0, a1) = concentrated_amounts_raw(b.liquidity, b.sqrt_price_x96, b.tick_lower, b.tick_upper);
        let usdc = a0 / 1e6;
        let nock = a1 / 1e16;
        let nock_usd = 0.020783;
        let total = usdc + nock * nock_usd;
        assert!((total - 71.45).abs() / 71.45 < 0.05, "B total {}", total);
        let usdc_pct = usdc / total * 100.0;
        assert!((usdc_pct - 62.82).abs() < 2.0, "USDC share {}%", usdc_pct);
        assert!(b.tick_lower <= b.tick && b.tick <= b.tick_upper);

        // Position A (v4 ETH/NOCK 2%): compounded to ~$156 since the $89.48 screenshot,
        // distribution preserved (~15% ETH / ~85% NOCK).
        let a = fixture_position_v4_nock_eth();
        let (a0, a1) = concentrated_amounts_raw(a.liquidity, a.sqrt_price_x96, a.tick_lower, a.tick_upper);
        let eth = a0 / 1e18;
        let nock = a1 / 1e16;
        let eth_usd = 2373.14;
        let total = eth * eth_usd + nock * nock_usd;
        assert!(total > 130.0 && total < 180.0, "A total {}", total);
        let eth_pct = eth * eth_usd / total * 100.0;
        assert!((eth_pct - 15.13).abs() < 3.0, "ETH share {}%", eth_pct);

        // Position C (v3 WETH/NOCK 1%): ≈ $18 with 99.17% WETH.
        let c = fixture_position_v3_weth_nock();
        let (a0, a1) = concentrated_amounts_raw(c.liquidity, c.sqrt_price_x96, c.tick_lower, c.tick_upper);
        let weth = a0 / 1e18;
        let nock = a1 / 1e16;
        let total = weth * eth_usd + nock * nock_usd;
        assert!((total - 18.13).abs() / 18.13 < 0.05, "C total {}", total);
        let weth_pct = weth * eth_usd / total * 100.0;
        assert!(weth_pct > 97.0, "WETH share {}%", weth_pct);
    }

    #[test]
    fn test_decode_v3_positions_return_golden() {
        // REAL 12-word return from NPM positions(6336662) on Base (Phase-1 discovery).
        let hex = "0x"
            .to_string()
            + "0000000000000000000000000000000000000000000000000000000000000000" // nonce
            + "0000000000000000000000000000000000000000000000000000000000000000" // operator
            + "0000000000000000000000004200000000000000000000000000000000000006" // token0 WETH
            + "0000000000000000000000009b5e262cf9bb04869ab40b19af91d2dc85761722" // token1 NOCK
            + "0000000000000000000000000000000000000000000000000000000000002710" // fee 10000 = 1%
            + "0000000000000000000000000000000000000000000000000000000000011300" // tickLower 70400
            + "0000000000000000000000000000000000000000000000000000000000012430" // tickUpper 74800
            + "000000000000000000000000000000000000000000000000121d7ef51ca5e1ae" // liquidity
            + "0000000000000000000000000000000000000000000000000000000000000000" // fgInside0Last
            + "0000000000000000000000000000000000000000000000000000000000000000" // fgInside1Last
            + "0000000000000000000000000000000000000000000000000000000000000000" // tokensOwed0
            + "0000000000000000000000000000000000000000000000000000000000000000"; // tokensOwed1
        let d = decode_v3_positions_return(&hex).expect("decode");
        assert_eq!(d.token0, "0x4200000000000000000000000000000000000006");
        assert_eq!(d.token1, "0x9b5e262cf9bb04869ab40b19af91d2dc85761722");
        assert_eq!(d.fee, 10000);
        assert_eq!(d.tick_lower, 70400);
        assert_eq!(d.tick_upper, 74800);
        assert_eq!(d.liquidity, 1305339058220032430);
        assert_eq!(d.tokens_owed0, 0);
        assert_eq!(d.tokens_owed1, 0);
        // Fee tier in bps as the pipeline derives it: fee/100 = 100 bps = 1%.
        assert_eq!((d.fee / 100) as u32, 100);
    }

    #[test]
    fn test_decode_v3_positions_return_rejects_short() {
        assert!(decode_v3_positions_return("0x1234").is_err());
    }

    #[test]
    fn test_nock_spot_from_positions() {
        // The price-source position (NOCK/USDC v4) supplies the NOCK/USD spot.
        let a = fixture_position_v4_nock_eth();
        let b = fixture_position_v4_usdc();
        let spot = nock_spot_from_positions(&[a.clone(), b.clone()]).expect("spot");
        assert!((spot - 0.020783).abs() < 0.0002, "spot {}", spot);

        // No price source configured -> None (caller fails the cycle explicitly).
        let mut b2 = b.clone();
        b2.is_price_source = false;
        assert!(nock_spot_from_positions(&[a, b2]).is_none());
    }

    #[test]
    fn test_app_config_positions() {
        // Real discovered identifiers (2026-10-24) as they appear in config.yaml.
        let yaml = r#"
database_url: "postgresql://localhost/test"
alchemy_api_keys: ["key1"]
tracked_wallets: ["0xee114cc97ec5adeb34ba5645e315b497d8487a45"]
sync_window_seconds: 60
positions:
  - id: "uniswap-v4-nock-usdc"
    protocol: "uniswap-v4"
    nft_id: 3163431
    pool_id: "0x91a9ef0a8dd231b99fa53a2a145a9e8bca7897e0d85852107abb997916f9f99c"
    currency0: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
    currency1: "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722"
    tick_lower: 267825
    tick_upper: 271050
    is_price_source: true
  - id: "uniswap-v4-nock-eth"
    protocol: "uniswap-v4"
    nft_id: 3163432
    pool_id: "0xd316152008e12105594c32686b9d0401fd2c60ca4805c84f280069d218b771a8"
    currency0: "0x0000000000000000000000000000000000000000"
    currency1: "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722"
    tick_lower: 65200
    tick_upper: 71200
  - id: "uniswap-v4-nock-usdc-wide"
    protocol: "uniswap-v4"
    nft_id: 3163450
    pool_id: "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13"
    currency0: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
    currency1: "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722"
    tick_lower: 262200
    tick_upper: 282300
  - id: "uniswap-v3-weth-nock"
    protocol: "uniswap-v3"
    nft_id: 6336662
    npm_address: "0x03a520b32C04BF3bEEf7BEb72E919cf822Ed34f1"
    pool_address: "0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a"
eth_price_pool: "0xd0b53d9277642d899df5c87a3966a349a798f224"
"#;
        let cfg: AppConfig = serde_yaml::from_str(yaml).expect("Failed to deserialize yaml");
        assert_eq!(cfg.positions.len(), 4);
        assert_eq!(cfg.positions[0].nft_id, 3163431);
        assert_eq!(cfg.positions[0].tick_lower, Some(267825));
        assert!(cfg.positions[0].is_price_source);
        assert_eq!(cfg.positions[1].protocol, "uniswap-v4");
        assert!(!cfg.positions[1].is_price_source);
        assert_eq!(cfg.positions[2].nft_id, 3163450);
        assert_eq!(cfg.positions[2].tick_lower, Some(262200));
        assert_eq!(cfg.positions[2].tick_upper, Some(282300));
        assert!(!cfg.positions[2].is_price_source);
        assert_eq!(cfg.positions[3].protocol, "uniswap-v3");
        assert_eq!(cfg.positions[3].npm_address.as_deref(), Some("0x03a520b32C04BF3bEEf7BEb72E919cf822Ed34f1"));
        assert_eq!(cfg.positions[3].pool_address.as_deref(), Some("0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a"));
        assert_eq!(cfg.eth_price_pool.as_deref(), Some("0xd0b53d9277642d899df5c87a3966a349a798f224"));
        assert_eq!(cfg.mcp_bearer_token, None);

        // Defaults: positions optional (worker fails the cycle with a clear error),
        // ETH price pool falls back to the verified canonical address.
        let minimal: AppConfig = serde_yaml::from_str("database_url: \"x\"").unwrap();
        assert!(minimal.positions.is_empty());
        assert_eq!(minimal.eth_price_pool.as_deref(), Some(DEFAULT_ETH_PRICE_POOL));
    }

    #[test]
    fn test_mcp_text_formatters() {
        let sample_data = json!({
            "contract_fees": [
                {
                    "contract_address": "0x7c5f5a4bbd8fd63184577525326123b519429bdc",
                    "fee_token_address": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
                    "fee_token_symbol": "USDC+NOCK (Uniswap V4)",
                    "amount": "0.030 USDC + 1.83 NOCK",
                    "amount_usd": "0.0753",
                    "claimable": true,
                    "gas_break_even_usd": "0.0043"
                },
                {
                    "contract_address": "0x85f1aa3a70fedd1c52705c15baed143e675cd626",
                    "fee_token_address": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
                    "fee_token_symbol": "USDC+NOCK (Aerodrome)",
                    "amount": "0.03772 USDC + 2.14164 NOCK",
                    "amount_usd": "0.0902",
                    "claimable": true,
                    "gas_break_even_usd": "0.0043"
                }
            ],
            "derived_metrics": {
                "global_status": {
                    "total_portfolio_usd": "2952.37",
                    "total_nock_amount": "106140",
                    "nock_floor_delta": "6140",
                    "total_usdc_amount": "349.68",
                    "high_water_mark_usd": "3200.25",
                    "shot_advice_action": "HOLD & ACCRETE",
                    "shot_advice_details": "Accreting yield steadily",
                    "total_fees_claimable_usd": "0.17",
                    "gas_to_fee_delta_usd": "0.16"
                }
            }
        });

        let state_text = format_portfolio_state_text(&sample_data);
        assert!(state_text.contains("106140 NOCK"));
        assert!(state_text.contains("+6140 above 100k"));
        assert!(state_text.contains("$2952.37 USD"));

        let advice_text = format_shot_advice_text(&sample_data);
        assert!(advice_text.contains("HOLD & ACCRETE"));
        assert!(advice_text.contains("$0.17 USD"));

        let claim_text = format_claim_simulation_text(&sample_data);
        assert!(claim_text.contains("Uniswap V4"));
        assert!(claim_text.contains("Aerodrome"));
        assert!(claim_text.contains("0.0753"));
    }

    #[test]
    fn test_claim_simulation_text_uses_live_gas_never_fabricates() {
        // Live per-contract gas break-even is surfaced verbatim; the historical
        // "~$0.005 USD" fabrication must never appear.
        let data = json!({
            "contract_fees": [
                { "fee_token_symbol": "USDC+NOCK (Uniswap V4)", "amount": "0.02 USDC",
                  "amount_usd": "0.0279", "gas_break_even_usd": "0.0041" },
                { "fee_token_symbol": "USDC+NOCK (Aerodrome)", "amount": "0.01 USDC",
                  "amount_usd": "0.0167", "gas_break_even_usd": "0.0042" }
            ]
        });
        let text = format_claim_simulation_text(&data);
        assert!(text.contains("0.0042"), "representative live gas (max) should be shown: {text}");
        assert!(!text.contains("~$0.005"), "fabricated constant gas must not appear: {text}");
        assert!(text.contains("Net Accretion if Harvested: $0.0404"), "net = gross - live gas: {text}");

        // Missing gas data degrades honestly instead of substituting a constant.
        let no_gas = json!({
            "contract_fees": [
                { "fee_token_symbol": "X", "amount": "1", "amount_usd": "0.50" }
            ]
        });
        let text2 = format_claim_simulation_text(&no_gas);
        assert!(text2.contains("unavailable"), "gas absence must say so: {text2}");
        assert!(!text2.contains("0.005"), "no fabricated gas constant: {text2}");
    }

    #[test]
    fn test_no_synthetic_bracket_emitted() {
        // The Tick Bracket payload contains ONLY brackets derived from live per-position
        // ticks. Empty positions -> zero brackets (the old fabricated Aerodrome
        // 0.00000 -> 999.00000 placeholder stays gone).
        let raw = RawOnChainData::default();
        let metrics = compute_derived_metrics(
            &raw,
            0.0,
            &OperatorProfile::default(),
            StructuralChannels { channels: vec![] },
            &[],
            &LedgerAggregate::default(),
            &MarketRegime::default(),
        );
        assert_eq!(metrics.tick_bracket_hypervisor.brackets.len(), 0);

        // With the four-position fixture, exactly four live brackets.
        let metrics = compute_derived_metrics(
            &four_position_fixture(),
            0.0,
            &OperatorProfile::default(),
            StructuralChannels { channels: vec![] },
            &[],
            &LedgerAggregate::default(),
            &MarketRegime::default(),
        );
        assert_eq!(metrics.tick_bracket_hypervisor.brackets.len(), 4);
        let bracket = &metrics.tick_bracket_hypervisor.brackets[0];
        assert_ne!(bracket.lower_price, "0.00000");
        assert_ne!(bracket.upper_price, "999.00000");
    }

    // ============================================================
    // Market price backfill & multi-timeframe regime (v0.3.0)
    // ============================================================

    // ============================================================
    // v2.0.0 per-position fixtures — REAL on-chain values from Phase-1 discovery
    // (2026-10-24, see design.md / ONCHAIN_INTEGRATION.md §5). The chain-read
    // fields (liquidity, sqrtPrice, ticks, fees) are golden anchors; amounts are
    // derived through the same math the pipeline uses so tests double as
    // regression checks on the screenshot magnitudes.
    // ============================================================

    fn fixture_position_v4_nock_eth() -> RawPosition {
        RawPosition {
            position_id: "uniswap-v4:base:3163432".to_string(),
            network: "base".to_string(),
            protocol: "uniswap-v4".to_string(),
            wallet: "0xee114cc97ec5adeb34ba5645e315b497d8487a45".to_string(),
            nft_id: 3163432,
            pool_id: "0xd316152008e12105594c32686b9d0401fd2c60ca4805c84f280069d218b771a8".to_string(),
            npm_address: None,
            fee_tier_bps: 200,
            token0: PositionTokenInfo { address: NATIVE_ETH.to_string(), symbol: "ETH".to_string(), decimals: 18 },
            token1: PositionTokenInfo { address: NOCK_BASE.to_string(), symbol: "NOCK".to_string(), decimals: 16 },
            liquidity: 8293339594552439540,
            sqrt_price_x96: U256::from_hex_be("0x21baa0a039c91a6726d2879fc9").unwrap(),
            tick: 70370,
            tick_lower: 65200,
            tick_upper: 71200,
            amount0: 0.0,
            amount1: 0.0,
            fee0: 0.0,
            fee1: 0.0,
            in_range: true,
            is_price_source: false,
        }
    }

    fn fixture_position_v4_usdc() -> RawPosition {
        RawPosition {
            position_id: "uniswap-v4:base:3163431".to_string(),
            network: "base".to_string(),
            protocol: "uniswap-v4".to_string(),
            wallet: "0xee114cc97ec5adeb34ba5645e315b497d8487a45".to_string(),
            nft_id: 3163431,
            pool_id: "0x91a9ef0a8dd231b99fa53a2a145a9e8bca7897e0d85852107abb997916f9f99c".to_string(),
            npm_address: None,
            fee_tier_bps: 75,
            token0: PositionTokenInfo { address: USDC_BASE.to_string(), symbol: "USDC".to_string(), decimals: 6 },
            token1: PositionTokenInfo { address: NOCK_BASE.to_string(), symbol: "NOCK".to_string(), decimals: 16 },
            liquidity: 320235280612027,
            sqrt_price_x96: U256::from_hex_be("0x0a9598f7fe4938c0312958557374bf").unwrap(),
            tick: 269008,
            tick_lower: 267825,
            tick_upper: 271050,
            amount0: 0.0,
            amount1: 0.0,
            fee0: 0.0,
            fee1: 0.0,
            in_range: true,
            is_price_source: true,
        }
    }

    fn fixture_position_v3_weth_nock() -> RawPosition {
        RawPosition {
            position_id: "uniswap-v3:base:6336662".to_string(),
            network: "base".to_string(),
            protocol: "uniswap-v3".to_string(),
            wallet: "0xee114cc97ec5adeb34ba5645e315b497d8487a45".to_string(),
            nft_id: 6336662,
            pool_id: "0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a".to_string(),
            npm_address: Some(UNISWAP_V3_NPM_BASE.to_string()),
            fee_tier_bps: 100,
            token0: PositionTokenInfo { address: WETH_BASE.to_string(), symbol: "WETH".to_string(), decimals: 18 },
            token1: PositionTokenInfo { address: NOCK_BASE.to_string(), symbol: "NOCK".to_string(), decimals: 16 },
            liquidity: 1305339058220032430,
            sqrt_price_x96: U256::from_hex_be("0x21d595ed92e90d8945a5ef7766").unwrap(),
            tick: 70433,
            tick_lower: 70400,
            tick_upper: 74800,
            amount0: 0.0,
            amount1: 0.0,
            fee0: 0.0,
            fee1: 0.0079878029, // v3 uncollected NOCK fees at discovery ($0.000166)
            in_range: true,
            is_price_source: false,
        }
    }

    /// Position D — USDC/NOCK, Uniswap v4, 3.00% fee, wide raft (change request:
    /// largest holding ≈ $207). Chain reads 2026-10-24 02:37 UTC: liquidity =
    /// PositionLiquidity(3163450) — the operator compounded post-mint (mint tx
    /// 0x6cdabada… +48.3e12, add +143.7e12, +1.2e12, partial remove −7.7e12);
    /// sqrtPrice/tick via StateView getSlot0; feeGrowthInside (0,0) — fees zero.
    fn fixture_position_v4_usdc_wide() -> RawPosition {
        RawPosition {
            position_id: "uniswap-v4:base:3163450".to_string(),
            network: "base".to_string(),
            protocol: "uniswap-v4".to_string(),
            wallet: "0xee114cc97ec5adeb34ba5645e315b497d8487a45".to_string(),
            nft_id: 3163450,
            pool_id: "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13".to_string(),
            npm_address: None,
            fee_tier_bps: 300,
            token0: PositionTokenInfo { address: USDC_BASE.to_string(), symbol: "USDC".to_string(), decimals: 6 },
            token1: PositionTokenInfo { address: NOCK_BASE.to_string(), symbol: "NOCK".to_string(), decimals: 16 },
            liquidity: 185454734478733,
            sqrt_price_x96: U256::from_hex_be("0xa9d5d9b7b19cc983c088d1f6cd97b").unwrap(),
            tick: 269065,
            tick_lower: 262200,
            tick_upper: 282300,
            amount0: 0.0,
            amount1: 0.0,
            fee0: 0.0,
            fee1: 0.0,
            in_range: true,
            is_price_source: false,
        }
    }

    /// The four-position portfolio fixture with amounts derived exactly like the
    /// live pipeline. Magnitude anchors from the operator screenshots (± drift):
    /// B ≈ $71, C ≈ $18–47, A ≈ $156 (compounded +75% since the $89.48 screenshot),
    /// D ≈ $208 (largest holding; minted 2026-10-24 morning).
    fn four_position_fixture() -> RawOnChainData {
        let mut a = fixture_position_v4_nock_eth();
        let mut b = fixture_position_v4_usdc();
        let mut c = fixture_position_v3_weth_nock();
        let mut d = fixture_position_v4_usdc_wide();
        for p in [&mut a, &mut b, &mut c, &mut d] {
            let (a0, a1) = concentrated_amounts_raw(p.liquidity, p.sqrt_price_x96, p.tick_lower, p.tick_upper);
            p.amount0 = a0 / 10f64.powi(p.token0.decimals);
            p.amount1 = a1 / 10f64.powi(p.token1.decimals);
        }
        let spot = nock_spot_from_positions(&[a.clone(), b.clone(), c.clone(), d.clone()]).expect("fixture price source");
        let mut raw = RawOnChainData::default();
        raw.positions = vec![a, b, c, d];
        raw.spot_price_nock_usd = spot;
        raw.live_eth_price_usd = 2373.14; // canonical v3 WETH/USDC pool at discovery
        raw.live_gas_cost_usd = 0.005;
        raw.wallet_balances = vec![
            WalletBalance {
                wallet_address: "0xee114cc97ec5adeb34ba5645e315b497d8487a45".to_string(),
                token_symbol: "NOCK".to_string(),
                token_address: NOCK_BASE.to_string(),
                amount: "110648.0751".to_string(),
                amount_usd: "2300.0".to_string(),
                updated_at: "now".to_string(),
            },
            WalletBalance {
                wallet_address: "0xee114cc97ec5adeb34ba5645e315b497d8487a45".to_string(),
                token_symbol: "USDC".to_string(),
                token_address: USDC_BASE.to_string(),
                amount: "134.4148".to_string(),
                amount_usd: "134.41".to_string(),
                updated_at: "now".to_string(),
            },
        ];
        raw
    }

    /// Raw fixture above the default 100k floor so the Overextension branch is
    /// reachable in advice tests (total NOCK ≈ 125,944, spot forced into the dip zone).
    fn raw_above_floor_fixture() -> RawOnChainData {
        let mut raw = four_position_fixture();
        raw.spot_price_nock_usd = 0.015; // dip zone (< $0.020 accumulation boundary)
        raw
    }

    #[test]
    fn test_parse_geckoterminal_ohlcv_normalizes_newest_first() {
        // Recorded fixture from the live endpoint (2026-10-12): ohlcv_list entries
        // are [open_time_unix_secs, open, high, low, close, volume], newest first.
        let body = json!({
            "data": {
                "id": "x",
                "type": "ohlcv_request_response",
                "attributes": {
                    "ohlcv_list": [
                        [1791763200, 0.0246178045182968, 0.0269877692445729, 0.022041369483387, 0.0239357572998183, 278995.4950266538],
                        [1791676800, 0.0268425121191988, 0.0271935570968404, 0.023807436927751, 0.0246178045182968, 263160.2698464311],
                        [1791590400, 0.0234294762339549, 0.0283541273531182, 0.0225412081009429, 0.0268425121191988],
                        ["not-a-number", 1, 2, 3, 4],
                        [1791504000, 1, 2, 3]
                    ]
                }
            }
        });
        let candles = parse_geckoterminal_ohlcv(&body);
        assert_eq!(candles.len(), 3, "malformed entries must be skipped");
        // Newest-first input normalized to ascending close order
        assert!(candles[0].timestamp < candles[1].timestamp);
        assert!(candles[1].timestamp < candles[2].timestamp);
        assert_eq!(
            candles[2].timestamp.format("%Y-%m-%d").to_string(),
            "2026-10-12"
        );
        assert!((candles[2].close - 0.0239357572998183).abs() < 1e-12);
        assert_eq!(candles[0].volume, None, "5-element entry has no volume");
        assert!(candles[2].volume.is_some());
    }

    #[test]
    fn test_parse_geckoterminal_ohlcv_rejects_garbage() {
        assert!(parse_geckoterminal_ohlcv(&json!({})).is_empty());
        assert!(parse_geckoterminal_ohlcv(&json!({ "data": {} })).is_empty());
        assert!(parse_geckoterminal_ohlcv(&json!({ "data": { "attributes": { "ohlcv_list": [] } } })).is_empty());
    }

    #[test]
    fn test_parse_dexscreener_price() {
        let body = json!({
            "schemaVersion": "1.0.0",
            "pairs": [
                { "chainId": "base", "dexId": "aerodrome", "priceUsd": "0.02396" }
            ]
        });
        let price = parse_dexscreener_price(&body).unwrap();
        assert!((price - 0.02396).abs() < 1e-12);
        // Missing/malformed shapes degrade to None, never fabricate a price
        assert!(parse_dexscreener_price(&json!({})).is_none());
        assert!(parse_dexscreener_price(&json!({ "pairs": [] })).is_none());
        assert!(parse_dexscreener_price(&json!({ "pairs": [{ "priceUsd": "abc" }] })).is_none());
        assert!(parse_dexscreener_price(&json!({ "pairs": [{ "priceUsd": "0" }] })).is_none());
    }

    #[test]
    fn test_plan_backfill_cases() {
        use chrono::TimeZone;
        let at = |d: i64, m: i64| Utc.with_ymd_and_hms(2026, 10, 12, 0, 0, 0).unwrap()
            + chrono::Duration::days(d)
            + chrono::Duration::minutes(m);
        let now = at(0, 0);

        // Empty table (first boot): full backfill + fallback seeding allowed
        let plan = plan_backfill(None, None, now, 15, 14);
        assert!(plan.backfill_hourly && plan.backfill_daily && plan.seed_fallback_point);

        // Fresh deep history, last point 5 minutes old: nothing to do
        let plan = plan_backfill(Some(at(0, -5)), Some(at(-30, 0)), now, 15, 14);
        assert_eq!(plan, BackfillPlan::default());

        // Laptop slept for 3 hours: hourly gap fill (daily depth still sufficient)
        let plan = plan_backfill(Some(at(0, -180)), Some(at(-30, 0)), now, 15, 14);
        assert!(plan.backfill_hourly && plan.seed_fallback_point && !plan.backfill_daily);

        // Fresh but shallow history: extend daily depth, no gap fill
        let plan = plan_backfill(Some(at(0, -5)), Some(at(-7, 0)), now, 15, 14);
        assert!(!plan.backfill_hourly && !plan.seed_fallback_point && plan.backfill_daily);
    }

    #[test]
    fn test_daily_closes_from_points() {
        use chrono::TimeZone;
        let at = |d: u32, h: u32| Utc.with_ymd_and_hms(2026, 10, d, h, 0, 0).unwrap();
        let points = vec![
            (at(1, 0), Some(0.10), 0.10), // day-1 daily candle close
            (at(1, 5), None, 0.105),      // day-1 spot: ignored, candle is authoritative
            (at(2, 5), None, 0.20),       // day-2 spots only
            (at(2, 10), None, 0.21),      // ...last spot of the day wins
            (at(3, 0), Some(0.30), 0.30), // day-3 candle only
        ];
        let closes = daily_closes_from_points(&points);
        assert_eq!(closes.len(), 3);
        assert!((closes[0] - 0.10).abs() < 1e-12);
        assert!((closes[1] - 0.21).abs() < 1e-12);
        assert!((closes[2] - 0.30).abs() < 1e-12);
        assert!(daily_closes_from_points(&[]).is_empty());
    }

    #[test]
    fn test_daily_rsi_invariant_to_provenance_mix_and_gaps() {
        use chrono::TimeZone;
        let day0 = Utc.with_ymd_and_hms(2026, 9, 1, 0, 0, 0).unwrap();
        // 16 daily closes, monotonic gains; day 5 carries spots instead of a candle
        // (backfilled vs contemporaneous mix) and day 7 is missing entirely (gap).
        let mut points = Vec::new();
        let mut expected = Vec::new();
        for i in 0..16u32 {
            let close = 100.0 + i as f64;
            if i == 5 {
                points.push((day0 + chrono::Duration::days(i as i64) + chrono::Duration::hours(5), None, close));
                points.push((day0 + chrono::Duration::days(i as i64) + chrono::Duration::hours(10), None, close + 0.001));
                expected.push(close + 0.001); // last spot of the day
            } else if i == 7 {
                continue; // provider gap: no data at all for this day
            } else {
                points.push((day0 + chrono::Duration::days(i as i64), Some(close), close));
                expected.push(close);
            }
        }
        let closes = daily_closes_from_points(&points);
        assert_eq!(closes, expected);
        let rsi_actual = compute_rsi_series(&closes, RSI_PERIOD).into_iter().last().flatten().unwrap();
        let rsi_expected = compute_rsi_series(&expected, RSI_PERIOD).into_iter().last().flatten().unwrap();
        assert!((rsi_actual - rsi_expected).abs() < 1e-9);
        assert!(rsi_actual > 99.0, "monotonic gains must pin daily RSI high");
    }

    #[test]
    fn test_overextension_cooldown_ignores_micro_pulse() {
        // The v0.2.x bug this feature fixes: a steep 4-hour intraday climb pins
        // the 5-minute RSI(14) near 100 while the daily trend stays healthy.
        // Deployment must NOT be suppressed — the cooldown keys on the daily RSI.
        let micro_closes: Vec<f64> = (1..=30).map(|i| 100.0 + i as f64).collect();
        let rsi_5m = compute_rsi_series(&micro_closes, RSI_PERIOD).into_iter().last().flatten().unwrap();
        assert!(rsi_5m >= RSI_OVERBOUGHT, "fixture must reproduce the micro pulse: {rsi_5m}");
        let macro_closes: Vec<f64> = vec![
            100.0, 99.5, 101.0, 100.5, 99.0, 100.0, 101.5, 100.0,
            99.5, 100.5, 101.0, 100.0, 99.0, 100.5, 101.0,
        ];
        let rsi_1d = compute_rsi_series(&macro_closes, RSI_PERIOD).into_iter().last().flatten().unwrap();
        assert!(rsi_1d < RSI_OVERBOUGHT, "healthy daily trend: {rsi_1d}");

        let regime = MarketRegime {
            rsi_5m: Some(rsi_5m),
            rsi_1d: Some(rsi_1d),
            overextended: rsi_1d >= RSI_OVERBOUGHT,
        };
        assert!(!regime.overextended);
        let metrics = compute_derived_metrics(
            &raw_above_floor_fixture(),
            0.0,
            &OperatorProfile::default(),
            StructuralChannels { channels: vec![] },
            &[],
            &LedgerAggregate::default(),
            &regime,
        );
        assert!(
            !metrics.global_status.shot_advice_action.contains("Overextension Cooldown"),
            "micro pulse must not trigger the cooldown: {}",
            metrics.global_status.shot_advice_action
        );
        assert_eq!(metrics.global_status.regime, "normal");
        assert!(metrics.global_status.rsi_14.is_some());
        assert!(metrics.global_status.rsi_14_1d.is_some());
    }

    #[test]
    fn test_overextension_cooldown_fires_on_daily_rsi() {
        // Monotonic multi-day gains pin the daily RSI(14) near 100: the cooldown
        // fires and cites the daily series explicitly.
        let macro_closes: Vec<f64> = (1..=20).map(|i| 100.0 + i as f64).collect();
        let rsi_1d = compute_rsi_series(&macro_closes, RSI_PERIOD).into_iter().last().flatten().unwrap();
        assert!(rsi_1d >= RSI_OVERBOUGHT);
        let regime = MarketRegime {
            rsi_5m: Some(40.0), // micro is calm; only the daily series decides
            rsi_1d: Some(rsi_1d),
            overextended: rsi_1d >= RSI_OVERBOUGHT,
        };
        let metrics = compute_derived_metrics(
            &raw_above_floor_fixture(),
            0.0,
            &OperatorProfile::default(),
            StructuralChannels { channels: vec![] },
            &[],
            &LedgerAggregate::default(),
            &regime,
        );
        assert!(metrics.global_status.shot_advice_action.contains("Overextension Cooldown"));
        assert!(metrics.global_status.shot_advice_details.contains("daily RSI(14)"));
        assert_eq!(metrics.global_status.regime, "overextended");
        assert_eq!(metrics.global_status.rsi_14.as_deref(), Some("40.0"));
    }

    // --- DB-gated integration tests (gracefully skipped without DATABASE_URL) ---

    async fn integration_pool() -> Option<PgPool> {
        let Ok(url) = std::env::var("DATABASE_URL") else {
            eprintln!("DATABASE_URL not set; skipping DB integration test");
            return None;
        };
        match PgPool::connect(&url).await {
            Ok(pool) => Some(pool),
            Err(e) => {
                eprintln!("Postgres unreachable ({e}); skipping DB integration test");
                None
            }
        }
    }

    async fn ensure_market_price_history_table(pool: &PgPool) {
        let ddl = "CREATE TABLE IF NOT EXISTS market_price_history (
                timestamp      TIMESTAMPTZ NOT NULL PRIMARY KEY,
                price_usd      FLOAT8 NOT NULL,
                open_usd       FLOAT8,
                high_usd       FLOAT8,
                low_usd        FLOAT8,
                close_usd      FLOAT8,
                volume_usd     FLOAT8,
                source         VARCHAR NOT NULL,
                is_backfilled  BOOLEAN NOT NULL DEFAULT false,
                backfilled_at  TIMESTAMPTZ
            )";
        if let Err(first_err) = sqlx::query(ddl).execute(pool).await {
            // Parallel test tasks racing CREATE TABLE IF NOT EXISTS can still lose
            // the pg_type rowtype race inside Postgres; a single retry settles it.
            if sqlx::query(ddl).execute(pool).await.is_err() {
                panic!("create market_price_history: {first_err}");
            }
        }
    }

    #[tokio::test]
    async fn test_candle_upsert_is_idempotent() {
        let Some(pool) = integration_pool().await else { return };
        ensure_market_price_history_table(&pool).await;
        let open = Utc::now();
        let candles = vec![PriceCandle {
            timestamp: open,
            open: 1.0,
            high: 2.0,
            low: 0.5,
            close: 1.5,
            volume: Some(9.0),
        }];
        let now = open + chrono::Duration::hours(2); // candle fully completed
        let first = upsert_price_candles(&pool, &candles, PRICE_SOURCE_GECKOTERMINAL, chrono::Duration::hours(1), now).await.unwrap();
        let rerun = upsert_price_candles(&pool, &candles, PRICE_SOURCE_GECKOTERMINAL, chrono::Duration::hours(1), now).await.unwrap();
        assert_eq!(first, 1, "first insert adds one row");
        assert_eq!(rerun, 0, "rerun must be a no-op");
        let count: i64 = sqlx::query("SELECT COUNT(*) FROM market_price_history WHERE timestamp = $1")
            .bind(open)
            .fetch_one(&pool)
            .await
            .unwrap()
            .try_get(0)
            .unwrap();
        assert_eq!(count, 1);
        // In-progress candle (open < one period ago) must be skipped, not frozen
        let partial = upsert_price_candles(
            &pool,
            &[PriceCandle { timestamp: now, open: 1.0, high: 1.0, low: 1.0, close: 1.0, volume: None }],
            PRICE_SOURCE_GECKOTERMINAL,
            chrono::Duration::hours(1),
            now,
        )
        .await
        .unwrap();
        assert_eq!(partial, 0, "in-progress candle must not be stored");
        sqlx::query("DELETE FROM market_price_history WHERE timestamp = $1")
            .bind(open)
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_backfill_never_overwrites_contemporaneous_row() {
        let Some(pool) = integration_pool().await else { return };
        ensure_market_price_history_table(&pool).await;
        let ts = Utc::now();
        insert_price_point(&pool, ts, 1.23, PRICE_SOURCE_RPC, false).await.unwrap();
        insert_price_point(&pool, ts, 9.99, PRICE_SOURCE_GECKOTERMINAL, true).await.unwrap();
        let row = sqlx::query(
            "SELECT price_usd, source, is_backfilled FROM market_price_history WHERE timestamp = $1",
        )
        .bind(ts)
        .fetch_one(&pool)
        .await
        .unwrap();
        let price: f64 = row.try_get("price_usd").unwrap();
        let source: String = row.try_get("source").unwrap();
        let backfilled: bool = row.try_get("is_backfilled").unwrap();
        assert!((price - 1.23).abs() < 1e-12, "contemporaneous observation must win");
        assert_eq!(source, PRICE_SOURCE_RPC);
        assert!(!backfilled);
        sqlx::query("DELETE FROM market_price_history WHERE timestamp = $1")
            .bind(ts)
            .execute(&pool)
            .await
            .unwrap();
    }
}
