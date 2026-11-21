/**
 * Positions found on-chain (v0.9.0) as the worker states them, copied from
 * ingestion-worker/testdata/position_detect/golden_position_detect.json, which
 * ingestion-worker/src/tests_position_detect.rs writes from recorded chain answers (the
 * operator's wallet at block 58646285; NFT #3163450 is its own Uniswap v4 range).
 * foundPositions.test.ts checks the copy against the file when the worker directory is
 * present.
 */
import type { PositionDetection } from './foundPositions';

/** nothing_found: derived_metrics.position_detection */
export const DETECTION_NOTHING_FOUND: PositionDetection = {
  "checked_at_block": 58646285,
  "dismissed": {
    "count": 0,
    "positions": [],
    "value_note": null,
    "value_usd": null
  },
  "found_not_counted": {
    "count": 0,
    "positions": [],
    "value_note": null,
    "value_usd": null
  },
  "ignored": {
    "empty": 5,
    "in_config": 1,
    "not_whitelisted": 37
  },
  "last_checked_at": "2026-11-17T12:47:16+00:00",
  "onchain_count": {
    "held": 43,
    "known": 43
  },
  "reason": null,
  "scan": {
    "coverage": "complete",
    "logs_behind_blocks": 0,
    "logs_from_block": 0,
    "sources": {
      "transfer_index": "not_used",
      "transfer_logs": "used"
    },
    "text": "Every Uniswap v4 position NFT sent to the wallet was read from the transfer logs, from the first block to this cycle's block."
  },
  "statement": "NockNock reads the wallet's Uniswap v4 position NFTs from the chain. It tracks one by itself only when the wallet minted it and its pool and ticks are exactly a rung of the ladder suggestion; anything else it finds waits for one tap and is not counted until then. It never signs or sends.",
  "status": "ok",
  "tracked": []
} as any;

/** found_not_counted: derived_metrics.position_detection */
export const DETECTION_FOUND: PositionDetection = {
  "checked_at_block": 58646285,
  "dismissed": {
    "count": 0,
    "positions": [],
    "value_note": null,
    "value_usd": null
  },
  "found_not_counted": {
    "count": 1,
    "positions": [
      {
        "bounds_usd": {
          "high": "0.04105518",
          "low": "0.00550148"
        },
        "choices": [
          "ladder:near",
          "ladder:deep",
          "lp",
          "dismiss"
        ],
        "holds": {
          "liquidity": "249864729574523",
          "token0": {
            "amount": "120.02",
            "symbol": "USDC"
          },
          "token1": {
            "amount": "8114.3626",
            "symbol": "NOCK"
          }
        },
        "in_range": true,
        "mint_block": 53008241,
        "mint_tx": "0x6cdabada1ff062b6d3b4ba87bfe04158dd8b14a91e93cabe11dc3e35b587e89e",
        "minted_by": "0xee114cc97ec5adeb34ba5645e315b497d8487a45",
        "nft_id": 3163450,
        "pool_id": "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13",
        "pool_label": "v4 USDC/NOCK 3.00%",
        "position_id": "uniswap-v4:base:3163450",
        "protocol": "uniswap-v4",
        "received_from": null,
        "tick_lower": 262200,
        "tick_upper": 282300,
        "value_note": null,
        "value_usd": "240.61",
        "why_not_automatic": [
          "no ladder rung is suggested now to match it against"
        ]
      }
    ],
    "value_note": null,
    "value_usd": "240.61"
  },
  "ignored": {
    "empty": 5,
    "not_whitelisted": 37
  },
  "last_checked_at": "2026-11-17T12:47:16+00:00",
  "onchain_count": {
    "held": 43,
    "known": 43
  },
  "reason": null,
  "scan": {
    "coverage": "complete",
    "logs_behind_blocks": 0,
    "logs_from_block": 0,
    "sources": {
      "transfer_index": "not_used",
      "transfer_logs": "used"
    },
    "text": "Every Uniswap v4 position NFT sent to the wallet was read from the transfer logs, from the first block to this cycle's block."
  },
  "statement": "NockNock reads the wallet's Uniswap v4 position NFTs from the chain. It tracks one by itself only when the wallet minted it and its pool and ticks are exactly a rung of the ladder suggestion; anything else it finds waits for one tap and is not counted until then. It never signs or sends.",
  "status": "ok",
  "tracked": []
} as any;

/** dismissed: derived_metrics.position_detection */
export const DETECTION_DISMISSED: PositionDetection = {
  "checked_at_block": 58646285,
  "dismissed": {
    "count": 1,
    "positions": [
      {
        "bounds_usd": {
          "high": "0.04105518",
          "low": "0.00550148"
        },
        "choices": [
          "ladder:near",
          "ladder:deep",
          "lp",
          "dismiss"
        ],
        "holds": {
          "liquidity": "249864729574523",
          "token0": {
            "amount": "120.02",
            "symbol": "USDC"
          },
          "token1": {
            "amount": "8114.3626",
            "symbol": "NOCK"
          }
        },
        "in_range": true,
        "mint_block": 53008241,
        "mint_tx": "0x6cdabada1ff062b6d3b4ba87bfe04158dd8b14a91e93cabe11dc3e35b587e89e",
        "minted_by": "0xee114cc97ec5adeb34ba5645e315b497d8487a45",
        "nft_id": 3163450,
        "pool_id": "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13",
        "pool_label": "v4 USDC/NOCK 3.00%",
        "position_id": "uniswap-v4:base:3163450",
        "protocol": "uniswap-v4",
        "received_from": null,
        "tick_lower": 262200,
        "tick_upper": 282300,
        "value_note": null,
        "value_usd": "240.61",
        "why_not_automatic": [
          "no ladder rung is suggested now to match it against"
        ]
      }
    ],
    "value_note": null,
    "value_usd": "240.61"
  },
  "found_not_counted": {
    "count": 0,
    "positions": [],
    "value_note": null,
    "value_usd": null
  },
  "ignored": {
    "empty": 5,
    "not_whitelisted": 37
  },
  "last_checked_at": "2026-11-17T12:47:16+00:00",
  "onchain_count": {
    "held": 43,
    "known": 43
  },
  "reason": null,
  "scan": {
    "coverage": "complete",
    "logs_behind_blocks": 0,
    "logs_from_block": 0,
    "sources": {
      "transfer_index": "not_used",
      "transfer_logs": "used"
    },
    "text": "Every Uniswap v4 position NFT sent to the wallet was read from the transfer logs, from the first block to this cycle's block."
  },
  "statement": "NockNock reads the wallet's Uniswap v4 position NFTs from the chain. It tracks one by itself only when the wallet minted it and its pool and ticks are exactly a rung of the ladder suggestion; anything else it finds waits for one tap and is not counted until then. It never signs or sends.",
  "status": "ok",
  "tracked": []
} as any;

/** tracked_by_itself: derived_metrics.position_detection */
export const DETECTION_TRACKED_BY_ITSELF: PositionDetection = {
  "checked_at_block": 58646285,
  "dismissed": {
    "count": 0,
    "positions": [],
    "value_note": null,
    "value_usd": null
  },
  "found_not_counted": {
    "count": 0,
    "positions": [],
    "value_note": null,
    "value_usd": null
  },
  "ignored": {
    "empty": 5,
    "not_whitelisted": 37
  },
  "last_checked_at": "2026-11-17T12:47:16+00:00",
  "onchain_count": {
    "held": 43,
    "known": 43
  },
  "reason": null,
  "scan": {
    "coverage": "complete",
    "logs_behind_blocks": 0,
    "logs_from_block": 0,
    "sources": {
      "transfer_index": "not_used",
      "transfer_logs": "used"
    },
    "text": "Every Uniswap v4 position NFT sent to the wallet was read from the transfer logs, from the first block to this cycle's block."
  },
  "statement": "NockNock reads the wallet's Uniswap v4 position NFTs from the chain. It tracks one by itself only when the wallet minted it and its pool and ticks are exactly a rung of the ladder suggestion; anything else it finds waits for one tap and is not counted until then. It never signs or sends.",
  "status": "ok",
  "tracked": [
    {
      "choices": [
        "ladder:near",
        "ladder:deep",
        "untrack"
      ],
      "config_id": "tracked-v4-3163450",
      "deposit_tx": "0x6cdabada1ff062b6d3b4ba87bfe04158dd8b14a91e93cabe11dc3e35b587e89e",
      "mint_block": 53008241,
      "nft_id": 3163450,
      "note": null,
      "position_id": "uniswap-v4:base:3163450",
      "role": "ladder",
      "rung": null,
      "rung_note": "no rung is set: the suggestion it matched named none, and NockNock assumes neither near nor deep. One tap sets it",
      "source": "detected",
      "state": "counted",
      "tick_lower": 262200,
      "tick_upper": 282300,
      "tracked_at": "2026-11-17T12:47:16+00:00"
    }
  ]
} as any;

/** tracked_by_tap: derived_metrics.position_detection */
export const DETECTION_TRACKED_BY_TAP: PositionDetection = {
  "checked_at_block": 58646285,
  "dismissed": {
    "count": 0,
    "positions": [],
    "value_note": null,
    "value_usd": null
  },
  "found_not_counted": {
    "count": 0,
    "positions": [],
    "value_note": null,
    "value_usd": null
  },
  "ignored": {
    "empty": 5,
    "not_whitelisted": 37
  },
  "last_checked_at": "2026-11-17T12:47:16+00:00",
  "onchain_count": {
    "held": 43,
    "known": 43
  },
  "reason": null,
  "scan": {
    "coverage": "complete",
    "logs_behind_blocks": 0,
    "logs_from_block": 0,
    "sources": {
      "transfer_index": "not_used",
      "transfer_logs": "used"
    },
    "text": "Every Uniswap v4 position NFT sent to the wallet was read from the transfer logs, from the first block to this cycle's block."
  },
  "statement": "NockNock reads the wallet's Uniswap v4 position NFTs from the chain. It tracks one by itself only when the wallet minted it and its pool and ticks are exactly a rung of the ladder suggestion; anything else it finds waits for one tap and is not counted until then. It never signs or sends.",
  "status": "ok",
  "tracked": [
    {
      "choices": [
        "untrack"
      ],
      "config_id": "tracked-v4-3163450",
      "deposit_tx": "0x6cdabada1ff062b6d3b4ba87bfe04158dd8b14a91e93cabe11dc3e35b587e89e",
      "mint_block": 53008241,
      "nft_id": 3163450,
      "note": null,
      "position_id": "uniswap-v4:base:3163450",
      "role": "ladder",
      "rung": "deep",
      "rung_note": null,
      "source": "confirmed",
      "state": "counted",
      "tick_lower": 262200,
      "tick_upper": 282300,
      "tracked_at": "2026-11-17T12:47:16+00:00"
    }
  ]
} as any;

/** partial: derived_metrics.position_detection */
export const DETECTION_PARTIAL: PositionDetection = {
  "checked_at_block": 58646285,
  "dismissed": {
    "count": 0,
    "positions": [],
    "value_note": null,
    "value_usd": null
  },
  "found_not_counted": {
    "count": 1,
    "positions": [
      {
        "bounds_usd": {
          "high": "0.04105518",
          "low": "0.00550148"
        },
        "choices": [
          "ladder:near",
          "ladder:deep",
          "lp",
          "dismiss"
        ],
        "holds": {
          "liquidity": "249864729574523",
          "token0": {
            "amount": "120.02",
            "symbol": "USDC"
          },
          "token1": {
            "amount": "8114.3626",
            "symbol": "NOCK"
          }
        },
        "in_range": true,
        "mint_block": 53008241,
        "mint_tx": "0x6cdabada1ff062b6d3b4ba87bfe04158dd8b14a91e93cabe11dc3e35b587e89e",
        "minted_by": "0xee114cc97ec5adeb34ba5645e315b497d8487a45",
        "nft_id": 3163450,
        "pool_id": "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13",
        "pool_label": "v4 USDC/NOCK 3.00%",
        "position_id": "uniswap-v4:base:3163450",
        "protocol": "uniswap-v4",
        "received_from": null,
        "tick_lower": 262200,
        "tick_upper": 282300,
        "value_note": null,
        "value_usd": "240.61",
        "why_not_automatic": [
          "no ladder rung is suggested now to match it against"
        ]
      }
    ],
    "value_note": null,
    "value_usd": "240.61"
  },
  "ignored": {
    "empty": 2
  },
  "last_checked_at": "2026-11-17T12:47:16+00:00",
  "onchain_count": {
    "held": 43,
    "known": 3
  },
  "reason": "transfers before block 52931286 are not read yet (this RPC does not serve older blocks (the RPC answered with error -32602: Archive requests require a personal token. Get one at: https://www.allnodes.com/publicnode); transfer index: the RPC answered with error -32601: Method not found); the wallet holds 43 Uniswap v4 position NFTs on-chain and 3 are known: 40 not found yet",
  "scan": {
    "coverage": "partial",
    "logs_behind_blocks": 0,
    "logs_from_block": 52931286,
    "sources": {
      "transfer_index": "failed",
      "transfer_logs": "used"
    },
    "text": "The wallet's position-NFT transfers were read from block 52931286 to this cycle's block. A position that reached the wallet before block 52931286 is not found yet: this RPC does not serve older blocks (the RPC answered with error -32602: Archive requests require a personal token. Get one at: https://www.allnodes.com/publicnode)."
  },
  "statement": "NockNock reads the wallet's Uniswap v4 position NFTs from the chain. It tracks one by itself only when the wallet minted it and its pool and ticks are exactly a rung of the ladder suggestion; anything else it finds waits for one tap and is not counted until then. It never signs or sends.",
  "status": "partial",
  "tracked": []
} as any;

/** unavailable: derived_metrics.position_detection */
export const DETECTION_UNAVAILABLE: PositionDetection = {
  "checked_at_block": null,
  "dismissed": {
    "count": 0,
    "positions": [],
    "value_note": null,
    "value_usd": null
  },
  "found_not_counted": {
    "count": 1,
    "positions": [
      {
        "bounds_usd": {
          "high": "0.04105518",
          "low": "0.00550148"
        },
        "choices": [
          "ladder:near",
          "ladder:deep",
          "lp",
          "dismiss"
        ],
        "holds": null,
        "in_range": null,
        "mint_block": 53008241,
        "mint_tx": "0x6cdabada1ff062b6d3b4ba87bfe04158dd8b14a91e93cabe11dc3e35b587e89e",
        "minted_by": "0xee114cc97ec5adeb34ba5645e315b497d8487a45",
        "nft_id": 3163450,
        "pool_id": "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13",
        "pool_label": "v4 USDC/NOCK 3.00%",
        "position_id": "uniswap-v4:base:3163450",
        "protocol": "uniswap-v4",
        "received_from": null,
        "tick_lower": 262200,
        "tick_upper": 282300,
        "value_note": "it was not read this cycle, so what it holds and its value are not stated",
        "value_usd": null,
        "why_not_automatic": [
          "no ladder rung is suggested now to match it against"
        ]
      }
    ],
    "value_note": "1 of 1 could not be valued this cycle, so no total is stated",
    "value_usd": null
  },
  "ignored": {
    "empty": 5,
    "not_whitelisted": 37
  },
  "last_checked_at": null,
  "onchain_count": {
    "held": null,
    "known": null
  },
  "reason": "New positions could not be checked this cycle (no Alchemy RPC key is configured): a deposit made since the last check would not be counted yet.",
  "scan": {
    "coverage": "none",
    "text": "New positions could not be checked this cycle (no Alchemy RPC key is configured): a deposit made since the last check would not be counted yet."
  },
  "statement": "NockNock reads the wallet's Uniswap v4 position NFTs from the chain. It tracks one by itself only when the wallet minted it and its pool and ticks are exactly a rung of the ladder suggestion; anything else it finds waits for one tap and is not counted until then. It never signs or sends.",
  "status": "unavailable",
  "tracked": []
} as any;
