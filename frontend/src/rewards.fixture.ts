/**
 * Fixture for the collectible-rewards helpers (issue #142).
 *
 * The figures are the ones the deployed v0.8.0 worker published for the real wallet on
 * 2026-11-17 at 11:05:11Z, eight minutes before the operator collected the CL gauge's
 * reward (ONCHAIN_INTEGRATION.md §9): $0.54 + $1.41 of LP fees on the two Uniswap
 * positions, 0.210015 AERO in the basic gauge and 17.654015 AERO in the CL gauge, at
 * $0.438859 per AERO. `OLD_*` is that 2.6.0 snapshot as it was; the rest adds the fields
 * a #142 worker derives from the same reads.
 */

export const BASIC_GAUGE = '0xc82782ee66d82c4d19db0429652091a085083526';
export const CL_GAUGE = '0x3bf70c880adc8370917994ab0df68e763b7a60fd';
export const STAKED_REASON = 'not LP-claimable (staked: swap fees on staked liquidity go to the gauge)';
export const BASIC_REASON = 'not LP-claimable (claimFees() reverts)';

export function reward(amount: string | null, usd: string | null, gauge: string | null, note: string | null = null) {
  return {
    status: amount === null ? 'not_read' : 'read',
    token_symbol: 'AERO',
    token_address: '0x3f5bF3EdAc59d650D8fDf9657a71F2f904A3a4Ee',
    amount,
    usd,
    gauge,
    collected_in: 'the Aero app',
    collect_url: 'https://app.aero.xyz',
    note,
  };
}

export const OLD_GLOBAL = {
  total_portfolio_usd: '2857.78',
  total_fees_accrued_usd: '1.95',
  total_fees_claimable_usd: '1.95',
  gas_to_fee_delta_usd: '1.94',
  aero_price_usd: '0.438859',
  wallet_aero_amount: '0.000000',
  wallet_aero_usd: '0.00',
  unclaimed_aero_amount: '17.864030',
  unclaimed_aero_usd: '7.84',
  aero_note: null,
};

export const GLOBAL = {
  ...OLD_GLOBAL,
  collectible_rewards_usd: '7.84',
  collectible_rewards_note: null,
  total_collectible_usd: '9.79',
};

export const OLD_UNISWAP_V4 = {
  position_id: 'uniswap-v4:base:3163450',
  protocol: 'uniswap-v4',
  nft_id: 3163450,
  fee_tier_bps: 300,
  token0: { symbol: 'USDC' },
  token1: { symbol: 'NOCK' },
  amount0: '92.933109',
  amount1: '7629.972773',
  fees_usd: '0.542769',
};

export const OLD_BASIC = {
  position_id: 'aerodrome:base:0x770fe8fc9248c6382950a86e39a9391b41d59e0a',
  protocol: 'aerodrome',
  nft_id: 0,
  fee_tier_bps: 30,
  token0: { symbol: 'USDC' },
  token1: { symbol: 'NOCK' },
  amount0: '35.307478',
  amount1: '2495.901913',
  fees_usd: '0.000000',
  unclaimed_aero: '0.210015',
  unclaimed_aero_usd: '0.09',
};

export const OLD_CL = {
  position_id: 'aerodrome-cl:base:2914490',
  protocol: 'aerodrome-cl',
  nft_id: 2914490,
  fee_tier_bps: 97,
  tick_spacing: 200,
  token0: { symbol: 'WETH' },
  token1: { symbol: 'NOCK' },
  amount0: '0.099668',
  amount1: '0.000000',
  fees_usd: '0.000000',
  fees_note: STAKED_REASON,
  staked_in_gauge: CL_GAUGE,
  unclaimed_aero: '17.654015',
  unclaimed_aero_usd: '7.75',
};

export const BASIC = { ...OLD_BASIC, collectible_reward: reward('0.210015', '0.09', BASIC_GAUGE) };
export const CL = { ...OLD_CL, collectible_reward: reward('17.654015', '7.75', CL_GAUGE) };
export const POSITIONS = [OLD_UNISWAP_V4, BASIC, CL];

/** `velocity_gas_sandbox.contracts` as the 2.6.0 worker published them. */
export const OLD_CONTRACTS = [
  {
    contract_name: 'Uniswap V4 (USDC/NOCK 3.00%, #3163450)',
    claimable_usd: '0.54',
    gas_break_even_usd: '0.0036',
    efficiency_ratio: 150.3,
    recommended_action: 'claim_now',
  },
  {
    contract_name: 'Aero USDC/NOCK 0.30% (full range)',
    claimable_usd: '0.00',
    gas_break_even_usd: '0.0036',
    efficiency_ratio: 0,
    recommended_action: 'not_claimable',
    note: `${BASIC_REASON}; unclaimed AERO 0.210015 ($0.09)`,
  },
  {
    contract_name: 'Aero CL WETH/NOCK 0.97% #2914490',
    claimable_usd: '0.00',
    gas_break_even_usd: '0.0036',
    efficiency_ratio: 0,
    recommended_action: 'not_claimable',
    note: `${STAKED_REASON}; unclaimed AERO 17.654015 ($7.75)`,
  },
];

export const CONTRACTS = [
  OLD_CONTRACTS[0],
  { ...OLD_CONTRACTS[1], earnings_kind: 'gauge_reward', reward: reward('0.210015', '0.09', BASIC_GAUGE), lp_fees_note: BASIC_REASON },
  { ...OLD_CONTRACTS[2], earnings_kind: 'gauge_reward', reward: reward('17.654015', '7.75', CL_GAUGE), lp_fees_note: STAKED_REASON },
];

/** `contract_fees`, index-aligned with POSITIONS. */
export const CONTRACT_FEES = [
  { fee_token_symbol: 'USDC+NOCK (Uniswap V4 #3163450)', amount: '0.154609 USDC + 28.05907500 NOCK', amount_usd: '0.542769', claimable: true },
  {
    fee_token_symbol: 'USDC+NOCK (Aerodrome)',
    amount: `${BASIC_REASON}; unclaimed AERO 0.210015 ($0.09)`,
    amount_usd: '0.000000',
    claimable: false,
    earnings_kind: 'gauge_reward',
    reward: reward('0.210015', '0.09', BASIC_GAUGE),
    lp_fees_note: BASIC_REASON,
  },
  {
    fee_token_symbol: 'WETH+NOCK (Aero CL #2914490)',
    amount: `${STAKED_REASON}; unclaimed AERO 17.654015 ($7.75)`,
    amount_usd: '0.000000',
    claimable: false,
    earnings_kind: 'gauge_reward',
    reward: reward('17.654015', '7.75', CL_GAUGE),
    lp_fees_note: STAKED_REASON,
  },
];

/** The claim payload a #142 worker returns for that cycle. Gas figures: the real
 *  eth_estimateGas for the basic gauge (390,860), and for the CL gauge the gas the
 *  operator's own claim used (418,528), both at 0.009 gwei and ETH $1,790.24. */
export const PAYLOAD = {
  chain_id: 8453,
  from: '0xeE114cC97eC5adeB34bA5645e315B497D8487a45',
  generated_at: '2026-11-17T11:05:30+00:00',
  gross_usd: '1.95',
  estimated_gas_usd: '0.0041',
  net_after_gas_usd: '1.9459',
  actions: [
    {
      platform: 'aerodrome',
      label: 'Claim Aero LP fees (aero-basic-nock-usdc)',
      to: '0x770fe8fc9248c6382950a86e39a9391b41d59e0a',
      data: '0xd294f093',
      value: '0x0',
      simulation: 'not_claimable',
      simulation_detail: `${BASIC_REASON}; reward 0.210015 AERO ($0.09), collectible from gauge ${BASIC_GAUGE} in the Aero app`,
    },
    {
      platform: 'aerodrome-cl',
      label: 'Collect Aero CL LP fees (aero-cl200-weth-nock #2914490)',
      to: '0x4857db576ee0e45f10525ec59527bfd10beec60b',
      data: '0xfc6f7865',
      value: '0x0',
      simulation: 'not_claimable',
      simulation_detail: `${STAKED_REASON}; reward 17.654015 AERO ($7.75), collectible from gauge ${CL_GAUGE} in the Aero app`,
    },
  ],
  collectible_lp_fees_usd: '1.95',
  collectible_rewards_usd: '7.84',
  collectible_rewards_note: null,
  total_collectible_usd: '9.79',
  rewards: [
    {
      position_id: OLD_BASIC.position_id,
      label: 'USDC+NOCK (Aerodrome)',
      ...reward('0.210015', '0.09', BASIC_GAUGE),
      gas_estimate: '0x5f6cc',
      gas_usd: '0.0063',
      gas_note: 'gas est. 390860 units (~$0.0063)',
    },
    {
      position_id: OLD_CL.position_id,
      label: 'WETH+NOCK (Aero CL #2914490)',
      ...reward('17.654015', '7.75', CL_GAUGE),
      gas_estimate: '0x662e0',
      gas_usd: '0.0067',
      gas_note: 'gas est. 418528 units (~$0.0067)',
    },
  ],
};
