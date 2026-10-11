//! NockNock Ingestion Brain — Pure WASM Logic
//!
//! Implements `nocknock:core/ingestion` WIT interface.
//! No HTTP, no DB, no system time — all provided by host via imports.

#![allow(unused, dead_code, static_mut_refs)]

use extism_pdk::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// ============================================================
// Host Imports (provided by ingestion-worker at runtime)
// ============================================================

#[allow(improper_ctypes)]
extern "C" {
    // Logger imports
    #[link_name = "logger:log"]
    fn host_log(msg: *const u8, len: usize);
    #[link_name = "logger:log-debug"]
    fn host_log_debug(msg: *const u8, len: usize);

    // HTTP imports (for fetching on-chain data)
    #[link_name = "http:get"]
    fn host_http_get(url: *const u8, len: usize) -> i32; // returns pointer to result
    #[link_name = "http:post"]
    fn host_http_post(url: *const u8, len: usize, body: *const u8, body_len: usize) -> i32;

    // Storage imports (for caching raw responses)
    #[link_name = "storage:read"]
    fn host_storage_read(key: *const u8, len: usize) -> i32;
    #[link_name = "storage:write"]
    fn host_storage_write(key: *const u8, len: usize, value: *const u8, value_len: usize);
    #[link_name = "storage:list-keys"]
    fn host_storage_list_keys(prefix: *const u8, len: usize) -> i32;
}

// Helper to call host functions safely
fn log(msg: &str) {
    unsafe { host_log(msg.as_ptr(), msg.len()) }
}
fn log_debug(msg: &str) {
    unsafe { host_log_debug(msg.as_ptr(), msg.len()) }
}

// ============================================================
// Input/Output Types (matching snapshot.yaml schemas)
// ============================================================

#[derive(Deserialize, Serialize, Debug)]
struct IngestionConfig {
    alchemy_api_keys: Vec<String>,
    tracked_wallets: Vec<String>,
    tracked_pools: Vec<String>,
    token_addresses: Vec<String>,
    tracked_ranges: Vec<String>,
    fee_contracts: Vec<String>,
    lp_tokens: Vec<String>,
    #[serde(default = "default_sync_window")]
    sync_window_seconds: u64,
}
fn default_sync_window() -> u64 { 300 }

#[derive(Deserialize, Serialize, Debug)]
struct ComputeSnapshotInput {
    config: IngestionConfig,
    now: String, // ISO8601
}

#[derive(Deserialize, Serialize, Debug)]
struct FreshnessInput {
    snapshot_id: String,
    max_age_seconds: u64,
    now: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct FreshnessResult {
    is_fresh: bool,
    age_seconds: u64,
    state_hash: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct RawOnChainData {
    wallet_balances_raw: serde_json::Value,
    pool_states_raw: serde_json::Value,
    range_states_raw: serde_json::Value,
    fee_states_raw: serde_json::Value,
}

#[derive(Deserialize, Serialize, Debug)]
struct SnapshotEnvelope {
    meta: SnapshotMeta,
    data: SnapshotData,
}

#[derive(Deserialize, Serialize, Debug)]
struct SnapshotMeta {
    generated_at: String,
    state_hash: String,
    version: String,
    snapshot_id: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct SnapshotData {
    wallet_balances: Vec<WalletBalance>,
    pool_mechanics: Vec<PoolMechanic>,
    concentrated_liquidity: Vec<ConcentratedLiquidity>,
    contract_fees: Vec<ContractFee>,
    derived_metrics: DerivedMetrics,
}

// Minimal inline types — full definitions in snapshot.yaml
#[derive(Deserialize, Serialize, Debug, Clone)]
struct WalletBalance {
    wallet_address: String,
    token_symbol: String,
    token_address: String,
    amount: String,
    amount_usd: String,
    updated_at: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
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

#[derive(Deserialize, Serialize, Debug, Clone)]
struct PoolAsset {
    token_address: String,
    token_symbol: String,
    reserve_amount: String,
    reserve_usd: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
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

#[derive(Deserialize, Serialize, Debug, Clone)]
struct ContractFee {
    contract_address: String,
    fee_token_address: String,
    fee_token_symbol: String,
    amount: String,
    amount_usd: String,
    claimable: bool,
    gas_break_even_usd: String,
}

// Derived metrics — the "Storyboard Visual Layout" pre-computed
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
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

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
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

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
struct AssetAggregationSeries {
    time_points: Vec<String>,
    safe_haven_series: Vec<String>,
    active_protocol_series: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
struct TotalValueVsBaseline {
    time_points: Vec<String>,
    portfolio_value_series: Vec<String>,
    token_price_series: Vec<String>,
    high_water_mark_series: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
struct TickBracketHypervisor {
    current_price: String,
    brackets: Vec<TickBracket>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct TickBracket {
    lower_price: String,
    upper_price: String,
    liquidity_usd: String,
    in_range: bool,
    distance_bps: i32,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
struct PortfolioDeviationMatrix {
    allocations: Vec<AllocationDeviation>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct AllocationDeviation {
    bucket_name: String,
    target_weight_bps: i32,
    actual_weight_bps: i32,
    deviation_bps: i32,
    status: String, // on_target, over_funded, under_funded, fast_running
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
struct StructuralChannels {
    channels: Vec<RegressionChannel>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
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

#[derive(Deserialize, Serialize, Debug, Clone)]
struct LinePoint {
    time: String,
    value: String,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
struct StrategyCounterfactual {
    time_points: Vec<String>,
    active_portfolio_series: Vec<String>,
    passive_baseline_series: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
struct VelocityGasSandbox {
    contracts: Vec<FeeCollectionIndex>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct FeeCollectionIndex {
    contract_address: String,
    contract_name: String,
    claimable_usd: String,
    gas_break_even_usd: String,
    efficiency_ratio: f64,
    recommended_action: String, // claim_now, wait, not_worthwhile
}

// ============================================================
// Helper: Deterministic State Hash
// ============================================================

fn compute_state_hash(data: &SnapshotData) -> String {
    // Canonical JSON: sorted keys, no whitespace, deterministic ordering
    let canonical = serde_json::to_vec(data).expect("serialization cannot fail");
    let mut hasher = Sha256::new();
    hasher.update(&canonical);
    format!("{:x}", hasher.finalize())
}

// ============================================================
// WASM Exports (matching ingestion.wit)
// ============================================================

#[no_mangle]
pub extern "C" fn compute_snapshot(input_ptr: i32, input_len: i32) -> i32 {
    let input_bytes = unsafe { std::slice::from_raw_parts(input_ptr as *const u8, input_len as usize) };
    let input_str = match std::str::from_utf8(input_bytes) {
        Ok(s) => s,
        Err(_) => return write_error("Invalid UTF-8 in input"),
    };

    let input: ComputeSnapshotInput = match serde_json::from_str(input_str) {
        Ok(v) => v,
        Err(e) => return write_error(&format!("Failed to parse input: {}", e)),
    };

    log_debug(&format!("compute_snapshot: now={}", input.now));

    // 1. Fetch raw on-chain data (via host HTTP)
    let raw_data = match fetch_raw_data(&input.config) {
        Ok(d) => d,
        Err(e) => return write_error(&format!("Fetch failed: {}", e)),
    };

    // 2. Derive metrics from raw data
    let derived = compute_derived_metrics(&raw_data);

    // 3. Build snapshot data (raw + derived)
    // Transform raw JSON values to structured types (placeholder implementations)
    let wallet_balances = transform_wallet_balances(&raw_data.wallet_balances_raw);
    let pool_mechanics = transform_pool_mechanics(&raw_data.pool_states_raw);
    let concentrated_liquidity = transform_concentrated_liquidity(&raw_data.range_states_raw);
    let contract_fees = transform_contract_fees(&raw_data.fee_states_raw);

    let snapshot_data = SnapshotData {
        wallet_balances,
        pool_mechanics,
        concentrated_liquidity,
        contract_fees,
        derived_metrics: derived,
    };

    // 4. Compute deterministic hash
    let state_hash = compute_state_hash(&snapshot_data);

    // 5. Build envelope
    let envelope = SnapshotEnvelope {
        meta: SnapshotMeta {
            generated_at: input.now.clone(),
            state_hash: state_hash.clone(),
            version: "1.0.0".to_string(),
            snapshot_id: ulid::Ulid::new().to_string(),
        },
        data: snapshot_data,
    };

    write_json(&envelope)
}

#[no_mangle]
pub extern "C" fn check_freshness(input_ptr: i32, input_len: i32) -> i32 {
    let input_bytes = unsafe { std::slice::from_raw_parts(input_ptr as *const u8, input_len as usize) };
    let input_str = match std::str::from_utf8(input_bytes) {
        Ok(s) => s,
        Err(_) => return write_error("Invalid UTF-8 in input"),
    };

    let input: FreshnessInput = match serde_json::from_str(input_str) {
        Ok(v) => v,
        Err(e) => return write_error(&format!("Failed to parse input: {}", e)),
    };

    // Host should have stored the snapshot; we read it from storage
    let stored = read_snapshot_from_storage(&input.snapshot_id);
    let (generated_at, state_hash) = match stored {
        Some((ts, hash)) => (ts, hash),
        None => return write_error("Snapshot not found"),
    };

    let now = chrono::DateTime::parse_from_rfc3339(&input.now).unwrap_or_else(|_| chrono::Utc::now().into());
    let generated = chrono::DateTime::parse_from_rfc3339(&generated_at).unwrap_or_else(|_| chrono::Utc::now().into());
    let age_seconds = (now - generated).num_seconds().max(0) as u64;

    let result = FreshnessResult {
        is_fresh: age_seconds <= input.max_age_seconds,
        age_seconds,
        state_hash,
    };

    write_json(&result)
}

#[no_mangle]
pub extern "C" fn derive_metrics(input_ptr: i32, input_len: i32) -> i32 {
    let input_bytes = unsafe { std::slice::from_raw_parts(input_ptr as *const u8, input_len as usize) };
    let input_str = match std::str::from_utf8(input_bytes) {
        Ok(s) => s,
        Err(_) => return write_error("Invalid UTF-8 in input"),
    };

    let raw: RawOnChainData = match serde_json::from_str(input_str) {
        Ok(v) => v,
        Err(e) => return write_error(&format!("Failed to parse input: {}", e)),
    };

    let derived = compute_derived_metrics(&raw);
    write_json(&derived)
}

// ============================================================
// Internal Implementation
// ============================================================

fn fetch_raw_data(config: &IngestionConfig) -> Result<RawOnChainData, String> {
    log_debug("fetch_raw_data: querying Alchemy...");
    // TODO: Implement parallel HTTP calls to Alchemy for:
    // - getTokenBalances for each wallet
    // - pool state via eth_call / subgraph
    // - concentrated liquidity positions
    // - fee accrual state
    // For now, return empty placeholder
    Ok(RawOnChainData {
        wallet_balances_raw: serde_json::json!([]),
        pool_states_raw: serde_json::json!([]),
        range_states_raw: serde_json::json!([]),
        fee_states_raw: serde_json::json!([]),
    })
}

fn compute_derived_metrics(raw: &RawOnChainData) -> DerivedMetrics {
    log_debug("compute_derived_metrics: computing aggregates...");
    // TODO: Implement all derived metric computations:
    // - Global status (totals, high-water mark)
    // - Asset aggregation series (historical safe vs active)
    // - Total value vs baseline (dual axis)
    // - Tick bracket hypervisor (current price vs range bounds)
    // - Portfolio deviation matrix (target vs actual weights)
    // - Structural channels (regression corridors)
    // - Strategy counterfactual (active vs passive)
    // - Velocity gas sandbox (claimable vs gas breakeven)
    DerivedMetrics::default()
}

fn transform_wallet_balances(raw: &serde_json::Value) -> Vec<WalletBalance> {
    // TODO: Parse Alchemy getTokenBalances response
    vec![]
}

fn transform_pool_mechanics(raw: &serde_json::Value) -> Vec<PoolMechanic> {
    // TODO: Parse pool state from subgraph/eth_call
    vec![]
}

fn transform_concentrated_liquidity(raw: &serde_json::Value) -> Vec<ConcentratedLiquidity> {
    // TODO: Parse Uniswap V4 position state
    vec![]
}

fn transform_contract_fees(raw: &serde_json::Value) -> Vec<ContractFee> {
    // TODO: Parse fee accrual state
    vec![]
}

fn read_snapshot_from_storage(snapshot_id: &str) -> Option<(String, String)> {
    // Host storage read — returns (generated_at, state_hash)
    // Placeholder: in real impl, call host_storage_read
    None
}

// ============================================================
// Output Helpers
// ============================================================

static mut OUTPUT_BUFFER: [u8; 1048576] = [0; 1048576]; // 1MB

fn write_json<T: Serialize>(value: &T) -> i32 {
    let json = match serde_json::to_string(value) {
        Ok(s) => s,
        Err(_) => return write_error("Failed to serialize output"),
    };
    write_result(&json)
}

fn write_error(msg: &str) -> i32 {
    let err = format!(r#"{{"error":"{}"}}"#, msg.replace('"', "\\\""));
    write_result(&err)
}

fn write_result(s: &str) -> i32 {
    let bytes = s.as_bytes();
    if bytes.len() >= 1048575 {
        let err = r#"{"error":"WASM output buffer overflow"}"#.as_bytes();
        unsafe { OUTPUT_BUFFER[..err.len()].copy_from_slice(err); OUTPUT_BUFFER[err.len()] = 0; }
        return unsafe { OUTPUT_BUFFER.as_ptr() as i32 };
    }
    unsafe {
        OUTPUT_BUFFER[..bytes.len()].copy_from_slice(bytes);
        OUTPUT_BUFFER[bytes.len()] = 0;
        OUTPUT_BUFFER.as_ptr() as i32
    }
}

// ============================================================
// Tests (run with `cargo test`)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_hash_determinism() {
        let data1 = SnapshotData {
            wallet_balances: vec![
                WalletBalance {
                    wallet_address: "0x123".to_string(),
                    token_symbol: "NOCK".to_string(),
                    token_address: "0x0".to_string(),
                    amount: "100".to_string(),
                    amount_usd: "50".to_string(),
                    updated_at: "2026-01-01T00:00:00Z".to_string(),
                }
            ],
            pool_mechanics: vec![],
            concentrated_liquidity: vec![],
            contract_fees: vec![],
            derived_metrics: DerivedMetrics::default(),
        };

        let data2 = SnapshotData {
            wallet_balances: vec![
                WalletBalance {
                    wallet_address: "0x123".to_string(),
                    token_symbol: "NOCK".to_string(),
                    token_address: "0x0".to_string(),
                    amount: "100".to_string(),
                    amount_usd: "50".to_string(),
                    updated_at: "2026-01-01T00:00:00Z".to_string(),
                }
            ],
            pool_mechanics: vec![],
            concentrated_liquidity: vec![],
            contract_fees: vec![],
            derived_metrics: DerivedMetrics::default(),
        };

        assert_eq!(compute_state_hash(&data1), compute_state_hash(&data2));
    }

    #[test]
    fn test_state_hash_changes_on_data_change() {
        let mut data1 = SnapshotData {
            wallet_balances: vec![
                WalletBalance {
                    wallet_address: "0x123".to_string(),
                    token_symbol: "NOCK".to_string(),
                    token_address: "0x0".to_string(),
                    amount: "100".to_string(),
                    amount_usd: "50".to_string(),
                    updated_at: "2026-01-01T00:00:00Z".to_string(),
                }
            ],
            pool_mechanics: vec![],
            concentrated_liquidity: vec![],
            contract_fees: vec![],
            derived_metrics: DerivedMetrics::default(),
        };

        let mut data2 = data1.clone();
        data2.wallet_balances[0].amount = "200".to_string();

        assert_ne!(compute_state_hash(&data1), compute_state_hash(&data2));
    }

    #[test]
    fn test_freshness_logic() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-10-09T17:23:29Z").unwrap();
        let generated = chrono::DateTime::parse_from_rfc3339("2026-10-09T17:20:00Z").unwrap();
        let age = (now - generated).num_seconds() as u64; // 209 seconds
        assert!(age < 300); // fresh within 5 min window

        let generated_old = chrono::DateTime::parse_from_rfc3339("2026-10-09T17:10:00Z").unwrap();
        let age_old = (now - generated_old).num_seconds() as u64; // 809 seconds
        assert!(age_old > 300); // stale
    }
}