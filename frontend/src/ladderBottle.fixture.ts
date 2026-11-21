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
    "The amounts are your own shape (operator_profile.ladder_rungs): until a near or deep rung is deposited it is sized to what the standing reserve allows at the gate price; from then on that rung fixes the plan. When the reserve allows less than what is left, every deposit is cut in proportion and the shortfall is stated. Fees and gas are not counted in them.",
    "A cap is in range now and holds NOCK; every other rung holds only USDC now and earns no fees until NOCK falls to its top.",
    "Each rung is measured by its liquidity, which does not change with the price: a deposited rung stays deposited when the price moves, and only what is left is listed. A remainder under $1.00 is not offered.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "Each fee line is a past measurement on our own tracked range scaled to the rung's liquidity: an estimate, not a forecast."
  ],
  "candidates": null,
  "excluded": [],
  "fit": {
    "allowed_usdc": "766.99",
    "needed_usdc": "766.99",
    "rule": "once a near or deep rung is deposited the cap takes only what the reserve leaves beside the near and deep rungs still to deposit; when they alone need more than the reserve allows, each deposit left is cut pro rata (by the same fraction), so the shape keeps its proportions, and the shortfall is stated",
    "shortfall_usdc": null,
    "status": "fits",
    "text": null
  },
  "found_note": null,
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
  "headline": "Ladder: 3 of 3 deposits left, up to $766.97 USDC and about 87 NOCK.",
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
    "headroom_usdc": "766.99",
    "lp_nock_budget": "86.7891",
    "nock_at_bottom": "71974.8945",
    "nock_bought_at_bottom": "71888.1054",
    "nock_now": "86.7891",
    "pool_price_usd": "0.01294400",
    "shape_rest_usdc": "765.76",
    "shape_total_usdc": "766.99",
    "shape_tracked_usdc": "0.00",
    "stable_at_gate_usdc": "950.02",
    "usdc_max": "766.97",
    "wallet_usdc": "1545.84",
    "wallet_usdc_after": "778.87"
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
      "text": "A fall to the ladder bottom $0.01002 (-23%) would buy about 71,888 NOCK in total with all $766.97, an average of $0.01067.",
      "usdc_spent": "766.97"
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
        "label": "$1.22 + 87 NOCK",
        "low_usd": "0.01274292",
        "rung": "cap",
        "status": "to_deposit",
        "usdc": "1.22"
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
        "note": "the cap holds the whole LP NOCK budget, 86.7891 NOCK; the curve pairs it with $1.23 USDC",
        "usdc_limit": "100.04",
        "usdc_limit_is": "near_share"
      },
      "deep_link": {
        "amount_to_enter": "Amount to enter in the app: 86.7890 NOCK, in the NOCK field; the app then works out the USDC side, about $1.22 at this reading.",
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
        "dilution": {
          "share_of_active_pct": "0.17"
        },
        "dilution_reason": null,
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
      "fit_factor": null,
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
        "binding": "nock",
        "field_to_type": "NOCK",
        "holds_less_than_budget": false,
        "is": "lp_nock_budget",
        "nock": "86.7891",
        "nock_to_enter": "86.7890",
        "text": "The NOCK side must not exceed 86.7891 NOCK, your LP NOCK budget, which binds now; the USDC side must not exceed $100.04, the most that keeps it at or below the near rung's USDC. The limits hold when the NOCK field is the one typed, at 86.7890 NOCK or less, and the app shows at most $100.04 of USDC: about $1.22 at this reading, more if the price rises inside the band before you deposit. When the USDC field is typed instead, the app works out the NOCK side from the price at that moment, and it can come out above the limit; neither the link nor the app applies a NOCK limit of its own.",
        "usdc": "100.04",
        "usdc_then_about": "1.22"
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
      "summary": "about 86.79 NOCK + $1.22 USDC, in range now",
      "target": {
        "counterparts": [],
        "deployed_usdc": "0.00",
        "liquidity": "138192115820500",
        "tracked_liquidity": "0",
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
      "would_sign": "In the Uniswap web app, with your own wallet: one new position in the USDC/NOCK 3% v4 pool, ticks 273600 to 273900, holding up to 86.7890 NOCK and about $1.22 USDC (the app may first ask for USDC and NOCK approvals). NockNock builds no transaction and signs nothing."
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
        "dilution": {
          "share_of_active_pct": "1.73"
        },
        "dilution_reason": null,
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
      "fit_factor": null,
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
        "liquidity": "1408301616732632",
        "tracked_liquidity": "0",
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
        "dilution": {
          "share_of_active_pct": "14.98"
        },
        "dilution_reason": null,
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
      "fit_factor": null,
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
        "liquidity": "14125167085436768",
        "tracked_liquidity": "0",
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
  "sentence": "Your ladder shape in the USDC/NOCK 3% v4 pool: 3 of 3 deposits are left, holding up to $766.97 USDC and 86.79 NOCK now: sized to leave the stable balance at or above your $950 reserve at $0.01. Fully converted at $0.01002 they would hold about 71,975 NOCK, about 71,888 of it bought, at an average of $0.01067.",
  "shape": {
    "below_minimum": 0,
    "deployed": 0,
    "found": 0,
    "held": 0,
    "left": 3,
    "none": 0,
    "of": 3,
    "over": 0,
    "passed": 0,
    "rest_rule": "what the reserve gate allows now, less the cap's USDC: no near or deep rung is deposited yet, so this follows the gate until one is",
    "rest_usdc": "765.76",
    "rungs": [
      {
        "bounds_usd": {
          "high": "0.01313098",
          "low": "0.01274292"
        },
        "counterparts": [],
        "deployed_nock": "0.0000",
        "deployed_usdc": "0.00",
        "found": [],
        "holds_now": null,
        "in_range_now": true,
        "index": 1,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "86.7891",
        "remaining_usdc": "1.22",
        "rung": "cap",
        "status": "to_deposit",
        "target_liquidity": "138192115820500",
        "target_nock": "86.7891",
        "target_usdc": "1.23",
        "text": "Deposit 1 of 3 (cap): left to deposit, about 86.79 NOCK + $1.22 USDC.",
        "tick_lower": 273600,
        "tick_upper": 273900,
        "tracked_liquidity": "0"
      },
      {
        "bounds_usd": {
          "high": "0.01274292",
          "low": "0.01096801"
        },
        "counterparts": [],
        "deployed_nock": null,
        "deployed_usdc": "0.00",
        "found": [],
        "holds_now": null,
        "in_range_now": false,
        "index": 2,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "0.0000",
        "remaining_usdc": "114.86",
        "rung": "near",
        "status": "to_deposit",
        "target_liquidity": "1408301616732632",
        "target_nock": null,
        "target_usdc": "114.86",
        "text": "Deposit 2 of 3 (near rung): left to deposit, $114.86 USDC.",
        "tick_lower": 273900,
        "tick_upper": 275400,
        "tracked_liquidity": "0"
      },
      {
        "bounds_usd": {
          "high": "0.01096801",
          "low": "0.01002405"
        },
        "counterparts": [],
        "deployed_nock": null,
        "deployed_usdc": "0.00",
        "found": [],
        "holds_now": null,
        "in_range_now": false,
        "index": 3,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "0.0000",
        "remaining_usdc": "650.89",
        "rung": "deep",
        "status": "to_deposit",
        "target_liquidity": "14125167085436768",
        "target_nock": null,
        "target_usdc": "650.90",
        "text": "Deposit 3 of 3 (deep rung): left to deposit, $650.89 USDC.",
        "tick_lower": 275400,
        "tick_upper": 276300,
        "tracked_liquidity": "0"
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
    "deposits_usdc": "766.97",
    "existing_ranges_usdc_at_gate": "171.15",
    "existing_ranges_usdc_now": "223.71",
    "gate_price_usd": "0.01000000",
    "rule": "the standing reserve is measured on wallet USDC plus USDC inside NOCK ranges, read at the gate price",
    "stable_at_gate_usdc": "950.02",
    "text": "After these deposits your wallet holds about $779 USDC; the $950 reserve counts the $224 of USDC inside your existing NOCK ranges. At $0.01 with every rung converted, the stable balance reads $950.02: the wallet's $778.87 plus $171.15 still inside those ranges.",
    "wallet_alone_below_reserve": true,
    "wallet_usdc_after": "778.87",
    "wallet_usdc_now": "1545.84"
  },
  "what_changed": "Your ladder shape has 3 of 3 deposits left in the USDC/NOCK 3% v4 pool: up to $766.97 USDC and about 87 NOCK."
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
    "The amounts are your own shape (operator_profile.ladder_rungs): until a near or deep rung is deposited it is sized to what the standing reserve allows at the gate price; from then on that rung fixes the plan. When the reserve allows less than what is left, every deposit is cut in proportion and the shortfall is stated. Fees and gas are not counted in them.",
    "A cap is in range now and holds NOCK; every other rung holds only USDC now and earns no fees until NOCK falls to its top.",
    "Each rung is measured by its liquidity, which does not change with the price: a deposited rung stays deposited when the price moves, and only what is left is listed. A remainder under $1.00 is not offered.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "Each fee line is a past measurement on our own tracked range scaled to the rung's liquidity: an estimate, not a forecast."
  ],
  "candidates": null,
  "excluded": [],
  "fit": {
    "allowed_usdc": "116.09",
    "needed_usdc": "116.09",
    "rule": "once a near or deep rung is deposited the cap takes only what the reserve leaves beside the near and deep rungs still to deposit; when they alone need more than the reserve allows, each deposit left is cut pro rata (by the same fraction), so the shape keeps its proportions, and the shortfall is stated",
    "shortfall_usdc": null,
    "status": "fits",
    "text": null
  },
  "found_note": null,
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
  "headline": "Ladder: 2 of 3 deposits left, up to $116.08 USDC and about 87 NOCK.",
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
    "headroom_usdc": "116.09",
    "lp_nock_budget": "86.7891",
    "nock_at_bottom": "9898.2556",
    "nock_bought_at_bottom": "9811.4665",
    "nock_now": "86.7891",
    "pool_price_usd": "0.01294400",
    "shape_rest_usdc": "765.76",
    "shape_total_usdc": "766.99",
    "shape_tracked_usdc": "650.90",
    "stable_at_gate_usdc": "950.01",
    "usdc_max": "116.08",
    "wallet_usdc": "894.94",
    "wallet_usdc_after": "778.86"
  },
  "outcomes": [
    {
      "avg_buy_price_usd": "0.01183214",
      "kind": "fall_to_bottom",
      "nock_bought": "9811",
      "pct_of_spot": "-15%",
      "price_usd": "0.01096801",
      "text": "A fall to $0.01097 (-15%), the bottom of what is left to deposit, would buy about 9,811 NOCK in total with all $116.08, an average of $0.01183.",
      "usdc_spent": "116.08"
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
        "label": "$1.22 + 87 NOCK",
        "low_usd": "0.01274292",
        "rung": "cap",
        "status": "to_deposit",
        "usdc": "1.22"
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
        "note": "the cap holds the whole LP NOCK budget, 86.7891 NOCK; the curve pairs it with $1.23 USDC",
        "usdc_limit": "1.23",
        "usdc_limit_is": "reserve"
      },
      "deep_link": {
        "amount_to_enter": "Amount to enter in the app: 86.7890 NOCK, in the NOCK field; the app then works out the USDC side, about $1.22 at this reading.",
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
        "dilution": {
          "share_of_active_pct": "0.17"
        },
        "dilution_reason": null,
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
      "fit_factor": null,
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
        "binding": "nock",
        "field_to_type": "NOCK",
        "holds_less_than_budget": false,
        "is": "lp_nock_budget",
        "nock": "86.7891",
        "nock_to_enter": "86.7890",
        "text": "The NOCK side must not exceed 86.7891 NOCK, your LP NOCK budget, which binds now; the USDC side must not exceed $1.23, what your reserve leaves for it beside the other deposits listed. The limits hold when the NOCK field is the one typed, at 86.7890 NOCK or less, and the app shows at most $1.23 of USDC: about $1.22 at this reading, more if the price rises inside the band before you deposit. When the USDC field is typed instead, the app works out the NOCK side from the price at that moment, and it can come out above the limit; neither the link nor the app applies a NOCK limit of its own.",
        "usdc": "1.23",
        "usdc_then_about": "1.22"
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
      "summary": "about 86.79 NOCK + $1.22 USDC, in range now",
      "target": {
        "counterparts": [],
        "deployed_usdc": "0.00",
        "liquidity": "138192115820500",
        "tracked_liquidity": "0",
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
      "would_sign": "In the Uniswap web app, with your own wallet: one new position in the USDC/NOCK 3% v4 pool, ticks 273600 to 273900, holding up to 86.7890 NOCK and about $1.22 USDC (the app may first ask for USDC and NOCK approvals). NockNock builds no transaction and signs nothing."
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
        "dilution": {
          "share_of_active_pct": "1.73"
        },
        "dilution_reason": null,
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
      "fit_factor": null,
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
        "liquidity": "1408301615002245",
        "tracked_liquidity": "0",
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
  "sentence": "Your ladder shape in the USDC/NOCK 3% v4 pool: 2 of 3 deposits are left (1 deployed), holding up to $116.08 USDC and 86.79 NOCK now: sized to leave the stable balance at or above your $950 reserve at $0.01. Fully converted at $0.01097 they would hold about 9,898 NOCK, about 9,811 of it bought, at an average of $0.01183.",
  "shape": {
    "below_minimum": 0,
    "deployed": 1,
    "found": 0,
    "held": 0,
    "left": 2,
    "none": 0,
    "of": 3,
    "over": 0,
    "passed": 0,
    "rest_rule": "frozen by the deep rung already deposited (the deposit order is not known for every rung, so the one implying the most): its $650.90 of USDC at its top is its share of $765.76, which the near and deep shares divide; it does not move with the price",
    "rest_usdc": "765.76",
    "rungs": [
      {
        "bounds_usd": {
          "high": "0.01313098",
          "low": "0.01274292"
        },
        "counterparts": [],
        "deployed_nock": "0.0000",
        "deployed_usdc": "0.00",
        "found": [],
        "holds_now": null,
        "in_range_now": true,
        "index": 1,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "86.7891",
        "remaining_usdc": "1.22",
        "rung": "cap",
        "status": "to_deposit",
        "target_liquidity": "138192115820500",
        "target_nock": "86.7891",
        "target_usdc": "1.23",
        "text": "Deposit 1 of 3 (cap): left to deposit, about 86.79 NOCK + $1.22 USDC.",
        "tick_lower": 273600,
        "tick_upper": 273900,
        "tracked_liquidity": "0"
      },
      {
        "bounds_usd": {
          "high": "0.01274292",
          "low": "0.01096801"
        },
        "counterparts": [],
        "deployed_nock": null,
        "deployed_usdc": "0.00",
        "found": [],
        "holds_now": null,
        "in_range_now": false,
        "index": 2,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "0.0000",
        "remaining_usdc": "114.86",
        "rung": "near",
        "status": "to_deposit",
        "target_liquidity": "1408301615002245",
        "target_nock": null,
        "target_usdc": "114.86",
        "text": "Deposit 2 of 3 (near rung): left to deposit, $114.86 USDC.",
        "tick_lower": 273900,
        "tick_upper": 275400,
        "tracked_liquidity": "0"
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
        "found": [],
        "holds_now": {
          "nock": "0.0000",
          "usdc": "650.90"
        },
        "in_range_now": false,
        "index": 3,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "0.0000",
        "remaining_usdc": "0.00",
        "rung": "deep",
        "status": "deployed",
        "target_liquidity": "14125167068081104",
        "target_nock": null,
        "target_usdc": "650.90",
        "text": "Deposit 3 of 3 (deep rung): done. It is deployed in v4 USDC/NOCK 3.00% #9000001, which holds $650.90 USDC and 0 NOCK now.",
        "tick_lower": 275400,
        "tick_upper": 276300,
        "tracked_liquidity": "14125167068081104"
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
    "deposits_usdc": "116.08",
    "existing_ranges_usdc_at_gate": "171.15",
    "existing_ranges_usdc_now": "874.61",
    "gate_price_usd": "0.01000000",
    "rule": "the standing reserve is measured on wallet USDC plus USDC inside NOCK ranges, read at the gate price",
    "stable_at_gate_usdc": "950.01",
    "text": "After these deposits your wallet holds about $779 USDC; the $950 reserve counts the $875 of USDC inside your existing NOCK ranges. At $0.01 with every rung converted, the stable balance reads $950.01: the wallet's $778.86 plus $171.15 still inside those ranges.",
    "wallet_alone_below_reserve": true,
    "wallet_usdc_after": "778.86",
    "wallet_usdc_now": "894.94"
  },
  "what_changed": "Your ladder shape has 2 of 3 deposits left in the USDC/NOCK 3% v4 pool: up to $116.08 USDC and about 87 NOCK."
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
    "The amounts are your own shape (operator_profile.ladder_rungs): until a near or deep rung is deposited it is sized to what the standing reserve allows at the gate price; from then on that rung fixes the plan. When the reserve allows less than what is left, every deposit is cut in proportion and the shortfall is stated. Fees and gas are not counted in them.",
    "A cap is in range now and holds NOCK; every other rung holds only USDC now and earns no fees until NOCK falls to its top.",
    "Each rung is measured by its liquidity, which does not change with the price: a deposited rung stays deposited when the price moves, and only what is left is listed. A remainder under $1.00 is not offered.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "Each fee line is a past measurement on our own tracked range scaled to the rung's liquidity: an estimate, not a forecast."
  ],
  "candidates": null,
  "excluded": [],
  "fit": {
    "allowed_usdc": "650.90",
    "needed_usdc": "650.90",
    "rule": "once a near or deep rung is deposited the cap takes only what the reserve leaves beside the near and deep rungs still to deposit; when they alone need more than the reserve allows, each deposit left is cut pro rata (by the same fraction), so the shape keeps its proportions, and the shortfall is stated",
    "shortfall_usdc": null,
    "status": "fits",
    "text": null
  },
  "found_note": null,
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
  "headline": "Ladder: 1 of 3 deposits left, up to $650.89 USDC.",
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
    "headroom_usdc": "650.90",
    "lp_nock_budget": "0.0000",
    "nock_at_bottom": "62076.6388",
    "nock_bought_at_bottom": "62076.6388",
    "nock_now": "0.0000",
    "pool_price_usd": "0.01294400",
    "shape_rest_usdc": "765.76",
    "shape_total_usdc": "766.99",
    "shape_tracked_usdc": "116.09",
    "stable_at_gate_usdc": "950.01",
    "usdc_max": "650.89",
    "wallet_usdc": "1429.75",
    "wallet_usdc_after": "778.86"
  },
  "outcomes": [
    {
      "avg_buy_price_usd": "0.01048542",
      "kind": "fall_to_bottom",
      "nock_bought": "62077",
      "pct_of_spot": "-23%",
      "price_usd": "0.01002405",
      "text": "A fall to $0.01002 (-23%), the bottom of what is left to deposit, would buy about 62,077 NOCK in total with all $650.89, an average of $0.01049.",
      "usdc_spent": "650.89"
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
        "nock_bought": "62076.6388",
        "nock_held": "62076.6388"
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
        "dilution": {
          "share_of_active_pct": "14.98"
        },
        "dilution_reason": null,
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
      "fit_factor": null,
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
        "liquidity": "14125167068081028",
        "tracked_liquidity": "0",
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
  "sentence": "Your ladder shape in the USDC/NOCK 3% v4 pool: 1 of 3 deposits is left (2 deployed), holding up to $650.89 USDC and 0 NOCK now: sized to leave the stable balance at or above your $950 reserve at $0.01. Fully converted at $0.01002 they would hold about 62,077 NOCK, about 62,077 of it bought, at an average of $0.01049.",
  "shape": {
    "below_minimum": 0,
    "deployed": 2,
    "found": 0,
    "held": 0,
    "left": 1,
    "none": 0,
    "of": 3,
    "over": 0,
    "passed": 0,
    "rest_rule": "frozen by the near rung already deposited (the deposit order is not known for every rung, so the one implying the most): its $114.86 of USDC at its top is its share of $765.76, which the near and deep shares divide; it does not move with the price",
    "rest_usdc": "765.76",
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
        "found": [],
        "holds_now": {
          "nock": "86.7891",
          "usdc": "1.23"
        },
        "in_range_now": true,
        "index": 1,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "0.0000",
        "remaining_usdc": "0.00",
        "rung": "cap",
        "status": "deployed",
        "target_liquidity": "138192109253769",
        "target_nock": "86.7891",
        "target_usdc": "1.23",
        "text": "Deposit 1 of 3 (cap): done. It is deployed in v4 USDC/NOCK 3.00% #9000002, which holds $1.23 USDC and 87 NOCK now.",
        "tick_lower": 273600,
        "tick_upper": 273900,
        "tracked_liquidity": "138192109253769"
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
        "found": [],
        "holds_now": {
          "nock": "0.0000",
          "usdc": "114.86"
        },
        "in_range_now": false,
        "index": 2,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "0.0000",
        "remaining_usdc": "0.00",
        "rung": "near",
        "status": "deployed",
        "target_liquidity": "1408301615002237",
        "target_nock": null,
        "target_usdc": "114.86",
        "text": "Deposit 2 of 3 (near rung): done. It is deployed in v4 USDC/NOCK 3.00% #9000003, which holds $114.86 USDC and 0 NOCK now.",
        "tick_lower": 273900,
        "tick_upper": 275400,
        "tracked_liquidity": "1408301615002237"
      },
      {
        "bounds_usd": {
          "high": "0.01096801",
          "low": "0.01002405"
        },
        "counterparts": [],
        "deployed_nock": null,
        "deployed_usdc": "0.00",
        "found": [],
        "holds_now": null,
        "in_range_now": false,
        "index": 3,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "0.0000",
        "remaining_usdc": "650.89",
        "rung": "deep",
        "status": "to_deposit",
        "target_liquidity": "14125167068081028",
        "target_nock": null,
        "target_usdc": "650.90",
        "text": "Deposit 3 of 3 (deep rung): left to deposit, $650.89 USDC.",
        "tick_lower": 275400,
        "tick_upper": 276300,
        "tracked_liquidity": "0"
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
    "deposits_usdc": "650.89",
    "existing_ranges_usdc_at_gate": "171.15",
    "existing_ranges_usdc_now": "339.80",
    "gate_price_usd": "0.01000000",
    "rule": "the standing reserve is measured on wallet USDC plus USDC inside NOCK ranges, read at the gate price",
    "stable_at_gate_usdc": "950.01",
    "text": "After these deposits your wallet holds about $779 USDC; the $950 reserve counts the $340 of USDC inside your existing NOCK ranges. At $0.01 with every rung converted, the stable balance reads $950.01: the wallet's $778.86 plus $171.15 still inside those ranges.",
    "wallet_alone_below_reserve": true,
    "wallet_usdc_after": "778.86",
    "wallet_usdc_now": "1429.75"
  },
  "what_changed": "Your ladder shape has 1 of 3 deposits left in the USDC/NOCK 3% v4 pool: up to $650.89 USDC."
} as any;

/** all_deployed: every rung deployed */
export const LADDER_BOTTLE_ALL_DEPLOYED: LadderSuggestion = {
  "after_deposit": null,
  "assumptions": [
    "The amounts are your own shape (operator_profile.ladder_rungs): until a near or deep rung is deposited it is sized to what the standing reserve allows at the gate price; from then on that rung fixes the plan. When the reserve allows less than what is left, every deposit is cut in proportion and the shortfall is stated. Fees and gas are not counted in them.",
    "A cap is in range now and holds NOCK; every other rung holds only USDC now and earns no fees until NOCK falls to its top.",
    "Each rung is measured by its liquidity, which does not change with the price: a deposited rung stays deposited when the price moves, and only what is left is listed. A remainder under $1.00 is not offered.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "Each fee line is a past measurement on our own tracked range scaled to the rung's liquidity: an estimate, not a forecast."
  ],
  "candidates": null,
  "excluded": [],
  "fit": null,
  "found_note": null,
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
    "below_minimum": 0,
    "deployed": 3,
    "found": 0,
    "held": 0,
    "left": 0,
    "none": 0,
    "of": 3,
    "over": 0,
    "passed": 0,
    "rest_rule": "frozen by the deep rung already deposited (the deposit order is not known for every rung, so the one implying the most): its $650.90 of USDC at its top is its share of $765.76, which the near and deep shares divide; it does not move with the price",
    "rest_usdc": "765.76",
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
        "found": [],
        "holds_now": {
          "nock": "86.7891",
          "usdc": "1.23"
        },
        "in_range_now": true,
        "index": 1,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "0.0000",
        "remaining_usdc": "0.00",
        "rung": "cap",
        "status": "deployed",
        "target_liquidity": "138192109253769",
        "target_nock": "86.7891",
        "target_usdc": "1.23",
        "text": "Deposit 1 of 3 (cap): done. It is deployed in v4 USDC/NOCK 3.00% #9000002, which holds $1.23 USDC and 87 NOCK now.",
        "tick_lower": 273600,
        "tick_upper": 273900,
        "tracked_liquidity": "138192109253769"
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
        "found": [],
        "holds_now": {
          "nock": "0.0000",
          "usdc": "114.86"
        },
        "in_range_now": false,
        "index": 2,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "0.0000",
        "remaining_usdc": "0.00",
        "rung": "near",
        "status": "deployed",
        "target_liquidity": "1408301615002245",
        "target_nock": null,
        "target_usdc": "114.86",
        "text": "Deposit 2 of 3 (near rung): done. It is deployed in v4 USDC/NOCK 3.00% #9000003, which holds $114.86 USDC and 0 NOCK now.",
        "tick_lower": 273900,
        "tick_upper": 275400,
        "tracked_liquidity": "1408301615002237"
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
        "found": [],
        "holds_now": {
          "nock": "0.0000",
          "usdc": "650.90"
        },
        "in_range_now": false,
        "index": 3,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "0.0000",
        "remaining_usdc": "0.00",
        "rung": "deep",
        "status": "deployed",
        "target_liquidity": "14125167068081104",
        "target_nock": null,
        "target_usdc": "650.90",
        "text": "Deposit 3 of 3 (deep rung): done. It is deployed in v4 USDC/NOCK 3.00% #9000001, which holds $650.90 USDC and 0 NOCK now.",
        "tick_lower": 275400,
        "tick_upper": 276300,
        "tracked_liquidity": "14125167068081104"
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
    "The amounts are your own shape (operator_profile.ladder_rungs): until a near or deep rung is deposited it is sized to what the standing reserve allows at the gate price; from then on that rung fixes the plan. When the reserve allows less than what is left, every deposit is cut in proportion and the shortfall is stated. Fees and gas are not counted in them.",
    "A cap is in range now and holds NOCK; every other rung holds only USDC now and earns no fees until NOCK falls to its top.",
    "Each rung is measured by its liquidity, which does not change with the price: a deposited rung stays deposited when the price moves, and only what is left is listed. A remainder under $1.00 is not offered.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "Each fee line is a past measurement on our own tracked range scaled to the rung's liquidity: an estimate, not a forecast."
  ],
  "candidates": null,
  "excluded": [],
  "fit": {
    "allowed_usdc": "773.86",
    "needed_usdc": "672.92",
    "rule": "once a near or deep rung is deposited the cap takes only what the reserve leaves beside the near and deep rungs still to deposit; when they alone need more than the reserve allows, each deposit left is cut pro rata (by the same fraction), so the shape keeps its proportions, and the shortfall is stated",
    "shortfall_usdc": null,
    "status": "fits",
    "text": null
  },
  "found_note": null,
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
      "max_usdc": "773.86",
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
  "headline": "Ladder: 2 of 3 deposits left, up to $672.91 USDC and about 15 NOCK.",
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
    "headroom_usdc": "773.86",
    "lp_nock_budget": "86.7891",
    "nock_at_bottom": "63631.5918",
    "nock_bought_at_bottom": "63616.5695",
    "nock_now": "15.0223",
    "pool_price_usd": "0.01130146",
    "shape_rest_usdc": "672.92",
    "shape_total_usdc": "773.86",
    "shape_tracked_usdc": "0.00",
    "stable_at_gate_usdc": "1050.96",
    "usdc_max": "672.91",
    "wallet_usdc": "1545.84",
    "wallet_usdc_after": "872.93"
  },
  "outcomes": [
    {
      "avg_buy_price_usd": "0.01057777",
      "kind": "fall_to_bottom",
      "nock_bought": "63617",
      "pct_of_spot": "-11%",
      "price_usd": "0.01002405",
      "text": "A fall to $0.01002 (-11%), the bottom of what is left to deposit, would buy about 63,617 NOCK in total with all $672.91, an average of $0.01058.",
      "usdc_spent": "672.91"
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
        "label": "$100.93 + 15 NOCK",
        "low_usd": "0.01096801",
        "rung": "cap",
        "status": "to_deposit",
        "usdc": "100.93"
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
        "note": "the budget's 86.7891 NOCK would pair with $583.16 USDC, more than the most that keeps it at or below the near rung's USDC, $100.94: the cap's liquidity is reduced until its USDC equals that, so it holds 15.0223 NOCK, less than the budget",
        "usdc_limit": "100.94",
        "usdc_limit_is": "near_share"
      },
      "deep_link": {
        "amount_to_enter": "Amount to enter in the app: $100.93 USDC, in the USDC field; the app then works out the NOCK side, about 15.0223 NOCK at this reading.",
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
        "dilution": {
          "share_of_active_pct": "7.38"
        },
        "dilution_reason": null,
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
      "fit_factor": null,
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
        "binding": "usdc",
        "field_to_type": "USDC",
        "holds_less_than_budget": true,
        "is": "near_usdc",
        "nock": "86.7891",
        "nock_then_about": "15.0223",
        "text": "The USDC side must not exceed $100.94, the most that keeps it at or below the near rung's USDC, which binds now, so the cap holds about 15.0223 NOCK, less than your 86.7891 NOCK LP budget. The limit holds when the USDC field is the one typed, at $100.93 or less. When the NOCK field is typed instead, the app works out the USDC side from the price at that moment, and it can come out above the limit; neither the link nor the app applies a USDC limit of its own.",
        "usdc": "100.94",
        "usdc_to_enter": "100.93"
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
      "summary": "about 15.02 NOCK + $100.93 USDC, in range now",
      "target": {
        "counterparts": [],
        "deployed_usdc": "0.00",
        "liquidity": "6388384533619972",
        "tracked_liquidity": "0",
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
      "would_sign": "In the Uniswap web app, with your own wallet: one new position in the USDC/NOCK 3% v4 pool, ticks 275100 to 275400, holding up to 15.0223 NOCK and about $100.93 USDC (the app may first ask for USDC and NOCK approvals). NockNock builds no transaction and signs nothing."
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
        "dilution": {
          "share_of_active_pct": "13.41"
        },
        "dilution_reason": null,
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
      "fit_factor": null,
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
        "liquidity": "12412612748768140",
        "tracked_liquidity": "0",
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
  "sentence": "Your ladder shape in the USDC/NOCK 3% v4 pool: 2 of 3 deposits are left (1 passed), holding up to $672.91 USDC and 15.02 NOCK now: sized to leave the stable balance at or above your $950 reserve at $0.01. Fully converted at $0.01002 they would hold about 63,632 NOCK, about 63,617 of it bought, at an average of $0.01058.",
  "shape": {
    "below_minimum": 0,
    "deployed": 0,
    "found": 0,
    "held": 0,
    "left": 2,
    "none": 0,
    "of": 3,
    "over": 0,
    "passed": 1,
    "rest_rule": "what the reserve gate allows now, less the cap's USDC: no near or deep rung is deposited yet, so this follows the gate until one is",
    "rest_usdc": "672.92",
    "rungs": [
      {
        "bounds_usd": {
          "high": "0.01130202",
          "low": "0.01096801"
        },
        "counterparts": [],
        "deployed_nock": "0.0000",
        "deployed_usdc": "0.00",
        "found": [],
        "holds_now": null,
        "in_range_now": true,
        "index": 1,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "15.0223",
        "remaining_usdc": "100.93",
        "rung": "cap",
        "status": "to_deposit",
        "target_liquidity": "6388384533619972",
        "target_nock": "15.0223",
        "target_usdc": "100.94",
        "text": "Deposit 1 of 3 (cap): left to deposit, about 15.02 NOCK + $100.93 USDC.",
        "tick_lower": 275100,
        "tick_upper": 275400,
        "tracked_liquidity": "0"
      },
      {
        "bounds_usd": null,
        "counterparts": [],
        "deployed_nock": null,
        "deployed_usdc": "0.00",
        "found": [],
        "holds_now": null,
        "in_range_now": null,
        "index": 2,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": "the price $0.01130146 is less than one tick spacing above this rung's bottom $0.01096801: the cap's band, the one containing the price, covers all that is left of it, so it has no range of its own",
        "remaining_nock": "0.0000",
        "remaining_usdc": "0.00",
        "rung": "near",
        "status": "passed",
        "target_liquidity": null,
        "target_nock": null,
        "target_usdc": "100.94",
        "text": "Deposit 2 of 3 (near rung): passed. The price $0.01130146 is less than one tick spacing above this rung's bottom $0.01096801: the cap's band, the one containing the price, covers all that is left of it, so it has no range of its own.",
        "tick_lower": null,
        "tick_upper": null,
        "tracked_liquidity": "0"
      },
      {
        "bounds_usd": {
          "high": "0.01096801",
          "low": "0.01002405"
        },
        "counterparts": [],
        "deployed_nock": null,
        "deployed_usdc": "0.00",
        "found": [],
        "holds_now": null,
        "in_range_now": false,
        "index": 3,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "0.0000",
        "remaining_usdc": "571.98",
        "rung": "deep",
        "status": "to_deposit",
        "target_liquidity": "12412612748768140",
        "target_nock": null,
        "target_usdc": "571.98",
        "text": "Deposit 3 of 3 (deep rung): left to deposit, $571.98 USDC.",
        "tick_lower": 275400,
        "tick_upper": 276300,
        "tracked_liquidity": "0"
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
    "deposits_usdc": "672.91",
    "existing_ranges_usdc_at_gate": "178.03",
    "existing_ranges_usdc_now": "202.53",
    "gate_price_usd": "0.01000000",
    "rule": "the standing reserve is measured on wallet USDC plus USDC inside NOCK ranges, read at the gate price",
    "stable_at_gate_usdc": "1050.96",
    "text": "After these deposits your wallet holds about $873 USDC; the $950 reserve counts the $203 of USDC inside your existing NOCK ranges. At $0.01 with every rung converted, the stable balance reads $1050.96: the wallet's $872.93 plus $178.03 still inside those ranges.",
    "wallet_alone_below_reserve": true,
    "wallet_usdc_after": "872.93",
    "wallet_usdc_now": "1545.84"
  },
  "what_changed": "Your ladder shape has 2 of 3 deposits left in the USDC/NOCK 3% v4 pool: up to $672.91 USDC and about 15 NOCK."
} as any;

/** near_over: the deep rung deposited first, the near rung later and larger: the near rung is over, the cap is listed */
export const LADDER_BOTTLE_OVER: LadderSuggestion = {
  "after_deposit": {
    "detection_note": null,
    "detection_status": null,
    "kind": "tracked_automatically",
    "text": "After you deposit there is nothing to copy or configure. NockNock looks for the wallet's new position NFT on-chain at the start of every cycle (CHECK NOW runs one at once), so a deposit is normally found in the first cycle after it lands. A position your wallet minted in this pool with exactly a rung's ticks is tracked as a ladder by itself, with its mint as the baseline. Any other range it finds is shown as found on-chain and is not counted until one tap says how to track it."
  },
  "assumptions": [
    "The amounts are your own shape (operator_profile.ladder_rungs): until a near or deep rung is deposited it is sized to what the standing reserve allows at the gate price; from then on that rung fixes the plan. When the reserve allows less than what is left, every deposit is cut in proportion and the shortfall is stated. Fees and gas are not counted in them.",
    "A cap is in range now and holds NOCK; every other rung holds only USDC now and earns no fees until NOCK falls to its top.",
    "Each rung is measured by its liquidity, which does not change with the price: a deposited rung stays deposited when the price moves, and only what is left is listed. A remainder under $1.00 is not offered.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "Each fee line is a past measurement on our own tracked range scaled to the rung's liquidity: an estimate, not a forecast."
  ],
  "candidates": null,
  "excluded": [],
  "fit": {
    "allowed_usdc": "3.79",
    "needed_usdc": "1.23",
    "rule": "once a near or deep rung is deposited the cap takes only what the reserve leaves beside the near and deep rungs still to deposit; when they alone need more than the reserve allows, each deposit left is cut pro rata (by the same fraction), so the shape keeps its proportions, and the shortfall is stated",
    "shortfall_usdc": null,
    "status": "fits",
    "text": null
  },
  "found_note": null,
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
      "margin_usdc": "2.57",
      "max_usdc": "3.79",
      "minimum_usdc": "1.00",
      "read_at_rule": "your ladder bottom (operator_profile.ladder_bottom_usd $0.01000000), below the deepest rung's bottom",
      "read_at_usd": "0.01000000",
      "rule": "the stable balance at the gate price stays at or above the reserve",
      "shape_total_usdc": "766.99",
      "stable_at_gate_usdc": "952.57",
      "status": "allows"
    },
    "shape_setting": {
      "key": "operator_profile.ladder_rungs",
      "status": "set"
    }
  },
  "headline": "Ladder: 1 of 3 deposits left, up to $1.22 USDC and about 87 NOCK.",
  "id": "ladder_suggestion:NOCK",
  "minimum_usdc": "1.00",
  "mint_note": "liquidity_raw is what these amounts buy, rounded down (LiquidityAmounts); amount0_raw and amount1_raw are what a mint of it pulls, rounded up (SqrtPriceMath), never more than the maxima (the rung's USDC and NOCK truncated to raw units)",
  "missing": [],
  "numbers": {
    "avg_buy_price_usd": "0.01284307",
    "binding": "reserve",
    "cash_reserve_usdc": "950.00",
    "existing_ranges_usdc_at_gate": "171.16",
    "existing_ranges_usdc_now": "1046.90",
    "gate_price_usd": "0.01000000",
    "headroom_usdc": "3.79",
    "lp_nock_budget": "86.7891",
    "nock_at_bottom": "182.2491",
    "nock_bought_at_bottom": "95.4600",
    "nock_now": "86.7891",
    "pool_price_usd": "0.01294400",
    "shape_rest_usdc": "765.76",
    "shape_total_usdc": "766.99",
    "shape_tracked_usdc": "823.20",
    "stable_at_gate_usdc": "952.58",
    "usdc_max": "1.22",
    "wallet_usdc": "782.64",
    "wallet_usdc_after": "781.42"
  },
  "outcomes": [
    {
      "avg_buy_price_usd": "0.01284307",
      "kind": "fall_to_bottom",
      "nock_bought": "95",
      "pct_of_spot": "-1.6%",
      "price_usd": "0.01274292",
      "text": "A fall to $0.01274 (-1.6%), the bottom of what is left to deposit, would buy about 95 NOCK in total with all $1.22, an average of $0.01284.",
      "usdc_spent": "1.22"
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
        "label": "$1.22 + 87 NOCK",
        "low_usd": "0.01274292",
        "rung": "cap",
        "status": "to_deposit",
        "usdc": "1.22"
      },
      {
        "deployed": true,
        "high_usd": "0.01274292",
        "index": 2,
        "label": "$172.30",
        "low_usd": "0.01096801",
        "rung": "near",
        "status": "over",
        "usdc": "172.30"
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
        "note": "the cap holds the whole LP NOCK budget, 86.7891 NOCK; the curve pairs it with $1.23 USDC",
        "usdc_limit": "3.80",
        "usdc_limit_is": "reserve"
      },
      "deep_link": {
        "amount_to_enter": "Amount to enter in the app: 86.7890 NOCK, in the NOCK field; the app then works out the USDC side, about $1.22 at this reading.",
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
        "dilution": {
          "share_of_active_pct": "0.17"
        },
        "dilution_reason": null,
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
      "fit_factor": null,
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
        "binding": "nock",
        "field_to_type": "NOCK",
        "holds_less_than_budget": false,
        "is": "lp_nock_budget",
        "nock": "86.7891",
        "nock_to_enter": "86.7890",
        "text": "The NOCK side must not exceed 86.7891 NOCK, your LP NOCK budget, which binds now; the USDC side must not exceed $3.80, what your reserve leaves for it beside the other deposits listed. The limits hold when the NOCK field is the one typed, at 86.7890 NOCK or less, and the app shows at most $3.80 of USDC: about $1.22 at this reading, more if the price rises inside the band before you deposit. When the USDC field is typed instead, the app works out the NOCK side from the price at that moment, and it can come out above the limit; neither the link nor the app applies a NOCK limit of its own.",
        "usdc": "3.80",
        "usdc_then_about": "1.22"
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
      "summary": "about 86.79 NOCK + $1.22 USDC, in range now",
      "target": {
        "counterparts": [],
        "deployed_usdc": "0.00",
        "liquidity": "138192115820500",
        "tracked_liquidity": "0",
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
      "would_sign": "In the Uniswap web app, with your own wallet: one new position in the USDC/NOCK 3% v4 pool, ticks 273600 to 273900, holding up to 86.7890 NOCK and about $1.22 USDC (the app may first ask for USDC and NOCK approvals). NockNock builds no transaction and signs nothing."
    }
  ],
  "sentence": "Your ladder shape in the USDC/NOCK 3% v4 pool: 1 of 3 deposits is left (1 deployed, 1 over target), holding up to $1.22 USDC and 86.79 NOCK now: sized to leave the stable balance at or above your $950 reserve at $0.01. Fully converted at $0.01274 they would hold about 182 NOCK, about 95 of it bought, at an average of $0.01284.",
  "shape": {
    "below_minimum": 0,
    "deployed": 1,
    "found": 0,
    "held": 0,
    "left": 1,
    "none": 0,
    "of": 3,
    "over": 1,
    "passed": 0,
    "rest_rule": "frozen by the deep rung already deposited (the first deposited, by its deposit block): its $650.90 of USDC at its top is its share of $765.76, which the near and deep shares divide; it does not move with the price",
    "rest_usdc": "765.76",
    "rungs": [
      {
        "bounds_usd": {
          "high": "0.01313098",
          "low": "0.01274292"
        },
        "counterparts": [],
        "deployed_nock": "0.0000",
        "deployed_usdc": "0.00",
        "found": [],
        "holds_now": null,
        "in_range_now": true,
        "index": 1,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "86.7891",
        "remaining_usdc": "1.22",
        "rung": "cap",
        "status": "to_deposit",
        "target_liquidity": "138192115820500",
        "target_nock": "86.7891",
        "target_usdc": "1.23",
        "text": "Deposit 1 of 3 (cap): left to deposit, about 86.79 NOCK + $1.22 USDC.",
        "tick_lower": 273600,
        "tick_upper": 273900,
        "tracked_liquidity": "0"
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
        "deployed_usdc": "172.30",
        "found": [],
        "holds_now": {
          "nock": "0.0000",
          "usdc": "172.30"
        },
        "in_range_now": false,
        "index": 2,
        "note": null,
        "of": 3,
        "over_by_usdc": "57.43",
        "reason": "the tracked rung holds $172.30 of USDC at its top, $57.43 more than its target",
        "remaining_nock": "0.0000",
        "remaining_usdc": "0.00",
        "rung": "near",
        "status": "over",
        "target_liquidity": "1408301615002245",
        "target_nock": null,
        "target_usdc": "114.86",
        "text": "Deposit 2 of 3 (near rung): over its target; it is stated and changes nothing else. The tracked rung holds $172.30 of USDC at its top, $57.43 more than its target.",
        "tick_lower": 273900,
        "tick_upper": 275400,
        "tracked_liquidity": "2112452422503355"
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
        "found": [],
        "holds_now": {
          "nock": "0.0000",
          "usdc": "650.90"
        },
        "in_range_now": false,
        "index": 3,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "0.0000",
        "remaining_usdc": "0.00",
        "rung": "deep",
        "status": "deployed",
        "target_liquidity": "14125167068081104",
        "target_nock": null,
        "target_usdc": "650.90",
        "text": "Deposit 3 of 3 (deep rung): done. It is deployed in v4 USDC/NOCK 3.00% #9000001, which holds $650.90 USDC and 0 NOCK now.",
        "tick_lower": 275400,
        "tick_upper": 276300,
        "tracked_liquidity": "14125167068081104"
      }
    ],
    "source": "operator_profile.ladder_rungs",
    "status": "computed",
    "total_usdc": "766.99",
    "tracked_usdc": "823.20",
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
    "deposits_usdc": "1.22",
    "existing_ranges_usdc_at_gate": "171.16",
    "existing_ranges_usdc_now": "1046.90",
    "gate_price_usd": "0.01000000",
    "rule": "the standing reserve is measured on wallet USDC plus USDC inside NOCK ranges, read at the gate price",
    "stable_at_gate_usdc": "952.58",
    "text": "After these deposits your wallet holds about $781 USDC; the $950 reserve counts the $1,047 of USDC inside your existing NOCK ranges. At $0.01 with every rung converted, the stable balance reads $952.58: the wallet's $781.42 plus $171.16 still inside those ranges.",
    "wallet_alone_below_reserve": true,
    "wallet_usdc_after": "781.42",
    "wallet_usdc_now": "782.64"
  },
  "what_changed": "Your ladder shape has 1 of 3 deposits left in the USDC/NOCK 3% v4 pool: up to $1.22 USDC and about 87 NOCK."
} as any;

/** deep_deployed_up_8_ticks: the deep rung deployed, then the price up 8 ticks: the cap takes what the reserve leaves */
export const LADDER_BOTTLE_DEEP_UP_8: LadderSuggestion = {
  "after_deposit": {
    "detection_note": null,
    "detection_status": null,
    "kind": "tracked_automatically",
    "text": "After you deposit there is nothing to copy or configure. NockNock looks for the wallet's new position NFT on-chain at the start of every cycle (CHECK NOW runs one at once), so a deposit is normally found in the first cycle after it lands. A position your wallet minted in this pool with exactly a rung's ticks is tracked as a ladder by itself, with its mint as the baseline. Any other range it finds is shown as found on-chain and is not counted until one tap says how to track it."
  },
  "assumptions": [
    "The amounts are your own shape (operator_profile.ladder_rungs): until a near or deep rung is deposited it is sized to what the standing reserve allows at the gate price; from then on that rung fixes the plan. When the reserve allows less than what is left, every deposit is cut in proportion and the shortfall is stated. Fees and gas are not counted in them.",
    "A cap is in range now and holds NOCK; every other rung holds only USDC now and earns no fees until NOCK falls to its top.",
    "Each rung is measured by its liquidity, which does not change with the price: a deposited rung stays deposited when the price moves, and only what is left is listed. A remainder under $1.00 is not offered.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "Each fee line is a past measurement on our own tracked range scaled to the rung's liquidity: an estimate, not a forecast."
  ],
  "candidates": null,
  "excluded": [],
  "fit": {
    "allowed_usdc": "116.09",
    "needed_usdc": "116.09",
    "rule": "once a near or deep rung is deposited the cap takes only what the reserve leaves beside the near and deep rungs still to deposit; when they alone need more than the reserve allows, each deposit left is cut pro rata (by the same fraction), so the shape keeps its proportions, and the shortfall is stated",
    "shortfall_usdc": null,
    "status": "fits",
    "text": null
  },
  "found_note": null,
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
      "nock_needed": "78.1397",
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
  "headline": "Ladder: 2 of 3 deposits left, up to $116.08 USDC and about 78 NOCK.",
  "id": "ladder_suggestion:NOCK",
  "minimum_usdc": "1.00",
  "mint_note": "liquidity_raw is what these amounts buy, rounded down (LiquidityAmounts); amount0_raw and amount1_raw are what a mint of it pulls, rounded up (SqrtPriceMath), never more than the maxima (the rung's USDC and NOCK truncated to raw units)",
  "missing": [],
  "numbers": {
    "avg_buy_price_usd": "0.01183221",
    "binding": "reserve",
    "cash_reserve_usdc": "950.00",
    "existing_ranges_usdc_at_gate": "171.15",
    "existing_ranges_usdc_now": "874.78",
    "gate_price_usd": "0.01000000",
    "headroom_usdc": "116.09",
    "lp_nock_budget": "86.7891",
    "nock_at_bottom": "9889.8205",
    "nock_bought_at_bottom": "9811.6808",
    "nock_now": "78.1396",
    "pool_price_usd": "0.01295436",
    "shape_rest_usdc": "765.76",
    "shape_total_usdc": "766.99",
    "shape_tracked_usdc": "650.90",
    "stable_at_gate_usdc": "950.01",
    "usdc_max": "116.08",
    "wallet_usdc": "894.94",
    "wallet_usdc_after": "778.86"
  },
  "outcomes": [
    {
      "avg_buy_price_usd": "0.01183221",
      "kind": "fall_to_bottom",
      "nock_bought": "9812",
      "pct_of_spot": "-15%",
      "price_usd": "0.01096801",
      "text": "A fall to $0.01097 (-15%), the bottom of what is left to deposit, would buy about 9,812 NOCK in total with all $116.08, an average of $0.01183.",
      "usdc_spent": "116.08"
    },
    {
      "kind": "rise",
      "nock_sold": "78",
      "pct_of_spot": "+1.4%",
      "price_usd": "0.01313098",
      "text": "A rise to $0.01313 (+1.4%) or above sells the cap's NOCK, about 78, for about $1.02 of USDC, and sells nothing else: every other rung holds 0 NOCK.",
      "usdc_received": "1.02"
    }
  ],
  "picture": {
    "bars": [
      {
        "deployed": false,
        "high_usd": "0.01313098",
        "index": 1,
        "label": "$1.22 + 78 NOCK",
        "low_usd": "0.01274292",
        "rung": "cap",
        "status": "to_deposit",
        "usdc": "1.22"
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
    "spot_usd": "0.01295436"
  },
  "reason": null,
  "rungs": [
    {
      "at_bottom": {
        "avg_buy_price_usd": "0.01284820",
        "nock_bought": "95.6743",
        "nock_held": "173.8140"
      },
      "bottom": {
        "pct_of_spot": "-1.63",
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
        "bound_by": "reserve",
        "note": "the budget's 86.7891 NOCK would pair with $1.37 USDC, more than the USDC the reserve leaves for it beside the other deposits listed, $1.23: the cap's liquidity is reduced until its USDC equals that, so it holds 78.1397 NOCK, less than the budget",
        "usdc_limit": "1.23",
        "usdc_limit_is": "reserve"
      },
      "deep_link": {
        "amount_to_enter": "Amount to enter in the app: $1.22 USDC, in the USDC field; the app then works out the NOCK side, about 78.1396 NOCK at this reading.",
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
        "dilution": {
          "share_of_active_pct": "0.16"
        },
        "dilution_reason": null,
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
        "per_day_in_range_diluted_usd": "0.80",
        "per_day_in_range_usd": "0.80",
        "reason": null,
        "replay_reason": null,
        "status": "measured",
        "stored_history_replay": {
          "edge_upper_usd": "0.14",
          "inside_upper_usd": "0.02"
        }
      },
      "fit_factor": null,
      "in_range_now": true,
      "index": 1,
      "mint": {
        "amount0_raw": "1229243",
        "amount1_raw": "781396383369252499",
        "liquidity_raw": "131796080146192"
      },
      "nock": {
        "address": "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722",
        "amount": "78.1396383369252499",
        "decimals": 16,
        "raw": "781396383369252499",
        "to_enter": "78.1396"
      },
      "nock_limit": {
        "binding": "usdc",
        "field_to_type": "USDC",
        "holds_less_than_budget": true,
        "is": "near_usdc",
        "nock": "86.7891",
        "nock_then_about": "78.1396",
        "text": "The USDC side must not exceed $1.23, what your reserve leaves for it beside the other deposits listed, which binds now, so the cap holds about 78.1396 NOCK, less than your 86.7891 NOCK LP budget. The limit holds when the USDC field is the one typed, at $1.22 or less. When the NOCK field is typed instead, the app works out the USDC side from the price at that moment, and it can come out above the limit; neither the link nor the app applies a USDC limit of its own.",
        "usdc": "1.23",
        "usdc_to_enter": "1.22"
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
        "pool_price_usd": "0.01295436",
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
      "summary": "about 78.14 NOCK + $1.22 USDC, in range now",
      "target": {
        "counterparts": [],
        "deployed_usdc": "0.00",
        "liquidity": "131796110999826",
        "tracked_liquidity": "0",
        "usdc": "1.23"
      },
      "tick_lower": 273600,
      "tick_upper": 273900,
      "top": {
        "pct_of_spot": "+1.36",
        "price_usd": "0.01313098",
        "rule": "the tick band containing the pool's tick (the cap)",
        "tick": 273600
      },
      "usdc": {
        "address": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
        "amount": "1.229243",
        "decimals": 6,
        "raw": "1229243",
        "to_enter": "1.22"
      },
      "would_sign": "In the Uniswap web app, with your own wallet: one new position in the USDC/NOCK 3% v4 pool, ticks 273600 to 273900, holding up to 78.1396 NOCK and about $1.22 USDC (the app may first ask for USDC and NOCK approvals). NockNock builds no transaction and signs nothing."
    },
    {
      "at_bottom": {
        "avg_buy_price_usd": "0.01182220",
        "nock_bought": "9716.0065",
        "nock_held": "9716.0065"
      },
      "bottom": {
        "pct_of_spot": "-15.33",
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
        "dilution": {
          "share_of_active_pct": "1.73"
        },
        "dilution_reason": null,
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
        "per_day_in_range_usd": "8.57",
        "reason": null,
        "replay_reason": null,
        "status": "measured",
        "stored_history_replay": {
          "edge_upper_usd": "5.16",
          "inside_upper_usd": "6.39"
        }
      },
      "fit_factor": null,
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
        "pool_price_usd": "0.01295436",
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
        "liquidity": "1408301615002245",
        "tracked_liquidity": "0",
        "usdc": "114.86"
      },
      "tick_lower": 273900,
      "tick_upper": 275400,
      "top": {
        "pct_of_spot": "-1.63",
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
  "sentence": "Your ladder shape in the USDC/NOCK 3% v4 pool: 2 of 3 deposits are left (1 deployed), holding up to $116.08 USDC and 78.14 NOCK now: sized to leave the stable balance at or above your $950 reserve at $0.01. Fully converted at $0.01097 they would hold about 9,890 NOCK, about 9,812 of it bought, at an average of $0.01183.",
  "shape": {
    "below_minimum": 0,
    "deployed": 1,
    "found": 0,
    "held": 0,
    "left": 2,
    "none": 0,
    "of": 3,
    "over": 0,
    "passed": 0,
    "rest_rule": "frozen by the deep rung already deposited (the deposit order is not known for every rung, so the one implying the most): its $650.90 of USDC at its top is its share of $765.76, which the near and deep shares divide; it does not move with the price",
    "rest_usdc": "765.76",
    "rungs": [
      {
        "bounds_usd": {
          "high": "0.01313098",
          "low": "0.01274292"
        },
        "counterparts": [],
        "deployed_nock": "0.0000",
        "deployed_usdc": "0.00",
        "found": [],
        "holds_now": null,
        "in_range_now": true,
        "index": 1,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "78.1397",
        "remaining_usdc": "1.22",
        "rung": "cap",
        "status": "to_deposit",
        "target_liquidity": "131796110999826",
        "target_nock": "78.1397",
        "target_usdc": "1.23",
        "text": "Deposit 1 of 3 (cap): left to deposit, about 78.14 NOCK + $1.22 USDC.",
        "tick_lower": 273600,
        "tick_upper": 273900,
        "tracked_liquidity": "0"
      },
      {
        "bounds_usd": {
          "high": "0.01274292",
          "low": "0.01096801"
        },
        "counterparts": [],
        "deployed_nock": null,
        "deployed_usdc": "0.00",
        "found": [],
        "holds_now": null,
        "in_range_now": false,
        "index": 2,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "0.0000",
        "remaining_usdc": "114.86",
        "rung": "near",
        "status": "to_deposit",
        "target_liquidity": "1408301615002245",
        "target_nock": null,
        "target_usdc": "114.86",
        "text": "Deposit 2 of 3 (near rung): left to deposit, $114.86 USDC.",
        "tick_lower": 273900,
        "tick_upper": 275400,
        "tracked_liquidity": "0"
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
        "found": [],
        "holds_now": {
          "nock": "0.0000",
          "usdc": "650.90"
        },
        "in_range_now": false,
        "index": 3,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "0.0000",
        "remaining_usdc": "0.00",
        "rung": "deep",
        "status": "deployed",
        "target_liquidity": "14125167068081104",
        "target_nock": null,
        "target_usdc": "650.90",
        "text": "Deposit 3 of 3 (deep rung): done. It is deployed in v4 USDC/NOCK 3.00% #9000001, which holds $650.90 USDC and 0 NOCK now.",
        "tick_lower": 275400,
        "tick_upper": 276300,
        "tracked_liquidity": "14125167068081104"
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
    "deposits_usdc": "116.08",
    "existing_ranges_usdc_at_gate": "171.15",
    "existing_ranges_usdc_now": "874.78",
    "gate_price_usd": "0.01000000",
    "rule": "the standing reserve is measured on wallet USDC plus USDC inside NOCK ranges, read at the gate price",
    "stable_at_gate_usdc": "950.01",
    "text": "After these deposits your wallet holds about $779 USDC; the $950 reserve counts the $875 of USDC inside your existing NOCK ranges. At $0.01 with every rung converted, the stable balance reads $950.01: the wallet's $778.86 plus $171.15 still inside those ranges.",
    "wallet_alone_below_reserve": true,
    "wallet_usdc_after": "778.86",
    "wallet_usdc_now": "894.94"
  },
  "what_changed": "Your ladder shape has 2 of 3 deposits left in the USDC/NOCK 3% v4 pool: up to $116.08 USDC and about 78 NOCK."
} as any;

/** cap_deployed_above_its_band: the cap deployed, then the price above its band: done, near and deep listed */
export const LADDER_BOTTLE_CAP_ABOVE: LadderSuggestion = {
  "after_deposit": {
    "detection_note": null,
    "detection_status": null,
    "kind": "tracked_automatically",
    "text": "After you deposit there is nothing to copy or configure. NockNock looks for the wallet's new position NFT on-chain at the start of every cycle (CHECK NOW runs one at once), so a deposit is normally found in the first cycle after it lands. A position your wallet minted in this pool with exactly a rung's ticks is tracked as a ladder by itself, with its mint as the baseline. Any other range it finds is shown as found on-chain and is not counted until one tap says how to track it."
  },
  "assumptions": [
    "The amounts are your own shape (operator_profile.ladder_rungs): until a near or deep rung is deposited it is sized to what the standing reserve allows at the gate price; from then on that rung fixes the plan. When the reserve allows less than what is left, every deposit is cut in proportion and the shortfall is stated. Fees and gas are not counted in them.",
    "A cap is in range now and holds NOCK; every other rung holds only USDC now and earns no fees until NOCK falls to its top.",
    "Each rung is measured by its liquidity, which does not change with the price: a deposited rung stays deposited when the price moves, and only what is left is listed. A remainder under $1.00 is not offered.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "Each fee line is a past measurement on our own tracked range scaled to the rung's liquidity: an estimate, not a forecast."
  ],
  "candidates": null,
  "excluded": [],
  "fit": {
    "allowed_usdc": "765.76",
    "needed_usdc": "765.76",
    "rule": "once a near or deep rung is deposited the cap takes only what the reserve leaves beside the near and deep rungs still to deposit; when they alone need more than the reserve allows, each deposit left is cut pro rata (by the same fraction), so the shape keeps its proportions, and the shortfall is stated",
    "shortfall_usdc": null,
    "status": "fits",
    "text": null
  },
  "found_note": null,
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
      "margin_usdc": "0.01",
      "max_usdc": "765.76",
      "minimum_usdc": "1.00",
      "read_at_rule": "your ladder bottom (operator_profile.ladder_bottom_usd $0.01000000), below the deepest rung's bottom",
      "read_at_usd": "0.01000000",
      "rule": "the stable balance at the gate price stays at or above the reserve",
      "shape_total_usdc": "768.12",
      "stable_at_gate_usdc": "950.01",
      "status": "allows"
    },
    "shape_setting": {
      "key": "operator_profile.ladder_rungs",
      "status": "set"
    }
  },
  "headline": "Ladder: 2 of 3 deposits left, up to $765.75 USDC.",
  "id": "ladder_suggestion:NOCK",
  "minimum_usdc": "1.00",
  "mint_note": "liquidity_raw is what these amounts buy, rounded down (LiquidityAmounts); amount0_raw and amount1_raw are what a mint of it pulls, rounded up (SqrtPriceMath), never more than the maxima (the rung's USDC and NOCK truncated to raw units)",
  "missing": [],
  "numbers": {
    "avg_buy_price_usd": "0.01068786",
    "binding": "reserve",
    "cash_reserve_usdc": "950.00",
    "existing_ranges_usdc_at_gate": "171.16",
    "existing_ranges_usdc_now": "229.55",
    "gate_price_usd": "0.01000000",
    "headroom_usdc": "765.76",
    "lp_nock_budget": "0.0000",
    "nock_at_bottom": "71647.6257",
    "nock_bought_at_bottom": "71647.6257",
    "nock_now": "0.0000",
    "pool_price_usd": "0.01315276",
    "shape_rest_usdc": "765.76",
    "shape_total_usdc": "768.12",
    "shape_tracked_usdc": "2.36",
    "stable_at_gate_usdc": "950.02",
    "usdc_max": "765.75",
    "wallet_usdc": "1544.61",
    "wallet_usdc_after": "778.86"
  },
  "outcomes": [
    {
      "avg_buy_price_usd": "0.01200086",
      "kind": "fall_inside",
      "nock_bought": "9571",
      "pct_of_spot": "-17%",
      "price_usd": "0.01096801",
      "text": "A fall to $0.01097 (-17%), the near rung's bottom, would buy about 9,571 NOCK for $114.86 of the USDC, an average of $0.01200.",
      "usdc_spent": "114.86"
    },
    {
      "avg_buy_price_usd": "0.01068786",
      "kind": "fall_to_bottom",
      "nock_bought": "71648",
      "pct_of_spot": "-24%",
      "price_usd": "0.01002405",
      "text": "A fall to $0.01002 (-24%), the bottom of what is left to deposit, would buy about 71,648 NOCK in total with all $765.75, an average of $0.01069.",
      "usdc_spent": "765.75"
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
        "label": "$2.36",
        "low_usd": "0.01274292",
        "rung": "cap",
        "status": "deployed",
        "usdc": "2.36"
      },
      {
        "deployed": false,
        "high_usd": "0.01313098",
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
    "spot_usd": "0.01315276"
  },
  "reason": null,
  "rungs": [
    {
      "at_bottom": {
        "avg_buy_price_usd": "0.01200086",
        "nock_bought": "9571.3112",
        "nock_held": "9571.3112"
      },
      "bottom": {
        "pct_of_spot": "-16.61",
        "price_usd": "0.01096801",
        "tick": 275400
      },
      "bounds_nock_per_usdc": {
        "high": "91.1742",
        "low": "76.1558"
      },
      "bounds_usd": {
        "high": "0.01313098",
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
        "url": "https://app.uniswap.org/positions/add/new?chain=base&protocolVersion=v4&currencyA=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913&currencyB=0x9B5E262cF9bb04869ab40b19AF91D2dc85761722&fee=%7B%22feeAmount%22%3A30000%2C%22isDynamic%22%3Afalse%2C%22tickSpacing%22%3A300%7D&priceRangeState=%7B%22fullRange%22%3Afalse%2C%22maxTick%22%3A275400%2C%22minTick%22%3A273600%2C%22priceInverted%22%3Afalse%7D&step=1"
      },
      "fee_estimate": {
        "coverage": {
          "note": "the whole range lies inside the measuring range",
          "whole": true
        },
        "dilution": {
          "share_of_active_pct": "1.43"
        },
        "dilution_reason": null,
        "in_range_share_reason": null,
        "in_range_share_stored_history": {
          "of_seconds": "547205",
          "seconds": "167621",
          "share_pct": "30.63"
        },
        "intervals": 1836,
        "measured_at": "2026-11-20T11:50:00Z",
        "measured_by": "uniswap-v4:base:3163450",
        "measured_days": "6.33",
        "measured_label": "v4 USDC/NOCK 3.00% #3163450",
        "per_day_in_range_diluted_usd": "7.03",
        "per_day_in_range_usd": "7.14",
        "reason": null,
        "replay_reason": null,
        "status": "measured",
        "stored_history_replay": {
          "edge_upper_usd": "5.31",
          "inside_upper_usd": "5.66"
        }
      },
      "fit_factor": null,
      "in_range_now": false,
      "index": 2,
      "mint": {
        "amount0_raw": "114863999",
        "amount1_raw": "0",
        "liquidity_raw": "1164689971849458"
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
        "pool_price_usd": "0.01315276",
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
        "liquidity": "1164689981987556",
        "tracked_liquidity": "0",
        "usdc": "114.86"
      },
      "tick_lower": 273600,
      "tick_upper": 275400,
      "top": {
        "pct_of_spot": "-0.17",
        "price_usd": "0.01313098",
        "rule": "the usable tick at or below the pool price (the first rung still drawable)",
        "tick": 273600
      },
      "usdc": {
        "address": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
        "amount": "114.863999",
        "decimals": 6,
        "raw": "114863999",
        "to_enter": "114.86"
      },
      "would_sign": "In the Uniswap web app, with your own wallet: one new position in the USDC/NOCK 3% v4 pool, ticks 273600 to 275400, holding up to $114.86 USDC and 0 NOCK (the app may first ask for a USDC approval). NockNock builds no transaction and signs nothing."
    },
    {
      "at_bottom": {
        "avg_buy_price_usd": "0.01048542",
        "nock_bought": "62076.3146",
        "nock_held": "62076.3146"
      },
      "bottom": {
        "pct_of_spot": "-23.79",
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
        "dilution": {
          "share_of_active_pct": "14.98"
        },
        "dilution_reason": null,
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
        "per_day_in_range_diluted_usd": "73.59",
        "per_day_in_range_usd": "86.55",
        "reason": null,
        "replay_reason": null,
        "status": "measured",
        "stored_history_replay": {
          "edge_upper_usd": "50.53",
          "inside_upper_usd": "32.80"
        }
      },
      "fit_factor": null,
      "in_range_now": false,
      "index": 3,
      "mint": {
        "amount0_raw": "650896000",
        "amount1_raw": "0",
        "liquidity_raw": "14125093284685342"
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
        "pool_price_usd": "0.01315276",
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
        "liquidity": "14125093284665436",
        "tracked_liquidity": "0",
        "usdc": "650.90"
      },
      "tick_lower": 275400,
      "tick_upper": 276300,
      "top": {
        "pct_of_spot": "-16.61",
        "price_usd": "0.01096801",
        "rule": "the nearest usable tick",
        "tick": 275400
      },
      "usdc": {
        "address": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
        "amount": "650.896000",
        "decimals": 6,
        "raw": "650896000",
        "to_enter": "650.89"
      },
      "would_sign": "In the Uniswap web app, with your own wallet: one new position in the USDC/NOCK 3% v4 pool, ticks 275400 to 276300, holding up to $650.89 USDC and 0 NOCK (the app may first ask for a USDC approval). NockNock builds no transaction and signs nothing."
    }
  ],
  "sentence": "Your ladder shape in the USDC/NOCK 3% v4 pool: 2 of 3 deposits are left (1 deployed), holding up to $765.75 USDC and 0 NOCK now: sized to leave the stable balance at or above your $950 reserve at $0.01. Fully converted at $0.01002 they would hold about 71,648 NOCK, about 71,648 of it bought, at an average of $0.01069.",
  "shape": {
    "below_minimum": 0,
    "deployed": 1,
    "found": 0,
    "held": 0,
    "left": 2,
    "none": 0,
    "of": 3,
    "over": 0,
    "passed": 0,
    "rest_rule": "what the reserve gate allows now, less the cap's USDC: no near or deep rung is deposited yet, so this follows the gate until one is",
    "rest_usdc": "765.76",
    "rungs": [
      {
        "bounds_usd": {
          "high": "0.01313098",
          "low": "0.01274292"
        },
        "counterparts": [
          "uniswap-v4:base:9000002"
        ],
        "deployed_nock": "0.0000",
        "deployed_usdc": "2.36",
        "found": [],
        "holds_now": {
          "nock": "0.0000",
          "usdc": "2.36"
        },
        "in_range_now": false,
        "index": 1,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "0.0000",
        "remaining_usdc": "0.00",
        "rung": "cap",
        "status": "deployed",
        "target_liquidity": "138192109253769",
        "target_nock": "0.0000",
        "target_usdc": "2.36",
        "text": "Deposit 1 of 3 (cap): done. It is deployed in v4 USDC/NOCK 3.00% #9000002, which holds $2.36 USDC and 0 NOCK now.",
        "tick_lower": 273600,
        "tick_upper": 273900,
        "tracked_liquidity": "138192109253769"
      },
      {
        "bounds_usd": {
          "high": "0.01313098",
          "low": "0.01096801"
        },
        "counterparts": [],
        "deployed_nock": null,
        "deployed_usdc": "0.00",
        "found": [],
        "holds_now": null,
        "in_range_now": false,
        "index": 2,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "0.0000",
        "remaining_usdc": "114.86",
        "rung": "near",
        "status": "to_deposit",
        "target_liquidity": "1164689981987556",
        "target_nock": null,
        "target_usdc": "114.86",
        "text": "Deposit 2 of 3 (near rung): left to deposit, $114.86 USDC.",
        "tick_lower": 273600,
        "tick_upper": 275400,
        "tracked_liquidity": "0"
      },
      {
        "bounds_usd": {
          "high": "0.01096801",
          "low": "0.01002405"
        },
        "counterparts": [],
        "deployed_nock": null,
        "deployed_usdc": "0.00",
        "found": [],
        "holds_now": null,
        "in_range_now": false,
        "index": 3,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "0.0000",
        "remaining_usdc": "650.89",
        "rung": "deep",
        "status": "to_deposit",
        "target_liquidity": "14125093284665436",
        "target_nock": null,
        "target_usdc": "650.90",
        "text": "Deposit 3 of 3 (deep rung): left to deposit, $650.89 USDC.",
        "tick_lower": 275400,
        "tick_upper": 276300,
        "tracked_liquidity": "0"
      }
    ],
    "source": "operator_profile.ladder_rungs",
    "status": "computed",
    "total_usdc": "768.12",
    "tracked_usdc": "2.36",
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
      "state": "holds_only_usdc"
    }
  ],
  "wallet_after": {
    "cash_reserve_usdc": "950.00",
    "deposits_usdc": "765.75",
    "existing_ranges_usdc_at_gate": "171.16",
    "existing_ranges_usdc_now": "229.55",
    "gate_price_usd": "0.01000000",
    "rule": "the standing reserve is measured on wallet USDC plus USDC inside NOCK ranges, read at the gate price",
    "stable_at_gate_usdc": "950.02",
    "text": "After these deposits your wallet holds about $779 USDC; the $950 reserve counts the $230 of USDC inside your existing NOCK ranges. At $0.01 with every rung converted, the stable balance reads $950.02: the wallet's $778.86 plus $171.16 still inside those ranges.",
    "wallet_alone_below_reserve": true,
    "wallet_usdc_after": "778.86",
    "wallet_usdc_now": "1544.61"
  },
  "what_changed": "Your ladder shape has 2 of 3 deposits left in the USDC/NOCK 3% v4 pool: up to $765.75 USDC."
} as any;

/** near_found_untracked: the near rung minted and found on-chain, not yet tracked: not offered again */
export const LADDER_BOTTLE_FOUND: LadderSuggestion = {
  "after_deposit": {
    "detection_note": null,
    "detection_status": "ok",
    "kind": "tracked_automatically",
    "text": "After you deposit there is nothing to copy or configure. NockNock looks for the wallet's new position NFT on-chain at the start of every cycle (CHECK NOW runs one at once), so a deposit is normally found in the first cycle after it lands. A position your wallet minted in this pool with exactly a rung's ticks is tracked as a ladder by itself, with its mint as the baseline. Any other range it finds is shown as found on-chain and is not counted until one tap says how to track it."
  },
  "assumptions": [
    "The amounts are your own shape (operator_profile.ladder_rungs): until a near or deep rung is deposited it is sized to what the standing reserve allows at the gate price; from then on that rung fixes the plan. When the reserve allows less than what is left, every deposit is cut in proportion and the shortfall is stated. Fees and gas are not counted in them.",
    "A cap is in range now and holds NOCK; every other rung holds only USDC now and earns no fees until NOCK falls to its top.",
    "Each rung is measured by its liquidity, which does not change with the price: a deposited rung stays deposited when the price moves, and only what is left is listed. A remainder under $1.00 is not offered.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "Each fee line is a past measurement on our own tracked range scaled to the rung's liquidity: an estimate, not a forecast."
  ],
  "candidates": null,
  "excluded": [],
  "fit": {
    "allowed_usdc": "652.12",
    "needed_usdc": "652.13",
    "rule": "once a near or deep rung is deposited the cap takes only what the reserve leaves beside the near and deep rungs still to deposit; when they alone need more than the reserve allows, each deposit left is cut pro rata (by the same fraction), so the shape keeps its proportions, and the shortfall is stated",
    "shortfall_usdc": null,
    "status": "fits",
    "text": null
  },
  "found_note": null,
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
      "max_usdc": "652.12",
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
  "headline": "Ladder: 2 of 3 deposits left, up to $652.11 USDC and about 87 NOCK.",
  "id": "ladder_suggestion:NOCK",
  "minimum_usdc": "1.00",
  "mint_note": "liquidity_raw is what these amounts buy, rounded down (LiquidityAmounts); amount0_raw and amount1_raw are what a mint of it pulls, rounded up (SqrtPriceMath), never more than the maxima (the rung's USDC and NOCK truncated to raw units)",
  "missing": [],
  "numbers": {
    "avg_buy_price_usd": "0.01048904",
    "binding": "reserve",
    "cash_reserve_usdc": "950.00",
    "existing_ranges_usdc_at_gate": "171.16",
    "existing_ranges_usdc_now": "223.71",
    "gate_price_usd": "0.01000000",
    "headroom_usdc": "652.12",
    "lp_nock_budget": "86.7891",
    "nock_at_bottom": "62258.8879",
    "nock_bought_at_bottom": "62172.0988",
    "nock_now": "86.7891",
    "pool_price_usd": "0.01294400",
    "shape_rest_usdc": "765.76",
    "shape_total_usdc": "766.99",
    "shape_tracked_usdc": "0.00",
    "stable_at_gate_usdc": "950.02",
    "usdc_max": "652.11",
    "wallet_usdc": "1430.97",
    "wallet_usdc_after": "778.86"
  },
  "outcomes": [
    {
      "avg_buy_price_usd": "0.01048904",
      "kind": "fall_to_bottom",
      "nock_bought": "62172",
      "pct_of_spot": "-23%",
      "price_usd": "0.01002405",
      "text": "A fall to $0.01002 (-23%), the bottom of what is left to deposit, would buy about 62,172 NOCK in total with all $652.11, an average of $0.01049.",
      "usdc_spent": "652.11"
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
        "label": "$1.22 + 87 NOCK",
        "low_usd": "0.01274292",
        "rung": "cap",
        "status": "to_deposit",
        "usdc": "1.22"
      },
      {
        "deployed": true,
        "high_usd": "0.01274292",
        "index": 2,
        "label": "$114.86",
        "low_usd": "0.01096801",
        "rung": "near",
        "status": "found",
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
        "note": "the cap holds the whole LP NOCK budget, 86.7891 NOCK; the curve pairs it with $1.23 USDC",
        "usdc_limit": "1.23",
        "usdc_limit_is": "reserve"
      },
      "deep_link": {
        "amount_to_enter": "Amount to enter in the app: 86.7890 NOCK, in the NOCK field; the app then works out the USDC side, about $1.22 at this reading.",
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
        "dilution": {
          "share_of_active_pct": "0.17"
        },
        "dilution_reason": null,
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
      "fit_factor": null,
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
        "binding": "nock",
        "field_to_type": "NOCK",
        "holds_less_than_budget": false,
        "is": "lp_nock_budget",
        "nock": "86.7891",
        "nock_to_enter": "86.7890",
        "text": "The NOCK side must not exceed 86.7891 NOCK, your LP NOCK budget, which binds now; the USDC side must not exceed $1.23, what your reserve leaves for it beside the other deposits listed. The limits hold when the NOCK field is the one typed, at 86.7890 NOCK or less, and the app shows at most $1.23 of USDC: about $1.22 at this reading, more if the price rises inside the band before you deposit. When the USDC field is typed instead, the app works out the NOCK side from the price at that moment, and it can come out above the limit; neither the link nor the app applies a NOCK limit of its own.",
        "usdc": "1.23",
        "usdc_then_about": "1.22"
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
      "summary": "about 86.79 NOCK + $1.22 USDC, in range now",
      "target": {
        "counterparts": [],
        "deployed_usdc": "0.00",
        "liquidity": "138192115820500",
        "tracked_liquidity": "0",
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
      "would_sign": "In the Uniswap web app, with your own wallet: one new position in the USDC/NOCK 3% v4 pool, ticks 273600 to 273900, holding up to 86.7890 NOCK and about $1.22 USDC (the app may first ask for USDC and NOCK approvals). NockNock builds no transaction and signs nothing."
    },
    {
      "at_bottom": {
        "avg_buy_price_usd": "0.01048542",
        "nock_bought": "62076.6388",
        "nock_held": "62076.6388"
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
        "dilution": {
          "share_of_active_pct": "14.98"
        },
        "dilution_reason": null,
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
      "fit_factor": null,
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
        "liquidity": "14125167068081028",
        "tracked_liquidity": "0",
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
  "sentence": "Your ladder shape in the USDC/NOCK 3% v4 pool: 2 of 3 deposits are left (1 found on-chain, being tracked), holding up to $652.11 USDC and 86.79 NOCK now: sized to leave the stable balance at or above your $950 reserve at $0.01. Fully converted at $0.01002 they would hold about 62,259 NOCK, about 62,172 of it bought, at an average of $0.01049.",
  "shape": {
    "below_minimum": 0,
    "deployed": 0,
    "found": 1,
    "held": 0,
    "left": 2,
    "none": 0,
    "of": 3,
    "over": 0,
    "passed": 0,
    "rest_rule": "frozen by the near rung already deposited (the first deposited, by its deposit block): its $114.86 of USDC at its top is its share of $765.76, which the near and deep shares divide; it does not move with the price",
    "rest_usdc": "765.76",
    "rungs": [
      {
        "bounds_usd": {
          "high": "0.01313098",
          "low": "0.01274292"
        },
        "counterparts": [],
        "deployed_nock": "0.0000",
        "deployed_usdc": "0.00",
        "found": [],
        "holds_now": null,
        "in_range_now": true,
        "index": 1,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "86.7891",
        "remaining_usdc": "1.22",
        "rung": "cap",
        "status": "to_deposit",
        "target_liquidity": "138192115820500",
        "target_nock": "86.7891",
        "target_usdc": "1.23",
        "text": "Deposit 1 of 3 (cap): left to deposit, about 86.79 NOCK + $1.22 USDC.",
        "tick_lower": 273600,
        "tick_upper": 273900,
        "tracked_liquidity": "0"
      },
      {
        "bounds_usd": {
          "high": "0.01274292",
          "low": "0.01096801"
        },
        "counterparts": [],
        "deployed_nock": null,
        "deployed_usdc": "114.86",
        "found": [
          "uniswap-v4:base:9000003"
        ],
        "holds_now": null,
        "in_range_now": false,
        "index": 2,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": "the position uniswap-v4:base:9000003, which your wallet minted, is this rung in this pool; NockNock tracks it by itself and counts it from then, so it is not offered again",
        "remaining_nock": "0.0000",
        "remaining_usdc": "0.00",
        "rung": "near",
        "status": "found",
        "target_liquidity": "1408301615002237",
        "target_nock": null,
        "target_usdc": "114.86",
        "text": "Deposit 2 of 3 (near rung): found on-chain, being tracked. The position uniswap-v4:base:9000003, which your wallet minted, is this rung in this pool; NockNock tracks it by itself and counts it from then, so it is not offered again.",
        "tick_lower": 273900,
        "tick_upper": 275400,
        "tracked_liquidity": "1408301615002237"
      },
      {
        "bounds_usd": {
          "high": "0.01096801",
          "low": "0.01002405"
        },
        "counterparts": [],
        "deployed_nock": null,
        "deployed_usdc": "0.00",
        "found": [],
        "holds_now": null,
        "in_range_now": false,
        "index": 3,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "0.0000",
        "remaining_usdc": "650.89",
        "rung": "deep",
        "status": "to_deposit",
        "target_liquidity": "14125167068081028",
        "target_nock": null,
        "target_usdc": "650.90",
        "text": "Deposit 3 of 3 (deep rung): left to deposit, $650.89 USDC.",
        "tick_lower": 275400,
        "tick_upper": 276300,
        "tracked_liquidity": "0"
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
    "deposits_usdc": "652.11",
    "existing_ranges_usdc_at_gate": "171.16",
    "existing_ranges_usdc_now": "223.71",
    "gate_price_usd": "0.01000000",
    "rule": "the standing reserve is measured on wallet USDC plus USDC inside NOCK ranges, read at the gate price",
    "stable_at_gate_usdc": "950.02",
    "text": "After these deposits your wallet holds about $779 USDC; the $950 reserve counts the $224 of USDC inside your existing NOCK ranges. At $0.01 with every rung converted, the stable balance reads $950.02: the wallet's $778.86 plus $171.16 still inside those ranges.",
    "wallet_alone_below_reserve": true,
    "wallet_usdc_after": "778.86",
    "wallet_usdc_now": "1430.97"
  },
  "what_changed": "Your ladder shape has 2 of 3 deposits left in the USDC/NOCK 3% v4 pool: up to $652.11 USDC and about 87 NOCK."
} as any;

/** near_cut_by_the_reserve: the near rung cut pro rata by fifty cents, the shortfall stated */
export const LADDER_BOTTLE_REDUCED: LadderSuggestion = {
  "after_deposit": {
    "detection_note": null,
    "detection_status": null,
    "kind": "tracked_automatically",
    "text": "After you deposit there is nothing to copy or configure. NockNock looks for the wallet's new position NFT on-chain at the start of every cycle (CHECK NOW runs one at once), so a deposit is normally found in the first cycle after it lands. A position your wallet minted in this pool with exactly a rung's ticks is tracked as a ladder by itself, with its mint as the baseline. Any other range it finds is shown as found on-chain and is not counted until one tap says how to track it."
  },
  "assumptions": [
    "The amounts are your own shape (operator_profile.ladder_rungs): until a near or deep rung is deposited it is sized to what the standing reserve allows at the gate price; from then on that rung fixes the plan. When the reserve allows less than what is left, every deposit is cut in proportion and the shortfall is stated. Fees and gas are not counted in them.",
    "A cap is in range now and holds NOCK; every other rung holds only USDC now and earns no fees until NOCK falls to its top.",
    "Each rung is measured by its liquidity, which does not change with the price: a deposited rung stays deposited when the price moves, and only what is left is listed. A remainder under $1.00 is not offered.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "Each fee line is a past measurement on our own tracked range scaled to the rung's liquidity: an estimate, not a forecast."
  ],
  "candidates": null,
  "excluded": [],
  "fit": {
    "allowed_usdc": "114.36",
    "needed_usdc": "114.86",
    "rule": "once a near or deep rung is deposited the cap takes only what the reserve leaves beside the near and deep rungs still to deposit; when they alone need more than the reserve allows, each deposit left is cut pro rata (by the same fraction), so the shape keeps its proportions, and the shortfall is stated",
    "shortfall_usdc": "0.50",
    "status": "reduced",
    "text": "Your $950 reserve, kept for below $0.01, allows $114.36 for what is left of your shape, $0.50 short of the $114.86 it would take: each deposit is cut by the same fraction, so the shape keeps its proportions."
  },
  "found_note": null,
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
      "max_usdc": "114.36",
      "minimum_usdc": "1.00",
      "read_at_rule": "your ladder bottom (operator_profile.ladder_bottom_usd $0.01000000), below the deepest rung's bottom",
      "read_at_usd": "0.01000000",
      "rule": "the stable balance at the gate price stays at or above the reserve",
      "shape_total_usdc": "766.99",
      "stable_at_gate_usdc": "950.00",
      "status": "reduces"
    },
    "shape_setting": {
      "key": "operator_profile.ladder_rungs",
      "status": "set"
    }
  },
  "headline": "Ladder: 1 of 3 deposits left, up to $114.36 USDC.",
  "id": "ladder_suggestion:NOCK",
  "minimum_usdc": "1.00",
  "mint_note": "liquidity_raw is what these amounts buy, rounded down (LiquidityAmounts); amount0_raw and amount1_raw are what a mint of it pulls, rounded up (SqrtPriceMath), never more than the maxima (the rung's USDC and NOCK truncated to raw units)",
  "missing": [],
  "numbers": {
    "avg_buy_price_usd": "0.01182220",
    "binding": "reserve",
    "cash_reserve_usdc": "950.00",
    "existing_ranges_usdc_at_gate": "171.16",
    "existing_ranges_usdc_now": "875.83",
    "gate_price_usd": "0.01000000",
    "headroom_usdc": "114.36",
    "lp_nock_budget": "0.0000",
    "nock_at_bottom": "9673.9876",
    "nock_bought_at_bottom": "9673.9876",
    "nock_now": "0.0000",
    "pool_price_usd": "0.01294400",
    "shape_rest_usdc": "765.76",
    "shape_total_usdc": "766.99",
    "shape_tracked_usdc": "652.13",
    "stable_at_gate_usdc": "950.01",
    "usdc_max": "114.36",
    "wallet_usdc": "893.21",
    "wallet_usdc_after": "778.85"
  },
  "outcomes": [
    {
      "avg_buy_price_usd": "0.01182220",
      "kind": "fall_to_bottom",
      "nock_bought": "9674",
      "pct_of_spot": "-15%",
      "price_usd": "0.01096801",
      "text": "A fall to $0.01097 (-15%), the bottom of what is left to deposit, would buy about 9,674 NOCK in total with all $114.36, an average of $0.01182.",
      "usdc_spent": "114.36"
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
        "deployed": false,
        "high_usd": "0.01274292",
        "index": 2,
        "label": "$114.36",
        "low_usd": "0.01096801",
        "rung": "near",
        "status": "to_deposit",
        "usdc": "114.36"
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
        "avg_buy_price_usd": "0.01182220",
        "nock_bought": "9673.9876",
        "nock_held": "9673.9876"
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
        "amount_to_enter": "Amount to enter in the app: $114.36 USDC",
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
        "dilution": {
          "share_of_active_pct": "1.72"
        },
        "dilution_reason": null,
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
        "per_day_in_range_diluted_usd": "8.38",
        "per_day_in_range_usd": "8.53",
        "reason": null,
        "replay_reason": null,
        "status": "measured",
        "stored_history_replay": {
          "edge_upper_usd": "5.13",
          "inside_upper_usd": "6.36"
        }
      },
      "fit_factor": "0.995675",
      "in_range_now": false,
      "index": 2,
      "mint": {
        "amount0_raw": "114367843",
        "amount1_raw": "0",
        "liquidity_raw": "1402211107697430"
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
      "summary": "$114.36 USDC, no NOCK, below the price now, after the reserve gate's cut",
      "target": {
        "counterparts": [],
        "deployed_usdc": "0.00",
        "liquidity": "1408301615002245",
        "tracked_liquidity": "0",
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
        "amount": "114.367843",
        "decimals": 6,
        "raw": "114367843",
        "to_enter": "114.36"
      },
      "would_sign": "In the Uniswap web app, with your own wallet: one new position in the USDC/NOCK 3% v4 pool, ticks 273900 to 275400, holding up to $114.36 USDC and 0 NOCK (the app may first ask for a USDC approval). NockNock builds no transaction and signs nothing."
    }
  ],
  "sentence": "Your ladder shape in the USDC/NOCK 3% v4 pool: 1 of 3 deposits is left (2 deployed), holding up to $114.36 USDC and 0 NOCK now: cut to what your $950 reserve at $0.01 allows. Fully converted at $0.01097 they would hold about 9,674 NOCK, about 9,674 of it bought, at an average of $0.01182.",
  "shape": {
    "below_minimum": 0,
    "deployed": 2,
    "found": 0,
    "held": 0,
    "left": 1,
    "none": 0,
    "of": 3,
    "over": 0,
    "passed": 0,
    "rest_rule": "frozen by the deep rung already deposited (the deposit order is not known for every rung, so the one implying the most): its $650.90 of USDC at its top is its share of $765.76, which the near and deep shares divide; it does not move with the price",
    "rest_usdc": "765.76",
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
        "found": [],
        "holds_now": {
          "nock": "86.7891",
          "usdc": "1.23"
        },
        "in_range_now": true,
        "index": 1,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "0.0000",
        "remaining_usdc": "0.00",
        "rung": "cap",
        "status": "deployed",
        "target_liquidity": "138192109253769",
        "target_nock": "86.7891",
        "target_usdc": "1.23",
        "text": "Deposit 1 of 3 (cap): done. It is deployed in v4 USDC/NOCK 3.00% #9000002, which holds $1.23 USDC and 87 NOCK now.",
        "tick_lower": 273600,
        "tick_upper": 273900,
        "tracked_liquidity": "138192109253769"
      },
      {
        "bounds_usd": {
          "high": "0.01274292",
          "low": "0.01096801"
        },
        "counterparts": [],
        "deployed_nock": null,
        "deployed_usdc": "0.00",
        "found": [],
        "holds_now": null,
        "in_range_now": false,
        "index": 2,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "0.0000",
        "remaining_usdc": "114.36",
        "rung": "near",
        "status": "to_deposit",
        "target_liquidity": "1408301615002245",
        "target_nock": null,
        "target_usdc": "114.86",
        "text": "Deposit 2 of 3 (near rung): left to deposit, $114.36 USDC after the reserve gate's cut.",
        "tick_lower": 273900,
        "tick_upper": 275400,
        "tracked_liquidity": "0"
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
        "found": [],
        "holds_now": {
          "nock": "0.0000",
          "usdc": "650.90"
        },
        "in_range_now": false,
        "index": 3,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "0.0000",
        "remaining_usdc": "0.00",
        "rung": "deep",
        "status": "deployed",
        "target_liquidity": "14125167068081104",
        "target_nock": null,
        "target_usdc": "650.90",
        "text": "Deposit 3 of 3 (deep rung): done. It is deployed in v4 USDC/NOCK 3.00% #9000001, which holds $650.90 USDC and 0 NOCK now.",
        "tick_lower": 275400,
        "tick_upper": 276300,
        "tracked_liquidity": "14125167068081104"
      }
    ],
    "source": "operator_profile.ladder_rungs",
    "status": "computed",
    "total_usdc": "766.99",
    "tracked_usdc": "652.13",
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
      "label": "v4 USDC/NOCK 3.00% #9000001",
      "position_id": "uniswap-v4:base:9000001",
      "rung": "deep",
      "state": "holds_only_usdc"
    }
  ],
  "wallet_after": {
    "cash_reserve_usdc": "950.00",
    "deposits_usdc": "114.36",
    "existing_ranges_usdc_at_gate": "171.16",
    "existing_ranges_usdc_now": "875.83",
    "gate_price_usd": "0.01000000",
    "rule": "the standing reserve is measured on wallet USDC plus USDC inside NOCK ranges, read at the gate price",
    "stable_at_gate_usdc": "950.01",
    "text": "After these deposits your wallet holds about $779 USDC; the $950 reserve counts the $876 of USDC inside your existing NOCK ranges. At $0.01 with every rung converted, the stable balance reads $950.01: the wallet's $778.85 plus $171.16 still inside those ranges.",
    "wallet_alone_below_reserve": true,
    "wallet_usdc_after": "778.85",
    "wallet_usdc_now": "893.21"
  },
  "what_changed": "Your ladder shape has 1 of 3 deposits left in the USDC/NOCK 3% v4 pool: up to $114.36 USDC."
} as any;

/** deep_held_for_nock: what is left of the deep rung needs NOCK the budget does not give */
export const LADDER_BOTTLE_HELD: LadderSuggestion = {
  "after_deposit": null,
  "assumptions": [
    "The amounts are your own shape (operator_profile.ladder_rungs): until a near or deep rung is deposited it is sized to what the standing reserve allows at the gate price; from then on that rung fixes the plan. When the reserve allows less than what is left, every deposit is cut in proportion and the shortfall is stated. Fees and gas are not counted in them.",
    "A cap is in range now and holds NOCK; every other rung holds only USDC now and earns no fees until NOCK falls to its top.",
    "Each rung is measured by its liquidity, which does not change with the price: a deposited rung stays deposited when the price moves, and only what is left is listed. A remainder under $1.00 is not offered.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "Each fee line is a past measurement on our own tracked range scaled to the rung's liquidity: an estimate, not a forecast."
  ],
  "candidates": null,
  "excluded": [],
  "fit": null,
  "found_note": null,
  "freshness_rule": "Shown only while the snapshot is fresh: the dashboard's sync status is not stale, or the worker's health is ok.",
  "gates": null,
  "headline": "Ladder: the range would need NOCK the floor budget does not allow.",
  "id": "ladder_suggestion:NOCK",
  "minimum_usdc": "1.00",
  "mint_note": null,
  "missing": [],
  "numbers": null,
  "outcomes": [],
  "picture": null,
  "reason": "Deposit 3 (deep rung): it would need 11679.2064 NOCK now, and the floor + tolerance budget has 1.0000 left for it. Nothing else is left to deposit.",
  "rungs": [],
  "sentence": null,
  "shape": {
    "below_minimum": 0,
    "deployed": 2,
    "found": 0,
    "held": 1,
    "left": 0,
    "none": 0,
    "of": 3,
    "over": 0,
    "passed": 0,
    "rest_rule": "frozen by the near rung already deposited (the first deposited, by its deposit block): its $114.86 of USDC at its top is its share of $765.76, which the near and deep shares divide; it does not move with the price",
    "rest_usdc": "765.76",
    "rungs": [
      {
        "bounds_usd": {
          "high": "0.01313098",
          "low": "0.01274292"
        },
        "counterparts": [
          "uniswap-v4:base:9000002"
        ],
        "deployed_nock": "182.2491",
        "deployed_usdc": "0.00",
        "found": [],
        "holds_now": {
          "nock": "182.2491",
          "usdc": "0.00"
        },
        "in_range_now": false,
        "index": 1,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "0.0000",
        "remaining_usdc": "0.00",
        "rung": "cap",
        "status": "deployed",
        "target_liquidity": "138192109253769",
        "target_nock": "182.2491",
        "target_usdc": "0.00",
        "text": "Deposit 1 of 3 (cap): done. It is deployed in v4 USDC/NOCK 3.00% #9000002, which holds $0.00 USDC and 182 NOCK now.",
        "tick_lower": 273600,
        "tick_upper": 273900,
        "tracked_liquidity": "138192109253769"
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
        "found": [],
        "holds_now": {
          "nock": "9716.0065",
          "usdc": "0.00"
        },
        "in_range_now": false,
        "index": 2,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": null,
        "remaining_nock": "0.0000",
        "remaining_usdc": "0.00",
        "rung": "near",
        "status": "deployed",
        "target_liquidity": "1408301615002237",
        "target_nock": null,
        "target_usdc": "114.86",
        "text": "Deposit 2 of 3 (near rung): done. It is deployed in v4 USDC/NOCK 3.00% #9000003, which holds $0.00 USDC and 9,716 NOCK now.",
        "tick_lower": 273900,
        "tick_upper": 275400,
        "tracked_liquidity": "1408301615002237"
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
        "deployed_usdc": "325.45",
        "found": [],
        "holds_now": {
          "nock": "11679.2064",
          "usdc": "199.53"
        },
        "in_range_now": true,
        "index": 3,
        "note": null,
        "of": 3,
        "over_by_usdc": null,
        "reason": "it would need 11679.2064 NOCK now, and the floor + tolerance budget has 1.0000 left for it",
        "remaining_nock": "11679.2064",
        "remaining_usdc": "199.53",
        "rung": "deep",
        "status": "held",
        "target_liquidity": "14125167068081028",
        "target_nock": null,
        "target_usdc": "650.90",
        "text": "Deposit 3 of 3 (deep rung): held back. It would need 11679.2064 NOCK now, and the floor + tolerance budget has 1.0000 left for it.",
        "tick_lower": 275400,
        "tick_upper": 276300,
        "tracked_liquidity": "7062583534040552"
      }
    ],
    "source": "operator_profile.ladder_rungs",
    "status": "held",
    "total_usdc": "765.76",
    "tracked_usdc": "199.53",
    "unmatched": []
  },
  "shown": false,
  "stated": null,
  "statement": "A measurement from this cycle's reads of what your ladder shape would do. NockNock does not choose the shape, the bottom or the amounts, builds no transaction, and signs and sends nothing.",
  "status": "nock_exceeds",
  "tier": "top",
  "token": "NOCK",
  "tracked": [
    {
      "label": "v4 USDC/NOCK 3.00% #9000002",
      "position_id": "uniswap-v4:base:9000002",
      "rung": "cap",
      "state": "holds_only_nock"
    },
    {
      "label": "v4 USDC/NOCK 3.00% #9000003",
      "position_id": "uniswap-v4:base:9000003",
      "rung": "near",
      "state": "holds_only_nock"
    },
    {
      "label": "v4 USDC/NOCK 3.00% #9000001",
      "position_id": "uniswap-v4:base:9000001",
      "rung": "deep",
      "state": "in_range"
    }
  ],
  "wallet_after": null,
  "what_changed": null
} as any;

/** bottle: the text both MCP transports print for it (get_shot_advice). */
export const LADDER_BOTTLE_TEXT: string = "• Ladder suggestion (a measurement, not advice): Ladder: 3 of 3 deposits left, up to $766.97 USDC and about 87 NOCK. Your ladder shape in the USDC/NOCK 3% v4 pool: 3 of 3 deposits are left, holding up to $766.97 USDC and 86.79 NOCK now: sized to leave the stable balance at or above your $950 reserve at $0.01. Fully converted at $0.01002 they would hold about 71,975 NOCK, about 71,888 of it bought, at an average of $0.01067.\n  - A fall to $0.01097 (-15%), the near rung's bottom, would buy about 9,811 NOCK for $116.09 of the USDC, an average of $0.01183.\n  - A fall to the ladder bottom $0.01002 (-23%) would buy about 71,888 NOCK in total with all $766.97, an average of $0.01067.\n  - A rise to $0.01313 (+1.4%) or above sells the cap's NOCK, about 87, for about $1.13 of USDC, and sells nothing else: every other rung holds 0 NOCK.\n  Reserve $950.00 holds at $0.01000000 (the reserve binds); NOCK budget: fits.\n  After these deposits your wallet holds about $779 USDC; the $950 reserve counts the $224 of USDC inside your existing NOCK ranges. At $0.01 with every rung converted, the stable balance reads $950.02: the wallet's $778.87 plus $171.15 still inside those ranges.\n  Stated, not gates: No accumulation plan is configured. Tax reserve: $0.00 (stated, not a gate). No cash target is set.\n  Assumptions: The amounts are your own shape (operator_profile.ladder_rungs): until a near or deep rung is deposited it is sized to what the standing reserve allows at the gate price; from then on that rung fixes the plan. When the reserve allows less than what is left, every deposit is cut in proportion and the shortfall is stated. Fees and gas are not counted in them. A cap is in range now and holds NOCK; every other rung holds only USDC now and earns no fees until NOCK falls to its top. Each rung is measured by its liquidity, which does not change with the price: a deposited rung stays deposited when the price moves, and only what is left is listed. A remainder under $1.00 is not offered. The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures. Each fee line is a past measurement on our own tracked range scaled to the rung's liquidity: an estimate, not a forecast.\n  Deposit 1 of 3 (cap): about 86.79 NOCK + $1.22 USDC, in range now. A new position in the USDC/NOCK 3% v4 pool, ticks 273600 to 273900 ($0.01274292 to $0.01313098; 76.1558 to 78.4750 NOCK per USDC).\n    Exact mint: liquidity 138192109253769, 1225999 raw USDC and 867890958577132038 raw NOCK.\n    Deposit link (pool and range prefilled): https://app.uniswap.org/positions/add/new?chain=base&protocolVersion=v4&currencyA=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913&currencyB=0x9B5E262cF9bb04869ab40b19AF91D2dc85761722&fee=%7B%22feeAmount%22%3A30000%2C%22isDynamic%22%3Afalse%2C%22tickSpacing%22%3A300%7D&priceRangeState=%7B%22fullRange%22%3Afalse%2C%22maxTick%22%3A273900%2C%22minTick%22%3A273600%2C%22priceInverted%22%3Afalse%7D&step=1\n    Amount to enter in the app: 86.7890 NOCK, in the NOCK field; the app then works out the USDC side, about $1.22 at this reading.\n    The NOCK side must not exceed 86.7891 NOCK, your LP NOCK budget, which binds now; the USDC side must not exceed $100.04, the most that keeps it at or below the near rung's USDC. The limits hold when the NOCK field is the one typed, at 86.7890 NOCK or less, and the app shows at most $100.04 of USDC: about $1.22 at this reading, more if the price rises inside the band before you deposit. When the USDC field is typed instead, the app works out the NOCK side from the price at that moment, and it can come out above the limit; neither the link nor the app applies a NOCK limit of its own.\n    Fees while in range (an estimate): $0.84/day undiluted, $0.84/day diluted (it would be 0.17% of the pool's active liquidity), measured over 6.33 days (1,836 intervals) on v4 USDC/NOCK 3.00% #3163450. In range 1.95% of the measured time. Replayed over it: at most $0.02 inside and $0.15 more at its edges (no lower bound).\n  Deposit 2 of 3 (near rung): $114.86 USDC, no NOCK, below the price now. A new position in the USDC/NOCK 3% v4 pool, ticks 273900 to 275400 ($0.01096801 to $0.01274292; 78.4750 to 91.1742 NOCK per USDC).\n    Exact mint: liquidity 1408301615002237, 114864600 raw USDC and 0 raw NOCK.\n    Deposit link (pool and range prefilled): https://app.uniswap.org/positions/add/new?chain=base&protocolVersion=v4&currencyA=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913&currencyB=0x9B5E262cF9bb04869ab40b19AF91D2dc85761722&fee=%7B%22feeAmount%22%3A30000%2C%22isDynamic%22%3Afalse%2C%22tickSpacing%22%3A300%7D&priceRangeState=%7B%22fullRange%22%3Afalse%2C%22maxTick%22%3A275400%2C%22minTick%22%3A273900%2C%22priceInverted%22%3Afalse%7D&step=1\n    Amount to enter in the app: $114.86 USDC\n    Fees while in range (an estimate): $8.56/day undiluted, $8.42/day diluted (it would be 1.73% of the pool's active liquidity), measured over 6.33 days (1,836 intervals) on v4 USDC/NOCK 3.00% #3163450. In range 28.63% of the measured time. Replayed over it: at most $6.39 inside and $5.16 more at its edges (no lower bound).\n  Deposit 3 of 3 (deep rung): $650.89 USDC, no NOCK, below the price now. A new position in the USDC/NOCK 3% v4 pool, ticks 275400 to 276300 ($0.01002405 to $0.01096801; 91.1742 to 99.7600 NOCK per USDC).\n    Exact mint: liquidity 14125167068081104, 650899400 raw USDC and 0 raw NOCK.\n    Deposit link (pool and range prefilled): https://app.uniswap.org/positions/add/new?chain=base&protocolVersion=v4&currencyA=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913&currencyB=0x9B5E262cF9bb04869ab40b19AF91D2dc85761722&fee=%7B%22feeAmount%22%3A30000%2C%22isDynamic%22%3Afalse%2C%22tickSpacing%22%3A300%7D&priceRangeState=%7B%22fullRange%22%3Afalse%2C%22maxTick%22%3A276300%2C%22minTick%22%3A275400%2C%22priceInverted%22%3Afalse%7D&step=1\n    Amount to enter in the app: $650.89 USDC\n    Fees while in range (an estimate): $85.90/day undiluted, $73.03/day diluted (it would be 14.98% of the pool's active liquidity), measured over 6.33 days (1,836 intervals) on v4 USDC/NOCK 3.00% #3163450. In range 5.82% of the measured time. Replayed over it: at most $32.55 inside and $50.00 more at its edges (no lower bound).\n  The link format is the app's own and undocumented: check the pool and range in the app against the figures here.\n  After you deposit: After you deposit there is nothing to copy or configure. NockNock looks for the wallet's new position NFT on-chain at the start of every cycle (CHECK NOW runs one at once), so a deposit is normally found in the first cycle after it lands. A position your wallet minted in this pool with exactly a rung's ticks is tracked as a ladder by itself, with its mint as the baseline. Any other range it finds is shown as found on-chain and is not counted until one tap says how to track it.\n  A measurement from this cycle's reads of what your ladder shape would do. NockNock does not choose the shape, the bottom or the amounts, builds no transaction, and signs and sends nothing.\n";
