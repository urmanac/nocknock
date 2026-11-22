/**
 * The worker's GET /v1/simulate/ladder bodies over the #166 fixture (spot $0.012944,
 * wallet USDC 1545.8357 and NOCK 90086.7891, floor 100,000 with 1,000 bps, reserve
 * $950), copied from ingestion-worker/testdata/ladder/golden_ladder.json, which
 * ingestion-worker/src/tests_ladder.rs writes. ladder.test.ts checks the copy against it.
 */
import type { LadderErrorBody, LadderSimulation } from './ladder';

export const SPOT_741_SNAPPED: LadderSimulation = {
  "assumptions": [
    "Every NOCK pool moves by the same percentage as spot, each from its own current price; row 0 of the grid is the snapshot.",
    "Fees and gas are not counted.",
    "WETH/NOCK ranges hold ETH/USD constant and add no USDC.",
    "Gas-side ranges with no NOCK leg (ETH/USDC, #151) are left out of the stable balance and the NOCK totals.",
    "The ladder's USDC comes from the wallet and its NOCK from wallet NOCK."
  ],
  "bottom": {
    "avg_buy_price_usd": "0.01130202",
    "nock_bought": "65563.4914",
    "nock_held": "65563.4914",
    "pct_of_spot": "-22.56",
    "price_usd": "0.01002405"
  },
  "bounds": {
    "high_pct": "-1.55",
    "high_rule": "the usable tick at or below the pool price (high = spot): it holds no NOCK",
    "high_usd": "0.01274292",
    "low_pct": "-22.56",
    "low_usd": "0.01002405",
    "tick_lower": 273900,
    "tick_upper": 276300
  },
  "fee_estimate": {
    "assumptions": [
      "Fee intensity per unit of liquidity is taken as uniform across ticks within the measuring range: the estimate is the measuring range's average fee per unit of liquidity times this range's liquidity, not a measurement inside it (band_measured is the measurement inside it, where there is one).",
      "Fees per unit of liquidity are shared by every range that contains the price (the same pool, the same fee tier).",
      "The pool's swap volume is taken as unchanged by the new liquidity: only its share dilutes.",
      "Dilution uses the pool's active liquidity at the current tick, read at 'latest' in the snapshot's cycle (not at the snapshot's block); the liquidity at the range's own ticks is not read and is taken as the same.",
      "The NOCK fee leg is valued at the current pool price, not at the price when it accrued.",
      "Intervals where the measuring range's liquidity changed or its fees fell (a collect, or a v4 add that collects) are skipped, so the measurement slightly undercounts; the seconds skipped are stated.",
      "Gas is not counted. Past fees are not a forecast.",
      "The stored market volume of another pool (market_price_history) is never used."
    ],
    "band_measured": null,
    "band_reason": null,
    "constants": {
      "band_min_seconds": 72000.0,
      "day_min_seconds": null,
      "max_gap_seconds": null,
      "note": "display constants, not settings",
      "window_days": null
    },
    "coverage": null,
    "dilution": null,
    "dilution_reason": null,
    "in_range_share_reason": null,
    "in_range_share_stored_history": null,
    "intervals": null,
    "measured_at": null,
    "measured_by": null,
    "measured_days": null,
    "measured_label": null,
    "measured_seconds": null,
    "measurement_age_seconds": null,
    "per_100_usd_per_day_in_range_usd": null,
    "per_day_in_range_diluted_usd": null,
    "per_day_in_range_nock_leg": null,
    "per_day_in_range_usd": null,
    "per_day_in_range_usdc_leg": null,
    "rate": null,
    "reason": "this snapshot has no fee_yield (2.10.0): fees are not measured in it",
    "replay_reason": null,
    "skipped": null,
    "skipped_seconds": null,
    "spread": null,
    "spread_reason": null,
    "status": "unavailable",
    "stored_history_replay": null
  },
  "floor_check": {
    "budget_note": null,
    "floor_breach_effect": {
      "after": {
        "crossing": {
          "level_nock": "100000",
          "multiple": "1.9626",
          "pct_above_spot": "96.3",
          "price_usd": "0.025404",
          "status": "crosses"
        },
        "nock_now": "109543",
        "reason": null,
        "status": "crosses",
        "tolerated": {
          "level_nock": "90000",
          "multiple": null,
          "pct_above_spot": null,
          "price_usd": null,
          "status": "never"
        }
      },
      "assumption": "Each pool is moved by the same multiple from its own current price. WETH/NOCK ranges hold ETH/USD constant (a NOCK/USD multiple is the same NOCK/ETH multiple). Full-range pools follow x*y=k on the wallet's share and never sell out. Uncollected NOCK fees count as held. Gas-side ranges with no NOCK leg hold no NOCK.",
      "before": {
        "crossing": {
          "level_nock": "100000",
          "multiple": "1.9626",
          "pct_above_spot": "96.3",
          "price_usd": "0.025404",
          "status": "crosses"
        },
        "nock_now": "109543",
        "reason": null,
        "status": "crosses",
        "tolerated": {
          "level_nock": "90000",
          "multiple": null,
          "pct_above_spot": null,
          "price_usd": null,
          "status": "never"
        }
      },
      "reason": "the ladder holds no NOCK at or above spot, so a rise sells none of it",
      "status": "none"
    },
    "lp_nock_budget": "86.7891",
    "max_usdc_within_budget": null,
    "nock_needed": "0.0000",
    "reason": null,
    "rebuild_note": "the rebuild's suggestions this cycle also draw 0 NOCK of the same budget: an alternative use of it, not subtracted",
    "rebuild_suggested_nock": "0.0000",
    "status": "fits"
  },
  "grid": [
    {
      "existing_nock_lp_nock": "19341.9535",
      "existing_nock_lp_usdc": "223.71",
      "ladder_nock": "0.0000",
      "ladder_nock_acquired": "0.0000",
      "ladder_usdc": "741.00",
      "markers": [
        "spot"
      ],
      "pct_of_spot": "0.00",
      "pool_price_usd": "0.01294400",
      "price_usd": "0.01294400",
      "reserve_headroom_usdc": "819.54",
      "stable_usdc": "1769.54",
      "total_tracked_nock": "109542.50"
    },
    {
      "existing_nock_lp_nock": "19603.9982",
      "existing_nock_lp_usdc": "220.32",
      "ladder_nock": "0.0000",
      "ladder_nock_acquired": "0.0000",
      "ladder_usdc": "741.00",
      "markers": [
        "top"
      ],
      "pct_of_spot": "-1.55",
      "pool_price_usd": "0.01274292",
      "price_usd": "0.01274292",
      "reserve_headroom_usdc": "816.16",
      "stable_usdc": "1766.16",
      "total_tracked_nock": "109804.55"
    },
    {
      "existing_nock_lp_nock": "20208.1461",
      "existing_nock_lp_usdc": "212.71",
      "ladder_nock": "9245.4120",
      "ladder_nock_acquired": "9245.4120",
      "ladder_usdc": "625.27",
      "markers": [],
      "pct_of_spot": "-5.00",
      "pool_price_usd": "0.01229680",
      "price_usd": "0.01229680",
      "reserve_headroom_usdc": "692.82",
      "stable_usdc": "1642.82",
      "total_tracked_nock": "119654.11"
    },
    {
      "existing_nock_lp_nock": "21145.5540",
      "existing_nock_lp_usdc": "201.43",
      "ladder_nock": "23590.7764",
      "ladder_nock_acquired": "23590.7764",
      "ladder_usdc": "453.57",
      "markers": [],
      "pct_of_spot": "-10.00",
      "pool_price_usd": "0.01164960",
      "price_usd": "0.01164960",
      "reserve_headroom_usdc": "509.83",
      "stable_usdc": "1459.83",
      "total_tracked_nock": "134936.88"
    },
    {
      "existing_nock_lp_nock": "22164.5030",
      "existing_nock_lp_usdc": "189.83",
      "ladder_nock": "39183.9830",
      "ladder_nock_acquired": "39183.9830",
      "ladder_usdc": "277.03",
      "markers": [],
      "pct_of_spot": "-15.00",
      "pool_price_usd": "0.01100240",
      "price_usd": "0.01100240",
      "reserve_headroom_usdc": "321.69",
      "stable_usdc": "1271.69",
      "total_tracked_nock": "151549.03"
    },
    {
      "existing_nock_lp_nock": "23277.5444",
      "existing_nock_lp_usdc": "177.88",
      "ladder_nock": "56217.1059",
      "ladder_nock_acquired": "56217.1059",
      "ladder_usdc": "95.22",
      "markers": [],
      "pct_of_spot": "-20.00",
      "pool_price_usd": "0.01035520",
      "price_usd": "0.01035520",
      "reserve_headroom_usdc": "127.93",
      "stable_usdc": "1077.93",
      "total_tracked_nock": "169695.20"
    },
    {
      "existing_nock_lp_nock": "23888.2905",
      "existing_nock_lp_usdc": "171.62",
      "ladder_nock": "65563.4914",
      "ladder_nock_acquired": "65563.4914",
      "ladder_usdc": "0.00",
      "markers": [
        "bottom"
      ],
      "pct_of_spot": "-22.56",
      "pool_price_usd": "0.01002405",
      "price_usd": "0.01002405",
      "reserve_headroom_usdc": "26.45",
      "stable_usdc": "976.45",
      "total_tracked_nock": "179652.33"
    },
    {
      "existing_nock_lp_nock": "24500.1121",
      "existing_nock_lp_usdc": "165.55",
      "ladder_nock": "65563.4914",
      "ladder_nock_acquired": "65563.4914",
      "ladder_usdc": "0.00",
      "markers": [],
      "pct_of_spot": "-25.00",
      "pool_price_usd": "0.00970800",
      "price_usd": "0.00970800",
      "reserve_headroom_usdc": "20.38",
      "stable_usdc": "970.38",
      "total_tracked_nock": "180264.15"
    },
    {
      "existing_nock_lp_nock": "25851.4339",
      "existing_nock_lp_usdc": "152.80",
      "ladder_nock": "65563.4914",
      "ladder_nock_acquired": "65563.4914",
      "ladder_usdc": "0.00",
      "markers": [],
      "pct_of_spot": "-30.00",
      "pool_price_usd": "0.00906080",
      "price_usd": "0.00906080",
      "reserve_headroom_usdc": "7.63",
      "stable_usdc": "957.63",
      "total_tracked_nock": "181615.47"
    },
    {
      "existing_nock_lp_nock": "27355.8217",
      "existing_nock_lp_usdc": "139.58",
      "ladder_nock": "65563.4914",
      "ladder_nock_acquired": "65563.4914",
      "ladder_usdc": "0.00",
      "markers": [],
      "pct_of_spot": "-35.00",
      "pool_price_usd": "0.00841360",
      "price_usd": "0.00841360",
      "reserve_headroom_usdc": "-5.58",
      "stable_usdc": "944.42",
      "total_tracked_nock": "183119.86"
    },
    {
      "existing_nock_lp_nock": "29044.5410",
      "existing_nock_lp_usdc": "125.85",
      "ladder_nock": "65563.4914",
      "ladder_nock_acquired": "65563.4914",
      "ladder_usdc": "0.00",
      "markers": [],
      "pct_of_spot": "-40.00",
      "pool_price_usd": "0.00776640",
      "price_usd": "0.00776640",
      "reserve_headroom_usdc": "-19.31",
      "stable_usdc": "930.69",
      "total_tracked_nock": "184808.58"
    }
  ],
  "inputs": {
    "grid_step_pct": 5.0,
    "grid_to_pct": -40.0,
    "high_requested": "spot",
    "low_requested": "0.01",
    "nock": null,
    "position_id": "uniswap-v4:base:3163450",
    "tick_spacing": null,
    "usdc": "741.00"
  },
  "pool": {
    "label": "v4 USDC/NOCK 3.00% #3163450",
    "note": "a ladder in this pool is a NEW position (a new NFT); add it to config.yaml to be tracked",
    "pool_price_usd": "0.01294400",
    "pool_price_vs_spot_pct": "0.00",
    "position_id": "uniswap-v4:base:3163450",
    "snapped": true,
    "spacing_source": "position",
    "tick_spacing": 300
  },
  "reserve_check": {
    "accumulation_plan_unspent": null,
    "accumulation_plan_unspent_reason": "no accumulation_plan is configured",
    "at_top": {
      "headroom_usdc": "819.54",
      "ladder_usdc": "741.00",
      "note": "The ladder is out of range above now and holds only USDC: the stable balance at spot is unchanged by it, and a rise takes none of it.",
      "out_of_range_above_now": true,
      "price_usd": "0.01294400",
      "stable_usdc": "1769.54"
    },
    "cash_reserve_usdc": "950.00",
    "existing_ranges_below_bottom": {
      "headroom_usdc": "-19.31",
      "note": "The existing NOCK ranges keep converting below the ladder's bottom: a measurement, not the gate.",
      "pct_of_spot": "-40.00",
      "price_usd": "0.00776640",
      "stable_usdc": "930.69"
    },
    "existing_ranges_below_bottom_reason": null,
    "existing_ranges_usdc_at_bottom": "171.62",
    "gate": "The gate is the stable balance at the ladder's own bottom (operator decision, 2026-11-20).",
    "headroom_at_spot_usdc": "819.54",
    "margin_usdc": "26.45",
    "max_usdc_for_this_range": "767.45",
    "reason": null,
    "stable_at_bottom_usdc": "976.45",
    "stable_at_spot_usdc": "1769.54",
    "status": "holds",
    "tax_reserve_usd": {
      "is": "stated, not a gate",
      "usd": "0.00"
    }
  },
  "sizing": {
    "liquidity_human": "58052.3879",
    "liquidity_raw": "5805238794314994",
    "nock_check": "not_given",
    "nock_given": null,
    "nock_needed_at_spot": "0.0000",
    "usdc_at_spot": "741.00"
  },
  "snapshot": {
    "age_minutes": 5,
    "age_seconds": 300,
    "generated_at": "2026-11-20T12:00:00Z",
    "snapshot_id": "00000000-0000-0000-0000-000000000000",
    "spot_source": "price-source pool sqrtPriceX96 (v4 USDC/NOCK 3.00% #3163450)",
    "spot_usd": "0.01294400",
    "version": "2.12.0",
    "worker_health": "ok"
  },
  "statement": "A what-if from the stored snapshot. NockNock reads nothing new, signs nothing, sends nothing, and does not choose the amounts.",
  "top": {
    "avg_sell_price_usd": null,
    "multiple_of_spot": "0.9845",
    "out_of_range_above_now": true,
    "pct_of_spot": "-1.55",
    "price_usd": "0.01274292",
    "usdc_held": "741.00"
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
      "nock_needed": "3408.9424",
      "pct_of_spot": "+1.44",
      "price_usd": "0.01313098",
      "selected": false,
      "tick": 273600
    }
  ],
  "top_neighbours_reason": null
};

export const CHAT_900_UNSNAPPED: LadderSimulation = {
  "assumptions": [
    "Every NOCK pool moves by the same percentage as spot, each from its own current price; row 0 of the grid is the snapshot.",
    "Fees and gas are not counted.",
    "WETH/NOCK ranges hold ETH/USD constant and add no USDC.",
    "Gas-side ranges with no NOCK leg (ETH/USDC, #151) are left out of the stable balance and the NOCK totals.",
    "The ladder's USDC comes from the wallet and its NOCK from wallet NOCK."
  ],
  "bottom": {
    "avg_buy_price_usd": "0.01082973",
    "nock_bought": "83104.5850",
    "nock_held": "87298.5623",
    "pct_of_spot": "-30.00",
    "price_usd": "0.00906080"
  },
  "bounds": {
    "high_pct": "+2.00",
    "high_rule": "unsnapped",
    "high_usd": "0.01320288",
    "low_pct": "-30.00",
    "low_usd": "0.00906080",
    "tick_lower": null,
    "tick_upper": null
  },
  "fee_estimate": {
    "assumptions": [
      "Fee intensity per unit of liquidity is taken as uniform across ticks within the measuring range: the estimate is the measuring range's average fee per unit of liquidity times this range's liquidity, not a measurement inside it (band_measured is the measurement inside it, where there is one).",
      "Fees per unit of liquidity are shared by every range that contains the price (the same pool, the same fee tier).",
      "The pool's swap volume is taken as unchanged by the new liquidity: only its share dilutes.",
      "Dilution uses the pool's active liquidity at the current tick, read at 'latest' in the snapshot's cycle (not at the snapshot's block); the liquidity at the range's own ticks is not read and is taken as the same.",
      "The NOCK fee leg is valued at the current pool price, not at the price when it accrued.",
      "Intervals where the measuring range's liquidity changed or its fees fell (a collect, or a v4 add that collects) are skipped, so the measurement slightly undercounts; the seconds skipped are stated.",
      "Gas is not counted. Past fees are not a forecast.",
      "The stored market volume of another pool (market_price_history) is never used."
    ],
    "band_measured": null,
    "band_reason": null,
    "constants": {
      "band_min_seconds": 72000.0,
      "day_min_seconds": null,
      "max_gap_seconds": null,
      "note": "display constants, not settings",
      "window_days": null
    },
    "coverage": null,
    "dilution": null,
    "dilution_reason": null,
    "in_range_share_reason": null,
    "in_range_share_stored_history": null,
    "intervals": null,
    "measured_at": null,
    "measured_by": null,
    "measured_days": null,
    "measured_label": null,
    "measured_seconds": null,
    "measurement_age_seconds": null,
    "per_100_usd_per_day_in_range_usd": null,
    "per_day_in_range_diluted_usd": null,
    "per_day_in_range_nock_leg": null,
    "per_day_in_range_usd": null,
    "per_day_in_range_usdc_leg": null,
    "rate": null,
    "reason": "this snapshot has no fee_yield (2.10.0): fees are not measured in it",
    "replay_reason": null,
    "skipped": null,
    "skipped_seconds": null,
    "spread": null,
    "spread_reason": null,
    "status": "unavailable",
    "stored_history_replay": null
  },
  "floor_check": {
    "budget_note": null,
    "floor_breach_effect": {
      "after": {
        "crossing": {
          "level_nock": "100000",
          "multiple": "1.4186",
          "pct_above_spot": "41.9",
          "price_usd": "0.018363",
          "status": "crosses"
        },
        "nock_now": "109543",
        "reason": null,
        "status": "crosses",
        "tolerated": {
          "level_nock": "90000",
          "multiple": "4.4330",
          "pct_above_spot": "343.3",
          "price_usd": "0.057381",
          "status": "crosses"
        }
      },
      "assumption": "Each pool is moved by the same multiple from its own current price. WETH/NOCK ranges hold ETH/USD constant (a NOCK/USD multiple is the same NOCK/ETH multiple). Full-range pools follow x*y=k on the wallet's share and never sell out. Uncollected NOCK fees count as held. Gas-side ranges with no NOCK leg hold no NOCK. The unsnapped ladder's bounds are rounded to whole ticks for this figure.",
      "before": {
        "crossing": {
          "level_nock": "100000",
          "multiple": "1.9626",
          "pct_above_spot": "96.3",
          "price_usd": "0.025404",
          "status": "crosses"
        },
        "nock_now": "109543",
        "reason": null,
        "status": "crosses",
        "tolerated": {
          "level_nock": "90000",
          "multiple": null,
          "pct_above_spot": null,
          "price_usd": null,
          "status": "never"
        }
      },
      "reason": "the ladder's NOCK leaves the wallet and sells as NOCK rises to the top",
      "status": "lowers"
    },
    "lp_nock_budget": "86.7891",
    "max_usdc_within_budget": "18.62",
    "nock_needed": "4193.9773",
    "reason": null,
    "rebuild_note": "the rebuild's suggestions this cycle also draw 0 NOCK of the same budget: an alternative use of it, not subtracted",
    "rebuild_suggested_nock": "0.0000",
    "status": "exceeds"
  },
  "grid": [
    {
      "existing_nock_lp_nock": "19013.4444",
      "existing_nock_lp_usdc": "228.03",
      "ladder_nock": "0.0000",
      "ladder_nock_acquired": "-4193.9773",
      "ladder_usdc": "954.83",
      "markers": [
        "top"
      ],
      "pct_of_spot": "+2.00",
      "pool_price_usd": "0.01320288",
      "price_usd": "0.01320288",
      "reserve_headroom_usdc": "878.69",
      "stable_usdc": "1828.69",
      "total_tracked_nock": "105020.02"
    },
    {
      "existing_nock_lp_nock": "19341.9535",
      "existing_nock_lp_usdc": "223.71",
      "ladder_nock": "4193.9773",
      "ladder_nock_acquired": "0.0000",
      "ladder_usdc": "900.00",
      "markers": [
        "spot"
      ],
      "pct_of_spot": "0.00",
      "pool_price_usd": "0.01294400",
      "price_usd": "0.01294400",
      "reserve_headroom_usdc": "819.54",
      "stable_usdc": "1769.54",
      "total_tracked_nock": "109542.50"
    },
    {
      "existing_nock_lp_nock": "20208.1461",
      "existing_nock_lp_usdc": "212.71",
      "ladder_nock": "15252.3984",
      "ladder_nock_acquired": "11058.4211",
      "ladder_usdc": "760.48",
      "markers": [],
      "pct_of_spot": "-5.00",
      "pool_price_usd": "0.01229680",
      "price_usd": "0.01229680",
      "reserve_headroom_usdc": "669.03",
      "stable_usdc": "1619.03",
      "total_tracked_nock": "121467.12"
    },
    {
      "existing_nock_lp_nock": "21145.5540",
      "existing_nock_lp_usdc": "201.43",
      "ladder_nock": "27220.0041",
      "ladder_nock_acquired": "23026.0269",
      "ladder_usdc": "617.25",
      "markers": [],
      "pct_of_spot": "-10.00",
      "pool_price_usd": "0.01164960",
      "price_usd": "0.01164960",
      "reserve_headroom_usdc": "514.51",
      "stable_usdc": "1464.51",
      "total_tracked_nock": "134372.13"
    },
    {
      "existing_nock_lp_nock": "22164.5030",
      "existing_nock_lp_usdc": "189.83",
      "ladder_nock": "40228.6209",
      "ladder_nock_acquired": "36034.6436",
      "ladder_usdc": "469.97",
      "markers": [],
      "pct_of_spot": "-15.00",
      "pool_price_usd": "0.01100240",
      "price_usd": "0.01100240",
      "reserve_headroom_usdc": "355.63",
      "stable_usdc": "1305.63",
      "total_tracked_nock": "148399.70"
    },
    {
      "existing_nock_lp_nock": "23277.5444",
      "existing_nock_lp_usdc": "177.88",
      "ladder_nock": "54438.4865",
      "ladder_nock_acquired": "50244.5092",
      "ladder_usdc": "318.30",
      "markers": [],
      "pct_of_spot": "-20.00",
      "pool_price_usd": "0.01035520",
      "price_usd": "0.01035520",
      "reserve_headroom_usdc": "192.01",
      "stable_usdc": "1142.01",
      "total_tracked_nock": "163722.60"
    },
    {
      "existing_nock_lp_nock": "24500.1121",
      "existing_nock_lp_usdc": "165.55",
      "ladder_nock": "70046.6414",
      "ladder_nock_acquired": "65852.6642",
      "ladder_usdc": "161.80",
      "markers": [],
      "pct_of_spot": "-25.00",
      "pool_price_usd": "0.00970800",
      "price_usd": "0.00970800",
      "reserve_headroom_usdc": "23.18",
      "stable_usdc": "973.18",
      "total_tracked_nock": "180553.32"
    },
    {
      "existing_nock_lp_nock": "25851.4339",
      "existing_nock_lp_usdc": "152.80",
      "ladder_nock": "87298.5623",
      "ladder_nock_acquired": "83104.5850",
      "ladder_usdc": "0.00",
      "markers": [
        "bottom"
      ],
      "pct_of_spot": "-30.00",
      "pool_price_usd": "0.00906080",
      "price_usd": "0.00906080",
      "reserve_headroom_usdc": "-151.37",
      "stable_usdc": "798.63",
      "total_tracked_nock": "199156.57"
    },
    {
      "existing_nock_lp_nock": "27355.8217",
      "existing_nock_lp_usdc": "139.58",
      "ladder_nock": "87298.5623",
      "ladder_nock_acquired": "83104.5850",
      "ladder_usdc": "0.00",
      "markers": [],
      "pct_of_spot": "-35.00",
      "pool_price_usd": "0.00841360",
      "price_usd": "0.00841360",
      "reserve_headroom_usdc": "-164.58",
      "stable_usdc": "785.42",
      "total_tracked_nock": "200660.96"
    },
    {
      "existing_nock_lp_nock": "29044.5410",
      "existing_nock_lp_usdc": "125.85",
      "ladder_nock": "87298.5623",
      "ladder_nock_acquired": "83104.5850",
      "ladder_usdc": "0.00",
      "markers": [],
      "pct_of_spot": "-40.00",
      "pool_price_usd": "0.00776640",
      "price_usd": "0.00776640",
      "reserve_headroom_usdc": "-178.31",
      "stable_usdc": "771.69",
      "total_tracked_nock": "202349.67"
    }
  ],
  "inputs": {
    "grid_step_pct": 5.0,
    "grid_to_pct": -40.0,
    "high_requested": "2%",
    "low_requested": "-30%",
    "nock": null,
    "position_id": null,
    "tick_spacing": null,
    "usdc": "900.00"
  },
  "pool": {
    "label": "v4 USDC/NOCK 3.00% #3163450",
    "note": "unsnapped: no position_id or tick_spacing was given; the app rounds each bound to a usable tick of the pool you choose",
    "pool_price_usd": "0.01294400",
    "pool_price_vs_spot_pct": "0.00",
    "position_id": null,
    "snapped": false,
    "spacing_source": null,
    "tick_spacing": null
  },
  "reserve_check": {
    "accumulation_plan_unspent": null,
    "accumulation_plan_unspent_reason": "no accumulation_plan is configured",
    "at_top": {
      "headroom_usdc": "878.69",
      "ladder_usdc": "954.83",
      "note": "Out of range above (read at the top), the ladder holds only USDC: its NOCK has sold on the way up.",
      "out_of_range_above_now": false,
      "price_usd": "0.01320288",
      "stable_usdc": "1828.69"
    },
    "cash_reserve_usdc": "950.00",
    "existing_ranges_below_bottom": {
      "headroom_usdc": "-178.31",
      "note": "The existing NOCK ranges keep converting below the ladder's bottom: a measurement, not the gate.",
      "pct_of_spot": "-40.00",
      "price_usd": "0.00776640",
      "stable_usdc": "771.69"
    },
    "existing_ranges_below_bottom_reason": null,
    "existing_ranges_usdc_at_bottom": "152.80",
    "gate": "The gate is the stable balance at the ladder's own bottom (operator decision, 2026-11-20).",
    "headroom_at_spot_usdc": "819.54",
    "margin_usdc": "-151.37",
    "max_usdc_for_this_range": "748.63",
    "reason": null,
    "stable_at_bottom_usdc": "798.63",
    "stable_at_spot_usdc": "1769.54",
    "status": "breached",
    "tax_reserve_usd": {
      "is": "stated, not a gate",
      "usd": "0.00"
    }
  },
  "sizing": {
    "liquidity_human": "48430.1458",
    "liquidity_raw": "4843014581127545",
    "nock_check": "not_given",
    "nock_given": null,
    "nock_needed_at_spot": "4193.9773",
    "usdc_at_spot": "900.00"
  },
  "snapshot": {
    "age_minutes": 5,
    "age_seconds": 300,
    "generated_at": "2026-11-20T12:00:00Z",
    "snapshot_id": "00000000-0000-0000-0000-000000000000",
    "spot_source": "price-source pool sqrtPriceX96 (v4 USDC/NOCK 3.00% #3163450)",
    "spot_usd": "0.01294400",
    "version": "2.12.0",
    "worker_health": "ok"
  },
  "statement": "A what-if from the stored snapshot. NockNock reads nothing new, signs nothing, sends nothing, and does not choose the amounts.",
  "top": {
    "avg_sell_price_usd": "0.01307280",
    "multiple_of_spot": "1.0200",
    "out_of_range_above_now": false,
    "pct_of_spot": "+2.00",
    "price_usd": "0.01320288",
    "usdc_held": "954.83"
  },
  "top_neighbours": null,
  "top_neighbours_reason": "unsnapped: there is no tick grid, so there are no neighbouring tops"
};

export const NOCK_TOO_MUCH: LadderErrorBody = {
  "bounds": {
    "high_pct": "0.00",
    "high_usd": "0.01294400",
    "low_pct": "-22.74",
    "low_usd": "0.01000000",
    "tick_lower": null,
    "tick_upper": null
  },
  "code": "NOCK_TOO_MUCH",
  "detail": "this range at the pool price takes 0.0000 NOCK with $741.00 USDC; 50 was given",
  "error": "LADDER_NOT_SIMULATED",
  "nock_needed": "0.0000"
};
