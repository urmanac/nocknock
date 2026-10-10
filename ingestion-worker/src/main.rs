use anyhow::Result;
use axum::{
    extract::Extension,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, FixedOffset, Utc};
use config::Config;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{types::Json as PgJson, PgPool, postgres::PgRow};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use ulid::Ulid;
use uuid::Uuid;

// ============================================================
// Configuration
// ============================================================

#[derive(Deserialize, Debug)]
struct AppConfig {
    database_url: String,
    alchemy_api_keys: Vec<String>,
    tracked_wallets: Vec<String>,
    tracked_pools: Vec<String>,
    token_addresses: Vec<String>,
    tracked_ranges: Vec<String>,
    fee_contracts: Vec<String>,
    lp_tokens: Vec<String>,
    sync_window_seconds: u64,
    port: u16,
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
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct AssetAggregationSeries {
    time_points: Vec<String>,
    safe_haven_series: Vec<String>,
    active_protocol_series: Vec<String>,
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
}

// ============================================================
// Main
// ============================================================

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Load config
    let config = Config::builder()
        .add_source(config::File::with_name("config"))
        .add_source(config::Environment::with_prefix("APP"))
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

    let state = Arc::new(AppState {
        db: db.clone(),
        http_client,
        config: Arc::new(config),
        latest_snapshot: Mutex::new(None),
    });

    // Start ingestion scheduler
    let state_clone = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_secs(
                state_clone.config.sync_window_seconds as u64,
            ),
        );
        interval.tick().await; // Skip first immediate tick
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
        .route(
            "/v1/snapshot/latest",
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
        .layer(Extension(state.clone()));

    let port = state.config.port;
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await?;
    println!(
        "🚀 NockNock ingestion worker listening on http://{}",
        listener.local_addr()?
    );
    axum::serve(listener, app).await?;

    Ok(())
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

    Json(json!({
        "status": status,
        "last_sync": last_sync.map(|dt| dt.to_rfc3339()),
        "ingestion_lag_seconds": ingestion_lag_seconds,
    }))
}

async fn get_latest_snapshot_handler(
    Extension(state): Extension<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let latest = state.latest_snapshot.lock().await;

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
        let cache_control = format!(
            "public, max-age={}, stale-while-revalidate={}",
            state.config.sync_window_seconds,
            state.config.sync_window_seconds * 2
        );
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
    let interval = params
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
        .filter_map(|r| serde_json::from_value(r.data).ok())
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

// ============================================================
// Ingestion Logic
// ============================================================

async fn run_ingestion_cycle(state: &Arc<AppState>) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    tracing::info!("Starting ingestion cycle at {now}");

    // Fetch raw data from Alchemy (simplified - real impl would parallelize)
    let raw_data = fetch_raw_data(state).await?;

    // Compute derived metrics
    let derived_metrics = compute_derived_metrics(&raw_data);

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
    let snapshot_id = envelope.meta.snapshot_id.clone();
    let state_hash = envelope.meta.state_hash.clone();

    // Store in database
    store_snapshot(&state.db, &envelope).await?;

    // Update in-memory cache
    *state.latest_snapshot.lock().await = Some(envelope);

    tracing::info!(
        "Ingestion complete: snapshot_id={}, hash={}",
        snapshot_id,
        &state_hash[..16]
    );

    Ok(())
}

async fn fetch_raw_data(state: &Arc<AppState>) -> Result<RawOnChainData> {
    // For MVP, return placeholder data
    // Real implementation would:
    // 1. Call Alchemy for wallet balances
    // 2. Call eth_call for pool states
    // 3. Call subgraphs for concentrated liquidity
    // 4. Call for fee accrual states
    Ok(RawOnChainData {
        wallet_balances: vec![WalletBalance {
            wallet_address: state.config.tracked_wallets[0].clone(),
            token_symbol: "NOCK".to_string(),
            token_address: "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722"
                .to_string(),
            amount: "96004520000000000".to_string(), // 96.00452 NOCK (16 decimals)
            amount_usd: "2450.12".to_string(),
            updated_at: Utc::now().to_rfc3339(),
        }],
        pool_mechanics: vec![PoolMechanic {
            pool_id: "0x85f1aa3a70fedd1c52705c15baed143e675cd626"
                .to_string(),
            platform: "Aerodrome".to_string(),
            pool_type: "volatile".to_string(),
            volatile_assets: vec![
                PoolAsset {
                    token_address: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
                        .to_string(),
                    token_symbol: "USDC".to_string(),
                    reserve_amount: "634533080000".to_string(), // 634,533.08 USDC (6 decimals)
                    reserve_usd: "634533.08".to_string(),
                },
                PoolAsset {
                    token_address: "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722"
                        .to_string(),
                    token_symbol: "NOCK".to_string(),
                    reserve_amount: "248238046900000000".to_string(), // 24,823,804.69 NOCK (16 decimals)
                    reserve_usd: "634533.08".to_string(),
                },
            ],
            liquidity_usd: "1269066.16".to_string(),
            fee_accrued_usd: "12.34".to_string(),
            fee_tier_bps: 25,
            tick_current: Some(-198000),
            sqrt_price_x96: Some("0x034f8b5c9e1a2d3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c".to_string()),
        }],
        concentrated_liquidity: vec![],
        contract_fees: vec![],
    })
}

fn compute_derived_metrics(_raw: &RawOnChainData) -> DerivedMetrics {
    // For MVP, return placeholder derived metrics
    // Real implementation would compute all 7 storyboard components
    DerivedMetrics {
        global_status: GlobalStatus {
            total_portfolio_usd: "2462.46".to_string(),
            total_native_usd: "0.00".to_string(),
            total_stable_usd: "2450.12".to_string(),
            total_fees_accrued_usd: "12.34".to_string(),
            total_fees_claimable_usd: "10.49".to_string(),
            gas_to_fee_delta_usd: "10.49".to_string(),
            high_water_mark_usd: "2500.00".to_string(),
            high_water_mark_date: Utc::now().to_rfc3339(),
        },
        asset_aggregation_series: AssetAggregationSeries {
            time_points: vec![Utc::now().to_rfc3339()],
            safe_haven_series: vec!["0.00".to_string()],
            active_protocol_series: vec!["2450.12".to_string()],
        },
        total_value_vs_baseline: TotalValueVsBaseline {
            time_points: vec![Utc::now().to_rfc3339()],
            portfolio_value_series: vec!["2462.46".to_string()],
            token_price_series: vec!["0.0255".to_string()],
            high_water_mark_series: vec!["2500.00".to_string()],
        },
        tick_bracket_hypervisor: TickBracketHypervisor {
            current_price: "0.0255".to_string(),
            brackets: vec![TickBracket {
                lower_price: "0.0250".to_string(),
                upper_price: "0.0260".to_string(),
                liquidity_usd: "500000.00".to_string(),
                in_range: true,
                distance_bps: -200,
            }],
        },
        portfolio_deviation_matrix: PortfolioDeviationMatrix {
            allocations: vec![AllocationDeviation {
                bucket_name: "Aerodrome NOCK/USDC".to_string(),
                target_weight_bps: 5000, // 50%
                actual_weight_bps: 4800, // 48%
                deviation_bps: -200,
                status: "under_funded".to_string(),
            }],
        },
        structural_channels: StructuralChannels { channels: vec![] },
        strategy_counterfactual: StrategyCounterfactual {
            time_points: vec![Utc::now().to_rfc3339()],
            active_portfolio_series: vec!["2462.46".to_string()],
            passive_baseline_series: vec!["2400.00".to_string()],
        },
        velocity_gas_sandbox: VelocityGasSandbox {
            contracts: vec![FeeCollectionIndex {
                contract_address: "0x85f1aa3a70fedd1c52705c15baed143e675cd626"
                    .to_string(),
                contract_name: "Aerodrome NOCK/USDC".to_string(),
                claimable_usd: "10.49".to_string(),
                gas_break_even_usd: "5.23".to_string(),
                efficiency_ratio: 2.01,
                recommended_action: "claim_now".to_string(),
            }],
        },
    }
}

fn compute_state_hash(data: &SnapshotData) -> String {
    // Canonical JSON: sorted keys, no whitespace
    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_vec(&data).unwrap());
    format!("{:x}", hasher.finalize())
}

async fn store_snapshot(db: &PgPool, envelope: &SnapshotEnvelope) -> Result<()> {
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