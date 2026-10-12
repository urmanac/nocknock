use anyhow::Result;
use axum::{
    extract::Extension,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
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
    #[serde(default = "default_v4_baseline_liquidity")]
    pub v4_baseline_liquidity: f64,
    #[serde(default = "default_v4_baseline_nock")]
    pub v4_baseline_nock: f64,
    #[serde(default = "default_v4_baseline_usdc")]
    pub v4_baseline_usdc: f64,
    #[serde(default = "default_v4_fee_baseline_usdc")]
    pub v4_fee_baseline_usdc: f64,
    #[serde(default = "default_v4_fee_baseline_nock")]
    pub v4_fee_baseline_nock: f64,
    #[serde(default = "default_target_weights")]
    pub target_weights_bps: TargetWeightsBps,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TargetWeightsBps {
    #[serde(default = "default_treasury_bps")]
    pub treasury: i32,
    #[serde(default = "default_aerodrome_bps")]
    pub aerodrome_lp: i32,
    #[serde(default = "default_uniswap_v4_bps")]
    pub uniswap_v4_lp: i32,
    #[serde(default = "default_cash_bps")]
    pub cash: i32,
}

fn default_treasury_floor() -> f64 { 100000.0 }
fn default_v4_baseline_liquidity() -> f64 { 65719361234409.0 }
fn default_v4_baseline_nock() -> f64 { 4004.31227 }
fn default_v4_baseline_usdc() -> f64 { 23.63241 }
fn default_v4_fee_baseline_usdc() -> f64 { 0.0245 }
fn default_v4_fee_baseline_nock() -> f64 { 1.4820 }
fn default_treasury_bps() -> i32 { 8000 }
fn default_aerodrome_bps() -> i32 { 1000 }
fn default_uniswap_v4_bps() -> i32 { 500 }
fn default_cash_bps() -> i32 { 500 }
fn default_target_weights() -> TargetWeightsBps {
    TargetWeightsBps {
        treasury: 8000,
        aerodrome_lp: 1000,
        uniswap_v4_lp: 500,
        cash: 500,
    }
}

impl Default for OperatorProfile {
    fn default() -> Self {
        Self {
            treasury_floor_nock: default_treasury_floor(),
            v4_baseline_liquidity: default_v4_baseline_liquidity(),
            v4_baseline_nock: default_v4_baseline_nock(),
            v4_baseline_usdc: default_v4_baseline_usdc(),
            v4_fee_baseline_usdc: default_v4_fee_baseline_usdc(),
            v4_fee_baseline_nock: default_v4_fee_baseline_nock(),
            target_weights_bps: default_target_weights(),
        }
    }
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
    #[serde(default)]
    v4_position_id: Option<u64>,
    #[serde(default)]
    v4_fee_baseline_usdc: Option<f64>,
    #[serde(default)]
    v4_fee_baseline_nock: Option<f64>,
    #[serde(default)]
    mcp_bearer_token: Option<String>,
    #[serde(default)]
    operator_profile: OperatorProfile,
}

impl AppConfig {
    pub fn profile(&self) -> OperatorProfile {
        let mut p = self.operator_profile.clone();
        if let Some(v) = self.v4_fee_baseline_usdc {
            p.v4_fee_baseline_usdc = v;
        }
        if let Some(v) = self.v4_fee_baseline_nock {
            p.v4_fee_baseline_nock = v;
        }
        p
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
    #[serde(default)]
    live_tranche_1_nock: f64,
    #[serde(default)]
    live_tranche_1_usdc: f64,
    #[serde(default)]
    live_tranche_2_nock: f64,
    #[serde(default)]
    live_tranche_2_usdc: f64,
    #[serde(default)]
    live_gas_cost_usd: f64,
    #[serde(default)]
    live_aero_fee_usdc: f64,
    #[serde(default)]
    live_aero_fee_nock: f64,
    #[serde(default)]
    live_uni_fee_usdc: f64,
    #[serde(default)]
    live_uni_fee_nock: f64,
    #[serde(default = "default_eth_price")]
    live_eth_price_usd: f64,
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
    tranche_1_nock: Vec<String>,
    #[serde(default)]
    tranche_2_nock: Vec<String>,
    #[serde(default)]
    tranche_3_nock: Vec<String>,
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
    distance_bps: i32,
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

        (StatusCode::OK, headers, Json(json!(envelope)))
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
) -> impl IntoResponse {
    let _interval = params
        .get("interval")
        .map(|s| s.as_str())
        .unwrap_or("1d");
    let limit = params
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    let rows = sqlx::query!(
        r#"
        SELECT snapshot_id, generated_at, state_hash, version, data
        FROM snapshots
        ORDER BY generated_at DESC
        LIMIT $1
        "#,
        limit as i64
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let snapshots: Vec<SnapshotEnvelope> = rows
        .into_iter()
        .filter_map(|r| {
            let data: SnapshotData = serde_json::from_value(r.data).ok()?;
            Some(SnapshotEnvelope {
                meta: SnapshotMeta {
                    generated_at: r.generated_at,
                    state_hash: r.state_hash,
                    version: r.version,
                    snapshot_id: r.snapshot_id,
                },
                data,
            })
        })
        .collect();

    Json(json!({
        "snapshots": snapshots,
        "pagination": {
            "limit": limit,
            "returned": snapshots.len(),
            "has_more": snapshots.len() == limit as usize,
        }
    }))
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
         • Execution Trigger: Batch Harvest shots unlock at >= $5.00 accumulated fees."
    )
}

pub fn format_claim_simulation_text(data: &serde_json::Value) -> String {
    let fees = data.get("contract_fees").and_then(|f| f.as_array());
    let mut details = String::new();
    let mut total_claimable_usd = 0.0f64;

    if let Some(list) = fees {
        for fee in list {
            let symbol = fee.get("fee_token_symbol").and_then(|s| s.as_str()).unwrap_or("Pool");
            let amount = fee.get("amount").and_then(|a| a.as_str()).unwrap_or("0");
            let amount_usd = fee.get("amount_usd").and_then(|a| a.as_str()).unwrap_or("0.00");
            let gas = fee.get("gas_break_even_usd").and_then(|g| g.as_str()).unwrap_or("0.005");
            if let Ok(usd_val) = amount_usd.parse::<f64>() {
                total_claimable_usd += usd_val;
            }
            details.push_str(&format!("  - {symbol}: {amount} (~${amount_usd} USD | Gas Est: ${gas})\n"));
        }
    }

    format!(
        "⚡ Fee Claim Simulation\n\
         =======================\n\
         • Total Gross Claimable: ${:.4} USD\n\
         • Estimated Base Gas: ~$0.005 USD\n\
         • Net Accretion if Harvested: ${:.4} USD\n\
         Pool Breakdown:\n{}",
        total_claimable_usd,
        (total_claimable_usd - 0.005).max(0.0),
        details
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
    let derived_metrics = compute_derived_metrics(&raw_data, prev_hwm, &profile, structural_channels,
        &counterfactual_history);

    // Build snapshot data
    let snapshot_data = SnapshotData {
        wallet_balances: raw_data.wallet_balances,
        pool_mechanics: raw_data.pool_mechanics,
        concentrated_liquidity: raw_data.concentrated_liquidity,
        contract_fees: raw_data.contract_fees,
        derived_metrics,
    };

    // Compute deterministic state hash
    let state_hash = compute_state_hash(&snapshot_data);

    // Build envelope
    let now = Utc::now();
    let envelope = SnapshotEnvelope {
        meta: SnapshotMeta {
            generated_at: now,
            state_hash: state_hash.clone(),
            version: "1.0.0".to_string(),
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

/// Represents an unsigned 256-bit integer as (hi: u128, lo: u128)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct U256(pub u128, pub u128);

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
}

async fn fetch_raw_data(state: &Arc<AppState>) -> Result<RawOnChainData> {
    let alchemy_key = state.config.alchemy_api_keys.first().cloned().unwrap_or_default();
    let alchemy_url = format!("https://base-mainnet.g.alchemy.com/v2/{}", alchemy_key);
    let wallet_addr = &state.config.tracked_wallets[0];
    let wallet_padded = format!("000000000000000000000000{}", wallet_addr.trim_start_matches("0x").to_lowercase());

    // 0. Fetch live ETH/USD spot price from Aerodrome vAMM-WETH/USDC (0x88A43bbDF9D098eEC7bCEda4e2494615dfD9bB9C)
    let eth_reserves_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": "0x88A43bbDF9D098eEC7bCEda4e2494615dfD9bB9C",
            "data": "0x0902f1ac"
        }, "latest"],
        "id": 99
    });
    let mut live_eth_price_usd = 2500.0f64;
    if let Some(res_json) = post_rpc_with_retry(&state.http_client, &alchemy_url, &eth_reserves_payload, 2).await {
        if let Some(hex_str) = res_json.get("result").and_then(|r| r.as_str()) {
            let clean = hex_str.trim_start_matches("0x");
            if clean.len() >= 128 {
                if let (Ok(r0), Ok(r1)) = (
                    u128::from_str_radix(&clean[..64], 16),
                    u128::from_str_radix(&clean[64..128], 16),
                ) {
                    let weth = r0 as f64 / 1e18;
                    let usdc = r1 as f64 / 1e6;
                    if weth > 0.0 {
                        live_eth_price_usd = usdc / weth;
                    }
                }
            }
        }
    }

    // 1. Fetch live gas price (eth_gasPrice)
    let gas_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_gasPrice",
        "params": [],
        "id": 10
    });
    let mut live_gas_cost_usd = 0.005f64;
    if let Some(res_json) = post_rpc_with_retry(&state.http_client, &alchemy_url, &gas_payload, 2).await {
        if let Some(hex_str) = res_json.get("result").and_then(|r| r.as_str()) {
            let clean = hex_str.trim_start_matches("0x");
            if let Ok(wei) = u128::from_str_radix(clean, 16) {
                let eth_gas_cost = (wei as f64 / 1e18) * 150_000.0;
                let usd_gas_cost = (eth_gas_cost * live_eth_price_usd) + 0.002;
                live_gas_cost_usd = usd_gas_cost.clamp(0.002, 0.05);
            }
        }
    }

    // 2. Fetch live reserves from Aerodrome pool 0x85f1aa3a70fedd1c52705c15baed143e675cd626
    let reserves_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": "0x85f1aa3a70fedd1c52705c15baed143e675cd626",
            "data": "0x0902f1ac"
        }, "latest"],
        "id": 1
    });

    let mut spot_price = 0.0f64;
    let mut reserve_usdc = 0.0f64;
    let mut reserve_nock = 0.0f64;
    let mut reserves_found = false;

    if let Some(res_json) = post_rpc_with_retry(&state.http_client, &alchemy_url, &reserves_payload, 2).await {
        if let Some(hex_str) = res_json.get("result").and_then(|r| r.as_str()) {
            let clean = hex_str.trim_start_matches("0x");
            if clean.len() >= 128 {
                if let (Ok(r0), Ok(r1)) = (
                    u128::from_str_radix(&clean[..64], 16),
                    u128::from_str_radix(&clean[64..128], 16),
                ) {
                    reserve_usdc = r0 as f64 / 1e6;
                    reserve_nock = r1 as f64 / 1e16;
                    if reserve_nock > 0.0 {
                        spot_price = reserve_usdc / reserve_nock;
                        reserves_found = true;
                    }
                }
            }
        }
    }

    if !reserves_found {
        return Err(anyhow::anyhow!("Failed to query live Aerodrome pool reserves from RPC"));
    }

    // 3. Fetch live LP balance and totalSupply on Aerodrome to calculate exact tranche 2 split
    let aero_bal_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": "0x85f1aa3a70fedd1c52705c15baed143e675cd626",
            "data": format!("0x70a08231{}", wallet_padded)
        }, "latest"],
        "id": 3
    });
    let aero_ts_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": "0x85f1aa3a70fedd1c52705c15baed143e675cd626",
            "data": "0x18160ddd"
        }, "latest"],
        "id": 4
    });
    let mut user_lp_balance = 0u128;
    let mut pool_total_supply = 0u128;
    if let Some(res_json) = post_rpc_with_retry(&state.http_client, &alchemy_url, &aero_bal_payload, 2).await {
        if let Some(hex_str) = res_json.get("result").and_then(|r| r.as_str()) {
            if let Ok(val) = u128::from_str_radix(hex_str.trim_start_matches("0x"), 16) {
                user_lp_balance = val;
            }
        }
    }
    if let Some(res_json) = post_rpc_with_retry(&state.http_client, &alchemy_url, &aero_ts_payload, 2).await {
        if let Some(hex_str) = res_json.get("result").and_then(|r| r.as_str()) {
            if let Ok(val) = u128::from_str_radix(hex_str.trim_start_matches("0x"), 16) {
                pool_total_supply = val;
            }
        }
    }
    if pool_total_supply == 0 {
        return Err(anyhow::anyhow!("Failed to query live Aerodrome total supply from RPC"));
    }
    let lp_share = user_lp_balance as f64 / pool_total_supply as f64;
    let live_t2_usdc = reserve_usdc * lp_share;
    let live_t2_nock = reserve_nock * lp_share;

    // 4. Fetch Uniswap V4 Position Info & Live Uncollected Fees via official StateView contract on Base
    // StateView: 0xa3c0c9b65bad0b08107aa264b0f3db444b867a71
    // PositionManager: 0x7c5f5a4bbd8fd63184577525326123b519429bdc
    // PoolId: 0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13
    let v4_pos_id = state.config.v4_position_id.unwrap_or(3126000);
    let v4_pos_hex = format!("{:064x}", v4_pos_id);
    let state_view_addr = "0xa3c0c9b65bad0b08107aa264b0f3db444b867a71";
    let pool_id_hex = "064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13";
    let pos_manager_padded = "0000000000000000000000007c5f5a4bbd8fd63184577525326123b519429bdc";
    let tick_lower_hex = format!("{:064x}", 223200u32);
    let tick_upper_hex = format!("{:064x}", 274200u32);

    // 4a. getPositionInfo(bytes32,address,int24,int24,bytes32) -> 0xdacf1d2f
    let get_pos_data = format!("0xdacf1d2f{}{}{}{}{}", pool_id_hex, pos_manager_padded, tick_lower_hex, tick_upper_hex, v4_pos_hex);
    let v4_pos_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": state_view_addr,
            "data": get_pos_data
        }, "latest"],
        "id": 5
    });

    // 4b. getFeeGrowthInside(bytes32,int24,int24) -> 0x53e9c1fb
    let get_fg_data = format!("0x53e9c1fb{}{}{}", pool_id_hex, tick_lower_hex, tick_upper_hex);
    let v4_fg_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": state_view_addr,
            "data": get_fg_data
        }, "latest"],
        "id": 6
    });

    let mut v4_liquidity_val = 0u128;
    let mut fg0_last = U256::default();
    let mut fg1_last = U256::default();
    let mut fg0_curr = U256::default();
    let mut fg1_curr = U256::default();
    let mut v4_live_query_success = false;

    if let (Some(res_pos), Some(res_fg)) = (
        post_rpc_with_retry(&state.http_client, &alchemy_url, &v4_pos_payload, 2).await,
        post_rpc_with_retry(&state.http_client, &alchemy_url, &v4_fg_payload, 2).await,
    ) {
        if let (Some(hex_pos), Some(hex_fg)) = (
            res_pos.get("result").and_then(|r| r.as_str()),
            res_fg.get("result").and_then(|r| r.as_str()),
        ) {
            let clean_pos = hex_pos.trim_start_matches("0x");
            let clean_fg = hex_fg.trim_start_matches("0x");
            if clean_pos.len() >= 192 && clean_fg.len() >= 128 {
                if let (Ok(liq), Ok(f0_l), Ok(f1_l), Ok(f0_c), Ok(f1_c)) = (
                    u128::from_str_radix(&clean_pos[0..64], 16),
                    U256::from_hex_be(&clean_pos[64..128]),
                    U256::from_hex_be(&clean_pos[128..192]),
                    U256::from_hex_be(&clean_fg[0..64]),
                    U256::from_hex_be(&clean_fg[64..128]),
                ) {
                    if liq > 0 {
                        v4_liquidity_val = liq;
                        fg0_last = f0_l;
                        fg1_last = f1_l;
                        fg0_curr = f0_c;
                        fg1_curr = f1_c;
                        v4_live_query_success = true;
                    }
                }
            }
        }
    }

    if !v4_live_query_success {
        return Err(anyhow::anyhow!("Failed to query live Uniswap V4 position and fee growth from StateView RPC"));
    }

    let delta_fg0 = fg0_curr.wrapping_sub(&fg0_last);
    let delta_fg1 = fg1_curr.wrapping_sub(&fg1_last);
    let fee0_raw = delta_fg0.mul_div_q128(v4_liquidity_val);
    let fee1_raw = delta_fg1.mul_div_q128(v4_liquidity_val);

    let live_uni_fee_usdc = fee0_raw as f64 / 1e6; // USDC has 6 decimals
    let live_uni_fee_nock = fee1_raw as f64 / 1e16; // NOCK has 16 decimals
    let live_uni_fee_usd = live_uni_fee_usdc + live_uni_fee_nock * spot_price;

    let profile = state.config.profile();
    let v4_scale = (v4_liquidity_val as f64) / profile.v4_baseline_liquidity;
    let v4_liquidity_str = v4_liquidity_val.to_string();
    let live_t1_nock = profile.v4_baseline_nock * v4_scale;
    let live_t1_usdc = profile.v4_baseline_usdc * v4_scale;

    tracing::info!(
        "Uniswap V4 live StateView query successful: liquidity={}, fees_usdc={:.6}, fees_nock={:.4}, fees_usd={:.4}",
        v4_liquidity_val, live_uni_fee_usdc, live_uni_fee_nock, live_uni_fee_usd
    );

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

    // 6. Live Trading Fees
    // Live query for Aerodrome uncollected trading fees via simulated claimFees() call from wallet
    let aero_claim_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "from": wallet_addr,
            "to": "0x85f1aa3a70fedd1c52705c15baed143e675cd626",
            "data": "0xd294f093"
        }, "latest"],
        "id": 6
    });

    let mut aero_fee_usdc = 0.0f64;
    let mut aero_fee_nock = 0.0f64;

    if let Some(res_json) = post_rpc_with_retry(&state.http_client, &alchemy_url, &aero_claim_payload, 2).await {
        if let Some(hex_str) = res_json.get("result").and_then(|r| r.as_str()) {
            let clean = hex_str.trim_start_matches("0x");
            if clean.len() >= 128 {
                if let (Ok(c0), Ok(c1)) = (
                    u128::from_str_radix(&clean[..64], 16),
                    u128::from_str_radix(&clean[64..128], 16),
                ) {
                    aero_fee_usdc = c0 as f64 / 1e6;
                    aero_fee_nock = c1 as f64 / 1e16;
                }
            }
        }
    }

    let aero_fee_usd = aero_fee_usdc + aero_fee_nock * spot_price;

    // Uniswap V4: Use LIVE on-chain uncollected fees from tokensOwed0/1 (queried above)
    // instead of scaling operator-profile baseline by liquidity ratio.

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
        pool_mechanics: vec![PoolMechanic {
            pool_id: "0x85f1aa3a70fedd1c52705c15baed143e675cd626".to_string(),
            platform: "Aerodrome".to_string(),
            pool_type: "volatile".to_string(),
            volatile_assets: vec![
                PoolAsset {
                    token_address: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".to_string(),
                    token_symbol: "USDC".to_string(),
                    reserve_amount: format!("{:.2}", reserve_usdc),
                    reserve_usd: format!("{:.2}", reserve_usdc),
                },
                PoolAsset {
                    token_address: "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722".to_string(),
                    token_symbol: "NOCK".to_string(),
                    reserve_amount: format!("{:.2}", reserve_nock),
                    reserve_usd: format!("{:.2}", reserve_nock * spot_price),
                },
            ],
            liquidity_usd: format!("{:.2}", reserve_usdc * 2.0),
            fee_accrued_usd: format!("{:.4}", aero_fee_usd),
            fee_tier_bps: 30, // 0.3%
            tick_current: Some(if spot_price > 0.0 {
                let raw_ratio = spot_price / 1e10;
                (raw_ratio.ln() / 1.0001f64.ln()).round() as i32
            } else {
                -266743
            }),
            sqrt_price_x96: None,
        }],
        concentrated_liquidity: vec![
            ConcentratedLiquidity {
                range_id: "3126000".to_string(),
                pool_id: "0x498581ff718922c3f8e6a244956af099b2652b2b".to_string(),
                lower_tick: -274200,
                upper_tick: -223200,
                lower_price: "0.01237".to_string(),
                upper_price: "2.02783".to_string(),
                liquidity_amount: v4_liquidity_str,
                tokens_owed_0: format!("{:.6}", live_uni_fee_usdc),
                tokens_owed_1: format!("{:.4}", live_uni_fee_nock),
                in_range: spot_price >= 0.01237 && spot_price <= 2.02783,
                fee_growth_inside_0_x128: fg0_curr.to_hex_be(),
                fee_growth_inside_1_x128: fg1_curr.to_hex_be(),
            }
        ],
        contract_fees: vec![
            ContractFee {
                contract_address: "0x7c5f5a4bbd8fd63184577525326123b519429bdc".to_string(),
                fee_token_address: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".to_string(),
                fee_token_symbol: "USDC+NOCK (Uniswap V4)".to_string(),
                amount: format!("{:.6} USDC + {:.4} NOCK", live_uni_fee_usdc, live_uni_fee_nock),
                amount_usd: format!("{:.4}", live_uni_fee_usd),
                claimable: true,
                gas_break_even_usd: format!("{:.4}", live_gas_cost_usd),
            },
            ContractFee {
                contract_address: "0x85f1aa3a70fedd1c52705c15baed143e675cd626".to_string(),
                fee_token_address: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".to_string(),
                fee_token_symbol: "USDC+NOCK (Aerodrome)".to_string(),
                amount: format!("{:.5} USDC + {:.5} NOCK", aero_fee_usdc, aero_fee_nock),
                amount_usd: format!("{:.4}", aero_fee_usd),
                claimable: true,
                gas_break_even_usd: format!("{:.4}", live_gas_cost_usd),
            },
        ],
        live_tranche_1_nock: live_t1_nock,
        live_tranche_1_usdc: live_t1_usdc,
        live_tranche_2_nock: live_t2_nock,
        live_tranche_2_usdc: live_t2_usdc,
        live_gas_cost_usd: live_gas_cost_usd,
        live_aero_fee_usdc: aero_fee_usdc,
        live_aero_fee_nock: aero_fee_nock,
        live_uni_fee_usdc: live_uni_fee_usdc,
        live_uni_fee_nock: live_uni_fee_nock,
        live_eth_price_usd,
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

fn compute_derived_metrics(raw: &RawOnChainData, prev_hwm: f64, profile: &OperatorProfile, structural_channels: StructuralChannels, counterfactual_history: &[(DateTime<Utc>, f64, f64)]) -> DerivedMetrics {
    // 1. Spot price calculation
    let spot_price = if let Some(pool) = raw.pool_mechanics.first() {
        if pool.volatile_assets.len() >= 2 {
            let r0: f64 = pool.volatile_assets[0].reserve_amount.parse().unwrap_or(0.0);
            let r1: f64 = pool.volatile_assets[1].reserve_amount.parse().unwrap_or(0.0);
            if r1 > 0.0 { r0 / r1 } else { 0.0 }
        } else { 0.0 }
    } else { 0.0 };

    let aero_fee_nock = raw.live_aero_fee_nock;
    let aero_fee_usdc = raw.live_aero_fee_usdc;
    let aero_fee_usd = aero_fee_usdc + aero_fee_nock * spot_price;

    let uni_fee_nock = raw.live_uni_fee_nock;
    let uni_fee_usdc = raw.live_uni_fee_usdc;
    let uni_fee_usd = uni_fee_usdc + uni_fee_nock * spot_price;

    let total_fees_usd = aero_fee_usd + uni_fee_usd;
    let gas_cost = if raw.live_gas_cost_usd > 0.0 { raw.live_gas_cost_usd } else { 0.0043 };

    // 3. Strategic Tranches
    // Tranche 1: Uniswap V4
    let t1_nock = raw.live_tranche_1_nock;
    let t1_usdc = raw.live_tranche_1_usdc;
    let t1_usd = t1_nock * spot_price + t1_usdc;

    // Tranche 2: Aerodrome Engine
    let t2_nock = raw.live_tranche_2_nock;
    let t2_usdc = raw.live_tranche_2_usdc;
    let t2_usd = t2_nock * spot_price + t2_usdc;

    // Tranche 3: Sovereign Protected Treasury
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

    let total_portfolio_usd = t1_usd + t2_usd + t3_usd + wallet_usdc;
    let total_native_usd = (t1_nock + t2_nock + t3_nock) * spot_price;
    let total_stable_usd = t1_usdc + t2_usdc + wallet_usdc;

    let floor = profile.treasury_floor_nock;
    let now_str = Utc::now().to_rfc3339();
    let total_nock = t1_nock + t2_nock + t3_nock + aero_fee_nock + uni_fee_nock;
    let nock_floor_delta = total_nock - floor;
    let current_hwm = prev_hwm.max(total_portfolio_usd);

    // Dynamic Shot Advice Engine
    let (advice_action, advice_details) = if total_nock < floor {
        (
            "DEFENSIVE LOCK (Floor Recovery)".to_string(),
            format!(
                "CRITICAL: Total NOCK stack ({:.0}) is below the {:.0} floor by {:.0} NOCK! All yield and stable reserves are locked to replenish the sovereign foundation.",
                total_nock, floor, floor - total_nock
            ),
        )
    } else if spot_price > 0.0 && spot_price < 0.020 {
        (
            "DIP REINVESTMENT SHOT (Deploy USDC)".to_string(),
            format!(
                "Spot price (${:.5}) has breached the $0.020 accumulation boundary! Deploy saved USDC ({:.2} available) to purchase NOCK at discount and compound buffer toward 120,000.",
                spot_price, total_stable_usd
            ),
        )
    } else if spot_price >= 2.02783 {
        (
            "RANGE EXIT FLASHPOINT (Build High Wall)".to_string(),
            format!(
                "Uniswap V4 runway has fully scaled out into 100% USDC at ${:.4}! Goal achieved. Take full principal (${:.2}) and build single-sided high walls at $5.00+.",
                spot_price, t1_usd
            ),
        )
    } else if total_fees_usd >= 5.00 {
        (
            "BATCH HARVEST SHOT (Execute Claim)".to_string(),
            format!(
                "Accumulated yield (${:.2}) exceeds $5.00 threshold at {:.0}x gas efficiency. Harvest fees and apply 60/40 Split Rule (40% to Tax Escrow, 60% Growth Capital).",
                total_fees_usd, total_fees_usd / gas_cost
            ),
        )
    } else {
        (
            "HOLD & ACCRETE (Dual Growth Engine Active)".to_string(),
            format!(
                "Total NOCK held is {:.0} (+{:.0} above {:.0} hard floor). AMMs are generating dual yield (+{:.2} USDC in V4 scale-out, +{:.2} in Aero). 0% realized loss. Maintain posture.",
                total_nock, nock_floor_delta, floor, uni_fee_usd, aero_fee_usd
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
        },
        asset_aggregation_series: AssetAggregationSeries {
            time_points: vec![now_str.clone()],
            safe_haven_series: vec![format!("{:.2}", t3_usd)],
            active_protocol_series: vec![format!("{:.2}", t1_usd + t2_usd)],
            tranche_1_nock: vec![format!("{:.2}", t1_nock)],
            tranche_2_nock: vec![format!("{:.2}", t2_nock)],
            tranche_3_nock: vec![format!("{:.2}", t3_nock)],
        },
        total_value_vs_baseline: TotalValueVsBaseline {
            time_points: vec![now_str.clone()],
            portfolio_value_series: vec![format!("{:.2}", total_portfolio_usd)],
            token_price_series: vec![format!("{:.5}", spot_price)],
            high_water_mark_series: vec![format!("{:.2}", current_hwm)],
        },
        tick_bracket_hypervisor: TickBracketHypervisor {
            current_price: format!("{:.5}", spot_price),
            brackets: vec![
                TickBracket {
                    lower_price: "0.01237".to_string(),
                    upper_price: "2.02783".to_string(),
                    liquidity_usd: format!("{:.2}", t1_usd),
                    in_range: spot_price >= 0.01237 && spot_price <= 2.02783,
                    distance_bps: -200,
                },
                TickBracket {
                    lower_price: "0.00000".to_string(),
                    upper_price: "999.00000".to_string(),
                    liquidity_usd: format!("{:.2}", t2_usd),
                    in_range: true,
                    distance_bps: 0,
                },
            ],
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

                let t2_weight = ((t2_usd / total_portfolio_usd) * 10000.0) as i32;
                let t2_dev = t2_weight - w.aerodrome_lp;

                let t1_weight = ((t1_usd / total_portfolio_usd) * 10000.0) as i32;
                let t1_dev = t1_weight - w.uniswap_v4_lp;

                let usdc_weight = ((wallet_usdc / total_portfolio_usd) * 10000.0) as i32;
                let usdc_dev = usdc_weight - w.cash;

                vec![
                    AllocationDeviation {
                        bucket_name: "Sovereign Treasury (Tranche 3)".to_string(),
                        target_weight_bps: w.treasury,
                        actual_weight_bps: t3_weight,
                        deviation_bps: t3_dev,
                        status: calc_status(t3_dev, w.treasury),
                    },
                    AllocationDeviation {
                        bucket_name: "Aerodrome LP (Tranche 2)".to_string(),
                        target_weight_bps: w.aerodrome_lp,
                        actual_weight_bps: t2_weight,
                        deviation_bps: t2_dev,
                        status: calc_status(t2_dev, w.aerodrome_lp),
                    },
                    AllocationDeviation {
                        bucket_name: "Uniswap V4 LP (Tranche 1)".to_string(),
                        target_weight_bps: w.uniswap_v4_lp,
                        actual_weight_bps: t1_weight,
                        deviation_bps: t1_dev,
                        status: calc_status(t1_dev, w.uniswap_v4_lp),
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
        velocity_gas_sandbox: VelocityGasSandbox {
            contracts: vec![
                FeeCollectionIndex {
                    contract_address: "0x7c5f5a4bbd8fd63184577525326123b519429bdc".to_string(),
                    contract_name: "Uniswap V4 (NOCK/USDC 3%)".to_string(),
                    claimable_usd: format!("{:.2}", uni_fee_usd),
                    gas_break_even_usd: format!("{:.4}", gas_cost),
                    efficiency_ratio: (uni_fee_usd / gas_cost * 100.0).round() / 100.0,
                    recommended_action: "claim_now".to_string(),
                },
                FeeCollectionIndex {
                    contract_address: "0x85f1aa3a70fedd1c52705c15baed143e675cd626".to_string(),
                    contract_name: "Aerodrome (USDC/NOCK 0.3%)".to_string(),
                    claimable_usd: format!("{:.2}", aero_fee_usd),
                    gas_break_even_usd: format!("{:.4}", gas_cost),
                    efficiency_ratio: (aero_fee_usd / gas_cost * 100.0).round() / 100.0,
                    recommended_action: "claim_now".to_string(),
                },
            ],
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
        let mut raw = RawOnChainData::default();
        raw.wallet_balances = vec![
            WalletBalance {
                wallet_address: "0x123".to_string(),
                token_symbol: "NOCK".to_string(),
                token_address: "0xNOCK".to_string(),
                amount: "80000.0".to_string(),
                amount_usd: "2080.0".to_string(),
                updated_at: "now".to_string(),
            },
            WalletBalance {
                wallet_address: "0x123".to_string(),
                token_symbol: "USDC".to_string(),
                token_address: "0xUSDC".to_string(),
                amount: "100.0".to_string(),
                amount_usd: "100.0".to_string(),
                updated_at: "now".to_string(),
            },
        ];
        // Total NOCK is 80,000 < 100,000 floor
        let profile = OperatorProfile::default();
        let metrics = compute_derived_metrics(&raw, 2500.0, &profile, StructuralChannels { channels: vec![] }, &[]);
        assert!(metrics.global_status.shot_advice_action.contains("DEFENSIVE LOCK"));
    }

    #[test]
    fn test_derived_metrics_batch_harvest_above_5_dollars() {
        let mut raw = RawOnChainData::default();
        raw.wallet_balances = vec![
            WalletBalance {
                wallet_address: "0x123".to_string(),
                token_symbol: "NOCK".to_string(),
                token_address: "0xNOCK".to_string(),
                amount: "96000.0".to_string(),
                amount_usd: "2496.0".to_string(),
                updated_at: "now".to_string(),
            },
        ];
        raw.live_tranche_1_nock = 4000.0;
        raw.live_tranche_2_nock = 6000.0;
        raw.live_aero_fee_usdc = 6.0; // Accumulated fees > $5.00
        raw.live_gas_cost_usd = 0.005;

        let profile = OperatorProfile::default();
        let metrics = compute_derived_metrics(&raw, 2500.0, &profile, StructuralChannels { channels: vec![] }, &[]);
        assert!(metrics.global_status.shot_advice_action.contains("BATCH HARVEST SHOT"));
    }

    #[test]
    fn test_derived_metrics_hold_and_accrete() {
        let mut raw = RawOnChainData::default();
        raw.wallet_balances = vec![
            WalletBalance {
                wallet_address: "0x123".to_string(),
                token_symbol: "NOCK".to_string(),
                token_address: "0xNOCK".to_string(),
                amount: "96000.0".to_string(),
                amount_usd: "2496.0".to_string(),
                updated_at: "now".to_string(),
            },
        ];
        raw.live_tranche_1_nock = 4000.0;
        raw.live_tranche_2_nock = 6000.0;
        raw.live_aero_fee_usdc = 0.25;
        raw.live_aero_fee_nock = 8.0;

        let profile = OperatorProfile::default();
        let metrics = compute_derived_metrics(&raw, 2500.0, &profile, StructuralChannels { channels: vec![] }, &[]);
        assert!(metrics.global_status.shot_advice_action.contains("HOLD & ACCRETE"));
    }

    #[test]
    fn test_operator_profile_custom_treasury_floor_and_weights() {
        let yaml = r#"
database_url: "postgresql://localhost/test"
operator_profile:
  treasury_floor_nock: 150000.0
  v4_baseline_liquidity: 65719361234409.0
  v4_baseline_nock: 4004.31
  v4_baseline_usdc: 23.63
  v4_fee_baseline_usdc: 0.05
  v4_fee_baseline_nock: 2.5
  target_weights_bps:
    treasury: 7000
    aerodrome_lp: 1500
    uniswap_v4_lp: 1000
    cash: 500
"#;
        let cfg: AppConfig = serde_yaml::from_str(yaml).expect("Failed to deserialize yaml");
        let profile = cfg.profile();
        assert_eq!(profile.treasury_floor_nock, 150000.0);
        assert_eq!(profile.target_weights_bps.treasury, 7000);
        assert_eq!(profile.target_weights_bps.aerodrome_lp, 1500);
        assert_eq!(profile.target_weights_bps.uniswap_v4_lp, 1000);
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
        let metrics = compute_derived_metrics(&raw, 3000.0, &profile, StructuralChannels { channels: vec![] }, &[]);
        assert_eq!(metrics.global_status.nock_floor_target, "150000");
        assert_eq!(metrics.global_status.nock_floor_delta, "-30000");
        assert!(metrics.global_status.shot_advice_action.contains("DEFENSIVE LOCK"));
        assert_eq!(metrics.portfolio_deviation_matrix.allocations[0].target_weight_bps, 7000);
        assert_eq!(metrics.portfolio_deviation_matrix.allocations[1].target_weight_bps, 1500);
        assert_eq!(metrics.portfolio_deviation_matrix.allocations[2].target_weight_bps, 1000);
        assert_eq!(metrics.portfolio_deviation_matrix.allocations[3].target_weight_bps, 500);
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
                },
                asset_aggregation_series: AssetAggregationSeries {
                    time_points: vec!["2026-10-11T00:00:00Z".to_string()],
                    safe_haven_series: vec!["48.00".to_string()],
                    active_protocol_series: vec!["2.00".to_string()],
                    tranche_1_nock: vec!["4000".to_string()],
                    tranche_2_nock: vec!["6000".to_string()],
                    tranche_3_nock: vec!["96004".to_string()],
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
    fn test_v4_liquidity_scaling() {
        // Test that liquidity scaling math works for tranche 1 position sizing
        let v4_liquidity_val = 81297590130616u128;
        let v4_scale = (v4_liquidity_val as f64) / 65719361234409.0;
        let baseline_nock = 1.4820f64;
        let baseline_usdc = 0.0245f64;

        let scaled_nock = baseline_nock * v4_scale;
        let scaled_usdc = baseline_usdc * v4_scale;

        assert!((v4_scale - 1.23704).abs() < 1e-4);
        assert!((scaled_nock - 1.8333).abs() < 1e-4);
        assert!((scaled_usdc - 0.0303).abs() < 1e-4);

        // Note: Uniswap V4 fees are now queried live via tokensOwed0/1 from positions()
        // instead of using scaled baselines. See run_ingestion_cycle for live fee query.
    }

    #[test]
    fn test_app_config_v4_parameters() {
        let yaml = r#"
database_url: "postgresql://localhost/test"
alchemy_api_keys: ["key1"]
tracked_wallets: ["0x123"]
tracked_pools: ["0x456"]
token_addresses: ["0x789"]
tracked_ranges: []
fee_contracts: []
lp_tokens: []
sync_window_seconds: 60
v4_position_id: 3126000
v4_fee_baseline_usdc: 0.0245
v4_fee_baseline_nock: 1.4820
"#;
        let cfg: AppConfig = serde_yaml::from_str(yaml).expect("Failed to deserialize yaml");
        assert_eq!(cfg.v4_position_id, Some(3126000));
        assert_eq!(cfg.v4_fee_baseline_usdc, Some(0.0245));
        assert_eq!(cfg.v4_fee_baseline_nock, Some(1.4820));
        assert_eq!(cfg.mcp_bearer_token, None);
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
}