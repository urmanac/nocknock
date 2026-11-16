/**
 * Frontend TypeScript types — mirror of snapshot.yaml DerivedMetrics
 * Generated from openapi-typescript in production; manual for now.
 */

export interface SnapshotMeta {
  generated_at: string;
  state_hash: string;
  version: string;
  snapshot_id: string;
}

export interface SnapshotEnvelope<T = unknown> {
  meta: SnapshotMeta;
  data: T;
  /** Backfill telemetry (v0.3.0); computed live by the API, not part of state_hash */
  data_provenance?: DataProvenance;
}

export interface DataProvenance {
  last_backfill_at: string | null;
  backfilled_points_count: number;
  /** Backfilled point count per provider (e.g. geckoterminal, dexscreener) */
  sources?: Record<string, number>;
}

// ============================================================
// Derived Metrics — "Storyboard Visual Layout" pre-computed
// ============================================================

export interface DerivedMetrics {
  global_status: GlobalStatus;
  asset_aggregation_series: AssetAggregationSeries;
  total_value_vs_baseline: TotalValueVsBaseline;
  tick_bracket_hypervisor: TickBracketHypervisor;
  portfolio_deviation_matrix: PortfolioDeviationMatrix;
  structural_channels: StructuralChannels;
  strategy_counterfactual: StrategyCounterfactual;
  velocity_gas_sandbox: VelocityGasSandbox;
  /** Dollar-cost-averaging plan status (2.1.0); null/absent when no plan is configured */
  accumulation_plan?: AccumulationPlanStatus | null;
  /** Standing LP-rebuild bias (2.1.0) */
  lp_rebuild?: LpRebuild;
}

/** Plan progress, derived from config + receipt-verified buys only (2.1.0) */
export interface AccumulationPlanStatus {
  plan_id: string;
  status: 'pending' | 'tranche_due' | 'waiting' | 'paused' | 'blocked' | 'complete' | 'invalid';
  venue: string;
  /** 1-based index of the next tranche (verified fills + 1) */
  tranche_index: number;
  tranches_total: number;
  /** Advised now, after guardrails ("0.00" unless a tranche is due) */
  tranche_usdc: string;
  nominal_tranche_usdc: string;
  next_due_at?: string | null;
  budget_usdc: string;
  spent_usdc: string;
  remaining_usdc: string;
  acquired_nock: string;
  fills: number;
  last_fill_at?: string | null;
  avg_price_usd?: string | null;
  /** NOCK the whole budget would have bought at the first fill's price */
  lump_sum_nock?: string | null;
  binding_constraint?: string | null;
  /** Protected cash reserve the plan never draws below */
  cash_reserve_usdc?: string | null;
}

/** LP weight vs target; rebuilding is a reposition with no net cash change (2.1.0) */
export interface LpRebuild {
  status: 'under_target' | 'on_target' | 'over_target' | 'no_positions' | 'no_target';
  target_usd: string;
  actual_usd: string;
  gap_usd: string;
  target_weight_bps: number;
  /** Concrete deposits that would close the gap (2.2.0) */
  suggestions?: LpSuggestion[];
  /** Tracked positions deliberately left out, with the reason (2.2.0) */
  excluded?: Array<{ position_id: string; nft_id: number; label: string; reason: string }>;
  /** NOCK tolerated below the floor in a full LP sell-out */
  floor_tolerance_nock?: string | null;
  /** Total NOCK left if every range sold all its NOCK */
  worst_case_nock?: string | null;
}

/** A suggested deposit into an existing tracked position (2.2.0) */
export interface LpSuggestion {
  position_id: string;
  nft_id: number;
  protocol: string;
  label: string;
  target_usd: string;
  current_usd: string;
  add_usd: string;
  token0_symbol: string;
  amount0: string;
  token1_symbol: string;
  amount1: string;
  limited_by?: string | null;
  /** Price multiple at which this range has sold all its NOCK (e.g. "1.44") */
  nock_sold_out_at_multiple?: string | null;
  /** The position's page in the Uniswap app */
  url: string;
}

export interface GlobalStatus {
  total_portfolio_usd: string;
  total_native_usd: string;
  total_stable_usd: string;
  total_fees_accrued_usd: string;
  total_fees_claimable_usd: string;
  gas_to_fee_delta_usd: string;
  high_water_mark_usd: string;
  high_water_mark_date: string;
  total_nock_amount?: string;
  nock_floor_delta?: string;
  nock_floor_target?: string;
  total_usdc_amount?: string;
  shot_advice_action?: string;
  shot_advice_details?: string;
  /** Market regime from daily RSI(14) overextension detection (v0.3.0) */
  regime?: 'normal' | 'overextended' | 'unknown';
  /** Wilder RSI(14) over 5-minute snapshot closes (short-term momentum) */
  rsi_14?: string | null;
  /** Wilder RSI(14) over daily closes — the "daily RSI"; drives `regime` */
  rsi_14_1d?: string | null;
  /** Live ETH/USD used to value gas and LP WETH (2.3.0); null when unpriced */
  eth_price_usd?: string | null;
  /** Wallet ETH + WETH in ETH: gas (2.3.0). Counted in the total; not cash, not deployable, no target */
  wallet_gas_eth?: string | null;
  /** wallet_gas_eth in USD (2.3.0) */
  wallet_gas_usd?: string | null;
  /** The ETH pillar in ETH: wallet gas + ETH/WETH legs of tracked LP (2.3.0) */
  total_eth_amount?: string | null;
  /** The ETH pillar in USD (2.3.0). native + stable + eth = total */
  total_eth_usd?: string | null;
}

export interface AssetAggregationSeries {
  time_points: string[];
  /** Wallet NOCK valued in USD */
  safe_haven_series: string[];
  /** Wallet NOCK in token units (v0.5.0); absent on older snapshots */
  wallet_nock_series?: string[];
  active_protocol_series: string[];
  /** NOCK units per tracked position (v2.0.0; replaces the retired tranche_1/2/3_nock series) */
  positions_nock?: Array<{ position_id: string; series: string[] }>;
}

export interface TotalValueVsBaseline {
  time_points: string[];
  portfolio_value_series: string[];
  token_price_series: string[];
  high_water_mark_series: string[];
}

export interface TickBracketHypervisor {
  current_price: string;
  brackets: TickBracket[];
}

export interface TickBracket {
  lower_price: string;
  upper_price: string;
  liquidity_usd: string;
  in_range: boolean;
  /** Null when the bracket's denomination differs from the USD/NOCK spot (ETH-pools). */
  distance_bps: number | null;
}

export interface PortfolioDeviationMatrix {
  allocations: AllocationDeviation[];
}

export interface AllocationDeviation {
  bucket_name: string;
  target_weight_bps: number;
  actual_weight_bps: number;
  deviation_bps: number;
  status: 'on_target' | 'over_funded' | 'under_funded' | 'fast_running' | 'no_target';
}

export interface StructuralChannels {
  channels: RegressionChannel[];
}

export interface RegressionChannel {
  start_time: string;
  end_time: string | null;
  center_line: LinePoint[];
  upper_1sd: LinePoint[];
  lower_1sd: LinePoint[];
  upper_2sd: LinePoint[];
  lower_2sd: LinePoint[];
  is_active: boolean;
  break_point: string | null;
}

export interface LinePoint {
  time: string;
  value: string;
}

export interface StrategyCounterfactual {
  time_points: string[];
  active_portfolio_series: string[];
  passive_baseline_series: string[];
}

export interface VelocityGasSandbox {
  contracts: FeeCollectionIndex[];
}

export interface FeeCollectionIndex {
  contract_address: string;
  contract_name: string;
  claimable_usd: string;
  gas_break_even_usd: string;
  efficiency_ratio: number;
  /** What the worker emits; 'not_claimable' is an Aero pool whose claimFees() reverts,
   *  or (v0.6.0) an Aero CL position staked in its gauge (see note). */
  recommended_action: 'claim_now' | 'claim_recommended' | 'accumulate' | 'not_claimable';
  /** With 'not_claimable': the position's own reason, when it has one (v0.6.0). */
  note?: string;
}

// ============================================================
// Raw snapshot data (for debugging / raw views)
// ============================================================

export interface WalletBalance {
  wallet_address: string;
  token_symbol: string;
  token_address: string;
  amount: string;
  amount_usd: string;
  updated_at: string;
}

export interface PoolMechanic {
  pool_id: string;
  platform: 'Aerodrome' | 'UniswapV4' | 'UniswapV3' | 'Velodrome' | 'Other';
  pool_type: 'volatile' | 'stable' | 'concentrated';
  /** The tracked wallet's amounts in this pool (not pool-wide reserves) */
  volatile_assets: PoolAsset[];
  liquidity_usd: string;
  fee_accrued_usd: string;
  fee_tier_bps: number;
  tick_current?: number;
  sqrt_price_x96?: string;
}

export interface PoolAsset {
  token_address: string;
  token_symbol: string;
  reserve_amount: string;
  reserve_usd: string;
}

export interface ConcentratedLiquidity {
  range_id: string;
  pool_id: string;
  lower_tick: number;
  upper_tick: number;
  lower_price: string;
  upper_price: string;
  liquidity_amount: string;
  tokens_owed_0: string;
  tokens_owed_1: string;
  in_range: boolean;
  fee_growth_inside_0_x128: string;
  fee_growth_inside_1_x128: string;
}

export interface ContractFee {
  contract_address: string;
  fee_token_address: string;
  fee_token_symbol: string;
  amount: string;
  amount_usd: string;
  claimable: boolean;
  gas_break_even_usd: string;
}

export interface PositionToken {
  address: string;
  symbol: string;
  decimals: number;
}

/** One tracked LP position (v2.0.0 — replaces the two-tranche model) */
export interface Position {
  position_id: string;
  network: string;
  wallet: string;
  /** 'aerodrome-cl' (v0.6.0): an Aero concentrated-liquidity NFT, staked in its gauge or not. */
  protocol: 'uniswap-v4' | 'uniswap-v3' | 'aerodrome' | 'aerodrome-cl';
  nft_id: number;
  pool_id: string;
  npm_address?: string | null;
  fee_tier_bps: number;
  token0: PositionToken;
  token1: PositionToken;
  amount0: string;
  amount1: string;
  amount_usd: string;
  tokens_owed_0: string;
  tokens_owed_1: string;
  fees_usd: string;
  lower_tick: number;
  upper_tick: number;
  lower_price: string;
  upper_price: string;
  liquidity_amount: string;
  in_range: boolean;
  is_price_source: boolean;
  /** False when the pool has no in-range liquidity (frozen price); null/absent = unknown (2.2.0) */
  pool_active?: boolean | null;
  /** True for a position spanning every price (Aerodrome volatile pool); bounds are not meaningful (2.2.0) */
  full_range?: boolean;
  /** aerodrome-cl only: the pool's tick spacing, 200 for a CL200 pool (v0.6.0) */
  tick_spacing?: number;
  /** aerodrome-cl only: the gauge holding the NFT, when this wallet staked it there (v0.6.0) */
  staked_in_gauge?: string;
  /** Present when tokens_owed_* and fees_usd are not a read of LP-claimable fees: the
   *  reason to show in their place, never "$0 earned" (v0.6.0) */
  fees_note?: string;
}

export interface RawSnapshotData {
  wallet_balances: WalletBalance[];
  positions: Position[];
  pool_mechanics: PoolMechanic[];
  /** DEPRECATED since v2.0.0 — superseded by positions; worker emits an empty array */
  concentrated_liquidity: ConcentratedLiquidity[];
  contract_fees: ContractFee[];
  derived_metrics: DerivedMetrics;
}