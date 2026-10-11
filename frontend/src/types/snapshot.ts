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
  total_usdc_amount?: string;
  shot_advice_action?: string;
  shot_advice_details?: string;
}

export interface AssetAggregationSeries {
  time_points: string[];
  safe_haven_series: string[];
  active_protocol_series: string[];
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
  distance_bps: number;
}

export interface PortfolioDeviationMatrix {
  allocations: AllocationDeviation[];
}

export interface AllocationDeviation {
  bucket_name: string;
  target_weight_bps: number;
  actual_weight_bps: number;
  deviation_bps: number;
  status: 'on_target' | 'over_funded' | 'under_funded' | 'fast_running';
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
  recommended_action: 'claim_now' | 'wait' | 'not_worthwhile';
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
  platform: 'Aerodrome' | 'UniswapV4' | 'Velodrome' | 'Other';
  pool_type: 'volatile' | 'stable' | 'concentrated';
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

export interface RawSnapshotData {
  wallet_balances: WalletBalance[];
  pool_mechanics: PoolMechanic[];
  concentrated_liquidity: ConcentratedLiquidity[];
  contract_fees: ContractFee[];
  derived_metrics: DerivedMetrics;
}