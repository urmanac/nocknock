import type { FloorBreach } from './floor';

/**
 * derived_metrics.floor_breach as the worker states it for the live book of
 * 2026-11-21T02:37:59Z (ingestion-worker/src/tests_floor_breach.rs pins the same
 * figures): floor 100,000, 1,000 bps tolerated, v4 USDC/NOCK #3163450 and the Aero
 * full-range pool.
 */
export const LIVE_FLOOR_BREACH: FloorBreach = {
  status: 'crosses',
  floor_nock: '100000',
  spot_price_usd: '0.012944',
  crossing: { level_nock: '100000', status: 'crosses', multiple: '1.9626', price_usd: '0.025405', pct_above_spot: '96.3' },
  tolerated: { level_nock: '90000', status: 'never', multiple: null, price_usd: null, pct_above_spot: null },
  nock_now: '109542',
  wallet_nock: '90087',
  fees_nock: '114',
  nock_if_all_sold: '90201',
  largest_seller: 'v4 USDC/NOCK 3.00% #3163450',
  first_to_sell_out: 'v4 USDC/NOCK 3.00% #3163450',
  positions: [
    {
      position_id: 'uniswap-v4:base:3163450',
      label: 'v4 USDC/NOCK 3.00% #3163450',
      full_range: false,
      nock_now: '10933',
      nock_at_crossing: '3798',
      sold_out_at_multiple: '3.1716',
      sold_out_at_price_usd: '0.041055',
      note: null,
    },
    {
      position_id: 'aerodrome:base:0x770fe8fc9248c6382950a86e39a9391b41d59e0a',
      label: 'Aero USDC/NOCK 0.30% (full range)',
      full_range: true,
      nock_now: '8408',
      nock_at_crossing: '6002',
      sold_out_at_multiple: null,
      sold_out_at_price_usd: null,
      note: 'full range: sells gradually as price rises and never empties',
    },
  ],
  assumption:
    'Each pool is moved by the same multiple from its own current price. WETH/NOCK ranges hold ETH/USD constant (a NOCK/USD multiple is the same NOCK/ETH multiple). Full-range pools follow x*y=k on the wallet\'s share and never sell out. Uncollected NOCK fees count as held. Gas-side ranges with no NOCK leg hold no NOCK.',
  reason: null,
};
