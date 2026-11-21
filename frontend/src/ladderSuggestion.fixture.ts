/**
 * The ladder suggestion (#168) as the worker states it, copied from
 * ingestion-worker/testdata/ladder_suggest/golden_ladder_suggest.json, which
 * ingestion-worker/src/tests_ladder_suggest.rs writes. LADDER_READY is the #166 fixture
 * (spot $0.012944, wallet USDC 1545.8357, reserve $950, bottom $0.01); LADDER_READY_LIVE
 * is the live snapshot of 2026-11-21. ladderSuggestion.test.ts checks the copy against
 * the file when the worker directory is present.
 */
import type { LadderSuggestion } from './ladderSuggestion';

/** ready: derived_metrics.ladder_suggestion */
export const LADDER_READY: LadderSuggestion = {
  "after_deposit": {
    "detection_note": null,
    "detection_status": null,
    "kind": "tracked_automatically",
    "text": "After you deposit there is nothing to copy or configure. NockNock looks for the wallet's new position NFT on-chain at the start of every cycle (CHECK NOW runs one at once), so a deposit is normally found in the first cycle after it lands. A position your wallet minted in this pool with exactly a rung's ticks is tracked as a ladder by itself, with its mint as the baseline. Any other range it finds is shown as found on-chain and is not counted until one tap says how to track it."
  },
  "assumptions": [
    "The amount is the most the standing reserve allows at the ladder's bottom; fees and gas are not counted.",
    "The top is the usable tick at or below spot: the range holds only USDC now and earns no fees until NOCK falls to its top.",
    "Below the bottom the other NOCK ranges keep converting; that reading is stated, not the gate.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "No fee estimate is stated yet; it joins this card with the rung settings."
  ],
  "candidates": null,
  "excluded": [
    {
      "position_id": "aerodrome:base:0x770fe8fc9248c6382950a86e39a9391b41d59e0a",
      "reason": "a basic Aero pool is full range; it cannot hold a ladder"
    }
  ],
  "freshness_rule": "Shown only while the snapshot is fresh: the dashboard's sync status is not stale, or the worker's health is ok.",
  "gates": {
    "bottom_setting": {
      "key": "operator_profile.ladder_bottom_usd",
      "status": "set",
      "value": "0.01"
    },
    "floor_breach_effect": "none",
    "ladder_tracked": {
      "reason": "no ladder is tracked (#167)",
      "status": "none"
    },
    "nock_budget": {
      "lp_nock_budget": "86.7891",
      "nock_needed": "0.0000",
      "reason": null,
      "status": "fits"
    },
    "reserve": {
      "binding": "reserve",
      "cash_reserve_usdc": "950.00",
      "key": "operator_profile.cash_reserve_usdc",
      "max_usdc": "766.99",
      "minimum_usdc": "1.00",
      "read_at_rule": "the lower of the snapped bottom and your ladder_bottom_usd",
      "read_at_usd": "0.01000000",
      "reserve_allows_usdc": "766.99",
      "rule": "the stable balance at your bottom stays at or above the reserve",
      "stable_at_gate_usdc": "950.00",
      "status": "allows",
      "wallet_usdc": "1545.83"
    }
  },
  "headline": "Ladder ready: up to $766.99 USDC in 1 deposit, 0 NOCK.",
  "id": "ladder_suggestion:NOCK",
  "minimum_usdc": "1.00",
  "missing": [],
  "numbers": {
    "avg_buy_price_usd": "0.01130202",
    "binding": "reserve",
    "cash_reserve_usdc": "950.00",
    "existing_ranges_usdc_at_gate": "171.16",
    "gate_price_usd": "0.01000000",
    "lp_nock_budget": "86.7891",
    "nock_at_bottom": "67863.0800",
    "nock_now": "0.0000",
    "spot_usd": "0.01294400",
    "stable_at_gate_usdc": "950.00",
    "usdc_max": "766.99",
    "wallet_usdc": "1545.84"
  },
  "outcomes": [
    {
      "avg_buy_price_usd": "0.01184072",
      "kind": "fall_inside",
      "nock_bought": "40558",
      "pct_of_spot": "-15%",
      "price_usd": "0.01100240",
      "text": "A fall to $0.01100 (-15%) would buy about 40,558 NOCK for $480.24 of the USDC, an average of $0.01184.",
      "usdc_spent": "480.24"
    },
    {
      "avg_buy_price_usd": "0.01130202",
      "kind": "fall_to_bottom",
      "nock_bought": "67863",
      "pct_of_spot": "-23%",
      "price_usd": "0.01002405",
      "text": "A fall to the ladder bottom $0.01002 (-23%) would buy about 67,863 NOCK with all $766.99, an average of $0.01130.",
      "usdc_spent": "766.99"
    },
    {
      "kind": "rise",
      "text": "A rise sells nothing: the ladder holds 0 NOCK, so its USDC stays USDC."
    }
  ],
  "picture": {
    "bars": [
      {
        "high_usd": "0.01274292",
        "index": 1,
        "label": "$766.99",
        "low_usd": "0.01002405",
        "usdc": "766.99"
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
        "avg_buy_price_usd": "0.01130202",
        "nock_held": "67863.0800"
      },
      "at_top": {
        "out_of_range_above_now": true,
        "usdc_held": "766.99"
      },
      "bottom": {
        "pct_of_spot": "-22.56",
        "price_usd": "0.01002405",
        "requested_usd": "0.01",
        "rule": "your ladder_bottom_usd, snapped to the nearest usable tick",
        "tick": 276300
      },
      "bounds_nock_per_usdc": {
        "high": "99.7600",
        "low": "78.4750"
      },
      "bounds_usd": {
        "high": "0.01274292",
        "low": "0.01002405"
      },
      "deep_link": {
        "amount_to_enter": "Amount to enter in the app: $766.99 USDC",
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
        "url": "https://app.uniswap.org/positions/add/new?chain=base&protocolVersion=v4&currencyA=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913&currencyB=0x9B5E262cF9bb04869ab40b19AF91D2dc85761722&fee=%7B%22feeAmount%22%3A30000%2C%22isDynamic%22%3Afalse%2C%22tickSpacing%22%3A300%7D&priceRangeState=%7B%22fullRange%22%3Afalse%2C%22maxTick%22%3A276300%2C%22minTick%22%3A273900%2C%22priceInverted%22%3Afalse%7D&step=1"
      },
      "index": 1,
      "nock": {
        "address": "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722",
        "amount": "0.0000",
        "decimals": 16,
        "raw": "0"
      },
      "of": 1,
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
      "rung": null,
      "rung_note": "one rung for now; near and deep rungs are operator settings that join this card later",
      "tick_lower": 273900,
      "tick_upper": 276300,
      "top": {
        "pct_of_spot": "-1.55",
        "price_usd": "0.01274292",
        "rule": "the usable tick at or below spot: the range holds no NOCK now",
        "tick": 273900
      },
      "top_neighbours": [
        {
          "fits_budget": true,
          "nock_needed": "0.0000",
          "pct_of_spot": "-1.55",
          "price_usd": "0.01274292",
          "selected": true,
          "tick": 273900
        },
        {
          "fits_budget": false,
          "nock_needed": "3528.5084",
          "pct_of_spot": "+1.44",
          "price_usd": "0.01313098",
          "selected": false,
          "tick": 273600
        }
      ],
      "usdc": {
        "address": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
        "amount": "766.99",
        "decimals": 6,
        "raw": "766990000"
      },
      "would_sign": "In the Uniswap web app, with your own wallet: one new position in the USDC/NOCK 3% v4 pool, ticks 273900 to 276300, holding up to $766.99 USDC and 0 NOCK (the app may first ask for a USDC approval). NockNock builds no transaction and signs nothing."
    }
  ],
  "sentence": "No ladder is tracked. A new USDC-only position in the USDC/NOCK 3% v4 pool from $0.01274 (-1.6%) down to $0.01002 (-23%) would hold up to $766.99 USDC and 0 NOCK now: the most that leaves the stable balance at your $950.00 reserve at your $0.01 bottom. Fully converted it would hold about 67,863 NOCK, an average of $0.01130.",
  "shown": true,
  "stated": {
    "accumulation_plan_unspent": null,
    "cash_target_usd": null,
    "existing_ranges_below_bottom": {
      "headroom_usdc": "-45.30",
      "note": "The existing NOCK ranges keep converting below the ladder's bottom: a measurement, not the gate.",
      "pct_of_spot": "-40.00",
      "price_usd": "0.00776640",
      "stable_usdc": "904.70"
    },
    "lines": [
      "No accumulation plan is configured.",
      "Tax reserve: $0.00 (stated, not a gate).",
      "No cash target is set.",
      "Below the bottom the other NOCK ranges keep converting: at $0.007766 the stable balance would be $904.70 (a measurement, not the gate)."
    ],
    "note": "The plan budget, the tax reserve and the cash target are stated, not gates: only the standing reserve gates the ladder's USDC.",
    "tax_reserve_usd": {
      "is": "stated, not a gate",
      "usd": "0.00"
    }
  },
  "statement": "A measurement from this cycle's reads of what the range would do. NockNock does not choose the bottom or the amount, builds no transaction, and signs and sends nothing.",
  "status": "ready",
  "tier": "top",
  "token": "NOCK",
  "tracked": [],
  "what_changed": "No ladder is tracked, and the standing reserve allows $766.99 in the USDC/NOCK 3% v4 pool down to your $0.01 bottom."
} as any;

/** ready_live: derived_metrics.ladder_suggestion */
export const LADDER_READY_LIVE: LadderSuggestion = {
  "after_deposit": {
    "detection_note": null,
    "detection_status": null,
    "kind": "tracked_automatically",
    "text": "After you deposit there is nothing to copy or configure. NockNock looks for the wallet's new position NFT on-chain at the start of every cycle (CHECK NOW runs one at once), so a deposit is normally found in the first cycle after it lands. A position your wallet minted in this pool with exactly a rung's ticks is tracked as a ladder by itself, with its mint as the baseline. Any other range it finds is shown as found on-chain and is not counted until one tap says how to track it."
  },
  "assumptions": [
    "The amount is the most the standing reserve allows at the ladder's bottom; fees and gas are not counted.",
    "The top is the usable tick at or below spot: the range holds only USDC now and earns no fees until NOCK falls to its top.",
    "Below the bottom the other NOCK ranges keep converting; that reading is stated, not the gate.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "No fee estimate is stated yet; it joins this card with the rung settings."
  ],
  "candidates": null,
  "excluded": [
    {
      "position_id": "aerodrome:base:0x770fe8fc9248c6382950a86e39a9391b41d59e0a",
      "reason": "a basic Aero pool is full range; it cannot hold a ladder"
    }
  ],
  "freshness_rule": "Shown only while the snapshot is fresh: the dashboard's sync status is not stale, or the worker's health is ok.",
  "gates": {
    "bottom_setting": {
      "key": "operator_profile.ladder_bottom_usd",
      "status": "set",
      "value": "0.01"
    },
    "floor_breach_effect": "none",
    "ladder_tracked": {
      "reason": "no ladder is tracked (#167)",
      "status": "none"
    },
    "nock_budget": {
      "lp_nock_budget": "86.7891",
      "nock_needed": "0.0000",
      "reason": null,
      "status": "fits"
    },
    "reserve": {
      "binding": "reserve",
      "cash_reserve_usdc": "950.00",
      "key": "operator_profile.cash_reserve_usdc",
      "max_usdc": "765.86",
      "minimum_usdc": "1.00",
      "read_at_rule": "the lower of the snapped bottom and your ladder_bottom_usd",
      "read_at_usd": "0.01000000",
      "reserve_allows_usdc": "765.86",
      "rule": "the stable balance at your bottom stays at or above the reserve",
      "stable_at_gate_usdc": "950.01",
      "status": "allows",
      "wallet_usdc": "1545.83"
    }
  },
  "headline": "Ladder ready: up to $765.86 USDC in 1 deposit, 0 NOCK.",
  "id": "ladder_suggestion:NOCK",
  "minimum_usdc": "1.00",
  "missing": [],
  "numbers": {
    "avg_buy_price_usd": "0.01130202",
    "binding": "reserve",
    "cash_reserve_usdc": "950.00",
    "existing_ranges_usdc_at_gate": "170.03",
    "gate_price_usd": "0.01000000",
    "lp_nock_budget": "86.7891",
    "nock_at_bottom": "67763.0979",
    "nock_now": "0.0000",
    "spot_usd": "0.01294450",
    "stable_at_gate_usdc": "950.01",
    "usdc_max": "765.86",
    "wallet_usdc": "1545.84"
  },
  "outcomes": [
    {
      "avg_buy_price_usd": "0.01184095",
      "kind": "fall_inside",
      "nock_bought": "40488",
      "pct_of_spot": "-15%",
      "price_usd": "0.01100283",
      "text": "A fall to $0.01100 (-15%) would buy about 40,488 NOCK for $479.41 of the USDC, an average of $0.01184.",
      "usdc_spent": "479.41"
    },
    {
      "avg_buy_price_usd": "0.01130202",
      "kind": "fall_to_bottom",
      "nock_bought": "67763",
      "pct_of_spot": "-23%",
      "price_usd": "0.01002405",
      "text": "A fall to the ladder bottom $0.01002 (-23%) would buy about 67,763 NOCK with all $765.86, an average of $0.01130.",
      "usdc_spent": "765.86"
    },
    {
      "kind": "rise",
      "text": "A rise sells nothing: the ladder holds 0 NOCK, so its USDC stays USDC."
    }
  ],
  "picture": {
    "bars": [
      {
        "high_usd": "0.01274292",
        "index": 1,
        "label": "$765.86",
        "low_usd": "0.01002405",
        "usdc": "765.86"
      }
    ],
    "bottom_requested_usd": "0.01",
    "bottom_usd": "0.01002405",
    "reserve_label": "$950 kept for below $0.01",
    "reserve_usdc": "950.00",
    "spot_usd": "0.01294450"
  },
  "reason": null,
  "rungs": [
    {
      "at_bottom": {
        "avg_buy_price_usd": "0.01130202",
        "nock_held": "67763.0979"
      },
      "at_top": {
        "out_of_range_above_now": true,
        "usdc_held": "765.86"
      },
      "bottom": {
        "pct_of_spot": "-22.56",
        "price_usd": "0.01002405",
        "requested_usd": "0.01",
        "rule": "your ladder_bottom_usd, snapped to the nearest usable tick",
        "tick": 276300
      },
      "bounds_nock_per_usdc": {
        "high": "99.7600",
        "low": "78.4750"
      },
      "bounds_usd": {
        "high": "0.01274292",
        "low": "0.01002405"
      },
      "deep_link": {
        "amount_to_enter": "Amount to enter in the app: $765.86 USDC",
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
        "url": "https://app.uniswap.org/positions/add/new?chain=base&protocolVersion=v4&currencyA=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913&currencyB=0x9B5E262cF9bb04869ab40b19AF91D2dc85761722&fee=%7B%22feeAmount%22%3A30000%2C%22isDynamic%22%3Afalse%2C%22tickSpacing%22%3A300%7D&priceRangeState=%7B%22fullRange%22%3Afalse%2C%22maxTick%22%3A276300%2C%22minTick%22%3A273900%2C%22priceInverted%22%3Afalse%7D&step=1"
      },
      "index": 1,
      "nock": {
        "address": "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722",
        "amount": "0.0000",
        "decimals": 16,
        "raw": "0"
      },
      "of": 1,
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
        "pool_price_usd": "0.01294450",
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
      "rung": null,
      "rung_note": "one rung for now; near and deep rungs are operator settings that join this card later",
      "tick_lower": 273900,
      "tick_upper": 276300,
      "top": {
        "pct_of_spot": "-1.56",
        "price_usd": "0.01274292",
        "rule": "the usable tick at or below spot: the range holds no NOCK now",
        "tick": 273900
      },
      "top_neighbours": [
        {
          "fits_budget": true,
          "nock_needed": "0.0000",
          "pct_of_spot": "-1.56",
          "price_usd": "0.01274292",
          "selected": true,
          "tick": 273900
        },
        {
          "fits_budget": false,
          "nock_needed": "3513.2226",
          "pct_of_spot": "+1.44",
          "price_usd": "0.01313098",
          "selected": false,
          "tick": 273600
        }
      ],
      "usdc": {
        "address": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
        "amount": "765.86",
        "decimals": 6,
        "raw": "765860000"
      },
      "would_sign": "In the Uniswap web app, with your own wallet: one new position in the USDC/NOCK 3% v4 pool, ticks 273900 to 276300, holding up to $765.86 USDC and 0 NOCK (the app may first ask for a USDC approval). NockNock builds no transaction and signs nothing."
    }
  ],
  "sentence": "No ladder is tracked. A new USDC-only position in the USDC/NOCK 3% v4 pool from $0.01274 (-1.6%) down to $0.01002 (-23%) would hold up to $765.86 USDC and 0 NOCK now: the most that leaves the stable balance at your $950.00 reserve at your $0.01 bottom. Fully converted it would hold about 67,763 NOCK, an average of $0.01130.",
  "shown": true,
  "stated": {
    "accumulation_plan_unspent": {
      "is": "stated, not a gate",
      "note": null,
      "usdc": "0.00"
    },
    "cash_target_usd": "184.18",
    "existing_ranges_below_bottom": {
      "headroom_usdc": "-45.15",
      "note": "The existing NOCK ranges keep converting below the ladder's bottom: a measurement, not the gate.",
      "pct_of_spot": "-40.00",
      "price_usd": "0.00776670",
      "stable_usdc": "904.85"
    },
    "lines": [
      "Plan budget not yet spent: $0.00 (stated, not a gate).",
      "Tax reserve: $8.70 (stated, not a gate).",
      "Cash target: $184.18 (5% of the portfolio; stated, not a gate).",
      "Below the bottom the other NOCK ranges keep converting: at $0.007767 the stable balance would be $904.85 (a measurement, not the gate)."
    ],
    "note": "The plan budget, the tax reserve and the cash target are stated, not gates: only the standing reserve gates the ladder's USDC.",
    "tax_reserve_usd": {
      "is": "stated, not a gate",
      "usd": "8.70"
    }
  },
  "statement": "A measurement from this cycle's reads of what the range would do. NockNock does not choose the bottom or the amount, builds no transaction, and signs and sends nothing.",
  "status": "ready",
  "tier": "top",
  "token": "NOCK",
  "tracked": [],
  "what_changed": "No ladder is tracked, and the standing reserve allows $765.86 in the USDC/NOCK 3% v4 pool down to your $0.01 bottom."
} as any;

/** needs_setting: derived_metrics.ladder_suggestion */
export const LADDER_NEEDS_SETTING: LadderSuggestion = {
  "after_deposit": null,
  "assumptions": [
    "The amount is the most the standing reserve allows at the ladder's bottom; fees and gas are not counted.",
    "The top is the usable tick at or below spot: the range holds only USDC now and earns no fees until NOCK falls to its top.",
    "Below the bottom the other NOCK ranges keep converting; that reading is stated, not the gate.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "No fee estimate is stated yet; it joins this card with the rung settings."
  ],
  "candidates": {
    "note": "candidate bottoms, a measurement; none is picked. The depth is the simulator's display grid, not an operator number.",
    "reason": null,
    "rows": [
      {
        "avg_buy_price_usd": "0.01255321",
        "existing_ranges_usdc_at_bottom": "213.91",
        "grid_pct": "-5",
        "nock_at_bottom": "64505",
        "nock_per_100_usdc": "7966",
        "pct_of_spot": "-4.46",
        "price_usd": "0.01236633",
        "tick": 274200,
        "usdc_ceiling": "809.74"
      },
      {
        "avg_buy_price_usd": "0.01218222",
        "existing_ranges_usdc_at_bottom": "201.37",
        "grid_pct": "-10",
        "nock_at_bottom": "65440",
        "nock_per_100_usdc": "8209",
        "pct_of_spot": "-10.03",
        "price_usd": "0.01164620",
        "tick": 274800,
        "usdc_ceiling": "797.20"
      },
      {
        "avg_buy_price_usd": "0.01182220",
        "existing_ranges_usdc_at_bottom": "189.20",
        "grid_pct": "-15",
        "nock_at_bottom": "66403",
        "nock_per_100_usdc": "8459",
        "pct_of_spot": "-15.27",
        "price_usd": "0.01096801",
        "tick": 275400,
        "usdc_ceiling": "785.03"
      },
      {
        "avg_buy_price_usd": "0.01147282",
        "existing_ranges_usdc_at_bottom": "177.39",
        "grid_pct": "-20",
        "nock_at_bottom": "67396",
        "nock_per_100_usdc": "8716",
        "pct_of_spot": "-20.20",
        "price_usd": "0.01032932",
        "tick": 276000,
        "usdc_ceiling": "773.22"
      },
      {
        "avg_buy_price_usd": "0.01113376",
        "existing_ranges_usdc_at_bottom": "165.93",
        "grid_pct": "-25",
        "nock_at_bottom": "68419",
        "nock_per_100_usdc": "8982",
        "pct_of_spot": "-24.85",
        "price_usd": "0.00972781",
        "tick": 276600,
        "usdc_ceiling": "761.76"
      },
      {
        "avg_buy_price_usd": "0.01080473",
        "existing_ranges_usdc_at_bottom": "154.81",
        "grid_pct": "-30",
        "nock_at_bottom": "69473",
        "nock_per_100_usdc": "9255",
        "pct_of_spot": "-29.22",
        "price_usd": "0.00916134",
        "tick": 277200,
        "usdc_ceiling": "750.64"
      },
      {
        "avg_buy_price_usd": "0.01032932",
        "existing_ranges_usdc_at_bottom": "138.74",
        "grid_pct": "-35",
        "nock_at_bottom": "71115",
        "nock_per_100_usdc": "9681",
        "pct_of_spot": "-35.31",
        "price_usd": "0.00837287",
        "tick": 278100,
        "usdc_ceiling": "734.57"
      },
      {
        "avg_buy_price_usd": "0.00987482",
        "existing_ranges_usdc_at_bottom": "123.37",
        "grid_pct": "-40",
        "nock_at_bottom": "72832",
        "nock_per_100_usdc": "10127",
        "pct_of_spot": "-40.88",
        "price_usd": "0.00765226",
        "tick": 279000,
        "usdc_ceiling": "719.20"
      }
    ],
    "skipped": [],
    "step_pct": "5",
    "tick_spacing": 300,
    "to_pct": "-40"
  },
  "excluded": [
    {
      "position_id": "aerodrome:base:0x770fe8fc9248c6382950a86e39a9391b41d59e0a",
      "reason": "a basic Aero pool is full range; it cannot hold a ladder"
    }
  ],
  "freshness_rule": "Shown only while the snapshot is fresh: the dashboard's sync status is not stale, or the worker's health is ok.",
  "gates": null,
  "headline": "Ladder: needs your bottom price (operator_profile.ladder_bottom_usd) before one is measured.",
  "id": "ladder_suggestion:NOCK",
  "minimum_usdc": "1.00",
  "missing": [
    "operator_profile.ladder_bottom_usd"
  ],
  "numbers": null,
  "outcomes": [],
  "picture": null,
  "reason": "No ladder is tracked. A suggestion needs operator_profile.ladder_bottom_usd (your bottom price; NockNock does not choose one).",
  "rungs": [],
  "sentence": null,
  "shown": false,
  "stated": null,
  "statement": "A measurement from this cycle's reads of what the range would do. NockNock does not choose the bottom or the amount, builds no transaction, and signs and sends nothing.",
  "status": "needs_setting",
  "tier": "top",
  "token": "NOCK",
  "tracked": [],
  "what_changed": null
} as any;

/** bottom_at_or_above_top: derived_metrics.ladder_suggestion */
export const LADDER_BOTTOM_ABOVE_TOP: LadderSuggestion = {
  "after_deposit": null,
  "assumptions": [
    "The amount is the most the standing reserve allows at the ladder's bottom; fees and gas are not counted.",
    "The top is the usable tick at or below spot: the range holds only USDC now and earns no fees until NOCK falls to its top.",
    "Below the bottom the other NOCK ranges keep converting; that reading is stated, not the gate.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "No fee estimate is stated yet; it joins this card with the rung settings."
  ],
  "candidates": null,
  "excluded": [
    {
      "position_id": "aerodrome:base:0x770fe8fc9248c6382950a86e39a9391b41d59e0a",
      "reason": "a basic Aero pool is full range; it cannot hold a ladder"
    }
  ],
  "freshness_rule": "Shown only while the snapshot is fresh: the dashboard's sync status is not stale, or the worker's health is ok.",
  "gates": null,
  "headline": "Ladder: NOCK is within one tick spacing of your bottom, so the range would be empty.",
  "id": "ladder_suggestion:NOCK",
  "minimum_usdc": "1.00",
  "missing": [],
  "numbers": null,
  "outcomes": [],
  "picture": null,
  "reason": "both bounds snap to tick 273900: one spacing of 300 ticks is 3.05% of price, wider than this range",
  "rungs": [],
  "sentence": null,
  "shown": false,
  "stated": null,
  "statement": "A measurement from this cycle's reads of what the range would do. NockNock does not choose the bottom or the amount, builds no transaction, and signs and sends nothing.",
  "status": "bottom_at_or_above_top",
  "tier": "top",
  "token": "NOCK",
  "tracked": [],
  "what_changed": null
} as any;

/** price_at_or_below_bottom: derived_metrics.ladder_suggestion */
export const LADDER_PRICE_BELOW_BOTTOM: LadderSuggestion = {
  "after_deposit": null,
  "assumptions": [
    "The amount is the most the standing reserve allows at the ladder's bottom; fees and gas are not counted.",
    "The top is the usable tick at or below spot: the range holds only USDC now and earns no fees until NOCK falls to its top.",
    "Below the bottom the other NOCK ranges keep converting; that reading is stated, not the gate.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "No fee estimate is stated yet; it joins this card with the rung settings."
  ],
  "candidates": null,
  "excluded": [
    {
      "position_id": "aerodrome:base:0x770fe8fc9248c6382950a86e39a9391b41d59e0a",
      "reason": "a basic Aero pool is full range; it cannot hold a ladder"
    }
  ],
  "freshness_rule": "Shown only while the snapshot is fresh: the dashboard's sync status is not stale, or the worker's health is ok.",
  "gates": null,
  "headline": "Ladder: NOCK is at or below your $0.02 ladder bottom, so none is measured above it.",
  "id": "ladder_suggestion:NOCK",
  "minimum_usdc": "1.00",
  "missing": [],
  "numbers": null,
  "outcomes": [],
  "picture": null,
  "reason": "NOCK is at or below your $0.02 ladder bottom (NOCK $0.01294 in the USDC/NOCK 3% v4 pool): no ladder is measured above it.",
  "rungs": [],
  "sentence": null,
  "shown": false,
  "stated": null,
  "statement": "A measurement from this cycle's reads of what the range would do. NockNock does not choose the bottom or the amount, builds no transaction, and signs and sends nothing.",
  "status": "price_at_or_below_bottom",
  "tier": "top",
  "token": "NOCK",
  "tracked": [],
  "what_changed": null
} as any;

/** no_target_pool: derived_metrics.ladder_suggestion */
export const LADDER_NO_TARGET_POOL: LadderSuggestion = {
  "after_deposit": null,
  "assumptions": [
    "The amount is the most the standing reserve allows at the ladder's bottom; fees and gas are not counted.",
    "The top is the usable tick at or below spot: the range holds only USDC now and earns no fees until NOCK falls to its top.",
    "Below the bottom the other NOCK ranges keep converting; that reading is stated, not the gate.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "No fee estimate is stated yet; it joins this card with the rung settings."
  ],
  "candidates": null,
  "excluded": [
    {
      "position_id": "uniswap-v4:base:3163450",
      "reason": "this pool has no in-range liquidity, so its price is frozen"
    },
    {
      "position_id": "aerodrome:base:0x770fe8fc9248c6382950a86e39a9391b41d59e0a",
      "reason": "a basic Aero pool is full range; it cannot hold a ladder"
    }
  ],
  "freshness_rule": "Shown only while the snapshot is fresh: the dashboard's sync status is not stale, or the worker's health is ok.",
  "gates": null,
  "headline": "Ladder: no tracked USDC/NOCK pool can hold one this cycle.",
  "id": "ladder_suggestion:NOCK",
  "minimum_usdc": "1.00",
  "missing": [],
  "numbers": null,
  "outcomes": [],
  "picture": null,
  "reason": "No tracked USDC/NOCK concentrated pool can hold a ladder this cycle; each pool's reason is listed.",
  "rungs": [],
  "sentence": null,
  "shown": false,
  "stated": null,
  "statement": "A measurement from this cycle's reads of what the range would do. NockNock does not choose the bottom or the amount, builds no transaction, and signs and sends nothing.",
  "status": "no_target_pool",
  "tier": "top",
  "token": "NOCK",
  "tracked": [],
  "what_changed": null
} as any;

/** tracked: derived_metrics.ladder_suggestion */
export const LADDER_TRACKED: LadderSuggestion = {
  "after_deposit": null,
  "assumptions": [
    "The amount is the most the standing reserve allows at the ladder's bottom; fees and gas are not counted.",
    "The top is the usable tick at or below spot: the range holds only USDC now and earns no fees until NOCK falls to its top.",
    "Below the bottom the other NOCK ranges keep converting; that reading is stated, not the gate.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "No fee estimate is stated yet; it joins this card with the rung settings."
  ],
  "candidates": null,
  "excluded": [],
  "freshness_rule": "Shown only while the snapshot is fresh: the dashboard's sync status is not stale, or the worker's health is ok.",
  "gates": null,
  "headline": "Ladder: tracked. No new one is suggested.",
  "id": "ladder_suggestion:NOCK",
  "minimum_usdc": "1.00",
  "missing": [],
  "numbers": null,
  "outcomes": [],
  "picture": null,
  "reason": "A ladder is tracked (uniswap-v4 #3163450): no new one is suggested. Its readings are in the Ladders card.",
  "rungs": [],
  "sentence": null,
  "shown": false,
  "stated": null,
  "statement": "A measurement from this cycle's reads of what the range would do. NockNock does not choose the bottom or the amount, builds no transaction, and signs and sends nothing.",
  "status": "ladder_tracked",
  "tier": "top",
  "token": "NOCK",
  "tracked": [
    {
      "label": "uniswap-v4 #3163450",
      "position_id": "uniswap-v4:base:3163450",
      "rung": null,
      "state": null
    }
  ],
  "what_changed": null
} as any;
