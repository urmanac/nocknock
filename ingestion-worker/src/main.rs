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
mod valuation;
use valuation::*;
mod onchain;
use onchain::*;
mod lp_rebuild;
use lp_rebuild::*;
mod advice;
use advice::*;
mod ledger;
use ledger::*;
mod ingest;
use ingest::*;
mod api;
use api::*;
mod claims;
use claims::*;
mod mcp_http;
use mcp_http::*;
mod cost_basis;
use cost_basis::*;
mod cost_basis_text;
use cost_basis_text::*;

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

/// Cost-basis inputs only the operator can supply (issue #98). Optional: without this
/// section NockNock reports the average cost of the plan's receipt-verified buys and
/// nothing else. It never fills these in and never derives them from the chain.
#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
pub struct CostBasisConfig {
    /// The operator's own figures for the NOCK held at one instant, read from wherever
    /// they keep their records. Unset = the whole-stack basis is "not configured".
    #[serde(default)]
    pub opening: Option<OpeningBasisConfig>,
}

/// An operator-entered opening basis. Every field is read loosely (any scalar) and then
/// checked by `resolve_opening_basis`, so a half-filled block or a placeholder word is
/// reported as a configuration error in the snapshot instead of stopping the worker or
/// being replaced by a default.
#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
pub struct OpeningBasisConfig {
    /// NOCK held at `as_of`. Must be greater than zero.
    #[serde(default)]
    pub nock_amount: Option<serde_json::Value>,
    /// What that NOCK cost in total, in USD. Must be greater than zero.
    #[serde(default)]
    pub cost_usd: Option<serde_json::Value>,
    /// The instant the figures were read: a full timestamp with a UTC offset, not in the
    /// future. Buys confirmed at or before it are taken to be inside the figures.
    #[serde(default)]
    pub as_of: Option<serde_json::Value>,
    /// Where the figures came from, in the operator's words. Shown beside every number
    /// that uses them.
    #[serde(default)]
    pub source: Option<serde_json::Value>,
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
    /// Optional operator-supplied cost-basis inputs (issue #98).
    #[serde(default)]
    cost_basis: CostBasisConfig,
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
    /// NOCK cost basis (v0.6.0, issue #98). Null in a snapshot stored before the field
    /// existed: unknown, never a zero basis.
    #[serde(default)]
    cost_basis: Option<CostBasis>,
}

/// NOCK cost basis in two layers (v0.6.0, issue #98). `plan_buys` is exact: it comes only
/// from receipt-verified rows in the accumulation ledger. `whole_stack` exists only when
/// the operator enters an opening basis in config; NockNock cannot derive it from the
/// chain and never invents it.
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
struct CostBasis {
    /// NOCK/USD spot the differences below are measured at; null when there is no price.
    spot_price_usd: Option<String>,
    plan_buys: PlanBuysBasis,
    whole_stack: WholeStackBasis,
}

/// Average cost of the receipt-verified buys, across every plan in the ledger.
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
struct PlanBuysBasis {
    /// ok | no_fills | unavailable
    status: String,
    /// Why the figures are unavailable; null otherwise.
    reason: Option<String>,
    fills: i64,
    spent_usdc: Option<String>,
    acquired_nock: Option<String>,
    /// Σ USDC spent / Σ NOCK received; null without a verified buy.
    avg_cost_usd: Option<String>,
    first_fill_at: Option<String>,
    last_fill_at: Option<String>,
    /// `acquired_nock` at the current spot; null when there is no price.
    value_at_spot_usd: Option<String>,
    /// `value_at_spot_usd` less `spent_usdc`, signed. A measurement, not advice.
    unrealized_usd: Option<String>,
    /// The same difference as a percentage of `spent_usdc`, signed.
    unrealized_pct: Option<String>,
    /// The same figures for each plan id, oldest plan first.
    #[serde(default)]
    plans: Vec<PlanBasis>,
    /// Exactly what these figures count, and what they leave out.
    #[serde(default)]
    includes: Vec<String>,
    #[serde(default)]
    excludes: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
struct PlanBasis {
    plan_id: String,
    fills: i64,
    spent_usdc: String,
    acquired_nock: String,
    avg_cost_usd: String,
    first_fill_at: String,
    last_fill_at: String,
}

/// Blended average of the operator-entered opening basis and the verified buys made
/// after it: (opening cost + USDC spent since) / (opening NOCK + NOCK bought since).
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
struct WholeStackBasis {
    /// not_configured | ok | invalid | unavailable
    status: String,
    /// The configuration error (invalid) or the failed read (unavailable); null otherwise.
    reason: Option<String>,
    /// The operator's own note on where the opening figures came from.
    source: Option<String>,
    /// The instant the opening figures were read (RFC 3339, UTC).
    as_of: Option<String>,
    opening_nock: Option<String>,
    opening_cost_usd: Option<String>,
    opening_avg_cost_usd: Option<String>,
    /// Verified buys confirmed after `as_of`, and their totals.
    fills_since: Option<i64>,
    bought_since_usdc: Option<String>,
    bought_since_nock: Option<String>,
    /// Opening plus bought since: the NOCK and the cost this average covers.
    basis_nock: Option<String>,
    basis_cost_usd: Option<String>,
    avg_cost_usd: Option<String>,
    /// `basis_nock` at the current spot, and its difference from `basis_cost_usd`.
    value_at_spot_usd: Option<String>,
    unrealized_usd: Option<String>,
    unrealized_pct: Option<String>,
    /// NOCK NockNock reads now (wallet plus tracked LP), for comparison with
    /// `basis_nock`: the two differ by everything the basis does not track.
    held_nock: Option<String>,
    #[serde(default)]
    includes: Vec<String>,
    #[serde(default)]
    excludes: Vec<String>,
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
    // A half-filled cost_basis.opening does not stop the worker: every cycle reports it
    // as `whole_stack.status = invalid` with this reason. Say it once in the log too.
    if let OpeningBasisState::Invalid(reason) = resolve_opening_basis(config.cost_basis.opening.as_ref(), Utc::now()) {
        tracing::error!("Configuration error: {reason}");
    }

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
        .route(
            "/v1/cost-basis/series",
            axum::routing::get(cost_basis_series_handler),
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

#[derive(Deserialize)]
struct RecordAccumulationRequest {
    tx_hash: String,
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
#[derive(Deserialize, Debug)]
struct RecordHarvestRequest {
    platform: String,
    tx_hash: String,
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

// ============================================================
// Unit Tests
// ============================================================

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_cost_basis;
