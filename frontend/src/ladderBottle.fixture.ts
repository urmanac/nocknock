/**
 * The ladder suggestion for a configured shape (operator_profile.ladder_rungs: the
 * operator's "bottle", cap | near | deep) as the worker states it, copied from
 * ingestion-worker/testdata/ladder_suggest/golden_ladder_bottle.json, which
 * ingestion-worker/src/tests_ladder_bottle.rs writes. The fixture is the #166 one (spot
 * $0.012944, wallet USDC 1545.8357, LP NOCK budget 86.7891, reserve $950, bottom $0.01)
 * with the recorded fee measurement. ladderBottle.test.ts checks the copy against the file
 * when the worker directory is present.
 */
import type { LadderSuggestion } from './ladderSuggestion';

/** bottle: the bottle with nothing deposited: three deposits */
export const LADDER_BOTTLE: LadderSuggestion = {
  "after_deposit": {
    "detection_note": null,
    "detection_status": null,
    "kind": "tracked_automatically",
    "text": "After you deposit there is nothing to copy or configure. NockNock looks for the wallet's new position NFT on-chain at the start of every cycle (CHECK NOW runs one at once), so a deposit is normally found in the first cycle after it lands. A position your wallet minted in this pool with exactly a rung's ticks is tracked as a ladder by itself, with its mint as the baseline. Any other range it finds is shown as found on-chain and is not counted until one tap says how to track it."
  },
  "assumptions": [
    "The amounts are your own shape (operator_profile.ladder_rungs) sized to the most the standing reserve allows at the gate price; fees and gas are not counted in them.",
    "A cap is in range now and holds NOCK; every other rung holds only USDC now and earns no fees until NOCK falls to its top.",
    "Rungs already tracked are part of the shape: their USDC counts toward its total, and only what is left is listed.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "Each fee line is a past measurement on our own tracked range scaled to the rung's liquidity: an estimate, not a forecast."
  ],
  "candidates": null,
  "excluded": [],
  "freshness_rule": "Shown only while the snapshot is fresh: the dashboard's sync status is not stale, or the worker's health is ok.",
  "gates": {
    "bottom_setting": {
      "key": "operator_profile.ladder_bottom_usd",
      "status": "set",
      "value": "0.01"
    },
    "floor_breach_effect": "lowers",
    "ladder_tracked": {
      "reason": "tracked ladder rungs are part of your shape: each is matched to a rung, and only what is left is listed",
      "status": "none"
    },
    "nock_budget": {
      "lp_nock_budget": "86.7891",
      "nock_needed": "86.7891",
      "reason": null,
      "status": "fits"
    },
    "reserve": {
      "binding": "reserve",
      "cash_reserve_usdc": "950.00",
      "key": "operator_profile.cash_reserve_usdc",
      "margin_usdc": "0.00",
      "max_usdc": "766.99",
      "minimum_usdc": "1.00",
      "read_at_rule": "your ladder bottom (operator_profile.ladder_bottom_usd $0.01000000), below the deepest rung's bottom",
      "read_at_usd": "0.01000000",
      "rule": "the stable balance at the gate price stays at or above the reserve",
      "shape_total_usdc": "766.99",
      "stable_at_gate_usdc": "950.00",
      "status": "allows"
    },
    "shape_setting": {
      "key": "operator_profile.ladder_rungs",
      "status": "set"
    }
  },
  "headline": "Ladder: 3 of 3 deposits left, up to $766.99 USDC and about 87 NOCK.",
  "id": "ladder_suggestion:NOCK",
  "minimum_usdc": "1.00",
  "mint_note": "liquidity_raw is what these amounts buy, rounded down (LiquidityAmounts); amount0_raw and amount1_raw are what a mint of it pulls, rounded up (SqrtPriceMath), never more than the maxima (the rung's USDC and NOCK truncated to raw units)",
  "missing": [],
  "numbers": {
    "avg_buy_price_usd": "0.01066922",
    "binding": "reserve",
    "cash_reserve_usdc": "950.00",
    "existing_ranges_usdc_at_gate": "171.15",
    "existing_ranges_usdc_now": "223.71",
    "gate_price_usd": "0.01000000",
    "lp_nock_budget": "86.7891",
    "nock_at_bottom": "71974.8945",
    "nock_bought_at_bottom": "71888.1054",
    "nock_now": "86.7891",
    "pool_price_usd": "0.01294400",
    "shape_total_usdc": "766.99",
    "shape_tracked_usdc": "0.00",
    "stable_at_gate_usdc": "950.00",
    "usdc_max": "766.99",
    "wallet_usdc": "1545.84",
    "wallet_usdc_after": "778.85"
  },
  "outcomes": [
    {
      "avg_buy_price_usd": "0.01183214",
      "kind": "fall_inside",
      "nock_bought": "9811",
      "pct_of_spot": "-15%",
      "price_usd": "0.01096801",
      "text": "A fall to $0.01097 (-15%), the near rung's bottom, would buy about 9,811 NOCK for $116.09 of the USDC, an average of $0.01183.",
      "usdc_spent": "116.09"
    },
    {
      "avg_buy_price_usd": "0.01066922",
      "kind": "fall_to_bottom",
      "nock_bought": "71888",
      "pct_of_spot": "-23%",
      "price_usd": "0.01002405",
      "text": "A fall to the ladder bottom $0.01002 (-23%) would buy about 71,888 NOCK in total with all $766.99, an average of $0.01067.",
      "usdc_spent": "766.99"
    },
    {
      "kind": "rise",
      "nock_sold": "87",
      "pct_of_spot": "+1.4%",
      "price_usd": "0.01313098",
      "text": "A rise to $0.01313 (+1.4%) or above sells the cap's NOCK, about 87, for about $1.13 of USDC, and sells nothing else: every other rung holds 0 NOCK.",
      "usdc_received": "1.13"
    }
  ],
  "picture": {
    "bars": [
      {
        "deployed": false,
        "high_usd": "0.01313098",
        "index": 1,
        "label": "$1.23 + 87 NOCK",
        "low_usd": "0.01274292",
        "rung": "cap",
        "status": "to_deposit",
        "usdc": "1.23"
      },
      {
        "deployed": false,
        "high_usd": "0.01274292",
        "index": 2,
        "label": "$114.86",
        "low_usd": "0.01096801",
        "rung": "near",
        "status": "to_deposit",
        "usdc": "114.86"
      },
      {
        "deployed": false,
        "high_usd": "0.01096801",
        "index": 3,
        "label": "$650.89",
        "low_usd": "0.01002405",
        "rung": "deep",
        "status": "to_deposit",
        "usdc": "650.89"
      }
    ],
    "bottom_requested_usd": "0.01",
    "bottom_usd": "0.01002405",
    "reserve_label": "$950 kept for below $0.01",
    "reserve_usdc": "950.00",
    "spot_usd": "0.01294400"
  },
  "reason": null,
  "rungs": [
    {
      "at_bottom": {
        "avg_buy_price_usd": "0.01284307",
        "nock_bought": "95.4600",
        "nock_held": "182.2491"
      },
      "bottom": {
        "pct_of_spot": "-1.55",
        "price_usd": "0.01274292",
        "tick": 273900
      },
      "bounds_nock_per_usdc": {
        "high": "78.4750",
        "low": "76.1558"
      },
      "bounds_usd": {
        "high": "0.01313098",
        "low": "0.01274292"
      },
      "cap": {
        "bound_by": "lp_nock_budget",
        "note": "the cap holds the whole LP NOCK budget, 86.7891 NOCK; the curve pairs it with $1.23 USDC"
      },
      "deep_link": {
        "amount_to_enter": "Amount to enter in the app: 86.7890 NOCK, in the NOCK field; the app then works out the USDC side, about $1.23 at this reading.",
        "format_note": "The link format is the app's own and undocumented: check the pool and range in the app against the figures here.",
        "host": "app.uniswap.org",
        "not_prefilled": [
          "amount"
        ],
        "prefilled": [
          "pair",
          "fee and tick spacing",
          "range ticks"
        ],
        "reason": null,
        "url": "https://app.uniswap.org/positions/add/new?chain=base&protocolVersion=v4&currencyA=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913&currencyB=0x9B5E262cF9bb04869ab40b19AF91D2dc85761722&fee=%7B%22feeAmount%22%3A30000%2C%22isDynamic%22%3Afalse%2C%22tickSpacing%22%3A300%7D&priceRangeState=%7B%22fullRange%22%3Afalse%2C%22maxTick%22%3A273900%2C%22minTick%22%3A273600%2C%22priceInverted%22%3Afalse%7D&step=1"
      },
      "fee_estimate": {
        "coverage": {
          "note": "the whole range lies inside the measuring range",
          "whole": true
        },
        "in_range_share_reason": null,
        "in_range_share_stored_history": {
          "of_seconds": "547205",
          "seconds": "10659",
          "share_pct": "1.95"
        },
        "intervals": 1836,
        "measured_at": "2026-11-20T11:50:00Z",
        "measured_by": "uniswap-v4:base:3163450",
        "measured_days": "6.33",
        "measured_label": "v4 USDC/NOCK 3.00% #3163450",
        "per_day_in_range_diluted_usd": "0.84",
        "per_day_in_range_usd": "0.84",
        "reason": null,
        "replay_reason": null,
        "status": "measured",
        "stored_history_replay": {
          "edge_upper_usd": "0.15",
          "inside_upper_usd": "0.02"
        }
      },
      "in_range_now": true,
      "index": 1,
      "mint": {
        "amount0_raw": "1225999",
        "amount1_raw": "867890958577132038",
        "liquidity_raw": "138192109253769"
      },
      "nock": {
        "address": "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722",
        "amount": "86.7890958577132038",
        "decimals": 16,
        "raw": "867890958577132038",
        "to_enter": "86.7890"
      },
      "nock_limit": {
        "field_to_type": "NOCK",
        "is": "lp_nock_budget",
        "nock": "86.7891",
        "nock_to_enter": "86.7890",
        "text": "The NOCK side must not exceed 86.7891 NOCK, your LP NOCK budget. The limit holds when the NOCK field is the one typed, at 86.7890 NOCK or less. When the USDC field is typed instead, the app works out the NOCK side from the price at that moment, and it can come out above the limit; neither the link nor the app applies a NOCK limit of its own.",
        "usdc_then_about": "1.23"
      },
      "of": 3,
      "pool": {
        "fee_raw": 30000,
        "fee_tier_bps": 300,
        "hooks": "0x0000000000000000000000000000000000000000",
        "label": "the USDC/NOCK 3% v4 pool",
        "network": "base",
        "new_position": true,
        "note": "a NEW position (a new NFT) in this pool; position_id is the tracked position the pool was read from",
        "npm_address": null,
        "pool_id": "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13",
        "pool_price_usd": "0.01294400",
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
        }
      },
      "rung": "cap",
      "status": "to_deposit",
      "summary": "about 86.79 NOCK + $1.23 USDC, in range now",
      "target": {
        "counterparts": [],
        "deployed_usdc": "0.00",
        "usdc": "1.23"
      },
      "tick_lower": 273600,
      "tick_upper": 273900,
      "top": {
        "pct_of_spot": "+1.44",
        "price_usd": "0.01313098",
        "rule": "the tick band containing the pool's tick (the cap)",
        "tick": 273600
      },
      "usdc": {
        "address": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
        "amount": "1.225999",
        "decimals": 6,
        "raw": "1225999",
        "to_enter": "1.22"
      },
      "would_sign": "In the Uniswap web app, with your own wallet: one new position in the USDC/NOCK 3% v4 pool, ticks 273600 to 273900, holding up to 86.7890 NOCK and about $1.23 USDC (the app may first ask for USDC and NOCK approvals). NockNock builds no transaction and signs nothing."
    },
    {
      "at_bottom": {
        "avg_buy_price_usd": "0.01182220",
        "nock_bought": "9716.0065",
        "nock_held": "9716.0065"
      },
      "bottom": {
        "pct_of_spot": "-15.27",
        "price_usd": "0.01096801",
        "tick": 275400
      },
      "bounds_nock_per_usdc": {
        "high": "91.1742",
        "low": "78.4750"
      },
      "bounds_usd": {
        "high": "0.01274292",
        "low": "0.01096801"
      },
      "cap": null,
      "deep_link": {
        "amount_to_enter": "Amount to enter in the app: $114.86 USDC",
        "format_note": "The link format is the app's own and undocumented: check the pool and range in the app against the figures here.",
        "host": "app.uniswap.org",
        "not_prefilled": [
          "amount"
        ],
        "prefilled": [
          "pair",
          "fee and tick spacing",
          "range ticks"
        ],
        "reason": null,
        "url": "https://app.uniswap.org/positions/add/new?chain=base&protocolVersion=v4&currencyA=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913&currencyB=0x9B5E262cF9bb04869ab40b19AF91D2dc85761722&fee=%7B%22feeAmount%22%3A30000%2C%22isDynamic%22%3Afalse%2C%22tickSpacing%22%3A300%7D&priceRangeState=%7B%22fullRange%22%3Afalse%2C%22maxTick%22%3A275400%2C%22minTick%22%3A273900%2C%22priceInverted%22%3Afalse%7D&step=1"
      },
      "fee_estimate": {
        "coverage": {
          "note": "the whole range lies inside the measuring range",
          "whole": true
        },
        "in_range_share_reason": null,
        "in_range_share_stored_history": {
          "of_seconds": "547205",
          "seconds": "156662",
          "share_pct": "28.63"
        },
        "intervals": 1836,
        "measured_at": "2026-11-20T11:50:00Z",
        "measured_by": "uniswap-v4:base:3163450",
        "measured_days": "6.33",
        "measured_label": "v4 USDC/NOCK 3.00% #3163450",
        "per_day_in_range_diluted_usd": "8.42",
        "per_day_in_range_usd": "8.56",
        "reason": null,
        "replay_reason": null,
        "status": "measured",
        "stored_history_replay": {
          "edge_upper_usd": "5.16",
          "inside_upper_usd": "6.39"
        }
      },
      "in_range_now": false,
      "index": 2,
      "mint": {
        "amount0_raw": "114864600",
        "amount1_raw": "0",
        "liquidity_raw": "1408301615002237"
      },
      "nock": {
        "address": "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722",
        "amount": "0.0000000000000000",
        "decimals": 16,
        "raw": "0",
        "to_enter": "0.0000"
      },
      "nock_limit": null,
      "of": 3,
      "pool": {
        "fee_raw": 30000,
        "fee_tier_bps": 300,
        "hooks": "0x0000000000000000000000000000000000000000",
        "label": "the USDC/NOCK 3% v4 pool",
        "network": "base",
        "new_position": true,
        "note": "a NEW position (a new NFT) in this pool; position_id is the tracked position the pool was read from",
        "npm_address": null,
        "pool_id": "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13",
        "pool_price_usd": "0.01294400",
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
        }
      },
      "rung": "near",
      "status": "to_deposit",
      "summary": "$114.86 USDC, no NOCK, below the price now",
      "target": {
        "counterparts": [],
        "deployed_usdc": "0.00",
        "usdc": "114.86"
      },
      "tick_lower": 273900,
      "tick_upper": 275400,
      "top": {
        "pct_of_spot": "-1.55",
        "price_usd": "0.01274292",
        "rule": "the usable tick at or below the pool price (the first rung still drawable)",
        "tick": 273900
      },
      "usdc": {
        "address": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
        "amount": "114.864600",
        "decimals": 6,
        "raw": "114864600",
        "to_enter": "114.86"
      },
      "would_sign": "In the Uniswap web app, with your own wallet: one new position in the USDC/NOCK 3% v4 pool, ticks 273900 to 275400, holding up to $114.86 USDC and 0 NOCK (the app may first ask for a USDC approval). NockNock builds no transaction and signs nothing."
    },
    {
      "at_bottom": {
        "avg_buy_price_usd": "0.01048542",
        "nock_bought": "62076.6389",
        "nock_held": "62076.6389"
      },
      "bottom": {
        "pct_of_spot": "-22.56",
        "price_usd": "0.01002405",
        "tick": 276300
      },
      "bounds_nock_per_usdc": {
        "high": "99.7600",
        "low": "91.1742"
      },
      "bounds_usd": {
        "high": "0.01096801",
        "low": "0.01002405"
      },
      "cap": null,
      "deep_link": {
        "amount_to_enter": "Amount to enter in the app: $650.89 USDC",
        "format_note": "The link format is the app's own and undocumented: check the pool and range in the app against the figures here.",
        "host": "app.uniswap.org",
        "not_prefilled": [
          "amount"
        ],
        "prefilled": [
          "pair",
          "fee and tick spacing",
          "range ticks"
        ],
        "reason": null,
        "url": "https://app.uniswap.org/positions/add/new?chain=base&protocolVersion=v4&currencyA=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913&currencyB=0x9B5E262cF9bb04869ab40b19AF91D2dc85761722&fee=%7B%22feeAmount%22%3A30000%2C%22isDynamic%22%3Afalse%2C%22tickSpacing%22%3A300%7D&priceRangeState=%7B%22fullRange%22%3Afalse%2C%22maxTick%22%3A276300%2C%22minTick%22%3A275400%2C%22priceInverted%22%3Afalse%7D&step=1"
      },
      "fee_estimate": {
        "coverage": {
          "note": "the whole range lies inside the measuring range",
          "whole": true
        },
        "in_range_share_reason": null,
        "in_range_share_stored_history": {
          "of_seconds": "547205",
          "seconds": "31832",
          "share_pct": "5.82"
        },
        "intervals": 1836,
        "measured_at": "2026-11-20T11:50:00Z",
        "measured_by": "uniswap-v4:base:3163450",
        "measured_days": "6.33",
        "measured_label": "v4 USDC/NOCK 3.00% #3163450",
        "per_day_in_range_diluted_usd": "73.03",
        "per_day_in_range_usd": "85.90",
        "reason": null,
        "replay_reason": null,
        "status": "measured",
        "stored_history_replay": {
          "edge_upper_usd": "50.00",
          "inside_upper_usd": "32.55"
        }
      },
      "in_range_now": false,
      "index": 3,
      "mint": {
        "amount0_raw": "650899400",
        "amount1_raw": "0",
        "liquidity_raw": "14125167068081104"
      },
      "nock": {
        "address": "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722",
        "amount": "0.0000000000000000",
        "decimals": 16,
        "raw": "0",
        "to_enter": "0.0000"
      },
      "nock_limit": null,
      "of": 3,
      "pool": {
        "fee_raw": 30000,
        "fee_tier_bps": 300,
        "hooks": "0x0000000000000000000000000000000000000000",
        "label": "the USDC/NOCK 3% v4 pool",
        "network": "base",
        "new_position": true,
        "note": "a NEW position (a new NFT) in this pool; position_id is the tracked position the pool was read from",
        "npm_address": null,
        "pool_id": "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13",
        "pool_price_usd": "0.01294400",
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
        }
      },
      "rung": "deep",
      "status": "to_deposit",
      "summary": "$650.89 USDC, no NOCK, below the price now",
      "target": {
        "counterparts": [],
        "deployed_usdc": "0.00",
        "usdc": "650.90"
      },
      "tick_lower": 275400,
      "tick_upper": 276300,
      "top": {
        "pct_of_spot": "-15.27",
        "price_usd": "0.01096801",
        "rule": "the nearest usable tick",
        "tick": 275400
      },
      "usdc": {
        "address": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
        "amount": "650.899400",
        "decimals": 6,
        "raw": "650899400",
        "to_enter": "650.89"
      },
      "would_sign": "In the Uniswap web app, with your own wallet: one new position in the USDC/NOCK 3% v4 pool, ticks 275400 to 276300, holding up to $650.89 USDC and 0 NOCK (the app may first ask for a USDC approval). NockNock builds no transaction and signs nothing."
    }
  ],
  "sentence": "Your ladder shape in the USDC/NOCK 3% v4 pool: 3 of 3 deposits are left, holding up to $766.99 USDC and 86.79 NOCK now: sized to leave the stable balance at or above your $950 reserve at $0.01. Fully converted at $0.01002 they would hold about 71,975 NOCK, about 71,888 of it bought, at an average of $0.01067.",
  "shape": {
    "deployed": 0,
    "left": 3,
    "none": 0,
    "of": 3,
    "over": 0,
    "passed": 0,
    "rungs": [
      {
        "bounds_usd": {
          "high": "0.01313098",
          "low": "0.01274292"
        },
        "counterparts": [],
        "deployed_nock": "0.0000",
        "deployed_usdc": "0.00",
        "holds_now": null,
        "in_range_now": true,
        "index": 1,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "86.7891",
        "remaining_usdc": "1.23",
        "rung": "cap",
        "status": "to_deposit",
        "target_nock": "86.7891",
        "target_usdc": "1.23",
        "text": "Deposit 1 of 3 (cap): left to deposit, about 86.79 NOCK + $1.23 USDC.",
        "tick_lower": 273600,
        "tick_upper": 273900
      },
      {
        "bounds_usd": {
          "high": "0.01274292",
          "low": "0.01096801"
        },
        "counterparts": [],
        "deployed_nock": null,
        "deployed_usdc": "0.00",
        "holds_now": null,
        "in_range_now": false,
        "index": 2,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": null,
        "remaining_usdc": "114.86",
        "rung": "near",
        "status": "to_deposit",
        "target_nock": null,
        "target_usdc": "114.86",
        "text": "Deposit 2 of 3 (near rung): left to deposit, $114.86 USDC.",
        "tick_lower": 273900,
        "tick_upper": 275400
      },
      {
        "bounds_usd": {
          "high": "0.01096801",
          "low": "0.01002405"
        },
        "counterparts": [],
        "deployed_nock": null,
        "deployed_usdc": "0.00",
        "holds_now": null,
        "in_range_now": false,
        "index": 3,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": null,
        "remaining_usdc": "650.90",
        "rung": "deep",
        "status": "to_deposit",
        "target_nock": null,
        "target_usdc": "650.90",
        "text": "Deposit 3 of 3 (deep rung): left to deposit, $650.90 USDC.",
        "tick_lower": 275400,
        "tick_upper": 276300
      }
    ],
    "source": "operator_profile.ladder_rungs",
    "status": "computed",
    "total_usdc": "766.99",
    "tracked_usdc": "0.00",
    "unmatched": []
  },
  "shown": true,
  "stated": {
    "accumulation_plan_unspent": null,
    "cash_target_usd": null,
    "existing_ranges_below_bottom": null,
    "lines": [
      "No accumulation plan is configured.",
      "Tax reserve: $0.00 (stated, not a gate).",
      "No cash target is set."
    ],
    "note": "The plan budget, the tax reserve and the cash target are stated, not gates: only the standing reserve gates the ladder's USDC.",
    "tax_reserve_usd": {
      "is": "stated, not a gate",
      "usd": "0.00"
    }
  },
  "statement": "A measurement from this cycle's reads of what your ladder shape would do. NockNock does not choose the shape, the bottom or the amounts, builds no transaction, and signs and sends nothing.",
  "status": "ready",
  "tier": "top",
  "token": "NOCK",
  "tracked": [],
  "wallet_after": {
    "cash_reserve_usdc": "950.00",
    "deposits_usdc": "766.99",
    "existing_ranges_usdc_at_gate": "171.15",
    "existing_ranges_usdc_now": "223.71",
    "gate_price_usd": "0.01000000",
    "rule": "the standing reserve is measured on wallet USDC plus USDC inside NOCK ranges, read at the gate price",
    "stable_at_gate_usdc": "950.00",
    "text": "After these deposits your wallet holds about $779 USDC; the $950 reserve counts the $224 of USDC inside your existing NOCK ranges. At $0.01 with every rung converted, the stable balance reads $950.00: the wallet's $778.85 plus $171.15 still inside those ranges.",
    "wallet_alone_below_reserve": true,
    "wallet_usdc_after": "778.85",
    "wallet_usdc_now": "1545.84"
  },
  "what_changed": "Your ladder shape has 3 of 3 deposits left in the USDC/NOCK 3% v4 pool: up to $766.99 USDC and about 87 NOCK."
} as any;

/** deep_deployed: the deep rung deployed: two deposits left */
export const LADDER_BOTTLE_DEEP_DEPLOYED: LadderSuggestion = {
  "after_deposit": {
    "detection_note": null,
    "detection_status": null,
    "kind": "tracked_automatically",
    "text": "After you deposit there is nothing to copy or configure. NockNock looks for the wallet's new position NFT on-chain at the start of every cycle (CHECK NOW runs one at once), so a deposit is normally found in the first cycle after it lands. A position your wallet minted in this pool with exactly a rung's ticks is tracked as a ladder by itself, with its mint as the baseline. Any other range it finds is shown as found on-chain and is not counted until one tap says how to track it."
  },
  "assumptions": [
    "The amounts are your own shape (operator_profile.ladder_rungs) sized to the most the standing reserve allows at the gate price; fees and gas are not counted in them.",
    "A cap is in range now and holds NOCK; every other rung holds only USDC now and earns no fees until NOCK falls to its top.",
    "Rungs already tracked are part of the shape: their USDC counts toward its total, and only what is left is listed.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "Each fee line is a past measurement on our own tracked range scaled to the rung's liquidity: an estimate, not a forecast."
  ],
  "candidates": null,
  "excluded": [],
  "freshness_rule": "Shown only while the snapshot is fresh: the dashboard's sync status is not stale, or the worker's health is ok.",
  "gates": {
    "bottom_setting": {
      "key": "operator_profile.ladder_bottom_usd",
      "status": "set",
      "value": "0.01"
    },
    "floor_breach_effect": "lowers",
    "ladder_tracked": {
      "reason": "tracked ladder rungs are part of your shape: each is matched to a rung, and only what is left is listed",
      "status": "counted"
    },
    "nock_budget": {
      "lp_nock_budget": "86.7891",
      "nock_needed": "86.7891",
      "reason": null,
      "status": "fits"
    },
    "reserve": {
      "binding": "reserve",
      "cash_reserve_usdc": "950.00",
      "key": "operator_profile.cash_reserve_usdc",
      "margin_usdc": "0.00",
      "max_usdc": "116.09",
      "minimum_usdc": "1.00",
      "read_at_rule": "your ladder bottom (operator_profile.ladder_bottom_usd $0.01000000), below the deepest rung's bottom",
      "read_at_usd": "0.01000000",
      "rule": "the stable balance at the gate price stays at or above the reserve",
      "shape_total_usdc": "766.99",
      "stable_at_gate_usdc": "950.00",
      "status": "allows"
    },
    "shape_setting": {
      "key": "operator_profile.ladder_rungs",
      "status": "set"
    }
  },
  "headline": "Ladder: 2 of 3 deposits left, up to $116.09 USDC and about 87 NOCK.",
  "id": "ladder_suggestion:NOCK",
  "minimum_usdc": "1.00",
  "mint_note": "liquidity_raw is what these amounts buy, rounded down (LiquidityAmounts); amount0_raw and amount1_raw are what a mint of it pulls, rounded up (SqrtPriceMath), never more than the maxima (the rung's USDC and NOCK truncated to raw units)",
  "missing": [],
  "numbers": {
    "avg_buy_price_usd": "0.01183214",
    "binding": "reserve",
    "cash_reserve_usdc": "950.00",
    "existing_ranges_usdc_at_gate": "171.15",
    "existing_ranges_usdc_now": "874.61",
    "gate_price_usd": "0.01000000",
    "lp_nock_budget": "86.7891",
    "nock_at_bottom": "9898.2556",
    "nock_bought_at_bottom": "9811.4665",
    "nock_now": "86.7891",
    "pool_price_usd": "0.01294400",
    "shape_total_usdc": "766.99",
    "shape_tracked_usdc": "650.90",
    "stable_at_gate_usdc": "950.00",
    "usdc_max": "116.09",
    "wallet_usdc": "894.94",
    "wallet_usdc_after": "778.85"
  },
  "outcomes": [
    {
      "avg_buy_price_usd": "0.01183214",
      "kind": "fall_to_bottom",
      "nock_bought": "9811",
      "pct_of_spot": "-15%",
      "price_usd": "0.01096801",
      "text": "A fall to $0.01097 (-15%), the bottom of what is left to deposit, would buy about 9,811 NOCK in total with all $116.09, an average of $0.01183.",
      "usdc_spent": "116.09"
    },
    {
      "kind": "rise",
      "nock_sold": "87",
      "pct_of_spot": "+1.4%",
      "price_usd": "0.01313098",
      "text": "A rise to $0.01313 (+1.4%) or above sells the cap's NOCK, about 87, for about $1.13 of USDC, and sells nothing else: every other rung holds 0 NOCK.",
      "usdc_received": "1.13"
    }
  ],
  "picture": {
    "bars": [
      {
        "deployed": false,
        "high_usd": "0.01313098",
        "index": 1,
        "label": "$1.23 + 87 NOCK",
        "low_usd": "0.01274292",
        "rung": "cap",
        "status": "to_deposit",
        "usdc": "1.23"
      },
      {
        "deployed": false,
        "high_usd": "0.01274292",
        "index": 2,
        "label": "$114.86",
        "low_usd": "0.01096801",
        "rung": "near",
        "status": "to_deposit",
        "usdc": "114.86"
      },
      {
        "deployed": true,
        "high_usd": "0.01096801",
        "index": 3,
        "label": "$650.90",
        "low_usd": "0.01002405",
        "rung": "deep",
        "status": "deployed",
        "usdc": "650.90"
      }
    ],
    "bottom_requested_usd": "0.01",
    "bottom_usd": "0.01002405",
    "reserve_label": "$950 kept for below $0.01",
    "reserve_usdc": "950.00",
    "spot_usd": "0.01294400"
  },
  "reason": null,
  "rungs": [
    {
      "at_bottom": {
        "avg_buy_price_usd": "0.01284307",
        "nock_bought": "95.4600",
        "nock_held": "182.2491"
      },
      "bottom": {
        "pct_of_spot": "-1.55",
        "price_usd": "0.01274292",
        "tick": 273900
      },
      "bounds_nock_per_usdc": {
        "high": "78.4750",
        "low": "76.1558"
      },
      "bounds_usd": {
        "high": "0.01313098",
        "low": "0.01274292"
      },
      "cap": {
        "bound_by": "lp_nock_budget",
        "note": "the cap holds the whole LP NOCK budget, 86.7891 NOCK; the curve pairs it with $1.23 USDC"
      },
      "deep_link": {
        "amount_to_enter": "Amount to enter in the app: 86.7890 NOCK, in the NOCK field; the app then works out the USDC side, about $1.23 at this reading.",
        "format_note": "The link format is the app's own and undocumented: check the pool and range in the app against the figures here.",
        "host": "app.uniswap.org",
        "not_prefilled": [
          "amount"
        ],
        "prefilled": [
          "pair",
          "fee and tick spacing",
          "range ticks"
        ],
        "reason": null,
        "url": "https://app.uniswap.org/positions/add/new?chain=base&protocolVersion=v4&currencyA=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913&currencyB=0x9B5E262cF9bb04869ab40b19AF91D2dc85761722&fee=%7B%22feeAmount%22%3A30000%2C%22isDynamic%22%3Afalse%2C%22tickSpacing%22%3A300%7D&priceRangeState=%7B%22fullRange%22%3Afalse%2C%22maxTick%22%3A273900%2C%22minTick%22%3A273600%2C%22priceInverted%22%3Afalse%7D&step=1"
      },
      "fee_estimate": {
        "coverage": {
          "note": "the whole range lies inside the measuring range",
          "whole": true
        },
        "in_range_share_reason": null,
        "in_range_share_stored_history": {
          "of_seconds": "547205",
          "seconds": "10659",
          "share_pct": "1.95"
        },
        "intervals": 1836,
        "measured_at": "2026-11-20T11:50:00Z",
        "measured_by": "uniswap-v4:base:3163450",
        "measured_days": "6.33",
        "measured_label": "v4 USDC/NOCK 3.00% #3163450",
        "per_day_in_range_diluted_usd": "0.84",
        "per_day_in_range_usd": "0.84",
        "reason": null,
        "replay_reason": null,
        "status": "measured",
        "stored_history_replay": {
          "edge_upper_usd": "0.15",
          "inside_upper_usd": "0.02"
        }
      },
      "in_range_now": true,
      "index": 1,
      "mint": {
        "amount0_raw": "1225999",
        "amount1_raw": "867890958577132038",
        "liquidity_raw": "138192109253769"
      },
      "nock": {
        "address": "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722",
        "amount": "86.7890958577132038",
        "decimals": 16,
        "raw": "867890958577132038",
        "to_enter": "86.7890"
      },
      "nock_limit": {
        "field_to_type": "NOCK",
        "is": "lp_nock_budget",
        "nock": "86.7891",
        "nock_to_enter": "86.7890",
        "text": "The NOCK side must not exceed 86.7891 NOCK, your LP NOCK budget. The limit holds when the NOCK field is the one typed, at 86.7890 NOCK or less. When the USDC field is typed instead, the app works out the NOCK side from the price at that moment, and it can come out above the limit; neither the link nor the app applies a NOCK limit of its own.",
        "usdc_then_about": "1.23"
      },
      "of": 3,
      "pool": {
        "fee_raw": 30000,
        "fee_tier_bps": 300,
        "hooks": "0x0000000000000000000000000000000000000000",
        "label": "the USDC/NOCK 3% v4 pool",
        "network": "base",
        "new_position": true,
        "note": "a NEW position (a new NFT) in this pool; position_id is the tracked position the pool was read from",
        "npm_address": null,
        "pool_id": "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13",
        "pool_price_usd": "0.01294400",
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
        }
      },
      "rung": "cap",
      "status": "to_deposit",
      "summary": "about 86.79 NOCK + $1.23 USDC, in range now",
      "target": {
        "counterparts": [],
        "deployed_usdc": "0.00",
        "usdc": "1.23"
      },
      "tick_lower": 273600,
      "tick_upper": 273900,
      "top": {
        "pct_of_spot": "+1.44",
        "price_usd": "0.01313098",
        "rule": "the tick band containing the pool's tick (the cap)",
        "tick": 273600
      },
      "usdc": {
        "address": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
        "amount": "1.225999",
        "decimals": 6,
        "raw": "1225999",
        "to_enter": "1.22"
      },
      "would_sign": "In the Uniswap web app, with your own wallet: one new position in the USDC/NOCK 3% v4 pool, ticks 273600 to 273900, holding up to 86.7890 NOCK and about $1.23 USDC (the app may first ask for USDC and NOCK approvals). NockNock builds no transaction and signs nothing."
    },
    {
      "at_bottom": {
        "avg_buy_price_usd": "0.01182220",
        "nock_bought": "9716.0065",
        "nock_held": "9716.0065"
      },
      "bottom": {
        "pct_of_spot": "-15.27",
        "price_usd": "0.01096801",
        "tick": 275400
      },
      "bounds_nock_per_usdc": {
        "high": "91.1742",
        "low": "78.4750"
      },
      "bounds_usd": {
        "high": "0.01274292",
        "low": "0.01096801"
      },
      "cap": null,
      "deep_link": {
        "amount_to_enter": "Amount to enter in the app: $114.86 USDC",
        "format_note": "The link format is the app's own and undocumented: check the pool and range in the app against the figures here.",
        "host": "app.uniswap.org",
        "not_prefilled": [
          "amount"
        ],
        "prefilled": [
          "pair",
          "fee and tick spacing",
          "range ticks"
        ],
        "reason": null,
        "url": "https://app.uniswap.org/positions/add/new?chain=base&protocolVersion=v4&currencyA=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913&currencyB=0x9B5E262cF9bb04869ab40b19AF91D2dc85761722&fee=%7B%22feeAmount%22%3A30000%2C%22isDynamic%22%3Afalse%2C%22tickSpacing%22%3A300%7D&priceRangeState=%7B%22fullRange%22%3Afalse%2C%22maxTick%22%3A275400%2C%22minTick%22%3A273900%2C%22priceInverted%22%3Afalse%7D&step=1"
      },
      "fee_estimate": {
        "coverage": {
          "note": "the whole range lies inside the measuring range",
          "whole": true
        },
        "in_range_share_reason": null,
        "in_range_share_stored_history": {
          "of_seconds": "547205",
          "seconds": "156662",
          "share_pct": "28.63"
        },
        "intervals": 1836,
        "measured_at": "2026-11-20T11:50:00Z",
        "measured_by": "uniswap-v4:base:3163450",
        "measured_days": "6.33",
        "measured_label": "v4 USDC/NOCK 3.00% #3163450",
        "per_day_in_range_diluted_usd": "8.42",
        "per_day_in_range_usd": "8.56",
        "reason": null,
        "replay_reason": null,
        "status": "measured",
        "stored_history_replay": {
          "edge_upper_usd": "5.16",
          "inside_upper_usd": "6.39"
        }
      },
      "in_range_now": false,
      "index": 2,
      "mint": {
        "amount0_raw": "114864600",
        "amount1_raw": "0",
        "liquidity_raw": "1408301615002237"
      },
      "nock": {
        "address": "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722",
        "amount": "0.0000000000000000",
        "decimals": 16,
        "raw": "0",
        "to_enter": "0.0000"
      },
      "nock_limit": null,
      "of": 3,
      "pool": {
        "fee_raw": 30000,
        "fee_tier_bps": 300,
        "hooks": "0x0000000000000000000000000000000000000000",
        "label": "the USDC/NOCK 3% v4 pool",
        "network": "base",
        "new_position": true,
        "note": "a NEW position (a new NFT) in this pool; position_id is the tracked position the pool was read from",
        "npm_address": null,
        "pool_id": "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13",
        "pool_price_usd": "0.01294400",
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
        }
      },
      "rung": "near",
      "status": "to_deposit",
      "summary": "$114.86 USDC, no NOCK, below the price now",
      "target": {
        "counterparts": [],
        "deployed_usdc": "0.00",
        "usdc": "114.86"
      },
      "tick_lower": 273900,
      "tick_upper": 275400,
      "top": {
        "pct_of_spot": "-1.55",
        "price_usd": "0.01274292",
        "rule": "the usable tick at or below the pool price (the first rung still drawable)",
        "tick": 273900
      },
      "usdc": {
        "address": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
        "amount": "114.864600",
        "decimals": 6,
        "raw": "114864600",
        "to_enter": "114.86"
      },
      "would_sign": "In the Uniswap web app, with your own wallet: one new position in the USDC/NOCK 3% v4 pool, ticks 273900 to 275400, holding up to $114.86 USDC and 0 NOCK (the app may first ask for a USDC approval). NockNock builds no transaction and signs nothing."
    }
  ],
  "sentence": "Your ladder shape in the USDC/NOCK 3% v4 pool: 2 of 3 deposits are left (1 deployed), holding up to $116.09 USDC and 86.79 NOCK now: sized to leave the stable balance at or above your $950 reserve at $0.01. Fully converted at $0.01097 they would hold about 9,898 NOCK, about 9,811 of it bought, at an average of $0.01183.",
  "shape": {
    "deployed": 1,
    "left": 2,
    "none": 0,
    "of": 3,
    "over": 0,
    "passed": 0,
    "rungs": [
      {
        "bounds_usd": {
          "high": "0.01313098",
          "low": "0.01274292"
        },
        "counterparts": [],
        "deployed_nock": "0.0000",
        "deployed_usdc": "0.00",
        "holds_now": null,
        "in_range_now": true,
        "index": 1,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "86.7891",
        "remaining_usdc": "1.23",
        "rung": "cap",
        "status": "to_deposit",
        "target_nock": "86.7891",
        "target_usdc": "1.23",
        "text": "Deposit 1 of 3 (cap): left to deposit, about 86.79 NOCK + $1.23 USDC.",
        "tick_lower": 273600,
        "tick_upper": 273900
      },
      {
        "bounds_usd": {
          "high": "0.01274292",
          "low": "0.01096801"
        },
        "counterparts": [],
        "deployed_nock": null,
        "deployed_usdc": "0.00",
        "holds_now": null,
        "in_range_now": false,
        "index": 2,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": null,
        "remaining_usdc": "114.86",
        "rung": "near",
        "status": "to_deposit",
        "target_nock": null,
        "target_usdc": "114.86",
        "text": "Deposit 2 of 3 (near rung): left to deposit, $114.86 USDC.",
        "tick_lower": 273900,
        "tick_upper": 275400
      },
      {
        "bounds_usd": {
          "high": "0.01096801",
          "low": "0.01002405"
        },
        "counterparts": [
          "uniswap-v4:base:9000001"
        ],
        "deployed_nock": null,
        "deployed_usdc": "650.90",
        "holds_now": {
          "nock": "0.0000",
          "usdc": "650.90"
        },
        "in_range_now": false,
        "index": 3,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": null,
        "remaining_usdc": "0.00",
        "rung": "deep",
        "status": "deployed",
        "target_nock": null,
        "target_usdc": "650.90",
        "text": "Deposit 3 of 3 (deep rung): done. It is deployed in v4 USDC/NOCK 3.00% #9000001, which holds $650.90 USDC and 0 NOCK now.",
        "tick_lower": 275400,
        "tick_upper": 276300
      }
    ],
    "source": "operator_profile.ladder_rungs",
    "status": "computed",
    "total_usdc": "766.99",
    "tracked_usdc": "650.90",
    "unmatched": []
  },
  "shown": true,
  "stated": {
    "accumulation_plan_unspent": null,
    "cash_target_usd": null,
    "existing_ranges_below_bottom": null,
    "lines": [
      "No accumulation plan is configured.",
      "Tax reserve: $0.00 (stated, not a gate).",
      "No cash target is set."
    ],
    "note": "The plan budget, the tax reserve and the cash target are stated, not gates: only the standing reserve gates the ladder's USDC.",
    "tax_reserve_usd": {
      "is": "stated, not a gate",
      "usd": "0.00"
    }
  },
  "statement": "A measurement from this cycle's reads of what your ladder shape would do. NockNock does not choose the shape, the bottom or the amounts, builds no transaction, and signs and sends nothing.",
  "status": "ready",
  "tier": "top",
  "token": "NOCK",
  "tracked": [
    {
      "label": "v4 USDC/NOCK 3.00% #9000001",
      "position_id": "uniswap-v4:base:9000001",
      "rung": "deep",
      "state": "holds_only_usdc"
    }
  ],
  "wallet_after": {
    "cash_reserve_usdc": "950.00",
    "deposits_usdc": "116.09",
    "existing_ranges_usdc_at_gate": "171.15",
    "existing_ranges_usdc_now": "874.61",
    "gate_price_usd": "0.01000000",
    "rule": "the standing reserve is measured on wallet USDC plus USDC inside NOCK ranges, read at the gate price",
    "stable_at_gate_usdc": "950.00",
    "text": "After these deposits your wallet holds about $779 USDC; the $950 reserve counts the $875 of USDC inside your existing NOCK ranges. At $0.01 with every rung converted, the stable balance reads $950.00: the wallet's $778.85 plus $171.15 still inside those ranges.",
    "wallet_alone_below_reserve": true,
    "wallet_usdc_after": "778.85",
    "wallet_usdc_now": "894.94"
  },
  "what_changed": "Your ladder shape has 2 of 3 deposits left in the USDC/NOCK 3% v4 pool: up to $116.09 USDC and about 87 NOCK."
} as any;

/** cap_and_near_deployed: the cap and the near rung deployed: one deposit left */
export const LADDER_BOTTLE_CAP_NEAR_DEPLOYED: LadderSuggestion = {
  "after_deposit": {
    "detection_note": null,
    "detection_status": null,
    "kind": "tracked_automatically",
    "text": "After you deposit there is nothing to copy or configure. NockNock looks for the wallet's new position NFT on-chain at the start of every cycle (CHECK NOW runs one at once), so a deposit is normally found in the first cycle after it lands. A position your wallet minted in this pool with exactly a rung's ticks is tracked as a ladder by itself, with its mint as the baseline. Any other range it finds is shown as found on-chain and is not counted until one tap says how to track it."
  },
  "assumptions": [
    "The amounts are your own shape (operator_profile.ladder_rungs) sized to the most the standing reserve allows at the gate price; fees and gas are not counted in them.",
    "A cap is in range now and holds NOCK; every other rung holds only USDC now and earns no fees until NOCK falls to its top.",
    "Rungs already tracked are part of the shape: their USDC counts toward its total, and only what is left is listed.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "Each fee line is a past measurement on our own tracked range scaled to the rung's liquidity: an estimate, not a forecast."
  ],
  "candidates": null,
  "excluded": [],
  "freshness_rule": "Shown only while the snapshot is fresh: the dashboard's sync status is not stale, or the worker's health is ok.",
  "gates": {
    "bottom_setting": {
      "key": "operator_profile.ladder_bottom_usd",
      "status": "set",
      "value": "0.01"
    },
    "floor_breach_effect": "none",
    "ladder_tracked": {
      "reason": "tracked ladder rungs are part of your shape: each is matched to a rung, and only what is left is listed",
      "status": "counted"
    },
    "nock_budget": {
      "lp_nock_budget": "0.0000",
      "nock_needed": "0.0000",
      "reason": null,
      "status": "fits"
    },
    "reserve": {
      "binding": "reserve",
      "cash_reserve_usdc": "950.00",
      "key": "operator_profile.cash_reserve_usdc",
      "margin_usdc": "0.00",
      "max_usdc": "650.90",
      "minimum_usdc": "1.00",
      "read_at_rule": "your ladder bottom (operator_profile.ladder_bottom_usd $0.01000000), below the deepest rung's bottom",
      "read_at_usd": "0.01000000",
      "rule": "the stable balance at the gate price stays at or above the reserve",
      "shape_total_usdc": "766.99",
      "stable_at_gate_usdc": "950.00",
      "status": "allows"
    },
    "shape_setting": {
      "key": "operator_profile.ladder_rungs",
      "status": "set"
    }
  },
  "headline": "Ladder: 1 of 3 deposits left, up to $650.90 USDC.",
  "id": "ladder_suggestion:NOCK",
  "minimum_usdc": "1.00",
  "mint_note": "liquidity_raw is what these amounts buy, rounded down (LiquidityAmounts); amount0_raw and amount1_raw are what a mint of it pulls, rounded up (SqrtPriceMath), never more than the maxima (the rung's USDC and NOCK truncated to raw units)",
  "missing": [],
  "numbers": {
    "avg_buy_price_usd": "0.01048542",
    "binding": "reserve",
    "cash_reserve_usdc": "950.00",
    "existing_ranges_usdc_at_gate": "171.15",
    "existing_ranges_usdc_now": "339.80",
    "gate_price_usd": "0.01000000",
    "lp_nock_budget": "0.0000",
    "nock_at_bottom": "62076.6389",
    "nock_bought_at_bottom": "62076.6389",
    "nock_now": "0.0000",
    "pool_price_usd": "0.01294400",
    "shape_total_usdc": "766.99",
    "shape_tracked_usdc": "116.09",
    "stable_at_gate_usdc": "950.00",
    "usdc_max": "650.90",
    "wallet_usdc": "1429.75",
    "wallet_usdc_after": "778.85"
  },
  "outcomes": [
    {
      "avg_buy_price_usd": "0.01048542",
      "kind": "fall_to_bottom",
      "nock_bought": "62077",
      "pct_of_spot": "-23%",
      "price_usd": "0.01002405",
      "text": "A fall to $0.01002 (-23%), the bottom of what is left to deposit, would buy about 62,077 NOCK in total with all $650.90, an average of $0.01049.",
      "usdc_spent": "650.90"
    },
    {
      "kind": "rise",
      "text": "A rise sells nothing: these deposits hold 0 NOCK, so their USDC stays USDC."
    }
  ],
  "picture": {
    "bars": [
      {
        "deployed": true,
        "high_usd": "0.01313098",
        "index": 1,
        "label": "$1.23 + 87 NOCK",
        "low_usd": "0.01274292",
        "rung": "cap",
        "status": "deployed",
        "usdc": "1.23"
      },
      {
        "deployed": true,
        "high_usd": "0.01274292",
        "index": 2,
        "label": "$114.86",
        "low_usd": "0.01096801",
        "rung": "near",
        "status": "deployed",
        "usdc": "114.86"
      },
      {
        "deployed": false,
        "high_usd": "0.01096801",
        "index": 3,
        "label": "$650.89",
        "low_usd": "0.01002405",
        "rung": "deep",
        "status": "to_deposit",
        "usdc": "650.89"
      }
    ],
    "bottom_requested_usd": "0.01",
    "bottom_usd": "0.01002405",
    "reserve_label": "$950 kept for below $0.01",
    "reserve_usdc": "950.00",
    "spot_usd": "0.01294400"
  },
  "reason": null,
  "rungs": [
    {
      "at_bottom": {
        "avg_buy_price_usd": "0.01048542",
        "nock_bought": "62076.6389",
        "nock_held": "62076.6389"
      },
      "bottom": {
        "pct_of_spot": "-22.56",
        "price_usd": "0.01002405",
        "tick": 276300
      },
      "bounds_nock_per_usdc": {
        "high": "99.7600",
        "low": "91.1742"
      },
      "bounds_usd": {
        "high": "0.01096801",
        "low": "0.01002405"
      },
      "cap": null,
      "deep_link": {
        "amount_to_enter": "Amount to enter in the app: $650.89 USDC",
        "format_note": "The link format is the app's own and undocumented: check the pool and range in the app against the figures here.",
        "host": "app.uniswap.org",
        "not_prefilled": [
          "amount"
        ],
        "prefilled": [
          "pair",
          "fee and tick spacing",
          "range ticks"
        ],
        "reason": null,
        "url": "https://app.uniswap.org/positions/add/new?chain=base&protocolVersion=v4&currencyA=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913&currencyB=0x9B5E262cF9bb04869ab40b19AF91D2dc85761722&fee=%7B%22feeAmount%22%3A30000%2C%22isDynamic%22%3Afalse%2C%22tickSpacing%22%3A300%7D&priceRangeState=%7B%22fullRange%22%3Afalse%2C%22maxTick%22%3A276300%2C%22minTick%22%3A275400%2C%22priceInverted%22%3Afalse%7D&step=1"
      },
      "fee_estimate": {
        "coverage": {
          "note": "the whole range lies inside the measuring range",
          "whole": true
        },
        "in_range_share_reason": null,
        "in_range_share_stored_history": {
          "of_seconds": "547205",
          "seconds": "31832",
          "share_pct": "5.82"
        },
        "intervals": 1836,
        "measured_at": "2026-11-20T11:50:00Z",
        "measured_by": "uniswap-v4:base:3163450",
        "measured_days": "6.33",
        "measured_label": "v4 USDC/NOCK 3.00% #3163450",
        "per_day_in_range_diluted_usd": "73.03",
        "per_day_in_range_usd": "85.90",
        "reason": null,
        "replay_reason": null,
        "status": "measured",
        "stored_history_replay": {
          "edge_upper_usd": "50.00",
          "inside_upper_usd": "32.55"
        }
      },
      "in_range_now": false,
      "index": 3,
      "mint": {
        "amount0_raw": "650899400",
        "amount1_raw": "0",
        "liquidity_raw": "14125167068081104"
      },
      "nock": {
        "address": "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722",
        "amount": "0.0000000000000000",
        "decimals": 16,
        "raw": "0",
        "to_enter": "0.0000"
      },
      "nock_limit": null,
      "of": 3,
      "pool": {
        "fee_raw": 30000,
        "fee_tier_bps": 300,
        "hooks": "0x0000000000000000000000000000000000000000",
        "label": "the USDC/NOCK 3% v4 pool",
        "network": "base",
        "new_position": true,
        "note": "a NEW position (a new NFT) in this pool; position_id is the tracked position the pool was read from",
        "npm_address": null,
        "pool_id": "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13",
        "pool_price_usd": "0.01294400",
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
        }
      },
      "rung": "deep",
      "status": "to_deposit",
      "summary": "$650.89 USDC, no NOCK, below the price now",
      "target": {
        "counterparts": [],
        "deployed_usdc": "0.00",
        "usdc": "650.90"
      },
      "tick_lower": 275400,
      "tick_upper": 276300,
      "top": {
        "pct_of_spot": "-15.27",
        "price_usd": "0.01096801",
        "rule": "the nearest usable tick",
        "tick": 275400
      },
      "usdc": {
        "address": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
        "amount": "650.899400",
        "decimals": 6,
        "raw": "650899400",
        "to_enter": "650.89"
      },
      "would_sign": "In the Uniswap web app, with your own wallet: one new position in the USDC/NOCK 3% v4 pool, ticks 275400 to 276300, holding up to $650.89 USDC and 0 NOCK (the app may first ask for a USDC approval). NockNock builds no transaction and signs nothing."
    }
  ],
  "sentence": "Your ladder shape in the USDC/NOCK 3% v4 pool: 1 of 3 deposits is left (2 deployed), holding up to $650.90 USDC and 0 NOCK now: sized to leave the stable balance at or above your $950 reserve at $0.01. Fully converted at $0.01002 they would hold about 62,077 NOCK, about 62,077 of it bought, at an average of $0.01049.",
  "shape": {
    "deployed": 2,
    "left": 1,
    "none": 0,
    "of": 3,
    "over": 0,
    "passed": 0,
    "rungs": [
      {
        "bounds_usd": {
          "high": "0.01313098",
          "low": "0.01274292"
        },
        "counterparts": [
          "uniswap-v4:base:9000002"
        ],
        "deployed_nock": "86.7891",
        "deployed_usdc": "1.23",
        "holds_now": {
          "nock": "86.7891",
          "usdc": "1.23"
        },
        "in_range_now": true,
        "index": 1,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "0.0000",
        "remaining_usdc": "0.00",
        "rung": "cap",
        "status": "deployed",
        "target_nock": "86.7891",
        "target_usdc": "1.23",
        "text": "Deposit 1 of 3 (cap): done. It is deployed in v4 USDC/NOCK 3.00% #9000002, which holds $1.23 USDC and 87 NOCK now.",
        "tick_lower": 273600,
        "tick_upper": 273900
      },
      {
        "bounds_usd": {
          "high": "0.01274292",
          "low": "0.01096801"
        },
        "counterparts": [
          "uniswap-v4:base:9000003"
        ],
        "deployed_nock": null,
        "deployed_usdc": "114.86",
        "holds_now": {
          "nock": "0.0000",
          "usdc": "114.86"
        },
        "in_range_now": false,
        "index": 2,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": null,
        "remaining_usdc": "0.00",
        "rung": "near",
        "status": "deployed",
        "target_nock": null,
        "target_usdc": "114.86",
        "text": "Deposit 2 of 3 (near rung): done. It is deployed in v4 USDC/NOCK 3.00% #9000003, which holds $114.86 USDC and 0 NOCK now.",
        "tick_lower": 273900,
        "tick_upper": 275400
      },
      {
        "bounds_usd": {
          "high": "0.01096801",
          "low": "0.01002405"
        },
        "counterparts": [],
        "deployed_nock": null,
        "deployed_usdc": "0.00",
        "holds_now": null,
        "in_range_now": false,
        "index": 3,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": null,
        "remaining_usdc": "650.90",
        "rung": "deep",
        "status": "to_deposit",
        "target_nock": null,
        "target_usdc": "650.90",
        "text": "Deposit 3 of 3 (deep rung): left to deposit, $650.90 USDC.",
        "tick_lower": 275400,
        "tick_upper": 276300
      }
    ],
    "source": "operator_profile.ladder_rungs",
    "status": "computed",
    "total_usdc": "766.99",
    "tracked_usdc": "116.09",
    "unmatched": []
  },
  "shown": true,
  "stated": {
    "accumulation_plan_unspent": null,
    "cash_target_usd": null,
    "existing_ranges_below_bottom": null,
    "lines": [
      "No accumulation plan is configured.",
      "Tax reserve: $0.00 (stated, not a gate).",
      "No cash target is set."
    ],
    "note": "The plan budget, the tax reserve and the cash target are stated, not gates: only the standing reserve gates the ladder's USDC.",
    "tax_reserve_usd": {
      "is": "stated, not a gate",
      "usd": "0.00"
    }
  },
  "statement": "A measurement from this cycle's reads of what your ladder shape would do. NockNock does not choose the shape, the bottom or the amounts, builds no transaction, and signs and sends nothing.",
  "status": "ready",
  "tier": "top",
  "token": "NOCK",
  "tracked": [
    {
      "label": "v4 USDC/NOCK 3.00% #9000002",
      "position_id": "uniswap-v4:base:9000002",
      "rung": "cap",
      "state": "in_range"
    },
    {
      "label": "v4 USDC/NOCK 3.00% #9000003",
      "position_id": "uniswap-v4:base:9000003",
      "rung": "near",
      "state": "holds_only_usdc"
    }
  ],
  "wallet_after": {
    "cash_reserve_usdc": "950.00",
    "deposits_usdc": "650.90",
    "existing_ranges_usdc_at_gate": "171.15",
    "existing_ranges_usdc_now": "339.80",
    "gate_price_usd": "0.01000000",
    "rule": "the standing reserve is measured on wallet USDC plus USDC inside NOCK ranges, read at the gate price",
    "stable_at_gate_usdc": "950.00",
    "text": "After these deposits your wallet holds about $779 USDC; the $950 reserve counts the $340 of USDC inside your existing NOCK ranges. At $0.01 with every rung converted, the stable balance reads $950.00: the wallet's $778.85 plus $171.15 still inside those ranges.",
    "wallet_alone_below_reserve": true,
    "wallet_usdc_after": "778.85",
    "wallet_usdc_now": "1429.75"
  },
  "what_changed": "Your ladder shape has 1 of 3 deposits left in the USDC/NOCK 3% v4 pool: up to $650.90 USDC."
} as any;

/** all_deployed: every rung deployed */
export const LADDER_BOTTLE_ALL_DEPLOYED: LadderSuggestion = {
  "after_deposit": null,
  "assumptions": [
    "The amounts are your own shape (operator_profile.ladder_rungs) sized to the most the standing reserve allows at the gate price; fees and gas are not counted in them.",
    "A cap is in range now and holds NOCK; every other rung holds only USDC now and earns no fees until NOCK falls to its top.",
    "Rungs already tracked are part of the shape: their USDC counts toward its total, and only what is left is listed.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "Each fee line is a past measurement on our own tracked range scaled to the rung's liquidity: an estimate, not a forecast."
  ],
  "candidates": null,
  "excluded": [],
  "freshness_rule": "Shown only while the snapshot is fresh: the dashboard's sync status is not stale, or the worker's health is ok.",
  "gates": null,
  "headline": "Ladder deployed: all 3 deposits are done.",
  "id": "ladder_suggestion:NOCK",
  "minimum_usdc": "1.00",
  "mint_note": null,
  "missing": [],
  "numbers": null,
  "outcomes": [],
  "picture": null,
  "reason": "Nothing is left to deposit: of the 3 rungs of your shape, 3 deployed. Their readings are in the Ladders card.",
  "rungs": [],
  "sentence": null,
  "shape": {
    "deployed": 3,
    "left": 0,
    "none": 0,
    "of": 3,
    "over": 0,
    "passed": 0,
    "rungs": [
      {
        "bounds_usd": {
          "high": "0.01313098",
          "low": "0.01274292"
        },
        "counterparts": [
          "uniswap-v4:base:9000002"
        ],
        "deployed_nock": "86.7891",
        "deployed_usdc": "1.23",
        "holds_now": {
          "nock": "86.7891",
          "usdc": "1.23"
        },
        "in_range_now": true,
        "index": 1,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "0.0000",
        "remaining_usdc": "0.00",
        "rung": "cap",
        "status": "deployed",
        "target_nock": "86.7891",
        "target_usdc": "1.23",
        "text": "Deposit 1 of 3 (cap): done. It is deployed in v4 USDC/NOCK 3.00% #9000002, which holds $1.23 USDC and 87 NOCK now.",
        "tick_lower": 273600,
        "tick_upper": 273900
      },
      {
        "bounds_usd": {
          "high": "0.01274292",
          "low": "0.01096801"
        },
        "counterparts": [
          "uniswap-v4:base:9000003"
        ],
        "deployed_nock": null,
        "deployed_usdc": "114.86",
        "holds_now": {
          "nock": "0.0000",
          "usdc": "114.86"
        },
        "in_range_now": false,
        "index": 2,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": null,
        "remaining_usdc": "0.00",
        "rung": "near",
        "status": "deployed",
        "target_nock": null,
        "target_usdc": "114.86",
        "text": "Deposit 2 of 3 (near rung): done. It is deployed in v4 USDC/NOCK 3.00% #9000003, which holds $114.86 USDC and 0 NOCK now.",
        "tick_lower": 273900,
        "tick_upper": 275400
      },
      {
        "bounds_usd": {
          "high": "0.01096801",
          "low": "0.01002405"
        },
        "counterparts": [
          "uniswap-v4:base:9000001"
        ],
        "deployed_nock": null,
        "deployed_usdc": "650.90",
        "holds_now": {
          "nock": "0.0000",
          "usdc": "650.90"
        },
        "in_range_now": false,
        "index": 3,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": null,
        "remaining_usdc": "0.00",
        "rung": "deep",
        "status": "deployed",
        "target_nock": null,
        "target_usdc": "650.90",
        "text": "Deposit 3 of 3 (deep rung): done. It is deployed in v4 USDC/NOCK 3.00% #9000001, which holds $650.90 USDC and 0 NOCK now.",
        "tick_lower": 275400,
        "tick_upper": 276300
      }
    ],
    "source": "operator_profile.ladder_rungs",
    "status": "deployed",
    "total_usdc": "766.99",
    "tracked_usdc": "766.99",
    "unmatched": []
  },
  "shown": false,
  "stated": null,
  "statement": "A measurement from this cycle's reads of what your ladder shape would do. NockNock does not choose the shape, the bottom or the amounts, builds no transaction, and signs and sends nothing.",
  "status": "ladder_deployed",
  "tier": "top",
  "token": "NOCK",
  "tracked": [
    {
      "label": "v4 USDC/NOCK 3.00% #9000002",
      "position_id": "uniswap-v4:base:9000002",
      "rung": "cap",
      "state": "in_range"
    },
    {
      "label": "v4 USDC/NOCK 3.00% #9000003",
      "position_id": "uniswap-v4:base:9000003",
      "rung": "near",
      "state": "holds_only_usdc"
    },
    {
      "label": "v4 USDC/NOCK 3.00% #9000001",
      "position_id": "uniswap-v4:base:9000001",
      "rung": "deep",
      "state": "holds_only_usdc"
    }
  ],
  "wallet_after": null,
  "what_changed": null
} as any;

/** near_passed: the price below the near rung: it is passed */
export const LADDER_BOTTLE_NEAR_PASSED: LadderSuggestion = {
  "after_deposit": {
    "detection_note": null,
    "detection_status": null,
    "kind": "tracked_automatically",
    "text": "After you deposit there is nothing to copy or configure. NockNock looks for the wallet's new position NFT on-chain at the start of every cycle (CHECK NOW runs one at once), so a deposit is normally found in the first cycle after it lands. A position your wallet minted in this pool with exactly a rung's ticks is tracked as a ladder by itself, with its mint as the baseline. Any other range it finds is shown as found on-chain and is not counted until one tap says how to track it."
  },
  "assumptions": [
    "The amounts are your own shape (operator_profile.ladder_rungs) sized to the most the standing reserve allows at the gate price; fees and gas are not counted in them.",
    "A cap is in range now and holds NOCK; every other rung holds only USDC now and earns no fees until NOCK falls to its top.",
    "Rungs already tracked are part of the shape: their USDC counts toward its total, and only what is left is listed.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "Each fee line is a past measurement on our own tracked range scaled to the rung's liquidity: an estimate, not a forecast."
  ],
  "candidates": null,
  "excluded": [],
  "freshness_rule": "Shown only while the snapshot is fresh: the dashboard's sync status is not stale, or the worker's health is ok.",
  "gates": {
    "bottom_setting": {
      "key": "operator_profile.ladder_bottom_usd",
      "status": "set",
      "value": "0.01"
    },
    "floor_breach_effect": "lowers",
    "ladder_tracked": {
      "reason": "tracked ladder rungs are part of your shape: each is matched to a rung, and only what is left is listed",
      "status": "none"
    },
    "nock_budget": {
      "lp_nock_budget": "86.7891",
      "nock_needed": "15.0223",
      "reason": null,
      "status": "fits"
    },
    "reserve": {
      "binding": "reserve",
      "cash_reserve_usdc": "950.00",
      "key": "operator_profile.cash_reserve_usdc",
      "margin_usdc": "100.94",
      "max_usdc": "672.92",
      "minimum_usdc": "1.00",
      "read_at_rule": "your ladder bottom (operator_profile.ladder_bottom_usd $0.01000000), below the deepest rung's bottom",
      "read_at_usd": "0.01000000",
      "rule": "the stable balance at the gate price stays at or above the reserve",
      "shape_total_usdc": "773.86",
      "stable_at_gate_usdc": "1050.94",
      "status": "allows"
    },
    "shape_setting": {
      "key": "operator_profile.ladder_rungs",
      "status": "set"
    }
  },
  "headline": "Ladder: 2 of 3 deposits left, up to $672.92 USDC and about 15 NOCK.",
  "id": "ladder_suggestion:NOCK",
  "minimum_usdc": "1.00",
  "mint_note": "liquidity_raw is what these amounts buy, rounded down (LiquidityAmounts); amount0_raw and amount1_raw are what a mint of it pulls, rounded up (SqrtPriceMath), never more than the maxima (the rung's USDC and NOCK truncated to raw units)",
  "missing": [],
  "numbers": {
    "avg_buy_price_usd": "0.01057777",
    "binding": "reserve",
    "cash_reserve_usdc": "950.00",
    "existing_ranges_usdc_at_gate": "178.03",
    "existing_ranges_usdc_now": "202.53",
    "gate_price_usd": "0.01000000",
    "lp_nock_budget": "86.7891",
    "nock_at_bottom": "63631.5918",
    "nock_bought_at_bottom": "63616.5695",
    "nock_now": "15.0223",
    "pool_price_usd": "0.01130146",
    "shape_total_usdc": "773.86",
    "shape_tracked_usdc": "0.00",
    "stable_at_gate_usdc": "1050.94",
    "usdc_max": "672.92",
    "wallet_usdc": "1545.84",
    "wallet_usdc_after": "872.91"
  },
  "outcomes": [
    {
      "avg_buy_price_usd": "0.01057777",
      "kind": "fall_to_bottom",
      "nock_bought": "63617",
      "pct_of_spot": "-11%",
      "price_usd": "0.01002405",
      "text": "A fall to $0.01002 (-11%), the bottom of what is left to deposit, would buy about 63,617 NOCK in total with all $672.92, an average of $0.01058.",
      "usdc_spent": "672.92"
    },
    {
      "kind": "rise",
      "nock_sold": "15",
      "pct_of_spot": "0.0%",
      "price_usd": "0.01130202",
      "text": "A rise to $0.01130 (0.0%) or above sells the cap's NOCK, about 15, for about $0.17 of USDC, and sells nothing else: every other rung holds 0 NOCK.",
      "usdc_received": "0.17"
    }
  ],
  "picture": {
    "bars": [
      {
        "deployed": false,
        "high_usd": "0.01130202",
        "index": 1,
        "label": "$100.94 + 15 NOCK",
        "low_usd": "0.01096801",
        "rung": "cap",
        "status": "to_deposit",
        "usdc": "100.94"
      },
      {
        "deployed": false,
        "high_usd": "0.01096801",
        "index": 3,
        "label": "$571.98",
        "low_usd": "0.01002405",
        "rung": "deep",
        "status": "to_deposit",
        "usdc": "571.98"
      }
    ],
    "bottom_requested_usd": "0.01",
    "bottom_usd": "0.01002405",
    "reserve_label": "$950 kept for below $0.01",
    "reserve_usdc": "950.00",
    "spot_usd": "0.01130146"
  },
  "reason": null,
  "rungs": [
    {
      "at_bottom": {
        "avg_buy_price_usd": "0.01113349",
        "nock_bought": "9066.1861",
        "nock_held": "9081.2084"
      },
      "bottom": {
        "pct_of_spot": "-2.95",
        "price_usd": "0.01096801",
        "tick": 275400
      },
      "bounds_nock_per_usdc": {
        "high": "91.1742",
        "low": "88.4797"
      },
      "bounds_usd": {
        "high": "0.01130202",
        "low": "0.01096801"
      },
      "cap": {
        "bound_by": "near_usdc",
        "note": "the budget's 86.7891 NOCK would pair with $583.16 USDC, more than the near rung's $100.94: the cap's liquidity is reduced until its USDC equals that, so it holds 15.0223 NOCK, less than the budget"
      },
      "deep_link": {
        "amount_to_enter": "Amount to enter in the app: 15.0223 NOCK, in the NOCK field; the app then works out the USDC side, about $100.94 at this reading.",
        "format_note": "The link format is the app's own and undocumented: check the pool and range in the app against the figures here.",
        "host": "app.uniswap.org",
        "not_prefilled": [
          "amount"
        ],
        "prefilled": [
          "pair",
          "fee and tick spacing",
          "range ticks"
        ],
        "reason": null,
        "url": "https://app.uniswap.org/positions/add/new?chain=base&protocolVersion=v4&currencyA=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913&currencyB=0x9B5E262cF9bb04869ab40b19AF91D2dc85761722&fee=%7B%22feeAmount%22%3A30000%2C%22isDynamic%22%3Afalse%2C%22tickSpacing%22%3A300%7D&priceRangeState=%7B%22fullRange%22%3Afalse%2C%22maxTick%22%3A275400%2C%22minTick%22%3A275100%2C%22priceInverted%22%3Afalse%7D&step=1"
      },
      "fee_estimate": {
        "coverage": {
          "note": "the whole range lies inside the measuring range",
          "whole": true
        },
        "in_range_share_reason": null,
        "in_range_share_stored_history": {
          "of_seconds": "547205",
          "seconds": "92976",
          "share_pct": "16.99"
        },
        "intervals": 1836,
        "measured_at": "2026-11-20T11:50:00Z",
        "measured_by": "uniswap-v4:base:3163450",
        "measured_days": "6.33",
        "measured_label": "v4 USDC/NOCK 3.00% #3163450",
        "per_day_in_range_diluted_usd": "33.83",
        "per_day_in_range_usd": "36.52",
        "reason": null,
        "replay_reason": null,
        "status": "measured",
        "stored_history_replay": {
          "edge_upper_usd": "27.60",
          "inside_upper_usd": "5.82"
        }
      },
      "in_range_now": true,
      "index": 1,
      "mint": {
        "amount0_raw": "100938260",
        "amount1_raw": "150223084567448895",
        "liquidity_raw": "6388384477943076"
      },
      "nock": {
        "address": "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722",
        "amount": "15.0223084567448895",
        "decimals": 16,
        "raw": "150223084567448895",
        "to_enter": "15.0223"
      },
      "nock_limit": {
        "field_to_type": "NOCK",
        "is": "lp_nock_budget",
        "nock": "86.7891",
        "nock_to_enter": "15.0223",
        "text": "The NOCK side must not exceed 86.7891 NOCK, your LP NOCK budget. The limit holds when the NOCK field is the one typed, at 15.0223 NOCK or less. When the USDC field is typed instead, the app works out the NOCK side from the price at that moment, and it can come out above the limit; neither the link nor the app applies a NOCK limit of its own.",
        "usdc_then_about": "100.94"
      },
      "of": 3,
      "pool": {
        "fee_raw": 30000,
        "fee_tier_bps": 300,
        "hooks": "0x0000000000000000000000000000000000000000",
        "label": "the USDC/NOCK 3% v4 pool",
        "network": "base",
        "new_position": true,
        "note": "a NEW position (a new NFT) in this pool; position_id is the tracked position the pool was read from",
        "npm_address": null,
        "pool_id": "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13",
        "pool_price_usd": "0.01130146",
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
        }
      },
      "rung": "cap",
      "status": "to_deposit",
      "summary": "about 15.02 NOCK + $100.94 USDC, in range now",
      "target": {
        "counterparts": [],
        "deployed_usdc": "0.00",
        "usdc": "100.94"
      },
      "tick_lower": 275100,
      "tick_upper": 275400,
      "top": {
        "pct_of_spot": "0.00",
        "price_usd": "0.01130202",
        "rule": "the tick band containing the pool's tick (the cap)",
        "tick": 275100
      },
      "usdc": {
        "address": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
        "amount": "100.938260",
        "decimals": 6,
        "raw": "100938260",
        "to_enter": "100.93"
      },
      "would_sign": "In the Uniswap web app, with your own wallet: one new position in the USDC/NOCK 3% v4 pool, ticks 275100 to 275400, holding up to 15.0223 NOCK and about $100.94 USDC (the app may first ask for USDC and NOCK approvals). NockNock builds no transaction and signs nothing."
    },
    {
      "at_bottom": {
        "avg_buy_price_usd": "0.01048542",
        "nock_bought": "54550.3834",
        "nock_held": "54550.3834"
      },
      "bottom": {
        "pct_of_spot": "-11.30",
        "price_usd": "0.01002405",
        "tick": 276300
      },
      "bounds_nock_per_usdc": {
        "high": "99.7600",
        "low": "91.1742"
      },
      "bounds_usd": {
        "high": "0.01096801",
        "low": "0.01002405"
      },
      "cap": null,
      "deep_link": {
        "amount_to_enter": "Amount to enter in the app: $571.98 USDC",
        "format_note": "The link format is the app's own and undocumented: check the pool and range in the app against the figures here.",
        "host": "app.uniswap.org",
        "not_prefilled": [
          "amount"
        ],
        "prefilled": [
          "pair",
          "fee and tick spacing",
          "range ticks"
        ],
        "reason": null,
        "url": "https://app.uniswap.org/positions/add/new?chain=base&protocolVersion=v4&currencyA=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913&currencyB=0x9B5E262cF9bb04869ab40b19AF91D2dc85761722&fee=%7B%22feeAmount%22%3A30000%2C%22isDynamic%22%3Afalse%2C%22tickSpacing%22%3A300%7D&priceRangeState=%7B%22fullRange%22%3Afalse%2C%22maxTick%22%3A276300%2C%22minTick%22%3A275400%2C%22priceInverted%22%3Afalse%7D&step=1"
      },
      "fee_estimate": {
        "coverage": {
          "note": "the whole range lies inside the measuring range",
          "whole": true
        },
        "in_range_share_reason": null,
        "in_range_share_stored_history": {
          "of_seconds": "547205",
          "seconds": "31832",
          "share_pct": "5.82"
        },
        "intervals": 1836,
        "measured_at": "2026-11-20T11:50:00Z",
        "measured_by": "uniswap-v4:base:3163450",
        "measured_days": "6.33",
        "measured_label": "v4 USDC/NOCK 3.00% #3163450",
        "per_day_in_range_diluted_usd": "61.45",
        "per_day_in_range_usd": "70.97",
        "reason": null,
        "replay_reason": null,
        "status": "measured",
        "stored_history_replay": {
          "edge_upper_usd": "40.31",
          "inside_upper_usd": "26.85"
        }
      },
      "in_range_now": false,
      "index": 3,
      "mint": {
        "amount0_raw": "571983478",
        "amount1_raw": "0",
        "liquidity_raw": "12412612743124502"
      },
      "nock": {
        "address": "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722",
        "amount": "0.0000000000000000",
        "decimals": 16,
        "raw": "0",
        "to_enter": "0.0000"
      },
      "nock_limit": null,
      "of": 3,
      "pool": {
        "fee_raw": 30000,
        "fee_tier_bps": 300,
        "hooks": "0x0000000000000000000000000000000000000000",
        "label": "the USDC/NOCK 3% v4 pool",
        "network": "base",
        "new_position": true,
        "note": "a NEW position (a new NFT) in this pool; position_id is the tracked position the pool was read from",
        "npm_address": null,
        "pool_id": "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13",
        "pool_price_usd": "0.01130146",
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
        }
      },
      "rung": "deep",
      "status": "to_deposit",
      "summary": "$571.98 USDC, no NOCK, below the price now",
      "target": {
        "counterparts": [],
        "deployed_usdc": "0.00",
        "usdc": "571.98"
      },
      "tick_lower": 275400,
      "tick_upper": 276300,
      "top": {
        "pct_of_spot": "-2.95",
        "price_usd": "0.01096801",
        "rule": "the usable tick at or below the pool price (the first rung still drawable)",
        "tick": 275400
      },
      "usdc": {
        "address": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
        "amount": "571.983478",
        "decimals": 6,
        "raw": "571983478",
        "to_enter": "571.98"
      },
      "would_sign": "In the Uniswap web app, with your own wallet: one new position in the USDC/NOCK 3% v4 pool, ticks 275400 to 276300, holding up to $571.98 USDC and 0 NOCK (the app may first ask for a USDC approval). NockNock builds no transaction and signs nothing."
    }
  ],
  "sentence": "Your ladder shape in the USDC/NOCK 3% v4 pool: 2 of 3 deposits are left (1 passed), holding up to $672.92 USDC and 15.02 NOCK now: sized to leave the stable balance at or above your $950 reserve at $0.01. Fully converted at $0.01002 they would hold about 63,632 NOCK, about 63,617 of it bought, at an average of $0.01058.",
  "shape": {
    "deployed": 0,
    "left": 2,
    "none": 0,
    "of": 3,
    "over": 0,
    "passed": 1,
    "rungs": [
      {
        "bounds_usd": {
          "high": "0.01130202",
          "low": "0.01096801"
        },
        "counterparts": [],
        "deployed_nock": "0.0000",
        "deployed_usdc": "0.00",
        "holds_now": null,
        "in_range_now": true,
        "index": 1,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "15.0223",
        "remaining_usdc": "100.94",
        "rung": "cap",
        "status": "to_deposit",
        "target_nock": "15.0223",
        "target_usdc": "100.94",
        "text": "Deposit 1 of 3 (cap): left to deposit, about 15.02 NOCK + $100.94 USDC.",
        "tick_lower": 275100,
        "tick_upper": 275400
      },
      {
        "bounds_usd": null,
        "counterparts": [],
        "deployed_nock": null,
        "deployed_usdc": "0.00",
        "holds_now": null,
        "in_range_now": null,
        "index": 2,
        "of": 3,
        "over_by_usdc": null,
        "reason": "the price is below this rung's range; a USDC-only deposit there is not possible",
        "remaining_nock": null,
        "remaining_usdc": "100.94",
        "rung": "near",
        "status": "passed",
        "target_nock": null,
        "target_usdc": "100.94",
        "text": "Deposit 2 of 3 (near rung): passed. The price is below this rung's range; a USDC-only deposit there is not possible.",
        "tick_lower": null,
        "tick_upper": null
      },
      {
        "bounds_usd": {
          "high": "0.01096801",
          "low": "0.01002405"
        },
        "counterparts": [],
        "deployed_nock": null,
        "deployed_usdc": "0.00",
        "holds_now": null,
        "in_range_now": false,
        "index": 3,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": null,
        "remaining_usdc": "571.98",
        "rung": "deep",
        "status": "to_deposit",
        "target_nock": null,
        "target_usdc": "571.98",
        "text": "Deposit 3 of 3 (deep rung): left to deposit, $571.98 USDC.",
        "tick_lower": 275400,
        "tick_upper": 276300
      }
    ],
    "source": "operator_profile.ladder_rungs",
    "status": "computed",
    "total_usdc": "773.86",
    "tracked_usdc": "0.00",
    "unmatched": []
  },
  "shown": true,
  "stated": {
    "accumulation_plan_unspent": null,
    "cash_target_usd": null,
    "existing_ranges_below_bottom": null,
    "lines": [
      "No accumulation plan is configured.",
      "Tax reserve: $0.00 (stated, not a gate).",
      "No cash target is set."
    ],
    "note": "The plan budget, the tax reserve and the cash target are stated, not gates: only the standing reserve gates the ladder's USDC.",
    "tax_reserve_usd": {
      "is": "stated, not a gate",
      "usd": "0.00"
    }
  },
  "statement": "A measurement from this cycle's reads of what your ladder shape would do. NockNock does not choose the shape, the bottom or the amounts, builds no transaction, and signs and sends nothing.",
  "status": "ready",
  "tier": "top",
  "token": "NOCK",
  "tracked": [],
  "wallet_after": {
    "cash_reserve_usdc": "950.00",
    "deposits_usdc": "672.92",
    "existing_ranges_usdc_at_gate": "178.03",
    "existing_ranges_usdc_now": "202.53",
    "gate_price_usd": "0.01000000",
    "rule": "the standing reserve is measured on wallet USDC plus USDC inside NOCK ranges, read at the gate price",
    "stable_at_gate_usdc": "1050.94",
    "text": "After these deposits your wallet holds about $873 USDC; the $950 reserve counts the $203 of USDC inside your existing NOCK ranges. At $0.01 with every rung converted, the stable balance reads $1050.94: the wallet's $872.91 plus $178.03 still inside those ranges.",
    "wallet_alone_below_reserve": true,
    "wallet_usdc_after": "872.91",
    "wallet_usdc_now": "1545.84"
  },
  "what_changed": "Your ladder shape has 2 of 3 deposits left in the USDC/NOCK 3% v4 pool: up to $672.92 USDC and about 15 NOCK."
} as any;

/** deep_over: a tracked deep rung over its target: no deposit is listed */
export const LADDER_BOTTLE_OVER: LadderSuggestion = {
  "after_deposit": null,
  "assumptions": [
    "The amounts are your own shape (operator_profile.ladder_rungs) sized to the most the standing reserve allows at the gate price; fees and gas are not counted in them.",
    "A cap is in range now and holds NOCK; every other rung holds only USDC now and earns no fees until NOCK falls to its top.",
    "Rungs already tracked are part of the shape: their USDC counts toward its total, and only what is left is listed.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "Each fee line is a past measurement on our own tracked range scaled to the rung's liquidity: an estimate, not a forecast."
  ],
  "candidates": null,
  "excluded": [],
  "freshness_rule": "Shown only while the snapshot is fresh: the dashboard's sync status is not stale, or the worker's health is ok.",
  "gates": null,
  "headline": "Ladder: no USDC is free for one at the ladder bottom.",
  "id": "ladder_suggestion:NOCK",
  "minimum_usdc": "1.00",
  "mint_note": null,
  "missing": [],
  "numbers": null,
  "outcomes": [],
  "picture": null,
  "reason": "With what is left of your shape deposited, the stable balance at $0.01 would read $819.82, below your $950 reserve by $130.18: no deposit is listed. A tracked rung holds more than its target, and the excess is not taken from the other rungs.",
  "rungs": [],
  "sentence": null,
  "shape": {
    "deployed": 0,
    "left": 2,
    "none": 0,
    "of": 3,
    "over": 1,
    "passed": 0,
    "rungs": [
      {
        "bounds_usd": {
          "high": "0.01313098",
          "low": "0.01274292"
        },
        "counterparts": [],
        "deployed_nock": "0.0000",
        "deployed_usdc": "0.00",
        "holds_now": null,
        "in_range_now": true,
        "index": 1,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "86.7891",
        "remaining_usdc": "1.23",
        "rung": "cap",
        "status": "to_deposit",
        "target_nock": "86.7891",
        "target_usdc": "1.23",
        "text": "Deposit 1 of 3 (cap): left to deposit, about 86.79 NOCK + $1.23 USDC.",
        "tick_lower": 273600,
        "tick_upper": 273900
      },
      {
        "bounds_usd": {
          "high": "0.01274292",
          "low": "0.01096801"
        },
        "counterparts": [],
        "deployed_nock": null,
        "deployed_usdc": "0.00",
        "holds_now": null,
        "in_range_now": false,
        "index": 2,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": null,
        "remaining_usdc": "114.86",
        "rung": "near",
        "status": "to_deposit",
        "target_nock": null,
        "target_usdc": "114.86",
        "text": "Deposit 2 of 3 (near rung): left to deposit, $114.86 USDC.",
        "tick_lower": 273900,
        "tick_upper": 275400
      },
      {
        "bounds_usd": {
          "high": "0.01096801",
          "low": "0.01002405"
        },
        "counterparts": [
          "uniswap-v4:base:9000001"
        ],
        "deployed_nock": null,
        "deployed_usdc": "781.08",
        "holds_now": {
          "nock": "0.0000",
          "usdc": "781.08"
        },
        "in_range_now": false,
        "index": 3,
        "of": 3,
        "over_by_usdc": "130.18",
        "reason": "the tracked rung holds $130.18 USDC more than its target",
        "remaining_nock": null,
        "remaining_usdc": "0.00",
        "rung": "deep",
        "status": "over",
        "target_nock": null,
        "target_usdc": "650.90",
        "text": "Deposit 3 of 3 (deep rung): over its target, so nothing more is listed for it. The tracked rung holds $130.18 USDC more than its target.",
        "tick_lower": 275400,
        "tick_upper": 276300
      }
    ],
    "source": "operator_profile.ladder_rungs",
    "status": "computed",
    "total_usdc": "766.99",
    "tracked_usdc": "781.08",
    "unmatched": []
  },
  "shown": false,
  "stated": null,
  "statement": "A measurement from this cycle's reads of what your ladder shape would do. NockNock does not choose the shape, the bottom or the amounts, builds no transaction, and signs and sends nothing.",
  "status": "no_headroom",
  "tier": "top",
  "token": "NOCK",
  "tracked": [
    {
      "label": "v4 USDC/NOCK 3.00% #9000001",
      "position_id": "uniswap-v4:base:9000001",
      "rung": "deep",
      "state": "holds_only_usdc"
    }
  ],
  "wallet_after": null,
  "what_changed": null
} as any;

/** bottle: the text both MCP transports print for it (get_shot_advice). */
export const LADDER_BOTTLE_TEXT: string = "• Ladder suggestion (a measurement, not advice): Ladder: 3 of 3 deposits left, up to $766.99 USDC and about 87 NOCK. Your ladder shape in the USDC/NOCK 3% v4 pool: 3 of 3 deposits are left, holding up to $766.99 USDC and 86.79 NOCK now: sized to leave the stable balance at or above your $950 reserve at $0.01. Fully converted at $0.01002 they would hold about 71,975 NOCK, about 71,888 of it bought, at an average of $0.01067.\n  - A fall to $0.01097 (-15%), the near rung's bottom, would buy about 9,811 NOCK for $116.09 of the USDC, an average of $0.01183.\n  - A fall to the ladder bottom $0.01002 (-23%) would buy about 71,888 NOCK in total with all $766.99, an average of $0.01067.\n  - A rise to $0.01313 (+1.4%) or above sells the cap's NOCK, about 87, for about $1.13 of USDC, and sells nothing else: every other rung holds 0 NOCK.\n  Reserve $950.00 holds at $0.01000000 (the reserve binds); NOCK budget: fits.\n  After these deposits your wallet holds about $779 USDC; the $950 reserve counts the $224 of USDC inside your existing NOCK ranges. At $0.01 with every rung converted, the stable balance reads $950.00: the wallet's $778.85 plus $171.15 still inside those ranges.\n  Stated, not gates: No accumulation plan is configured. Tax reserve: $0.00 (stated, not a gate). No cash target is set.\n  Assumptions: The amounts are your own shape (operator_profile.ladder_rungs) sized to the most the standing reserve allows at the gate price; fees and gas are not counted in them. A cap is in range now and holds NOCK; every other rung holds only USDC now and earns no fees until NOCK falls to its top. Rungs already tracked are part of the shape: their USDC counts toward its total, and only what is left is listed. The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures. Each fee line is a past measurement on our own tracked range scaled to the rung's liquidity: an estimate, not a forecast.\n  Deposit 1 of 3 (cap): about 86.79 NOCK + $1.23 USDC, in range now. A new position in the USDC/NOCK 3% v4 pool, ticks 273600 to 273900 ($0.01274292 to $0.01313098; 76.1558 to 78.4750 NOCK per USDC).\n    Exact mint: liquidity 138192109253769, 1225999 raw USDC and 867890958577132038 raw NOCK.\n    Deposit link (pool and range prefilled): https://app.uniswap.org/positions/add/new?chain=base&protocolVersion=v4&currencyA=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913&currencyB=0x9B5E262cF9bb04869ab40b19AF91D2dc85761722&fee=%7B%22feeAmount%22%3A30000%2C%22isDynamic%22%3Afalse%2C%22tickSpacing%22%3A300%7D&priceRangeState=%7B%22fullRange%22%3Afalse%2C%22maxTick%22%3A273900%2C%22minTick%22%3A273600%2C%22priceInverted%22%3Afalse%7D&step=1\n    Amount to enter in the app: 86.7890 NOCK, in the NOCK field; the app then works out the USDC side, about $1.23 at this reading.\n    The NOCK side must not exceed 86.7891 NOCK, your LP NOCK budget. The limit holds when the NOCK field is the one typed, at 86.7890 NOCK or less. When the USDC field is typed instead, the app works out the NOCK side from the price at that moment, and it can come out above the limit; neither the link nor the app applies a NOCK limit of its own.\n    Fees while in range (an estimate): $0.84/day, measured over 6.33 days (1,836 intervals) on v4 USDC/NOCK 3.00% #3163450. In range 1.95% of the measured time. Replayed over it: at most $0.02 inside and $0.15 more at its edges (no lower bound).\n  Deposit 2 of 3 (near rung): $114.86 USDC, no NOCK, below the price now. A new position in the USDC/NOCK 3% v4 pool, ticks 273900 to 275400 ($0.01096801 to $0.01274292; 78.4750 to 91.1742 NOCK per USDC).\n    Exact mint: liquidity 1408301615002237, 114864600 raw USDC and 0 raw NOCK.\n    Deposit link (pool and range prefilled): https://app.uniswap.org/positions/add/new?chain=base&protocolVersion=v4&currencyA=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913&currencyB=0x9B5E262cF9bb04869ab40b19AF91D2dc85761722&fee=%7B%22feeAmount%22%3A30000%2C%22isDynamic%22%3Afalse%2C%22tickSpacing%22%3A300%7D&priceRangeState=%7B%22fullRange%22%3Afalse%2C%22maxTick%22%3A275400%2C%22minTick%22%3A273900%2C%22priceInverted%22%3Afalse%7D&step=1\n    Amount to enter in the app: $114.86 USDC\n    Fees while in range (an estimate): $8.56/day, measured over 6.33 days (1,836 intervals) on v4 USDC/NOCK 3.00% #3163450. In range 28.63% of the measured time. Replayed over it: at most $6.39 inside and $5.16 more at its edges (no lower bound).\n  Deposit 3 of 3 (deep rung): $650.89 USDC, no NOCK, below the price now. A new position in the USDC/NOCK 3% v4 pool, ticks 275400 to 276300 ($0.01002405 to $0.01096801; 91.1742 to 99.7600 NOCK per USDC).\n    Exact mint: liquidity 14125167068081104, 650899400 raw USDC and 0 raw NOCK.\n    Deposit link (pool and range prefilled): https://app.uniswap.org/positions/add/new?chain=base&protocolVersion=v4&currencyA=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913&currencyB=0x9B5E262cF9bb04869ab40b19AF91D2dc85761722&fee=%7B%22feeAmount%22%3A30000%2C%22isDynamic%22%3Afalse%2C%22tickSpacing%22%3A300%7D&priceRangeState=%7B%22fullRange%22%3Afalse%2C%22maxTick%22%3A276300%2C%22minTick%22%3A275400%2C%22priceInverted%22%3Afalse%7D&step=1\n    Amount to enter in the app: $650.89 USDC\n    Fees while in range (an estimate): $85.90/day, measured over 6.33 days (1,836 intervals) on v4 USDC/NOCK 3.00% #3163450. In range 5.82% of the measured time. Replayed over it: at most $32.55 inside and $50.00 more at its edges (no lower bound).\n  The link format is the app's own and undocumented: check the pool and range in the app against the figures here.\n  After you deposit: After you deposit there is nothing to copy or configure. NockNock looks for the wallet's new position NFT on-chain at the start of every cycle (CHECK NOW runs one at once), so a deposit is normally found in the first cycle after it lands. A position your wallet minted in this pool with exactly a rung's ticks is tracked as a ladder by itself, with its mint as the baseline. Any other range it finds is shown as found on-chain and is not counted until one tap says how to track it.\n  A measurement from this cycle's reads of what your ladder shape would do. NockNock does not choose the shape, the bottom or the amounts, builds no transaction, and signs and sends nothing.\n";
