/**
 * Tracked ladders (#167) as the worker states them, copied from
 * ingestion-worker/testdata/golden_snapshot_ladder.json (ladderTrack.test.ts checks the
 * copy against the file when the worker directory is present). The near rung is a real
 * Uniswap v4 mint (NFT #3240369, tx 0xafe42fe2…, block 59660587); the deep rung is
 * synthetic and has no deposit_tx.
 */
import type { LadderReading, Position } from './types/snapshot';

/** ladders_above_range: data.ladders */
export const LADDERS_ABOVE: LadderReading[] = [
  {
    "config_id": "ladder-near",
    "fees_usd": "0.00",
    "high_pct_of_spot": "-16.76",
    "high_price_usd": "0.01720088",
    "label": "v4 USDC/NOCK 3.00% #3240369",
    "low_pct_of_spot": null,
    "low_price_usd": null,
    "nock_now": "0.0000",
    "open_end": "the bottom reaches the pool's lowest usable price (tick 887100): the range buys all the way down, so no bottom price and no NOCK at the bottom are stated",
    "pool_price_usd": "0.02066445",
    "position_id": "uniswap-v4:base:3240369",
    "rung": "near",
    "rung_note": null,
    "since_deposit": {
      "avg_buy_price_usd": null,
      "current_liquidity": "934864722135742",
      "deposit_block": 59660587,
      "deposit_liquidity": "934864722135742",
      "deposit_tx": "0xafe42fe29de67337d5b489226dbfdf8a8b315e2deabaf6c744b47c2dd9ff7a58",
      "deposited_at": "2026-11-19T21:08:13+00:00",
      "deposited_nock": "247.5790",
      "deposited_usdc": "1221.85",
      "net_nock_bought": "-247.5790",
      "net_usdc_spent": "-4.24",
      "reason": null,
      "status": "verified"
    },
    "spot_price_usd": "0.02066445",
    "state": "holds_only_usdc",
    "state_text": "holds only USDC: the price is above the range, so it converts nothing until NOCK falls into it at $0.01720088",
    "usdc_now": "1226.09",
    "value_usd": "1226.09",
    "vs_top": {
      "converted_pct": "0.00",
      "curve_avg_price_usd": null,
      "nock_at_bottom": null,
      "nock_held_vs_top": "0.0000",
      "usdc_at_top": "1226.09",
      "usdc_converted_vs_top": "0.00"
    }
  },
  {
    "config_id": "ladder-deep",
    "fees_usd": "0.00",
    "high_pct_of_spot": "-50.01",
    "high_price_usd": "0.01032932",
    "label": "v4 USDC/NOCK 3.00% #3299001",
    "low_pct_of_spot": "-67.16",
    "low_price_usd": "0.00678699",
    "nock_now": "0.0000",
    "open_end": null,
    "pool_price_usd": "0.02066445",
    "position_id": "uniswap-v4:base:3299001",
    "rung": null,
    "rung_note": "rung is not set in config.yaml (near or deep): NockNock assumes neither",
    "since_deposit": {
      "avg_buy_price_usd": null,
      "current_liquidity": "400000000000000",
      "deposit_block": null,
      "deposit_liquidity": null,
      "deposit_tx": null,
      "deposited_at": null,
      "deposited_nock": null,
      "deposited_usdc": null,
      "net_nock_bought": null,
      "net_usdc_spent": null,
      "reason": "no deposit_tx is set for this ladder in config.yaml: NockNock has no baseline for what it bought and does not invent one",
      "status": "no_baseline"
    },
    "spot_price_usd": "0.02066445",
    "state": "holds_only_usdc",
    "state_text": "holds only USDC: the price is above the range, so it converts nothing until NOCK falls into it at $0.01032932",
    "usdc_now": "77.00",
    "value_usd": "77.00",
    "vs_top": {
      "converted_pct": "0.00",
      "curve_avg_price_usd": null,
      "nock_at_bottom": "9196.4045",
      "nock_held_vs_top": "0.0000",
      "usdc_at_top": "77.00",
      "usdc_converted_vs_top": "0.00"
    }
  }
];

/** ladders_above_range: data.positions */
export const POSITIONS_ABOVE: Position[] = [
  {
    "amount0": "129.038249",
    "amount1": "3748.278215",
    "amount_usd": "206.49",
    "fee_tier_bps": 300,
    "fees_usd": "0.000000",
    "full_range": false,
    "in_range": true,
    "is_price_source": true,
    "liquidity_amount": "185454734478733",
    "lower_price": "0.00550148",
    "lower_tick": 262200,
    "network": "base",
    "nft_id": 3163450,
    "pool_active": true,
    "pool_id": "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13",
    "position_id": "uniswap-v4:base:3163450",
    "protocol": "uniswap-v4",
    "tick_spacing": 300,
    "token0": {
      "address": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
      "decimals": 6,
      "symbol": "USDC"
    },
    "token1": {
      "address": "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722",
      "decimals": 16,
      "symbol": "NOCK"
    },
    "tokens_owed_0": "0.000000",
    "tokens_owed_1": "0.000000",
    "upper_price": "0.04106",
    "upper_tick": 282300,
    "wallet": "0xee114cc97ec5adeb34ba5645e315b497d8487a45"
  },
  {
    "amount0": "0.007567",
    "amount1": "7.288445",
    "amount_usd": "13.77",
    "fee_tier_bps": 100,
    "fees_usd": "0.000000",
    "full_range": false,
    "in_range": true,
    "is_price_source": false,
    "liquidity_amount": "1305339058220032430",
    "lower_price": "0.00000564468",
    "lower_tick": 70400,
    "network": "base",
    "nft_id": 6336662,
    "npm_address": "0x03a520b32c04bf3beef7beb72e919cf822ed34f1",
    "pool_active": true,
    "pool_id": "0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a",
    "position_id": "uniswap-v3:base:6336662",
    "protocol": "uniswap-v3",
    "token0": {
      "address": "0x4200000000000000000000000000000000000006",
      "decimals": 18,
      "symbol": "WETH"
    },
    "token1": {
      "address": "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722",
      "decimals": 16,
      "symbol": "NOCK"
    },
    "tokens_owed_0": "0.000000",
    "tokens_owed_1": "0.000000",
    "upper_price": "0.00000876435",
    "upper_tick": 74800,
    "wallet": "0xee114cc97ec5adeb34ba5645e315b497d8487a45"
  },
  {
    "amount0": "1226.094998",
    "amount1": "0.000000",
    "amount_usd": "1226.09",
    "fee_tier_bps": 300,
    "fees_usd": "0.000000",
    "full_range": false,
    "in_range": false,
    "is_price_source": false,
    "liquidity_amount": "934864722135742",
    "lower_price": "0.0000000000000000000000000000298994",
    "lower_tick": 270900,
    "network": "base",
    "nft_id": 3240369,
    "pool_active": true,
    "pool_id": "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13",
    "position_id": "uniswap-v4:base:3240369",
    "protocol": "uniswap-v4",
    "role": "ladder",
    "rung": "near",
    "tick_spacing": 300,
    "token0": {
      "address": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
      "decimals": 6,
      "symbol": "USDC"
    },
    "token1": {
      "address": "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722",
      "decimals": 16,
      "symbol": "NOCK"
    },
    "tokens_owed_0": "0.000000",
    "tokens_owed_1": "0.000000",
    "upper_price": "0.01720",
    "upper_tick": 887100,
    "wallet": "0xee114cc97ec5adeb34ba5645e315b497d8487a45"
  },
  {
    "amount0": "77.000291",
    "amount1": "0.000000",
    "amount_usd": "77.00",
    "fee_tier_bps": 300,
    "fees_usd": "0.000000",
    "full_range": false,
    "in_range": false,
    "is_price_source": false,
    "liquidity_amount": "400000000000000",
    "lower_price": "0.00678699",
    "lower_tick": 276000,
    "network": "base",
    "nft_id": 3299001,
    "pool_active": true,
    "pool_id": "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13",
    "position_id": "uniswap-v4:base:3299001",
    "protocol": "uniswap-v4",
    "role": "ladder",
    "tick_spacing": 300,
    "token0": {
      "address": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
      "decimals": 6,
      "symbol": "USDC"
    },
    "token1": {
      "address": "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722",
      "decimals": 16,
      "symbol": "NOCK"
    },
    "tokens_owed_0": "0.000000",
    "tokens_owed_1": "0.000000",
    "upper_price": "0.01033",
    "upper_tick": 280200,
    "wallet": "0xee114cc97ec5adeb34ba5645e315b497d8487a45"
  }
] as Position[];

/** ladders_above_range: the reserve fields of global_status */
export const RESERVE_ABOVE = {
  "cash_reserve_usdc": "950.00",
  "reserve_stable_usdc": "2485.13",
  "reserve_headroom_usdc": "1535.13",
  "reserve_ladder_usdc": "1303.10",
  "dry_powder_usdc": "800.09"
};

/** ladders_above_range: derived_metrics.cost_basis */
export const COST_BASIS_ABOVE = {
  "plan_buys": {
    "acquired_nock": null,
    "avg_cost_usd": null,
    "excludes": [
      "NOCK held before the first recorded buy, and any buy that was not recorded.",
      "Everything that is not a buy: sells, NOCK that LP ranges sell or buy back, fees earned.",
      "Gas paid in ETH."
    ],
    "fills": 0,
    "first_fill_at": null,
    "includes": [
      "Every receipt-verified buy in the accumulation ledger, across all plans: the USDC that left the wallet and the NOCK that entered it in each recorded transaction. Swap fees and price impact are inside those amounts."
    ],
    "last_fill_at": null,
    "plans": [],
    "reason": null,
    "spent_usdc": null,
    "status": "no_fills",
    "unrealized_pct": null,
    "unrealized_usd": null,
    "value_at_spot_usd": null
  },
  "spot_price_usd": "0.020664",
  "whole_stack": {
    "as_of": "2026-11-19T00:00:00+00:00",
    "avg_cost_usd": "0.016929",
    "basis_cost_usd": "1525.05",
    "basis_nock": "90086.7900",
    "bought_since_nock": "0.0000",
    "bought_since_usdc": "0.00",
    "excludes": [
      "Buys confirmed at or before 2026-11-19T00:00:00+00:00: they are taken to be inside the opening figures and are not added again.",
      "Everything after 2026-11-19T00:00:00+00:00 that is not a recorded buy: sells, NOCK that LP ranges sell or buy back, fees earned, transfers in or out, unrecorded buys.",
      "Realized gains and losses, and gas paid in ETH.",
      "Tracked ladders left out: uniswap-v4:base:3240369: no net buy since its deposit (-247.5790 NOCK for $-4.24); uniswap-v4:base:3299001: no verified deposit baseline (no_baseline)."
    ],
    "fills_since": 0,
    "held_nock": "119503",
    "includes": [
      "The opening figures entered in config: 90086.7900 NOCK for $1525.05 as of 2026-11-19T00:00:00+00:00, source: test figures. NockNock did not read or check them.",
      "Every receipt-verified buy confirmed after 2026-11-19T00:00:00+00:00: 0 so far.",
      "The net NOCK bought by 0 of 2 tracked ladders since a receipt-verified deposit after 2026-11-19T00:00:00+00:00: 0.0000 NOCK for $0.00, measured from each deposit_tx receipt to now."
    ],
    "ladder_buys_since": {
      "ladders_counted": 0,
      "nock": "0.0000",
      "not_counted": [
        "uniswap-v4:base:3240369: no net buy since its deposit (-247.5790 NOCK for $-4.24)",
        "uniswap-v4:base:3299001: no verified deposit baseline (no_baseline)"
      ],
      "usdc": "0.00"
    },
    "opening_avg_cost_usd": "0.016929",
    "opening_cost_usd": "1525.05",
    "opening_nock": "90086.7900",
    "reason": null,
    "source": "test figures",
    "status": "ok",
    "unrealized_pct": "22.07",
    "unrealized_usd": "336.54",
    "value_at_spot_usd": "1861.59"
  }
};

/** ladders_above_range: the deviation rows */
export const DEVIATION_ABOVE = [
  {
    "actual_weight_bps": 4728,
    "bucket_name": "Sovereign Treasury",
    "deviation_bps": -3272,
    "status": "under_funded",
    "target_weight_bps": 8000
  },
  {
    "actual_weight_bps": 435,
    "bucket_name": "NOCK LP (NOCK-pair positions)",
    "deviation_bps": -1065,
    "status": "under_funded",
    "target_weight_bps": 1500
  },
  {
    "actual_weight_bps": 2081,
    "bucket_name": "Liquid USDC (Escrow/Vault)",
    "deviation_bps": 1581,
    "status": "over_funded",
    "target_weight_bps": 500
  },
  {
    "actual_weight_bps": 177,
    "bucket_name": "Gas (wallet ETH+WETH)",
    "deviation_bps": 177,
    "status": "no_target",
    "target_weight_bps": 0
  },
  {
    "actual_weight_bps": 0,
    "bucket_name": "AERO (wallet)",
    "deviation_bps": 0,
    "status": "no_target",
    "target_weight_bps": 0
  },
  {
    "actual_weight_bps": 2576,
    "bucket_name": "NOCK ladders (measured)",
    "deviation_bps": 2576,
    "status": "no_target",
    "target_weight_bps": 0
  }
];

/** near_rung_bought_plan_due: data.ladders */
export const LADDERS_BOUGHT: LadderReading[] = [
  {
    "config_id": "ladder-near",
    "fees_usd": "0.00",
    "high_pct_of_spot": "+16.18",
    "high_price_usd": "0.01720088",
    "label": "v4 USDC/NOCK 3.00% #3240369",
    "low_pct_of_spot": null,
    "low_price_usd": null,
    "nock_now": "5551.3674",
    "open_end": "the bottom reaches the pool's lowest usable price (tick 887100): the range buys all the way down, so no bottom price and no NOCK at the bottom are stated",
    "pool_price_usd": "0.01480505",
    "position_id": "uniswap-v4:base:3240369",
    "rung": "near",
    "rung_note": null,
    "since_deposit": {
      "avg_buy_price_usd": "0.01590283",
      "current_liquidity": "934864722135742",
      "deposit_block": 59660587,
      "deposit_liquidity": "934864722135742",
      "deposit_tx": "0xafe42fe29de67337d5b489226dbfdf8a8b315e2deabaf6c744b47c2dd9ff7a58",
      "deposited_at": "2026-11-19T21:08:13+00:00",
      "deposited_nock": "247.5790",
      "deposited_usdc": "1221.85",
      "net_nock_bought": "5303.7885",
      "net_usdc_spent": "84.35",
      "reason": null,
      "status": "verified"
    },
    "spot_price_usd": "0.01480505",
    "state": "in_range",
    "state_text": "in range: it converts USDC to NOCK as the price falls and NOCK back to USDC as it rises",
    "usdc_now": "1137.51",
    "value_usd": "1219.69",
    "vs_top": {
      "converted_pct": "7.23",
      "curve_avg_price_usd": "0.01595807",
      "nock_at_bottom": null,
      "nock_held_vs_top": "5551.3674",
      "usdc_at_top": "1226.09",
      "usdc_converted_vs_top": "88.59"
    }
  },
  {
    "config_id": "ladder-deep",
    "fees_usd": "0.00",
    "high_pct_of_spot": "-30.23",
    "high_price_usd": "0.01032932",
    "label": "v4 USDC/NOCK 3.00% #3299001",
    "low_pct_of_spot": "-54.16",
    "low_price_usd": "0.00678699",
    "nock_now": "0.0000",
    "open_end": null,
    "pool_price_usd": "0.01480505",
    "position_id": "uniswap-v4:base:3299001",
    "rung": null,
    "rung_note": "rung is not set in config.yaml (near or deep): NockNock assumes neither",
    "since_deposit": {
      "avg_buy_price_usd": null,
      "current_liquidity": "400000000000000",
      "deposit_block": null,
      "deposit_liquidity": null,
      "deposit_tx": null,
      "deposited_at": null,
      "deposited_nock": null,
      "deposited_usdc": null,
      "net_nock_bought": null,
      "net_usdc_spent": null,
      "reason": "no deposit_tx is set for this ladder in config.yaml: NockNock has no baseline for what it bought and does not invent one",
      "status": "no_baseline"
    },
    "spot_price_usd": "0.01480505",
    "state": "holds_only_usdc",
    "state_text": "holds only USDC: the price is above the range, so it converts nothing until NOCK falls into it at $0.01032932",
    "usdc_now": "77.00",
    "value_usd": "77.00",
    "vs_top": {
      "converted_pct": "0.00",
      "curve_avg_price_usd": null,
      "nock_at_bottom": "9196.4045",
      "nock_held_vs_top": "0.0000",
      "usdc_at_top": "77.00",
      "usdc_converted_vs_top": "0.00"
    }
  }
];

/** near_rung_bought_plan_due: data.positions */
export const POSITIONS_BOUGHT: Position[] = [
  {
    "amount0": "88.098438",
    "amount1": "6088.887892",
    "amount_usd": "178.24",
    "fee_tier_bps": 300,
    "fees_usd": "0.000000",
    "full_range": false,
    "in_range": true,
    "is_price_source": true,
    "liquidity_amount": "185454734478733",
    "lower_price": "0.00550148",
    "lower_tick": 262200,
    "network": "base",
    "nft_id": 3163450,
    "pool_active": true,
    "pool_id": "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13",
    "position_id": "uniswap-v4:base:3163450",
    "protocol": "uniswap-v4",
    "tick_spacing": 300,
    "token0": {
      "address": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
      "decimals": 6,
      "symbol": "USDC"
    },
    "token1": {
      "address": "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722",
      "decimals": 16,
      "symbol": "NOCK"
    },
    "tokens_owed_0": "0.000000",
    "tokens_owed_1": "0.000000",
    "upper_price": "0.04106",
    "upper_tick": 282300,
    "wallet": "0xee114cc97ec5adeb34ba5645e315b497d8487a45"
  },
  {
    "amount0": "0.007567",
    "amount1": "7.288445",
    "amount_usd": "13.73",
    "fee_tier_bps": 100,
    "fees_usd": "0.000000",
    "full_range": false,
    "in_range": true,
    "is_price_source": false,
    "liquidity_amount": "1305339058220032430",
    "lower_price": "0.00000564468",
    "lower_tick": 70400,
    "network": "base",
    "nft_id": 6336662,
    "npm_address": "0x03a520b32c04bf3beef7beb72e919cf822ed34f1",
    "pool_active": true,
    "pool_id": "0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a",
    "position_id": "uniswap-v3:base:6336662",
    "protocol": "uniswap-v3",
    "token0": {
      "address": "0x4200000000000000000000000000000000000006",
      "decimals": 18,
      "symbol": "WETH"
    },
    "token1": {
      "address": "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722",
      "decimals": 16,
      "symbol": "NOCK"
    },
    "tokens_owed_0": "0.000000",
    "tokens_owed_1": "0.000000",
    "upper_price": "0.00000876435",
    "upper_tick": 74800,
    "wallet": "0xee114cc97ec5adeb34ba5645e315b497d8487a45"
  },
  {
    "amount0": "1137.505913",
    "amount1": "5551.367441",
    "amount_usd": "1219.69",
    "fee_tier_bps": 300,
    "fees_usd": "0.000000",
    "full_range": false,
    "in_range": true,
    "is_price_source": false,
    "liquidity_amount": "934864722135742",
    "lower_price": "0.0000000000000000000000000000298994",
    "lower_tick": 270900,
    "network": "base",
    "nft_id": 3240369,
    "pool_active": true,
    "pool_id": "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13",
    "position_id": "uniswap-v4:base:3240369",
    "protocol": "uniswap-v4",
    "role": "ladder",
    "rung": "near",
    "tick_spacing": 300,
    "token0": {
      "address": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
      "decimals": 6,
      "symbol": "USDC"
    },
    "token1": {
      "address": "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722",
      "decimals": 16,
      "symbol": "NOCK"
    },
    "tokens_owed_0": "0.000000",
    "tokens_owed_1": "0.000000",
    "upper_price": "0.01720",
    "upper_tick": 887100,
    "wallet": "0xee114cc97ec5adeb34ba5645e315b497d8487a45"
  },
  {
    "amount0": "77.000291",
    "amount1": "0.000000",
    "amount_usd": "77.00",
    "fee_tier_bps": 300,
    "fees_usd": "0.000000",
    "full_range": false,
    "in_range": false,
    "is_price_source": false,
    "liquidity_amount": "400000000000000",
    "lower_price": "0.00678699",
    "lower_tick": 276000,
    "network": "base",
    "nft_id": 3299001,
    "pool_active": true,
    "pool_id": "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13",
    "position_id": "uniswap-v4:base:3299001",
    "protocol": "uniswap-v4",
    "role": "ladder",
    "tick_spacing": 300,
    "token0": {
      "address": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
      "decimals": 6,
      "symbol": "USDC"
    },
    "token1": {
      "address": "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722",
      "decimals": 16,
      "symbol": "NOCK"
    },
    "tokens_owed_0": "0.000000",
    "tokens_owed_1": "0.000000",
    "upper_price": "0.01033",
    "upper_tick": 280200,
    "wallet": "0xee114cc97ec5adeb34ba5645e315b497d8487a45"
  }
] as Position[];

/** near_rung_bought_plan_due: the reserve fields of global_status */
export const RESERVE_BOUGHT = {
  "cash_reserve_usdc": "950.00",
  "reserve_stable_usdc": "2355.60",
  "reserve_headroom_usdc": "1405.60",
  "reserve_ladder_usdc": "1214.51",
  "dry_powder_usdc": "835.73"
};

/** near_rung_bought_plan_due: derived_metrics.cost_basis */
export const COST_BASIS_BOUGHT = {
  "plan_buys": {
    "acquired_nock": null,
    "avg_cost_usd": null,
    "excludes": [
      "NOCK held before the first recorded buy, and any buy that was not recorded.",
      "Everything that is not a buy: sells, NOCK that LP ranges sell or buy back, fees earned.",
      "Gas paid in ETH."
    ],
    "fills": 0,
    "first_fill_at": null,
    "includes": [
      "Every receipt-verified buy in the accumulation ledger, across all plans: the USDC that left the wallet and the NOCK that entered it in each recorded transaction. Swap fees and price impact are inside those amounts."
    ],
    "last_fill_at": null,
    "plans": [],
    "reason": null,
    "spent_usdc": null,
    "status": "no_fills",
    "unrealized_pct": null,
    "unrealized_usd": null,
    "value_at_spot_usd": null
  },
  "spot_price_usd": "0.014805",
  "whole_stack": {
    "as_of": "2026-11-19T00:00:00+00:00",
    "avg_cost_usd": "0.016872",
    "basis_cost_usd": "1609.40",
    "basis_nock": "95390.5785",
    "bought_since_nock": "0.0000",
    "bought_since_usdc": "0.00",
    "excludes": [
      "Buys confirmed at or before 2026-11-19T00:00:00+00:00: they are taken to be inside the opening figures and are not added again.",
      "Everything after 2026-11-19T00:00:00+00:00 that is not a recorded buy: sells, NOCK that LP ranges sell or buy back, fees earned, transfers in or out, unrecorded buys.",
      "Realized gains and losses, and gas paid in ETH.",
      "Tracked ladders left out: uniswap-v4:base:3299001: no verified deposit baseline (no_baseline)."
    ],
    "fills_since": 0,
    "held_nock": "127395",
    "includes": [
      "The opening figures entered in config: 90086.7900 NOCK for $1525.05 as of 2026-11-19T00:00:00+00:00, source: test figures. NockNock did not read or check them.",
      "Every receipt-verified buy confirmed after 2026-11-19T00:00:00+00:00: 0 so far.",
      "The net NOCK bought by 1 of 2 tracked ladders since a receipt-verified deposit after 2026-11-19T00:00:00+00:00: 5303.7885 NOCK for $84.35, measured from each deposit_tx receipt to now."
    ],
    "ladder_buys_since": {
      "ladders_counted": 1,
      "nock": "5303.7885",
      "not_counted": [
        "uniswap-v4:base:3299001: no verified deposit baseline (no_baseline)"
      ],
      "usdc": "84.35"
    },
    "opening_avg_cost_usd": "0.016929",
    "opening_cost_usd": "1525.05",
    "opening_nock": "90086.7900",
    "reason": null,
    "source": "test figures",
    "status": "ok",
    "unrealized_pct": "-12.25",
    "unrealized_usd": "-197.14",
    "value_at_spot_usd": "1412.26"
  }
};

/** near_rung_bought_plan_due: the deviation rows */
export const DEVIATION_BOUGHT = [
  {
    "actual_weight_bps": 3943,
    "bucket_name": "Sovereign Treasury",
    "deviation_bps": -4057,
    "status": "under_funded",
    "target_weight_bps": 8000
  },
  {
    "actual_weight_bps": 441,
    "bucket_name": "NOCK LP (NOCK-pair positions)",
    "deviation_bps": -1059,
    "status": "under_funded",
    "target_weight_bps": 1500
  },
  {
    "actual_weight_bps": 2423,
    "bucket_name": "Liquid USDC (Escrow/Vault)",
    "deviation_bps": 1923,
    "status": "over_funded",
    "target_weight_bps": 500
  },
  {
    "actual_weight_bps": 207,
    "bucket_name": "Gas (wallet ETH+WETH)",
    "deviation_bps": 207,
    "status": "no_target",
    "target_weight_bps": 0
  },
  {
    "actual_weight_bps": 0,
    "bucket_name": "AERO (wallet)",
    "deviation_bps": 0,
    "status": "no_target",
    "target_weight_bps": 0
  },
  {
    "actual_weight_bps": 2984,
    "bucket_name": "NOCK ladders (measured)",
    "deviation_bps": 2984,
    "status": "no_target",
    "target_weight_bps": 0
  }
];
