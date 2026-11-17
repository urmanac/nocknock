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
mod aero_rewards;
use aero_rewards::*;
mod aero_text;
use aero_text::*;
mod fill_verify;
use fill_verify::*;
mod fill_detection;
use fill_detection::*;
mod fill_text;
use fill_text::*;
mod route_quotes;
use route_quotes::*;
mod quote_text;
use quote_text::*;
mod cost_basis;
use cost_basis::*;
mod cost_basis_text;
use cost_basis_text::*;
mod rewards;
use rewards::*;
mod reward_text;
use reward_text::*;

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
    /// "uniswap-v4" | "uniswap-v3" | "aerodrome" (volatile x*y=k pool, full range) |
    /// "aerodrome-cl" (Aero concentrated-liquidity NFT, staked or not; v0.6.0, #91).
    /// Any other value is a configuration error (adapters::validate_positions_config).
    pub protocol: String,
    #[serde(default = "default_network")]
    pub network: String,
    /// Position NFT id (Uniswap, aerodrome-cl). Unused for aerodrome, whose position is
    /// an LP-token balance in `pool_address`.
    #[serde(default)]
    pub nft_id: u64,
    /// aerodrome only: swap fee in basis points, for display (default 30 = 0.30%).
    #[serde(default)]
    pub fee_bps: Option<u32>,
    /// aerodrome: the pool's gauge. LP tokens staked there (the aero.xyz app stakes
    /// on deposit) are added to the wallet's own balance; without it a staked position
    /// reads as empty.
    /// aerodrome-cl: optional. The gauge is read from the pool itself; when this is set
    /// it must match, or the cycle fails with a configuration error.
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
    /// v3: the canonical NonfungiblePositionManager for the network.
    /// aerodrome-cl: the Aero CL (Slipstream) position manager that minted the NFT.
    #[serde(default)]
    pub npm_address: Option<String>,
    /// v3, aerodrome, aerodrome-cl: pool contract address (v3: pre-resolved via factory
    /// getPool; aerodrome-cl: checked each cycle against the NFT's tokens and tick spacing).
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

/// Fill detection for the accumulation plan (issue #118). It only reads the wallet's
/// NOCK transfer history and offers what it finds; it never records a buy. There are
/// no thresholds here: nothing is filtered by size or price.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FillDetectionConfig {
    /// On by default while an accumulation_plan is configured. Set false to stop the
    /// worker reading the transfer history; recording by transaction hash still works.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for FillDetectionConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// Route quotes for a due tranche (issue #115). Read-only quoter calls for the plan's own
/// tranche amount; nothing is signed, sized or sent. There are no numbers here: the
/// amount quoted is the tranche the plan produced, and the candidate routes are the
/// verified table in route_quotes.rs (ONCHAIN_INTEGRATION.md §11).
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RouteQuotesConfig {
    /// On by default while an accumulation_plan is configured. Set false to stop the
    /// worker calling the quoters; the due tranche is advised exactly as before.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for RouteQuotesConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
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

fn default_aero_price_pool() -> Option<String> {
    Some(DEFAULT_AERO_PRICE_POOL.to_string())
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
    /// AERO/USDC pool whose slot0 prices AERO, the fourth pillar (v0.6.0, #93). Verified
    /// default: the Aero CL200 AERO/USDC pool (ONCHAIN_INTEGRATION.md §9). Checked every
    /// cycle: its tokens must be AERO and USDC and it must have in-range liquidity.
    #[serde(default = "default_aero_price_pool")]
    aero_price_pool: Option<String>,
    #[serde(default)]
    mcp_bearer_token: Option<String>,
    #[serde(default)]
    operator_profile: OperatorProfile,
    /// Optional dollar-cost-averaging plan (issue #79).
    #[serde(default)]
    accumulation_plan: Option<AccumulationPlanConfig>,
    /// Fill detection for the plan (issue #118); on unless switched off.
    #[serde(default)]
    fill_detection: FillDetectionConfig,
    /// Route quotes for a due tranche (issue #115); on unless switched off.
    #[serde(default)]
    route_quotes: RouteQuotesConfig,
    /// Optional operator-supplied cost-basis inputs (issue #98). An Option so that a bare
    /// `cost_basis:` key with everything under it commented out still loads.
    #[serde(default)]
    cost_basis: Option<CostBasisConfig>,
    #[serde(default)]
    notifications: NotificationConfig,
    #[serde(default)]
    market_data: MarketDataConfig,
}

impl AppConfig {
    pub fn profile(&self) -> OperatorProfile {
        self.operator_profile.clone()
    }
    /// The operator's opening basis block as written, unchecked (issue #98).
    fn opening_basis(&self) -> Option<&OpeningBasisConfig> {
        self.cost_basis.as_ref().and_then(|c| c.opening.as_ref())
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
    /// LP-claimable (shown as such, never as $0 earned). Since v0.6.0 (#91) also an
    /// Aero CL position staked in its gauge (swap fees go to the gauge); its
    /// `position_facts` entry carries the reason.
    fees_not_lp_claimable: Vec<String>,
    /// Facts only some protocols have (tick spacing, gauge staking, why a fee figure is
    /// absent), keyed by position_id (v0.6.0, #91). No entry means none apply.
    position_facts: HashMap<String, PositionFacts>,
    /// AERO, the fourth pillar (v0.6.0, #93): wallet AERO, the AERO/USD price and the
    /// unclaimed AERO of each staked Aero position. The default (test fixtures that do
    /// not set it) is "nothing read": every AERO field is then null, never zero.
    aero: AeroReads,
}

/// One cycle's AERO reads (v0.6.0, #93). Built by fetch_aero_reads and test fixtures.
#[derive(Debug, Default, Clone, PartialEq)]
struct AeroReads {
    /// Wallet AERO in human units. A required read in production (it is counted in the
    /// total, so a failed read fails the cycle); None only in fixtures.
    wallet_aero: Option<f64>,
    /// AERO/USD from `aero_price_pool`, stale-pool guarded. None when it could not be
    /// read; `price_note` then says why. Required only while the wallet holds AERO.
    price_usd: Option<f64>,
    price_note: Option<String>,
    /// One entry per tracked Aero position that can earn AERO. None = not read at all
    /// (fixtures); Some(empty) = read, and no tracked position is staked.
    unclaimed: Option<Vec<AeroUnclaimed>>,
}

/// What one cycle learned about one position's unclaimed AERO (v0.6.0, #93).
#[derive(Debug, Clone, PartialEq)]
struct AeroUnclaimed {
    position_id: String,
    /// The gauge `earned` was read from; None when the position has no gauge to read.
    gauge: Option<String>,
    /// Unclaimed AERO in human units; None when it was not read (`note` says why).
    amount: Option<f64>,
    note: Option<String>,
}

/// The AERO pillar's figures, each None when it cannot be stated: an unread balance,
/// an unread gauge, or an unpriced AERO. Never a zero stand-in. (v0.6.0, #93)
#[derive(Debug, Default, Clone, PartialEq)]
struct AeroPillar {
    wallet_amount: Option<f64>,
    wallet_usd: Option<f64>,
    unclaimed_amount: Option<f64>,
    unclaimed_usd: Option<f64>,
    price_usd: Option<f64>,
    /// Stated reason for every None above that a live cycle can produce.
    note: Option<String>,
}

/// Per-position facts that only some protocols have (v0.6.0, #91). Carried beside
/// RawPosition rather than in it; stamped onto the snapshot by apply_position_facts.
#[derive(Debug, Default, Clone, PartialEq)]
struct PositionFacts {
    /// Aero CL: the pool's tick spacing (the "200" in CL200).
    tick_spacing: Option<i32>,
    /// Aero CL: the gauge holding the NFT, when the tracked wallet staked it there.
    staked_in_gauge: Option<String>,
    /// Why this position reports no LP-claimable fee figure. None when fees were read.
    fees_note: Option<String>,
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
    /// Aero CL only: the pool's tick spacing, e.g. 200 for a CL200 pool. Absent for
    /// every other protocol. (v0.6.0, #91)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tick_spacing: Option<i32>,
    /// Aero CL only: the gauge the tracked wallet staked this NFT in. Absent when the
    /// wallet holds the NFT itself and for every other protocol. (v0.6.0, #91)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    staked_in_gauge: Option<String>,
    /// Present when tokens_owed_* and fees_usd are NOT a read of LP-claimable fees:
    /// the reason, to show in their place (never "$0 earned"). Absent when fees were
    /// read. (v0.6.0, #91)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fees_note: Option<String>,
    /// AERO this position has earned in its gauge and not claimed, in AERO (gauge
    /// `earned`). Present only for an Aero position whose gauge was read. Not counted in
    /// amount_usd, fees_usd or the portfolio total. (v0.6.0, #93)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    unclaimed_aero: Option<String>,
    /// unclaimed_aero at this cycle's AERO/USD price; absent when AERO is unpriced. (v0.6.0, #93)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    unclaimed_aero_usd: Option<String>,
    /// Why unclaimed_aero is absent for an Aero position that could earn AERO: no gauge
    /// is configured, or the gauge read failed. Never shown as 0. (v0.6.0, #93)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    unclaimed_aero_note: Option<String>,
    /// The position's gauge reward as collectible earnings (#142): the token, the amount,
    /// its USD value, the gauge it is collected from and how. Present for every Aero
    /// position that can earn a reward, also when the gauge could not be read (`status`
    /// is then `not_read` and `note` says why). The same figures as `unclaimed_aero*`,
    /// structured so a consumer need not parse a note. A statement of fact, never
    /// advice, and not counted in amount_usd, fees_usd or the portfolio total.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    collectible_reward: Option<CollectibleReward>,
}

/// A gauge reward stated as collectible earnings (#142). It says what is there and where
/// it is collected. It never says to claim, sell, hold or accumulate it.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct CollectibleReward {
    /// read | not_read. `not_read` has no amount and no USD value, and `note` says why.
    status: String,
    /// "AERO".
    token_symbol: String,
    token_address: String,
    /// Reward earned and not collected, in the token, 6 decimals (gauge `earned`).
    /// Null when the gauge was not read: never 0 for something not read.
    #[serde(default)]
    amount: Option<String>,
    /// `amount` at the cycle's AERO/USD price, 2 decimals. Null when not read or when
    /// AERO is unpriced. "0.00" for a read zero amount, which needs no price.
    #[serde(default)]
    usd: Option<String>,
    /// The gauge the reward is collected from. Null when the position has none configured.
    #[serde(default)]
    gauge: Option<String>,
    /// How it is collected: "the Aero app". NockNock builds no transaction for it.
    collected_in: String,
    /// The Aero app (app.aero.xyz, never the retired aerodrome.finance).
    collect_url: String,
    /// Why the reward was not read. Null when it was.
    #[serde(default)]
    note: Option<String>,
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
    /// `gauge_reward` when this position's reported earnings are its gauge reward: its
    /// LP fees are not LP-claimable and it has a `reward`. Absent for a row whose
    /// earnings are LP fees. Never claim_now, claim_recommended or accumulate. (#142)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    earnings_kind: Option<String>,
    /// The position's gauge reward (positions[].collectible_reward), when it has one. (#142)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reward: Option<CollectibleReward>,
    /// The reason alone why the LP fees are not LP-claimable; absent when they are. (#142)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lp_fees_note: Option<String>,
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
    /// Verified buys counted. Null when the ledger could not be read (unknown, not 0);
    /// 0 only when it was read and holds none.
    fills: Option<i64>,
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
    /// NOCK NockNock reads now (wallet plus tracked LP), to the nearest whole NOCK, for
    /// comparison with `basis_nock`: the two differ by everything the basis does not track.
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
    /// Swaps found on-chain that look like plan buys, offered for confirmation, and
    /// whether the check ran (v0.6.0, issue #118). Offers are not progress: `fills`,
    /// `spent_usdc` and the cadence count ledger rows only. Null in snapshots from
    /// before v0.6.0.
    #[serde(default)]
    fill_detection: Option<FillDetection>,
    /// Read-only quotes for this exact tranche on Aero and on Uniswap (v0.7.0, issue
    /// #115). Present only while `status` is `tranche_due` and quoting is on; null
    /// otherwise and in older snapshots. A quote is a contract read at a stated block:
    /// the whole object is left out of the state hash, so it never forces a new
    /// snapshot row.
    #[serde(default)]
    route_quotes: Option<RouteQuotes>,
}

/// Quotes for one due tranche, both venues read at the same block (v0.7.0, issue #115).
/// It compares execution routes for a buy the operator's plan already sized. It is not
/// a recommendation to buy, and nothing here is an estimate: a venue that could not be
/// quoted is `unavailable` with the reason.
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
struct RouteQuotes {
    /// ok (both venues quoted) | partial (one) | unavailable (none).
    status: String,
    /// Plain-words reason for partial and unavailable.
    #[serde(default)]
    detail: Option<String>,
    /// The plan's own tranche amount that was quoted, in USDC. Never another size.
    amount_usdc: String,
    /// Block every quote was read at, and that block's time. Null when no block was read.
    #[serde(default)]
    block_number: Option<i64>,
    #[serde(default)]
    quoted_at: Option<String>,
    /// The snapshot's NOCK/USD reference spot the costs are measured against.
    #[serde(default)]
    reference_spot_usd: Option<String>,
    #[serde(default)]
    venues: Vec<VenueQuote>,
    /// The venue that returns more NOCK at that block. Null unless both venues were
    /// quoted and their amounts differ.
    #[serde(default)]
    more_nock_venue: Option<String>,
    /// How much more, in percent of the other venue's amount.
    #[serde(default)]
    more_nock_by_pct: Option<String>,
    /// The comparison as one sentence of fact.
    #[serde(default)]
    comparison: String,
}

/// One venue's best verified route for the tranche, or why it could not be quoted.
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
struct VenueQuote {
    /// "Aero" | "Uniswap"
    venue: String,
    /// quoted | unavailable
    status: String,
    /// Why, when unavailable.
    #[serde(default)]
    reason: Option<String>,
    /// The venue's own swap page. No token or amount is prefilled.
    swap_url: String,
    #[serde(default)]
    nock_out: Option<String>,
    /// USDC per NOCK for this amount along this route, eight decimals.
    #[serde(default)]
    effective_price_usd: Option<String>,
    /// (effective price / reference spot − 1) × 100. Null without a reference spot.
    #[serde(default)]
    cost_over_spot_pct: Option<String>,
    /// The best route's tokens, e.g. "USDC → WETH → NOCK".
    #[serde(default)]
    route: Option<String>,
    #[serde(default)]
    hops: Vec<RouteHop>,
    /// Every candidate route tried for this venue with its own result.
    #[serde(default)]
    routes_tried: Vec<RouteAttempt>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
struct RouteHop {
    /// Pool address, or the pool id for a Uniswap v4 pool.
    pool: String,
    /// "Aero basic volatile" | "Aero concentrated" | "Uniswap v3" | "Uniswap v4"
    pool_kind: String,
    token_in: String,
    token_out: String,
    /// Swap fee in percent. Null when the fee read failed (the quote itself stands).
    #[serde(default)]
    fee_pct: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
struct RouteAttempt {
    route_id: String,
    route: String,
    #[serde(default)]
    nock_out: Option<String>,
    /// Why this route gave no quote.
    #[serde(default)]
    reason: Option<String>,
}

/// One cycle's fill-detection result (v0.6.0, issue #118).
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
struct FillDetection {
    /// ok | partial | unavailable | not_run. `unavailable` means "could not check",
    /// which is not "checked, nothing found" (`ok` with no offers).
    status: String,
    /// Plain-words reason for partial, unavailable and not_run.
    #[serde(default)]
    detail: Option<String>,
    /// How far back the transfer history was read: the plan anchor after a complete
    /// check, a later time when the check was cut short. Null when it did not run.
    #[serde(default)]
    checked_back_to: Option<String>,
    /// Time of the last successful check. A wall-clock field: it is left out of the
    /// state hash, so it never forces a new snapshot row.
    #[serde(default)]
    last_checked_at: Option<String>,
    /// Detected swaps waiting for the operator: confirm, or mark as not part of the plan.
    #[serde(default)]
    offers: Vec<FillOffer>,
    /// Detected swaps the operator marked "not part of the plan". Kept and restorable.
    #[serde(default)]
    dismissed: Vec<FillOffer>,
}

/// A detected USDC→NOCK swap. Every figure comes from the transaction's receipt and
/// block header, checked with the rules of the record endpoint.
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
struct FillOffer {
    tx_hash: String,
    /// Block time of the swap.
    confirmed_at: String,
    block_number: i64,
    usdc_spent: String,
    nock_received: String,
    effective_price_usd: String,
    /// early | on_schedule: early when the swap landed less than one plan interval
    /// after the buy before it (a recorded fill, or an older swap still on offer).
    timing: String,
    /// Stored NOCK/USD price nearest the swap's block time; null when none was near.
    #[serde(default)]
    reference_spot_usd: Option<String>,
    #[serde(default)]
    reference_spot_source: Option<String>,
    /// (effective price / reference spot − 1) × 10,000; null without a reference.
    #[serde(default)]
    execution_cost_bps: Option<i32>,
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
    /// AERO/USD from the AERO price pool (v0.6.0, #93). Null when it could not be read
    /// (`aero_note` says why) or the field is absent from an older snapshot.
    #[serde(default)]
    aero_price_usd: Option<String>,
    /// Wallet AERO, the fourth pillar (v0.6.0, #93). Counted in total_portfolio_usd; never
    /// cash, never deployable, no target, and NockNock gives no advice about it.
    #[serde(default)]
    wallet_aero_amount: Option<String>,
    #[serde(default)]
    wallet_aero_usd: Option<String>,
    /// AERO earned in the gauges of tracked staked positions and not claimed (v0.6.0,
    /// #93). NOT counted in total_portfolio_usd, like uncollected LP fees. Null when any
    /// Aero position's gauge was not read (a failed read, a gauge that pays another
    /// token, or no gauge_address): never a partial sum, never 0.
    #[serde(default)]
    unclaimed_aero_amount: Option<String>,
    #[serde(default)]
    unclaimed_aero_usd: Option<String>,
    /// Stated reason whenever an AERO figure above is null in a live cycle (v0.6.0, #93).
    #[serde(default)]
    aero_note: Option<String>,
    /// Collectible gauge rewards of the tracked staked positions, in USD (#142): the
    /// tracked gauges' summed amount at the cycle's AERO price, the same figure as
    /// `unclaimed_aero_usd`. It can differ by a cent per position from the sum of the
    /// rounded positions[].collectible_reward.usd values. Null when a gauge was not read
    /// or a non-zero reward is unpriced (`collectible_rewards_note` says which): never a
    /// partial sum, never 0 for something not read. NOT in total_portfolio_usd, and no
    /// directive, notification or threshold reads it.
    #[serde(default)]
    collectible_rewards_usd: Option<String>,
    /// Why `collectible_rewards_usd` and `total_collectible_usd` are null in a live cycle.
    #[serde(default)]
    collectible_rewards_note: Option<String>,
    /// Collectible LP fees (`total_fees_claimable_usd`, which keeps meaning LP fees only)
    /// plus `collectible_rewards_usd` (#142). Null whenever the rewards part is null.
    /// A statement of what is collectible; NOT in total_portfolio_usd.
    #[serde(default)]
    total_collectible_usd: Option<String>,
}

/// Snapshot spec version the worker emits (specs/core/snapshot.yaml info.version).
const SNAPSHOT_SPEC_VERSION: &str = "2.7.0";

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
    /// With `not_claimable`: the position's own reason, when it has one (an Aero CL
    /// position staked in its gauge). Absent otherwise. (v0.6.0, #91)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    /// `gauge_reward` when the row's reported earnings are the position's gauge reward
    /// (`reward`), drawn as its own series. Only ever beside `not_claimable`: a row with
    /// a fee action (claim_now, claim_recommended, accumulate) carries no reward, so no
    /// harvest word ever labels AERO. Absent for a row whose earnings are LP fees. (#142)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    earnings_kind: Option<String>,
    /// The gauge reward: amount, USD, the gauge and how it is collected. (#142)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reward: Option<CollectibleReward>,
    /// With `not_claimable`: the reason alone, without the AERO statement `note` adds. (#142)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lp_fees_note: Option<String>,
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
    // An unknown position protocol fails every cycle with this message (visible in
    // /health); say it once at boot as well. The worker keeps serving so the error
    // can be read from the dashboard and MCP rather than from a crash loop.
    if let Err(e) = validate_positions_config(&config.positions) {
        tracing::error!("{e}");
    }
    // A half-filled cost_basis.opening does not stop the worker: every cycle reports it
    // as `whole_stack.status = invalid` with this reason. Say it once in the log too.
    if let OpeningBasisState::Invalid(reason) = resolve_opening_basis(config.opening_basis(), Utc::now()) {
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
        let mut data: SnapshotData = serde_json::from_value(data_val).ok()?;
        // Stored quotes are not current ones (issue #115); the first cycle reads new ones.
        forget_stored_route_quotes(&mut data);
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
            "/v1/accumulation/candidates",
            axum::routing::get(accumulation_candidates_handler),
        )
        .route(
            "/v1/accumulation/candidates/dismiss",
            axum::routing::post(dismiss_candidate_handler),
        )
        .route(
            "/v1/accumulation/candidates/restore",
            axum::routing::post(restore_candidate_handler),
        )
        .route(
            "/v1/accumulation/quotes/latest",
            axum::routing::get(latest_route_quotes_handler),
        )
        .route(
            "/v1/accumulation/quotes/refresh",
            axum::routing::post(refresh_route_quotes_handler),
        )
        .route(
            "/v1/tax/export",
            axum::routing::get(tax_export_handler),
        )
        .route(
            "/v1/notifications/status",
            axum::routing::get(notification_status_handler),
        )
        .route(
            "/v1/notifications/test",
            axum::routing::post(notification_test_handler),
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
    /// The Pushover *user key*.
    pushover_user: Option<String>,
    /// The same key under the name the example config documented until v0.5.3, while the
    /// worker read only `pushover_user`: a copied example left Pushover silently off
    /// (#89). Its own field, not a serde alias: an alias makes a config that carries both
    /// names fail to load ("duplicate field"), which stops the worker at startup. Read
    /// only through `notify::pushover_user_key`, where `pushover_user` wins.
    pushover_user_key: Option<String>,
    webhook_url: Option<String>,
    cockpit_url: Option<String>,
}

/// One notification channel's state (#89): what `GET /v1/notifications/status` and
/// `POST /v1/notifications/test` report per channel. `status` is `configured`,
/// `incomplete` or `not_configured` before a send, and `delivered` or `failed` after one.
/// `detail` never carries a token, a user key or a webhook URL.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct ChannelReport {
    channel: String,
    status: String,
    detail: String,
}

/// Where each provider is reached. Production uses the defaults; tests point these at a
/// local server so delivery is exercised without touching the providers.
#[derive(Debug, Clone)]
struct NotifyEndpoints {
    telegram_base: String,
    pushover_url: String,
}

/// One cycle's reading of the price-source range against spot (#89). Kept in memory
/// from one cycle to the next so a range exit can be pushed when the directive's action
/// does not change with it. Never stored and never part of a snapshot.
#[derive(Debug, Clone, PartialEq)]
struct RangeTopReading {
    position_id: String,
    label: String,
    spot: f64,
    top: f64,
    /// Spot is at or above the top of the range: the RANGE EXIT condition.
    above: bool,
}

impl Default for NotifyEndpoints {
    fn default() -> Self {
        Self {
            telegram_base: "https://api.telegram.org".to_string(),
            pushover_url: "https://api.pushover.net/1/messages.json".to_string(),
        }
    }
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
    /// Collectible LP fees: the same figure as `gross_usd`, under its own name (#142).
    collectible_lp_fees_usd: Option<String>,
    /// Collectible gauge rewards in USD; null when not totalled (the note says why). (#142)
    collectible_rewards_usd: Option<String>,
    collectible_rewards_note: Option<String>,
    /// LP fees plus gauge rewards; null whenever the rewards part is null. (#142)
    total_collectible_usd: Option<String>,
    /// One row per position with a gauge reward (#142). Information only: a row has no
    /// `to` and no `data`, so nothing here can be sent to a wallet. NockNock builds no
    /// transaction for a reward; it is collected in the Aero app.
    rewards: Vec<ClaimRewardRow>,
}

/// A position's gauge reward in the claim payload (#142): what is collectible, where,
/// and what collecting it would cost in gas when that call is verified for the gauge.
#[derive(Serialize, Debug, Clone, PartialEq)]
struct ClaimRewardRow {
    position_id: String,
    label: String,
    /// read | not_read
    status: String,
    token_symbol: String,
    amount: Option<String>,
    usd: Option<String>,
    gauge: Option<String>,
    collected_in: String,
    collect_url: String,
    /// Why the reward was not read; null when it was.
    note: Option<String>,
    /// Gas units of the verified claim call, hex, estimated from the wallet. Null unless
    /// the call is verified for this gauge and the estimate succeeded.
    gas_estimate: Option<String>,
    gas_usd: Option<String>,
    /// The gas statement in words, e.g. why there is no estimate.
    gas_note: String,
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
mod tests_aero_cl;
#[cfg(test)]
mod tests_aero_rewards;
#[cfg(test)]
mod tests_fills;
#[cfg(test)]
mod tests_quotes;
#[cfg(test)]
mod tests_cost_basis;
#[cfg(test)]
mod tests_notify;
#[cfg(test)]
mod tests_rewards;
