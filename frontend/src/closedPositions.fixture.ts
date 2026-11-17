/**
 * Fixture for the closed-position helpers (issue #144).
 *
 * These are the worker's own published values for the tracked wallet's real book at
 * Base block 58646285 (2026-11-17, after the operator withdrew Aero CL NFT #2914490 in
 * block 58637717), copied from ingestion-worker/testdata/golden_snapshot_closed.json.
 * closedPositions.test.ts checks the copy against that file when it is present.
 *
 * - WITHDRAWN: the book with the withdrawn Aero CL entry still configured.
 * - V4_SOURCE_CLOSED: the same book with its Uniswap v4 price-source entry pointed at a
 *   token id that does not exist (#3229273, burned by another account in block
 *   58641149); the pool still supplies the NOCK/USD spot.
 */

export const WITHDRAWN_CLOSED = [
  {
    "config_id": "aero-cl200-weth-nock",
    "evidence": [
      {
        "answer": "Error(\"ID\")",
        "call": "positions(uint256)",
        "contract": "0x4857db576ee0e45f10525ec59527bfd10beec60b",
        "outcome": "reverted",
        "raw": "0x08c379a0000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000024944000000000000000000000000000000000000000000000000000000000000"
      },
      {
        "answer": "Error(\"ERC721: owner query for nonexistent token\")",
        "call": "ownerOf(uint256)",
        "contract": "0x4857db576ee0e45f10525ec59527bfd10beec60b",
        "outcome": "reverted",
        "raw": "0x08c379a0000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000294552433732313a206f776e657220717565727920666f72206e6f6e6578697374656e7420746f6b656e0000000000000000000000000000000000000000000000"
      }
    ],
    "first_seen_closed_at": "2026-11-17T13:00:00+00:00",
    "label": "Aero CL #2914490 (aero-cl200-weth-nock)",
    "network": "base",
    "nft_id": 2914490,
    "pool_id": "0x957256c98d61a714081de2ee70ae3a96c9b40874",
    "position_id": "aerodrome-cl:base:2914490",
    "position_manager": "0x4857db576ee0e45f10525ec59527bfd10beec60b",
    "protocol": "aerodrome-cl",
    "reason": "NFT #2914490 does not exist on position manager 0x4857db576ee0e45f10525ec59527bfd10beec60b: withdrawn and burned, or a mistyped nft_id",
    "status": "closed"
  }
];

export const WITHDRAWN_POSITIONS = [
  {
    "position_id": "uniswap-v4:base:3163450",
    "protocol": "uniswap-v4",
    "nft_id": 3163450,
    "pool_id": "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13",
    "fee_tier_bps": 300,
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
    "amount0": "120.022849",
    "amount1": "8114.362638",
    "amount_usd": "241.21",
    "tokens_owed_0": "0.000000",
    "tokens_owed_1": "0.000000",
    "fees_usd": "0.000000",
    "lower_price": "0.00550",
    "upper_price": "0.04106",
    "in_range": true,
    "is_price_source": true,
    "pool_active": true,
    "full_range": false,
    "network": "base"
  },
  {
    "position_id": "uniswap-v3:base:6336662",
    "protocol": "uniswap-v3",
    "nft_id": 6336662,
    "pool_id": "0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a",
    "fee_tier_bps": 100,
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
    "amount0": "0.090741",
    "amount1": "2294.092017",
    "amount_usd": "199.46",
    "tokens_owed_0": "0.000581",
    "tokens_owed_1": "61.508693",
    "fees_usd": "1.976558",
    "lower_price": "0.00000564",
    "upper_price": "0.00000876",
    "in_range": true,
    "is_price_source": false,
    "pool_active": true,
    "full_range": false,
    "network": "base"
  },
  {
    "position_id": "aerodrome:base:0x770fe8fc9248c6382950a86e39a9391b41d59e0a",
    "protocol": "aerodrome",
    "nft_id": 0,
    "pool_id": "0x770fe8fc9248c6382950a86e39a9391b41d59e0a",
    "fee_tier_bps": 30,
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
    "amount0": "78.578559",
    "amount1": "5287.527135",
    "amount_usd": "157.55",
    "tokens_owed_0": "0.000000",
    "tokens_owed_1": "0.000000",
    "fees_usd": "0.000000",
    "lower_price": "0",
    "upper_price": "0",
    "in_range": true,
    "is_price_source": false,
    "pool_active": true,
    "full_range": true,
    "network": "base"
  }
];

export const WITHDRAWN_GLOBAL = {
  "total_portfolio_usd": "2991.44",
  "total_native_usd": "1598.80",
  "total_stable_usd": "1011.68",
  "total_eth_usd": "373.16",
  "closed_positions_count": 1,
  "closed_positions_note": "1 configured position is closed and is not counted in any total: Aero CL #2914490 (aero-cl200-weth-nock) (NFT #2914490 no longer exists: withdrawn and burned, or a mistyped nft_id). Its entry can be removed from config.yaml."
};

export const V4_SOURCE_CLOSED = [
  {
    "config_id": "uniswap-v4-nock-usdc-gone",
    "evidence": [
      {
        "answer": "Error(\"NOT_MINTED\")",
        "call": "ownerOf(uint256)",
        "contract": "0x7c5f5a4bbd8fd63184577525326123b519429bdc",
        "outcome": "reverted",
        "raw": "0x08c379a00000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000a4e4f545f4d494e54454400000000000000000000000000000000000000000000"
      },
      {
        "answer": "192 zero bytes: no pool and no position info is stored for this id",
        "call": "getPoolAndPositionInfo(uint256)",
        "contract": "0x7c5f5a4bbd8fd63184577525326123b519429bdc",
        "outcome": "returned",
        "raw": "0x000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
      }
    ],
    "first_seen_closed_at": "2026-11-17T13:00:00+00:00",
    "label": "v4 USDC/NOCK #3229273 (uniswap-v4-nock-usdc-gone)",
    "network": "base",
    "nft_id": 3229273,
    "pool_id": "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13",
    "position_id": "uniswap-v4:base:3229273",
    "position_manager": "0x7c5f5a4bbd8fd63184577525326123b519429bdc",
    "protocol": "uniswap-v4",
    "reason": "NFT #3229273 does not exist on position manager 0x7c5f5a4bbd8fd63184577525326123b519429bdc: withdrawn and burned, or a mistyped nft_id",
    "status": "closed",
    "supplies_spot_price": true
  }
];

export const V4_SOURCE_POSITIONS = [
  {
    "position_id": "uniswap-v3:base:6336662",
    "protocol": "uniswap-v3",
    "nft_id": 6336662,
    "pool_id": "0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a",
    "fee_tier_bps": 100,
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
    "amount0": "0.090741",
    "amount1": "2294.092017",
    "amount_usd": "199.46",
    "tokens_owed_0": "0.000581",
    "tokens_owed_1": "61.508693",
    "fees_usd": "1.976558",
    "lower_price": "0.00000564",
    "upper_price": "0.00000876",
    "in_range": true,
    "is_price_source": false,
    "pool_active": true,
    "full_range": false,
    "network": "base"
  },
  {
    "position_id": "aerodrome:base:0x770fe8fc9248c6382950a86e39a9391b41d59e0a",
    "protocol": "aerodrome",
    "nft_id": 0,
    "pool_id": "0x770fe8fc9248c6382950a86e39a9391b41d59e0a",
    "fee_tier_bps": 30,
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
    "amount0": "78.578559",
    "amount1": "5287.527135",
    "amount_usd": "157.55",
    "tokens_owed_0": "0.000000",
    "tokens_owed_1": "0.000000",
    "fees_usd": "0.000000",
    "lower_price": "0",
    "upper_price": "0",
    "in_range": true,
    "is_price_source": false,
    "pool_active": true,
    "full_range": true,
    "network": "base"
  }
];

/** The live book as a snapshot `data` object: three positions, one closed entry. */
export const WITHDRAWN_DATA = {
  positions: WITHDRAWN_POSITIONS,
  closed_positions: WITHDRAWN_CLOSED,
  derived_metrics: { global_status: WITHDRAWN_GLOBAL },
};

/** The same snapshot as a worker from before 2.8.0 published it: no closed list. */
export const OLD_DATA = {
  positions: WITHDRAWN_POSITIONS,
  derived_metrics: {
    global_status: Object.fromEntries(
      Object.entries(WITHDRAWN_GLOBAL).filter(([k]) => !k.startsWith('closed_positions')),
    ),
  },
};

export const V4_SOURCE_DATA = { positions: V4_SOURCE_POSITIONS, closed_positions: V4_SOURCE_CLOSED };
