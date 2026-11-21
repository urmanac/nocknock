/**
 * Calls to action and the ladder maintenance (2.11.0, #169) as the worker states them for
 * the operator's real bottle (chudmax, 2026-11-21T16:10Z), copied from
 * ingestion-worker/testdata/ladder_maintenance/golden_events_text.json, which
 * ingestion-worker/src/tests_ladder_maintenance.rs writes. events.test.ts checks the copy
 * against the file when the worker directory is present.
 */
import type { LadderMaintenance, SnapshotEvent } from './events';

export interface EventsFixture {
  derived_metrics: { events?: SnapshotEvent[]; ladder_maintenance?: LadderMaintenance };
}

/** live_unset: the live bottle with every restraint setting unset (the reserve is at the top: wallet USDC is below $950) */
export const EVENTS_LIVE_UNSET: EventsFixture = {
  "derived_metrics": {
    "events": [
      {
        "card": "reserve",
        "deep_link": "#event-reserve-below",
        "id": "reserve_below",
        "kind": "reserve_below",
        "links": [],
        "numbers": {
          "eth_covering_pct_above_now": "9.1",
          "eth_price_covering_shortfall_usd": "1830.74",
          "eth_price_usd": "1677.54",
          "eth_ranges_cover_shortfall": false,
          "eth_ranges_usdc": "99.09",
          "eth_ranges_weth": "0.2345",
          "eth_ranges_weth_at_covering": "0.1933",
          "ladder_usdc": "768.24",
          "nock_lp_usdc": "233.86",
          "reserve_usdc": "950.00",
          "stable_headroom_usdc": "830.87",
          "stable_usdc": "1780.87",
          "wallet_shortfall_usdc": "171.23",
          "wallet_usdc": "778.77",
          "wallet_usdc_if_eth_ranges_withdrawn": "877.85"
        },
        "since": "2026-11-21T16:10:35.795544312+00:00",
        "source": "derived_metrics.global_status",
        "text": "Wallet USDC $778.77 is $171.23 below your $950.00 reserve. The stable balance the reserve gate counts is $1780.87 (wallet USDC $778.77 + NOCK LP USDC $233.86 + ladder USDC $768.24), $830.87 above it. The gas-side ETH/USDC ranges (outside the reserve gate) hold $99.09 USDC and 0.2345 WETH; withdrawn now, wallet USDC would be $877.85. Their USDC side alone would cover the $171.23 shortfall at ETH about $1830.74 (+9.1% from $1677.54), each range on its own pool's curve, with about 0.1933 WETH still in them there.",
        "tier": "top",
        "title": "WALLET USDC BELOW THE STANDING RESERVE",
        "what_changed": "Wallet USDC alone is below your standing reserve.",
        "would_sign": null
      },
      {
        "card": "directive",
        "deep_link": "#event-directive",
        "id": "directive",
        "kind": "directive",
        "links": [],
        "numbers": {},
        "since": "2026-11-21T16:10:35.795544312+00:00",
        "source": "derived_metrics.global_status.shot_advice_action",
        "text": "Total NOCK held is 108693 (+8693 above the 100000 floor). 2 NOCK LP positions tracked: 1 ranged (1 in range), 1 full range. +2 ETH/USDC ranges, measured only (2 in range). +3 ladders, measured only (3 holding only USDC). $3.02 of LP fees uncollected. Liquid dry powder: $589.61 (wallet USDC $778.77 less the larger of the $189.15 cash target and the $8.70 virtual tax reserve). The $950.00 protected cash reserve is measured on wallet USDC plus the $233.87 of USDC in NOCK LP and the $768.24 in ladders: $1780.87, $830.87 above it. The $1002.11 of USDC inside NOCK LP and ladders ($768.24 of it in ladders) is working capital, not dry powder: it stays in its ranges. Standing bias: NOCK LP holds $485.76 against a $756.61 target (20% of portfolio), $270.85 short. Re-establishing ranges from NOCK and USDC already held is a reposition, not a purchase: the stable total is unchanged, only its location moves. No tracked position is eligible for a deposit right now.",
        "tier": "quiet",
        "title": "HOLD & ACCRETE (Dual Growth Engine Active)",
        "what_changed": "The directive is HOLD & ACCRETE (Dual Growth Engine Active).",
        "would_sign": null
      },
      {
        "card": "ladders",
        "deep_link": "#event-ladder-cap-sold-uniswap-v4-base-3244699",
        "id": "ladder_cap_sold:uniswap-v4:base:3244699",
        "kind": "ladder_cap_sold",
        "links": [],
        "numbers": {
          "fees_earned_usd": "0.04",
          "nock_now": "0.0000",
          "pool_price_usd": "0.01355744",
          "top_usd": "0.01353085",
          "usdc_now": "27.63"
        },
        "since": "2026-11-21T16:10:35.795544312+00:00",
        "source": "derived_metrics.ladder_maintenance.rungs",
        "text": "Cap #3244699 is out of range above: it sold its NOCK, as a cap does when the price rises through its band, and holds $27.63 USDC ($0.04 of uncollected fees). NockNock does not suggest putting NOCK back into it; a cap is offered only by the ladder suggestion card.",
        "tier": "quiet",
        "title": "LADDER: CAP SOLD ITS NOCK",
        "what_changed": "The price rose above Cap #3244699's band, so it sold its NOCK.",
        "would_sign": null
      },
      {
        "card": "collectible",
        "deep_link": "#event-harvest",
        "id": "harvest",
        "kind": "harvest",
        "links": [],
        "numbers": {
          "claimable_usd": "3.02",
          "gas_per_claim_usd": "0.0035",
          "positions": "3"
        },
        "since": "2026-11-21T16:10:35.795544312+00:00",
        "source": "derived_metrics.velocity_gas_sandbox.contracts",
        "text": "$3.02 of LP fees in 3 positions can be collected for about $0.0035 of gas each. Optional; the BATCH HARVEST directive and its push still follow the $5.00 rule.",
        "tier": "quiet",
        "title": "LP FEES COLLECTABLE",
        "what_changed": "Uncollected LP fees cover their own claim's gas.",
        "would_sign": "One collect transaction per position, in the venue's app or the Lite cockpit's harvest."
      },
      {
        "card": "collectible",
        "deep_link": "#event-aero-rewards",
        "id": "aero_rewards",
        "kind": "aero_rewards",
        "links": [],
        "numbers": {
          "rewards_usd": "1.72"
        },
        "since": "2026-11-21T16:10:35.795544312+00:00",
        "source": "derived_metrics.global_status.collectible_rewards_usd",
        "text": "$1.72 of AERO rewards can be collected in the Aero app. Optional.",
        "tier": "quiet",
        "title": "AERO REWARDS COLLECTABLE",
        "what_changed": "Gauge rewards are waiting in the Aero app.",
        "would_sign": null
      },
      {
        "card": "floor_health",
        "deep_link": "#event-floor-crossing",
        "id": "floor_crossing",
        "kind": "floor_crossing",
        "links": [],
        "numbers": {
          "level_nock": "100000",
          "pct_above_spot": "86.0",
          "price_usd": "0.025221"
        },
        "since": "2026-11-21T16:10:35.795544312+00:00",
        "source": "derived_metrics.floor_breach.crossing",
        "text": "If NOCK rose to $0.025221 (+86.0%), the ranges would have sold the stack down to the 100000 NOCK floor.",
        "tier": "quiet",
        "title": "FLOOR-CROSSING PRICE",
        "what_changed": "The deployed ranges would sell the stack below the floor above a price.",
        "would_sign": null
      }
    ],
    "ladder_maintenance": {
      "gap_limit_seconds": 390,
      "history": "The clocks (out of range since, the last suggestion, the lowest price and the most converted a deep rung saw) are carried from the previous snapshot: in memory from cycle to cycle, and from the latest stored snapshot after a restart. A gap longer than the sync window plus 90 s restarts the out-of-range clock and any standing suggestion; the lowest price and the most converted are kept, with a note that the price was not watched in the gap. No table holds them: a lost database restarts them.",
      "rules": {
        "deep_bottom_report": {
          "invalid": [],
          "needs": [],
          "status": "on",
          "text": "A deep rung fully converted at its bottom is reported; there is no setting."
        },
        "deep_pull": {
          "invalid": [],
          "needs": [
            "operator_profile.ladder_maintenance.deep_pull_fill_fraction",
            "operator_profile.ladder_maintenance.deep_pull_recovery_pct",
            "operator_profile.ladder_maintenance.cooldown_hours",
            "operator_profile.ladder_maintenance.hysteresis_pct"
          ],
          "status": "off",
          "text": "The deep-rung pull rule is off until you set operator_profile.ladder_maintenance.deep_pull_fill_fraction, operator_profile.ladder_maintenance.deep_pull_recovery_pct, operator_profile.ladder_maintenance.cooldown_hours and operator_profile.ladder_maintenance.hysteresis_pct."
        },
        "near_move": {
          "invalid": [],
          "needs": [
            "operator_profile.ladder_maintenance.min_hours_out_of_range",
            "operator_profile.ladder_maintenance.cooldown_hours",
            "operator_profile.ladder_maintenance.hysteresis_pct",
            "operator_profile.ladder_maintenance.max_gas_payback_days"
          ],
          "status": "off",
          "text": "The near-rung move rule is off until you set operator_profile.ladder_maintenance.min_hours_out_of_range, operator_profile.ladder_maintenance.cooldown_hours, operator_profile.ladder_maintenance.hysteresis_pct and operator_profile.ladder_maintenance.max_gas_payback_days."
        }
      },
      "rungs": [
        {
          "bottom_usd": "0.01313098",
          "clock": {
            "above_since": "2026-11-21T16:10:35.795544312+00:00",
            "episode": null,
            "last_cleared_at": null,
            "suggested_since": null
          },
          "event": {
            "kind": "ladder_cap_sold",
            "links": [],
            "numbers": {
              "fees_earned_usd": "0.04",
              "nock_now": "0.0000",
              "pool_price_usd": "0.01355744",
              "top_usd": "0.01353085",
              "usdc_now": "27.63"
            },
            "text": "Cap #3244699 is out of range above: it sold its NOCK, as a cap does when the price rises through its band, and holds $27.63 USDC ($0.04 of uncollected fees). NockNock does not suggest putting NOCK back into it; a cap is offered only by the ladder suggestion card.",
            "tier": "quiet",
            "title": "LADDER: CAP SOLD ITS NOCK",
            "what_changed": "The price rose above Cap #3244699's band, so it sold its NOCK.",
            "would_sign": null
          },
          "fees_earned_usd": "0.04",
          "gates": [],
          "label": "v4 USDC/NOCK 3.00% #3244699",
          "move": null,
          "nock_now": "0.0000",
          "pool_price_usd": "0.01355744",
          "position_id": "uniswap-v4:base:3244699",
          "pull": null,
          "rule": "cap_statement",
          "rung": "cap",
          "short_label": "Cap #3244699",
          "state": "holds_only_usdc",
          "status": "sold",
          "text": "Cap #3244699 is out of range above: it sold its NOCK, as a cap does when the price rises through its band, and holds $27.63 USDC ($0.04 of uncollected fees). NockNock does not suggest putting NOCK back into it; a cap is offered only by the ladder suggestion card.",
          "tick_lower": 273300,
          "tick_upper": 273600,
          "top_usd": "0.01353085",
          "usdc_now": "27.63"
        },
        {
          "bottom_usd": "0.01096801",
          "clock": {
            "above_since": "2026-11-21T16:10:35.795544312+00:00",
            "episode": null,
            "last_cleared_at": null,
            "suggested_since": null
          },
          "event": null,
          "fees_earned_usd": "0.00",
          "gates": [],
          "label": "v4 USDC/NOCK 3.00% #3244700",
          "move": null,
          "nock_now": "0.0000",
          "pool_price_usd": "0.01355744",
          "position_id": "uniswap-v4:base:3244700",
          "pull": null,
          "rule": "near_move",
          "rung": "near",
          "short_label": "Near rung #3244700",
          "state": "holds_only_usdc",
          "status": "off",
          "text": "Near rung #3244700 has been out of range above since 2026-11-21 16:10 UTC (0.0 h): it holds $111.09 USDC and 0 NOCK and earns nothing until NOCK falls to its top $0.01313. The pool price $0.01356 is 3.25% above that top. The near-rung move rule is off until you set operator_profile.ladder_maintenance.min_hours_out_of_range, operator_profile.ladder_maintenance.cooldown_hours, operator_profile.ladder_maintenance.hysteresis_pct and operator_profile.ladder_maintenance.max_gas_payback_days.",
          "tick_lower": 273600,
          "tick_upper": 275400,
          "top_usd": "0.01313098",
          "usdc_now": "111.09"
        },
        {
          "bottom_usd": "0.01002405",
          "clock": {
            "above_since": "2026-11-21T16:10:35.795544312+00:00",
            "episode": null,
            "last_cleared_at": null,
            "suggested_since": null
          },
          "event": null,
          "fees_earned_usd": "0.00",
          "gates": [],
          "label": "v4 USDC/NOCK 3.00% #3244701",
          "move": null,
          "nock_now": "0.0000",
          "pool_price_usd": "0.01355744",
          "position_id": "uniswap-v4:base:3244701",
          "pull": null,
          "rule": "deep_pull",
          "rung": "deep",
          "short_label": "Deep rung #3244701",
          "state": "holds_only_usdc",
          "status": "waiting_for_dip",
          "text": "Deep rung #3244701 holds only USDC ($629.52): the price is above it, so it waits for NOCK to fall to its top $0.01097. It is never moved up or chased; nothing to do.",
          "tick_lower": 275400,
          "tick_upper": 276300,
          "top_usd": "0.01096801",
          "usdc_now": "629.52"
        }
      ],
      "settings": {
        "cooldown_hours": null,
        "deep_pull_fill_fraction": null,
        "deep_pull_recovery_pct": null,
        "hysteresis_pct": null,
        "max_gas_payback_days": null,
        "min_hours_out_of_range": null
      },
      "settings_key": "operator_profile.ladder_maintenance",
      "statement": "Measurements against your own restraint settings (operator_profile.ladder_maintenance, no defaults). NockNock builds no transaction and signs and sends nothing; whether and when is your decision.",
      "watched_since_previous": false
    }
  }
} as unknown as EventsFixture;

/** near_move: the near rung 4% above its top after 25 h, every gate passed (the operator's chosen settings 24/24/2/30/0.5/10) */
export const EVENTS_NEAR_MOVE: EventsFixture = {
  "derived_metrics": {
    "events": [
      {
        "card": "reserve",
        "deep_link": "#event-reserve-below",
        "id": "reserve_below",
        "kind": "reserve_below",
        "links": [],
        "numbers": {
          "eth_covering_pct_above_now": "9.1",
          "eth_price_covering_shortfall_usd": "1830.74",
          "eth_price_usd": "1677.54",
          "eth_ranges_cover_shortfall": false,
          "eth_ranges_usdc": "99.09",
          "eth_ranges_weth": "0.2345",
          "eth_ranges_weth_at_covering": "0.1933",
          "ladder_usdc": "768.24",
          "nock_lp_usdc": "233.86",
          "reserve_usdc": "950.00",
          "stable_headroom_usdc": "830.87",
          "stable_usdc": "1780.87",
          "wallet_shortfall_usdc": "171.23",
          "wallet_usdc": "778.77",
          "wallet_usdc_if_eth_ranges_withdrawn": "877.85"
        },
        "since": "2026-11-22T17:30:00+00:00",
        "source": "derived_metrics.global_status",
        "text": "Wallet USDC $778.77 is $171.23 below your $950.00 reserve. The stable balance the reserve gate counts is $1780.87 (wallet USDC $778.77 + NOCK LP USDC $233.86 + ladder USDC $768.24), $830.87 above it. The gas-side ETH/USDC ranges (outside the reserve gate) hold $99.09 USDC and 0.2345 WETH; withdrawn now, wallet USDC would be $877.85. Their USDC side alone would cover the $171.23 shortfall at ETH about $1830.74 (+9.1% from $1677.54), each range on its own pool's curve, with about 0.1933 WETH still in them there.",
        "tier": "top",
        "title": "WALLET USDC BELOW THE STANDING RESERVE",
        "what_changed": "Wallet USDC alone is below your standing reserve.",
        "would_sign": null
      },
      {
        "card": "ladders",
        "deep_link": "#event-ladder-near-move-uniswap-v4-base-3244700",
        "id": "ladder_near_move:uniswap-v4:base:3244700",
        "kind": "ladder_near_move",
        "links": [
          {
            "label": "This rung in the Uniswap app (remove)",
            "url": "https://app.uniswap.org/positions/v4/base/3244700"
          },
          {
            "label": "The moved range in the Uniswap app (new position, no amount prefilled)",
            "url": "https://app.uniswap.org/positions/add/new?chain=base&protocolVersion=v4&currencyA=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913&currencyB=0x9B5E262cF9bb04869ab40b19AF91D2dc85761722&fee=%7B%22feeAmount%22%3A30000%2C%22isDynamic%22%3Afalse%2C%22tickSpacing%22%3A300%7D&priceRangeState=%7B%22fullRange%22%3Afalse%2C%22maxTick%22%3A275100%2C%22minTick%22%3A273300%2C%22priceInverted%22%3Afalse%7D&step=1"
          }
        ],
        "numbers": {
          "fees_earned_usd": "0.00",
          "fees_per_day_in_range_usd": "6.4921",
          "gas_usd": "0.0070",
          "hours_out_of_range": "25.3",
          "new_bottom_usd": "0.01130202",
          "new_tick_lower": "273300",
          "new_tick_upper": "275100",
          "new_top_usd": "0.01353085",
          "payback_days_in_range": "0.0011",
          "pct_above_top": "4.00",
          "pool_price_usd": "0.01365622",
          "top_usd": "0.01313098",
          "usdc": "111.09",
          "width_ticks": "1800"
        },
        "since": "2026-11-22T17:30:00+00:00",
        "source": "derived_metrics.ladder_maintenance.rungs",
        "text": "Near rung #3244700 has been out of range above since 2026-11-21 16:10 UTC (25.3 h): it holds $111.09 USDC and 0 NOCK and earns nothing until NOCK falls to its top $0.01313. The pool price $0.01366 is 4.00% above that top. A move to just below the price, keeping its width (1,800 ticks), would be ticks 273300..275100, $0.01130 to $0.01353, with the same $111.09 USDC and 0 NOCK; it would open 0.92% below the price, so it earns fees only once NOCK dips into it. $0.01097 to $0.01130 is then covered by no rung (between the deep rung's top and the moved range's bottom). It has earned $0.00 in fees so far. Out of range above: 25.3 h of your 24 h; hysteresis: 4.00% above its top, your 2%; cooldown: 26.0 h since its deposit of your 24 h; gas against fees: ≈$0.0070 of gas (two transactions, an estimate) against ≈$6.4921 a day of fees while in range at the pool's measured rate, repaid in 0.0011 days in range, your 30. Whether and when is your decision.",
        "tier": "top",
        "title": "LADDER: MOVE THE NEAR RUNG",
        "what_changed": "Near rung #3244700 has been out of range above past every gate you set.",
        "would_sign": "Two transactions in your wallet: remove all of Near rung #3244700's liquidity (its fees come with it), then mint ticks 273300..275100 in the same pool with $111.09 USDC and 0 NOCK. NockNock builds neither and signs nothing; the new position is found on-chain after it lands."
      },
      {
        "card": "ladders",
        "deep_link": "#event-ladder-rule-cannot-fire-uniswap-v4-base-3244701",
        "id": "ladder_rule_cannot_fire:uniswap-v4:base:3244701",
        "kind": "ladder_rule_cannot_fire",
        "links": [],
        "numbers": {
          "bottom_usd": "0.01002405",
          "deep_pull_recovery_pct": "10",
          "low_usd": "0.01002405",
          "recovery_point_usd": "0.01102646",
          "rung_width_pct": "9.42",
          "top_usd": "0.01096801"
        },
        "position_id": "uniswap-v4:base:3244701",
        "setting": "operator_profile.ladder_maintenance.deep_pull_recovery_pct",
        "since": "2026-11-22T17:30:00+00:00",
        "source": "derived_metrics.ladder_maintenance.rungs",
        "text": "Deep rung #3244701: the deep-rung pull cannot fire for this rung with your operator_profile.ladder_maintenance.deep_pull_recovery_pct of 10%. The rung is 9.42% wide (its top $0.01097 is 9.42% above its bottom $0.01002), so a 10% recovery from its bottom, $0.01002 lands at $0.01103, at or above its top, and by then the rung has sold all the NOCK it bought back for USDC. A recovery below 9.42% would let the pull be stated before that.",
        "tier": "top",
        "title": "LADDER: DEEP PULL CANNOT FIRE",
        "what_changed": "Your deep_pull_recovery_pct is at or above Deep rung #3244701's width.",
        "would_sign": null
      },
      {
        "card": "directive",
        "deep_link": "#event-directive",
        "id": "directive",
        "kind": "directive",
        "links": [],
        "numbers": {},
        "since": "2026-11-22T17:30:00+00:00",
        "source": "derived_metrics.global_status.shot_advice_action",
        "text": "Total NOCK held is 108693 (+8693 above the 100000 floor). 2 NOCK LP positions tracked: 1 ranged (1 in range), 1 full range. +2 ETH/USDC ranges, measured only (2 in range). +3 ladders, measured only (3 holding only USDC). $3.02 of LP fees uncollected. Liquid dry powder: $589.61 (wallet USDC $778.77 less the larger of the $189.15 cash target and the $8.70 virtual tax reserve). The $950.00 protected cash reserve is measured on wallet USDC plus the $233.87 of USDC in NOCK LP and the $768.24 in ladders: $1780.87, $830.87 above it. The $1002.11 of USDC inside NOCK LP and ladders ($768.24 of it in ladders) is working capital, not dry powder: it stays in its ranges. Standing bias: NOCK LP holds $485.76 against a $756.61 target (20% of portfolio), $270.85 short. Re-establishing ranges from NOCK and USDC already held is a reposition, not a purchase: the stable total is unchanged, only its location moves. No tracked position is eligible for a deposit right now.",
        "tier": "quiet",
        "title": "HOLD & ACCRETE (Dual Growth Engine Active)",
        "what_changed": "The directive is HOLD & ACCRETE (Dual Growth Engine Active).",
        "would_sign": null
      },
      {
        "card": "ladders",
        "deep_link": "#event-ladder-cap-sold-uniswap-v4-base-3244699",
        "id": "ladder_cap_sold:uniswap-v4:base:3244699",
        "kind": "ladder_cap_sold",
        "links": [],
        "numbers": {
          "fees_earned_usd": "0.04",
          "nock_now": "0.0000",
          "pool_price_usd": "0.01365622",
          "top_usd": "0.01353085",
          "usdc_now": "27.63"
        },
        "since": "2026-11-22T17:30:00+00:00",
        "source": "derived_metrics.ladder_maintenance.rungs",
        "text": "Cap #3244699 is out of range above: it sold its NOCK, as a cap does when the price rises through its band, and holds $27.63 USDC ($0.04 of uncollected fees). NockNock does not suggest putting NOCK back into it; a cap is offered only by the ladder suggestion card.",
        "tier": "quiet",
        "title": "LADDER: CAP SOLD ITS NOCK",
        "what_changed": "The price rose above Cap #3244699's band, so it sold its NOCK.",
        "would_sign": null
      },
      {
        "card": "collectible",
        "deep_link": "#event-harvest",
        "id": "harvest",
        "kind": "harvest",
        "links": [],
        "numbers": {
          "claimable_usd": "3.02",
          "gas_per_claim_usd": "0.0035",
          "positions": "3"
        },
        "since": "2026-11-22T17:30:00+00:00",
        "source": "derived_metrics.velocity_gas_sandbox.contracts",
        "text": "$3.02 of LP fees in 3 positions can be collected for about $0.0035 of gas each. Optional; the BATCH HARVEST directive and its push still follow the $5.00 rule.",
        "tier": "quiet",
        "title": "LP FEES COLLECTABLE",
        "what_changed": "Uncollected LP fees cover their own claim's gas.",
        "would_sign": "One collect transaction per position, in the venue's app or the Lite cockpit's harvest."
      },
      {
        "card": "collectible",
        "deep_link": "#event-aero-rewards",
        "id": "aero_rewards",
        "kind": "aero_rewards",
        "links": [],
        "numbers": {
          "rewards_usd": "1.72"
        },
        "since": "2026-11-22T17:30:00+00:00",
        "source": "derived_metrics.global_status.collectible_rewards_usd",
        "text": "$1.72 of AERO rewards can be collected in the Aero app. Optional.",
        "tier": "quiet",
        "title": "AERO REWARDS COLLECTABLE",
        "what_changed": "Gauge rewards are waiting in the Aero app.",
        "would_sign": null
      },
      {
        "card": "floor_health",
        "deep_link": "#event-floor-crossing",
        "id": "floor_crossing",
        "kind": "floor_crossing",
        "links": [],
        "numbers": {
          "level_nock": "100000",
          "pct_above_spot": "86.0",
          "price_usd": "0.025221"
        },
        "since": "2026-11-22T17:30:00+00:00",
        "source": "derived_metrics.floor_breach.crossing",
        "text": "If NOCK rose to $0.025221 (+86.0%), the ranges would have sold the stack down to the 100000 NOCK floor.",
        "tier": "quiet",
        "title": "FLOOR-CROSSING PRICE",
        "what_changed": "The deployed ranges would sell the stack below the floor above a price.",
        "would_sign": null
      }
    ],
    "ladder_maintenance": {
      "gap_limit_seconds": 390,
      "history": "The clocks (out of range since, the last suggestion, the lowest price and the most converted a deep rung saw) are carried from the previous snapshot: in memory from cycle to cycle, and from the latest stored snapshot after a restart. A gap longer than the sync window plus 90 s restarts the out-of-range clock and any standing suggestion; the lowest price and the most converted are kept, with a note that the price was not watched in the gap. No table holds them: a lost database restarts them.",
      "rules": {
        "deep_bottom_report": {
          "invalid": [],
          "needs": [],
          "status": "on",
          "text": "A deep rung fully converted at its bottom is reported; there is no setting."
        },
        "deep_pull": {
          "invalid": [],
          "needs": [],
          "status": "on",
          "text": "The deep-rung pull rule is on, with your settings."
        },
        "near_move": {
          "invalid": [],
          "needs": [],
          "status": "on",
          "text": "The near-rung move rule is on, with your settings."
        }
      },
      "rungs": [
        {
          "bottom_usd": "0.01313098",
          "clock": {
            "above_since": "2026-11-22T17:30:00+00:00",
            "episode": null,
            "last_cleared_at": null,
            "suggested_since": null
          },
          "event": {
            "kind": "ladder_cap_sold",
            "links": [],
            "numbers": {
              "fees_earned_usd": "0.04",
              "nock_now": "0.0000",
              "pool_price_usd": "0.01365622",
              "top_usd": "0.01353085",
              "usdc_now": "27.63"
            },
            "text": "Cap #3244699 is out of range above: it sold its NOCK, as a cap does when the price rises through its band, and holds $27.63 USDC ($0.04 of uncollected fees). NockNock does not suggest putting NOCK back into it; a cap is offered only by the ladder suggestion card.",
            "tier": "quiet",
            "title": "LADDER: CAP SOLD ITS NOCK",
            "what_changed": "The price rose above Cap #3244699's band, so it sold its NOCK.",
            "would_sign": null
          },
          "fees_earned_usd": "0.04",
          "gates": [],
          "label": "v4 USDC/NOCK 3.00% #3244699",
          "move": null,
          "nock_now": "0.0000",
          "pool_price_usd": "0.01365622",
          "position_id": "uniswap-v4:base:3244699",
          "pull": null,
          "rule": "cap_statement",
          "rung": "cap",
          "short_label": "Cap #3244699",
          "state": "holds_only_usdc",
          "status": "sold",
          "text": "Cap #3244699 is out of range above: it sold its NOCK, as a cap does when the price rises through its band, and holds $27.63 USDC ($0.04 of uncollected fees). NockNock does not suggest putting NOCK back into it; a cap is offered only by the ladder suggestion card.",
          "tick_lower": 273300,
          "tick_upper": 273600,
          "top_usd": "0.01353085",
          "usdc_now": "27.63"
        },
        {
          "bottom_usd": "0.01096801",
          "clock": {
            "above_since": "2026-11-21T16:10:35+00:00",
            "episode": null,
            "last_cleared_at": null,
            "suggested_since": "2026-11-22T17:30:00+00:00"
          },
          "event": {
            "kind": "ladder_near_move",
            "links": [
              {
                "label": "This rung in the Uniswap app (remove)",
                "url": "https://app.uniswap.org/positions/v4/base/3244700"
              },
              {
                "label": "The moved range in the Uniswap app (new position, no amount prefilled)",
                "url": "https://app.uniswap.org/positions/add/new?chain=base&protocolVersion=v4&currencyA=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913&currencyB=0x9B5E262cF9bb04869ab40b19AF91D2dc85761722&fee=%7B%22feeAmount%22%3A30000%2C%22isDynamic%22%3Afalse%2C%22tickSpacing%22%3A300%7D&priceRangeState=%7B%22fullRange%22%3Afalse%2C%22maxTick%22%3A275100%2C%22minTick%22%3A273300%2C%22priceInverted%22%3Afalse%7D&step=1"
              }
            ],
            "numbers": {
              "fees_earned_usd": "0.00",
              "fees_per_day_in_range_usd": "6.4921",
              "gas_usd": "0.0070",
              "hours_out_of_range": "25.3",
              "new_bottom_usd": "0.01130202",
              "new_tick_lower": "273300",
              "new_tick_upper": "275100",
              "new_top_usd": "0.01353085",
              "payback_days_in_range": "0.0011",
              "pct_above_top": "4.00",
              "pool_price_usd": "0.01365622",
              "top_usd": "0.01313098",
              "usdc": "111.09",
              "width_ticks": "1800"
            },
            "since": "2026-11-22T17:30:00+00:00",
            "text": "Near rung #3244700 has been out of range above since 2026-11-21 16:10 UTC (25.3 h): it holds $111.09 USDC and 0 NOCK and earns nothing until NOCK falls to its top $0.01313. The pool price $0.01366 is 4.00% above that top. A move to just below the price, keeping its width (1,800 ticks), would be ticks 273300..275100, $0.01130 to $0.01353, with the same $111.09 USDC and 0 NOCK; it would open 0.92% below the price, so it earns fees only once NOCK dips into it. $0.01097 to $0.01130 is then covered by no rung (between the deep rung's top and the moved range's bottom). It has earned $0.00 in fees so far. Out of range above: 25.3 h of your 24 h; hysteresis: 4.00% above its top, your 2%; cooldown: 26.0 h since its deposit of your 24 h; gas against fees: ≈$0.0070 of gas (two transactions, an estimate) against ≈$6.4921 a day of fees while in range at the pool's measured rate, repaid in 0.0011 days in range, your 30. Whether and when is your decision.",
            "tier": "top",
            "title": "LADDER: MOVE THE NEAR RUNG",
            "what_changed": "Near rung #3244700 has been out of range above past every gate you set.",
            "would_sign": "Two transactions in your wallet: remove all of Near rung #3244700's liquidity (its fees come with it), then mint ticks 273300..275100 in the same pool with $111.09 USDC and 0 NOCK. NockNock builds neither and signs nothing; the new position is found on-chain after it lands."
          },
          "fees_earned_usd": "0.00",
          "gates": [
            {
              "gate": "min_hours_out_of_range",
              "measured": "25.3",
              "passes": true,
              "setting": "24",
              "text": "out of range above: 25.3 h of your 24 h"
            },
            {
              "gate": "hysteresis_pct",
              "measured": "4.00",
              "passes": true,
              "setting": "2",
              "text": "hysteresis: 4.00% above its top, your 2%"
            },
            {
              "gate": "cooldown_hours",
              "measured": "26.0",
              "passes": true,
              "setting": "24",
              "text": "cooldown: 26.0 h since its deposit of your 24 h"
            },
            {
              "gate": "max_gas_payback_days",
              "measured": "0.0011",
              "passes": true,
              "setting": "30",
              "text": "gas against fees: ≈$0.0070 of gas (two transactions, an estimate) against ≈$6.4921 a day of fees while in range at the pool's measured rate, repaid in 0.0011 days in range, your 30"
            }
          ],
          "label": "v4 USDC/NOCK 3.00% #3244700",
          "move": {
            "deep_top_usd": "0.01096801",
            "fee_rate_from": "v4 USDC/NOCK 3.00% #3163450",
            "fee_rate_reason": null,
            "fees_earned_usd": "0.00",
            "fees_per_day_in_range_usd": "6.4921",
            "gas_basis": "two transactions (remove the rung, then mint the moved range) at the worker's per-transaction estimate (150,000 gas plus the Base L1 buffer); a v4 mint can use more",
            "gas_usd": "0.0070",
            "liquidity_raw": "1109653223698976",
            "new_bottom_usd": "0.01130202",
            "new_position_url": "https://app.uniswap.org/positions/add/new?chain=base&protocolVersion=v4&currencyA=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913&currencyB=0x9B5E262cF9bb04869ab40b19AF91D2dc85761722&fee=%7B%22feeAmount%22%3A30000%2C%22isDynamic%22%3Afalse%2C%22tickSpacing%22%3A300%7D&priceRangeState=%7B%22fullRange%22%3Afalse%2C%22maxTick%22%3A275100%2C%22minTick%22%3A273300%2C%22priceInverted%22%3Afalse%7D&step=1",
            "new_tick_lower": 273300,
            "new_tick_upper": 275100,
            "new_top_usd": "0.01353085",
            "nock": "0",
            "opens_pct_of_price": "-0.92",
            "payback_days_in_range": "0.0011",
            "remove_url": "https://app.uniswap.org/positions/v4/base/3244700",
            "uncovered_from_usd": "0.01096801",
            "uncovered_to_usd": "0.01130202",
            "usdc": "111.09",
            "width_ticks": 1800
          },
          "nock_now": "0.0000",
          "pool_price_usd": "0.01365622",
          "position_id": "uniswap-v4:base:3244700",
          "pull": null,
          "rule": "near_move",
          "rung": "near",
          "short_label": "Near rung #3244700",
          "state": "holds_only_usdc",
          "status": "suggested",
          "text": "Near rung #3244700 has been out of range above since 2026-11-21 16:10 UTC (25.3 h): it holds $111.09 USDC and 0 NOCK and earns nothing until NOCK falls to its top $0.01313. The pool price $0.01366 is 4.00% above that top. A move to just below the price, keeping its width (1,800 ticks), would be ticks 273300..275100, $0.01130 to $0.01353, with the same $111.09 USDC and 0 NOCK; it would open 0.92% below the price, so it earns fees only once NOCK dips into it. $0.01097 to $0.01130 is then covered by no rung (between the deep rung's top and the moved range's bottom). It has earned $0.00 in fees so far. Out of range above: 25.3 h of your 24 h; hysteresis: 4.00% above its top, your 2%; cooldown: 26.0 h since its deposit of your 24 h; gas against fees: ≈$0.0070 of gas (two transactions, an estimate) against ≈$6.4921 a day of fees while in range at the pool's measured rate, repaid in 0.0011 days in range, your 30. Whether and when is your decision.",
          "tick_lower": 273600,
          "tick_upper": 275400,
          "top_usd": "0.01313098",
          "usdc_now": "111.09"
        },
        {
          "bottom_usd": "0.01002405",
          "clock": {
            "above_since": "2026-11-22T17:30:00+00:00",
            "episode": null,
            "last_cleared_at": null,
            "suggested_since": null
          },
          "event": null,
          "fees_earned_usd": "0.00",
          "gates": [],
          "label": "v4 USDC/NOCK 3.00% #3244701",
          "move": null,
          "nock_now": "0.0000",
          "pool_price_usd": "0.01365622",
          "position_id": "uniswap-v4:base:3244701",
          "pull": null,
          "rule": "deep_pull",
          "rung": "deep",
          "setting_event": {
            "kind": "ladder_rule_cannot_fire",
            "links": [],
            "numbers": {
              "bottom_usd": "0.01002405",
              "deep_pull_recovery_pct": "10",
              "low_usd": "0.01002405",
              "recovery_point_usd": "0.01102646",
              "rung_width_pct": "9.42",
              "top_usd": "0.01096801"
            },
            "position_id": "uniswap-v4:base:3244701",
            "setting": "operator_profile.ladder_maintenance.deep_pull_recovery_pct",
            "text": "Deep rung #3244701: the deep-rung pull cannot fire for this rung with your operator_profile.ladder_maintenance.deep_pull_recovery_pct of 10%. The rung is 9.42% wide (its top $0.01097 is 9.42% above its bottom $0.01002), so a 10% recovery from its bottom, $0.01002 lands at $0.01103, at or above its top, and by then the rung has sold all the NOCK it bought back for USDC. A recovery below 9.42% would let the pull be stated before that.",
            "tier": "top",
            "title": "LADDER: DEEP PULL CANNOT FIRE",
            "what_changed": "Your deep_pull_recovery_pct is at or above Deep rung #3244701's width.",
            "would_sign": null
          },
          "short_label": "Deep rung #3244701",
          "state": "holds_only_usdc",
          "status": "rule_cannot_fire",
          "text": "Deep rung #3244701: the deep-rung pull cannot fire for this rung with your operator_profile.ladder_maintenance.deep_pull_recovery_pct of 10%. The rung is 9.42% wide (its top $0.01097 is 9.42% above its bottom $0.01002), so a 10% recovery from its bottom, $0.01002 lands at $0.01103, at or above its top, and by then the rung has sold all the NOCK it bought back for USDC. A recovery below 9.42% would let the pull be stated before that. This cycle: Deep rung #3244701 holds only USDC ($629.52): the price is above it, so it waits for NOCK to fall to its top $0.01097. It is never moved up or chased; nothing to do.",
          "tick_lower": 275400,
          "tick_upper": 276300,
          "top_usd": "0.01096801",
          "usdc_now": "629.52"
        }
      ],
      "settings": {
        "cooldown_hours": "24",
        "deep_pull_fill_fraction": "0.5",
        "deep_pull_recovery_pct": "10",
        "hysteresis_pct": "2",
        "max_gas_payback_days": "30",
        "min_hours_out_of_range": "24"
      },
      "settings_key": "operator_profile.ladder_maintenance",
      "statement": "Measurements against your own restraint settings (operator_profile.ladder_maintenance, no defaults). NockNock builds no transaction and signs and sends nothing; whether and when is your decision.",
      "watched_since_previous": true
    }
  }
} as unknown as EventsFixture;

/** deep_converted_stale: every rung below its range: the deep rung fully converted */
export const EVENTS_DEEP_CONVERTED: EventsFixture = {
  "derived_metrics": {
    "events": [
      {
        "card": "reserve",
        "deep_link": "#event-reserve-below",
        "id": "reserve_below",
        "kind": "reserve_below",
        "links": [],
        "numbers": {
          "eth_covering_pct_above_now": "9.1",
          "eth_price_covering_shortfall_usd": "1830.74",
          "eth_price_usd": "1677.54",
          "eth_ranges_cover_shortfall": false,
          "eth_ranges_usdc": "99.09",
          "eth_ranges_weth": "0.2345",
          "eth_ranges_weth_at_covering": "0.1933",
          "ladder_usdc": "768.24",
          "nock_lp_usdc": "233.86",
          "reserve_usdc": "950.00",
          "stable_headroom_usdc": "830.87",
          "stable_usdc": "1780.87",
          "wallet_shortfall_usdc": "171.23",
          "wallet_usdc": "778.77",
          "wallet_usdc_if_eth_ranges_withdrawn": "877.85"
        },
        "since": "2026-11-21T16:10:35.795544312+00:00",
        "source": "derived_metrics.global_status",
        "text": "Wallet USDC $778.77 is $171.23 below your $950.00 reserve. The stable balance the reserve gate counts is $1780.87 (wallet USDC $778.77 + NOCK LP USDC $233.86 + ladder USDC $768.24), $830.87 above it. The gas-side ETH/USDC ranges (outside the reserve gate) hold $99.09 USDC and 0.2345 WETH; withdrawn now, wallet USDC would be $877.85. Their USDC side alone would cover the $171.23 shortfall at ETH about $1830.74 (+9.1% from $1677.54), each range on its own pool's curve, with about 0.1933 WETH still in them there.",
        "tier": "top",
        "title": "WALLET USDC BELOW THE STANDING RESERVE",
        "what_changed": "Wallet USDC alone is below your standing reserve.",
        "would_sign": null
      },
      {
        "card": "ladders",
        "deep_link": "#event-ladder-deep-converted-uniswap-v4-base-3244701",
        "id": "ladder_deep_converted:uniswap-v4:base:3244701",
        "kind": "ladder_deep_converted",
        "links": [],
        "numbers": {
          "avg_buy_price_usd": "0.01048542",
          "bottom_usd": "0.01002405",
          "nock_now": "60037.6735",
          "pool_price_usd": "0.00990000",
          "top_usd": "0.01096801",
          "usdc_now": "0.00"
        },
        "since": "2026-11-21T16:10:35.795544312+00:00",
        "source": "derived_metrics.ladder_maintenance.rungs",
        "text": "Deep rung #3244701 is fully converted: the price $0.009900 is at or below its bottom $0.01002, and it holds 60,038 NOCK bought between $0.01097 and $0.01002, an average of $0.01049, and no USDC. NockNock does not suggest a lower ladder: the reserve kept below the ladder's bottom stays where it is, and the rung waits.",
        "tier": "top",
        "title": "LADDER: DEEP RUNG FULLY CONVERTED",
        "what_changed": "The price reached Deep rung #3244701's bottom: all of its USDC is NOCK now.",
        "would_sign": null
      },
      {
        "card": "directive",
        "deep_link": "#event-directive",
        "id": "directive",
        "kind": "directive",
        "links": [],
        "numbers": {},
        "since": "2026-11-21T16:10:35.795544312+00:00",
        "source": "derived_metrics.global_status.shot_advice_action",
        "text": "Total NOCK held is 108693 (+8693 above the 100000 floor). 2 NOCK LP positions tracked: 1 ranged (1 in range), 1 full range. +2 ETH/USDC ranges, measured only (2 in range). +3 ladders, measured only (3 holding only USDC). $3.02 of LP fees uncollected. Liquid dry powder: $589.61 (wallet USDC $778.77 less the larger of the $189.15 cash target and the $8.70 virtual tax reserve). The $950.00 protected cash reserve is measured on wallet USDC plus the $233.87 of USDC in NOCK LP and the $768.24 in ladders: $1780.87, $830.87 above it. The $1002.11 of USDC inside NOCK LP and ladders ($768.24 of it in ladders) is working capital, not dry powder: it stays in its ranges. Standing bias: NOCK LP holds $485.76 against a $756.61 target (20% of portfolio), $270.85 short. Re-establishing ranges from NOCK and USDC already held is a reposition, not a purchase: the stable total is unchanged, only its location moves. No tracked position is eligible for a deposit right now.",
        "tier": "quiet",
        "title": "HOLD & ACCRETE (Dual Growth Engine Active)",
        "what_changed": "The directive is HOLD & ACCRETE (Dual Growth Engine Active).",
        "would_sign": null
      },
      {
        "card": "collectible",
        "deep_link": "#event-harvest",
        "id": "harvest",
        "kind": "harvest",
        "links": [],
        "numbers": {
          "claimable_usd": "3.02",
          "gas_per_claim_usd": "0.0035",
          "positions": "3"
        },
        "since": "2026-11-21T16:10:35.795544312+00:00",
        "source": "derived_metrics.velocity_gas_sandbox.contracts",
        "text": "$3.02 of LP fees in 3 positions can be collected for about $0.0035 of gas each. Optional; the BATCH HARVEST directive and its push still follow the $5.00 rule.",
        "tier": "quiet",
        "title": "LP FEES COLLECTABLE",
        "what_changed": "Uncollected LP fees cover their own claim's gas.",
        "would_sign": "One collect transaction per position, in the venue's app or the Lite cockpit's harvest."
      },
      {
        "card": "collectible",
        "deep_link": "#event-aero-rewards",
        "id": "aero_rewards",
        "kind": "aero_rewards",
        "links": [],
        "numbers": {
          "rewards_usd": "1.72"
        },
        "since": "2026-11-21T16:10:35.795544312+00:00",
        "source": "derived_metrics.global_status.collectible_rewards_usd",
        "text": "$1.72 of AERO rewards can be collected in the Aero app. Optional.",
        "tier": "quiet",
        "title": "AERO REWARDS COLLECTABLE",
        "what_changed": "Gauge rewards are waiting in the Aero app.",
        "would_sign": null
      },
      {
        "card": "floor_health",
        "deep_link": "#event-floor-crossing",
        "id": "floor_crossing",
        "kind": "floor_crossing",
        "links": [],
        "numbers": {
          "level_nock": "100000",
          "pct_above_spot": "86.0",
          "price_usd": "0.025221"
        },
        "since": "2026-11-21T16:10:35.795544312+00:00",
        "source": "derived_metrics.floor_breach.crossing",
        "text": "If NOCK rose to $0.025221 (+86.0%), the ranges would have sold the stack down to the 100000 NOCK floor.",
        "tier": "quiet",
        "title": "FLOOR-CROSSING PRICE",
        "what_changed": "The deployed ranges would sell the stack below the floor above a price.",
        "would_sign": null
      }
    ],
    "ladder_maintenance": {
      "gap_limit_seconds": 390,
      "history": "The clocks (out of range since, the last suggestion, the lowest price and the most converted a deep rung saw) are carried from the previous snapshot: in memory from cycle to cycle, and from the latest stored snapshot after a restart. A gap longer than the sync window plus 90 s restarts the out-of-range clock and any standing suggestion; the lowest price and the most converted are kept, with a note that the price was not watched in the gap. No table holds them: a lost database restarts them.",
      "rules": {
        "deep_bottom_report": {
          "invalid": [],
          "needs": [],
          "status": "on",
          "text": "A deep rung fully converted at its bottom is reported; there is no setting."
        },
        "deep_pull": {
          "invalid": [],
          "needs": [],
          "status": "on",
          "text": "The deep-rung pull rule is on, with your settings."
        },
        "near_move": {
          "invalid": [],
          "needs": [],
          "status": "on",
          "text": "The near-rung move rule is on, with your settings."
        }
      },
      "rungs": [
        {
          "bottom_usd": "0.01313098",
          "clock": {
            "above_since": null,
            "episode": null,
            "last_cleared_at": null,
            "suggested_since": null
          },
          "event": null,
          "fees_earned_usd": "0.04",
          "gates": [],
          "label": "v4 USDC/NOCK 3.00% #3244699",
          "move": null,
          "nock_now": "2073.1118",
          "pool_price_usd": "0.00990000",
          "position_id": "uniswap-v4:base:3244699",
          "pull": null,
          "rule": "cap_statement",
          "rung": "cap",
          "short_label": "Cap #3244699",
          "state": "holds_only_nock",
          "status": "holds_nock",
          "text": "Cap #3244699 holds only NOCK (2,073 NOCK): the price is below its band. Nothing to do.",
          "tick_lower": 273300,
          "tick_upper": 273600,
          "top_usd": "0.01353085",
          "usdc_now": "0.00"
        },
        {
          "bottom_usd": "0.01096801",
          "clock": {
            "above_since": null,
            "episode": null,
            "last_cleared_at": null,
            "suggested_since": null
          },
          "event": null,
          "fees_earned_usd": "0.00",
          "gates": [],
          "label": "v4 USDC/NOCK 3.00% #3244700",
          "move": null,
          "nock_now": "9256.8338",
          "pool_price_usd": "0.00990000",
          "position_id": "uniswap-v4:base:3244700",
          "pull": null,
          "rule": "near_move",
          "rung": "near",
          "short_label": "Near rung #3244700",
          "state": "holds_only_nock",
          "status": "holds_nock",
          "text": "Near rung #3244700 holds only NOCK (9,257 NOCK): it bought on the way down to its bottom $0.01097. A rebound into it sells that NOCK back for USDC (the scalp). Nothing to do.",
          "tick_lower": 273600,
          "tick_upper": 275400,
          "top_usd": "0.01313098",
          "usdc_now": "0.00"
        },
        {
          "bottom_usd": "0.01002405",
          "clock": {
            "above_since": null,
            "episode": {
              "gap_note": null,
              "low_at": "2026-11-21T16:10:35.795544312+00:00",
              "low_price_usd": "0.00990000",
              "peak_converted_pct": "100.00",
              "peak_nock": "60037.6735",
              "started_at": "2026-11-21T16:10:35.795544312+00:00"
            },
            "last_cleared_at": null,
            "suggested_since": null
          },
          "event": {
            "kind": "ladder_deep_converted",
            "links": [],
            "numbers": {
              "avg_buy_price_usd": "0.01048542",
              "bottom_usd": "0.01002405",
              "nock_now": "60037.6735",
              "pool_price_usd": "0.00990000",
              "top_usd": "0.01096801",
              "usdc_now": "0.00"
            },
            "text": "Deep rung #3244701 is fully converted: the price $0.009900 is at or below its bottom $0.01002, and it holds 60,038 NOCK bought between $0.01097 and $0.01002, an average of $0.01049, and no USDC. NockNock does not suggest a lower ladder: the reserve kept below the ladder's bottom stays where it is, and the rung waits.",
            "tier": "top",
            "title": "LADDER: DEEP RUNG FULLY CONVERTED",
            "what_changed": "The price reached Deep rung #3244701's bottom: all of its USDC is NOCK now.",
            "would_sign": null
          },
          "fees_earned_usd": "0.00",
          "gates": [
            {
              "gate": "deep_pull_fill_fraction",
              "measured": "1.0000",
              "passes": true,
              "setting": "0.5",
              "text": "converted: up to 100.00% of its USDC, your 50%"
            },
            {
              "gate": "deep_pull_recovery_pct",
              "measured": "0.00",
              "passes": false,
              "setting": "10",
              "text": "recovery: the price is 0.00% above the lowest seen, your 10%"
            },
            {
              "gate": "hysteresis_pct",
              "measured": "0.01067220",
              "passes": true,
              "setting": "2",
              "text": "hysteresis: once stated, the pull stands until the price falls below $0.01067 (2% under the recovery point) or makes a new low"
            },
            {
              "gate": "cooldown_hours",
              "measured": "0.7",
              "passes": false,
              "setting": "24",
              "text": "cooldown: 0.7 h since its deposit of your 24 h"
            }
          ],
          "label": "v4 USDC/NOCK 3.00% #3244701",
          "move": null,
          "nock_now": "60037.6735",
          "pool_price_usd": "0.00990000",
          "position_id": "uniswap-v4:base:3244701",
          "pull": {
            "avg_buy_price_usd": "0.01048542",
            "fees_earned_usd": "0.00",
            "gas_basis": "one transaction (remove the rung) at the worker's per-transaction estimate (150,000 gas plus the Base L1 buffer)",
            "gas_usd": "0.0035",
            "low_at": "2026-11-21T16:10:35.795544312+00:00",
            "low_price_usd": "0.00990000",
            "nock_now": "60037.6735",
            "nock_sold_back_since_low": "0.0000",
            "peak_converted_pct": "100.00",
            "peak_nock": "60037.6735",
            "recovery_pct_now": "0.00",
            "recovery_point_usd": "0.01089000",
            "remove_url": "https://app.uniswap.org/positions/v4/base/3244701",
            "usdc_now": "0.00"
          },
          "rule": "deep_pull",
          "rung": "deep",
          "short_label": "Deep rung #3244701",
          "state": "holds_only_nock",
          "status": "fully_converted",
          "text": "Deep rung #3244701 is fully converted: the price $0.009900 is at or below its bottom $0.01002, and it holds 60,038 NOCK bought between $0.01097 and $0.01002, an average of $0.01049, and no USDC. NockNock does not suggest a lower ladder: the reserve kept below the ladder's bottom stays where it is, and the rung waits.",
          "tick_lower": 275400,
          "tick_upper": 276300,
          "top_usd": "0.01096801",
          "usdc_now": "0.00"
        }
      ],
      "settings": {
        "cooldown_hours": "24",
        "deep_pull_fill_fraction": "0.5",
        "deep_pull_recovery_pct": "10",
        "hysteresis_pct": "2",
        "max_gas_payback_days": "30",
        "min_hours_out_of_range": "24"
      },
      "settings_key": "operator_profile.ladder_maintenance",
      "statement": "Measurements against your own restraint settings (operator_profile.ladder_maintenance, no defaults). NockNock builds no transaction and signs and sends nothing; whether and when is your decision.",
      "watched_since_previous": false
    }
  }
} as unknown as EventsFixture;

/** predates: a snapshot from before 2.11.0 */
export const EVENTS_PREDATES: EventsFixture = {
  "derived_metrics": {}
} as unknown as EventsFixture;
