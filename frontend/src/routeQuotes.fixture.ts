/**
 * The route quotes the worker would have put in the snapshot at Base block 58081910
 * (2026-11-16), for the plan's 20 USDC tranche: real quoter answers, assembled by the
 * worker's own code. A copy of
 * ingestion-worker/testdata/quotes/route_quotes_payload_block_58081910.json, which a
 * worker test pins byte for byte. Typed as RouteQuotes, so a field the worker emits and
 * the dashboard's type does not know fails the type check.
 */
import type { RouteQuotes } from './types/snapshot';

export const REAL_ROUTE_QUOTES: RouteQuotes = {
  "status": "ok",
  "detail": null,
  "amount_usdc": "20.00",
  "block_number": 58081910,
  "quoted_at": "2026-11-16T05:25:57+00:00",
  "reference_spot_usd": "0.01128675",
  "venues": [
    {
      "venue": "Aero",
      "status": "quoted",
      "reason": null,
      "swap_url": "https://app.aero.xyz/swap",
      "nock_out": "1773.0768618618146014",
      "effective_price_usd": "0.01127983",
      "cost_over_spot_pct": "-0.06",
      "route": "USDC → USDT → WETH → NOCK",
      "hops": [
        {
          "pool": "0xd448990883e3bf9cdd8ebca368a3ecf9d629aaf7",
          "pool_kind": "Aero concentrated",
          "token_in": "USDC",
          "token_out": "USDT",
          "fee_pct": "0.0008"
        },
        {
          "pool": "0x2dbe0e264c7c246e05c6614794899937f264cf7b",
          "pool_kind": "Aero concentrated",
          "token_in": "USDT",
          "token_out": "WETH",
          "fee_pct": "0.0468"
        },
        {
          "pool": "0x957256c98d61a714081de2ee70ae3a96c9b40874",
          "pool_kind": "Aero concentrated",
          "token_in": "WETH",
          "token_out": "NOCK",
          "fee_pct": "0.97"
        }
      ],
      "routes_tried": [
        {
          "route_id": "aero-basic-direct",
          "route": "USDC → NOCK",
          "nock_out": "1742.2508413591799568",
          "reason": null
        },
        {
          "route_id": "aero-cl50-weth",
          "route": "USDC → WETH → NOCK",
          "nock_out": "1772.5259725360589554",
          "reason": null
        },
        {
          "route_id": "aero-cl100-weth",
          "route": "USDC → WETH → NOCK",
          "nock_out": "1772.200440812282513",
          "reason": null
        },
        {
          "route_id": "aero-usdt-weth",
          "route": "USDC → USDT → WETH → NOCK",
          "nock_out": "1773.0768618618146014",
          "reason": null
        }
      ]
    },
    {
      "venue": "Uniswap",
      "status": "quoted",
      "reason": null,
      "swap_url": "https://app.uniswap.org/swap",
      "nock_out": "1772.2085413951150619",
      "effective_price_usd": "0.01128535",
      "cost_over_spot_pct": "-0.01",
      "route": "USDC → WETH → NOCK",
      "hops": [
        {
          "pool": "0xd0b53d9277642d899df5c87a3966a349a798f224",
          "pool_kind": "Uniswap v3",
          "token_in": "USDC",
          "token_out": "WETH",
          "fee_pct": "0.05"
        },
        {
          "pool": "0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a",
          "pool_kind": "Uniswap v3",
          "token_in": "WETH",
          "token_out": "NOCK",
          "fee_pct": "1"
        }
      ],
      "routes_tried": [
        {
          "route_id": "uniswap-v4-direct",
          "route": "USDC → NOCK",
          "nock_out": "1716.8660568388805278",
          "reason": null
        },
        {
          "route_id": "uniswap-v3-weth",
          "route": "USDC → WETH → NOCK",
          "nock_out": "1772.2085413951150619",
          "reason": null
        },
        {
          "route_id": "uniswap-v4-eth",
          "route": "USDC → ETH → NOCK",
          "nock_out": "1692.7635343644656443",
          "reason": null
        }
      ]
    }
  ],
  "more_nock_venue": "Aero",
  "more_nock_by_pct": "0.05",
  "comparison": "Aero returns more NOCK by 0.05% for $20.00 USDC at block 58081910: 1773.0768 NOCK against 1772.2085 on Uniswap."
};
