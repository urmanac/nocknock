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
mod evm;
use evm::*;
mod calldata;
use calldata::*;
mod contracts;
use contracts::*;
mod mcp_text;
use mcp_text::*;
mod plan;
use plan::*;
mod market;
use market::*;
mod notify;
use notify::*;
mod rpc;
use rpc::*;
mod adapters;
use adapters::*;

// ============================================================
// Configuration
// ============================================================

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OperatorProfile {
    #[serde(default = "default_treasury_floor")]
    pub treasury_floor_nock: f64,
    /// How far below the floor the stack may fall if every LP range sold all its NOCK,
    /// in basis points of the floor (operator decision, 2026-11-14). A range sells
    /// gradually as price rises, so the operator accepts placing a little more NOCK in
    /// LP than sits above the floor, to trade more and earn fees. 0 = none.
    #[serde(default)]
    pub lp_floor_tolerance_bps: u32,
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

/// No floor unless the operator sets one (v0.5.1). A floor is the operator's own number;
/// 0 means "no floor rule", never someone else's 100,000.
fn default_treasury_floor() -> f64 { 0.0 }
/// No virtual tax reserve unless the operator sets a ratio (v0.5.1, D14): 0 = no rule.
fn default_tax_reserve_ratio() -> f64 { 0.0 }
/// Target weights are the operator's own numbers (v0.5.1, D6): 0 = no target, reported
/// as `no_target` and never used to size anything.
fn default_treasury_bps() -> i32 { 0 }
fn default_uniswap_lp_bps() -> i32 { 0 }
fn default_cash_bps() -> i32 { 0 }
fn default_target_weights() -> TargetWeightsBps {
    // v2.0.0: the Aerodrome tranche was liquidated (2026-10-23); the LP target is now
    // the aggregate across all tracked Uniswap positions. Legacy configs carrying
    // `aerodrome_lp` / `uniswap_v4_lp` keys still parse (unknown fields are ignored).
    TargetWeightsBps {
        treasury: default_treasury_bps(),
        uniswap_lp: default_uniswap_lp_bps(),
        cash: default_cash_bps(),
    }
}

impl Default for OperatorProfile {
    fn default() -> Self {
        Self {
            treasury_floor_nock: default_treasury_floor(),
            lp_floor_tolerance_bps: 0,
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
    /// "uniswap-v4" | "uniswap-v3" | "aerodrome" (volatile x*y=k pool, full range)
    pub protocol: String,
    #[serde(default = "default_network")]
    pub network: String,
    /// Position NFT id (Uniswap). Unused for aerodrome, whose position is an LP-token
    /// balance in `pool_address`.
    #[serde(default)]
    pub nft_id: u64,
    /// aerodrome only: swap fee in basis points, for display (default 30 = 0.30%).
    #[serde(default)]
    pub fee_bps: Option<u32>,
    /// aerodrome only: the pool's gauge. LP tokens staked there (the aero.xyz app stakes
    /// on deposit) are added to the wallet's own balance; without it a staked position
    /// reads as empty.
    #[serde(default)]
    pub gauge_address: Option<String>,
    /// Where the operator adds liquidity to this position, when the default link is
    /// wrong (for example a pool on the new Aero app:
    /// https://app.aero.xyz/deposit/8453-<pool address>).
    #[serde(default)]
    pub deposit_url: Option<String>,
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
    /// Share of the LP target this position should hold when rebuilding, in basis
    /// points. Unset = equal split across every eligible position; 0 = never suggest.
    #[serde(default)]
    pub rebuild_weight_bps: Option<u32>,
    /// Operator rule (2026-11-14): "we should not generally aim to fill positions that
    /// are out of range". Set true to make an exception for this position.
    #[serde(default)]
    pub rebuild_when_out_of_range: bool,
}

fn default_network() -> String {
    "base".to_string()
}

/// Dollar-cost-averaging plan for NOCK accumulation (issue #79). Optional: without
/// this section the advice engine behaves as before. The plan only PACES buys — the
/// operator executes each one and records its transaction hash; nothing here signs
/// or sends a transaction.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AccumulationPlanConfig {
    /// Operator-chosen label; keys the accumulation ledger.
    pub id: String,
    /// Total USDC to deploy over the whole plan.
    pub budget_usdc: f64,
    /// Number of equal tranches the budget is split into.
    pub tranches: u32,
    /// Minimum spacing between buys.
    pub interval_hours: f64,
    /// First tranche is not due before this instant (default: immediately).
    #[serde(default)]
    pub starts_at: Option<DateTime<Utc>>,
    /// Where the operator executes (label only in Phase 1).
    #[serde(default = "default_plan_venue")]
    pub venue: String,
    /// Optional ceiling: tranches pause while spot is above this price.
    #[serde(default)]
    pub max_price_usd: Option<f64>,
    /// Pause while daily RSI(14) >= 70 (default true).
    #[serde(default = "default_true")]
    pub pause_when_overextended: bool,
    /// Never advise spending liquid USDC below `target_weights_bps.cash` (default true).
    #[serde(default = "default_true")]
    pub respect_cash_target: bool,
    /// Protected cash reserve (operator order, 2026-11-14: "we have lots of cash, we
    /// want to keep it"). The plan never advises a buy that would take the stable
    /// balance (wallet USDC plus USDC inside LP; never ETH, which is gas) below this
    /// amount, at any price. It binds only while an accumulation_plan is configured;
    /// with no plan NockNock advises no buys at all. Independent of the long-run cash
    /// target. The user sets it; NockNock never chooses it.
    #[serde(default)]
    pub cash_reserve_usdc: Option<f64>,
}

fn default_plan_venue() -> String {
    // Aerodrome and Velodrome merged into Aero (aero.xyz) on 2026-10-21; see
    // ONCHAIN_INTEGRATION.md §6. The label is informational: receipt verification
    // is contract-agnostic.
    "Aero".to_string()
}

fn default_true() -> bool {
    true
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
    /// Optional dollar-cost-averaging plan (issue #79).
    #[serde(default)]
    accumulation_plan: Option<AccumulationPlanConfig>,
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
/// No default wallet (v0.5.1): an unset `tracked_wallets` is an explicit error at the top
/// of each cycle, never someone else's address.
fn default_tracked_wallets() -> Vec<String> {
    Vec::new()
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
    /// Pool used for third-party OHLCV lookups (RSI history). May be a legacy Aerodrome
    /// pool: see ONCHAIN_INTEGRATION.md §6 for the dependency this creates.
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

/// One cycle's live reads. Built only by fetch_raw_data (and test fixtures); it is never
/// deserialized, so no field carries a serde default that could stand in for a live read.
#[derive(Debug, Default)]
struct RawOnChainData {
    wallet_balances: Vec<WalletBalance>,
    pool_mechanics: Vec<PoolMechanic>,
    concentrated_liquidity: Vec<ConcentratedLiquidity>,
    contract_fees: Vec<ContractFee>,
    /// Per-position on-chain reads (v2.0.0) — replaces live_tranche_1_*/2_* and
    /// the Aerodrome LP-share/claimFees inputs.
    positions: Vec<RawPosition>,
    /// Live NOCK/USD spot from the designated price-source position pool.
    spot_price_nock_usd: f64,
    live_gas_cost_usd: f64,
    /// Live ETH/USD from the configured WETH/USDC pool (stale-pool guarded). 0.0 only in
    /// test fixtures that do not set it: ETH is then unpriced, never priced at a default.
    live_eth_price_usd: f64,
    /// In-range liquidity of each tracked position's pool, keyed by position_id
    /// (decimal string). A missing key means the read failed this cycle: unknown, not zero.
    pool_liquidity: HashMap<String, String>,
    /// Native ETH and WETH held by the wallet: the gas pillar (issue #94). Both reads
    /// are required in production (a failed read fails the cycle); None is reachable only
    /// in test fixtures and makes every gas field null, never zero. Gas is counted in
    /// the portfolio total but is never cash, never dry powder and never funds an LP leg.
    wallet_eth: Option<f64>,
    wallet_weth: Option<f64>,
    /// Per-position rebuild controls from config, keyed by position_id.
    rebuild_rules: HashMap<String, RebuildRule>,
    /// position_ids of Aero positions whose claimFees() reverts: their fees are not
    /// LP-claimable (shown as such, never as $0 earned).
    fees_not_lp_claimable: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
struct RebuildRule {
    weight_bps: Option<u32>,
    when_out_of_range: bool,
    #[serde(default)]
    deposit_url: Option<String>,
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
    /// False when the position's pool has no in-range liquidity: its price is frozen,
    /// so `in_range` (judged against that pool's own tick) says nothing about the
    /// market. Null when the pool read failed this cycle. (2.2.0)
    #[serde(default)]
    pool_active: Option<bool>,
    /// True for a position that spans every price (an Aerodrome volatile pool): it has
    /// no range to leave, and lower_price/upper_price are not meaningful. (2.2.0)
    #[serde(default)]
    full_range: bool,
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
    /// Dollar-cost-averaging plan status (snapshot 2.1.0); null when no plan is configured.
    #[serde(default)]
    accumulation_plan: Option<AccumulationPlanStatus>,
    /// Standing LP-rebuild bias (snapshot 2.1.0).
    #[serde(default)]
    lp_rebuild: LpRebuild,
}

/// Plan progress as emitted in snapshots. Everything here derives from the plan
/// config plus receipt-verified rows in `accumulation_events` (zero-mock).
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
struct AccumulationPlanStatus {
    plan_id: String,
    /// pending | tranche_due | waiting | paused | blocked | complete | invalid
    status: String,
    venue: String,
    /// 1-based index of the NEXT tranche (verified fills + 1).
    tranche_index: u32,
    tranches_total: u32,
    /// Amount advised for the next tranche after all guardrails ("0.00" unless due).
    tranche_usdc: String,
    /// budget / tranches, before guardrails.
    nominal_tranche_usdc: String,
    next_due_at: Option<String>,
    budget_usdc: String,
    spent_usdc: String,
    remaining_usdc: String,
    acquired_nock: String,
    fills: i64,
    last_fill_at: Option<String>,
    /// Σ USDC spent / Σ NOCK received across verified fills; null before the first fill.
    avg_price_usd: Option<String>,
    /// NOCK the whole budget would have bought at the first fill's price — the
    /// "bought it all on day one" counterfactual; null before the first fill.
    lump_sum_nock: Option<String>,
    /// Plain-words reason when paused / blocked / invalid, or the limiting guardrail
    /// when a due tranche was sized below nominal.
    binding_constraint: Option<String>,
    /// Protected cash reserve the plan never draws below; null when none is set.
    #[serde(default)]
    cash_reserve_usdc: Option<String>,
}

/// Standing bias toward re-establishing LP ranges. Moving NOCK and USDC the wallet
/// already holds into positions is a reposition, not a purchase: the stable total
/// (`total_stable_usd` = wallet USDC + USDC inside positions) does not change.
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
struct LpRebuild {
    /// under_target | on_target | over_target
    status: String,
    target_usd: String,
    actual_usd: String,
    /// target − actual, floored at zero.
    gap_usd: String,
    target_weight_bps: i32,
    /// Concrete deposits that would close the gap, one per eligible position (2.2.0).
    #[serde(default)]
    suggestions: Vec<LpSuggestion>,
    /// Tracked positions deliberately left out, with the reason (2.2.0).
    #[serde(default)]
    excluded: Vec<LpExclusion>,
    /// NOCK the operator tolerates below the floor in a full LP sell-out (2.2.0).
    #[serde(default)]
    floor_tolerance_nock: Option<String>,
    /// Total NOCK left if every range, existing and suggested, sold all its NOCK (2.2.0).
    #[serde(default)]
    worst_case_nock: Option<String>,
}

/// One suggested deposit into an existing tracked position. Amounts are what the
/// position's range takes at the pool's current price for `add_usd` of value; the
/// operator makes the deposit in the Uniswap app, and the worker sees it on-chain.
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
struct LpSuggestion {
    position_id: String,
    nft_id: u64,
    protocol: String,
    label: String,
    /// This position's share of the LP target, and what it holds now.
    target_usd: String,
    current_usd: String,
    add_usd: String,
    token0_symbol: String,
    amount0: String,
    token1_symbol: String,
    amount1: String,
    /// Guardrail that shrank the suggestion below its target, in plain words.
    limited_by: Option<String>,
    /// Price multiple (NOCK against the pool's other token) at which this range has
    /// sold ALL of its NOCK: 1.44 means a 44% rise empties it. Selling is gradual on
    /// the way there. Null when the range holds no NOCK at the current price.
    #[serde(default)]
    nock_sold_out_at_multiple: Option<String>,
    /// Where to act: the position's page in the Uniswap app.
    url: String,
}

/// How far NOCK must rise against the pool's other token before the range has sold all
/// of its NOCK. NOCK as token1 is exhausted at the lower tick, as token0 at the upper.
/// Pure.
fn nock_sell_out_multiple(p: &RawPosition) -> Option<f64> {
    if is_full_range(p.tick_lower, p.tick_upper) {
        return None; // sells gradually forever; never empties
    }
    let ticks = if is_nock_token(&p.token1) {
        p.tick - p.tick_lower
    } else if is_nock_token(&p.token0) {
        p.tick_upper - p.tick
    } else {
        return None;
    };
    if ticks <= 0 {
        return None; // already past the edge: the range holds no NOCK
    }
    Some(1.0001f64.powi(ticks))
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
struct LpExclusion {
    position_id: String,
    nft_id: u64,
    label: String,
    reason: String,
}

/// What the wallet can put into LP without breaking another rule.
#[derive(Debug, Default, Clone, Copy)]
struct RebuildFunds {
    /// NOCK that may go into LP: what sits above the floor plus the tolerated overage.
    nock: f64,
    /// The tolerated overage included in `nock` (for the explanation text).
    floor_tolerance_nock: f64,
    /// Whether the operator set a floor. Without one, `nock` is the wallet stack and the
    /// explanation must not name a floor rule that does not exist.
    floor_set: bool,
    /// Liquid USDC above what must stay in the wallet (cash target, tax reserve and
    /// the accumulation plan's remaining tranches).
    usdc: f64,
}

/// Smallest deposit worth suggesting. Below this a suggestion is noise (a $2 top-up
/// costs attention and gas for nothing) and the position is listed as not fundable.
const LP_SUGGESTION_MIN_USD: f64 = 5.0;

/// Why a position with an ETH or WETH leg is never filled (issue #94): wallet ETH and
/// WETH are gas, counted in the total but never deployed. Its share goes to the others.
const LP_GAS_LEG_REASON: &str =
    "needs ETH/WETH, which is gas, not investable capital: NockNock never funds an ETH leg from wallet gas, so this position's share goes to the other eligible positions";

/// Human amounts of token0/token1 that one USD of value buys in this position's range
/// at the pool's current price. None when the range has no value at this price or a
/// leg cannot be priced. Pure.
fn amounts_per_usd(p: &RawPosition, nock_usd: f64, eth_usd: f64) -> Option<(f64, f64)> {
    // Any liquidity works as the probe: amounts are linear in L.
    let (r0, r1) = concentrated_amounts_raw(1_000_000_000_000_000_000, p.sqrt_price_x96, p.tick_lower, p.tick_upper);
    let a0 = r0 / 10f64.powi(p.token0.decimals);
    let a1 = r1 / 10f64.powi(p.token1.decimals);
    let p0 = leg_usd_price(&p.token0, nock_usd, eth_usd);
    let p1 = leg_usd_price(&p.token1, nock_usd, eth_usd);
    if (a0 > 0.0 && p0 <= 0.0) || (a1 > 0.0 && p1 <= 0.0) {
        return None;
    }
    let value = a0 * p0 + a1 * p1;
    if !(value > 0.0) || !value.is_finite() {
        return None;
    }
    Some((a0 / value, a1 / value))
}

/// Size the LP rebuild: split the LP target across eligible tracked positions and work
/// out what each deposit takes. Pure — no I/O, unit-testable.
///
/// Eligibility: the pool must have in-range liquidity (a dead pool's price is frozen,
/// so a deposit there would sit alone at a stale price), and the position must be in
/// range unless its config makes an exception, and it must not need ETH/WETH: wallet
/// ETH is gas, not investable capital (issue #94), so a position with an ETH or WETH
/// leg is excluded and its share is redistributed to the eligible positions. Funding:
/// a suggestion is scaled down, never up, to what the wallet can supply; the limit is
/// named.
fn lp_rebuild_suggestions(
    raw: &RawOnChainData,
    lp_target_usd: f64,
    funds: RebuildFunds,
) -> (Vec<LpSuggestion>, Vec<LpExclusion>) {
    let nock_usd = raw.spot_price_nock_usd;
    let eth_usd = raw.live_eth_price_usd;
    let mut excluded = Vec::new();
    let mut eligible: Vec<(&RawPosition, Option<u32>)> = Vec::new();
    for p in &raw.positions {
        let rule = raw.rebuild_rules.get(&p.position_id).cloned().unwrap_or_default();
        let mut skip = |reason: &str| {
            excluded.push(LpExclusion {
                position_id: p.position_id.clone(),
                nft_id: p.nft_id,
                label: position_label(p),
                reason: reason.to_string(),
            })
        };
        if rule.weight_bps == Some(0) {
            skip("excluded in config (rebuild_weight_bps: 0)");
            continue;
        }
        match raw.pool_liquidity.get(&p.position_id).and_then(|v| v.parse::<u128>().ok()) {
            None => {
                skip("pool liquidity could not be read this cycle");
                continue;
            }
            Some(0) => {
                skip("pool has no in-range liquidity, so its price is frozen; a deposit would sit alone at a stale price");
                continue;
            }
            Some(_) => {}
        }
        if !p.in_range && !rule.when_out_of_range {
            skip("position is out of range (out-of-range positions are not filled by default)");
            continue;
        }
        if is_eth_token(&p.token0) || is_eth_token(&p.token1) {
            skip(LP_GAS_LEG_REASON);
            continue;
        }
        eligible.push((p, rule.weight_bps));
    }

    // Weights: explicit bps where given; positions without one share what is left equally.
    // When every eligible position has an explicit weight and they sum to less than the
    // whole, they are scaled up in proportion: the share of an excluded position (an
    // ETH/WETH one, a dead pool) goes to the eligible ones, so the LP target can be met.
    let explicit: u32 = eligible.iter().filter_map(|(_, w)| *w).sum();
    let unweighted = eligible.iter().filter(|(_, w)| w.is_none()).count() as f64;
    let leftover = (10000.0 - explicit as f64).max(0.0);
    let explicit_scale = if unweighted == 0.0 && explicit > 0 && explicit < 10000 {
        10000.0 / explicit as f64
    } else {
        1.0
    };

    // Pass 1: what each position wants, and what that takes of each token.
    struct Want<'a> {
        p: &'a RawPosition,
        target_usd: f64,
        current_usd: f64,
        want_usd: f64,
        per0: f64,
        per1: f64,
    }
    let mut wants: Vec<Want> = Vec::new();
    for (p, w) in eligible {
        let weight = match w {
            Some(bps) => bps as f64 * explicit_scale,
            None => leftover / unweighted,
        } / 10000.0;
        let p0 = leg_usd_price(&p.token0, nock_usd, eth_usd);
        let p1 = leg_usd_price(&p.token1, nock_usd, eth_usd);
        let current_usd = p.amount0 * p0 + p.amount1 * p1;
        let target_usd = lp_target_usd * weight;
        let want_usd = target_usd - current_usd;
        if want_usd < LP_SUGGESTION_MIN_USD {
            continue; // at, above or within dust of its share: never suggest unwinding
        }
        match amounts_per_usd(p, nock_usd, eth_usd) {
            Some((per0, per1)) => wants.push(Want { p, target_usd, current_usd, want_usd, per0, per1 }),
            None => excluded.push(LpExclusion {
                position_id: p.position_id.clone(),
                nft_id: p.nft_id,
                label: position_label(p),
                reason: "the range holds no value at the current price, or a leg cannot be priced".to_string(),
            }),
        }
    }

    // Never suggest more than the aggregate gap. Positions already above their share
    // are left alone (no unwinding), so the others' shares can add up to more than
    // what LP as a whole is short by; scale them down together so a completed rebuild
    // lands on the LP target instead of overshooting it.
    let lp_now: f64 = raw
        .positions
        .iter()
        .map(|p| {
            p.amount0 * leg_usd_price(&p.token0, nock_usd, eth_usd)
                + p.amount1 * leg_usd_price(&p.token1, nock_usd, eth_usd)
        })
        .sum();
    let lp_gap = (lp_target_usd - lp_now).max(0.0);
    let total_want: f64 = wants.iter().map(|w| w.want_usd).sum();
    if total_want > lp_gap && total_want > 0.0 {
        let k = lp_gap / total_want;
        for w in wants.iter_mut() {
            w.want_usd *= k;
            // Report the share actually being filled, so the client does not show
            // "$128 of $162 wanted" for a deposit that is exactly what is wanted.
            w.target_usd = w.current_usd + w.want_usd;
        }
    }

    // Pass 2: total demand per funding source, and how far each must be scaled back.
    // Every position drawing on a short source is scaled by the same factor, so the
    // outcome does not depend on the order positions are listed in config.
    #[derive(Clone, Copy, PartialEq)]
    enum Src {
        Nock,
        Usdc,
        Unfundable,
    }
    // ETH/WETH legs never reach here (excluded at eligibility): gas is not a source.
    let src_of = |t: &PositionTokenInfo| {
        if is_nock_token(t) {
            Src::Nock
        } else if is_usdc_token(t) {
            Src::Usdc
        } else {
            Src::Unfundable
        }
    };
    let (mut need_nock, mut need_usdc) = (0.0f64, 0.0f64);
    for w in &wants {
        for (tok, per) in [(&w.p.token0, w.per0), (&w.p.token1, w.per1)] {
            let amt = per * w.want_usd;
            match src_of(tok) {
                Src::Nock => need_nock += amt,
                Src::Usdc => need_usdc += amt,
                Src::Unfundable => {}
            }
        }
    }
    let factor = |avail: f64, need: f64| if need > 0.0 { (avail.max(0.0) / need).min(1.0) } else { 1.0 };
    // (scale factor, plain-words reason) per source.
    let nock_f = (
        factor(funds.nock, need_nock),
        if !funds.floor_set {
            format!("only {:.0} NOCK is in the wallet", funds.nock.max(0.0))
        } else if funds.floor_tolerance_nock > 0.0 {
            format!(
                "only {:.0} NOCK may go into LP: what sits above the hard floor plus the {:.0} tolerated below it (a range sells its NOCK as price rises)",
                funds.nock.max(0.0),
                funds.floor_tolerance_nock
            )
        } else {
            format!(
                "only {:.0} NOCK sits above the hard floor (a range can sell its NOCK as price rises, so floor NOCK is never placed in LP)",
                funds.nock.max(0.0)
            )
        },
    );
    let usdc_f = (
        factor(funds.usdc, need_usdc),
        format!(
            "only ${:.2} of wallet USDC is free after the cash target, tax reserve and remaining DCA tranches",
            funds.usdc.max(0.0)
        ),
    );
    let mut suggestions = Vec::new();
    for w in wants {
        let p = w.p;
        let mut scale = 1.0f64;
        let mut limited_by: Option<String> = None;
        for (tok, per) in [(&p.token0, w.per0), (&p.token1, w.per1)] {
            if per <= 0.0 {
                continue;
            }
            let (f, why) = match src_of(tok) {
                Src::Nock => (nock_f.0, nock_f.1.clone()),
                Src::Usdc => (usdc_f.0, usdc_f.1.clone()),
                Src::Unfundable => (0.0, format!("{} is not a token the engine can fund", tok.symbol)),
            };
            if f < scale - 1e-9 {
                scale = f;
                limited_by = Some(why);
            }
        }
        let add_usd = w.want_usd * scale;
        if add_usd < LP_SUGGESTION_MIN_USD {
            excluded.push(LpExclusion {
                position_id: p.position_id.clone(),
                nft_id: p.nft_id,
                label: position_label(p),
                reason: format!("cannot be funded: {}", limited_by.unwrap_or_else(|| "amount below $1".to_string())),
            });
            continue;
        }
        let (amt0, amt1) = (w.per0 * add_usd, w.per1 * add_usd);
        let fmt_amt = |tok: &PositionTokenInfo, amt: f64| -> String {
            if is_eth_token(tok) {
                format!("{:.6}", amt)
            } else if is_usdc_token(tok) {
                format!("{:.2}", amt)
            } else {
                format!("{:.0}", amt)
            }
        };
        suggestions.push(LpSuggestion {
            position_id: p.position_id.clone(),
            nft_id: p.nft_id,
            protocol: p.protocol.clone(),
            label: position_label(p),
            target_usd: format!("{:.2}", w.target_usd),
            current_usd: format!("{:.2}", w.current_usd),
            add_usd: format!("{:.2}", add_usd),
            token0_symbol: p.token0.symbol.clone(),
            amount0: fmt_amt(&p.token0, amt0),
            token1_symbol: p.token1.symbol.clone(),
            amount1: fmt_amt(&p.token1, amt1),
            limited_by,
            nock_sold_out_at_multiple: nock_sell_out_multiple(p).map(|m| format!("{:.2}", m)),
            url: deposit_url_for(p, &raw.rebuild_rules.get(&p.position_id).cloned().unwrap_or_default()),
        });
    }
    (suggestions, excluded)
}

/// Aggregate of receipt-verified fills for one plan id.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct PlanFills {
    count: i64,
    spent_usdc: f64,
    acquired_nock: f64,
    last_at: Option<DateTime<Utc>>,
    /// Effective price of the earliest fill (USDC per NOCK).
    first_price: Option<f64>,
}

/// Live inputs the plan must respect. All in USD except `spot_price` (USD per NOCK).
#[derive(Debug, Default, Clone, Copy)]
struct PlanGuards {
    overextended: bool,
    spot_price: f64,
    /// Liquid USDC in the wallet (USDC inside LP positions cannot be spent).
    wallet_usdc: f64,
    /// USDC held inside LP positions. Counts toward the protected cash reserve
    /// (operator decision, 2026-11-14: rebuilding LP is cash-neutral).
    lp_usdc: f64,
    tax_reserve_usd: f64,
    cash_target_usd: f64,
}

/// Everything `compute_derived_metrics` needs about the plan for one cycle.
#[derive(Debug, Clone)]
struct PlanContext {
    cfg: Option<AccumulationPlanConfig>,
    fills: PlanFills,
    /// True when the accumulation ledger could not be read this cycle. The plan then
    /// reports `blocked` — never "zero fills", which would re-advise a buy already made.
    ledger_unavailable: bool,
    now: DateTime<Utc>,
}

impl PlanContext {
    fn none() -> Self {
        Self { cfg: None, fills: PlanFills::default(), ledger_unavailable: false, now: Utc::now() }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum PlanDirective {
    Invalid { reason: String },
    Pending { starts_at: DateTime<Utc> },
    TrancheDue { usdc: f64, limited_by: Option<String> },
    Waiting { next_due_at: DateTime<Utc> },
    Paused { reason: String },
    Blocked { reason: String },
    Complete,
}

/// LP weight vs target, with the same tolerance band as the deviation matrix.
fn lp_rebuild_status(lp_usd: f64, total_portfolio_usd: f64, target_weight_bps: i32) -> LpRebuild {
    let target_usd = total_portfolio_usd * (target_weight_bps as f64 / 10000.0);
    let weight_bps = if total_portfolio_usd > 0.0 {
        ((lp_usd / total_portfolio_usd) * 10000.0) as i32
    } else {
        0
    };
    let dev = weight_bps - target_weight_bps;
    let tol = (target_weight_bps as f64 * 0.20).max(100.0) as i32;
    // No LP target set: nothing to measure against, so no rebuild is ever suggested.
    let status = if target_weight_bps <= 0 {
        "no_target"
    } else if total_portfolio_usd <= 0.0 {
        "on_target"
    } else if dev < -tol {
        "under_target"
    } else if dev > tol {
        "over_target"
    } else {
        "on_target"
    };
    LpRebuild {
        status: status.to_string(),
        target_usd: format!("{:.2}", target_usd),
        actual_usd: format!("{:.2}", lp_usd),
        gap_usd: format!("{:.2}", (target_usd - lp_usd).max(0.0)),
        target_weight_bps,
        suggestions: Vec::new(),
        excluded: Vec::new(),
        floor_tolerance_nock: None,
        worst_case_nock: None,
    }
}

/// One-line summary of the tracked LP positions, built from the positions themselves
/// (issue #95; a port of the frontend `positionsSummary`). A pool reading zero in-range
/// liquidity counts as stale; a pool whose read failed is not called stale. It claims
/// nothing about yield or profit and loss.
///
/// The flag is true only when at least one ranged position holds liquidity, is in
/// range, and sits on a pool read live this cycle: the only case in which the old
/// "(Dual Growth Engine Active)" label is a statement of fact.
fn lp_positions_summary(raw: &RawOnChainData) -> (String, bool) {
    let n = raw.positions.len();
    if n == 0 {
        return ("No LP positions tracked.".to_string(), false);
    }
    let pool_active = |p: &RawPosition| -> Option<bool> {
        raw.pool_liquidity
            .get(&p.position_id)
            .and_then(|v| v.parse::<u128>().ok())
            .map(|l| l > 0)
    };
    let full = raw.positions.iter().filter(|p| is_full_range(p.tick_lower, p.tick_upper)).count();
    let ranged: Vec<&RawPosition> = raw.positions.iter().filter(|p| !is_full_range(p.tick_lower, p.tick_upper)).collect();
    // A ranged position holding no liquidity is empty: neither in nor out of range.
    let empty = ranged.iter().filter(|p| p.liquidity == 0).count();
    let held: Vec<&&RawPosition> = ranged.iter().filter(|p| p.liquidity > 0).collect();
    let stale = held.iter().filter(|p| pool_active(p) == Some(false)).count();
    let in_range = held.iter().filter(|p| p.in_range && pool_active(p) != Some(false)).count();
    let out = held.len() - in_range - stale;
    // A failed pool read (None) does not disprove activity: the same rule the in-range
    // count above uses, so one RPC hiccup cannot flip the HOLD label.
    let engine_active = ranged
        .iter()
        .any(|p| p.in_range && p.liquidity > 0 && pool_active(p) != Some(false));
    let mut parts: Vec<String> = Vec::new();
    if !ranged.is_empty() {
        let mut detail = vec![format!("{} in range", in_range)];
        if out > 0 {
            detail.push(format!("{} out of range", out));
        }
        if stale > 0 {
            detail.push(format!("{} on a stale pool", stale));
        }
        if empty > 0 {
            detail.push(format!("{} empty", empty));
        }
        parts.push(format!("{} ranged ({})", ranged.len(), detail.join(", ")));
    }
    if full > 0 {
        parts.push(format!("{} full range", full));
    }
    (
        format!("{} LP position{} tracked: {}.", n, if n == 1 { "" } else { "s" }, parts.join(", ")),
        engine_active,
    )
}

/// Standing bias (operator order, 2026-11-14): rebuilding LP tranches leaves the net
/// cash position unchanged, so it is always worth doing. When LP is under target the
/// rebuild becomes the primary directive whenever the engine would otherwise say
/// "nothing to do", and is appended to every other directive. It never outranks a due
/// tranche or a safety directive, so those transitions still notify.
fn apply_lp_rebuild_bias(action: String, details: String, lp: &LpRebuild) -> (String, String) {
    if lp.status != "under_target" {
        return (action, details);
    }
    let sentence = format!(
        "LP holds ${} against a ${} target ({:.0}% of portfolio), ${} short. Re-establishing ranges from NOCK and USDC already held is a reposition, not a purchase: the stable total is unchanged, only its location moves.",
        lp.actual_usd,
        lp.target_usd,
        lp.target_weight_bps as f64 / 100.0,
        lp.gap_usd
    );
    let sentence = if lp.suggestions.is_empty() {
        format!("{} No tracked position is eligible for a deposit right now.", sentence)
    } else {
        let steps: Vec<String> = lp
            .suggestions
            .iter()
            .map(|sg| {
                let mut legs = Vec::new();
                if sg.amount0.parse::<f64>().map_or(false, |a| a > 0.0) {
                    legs.push(format!("{} {}", sg.amount0, sg.token0_symbol));
                }
                if sg.amount1.parse::<f64>().map_or(false, |a| a > 0.0) {
                    legs.push(format!("{} {}", sg.amount1, sg.token1_symbol));
                }
                format!(
                    "add {} (${}) to {}{}",
                    legs.join(" + "),
                    sg.add_usd,
                    sg.label,
                    sg.nock_sold_out_at_multiple
                        .as_ref()
                        .map(|m| format!(" (sells all its NOCK by a {}x rise)", m))
                        .unwrap_or_default()
                )
            })
            .collect();
        let worst = lp
            .worst_case_nock
            .as_ref()
            .map(|w| format!(" If every range sold out, {} NOCK would remain.", w))
            .unwrap_or_default();
        format!("{} Suggested: {}.{}", sentence, steps.join("; "), worst)
    };
    // With nothing fundable the note is informational only: it must not replace the
    // directive, or "rebuild" would sit on top permanently with no step to take.
    if lp.suggestions.is_empty() {
        return (action, format!("{} Standing bias: {}", details, sentence));
    }
    let idle = action.starts_with("HOLD & ACCRETE")
        || action.starts_with("DIP ZONE")
        || action.starts_with("DCA ON SCHEDULE")
        || action.starts_with("DCA PLAN PENDING")
        || action.starts_with("DCA PAUSED")
        || action.starts_with("DCA BLOCKED");
    if idle {
        // The dip-zone crossing must still change the action (and so notify): the rebuild
        // carries it rather than swallowing it.
        let rebuild = if action.starts_with("DIP ZONE") {
            format!("REBUILD LP TRANCHES (No Net Cash Change) · {}", action)
        } else {
            "REBUILD LP TRANCHES (No Net Cash Change)".to_string()
        };
        (
            rebuild,
            format!("{} Meanwhile: {} — {}", sentence, action, details),
        )
    } else {
        (action, format!("{} Standing bias: {}", details, sentence))
    }
}

/// Virtual harvest sub-ledger over receipt-verified harvests (no on-chain address change).
/// The split follows `operator_profile.tax_reserve_ratio`; 0 (the default) books no reserve.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct SovereignLedger {
    cumulative_harvested_usd: String,
    tax_reserve_usd: String,
    growth_dry_powder_usd: String,
    harvest_count: i64,
    last_harvest_at: Option<String>,
    /// The configured ratio the split uses, e.g. "0.40"; "0.00" when none is configured (2.4.0).
    #[serde(default)]
    tax_reserve_ratio: String,
    /// Harvest rows with no USD value (no price found at their block), left out of the
    /// total rather than valued at a substitute price (2.4.0).
    #[serde(default)]
    unpriced_count: i64,
}

#[derive(Debug, Default, Clone, Copy)]
struct LedgerAggregate {
    total_usd: f64,
    harvest_count: i64,
    last_at: Option<DateTime<Utc>>,
    unpriced_count: i64,
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
    /// Live ETH/USD used to value gas and LP WETH (2.3.0). Null when unpriced or absent
    /// from an older snapshot: every 2.3.0 field is optional so stored rows still parse.
    #[serde(default)]
    eth_price_usd: Option<String>,
    /// Wallet ETH + WETH, the gas pillar (2.3.0). Counted in total_portfolio_usd; never
    /// cash, never deployable, no target.
    #[serde(default)]
    wallet_gas_eth: Option<String>,
    #[serde(default)]
    wallet_gas_usd: Option<String>,
    /// The ETH pillar: wallet gas plus the ETH/WETH legs of tracked LP (2.3.0).
    #[serde(default)]
    total_eth_amount: Option<String>,
    #[serde(default)]
    total_eth_usd: Option<String>,
}

/// Snapshot spec version the worker emits (specs/core/snapshot.yaml info.version).
const SNAPSHOT_SPEC_VERSION: &str = "2.4.0";

/// The gas (wallet ETH + WETH) and ETH pillar figures, each None when it cannot be
/// measured: unread wallet balances or an unpriced ETH. Never a zero stand-in.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct GasPillars {
    wallet_gas_eth: Option<f64>,
    wallet_gas_usd: Option<f64>,
    total_eth_amount: Option<f64>,
    total_eth_usd: Option<f64>,
}

/// Pure: wallet gas and the ETH pillar from the wallet reads, the ETH inside LP and the
/// live ETH price. The pillar is wallet gas plus LP ETH/WETH legs.
fn gas_pillars(wallet_eth: Option<f64>, wallet_weth: Option<f64>, lp_eth: f64, eth_px: f64) -> GasPillars {
    let priced = eth_px.is_finite() && eth_px > 0.0;
    let gas_eth = match (wallet_eth, wallet_weth) {
        (Some(e), Some(w)) => Some(e + w),
        _ => None,
    };
    let total_eth = gas_eth.map(|g| g + lp_eth);
    GasPillars {
        wallet_gas_eth: gas_eth,
        wallet_gas_usd: gas_eth.filter(|_| priced).map(|g| g * eth_px),
        total_eth_amount: total_eth,
        total_eth_usd: total_eth.filter(|_| priced).map(|t| t * eth_px),
    }
}

/// The high-water mark's date (issue #94, N2): the time of the earliest stored row at the
/// peak, unless the current total sets a new mark, in which case it is now. `prev` is
/// the peak row (value, generated_at) or None on an empty table. Pure.
fn resolve_high_water_mark_date(prev: Option<(f64, DateTime<Utc>)>, current_total_usd: f64, now: &str) -> String {
    match prev {
        Some((v, at)) if v >= current_total_usd => at.to_rfc3339(),
        _ => now.to_string(),
    }
}

/// A stored payload without the field has no known floor: "" reads as unavailable,
/// never someone else's 100000.
fn default_floor_target_string() -> String {
    String::new()
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct AssetAggregationSeries {
    time_points: Vec<String>,
    /// Wallet NOCK valued in USD (kept for the dollar composition panels).
    safe_haven_series: Vec<String>,
    /// Wallet NOCK in token units (issue #85): what the floor chart must plot.
    #[serde(default)]
    wallet_nock_series: Vec<String>,
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
    /// True when the position spans every price; its bounds are then not meaningful
    /// and clients should not draw it as a bracket. (2.2.0)
    #[serde(default)]
    full_range: bool,
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
    /// Error from the most recent ingestion cycle, cleared by the next success (issue #87).
    last_ingest_error: std::sync::Mutex<Option<(DateTime<Utc>, String)>>,
}

/// Grace added to the sync window before health calls the data stale; matches the
/// dashboard's own freshness allowance.
const HEALTH_SYNC_GRACE_SECONDS: i64 = 90;

/// Health verdict (issue #87). Pure. A snapshot existing is not enough to be "ok":
/// the most recent cycle must have succeeded and the data must be inside the sync
/// window plus grace.
fn health_status(has_snapshot: bool, lag_seconds: i64, sync_window_seconds: i64, last_cycle_failed: bool) -> &'static str {
    if last_cycle_failed {
        "degraded"
    } else if !has_snapshot {
        "starting"
    } else if lag_seconds > sync_window_seconds + HEALTH_SYNC_GRACE_SECONDS {
        "degraded"
    } else {
        "ok"
    }
}

/// Run one ingestion cycle and remember whether it failed, for /health.
async fn run_cycle_tracked(state: &Arc<AppState>) -> Result<()> {
    let result = run_ingestion_cycle(state).await;
    if let Ok(mut slot) = state.last_ingest_error.lock() {
        *slot = result.as_ref().err().map(|e| (Utc::now(), e.to_string()));
    }
    result
}

// ============================================================
// Main
// ============================================================

/// Build the worker config from layered sources (issue #86).
///
/// `example_path` is the base layer. It is loaded with an explicit YAML format: the
/// `config` crate infers format from the file extension, and `.example` is not one it
/// knows, so `File::with_name("config.yaml.example")` was silently skipped and a
/// partial config.yaml (for instance one without `positions:`) failed ingestion.
/// `config_path` None means "config.{yaml,toml,json,...}" in the working directory.
fn load_app_config(example_path: &str, config_path: Option<&str>, include_env: bool) -> Result<AppConfig> {
    let mut builder = Config::builder()
        .add_source(config::File::new(example_path, config::FileFormat::Yaml).required(false));
    builder = match config_path {
        Some(path) => builder.add_source(config::File::new(path, config::FileFormat::Yaml).required(false)),
        None => builder.add_source(config::File::with_name("config").required(false)),
    };
    if include_env {
        builder = builder
            .add_source(config::Environment::with_prefix("APP").separator("__"))
            .add_source(config::Environment::default());
    }
    Ok(builder.build()?.try_deserialize::<AppConfig>()?)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Load config: config.yaml.example is the base layer, config.yaml overrides it,
    // environment variables override both.
    let config = load_app_config("config.yaml.example", None, true)?;

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
        last_ingest_error: std::sync::Mutex::new(None),
    });

    // Historical price backfill (v0.3.0): heal multi-hour/multi-day gaps left by
    // laptop sleep or downtime before the first cycle computes the daily RSI.
    // Fail-open by design; never blocks startup.
    ensure_price_history(&state).await;

    // Start ingestion scheduler
    let state_clone = state.clone();
    tokio::spawn(async move {
        // Run immediately on boot to guarantee freshness
        if let Err(e) = run_cycle_tracked(&state_clone).await {
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
            if let Err(e) = run_cycle_tracked(&state_clone).await {
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
            "/v1/execute/record-accumulation",
            axum::routing::post(record_accumulation_handler),
        )
        .route(
            "/v1/accumulation/events",
            axum::routing::get(accumulation_events_handler),
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
    let last_error: Option<(DateTime<Utc>, String)> =
        state.last_ingest_error.lock().ok().and_then(|slot| slot.clone());
    let last_sync: Option<DateTime<Utc>> = latest.as_ref().map(|s| s.meta.generated_at);
    let ingestion_lag_seconds: i64 = last_sync
        .map(|ts| {
            Utc::now().signed_duration_since(ts).num_seconds().max(0)
        })
        .unwrap_or(0);

    let sync_duration_ms = state.last_sync_duration_ms.load(std::sync::atomic::Ordering::Relaxed);

    let status = health_status(
        latest.is_some(),
        ingestion_lag_seconds,
        state.config.sync_window_seconds as i64,
        last_error.is_some(),
    );
    Json(json!({
        "status": status,
        "last_error": last_error.as_ref().map(|(_, e)| e.clone()),
        "last_error_at": last_error.as_ref().map(|(t, _)| t.to_rfc3339()),
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
        let last_error = state.last_ingest_error.lock().ok().and_then(|slot| slot.clone());
        (
            StatusCode::SERVICE_UNAVAILABLE,
            HeaderMap::new(),
            Json(no_snapshot_body(last_error.as_ref().map(|(_, e)| e.as_str()))),
        )
    }
}

/// 503 NO_SNAPSHOT body. Pure. On a first run the real cause (a placeholder key, a
/// missing price source, an NFT the wallet does not own) is only in the last ingestion
/// error, so it travels as `detail`; the dashboard and MCP show it instead of a bare
/// "No snapshot available yet".
fn no_snapshot_body(last_error: Option<&str>) -> serde_json::Value {
    let mut body = json!({
        "error": "NO_SNAPSHOT",
        "message": "No snapshot available yet"
    });
    if let Some(e) = last_error.filter(|e| !e.trim().is_empty()) {
        body["detail"] = json!(e);
    }
    body
}

async fn trigger_ingest_handler(
    Extension(state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    match run_cycle_tracked(&state).await {
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

/// Rebuild an envelope from a stored `snapshots` row. Pure. The `data` column holds
/// only the `SnapshotData` payload (see `store_snapshot`); meta comes from the row.
fn envelope_from_stored(
    snapshot_id: Uuid,
    generated_at: DateTime<Utc>,
    state_hash: String,
    version: String,
    data: serde_json::Value,
) -> Result<SnapshotEnvelope, serde_json::Error> {
    Ok(SnapshotEnvelope {
        meta: SnapshotMeta { generated_at, state_hash, version, snapshot_id },
        data: serde_json::from_value::<SnapshotData>(data)?,
    })
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
        // store_snapshot writes only the SnapshotData payload into `data`; the envelope
        // meta lives in the row's own columns.
        if let Ok(envelope) =
            envelope_from_stored(row.snapshot_id, row.generated_at, row.state_hash, row.version, row.data)
        {
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
                        "description": "Simulates gas expenditure vs uncollected LP fees across the tracked LP positions.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {},
                            "required": []
                        }
                    },
                    {
                        "name": "get_accumulation_plan",
                        "description": "Returns the dollar-cost-averaging plan: whether a NOCK tranche is due, its size, progress from receipt-verified buys, average cost, and the LP-rebuild bias.",
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
            let last_error: Option<String> =
                state.last_ingest_error.lock().ok().and_then(|slot| slot.clone()).map(|(_, e)| e);
            let result_text = if let Some(ref env) = *latest {
                let data_val = serde_json::to_value(&env.data).unwrap_or_default();
                let body = match tool_name {
                    "get_portfolio_state" => format_portfolio_state_text(&data_val),
                    "get_shot_advice" => format_shot_advice_text(&data_val),
                    "simulate_claim_fees" => format_claim_simulation_text(&data_val),
                    "get_accumulation_plan" => format_accumulation_plan_text(&data_val),
                    _ => format!("Unknown tool: {tool_name}"),
                };
                let now = Utc::now();
                let lag = now.signed_duration_since(env.meta.generated_at).num_seconds().max(0);
                let health = health_status(true, lag, state.config.sync_window_seconds as i64, last_error.is_some());
                format!("{body}\n{}", snapshot_source_line(env.meta.generated_at, now, health))
            } else {
                no_snapshot_mcp_text(last_error.as_deref())
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

    // Persistent high-water mark: the peak stored total and the earliest row that reached
    // it (issue #94, N2). An empty table (a fresh start) has no mark yet. A database error
    // fails the cycle: it must never silently reset the mark to the current value.
    let hwm_row = sqlx::query(
        r#"SELECT generated_at, (data->'derived_metrics'->'global_status'->>'total_portfolio_usd')::float8 AS v
           FROM snapshots
           WHERE (data->'derived_metrics'->'global_status'->>'total_portfolio_usd') IS NOT NULL
           ORDER BY v DESC, generated_at ASC
           LIMIT 1"#,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| anyhow::anyhow!("High-water mark query failed: {e}"))?;
    let prev_peak: Option<(f64, DateTime<Utc>)> = match hwm_row {
        None => None,
        Some(r) => Some((
            r.try_get::<f64, _>("v").map_err(|e| anyhow::anyhow!("High-water mark value unreadable: {e}"))?,
            r.try_get::<DateTime<Utc>, _>("generated_at")
                .map_err(|e| anyhow::anyhow!("High-water mark date unreadable: {e}"))?,
        )),
    };
    let prev_hwm: f64 = prev_peak.map(|(v, _)| v).unwrap_or(0.0);

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
                MAX(recorded_at) AS last_at, \
                COUNT(*) FILTER (WHERE usd_value IS NULL) AS unpriced \
         FROM harvest_events",
    )
    .fetch_one(&state.db)
    .await
    {
        Ok(r) => LedgerAggregate {
            total_usd: r.try_get::<f64, _>("total_usd").unwrap_or(0.0),
            harvest_count: r.try_get::<i64, _>("harvests").unwrap_or(0),
            last_at: r.try_get::<Option<DateTime<Utc>>, _>("last_at").ok().flatten(),
            unpriced_count: r.try_get::<i64, _>("unpriced").unwrap_or(0),
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

    let plan_ctx = load_plan_context(state).await;
    let mut derived_metrics = compute_derived_metrics_with_plan(&raw_data, prev_hwm, &profile, structural_channels,
        &counterfactual_history, &ledger_agg, &regime, &plan_ctx);
    {
        let gs = &mut derived_metrics.global_status;
        let current_total: f64 = gs.total_portfolio_usd.parse().unwrap_or(f64::INFINITY);
        gs.high_water_mark_date = resolve_high_water_mark_date(prev_peak, current_total, &gs.high_water_mark_date);
    }
    // Capture directive strings before the metrics move into the snapshot envelope
    let next_directive_action = derived_metrics.global_status.shot_advice_action.clone();
    let next_directive_details = derived_metrics.global_status.shot_advice_details.clone();

    // Build snapshot data
    let snapshot_data = SnapshotData {
        wallet_balances: raw_data.wallet_balances,
        positions: {
            let mut positions = build_position_snapshots(&raw_data.positions, raw_data.spot_price_nock_usd, raw_data.live_eth_price_usd);
            apply_pool_activity(&mut positions, &raw_data.pool_liquidity);
            positions
        },
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
            version: SNAPSHOT_SPEC_VERSION.to_string(),
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

/// Live ETH/USD spot from the canonical Uniswap v3 WETH/USDC 0.05% pool slot0
/// (`eth_price_pool` config, default DEFAULT_ETH_PRICE_POOL — factory-resolved and
/// price-verified at Phase-1 discovery). token0 = WETH (18 dec), token1 = USDC (6 dec).
/// Zero-mock: returns None when the RPC query fails — callers must degrade honestly
/// instead of valuing gas at a fabricated price.
async fn fetch_eth_price_usd(http_client: &Client, alchemy_url: &str, pool: &str) -> Option<f64> {
    fetch_eth_price_usd_at(http_client, alchemy_url, pool, "latest").await
}

/// ETH/USD from the price pool's slot0 at a given block tag ("latest" or a hex block
/// number; historical blocks need an archive RPC, which Alchemy provides). None on failure.
async fn fetch_eth_price_usd_at(http_client: &Client, alchemy_url: &str, pool: &str, block_tag: &str) -> Option<f64> {
    let payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": pool,
            "data": V3_SLOT0
        }, block_tag],
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
/// keccak256("Transfer(address,address,uint256)") — verified against the repo keccak tooling.
const TRANSFER_EVENT_TOPIC: &str =
    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
/// WETH9 `Withdrawal(address indexed src, uint256 wad)` — checked against live Base WETH
/// logs on 2026-11-15 (one indexed topic, amount in data).
const WETH_WITHDRAWAL_TOPIC: &str =
    "0x7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b65";
/// Uniswap v3 NonfungiblePositionManager on Base.
const UNISWAP_V3_NPM: &str = "0x03a520b32C04BF3bEEf7BEb72E919cf822Ed34f1";
/// Ledger token address for fees paid out as native ETH (unwrapped WETH). Native ETH has no
/// contract, so this conventional sentinel keeps (tx_hash, token_address) unique per token.
const NATIVE_ETH_LEDGER_ADDRESS: &str = "0xEeeeeEeeeEeEeEeEeEeEEEeeeeEeeeeeeeEEeE";

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
    /// advice is suppressed; NockNock advises no sale and no buy.
    overextended: bool,
}

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

/// Extract what a fee claim paid the sovereign wallet from a transaction receipt:
/// - ERC-20 Transfer logs of USDC, NOCK or WETH to the wallet;
/// - WETH unwrapped by a Uniswap position manager (a WETH `Withdrawal` whose `src` is the
///   v3 NonfungiblePositionManager or the v4 PositionManager) in a transaction the wallet
///   sent. The Uniswap app collects WETH fees this way and forwards native ETH, which
///   leaves no Transfer log; it is booked as "ETH".
/// Logs for tokens we do not track are ignored with a warning rather than guessed at.
fn parse_harvest_transfers(receipt: &serde_json::Value, wallet: &str) -> Vec<ParsedHarvestTransfer> {
    let mut out = Vec::new();
    let wallet_padded = format!(
        "000000000000000000000000{}",
        wallet.trim_start_matches("0x").to_lowercase()
    );
    let Some(logs) = receipt.get("logs").and_then(|l| l.as_array()) else {
        return out;
    };
    let sent_by_wallet = receipt
        .get("from")
        .and_then(|f| f.as_str())
        .is_some_and(|f| f.eq_ignore_ascii_case(wallet));
    for log in logs {
        let Some(topics) = log.get("topics").and_then(|t| t.as_array()) else {
            continue;
        };
        // Native ETH paid out by a position manager unwrapping WETH.
        if sent_by_wallet && topics.len() == 2 {
            let is_weth = log
                .get("address")
                .and_then(|a| a.as_str())
                .is_some_and(|a| a.eq_ignore_ascii_case(WETH_BASE));
            let is_withdrawal = topics[0]
                .as_str()
                .is_some_and(|t| t.eq_ignore_ascii_case(WETH_WITHDRAWAL_TOPIC));
            if is_weth && is_withdrawal {
                let src = topics[1].as_str().unwrap_or("").trim_start_matches("0x");
                let src_addr = if src.len() == 64 { format!("0x{}", src[24..].to_lowercase()) } else { String::new() };
                let by_position_manager = src_addr.eq_ignore_ascii_case(UNISWAP_V3_NPM)
                    || src_addr.eq_ignore_ascii_case(UNISWAP_V4_POSM_BASE);
                let amount = log
                    .get("data")
                    .and_then(|d| d.as_str())
                    .and_then(|d| u128::from_str_radix(d.trim_start_matches("0x"), 16).ok());
                if let (true, Some(amount)) = (by_position_manager, amount) {
                    out.push(ParsedHarvestTransfer {
                        token_address: NATIVE_ETH_LEDGER_ADDRESS.to_string(),
                        token_symbol: "ETH".to_string(),
                        from_address: src_addr,
                        recipient: wallet.to_string(),
                        amount_raw: amount.to_string(),
                        amount_human: format_units(amount, 18),
                        decimals: 18,
                    });
                }
                continue;
            }
        }
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
        } else if token_addr.eq_ignore_ascii_case(WETH_BASE) {
            ("WETH", 18)
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

/// Net effect of a transaction on the sovereign wallet for a USDC→NOCK purchase:
/// `(usdc_spent_raw, nock_received_raw)`. Sums every USDC/NOCK Transfer log touching
/// the wallet (multi-hop routes and refunds net out) and returns `None` unless the
/// wallet ended with strictly less USDC and strictly more NOCK. Pure — unit-testable.
fn parse_accumulation_swap(receipt: &serde_json::Value, wallet: &str) -> Option<(u128, u128)> {
    let wallet_padded = format!(
        "000000000000000000000000{}",
        wallet.trim_start_matches("0x").to_lowercase()
    );
    let logs = receipt.get("logs").and_then(|l| l.as_array())?;
    let (mut usdc_out, mut usdc_in, mut nock_out, mut nock_in) = (0u128, 0u128, 0u128, 0u128);
    for log in logs {
        let Some(topics) = log.get("topics").and_then(|t| t.as_array()) else { continue };
        if topics.len() < 3 {
            continue;
        }
        let (Some(t0), Some(from_word), Some(to_word)) =
            (topics[0].as_str(), topics[1].as_str(), topics[2].as_str())
        else {
            continue;
        };
        if !t0.eq_ignore_ascii_case(TRANSFER_EVENT_TOPIC) {
            continue;
        }
        let Some(token) = log.get("address").and_then(|a| a.as_str()) else { continue };
        let is_usdc = token.eq_ignore_ascii_case(USDC_BASE);
        let is_nock = token.eq_ignore_ascii_case(NOCK_BASE);
        if !is_usdc && !is_nock {
            continue;
        }
        let Some(amount) = log
            .get("data")
            .and_then(|d| d.as_str())
            .and_then(|d| u128::from_str_radix(d.trim_start_matches("0x"), 16).ok())
        else {
            continue;
        };
        let from_wallet = from_word.trim_start_matches("0x").eq_ignore_ascii_case(&wallet_padded);
        let to_wallet = to_word.trim_start_matches("0x").eq_ignore_ascii_case(&wallet_padded);
        match (is_usdc, from_wallet, to_wallet) {
            (true, true, false) => usdc_out = usdc_out.saturating_add(amount),
            (true, false, true) => usdc_in = usdc_in.saturating_add(amount),
            (false, true, false) => nock_out = nock_out.saturating_add(amount),
            (false, false, true) => nock_in = nock_in.saturating_add(amount),
            _ => {}
        }
    }
    if usdc_out > usdc_in && nock_in > nock_out {
        Some((usdc_out - usdc_in, nock_in - nock_out))
    } else {
        None
    }
}

#[derive(Deserialize)]
struct RecordAccumulationRequest {
    tx_hash: String,
}

/// POST /v1/execute/record-accumulation — attach a confirmed USDC→NOCK purchase to
/// the configured accumulation plan (issue #79). The swap may have been made anywhere
/// (aero.xyz, a wallet's swap screen): acceptance depends only on the on-chain
/// receipt showing USDC leaving, and NOCK entering, the tracked wallet. Amounts come
/// from the Transfer logs. Idempotent on tx_hash.
async fn record_accumulation_handler(
    Extension(state): Extension<Arc<AppState>>,
    Json(req): Json<RecordAccumulationRequest>,
) -> impl IntoResponse {
    let err = |code: StatusCode, error: &str, detail: &str| {
        (code, Json(json!({ "error": error, "detail": detail }))).into_response()
    };
    let Some(plan) = state.config.accumulation_plan.clone() else {
        return err(StatusCode::CONFLICT, "NO_PLAN", "No accumulation_plan is configured; nothing to record against");
    };
    if !is_valid_tx_hash(&req.tx_hash) {
        return err(StatusCode::UNPROCESSABLE_ENTITY, "INVALID_TX_HASH", "tx_hash must be a 32-byte hex string");
    }
    let tx_hash = if req.tx_hash.starts_with("0x") {
        req.tx_hash.to_lowercase()
    } else {
        format!("0x{}", req.tx_hash.to_lowercase())
    };
    let alchemy_key = state.config.alchemy_api_keys.first().cloned().unwrap_or_default();
    if alchemy_key.is_empty() || alchemy_key == "YOUR_ALCHEMY_API_KEY" {
        return err(StatusCode::SERVICE_UNAVAILABLE, "NO_RPC", "No Alchemy RPC key configured; cannot verify receipts");
    }
    let alchemy_url = format!("https://base-mainnet.g.alchemy.com/v2/{}", alchemy_key);
    let Some(wallet) = state.config.tracked_wallets.first().cloned() else {
        return err(StatusCode::SERVICE_UNAVAILABLE, "NO_TRACKED_WALLET", "No tracked wallet configured; cannot verify receipts");
    };

    let receipt_payload = json!({
        "jsonrpc": "2.0", "method": "eth_getTransactionReceipt", "params": [tx_hash], "id": 42
    });
    let receipt = match post_rpc_with_retry(&state.http_client, &alchemy_url, &receipt_payload, 2).await {
        Some(res) => res.get("result").cloned().unwrap_or(serde_json::Value::Null),
        None => return err(StatusCode::SERVICE_UNAVAILABLE, "RPC_UNAVAILABLE", "Could not query receipt; not recorded"),
    };
    if receipt.is_null() {
        return err(
            StatusCode::NOT_FOUND,
            "RECEIPT_NOT_FOUND",
            "Transaction not found on-chain. If the wallet shows it approved, it may still be pending broadcast; retry once it lands.",
        );
    }
    if receipt.get("status").and_then(|s| s.as_str()) != Some("0x1") {
        return err(StatusCode::UNPROCESSABLE_ENTITY, "TX_REVERTED", "Receipt status is not success; nothing recorded");
    }
    let Some((usdc_raw, nock_raw)) = parse_accumulation_swap(&receipt, &wallet) else {
        return err(
            StatusCode::UNPROCESSABLE_ENTITY,
            "NOT_A_NOCK_PURCHASE",
            "Receipt does not show USDC leaving and NOCK entering the sovereign wallet",
        );
    };
    let usdc_spent = format_units(usdc_raw, 6);
    let nock_received = format_units(nock_raw, 16);
    let (usdc_f, nock_f) = (
        usdc_spent.parse::<f64>().unwrap_or(0.0),
        nock_received.parse::<f64>().unwrap_or(0.0),
    );
    if !(usdc_f > 0.0 && nock_f > 0.0) {
        return err(StatusCode::UNPROCESSABLE_ENTITY, "NOT_A_NOCK_PURCHASE", "Parsed amounts are not positive");
    }
    let effective_price = format!("{:.8}", usdc_f / nock_f);

    let block_number: i64 = receipt
        .get("blockNumber")
        .and_then(|b| b.as_str())
        .and_then(|s| i64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        .unwrap_or(0);
    // The fill time drives the plan's cadence, so it must be the block time. Unlike
    // the harvest ledger there is no wall-clock fallback: a wrong time would shift
    // when the next tranche comes due.
    let block_payload = json!({
        "jsonrpc": "2.0", "id": 43, "method": "eth_getBlockByNumber",
        "params": [format!("0x{block_number:x}"), false]
    });
    let confirmed_at: Option<DateTime<Utc>> = if block_number > 0 {
        post_rpc_with_retry(&state.http_client, &alchemy_url, &block_payload, 2)
            .await
            .and_then(|res| res.get("result").cloned())
            .and_then(|blk| {
                blk.get("timestamp")
                    .and_then(|t| t.as_str())
                    .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            })
            .and_then(|ts| Utc.timestamp_opt(ts as i64, 0).single())
    } else {
        None
    };
    let Some(confirmed_at) = confirmed_at else {
        return err(StatusCode::SERVICE_UNAVAILABLE, "RPC_UNAVAILABLE", "Could not read the block timestamp; not recorded");
    };

    // A buy made before the plan existed is not part of the plan: without this an old
    // hash pasted by mistake would advance, or even complete, a brand-new plan.
    let registered_at = match plan_registered_at(&state.db, &plan.id).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Accumulation plan registry unavailable: {e}");
            return err(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", "Could not read the plan registry; nothing recorded");
        }
    };
    let anchor = plan_anchor(plan.starts_at, registered_at);
    if confirmed_at < anchor {
        return err(
            StatusCode::UNPROCESSABLE_ENTITY,
            "BEFORE_PLAN_START",
            &format!(
                "This purchase was confirmed at {}, before plan \"{}\" began ({}). To count earlier buys, set accumulation_plan.starts_at to an earlier time.",
                confirmed_at.to_rfc3339(),
                plan.id,
                anchor.to_rfc3339()
            ),
        );
    }

    let res = sqlx::query(
        "INSERT INTO accumulation_events \
            (plan_id, tx_hash, venue, usdc_spent_raw, nock_received_raw, usdc_spent, nock_received, effective_price_usd, block_number, confirmed_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
         ON CONFLICT (tx_hash) DO NOTHING",
    )
    .bind(&plan.id)
    .bind(&tx_hash)
    .bind(&plan.venue)
    .bind(usdc_raw.to_string())
    .bind(nock_raw.to_string())
    .bind(&usdc_spent)
    .bind(&nock_received)
    .bind(&effective_price)
    .bind(block_number)
    .bind(confirmed_at)
    .execute(&state.db)
    .await;
    match res {
        Ok(r) => Json(json!({
            "recorded": r.rows_affected() > 0,
            "duplicate": r.rows_affected() == 0,
            "plan_id": plan.id,
            "tx_hash": tx_hash,
            "usdc_spent": usdc_spent,
            "nock_received": nock_received,
            "effective_price_usd": effective_price,
            "block_number": block_number,
            "confirmed_at": confirmed_at.to_rfc3339(),
            "note": "Plan status updates on the next ingestion cycle (POST /v1/ingest to refresh now)."
        }))
        .into_response(),
        Err(e) => {
            tracing::error!("Failed to insert accumulation event: {e}");
            err(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", "Ledger insert failed; nothing recorded")
        }
    }
}

/// GET /v1/accumulation/events — receipt-verified fills, oldest first.
async fn accumulation_events_handler(Extension(state): Extension<Arc<AppState>>) -> impl IntoResponse {
    let rows = sqlx::query(
        "SELECT plan_id, tx_hash, venue, usdc_spent, nock_received, effective_price_usd, block_number, confirmed_at \
         FROM accumulation_events ORDER BY confirmed_at ASC, id ASC",
    )
    .fetch_all(&state.db)
    .await;
    match rows {
        Ok(rows) => {
            let events: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    json!({
                        "plan_id": r.try_get::<String, _>("plan_id").unwrap_or_default(),
                        "tx_hash": r.try_get::<String, _>("tx_hash").unwrap_or_default(),
                        "venue": r.try_get::<String, _>("venue").unwrap_or_default(),
                        "usdc_spent": r.try_get::<String, _>("usdc_spent").unwrap_or_default(),
                        "nock_received": r.try_get::<String, _>("nock_received").unwrap_or_default(),
                        "effective_price_usd": r.try_get::<String, _>("effective_price_usd").unwrap_or_default(),
                        "block_number": r.try_get::<i64, _>("block_number").unwrap_or(0),
                        "confirmed_at": r
                            .try_get::<DateTime<Utc>, _>("confirmed_at")
                            .map(|t| t.to_rfc3339())
                            .unwrap_or_default(),
                    })
                })
                .collect();
            Json(json!({ "events": events })).into_response()
        }
        Err(e) => {
            tracing::error!("Accumulation events query failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "DB_ERROR", "detail": "Could not read the accumulation ledger" })),
            )
                .into_response()
        }
    }
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

/// True when a position has no liquidity and no unclaimed fees. Pure.
fn nothing_to_collect(liquidity: &str, owed0: &str, owed1: &str) -> bool {
    let zero = |v: &str| v.parse::<f64>().map_or(false, |x| x == 0.0);
    zero(liquidity) && zero(owed0) && zero(owed1)
}

/// What NockNock says about an Aero pool whose claimFees() reverts: its swap fees are
/// not paid to LPs that way, and the AERO rewards a staked LP earns are not read yet.
const AERO_NOT_LP_CLAIMABLE: &str = "not LP-claimable (claimFees() reverts); AERO rewards not read (#93)";

/// Map an Aero claimFees() eth_call to (simulation, detail). Pure. A revert means the
/// pool pays no LP-claimable fees ("not_claimable"); only a transport failure is
/// "unavailable".
fn aero_claim_simulation(result: &Result<Option<String>>) -> (&'static str, Option<String>) {
    match result {
        Ok(Some(_)) => ("success", None),
        Ok(None) => ("not_claimable", Some(AERO_NOT_LP_CLAIMABLE.to_string())),
        Err(e) => ("unavailable", Some(format!("RPC unavailable for claimFees simulation: {e}"))),
    }
}

/// Gas and net totals for the claim payload. Pure. Actions that cannot execute
/// ("not_claimable") are left out, so they never null the totals of the ones that can.
/// `gas_usd` per action is None when its estimate failed; any such failure among the
/// executable actions makes the total unknown rather than understated.
fn claim_payload_totals(
    actions: &[ClaimFeeAction],
    gross_usd: &str,
    gas_price_and_eth_known: bool,
) -> (Option<String>, Option<String>) {
    let executable: Vec<&ClaimFeeAction> =
        actions.iter().filter(|a| a.simulation != "not_claimable").collect();
    let estimated_gas_usd = if gas_price_and_eth_known && !executable.is_empty() {
        executable
            .iter()
            .map(|a| a.gas_usd.as_deref().and_then(|g| g.parse::<f64>().ok()))
            .sum::<Option<f64>>()
            .map(|g| format!("{:.4}", g))
    } else {
        None
    };
    let net_after_gas_usd = match (gross_usd.trim(), &estimated_gas_usd) {
        (g, Some(e)) if !g.is_empty() => {
            let gross: f64 = g.parse().unwrap_or(0.0);
            Some(format!("{:.4}", gross - e.parse::<f64>().unwrap_or(0.0)))
        }
        _ => None,
    };
    (estimated_gas_usd, net_after_gas_usd)
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
    revert_is_not_claimable: bool,
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
        if nothing_to_collect(&p.liquidity_amount, &p.tokens_owed_0, &p.tokens_owed_1) {
            // An empty position's collect reverts; that is not an RPC outage (issue #87).
            action.expected_usd = Some("0.0000".to_string());
            action.simulation_detail =
                Some("Nothing to collect: this position holds no liquidity and no fees".to_string());
            return;
        }
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
    if revert_is_not_claimable {
        // Aero claimFees(): a revert is an answer (no LP-claimable fees), not an outage.
        let (sim, detail) = aero_claim_simulation(&eth_call_allow_revert(http_client, alchemy_url, &sim_payload).await);
        action.simulation = sim.to_string();
        if detail.is_some() {
            action.simulation_detail = detail;
        }
        if action.simulation == "not_claimable" {
            action.expected_usd = None;
        }
        return;
    }
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
            let last_error = state.last_ingest_error.lock().ok().and_then(|slot| slot.clone());
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(no_snapshot_body(last_error.as_ref().map(|(_, e)| e.as_str()))),
            )
                .into_response();
        }
    };
    let gross_usd = snapshot.data.derived_metrics.global_status.total_fees_claimable_usd.clone();

    let deadline = (Utc::now().timestamp() as u64) + 600;
    let mut actions: Vec<ClaimFeeAction> = Vec::new();

    // --- Per-position Uniswap v4 collect actions ---
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
    let aero_configs: Vec<&PositionConfig> = state
        .config
        .positions
        .iter()
        .filter(|p| p.protocol == "aerodrome")
        .collect();
    if v4_configs.is_empty() && v3_configs.is_empty() && aero_configs.is_empty() {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(json!({
            "error": "NO_CLAIMABLE_POSITIONS",
            "detail": "No LP positions configured; cannot build claim transactions"
        }))).into_response();
    }

    // Live prices for token-aware USD valuation (degrade to None, never fabricate).
    let eth_price_pool = state.config.eth_price_pool.clone().unwrap_or_else(|| DEFAULT_ETH_PRICE_POOL.to_string());
    let eth_price = fetch_eth_price_usd(&state.http_client, &alchemy_url, &eth_price_pool).await;
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
            &mut action, &pos_id, 20 + idx as i32, nock_spot, eth_price, false).await;
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
            &mut action, &pos_id, 20 + (v4_configs.len() + idx) as i32, nock_spot, eth_price, false).await;
        actions.push(action);
    }

    // --- Aerodrome volatile-pool fee claims (issue #88) ---
    // claimFees() on the pool pays the caller's accrued swap fees; selector 0xd294f093
    // verified against this pool (ONCHAIN_INTEGRATION.md §2).
    for (idx, pos) in aero_configs.iter().enumerate() {
        let Some(pool) = pos.pool_address.as_deref() else {
            continue;
        };
        let mut action = ClaimFeeAction {
            platform: "aerodrome".to_string(),
            label: format!("Claim Aero LP fees ({})", pos.id),
            to: pool.to_string(),
            data: "0xd294f093".to_string(),
            value: "0x0".to_string(),
            expected_amount_0: None,
            expected_amount_1: None,
            expected_usd: None,
            gas_estimate: None,
            gas_usd: None,
            simulation: "unavailable".to_string(),
            simulation_detail: None,
        };
        let pos_id = position_key(pos);
        finish_claim_action(&state.http_client, &alchemy_url, &wallet, &snapshot,
            &mut action, &pos_id, 40 + idx as i32, nock_spot, eth_price, true).await;
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
    for (idx, action) in actions.iter_mut().enumerate() {
        // A pool whose claimFees() reverts has nothing to execute: no estimate, and it
        // stays out of the totals (claim_payload_totals).
        if action.simulation == "not_claimable" {
            continue;
        }
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
                    }
                }
                None => {
                    action.simulation_detail = Some(action.simulation_detail.clone().unwrap_or_default()
                        + "; gas estimation failed");
                }
            },
            None => {}
        }
    }
    let (estimated_gas_usd, net_after_gas_usd) =
        claim_payload_totals(&actions, &gross_usd, gas_price_wei.is_some() && eth_price.is_some());

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

/// How far from a claim's block time a stored NOCK price may be and still value it. Live
/// cycles store one about every 5 minutes; backfilled candles are coarser.
const HARVEST_NOCK_PRICE_WINDOW_MINUTES: i64 = 60;

/// Unit price for a harvested token: USDC is $1 by definition of the ledger, NOCK and
/// ETH/WETH use the prices found for the claim's block. Pure.
fn harvest_unit_price<'a>(
    symbol: &str,
    nock: Option<&'a (f64, String)>,
    eth: Option<&'a (f64, String)>,
) -> Option<(f64, &'a str)> {
    match symbol {
        "USDC" => Some((1.0, "USDC at $1")),
        "NOCK" => nock.map(|(p, s)| (*p, s.as_str())),
        "ETH" | "WETH" => eth.map(|(p, s)| (*p, s.as_str())),
        _ => None,
    }
}

/// USD value of a harvested amount at a unit price; None when the price is unknown or
/// the amount does not parse. Pure.
fn harvest_usd_value(amount_human: &str, unit_price: Option<f64>) -> Option<String> {
    let amount = amount_human.parse::<f64>().ok()?;
    let price = unit_price.filter(|p| p.is_finite() && *p > 0.0)?;
    Some(format!("{:.4}", amount * price))
}

/// The stored NOCK/USD price nearest `at`, within HARVEST_NOCK_PRICE_WINDOW_MINUTES, with
/// its provenance. None when no stored price is close enough or the query fails.
async fn nock_price_near(db: &PgPool, at: DateTime<Utc>) -> Option<(f64, String)> {
    let window = chrono::Duration::minutes(HARVEST_NOCK_PRICE_WINDOW_MINUTES);
    let row = sqlx::query(
        "SELECT timestamp, price_usd, source FROM market_price_history \
         WHERE timestamp BETWEEN $2 AND $3 \
         ORDER BY ABS(EXTRACT(EPOCH FROM (timestamp - $1))) ASC LIMIT 1",
    )
    .bind(at)
    .bind(at - window)
    .bind(at + window)
    .fetch_optional(db)
    .await
    .map_err(|e| tracing::warn!("NOCK price lookup for a harvest failed: {e}"))
    .ok()??;
    let ts: DateTime<Utc> = row.try_get("timestamp").ok()?;
    let price: f64 = row.try_get("price_usd").ok()?;
    let source: String = row.try_get("source").unwrap_or_default();
    (price.is_finite() && price > 0.0).then(|| (price, format!("market_price_history {source} at {}", ts.to_rfc3339())))
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
            Json(json!({ "error": "NO_TRACKED_TRANSFERS", "detail": "Receipt pays the sovereign wallet no tracked token (USDC, NOCK, WETH, or ETH unwrapped by a Uniswap position manager)" })),
        )
            .into_response();
    }

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

    // USD valuation at the claim's block, never at recording time: NOCK from the stored
    // price nearest the block time, ETH/WETH from the ETH price pool read at that block.
    // A price we cannot find is an explicit null, never a substitute.
    let needs_nock = transfers.iter().any(|t| t.token_symbol == "NOCK");
    let needs_eth = transfers.iter().any(|t| matches!(t.token_symbol.as_str(), "ETH" | "WETH"));
    let nock_price: Option<(f64, String)> = if needs_nock {
        nock_price_near(&state.db, confirmed_at).await
    } else {
        None
    };
    let eth_price: Option<(f64, String)> = if needs_eth && block_number > 0 {
        let pool = state.config.eth_price_pool.clone().unwrap_or_else(|| DEFAULT_ETH_PRICE_POOL.to_string());
        fetch_eth_price_usd_at(&state.http_client, &alchemy_url, &pool, &format!("0x{block_number:x}"))
            .await
            .map(|p| (p, format!("eth_price_pool {pool} slot0 at block {block_number}")))
    } else {
        None
    };

    let mut recorded = Vec::new();
    let mut skipped = 0usize;
    for t in &transfers {
        let price = harvest_unit_price(&t.token_symbol, nock_price.as_ref(), eth_price.as_ref());
        let usd_value = harvest_usd_value(&t.amount_human, price.map(|(p, _)| p));
        let price_usd = price.map(|(p, _)| format!("{p}"));
        let price_source = price.map(|(_, s)| s.to_string());
        let res = sqlx::query(
            "INSERT INTO harvest_events (platform, tx_hash, token_address, token_symbol, from_address, recipient, amount_raw, amount_human, decimals, usd_value, block_number, confirmed_at, price_usd, price_source)              VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)              ON CONFLICT (tx_hash, token_address) DO NOTHING",
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
        .bind(&price_usd)
        .bind(&price_source)
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
                    "price_usd": price_usd,
                    "price_source": price_source,
                    "block_number": block_number,
                    "confirmed_at": confirmed_at.to_rfc3339(),
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
    let Some(wallet_addr) = state.config.tracked_wallets.first() else {
        return Err(anyhow::anyhow!(
            "No tracked wallet configured: set tracked_wallets in config.yaml to the address NockNock should read"
        ));
    };

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
    // 0b. Stale-price guard (issue #94), as for the NOCK spot: wallet gas and LP WETH are
    //     valued in the total, so a frozen pool's price would be a mock number.
    let eth_liq_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "to": eth_price_pool, "data": V3_LIQUIDITY }, "latest"],
        "id": 98
    });
    let eth_pool_liquidity = post_rpc_with_retry(&state.http_client, &alchemy_url, &eth_liq_payload, 2)
        .await
        .and_then(|r| r.get("result").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .and_then(|s| {
            let clean = s.trim_start_matches("0x");
            if clean.len() < 64 {
                return None;
            }
            u128::from_str_radix(&clean[clean.len() - 32..], 16).ok()
        })
        .ok_or_else(|| {
            anyhow::anyhow!("Failed to query in-range liquidity of the ETH/USD price pool ({})", eth_price_pool)
        })?;
    check_eth_price_pool_liquidity(&eth_price_pool, eth_pool_liquidity)?;

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
    let mut fees_not_lp_claimable: Vec<String> = Vec::new();
    for pos in &state.config.positions {
        let raw = match pos.protocol.as_str() {
            "uniswap-v4" => {
                fetch_v4_position(&state.http_client, &alchemy_url, pos, wallet_addr).await?
            }
            "uniswap-v3" => {
                fetch_v3_position(&state.http_client, &alchemy_url, pos, wallet_addr).await?
            }
            "aerodrome" => {
                let (raw, fees_claimable) =
                    fetch_aerodrome_position(&state.http_client, &alchemy_url, pos, wallet_addr).await?;
                if !fees_claimable {
                    fees_not_lp_claimable.push(raw.position_id.clone());
                }
                raw
            }
            other => {
                return Err(anyhow::anyhow!(
                    "Position {} has unsupported protocol '{}' (expected uniswap-v4, uniswap-v3 or aerodrome)",
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

    // 3b. Stale-price guard (zero-mock): a pool with no in-range liquidity cannot
    //     trade, so its sqrtPrice stays frozen wherever the last liquidity was pulled.
    //     Publishing that as "live" spot is a data-integrity bug — fail the cycle.
    if let Some(src) = raw_positions.iter().find(|p| p.is_price_source) {
        let pool_liquidity =
            fetch_pool_in_range_liquidity(&state.http_client, &alchemy_url, src).await?;
        check_price_source_liquidity(&src.position_id, &src.pool_id, pool_liquidity)?;
    }

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

    let balances_json = post_rpc_with_retry(&state.http_client, &alchemy_url, &balances_payload, 2)
        .await
        .ok_or_else(|| anyhow::anyhow!("Failed to query live token balances from Alchemy RPC"))?;
    let (nock_balance, usdc_balance) = parse_wallet_token_balances(&balances_json)?;

    let nock_balance_usd = nock_balance * spot_price;
    let now_str = Utc::now().to_rfc3339();

    // 5. Snapshot-level builders (sync, unit-testable): USD pricing, pool mechanics,
    //    and per-position contract fees.
    // 5a. Pool activity per tracked position (issue #83). A failed read leaves the
    //     key absent: unknown, never assumed live or dead.
    let mut pool_liquidity: HashMap<String, String> = HashMap::new();
    for p in &raw_positions {
        match fetch_pool_in_range_liquidity(&state.http_client, &alchemy_url, p).await {
            Ok(l) => {
                pool_liquidity.insert(p.position_id.clone(), l.to_string());
            }
            Err(e) => tracing::warn!("pool liquidity read failed for {}: {e}", p.position_id),
        }
    }
    // 5b. ETH and WETH balances: the gas pillar (issue #94). Counted in the total, so both
    //     reads are required, like NOCK and USDC: a failed read fails the cycle.
    let eth_balance_payload = json!({
        "jsonrpc": "2.0", "method": "eth_getBalance", "params": [wallet_addr, "latest"], "id": 60
    });
    let wallet_eth = post_rpc_with_retry(&state.http_client, &alchemy_url, &eth_balance_payload, 2)
        .await
        .and_then(|r| r.get("result").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .and_then(|h| u128::from_str_radix(h.trim_start_matches("0x"), 16).ok())
        .map(|wei| wei as f64 / 1e18);
    let weth_balance_payload = json!({
        "jsonrpc": "2.0", "method": "eth_call", "id": 61,
        "params": [{
            "to": WETH_BASE,
            "data": format!("0x70a08231000000000000000000000000{}", wallet_addr.trim_start_matches("0x").to_lowercase())
        }, "latest"]
    });
    let wallet_weth = post_rpc_with_retry(&state.http_client, &alchemy_url, &weth_balance_payload, 2)
        .await
        .and_then(|r| r.get("result").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .filter(|h| h.len() > 2)
        .and_then(|h| u128::from_str_radix(h.trim_start_matches("0x"), 16).ok())
        .map(|wei| wei as f64 / 1e18);
    let (gas_eth, gas_weth) = require_gas_balances(wallet_eth, wallet_weth)?;
    // 5c. Per-position rebuild rules from config, keyed like RawPosition.position_id.
    let rebuild_rules: HashMap<String, RebuildRule> = state
        .config
        .positions
        .iter()
        .map(|c| {
            (
                position_key(c),
                RebuildRule {
                    weight_bps: c.rebuild_weight_bps,
                    when_out_of_range: c.rebuild_when_out_of_range,
                    deposit_url: c.deposit_url.clone(),
                },
            )
        })
        .collect();

    let mut positions = build_position_snapshots(&raw_positions, spot_price, live_eth_price_usd);
    apply_pool_activity(&mut positions, &pool_liquidity);
    let pool_mechanics = build_pool_mechanics(&raw_positions, spot_price, live_eth_price_usd);
    let contract_fees = build_contract_fees(&positions, live_gas_cost_usd, &fees_not_lp_claimable);

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
            // Gas rows (2.3.0) come after NOCK and USDC, which keep indexes 0 and 1.
            WalletBalance {
                wallet_address: wallet_addr.clone(),
                token_symbol: "ETH".to_string(),
                token_address: NATIVE_ETH.to_string(),
                amount: format!("{:.6}", gas_eth),
                amount_usd: format!("{:.2}", gas_eth * live_eth_price_usd),
                updated_at: now_str.clone(),
            },
            WalletBalance {
                wallet_address: wallet_addr.clone(),
                token_symbol: "WETH".to_string(),
                token_address: WETH_BASE.to_string(),
                amount: format!("{:.6}", gas_weth),
                amount_usd: format!("{:.2}", gas_weth * live_eth_price_usd),
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
        pool_liquidity,
        wallet_eth: Some(gas_eth),
        wallet_weth: Some(gas_weth),
        rebuild_rules,
        fees_not_lp_claimable,
    })
}

const USDC_BASE_LC: &str = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913";
const NOCK_BASE_LC: &str = "0x9b5e262cf9bb04869ab40b19af91d2dc85761722";

/// Wallet NOCK and USDC from an `alchemy_getTokenBalances` response, in human units.
/// Every token must be present with a readable balance: a missing entry, an error, or a
/// null `tokenBalance` fails the cycle instead of reading as 0 (issue #94, N14). Pure.
fn parse_wallet_token_balances(res_json: &serde_json::Value) -> Result<(f64, f64)> {
    let balances = res_json
        .get("result")
        .and_then(|r| r.get("tokenBalances"))
        .and_then(|t| t.as_array())
        .ok_or_else(|| anyhow::anyhow!("Failed to query live token balances from Alchemy RPC: no tokenBalances in the response"))?;
    let read = |want: &str, symbol: &str, decimals: i32| -> Result<f64> {
        let entry = balances
            .iter()
            .find(|b| {
                b.get("contractAddress").and_then(|a| a.as_str()).map(|a| a.to_lowercase()).as_deref() == Some(want)
            })
            .ok_or_else(|| anyhow::anyhow!("Wallet {symbol} balance missing from the token balance response"))?;
        if let Some(err) = entry.get("error").filter(|e| !e.is_null()) {
            return Err(anyhow::anyhow!("Wallet {symbol} balance read failed: {err}"));
        }
        let hex = entry
            .get("tokenBalance")
            .and_then(|h| h.as_str())
            .ok_or_else(|| anyhow::anyhow!("Wallet {symbol} balance is null in the token balance response; not read as 0"))?;
        let clean = hex.trim_start_matches("0x");
        let raw = if clean.is_empty() {
            0u128 // "0x" is an explicit zero balance
        } else {
            u128::from_str_radix(clean, 16)
                .map_err(|_| anyhow::anyhow!("Wallet {symbol} balance '{hex}' is not a hex quantity"))?
        };
        Ok(raw as f64 / 10f64.powi(decimals))
    };
    let usdc = read(USDC_BASE_LC, "USDC", 6)?;
    let nock = read(NOCK_BASE_LC, "NOCK", 16)?;
    Ok((nock, usdc))
}

/// The wallet's ETH and WETH (gas) balances are counted in the total, so both reads
/// are required: a failed read fails the cycle rather than counting gas as zero. Pure.
fn require_gas_balances(eth: Option<f64>, weth: Option<f64>) -> Result<(f64, f64)> {
    match (eth, weth) {
        (Some(e), Some(w)) => Ok((e, w)),
        (e, w) => Err(anyhow::anyhow!(
            "Failed to read wallet {} (gas) balance: gas is counted in the portfolio total, so the cycle fails rather than counting it as 0",
            match (e.is_none(), w.is_none()) {
                (true, true) => "ETH and WETH",
                (true, false) => "ETH",
                _ => "WETH",
            }
        )),
    }
}

/// The ETH/USD price pool must have in-range liquidity, or its price is frozen and would
/// value gas and LP WETH at a stale number (issue #94). Pure — unit-testable.
fn check_eth_price_pool_liquidity(pool: &str, in_range_liquidity: u128) -> Result<()> {
    if in_range_liquidity == 0 {
        return Err(anyhow::anyhow!(
            "Stale ETH/USD price: price pool {} has zero in-range liquidity, so its price is frozen. \
             Set eth_price_pool to a WETH/USDC pool with active liquidity.",
            pool
        ));
    }
    Ok(())
}

/// A price-source pool must have active liquidity, otherwise its price is frozen
/// and must never be presented as live spot. Pure — unit-testable.
fn check_price_source_liquidity(position_id: &str, pool_id: &str, in_range_liquidity: u128) -> Result<()> {
    if in_range_liquidity == 0 {
        return Err(anyhow::anyhow!(
            "Stale NOCK/USD spot: price-source pool {} (position {}) has zero in-range liquidity, \
             so its price is frozen. Move is_price_source to a pool with active liquidity.",
            pool_id,
            position_id
        ));
    }
    Ok(())
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
    // A zero sqrtPrice (uninitialized pool) gives +inf: not a price.
    if price.is_finite() && price > 0.0 { Some(price) } else { None }
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
                lower_price: if is_full_range(p.tick_lower, p.tick_upper) {
                    "0".to_string()
                } else {
                    fmt_price(tick_to_display_price_dec(p.tick_upper, p.token0.decimals, p.token1.decimals))
                },
                upper_price: if is_full_range(p.tick_lower, p.tick_upper) {
                    "0".to_string()
                } else {
                    fmt_price(tick_to_display_price_dec(p.tick_lower, p.token0.decimals, p.token1.decimals))
                },
                liquidity_amount: p.liquidity.to_string(),
                in_range: p.in_range,
                is_price_source: p.is_price_source,
                pool_active: None, // filled by apply_pool_activity once pool reads are in
                full_range: is_full_range(p.tick_lower, p.tick_upper),
            }
        })
        .collect()
}

/// Stamp each snapshot position with whether its pool is live. Unknown stays None.
fn apply_pool_activity(positions: &mut [Position], pool_liquidity: &HashMap<String, String>) {
    for p in positions.iter_mut() {
        p.pool_active = pool_liquidity
            .get(&p.position_id)
            .and_then(|v| v.parse::<u128>().ok())
            .map(|l| l > 0);
    }
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

/// One ContractFee per tracked position with its unclaimed fees. A position listed in
/// `not_lp_claimable` (an Aero pool whose claimFees() reverts) says so in `amount`
/// and is not claimable, rather than reading as $0 of fees.
fn build_contract_fees(positions: &[Position], live_gas_cost_usd: f64, not_lp_claimable: &[String]) -> Vec<ContractFee> {
    positions
        .iter()
        .map(|p| {
            let not_claimable = not_lp_claimable.iter().any(|id| id == &p.position_id);
            let label = match p.protocol.as_str() {
                "uniswap-v4" => "Uniswap V4".to_string(),
                "uniswap-v3" => "Uniswap V3".to_string(),
                "aerodrome" => "Aerodrome".to_string(),
                _ => p.protocol.clone(),
            };
            ContractFee {
                contract_address: if p.protocol == "uniswap-v3" {
                    p.npm_address.clone().unwrap_or_else(|| p.pool_id.clone())
                } else if p.protocol == "aerodrome" {
                    p.pool_id.clone()
                } else {
                    UNISWAP_V4_POSM_BASE.to_string()
                },
                fee_token_address: p.token0.address.clone(),
                fee_token_symbol: if p.protocol == "aerodrome" {
                    format!("{}+{} ({})", p.token0.symbol, p.token1.symbol, label)
                } else {
                    format!("{}+{} ({} #{})", p.token0.symbol, p.token1.symbol, label, p.nft_id)
                },
                amount: if not_claimable {
                    AERO_NOT_LP_CLAIMABLE.to_string()
                } else {
                    format!(
                        "{:.p0$} {} + {:.p1$} {}",
                        p.tokens_owed_0.parse::<f64>().unwrap_or(0.0),
                        p.token0.symbol,
                        p.tokens_owed_1.parse::<f64>().unwrap_or(0.0),
                        p.token1.symbol,
                        p0 = (p.token0.decimals as usize).min(8),
                        p1 = (p.token1.decimals as usize).min(8),
                    )
                },
                amount_usd: p.fees_usd.clone(),
                claimable: !not_claimable && p.fees_usd.parse::<f64>().unwrap_or(0.0) > 0.0,
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

/// Pure core of the Aerodrome read (issue #88): turn pool reserves, LP supply, the
/// wallet's LP balance and its claimable fees into a RawPosition. A volatile pool is
/// x*y=k across every price, so it is modelled as a full-range position: the wallet
/// owns `lp_balance / total_supply` of each reserve, it is always in range, and its
/// price is reserve1/reserve0. Unit-testable.
#[allow(clippy::too_many_arguments)]
fn aerodrome_position_from_reads(
    cfg: &PositionConfig,
    wallet: &str,
    pool: &str,
    token0: PositionTokenInfo,
    token1: PositionTokenInfo,
    reserve0_raw: u128,
    reserve1_raw: u128,
    total_supply: u128,
    lp_balance: u128,
    fee0_raw: u128,
    fee1_raw: u128,
) -> Result<RawPosition> {
    if reserve0_raw == 0 || reserve1_raw == 0 || total_supply == 0 {
        return Err(anyhow::anyhow!(
            "position {}: Aerodrome pool {} has empty reserves or no LP supply",
            cfg.id,
            pool
        ));
    }
    let share = lp_balance as f64 / total_supply as f64;
    let amount0 = reserve0_raw as f64 * share / 10f64.powi(token0.decimals);
    let amount1 = reserve1_raw as f64 * share / 10f64.powi(token1.decimals);
    // sqrtPriceX96 equivalent of the pool's spot, so the shared price and sizing math
    // (nock_spot_from_positions, amounts_per_usd) works unchanged.
    let raw_price = reserve1_raw as f64 / reserve0_raw as f64; // token1_raw per token0_raw
    let sqrt_x96 = raw_price.sqrt() * 79228162514264337593543950336.0; // * 2^96
    let two128 = 340282366920938463463374607431768211456.0f64;
    let hi = (sqrt_x96 / two128).floor();
    let sqrt_price_x96 = U256(hi as u128, (sqrt_x96 - hi * two128) as u128);
    let tick = (raw_price.ln() / 1.0001f64.ln()).floor() as i32;
    Ok(RawPosition {
        position_id: position_key(cfg),
        network: cfg.network.clone(),
        protocol: "aerodrome".to_string(),
        wallet: wallet.to_string(),
        nft_id: 0,
        pool_id: pool.to_lowercase(),
        npm_address: None,
        fee_tier_bps: cfg.fee_bps.unwrap_or(30),
        token0: token0.clone(),
        token1: token1.clone(),
        liquidity: lp_balance,
        sqrt_price_x96,
        tick,
        tick_lower: MIN_TICK,
        tick_upper: MAX_TICK,
        amount0,
        amount1,
        fee0: fee0_raw as f64 / 10f64.powi(token0.decimals),
        fee1: fee1_raw as f64 / 10f64.powi(token1.decimals),
        in_range: true,
        is_price_source: cfg.is_price_source,
    })
}

/// Read an Aerodrome volatile-pool position (issue #88). Selectors are the ones the
/// pre-v0.4.0 worker used against this same pool (ONCHAIN_INTEGRATION.md §2):
/// getReserves 0x0902f1ac, totalSupply 0x18160ddd, balanceOf 0x70a08231, claimFees
/// 0xd294f093 simulated from the wallet. Every read failing fails the cycle (zero-mock).
async fn fetch_aerodrome_position(
    http_client: &Client,
    alchemy_url: &str,
    cfg: &PositionConfig,
    wallet: &str,
) -> Result<(RawPosition, bool)> {
    let pool = cfg
        .pool_address
        .clone()
        .ok_or_else(|| anyhow::anyhow!("position {}: aerodrome positions need pool_address", cfg.id))?;
    let wallet_word = format!("000000000000000000000000{}", wallet.trim_start_matches("0x").to_lowercase());
    let call = |data: String, from: Option<String>, id: i32| {
        let mut tx = json!({ "to": pool, "data": data });
        if let Some(f) = from {
            tx["from"] = json!(f);
        }
        json!({ "jsonrpc": "2.0", "method": "eth_call", "params": [tx, "latest"], "id": id })
    };
    let read = |payload: serde_json::Value, what: &'static str| {
        let cfg_id = cfg.id.clone();
        async move {
            post_rpc_with_retry(http_client, alchemy_url, &payload, 2)
                .await
                .and_then(|r| r.get("result").and_then(|v| v.as_str()).map(|s| s.trim_start_matches("0x").to_string()))
                .ok_or_else(|| anyhow::anyhow!("position {}: Aerodrome {} read failed", cfg_id, what))
        }
    };
    let word = |hex: &str, i: usize, what: &str| -> Result<u128> {
        let start = i * 64;
        if hex.len() < start + 64 {
            return Err(anyhow::anyhow!("position {}: Aerodrome {} returned short output", cfg.id, what));
        }
        // uint112/uint256 values here fit u128; take the low 32 hex bytes.
        u128::from_str_radix(&hex[start + 32..start + 64], 16)
            .map_err(|e| anyhow::anyhow!("position {}: Aerodrome {} unparseable: {e}", cfg.id, what))
    };

    let t0 = read(call("0x0dfe1681".to_string(), None, 70), "token0").await?;
    let t1 = read(call("0xd21220a7".to_string(), None, 71), "token1").await?;
    if t0.len() < 64 || t1.len() < 64 {
        return Err(anyhow::anyhow!("position {}: Aerodrome token0/token1 returned short output", cfg.id));
    }
    let token0 = token_info(&t0[24..64])?;
    let token1 = token_info(&t1[24..64])?;
    let reserves = read(call("0x0902f1ac".to_string(), None, 72), "getReserves").await?;
    let supply = read(call("0x18160ddd".to_string(), None, 73), "totalSupply").await?;
    let balance = read(call(format!("0x70a08231{wallet_word}"), None, 74), "balanceOf").await?;
    let mut lp_balance = word(&balance, 0, "balanceOf")?;
    // LP tokens staked in the gauge still belong to the wallet (issue #88 follow-up:
    // the operator's first aero.xyz deposit was staked automatically and read as $0).
    if let Some(gauge) = cfg.gauge_address.as_ref().filter(|g| !g.trim().is_empty()) {
        let payload = json!({
            "jsonrpc": "2.0", "method": "eth_call", "id": 76,
            "params": [{ "to": gauge, "data": format!("0x70a08231{wallet_word}") }, "latest"]
        });
        let staked_hex = read(payload, "gauge balanceOf").await?;
        let staked = word(&staked_hex, 0, "gauge balanceOf")?;
        lp_balance = lp_balance.saturating_add(staked);
    }
    // claimFees() simulated from the wallet returns exactly what a claim would pay.
    // With no LP there is nothing to simulate, so the call is skipped.
    //
    // Not every volatile pool pays fees to LPs this way: on the new Aero pools
    // claimFees() reverts (checked 2026-11-14 on 0x770fE8Fc…). A revert is an answer
    // ("nothing claimable here"), so fees read as zero; only a transport failure, where
    // the node gave no answer at all, fails the cycle.
    // The bool records whether claimFees() answers at all, so the revert is shown as
    // "not LP-claimable" rather than as $0 of fees earned.
    let (fee0_raw, fee1_raw, fees_claimable) = if lp_balance > 0 {
        let payload = call("0xd294f093".to_string(), Some(wallet.to_string()), 75);
        match eth_call_allow_revert(http_client, alchemy_url, &payload).await {
            Err(e) => {
                return Err(anyhow::anyhow!("position {}: Aerodrome claimFees read failed: {e}", cfg.id));
            }
            Ok(Some(hex)) => (word(&hex, 0, "claimFees")?, word(&hex, 1, "claimFees")?, true),
            Ok(None) => {
                tracing::info!(
                    "position {}: claimFees() reverted; this pool has no LP-claimable fees",
                    cfg.id
                );
                (0, 0, false)
            }
        }
    } else {
        (0, 0, true)
    };
    let raw = aerodrome_position_from_reads(
        cfg,
        wallet,
        &pool,
        token0,
        token1,
        word(&reserves, 0, "getReserves")?,
        word(&reserves, 1, "getReserves")?,
        word(&supply, 0, "totalSupply")?,
        lp_balance,
        fee0_raw,
        fee1_raw,
    )?;
    tracing::info!(
        "aerodrome position {}: LP balance={}, amounts {:.4} {} / {:.4} {}, fees {:.6}/{:.6}",
        cfg.id, raw.liquidity, raw.amount0, raw.token0.symbol, raw.amount1, raw.token1.symbol, raw.fee0, raw.fee1
    );
    Ok((raw, fees_claimable))
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

/// Plan-less entry point (unit tests and callers without an accumulation plan).
#[allow(dead_code)]
fn compute_derived_metrics(raw: &RawOnChainData, prev_hwm: f64, profile: &OperatorProfile, structural_channels: StructuralChannels, counterfactual_history: &[(DateTime<Utc>, f64, f64)], ledger: &LedgerAggregate, regime: &MarketRegime) -> DerivedMetrics {
    compute_derived_metrics_with_plan(raw, prev_hwm, profile, structural_channels, counterfactual_history, ledger, regime, &PlanContext::none())
}

#[allow(clippy::too_many_arguments)]
fn compute_derived_metrics_with_plan(raw: &RawOnChainData, prev_hwm: f64, profile: &OperatorProfile, structural_channels: StructuralChannels, counterfactual_history: &[(DateTime<Utc>, f64, f64)], ledger: &LedgerAggregate, regime: &MarketRegime, plan: &PlanContext) -> DerivedMetrics {
    // 1. Spot price: live NOCK/USD from the designated price-source position pool
    //    (zero-mock: the cycle fails upstream when the read fails).
    let spot_price = raw.spot_price_nock_usd;

    // 2. LP aggregates across all tracked positions (v2.0.0; replaces tranche 1/2 math).
    let mut lp_nock = 0.0f64;
    let mut lp_usdc = 0.0f64;
    let mut lp_usd = 0.0f64;
    let mut lp_fee_nock = 0.0f64;
    let mut lp_fee_usd = 0.0f64;
    // ETH/WETH inside tracked LP: the LP part of the ETH pillar (issue #94).
    let mut lp_eth = 0.0f64;
    // Per-position (address, label, fee_usd, LP-claimable) tuples for the velocity panel.
    let mut fee_entries: Vec<(String, String, f64, bool)> = Vec::new();
    for p in &raw.positions {
        let p0 = leg_usd_price(&p.token0, spot_price, raw.live_eth_price_usd);
        let p1 = leg_usd_price(&p.token1, spot_price, raw.live_eth_price_usd);
        lp_usd += p.amount0 * p0 + p.amount1 * p1;
        if is_usdc_token(&p.token0) { lp_usdc += p.amount0; }
        if is_usdc_token(&p.token1) { lp_usdc += p.amount1; }
        if is_nock_token(&p.token0) { lp_nock += p.amount0; }
        if is_nock_token(&p.token1) { lp_nock += p.amount1; }
        if is_eth_token(&p.token0) { lp_eth += p.amount0; }
        if is_eth_token(&p.token1) { lp_eth += p.amount1; }
        let fee_usd = p.fee0 * p0 + p.fee1 * p1;
        lp_fee_usd += fee_usd;
        if is_nock_token(&p.token0) { lp_fee_nock += p.fee0; }
        if is_nock_token(&p.token1) { lp_fee_nock += p.fee1; }
        fee_entries.push((
            if p.protocol == "uniswap-v3" {
                p.npm_address.clone().unwrap_or_else(|| p.pool_id.clone())
            } else if p.protocol == "aerodrome" {
                p.pool_id.clone()
            } else {
                UNISWAP_V4_POSM_BASE.to_string()
            },
            if p.protocol == "aerodrome" { position_label(p) } else { format!(
                "{} ({}/{} {:.2}%, #{})",
                if p.protocol == "uniswap-v4" { "Uniswap V4" } else { "Uniswap V3" },
                p.token0.symbol,
                p.token1.symbol,
                p.fee_tier_bps as f64 / 100.0,
                p.nft_id
            ) },
            fee_usd,
            !raw.fees_not_lp_claimable.contains(&p.position_id),
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

    // Gas pillar (issue #94): wallet ETH + WETH, counted in the total at the live ETH
    // price, but never cash, never dry powder, never a funding source and never under a
    // target. Unread (tests only) or unpriced gas is null, never zero.
    let eth_px = raw.live_eth_price_usd;
    let eth_priced = eth_px.is_finite() && eth_px > 0.0;
    let pillars = gas_pillars(raw.wallet_eth, raw.wallet_weth, lp_eth, eth_px);
    let wallet_gas_usd = pillars.wallet_gas_usd.unwrap_or(0.0);

    // One definition of the portfolio: every pillar, gas included. Targets, weights, the
    // high-water mark and history all use it.
    let total_portfolio_usd = lp_usd + t3_usd + wallet_usdc + wallet_gas_usd;
    // NOCK only (wallet + LP); ETH is its own pillar (total_eth_usd).
    let total_native_usd = (lp_nock + t3_nock) * spot_price;
    let total_stable_usd = lp_usdc + wallet_usdc;

    let floor = profile.treasury_floor_nock;
    let now_str = Utc::now().to_rfc3339();
    let total_nock = lp_nock + t3_nock + lp_fee_nock;
    let nock_floor_delta = total_nock - floor;
    let current_hwm = prev_hwm.max(total_portfolio_usd);

    // Range top of the price-source position (display units) — feeds the RANGE EXIT
    // branch; replaces the retired hardcoded V4 tick constants.
    let range_top_price = raw.positions.iter().find(|p| p.is_price_source && !is_full_range(p.tick_lower, p.tick_upper)).map(|p| {
        tick_to_display_price_dec(p.tick_lower, p.token0.decimals, p.token1.decimals)
    });

    // Liquid dry powder (issue #63): only wallet USDC is spendable. USDC inside LP is
    // working capital (a USDC/NOCK range converts it to NOCK as the price falls), so it
    // neither covers the virtual tax reserve nor counts as dry powder: the same rule
    // plan_directive's tax cap and the LP rebuild's liquid floor already use.
    let tax_reserve_usd = ledger.total_usd * profile.tax_reserve_ratio;
    let tax_guardrail_blocked = tax_reserve_usd > 0.0 && wallet_usdc < tax_reserve_usd;
    // Cash target (issue #79) and tax reserve overlap: the larger one stays in the wallet.
    let cash_target_usd = total_portfolio_usd * (profile.target_weights_bps.cash as f64 / 10000.0);
    let deployable_usdc = (wallet_usdc - tax_reserve_usd.max(cash_target_usd)).max(0.0);
    let lp_usdc_note = if lp_usdc >= 0.005 {
        format!(" The ${:.2} of USDC inside LP is working capital, not dry powder: it stays in its ranges.", lp_usdc)
    } else {
        String::new()
    };
    // Only rules the operator set are named: an unset cash weight or tax ratio is no rule,
    // never a default deduction.
    let cash_target_set = profile.target_weights_bps.cash > 0;
    let tax_ratio_set = profile.tax_reserve_ratio > 0.0;
    let dry_powder_amount = if deployable_usdc >= PLAN_DUST_USDC { format!("${:.2}", deployable_usdc) } else { "none".to_string() };
    let dry_powder_sentence = match (cash_target_set, tax_ratio_set) {
        (true, true) => format!(
            "Liquid dry powder: {} (wallet USDC ${:.2} less the larger of the ${:.2} cash target and the ${:.2} virtual tax reserve).",
            dry_powder_amount, wallet_usdc, cash_target_usd, tax_reserve_usd
        ),
        (true, false) => format!(
            "Liquid dry powder: {} (wallet USDC ${:.2} less the ${:.2} cash target).",
            dry_powder_amount, wallet_usdc, cash_target_usd
        ),
        (false, true) => format!(
            "Liquid dry powder: {} (wallet USDC ${:.2} less the ${:.2} virtual tax reserve).",
            dry_powder_amount, wallet_usdc, tax_reserve_usd
        ),
        (false, false) => format!(
            "Liquid dry powder: {} (wallet USDC; no cash target or tax reserve ratio is configured).",
            dry_powder_amount
        ),
    };
    // ETH is gas: stated as a fact when the wallet has none, never sized.
    let gas_note = match (raw.wallet_eth, raw.wallet_weth) {
        (Some(e), Some(w)) if e <= 0.0 && w <= 0.0 => " The wallet holds no ETH; every action on Base pays its gas in ETH.",
        _ => "",
    };
    let own_plan_hint = "add an accumulation_plan with your own budget_usdc, tranches and interval_hours";

    // Accumulation plan (issue #79): pure decision over config + verified fills.
    let plan_status: Option<AccumulationPlanStatus> = plan.cfg.as_ref().map(|cfg| {
        let guards = PlanGuards {
            overextended: regime.overextended,
            spot_price,
            wallet_usdc,
            lp_usdc,
            tax_reserve_usd,
            cash_target_usd,
        };
        let directive = plan_directive(cfg, &plan.fills, &guards, plan.ledger_unavailable, plan.now);
        build_plan_status(cfg, &plan.fills, &directive)
    });
    // While a plan exists it owns the buy decision: the lump-sum dip arms are retired
    // (a completed plan must not be followed by "deploy everything that is left").
    let plan_configured = plan_status.is_some();
    let active_plan = plan_status.as_ref().filter(|p| p.status != "complete");
    // An operator who opts out of the RSI pause keeps buying through an overextension.
    let plan_overrides_cooldown = active_plan.is_some()
        && plan.cfg.as_ref().map_or(false, |c| !c.pause_when_overextended);
    let mut lp_rebuild = lp_rebuild_status(lp_usd, total_portfolio_usd, profile.target_weights_bps.uniswap_lp);
    // Nothing tracked: there is no range to rebuild, so no suggestion and no bias text
    // (a user who never held LP is not told to "re-establish" one).
    if raw.positions.is_empty() {
        lp_rebuild.status = "no_positions".to_string();
    }
    if lp_rebuild.status == "under_target" {
        let lp_target_usd = total_portfolio_usd * (profile.target_weights_bps.uniswap_lp as f64 / 10000.0);
        // USDC that must stay liquid: the cash target, or the tax reserve plus the
        // accumulation plan's unspent budget, whichever is larger.
        let plan_remaining = active_plan
            .and_then(|p| p.remaining_usdc.parse::<f64>().ok())
            .unwrap_or(0.0);
        let liquid_floor = cash_target_usd.max(tax_reserve_usd + plan_remaining);
        let floor_tolerance_nock = floor * (profile.lp_floor_tolerance_bps as f64 / 10000.0);
        let funds = RebuildFunds {
            // A range sells its NOCK as price rises, so in a large enough rally NOCK
            // placed in LP is converted away entirely. LP may hold the NOCK above the
            // hard floor plus the operator's tolerated overage, which bounds how far
            // below the floor a full sell-out of every range could leave the stack.
            nock: wallet_nock - floor + floor_tolerance_nock,
            floor_tolerance_nock,
            floor_set: floor > 0.0,
            usdc: wallet_usdc - liquid_floor,
        };
        let (suggestions, excluded) = lp_rebuild_suggestions(raw, lp_target_usd, funds);
        // Worst case for the floor: every range sells all its NOCK, so only wallet
        // NOCK not being deposited remains.
        let suggested_nock: f64 = suggestions
            .iter()
            .map(|sg| {
                (if sg.token0_symbol == "NOCK" { sg.amount0.parse::<f64>().unwrap_or(0.0) } else { 0.0 })
                    + (if sg.token1_symbol == "NOCK" { sg.amount1.parse::<f64>().unwrap_or(0.0) } else { 0.0 })
            })
            .sum();
        lp_rebuild.worst_case_nock = Some(format!("{:.0}", wallet_nock - suggested_nock));
        lp_rebuild.floor_tolerance_nock = Some(format!("{:.0}", floor_tolerance_nock));
        lp_rebuild.suggestions = suggestions;
        lp_rebuild.excluded = excluded;
    }

    // Dynamic Shot Advice Engine (v0.5.1). Every figure comes from config or the chain;
    // NockNock never sizes a buy outside the operator's own accumulation plan.
    let (lp_summary, engine_active) = lp_positions_summary(raw);
    // RANGE EXIT reports the price-source range's own value, never the whole LP book.
    let range_exit: Option<(String, String)> = raw
        .positions
        .iter()
        .find(|p| p.is_price_source && !is_full_range(p.tick_lower, p.tick_upper))
        .zip(range_top_price)
        // An emptied range holds nothing to exit: it earns nothing in or out of range.
        .filter(|(p, _)| p.liquidity > 0)
        .filter(|(_, top)| spot_price > 0.0 && *top > 0.0 && spot_price >= *top)
        .map(|(p, top)| {
            let p0 = leg_usd_price(&p.token0, spot_price, raw.live_eth_price_usd);
            let p1 = leg_usd_price(&p.token1, spot_price, raw.live_eth_price_usd);
            let pos_usd = p.amount0 * p0 + p.amount1 * p1;
            (
                "RANGE EXIT FLASHPOINT (Above Range)".to_string(),
                format!(
                    "Spot ${:.4} is at or above the top of the price-source range (${:.4}). That range ({}) holds ${:.2} ({:.2} {} + {:.2} {}) and earns no fees unless the price falls back into it. Whether to leave it, withdraw it or open a new range is your decision; NockNock does not choose range bounds.",
                    spot_price,
                    top,
                    position_label(p),
                    pos_usd,
                    p.amount0,
                    p.token0.symbol,
                    p.amount1,
                    p.token1.symbol
                ),
            )
        });
    // BATCH HARVEST: the split comes from the configured ratio, and the text says what the
    // harvest ledger actually books (USDC, NOCK and WETH/ETH, each valued at the claim's block).
    let harvest: Option<(String, String)> = (total_fees_usd >= 5.00).then(|| {
        let gas_efficiency = if gas_cost > 0.0 {
            format!("{:.0}x", total_fees_usd / gas_cost)
        } else {
            "unknown".to_string()
        };
        let r = profile.tax_reserve_ratio * 100.0;
        let booking = if tax_ratio_set {
            format!(
                "NockNock books {:.0}% of the USDC, NOCK and WETH/ETH received to the virtual tax reserve and {:.0}% to growth",
                r,
                100.0 - r
            )
        } else {
            "NockNock records the USDC, NOCK and WETH/ETH received; no tax_reserve_ratio is configured, so none of it is booked to a virtual tax reserve".to_string()
        };
        (
            "BATCH HARVEST SHOT (Execute Claim)".to_string(),
            format!(
                "Uncollected LP fees (${:.2}) have reached the fixed $5.00 harvest threshold ({} gas efficiency). After harvesting, record the transaction: {}, each valued at the claim's block. Bookkeeping only; nothing moves on-chain.",
                total_fees_usd,
                gas_efficiency,
                booking
            ),
        )
    });
    // A plan with a tranche due (or a config error to report) leads. A plan with nothing
    // due (waiting, pending, paused, blocked) yields to a range exit or a due harvest (N11).
    let plan_leads = active_plan.filter(|p| !matches!(p.status.as_str(), "waiting" | "pending" | "paused" | "blocked"));
    let exit_or_harvest = range_exit.clone().or_else(|| harvest.clone());
    let wallet_label = raw
        .wallet_balances
        .first()
        .map(|w| format!("The tracked wallet {}", w.wallet_address))
        .unwrap_or_else(|| "The tracked wallet".to_string());
    let dip_zone = spot_price > 0.0 && spot_price < 0.020;

    let (advice_action, advice_details) = if floor > 0.0 && total_nock < floor {
        let deficit = format!(
            "Total NOCK ({:.0}) is below the {:.0} floor by {:.0} NOCK.",
            total_nock,
            floor,
            floor - total_nock
        );
        match active_plan {
            // The plan stays visible: a tranche transition changes the action, so it notifies.
            Some(ps) => {
                let (plan_action, plan_details) = plan_advice(ps, spot_price);
                (
                    format!("DEFENSIVE LOCK (Floor Recovery) · {}", plan_action),
                    format!("{} Your accumulation plan is the recovery path: {}", deficit, plan_details),
                )
            }
            // A configured plan that is complete is not "no plan": say so, by its id.
            None => match plan_status.as_ref() {
                Some(done) => (
                    "DEFENSIVE LOCK (Floor Recovery)".to_string(),
                    format!(
                        "{} Your accumulation plan \"{}\" is complete. NockNock does not size recovery buys: extend its budget_usdc or add a new plan with your own budget, tranches and interval to have them paced.",
                        deficit, done.plan_id
                    ),
                ),
                None => (
                    "DEFENSIVE LOCK (Floor Recovery)".to_string(),
                    format!(
                        "{} NockNock does not size recovery buys: {} to have them paced.",
                        deficit, own_plan_hint
                    ),
                ),
            },
        }
    } else if !plan_configured && total_portfolio_usd - wallet_gas_usd <= 0.0 && total_nock <= 0.0 {
        // Gas alone is not something to track or deploy: a gas-only wallet is still empty.
        let gas_held = match pillars.wallet_gas_eth {
            Some(g) if g > 0.0 => format!(
                " It holds {:.6} ETH/WETH for gas, counted in the total; gas is not cash and is never deployed.",
                g
            ),
            _ => String::new(),
        };
        (
            "NOTHING TO TRACK YET (Empty Wallet)".to_string(),
            format!(
                "{} holds no NOCK, no USDC and no LP value that NockNock reads. Once it is funded NockNock reports it here. To have NOCK buys paced, {}; set operator_profile.treasury_floor_nock only if you want a floor alarm.{}{}",
                wallet_label, own_plan_hint, gas_note, gas_held
            ),
        )
    } else if regime.overextended && !plan_overrides_cooldown {
        let cooldown = format!(
            "NOCK spot (${:.5}) is overextended: daily RSI(14) {} is at or above 70.{} NockNock advises no sale and no buy while it stays there.",
            spot_price,
            regime.rsi_1d.map(|r| format!("{:.0}", r)).unwrap_or_else(|| "unavailable".to_string()),
            if active_plan.is_some() {
                " Your accumulation plan pauses buys until it cools."
            } else {
                ""
            }
        );
        // A cooldown has nothing to buy, so a range exit or a due harvest outranks it
        // (N11); the overextension fact stays in the details.
        match exit_or_harvest.as_ref() {
            Some((a, d)) => (a.clone(), format!("{} {}", d, cooldown)),
            None => ("OVEREXTENDED (Daily RSI ≥ 70, No Buy Advised)".to_string(), cooldown),
        }
    } else if let Some(ps) = plan_leads {
        plan_advice(ps, spot_price)
    } else if let Some(adv) = exit_or_harvest.clone() {
        // Outranks a plan with nothing due (N11) and every planless arm below, none of
        // which has anything to buy.
        adv
    } else if let Some(ps) = active_plan {
        plan_advice(ps, spot_price)
    } else if !plan_configured && dip_zone && tax_guardrail_blocked {
        (
            "TAX RESERVE SHORT (No Buy Advised)".to_string(),
            format!(
                "Spot price (${:.5}) is below the $0.020 accumulation line, but the virtual tax reserve (${:.2}) exceeds liquid wallet USDC (${:.2}).{} No buy is advised while wallet USDC is below the reserve.",
                spot_price, tax_reserve_usd, wallet_usdc, lp_usdc_note
            ),
        )
    } else if !plan_configured && total_nock <= 0.0 && wallet_usdc >= PLAN_DUST_USDC {
        (
            "BUILD FROM ZERO (No Buy Plan Configured)".to_string(),
            format!(
                "{} holds ${:.2} of USDC and no NOCK.{} NockNock does not choose amounts for you: {} (optionally cash_reserve_usdc and max_price_usd) and it will pace each NOCK buy and verify it on-chain.{}",
                wallet_label, wallet_usdc, lp_usdc_note, own_plan_hint, gas_note
            ),
        )
    } else if !plan_configured && dip_zone && deployable_usdc >= PLAN_DUST_USDC {
        // Last arm before HOLD: it must never hide a range exit or a due harvest.
        (
            "DIP ZONE (No Buy Plan Configured)".to_string(),
            format!(
                "Spot ${:.5} is below the $0.020 accumulation line and the wallet holds ${:.2} of liquid USDC. {}{} NockNock does not size buys outside a plan: {} and it will pace them.{}",
                spot_price, wallet_usdc, dry_powder_sentence, lp_usdc_note, own_plan_hint, gas_note
            ),
        )
    } else {
        let floor_text = if floor > 0.0 {
            format!("Total NOCK held is {:.0} (+{:.0} above the {:.0} floor).", total_nock, nock_floor_delta, floor)
        } else {
            format!("Total NOCK held is {:.0}; no floor configured.", total_nock)
        };
        // The "HOLD & ACCRETE" prefix stays (apply_lp_rebuild_bias keys idle detection on
        // it); the parenthetical states only what the positions show.
        let label = if engine_active {
            "HOLD & ACCRETE (Dual Growth Engine Active)"
        } else if raw.positions.is_empty() {
            "HOLD & ACCRETE (No LP Tracked)"
        } else if lp_usd <= 0.0 {
            "HOLD & ACCRETE (LP Empty)"
        } else {
            "HOLD & ACCRETE (No Ranged LP Confirmed In Range)"
        };
        let dip_note = if !plan_configured && dip_zone {
            format!(
                " Spot (${:.5}) is below the $0.020 accumulation line. NockNock does not size buys outside a plan: {} to have them paced.",
                spot_price, own_plan_hint
            )
        } else {
            String::new()
        };
        (
            label.to_string(),
            format!(
                "{} {}{} {}{}{}",
                floor_text,
                lp_summary,
                if raw.positions.is_empty() { String::new() } else { format!(" ${:.2} of LP fees uncollected.", total_fees_usd) },
                dry_powder_sentence,
                lp_usdc_note,
                dip_note
            ),
        )
    };

    let (advice_action, advice_details) = apply_lp_rebuild_bias(advice_action, advice_details, &lp_rebuild);

    DerivedMetrics {
        global_status: GlobalStatus {
            total_portfolio_usd: format!("{:.2}", total_portfolio_usd),
            total_native_usd: format!("{:.2}", total_native_usd),
            total_stable_usd: format!("{:.2}", total_stable_usd),
            total_fees_accrued_usd: format!("{:.2}", total_fees_usd),
            total_fees_claimable_usd: format!("{:.2}", total_fees_usd),
            // Signed: fees less two claims' gas. A negative net is shown, never clamped to 0.
            gas_to_fee_delta_usd: format!("{:.2}", total_fees_usd - (gas_cost * 2.0)),
            high_water_mark_usd: format!("{:.2}", current_hwm),
            // The cycle replaces this with the peak row's own time when the mark was set
            // earlier (resolve_high_water_mark_date); here it is a new mark.
            high_water_mark_date: now_str.clone(),
            total_nock_amount: format!("{:.0}", total_nock),
            nock_floor_delta: format!("{:.0}", nock_floor_delta),
            nock_floor_target: format!("{:.0}", floor),
            total_usdc_amount: format!("{:.2}", total_stable_usd),
            shot_advice_action: advice_action,
            shot_advice_details: advice_details,
            regime: regime_label(&regime).to_string(),
            rsi_14: regime.rsi_5m.map(|r| format!("{:.1}", r)),
            rsi_14_1d: regime.rsi_1d.map(|r| format!("{:.1}", r)),
            eth_price_usd: eth_priced.then(|| format!("{:.2}", eth_px)),
            wallet_gas_eth: pillars.wallet_gas_eth.map(|v| format!("{:.6}", v)),
            wallet_gas_usd: pillars.wallet_gas_usd.map(|v| format!("{:.2}", v)),
            total_eth_amount: pillars.total_eth_amount.map(|v| format!("{:.6}", v)),
            total_eth_usd: pillars.total_eth_usd.map(|v| format!("{:.2}", v)),
        },
        asset_aggregation_series: AssetAggregationSeries {
            time_points: vec![now_str.clone()],
            safe_haven_series: vec![format!("{:.2}", t3_usd)],
            wallet_nock_series: vec![format!("{:.4}", t3_nock)],
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
                    let full_range = is_full_range(p.tick_lower, p.tick_upper);
                    TickBracket {
                        lower_price: if full_range { "0".to_string() } else { fmt_price(lower_disp) },
                        upper_price: if full_range { "0".to_string() } else { fmt_price(upper_disp) },
                        liquidity_usd: format!("{:.2}", pos_usd),
                        in_range: p.in_range,
                        full_range,
                        // Spot is USD/NOCK; brackets are token0-per-NOCK. Only USD-
                        // denominated token0 (USDC) yields an honest distance.
                        distance_bps: if full_range {
                            None
                        } else if p.token0.symbol == "USDC" {
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
                    // An unset weight (0) is no target: a measurement, never "over-funded".
                    if target_weight_bps <= 0 {
                        "no_target".to_string()
                    } else if deviation_bps > tol {
                        "over_funded".to_string()
                    } else if deviation_bps < -tol {
                        "under_funded".to_string()
                    } else {
                        "on_target".to_string()
                    }
                };

                let w = &profile.target_weights_bps;
                // An empty portfolio has no weights: 0/0 is never computed.
                let weight = |usd: f64| -> i32 {
                    if total_portfolio_usd > 0.0 { ((usd / total_portfolio_usd) * 10000.0) as i32 } else { 0 }
                };
                let t3_weight = weight(t3_usd);
                let t3_dev = t3_weight - w.treasury;

                let lp_weight = weight(lp_usd);
                let lp_dev = lp_weight - w.uniswap_lp;

                let usdc_weight = weight(wallet_usdc);
                let usdc_dev = usdc_weight - w.cash;

                // Gas has no target: shown as a measured share, never under pressure.
                let gas_weight = weight(wallet_gas_usd);

                vec![
                    AllocationDeviation {
                        bucket_name: "Sovereign Treasury".to_string(),
                        target_weight_bps: w.treasury,
                        actual_weight_bps: t3_weight,
                        deviation_bps: t3_dev,
                        status: calc_status(t3_dev, w.treasury),
                    },
                    AllocationDeviation {
                        bucket_name: "LP (all tracked positions)".to_string(),
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
                    AllocationDeviation {
                        bucket_name: "Gas (wallet ETH+WETH)".to_string(),
                        target_weight_bps: 0,
                        actual_weight_bps: gas_weight,
                        deviation_bps: gas_weight,
                        status: "no_target".to_string(),
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
            tax_reserve_ratio: format!("{:.2}", profile.tax_reserve_ratio),
            unpriced_count: ledger.unpriced_count,
        },
        accumulation_plan: plan_status,
        lp_rebuild,
        velocity_gas_sandbox: VelocityGasSandbox {
            contracts: fee_entries
                .iter()
                .map(|(addr, name, fee_usd, lp_claimable)| FeeCollectionIndex {
                    contract_address: addr.clone(),
                    contract_name: name.clone(),
                    claimable_usd: format!("{:.2}", fee_usd),
                    gas_break_even_usd: format!("{:.4}", gas_cost),
                    efficiency_ratio: if gas_cost > 0.0 {
                        (fee_usd / gas_cost * 100.0).round() / 100.0
                    } else {
                        0.0
                    },
                    // A reverting claimFees() is not "$0 earned, accumulate" (#93).
                    recommended_action: if *lp_claimable {
                        recommended_claim_action(*fee_usd, gas_cost).to_string()
                    } else {
                        "not_claimable".to_string()
                    },
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
        // The floor is the operator's own number (v0.5.1: there is no default floor).
        let profile = operator_profile_2026_11();
        let metrics = compute_derived_metrics(&raw, 2500.0, &profile, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        assert_eq!(metrics.global_status.shot_advice_action, "DEFENSIVE LOCK (Floor Recovery)");
        let d = &metrics.global_status.shot_advice_details;
        assert!(d.contains("is below the 100000 floor by"), "{d}");
        assert!(d.contains("accumulation_plan"));
        for banned in ["CRITICAL", "locked", "sovereign foundation", "Deploy $", "Buy $"] {
            assert!(!d.contains(banned), "{banned} in: {d}");
        }
        // With the default profile there is no floor, so no lock.
        let none = compute_derived_metrics(&raw, 2500.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        assert!(!none.global_status.shot_advice_action.contains("DEFENSIVE LOCK"));
        assert_eq!(none.global_status.nock_floor_target, "0");
    }

    /// An operator's own profile, every number set explicitly (floor 100,000, tax ratio
    /// 0.40, weights 8000/1500/500, the values these tests were written against). The
    /// code defaults choose nothing: no floor, no tax ratio, no target weights.
    fn operator_profile_2026_11() -> OperatorProfile {
        OperatorProfile {
            treasury_floor_nock: 100_000.0,
            lp_floor_tolerance_bps: 0,
            tax_reserve_ratio: 0.40,
            target_weights_bps: TargetWeightsBps { treasury: 8000, uniswap_lp: 1500, cash: 500 },
        }
    }

    /// The same explicit numbers without a floor.
    fn operator_profile_no_floor() -> OperatorProfile {
        OperatorProfile { treasury_floor_nock: 0.0, ..operator_profile_2026_11() }
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
        // #95: no P/L claim, no yield claim, no "Uniswap position(s)".
        let d = &metrics.global_status.shot_advice_details;
        for banned in ["realized", "0%", "Uniswap position", "generating yield"] {
            assert!(!d.contains(banned), "{banned} in: {d}");
        }
        // Pools were not read in this fixture: an unread pool does not disprove activity,
        // the same rule the "N in range" count uses, so the label matches the details.
        assert_eq!(metrics.global_status.shot_advice_action, "HOLD & ACCRETE (Dual Growth Engine Active)");
        assert!(d.contains("4 ranged (4 in range)"), "{d}");
    }

    #[test]
    fn test_hold_advice_states_no_pnl_and_names_lp_positions() {
        // Issue #95: HOLD is built from the positions. Four ranged positions on live
        // pools plus an Aero full-range pool; one pool then reads zero liquidity.
        let mut raw = four_position_fixture();
        for p in &raw.positions {
            raw.pool_liquidity.insert(p.position_id.clone(), "80173935548453779".to_string());
        }
        let aero = aero_position(false);
        raw.pool_liquidity.insert(aero.position_id.clone(), "342542824101829799".to_string());
        raw.positions.push(aero);
        let m = compute_derived_metrics(&raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        assert_eq!(m.global_status.shot_advice_action, "HOLD & ACCRETE (Dual Growth Engine Active)");
        let d = &m.global_status.shot_advice_details;
        assert!(d.starts_with("Total NOCK held is "), "{d}");
        assert!(d.contains("; no floor configured."), "{d}");
        assert!(d.contains("5 LP positions tracked: 4 ranged (4 in range), 1 full range."), "{d}");
        assert!(d.contains("of LP fees uncollected."), "{d}");
        assert!(d.contains("Liquid dry powder: "), "{d}");
        for banned in ["realized", "0%", "Uniswap position", "generating yield", "Maintain posture", "hard floor"] {
            assert!(!d.contains(banned), "{banned} in: {d}");
        }

        // A stale pool is called stale, and the label stops claiming growth once no
        // ranged position is confirmed in range on a live pool.
        raw.pool_liquidity.insert("uniswap-v4:base:3163431".to_string(), "0".to_string());
        let m = compute_derived_metrics(&raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        assert!(m.global_status.shot_advice_details.contains("4 ranged (3 in range, 1 on a stale pool), 1 full range"));
        for p in raw.positions.iter().filter(|p| !is_full_range(p.tick_lower, p.tick_upper)).map(|p| p.position_id.clone()).collect::<Vec<_>>() {
            raw.pool_liquidity.insert(p, "0".to_string());
        }
        let m = compute_derived_metrics(&raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        assert_eq!(m.global_status.shot_advice_action, "HOLD & ACCRETE (No Ranged LP Confirmed In Range)");
    }

    #[test]
    fn test_hold_advice_replays_the_2026_11_14_empty_book() {
        // 05:36Z on 2026-11-14: four tracked positions, all emptied, and HOLD said
        // "4 Uniswap position(s) generating yield ... 0% realized loss". With the plan
        // complete (no dip arm), HOLD now states the empty book.
        let raw = liquid_wallet_fixture();
        let fills = PlanFills { count: 5, spent_usdc: 500.0, acquired_nock: 40500.0, last_at: Some(t("2026-11-15T12:00:00Z")), first_price: Some(0.0125) };
        let ctx = PlanContext { cfg: Some(plan_cfg()), fills, ledger_unavailable: false, now: t("2026-11-20T12:00:00Z") };
        let m = metrics_with_plan(&raw, &ctx, &MarketRegime::default());
        assert_eq!(m.global_status.shot_advice_action, "HOLD & ACCRETE (LP Empty)");
        let d = &m.global_status.shot_advice_details;
        assert!(d.starts_with("Total NOCK held is 115748 (+15748 above the 100000 floor). 4 LP positions tracked: 4 ranged (0 in range, 4 empty). $0.00 of LP fees uncollected. Liquid dry powder: $929.22 (wallet USDC $1053.00 less the larger of the $123.78 cash target and the $0.00 virtual tax reserve)."), "{d}");
        for banned in ["realized", "generating yield", "Uniswap position", "Deploy $", "120,000", "CRITICAL"] {
            assert!(!d.contains(banned), "{banned} in: {d}");
        }
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
        assert!(names.iter().any(|n| n.contains("LP (all tracked positions)")));
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
        // aerodrome_lp/uniswap_v4_lp are ignored; an unset uniswap_lp is no target (0).
        assert_eq!(profile.target_weights_bps.uniswap_lp, 0);
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
        assert_eq!(metrics.portfolio_deviation_matrix.allocations[1].target_weight_bps, 0);
        assert_eq!(metrics.portfolio_deviation_matrix.allocations[1].status, "no_target");
        assert_eq!(metrics.portfolio_deviation_matrix.allocations[2].target_weight_bps, 500);
    }

    #[test]
    fn test_operator_profile_default_tax_reserve_ratio() {
        // R11: configs without the fields still parse, and the code chooses nothing:
        // no tax ratio, no target weights, no floor, no wallet, no plan.
        let yaml = r#"
database_url: "postgresql://localhost/test"
"#;
        let cfg: AppConfig = serde_yaml::from_str(yaml).expect("Failed to deserialize yaml");
        let p = cfg.profile();
        assert_eq!(p.tax_reserve_ratio, 0.0);
        assert_eq!(p.treasury_floor_nock, 0.0);
        assert_eq!((p.target_weights_bps.treasury, p.target_weights_bps.uniswap_lp, p.target_weights_bps.cash), (0, 0, 0));
        assert!(cfg.tracked_wallets.is_empty());
        assert!(cfg.accumulation_plan.is_none());
        let d = OperatorProfile::default();
        assert_eq!((d.tax_reserve_ratio, d.treasury_floor_nock, d.target_weights_bps.cash), (0.0, 0.0, 0));
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
            unpriced_count: 0,
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
        // Issue #63: only wallet USDC backs the virtual tax reserve. USDC inside LP
        // (B+D in this fixture) does not cover it and is not dry powder.
        let mut raw = four_position_fixture();
        raw.spot_price_nock_usd = 0.015; // dip zone (< 0.020)
        raw.wallet_balances[1].amount = "10.0".to_string();
        let profile = operator_profile_no_floor(); // ratio 0.40
        let ledger = LedgerAggregate { total_usd: 500.0, harvest_count: 3, last_at: None, unpriced_count: 0 };
        let metrics = compute_derived_metrics(
            &raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[], &ledger, &MarketRegime::default(),
        );
        assert_eq!(metrics.global_status.shot_advice_action, "TAX RESERVE SHORT (No Buy Advised)");
        let d = &metrics.global_status.shot_advice_details;
        assert!(d.contains("the virtual tax reserve ($200.00) exceeds liquid wallet USDC ($10.00)."), "{d}");
        assert!(d.contains("of USDC inside LP is working capital, not dry powder"), "{d}");
        assert!(d.contains("No buy is advised while wallet USDC is below the reserve."), "{d}");
        assert!(!d.contains("60/40"));
        assert!(!d.contains("SWEEP") && !d.contains("sweep"), "{d}");

        // A $16 reserve still exceeds the $10 wallet: LP USDC no longer covers it.
        let small = LedgerAggregate { total_usd: 40.0, harvest_count: 1, last_at: None, unpriced_count: 0 };
        let metrics = compute_derived_metrics(
            &raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[], &small, &MarketRegime::default(),
        );
        assert_eq!(metrics.global_status.shot_advice_action, "TAX RESERVE SHORT (No Buy Advised)");
        assert!(metrics.global_status.shot_advice_details.contains("exceeds liquid wallet USDC ($10.00)"));

        // $50 covers the reserve but not the cash target: nothing is dry powder, so the
        // engine holds and says so.
        raw.wallet_balances[1].amount = "50.0".to_string();
        let metrics = compute_derived_metrics(
            &raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[], &small, &MarketRegime::default(),
        );
        assert!(metrics.global_status.shot_advice_action.starts_with("HOLD & ACCRETE"));
        let total: f64 = metrics.global_status.total_portfolio_usd.parse().unwrap();
        assert!(
            metrics.global_status.shot_advice_details.contains(&format!(
                "Liquid dry powder: none (wallet USDC $50.00 less the larger of the ${:.2} cash target and the $16.00 virtual tax reserve).",
                total * 0.05
            )),
            "{}", metrics.global_status.shot_advice_details
        );

        // Liquid USDC well above the cash target: DIP ZONE states the dry powder and
        // sizes nothing (no lump-sum "Deploy $" without a plan).
        raw.wallet_balances[1].amount = "5000.0".to_string();
        let metrics = compute_derived_metrics(
            &raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[], &small, &MarketRegime::default(),
        );
        assert_eq!(metrics.global_status.shot_advice_action, "DIP ZONE (No Buy Plan Configured)");
        let total: f64 = metrics.global_status.total_portfolio_usd.parse().unwrap();
        let d = &metrics.global_status.shot_advice_details;
        assert!(
            d.contains(&format!(
                "Liquid dry powder: ${:.2} (wallet USDC $5000.00 less the larger of the ${:.2} cash target and the $16.00 virtual tax reserve).",
                5000.0 - total * 0.05,
                total * 0.05
            )),
            "{d}"
        );
        for banned in ["Deploy $", "120,000", "at discount", "CRITICAL"] {
            assert!(!d.contains(banned), "{banned} in: {d}");
        }
    }

    // ---------------- Accumulation plan (issue #79) ----------------

    fn plan_cfg() -> AccumulationPlanConfig {
        AccumulationPlanConfig {
            id: "test-plan".to_string(),
            budget_usdc: 500.0,
            tranches: 5,
            interval_hours: 24.0,
            starts_at: None,
            venue: "Aero".to_string(),
            max_price_usd: None,
            pause_when_overextended: true,
            respect_cash_target: true,
            cash_reserve_usdc: None,
        }
    }

    #[test]
    fn test_plan_never_draws_below_the_protected_cash_reserve() {
        // The operator's 2026-11-14 setup: $1,053 liquid, $950 protected, $100 over 5 buys.
        let mut cfg = plan_cfg();
        cfg.budget_usdc = 100.0;
        cfg.cash_reserve_usdc = Some(950.0);
        let now = t("2026-11-15T12:00:00Z");
        // Nominal $20 fits above the reserve ($103 of headroom): full tranche, no limit named.
        assert_eq!(
            plan_directive(&cfg, &PlanFills::default(), &roomy_guards(), false, now),
            PlanDirective::TrancheDue { usdc: 20.0, limited_by: None }
        );
        // Headroom shrinks to $8 (e.g. USDC moved into LP): the tranche shrinks and says why.
        let mut guards = roomy_guards();
        guards.wallet_usdc = 958.0;
        match plan_directive(&cfg, &PlanFills::default(), &guards, false, now) {
            PlanDirective::TrancheDue { usdc, limited_by } => {
                assert!((usdc - 8.0).abs() < 1e-9);
                assert!(limited_by.unwrap().contains("protected cash reserve"));
            }
            other => panic!("expected a shrunk tranche, got {other:?}"),
        }
        // At or below the reserve nothing is spendable, however cheap NOCK is.
        guards.wallet_usdc = 950.0;
        guards.spot_price = 0.0001;
        let d = plan_directive(&cfg, &PlanFills::default(), &guards, false, now);
        assert!(matches!(d, PlanDirective::Blocked { ref reason } if reason.contains("protected cash reserve")));
        // The reserve binds even when the operator opts out of the 5% cash target.
        cfg.respect_cash_target = false;
        assert!(matches!(
            plan_directive(&cfg, &PlanFills::default(), &guards, false, now),
            PlanDirective::Blocked { .. }
        ));
        // A reserve protects against an oversized budget too: the example plan
        // ($500 / 5) can only ever take the $103 above a $950 reserve.
        let mut big = plan_cfg();
        big.cash_reserve_usdc = Some(950.0);
        match plan_directive(&big, &PlanFills::default(), &roomy_guards(), false, now) {
            PlanDirective::TrancheDue { usdc, limited_by } => {
                assert!((usdc - 100.0).abs() < 1e-9 && limited_by.is_none());
            }
            other => panic!("{other:?}"),
        }
        let mut after_one = roomy_guards();
        after_one.wallet_usdc = 953.0; // one $100 buy later
        match plan_directive(&big, &PlanFills { count: 1, spent_usdc: 100.0, acquired_nock: 8000.0, last_at: Some(t("2026-11-14T00:00:00Z")), first_price: Some(0.0125) }, &after_one, false, now) {
            PlanDirective::TrancheDue { usdc, .. } => assert!((usdc - 3.0).abs() < 1e-9),
            other => panic!("{other:?}"),
        }
        // Negative reserve is a config error, stated explicitly.
        cfg.cash_reserve_usdc = Some(-1.0);
        assert!(matches!(plan_directive(&cfg, &PlanFills::default(), &roomy_guards(), false, now), PlanDirective::Invalid { .. }));
    }

    #[test]
    fn test_advice_states_the_protected_reserve() {
        let raw = liquid_wallet_fixture();
        let mut cfg = plan_cfg();
        cfg.budget_usdc = 100.0;
        cfg.cash_reserve_usdc = Some(950.0);
        let ctx = PlanContext { cfg: Some(cfg), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-15T12:00:00Z") };
        let m = metrics_with_plan(&raw, &ctx, &MarketRegime::default());
        assert_eq!(m.global_status.shot_advice_action, "DCA TRANCHE DUE (1 of 5)");
        assert!(m.global_status.shot_advice_details.contains("Buy $20.00 of NOCK"));
        assert!(m.global_status.shot_advice_details.contains("never spends the stable balance (wallet + LP USDC) below $950.00"));
        assert_eq!(m.accumulation_plan.unwrap().cash_reserve_usdc.as_deref(), Some("950.00"));
    }

    fn roomy_guards() -> PlanGuards {
        PlanGuards { overextended: false, spot_price: 0.0123, wallet_usdc: 1053.0, lp_usdc: 0.0, tax_reserve_usd: 0.0, cash_target_usd: 124.0 }
    }

    fn t(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn test_plan_first_tranche_due_immediately_at_nominal_size() {
        let d = plan_directive(&plan_cfg(), &PlanFills::default(), &roomy_guards(), false, t("2026-11-15T12:00:00Z"));
        assert_eq!(d, PlanDirective::TrancheDue { usdc: 100.0, limited_by: None });
    }

    #[test]
    fn test_plan_pending_before_start() {
        let mut cfg = plan_cfg();
        cfg.starts_at = Some(t("2026-11-20T00:00:00Z"));
        let d = plan_directive(&cfg, &PlanFills::default(), &roomy_guards(), false, t("2026-11-15T12:00:00Z"));
        assert_eq!(d, PlanDirective::Pending { starts_at: t("2026-11-20T00:00:00Z") });
    }

    #[test]
    fn test_plan_waits_out_the_interval_then_is_due() {
        let fills = PlanFills { count: 1, spent_usdc: 100.0, acquired_nock: 8000.0, last_at: Some(t("2026-11-15T12:00:00Z")), first_price: Some(0.0125) };
        let early = plan_directive(&plan_cfg(), &fills, &roomy_guards(), false, t("2026-11-16T11:59:00Z"));
        assert_eq!(early, PlanDirective::Waiting { next_due_at: t("2026-11-16T12:00:00Z") });
        let on_time = plan_directive(&plan_cfg(), &fills, &roomy_guards(), false, t("2026-11-16T12:00:00Z"));
        assert_eq!(on_time, PlanDirective::TrancheDue { usdc: 100.0, limited_by: None });
    }

    #[test]
    fn test_plan_missed_windows_do_not_stack() {
        // Operator away for ten intervals: exactly one nominal tranche is due, never ten.
        let fills = PlanFills { count: 1, spent_usdc: 100.0, acquired_nock: 8000.0, last_at: Some(t("2026-11-15T12:00:00Z")), first_price: Some(0.0125) };
        let d = plan_directive(&plan_cfg(), &fills, &roomy_guards(), false, t("2026-11-25T12:00:00Z"));
        assert_eq!(d, PlanDirective::TrancheDue { usdc: 100.0, limited_by: None });
    }

    #[test]
    fn test_plan_pauses_when_overextended_unless_opted_out() {
        let mut guards = roomy_guards();
        guards.overextended = true;
        let d = plan_directive(&plan_cfg(), &PlanFills::default(), &guards, false, t("2026-11-15T12:00:00Z"));
        assert!(matches!(d, PlanDirective::Paused { ref reason } if reason.contains("RSI")));
        let mut cfg = plan_cfg();
        cfg.pause_when_overextended = false;
        let d = plan_directive(&cfg, &PlanFills::default(), &guards, false, t("2026-11-15T12:00:00Z"));
        assert!(matches!(d, PlanDirective::TrancheDue { .. }));
    }

    #[test]
    fn test_plan_pauses_above_price_ceiling() {
        let mut cfg = plan_cfg();
        cfg.max_price_usd = Some(0.0100);
        let d = plan_directive(&cfg, &PlanFills::default(), &roomy_guards(), false, t("2026-11-15T12:00:00Z"));
        assert!(matches!(d, PlanDirective::Paused { ref reason } if reason.contains("ceiling")));
    }

    #[test]
    fn test_plan_tranche_shrinks_to_cash_target_and_names_it() {
        // $160 liquid, $124 cash target -> only $36 spendable, below the $100 nominal.
        let mut guards = roomy_guards();
        guards.wallet_usdc = 160.0;
        match plan_directive(&plan_cfg(), &PlanFills::default(), &guards, false, t("2026-11-15T12:00:00Z")) {
            PlanDirective::TrancheDue { usdc, limited_by } => {
                assert!((usdc - 36.0).abs() < 1e-9);
                assert!(limited_by.unwrap().contains("cash target"));
            }
            other => panic!("expected a shrunk tranche, got {other:?}"),
        }
        // Opting out of the cash target restores the nominal size.
        let mut cfg = plan_cfg();
        cfg.respect_cash_target = false;
        assert_eq!(
            plan_directive(&cfg, &PlanFills::default(), &guards, false, t("2026-11-15T12:00:00Z")),
            PlanDirective::TrancheDue { usdc: 100.0, limited_by: None }
        );
    }

    #[test]
    fn test_plan_blocked_when_nothing_is_spendable() {
        let mut guards = roomy_guards();
        guards.wallet_usdc = 100.0; // below the $124 cash target
        let d = plan_directive(&plan_cfg(), &PlanFills::default(), &guards, false, t("2026-11-15T12:00:00Z"));
        assert!(matches!(d, PlanDirective::Blocked { ref reason } if reason.contains("cash target")));
        // Tax reserve larger than the wallet blocks too, and is named.
        let mut guards = roomy_guards();
        guards.tax_reserve_usd = 2000.0;
        let d = plan_directive(&plan_cfg(), &PlanFills::default(), &guards, false, t("2026-11-15T12:00:00Z"));
        assert!(matches!(d, PlanDirective::Blocked { ref reason } if reason.contains("tax reserve")));
    }

    #[test]
    fn test_plan_last_tranche_is_capped_to_remaining_budget_then_completes() {
        let fills = PlanFills { count: 5, spent_usdc: 470.0, acquired_nock: 38000.0, last_at: Some(t("2026-11-10T00:00:00Z")), first_price: Some(0.0125) };
        match plan_directive(&plan_cfg(), &fills, &roomy_guards(), false, t("2026-11-15T12:00:00Z")) {
            PlanDirective::TrancheDue { usdc, limited_by } => {
                assert!((usdc - 30.0).abs() < 1e-9);
                assert!(limited_by.unwrap().contains("plan budget remains"));
            }
            other => panic!("expected the remainder tranche, got {other:?}"),
        }
        let done = PlanFills { spent_usdc: 499.60, ..fills };
        assert_eq!(plan_directive(&plan_cfg(), &done, &roomy_guards(), false, t("2026-11-15T12:00:00Z")), PlanDirective::Complete);
    }

    #[test]
    fn test_plan_invalid_config_and_unreadable_ledger_are_explicit() {
        let mut cfg = plan_cfg();
        cfg.tranches = 0;
        assert!(matches!(
            plan_directive(&cfg, &PlanFills::default(), &roomy_guards(), false, t("2026-11-15T12:00:00Z")),
            PlanDirective::Invalid { .. }
        ));
        // Zero-mock: an unreadable ledger must never look like "no buys yet".
        let d = plan_directive(&plan_cfg(), &PlanFills::default(), &roomy_guards(), true, t("2026-11-15T12:00:00Z"));
        assert!(matches!(d, PlanDirective::Blocked { ref reason } if reason.contains("ledger")));
    }

    #[test]
    fn test_plan_status_reports_average_cost_and_lump_sum_counterfactual() {
        let fills = PlanFills { count: 2, spent_usdc: 200.0, acquired_nock: 17000.0, last_at: Some(t("2026-11-16T12:00:00Z")), first_price: Some(0.0125) };
        let d = plan_directive(&plan_cfg(), &fills, &roomy_guards(), false, t("2026-11-16T13:00:00Z"));
        let ps = build_plan_status(&plan_cfg(), &fills, &d);
        assert_eq!(ps.status, "waiting");
        assert_eq!(ps.tranche_index, 3);
        assert_eq!(ps.tranches_total, 5);
        assert_eq!(ps.spent_usdc, "200.00");
        assert_eq!(ps.remaining_usdc, "300.00");
        assert_eq!(ps.avg_price_usd.as_deref(), Some("0.011765")); // 200 / 17000
        assert_eq!(ps.lump_sum_nock.as_deref(), Some("40000.0000")); // 500 / 0.0125
        assert_eq!(ps.next_due_at.as_deref(), Some("2026-11-17T12:00:00+00:00"));
        // Before any fill there is no average and no counterfactual (never fabricated).
        let empty = build_plan_status(&plan_cfg(), &PlanFills::default(), &PlanDirective::TrancheDue { usdc: 100.0, limited_by: None });
        assert_eq!(empty.avg_price_usd, None);
        assert_eq!(empty.lump_sum_nock, None);
        assert_eq!(empty.tranche_usdc, "100.00");
    }

    #[test]
    fn test_accumulation_plan_config_parses_with_defaults() {
        let cfg: AppConfig = serde_yaml::from_str(
            "accumulation_plan:\n  id: reentry\n  budget_usdc: 500\n  tranches: 5\n  interval_hours: 24\n",
        )
        .unwrap();
        let plan = cfg.accumulation_plan.expect("plan parsed");
        assert_eq!(plan.venue, "Aero");
        assert!(plan.pause_when_overextended && plan.respect_cash_target);
        assert!(plan.starts_at.is_none() && plan.max_price_usd.is_none());
        // Absent section keeps the feature off.
        let none: AppConfig = serde_yaml::from_str("port: 8080\n").unwrap();
        assert!(none.accumulation_plan.is_none());
    }

    fn liquid_wallet_fixture() -> RawOnChainData {
        // The 2026-11-14 wallet: fully liquid, every tracked position emptied.
        let mut raw = four_position_fixture();
        raw.spot_price_nock_usd = 0.01229;
        for p in raw.positions.iter_mut() {
            p.liquidity = 0;
            p.amount0 = 0.0;
            p.amount1 = 0.0;
            p.fee0 = 0.0;
            p.fee1 = 0.0;
        }
        raw.wallet_balances[0].amount = "115747.8001".to_string();
        raw.wallet_balances[1].amount = "1053.0".to_string();
        raw
    }

    fn metrics_with_plan(raw: &RawOnChainData, ctx: &PlanContext, regime: &MarketRegime) -> DerivedMetrics {
        compute_derived_metrics_with_plan(
            raw, 0.0, &operator_profile_2026_11(), StructuralChannels { channels: vec![] }, &[],
            &LedgerAggregate::default(), regime, ctx,
        )
    }

    #[test]
    fn test_advice_plan_replaces_lump_sum_dip_shot() {
        let raw = liquid_wallet_fixture();
        assert_eq!(raw.wallet_balances[0].token_symbol, "NOCK");
        assert_eq!(raw.wallet_balances[1].token_symbol, "USDC");
        // No plan: the dip zone is stated as a fact and nothing is sized (v0.5.1).
        let legacy = metrics_with_plan(&raw, &PlanContext::none(), &MarketRegime::default());
        assert_eq!(legacy.global_status.shot_advice_action, "DIP ZONE (No Buy Plan Configured)");
        assert!(!legacy.global_status.shot_advice_details.contains("Deploy $"));
        assert!(legacy.accumulation_plan.is_none());

        let ctx = PlanContext { cfg: Some(plan_cfg()), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-15T12:00:00Z") };
        let m = metrics_with_plan(&raw, &ctx, &MarketRegime::default());
        assert_eq!(m.global_status.shot_advice_action, "DCA TRANCHE DUE (1 of 5)");
        assert!(m.global_status.shot_advice_details.contains("Buy $100.00 of NOCK on Aero (spot"));
        assert!(!m.global_status.shot_advice_details.contains("Deploy $"));
        let ps = m.accumulation_plan.expect("plan status emitted");
        assert_eq!(ps.status, "tranche_due");
        assert_eq!(ps.tranche_usdc, "100.00");
    }

    #[test]
    fn test_advice_lp_rebuild_bias_leads_when_idle_and_rides_along_when_due() {
        // LP is 0% against a 15% target, and pools are readable, so deposits can be suggested.
        // (Without a fundable suggestion the rebuild note never leads; see
        // test_rebuild_does_not_lead_or_nag_when_nothing_is_fundable.)
        let raw = rebuild_fixture();
        // A due tranche stays the primary directive; the rebuild bias is appended.
        let due = PlanContext { cfg: Some(plan_cfg()), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-15T12:00:00Z") };
        let m = metrics_with_plan(&raw, &due, &MarketRegime::default());
        assert!(m.global_status.shot_advice_action.starts_with("DCA TRANCHE DUE"));
        assert!(m.global_status.shot_advice_details.contains("Standing bias: LP holds $0.00"));
        assert_eq!(m.lp_rebuild.status, "under_target");

        // Between tranches there is nothing to buy, so the rebuild leads.
        let fills = PlanFills { count: 1, spent_usdc: 100.0, acquired_nock: 8000.0, last_at: Some(t("2026-11-15T12:00:00Z")), first_price: Some(0.0125) };
        let waiting = PlanContext { cfg: Some(plan_cfg()), fills, ledger_unavailable: false, now: t("2026-11-15T18:00:00Z") };
        let m = metrics_with_plan(&raw, &waiting, &MarketRegime::default());
        assert_eq!(m.global_status.shot_advice_action, "REBUILD LP TRANCHES (No Net Cash Change)");
        assert!(m.global_status.shot_advice_details.contains("reposition, not a purchase"));
        assert!(m.global_status.shot_advice_details.contains("DCA ON SCHEDULE (2 of 5 next)"));
        assert_eq!(m.accumulation_plan.unwrap().status, "waiting");

        // Safety directives are never displaced, only annotated.
        let hot = MarketRegime { rsi_5m: None, rsi_1d: Some(82.0), overextended: true };
        let m = metrics_with_plan(&raw, &due, &hot);
        assert_eq!(m.global_status.shot_advice_action, "OVEREXTENDED (Daily RSI ≥ 70, No Buy Advised)");
        assert!(m.global_status.shot_advice_details.contains("Standing bias"));
        assert_eq!(m.accumulation_plan.unwrap().status, "paused");
    }

    #[test]
    fn test_advice_completed_plan_does_not_fall_back_to_lump_sum() {
        let raw = liquid_wallet_fixture();
        let fills = PlanFills { count: 5, spent_usdc: 500.0, acquired_nock: 40500.0, last_at: Some(t("2026-11-15T12:00:00Z")), first_price: Some(0.0125) };
        let ctx = PlanContext { cfg: Some(plan_cfg()), fills, ledger_unavailable: false, now: t("2026-11-20T12:00:00Z") };
        let m = metrics_with_plan(&raw, &ctx, &MarketRegime::default());
        assert!(!m.global_status.shot_advice_action.contains("DIP REINVESTMENT SHOT"));
        assert!(!m.global_status.shot_advice_action.starts_with("DCA"));
        let ps = m.accumulation_plan.unwrap();
        assert_eq!(ps.status, "complete");
        assert_eq!(ps.tranche_index, 5);
    }

    #[test]
    fn test_lp_rebuild_status_uses_the_deviation_matrix_tolerance() {
        // 15% target on $1,000: tolerance is 20% of target = 300 bps -> under below 12%.
        assert_eq!(lp_rebuild_status(0.0, 1000.0, 1500).status, "under_target");
        assert_eq!(lp_rebuild_status(119.0, 1000.0, 1500).status, "under_target");
        assert_eq!(lp_rebuild_status(130.0, 1000.0, 1500).status, "on_target");
        assert_eq!(lp_rebuild_status(200.0, 1000.0, 1500).status, "over_target");
        let lp = lp_rebuild_status(0.0, 2475.27, 1500);
        assert_eq!(lp.target_usd, "371.29");
        assert_eq!(lp.gap_usd, "371.29");
        // On target: the bias leaves advice untouched.
        let on = lp_rebuild_status(150.0, 1000.0, 1500);
        let (a, d) = apply_lp_rebuild_bias("HOLD & ACCRETE (x)".to_string(), "details".to_string(), &on);
        assert_eq!((a.as_str(), d.as_str()), ("HOLD & ACCRETE (x)", "details"));
    }

    #[test]
    fn test_plan_anchor_prefers_operator_start_over_first_sight() {
        let registered = t("2026-11-14T05:00:00Z");
        // No starts_at: buys count from the moment the worker first saw the plan, so a
        // week-old swap (the 2026-11-06 hash used in the playtest) is before the anchor.
        assert_eq!(plan_anchor(None, registered), registered);
        assert!(t("2026-11-06T03:23:19Z") < plan_anchor(None, registered));
        // An explicit starts_at wins, including one that reaches back on purpose.
        assert_eq!(plan_anchor(Some(t("2026-11-01T00:00:00Z")), registered), t("2026-11-01T00:00:00Z"));
        assert!(t("2026-11-06T03:23:19Z") >= plan_anchor(Some(t("2026-11-01T00:00:00Z")), registered));
    }

    // ---------------- Pre-0.5.0 fixes (issues #85, #86, #87) ----------------

    #[test]
    fn test_wallet_nock_series_is_token_units_not_dollars() {
        let raw = liquid_wallet_fixture(); // 115,747.8001 NOCK at $0.01229
        let m = metrics_with_plan(&raw, &PlanContext::none(), &MarketRegime::default());
        let ag = &m.asset_aggregation_series;
        assert_eq!(ag.wallet_nock_series, vec!["115747.8001".to_string()]);
        // The USD series is unchanged and is NOT the token count.
        let usd: f64 = ag.safe_haven_series[0].parse().unwrap();
        assert!((usd - 115_747.8001 * 0.01229).abs() < 0.01);
        assert!(usd < 2_000.0);
    }

    #[test]
    fn test_example_config_is_the_base_layer() {
        let dir = std::env::temp_dir().join(format!("nocknock-cfg-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let example = dir.join("config.yaml.example");
        let main = dir.join("config.yaml");
        std::fs::write(
            &example,
            "alchemy_api_keys: [\"YOUR_ALCHEMY_API_KEY\"]\nport: 8080\nsync_window_seconds: 300\npositions:\n  - id: \"p1\"\n    protocol: \"uniswap-v4\"\n    nft_id: 3163450\n    is_price_source: true\noperator_profile:\n  treasury_floor_nock: 100000.0\n",
        )
        .unwrap();
        // A partial operator config: key and port only, exactly the shape that failed
        // ingestion on 2026-11-14 with "No positions configured".
        std::fs::write(&main, "alchemy_api_keys: [\"real-key\"]\nport: 8081\noperator_profile:\n  lp_floor_tolerance_bps: 500\n").unwrap();
        let cfg = load_app_config(example.to_str().unwrap(), Some(main.to_str().unwrap()), false).unwrap();
        assert_eq!(cfg.positions.len(), 1, "positions must come from the example base layer");
        assert_eq!(cfg.positions[0].nft_id, 3163450);
        assert_eq!(cfg.alchemy_api_keys, vec!["real-key".to_string()]);
        assert_eq!(cfg.port, 8081);
        // Nested sections merge key by key.
        assert_eq!(cfg.operator_profile.treasury_floor_nock, 100000.0);
        assert_eq!(cfg.operator_profile.lp_floor_tolerance_bps, 500);
        // With no config.yaml at all the example alone still parses.
        let only = load_app_config(example.to_str().unwrap(), Some(dir.join("absent.yaml").to_str().unwrap()), false).unwrap();
        assert_eq!(only.positions.len(), 1);
        assert_eq!(only.port, 8080);
        let _ = std::fs::remove_file(&example);
        let _ = std::fs::remove_file(&main);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn test_health_status_is_not_ok_when_ingestion_is_failing_or_stale() {
        // Healthy: fresh snapshot, last cycle fine.
        assert_eq!(health_status(true, 10, 300, false), "ok");
        assert_eq!(health_status(true, 390, 300, false), "ok"); // inside window + 90s grace
        // The 2026-11-14 incident: old snapshot, every cycle failing, lag 491s.
        assert_eq!(health_status(true, 491, 300, true), "degraded");
        // Failing counts even while the last snapshot is still recent.
        assert_eq!(health_status(true, 5, 300, true), "degraded");
        // Stale without a recorded error (scheduler stuck) is degraded too.
        assert_eq!(health_status(true, 391, 300, false), "degraded");
        // Boot: nothing yet and nothing failed is "starting"; a failed first cycle is not.
        assert_eq!(health_status(false, 0, 300, false), "starting");
        assert_eq!(health_status(false, 0, 300, true), "degraded");
    }

    #[test]
    fn test_nothing_to_collect_only_for_truly_empty_positions() {
        assert!(nothing_to_collect("0", "0.000000", "0.000000"));
        assert!(!nothing_to_collect("0", "0.000001", "0.000000")); // fees left behind
        assert!(!nothing_to_collect("48295502692597", "0.000000", "0.000000")); // funded
        assert!(!nothing_to_collect("not-a-number", "0", "0")); // unknown is not "empty"
    }

    // ---------------- Aerodrome full-range tranche (issue #88) ----------------

    fn aero_cfg(price_source: bool) -> PositionConfig {
        serde_yaml::from_str(&format!(
            "id: aerodrome-vamm-nock-usdc\nprotocol: aerodrome\npool_address: \"0x85f1aa3a70fedd1c52705c15baed143e675cd626\"\nis_price_source: {price_source}\n"
        ))
        .unwrap()
    }

    /// The legacy vAMM as read on 2026-11-14 (block 57409285): 369,321.72 USDC and
    /// 31,839,446 NOCK in reserve, LP supply 342542824101829799; wallet given 1%.
    fn aero_position(price_source: bool) -> RawPosition {
        aerodrome_position_from_reads(
            &aero_cfg(price_source),
            "0xeE114cC97eC5adeB34bA5645e315B497D8487a45",
            "0x85f1aa3a70fedd1c52705c15baed143e675cd626",
            token_info(USDC_BASE).unwrap(),
            token_info(NOCK_BASE).unwrap(),
            369_321_720_000,
            318_394_460_000_000_000_000_000,
            342_542_824_101_829_799,
            3_425_428_241_018_297, // 1% of supply
            440_190,               // 0.44019 USDC of fees
            250_000_000_000_000_000, // 25 NOCK of fees
        )
        .unwrap()
    }

    #[test]
    fn test_aerodrome_position_is_a_share_of_reserves_and_full_range() {
        let p = aero_position(false);
        assert_eq!(p.protocol, "aerodrome");
        assert_eq!(p.position_id, "aerodrome:base:0x85f1aa3a70fedd1c52705c15baed143e675cd626");
        assert_eq!((p.token0.symbol.as_str(), p.token1.symbol.as_str()), ("USDC", "NOCK"));
        // 1% of each reserve.
        assert!((p.amount0 - 3693.2172).abs() < 0.01, "USDC {}", p.amount0);
        assert!((p.amount1 - 318_394.46).abs() < 1.0, "NOCK {}", p.amount1);
        assert!((p.fee0 - 0.44019).abs() < 1e-9 && (p.fee1 - 25.0).abs() < 1e-9);
        assert_eq!(p.fee_tier_bps, 30);
        // Full range: always in range, never "sells out".
        assert!(p.in_range && is_full_range(p.tick_lower, p.tick_upper));
        assert_eq!(nock_sell_out_multiple(&p), None);
        assert_eq!(position_label(&p), "Aero USDC/NOCK 0.30% (full range)");
        // An x*y=k position is 50/50 by value at the pool's own price.
        let spot = 369_321.72 / 31_839_446.0;
        let (usdc_per_usd, nock_per_usd) = amounts_per_usd(&p, spot, 1830.0).unwrap();
        assert!((usdc_per_usd - 0.5).abs() < 1e-6, "USDC leg {usdc_per_usd}");
        assert!((nock_per_usd * spot - 0.5).abs() < 1e-6);
        // Snapshot view flags it so clients do not draw a bracket or print a range.
        let snap = build_position_snapshots(&[p.clone()], spot, 1830.0);
        assert!(snap[0].full_range);
        assert_eq!((snap[0].lower_price.as_str(), snap[0].upper_price.as_str()), ("0", "0"));
        assert!((snap[0].amount_usd.parse::<f64>().unwrap() - 2.0 * 3693.2172).abs() < 0.05);
    }

    #[test]
    fn test_aerodrome_pool_can_be_the_price_source() {
        // The deepest NOCK/USDC market gives the spot: reserve ratio, to within float noise.
        let p = aero_position(true);
        let spot = nock_spot_from_positions(&[p]).expect("spot from reserves");
        assert!((spot - 369_321.72 / 31_839_446.0).abs() < 1e-9, "spot {spot}");
        assert!((spot - 0.0116).abs() < 0.0001);
    }

    #[test]
    fn test_aerodrome_rejects_an_empty_pool_and_handles_no_lp() {
        let empty = aerodrome_position_from_reads(
            &aero_cfg(false), "0xw", "0xpool", token_info(USDC_BASE).unwrap(), token_info(NOCK_BASE).unwrap(),
            0, 0, 0, 0, 0, 0,
        );
        assert!(empty.is_err(), "an empty pool has no price and must not be read as zero");
        // Wallet holds no LP yet: a valid, empty position that still carries the pool price.
        let none = aerodrome_position_from_reads(
            &aero_cfg(true), "0xw", "0xpool", token_info(USDC_BASE).unwrap(), token_info(NOCK_BASE).unwrap(),
            369_321_720_000, 318_394_460_000_000_000_000_000, 342_542_824_101_829_799, 0, 0, 0,
        )
        .unwrap();
        assert_eq!((none.amount0, none.amount1, none.liquidity), (0.0, 0.0, 0));
        assert!(nock_spot_from_positions(&[none]).is_some());
    }

    // ---------------- v0.5.2 surfaces: Aero fees that cannot be claimed ----------------

    #[test]
    fn test_aero_claim_revert_is_not_an_rpc_outage() {
        let (sim, detail) = aero_claim_simulation(&Ok(None));
        assert_eq!(sim, "not_claimable");
        let d = detail.unwrap();
        assert!(d.contains("claimFees() reverts") && d.contains("AERO rewards not read (#93)"), "{d}");
        assert!(!d.contains("RPC unavailable"));
        assert_eq!(aero_claim_simulation(&Ok(Some("00".to_string()))), ("success", None));
        let (sim, detail) = aero_claim_simulation(&Err(anyhow::anyhow!("network error")));
        assert_eq!(sim, "unavailable");
        assert!(detail.unwrap().contains("RPC unavailable"));
    }

    fn claim_action(platform: &str, simulation: &str, gas_usd: Option<&str>) -> ClaimFeeAction {
        ClaimFeeAction {
            platform: platform.to_string(),
            label: platform.to_string(),
            to: "0x0".to_string(),
            data: "0x".to_string(),
            value: "0x0".to_string(),
            expected_amount_0: None,
            expected_amount_1: None,
            expected_usd: None,
            gas_estimate: None,
            gas_usd: gas_usd.map(|g| g.to_string()),
            simulation: simulation.to_string(),
            simulation_detail: None,
        }
    }

    #[test]
    fn test_claim_payload_totals_ignore_non_claimable_pools() {
        // The 2026-11-14 payload: v4 $0.0014, v3 $0.0027, and a reverting Aero claim
        // that used to null both totals.
        let actions = vec![
            claim_action("uniswap-v4", "success", Some("0.0014")),
            claim_action("uniswap-v3", "success", Some("0.0027")),
            claim_action("aerodrome", "not_claimable", None),
        ];
        let (gas, net) = claim_payload_totals(&actions, "0.30", true);
        assert_eq!(gas.as_deref(), Some("0.0041"));
        assert_eq!(net.as_deref(), Some("0.2959"));
        // An executable action whose estimate failed still makes the total unknown.
        let failed = vec![
            claim_action("uniswap-v4", "success", Some("0.0014")),
            claim_action("uniswap-v3", "success", None),
        ];
        assert_eq!(claim_payload_totals(&failed, "0.30", true), (None, None));
        // No live gas price or ETH price: unknown, never zero.
        assert_eq!(claim_payload_totals(&actions, "0.30", false), (None, None));
        // Nothing executable: no total to report.
        let only_aero = vec![claim_action("aerodrome", "not_claimable", None)];
        assert_eq!(claim_payload_totals(&only_aero, "0.00", true), (None, None));
    }

    #[test]
    fn test_reverting_aero_pool_is_not_shown_as_zero_earned() {
        let mut raw = four_position_fixture();
        raw.live_gas_cost_usd = 0.0037;
        let mut aero = aero_position(false);
        aero.fee0 = 0.0;
        aero.fee1 = 0.0;
        raw.fees_not_lp_claimable = vec![aero.position_id.clone()];
        raw.positions.push(aero.clone());
        let m = compute_derived_metrics(&raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        let contracts = &m.velocity_gas_sandbox.contracts;
        let row = contracts.iter().find(|c| c.contract_name.starts_with("Aero")).expect("aero row");
        assert_eq!(row.recommended_action, "not_claimable");
        // The Uniswap rows keep the gas economics.
        assert!(contracts.iter().filter(|c| !c.contract_name.starts_with("Aero")).all(|c| c.recommended_action != "not_claimable"));

        // Contract fees (the MCP claim simulation) say so too, with no gas counted.
        let snaps = build_position_snapshots(&[aero], 0.0116, 1830.0);
        let fees = build_contract_fees(&snaps, 0.0037, &raw.fees_not_lp_claimable);
        assert_eq!(fees[0].amount, AERO_NOT_LP_CLAIMABLE);
        assert!(!fees[0].claimable);
        let text = format_claim_simulation_text(&json!({ "contract_fees": fees }));
        assert!(text.contains("not LP-claimable (claimFees() reverts); AERO rewards not read (#93)"), "{text}");
        assert!(text.contains("Estimated Gas: unavailable"), "{text}");
        // Without the flag the same position reads as ordinary fees.
        let plain = build_contract_fees(&snaps, 0.0037, &[]);
        assert_ne!(plain[0].amount, AERO_NOT_LP_CLAIMABLE);
    }

    // ---------------- v0.5.2 surfaces: snapshot by id, first-run errors, MCP ----------------

    #[test]
    fn test_envelope_from_stored_round_trips_the_stored_payload() {
        // store_snapshot writes only the SnapshotData payload into `data`.
        let data = sample_snapshot_data();
        let stored = serde_json::to_value(&data).unwrap();
        let id = Uuid::new_v4();
        let at = DateTime::parse_from_rfc3339("2026-11-14T06:00:00Z").unwrap().with_timezone(&Utc);
        let env = envelope_from_stored(id, at, "ab".repeat(32), "2.2.0".to_string(), stored).expect("stored rows parse");
        assert_eq!(env.meta.snapshot_id, id);
        assert_eq!(env.meta.generated_at, at);
        assert_eq!(env.meta.version, "2.2.0");
        assert_eq!(
            serde_json::to_value(&env.data).unwrap(),
            serde_json::to_value(&data).unwrap()
        );
        // The old handler's assumption (a whole envelope in `data`) is not what is stored.
        let wrong = json!({ "meta": {}, "data": serde_json::to_value(&data).unwrap() });
        assert!(envelope_from_stored(id, at, String::new(), String::new(), wrong).is_err());
    }

    #[test]
    fn test_no_snapshot_body_carries_the_last_ingest_error() {
        let cause = "No NOCK/USD spot: exactly one configured position must have is_price_source: true";
        let body = no_snapshot_body(Some(cause));
        assert_eq!(body["error"], "NO_SNAPSHOT");
        assert_eq!(body["detail"], cause);
        assert!(no_snapshot_body(None).get("detail").is_none());
        assert!(no_snapshot_body(Some("  ")).get("detail").is_none());

        let mcp = no_snapshot_mcp_text(Some(cause));
        assert!(mcp.contains("No snapshot available yet") && mcp.contains(&format!("Last ingestion error: {cause}")), "{mcp}");
        assert!(!no_snapshot_mcp_text(None).contains("error"));
    }

    #[test]
    fn test_mcp_text_shows_snapshot_time_and_health_not_live_api() {
        let at = DateTime::parse_from_rfc3339("2026-11-14T06:00:00Z").unwrap().with_timezone(&Utc);
        let now = at + chrono::Duration::minutes(12);
        let line = snapshot_source_line(at, now, "degraded");
        assert_eq!(line, "Snapshot generated 2026-11-14T06:00:00Z (12 min ago) · worker health: degraded");
        let data = serde_json::to_value(sample_snapshot_data()).unwrap();
        for text in [format_portfolio_state_text(&data), format_shot_advice_text(&data)] {
            assert!(!text.contains("Live API") && !text.contains("Tier 1 Connected"), "{text}");
        }
        // The harvest rule names the precedence the engine applies.
        let advice = format_shot_advice_text(&data);
        assert!(advice.contains("only when no DCA tranche is due"), "{advice}");
        assert!(advice.contains("fixed $5.00 threshold"));
    }

    #[test]
    fn test_aerodrome_joins_the_rebuild_split_and_counts_toward_lp() {
        let mut raw = rebuild_fixture();
        let mut aero = aero_position(false);
        aero.amount0 = 0.0;
        aero.amount1 = 0.0;
        aero.liquidity = 0;
        raw.pool_liquidity.insert(aero.position_id.clone(), "342542824101829799".to_string());
        raw.positions.push(aero);
        let (s, x) = lp_rebuild_suggestions(&raw, 400.0, roomy_funds());
        // Two eligible positions: the dead 0.75% pool stays out, and the two ranges with
        // an ETH/WETH leg are gas-excluded (issue #94). Half each.
        assert_eq!(s.len(), 2);
        assert_eq!(x.iter().filter(|e| e.reason.contains("gas")).count(), 2);
        let a = s.iter().find(|x| x.protocol == "aerodrome").expect("aerodrome suggested");
        assert_eq!(a.target_usd, "200.00");
        assert_eq!(a.nock_sold_out_at_multiple, None);
        // The link is the pool's deposit page in the Aero app: never aerodrome.finance
        // (retired) and never a Uniswap link.
        assert_eq!(a.url, "https://app.aero.xyz/deposit/8453-0x85f1aa3a70fedd1c52705c15baed143e675cd626");
        assert!(!a.url.contains("aerodrome.finance") && !a.url.contains("uniswap"));
        // A config override wins.
        raw.rebuild_rules.insert(
            "aerodrome:base:0x85f1aa3a70fedd1c52705c15baed143e675cd626".to_string(),
            RebuildRule { weight_bps: None, when_out_of_range: false, deposit_url: Some("https://app.aero.xyz/portfolio".to_string()) },
        );
        let (o, _) = lp_rebuild_suggestions(&raw, 400.0, roomy_funds());
        assert_eq!(o.iter().find(|x| x.protocol == "aerodrome").unwrap().url, "https://app.aero.xyz/portfolio");
        raw.rebuild_rules.clear();
        // About half the deposit is USDC and half NOCK. The split is exactly 50/50 at
        // the pool's own price; this fixture's engine spot ($0.01229) sits 6% above the
        // pool's ($0.0116), so the NOCK leg is valued slightly higher here.
        let usdc_leg: f64 = a.amount0.parse().unwrap();
        assert!((90.0..=110.0).contains(&usdc_leg), "USDC leg {usdc_leg}");
        // A config weight makes it the core holding.
        raw.rebuild_rules.insert(
            "aerodrome:base:0x85f1aa3a70fedd1c52705c15baed143e675cd626".to_string(),
            RebuildRule { weight_bps: Some(7000), when_out_of_range: false, deposit_url: None },
        );
        let (s, _) = lp_rebuild_suggestions(&raw, 400.0, roomy_funds());
        assert_eq!(s.iter().find(|x| x.protocol == "aerodrome").unwrap().target_usd, "280.00");
        assert_eq!(sg(&s, 3163450).target_usd, "120.00");

        // Funded: its value and NOCK count in LP totals; its bracket is flagged full range.
        let mut funded = rebuild_fixture();
        funded.positions.push(aero_position(false));
        let m = metrics_with_plan(&funded, &PlanContext::none(), &MarketRegime::default());
        let lp_usd: f64 = m.asset_aggregation_series.active_protocol_series[0].parse().unwrap();
        assert!(lp_usd > 3693.0, "LP total must include the Aerodrome USDC leg, got {lp_usd}");
        let b = m.tick_bracket_hypervisor.brackets.last().unwrap();
        assert!(b.full_range && b.distance_bps.is_none());
        assert_eq!(m.asset_aggregation_series.positions_nock.last().unwrap().position_id,
            "aerodrome:base:0x85f1aa3a70fedd1c52705c15baed143e675cd626");
    }

    #[test]
    fn test_rebuild_fills_only_the_gap_when_other_positions_are_above_their_share() {
        // LP holds $358 ($250 in the USDC/NOCK range, $108 in the WETH/NOCK range),
        // Aerodrome is empty, and the LP target is $486. The WETH/NOCK range is
        // gas-excluded (issue #94), so the two eligible positions share $243 each: the
        // USDC/NOCK range is over its share and is left alone, and the Aerodrome
        // suggestion is the remaining gap ($128), not its full $243 share.
        let mut raw = rebuild_fixture();
        let spot = raw.spot_price_nock_usd;
        for p in raw.positions.iter_mut() {
            match p.nft_id {
                3163450 => { p.amount0 = 71.0; p.amount1 = (250.0 - 71.0) / spot; }
                6336662 => { p.amount0 = 0.0; p.amount1 = 108.0 / spot; }
                _ => {}
            }
        }
        let mut aero = aero_position(false);
        aero.amount0 = 0.0;
        aero.amount1 = 0.0;
        aero.liquidity = 0;
        raw.pool_liquidity.insert(aero.position_id.clone(), "342542824101829799".to_string());
        raw.positions.push(aero);
        let (s, _) = lp_rebuild_suggestions(&raw, 486.0, roomy_funds());
        assert_eq!(s.len(), 1, "only positions below their share are suggested: {s:?}");
        let total: f64 = s.iter().map(|x| x.add_usd.parse::<f64>().unwrap()).sum();
        assert!((total - 128.0).abs() < 0.05, "suggestions must sum to the $128 gap, got {total}");
        // Neither funded Uniswap range is asked to change.
        assert!(s.iter().all(|x| x.nft_id != 3163450 && x.nft_id != 6336662));
        let a = s.iter().find(|x| x.protocol == "aerodrome").unwrap();
        assert!(a.add_usd.parse::<f64>().unwrap() > 60.0);
        // The stated target is what is being filled: nothing is reported as held back.
        assert_eq!(a.target_usd, a.add_usd);
        assert_eq!(a.limited_by, None);
    }

    #[test]
    fn test_eth_call_revert_is_an_answer_not_a_failure() {
        // Returned data.
        assert_eq!(
            classify_eth_call_response(&json!({ "jsonrpc": "2.0", "id": 1, "result": "0x00ff" })).unwrap(),
            Some("00ff".to_string())
        );
        // The new Aero pool's claimFees(): Alchemy answers code 3, "execution reverted".
        // This failed the whole ingestion cycle on 2026-11-14 once LP was held there.
        let reverted = json!({ "jsonrpc": "2.0", "id": 1, "error": { "code": 3, "message": "execution reverted" } });
        assert_eq!(classify_eth_call_response(&reverted).unwrap(), None);
        // A node-side problem is NOT a revert and must still fail the read.
        let limited = json!({ "jsonrpc": "2.0", "id": 1, "error": { "code": 429, "message": "rate limit exceeded" } });
        assert!(classify_eth_call_response(&limited).is_err());
        assert!(classify_eth_call_response(&json!({ "jsonrpc": "2.0", "id": 1 })).is_err());
    }

    #[test]
    fn test_rebuild_does_not_lead_or_nag_when_nothing_is_fundable() {
        // LP is under target but only dust could be added (the 2026-11-14 state after
        // the operator's deposits used up the floor tolerance: 89 NOCK of headroom).
        let raw = rebuild_fixture();
        let dust = RebuildFunds { nock: 89.0, floor_tolerance_nock: 10_000.0, floor_set: true, usdc: 10_000.0 };
        let (s, x) = lp_rebuild_suggestions(&raw, 300.0, dust);
        assert!(s.is_empty(), "a ~$2 top-up is not worth suggesting: {s:?}");
        assert!(x.iter().any(|e| e.reason.contains("cannot be funded")));
        // With no step to take, the rebuild note annotates the directive instead of replacing it.
        let lp = LpRebuild { status: "under_target".to_string(), target_usd: "468.00".to_string(), actual_usd: "423.00".to_string(), gap_usd: "45.00".to_string(), target_weight_bps: 2000, ..Default::default() };
        let (action, details) = apply_lp_rebuild_bias("DCA ON SCHEDULE (2 of 5 next)".to_string(), "Nothing to buy yet.".to_string(), &lp);
        assert_eq!(action, "DCA ON SCHEDULE (2 of 5 next)");
        assert!(details.contains("No tracked position is eligible"));
    }

    // ---------------- LP rebuild suggestions (issue #83) ----------------

    /// Liquid wallet with every tracked pool live except the v4 0.75% pool (#3163431),
    /// which is dead exactly as on 2026-11-14. ETH balance read; no WETH.
    fn rebuild_fixture() -> RawOnChainData {
        let mut raw = liquid_wallet_fixture();
        for p in &raw.positions {
            let live = if p.nft_id == 3163431 { "0" } else { "80173935548453779" };
            raw.pool_liquidity.insert(p.position_id.clone(), live.to_string());
        }
        raw.wallet_eth = Some(0.114226);
        raw.wallet_weth = Some(0.0);
        raw
    }

    fn roomy_funds() -> RebuildFunds {
        RebuildFunds { nock: 1_000_000.0, floor_tolerance_nock: 0.0, floor_set: true, usdc: 10_000.0 }
    }

    #[test]
    fn test_floor_tolerance_lets_lp_hold_a_bounded_overage() {
        // Operator decision: a little more NOCK than sits above the floor may go into
        // LP, because ranges sell gradually. The overage is bounded and reported.
        let raw = rebuild_fixture(); // wallet 115,747.8 NOCK; floor 100,000
        let strict = metrics_with_plan(&raw, &PlanContext::none(), &MarketRegime::default());
        let nock_in = |m: &DerivedMetrics| -> f64 {
            m.lp_rebuild.suggestions.iter().map(|sg| {
                (if sg.token0_symbol == "NOCK" { sg.amount0.parse::<f64>().unwrap() } else { 0.0 })
                    + (if sg.token1_symbol == "NOCK" { sg.amount1.parse::<f64>().unwrap() } else { 0.0 })
            }).sum()
        };
        assert!(nock_in(&strict) <= 15_747.8 + 3.0);
        assert_eq!(strict.lp_rebuild.floor_tolerance_nock.as_deref(), Some("0"));

        let mut profile = operator_profile_2026_11();
        profile.lp_floor_tolerance_bps = 500; // 5% of the floor = 5,000 NOCK
        let loose = compute_derived_metrics_with_plan(
            &raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[],
            &LedgerAggregate::default(), &MarketRegime::default(), &PlanContext::none(),
        );
        assert_eq!(loose.lp_rebuild.floor_tolerance_nock.as_deref(), Some("5000"));
        // More NOCK may go in than strictly sits above the floor, but never beyond the tolerance.
        assert!(nock_in(&loose) >= nock_in(&strict));
        assert!(nock_in(&loose) <= 15_747.8 + 5_000.0 + 3.0);
        // The worst case (every range sells out) is stated and stays within the tolerance.
        let worst: f64 = loose.lp_rebuild.worst_case_nock.as_deref().unwrap().parse().unwrap();
        assert!((worst - (115_747.8001 - nock_in(&loose))).abs() < 2.0);
        assert!(worst >= 100_000.0 - 5_000.0 - 3.0, "worst case {worst} must stay within 5% of the floor");
        assert!(loose.global_status.shot_advice_details.contains("If every range sold out"));
    }

    #[test]
    fn test_nock_sell_out_multiple_from_ticks() {
        let raw = rebuild_fixture();
        // The 2026-11-14 live ticks: the wide USDC/NOCK range needs a ~3.6x rise to sell
        // all its NOCK; the narrow WETH/NOCK ladder only ~1.44x.
        let mut wide = raw.positions.iter().find(|p| p.nft_id == 3163450).unwrap().clone();
        wide.tick = 274970;
        assert!((nock_sell_out_multiple(&wide).unwrap() - 3.586).abs() < 0.01);
        let mut ladder = raw.positions.iter().find(|p| p.nft_id == 6336662).unwrap().clone();
        ladder.tick = 74039;
        assert!((nock_sell_out_multiple(&ladder).unwrap() - 1.439).abs() < 0.01);
        // Past the edge the range holds no NOCK, so there is nothing left to sell.
        ladder.tick = ladder.tick_lower;
        assert_eq!(nock_sell_out_multiple(&ladder), None);
    }

    fn sg<'a>(v: &'a [LpSuggestion], nft: u64) -> &'a LpSuggestion {
        v.iter().find(|s| s.nft_id == nft).unwrap_or_else(|| panic!("no suggestion for #{nft}"))
    }

    #[test]
    fn test_amounts_per_usd_is_worth_one_dollar() {
        let raw = rebuild_fixture();
        for p in &raw.positions {
            let (a0, a1) = amounts_per_usd(p, raw.spot_price_nock_usd, raw.live_eth_price_usd).expect("priced");
            let value = a0 * leg_usd_price(&p.token0, raw.spot_price_nock_usd, raw.live_eth_price_usd)
                + a1 * leg_usd_price(&p.token1, raw.spot_price_nock_usd, raw.live_eth_price_usd);
            assert!((value - 1.0).abs() < 1e-9, "{}: $1 of range must be worth $1, got {value}", p.position_id);
            assert!(a0 >= 0.0 && a1 >= 0.0);
        }
    }

    #[test]
    fn test_rebuild_skips_dead_pools_and_splits_equally() {
        let raw = rebuild_fixture();
        let (s, x) = lp_rebuild_suggestions(&raw, 300.0, roomy_funds());
        // Dead pool is excluded even though its frozen tick still reads "in range".
        let dead = x.iter().find(|e| e.nft_id == 3163431).expect("dead pool excluded");
        assert!(dead.reason.contains("no in-range liquidity"));
        assert!(raw.positions.iter().find(|p| p.nft_id == 3163431).unwrap().in_range);
        // Of the three live, in-range positions two need ETH/WETH, which is gas (issue
        // #94): the USDC/NOCK range takes the whole target.
        assert_eq!(s.len(), 1);
        let one = sg(&s, 3163450);
        assert_eq!(one.target_usd, "300.00");
        assert_eq!(one.add_usd, "300.00");
        assert_eq!(one.limited_by, None);
        for nft in [3163432u64, 6336662] {
            assert!(x.iter().any(|e| e.nft_id == nft && e.reason.contains("gas, not investable capital")));
        }
        assert_eq!(sg(&s, 3163450).url, "https://app.uniswap.org/positions/v4/base/3163450");
        // The USDC/NOCK range spans spot, so its deposit takes both tokens.
        let wide = sg(&s, 3163450);
        assert_eq!((wide.token0_symbol.as_str(), wide.token1_symbol.as_str()), ("USDC", "NOCK"));
        assert!(wide.amount0.parse::<f64>().unwrap() > 0.0 && wide.amount1.parse::<f64>().unwrap() > 0.0);
    }

    #[test]
    fn test_rebuild_does_not_fill_out_of_range_positions_by_default() {
        // Operator rule: "we should not generally aim to fill positions that are out of range".
        let mut raw = rebuild_fixture();
        raw.positions.iter_mut().find(|p| p.nft_id == 3163432).unwrap().in_range = false;
        let (s, x) = lp_rebuild_suggestions(&raw, 300.0, roomy_funds());
        assert!(s.iter().all(|one| one.nft_id != 3163432));
        assert!(x.iter().any(|e| e.nft_id == 3163432 && e.reason.contains("out of range")));
        // The USDC/NOCK range takes the whole target (the WETH/NOCK range is gas-excluded).
        assert_eq!(s.len(), 1);
        assert_eq!(sg(&s, 3163450).target_usd, "300.00");
        // "Generally": a per-position exception brings an out-of-range USDC/NOCK range back.
        let mut raw = rebuild_fixture();
        raw.positions.iter_mut().find(|p| p.nft_id == 3163450).unwrap().in_range = false;
        let (s, x) = lp_rebuild_suggestions(&raw, 300.0, roomy_funds());
        assert!(s.is_empty());
        assert!(x.iter().any(|e| e.nft_id == 3163450 && e.reason.contains("out of range")));
        raw.rebuild_rules.insert(
            "uniswap-v4:base:3163450".to_string(),
            RebuildRule { weight_bps: None, when_out_of_range: true, deposit_url: None },
        );
        let (s, _) = lp_rebuild_suggestions(&raw, 300.0, roomy_funds());
        assert_eq!(s.len(), 1);
        // The exception never overrides the gas rule: an ETH-leg range stays out.
        let mut raw = rebuild_fixture();
        raw.positions.iter_mut().find(|p| p.nft_id == 3163432).unwrap().in_range = false;
        raw.rebuild_rules.insert(
            "uniswap-v4:base:3163432".to_string(),
            RebuildRule { weight_bps: None, when_out_of_range: true, deposit_url: None },
        );
        let (s, x) = lp_rebuild_suggestions(&raw, 300.0, roomy_funds());
        assert!(s.iter().all(|one| one.nft_id != 3163432));
        assert!(x.iter().any(|e| e.nft_id == 3163432 && e.reason.contains("gas")));
    }

    #[test]
    fn test_rebuild_weights_and_config_exclusion() {
        let mut raw = rebuild_fixture();
        raw.rebuild_rules.insert("uniswap-v4:base:3163450".to_string(), RebuildRule { weight_bps: Some(7000), when_out_of_range: false, deposit_url: None });
        raw.rebuild_rules.insert("uniswap-v4:base:3163432".to_string(), RebuildRule { weight_bps: Some(0), when_out_of_range: false, deposit_url: None });
        let (s, x) = lp_rebuild_suggestions(&raw, 1000.0, roomy_funds());
        // The only unweighted position (#6336662, WETH/NOCK) is gas-excluded, so its share
        // is redistributed: the one eligible position's 7000 bps scales to the whole.
        assert_eq!(sg(&s, 3163450).target_usd, "1000.00");
        assert!(s.iter().all(|one| one.nft_id != 6336662));
        assert!(x.iter().any(|e| e.nft_id == 6336662 && e.reason.contains("gas")));
        assert!(x.iter().any(|e| e.nft_id == 3163432 && e.reason.contains("rebuild_weight_bps: 0")));
        // With an unweighted eligible position, explicit weights are kept as set and the
        // unweighted one takes what is left.
        let mut aero = aero_position(false);
        aero.amount0 = 0.0;
        aero.amount1 = 0.0;
        aero.liquidity = 0;
        raw.pool_liquidity.insert(aero.position_id.clone(), "342542824101829799".to_string());
        raw.positions.push(aero);
        let (s, _) = lp_rebuild_suggestions(&raw, 1000.0, roomy_funds());
        assert_eq!(sg(&s, 3163450).target_usd, "700.00");
        assert_eq!(s.iter().find(|x| x.protocol == "aerodrome").unwrap().target_usd, "300.00");
    }

    #[test]
    fn test_rebuild_never_places_floor_nock_in_lp_and_scales_fairly() {
        // Two eligible NOCK-bearing positions: the USDC/NOCK range and an empty Aero pool
        // (the ETH-leg ranges are gas-excluded).
        let with_aero = || {
            let mut raw = rebuild_fixture();
            let mut aero = aero_position(false);
            aero.amount0 = 0.0;
            aero.amount1 = 0.0;
            aero.liquidity = 0;
            raw.pool_liquidity.insert(aero.position_id.clone(), "342542824101829799".to_string());
            raw.positions.push(aero);
            raw
        };
        let raw = with_aero();
        // Unconstrained NOCK demand for the two deposits.
        let (free, _) = lp_rebuild_suggestions(&raw, 900.0, roomy_funds());
        let nock_of = |v: &[LpSuggestion]| -> f64 {
            v.iter()
                .map(|one| {
                    (if one.token0_symbol == "NOCK" { one.amount0.parse::<f64>().unwrap() } else { 0.0 })
                        + (if one.token1_symbol == "NOCK" { one.amount1.parse::<f64>().unwrap() } else { 0.0 })
                })
                .sum()
        };
        let demand = nock_of(&free);
        assert!(demand > 1000.0);
        // Only half of that sits above the floor.
        let tight = RebuildFunds { nock: demand / 2.0, floor_tolerance_nock: 0.0, floor_set: true, usdc: 10_000.0 };
        let (s, _) = lp_rebuild_suggestions(&raw, 900.0, tight);
        assert!(nock_of(&s) <= demand / 2.0 + 3.0, "LP NOCK {} must stay within the {} above the floor", nock_of(&s), demand / 2.0);
        // Every position that needs NOCK is scaled by the same factor and says why.
        assert_eq!(s.len(), 2);
        for one in &s {
            assert_eq!(one.add_usd, "225.00", "{} should be halved from $450", one.label);
            assert!(one.limited_by.as_ref().unwrap().contains("hard floor"));
        }
        // Order independence: reversing the config order changes nothing.
        let mut reversed = with_aero();
        reversed.positions.reverse();
        let (r, _) = lp_rebuild_suggestions(&reversed, 900.0, tight);
        for one in &s {
            let twin = r.iter().find(|x| x.position_id == one.position_id).unwrap();
            assert_eq!(twin.add_usd, one.add_usd);
        }
        // Nothing above the floor at all: no NOCK-bearing deposit is suggested.
        let none = RebuildFunds { nock: 0.0, floor_tolerance_nock: 0.0, floor_set: true, usdc: 10_000.0 };
        let (s, x) = lp_rebuild_suggestions(&raw, 900.0, none);
        assert!(s.is_empty());
        assert!(x.iter().any(|e| e.reason.contains("cannot be funded") && e.reason.contains("hard floor")));
    }

    #[test]
    fn test_rebuild_unread_balances_are_never_assumed() {
        let mut raw = rebuild_fixture();
        // Pool read failed for one position: unknown, so not suggested.
        raw.pool_liquidity.remove("uniswap-v4:base:3163450");
        let (s, x) = lp_rebuild_suggestions(&raw, 300.0, roomy_funds());
        assert!(s.iter().all(|one| one.nft_id != 3163450));
        assert!(x.iter().any(|e| e.nft_id == 3163450 && e.reason.contains("could not be read")));
        // Wallet gas is never a funding source, read or not: the ETH-leg ranges are
        // excluded either way, with the gas reason, and the USDC/NOCK range stands.
        for (eth, weth) in [(None, None), (Some(0.114226), Some(0.0)), (Some(10.0), Some(10.0))] {
            let mut raw = rebuild_fixture();
            raw.wallet_eth = eth;
            raw.wallet_weth = weth;
            let (s, x) = lp_rebuild_suggestions(&raw, 300.0, roomy_funds());
            assert!(s.iter().all(|one| one.nft_id != 6336662 && one.nft_id != 3163432));
            assert!(x.iter().any(|e| e.nft_id == 6336662 && e.reason.contains("gas")));
            assert!(s.iter().any(|one| one.nft_id == 3163450));
        }
    }

    #[test]
    fn test_reserve_counts_usdc_inside_lp() {
        // $157 of USDC moved into a range: wallet 876, LP 157, stable total still 1,033.
        let mut cfg = plan_cfg();
        cfg.budget_usdc = 100.0;
        cfg.cash_reserve_usdc = Some(950.0);
        let mut guards = roomy_guards();
        guards.wallet_usdc = 876.0;
        guards.lp_usdc = 157.0;
        let now = t("2026-11-15T12:00:00Z");
        // Rebuilding LP did not consume reserve headroom: the $20 tranche still stands.
        assert_eq!(
            plan_directive(&cfg, &PlanFills::default(), &guards, false, now),
            PlanDirective::TrancheDue { usdc: 20.0, limited_by: None }
        );
        // If the range converts that USDC to NOCK on a dip, the stable total falls and
        // the plan stops spending: the reserve is honest about what is left.
        guards.lp_usdc = 0.0;
        let d = plan_directive(&cfg, &PlanFills::default(), &guards, false, now);
        assert!(matches!(d, PlanDirective::Blocked { ref reason } if reason.contains("protected cash reserve")));
    }

    #[test]
    fn test_advice_carries_concrete_rebuild_steps_and_pool_activity() {
        let raw = rebuild_fixture();
        let m = metrics_with_plan(&raw, &PlanContext::none(), &MarketRegime::default());
        assert_eq!(m.lp_rebuild.status, "under_target");
        assert!(!m.lp_rebuild.suggestions.is_empty());
        assert!(m.global_status.shot_advice_details.contains("Suggested: add "));
        assert!(m.global_status.shot_advice_details.contains("#3163450"));
        assert!(!m.global_status.shot_advice_details.contains("#3163431"), "the dead pool must not be suggested");
        // NOCK placed in LP never exceeds what sits above the 100,000 floor.
        let lp_nock: f64 = m.lp_rebuild.suggestions.iter().map(|one| {
            (if one.token0_symbol == "NOCK" { one.amount0.parse::<f64>().unwrap() } else { 0.0 })
                + (if one.token1_symbol == "NOCK" { one.amount1.parse::<f64>().unwrap() } else { 0.0 })
        }).sum();
        assert!(lp_nock <= 115_747.8001 - 100_000.0 + 3.0, "LP NOCK {lp_nock} exceeds the NOCK above the floor");
        // Snapshot positions are stamped live/dead/unknown.
        let mut positions = build_position_snapshots(&raw.positions, raw.spot_price_nock_usd, raw.live_eth_price_usd);
        assert!(positions.iter().all(|p| p.pool_active.is_none()));
        apply_pool_activity(&mut positions, &raw.pool_liquidity);
        assert_eq!(positions.iter().find(|p| p.nft_id == 3163431).unwrap().pool_active, Some(false));
        assert_eq!(positions.iter().find(|p| p.nft_id == 3163450).unwrap().pool_active, Some(true));
        // With no pool reads at all, nothing is suggested and the advice says so.
        let blind = liquid_wallet_fixture();
        let m = metrics_with_plan(&blind, &PlanContext::none(), &MarketRegime::default());
        assert!(m.lp_rebuild.suggestions.is_empty());
        assert!(m.global_status.shot_advice_details.contains("No tracked position is eligible"));
    }

    fn transfer_log(token: &str, from: &str, to: &str, amount: u128) -> serde_json::Value {
        let pad = |a: &str| format!("0x000000000000000000000000{}", a.trim_start_matches("0x").to_lowercase());
        json!({
            "address": token,
            "topics": [TRANSFER_EVENT_TOPIC, pad(from), pad(to)],
            "data": format!("0x{:064x}", amount)
        })
    }

    #[test]
    fn test_parse_accumulation_swap_nets_wallet_flows() {
        let wallet = "0xeE114cC97eC5adeB34bA5645e315B497D8487a45";
        let pool = "0x85f1aa3a70fedd1c52705c15baed143e675cd626";
        let other = "0x1111111111111111111111111111111111111111";
        // $100 USDC out, 8,100 NOCK in (16 decimals); an unrelated transfer is ignored.
        let receipt = json!({ "logs": [
            transfer_log(USDC_BASE, wallet, pool, 100_000_000),
            transfer_log(NOCK_BASE, pool, wallet, 81_000_000_000_000_000_000),
            transfer_log(NOCK_BASE, pool, other, 5_000_000_000_000_000_000),
        ]});
        assert_eq!(parse_accumulation_swap(&receipt, wallet), Some((100_000_000, 81_000_000_000_000_000_000)));
        assert_eq!(format_units(81_000_000_000_000_000_000, 16), "8100");

        // A partial USDC refund nets out.
        let refund = json!({ "logs": [
            transfer_log(USDC_BASE, wallet, pool, 100_000_000),
            transfer_log(USDC_BASE, pool, wallet, 1_000_000),
            transfer_log(NOCK_BASE, pool, wallet, 80_000_000_000_000_000_000),
        ]});
        assert_eq!(parse_accumulation_swap(&refund, wallet), Some((99_000_000, 80_000_000_000_000_000_000)));

        // A SELL (NOCK out, USDC in), a fee claim (both in) and someone else's swap are rejected.
        let sell = json!({ "logs": [
            transfer_log(NOCK_BASE, wallet, pool, 80_000_000_000_000_000_000),
            transfer_log(USDC_BASE, pool, wallet, 99_000_000),
        ]});
        assert_eq!(parse_accumulation_swap(&sell, wallet), None);
        let claim = json!({ "logs": [
            transfer_log(USDC_BASE, pool, wallet, 1_000_000),
            transfer_log(NOCK_BASE, pool, wallet, 10_000_000_000_000_000),
        ]});
        assert_eq!(parse_accumulation_swap(&claim, wallet), None);
        let foreign = json!({ "logs": [
            transfer_log(USDC_BASE, other, pool, 100_000_000),
            transfer_log(NOCK_BASE, pool, other, 81_000_000_000_000_000_000),
        ]});
        assert_eq!(parse_accumulation_swap(&foreign, wallet), None);
        assert_eq!(parse_accumulation_swap(&json!({}), wallet), None);
    }

    #[test]
    fn test_accumulation_plan_mcp_text() {
        let raw = liquid_wallet_fixture();
        let ctx = PlanContext { cfg: Some(plan_cfg()), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-15T12:00:00Z") };
        let m = metrics_with_plan(&raw, &ctx, &MarketRegime::default());
        let data = json!({ "derived_metrics": serde_json::to_value(&m).unwrap() });
        let text = format_accumulation_plan_text(&data);
        assert!(text.contains("Status: tranche_due"));
        assert!(text.contains("1 of 5"));
        assert!(text.contains("$100.00 due now"));
        // A finished plan never reports a tranche as due.
        let fills = PlanFills { count: 5, spent_usdc: 500.0, acquired_nock: 40500.0, last_at: Some(t("2026-11-15T12:00:00Z")), first_price: Some(0.0125) };
        let done_ctx = PlanContext { cfg: Some(plan_cfg()), fills, ledger_unavailable: false, now: t("2026-11-20T12:00:00Z") };
        let done = metrics_with_plan(&raw, &done_ctx, &MarketRegime::default());
        let done_text = format_accumulation_plan_text(&json!({ "derived_metrics": serde_json::to_value(&done).unwrap() }));
        assert!(done_text.contains("Next tranche: none (plan complete)"));
        // Between tranches the text states the size and never "due now".
        let wfills = PlanFills { count: 1, spent_usdc: 100.0, acquired_nock: 8000.0, last_at: Some(t("2026-11-15T12:00:00Z")), first_price: Some(0.0125) };
        let wctx = PlanContext { cfg: Some(plan_cfg()), fills: wfills, ledger_unavailable: false, now: t("2026-11-15T18:00:00Z") };
        let waiting = metrics_with_plan(&raw, &wctx, &MarketRegime::default());
        let wtext = format_accumulation_plan_text(&json!({ "derived_metrics": serde_json::to_value(&waiting).unwrap() }));
        assert!(wtext.contains("Next tranche: 2 of 5 — $100.00 when due"));
        assert!(!wtext.contains("due now"));
        assert!(!done_text.contains("due now"));
        assert!(text.contains("LP rebuild bias: under_target"));
        let rb = rebuild_fixture();
        let rm = metrics_with_plan(&rb, &PlanContext::none(), &MarketRegime::default());
        let rtext = format_accumulation_plan_text(&json!({ "derived_metrics": serde_json::to_value(&rm).unwrap() }));
        assert!(rtext.contains("→ add ") && rtext.contains("https://app.uniswap.org/positions/v4/base/3163450"));
        assert!(rtext.contains("✗ v4 USDC/NOCK 0.75% #3163431: pool has no in-range liquidity"));
        // Before the first fill the average is "n/a", never a made-up number.
        assert!(text.contains("average $n/a"));
        let none = format_accumulation_plan_text(&json!({ "derived_metrics": { "lp_rebuild": {} } }));
        assert!(none.contains("No accumulation_plan is configured"));
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
                    ..Default::default()
                },
                sovereign_ledger: Default::default(),
                accumulation_plan: None,
                lp_rebuild: LpRebuild::default(),
                asset_aggregation_series: AssetAggregationSeries {
                    time_points: vec!["2026-10-11T00:00:00Z".to_string()],
                    safe_haven_series: vec!["48.00".to_string()],
                    wallet_nock_series: vec![],
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

    fn word(addr: &str) -> String {
        format!("0x000000000000000000000000{}", addr.trim_start_matches("0x").to_lowercase())
    }

    #[test]
    fn test_parse_harvest_transfers_books_weth_fees() {
        // #111: a v3 WETH/NOCK collect straight to the wallet pays WETH by Transfer.
        let wallet = "0xeE114cC97eC5adeB34bA5645e315B497D8487a45";
        let pool = "0x1111111111111111111111111111111111111111";
        let receipt = json!({
            "status": "0x1", "from": wallet,
            "logs": [
                { "address": WETH_BASE, "topics": [TRANSFER_EVENT_TOPIC, word(pool), word(wallet)],
                  "data": "0x00000000000000000000000000000000000000000000000000038d7ea4c68000" },
                { "address": NOCK_BASE, "topics": [TRANSFER_EVENT_TOPIC, word(pool), word(wallet)],
                  "data": "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000" }
            ]
        });
        let t = parse_harvest_transfers(&receipt, wallet);
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].token_symbol.as_str(), t[0].decimals, t[0].amount_human.as_str()), ("WETH", 18, "0.001"));
        assert_eq!(t[1].token_symbol, "NOCK");
    }

    #[test]
    fn test_parse_harvest_transfers_books_eth_unwrapped_by_a_position_manager() {
        // #111: the Uniswap app collects to the NPM and unwraps; the wallet gets native ETH,
        // which leaves only a WETH Withdrawal by the NPM.
        let wallet = "0xeE114cC97eC5adeB34bA5645e315B497D8487a45";
        let pool = "0x1111111111111111111111111111111111111111";
        let withdrawal = |src: &str| json!({ "address": WETH_BASE, "topics": [WETH_WITHDRAWAL_TOPIC, word(src)],
            "data": "0x00000000000000000000000000000000000000000000000000071afd498d0000" });
        let weth_to_npm = json!({ "address": WETH_BASE, "topics": [TRANSFER_EVENT_TOPIC, word(pool), word(UNISWAP_V3_NPM)],
            "data": "0x00000000000000000000000000000000000000000000000000071afd498d0000" });
        let receipt = json!({ "status": "0x1", "from": wallet, "logs": [weth_to_npm.clone(), withdrawal(UNISWAP_V3_NPM)] });
        let t = parse_harvest_transfers(&receipt, wallet);
        assert_eq!(t.len(), 1, "the WETH Transfer to the NPM is not the wallet's; only the unwrap is booked");
        assert_eq!(t[0].token_symbol, "ETH");
        assert_eq!(t[0].token_address, NATIVE_ETH_LEDGER_ADDRESS);
        assert_eq!(t[0].amount_human, "0.002");
        assert_eq!(t[0].from_address, UNISWAP_V3_NPM.to_lowercase());
        // The v4 PositionManager counts too.
        let r4 = json!({ "status": "0x1", "from": wallet, "logs": [withdrawal(UNISWAP_V4_POSM_BASE)] });
        assert_eq!(parse_harvest_transfers(&r4, wallet).len(), 1);
        // A router or anyone else unwrapping is not a fee payout.
        let other = json!({ "status": "0x1", "from": wallet, "logs": [withdrawal(pool)] });
        assert!(parse_harvest_transfers(&other, wallet).is_empty());
        // A transaction someone else sent never books an unwrap to this wallet.
        let not_ours = json!({ "status": "0x1", "from": pool, "logs": [withdrawal(UNISWAP_V3_NPM)] });
        assert!(parse_harvest_transfers(&not_ours, wallet).is_empty());
    }

    #[test]
    fn test_sovereign_ledger_reports_its_ratio_and_unpriced_rows() {
        // #102: the panel's labels come from the data, so the ledger carries the ratio.
        let raw = four_position_fixture();
        let agg = LedgerAggregate { total_usd: 10.0, harvest_count: 2, last_at: None, unpriced_count: 1 };
        let m = compute_derived_metrics(&raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &agg, &MarketRegime::default());
        let l = &m.sovereign_ledger;
        assert_eq!((l.tax_reserve_ratio.as_str(), l.tax_reserve_usd.as_str(), l.growth_dry_powder_usd.as_str()), ("0.00", "0.00", "10.00"));
        assert_eq!(l.unpriced_count, 1);
        let mut profile = OperatorProfile::default();
        profile.tax_reserve_ratio = 0.25;
        let m = compute_derived_metrics(&raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[], &agg, &MarketRegime::default());
        let l = &m.sovereign_ledger;
        assert_eq!((l.tax_reserve_ratio.as_str(), l.tax_reserve_usd.as_str(), l.growth_dry_powder_usd.as_str()), ("0.25", "2.50", "7.50"));
    }

    #[test]
    fn test_harvest_valuation_uses_the_block_prices_or_nothing() {
        let nock = (0.0125, "market_price_history aerodrome_rpc at T".to_string());
        let eth = (1790.0, "eth_price_pool slot0 at block 1".to_string());
        assert_eq!(harvest_unit_price("USDC", None, None), Some((1.0, "USDC at $1")));
        assert_eq!(harvest_unit_price("NOCK", Some(&nock), Some(&eth)).map(|p| p.0), Some(0.0125));
        assert_eq!(harvest_unit_price("WETH", Some(&nock), Some(&eth)).map(|p| p.0), Some(1790.0));
        assert_eq!(harvest_unit_price("ETH", None, Some(&eth)).map(|p| p.1), Some("eth_price_pool slot0 at block 1"));
        // No price found: no value, never a substitute.
        assert_eq!(harvest_unit_price("NOCK", None, Some(&eth)), None);
        assert_eq!(harvest_unit_price("AERO", Some(&nock), Some(&eth)), None);
        assert_eq!(harvest_usd_value("0.002", Some(1790.0)).as_deref(), Some("3.5800"));
        assert_eq!(harvest_usd_value("1000", Some(0.0125)).as_deref(), Some("12.5000"));
        assert_eq!(harvest_usd_value("1000", None), None);
        assert_eq!(harvest_usd_value("1000", Some(0.0)), None);
        assert_eq!(harvest_usd_value("abc", Some(1.0)), None);
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
    fn test_price_source_liquidity_guard() {
        // Live pool: any non-zero active liquidity passes.
        assert!(check_price_source_liquidity("uniswap-v4:base:3163450", "0x064a", 80173935548453779).is_ok());
        // Dead pool (the v4 0.75% pool after the 2026-11 liquidation): frozen price is rejected.
        let err = check_price_source_liquidity("uniswap-v4:base:3163431", "0x91a9", 0).unwrap_err();
        assert!(err.to_string().contains("zero in-range liquidity"));
        assert!(err.to_string().contains("0x91a9"));
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
                    "nock_floor_target": "100000",
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
        assert!(state_text.contains("+6140 above the 100000 NOCK floor"), "{state_text}");
        assert!(!state_text.contains("100k"));
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
            !metrics.global_status.shot_advice_action.starts_with("OVEREXTENDED"),
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
        assert_eq!(metrics.global_status.shot_advice_action, "OVEREXTENDED (Daily RSI ≥ 70, No Buy Advised)");
        assert!(metrics.global_status.shot_advice_details.contains("daily RSI(14)"));
        assert_eq!(metrics.global_status.regime, "overextended");
        assert_eq!(metrics.global_status.rsi_14.as_deref(), Some("40.0"));
    }

    // ============================================================
    // From zero (v0.5.1): a newcomer's own numbers, and nothing else
    // ============================================================

    /// Text NockNock must never produce for a newcomer (or anyone): sized lump sums,
    /// someone else's goal, alarm words, and uncomputed P/L or yield claims.
    const NEVER_IN_ADVICE: [&str; 5] = ["Deploy $", "120,000", "CRITICAL", "realized loss", "generating yield"];

    fn assert_no_invented_text(details: &str) {
        for banned in NEVER_IN_ADVICE {
            assert!(!details.contains(banned), "{banned:?} in: {details}");
        }
    }

    /// A newcomer's wallet: no LP, gas read as zero, every amount a test input.
    fn from_zero_raw(usdc: f64, nock: f64) -> RawOnChainData {
        let mut raw = RawOnChainData::default();
        raw.spot_price_nock_usd = 0.01153;
        raw.live_eth_price_usd = 3000.0;
        raw.live_gas_cost_usd = 0.004;
        raw.wallet_eth = Some(0.0);
        raw.wallet_weth = Some(0.0);
        let row = |sym: &str, addr: &str, amount: f64| WalletBalance {
            wallet_address: "0x00000000000000000000000000000000000000aa".to_string(),
            token_symbol: sym.to_string(),
            token_address: addr.to_string(),
            amount: format!("{:.4}", amount),
            amount_usd: "0".to_string(),
            updated_at: "now".to_string(),
        };
        raw.wallet_balances = vec![row("NOCK", NOCK_BASE, nock), row("USDC", USDC_BASE, usdc)];
        raw
    }

    /// The newcomer's own plan: $100 over 5 tranches, 24 h apart.
    fn own_plan() -> AccumulationPlanConfig {
        AccumulationPlanConfig {
            id: "my-first-plan".to_string(),
            budget_usdc: 100.0,
            tranches: 5,
            interval_hours: 24.0,
            starts_at: None,
            venue: "Aero".to_string(),
            max_price_usd: None,
            pause_when_overextended: true,
            respect_cash_target: true,
            cash_reserve_usdc: None,
        }
    }

    fn zero_metrics(raw: &RawOnChainData, profile: &OperatorProfile, ctx: &PlanContext) -> DerivedMetrics {
        compute_derived_metrics_with_plan(
            raw, 0.0, profile, StructuralChannels { channels: vec![] }, &[],
            &LedgerAggregate::default(), &MarketRegime::default(), ctx,
        )
    }

    fn own_plan_due() -> PlanContext {
        PlanContext { cfg: Some(own_plan()), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-15T12:00:00Z") }
    }

    fn own_plan_after_first_fill() -> PlanContext {
        PlanContext {
            cfg: Some(own_plan()),
            fills: PlanFills { count: 1, spent_usdc: 20.0, acquired_nock: 1734.6054, last_at: Some(t("2026-11-15T12:00:00Z")), first_price: Some(0.01153) },
            ledger_unavailable: false,
            now: t("2026-11-15T18:00:00Z"),
        }
    }

    #[test]
    fn test_from_zero_usdc_only_without_plan_never_sizes_a_buy() {
        // R1
        let m = zero_metrics(&from_zero_raw(500.0, 0.0), &OperatorProfile::default(), &PlanContext::none());
        let g = &m.global_status;
        assert_eq!(g.total_portfolio_usd, "500.00");
        assert_eq!(g.total_native_usd, "0.00");
        assert_eq!(g.total_stable_usd, "500.00");
        assert_eq!(g.total_nock_amount, "0");
        assert_eq!(g.nock_floor_target, "0");
        assert_eq!(g.high_water_mark_usd, "500.00");
        assert_eq!(g.shot_advice_action, "BUILD FROM ZERO (No Buy Plan Configured)");
        let d = &g.shot_advice_details;
        assert!(d.contains("$500.00") && d.contains("accumulation_plan") && d.contains("no ETH"), "{d}");
        assert_no_invented_text(d);
        for banned in ["Buy $", "DEFENSIVE LOCK", "Standing bias", "Re-establishing"] {
            assert!(!d.contains(banned), "{banned} in: {d}");
        }
        assert!(m.accumulation_plan.is_none());
        assert_eq!(m.lp_rebuild.status, "no_positions");
        assert!(m.lp_rebuild.suggestions.is_empty() && m.lp_rebuild.excluded.is_empty());
        let w: Vec<i32> = m.portfolio_deviation_matrix.allocations.iter().map(|a| a.actual_weight_bps).collect();
        // The fourth row is gas: a measured share with no target.
        assert_eq!(w, vec![0, 0, 10000, 0]);
        assert_eq!(m.portfolio_deviation_matrix.allocations[3].status, "no_target");
        assert!(m.tick_bracket_hypervisor.brackets.is_empty());
        assert!(m.asset_aggregation_series.positions_nock.is_empty());
        assert!(m.velocity_gas_sandbox.contracts.is_empty());
    }

    #[test]
    fn test_from_zero_empty_wallet_is_an_explicit_empty_state() {
        // R2
        let m = zero_metrics(&from_zero_raw(0.0, 0.0), &OperatorProfile::default(), &PlanContext::none());
        assert_eq!(m.global_status.shot_advice_action, "NOTHING TO TRACK YET (Empty Wallet)");
        let d = &m.global_status.shot_advice_details;
        assert!(d.contains("0x00000000000000000000000000000000000000aa"), "{d}");
        assert!(d.contains("holds no NOCK, no USDC and no LP"));
        assert!(d.contains("accumulation_plan") && d.contains("no ETH"));
        assert!(!d.contains("Maintain posture"));
        assert_no_invented_text(d);
        assert_eq!(m.global_status.total_portfolio_usd, "0.00");
        assert_eq!(m.global_status.high_water_mark_usd, "0.00");
        let w: Vec<i32> = m.portfolio_deviation_matrix.allocations.iter().map(|a| a.actual_weight_bps).collect();
        assert_eq!(w, vec![0, 0, 0, 0]);
        assert_eq!(m.lp_rebuild.status, "no_positions");
        // Gas held: the gas fact is not stated.
        let mut gas = from_zero_raw(0.0, 0.0);
        gas.wallet_eth = Some(0.01);
        let m = zero_metrics(&gas, &OperatorProfile::default(), &PlanContext::none());
        assert!(!m.global_status.shot_advice_details.contains("no ETH"));
        // A gas-only wallet is still empty: the gas counts in the total (0.01 x $3000)
        // but is never something to deploy.
        assert_eq!(m.global_status.shot_advice_action, "NOTHING TO TRACK YET (Empty Wallet)");
        assert_eq!(m.global_status.total_portfolio_usd, "30.00");
        assert_eq!(m.global_status.wallet_gas_usd.as_deref(), Some("30.00"));
        assert!(m.global_status.shot_advice_details.contains("never deployed"));
        let w: Vec<i32> = m.portfolio_deviation_matrix.allocations.iter().map(|a| a.actual_weight_bps).collect();
        assert_eq!(w, vec![0, 0, 0, 10000]);
    }

    #[test]
    fn test_from_zero_users_own_plan_leads_and_uses_only_their_numbers() {
        // R3
        let m = zero_metrics(&from_zero_raw(500.0, 0.0), &OperatorProfile::default(), &own_plan_due());
        assert_eq!(m.global_status.shot_advice_action, "DCA TRANCHE DUE (1 of 5)");
        let d = &m.global_status.shot_advice_details;
        assert!(d.starts_with("Buy $20.00 of NOCK on Aero (spot $0.01153)."), "{d}");
        assert!(d.contains("Plan \"my-first-plan\": $0.00 of $100.00 deployed over 0 verified buy(s)."), "{d}");
        assert!(!d.contains("DEFENSIVE LOCK") && !d.contains("Standing bias"));
        assert_no_invented_text(d);
        let ps = m.accumulation_plan.unwrap();
        assert_eq!(ps.status, "tranche_due");
        assert_eq!(ps.tranche_usdc, "20.00");
        assert_eq!(ps.nominal_tranche_usdc, "20.00");
        assert_eq!(ps.budget_usdc, "100.00");
        assert_eq!(ps.binding_constraint, None);
    }

    #[test]
    fn test_from_zero_after_first_fill_waits_on_the_users_interval() {
        // R4
        let m = zero_metrics(&from_zero_raw(480.0, 1734.6054), &OperatorProfile::default(), &own_plan_after_first_fill());
        assert_eq!(m.global_status.shot_advice_action, "DCA ON SCHEDULE (2 of 5 next)");
        assert_eq!(
            m.global_status.shot_advice_details,
            "Nothing to buy yet: the next $20.00 tranche is due at 2026-11-16T12:00:00+00:00. Plan \"my-first-plan\": $20.00 of $100.00 deployed over 1 verified buy(s), average cost $0.011530."
        );
        assert_eq!(m.global_status.total_nock_amount, "1735");
        assert_eq!(m.global_status.total_portfolio_usd, "500.00");
        let ps = m.accumulation_plan.unwrap();
        assert_eq!(ps.tranche_index, 2);
        assert_eq!(ps.fills, 1);
        assert_eq!(ps.avg_price_usd.as_deref(), Some("0.011530"));
        assert_eq!(ps.lump_sum_nock.as_deref(), Some("8673.0269"));
        assert_eq!(ps.remaining_usdc, "80.00");
        assert_eq!(ps.next_due_at.as_deref(), Some("2026-11-16T12:00:00+00:00"));
    }

    #[test]
    fn test_from_zero_users_own_floor_keeps_the_plan_visible_and_notifying() {
        // R5: the user's own 100,000 floor, with their own plan.
        let profile = operator_profile_2026_11();
        let due = zero_metrics(&from_zero_raw(500.0, 0.0), &profile, &own_plan_due());
        assert_eq!(due.global_status.shot_advice_action, "DEFENSIVE LOCK (Floor Recovery) · DCA TRANCHE DUE (1 of 5)");
        let d = &due.global_status.shot_advice_details;
        assert!(d.contains("is below the 100000 floor by 100000 NOCK"), "{d}");
        assert!(d.contains("Your accumulation plan is the recovery path: Buy $20.00 of NOCK on Aero (spot $0.01153)"), "{d}");
        assert!(!d.contains("stable reserves are locked"));
        assert_no_invented_text(d);
        assert_eq!(due.global_status.nock_floor_target, "100000");
        assert_eq!(due.global_status.nock_floor_delta, "-100000");
        assert_eq!(due.accumulation_plan.as_ref().unwrap().status, "tranche_due");

        let waiting = zero_metrics(&from_zero_raw(480.0, 1734.6054), &profile, &own_plan_after_first_fill());
        assert_eq!(waiting.global_status.shot_advice_action, "DEFENSIVE LOCK (Floor Recovery) · DCA ON SCHEDULE (2 of 5 next)");
        assert!(waiting.global_status.shot_advice_details.contains("below the 100000 floor by 98265 NOCK"));
        // The tranche transition changes the action, so it notifies.
        assert!(directive_transition_notification(
            Some(&due.global_status.shot_advice_action),
            &waiting.global_status.shot_advice_action,
            &waiting.global_status.shot_advice_details,
            None,
        )
        .is_some());
    }

    #[test]
    fn test_from_zero_users_own_floor_without_plan_points_to_a_plan() {
        // R6
        let m = zero_metrics(&from_zero_raw(500.0, 0.0), &operator_profile_2026_11(), &PlanContext::none());
        assert_eq!(m.global_status.shot_advice_action, "DEFENSIVE LOCK (Floor Recovery)");
        let d = &m.global_status.shot_advice_details;
        assert!(d.contains("below the 100000 floor by 100000 NOCK") && d.contains("accumulation_plan"), "{d}");
        assert!(!d.contains("Buy $"));
        assert_no_invented_text(d);
    }

    #[test]
    fn test_planless_dip_zone_states_facts_only() {
        // R7: the operator's 2026-11-14 wallet with no plan section.
        let raw = liquid_wallet_fixture();
        let m = compute_derived_metrics_with_plan(
            &raw, 0.0, &operator_profile_2026_11(), StructuralChannels { channels: vec![] }, &[],
            &LedgerAggregate::default(), &MarketRegime::default(), &PlanContext::none(),
        );
        assert_eq!(m.global_status.shot_advice_action, "DIP ZONE (No Buy Plan Configured)");
        let d = &m.global_status.shot_advice_details;
        assert!(d.contains("$0.01229") && d.contains("$1053.00") && d.contains("accumulation_plan"), "{d}");
        assert!(d.contains("Liquid dry powder: $929.22 (wallet USDC $1053.00 less the larger of the $123.78 cash target and the $0.00 virtual tax reserve)."), "{d}");
        // The whole text, including the appended LP-bias sentence, invents nothing.
        assert!(d.contains("Standing bias: "), "{d}");
        assert_no_invented_text(d);
    }

    #[test]
    fn test_due_tranche_leads_but_waiting_plan_yields_to_harvest_and_range_exit() {
        // N11: a plan with nothing due no longer hides a due harvest or a range exit.
        let mut raw = four_position_fixture();
        raw.wallet_balances[1].amount = "5000.0".to_string(); // the tranche is fundable
        raw.positions[1].fee0 = 6.0; // fees over the fixed $5.00 threshold
        let profile = OperatorProfile::default();
        let run = |raw: &RawOnChainData, ctx: &PlanContext| {
            compute_derived_metrics_with_plan(raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default(), ctx)
        };
        let due = PlanContext { cfg: Some(plan_cfg()), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-15T12:00:00Z") };
        let m = run(&raw, &due);
        assert_eq!(m.global_status.shot_advice_action, "DCA TRANCHE DUE (1 of 5)");

        let fills = PlanFills { count: 1, spent_usdc: 100.0, acquired_nock: 8000.0, last_at: Some(t("2026-11-15T12:00:00Z")), first_price: Some(0.0125) };
        let waiting = PlanContext { cfg: Some(plan_cfg()), fills, ledger_unavailable: false, now: t("2026-11-15T18:00:00Z") };
        let m = run(&raw, &waiting);
        assert_eq!(m.global_status.shot_advice_action, "BATCH HARVEST SHOT (Execute Claim)");
        assert_eq!(m.accumulation_plan.as_ref().unwrap().status, "waiting");

        // Spot above the price-source range's top: RANGE EXIT leads and reports that
        // range's own value, not the whole LP book, and no invented $5.00 wall.
        let src = raw.positions.iter().find(|p| p.is_price_source).unwrap().clone();
        let top = tick_to_display_price_dec(src.tick_lower, src.token0.decimals, src.token1.decimals);
        raw.spot_price_nock_usd = top * 1.01;
        let m = run(&raw, &waiting);
        assert_eq!(m.global_status.shot_advice_action, "RANGE EXIT FLASHPOINT (Above Range)");
        let d = &m.global_status.shot_advice_details;
        let spot = raw.spot_price_nock_usd;
        let pos_usd = src.amount0 * leg_usd_price(&src.token0, spot, raw.live_eth_price_usd)
            + src.amount1 * leg_usd_price(&src.token1, spot, raw.live_eth_price_usd);
        assert!(d.contains(&format!("That range ({}) holds ${:.2}", position_label(&src), pos_usd)), "{d}");
        let lp_total: f64 = m.asset_aggregation_series.active_protocol_series[0].parse().unwrap();
        let exit_text = d.split(" Standing bias:").next().unwrap();
        assert!(!exit_text.contains(&format!("${:.2}", lp_total)), "the LP book total must not be the range's value: {d}");
        assert!(!d.contains("$5.00") && !d.contains("High Wall"), "{d}");
        // Blocked and paused plans yield too; a due tranche still leads.
        let m = run(&raw, &due);
        assert_eq!(m.global_status.shot_advice_action, "DCA TRANCHE DUE (1 of 5)");
    }

    #[test]
    fn test_sweep_to_stable_states_facts_and_instructs_no_sale() {
        // N13: the overextension arm states the daily RSI and the plan's pause.
        let raw = raw_above_floor_fixture();
        let hot = MarketRegime { rsi_5m: None, rsi_1d: Some(82.0), overextended: true };
        let ctx = PlanContext { cfg: Some(plan_cfg()), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-15T12:00:00Z") };
        let m = compute_derived_metrics_with_plan(&raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &hot, &ctx);
        // The label is a measurement, not an instruction to sell (N13, G11).
        assert_eq!(m.global_status.shot_advice_action, "OVEREXTENDED (Daily RSI ≥ 70, No Buy Advised)");
        assert!(!m.global_status.shot_advice_action.contains("SWEEP"));
        let d = &m.global_status.shot_advice_details;
        assert!(d.starts_with("NOCK spot ($0.01500) is overextended: daily RSI(14) 82 is at or above 70. Your accumulation plan pauses buys until it cools."), "{d}");
        for banned in ["sweep to the stable stack", "suppress deployment", "Buying tops", "sell"] {
            assert!(!d.contains(banned), "{banned} in: {d}");
        }
        assert_eq!(m.accumulation_plan.unwrap().status, "paused");
        // Without a plan there is no pause to mention.
        let m = compute_derived_metrics(&raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &hot);
        assert!(!m.global_status.shot_advice_details.contains("accumulation plan"));
    }

    #[test]
    fn test_batch_harvest_text_follows_the_configured_tax_ratio() {
        let mut raw = four_position_fixture();
        raw.positions[1].fee0 = 6.0;
        // No ratio configured: nothing is booked to a reserve, and no percentage is named.
        let m = compute_derived_metrics(&raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        let d = &m.global_status.shot_advice_details;
        assert!(d.contains("no tax_reserve_ratio is configured, so none of it is booked to a virtual tax reserve"), "{d}");
        assert!(!d.contains('%'), "{d}");
        let mut profile = operator_profile_no_floor();
        let m = compute_derived_metrics(&raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        let d = &m.global_status.shot_advice_details;
        assert!(d.contains("books 40% of the USDC, NOCK and WETH/ETH received to the virtual tax reserve and 60% to growth"), "{d}");
        assert!(d.contains("valued at the claim's block") && !d.contains("not booked"), "{d}");
        profile.tax_reserve_ratio = 0.25;
        let m = compute_derived_metrics(&raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        let d = &m.global_status.shot_advice_details;
        assert!(d.contains("books 25% of the USDC, NOCK and WETH/ETH received to the virtual tax reserve and 75% to growth"), "{d}");
        assert!(!d.contains("60/40") && !d.contains("40%") && !d.contains("Tax Escrow"), "{d}");
    }

    #[test]
    fn test_mcp_formatters_from_zero() {
        // R14 (worker side): the floor comes from the snapshot, missing fields say so.
        let data = json!({"derived_metrics":{"global_status":{"total_portfolio_usd":"500.00","total_nock_amount":"0","nock_floor_delta":"0","nock_floor_target":"0","total_usdc_amount":"500.00","high_water_mark_usd":"500.00","shot_advice_action":"BUILD FROM ZERO (No Buy Plan Configured)","total_fees_claimable_usd":"0.00","gas_to_fee_delta_usd":"0.00"},"accumulation_plan":null,"lp_rebuild":{"status":"no_positions","actual_usd":"0.00","target_usd":"0.00","gap_usd":"0.00","suggestions":[],"excluded":[]}},"contract_fees":[]});
        let state = format_portfolio_state_text(&data);
        assert!(state.contains("Sovereign NOCK Stack: 0 NOCK (no floor configured)"), "{state}");
        assert!(!state.contains("100k"));
        let mut below = data.clone();
        below["derived_metrics"]["global_status"]["nock_floor_target"] = json!("100000");
        below["derived_metrics"]["global_status"]["nock_floor_delta"] = json!("-100000");
        let text = format_portfolio_state_text(&below);
        assert!(text.contains("100000 below the 100000 NOCK floor") && !text.contains("above"), "{text}");
        let plan = format_accumulation_plan_text(&data);
        assert!(plan.contains("No accumulation_plan is configured") && plan.contains("No LP positions are tracked"), "{plan}");
        assert!(!plan.contains("Rebuilding ranges"));
        let empty = format_shot_advice_text(&json!({}));
        assert!(empty.contains("Recommended Action: unavailable"), "{empty}");
        assert!(!empty.contains("HOLD") && !empty.contains("Accreting") && !empty.contains("0.00"));
        let empty_state = format_portfolio_state_text(&json!({}));
        assert!(empty_state.contains("floor unavailable") && empty_state.contains("Tactical Directive: unavailable"));
        assert!(!empty_state.contains("HOLD") && !empty_state.contains("0.00"));
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

    // ---- Review findings on the v0.5.1 engine commit ----

    #[test]
    fn test_regime_unknown_without_daily_rsi() {
        // R16: no daily RSI is no regime reading, never "normal".
        let raw = raw_above_floor_fixture();
        let run = |r: &MarketRegime| compute_derived_metrics(&raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), r);
        assert_eq!(run(&MarketRegime::default()).global_status.regime, "unknown");
        assert_eq!(run(&MarketRegime { rsi_5m: None, rsi_1d: Some(34.2), overextended: false }).global_status.regime, "normal");
        assert_eq!(run(&MarketRegime { rsi_5m: None, rsi_1d: Some(82.0), overextended: true }).global_status.regime, "overextended");
    }

    #[test]
    fn test_degenerate_math_edges() {
        // R17: a flat series has no RSI (not 100), and a zero sqrtPrice is no spot.
        assert!(compute_rsi_series(&[0.0115; 20], 14).iter().all(|x| x.is_none()));
        let mut src = four_position_fixture().positions.into_iter().find(|p| p.is_price_source).unwrap();
        assert!(nock_spot_from_positions(std::slice::from_ref(&src)).is_some());
        src.sqrt_price_x96 = U256(0, 0);
        assert_eq!(nock_spot_from_positions(std::slice::from_ref(&src)), None);
        // Without a daily RSI the engine never reports an overextension.
        let m = zero_metrics(&from_zero_raw(500.0, 1000.0), &OperatorProfile::default(), &PlanContext::none());
        assert!(!m.global_status.shot_advice_action.starts_with("OVEREXTENDED"));
    }

    #[test]
    fn test_unset_weights_and_ratio_size_nothing() {
        // A user with NOCK, USDC and a tracked range but no weights or tax ratio: no cash
        // target or reserve is named, the LP target is no_target, and no bucket is
        // labelled over- or under-funded against numbers they never set.
        let mut raw = liquid_wallet_fixture();
        raw.spot_price_nock_usd = 0.025; // above the dip line: HOLD
        let m = compute_derived_metrics(&raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        let d = &m.global_status.shot_advice_details;
        assert!(m.global_status.shot_advice_action.starts_with("HOLD & ACCRETE"), "{}", m.global_status.shot_advice_action);
        assert!(d.contains("Liquid dry powder: $1053.00 (wallet USDC; no cash target or tax reserve ratio is configured)."), "{d}");
        for banned in ["less the", "cash target)", "virtual tax reserve", "Standing bias", "Re-establishing"] {
            assert!(!d.contains(banned), "{banned} in: {d}");
        }
        assert_eq!(m.lp_rebuild.status, "no_target");
        assert!(m.lp_rebuild.suggestions.is_empty());
        assert!(m.portfolio_deviation_matrix.allocations.iter().all(|a| a.status == "no_target" && a.target_weight_bps == 0));
        assert_eq!(m.sovereign_ledger.tax_reserve_usd, "0.00");
        // Set only a cash weight: only the cash target is named.
        let mut cash_only = OperatorProfile::default();
        cash_only.target_weights_bps.cash = 500;
        let m = compute_derived_metrics(&raw, 0.0, &cash_only, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        let d = &m.global_status.shot_advice_details;
        assert!(d.contains("cash target).") && !d.contains("virtual tax reserve"), "{d}");
        assert_eq!(m.portfolio_deviation_matrix.allocations[2].target_weight_bps, 500);
        assert_ne!(m.portfolio_deviation_matrix.allocations[2].status, "no_target");
    }

    #[test]
    fn test_rebuild_limit_names_no_floor_when_none_is_set() {
        let raw = rebuild_fixture();
        let demand: f64 = lp_rebuild_suggestions(&raw, 300.0, roomy_funds()).0.iter().map(|sg| {
            (if sg.token0_symbol == "NOCK" { sg.amount0.parse::<f64>().unwrap() } else { 0.0 })
                + (if sg.token1_symbol == "NOCK" { sg.amount1.parse::<f64>().unwrap() } else { 0.0 })
        }).sum();
        let tight = RebuildFunds { nock: demand / 2.0, floor_tolerance_nock: 0.0, floor_set: false, usdc: 10_000.0 };
        let (s, _) = lp_rebuild_suggestions(&raw, 300.0, tight);
        let limited: Vec<&String> = s.iter().filter_map(|sg| sg.limited_by.as_ref()).collect();
        assert!(!limited.is_empty());
        for l in limited {
            assert!(l.contains("NOCK is in the wallet") && !l.contains("floor"), "{l}");
        }
    }

    #[test]
    fn test_defensive_lock_with_a_completed_plan_names_it() {
        // Floor set, plan complete, NOCK below the floor: the plan exists and is done.
        let mut raw = liquid_wallet_fixture();
        raw.wallet_balances[0].amount = "90000.0".to_string();
        let fills = PlanFills { count: 5, spent_usdc: 500.0, acquired_nock: 40500.0, last_at: Some(t("2026-11-15T12:00:00Z")), first_price: Some(0.0125) };
        let ctx = PlanContext { cfg: Some(plan_cfg()), fills, ledger_unavailable: false, now: t("2026-11-20T12:00:00Z") };
        let m = metrics_with_plan(&raw, &ctx, &MarketRegime::default());
        assert!(m.global_status.shot_advice_action.starts_with("DEFENSIVE LOCK (Floor Recovery)"));
        let d = &m.global_status.shot_advice_details;
        assert!(d.contains("Your accumulation plan \"test-plan\" is complete."), "{d}");
        assert!(!d.contains("add an accumulation_plan"), "{d}");
    }

    #[test]
    fn test_harvest_and_range_exit_outrank_planless_no_buy_arms() {
        // No plan, dip zone, $5000 liquid: DIP ZONE would apply, but a due harvest leads.
        let mut raw = four_position_fixture();
        raw.spot_price_nock_usd = 0.015;
        raw.wallet_balances[1].amount = "5000.0".to_string();
        raw.positions[1].fee0 = 6.0;
        let run = |raw: &RawOnChainData, r: &MarketRegime| compute_derived_metrics(raw, 0.0, &operator_profile_no_floor(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), r);
        assert_eq!(run(&raw, &MarketRegime::default()).global_status.shot_advice_action, "BATCH HARVEST SHOT (Execute Claim)");
        // BUILD FROM ZERO with fees still due on an emptied range: the harvest leads.
        let mut zero = from_zero_raw(500.0, 0.0);
        let mut p = four_position_fixture().positions[1].clone();
        p.liquidity = 0;
        p.amount0 = 0.0;
        p.amount1 = 0.0;
        p.fee0 = 6.0;
        zero.positions.push(p);
        let m = zero_metrics(&zero, &OperatorProfile::default(), &PlanContext::none());
        assert_eq!(m.global_status.shot_advice_action, "BATCH HARVEST SHOT (Execute Claim)");
        // An overextension has nothing to buy either: the harvest leads and the
        // cooldown fact rides along in the details.
        let hot = MarketRegime { rsi_5m: None, rsi_1d: Some(82.0), overextended: true };
        let m = run(&raw, &hot);
        assert_eq!(m.global_status.shot_advice_action, "BATCH HARVEST SHOT (Execute Claim)");
        assert!(m.global_status.shot_advice_details.contains("daily RSI(14) 82 is at or above 70"), "{}", m.global_status.shot_advice_details);
    }

    #[test]
    fn test_emptied_price_source_range_is_never_a_range_exit() {
        // The 2026-11-14 book: every range emptied, plan waiting, spot above the
        // price-source top. Nothing to exit, so the rebuild keeps the lead.
        let mut raw = rebuild_fixture();
        let src = raw.positions.iter().find(|p| p.is_price_source).unwrap().clone();
        let top = tick_to_display_price_dec(src.tick_lower, src.token0.decimals, src.token1.decimals);
        raw.spot_price_nock_usd = top * 1.01;
        let fills = PlanFills { count: 1, spent_usdc: 100.0, acquired_nock: 8000.0, last_at: Some(t("2026-11-15T12:00:00Z")), first_price: Some(0.0125) };
        let waiting = PlanContext { cfg: Some(plan_cfg()), fills, ledger_unavailable: false, now: t("2026-11-15T18:00:00Z") };
        let m = metrics_with_plan(&raw, &waiting, &MarketRegime::default());
        assert!(!m.global_status.shot_advice_action.contains("RANGE EXIT"), "{}", m.global_status.shot_advice_action);
        assert!(!m.global_status.shot_advice_details.contains("RANGE EXIT"));
    }

    #[test]
    fn test_dip_zone_crossing_still_notifies_under_the_rebuild_bias() {
        // No plan, LP under target with fundable suggestions: above $0.020 the rebuild
        // replaces HOLD; below it, it carries DIP ZONE, so the crossing changes the action.
        let mut raw = rebuild_fixture();
        let none = PlanContext::none();
        let profile = operator_profile_no_floor();
        let run = |raw: &RawOnChainData| compute_derived_metrics_with_plan(raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default(), &none);
        raw.spot_price_nock_usd = 0.025;
        let above = run(&raw);
        raw.spot_price_nock_usd = 0.01229;
        let below = run(&raw);
        assert_eq!(above.global_status.shot_advice_action, "REBUILD LP TRANCHES (No Net Cash Change)");
        assert_eq!(below.global_status.shot_advice_action, "REBUILD LP TRANCHES (No Net Cash Change) · DIP ZONE (No Buy Plan Configured)");
        assert!(directive_transition_notification(
            Some(&above.global_status.shot_advice_action),
            &below.global_status.shot_advice_action,
            &below.global_status.shot_advice_details,
            None
        )
        .is_some());
    }

    #[test]
    fn test_hold_label_survives_a_failed_pool_read_and_never_pushes_between_variants() {
        let mut raw = four_position_fixture();
        for p in &raw.positions {
            raw.pool_liquidity.insert(p.position_id.clone(), "80173935548453779".to_string());
        }
        let run = |raw: &RawOnChainData| compute_derived_metrics(raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        let a = run(&raw).global_status.shot_advice_action;
        // One cycle's pool reads fail: the keys are absent, which disproves nothing.
        raw.pool_liquidity.clear();
        let b = run(&raw).global_status.shot_advice_action;
        assert_eq!(a, "HOLD & ACCRETE (Dual Growth Engine Active)");
        assert_eq!(a, b);
        // A real change between HOLD variants (a range drifting out) does not push.
        assert!(directive_transition_notification(Some("HOLD & ACCRETE (Dual Growth Engine Active)"), "HOLD & ACCRETE (No Ranged LP Confirmed In Range)", "d", None).is_none());
        assert!(directive_transition_notification(Some("HOLD & ACCRETE (LP Empty)"), "DCA TRANCHE DUE (1 of 5)", "d", None).is_some());
    }

    #[test]
    fn test_missing_floor_field_is_unavailable_not_100000() {
        let g: GlobalStatus = serde_json::from_value(json!({
            "total_portfolio_usd": "1", "total_native_usd": "1", "total_stable_usd": "0",
            "total_fees_accrued_usd": "0", "total_fees_claimable_usd": "0", "gas_to_fee_delta_usd": "0",
            "high_water_mark_usd": "1", "high_water_mark_date": "x"
        }))
        .unwrap();
        assert_eq!(g.nock_floor_target, "");
        let v = serde_json::to_value(&g).unwrap();
        assert_eq!(floor_clause(Some(&v)), "floor unavailable");
    }

    #[test]
    fn test_shot_advice_mcp_states_the_real_harvest_rule_and_a_signed_net() {
        let data = json!({"derived_metrics":{"global_status":{"shot_advice_action":"HOLD & ACCRETE (LP Empty)","shot_advice_details":"d","total_fees_claimable_usd":"0.50","gas_to_fee_delta_usd":"-0.01"}}});
        let t = format_shot_advice_text(&data);
        assert!(t.contains("fixed $5.00 threshold"), "{t}");
        assert!(!t.contains("2× live gas"), "{t}");
        assert!(t.contains("-$0.01 USD") && !t.contains("+$-"), "{t}");
        // The engine no longer clamps a negative net to 0.
        let mut raw = four_position_fixture();
        raw.live_gas_cost_usd = 10.0;
        let m = compute_derived_metrics(&raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        assert!(m.global_status.gas_to_fee_delta_usd.starts_with('-'), "{}", m.global_status.gas_to_fee_delta_usd);
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

    // ---------------- Gas pillar (issue #94) ----------------

    fn gas_metrics(raw: &RawOnChainData) -> DerivedMetrics {
        compute_derived_metrics_with_plan(
            raw, 0.0, &operator_profile_2026_11(), StructuralChannels { channels: vec![] }, &[],
            &LedgerAggregate::default(), &MarketRegime::default(), &PlanContext::none(),
        )
    }

    fn usd_of(v: &str) -> f64 {
        v.parse::<f64>().unwrap()
    }

    #[test]
    fn test_gas_counted_in_total_never_cash_or_deployable() {
        let mut no_gas = four_position_fixture();
        no_gas.wallet_eth = Some(0.0);
        no_gas.wallet_weth = Some(0.0);
        let mut gas = four_position_fixture();
        gas.wallet_eth = Some(0.0125);
        gas.wallet_weth = Some(0.0);
        let (a, b) = (gas_metrics(&no_gas), gas_metrics(&gas));
        let px = 2373.14;
        let (ga, gb) = (&a.global_status, &b.global_status);
        // The total rises by exactly the wallet gas.
        assert!((usd_of(&gb.total_portfolio_usd) - usd_of(&ga.total_portfolio_usd) - 0.0125 * px).abs() <= 0.011);
        assert_eq!(gb.wallet_gas_eth.as_deref(), Some("0.012500"));
        assert_eq!(gb.wallet_gas_usd.as_deref(), Some(format!("{:.2}", 0.0125 * px).as_str()));
        assert_eq!(gb.eth_price_usd.as_deref(), Some("2373.14"));
        // Gas is never cash: the stable pillar and the USDC amount are unchanged.
        assert_eq!(ga.total_stable_usd, gb.total_stable_usd);
        assert_eq!(ga.total_usdc_amount, gb.total_usdc_amount);
        // ...and never dry powder: the liquid dry powder sentence names wallet USDC only.
        for g in [ga, gb] {
            assert!(!g.shot_advice_details.contains("ETH is deployable"));
            if let Some(i) = g.shot_advice_details.find("Liquid dry powder") {
                let rest = &g.shot_advice_details[i..];
                let sentence = &rest[..rest.find(").").map(|j| j + 2).unwrap_or(rest.len())];
                assert!(!sentence.contains("ETH"), "dry powder must be USDC only: {sentence}");
            }
        }
        // The liquid USDC bucket measures wallet USDC only; gas has its own row, no target.
        let row = |m: &DerivedMetrics, name: &str| m.portfolio_deviation_matrix.allocations.iter().find(|r| r.bucket_name.starts_with(name)).cloned().unwrap();
        let gas_row = row(&b, "Gas");
        assert_eq!((gas_row.target_weight_bps, gas_row.status.as_str()), (0, "no_target"));
        assert!(gas_row.actual_weight_bps > 0);
        let usdc_b = row(&b, "Liquid USDC");
        let expect = ((134.4148 / usd_of(&gb.total_portfolio_usd)) * 10000.0) as i32;
        assert_eq!(usdc_b.actual_weight_bps, expect);
        // Weights are shares of the one full total: the four rows cover it.
        let sum: i32 = b.portfolio_deviation_matrix.allocations.iter().map(|r| r.actual_weight_bps).sum();
        assert!((9996..=10000).contains(&sum), "weights sum to {sum}");
    }

    #[test]
    fn test_pillars_partition_total() {
        let mut raw = four_position_fixture();
        raw.wallet_eth = Some(0.0125);
        raw.wallet_weth = Some(0.003);
        let m = gas_metrics(&raw);
        let g = &m.global_status;
        let native = usd_of(&g.total_native_usd);
        let stable = usd_of(&g.total_stable_usd);
        let eth = usd_of(g.total_eth_usd.as_deref().unwrap());
        let total = usd_of(&g.total_portfolio_usd);
        assert!((native + stable + eth - total).abs() <= 0.02, "{native} + {stable} + {eth} != {total}");
        // The ETH pillar is wallet gas plus every ETH/WETH leg in LP.
        let lp_eth: f64 = raw.positions.iter().map(|p| {
            (if is_eth_token(&p.token0) { p.amount0 } else { 0.0 }) + (if is_eth_token(&p.token1) { p.amount1 } else { 0.0 })
        }).sum();
        assert!(lp_eth > 0.0, "fixture must hold ETH in LP");
        let amount = usd_of(g.total_eth_amount.as_deref().unwrap());
        assert!((amount - (0.0155 + lp_eth)).abs() < 1e-6, "{amount}");
        assert!(eth >= usd_of(g.wallet_gas_usd.as_deref().unwrap()));
    }

    #[test]
    fn test_moving_eth_wallet_to_lp_is_total_neutral() {
        let mut before = four_position_fixture();
        before.wallet_eth = Some(0.05);
        before.wallet_weth = Some(0.0);
        let mut after = four_position_fixture();
        after.wallet_eth = Some(0.04);
        after.wallet_weth = Some(0.0);
        after.positions.iter_mut().find(|p| p.nft_id == 6336662).unwrap().amount0 += 0.01;
        let (a, b) = (gas_metrics(&before), gas_metrics(&after));
        assert!((usd_of(&a.global_status.total_portfolio_usd) - usd_of(&b.global_status.total_portfolio_usd)).abs() <= 0.01);
        assert!((usd_of(a.global_status.total_eth_usd.as_deref().unwrap()) - usd_of(b.global_status.total_eth_usd.as_deref().unwrap())).abs() <= 0.01);
        // The high-water mark sees no gain from the move.
        assert!((usd_of(&a.global_status.high_water_mark_usd) - usd_of(&b.global_status.high_water_mark_usd)).abs() <= 0.01);
    }

    #[test]
    fn test_unread_or_unpriced_gas_is_null_never_zero() {
        let mut raw = four_position_fixture();
        raw.wallet_eth = None;
        raw.wallet_weth = Some(0.0);
        let m = gas_metrics(&raw);
        let g = &m.global_status;
        assert!(g.wallet_gas_eth.is_none() && g.wallet_gas_usd.is_none());
        assert!(g.total_eth_amount.is_none() && g.total_eth_usd.is_none());
        let v = serde_json::to_value(g).unwrap();
        for k in ["wallet_gas_eth", "wallet_gas_usd", "total_eth_amount", "total_eth_usd"] {
            assert!(v[k].is_null(), "{k} must be null, got {}", v[k]);
        }
        // Unpriced ETH: the amount is known, its USD value is not.
        let mut raw = four_position_fixture();
        raw.wallet_eth = Some(0.01);
        raw.wallet_weth = Some(0.0);
        raw.live_eth_price_usd = 0.0;
        let g = gas_metrics(&raw).global_status;
        assert_eq!(g.wallet_gas_eth.as_deref(), Some("0.010000"));
        assert!(g.wallet_gas_usd.is_none() && g.total_eth_usd.is_none() && g.eth_price_usd.is_none());
        // Older snapshots without the 2.3.0 fields still parse.
        let old: GlobalStatus = serde_json::from_value(json!({
            "total_portfolio_usd": "1", "total_native_usd": "1", "total_stable_usd": "0",
            "total_fees_accrued_usd": "0", "total_fees_claimable_usd": "0", "gas_to_fee_delta_usd": "0",
            "high_water_mark_usd": "1", "high_water_mark_date": "x"
        })).unwrap();
        assert!(old.total_eth_usd.is_none() && old.eth_price_usd.is_none());
    }

    #[test]
    fn test_gas_pillars_helper() {
        let p = gas_pillars(Some(0.01), Some(0.002), 0.02, 2000.0);
        assert!((p.wallet_gas_eth.unwrap() - 0.012).abs() < 1e-12);
        assert!((p.wallet_gas_usd.unwrap() - 24.0).abs() < 1e-9);
        assert!((p.total_eth_amount.unwrap() - 0.032).abs() < 1e-12);
        assert!((p.total_eth_usd.unwrap() - 64.0).abs() < 1e-9);
        assert_eq!(gas_pillars(Some(0.01), None, 0.02, 2000.0), GasPillars::default());
        assert_eq!(gas_pillars(Some(0.01), Some(0.0), 0.0, f64::NAN).wallet_gas_usd, None);
    }

    #[test]
    fn test_gas_balances_required() {
        assert_eq!(require_gas_balances(Some(0.0125), Some(0.0)).unwrap(), (0.0125, 0.0));
        let e = require_gas_balances(None, Some(0.0)).unwrap_err().to_string();
        assert!(e.contains("ETH") && e.contains("gas"), "{e}");
        let w = require_gas_balances(Some(0.1), None).unwrap_err().to_string();
        assert!(w.contains("WETH"), "{w}");
        assert!(require_gas_balances(None, None).unwrap_err().to_string().contains("ETH and WETH"));
    }

    #[test]
    fn test_every_wallet_token_balance_must_be_present() {
        let ok = json!({"result": {"tokenBalances": [
            {"contractAddress": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913", "tokenBalance": "0x3b9aca00"},
            {"contractAddress": "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722", "tokenBalance": "0x2386f26fc10000"}
        ]}});
        let (nock, usdc) = parse_wallet_token_balances(&ok).unwrap();
        assert!((usdc - 1000.0).abs() < 1e-9 && (nock - 1.0).abs() < 1e-9, "{nock} {usdc}");
        // A null balance is a failed read, never 0.
        let null_usdc = json!({"result": {"tokenBalances": [
            {"contractAddress": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913", "tokenBalance": null},
            {"contractAddress": "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722", "tokenBalance": "0x1"}
        ]}});
        assert!(parse_wallet_token_balances(&null_usdc).unwrap_err().to_string().contains("USDC"));
        // A missing token fails too (one present token used to be enough).
        let only_usdc = json!({"result": {"tokenBalances": [
            {"contractAddress": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913", "tokenBalance": "0x1"}
        ]}});
        assert!(parse_wallet_token_balances(&only_usdc).unwrap_err().to_string().contains("NOCK"));
        let errored = json!({"result": {"tokenBalances": [
            {"contractAddress": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913", "tokenBalance": "0x1"},
            {"contractAddress": "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722", "tokenBalance": null, "error": "rate limited"}
        ]}});
        assert!(parse_wallet_token_balances(&errored).is_err());
        assert!(parse_wallet_token_balances(&json!({"error": {"code": 429}})).is_err());
        // "0x" is an explicit zero balance, not a failed read.
        let zero = json!({"result": {"tokenBalances": [
            {"contractAddress": "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913", "tokenBalance": "0x"},
            {"contractAddress": "0x9b5e262cf9bb04869ab40b19af91d2dc85761722", "tokenBalance": "0x0"}
        ]}});
        assert_eq!(parse_wallet_token_balances(&zero).unwrap(), (0.0, 0.0));
    }

    #[test]
    fn test_eth_price_pool_needs_in_range_liquidity() {
        assert!(check_eth_price_pool_liquidity("0xd0b5", 1).is_ok());
        let e = check_eth_price_pool_liquidity("0xd0b5", 0).unwrap_err().to_string();
        assert!(e.contains("Stale ETH/USD") && e.contains("0xd0b5"), "{e}");
    }

    #[test]
    fn test_hwm_date_is_the_peak_rows_time_not_now() {
        let peak_at = t("2026-11-14T04:38:29Z");
        let now = "2026-11-14T18:11:04+00:00";
        // Below the stored peak: the date is when the peak was set.
        assert_eq!(resolve_high_water_mark_date(Some((2475.27, peak_at)), 2390.58, now), peak_at.to_rfc3339());
        // A tie keeps the earlier mark (the query orders ties by generated_at ASC).
        assert_eq!(resolve_high_water_mark_date(Some((2475.27, peak_at)), 2475.27, now), peak_at.to_rfc3339());
        // A new mark is set now.
        assert_eq!(resolve_high_water_mark_date(Some((2475.27, peak_at)), 2500.0, now), now);
        // Empty table (a fresh start): the current value is the first mark.
        assert_eq!(resolve_high_water_mark_date(None, 10.0, now), now);
    }

    #[test]
    fn test_snapshot_version_matches_the_spec() {
        let spec = include_str!("../../specs/core/snapshot.yaml");
        assert!(
            spec.contains(&format!("version: {}", SNAPSHOT_SPEC_VERSION))
                || spec.contains(&format!("version: \"{}\"", SNAPSHOT_SPEC_VERSION))
                || spec.contains(&format!("version: '{}'", SNAPSHOT_SPEC_VERSION)),
            "specs/core/snapshot.yaml info.version must be {SNAPSHOT_SPEC_VERSION}"
        );
    }

    #[test]
    fn test_mcp_state_text_reports_gas_and_pillars() {
        let data = json!({"derived_metrics": {"global_status": {
            "total_portfolio_usd": "2413.57", "total_native_usd": "1312.88", "total_stable_usd": "1033.71",
            "total_eth_usd": "66.98", "wallet_gas_eth": "0.012500", "wallet_gas_usd": "22.99",
            "total_nock_amount": "115000", "nock_floor_target": "0", "nock_floor_delta": "115000",
            "total_usdc_amount": "1033.71", "high_water_mark_usd": "2475.27", "shot_advice_action": "HOLD"
        }}});
        let text = format_portfolio_state_text(&data);
        assert!(text.contains("Pillars: NOCK $1312.88 · USDC $1033.71 · ETH $66.98"), "{text}");
        assert!(text.contains("Gas held (wallet ETH+WETH): 0.012500 ETH ($22.99), counted in the total; not cash, not deployable"), "{text}");
        // A pre-2.3.0 snapshot says so, never $0.00.
        let mut old = data.clone();
        for k in ["total_eth_usd", "wallet_gas_eth", "wallet_gas_usd"] {
            old["derived_metrics"]["global_status"].as_object_mut().unwrap().remove(k);
        }
        let text = format_portfolio_state_text(&old);
        assert!(text.contains("ETH not reported (snapshot predates 2.3.0)"), "{text}");
        assert!(text.contains("Gas held (wallet ETH+WETH): not reported (snapshot predates 2.3.0)"), "{text}");
        assert!(!text.contains("$0.00"));
    }

    // ---------------- Golden snapshot (issue #97) ----------------
    // Pins the whole pure path from one cycle's reads to the stored payload, the state
    // hash and the MCP text, byte for byte. Added before the module split; a move-only
    // refactor must leave ingestion-worker/testdata/golden_snapshot.json untouched.
    // Regenerate only for an intended behaviour change:
    //   NOCKNOCK_BLESS_GOLDEN=1 cargo test test_golden_snapshot

    /// The steps fetch_raw_data (its tail) and run_ingestion_cycle apply after the reads.
    #[allow(clippy::too_many_arguments)]
    fn golden_snapshot(
        mut raw: RawOnChainData,
        prev_hwm: f64,
        profile: &OperatorProfile,
        history: &[(DateTime<Utc>, f64)],
        counterfactual: &[(DateTime<Utc>, f64, f64)],
        ledger: &LedgerAggregate,
        regime: &MarketRegime,
        plan: &PlanContext,
    ) -> SnapshotData {
        let (spot, eth) = (raw.spot_price_nock_usd, raw.live_eth_price_usd);
        let mut positions = build_position_snapshots(&raw.positions, spot, eth);
        apply_pool_activity(&mut positions, &raw.pool_liquidity);
        raw.pool_mechanics = build_pool_mechanics(&raw.positions, spot, eth);
        raw.contract_fees = build_contract_fees(&positions, raw.live_gas_cost_usd, &raw.fees_not_lp_claimable);
        let derived_metrics = compute_derived_metrics_with_plan(
            &raw, prev_hwm, profile, compute_structural_channels_sync(history), counterfactual, ledger, regime, plan,
        );
        SnapshotData {
            wallet_balances: raw.wallet_balances,
            positions,
            pool_mechanics: raw.pool_mechanics,
            concentrated_liquidity: raw.concentrated_liquidity,
            contract_fees: raw.contract_fees,
            derived_metrics,
        }
    }

    fn golden_case(data: &SnapshotData) -> serde_json::Value {
        let full = serde_json::to_value(data).unwrap();
        json!({
            "state_hash": compute_state_hash(data),
            "snapshot": canonical_state_json(data),
            "mcp_text": {
                "get_portfolio_state": format_portfolio_state_text(&full),
                "get_shot_advice": format_shot_advice_text(&full),
                "simulate_claim_fees": format_claim_simulation_text(&full),
                "get_accumulation_plan": format_accumulation_plan_text(&full),
            },
        })
    }

    #[test]
    fn test_golden_snapshot_is_byte_identical() {
        let no_plan = PlanContext { cfg: None, fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-15T12:00:00Z") };

        // 1. The fixed fixture named by #97, exactly as the screenshot test uses it.
        let four = golden_snapshot(
            four_position_fixture(), 2500.0, &OperatorProfile::default(), &[], &[],
            &LedgerAggregate::default(), &MarketRegime::default(), &no_plan,
        );

        // 2. Every adapter and engine input at once: four Uniswap positions, the Aero
        //    volatile pool with a reverting claimFees(), gas read, pool activity, a
        //    harvest ledger, both RSI reads, history, and a plan with a tranche due.
        let mut raw = rebuild_fixture();
        let mut aero = aero_position(false);
        // 0.01% of the pool instead of the fixture's 1%, so LP stays under its target.
        aero.liquidity /= 100;
        aero.amount0 /= 100.0;
        aero.amount1 /= 100.0;
        raw.pool_liquidity.insert(aero.position_id.clone(), "342542824101829799".to_string());
        raw.fees_not_lp_claimable = vec![aero.position_id.clone()];
        raw.positions.push(aero);
        let history: Vec<(DateTime<Utc>, f64)> = (0..22)
            .map(|i| (t("2026-11-14T00:00:00Z") + chrono::Duration::minutes(5 * i), 2400.0 + (i as f64) * 1.25))
            .collect();
        let counterfactual = [
            (t("2026-11-14T00:00:00Z"), 2400.0, 1400.0),
            (t("2026-11-14T00:05:00Z"), 2401.25, 1400.5),
        ];
        let ledger = LedgerAggregate { total_usd: 12.5, harvest_count: 2, last_at: Some(t("2026-11-13T09:30:00Z")), unpriced_count: 1 };
        let regime = MarketRegime { rsi_5m: Some(48.2), rsi_1d: Some(55.4), overextended: false };
        let plan = PlanContext { cfg: Some(plan_cfg()), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-15T12:00:00Z") };
        let whole = golden_snapshot(raw, 2600.0, &operator_profile_2026_11(), &history, &counterfactual, &ledger, &regime, &plan);

        // 3. A wallet starting from zero: USDC only, no positions, no plan, no rules.
        let zero = golden_snapshot(
            from_zero_raw(500.0, 0.0), 0.0, &OperatorProfile::default(), &[], &[],
            &LedgerAggregate::default(), &MarketRegime::default(), &no_plan,
        );

        let doc = json!({
            "four_position_fixture": golden_case(&four),
            "whole_book_plan_due": golden_case(&whole),
            "from_zero": golden_case(&zero),
        });
        let actual = format!("{}\n", serde_json::to_string_pretty(&doc).unwrap());
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/golden_snapshot.json");
        if std::env::var("NOCKNOCK_BLESS_GOLDEN").is_ok() {
            std::fs::write(path, &actual).unwrap();
        }
        let expected = std::fs::read_to_string(path).expect("testdata/golden_snapshot.json is checked in");
        assert!(
            actual == expected,
            "snapshot bytes changed; compare {path} with a fresh NOCKNOCK_BLESS_GOLDEN=1 run and `git diff` it"
        );
    }
}
