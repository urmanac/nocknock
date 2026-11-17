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
  /** NOCK cost basis (v0.6.0, #98); null/absent from a worker that predates it */
  cost_basis?: import('../costBasis').CostBasis | null;
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
  /**
   * Swaps found on-chain that look like plan buys, and whether the check ran (v0.6.0).
   * Offers are not progress: fills, spend and cadence count recorded buys only.
   * Null or absent in snapshots from before v0.6.0.
   */
  fill_detection?: FillDetection | null;
  /**
   * Read-only quotes for this exact tranche on Aero and on Uniswap (v0.7.0, issue #115).
   * Present only while a tranche is due and quoting is on; null or absent otherwise and
   * in older snapshots. Not part of state_hash.
   */
  route_quotes?: RouteQuotes | null;
  /**
   * True when the worker could not read the accumulation ledger this cycle (2.8.1,
   * issue #130). spent_usdc, remaining_usdc and acquired_nock then read "unknown", and
   * fills and tranche_index are not progress. Absent when the ledger was read.
   */
  progress_unknown?: boolean;
}

/**
 * Quotes for one due tranche, both venues read at the same block (v0.7.0, issue #115).
 * A comparison of execution routes for a buy the plan already sized: not a
 * recommendation, and never an estimate. A venue that could not be quoted is
 * `unavailable` with its reason.
 */
export interface RouteQuotes {
  /** ok (both venues quoted) | partial (one) | unavailable (none) */
  status: 'ok' | 'partial' | 'unavailable';
  detail?: string | null;
  /** The plan's own tranche amount that was quoted, in USDC */
  amount_usdc: string;
  /** Block every quote was read at, and that block's time; null when nothing was read */
  block_number?: number | null;
  quoted_at?: string | null;
  /** The snapshot's NOCK/USD reference spot the costs are measured against */
  reference_spot_usd?: string | null;
  venues: VenueQuote[];
  /** Null unless both venues were quoted and their amounts differ */
  more_nock_venue?: string | null;
  more_nock_by_pct?: string | null;
  /** The comparison as one sentence of fact, written by the worker */
  comparison: string;
}

/** One venue's best verified route, or why it could not be quoted (v0.7.0) */
export interface VenueQuote {
  venue: string;
  status: 'quoted' | 'unavailable';
  reason?: string | null;
  /** The venue's own swap page; nothing is prefilled */
  swap_url: string;
  nock_out?: string | null;
  effective_price_usd?: string | null;
  /** (effective price / reference spot − 1) × 100; null without a reference spot */
  cost_over_spot_pct?: string | null;
  route?: string | null;
  hops?: RouteHop[];
  routes_tried?: RouteAttempt[];
}

export interface RouteHop {
  /** Pool address, or the pool id for a Uniswap v4 pool */
  pool: string;
  pool_kind: string;
  token_in: string;
  token_out: string;
  /** Swap fee in percent; null when the fee read failed */
  fee_pct?: string | null;
}

export interface RouteAttempt {
  route_id: string;
  route: string;
  nock_out?: string | null;
  reason?: string | null;
}

/** One cycle's fill-detection result (v0.6.0, issue #118) */
export interface FillDetection {
  /** `unavailable` is "could not check", not "checked, nothing found" (`ok`, no offers) */
  status: 'ok' | 'partial' | 'unavailable' | 'not_run';
  /** Plain-words reason for partial, unavailable and not_run */
  detail?: string | null;
  /** How far back the transfer history was read (the plan anchor after a complete check) */
  checked_back_to?: string | null;
  /** Time of the last successful check; not part of state_hash */
  last_checked_at?: string | null;
  /** Detected swaps waiting for an answer: confirm, or mark as not part of the plan */
  offers: FillOffer[];
  /** Detected swaps marked "not part of the plan"; kept and restorable */
  dismissed: FillOffer[];
}

/** A detected USDC→NOCK swap, read from its receipt and block header */
export interface FillOffer {
  tx_hash: string;
  /** Block time of the swap */
  confirmed_at: string;
  block_number: number;
  usdc_spent: string;
  nock_received: string;
  effective_price_usd: string;
  /** early: less than one plan interval after the buy before it (a recorded fill or an older open offer) */
  timing: 'early' | 'on_schedule';
  /** Stored NOCK/USD price nearest the swap's block time; null when none was near */
  reference_spot_usd?: string | null;
  reference_spot_source?: string | null;
  /** (effective price / reference spot − 1) × 10,000; null without a reference */
  execution_cost_bps?: number | null;
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
  /** The ETH pillar in USD (2.3.0). native + stable + eth + wallet AERO = total */
  total_eth_usd?: string | null;
  /** AERO/USD from the AERO price pool (v0.6.0, #93); null when it could not be read */
  aero_price_usd?: string | null;
  /** Wallet AERO, the fourth pillar (v0.6.0, #93). Counted in the total; not cash, not
   *  deployable, no target, and no advice is given about it */
  wallet_aero_amount?: string | null;
  /** wallet_aero_amount in USD: the AERO pillar (v0.6.0, #93) */
  wallet_aero_usd?: string | null;
  /** AERO earned in the gauges of tracked staked positions and not claimed (v0.6.0, #93).
   *  NOT in total_portfolio_usd. Null when any Aero position's gauge was not read (a
   *  failed read, a gauge that pays another token, or no gauge_address): never a partial
   *  sum, never 0 for something not read */
  unclaimed_aero_amount?: string | null;
  unclaimed_aero_usd?: string | null;
  /** The worker's stated reason whenever an AERO figure above is null (v0.6.0, #93) */
  aero_note?: string | null;
  /** Collectible gauge rewards of the tracked staked positions in USD (#142): the gauges'
   *  summed amount at the cycle's AERO price, the same figure as unclaimed_aero_usd (it
   *  can differ by a cent from the sum of the rounded per-position values). Null when a
   *  gauge was not read or a reward is unpriced (collectible_rewards_note says which):
   *  never a partial sum. NOT in total_portfolio_usd; no directive reads it. Absent (or
   *  null with a null note) on an older snapshot: clients then add no reward line */
  collectible_rewards_usd?: string | null;
  /** Why collectible_rewards_usd and total_collectible_usd are null (#142) */
  collectible_rewards_note?: string | null;
  /** total_fees_claimable_usd (LP fees only) plus collectible_rewards_usd (#142). Null
   *  whenever the rewards part is null. NOT in total_portfolio_usd */
  total_collectible_usd?: string | null;
  /** How many configured positions are closed: the length of data.closed_positions.
   *  Absent when none is (2.8.0, #144) */
  closed_positions_count?: number;
  /** One statement naming the closed positions, that they are in no total and that
   *  their config entries can be removed. Absent when none is closed (2.8.0, #144) */
  closed_positions_note?: string;
}

/** A gauge reward stated as collectible earnings (#142): what is there and where it is
 *  collected. Never advice, and not counted in the portfolio total. */
export interface CollectibleReward {
  /** 'not_read' has no amount and no USD value; note says why */
  status: 'read' | 'not_read';
  /** 'AERO' */
  token_symbol: string;
  token_address: string;
  /** Reward earned and not collected, 6 decimals; null when not read (never 0) */
  amount?: string | null;
  /** amount at the cycle's AERO/USD price; null when not read or unpriced */
  usd?: string | null;
  /** The gauge the reward is collected from; null when none is configured */
  gauge?: string | null;
  /** 'the Aero app' */
  collected_in: string;
  /** Always on app.aero.xyz */
  collect_url: string;
  note?: string | null;
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
  /** With 'not_claimable': the position's own reason, when it has one (v0.6.0),
   *  followed by its unclaimed AERO or why that was not read (#93). A claimable row
   *  carries no AERO statement: its action is about LP fees only. */
  note?: string;
  /** 'gauge_reward' when the row's reported earnings are the position's gauge reward,
   *  drawn as its own series (#142). Only ever beside 'not_claimable'; absent for a row
   *  whose earnings are LP fees. Never one of the harvest words */
  earnings_kind?: 'gauge_reward';
  /** The gauge reward the row reports as the position's earnings (#142) */
  reward?: CollectibleReward;
  /** With 'not_claimable': the reason alone, without the AERO statement (#142) */
  lp_fees_note?: string;
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
  /** 'gauge_reward' when the position's earnings are its gauge reward (#142) */
  earnings_kind?: 'gauge_reward';
  /** The position's gauge reward, when it has one (#142) */
  reward?: CollectibleReward;
  /** The reason alone why the LP fees are not LP-claimable (#142) */
  lp_fees_note?: string;
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
  /** AERO earned in this position's gauge and not claimed (v0.6.0, #93). Present only
   *  when the gauge was read. Not in amount_usd, fees_usd or the portfolio total */
  unclaimed_aero?: string;
  /** unclaimed_aero in USD; absent when AERO is unpriced (v0.6.0, #93) */
  unclaimed_aero_usd?: string;
  /** Why unclaimed_aero is absent for an Aero position that could earn AERO (v0.6.0, #93) */
  unclaimed_aero_note?: string;
  /** The position's gauge reward as collectible earnings: the same read as
   *  unclaimed_aero*, structured (#142). Present for every Aero position that can earn
   *  a reward, also when the gauge was not read (status 'not_read'). Absent otherwise
   *  and on an older snapshot */
  collectible_reward?: CollectibleReward;
}

/** One read that proved a position closed (2.8.0, #144). */
export interface ClosedEvidence {
  /** The call's signature, e.g. 'ownerOf(uint256)' */
  call: string;
  contract: string;
  outcome: 'reverted' | 'returned';
  /** The answer in words, e.g. 'Error("ERC721: owner query for nonexistent token")' */
  answer: string;
  /** Raw revert data or returned data, hex with 0x */
  raw: string;
}

/** A configured position whose NFT does not exist (2.8.0, #144): withdrawn and burned,
 *  or a mistyped nft_id. It carries no amount and no value: nothing was read from it,
 *  and its last known value is not a current one. */
export interface ClosedPosition {
  /** The entry's `id` in config.yaml, which the operator can remove */
  config_id: string;
  position_id: string;
  protocol: string;
  network: string;
  nft_id: number;
  pool_id?: string | null;
  position_manager: string;
  /** Built from the config entry alone: venue, pair when config states it, NFT id, config id */
  label: string;
  status: 'closed';
  reason: string;
  evidence: ClosedEvidence[];
  /** When this worker first recorded it as closed; not the time of the burn */
  first_seen_closed_at?: string | null;
  /** True when the entry's pool still supplies the NOCK/USD spot (Uniswap v4 only):
   *  another position must be flagged is_price_source before the entry is removed */
  supplies_spot_price?: boolean;
}

export interface RawSnapshotData {
  wallet_balances: WalletBalance[];
  positions: Position[];
  pool_mechanics: PoolMechanic[];
  /** DEPRECATED since v2.0.0 — superseded by positions; worker emits an empty array */
  concentrated_liquidity: ConcentratedLiquidity[];
  contract_fees: ContractFee[];
  /** Configured positions whose NFT provably no longer exists (2.8.0, #144). Never in
   *  `positions`; nothing of them is in any total. Absent when there are none and on
   *  every older snapshot */
  closed_positions?: ClosedPosition[];
  derived_metrics: DerivedMetrics;
}