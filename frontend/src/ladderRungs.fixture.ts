/**
 * The worker's GET /v1/simulate/ladder bodies for a ladder of rungs and for the
 * operator's configured shape (2.10.0), over the #166 fixture with the recorded fee
 * measurement, copied from ingestion-worker/testdata/ladder/golden_ladder_rungs.json, which
 * ingestion-worker/src/tests_ladder_rungs.rs writes. Depths, amounts and splits are fixture
 * inputs only. ladderRungs.test.ts checks the copy against it.
 */
import type { LadderSimulation } from './ladder';

export const TWO_RUNGS_MEASURED: LadderSimulation = {
  "assumptions": [
    "Every NOCK pool moves by the same percentage as spot, each from its own current price; row 0 of the grid is the snapshot.",
    "Fees and gas are not counted.",
    "WETH/NOCK ranges hold ETH/USD constant and add no USDC.",
    "Gas-side ranges with no NOCK leg (ETH/USDC, #151) are left out of the stable balance and the NOCK totals.",
    "The ladder's USDC comes from the wallet and its NOCK from wallet NOCK.",
    "Each rung is its own position (its own NFT) in the same pool; overlapping rungs are allowed and their liquidity adds.",
    "The rungs share one reserve gate and one NOCK budget.",
    "A deep rung keeps its NOCK on a rebound only if it is pulled when it is fully NOCK; pulling is a withdrawal you sign, and its gas is not counted."
  ],
  "bottom": {
    "avg_buy_price_usd": "0.01075813",
    "describes": "the sum over the rungs at the deepest rung's bottom",
    "nock_bought": "55771.7551",
    "nock_held": "55771.7551",
    "pct_of_spot": "-22.56",
    "price_usd": "0.01002405"
  },
  "bounds": {
    "describes": "the span from the deepest rung's bottom to the highest rung's top",
    "high_pct": "-1.55",
    "high_rule": "per rung: rungs[].bounds.high_rule",
    "high_usd": "0.01274292",
    "low_pct": "-22.56",
    "low_usd": "0.01002405",
    "tick_lower": null,
    "tick_upper": null
  },
  "combined": {
    "avg_buy_price_usd": "0.01075813",
    "avg_kept_price_usd": "0.01048542",
    "deep_rungs": 1,
    "deepest_bottom_usd": "0.01002405",
    "gate_price_rule": "the deepest rung's bottom: operator_profile.ladder_bottom_usd is not set, so the gate falls back to it",
    "gate_price_usd": "0.01002405",
    "max_rungs": 4,
    "max_rungs_note": "display constants, not settings",
    "max_usdc_total": "767.45",
    "nock_bought_at_deepest_bottom": "55771.7551",
    "nock_bought_by_deep_rungs": "47685.2789",
    "nock_held_at_deepest_bottom": "55771.7551",
    "nock_kept_if_deep_rungs_pulled": "47685.2789",
    "overlaps": [],
    "rung_count": 2,
    "total_nock_needed": "0.0000",
    "total_usdc": "600.00"
  },
  "fee_estimate": null,
  "fee_estimate_reason": "per rung: rungs[].fee_estimate; each grid row sums the rungs in range there",
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
    "nock_needed_by_rung": [
      {
        "index": 1,
        "nock_needed": "0.0000"
      },
      {
        "index": 2,
        "nock_needed": "0.0000"
      }
    ],
    "reason": null,
    "rebuild_note": "the rebuild's suggestions this cycle also draw 0 NOCK of the same budget: an alternative use of it, not subtracted",
    "rebuild_suggested_nock": "0.0000",
    "status": "fits"
  },
  "grid": [
    {
      "est_fee_reason": "no rung is in range at this price, so none earns fees",
      "est_fee_usd_per_day": "0.00",
      "est_fee_usd_per_day_diluted": "0.00",
      "existing_nock_lp_nock": "19341.9535",
      "existing_nock_lp_usdc": "223.71",
      "ladder_nock": "0.0000",
      "ladder_nock_acquired": "0.0000",
      "ladder_usdc": "600.00",
      "markers": [
        "spot"
      ],
      "pct_of_spot": "0.00",
      "pool_price_usd": "0.01294400",
      "price_usd": "0.01294400",
      "reserve_headroom_usdc": "819.54",
      "rungs": [
        {
          "in_range": false,
          "index": 1,
          "ladder_nock": "0.0000",
          "ladder_usdc": "100.00"
        },
        {
          "in_range": false,
          "index": 2,
          "ladder_nock": "0.0000",
          "ladder_usdc": "500.00"
        }
      ],
      "rungs_in_range": [],
      "stable_usdc": "1769.54",
      "total_tracked_nock": "109542.50"
    },
    {
      "est_fee_reason": null,
      "est_fee_usd_per_day": "18.23",
      "est_fee_usd_per_day_diluted": "17.57",
      "existing_nock_lp_nock": "19603.9982",
      "existing_nock_lp_usdc": "220.32",
      "ladder_nock": "0.0000",
      "ladder_nock_acquired": "0.0000",
      "ladder_usdc": "600.00",
      "markers": [
        "r1 top"
      ],
      "pct_of_spot": "-1.55",
      "pool_price_usd": "0.01274292",
      "price_usd": "0.01274292",
      "reserve_headroom_usdc": "816.16",
      "rungs": [
        {
          "in_range": true,
          "index": 1,
          "ladder_nock": "0.0000",
          "ladder_usdc": "100.00"
        },
        {
          "in_range": false,
          "index": 2,
          "ladder_nock": "0.0000",
          "ladder_usdc": "500.00"
        }
      ],
      "rungs_in_range": [
        1
      ],
      "stable_usdc": "1766.16",
      "total_tracked_nock": "109804.55"
    },
    {
      "est_fee_reason": null,
      "est_fee_usd_per_day": "18.23",
      "est_fee_usd_per_day_diluted": "17.57",
      "existing_nock_lp_nock": "20208.1461",
      "existing_nock_lp_usdc": "212.71",
      "ladder_nock": "4773.8637",
      "ladder_nock_acquired": "4773.8637",
      "ladder_usdc": "540.24",
      "markers": [],
      "pct_of_spot": "-5.00",
      "pool_price_usd": "0.01229680",
      "price_usd": "0.01229680",
      "reserve_headroom_usdc": "748.79",
      "rungs": [
        {
          "in_range": true,
          "index": 1,
          "ladder_nock": "4773.8637",
          "ladder_usdc": "40.24"
        },
        {
          "in_range": false,
          "index": 2,
          "ladder_nock": "0.0000",
          "ladder_usdc": "500.00"
        }
      ],
      "rungs_in_range": [
        1
      ],
      "stable_usdc": "1698.79",
      "total_tracked_nock": "115182.56"
    },
    {
      "est_fee_reason": "no rung is in range at this price, so none earns fees",
      "est_fee_usd_per_day": "0.00",
      "est_fee_usd_per_day_diluted": "0.00",
      "existing_nock_lp_nock": "20627.3680",
      "existing_nock_lp_usdc": "207.59",
      "ladder_nock": "8086.4763",
      "ladder_nock_acquired": "8086.4763",
      "ladder_usdc": "500.00",
      "markers": [
        "r1 bottom"
      ],
      "pct_of_spot": "-7.29",
      "pool_price_usd": "0.01200086",
      "price_usd": "0.01200086",
      "reserve_headroom_usdc": "703.43",
      "rungs": [
        {
          "in_range": false,
          "index": 1,
          "ladder_nock": "8086.4763",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": false,
          "index": 2,
          "ladder_nock": "0.0000",
          "ladder_usdc": "500.00"
        }
      ],
      "rungs_in_range": [],
      "stable_usdc": "1653.43",
      "total_tracked_nock": "118914.39"
    },
    {
      "est_fee_reason": "no rung is in range at this price, so none earns fees",
      "est_fee_usd_per_day": "0.00",
      "est_fee_usd_per_day_diluted": "0.00",
      "existing_nock_lp_nock": "21145.5540",
      "existing_nock_lp_usdc": "201.43",
      "ladder_nock": "8086.4763",
      "ladder_nock_acquired": "8086.4763",
      "ladder_usdc": "500.00",
      "markers": [],
      "pct_of_spot": "-10.00",
      "pool_price_usd": "0.01164960",
      "price_usd": "0.01164960",
      "reserve_headroom_usdc": "697.26",
      "rungs": [
        {
          "in_range": false,
          "index": 1,
          "ladder_nock": "8086.4763",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": false,
          "index": 2,
          "ladder_nock": "0.0000",
          "ladder_usdc": "500.00"
        }
      ],
      "rungs_in_range": [],
      "stable_usdc": "1647.26",
      "total_tracked_nock": "119432.58"
    },
    {
      "est_fee_reason": "no rung is in range at this price, so none earns fees",
      "est_fee_usd_per_day": "0.00",
      "est_fee_usd_per_day_diluted": "0.00",
      "existing_nock_lp_nock": "22164.5030",
      "existing_nock_lp_usdc": "189.83",
      "ladder_nock": "8086.4763",
      "ladder_nock_acquired": "8086.4763",
      "ladder_usdc": "500.00",
      "markers": [],
      "pct_of_spot": "-15.00",
      "pool_price_usd": "0.01100240",
      "price_usd": "0.01100240",
      "reserve_headroom_usdc": "685.66",
      "rungs": [
        {
          "in_range": false,
          "index": 1,
          "ladder_nock": "8086.4763",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": false,
          "index": 2,
          "ladder_nock": "0.0000",
          "ladder_usdc": "500.00"
        }
      ],
      "rungs_in_range": [],
      "stable_usdc": "1635.66",
      "total_tracked_nock": "120451.53"
    },
    {
      "est_fee_reason": null,
      "est_fee_usd_per_day": "65.98",
      "est_fee_usd_per_day_diluted": "58.12",
      "existing_nock_lp_nock": "22221.1519",
      "existing_nock_lp_usdc": "189.20",
      "ladder_nock": "8086.4763",
      "ladder_nock_acquired": "8086.4763",
      "ladder_usdc": "500.00",
      "markers": [
        "r2 top"
      ],
      "pct_of_spot": "-15.27",
      "pool_price_usd": "0.01096801",
      "price_usd": "0.01096801",
      "reserve_headroom_usdc": "685.04",
      "rungs": [
        {
          "in_range": false,
          "index": 1,
          "ladder_nock": "8086.4763",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": true,
          "index": 2,
          "ladder_nock": "0.0000",
          "ladder_usdc": "500.00"
        }
      ],
      "rungs_in_range": [
        2
      ],
      "stable_usdc": "1635.04",
      "total_tracked_nock": "120508.18"
    },
    {
      "est_fee_reason": null,
      "est_fee_usd_per_day": "65.98",
      "est_fee_usd_per_day_diluted": "58.12",
      "existing_nock_lp_nock": "23277.5444",
      "existing_nock_lp_usdc": "177.88",
      "ladder_nock": "38302.5427",
      "ladder_nock_acquired": "38302.5427",
      "ladder_usdc": "177.98",
      "markers": [],
      "pct_of_spot": "-20.00",
      "pool_price_usd": "0.01035520",
      "price_usd": "0.01035520",
      "reserve_headroom_usdc": "351.69",
      "rungs": [
        {
          "in_range": false,
          "index": 1,
          "ladder_nock": "8086.4763",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": true,
          "index": 2,
          "ladder_nock": "30216.0664",
          "ladder_usdc": "177.98"
        }
      ],
      "rungs_in_range": [
        2
      ],
      "stable_usdc": "1301.69",
      "total_tracked_nock": "151780.64"
    },
    {
      "est_fee_reason": "no rung is in range at this price, so none earns fees",
      "est_fee_usd_per_day": "0.00",
      "est_fee_usd_per_day_diluted": "0.00",
      "existing_nock_lp_nock": "23888.2905",
      "existing_nock_lp_usdc": "171.62",
      "ladder_nock": "55771.7551",
      "ladder_nock_acquired": "55771.7551",
      "ladder_usdc": "0.00",
      "markers": [
        "r2 bottom",
        "gate"
      ],
      "pct_of_spot": "-22.56",
      "pool_price_usd": "0.01002405",
      "price_usd": "0.01002405",
      "reserve_headroom_usdc": "167.45",
      "rungs": [
        {
          "in_range": false,
          "index": 1,
          "ladder_nock": "8086.4763",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": false,
          "index": 2,
          "ladder_nock": "47685.2789",
          "ladder_usdc": "0.00"
        }
      ],
      "rungs_in_range": [],
      "stable_usdc": "1117.45",
      "total_tracked_nock": "169860.59"
    },
    {
      "est_fee_reason": "no rung is in range at this price, so none earns fees",
      "est_fee_usd_per_day": "0.00",
      "est_fee_usd_per_day_diluted": "0.00",
      "existing_nock_lp_nock": "24500.1121",
      "existing_nock_lp_usdc": "165.55",
      "ladder_nock": "55771.7551",
      "ladder_nock_acquired": "55771.7551",
      "ladder_usdc": "0.00",
      "markers": [],
      "pct_of_spot": "-25.00",
      "pool_price_usd": "0.00970800",
      "price_usd": "0.00970800",
      "reserve_headroom_usdc": "161.38",
      "rungs": [
        {
          "in_range": false,
          "index": 1,
          "ladder_nock": "8086.4763",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": false,
          "index": 2,
          "ladder_nock": "47685.2789",
          "ladder_usdc": "0.00"
        }
      ],
      "rungs_in_range": [],
      "stable_usdc": "1111.38",
      "total_tracked_nock": "170472.42"
    },
    {
      "est_fee_reason": "no rung is in range at this price, so none earns fees",
      "est_fee_usd_per_day": "0.00",
      "est_fee_usd_per_day_diluted": "0.00",
      "existing_nock_lp_nock": "25851.4339",
      "existing_nock_lp_usdc": "152.80",
      "ladder_nock": "55771.7551",
      "ladder_nock_acquired": "55771.7551",
      "ladder_usdc": "0.00",
      "markers": [],
      "pct_of_spot": "-30.00",
      "pool_price_usd": "0.00906080",
      "price_usd": "0.00906080",
      "reserve_headroom_usdc": "148.63",
      "rungs": [
        {
          "in_range": false,
          "index": 1,
          "ladder_nock": "8086.4763",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": false,
          "index": 2,
          "ladder_nock": "47685.2789",
          "ladder_usdc": "0.00"
        }
      ],
      "rungs_in_range": [],
      "stable_usdc": "1098.63",
      "total_tracked_nock": "171823.74"
    },
    {
      "est_fee_reason": "no rung is in range at this price, so none earns fees",
      "est_fee_usd_per_day": "0.00",
      "est_fee_usd_per_day_diluted": "0.00",
      "existing_nock_lp_nock": "27355.8217",
      "existing_nock_lp_usdc": "139.58",
      "ladder_nock": "55771.7551",
      "ladder_nock_acquired": "55771.7551",
      "ladder_usdc": "0.00",
      "markers": [],
      "pct_of_spot": "-35.00",
      "pool_price_usd": "0.00841360",
      "price_usd": "0.00841360",
      "reserve_headroom_usdc": "135.42",
      "rungs": [
        {
          "in_range": false,
          "index": 1,
          "ladder_nock": "8086.4763",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": false,
          "index": 2,
          "ladder_nock": "47685.2789",
          "ladder_usdc": "0.00"
        }
      ],
      "rungs_in_range": [],
      "stable_usdc": "1085.42",
      "total_tracked_nock": "173328.13"
    },
    {
      "est_fee_reason": "no rung is in range at this price, so none earns fees",
      "est_fee_usd_per_day": "0.00",
      "est_fee_usd_per_day_diluted": "0.00",
      "existing_nock_lp_nock": "29044.5410",
      "existing_nock_lp_usdc": "125.85",
      "ladder_nock": "55771.7551",
      "ladder_nock_acquired": "55771.7551",
      "ladder_usdc": "0.00",
      "markers": [],
      "pct_of_spot": "-40.00",
      "pool_price_usd": "0.00776640",
      "price_usd": "0.00776640",
      "reserve_headroom_usdc": "121.69",
      "rungs": [
        {
          "in_range": false,
          "index": 1,
          "ladder_nock": "8086.4763",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": false,
          "index": 2,
          "ladder_nock": "47685.2789",
          "ladder_usdc": "0.00"
        }
      ],
      "rungs_in_range": [],
      "stable_usdc": "1071.69",
      "total_tracked_nock": "175016.84"
    }
  ],
  "inputs": {
    "grid_step_pct": 5.0,
    "grid_to_pct": -40.0,
    "high_requested": null,
    "low_requested": null,
    "nock": null,
    "position_id": "uniswap-v4:base:3163450",
    "rungs": [
      {
        "high_requested": "spot",
        "index": 1,
        "low_requested": "0.012",
        "nock": null,
        "rung": "near",
        "usdc": "100.00"
      },
      {
        "high_requested": "0.011",
        "index": 2,
        "low_requested": "0.01",
        "nock": null,
        "rung": "deep",
        "usdc": "500.00"
      }
    ],
    "split_step_note": "display constants, not settings",
    "split_step_pct": 10.0,
    "tick_spacing": null,
    "usdc": "600.00"
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
      "ladder_usdc": "600.00",
      "note": "Every rung is out of range above now and holds only USDC: the stable balance at spot is unchanged by them, and a rise takes none of it.",
      "out_of_range_above_now": true,
      "price_usd": "0.01294400",
      "stable_usdc": "1769.54"
    },
    "cash_reserve_usdc": "950.00",
    "existing_ranges_below_bottom": {
      "headroom_usdc": "121.69",
      "note": "The existing NOCK ranges keep converting below the ladder's bottom: a measurement, not the gate.",
      "pct_of_spot": "-40.00",
      "price_usd": "0.00776640",
      "stable_usdc": "1071.69"
    },
    "existing_ranges_below_bottom_reason": null,
    "existing_ranges_usdc_at_bottom": "171.62",
    "gate": "The gate is the stable balance where every rung is fully NOCK: at the lower of the deepest rung's bottom and your ladder bottom (operator_profile.ladder_bottom_usd) when it is set, otherwise at the deepest rung's bottom; the *_at_bottom figures are read at gate_price_usd.",
    "gate_price_rule": "the deepest rung's bottom: operator_profile.ladder_bottom_usd is not set, so the gate falls back to it",
    "gate_price_usd": "0.01002405",
    "headroom_at_spot_usdc": "819.54",
    "margin_usdc": "167.45",
    "max_usdc_for_this_range": "767.45",
    "reason": null,
    "stable_at_bottom_usdc": "1117.45",
    "stable_at_spot_usdc": "1769.54",
    "status": "holds",
    "tax_reserve_usd": {
      "is": "stated, not a gate",
      "usd": "0.00"
    }
  },
  "rungs": [
    {
      "bottom": {
        "avg_buy_price_usd": "0.01236633",
        "nock_bought": "8086.4763",
        "nock_held": "8086.4763",
        "pct_of_spot": "-7.29",
        "price_usd": "0.01200086"
      },
      "bounds": {
        "high_pct": "-1.55",
        "high_rule": "the usable tick at or below the pool price (high = spot): it holds no NOCK",
        "high_usd": "0.01274292",
        "low_pct": "-7.29",
        "low_usd": "0.01200086",
        "tick_lower": 273900,
        "tick_upper": 274500
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
        "band_measured": {
          "days": "0.01",
          "seconds": "600",
          "usd_per_day": null,
          "usd_total_upper": "1.31"
        },
        "band_reason": "only 600 s of stored intervals sat wholly inside: below the 72000 s a per-day figure needs, so only the dollars and seconds are stated",
        "constants": {
          "band_min_seconds": 72000.0,
          "day_min_seconds": 72000,
          "max_gap_seconds": 7200,
          "note": "display constants, not settings",
          "window_days": 30
        },
        "coverage": {
          "high_usd": "0.01274292",
          "low_usd": "0.01200086",
          "note": "the whole range lies inside the measuring range",
          "whole": true
        },
        "dilution": {
          "active_liquidity_raw": "80173935548453779",
          "factor": "0.963960",
          "new_liquidity_raw": "2997532031710744",
          "share_of_active_pct": "3.60"
        },
        "dilution_reason": null,
        "in_range_share_reason": null,
        "in_range_share_stored_history": {
          "edge_seconds": "600",
          "note": "the share of the measured seconds whose two endpoint ticks were both inside this range: a past measurement, not a forecast",
          "of_seconds": "547205",
          "seconds": "600",
          "share_pct": "0.11"
        },
        "intervals": 1836,
        "measured_at": "2026-11-20T11:50:00Z",
        "measured_by": "uniswap-v4:base:3163450",
        "measured_days": "6.33",
        "measured_label": "v4 USDC/NOCK 3.00% #3163450",
        "measured_seconds": 547205.136,
        "measurement_age_seconds": 900,
        "per_100_usd_per_day_in_range_usd": "18.23",
        "per_day_in_range_diluted_usd": "17.57",
        "per_day_in_range_nock_leg": "664.1278",
        "per_day_in_range_usd": "18.23",
        "per_day_in_range_usdc_leg": "9.63",
        "rate": {
          "nock_per_liquidity_day": "2.215582e-13",
          "usd_per_1e15_liquidity_day": "6.0811",
          "usdc_per_liquidity_day": "3.213286e-15",
          "valued_at_pool_price_usd": "0.01294400"
        },
        "reason": null,
        "replay_reason": null,
        "skipped": {
          "fees_fell": 6,
          "gap": 1,
          "liquidity_changed": 4,
          "out_of_range": 0,
          "pool_inactive": 0,
          "staked": 0,
          "unreadable": 0,
          "zero_liquidity": 137
        },
        "skipped_seconds": {
          "fees_fell": 1776.479,
          "gap": 14179.252,
          "liquidity_changed": 1202.24,
          "out_of_range": 0.0,
          "pool_inactive": 0.0,
          "staked": 0.0,
          "unreadable": 0.0,
          "zero_liquidity": 38413.578
        },
        "spread": {
          "day_min_seconds": 72000.0,
          "days": [
            {
              "counted": false,
              "date": "2026-11-14",
              "seconds": 30699.262,
              "usd_per_day": "11.51"
            },
            {
              "counted": true,
              "date": "2026-11-15",
              "seconds": 86488.395,
              "usd_per_day": "15.02"
            },
            {
              "counted": true,
              "date": "2026-11-16",
              "seconds": 86006.946,
              "usd_per_day": "21.56"
            },
            {
              "counted": false,
              "date": "2026-11-17",
              "seconds": 71106.303,
              "usd_per_day": "29.18"
            },
            {
              "counted": true,
              "date": "2026-11-18",
              "seconds": 86146.129,
              "usd_per_day": "9.28"
            },
            {
              "counted": true,
              "date": "2026-11-19",
              "seconds": 85799.496,
              "usd_per_day": "23.95"
            },
            {
              "counted": true,
              "date": "2026-11-20",
              "seconds": 86399.664,
              "usd_per_day": "15.86"
            },
            {
              "counted": false,
              "date": "2026-11-21",
              "seconds": 14558.942,
              "usd_per_day": "11.57"
            }
          ],
          "days_counted": 5,
          "max_usd_per_day": "23.95",
          "median_usd_per_day": "15.86",
          "min_usd_per_day": "9.28",
          "note": "UTC days; an interval counts on the date it starts. Every day is listed; the spread is over the days counted."
        },
        "spread_reason": null,
        "status": "measured",
        "stored_history_replay": {
          "edge_seconds": "600",
          "edge_upper_usd": "4.70",
          "inside_seconds": "600",
          "inside_upper_usd": "1.31",
          "measured_days": "6.33",
          "note": "At most: from endpoint ticks sampled every ~5 minutes, an interval counts when both ends sat inside the range (inside) or one did (edge); the price can leave and return within an interval, and a part of a range never earns more per unit of liquidity than the range containing it. No lower bound is claimed (it can be $0).",
          "undiluted": true
        }
      },
      "if_pulled_at_fill": null,
      "index": 1,
      "inputs": {
        "high_requested": "spot",
        "low_requested": "0.012",
        "nock": null,
        "usdc": "100.00"
      },
      "max_usdc_with_the_others_as_given": "267.45",
      "mint": {
        "amount0_raw": "100000000",
        "amount1_raw": "0",
        "liquidity_raw": "2997532031714950",
        "nock": "0.0000000000000000",
        "nock_max_raw": "0",
        "nock_raw": "0",
        "note": "liquidity_raw is what these amounts buy, rounded down (LiquidityAmounts); amount0_raw and amount1_raw are what a mint of it pulls, rounded up (SqrtPriceMath), never more than the maxima (the rung's USDC and NOCK truncated to raw units)",
        "status": "exact",
        "tick": 273743,
        "usdc": "100.000000",
        "usdc_max_raw": "100000000",
        "usdc_raw": "100000000"
      },
      "round_trip": {
        "back_to_usd": "0.01294400",
        "down_to_usd": "0.01200086",
        "nock_back": "0.0000",
        "nock_start": "0.0000",
        "text": "A fall to $0.01200086 and a return to $0.01294400 (the pool price now) hands back $100.00 USDC and 0.0000 NOCK, what this rung started with: the round trip's gain is the fees it earned in range (fee_estimate), and gas is not counted.",
        "usdc_back": "100.00",
        "usdc_start": "100.00"
      },
      "rung": "near",
      "sizing": {
        "liquidity_human": "29975.3203",
        "liquidity_raw": "2997532031710744",
        "nock_check": "not_given",
        "nock_given": null,
        "nock_needed_at_spot": "0.0000",
        "usdc_at_spot": "100.00"
      },
      "top": {
        "out_of_range_above_now": true,
        "pct_of_spot": "-1.55",
        "price_usd": "0.01274292",
        "usdc_held": "100.00"
      }
    },
    {
      "bottom": {
        "avg_buy_price_usd": "0.01048542",
        "nock_bought": "47685.2789",
        "nock_held": "47685.2789",
        "pct_of_spot": "-22.56",
        "price_usd": "0.01002405"
      },
      "bounds": {
        "high_pct": "-15.27",
        "high_rule": "the nearest usable tick",
        "high_usd": "0.01096801",
        "low_pct": "-22.56",
        "low_usd": "0.01002405",
        "tick_lower": 275400,
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
        "band_measured": {
          "days": "0.37",
          "seconds": "31832",
          "usd_per_day": null,
          "usd_total_upper": "25.00"
        },
        "band_reason": "only 31832 s of stored intervals sat wholly inside: below the 72000 s a per-day figure needs, so only the dollars and seconds are stated",
        "constants": {
          "band_min_seconds": 72000.0,
          "day_min_seconds": 72000,
          "max_gap_seconds": 7200,
          "note": "display constants, not settings",
          "window_days": 30
        },
        "coverage": {
          "high_usd": "0.01096801",
          "low_usd": "0.01002405",
          "note": "the whole range lies inside the measuring range",
          "whole": true
        },
        "dilution": {
          "active_liquidity_raw": "80173935548453779",
          "factor": "0.880796",
          "new_liquidity_raw": "10850499376755608",
          "share_of_active_pct": "11.92"
        },
        "dilution_reason": null,
        "in_range_share_reason": null,
        "in_range_share_stored_history": {
          "edge_seconds": "1801",
          "note": "the share of the measured seconds whose two endpoint ticks were both inside this range: a past measurement, not a forecast",
          "of_seconds": "547205",
          "seconds": "31832",
          "share_pct": "5.82"
        },
        "intervals": 1836,
        "measured_at": "2026-11-20T11:50:00Z",
        "measured_by": "uniswap-v4:base:3163450",
        "measured_days": "6.33",
        "measured_label": "v4 USDC/NOCK 3.00% #3163450",
        "measured_seconds": 547205.136,
        "measurement_age_seconds": 900,
        "per_100_usd_per_day_in_range_usd": "13.20",
        "per_day_in_range_diluted_usd": "58.12",
        "per_day_in_range_nock_leg": "2404.0171",
        "per_day_in_range_usd": "65.98",
        "per_day_in_range_usdc_leg": "34.87",
        "rate": {
          "nock_per_liquidity_day": "2.215582e-13",
          "usd_per_1e15_liquidity_day": "6.0811",
          "usdc_per_liquidity_day": "3.213286e-15",
          "valued_at_pool_price_usd": "0.01294400"
        },
        "reason": null,
        "replay_reason": null,
        "skipped": {
          "fees_fell": 6,
          "gap": 1,
          "liquidity_changed": 4,
          "out_of_range": 0,
          "pool_inactive": 0,
          "staked": 0,
          "unreadable": 0,
          "zero_liquidity": 137
        },
        "skipped_seconds": {
          "fees_fell": 1776.479,
          "gap": 14179.252,
          "liquidity_changed": 1202.24,
          "out_of_range": 0.0,
          "pool_inactive": 0.0,
          "staked": 0.0,
          "unreadable": 0.0,
          "zero_liquidity": 38413.578
        },
        "spread": {
          "day_min_seconds": 72000.0,
          "days": [
            {
              "counted": false,
              "date": "2026-11-14",
              "seconds": 30699.262,
              "usd_per_day": "41.66"
            },
            {
              "counted": true,
              "date": "2026-11-15",
              "seconds": 86488.395,
              "usd_per_day": "54.36"
            },
            {
              "counted": true,
              "date": "2026-11-16",
              "seconds": 86006.946,
              "usd_per_day": "78.04"
            },
            {
              "counted": false,
              "date": "2026-11-17",
              "seconds": 71106.303,
              "usd_per_day": "105.64"
            },
            {
              "counted": true,
              "date": "2026-11-18",
              "seconds": 86146.129,
              "usd_per_day": "33.60"
            },
            {
              "counted": true,
              "date": "2026-11-19",
              "seconds": 85799.496,
              "usd_per_day": "86.69"
            },
            {
              "counted": true,
              "date": "2026-11-20",
              "seconds": 86399.664,
              "usd_per_day": "57.42"
            },
            {
              "counted": false,
              "date": "2026-11-21",
              "seconds": 14558.942,
              "usd_per_day": "41.88"
            }
          ],
          "days_counted": 5,
          "max_usd_per_day": "86.69",
          "median_usd_per_day": "57.42",
          "min_usd_per_day": "33.60",
          "note": "UTC days; an interval counts on the date it starts. Every day is listed; the spread is over the days counted."
        },
        "spread_reason": null,
        "status": "measured",
        "stored_history_replay": {
          "edge_seconds": "1801",
          "edge_upper_usd": "38.41",
          "inside_seconds": "31832",
          "inside_upper_usd": "25.00",
          "measured_days": "6.33",
          "note": "At most: from endpoint ticks sampled every ~5 minutes, an interval counts when both ends sat inside the range (inside) or one did (edge); the price can leave and return within an interval, and a part of a range never earns more per unit of liquidity than the range containing it. No lower bound is claimed (it can be $0).",
          "undiluted": true
        }
      },
      "if_pulled_at_fill": {
        "at_usd": "0.01002405",
        "avg_price_usd": "0.01048542",
        "nock_bought": "47685.2789",
        "nock_kept": "47685.2789",
        "note": "Pulled when fully NOCK (at its bottom), the rung keeps this NOCK on a rebound; left in place, a rebound sells it back. Pulling is a withdrawal you sign; gas is not counted."
      },
      "index": 2,
      "inputs": {
        "high_requested": "0.011",
        "low_requested": "0.01",
        "nock": null,
        "usdc": "500.00"
      },
      "max_usdc_with_the_others_as_given": "667.45",
      "mint": {
        "amount0_raw": "500000000",
        "amount1_raw": "0",
        "liquidity_raw": "10850499376770899",
        "nock": "0.0000000000000000",
        "nock_max_raw": "0",
        "nock_raw": "0",
        "note": "liquidity_raw is what these amounts buy, rounded down (LiquidityAmounts); amount0_raw and amount1_raw are what a mint of it pulls, rounded up (SqrtPriceMath), never more than the maxima (the rung's USDC and NOCK truncated to raw units)",
        "status": "exact",
        "tick": 273743,
        "usdc": "500.000000",
        "usdc_max_raw": "500000000",
        "usdc_raw": "500000000"
      },
      "round_trip": null,
      "rung": "deep",
      "sizing": {
        "liquidity_human": "108504.9938",
        "liquidity_raw": "10850499376755608",
        "nock_check": "not_given",
        "nock_given": null,
        "nock_needed_at_spot": "0.0000",
        "usdc_at_spot": "500.00"
      },
      "top": {
        "out_of_range_above_now": true,
        "pct_of_spot": "-15.27",
        "price_usd": "0.01096801",
        "usdc_held": "500.00"
      }
    }
  ],
  "sizing": {
    "describes": "the sum over the rungs; liquidity is per rung (rungs[].sizing)",
    "liquidity_human": null,
    "liquidity_raw": null,
    "nock_check": "per_rung",
    "nock_given": null,
    "nock_needed_at_spot": "0.0000",
    "usdc_at_spot": "600.00"
  },
  "snapshot": {
    "age_minutes": 5,
    "age_seconds": 300,
    "generated_at": "2026-11-20T12:00:00Z",
    "snapshot_id": "00000000-0000-0000-0000-000000000000",
    "spot_source": "price-source pool sqrtPriceX96 (v4 USDC/NOCK 3.00% #3163450)",
    "spot_usd": "0.01294400",
    "version": "2.10.0",
    "worker_health": "ok"
  },
  "statement": "A what-if from the stored snapshot. NockNock reads nothing new, signs nothing, sends nothing, and does not choose the amounts.",
  "top": {
    "avg_sell_price_usd": null,
    "describes": "the sum over the rungs at the highest rung's top",
    "multiple_of_spot": "0.9845",
    "out_of_range_above_now": true,
    "pct_of_spot": "-1.55",
    "price_usd": "0.01274292",
    "usdc_held": "600.00"
  },
  "top_neighbours": null,
  "top_neighbours_reason": "a ladder of rungs lists no neighbouring tops; each rung's bounds are in rungs[]",
  "tradeoff": {
    "columns_note": "Fee per day in range is always shown with the in-range share of stored history and the replay upper bounds: a high rate while in range is not a rate per day.",
    "deep_top": {
      "max_rows": 12,
      "max_rows_note": "display constants, not settings",
      "rows": [
        {
          "avg_buy_price_usd": "0.01117868",
          "avg_kept_price_usd": "0.01096801",
          "budget_status": "fits",
          "deep_high_usd": "0.01200086",
          "gate_price_usd": "0.01002405",
          "nock_at_deepest_bottom": "53673.5854",
          "nock_by_depth": [
            {
              "nock_held": "8086.4763",
              "pct_of_spot": "-7.29",
              "price_usd": "0.01200086"
            },
            {
              "nock_held": "53673.5854",
              "pct_of_spot": "-22.56",
              "price_usd": "0.01002405"
            }
          ],
          "nock_kept_if_deep_rungs_pulled": "45587.1091",
          "nock_needed": "0.0000",
          "per_rung": [
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "18.23",
              "fee_per_day_in_range_diluted_usd": "17.57",
              "fee_per_day_in_range_usd": "18.23",
              "high_usd": "0.01274292",
              "in_range_share_pct": "0.11",
              "index": 1,
              "low_usd": "0.01200086",
              "note": null,
              "replay_edge_upper_usd": "4.70",
              "replay_inside_upper_usd": "1.31",
              "rung": "near",
              "usdc": "100.00"
            },
            {
              "band_usd_per_day": "20.56",
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "6.45",
              "fee_per_day_in_range_diluted_usd": "30.25",
              "fee_per_day_in_range_usd": "32.25",
              "high_usd": "0.01200086",
              "in_range_share_pct": "34.61",
              "index": 2,
              "low_usd": "0.01002405",
              "note": null,
              "replay_edge_upper_usd": "7.67",
              "replay_inside_upper_usd": "45.07",
              "rung": "deep",
              "usdc": "500.00"
            }
          ],
          "reserve_margin_usdc": "167.45",
          "reserve_status": "holds",
          "total_usdc": "600.00"
        },
        {
          "avg_buy_price_usd": "0.01103702",
          "avg_kept_price_usd": "0.01080473",
          "budget_status": "fits",
          "deep_high_usd": "0.01164620",
          "gate_price_usd": "0.01002405",
          "nock_at_deepest_bottom": "54362.5116",
          "nock_by_depth": [
            {
              "nock_held": "8086.4763",
              "pct_of_spot": "-7.29",
              "price_usd": "0.01200086"
            },
            {
              "nock_held": "54362.5116",
              "pct_of_spot": "-22.56",
              "price_usd": "0.01002405"
            }
          ],
          "nock_kept_if_deep_rungs_pulled": "46276.0353",
          "nock_needed": "0.0000",
          "per_rung": [
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "18.23",
              "fee_per_day_in_range_diluted_usd": "17.57",
              "fee_per_day_in_range_usd": "18.23",
              "high_usd": "0.01274292",
              "in_range_share_pct": "0.11",
              "index": 1,
              "low_usd": "0.01200086",
              "note": null,
              "replay_edge_upper_usd": "4.70",
              "replay_inside_upper_usd": "1.31",
              "rung": "near",
              "usdc": "100.00"
            },
            {
              "band_usd_per_day": "24.86",
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "7.80",
              "fee_per_day_in_range_diluted_usd": "36.11",
              "fee_per_day_in_range_usd": "38.99",
              "high_usd": "0.01164620",
              "in_range_share_pct": "34.61",
              "index": 2,
              "low_usd": "0.01002405",
              "note": null,
              "replay_edge_upper_usd": "9.28",
              "replay_inside_upper_usd": "54.50",
              "rung": "deep",
              "usdc": "500.00"
            }
          ],
          "reserve_margin_usdc": "167.45",
          "reserve_status": "holds",
          "total_usdc": "600.00"
        },
        {
          "avg_buy_price_usd": "0.01089684",
          "avg_kept_price_usd": "0.01064388",
          "budget_status": "fits",
          "deep_high_usd": "0.01130202",
          "gate_price_usd": "0.01002405",
          "nock_at_deepest_bottom": "55061.8491",
          "nock_by_depth": [
            {
              "nock_held": "8086.4763",
              "pct_of_spot": "-7.29",
              "price_usd": "0.01200086"
            },
            {
              "nock_held": "55061.8491",
              "pct_of_spot": "-22.56",
              "price_usd": "0.01002405"
            }
          ],
          "nock_kept_if_deep_rungs_pulled": "46975.3728",
          "nock_needed": "0.0000",
          "per_rung": [
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "18.23",
              "fee_per_day_in_range_diluted_usd": "17.57",
              "fee_per_day_in_range_usd": "18.23",
              "high_usd": "0.01274292",
              "in_range_share_pct": "0.11",
              "index": 1,
              "low_usd": "0.01200086",
              "note": null,
              "replay_edge_upper_usd": "4.70",
              "replay_inside_upper_usd": "1.31",
              "rung": "near",
              "usdc": "100.00"
            },
            {
              "band_usd_per_day": "32.48",
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "9.82",
              "fee_per_day_in_range_diluted_usd": "44.62",
              "fee_per_day_in_range_usd": "49.11",
              "high_usd": "0.01130202",
              "in_range_share_pct": "23.08",
              "index": 2,
              "low_usd": "0.01002405",
              "note": null,
              "replay_edge_upper_usd": "16.62",
              "replay_inside_upper_usd": "47.48",
              "rung": "deep",
              "usdc": "500.00"
            }
          ],
          "reserve_margin_usdc": "167.45",
          "reserve_status": "holds",
          "total_usdc": "600.00"
        },
        {
          "avg_buy_price_usd": "0.01075813",
          "avg_kept_price_usd": "0.01048542",
          "budget_status": "fits",
          "deep_high_usd": "0.01096801",
          "gate_price_usd": "0.01002405",
          "nock_at_deepest_bottom": "55771.7551",
          "nock_by_depth": [
            {
              "nock_held": "8086.4763",
              "pct_of_spot": "-7.29",
              "price_usd": "0.01200086"
            },
            {
              "nock_held": "55771.7551",
              "pct_of_spot": "-22.56",
              "price_usd": "0.01002405"
            }
          ],
          "nock_kept_if_deep_rungs_pulled": "47685.2789",
          "nock_needed": "0.0000",
          "per_rung": [
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "18.23",
              "fee_per_day_in_range_diluted_usd": "17.57",
              "fee_per_day_in_range_usd": "18.23",
              "high_usd": "0.01274292",
              "in_range_share_pct": "0.11",
              "index": 1,
              "low_usd": "0.01200086",
              "note": null,
              "replay_edge_upper_usd": "4.70",
              "replay_inside_upper_usd": "1.31",
              "rung": "near",
              "usdc": "100.00"
            },
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "13.20",
              "fee_per_day_in_range_diluted_usd": "58.12",
              "fee_per_day_in_range_usd": "65.98",
              "high_usd": "0.01096801",
              "in_range_share_pct": "5.82",
              "index": 2,
              "low_usd": "0.01002405",
              "note": null,
              "replay_edge_upper_usd": "38.41",
              "replay_inside_upper_usd": "25.00",
              "rung": "deep",
              "usdc": "500.00"
            }
          ],
          "reserve_margin_usdc": "167.45",
          "reserve_status": "holds",
          "total_usdc": "600.00"
        },
        {
          "avg_buy_price_usd": "0.01062090",
          "avg_kept_price_usd": "0.01032932",
          "budget_status": "fits",
          "deep_high_usd": "0.01064388",
          "gate_price_usd": "0.01002405",
          "nock_at_deepest_bottom": "56492.3895",
          "nock_by_depth": [
            {
              "nock_held": "8086.4763",
              "pct_of_spot": "-7.29",
              "price_usd": "0.01200086"
            },
            {
              "nock_held": "56492.3895",
              "pct_of_spot": "-22.56",
              "price_usd": "0.01002405"
            }
          ],
          "nock_kept_if_deep_rungs_pulled": "48405.9133",
          "nock_needed": "0.0000",
          "per_rung": [
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "18.23",
              "fee_per_day_in_range_diluted_usd": "17.57",
              "fee_per_day_in_range_usd": "18.23",
              "high_usd": "0.01274292",
              "in_range_share_pct": "0.11",
              "index": 1,
              "low_usd": "0.01200086",
              "note": null,
              "replay_edge_upper_usd": "4.70",
              "replay_inside_upper_usd": "1.31",
              "rung": "near",
              "usdc": "100.00"
            },
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "19.94",
              "fee_per_day_in_range_diluted_usd": "82.79",
              "fee_per_day_in_range_usd": "99.72",
              "high_usd": "0.01064388",
              "in_range_share_pct": "0.05",
              "index": 2,
              "low_usd": "0.01002405",
              "note": null,
              "replay_edge_upper_usd": "26.42",
              "replay_inside_upper_usd": "2.96",
              "rung": "deep",
              "usdc": "500.00"
            }
          ],
          "reserve_margin_usdc": "167.45",
          "reserve_status": "holds",
          "total_usdc": "600.00"
        },
        {
          "avg_buy_price_usd": "0.01048513",
          "avg_kept_price_usd": "0.01017554",
          "budget_status": "fits",
          "deep_high_usd": "0.01032932",
          "gate_price_usd": "0.01002405",
          "nock_at_deepest_bottom": "57223.9144",
          "nock_by_depth": [
            {
              "nock_held": "8086.4763",
              "pct_of_spot": "-7.29",
              "price_usd": "0.01200086"
            },
            {
              "nock_held": "57223.9144",
              "pct_of_spot": "-22.56",
              "price_usd": "0.01002405"
            }
          ],
          "nock_kept_if_deep_rungs_pulled": "49137.4381",
          "nock_needed": "0.0000",
          "per_rung": [
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "18.23",
              "fee_per_day_in_range_diluted_usd": "17.57",
              "fee_per_day_in_range_usd": "18.23",
              "high_usd": "0.01274292",
              "in_range_share_pct": "0.11",
              "index": 1,
              "low_usd": "0.01200086",
              "note": null,
              "replay_edge_upper_usd": "4.70",
              "replay_inside_upper_usd": "1.31",
              "rung": "near",
              "usdc": "100.00"
            },
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "40.19",
              "fee_per_day_in_range_diluted_usd": "142.30",
              "fee_per_day_in_range_usd": "200.96",
              "high_usd": "0.01032932",
              "in_range_share_pct": "0.00",
              "index": 2,
              "low_usd": "0.01002405",
              "note": null,
              "replay_edge_upper_usd": "41.03",
              "replay_inside_upper_usd": "0.00",
              "rung": "deep",
              "usdc": "500.00"
            }
          ],
          "reserve_margin_usdc": "167.45",
          "reserve_status": "holds",
          "total_usdc": "600.00"
        }
      ],
      "skipped_rows": [],
      "ticks_between": 6,
      "truncated": false
    },
    "deep_top_reason": null,
    "near_depth": {
      "max_rows": 12,
      "max_rows_note": "display constants, not settings",
      "rows": [
        {
          "avg_buy_price_usd": "0.01078141",
          "avg_kept_price_usd": "0.01048542",
          "budget_status": "fits",
          "gate_price_usd": "0.01002405",
          "near_low_usd": "0.01236633",
          "nock_at_deepest_bottom": "55651.3692",
          "nock_by_depth": [
            {
              "nock_held": "7966.0903",
              "pct_of_spot": "-4.46",
              "price_usd": "0.01236633"
            },
            {
              "nock_held": "55651.3692",
              "pct_of_spot": "-22.56",
              "price_usd": "0.01002405"
            }
          ],
          "nock_kept_if_deep_rungs_pulled": "47685.2789",
          "nock_needed": "0.0000",
          "per_rung": [
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "36.19",
              "fee_per_day_in_range_diluted_usd": "33.69",
              "fee_per_day_in_range_usd": "36.19",
              "high_usd": "0.01274292",
              "in_range_share_pct": "0.11",
              "index": 1,
              "low_usd": "0.01236633",
              "note": null,
              "replay_edge_upper_usd": "9.34",
              "replay_inside_upper_usd": "2.60",
              "rung": "near",
              "usdc": "100.00"
            },
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "13.20",
              "fee_per_day_in_range_diluted_usd": "58.12",
              "fee_per_day_in_range_usd": "65.98",
              "high_usd": "0.01096801",
              "in_range_share_pct": "5.82",
              "index": 2,
              "low_usd": "0.01002405",
              "note": null,
              "replay_edge_upper_usd": "38.41",
              "replay_inside_upper_usd": "25.00",
              "rung": "deep",
              "usdc": "500.00"
            }
          ],
          "reserve_margin_usdc": "167.45",
          "reserve_status": "holds",
          "total_usdc": "600.00"
        },
        {
          "avg_buy_price_usd": "0.01075813",
          "avg_kept_price_usd": "0.01048542",
          "budget_status": "fits",
          "gate_price_usd": "0.01002405",
          "near_low_usd": "0.01200086",
          "nock_at_deepest_bottom": "55771.7551",
          "nock_by_depth": [
            {
              "nock_held": "8086.4763",
              "pct_of_spot": "-7.29",
              "price_usd": "0.01200086"
            },
            {
              "nock_held": "55771.7551",
              "pct_of_spot": "-22.56",
              "price_usd": "0.01002405"
            }
          ],
          "nock_kept_if_deep_rungs_pulled": "47685.2789",
          "nock_needed": "0.0000",
          "per_rung": [
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "18.23",
              "fee_per_day_in_range_diluted_usd": "17.57",
              "fee_per_day_in_range_usd": "18.23",
              "high_usd": "0.01274292",
              "in_range_share_pct": "0.11",
              "index": 1,
              "low_usd": "0.01200086",
              "note": null,
              "replay_edge_upper_usd": "4.70",
              "replay_inside_upper_usd": "1.31",
              "rung": "near",
              "usdc": "100.00"
            },
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "13.20",
              "fee_per_day_in_range_diluted_usd": "58.12",
              "fee_per_day_in_range_usd": "65.98",
              "high_usd": "0.01096801",
              "in_range_share_pct": "5.82",
              "index": 2,
              "low_usd": "0.01002405",
              "note": null,
              "replay_edge_upper_usd": "38.41",
              "replay_inside_upper_usd": "25.00",
              "rung": "deep",
              "usdc": "500.00"
            }
          ],
          "reserve_margin_usdc": "167.45",
          "reserve_status": "holds",
          "total_usdc": "600.00"
        },
        {
          "avg_buy_price_usd": "0.01073461",
          "avg_kept_price_usd": "0.01048542",
          "budget_status": "fits",
          "gate_price_usd": "0.01002405",
          "near_low_usd": "0.01164620",
          "nock_at_deepest_bottom": "55893.9604",
          "nock_by_depth": [
            {
              "nock_held": "8208.6815",
              "pct_of_spot": "-10.03",
              "price_usd": "0.01164620"
            },
            {
              "nock_held": "55893.9604",
              "pct_of_spot": "-22.56",
              "price_usd": "0.01002405"
            }
          ],
          "nock_kept_if_deep_rungs_pulled": "47685.2789",
          "nock_needed": "0.0000",
          "per_rung": [
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "12.24",
              "fee_per_day_in_range_diluted_usd": "11.94",
              "fee_per_day_in_range_usd": "12.24",
              "high_usd": "0.01274292",
              "in_range_share_pct": "0.11",
              "index": 1,
              "low_usd": "0.01164620",
              "note": null,
              "replay_edge_upper_usd": "3.16",
              "replay_inside_upper_usd": "0.88",
              "rung": "near",
              "usdc": "100.00"
            },
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "13.20",
              "fee_per_day_in_range_diluted_usd": "58.12",
              "fee_per_day_in_range_usd": "65.98",
              "high_usd": "0.01096801",
              "in_range_share_pct": "5.82",
              "index": 2,
              "low_usd": "0.01002405",
              "note": null,
              "replay_edge_upper_usd": "38.41",
              "replay_inside_upper_usd": "25.00",
              "rung": "deep",
              "usdc": "500.00"
            }
          ],
          "reserve_margin_usdc": "167.45",
          "reserve_status": "holds",
          "total_usdc": "600.00"
        },
        {
          "avg_buy_price_usd": "0.01071084",
          "avg_kept_price_usd": "0.01048542",
          "budget_status": "fits",
          "gate_price_usd": "0.01002405",
          "near_low_usd": "0.01130202",
          "nock_at_deepest_bottom": "56018.0125",
          "nock_by_depth": [
            {
              "nock_held": "8332.7336",
              "pct_of_spot": "-12.69",
              "price_usd": "0.01130202"
            },
            {
              "nock_held": "56018.0125",
              "pct_of_spot": "-22.56",
              "price_usd": "0.01002405"
            }
          ],
          "nock_kept_if_deep_rungs_pulled": "47685.2789",
          "nock_needed": "0.0000",
          "per_rung": [
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "9.25",
              "fee_per_day_in_range_diluted_usd": "9.08",
              "fee_per_day_in_range_usd": "9.25",
              "high_usd": "0.01274292",
              "in_range_share_pct": "11.47",
              "index": 1,
              "low_usd": "0.01130202",
              "note": null,
              "replay_edge_upper_usd": "3.32",
              "replay_inside_upper_usd": "3.72",
              "rung": "near",
              "usdc": "100.00"
            },
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "13.20",
              "fee_per_day_in_range_diluted_usd": "58.12",
              "fee_per_day_in_range_usd": "65.98",
              "high_usd": "0.01096801",
              "in_range_share_pct": "5.82",
              "index": 2,
              "low_usd": "0.01002405",
              "note": null,
              "replay_edge_upper_usd": "38.41",
              "replay_inside_upper_usd": "25.00",
              "rung": "deep",
              "usdc": "500.00"
            }
          ],
          "reserve_margin_usdc": "167.45",
          "reserve_status": "holds",
          "total_usdc": "600.00"
        },
        {
          "avg_buy_price_usd": "0.01068682",
          "avg_kept_price_usd": "0.01048542",
          "budget_status": "fits",
          "gate_price_usd": "0.01002405",
          "near_low_usd": "0.01096801",
          "nock_at_deepest_bottom": "56143.9393",
          "nock_by_depth": [
            {
              "nock_held": "8458.6604",
              "pct_of_spot": "-15.27",
              "price_usd": "0.01096801"
            },
            {
              "nock_held": "56143.9393",
              "pct_of_spot": "-22.56",
              "price_usd": "0.01002405"
            }
          ],
          "nock_kept_if_deep_rungs_pulled": "47685.2789",
          "nock_needed": "0.0000",
          "per_rung": [
            {
              "band_usd_per_day": "3.07",
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "7.46",
              "fee_per_day_in_range_diluted_usd": "7.34",
              "fee_per_day_in_range_usd": "7.46",
              "high_usd": "0.01274292",
              "in_range_share_pct": "28.63",
              "index": 1,
              "low_usd": "0.01096801",
              "note": null,
              "replay_edge_upper_usd": "4.49",
              "replay_inside_upper_usd": "5.56",
              "rung": "near",
              "usdc": "100.00"
            },
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "13.20",
              "fee_per_day_in_range_diluted_usd": "58.12",
              "fee_per_day_in_range_usd": "65.98",
              "high_usd": "0.01096801",
              "in_range_share_pct": "5.82",
              "index": 2,
              "low_usd": "0.01002405",
              "note": null,
              "replay_edge_upper_usd": "38.41",
              "replay_inside_upper_usd": "25.00",
              "rung": "deep",
              "usdc": "500.00"
            }
          ],
          "reserve_margin_usdc": "167.45",
          "reserve_status": "holds",
          "total_usdc": "600.00"
        }
      ],
      "skipped_rows": [],
      "ticks_between": 5,
      "truncated": false
    },
    "near_depth_reason": null,
    "order": "Rows are in the order of the quantity varied and are not ranked.",
    "split": {
      "rows": [
        {
          "avg_buy_price_usd": "0.01048542",
          "avg_kept_price_usd": "0.01048542",
          "budget_status": "fits",
          "first_rung_share_pct": "0.00",
          "gate_price_usd": "0.01002405",
          "nock_at_deepest_bottom": "57222.3347",
          "nock_by_depth": [
            {
              "nock_held": "57222.3347",
              "pct_of_spot": "-22.56",
              "price_usd": "0.01002405"
            }
          ],
          "nock_kept_if_deep_rungs_pulled": "57222.3347",
          "nock_needed": "0.0000",
          "per_rung": [
            {
              "band_usd_per_day": null,
              "coverage_whole": null,
              "fee_per_100_usd_per_day_in_range_usd": null,
              "fee_per_day_in_range_diluted_usd": null,
              "fee_per_day_in_range_usd": null,
              "high_usd": "0.01274292",
              "in_range_share_pct": null,
              "index": 1,
              "low_usd": "0.01200086",
              "note": "no USDC in this rung in this row",
              "replay_edge_upper_usd": null,
              "replay_inside_upper_usd": null,
              "rung": "near",
              "usdc": "0.00"
            },
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "13.20",
              "fee_per_day_in_range_diluted_usd": "68.12",
              "fee_per_day_in_range_usd": "79.18",
              "high_usd": "0.01096801",
              "in_range_share_pct": "5.82",
              "index": 2,
              "low_usd": "0.01002405",
              "note": null,
              "replay_edge_upper_usd": "46.09",
              "replay_inside_upper_usd": "30.00",
              "rung": "deep",
              "usdc": "600.00"
            }
          ],
          "reserve_margin_usdc": "167.45",
          "reserve_status": "holds",
          "total_usdc": "600.00"
        },
        {
          "avg_buy_price_usd": "0.01064736",
          "avg_kept_price_usd": "0.01048542",
          "budget_status": "fits",
          "first_rung_share_pct": "10.00",
          "gate_price_usd": "0.01002405",
          "nock_at_deepest_bottom": "56351.9869",
          "nock_by_depth": [
            {
              "nock_held": "4851.8858",
              "pct_of_spot": "-7.29",
              "price_usd": "0.01200086"
            },
            {
              "nock_held": "56351.9869",
              "pct_of_spot": "-22.56",
              "price_usd": "0.01002405"
            }
          ],
          "nock_kept_if_deep_rungs_pulled": "51500.1012",
          "nock_needed": "0.0000",
          "per_rung": [
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "18.23",
              "fee_per_day_in_range_diluted_usd": "10.70",
              "fee_per_day_in_range_usd": "10.94",
              "high_usd": "0.01274292",
              "in_range_share_pct": "0.11",
              "index": 1,
              "low_usd": "0.01200086",
              "note": null,
              "replay_edge_upper_usd": "2.82",
              "replay_inside_upper_usd": "0.78",
              "rung": "near",
              "usdc": "60.00"
            },
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "13.20",
              "fee_per_day_in_range_diluted_usd": "62.17",
              "fee_per_day_in_range_usd": "71.26",
              "high_usd": "0.01096801",
              "in_range_share_pct": "5.82",
              "index": 2,
              "low_usd": "0.01002405",
              "note": null,
              "replay_edge_upper_usd": "41.48",
              "replay_inside_upper_usd": "27.00",
              "rung": "deep",
              "usdc": "540.00"
            }
          ],
          "reserve_margin_usdc": "167.45",
          "reserve_status": "holds",
          "total_usdc": "600.00"
        },
        {
          "avg_buy_price_usd": "0.01075813",
          "avg_kept_price_usd": "0.01048542",
          "budget_status": "fits",
          "first_rung_share_pct": "16.67",
          "gate_price_usd": "0.01002405",
          "nock_at_deepest_bottom": "55771.7551",
          "nock_by_depth": [
            {
              "nock_held": "8086.4763",
              "pct_of_spot": "-7.29",
              "price_usd": "0.01200086"
            },
            {
              "nock_held": "55771.7551",
              "pct_of_spot": "-22.56",
              "price_usd": "0.01002405"
            }
          ],
          "nock_kept_if_deep_rungs_pulled": "47685.2789",
          "nock_needed": "0.0000",
          "per_rung": [
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "18.23",
              "fee_per_day_in_range_diluted_usd": "17.57",
              "fee_per_day_in_range_usd": "18.23",
              "high_usd": "0.01274292",
              "in_range_share_pct": "0.11",
              "index": 1,
              "low_usd": "0.01200086",
              "note": null,
              "replay_edge_upper_usd": "4.70",
              "replay_inside_upper_usd": "1.31",
              "rung": "near",
              "usdc": "100.00"
            },
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "13.20",
              "fee_per_day_in_range_diluted_usd": "58.12",
              "fee_per_day_in_range_usd": "65.98",
              "high_usd": "0.01096801",
              "in_range_share_pct": "5.82",
              "index": 2,
              "low_usd": "0.01002405",
              "note": null,
              "replay_edge_upper_usd": "38.41",
              "replay_inside_upper_usd": "25.00",
              "rung": "deep",
              "usdc": "500.00"
            }
          ],
          "reserve_margin_usdc": "167.45",
          "reserve_status": "holds",
          "total_usdc": "600.00"
        },
        {
          "avg_buy_price_usd": "0.01081439",
          "avg_kept_price_usd": "0.01048542",
          "budget_status": "fits",
          "first_rung_share_pct": "20.00",
          "gate_price_usd": "0.01002405",
          "nock_at_deepest_bottom": "55481.6392",
          "nock_by_depth": [
            {
              "nock_held": "9703.7715",
              "pct_of_spot": "-7.29",
              "price_usd": "0.01200086"
            },
            {
              "nock_held": "55481.6392",
              "pct_of_spot": "-22.56",
              "price_usd": "0.01002405"
            }
          ],
          "nock_kept_if_deep_rungs_pulled": "45777.8677",
          "nock_needed": "0.0000",
          "per_rung": [
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "18.23",
              "fee_per_day_in_range_diluted_usd": "20.93",
              "fee_per_day_in_range_usd": "21.87",
              "high_usd": "0.01274292",
              "in_range_share_pct": "0.11",
              "index": 1,
              "low_usd": "0.01200086",
              "note": null,
              "replay_edge_upper_usd": "5.64",
              "replay_inside_upper_usd": "1.57",
              "rung": "near",
              "usdc": "120.00"
            },
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "13.20",
              "fee_per_day_in_range_diluted_usd": "56.06",
              "fee_per_day_in_range_usd": "63.34",
              "high_usd": "0.01096801",
              "in_range_share_pct": "5.82",
              "index": 2,
              "low_usd": "0.01002405",
              "note": null,
              "replay_edge_upper_usd": "36.87",
              "replay_inside_upper_usd": "24.00",
              "rung": "deep",
              "usdc": "480.00"
            }
          ],
          "reserve_margin_usdc": "167.45",
          "reserve_status": "holds",
          "total_usdc": "600.00"
        },
        {
          "avg_buy_price_usd": "0.01098674",
          "avg_kept_price_usd": "0.01048542",
          "budget_status": "fits",
          "first_rung_share_pct": "30.00",
          "gate_price_usd": "0.01002405",
          "nock_at_deepest_bottom": "54611.2915",
          "nock_by_depth": [
            {
              "nock_held": "14555.6573",
              "pct_of_spot": "-7.29",
              "price_usd": "0.01200086"
            },
            {
              "nock_held": "54611.2915",
              "pct_of_spot": "-22.56",
              "price_usd": "0.01002405"
            }
          ],
          "nock_kept_if_deep_rungs_pulled": "40055.6343",
          "nock_needed": "0.0000",
          "per_rung": [
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "18.23",
              "fee_per_day_in_range_diluted_usd": "30.74",
              "fee_per_day_in_range_usd": "32.81",
              "high_usd": "0.01274292",
              "in_range_share_pct": "0.11",
              "index": 1,
              "low_usd": "0.01200086",
              "note": null,
              "replay_edge_upper_usd": "8.47",
              "replay_inside_upper_usd": "2.35",
              "rung": "near",
              "usdc": "180.00"
            },
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "13.20",
              "fee_per_day_in_range_diluted_usd": "49.77",
              "fee_per_day_in_range_usd": "55.43",
              "high_usd": "0.01096801",
              "in_range_share_pct": "5.82",
              "index": 2,
              "low_usd": "0.01002405",
              "note": null,
              "replay_edge_upper_usd": "32.26",
              "replay_inside_upper_usd": "21.00",
              "rung": "deep",
              "usdc": "420.00"
            }
          ],
          "reserve_margin_usdc": "167.45",
          "reserve_status": "holds",
          "total_usdc": "600.00"
        },
        {
          "avg_buy_price_usd": "0.01116467",
          "avg_kept_price_usd": "0.01048542",
          "budget_status": "fits",
          "first_rung_share_pct": "40.00",
          "gate_price_usd": "0.01002405",
          "nock_at_deepest_bottom": "53740.9438",
          "nock_by_depth": [
            {
              "nock_held": "19407.5430",
              "pct_of_spot": "-7.29",
              "price_usd": "0.01200086"
            },
            {
              "nock_held": "53740.9438",
              "pct_of_spot": "-22.56",
              "price_usd": "0.01002405"
            }
          ],
          "nock_kept_if_deep_rungs_pulled": "34333.4008",
          "nock_needed": "0.0000",
          "per_rung": [
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "18.23",
              "fee_per_day_in_range_diluted_usd": "40.15",
              "fee_per_day_in_range_usd": "43.75",
              "high_usd": "0.01274292",
              "in_range_share_pct": "0.11",
              "index": 1,
              "low_usd": "0.01200086",
              "note": null,
              "replay_edge_upper_usd": "11.29",
              "replay_inside_upper_usd": "3.14",
              "rung": "near",
              "usdc": "240.00"
            },
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "13.20",
              "fee_per_day_in_range_diluted_usd": "43.29",
              "fee_per_day_in_range_usd": "47.51",
              "high_usd": "0.01096801",
              "in_range_share_pct": "5.82",
              "index": 2,
              "low_usd": "0.01002405",
              "note": null,
              "replay_edge_upper_usd": "27.65",
              "replay_inside_upper_usd": "18.00",
              "rung": "deep",
              "usdc": "360.00"
            }
          ],
          "reserve_margin_usdc": "167.45",
          "reserve_status": "holds",
          "total_usdc": "600.00"
        },
        {
          "avg_buy_price_usd": "0.01134846",
          "avg_kept_price_usd": "0.01048542",
          "budget_status": "fits",
          "first_rung_share_pct": "50.00",
          "gate_price_usd": "0.01002405",
          "nock_at_deepest_bottom": "52870.5961",
          "nock_by_depth": [
            {
              "nock_held": "24259.4288",
              "pct_of_spot": "-7.29",
              "price_usd": "0.01200086"
            },
            {
              "nock_held": "52870.5961",
              "pct_of_spot": "-22.56",
              "price_usd": "0.01002405"
            }
          ],
          "nock_kept_if_deep_rungs_pulled": "28611.1673",
          "nock_needed": "0.0000",
          "per_rung": [
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "18.23",
              "fee_per_day_in_range_diluted_usd": "49.17",
              "fee_per_day_in_range_usd": "54.69",
              "high_usd": "0.01274292",
              "in_range_share_pct": "0.11",
              "index": 1,
              "low_usd": "0.01200086",
              "note": null,
              "replay_edge_upper_usd": "14.11",
              "replay_inside_upper_usd": "3.92",
              "rung": "near",
              "usdc": "300.00"
            },
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "13.20",
              "fee_per_day_in_range_diluted_usd": "36.62",
              "fee_per_day_in_range_usd": "39.59",
              "high_usd": "0.01096801",
              "in_range_share_pct": "5.82",
              "index": 2,
              "low_usd": "0.01002405",
              "note": null,
              "replay_edge_upper_usd": "23.05",
              "replay_inside_upper_usd": "15.00",
              "rung": "deep",
              "usdc": "300.00"
            }
          ],
          "reserve_margin_usdc": "167.45",
          "reserve_status": "holds",
          "total_usdc": "600.00"
        },
        {
          "avg_buy_price_usd": "0.01153841",
          "avg_kept_price_usd": "0.01048542",
          "budget_status": "fits",
          "first_rung_share_pct": "60.00",
          "gate_price_usd": "0.01002405",
          "nock_at_deepest_bottom": "52000.2484",
          "nock_by_depth": [
            {
              "nock_held": "29111.3145",
              "pct_of_spot": "-7.29",
              "price_usd": "0.01200086"
            },
            {
              "nock_held": "52000.2484",
              "pct_of_spot": "-22.56",
              "price_usd": "0.01002405"
            }
          ],
          "nock_kept_if_deep_rungs_pulled": "22888.9339",
          "nock_needed": "0.0000",
          "per_rung": [
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "18.23",
              "fee_per_day_in_range_diluted_usd": "57.84",
              "fee_per_day_in_range_usd": "65.62",
              "high_usd": "0.01274292",
              "in_range_share_pct": "0.11",
              "index": 1,
              "low_usd": "0.01200086",
              "note": null,
              "replay_edge_upper_usd": "16.93",
              "replay_inside_upper_usd": "4.71",
              "rung": "near",
              "usdc": "360.00"
            },
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "13.20",
              "fee_per_day_in_range_diluted_usd": "29.74",
              "fee_per_day_in_range_usd": "31.67",
              "high_usd": "0.01096801",
              "in_range_share_pct": "5.82",
              "index": 2,
              "low_usd": "0.01002405",
              "note": null,
              "replay_edge_upper_usd": "18.44",
              "replay_inside_upper_usd": "12.00",
              "rung": "deep",
              "usdc": "240.00"
            }
          ],
          "reserve_margin_usdc": "167.45",
          "reserve_status": "holds",
          "total_usdc": "600.00"
        },
        {
          "avg_buy_price_usd": "0.01173482",
          "avg_kept_price_usd": "0.01048542",
          "budget_status": "fits",
          "first_rung_share_pct": "70.00",
          "gate_price_usd": "0.01002405",
          "nock_at_deepest_bottom": "51129.9007",
          "nock_by_depth": [
            {
              "nock_held": "33963.2003",
              "pct_of_spot": "-7.29",
              "price_usd": "0.01200086"
            },
            {
              "nock_held": "51129.9007",
              "pct_of_spot": "-22.56",
              "price_usd": "0.01002405"
            }
          ],
          "nock_kept_if_deep_rungs_pulled": "17166.7004",
          "nock_needed": "0.0000",
          "per_rung": [
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "18.23",
              "fee_per_day_in_range_diluted_usd": "66.17",
              "fee_per_day_in_range_usd": "76.56",
              "high_usd": "0.01274292",
              "in_range_share_pct": "0.11",
              "index": 1,
              "low_usd": "0.01200086",
              "note": null,
              "replay_edge_upper_usd": "19.75",
              "replay_inside_upper_usd": "5.49",
              "rung": "near",
              "usdc": "420.00"
            },
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "13.20",
              "fee_per_day_in_range_diluted_usd": "22.65",
              "fee_per_day_in_range_usd": "23.75",
              "high_usd": "0.01096801",
              "in_range_share_pct": "5.82",
              "index": 2,
              "low_usd": "0.01002405",
              "note": null,
              "replay_edge_upper_usd": "13.83",
              "replay_inside_upper_usd": "9.00",
              "rung": "deep",
              "usdc": "180.00"
            }
          ],
          "reserve_margin_usdc": "167.45",
          "reserve_status": "holds",
          "total_usdc": "600.00"
        },
        {
          "avg_buy_price_usd": "0.01193803",
          "avg_kept_price_usd": "0.01048542",
          "budget_status": "fits",
          "first_rung_share_pct": "80.00",
          "gate_price_usd": "0.01002405",
          "nock_at_deepest_bottom": "50259.5530",
          "nock_by_depth": [
            {
              "nock_held": "38815.0860",
              "pct_of_spot": "-7.29",
              "price_usd": "0.01200086"
            },
            {
              "nock_held": "50259.5530",
              "pct_of_spot": "-22.56",
              "price_usd": "0.01002405"
            }
          ],
          "nock_kept_if_deep_rungs_pulled": "11444.4669",
          "nock_needed": "0.0000",
          "per_rung": [
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "18.23",
              "fee_per_day_in_range_diluted_usd": "74.18",
              "fee_per_day_in_range_usd": "87.50",
              "high_usd": "0.01274292",
              "in_range_share_pct": "0.11",
              "index": 1,
              "low_usd": "0.01200086",
              "note": null,
              "replay_edge_upper_usd": "22.58",
              "replay_inside_upper_usd": "6.28",
              "rung": "near",
              "usdc": "480.00"
            },
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "13.20",
              "fee_per_day_in_range_diluted_usd": "15.34",
              "fee_per_day_in_range_usd": "15.84",
              "high_usd": "0.01096801",
              "in_range_share_pct": "5.82",
              "index": 2,
              "low_usd": "0.01002405",
              "note": null,
              "replay_edge_upper_usd": "9.22",
              "replay_inside_upper_usd": "6.00",
              "rung": "deep",
              "usdc": "120.00"
            }
          ],
          "reserve_margin_usdc": "167.45",
          "reserve_status": "holds",
          "total_usdc": "600.00"
        },
        {
          "avg_buy_price_usd": "0.01214840",
          "avg_kept_price_usd": "0.01048542",
          "budget_status": "fits",
          "first_rung_share_pct": "90.00",
          "gate_price_usd": "0.01002405",
          "nock_at_deepest_bottom": "49389.2053",
          "nock_by_depth": [
            {
              "nock_held": "43666.9718",
              "pct_of_spot": "-7.29",
              "price_usd": "0.01200086"
            },
            {
              "nock_held": "49389.2053",
              "pct_of_spot": "-22.56",
              "price_usd": "0.01002405"
            }
          ],
          "nock_kept_if_deep_rungs_pulled": "5722.2335",
          "nock_needed": "0.0000",
          "per_rung": [
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "18.23",
              "fee_per_day_in_range_diluted_usd": "81.90",
              "fee_per_day_in_range_usd": "98.43",
              "high_usd": "0.01274292",
              "in_range_share_pct": "0.11",
              "index": 1,
              "low_usd": "0.01200086",
              "note": null,
              "replay_edge_upper_usd": "25.40",
              "replay_inside_upper_usd": "7.06",
              "rung": "near",
              "usdc": "540.00"
            },
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "13.20",
              "fee_per_day_in_range_diluted_usd": "7.79",
              "fee_per_day_in_range_usd": "7.92",
              "high_usd": "0.01096801",
              "in_range_share_pct": "5.82",
              "index": 2,
              "low_usd": "0.01002405",
              "note": null,
              "replay_edge_upper_usd": "4.61",
              "replay_inside_upper_usd": "3.00",
              "rung": "deep",
              "usdc": "60.00"
            }
          ],
          "reserve_margin_usdc": "167.45",
          "reserve_status": "holds",
          "total_usdc": "600.00"
        },
        {
          "avg_buy_price_usd": "0.01236633",
          "avg_kept_price_usd": null,
          "budget_status": "fits",
          "first_rung_share_pct": "100.00",
          "gate_price_usd": "0.01002405",
          "nock_at_deepest_bottom": "48518.8575",
          "nock_by_depth": [
            {
              "nock_held": "48518.8575",
              "pct_of_spot": "-7.29",
              "price_usd": "0.01200086"
            }
          ],
          "nock_kept_if_deep_rungs_pulled": "0.0000",
          "nock_needed": "0.0000",
          "per_rung": [
            {
              "band_usd_per_day": null,
              "coverage_whole": true,
              "fee_per_100_usd_per_day_in_range_usd": "18.23",
              "fee_per_day_in_range_diluted_usd": "89.33",
              "fee_per_day_in_range_usd": "109.37",
              "high_usd": "0.01274292",
              "in_range_share_pct": "0.11",
              "index": 1,
              "low_usd": "0.01200086",
              "note": null,
              "replay_edge_upper_usd": "28.22",
              "replay_inside_upper_usd": "7.85",
              "rung": "near",
              "usdc": "600.00"
            },
            {
              "band_usd_per_day": null,
              "coverage_whole": null,
              "fee_per_100_usd_per_day_in_range_usd": null,
              "fee_per_day_in_range_diluted_usd": null,
              "fee_per_day_in_range_usd": null,
              "high_usd": "0.01096801",
              "in_range_share_pct": null,
              "index": 2,
              "low_usd": "0.01002405",
              "note": "no USDC in this rung in this row",
              "replay_edge_upper_usd": null,
              "replay_inside_upper_usd": null,
              "rung": "deep",
              "usdc": "0.00"
            }
          ],
          "reserve_margin_usdc": "167.45",
          "reserve_status": "holds",
          "total_usdc": "600.00"
        }
      ],
      "step_note": "display constants, not settings",
      "step_pct": 10.0,
      "varies": "the share of the $600.00 total in r1; r2 holds the rest"
    },
    "split_reason": null,
    "statement": "Rows are measurements at your depths; NockNock does not choose the split or the depths."
  },
  "tradeoff_reason": null
};

export const THE_BOTTLE_CONFIGURED: LadderSimulation = {
  "assumptions": [
    "Every NOCK pool moves by the same percentage as spot, each from its own current price; row 0 of the grid is the snapshot.",
    "Fees and gas are not counted.",
    "WETH/NOCK ranges hold ETH/USD constant and add no USDC.",
    "Gas-side ranges with no NOCK leg (ETH/USDC, #151) are left out of the stable balance and the NOCK totals.",
    "The ladder's USDC comes from the wallet and its NOCK from wallet NOCK.",
    "Each rung is its own position (its own NFT) in the same pool; overlapping rungs are allowed and their liquidity adds.",
    "The rungs share one reserve gate and one NOCK budget.",
    "A deep rung keeps its NOCK on a rebound only if it is pulled when it is fully NOCK; pulling is a withdrawal you sign, and its gas is not counted."
  ],
  "bottom": {
    "avg_buy_price_usd": "0.01066922",
    "describes": "the sum over the rungs at the deepest rung's bottom",
    "nock_bought": "71888.1054",
    "nock_held": "71974.8945",
    "pct_of_spot": "-22.56",
    "price_usd": "0.01002405"
  },
  "bounds": {
    "describes": "the span from the deepest rung's bottom to the highest rung's top",
    "high_pct": "+1.44",
    "high_rule": "per rung: rungs[].bounds.high_rule",
    "high_usd": "0.01313098",
    "low_pct": "-22.56",
    "low_usd": "0.01002405",
    "tick_lower": null,
    "tick_upper": null
  },
  "combined": {
    "avg_buy_price_usd": "0.01066922",
    "avg_kept_price_usd": "0.01048542",
    "deep_rungs": 1,
    "deepest_bottom_usd": "0.01002405",
    "gate_price_rule": "your ladder bottom (operator_profile.ladder_bottom_usd $0.01000000), below the deepest rung's bottom",
    "gate_price_usd": "0.01000000",
    "max_rungs": 4,
    "max_rungs_note": "display constants, not settings",
    "max_usdc_total": "766.99",
    "nock_bought_at_deepest_bottom": "71888.1054",
    "nock_bought_by_deep_rungs": "62076.6389",
    "nock_held_at_deepest_bottom": "71974.8945",
    "nock_kept_if_deep_rungs_pulled": "62076.6389",
    "overlaps": [],
    "rung_count": 3,
    "total_nock_needed": "86.7891",
    "total_usdc": "766.99"
  },
  "fee_estimate": null,
  "fee_estimate_reason": "per rung: rungs[].fee_estimate; each grid row sums the rungs in range there",
  "floor_check": {
    "budget_note": null,
    "floor_breach_effect": {
      "after": {
        "crossing": {
          "level_nock": "100000",
          "multiple": "1.9484",
          "pct_above_spot": "94.8",
          "price_usd": "0.025220",
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
      "reason": "the ladder's NOCK leaves the wallet and sells as NOCK rises to the top",
      "status": "lowers"
    },
    "lp_nock_budget": "86.7891",
    "max_usdc_within_budget": null,
    "nock_needed": "86.7891",
    "nock_needed_by_rung": [
      {
        "index": 1,
        "nock_needed": "86.7891"
      },
      {
        "index": 2,
        "nock_needed": "0.0000"
      },
      {
        "index": 3,
        "nock_needed": "0.0000"
      }
    ],
    "reason": null,
    "rebuild_note": "the rebuild's suggestions this cycle also draw 0 NOCK of the same budget: an alternative use of it, not subtracted",
    "rebuild_suggested_nock": "0.0000",
    "status": "fits"
  },
  "grid": [
    {
      "est_fee_reason": null,
      "est_fee_usd_per_day": "0.84",
      "est_fee_usd_per_day_diluted": "0.84",
      "existing_nock_lp_nock": "19103.7111",
      "existing_nock_lp_usdc": "226.83",
      "ladder_nock": "0.0000",
      "ladder_nock_acquired": "-86.7891",
      "ladder_usdc": "768.12",
      "markers": [
        "r1 top"
      ],
      "pct_of_spot": "+1.44",
      "pool_price_usd": "0.01313098",
      "price_usd": "0.01313098",
      "reserve_headroom_usdc": "823.80",
      "rungs": [
        {
          "in_range": true,
          "index": 1,
          "ladder_nock": "0.0000",
          "ladder_usdc": "2.36"
        },
        {
          "in_range": false,
          "index": 2,
          "ladder_nock": "0.0000",
          "ladder_usdc": "114.86"
        },
        {
          "in_range": false,
          "index": 3,
          "ladder_nock": "0.0000",
          "ladder_usdc": "650.90"
        }
      ],
      "rungs_in_range": [
        1
      ],
      "stable_usdc": "1773.80",
      "total_tracked_nock": "109217.47"
    },
    {
      "est_fee_reason": null,
      "est_fee_usd_per_day": "0.84",
      "est_fee_usd_per_day_diluted": "0.84",
      "existing_nock_lp_nock": "19341.9535",
      "existing_nock_lp_usdc": "223.71",
      "ladder_nock": "86.7891",
      "ladder_nock_acquired": "0.0000",
      "ladder_usdc": "766.99",
      "markers": [
        "spot"
      ],
      "pct_of_spot": "0.00",
      "pool_price_usd": "0.01294400",
      "price_usd": "0.01294400",
      "reserve_headroom_usdc": "819.54",
      "rungs": [
        {
          "in_range": true,
          "index": 1,
          "ladder_nock": "86.7891",
          "ladder_usdc": "1.23"
        },
        {
          "in_range": false,
          "index": 2,
          "ladder_nock": "0.0000",
          "ladder_usdc": "114.86"
        },
        {
          "in_range": false,
          "index": 3,
          "ladder_nock": "0.0000",
          "ladder_usdc": "650.90"
        }
      ],
      "rungs_in_range": [
        1
      ],
      "stable_usdc": "1769.54",
      "total_tracked_nock": "109542.50"
    },
    {
      "est_fee_reason": null,
      "est_fee_usd_per_day": "8.56",
      "est_fee_usd_per_day_diluted": "8.42",
      "existing_nock_lp_nock": "19603.9982",
      "existing_nock_lp_usdc": "220.32",
      "ladder_nock": "182.2491",
      "ladder_nock_acquired": "95.4600",
      "ladder_usdc": "765.76",
      "markers": [
        "r1 bottom",
        "r2 top"
      ],
      "pct_of_spot": "-1.55",
      "pool_price_usd": "0.01274292",
      "price_usd": "0.01274292",
      "reserve_headroom_usdc": "814.93",
      "rungs": [
        {
          "in_range": false,
          "index": 1,
          "ladder_nock": "182.2491",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": true,
          "index": 2,
          "ladder_nock": "0.0000",
          "ladder_usdc": "114.86"
        },
        {
          "in_range": false,
          "index": 3,
          "ladder_nock": "0.0000",
          "ladder_usdc": "650.90"
        }
      ],
      "rungs_in_range": [
        2
      ],
      "stable_usdc": "1764.93",
      "total_tracked_nock": "109900.01"
    },
    {
      "est_fee_reason": null,
      "est_fee_usd_per_day": "8.56",
      "est_fee_usd_per_day_diluted": "8.42",
      "existing_nock_lp_nock": "20208.1461",
      "existing_nock_lp_usdc": "212.71",
      "ladder_nock": "2425.1075",
      "ladder_nock_acquired": "2338.3184",
      "ladder_usdc": "737.69",
      "markers": [],
      "pct_of_spot": "-5.00",
      "pool_price_usd": "0.01229680",
      "price_usd": "0.01229680",
      "reserve_headroom_usdc": "779.25",
      "rungs": [
        {
          "in_range": false,
          "index": 1,
          "ladder_nock": "182.2491",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": true,
          "index": 2,
          "ladder_nock": "2242.8584",
          "ladder_usdc": "86.79"
        },
        {
          "in_range": false,
          "index": 3,
          "ladder_nock": "0.0000",
          "ladder_usdc": "650.90"
        }
      ],
      "rungs_in_range": [
        2
      ],
      "stable_usdc": "1729.25",
      "total_tracked_nock": "112747.01"
    },
    {
      "est_fee_reason": null,
      "est_fee_usd_per_day": "8.56",
      "est_fee_usd_per_day_diluted": "8.42",
      "existing_nock_lp_nock": "21145.5540",
      "existing_nock_lp_usdc": "201.43",
      "ladder_nock": "5905.1711",
      "ladder_nock_acquired": "5818.3820",
      "ladder_usdc": "696.04",
      "markers": [],
      "pct_of_spot": "-10.00",
      "pool_price_usd": "0.01164960",
      "price_usd": "0.01164960",
      "reserve_headroom_usdc": "726.31",
      "rungs": [
        {
          "in_range": false,
          "index": 1,
          "ladder_nock": "182.2491",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": true,
          "index": 2,
          "ladder_nock": "5722.9220",
          "ladder_usdc": "45.14"
        },
        {
          "in_range": false,
          "index": 3,
          "ladder_nock": "0.0000",
          "ladder_usdc": "650.90"
        }
      ],
      "rungs_in_range": [
        2
      ],
      "stable_usdc": "1676.31",
      "total_tracked_nock": "117164.48"
    },
    {
      "est_fee_reason": null,
      "est_fee_usd_per_day": "8.56",
      "est_fee_usd_per_day_diluted": "8.42",
      "existing_nock_lp_nock": "22164.5030",
      "existing_nock_lp_usdc": "189.83",
      "ladder_nock": "9687.9505",
      "ladder_nock_acquired": "9601.1614",
      "ladder_usdc": "653.21",
      "markers": [],
      "pct_of_spot": "-15.00",
      "pool_price_usd": "0.01100240",
      "price_usd": "0.01100240",
      "reserve_headroom_usdc": "671.88",
      "rungs": [
        {
          "in_range": false,
          "index": 1,
          "ladder_nock": "182.2491",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": true,
          "index": 2,
          "ladder_nock": "9505.7014",
          "ladder_usdc": "2.31"
        },
        {
          "in_range": false,
          "index": 3,
          "ladder_nock": "0.0000",
          "ladder_usdc": "650.90"
        }
      ],
      "rungs_in_range": [
        2
      ],
      "stable_usdc": "1621.88",
      "total_tracked_nock": "121966.21"
    },
    {
      "est_fee_reason": null,
      "est_fee_usd_per_day": "85.90",
      "est_fee_usd_per_day_diluted": "73.03",
      "existing_nock_lp_nock": "22221.1519",
      "existing_nock_lp_usdc": "189.20",
      "ladder_nock": "9898.2556",
      "ladder_nock_acquired": "9811.4665",
      "ladder_usdc": "650.90",
      "markers": [
        "r2 bottom",
        "r3 top"
      ],
      "pct_of_spot": "-15.27",
      "pool_price_usd": "0.01096801",
      "price_usd": "0.01096801",
      "reserve_headroom_usdc": "668.94",
      "rungs": [
        {
          "in_range": false,
          "index": 1,
          "ladder_nock": "182.2491",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": false,
          "index": 2,
          "ladder_nock": "9716.0065",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": true,
          "index": 3,
          "ladder_nock": "0.0000",
          "ladder_usdc": "650.90"
        }
      ],
      "rungs_in_range": [
        3
      ],
      "stable_usdc": "1618.94",
      "total_tracked_nock": "122233.17"
    },
    {
      "est_fee_reason": null,
      "est_fee_usd_per_day": "85.90",
      "est_fee_usd_per_day_diluted": "73.03",
      "existing_nock_lp_nock": "23277.5444",
      "existing_nock_lp_usdc": "177.88",
      "ladder_nock": "49233.4946",
      "ladder_nock_acquired": "49146.7055",
      "ladder_usdc": "231.70",
      "markers": [],
      "pct_of_spot": "-20.00",
      "pool_price_usd": "0.01035520",
      "price_usd": "0.01035520",
      "reserve_headroom_usdc": "238.42",
      "rungs": [
        {
          "in_range": false,
          "index": 1,
          "ladder_nock": "182.2491",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": false,
          "index": 2,
          "ladder_nock": "9716.0065",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": true,
          "index": 3,
          "ladder_nock": "39335.2390",
          "ladder_usdc": "231.70"
        }
      ],
      "rungs_in_range": [
        3
      ],
      "stable_usdc": "1188.42",
      "total_tracked_nock": "162624.80"
    },
    {
      "est_fee_reason": "no rung is in range at this price, so none earns fees",
      "est_fee_usd_per_day": "0.00",
      "est_fee_usd_per_day_diluted": "0.00",
      "existing_nock_lp_nock": "23888.2905",
      "existing_nock_lp_usdc": "171.62",
      "ladder_nock": "71974.8945",
      "ladder_nock_acquired": "71888.1054",
      "ladder_usdc": "0.00",
      "markers": [
        "r3 bottom"
      ],
      "pct_of_spot": "-22.56",
      "pool_price_usd": "0.01002405",
      "price_usd": "0.01002405",
      "reserve_headroom_usdc": "0.46",
      "rungs": [
        {
          "in_range": false,
          "index": 1,
          "ladder_nock": "182.2491",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": false,
          "index": 2,
          "ladder_nock": "9716.0065",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": false,
          "index": 3,
          "ladder_nock": "62076.6389",
          "ladder_usdc": "0.00"
        }
      ],
      "rungs_in_range": [],
      "stable_usdc": "950.46",
      "total_tracked_nock": "185976.94"
    },
    {
      "est_fee_reason": "no rung is in range at this price, so none earns fees",
      "est_fee_usd_per_day": "0.00",
      "est_fee_usd_per_day_diluted": "0.00",
      "existing_nock_lp_nock": "23933.8327",
      "existing_nock_lp_usdc": "171.16",
      "ladder_nock": "71974.8945",
      "ladder_nock_acquired": "71888.1054",
      "ladder_usdc": "0.00",
      "markers": [
        "gate"
      ],
      "pct_of_spot": "-22.74",
      "pool_price_usd": "0.01000000",
      "price_usd": "0.01000000",
      "reserve_headroom_usdc": "0.00",
      "rungs": [
        {
          "in_range": false,
          "index": 1,
          "ladder_nock": "182.2491",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": false,
          "index": 2,
          "ladder_nock": "9716.0065",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": false,
          "index": 3,
          "ladder_nock": "62076.6389",
          "ladder_usdc": "0.00"
        }
      ],
      "rungs_in_range": [],
      "stable_usdc": "950.00",
      "total_tracked_nock": "186022.49"
    },
    {
      "est_fee_reason": "no rung is in range at this price, so none earns fees",
      "est_fee_usd_per_day": "0.00",
      "est_fee_usd_per_day_diluted": "0.00",
      "existing_nock_lp_nock": "24500.1121",
      "existing_nock_lp_usdc": "165.55",
      "ladder_nock": "71974.8945",
      "ladder_nock_acquired": "71888.1054",
      "ladder_usdc": "0.00",
      "markers": [],
      "pct_of_spot": "-25.00",
      "pool_price_usd": "0.00970800",
      "price_usd": "0.00970800",
      "reserve_headroom_usdc": "-5.61",
      "rungs": [
        {
          "in_range": false,
          "index": 1,
          "ladder_nock": "182.2491",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": false,
          "index": 2,
          "ladder_nock": "9716.0065",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": false,
          "index": 3,
          "ladder_nock": "62076.6389",
          "ladder_usdc": "0.00"
        }
      ],
      "rungs_in_range": [],
      "stable_usdc": "944.39",
      "total_tracked_nock": "186588.77"
    },
    {
      "est_fee_reason": "no rung is in range at this price, so none earns fees",
      "est_fee_usd_per_day": "0.00",
      "est_fee_usd_per_day_diluted": "0.00",
      "existing_nock_lp_nock": "25851.4339",
      "existing_nock_lp_usdc": "152.80",
      "ladder_nock": "71974.8945",
      "ladder_nock_acquired": "71888.1054",
      "ladder_usdc": "0.00",
      "markers": [],
      "pct_of_spot": "-30.00",
      "pool_price_usd": "0.00906080",
      "price_usd": "0.00906080",
      "reserve_headroom_usdc": "-18.36",
      "rungs": [
        {
          "in_range": false,
          "index": 1,
          "ladder_nock": "182.2491",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": false,
          "index": 2,
          "ladder_nock": "9716.0065",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": false,
          "index": 3,
          "ladder_nock": "62076.6389",
          "ladder_usdc": "0.00"
        }
      ],
      "rungs_in_range": [],
      "stable_usdc": "931.64",
      "total_tracked_nock": "187940.09"
    },
    {
      "est_fee_reason": "no rung is in range at this price, so none earns fees",
      "est_fee_usd_per_day": "0.00",
      "est_fee_usd_per_day_diluted": "0.00",
      "existing_nock_lp_nock": "27355.8217",
      "existing_nock_lp_usdc": "139.58",
      "ladder_nock": "71974.8945",
      "ladder_nock_acquired": "71888.1054",
      "ladder_usdc": "0.00",
      "markers": [],
      "pct_of_spot": "-35.00",
      "pool_price_usd": "0.00841360",
      "price_usd": "0.00841360",
      "reserve_headroom_usdc": "-31.57",
      "rungs": [
        {
          "in_range": false,
          "index": 1,
          "ladder_nock": "182.2491",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": false,
          "index": 2,
          "ladder_nock": "9716.0065",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": false,
          "index": 3,
          "ladder_nock": "62076.6389",
          "ladder_usdc": "0.00"
        }
      ],
      "rungs_in_range": [],
      "stable_usdc": "918.43",
      "total_tracked_nock": "189444.48"
    },
    {
      "est_fee_reason": "no rung is in range at this price, so none earns fees",
      "est_fee_usd_per_day": "0.00",
      "est_fee_usd_per_day_diluted": "0.00",
      "existing_nock_lp_nock": "29044.5410",
      "existing_nock_lp_usdc": "125.85",
      "ladder_nock": "71974.8945",
      "ladder_nock_acquired": "71888.1054",
      "ladder_usdc": "0.00",
      "markers": [],
      "pct_of_spot": "-40.00",
      "pool_price_usd": "0.00776640",
      "price_usd": "0.00776640",
      "reserve_headroom_usdc": "-45.30",
      "rungs": [
        {
          "in_range": false,
          "index": 1,
          "ladder_nock": "182.2491",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": false,
          "index": 2,
          "ladder_nock": "9716.0065",
          "ladder_usdc": "0.00"
        },
        {
          "in_range": false,
          "index": 3,
          "ladder_nock": "62076.6389",
          "ladder_usdc": "0.00"
        }
      ],
      "rungs_in_range": [],
      "stable_usdc": "904.70",
      "total_tracked_nock": "191133.19"
    }
  ],
  "inputs": {
    "grid_step_pct": 5.0,
    "grid_to_pct": -40.0,
    "high_requested": null,
    "low_requested": null,
    "nock": null,
    "position_id": "uniswap-v4:base:3163450",
    "rungs": [
      {
        "high_requested": "0.013130977294699456",
        "index": 1,
        "low_requested": "0.012742917379855624",
        "nock": null,
        "rung": "cap",
        "usdc": "1.23"
      },
      {
        "high_requested": "0.012742917379855624",
        "index": 2,
        "low_requested": "0.010968012886072376",
        "nock": null,
        "rung": "near",
        "usdc": "114.86"
      },
      {
        "high_requested": "0.010968012886072376",
        "index": 3,
        "low_requested": "0.0100240541221736",
        "nock": null,
        "rung": "deep",
        "usdc": "650.90"
      }
    ],
    "shape": {
      "cap": "sized",
      "cap_reason": null,
      "dust": {
        "liquidity_tolerance": 1e-06,
        "note": "display constants, not settings",
        "usdc": 1.0
      },
      "fit": {
        "allowed_rule": "the shared reserve gate's most at $0.01000000: the wallet's USDC plus the USDC every range holds there, less the $950.00 reserve, and never more than the wallet holds",
        "allowed_usdc": "766.99",
        "factor": null,
        "minimum_usdc": "1.00",
        "needed_usdc": "766.99",
        "rule": "once a near or deep rung is deposited the cap takes only what the reserve leaves beside the near and deep rungs still to deposit; when they alone need more than the reserve allows, each deposit left is cut pro rata (by the same fraction), so the shape keeps its proportions, and the shortfall is stated",
        "shortfall_usdc": null,
        "status": "fits"
      },
      "gate_price_rule": "your ladder bottom (operator_profile.ladder_bottom_usd $0.01000000), below the deepest rung's bottom",
      "gate_price_usd": "0.01000000",
      "headroom_rule": "the shared reserve gate's most at $0.01000000: the wallet's USDC plus the USDC every range holds there, less the $950.00 reserve, and never more than the wallet holds",
      "headroom_usdc": "766.99",
      "note": "the operator's own shape, sized here; NockNock does not choose it",
      "plan": [
        {
          "cap": {
            "band_rule": "the band 273600..273900 of the 300-tick grid contains the pool's tick 273743",
            "bound_by": "lp_nock_budget",
            "limit_rule": "never more USDC than the first near rung's target; once a near or deep rung is deposited, never more than the reserve leaves for it beside the other deposits listed",
            "lp_nock_budget": "86.7891",
            "nock": "86.7891",
            "note": "the cap holds the whole LP NOCK budget, 86.7891 NOCK; the curve pairs it with $1.23 USDC",
            "pool_tick": 273743,
            "tick_lower": 273600,
            "tick_upper": 273900,
            "usdc": "1.23",
            "usdc_limit": "100.04",
            "usdc_limit_is": "near_share"
          },
          "counterparts": [],
          "deployed_nock": "0.0000",
          "deployed_usdc": "0.000000",
          "fit_factor": null,
          "found": [],
          "index": 1,
          "note": null,
          "over_by_usdc": null,
          "reason": null,
          "remaining_liquidity": "138192115820500",
          "remaining_nock": "86.7891",
          "remaining_usdc": "1.225999",
          "remaining_usdc_at_top": null,
          "rung": "cap",
          "share": null,
          "status": "to_deposit",
          "target_liquidity": "138192115820500",
          "target_nock": "86.7891",
          "target_usdc": "1.225999",
          "tick_lower": 273600,
          "tick_upper": 273900,
          "tracked_liquidity": "0"
        },
        {
          "cap": null,
          "counterparts": [],
          "deployed_nock": null,
          "deployed_usdc": "0.000000",
          "fit_factor": null,
          "found": [],
          "index": 2,
          "note": null,
          "over_by_usdc": null,
          "reason": null,
          "remaining_liquidity": "1408301616732632",
          "remaining_nock": "0.0000",
          "remaining_usdc": "114.864600",
          "remaining_usdc_at_top": "114.864600",
          "rung": "near",
          "share": "0.15",
          "status": "to_deposit",
          "target_liquidity": "1408301616732632",
          "target_nock": null,
          "target_usdc": "114.864600",
          "tick_lower": 273900,
          "tick_upper": 275400,
          "tracked_liquidity": "0"
        },
        {
          "cap": null,
          "counterparts": [],
          "deployed_nock": null,
          "deployed_usdc": "0.000000",
          "fit_factor": null,
          "found": [],
          "index": 3,
          "note": null,
          "over_by_usdc": null,
          "reason": null,
          "remaining_liquidity": "14125167085436768",
          "remaining_nock": "0.0000",
          "remaining_usdc": "650.899401",
          "remaining_usdc_at_top": "650.899401",
          "rung": "deep",
          "share": "0.85",
          "status": "to_deposit",
          "target_liquidity": "14125167085436768",
          "target_nock": null,
          "target_usdc": "650.899401",
          "tick_lower": 275400,
          "tick_upper": 276300,
          "tracked_liquidity": "0"
        }
      ],
      "rest_note": "the near and deep shares divide this USDC, measured at each rung's top",
      "rest_rule": "what the reserve gate allows now, less the cap's USDC: no near or deep rung is deposited yet, so this follows the gate until one is",
      "rest_usdc": "765.76",
      "rungs": [
        {
          "high": "band",
          "low_usd": null,
          "rung": "cap",
          "share": null
        },
        {
          "high": "spot",
          "low_usd": "0.01100000",
          "rung": "near",
          "share": "0.15"
        },
        {
          "high": "0.01100000",
          "low_usd": "0.01000000",
          "rung": "deep",
          "share": "0.85"
        }
      ],
      "source": "operator_profile.ladder_rungs",
      "status": "computed",
      "total_rule": "the near and deep rungs' USDC at their tops plus the cap's USDC now",
      "total_usdc": "766.99",
      "tracked_note": "Each configured rung's target is a liquidity on its own ticks; a liquidity does not change with the price, so a deployed rung stays deployed when the price moves, in range or out of it. Tracked ladder rungs in this pool (role: ladder) are matched to configured rungs by role, then by ticks (exact ticks first, otherwise the configured rung of that role in order); a position found on-chain and not yet tracked counts for the rung whose pool and ticks it matches exactly. A near or deep rung is measured by what it holds as all USDC at its top (liquidity times the USDC one unit holds there). The plan is frozen by the first near or deep rung deposited: the shares divide what that rung's USDC at its top implies (the largest, when several are deposited), never the total again. Until then the shares divide what the reserve gate allows now, less the cap. A tracked cap is the cap: its liquidity is its target. What is left to deposit is drawn on a matched rung's own ticks.",
      "tracked_usdc": "0.00",
      "unmatched": []
    },
    "split_step_note": "display constants, not settings",
    "split_step_pct": 10.0,
    "tick_spacing": null,
    "usdc": "766.99"
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
      "headroom_usdc": "823.80",
      "ladder_usdc": "768.12",
      "note": "Out of range above (read at the highest top), every rung holds only USDC: its NOCK has sold on the way up.",
      "out_of_range_above_now": false,
      "price_usd": "0.01313098",
      "stable_usdc": "1773.80"
    },
    "cash_reserve_usdc": "950.00",
    "existing_ranges_below_bottom": {
      "headroom_usdc": "-45.30",
      "note": "The existing NOCK ranges keep converting below the ladder's bottom: a measurement, not the gate.",
      "pct_of_spot": "-40.00",
      "price_usd": "0.00776640",
      "stable_usdc": "904.70"
    },
    "existing_ranges_below_bottom_reason": null,
    "existing_ranges_usdc_at_bottom": "171.16",
    "gate": "The gate is the stable balance where every rung is fully NOCK: at the lower of the deepest rung's bottom and your ladder bottom (operator_profile.ladder_bottom_usd) when it is set, otherwise at the deepest rung's bottom; the *_at_bottom figures are read at gate_price_usd.",
    "gate_price_rule": "your ladder bottom (operator_profile.ladder_bottom_usd $0.01000000), below the deepest rung's bottom",
    "gate_price_usd": "0.01000000",
    "headroom_at_spot_usdc": "819.54",
    "margin_usdc": "0.00",
    "max_usdc_for_this_range": "766.99",
    "reason": null,
    "stable_at_bottom_usdc": "950.00",
    "stable_at_spot_usdc": "1769.54",
    "status": "holds",
    "tax_reserve_usd": {
      "is": "stated, not a gate",
      "usd": "0.00"
    }
  },
  "rungs": [
    {
      "bottom": {
        "avg_buy_price_usd": "0.01284307",
        "nock_bought": "95.4600",
        "nock_held": "182.2491",
        "pct_of_spot": "-1.55",
        "price_usd": "0.01274292"
      },
      "bounds": {
        "high_pct": "+1.44",
        "high_rule": "the tick band containing the pool's tick (the cap)",
        "high_usd": "0.01313098",
        "low_pct": "-1.55",
        "low_usd": "0.01274292",
        "tick_lower": 273600,
        "tick_upper": 273900
      },
      "cap": {
        "band_rule": "the band 273600..273900 of the 300-tick grid contains the pool's tick 273743",
        "bound_by": "lp_nock_budget",
        "limit_rule": "never more USDC than the first near rung's target; once a near or deep rung is deposited, never more than the reserve leaves for it beside the other deposits listed",
        "lp_nock_budget": "86.7891",
        "nock": "86.7891",
        "note": "the cap holds the whole LP NOCK budget, 86.7891 NOCK; the curve pairs it with $1.23 USDC",
        "pool_tick": 273743,
        "tick_lower": 273600,
        "tick_upper": 273900,
        "usdc": "1.23",
        "usdc_limit": "100.04",
        "usdc_limit_is": "near_share"
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
        "band_measured": {
          "days": "0.12",
          "seconds": "10659",
          "usd_per_day": null,
          "usd_total_upper": "0.02"
        },
        "band_reason": "only 10659 s of stored intervals sat wholly inside: below the 72000 s a per-day figure needs, so only the dollars and seconds are stated",
        "constants": {
          "band_min_seconds": 72000.0,
          "day_min_seconds": 72000,
          "max_gap_seconds": 7200,
          "note": "display constants, not settings",
          "window_days": 30
        },
        "coverage": {
          "high_usd": "0.01313098",
          "low_usd": "0.01274292",
          "note": "the whole range lies inside the measuring range",
          "whole": true
        },
        "dilution": {
          "active_liquidity_raw": "80173935548453779",
          "factor": "0.998279",
          "new_liquidity_raw": "138192115820500",
          "share_of_active_pct": "0.17"
        },
        "dilution_reason": null,
        "in_range_share_reason": null,
        "in_range_share_stored_history": {
          "edge_seconds": "900",
          "note": "the share of the measured seconds whose two endpoint ticks were both inside this range: a past measurement, not a forecast",
          "of_seconds": "547205",
          "seconds": "10659",
          "share_pct": "1.95"
        },
        "intervals": 1836,
        "measured_at": "2026-11-20T11:50:00Z",
        "measured_by": "uniswap-v4:base:3163450",
        "measured_days": "6.33",
        "measured_label": "v4 USDC/NOCK 3.00% #3163450",
        "measured_seconds": 547205.136,
        "measurement_age_seconds": 900,
        "per_100_usd_per_day_in_range_usd": "35.77",
        "per_day_in_range_diluted_usd": "0.84",
        "per_day_in_range_nock_leg": "30.6176",
        "per_day_in_range_usd": "0.84",
        "per_day_in_range_usdc_leg": "0.44",
        "rate": {
          "nock_per_liquidity_day": "2.215582e-13",
          "usd_per_1e15_liquidity_day": "6.0811",
          "usdc_per_liquidity_day": "3.213286e-15",
          "valued_at_pool_price_usd": "0.01294400"
        },
        "reason": null,
        "replay_reason": null,
        "skipped": {
          "fees_fell": 6,
          "gap": 1,
          "liquidity_changed": 4,
          "out_of_range": 0,
          "pool_inactive": 0,
          "staked": 0,
          "unreadable": 0,
          "zero_liquidity": 137
        },
        "skipped_seconds": {
          "fees_fell": 1776.479,
          "gap": 14179.252,
          "liquidity_changed": 1202.24,
          "out_of_range": 0.0,
          "pool_inactive": 0.0,
          "staked": 0.0,
          "unreadable": 0.0,
          "zero_liquidity": 38413.578
        },
        "spread": {
          "day_min_seconds": 72000.0,
          "days": [
            {
              "counted": false,
              "date": "2026-11-14",
              "seconds": 30699.262,
              "usd_per_day": "0.53"
            },
            {
              "counted": true,
              "date": "2026-11-15",
              "seconds": 86488.395,
              "usd_per_day": "0.69"
            },
            {
              "counted": true,
              "date": "2026-11-16",
              "seconds": 86006.946,
              "usd_per_day": "0.99"
            },
            {
              "counted": false,
              "date": "2026-11-17",
              "seconds": 71106.303,
              "usd_per_day": "1.35"
            },
            {
              "counted": true,
              "date": "2026-11-18",
              "seconds": 86146.129,
              "usd_per_day": "0.43"
            },
            {
              "counted": true,
              "date": "2026-11-19",
              "seconds": 85799.496,
              "usd_per_day": "1.10"
            },
            {
              "counted": true,
              "date": "2026-11-20",
              "seconds": 86399.664,
              "usd_per_day": "0.73"
            },
            {
              "counted": false,
              "date": "2026-11-21",
              "seconds": 14558.942,
              "usd_per_day": "0.53"
            }
          ],
          "days_counted": 5,
          "max_usd_per_day": "1.10",
          "median_usd_per_day": "0.73",
          "min_usd_per_day": "0.43",
          "note": "UTC days; an interval counts on the date it starts. Every day is listed; the spread is over the days counted."
        },
        "spread_reason": null,
        "status": "measured",
        "stored_history_replay": {
          "edge_seconds": "900",
          "edge_upper_usd": "0.15",
          "inside_seconds": "10659",
          "inside_upper_usd": "0.02",
          "measured_days": "6.33",
          "note": "At most: from endpoint ticks sampled every ~5 minutes, an interval counts when both ends sat inside the range (inside) or one did (edge); the price can leave and return within an interval, and a part of a range never earns more per unit of liquidity than the range containing it. No lower bound is claimed (it can be $0).",
          "undiluted": true
        }
      },
      "if_pulled_at_fill": null,
      "index": 1,
      "inputs": {
        "high_requested": "0.013130977294699456",
        "low_requested": "0.012742917379855624",
        "nock": null,
        "usdc": "1.23"
      },
      "max_usdc_with_the_others_as_given": "1.23",
      "mint": {
        "amount0_raw": "1225999",
        "amount1_raw": "867890958577132038",
        "liquidity_raw": "138192109253769",
        "nock": "86.7890958577132038",
        "nock_max_raw": "867891000000000000",
        "nock_raw": "867890958577132038",
        "note": "liquidity_raw is what these amounts buy, rounded down (LiquidityAmounts); amount0_raw and amount1_raw are what a mint of it pulls, rounded up (SqrtPriceMath), never more than the maxima (the rung's USDC and NOCK truncated to raw units)",
        "status": "exact",
        "tick": 273743,
        "usdc": "1.225999",
        "usdc_max_raw": "1225999",
        "usdc_raw": "1225999"
      },
      "plan": {
        "counterparts": [],
        "deployed_usdc": "0.000000",
        "remaining_usdc": "1.225999",
        "status": "to_deposit",
        "target_usdc": "1.225999"
      },
      "round_trip": {
        "back_to_usd": "0.01294400",
        "down_to_usd": "0.01274292",
        "nock_back": "86.7891",
        "nock_start": "86.7891",
        "text": "A fall to $0.01274292 and a return to $0.01294400 (the pool price now) hands back $1.23 USDC and 86.7891 NOCK, what this rung started with: the round trip's gain is the fees it earned in range (fee_estimate), and gas is not counted.",
        "usdc_back": "1.23",
        "usdc_start": "1.23"
      },
      "rung": "cap",
      "sizing": {
        "liquidity_human": "1381.9212",
        "liquidity_raw": "138192115820500",
        "nock_check": "not_given",
        "nock_given": null,
        "nock_needed_at_spot": "86.7891",
        "usdc_at_spot": "1.23"
      },
      "top": {
        "out_of_range_above_now": false,
        "pct_of_spot": "+1.44",
        "price_usd": "0.01313098",
        "usdc_held": "2.36"
      }
    },
    {
      "bottom": {
        "avg_buy_price_usd": "0.01182220",
        "nock_bought": "9716.0065",
        "nock_held": "9716.0065",
        "pct_of_spot": "-15.27",
        "price_usd": "0.01096801"
      },
      "bounds": {
        "high_pct": "-1.55",
        "high_rule": "the usable tick at or below the pool price (the first rung still drawable)",
        "high_usd": "0.01274292",
        "low_pct": "-15.27",
        "low_usd": "0.01096801",
        "tick_lower": 273900,
        "tick_upper": 275400
      },
      "cap": null,
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
        "band_measured": {
          "days": "1.81",
          "seconds": "156662",
          "usd_per_day": "3.52",
          "usd_total_upper": "6.39"
        },
        "band_reason": null,
        "constants": {
          "band_min_seconds": 72000.0,
          "day_min_seconds": 72000,
          "max_gap_seconds": 7200,
          "note": "display constants, not settings",
          "window_days": 30
        },
        "coverage": {
          "high_usd": "0.01274292",
          "low_usd": "0.01096801",
          "note": "the whole range lies inside the measuring range",
          "whole": true
        },
        "dilution": {
          "active_liquidity_raw": "80173935548453779",
          "factor": "0.982738",
          "new_liquidity_raw": "1408301616732632",
          "share_of_active_pct": "1.73"
        },
        "dilution_reason": null,
        "in_range_share_reason": null,
        "in_range_share_stored_history": {
          "edge_seconds": "2101",
          "note": "the share of the measured seconds whose two endpoint ticks were both inside this range: a past measurement, not a forecast",
          "of_seconds": "547205",
          "seconds": "156662",
          "share_pct": "28.63"
        },
        "intervals": 1836,
        "measured_at": "2026-11-20T11:50:00Z",
        "measured_by": "uniswap-v4:base:3163450",
        "measured_days": "6.33",
        "measured_label": "v4 USDC/NOCK 3.00% #3163450",
        "measured_seconds": 547205.136,
        "measurement_age_seconds": 900,
        "per_100_usd_per_day_in_range_usd": "7.46",
        "per_day_in_range_diluted_usd": "8.42",
        "per_day_in_range_nock_leg": "312.0208",
        "per_day_in_range_usd": "8.56",
        "per_day_in_range_usdc_leg": "4.53",
        "rate": {
          "nock_per_liquidity_day": "2.215582e-13",
          "usd_per_1e15_liquidity_day": "6.0811",
          "usdc_per_liquidity_day": "3.213286e-15",
          "valued_at_pool_price_usd": "0.01294400"
        },
        "reason": null,
        "replay_reason": null,
        "skipped": {
          "fees_fell": 6,
          "gap": 1,
          "liquidity_changed": 4,
          "out_of_range": 0,
          "pool_inactive": 0,
          "staked": 0,
          "unreadable": 0,
          "zero_liquidity": 137
        },
        "skipped_seconds": {
          "fees_fell": 1776.479,
          "gap": 14179.252,
          "liquidity_changed": 1202.24,
          "out_of_range": 0.0,
          "pool_inactive": 0.0,
          "staked": 0.0,
          "unreadable": 0.0,
          "zero_liquidity": 38413.578
        },
        "spread": {
          "day_min_seconds": 72000.0,
          "days": [
            {
              "counted": false,
              "date": "2026-11-14",
              "seconds": 30699.262,
              "usd_per_day": "5.41"
            },
            {
              "counted": true,
              "date": "2026-11-15",
              "seconds": 86488.395,
              "usd_per_day": "7.06"
            },
            {
              "counted": true,
              "date": "2026-11-16",
              "seconds": 86006.946,
              "usd_per_day": "10.13"
            },
            {
              "counted": false,
              "date": "2026-11-17",
              "seconds": 71106.303,
              "usd_per_day": "13.71"
            },
            {
              "counted": true,
              "date": "2026-11-18",
              "seconds": 86146.129,
              "usd_per_day": "4.36"
            },
            {
              "counted": true,
              "date": "2026-11-19",
              "seconds": 85799.496,
              "usd_per_day": "11.25"
            },
            {
              "counted": true,
              "date": "2026-11-20",
              "seconds": 86399.664,
              "usd_per_day": "7.45"
            },
            {
              "counted": false,
              "date": "2026-11-21",
              "seconds": 14558.942,
              "usd_per_day": "5.44"
            }
          ],
          "days_counted": 5,
          "max_usd_per_day": "11.25",
          "median_usd_per_day": "7.45",
          "min_usd_per_day": "4.36",
          "note": "UTC days; an interval counts on the date it starts. Every day is listed; the spread is over the days counted."
        },
        "spread_reason": null,
        "status": "measured",
        "stored_history_replay": {
          "edge_seconds": "2101",
          "edge_upper_usd": "5.16",
          "inside_seconds": "156662",
          "inside_upper_usd": "6.39",
          "measured_days": "6.33",
          "note": "At most: from endpoint ticks sampled every ~5 minutes, an interval counts when both ends sat inside the range (inside) or one did (edge); the price can leave and return within an interval, and a part of a range never earns more per unit of liquidity than the range containing it. No lower bound is claimed (it can be $0).",
          "undiluted": true
        }
      },
      "if_pulled_at_fill": null,
      "index": 2,
      "inputs": {
        "high_requested": "0.012742917379855624",
        "low_requested": "0.010968012886072376",
        "nock": null,
        "usdc": "114.86"
      },
      "max_usdc_with_the_others_as_given": "114.86",
      "mint": {
        "amount0_raw": "114864600",
        "amount1_raw": "0",
        "liquidity_raw": "1408301615002237",
        "nock": "0.0000000000000000",
        "nock_max_raw": "0",
        "nock_raw": "0",
        "note": "liquidity_raw is what these amounts buy, rounded down (LiquidityAmounts); amount0_raw and amount1_raw are what a mint of it pulls, rounded up (SqrtPriceMath), never more than the maxima (the rung's USDC and NOCK truncated to raw units)",
        "status": "exact",
        "tick": 273743,
        "usdc": "114.864600",
        "usdc_max_raw": "114864600",
        "usdc_raw": "114864600"
      },
      "plan": {
        "counterparts": [],
        "deployed_usdc": "0.000000",
        "remaining_usdc": "114.864600",
        "status": "to_deposit",
        "target_usdc": "114.864600"
      },
      "round_trip": {
        "back_to_usd": "0.01294400",
        "down_to_usd": "0.01096801",
        "nock_back": "0.0000",
        "nock_start": "0.0000",
        "text": "A fall to $0.01096801 and a return to $0.01294400 (the pool price now) hands back $114.86 USDC and 0.0000 NOCK, what this rung started with: the round trip's gain is the fees it earned in range (fee_estimate), and gas is not counted.",
        "usdc_back": "114.86",
        "usdc_start": "114.86"
      },
      "rung": "near",
      "sizing": {
        "liquidity_human": "14083.0162",
        "liquidity_raw": "1408301616732632",
        "nock_check": "not_given",
        "nock_given": null,
        "nock_needed_at_spot": "0.0000",
        "usdc_at_spot": "114.86"
      },
      "top": {
        "out_of_range_above_now": true,
        "pct_of_spot": "-1.55",
        "price_usd": "0.01274292",
        "usdc_held": "114.86"
      }
    },
    {
      "bottom": {
        "avg_buy_price_usd": "0.01048542",
        "nock_bought": "62076.6389",
        "nock_held": "62076.6389",
        "pct_of_spot": "-22.56",
        "price_usd": "0.01002405"
      },
      "bounds": {
        "high_pct": "-15.27",
        "high_rule": "the nearest usable tick",
        "high_usd": "0.01096801",
        "low_pct": "-22.56",
        "low_usd": "0.01002405",
        "tick_lower": 275400,
        "tick_upper": 276300
      },
      "cap": null,
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
        "band_measured": {
          "days": "0.37",
          "seconds": "31832",
          "usd_per_day": null,
          "usd_total_upper": "32.55"
        },
        "band_reason": "only 31832 s of stored intervals sat wholly inside: below the 72000 s a per-day figure needs, so only the dollars and seconds are stated",
        "constants": {
          "band_min_seconds": 72000.0,
          "day_min_seconds": 72000,
          "max_gap_seconds": 7200,
          "note": "display constants, not settings",
          "window_days": 30
        },
        "coverage": {
          "high_usd": "0.01096801",
          "low_usd": "0.01002405",
          "note": "the whole range lies inside the measuring range",
          "whole": true
        },
        "dilution": {
          "active_liquidity_raw": "80173935548453779",
          "factor": "0.850209",
          "new_liquidity_raw": "14125167085436768",
          "share_of_active_pct": "14.98"
        },
        "dilution_reason": null,
        "in_range_share_reason": null,
        "in_range_share_stored_history": {
          "edge_seconds": "1801",
          "note": "the share of the measured seconds whose two endpoint ticks were both inside this range: a past measurement, not a forecast",
          "of_seconds": "547205",
          "seconds": "31832",
          "share_pct": "5.82"
        },
        "intervals": 1836,
        "measured_at": "2026-11-20T11:50:00Z",
        "measured_by": "uniswap-v4:base:3163450",
        "measured_days": "6.33",
        "measured_label": "v4 USDC/NOCK 3.00% #3163450",
        "measured_seconds": 547205.136,
        "measurement_age_seconds": 900,
        "per_100_usd_per_day_in_range_usd": "13.20",
        "per_day_in_range_diluted_usd": "73.03",
        "per_day_in_range_nock_leg": "3129.5466",
        "per_day_in_range_usd": "85.90",
        "per_day_in_range_usdc_leg": "45.39",
        "rate": {
          "nock_per_liquidity_day": "2.215582e-13",
          "usd_per_1e15_liquidity_day": "6.0811",
          "usdc_per_liquidity_day": "3.213286e-15",
          "valued_at_pool_price_usd": "0.01294400"
        },
        "reason": null,
        "replay_reason": null,
        "skipped": {
          "fees_fell": 6,
          "gap": 1,
          "liquidity_changed": 4,
          "out_of_range": 0,
          "pool_inactive": 0,
          "staked": 0,
          "unreadable": 0,
          "zero_liquidity": 137
        },
        "skipped_seconds": {
          "fees_fell": 1776.479,
          "gap": 14179.252,
          "liquidity_changed": 1202.24,
          "out_of_range": 0.0,
          "pool_inactive": 0.0,
          "staked": 0.0,
          "unreadable": 0.0,
          "zero_liquidity": 38413.578
        },
        "spread": {
          "day_min_seconds": 72000.0,
          "days": [
            {
              "counted": false,
              "date": "2026-11-14",
              "seconds": 30699.262,
              "usd_per_day": "54.23"
            },
            {
              "counted": true,
              "date": "2026-11-15",
              "seconds": 86488.395,
              "usd_per_day": "70.76"
            },
            {
              "counted": true,
              "date": "2026-11-16",
              "seconds": 86006.946,
              "usd_per_day": "101.59"
            },
            {
              "counted": false,
              "date": "2026-11-17",
              "seconds": 71106.303,
              "usd_per_day": "137.52"
            },
            {
              "counted": true,
              "date": "2026-11-18",
              "seconds": 86146.129,
              "usd_per_day": "43.74"
            },
            {
              "counted": true,
              "date": "2026-11-19",
              "seconds": 85799.496,
              "usd_per_day": "112.86"
            },
            {
              "counted": true,
              "date": "2026-11-20",
              "seconds": 86399.664,
              "usd_per_day": "74.75"
            },
            {
              "counted": false,
              "date": "2026-11-21",
              "seconds": 14558.942,
              "usd_per_day": "54.52"
            }
          ],
          "days_counted": 5,
          "max_usd_per_day": "112.86",
          "median_usd_per_day": "74.75",
          "min_usd_per_day": "43.74",
          "note": "UTC days; an interval counts on the date it starts. Every day is listed; the spread is over the days counted."
        },
        "spread_reason": null,
        "status": "measured",
        "stored_history_replay": {
          "edge_seconds": "1801",
          "edge_upper_usd": "50.00",
          "inside_seconds": "31832",
          "inside_upper_usd": "32.55",
          "measured_days": "6.33",
          "note": "At most: from endpoint ticks sampled every ~5 minutes, an interval counts when both ends sat inside the range (inside) or one did (edge); the price can leave and return within an interval, and a part of a range never earns more per unit of liquidity than the range containing it. No lower bound is claimed (it can be $0).",
          "undiluted": true
        }
      },
      "if_pulled_at_fill": {
        "at_usd": "0.01002405",
        "avg_price_usd": "0.01048542",
        "nock_bought": "62076.6389",
        "nock_kept": "62076.6389",
        "note": "Pulled when fully NOCK (at its bottom), the rung keeps this NOCK on a rebound; left in place, a rebound sells it back. Pulling is a withdrawal you sign; gas is not counted."
      },
      "index": 3,
      "inputs": {
        "high_requested": "0.010968012886072376",
        "low_requested": "0.0100240541221736",
        "nock": null,
        "usdc": "650.90"
      },
      "max_usdc_with_the_others_as_given": "650.90",
      "mint": {
        "amount0_raw": "650899400",
        "amount1_raw": "0",
        "liquidity_raw": "14125167068081104",
        "nock": "0.0000000000000000",
        "nock_max_raw": "0",
        "nock_raw": "0",
        "note": "liquidity_raw is what these amounts buy, rounded down (LiquidityAmounts); amount0_raw and amount1_raw are what a mint of it pulls, rounded up (SqrtPriceMath), never more than the maxima (the rung's USDC and NOCK truncated to raw units)",
        "status": "exact",
        "tick": 273743,
        "usdc": "650.899400",
        "usdc_max_raw": "650899400",
        "usdc_raw": "650899400"
      },
      "plan": {
        "counterparts": [],
        "deployed_usdc": "0.000000",
        "remaining_usdc": "650.899401",
        "status": "to_deposit",
        "target_usdc": "650.899401"
      },
      "round_trip": null,
      "rung": "deep",
      "sizing": {
        "liquidity_human": "141251.6709",
        "liquidity_raw": "14125167085436768",
        "nock_check": "not_given",
        "nock_given": null,
        "nock_needed_at_spot": "0.0000",
        "usdc_at_spot": "650.90"
      },
      "top": {
        "out_of_range_above_now": true,
        "pct_of_spot": "-15.27",
        "price_usd": "0.01096801",
        "usdc_held": "650.90"
      }
    }
  ],
  "shape_status": "computed",
  "sizing": {
    "describes": "the sum over the rungs; liquidity is per rung (rungs[].sizing)",
    "liquidity_human": null,
    "liquidity_raw": null,
    "nock_check": "per_rung",
    "nock_given": null,
    "nock_needed_at_spot": "86.7891",
    "usdc_at_spot": "766.99"
  },
  "snapshot": {
    "age_minutes": 5,
    "age_seconds": 300,
    "generated_at": "2026-11-20T12:00:00Z",
    "snapshot_id": "00000000-0000-0000-0000-000000000000",
    "spot_source": "price-source pool sqrtPriceX96 (v4 USDC/NOCK 3.00% #3163450)",
    "spot_usd": "0.01294400",
    "version": "2.10.0",
    "worker_health": "ok"
  },
  "statement": "A what-if from the stored snapshot. NockNock reads nothing new, signs nothing, sends nothing, and does not choose the amounts.",
  "top": {
    "avg_sell_price_usd": "0.01303715",
    "describes": "the sum over the rungs at the highest rung's top",
    "multiple_of_spot": "1.0144",
    "out_of_range_above_now": false,
    "pct_of_spot": "+1.44",
    "price_usd": "0.01313098",
    "usdc_held": "768.12"
  },
  "top_neighbours": null,
  "top_neighbours_reason": "a ladder of rungs lists no neighbouring tops; each rung's bounds are in rungs[]",
  "tradeoff": {
    "columns_note": "Fee per day in range is always shown with the in-range share of stored history and the replay upper bounds: a high rate while in range is not a rate per day.",
    "deep_top": null,
    "deep_top_reason": "the depth tables are drawn for one near and one deep rung",
    "near_depth": null,
    "near_depth_reason": "the depth tables are drawn for one near and one deep rung",
    "order": "Rows are in the order of the quantity varied and are not ranked.",
    "split": null,
    "split_reason": "the split table is drawn for two rungs",
    "statement": "Rows are measurements at your depths; NockNock does not choose the split or the depths."
  },
  "tradeoff_reason": null
};
