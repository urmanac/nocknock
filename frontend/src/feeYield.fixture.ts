/**
 * derived_metrics.fee_yield of the #166 fixture with the recorded chudmax history of
 * #3163450 (2.10.0), copied from ingestion-worker/testdata/fee_yield/golden_fee_yield.json,
 * which ingestion-worker/src/tests_fee_yield.rs writes. feeYield.test.ts checks the copy.
 */
import type { FeeYield } from './feeYield';

export const FEE_YIELD: FeeYield = {
  "computed_at": "2026-11-20T11:50:00Z",
  "constants_note": "window_days, max_gap_seconds, day_min_seconds and refresh_seconds are measurement and display constants, not settings",
  "day_min_seconds": 72000,
  "excluded": [
    {
      "position_id": "aerodrome:base:0x770fe8fc9248c6382950a86e39a9391b41d59e0a",
      "reason": "a full-range position (a basic Aero pool): it has no range to measure, and its claimFees reverts"
    }
  ],
  "max_gap_seconds": 7200,
  "pools": [
    {
      "by_day": [
        {
          "date": "2026-11-14",
          "nock_per_liquidity_day": "2.582094e-13",
          "seconds": 30699.262,
          "usd_per_liquidity_day": "3.839248e-15",
          "usdc_per_liquidity_day": "4.969857e-16"
        },
        {
          "date": "2026-11-15",
          "nock_per_liquidity_day": "1.846029e-13",
          "seconds": 86488.395,
          "usd_per_liquidity_day": "5.009633e-15",
          "usdc_per_liquidity_day": "2.620133e-15"
        },
        {
          "date": "2026-11-16",
          "nock_per_liquidity_day": "1.651645e-13",
          "seconds": 86006.946,
          "usd_per_liquidity_day": "7.192070e-15",
          "usdc_per_liquidity_day": "5.054181e-15"
        },
        {
          "date": "2026-11-17",
          "nock_per_liquidity_day": "3.273413e-13",
          "seconds": 71106.303,
          "usd_per_liquidity_day": "9.735534e-15",
          "usdc_per_liquidity_day": "5.498428e-15"
        },
        {
          "date": "2026-11-18",
          "nock_per_liquidity_day": "6.401196e-14",
          "seconds": 86146.129,
          "usd_per_liquidity_day": "3.096424e-15",
          "usdc_per_liquidity_day": "2.267853e-15"
        },
        {
          "date": "2026-11-19",
          "nock_per_liquidity_day": "2.296357e-13",
          "seconds": 85799.496,
          "usd_per_liquidity_day": "7.989884e-15",
          "usdc_per_liquidity_day": "5.017479e-15"
        },
        {
          "date": "2026-11-20",
          "nock_per_liquidity_day": "3.507609e-13",
          "seconds": 86399.664,
          "usd_per_liquidity_day": "5.291720e-15",
          "usdc_per_liquidity_day": "7.514707e-16"
        },
        {
          "date": "2026-11-21",
          "nock_per_liquidity_day": "2.981667e-13",
          "seconds": 14558.942,
          "usd_per_liquidity_day": "3.859470e-15",
          "usdc_per_liquidity_day": "0.000000e0"
        }
      ],
      "covered_tick_lower": 262200,
      "covered_tick_upper": 282300,
      "fee_tier_bps": 300,
      "from": "2026-11-14T04:38:29Z",
      "intervals": 1836,
      "label": "v4 USDC/NOCK 3.00% #3163450",
      "measured_by": "uniswap-v4:base:3163450",
      "measured_seconds": 547205.136,
      "nock_per_liquidity_day": "2.215582e-13",
      "pool_id": "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13",
      "pool_price_usd": "0.01294400",
      "reason": null,
      "rows": 1985,
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
      "spans": [
        {
          "bin_hi": 269700,
          "bin_lo": 269700,
          "nock_per_liquidity": "0.000000e0",
          "seconds": 299.659,
          "usdc_per_liquidity": "0.000000e0"
        },
        {
          "bin_hi": 270000,
          "bin_lo": 269700,
          "nock_per_liquidity": "1.787259e-14",
          "seconds": 300.093,
          "usdc_per_liquidity": "0.000000e0"
        },
        {
          "bin_hi": 271200,
          "bin_lo": 269700,
          "nock_per_liquidity": "0.000000e0",
          "seconds": 300.153,
          "usdc_per_liquidity": "3.115372e-15"
        },
        {
          "bin_hi": 270000,
          "bin_lo": 270000,
          "nock_per_liquidity": "2.799415e-14",
          "seconds": 34799.363,
          "usdc_per_liquidity": "6.345010e-20"
        },
        {
          "bin_hi": 270300,
          "bin_lo": 270000,
          "nock_per_liquidity": "4.943626e-14",
          "seconds": 599.576,
          "usdc_per_liquidity": "0.000000e0"
        },
        {
          "bin_hi": 270900,
          "bin_lo": 270000,
          "nock_per_liquidity": "0.000000e0",
          "seconds": 300.176,
          "usdc_per_liquidity": "1.867136e-15"
        },
        {
          "bin_hi": 270300,
          "bin_lo": 270300,
          "nock_per_liquidity": "1.909862e-14",
          "seconds": 14100.257,
          "usdc_per_liquidity": "3.525006e-20"
        },
        {
          "bin_hi": 270600,
          "bin_lo": 270300,
          "nock_per_liquidity": "2.374865e-14",
          "seconds": 600.184,
          "usdc_per_liquidity": "0.000000e0"
        },
        {
          "bin_hi": 270600,
          "bin_lo": 270600,
          "nock_per_liquidity": "3.188204e-14",
          "seconds": 13200.372,
          "usdc_per_liquidity": "0.000000e0"
        },
        {
          "bin_hi": 270900,
          "bin_lo": 270600,
          "nock_per_liquidity": "4.512589e-14",
          "seconds": 599.649,
          "usdc_per_liquidity": "0.000000e0"
        },
        {
          "bin_hi": 270900,
          "bin_lo": 270900,
          "nock_per_liquidity": "3.272684e-14",
          "seconds": 38699.706,
          "usdc_per_liquidity": "0.000000e0"
        },
        {
          "bin_hi": 271200,
          "bin_lo": 270900,
          "nock_per_liquidity": "5.865831e-15",
          "seconds": 299.779,
          "usdc_per_liquidity": "0.000000e0"
        },
        {
          "bin_hi": 271200,
          "bin_lo": 271200,
          "nock_per_liquidity": "1.082447e-14",
          "seconds": 12300.212,
          "usdc_per_liquidity": "0.000000e0"
        },
        {
          "bin_hi": 271500,
          "bin_lo": 271200,
          "nock_per_liquidity": "2.975228e-14",
          "seconds": 600.094,
          "usdc_per_liquidity": "3.668015e-16"
        },
        {
          "bin_hi": 271500,
          "bin_lo": 271500,
          "nock_per_liquidity": "4.492832e-14",
          "seconds": 63023.308,
          "usdc_per_liquidity": "8.238283e-16"
        },
        {
          "bin_hi": 271800,
          "bin_lo": 271500,
          "nock_per_liquidity": "1.404995e-14",
          "seconds": 899.969,
          "usdc_per_liquidity": "1.070560e-15"
        },
        {
          "bin_hi": 272100,
          "bin_lo": 271500,
          "nock_per_liquidity": "7.411948e-14",
          "seconds": 299.749,
          "usdc_per_liquidity": "0.000000e0"
        },
        {
          "bin_hi": 271800,
          "bin_lo": 271800,
          "nock_per_liquidity": "4.845599e-15",
          "seconds": 26425.043,
          "usdc_per_liquidity": "0.000000e0"
        },
        {
          "bin_hi": 272100,
          "bin_lo": 272100,
          "nock_per_liquidity": "3.245414e-14",
          "seconds": 38398.187,
          "usdc_per_liquidity": "1.818740e-16"
        },
        {
          "bin_hi": 272400,
          "bin_lo": 272100,
          "nock_per_liquidity": "4.307908e-14",
          "seconds": 1485.877,
          "usdc_per_liquidity": "7.386827e-16"
        },
        {
          "bin_hi": 272700,
          "bin_lo": 272100,
          "nock_per_liquidity": "6.140030e-14",
          "seconds": 299.94,
          "usdc_per_liquidity": "0.000000e0"
        },
        {
          "bin_hi": 273000,
          "bin_lo": 272100,
          "nock_per_liquidity": "1.142992e-13",
          "seconds": 300.068,
          "usdc_per_liquidity": "0.000000e0"
        },
        {
          "bin_hi": 273300,
          "bin_lo": 272100,
          "nock_per_liquidity": "0.000000e0",
          "seconds": 300.244,
          "usdc_per_liquidity": "1.852655e-15"
        },
        {
          "bin_hi": 272400,
          "bin_lo": 272400,
          "nock_per_liquidity": "5.005972e-14",
          "seconds": 20423.574,
          "usdc_per_liquidity": "6.373314e-16"
        },
        {
          "bin_hi": 272700,
          "bin_lo": 272400,
          "nock_per_liquidity": "3.952374e-14",
          "seconds": 1200.263,
          "usdc_per_liquidity": "6.637412e-16"
        },
        {
          "bin_hi": 272700,
          "bin_lo": 272700,
          "nock_per_liquidity": "4.806875e-14",
          "seconds": 17699.992,
          "usdc_per_liquidity": "2.081465e-16"
        },
        {
          "bin_hi": 273000,
          "bin_lo": 272700,
          "nock_per_liquidity": "7.264259e-14",
          "seconds": 1500.088,
          "usdc_per_liquidity": "9.967422e-16"
        },
        {
          "bin_hi": 273000,
          "bin_lo": 273000,
          "nock_per_liquidity": "6.132246e-14",
          "seconds": 32096.18,
          "usdc_per_liquidity": "6.569999e-16"
        },
        {
          "bin_hi": 273300,
          "bin_lo": 273000,
          "nock_per_liquidity": "2.549960e-14",
          "seconds": 899.951,
          "usdc_per_liquidity": "8.287011e-16"
        },
        {
          "bin_hi": 273300,
          "bin_lo": 273300,
          "nock_per_liquidity": "1.222792e-14",
          "seconds": 23099.99,
          "usdc_per_liquidity": "0.000000e0"
        },
        {
          "bin_hi": 273600,
          "bin_lo": 273300,
          "nock_per_liquidity": "5.024297e-14",
          "seconds": 600.181,
          "usdc_per_liquidity": "3.213239e-16"
        },
        {
          "bin_hi": 273600,
          "bin_lo": 273600,
          "nock_per_liquidity": "0.000000e0",
          "seconds": 10658.593,
          "usdc_per_liquidity": "1.785221e-16"
        },
        {
          "bin_hi": 273900,
          "bin_lo": 273600,
          "nock_per_liquidity": "0.000000e0",
          "seconds": 300.107,
          "usdc_per_liquidity": "1.221905e-16"
        },
        {
          "bin_hi": 273900,
          "bin_lo": 273900,
          "nock_per_liquidity": "0.000000e0",
          "seconds": 599.945,
          "usdc_per_liquidity": "4.363339e-16"
        },
        {
          "bin_hi": 274800,
          "bin_lo": 273900,
          "nock_per_liquidity": "0.000000e0",
          "seconds": 300.022,
          "usdc_per_liquidity": "1.446881e-15"
        },
        {
          "bin_hi": 274800,
          "bin_lo": 274800,
          "nock_per_liquidity": "3.050288e-14",
          "seconds": 61868.354,
          "usdc_per_liquidity": "1.677700e-16"
        },
        {
          "bin_hi": 275100,
          "bin_lo": 274800,
          "nock_per_liquidity": "2.977878e-14",
          "seconds": 917.596,
          "usdc_per_liquidity": "7.359993e-16"
        },
        {
          "bin_hi": 275400,
          "bin_lo": 274800,
          "nock_per_liquidity": "7.231376e-14",
          "seconds": 300.049,
          "usdc_per_liquidity": "0.000000e0"
        },
        {
          "bin_hi": 275100,
          "bin_lo": 275100,
          "nock_per_liquidity": "3.601733e-14",
          "seconds": 92976.057,
          "usdc_per_liquidity": "5.041188e-16"
        },
        {
          "bin_hi": 275400,
          "bin_lo": 275100,
          "nock_per_liquidity": "1.057090e-13",
          "seconds": 1500.555,
          "usdc_per_liquidity": "1.235485e-15"
        },
        {
          "bin_hi": 275400,
          "bin_lo": 275400,
          "nock_per_liquidity": "3.826870e-15",
          "seconds": 30932.013,
          "usdc_per_liquidity": "4.631253e-16"
        },
        {
          "bin_hi": 275700,
          "bin_lo": 275400,
          "nock_per_liquidity": "0.000000e0",
          "seconds": 299.953,
          "usdc_per_liquidity": "5.501705e-16"
        },
        {
          "bin_hi": 276000,
          "bin_lo": 275400,
          "nock_per_liquidity": "8.197511e-14",
          "seconds": 299.874,
          "usdc_per_liquidity": "0.000000e0"
        },
        {
          "bin_hi": 276000,
          "bin_lo": 275700,
          "nock_per_liquidity": "0.000000e0",
          "seconds": 300.14,
          "usdc_per_liquidity": "1.804107e-16"
        }
      ],
      "spans_reason": null,
      "status": "measured",
      "tick_spacing": 300,
      "to": "2026-11-21T04:04:46Z",
      "usd_per_1e15_liquidity_day": "6.0811",
      "usd_per_liquidity_day": "6.081135e-15",
      "usdc_per_liquidity_day": "3.213286e-15",
      "zero_fee_gap_intervals": 0
    }
  ],
  "reason": null,
  "refresh_seconds": 3600,
  "statement": "Measured from stored snapshots of our own tracked ranges: swap fees per unit of liquidity while in range. A past measurement, not a forecast. The stored market volume of another pool is never used.",
  "status": "measured",
  "window_days": 30
};
