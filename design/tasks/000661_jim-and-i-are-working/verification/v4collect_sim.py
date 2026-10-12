#!/usr/bin/env python3
"""Empirically determine the deployed Uniswap V4 PositionManager (Base) collect-fees
encoding: action bytes + unlockData layout, verified by live eth_call simulation.

Run: python3 v4collect_sim.py   (needs only stdlib; hits https://mainnet.base.org)

This is the same discovery pattern the repo used for StateView in PR #25:
never trust memory over the chain - simulate and see what does not revert.
"""
import json, urllib.request

RPC = "https://mainnet.base.org"

def keccak256(data: bytes) -> bytes:
    """keccak-f[1600] with keccak padding (0x01...0x80), NOT sha3 padding (0x06)."""
    RC = [
        0x0000000000000001, 0x0000000000008082, 0x800000000000808A, 0x8000000080008000,
        0x000000000000808B, 0x0000000080000001, 0x8000000080008081, 0x8000000000008009,
        0x000000000000008A, 0x0000000000000088, 0x0000000080008009, 0x000000008000000A,
        0x000000008000808B, 0x800000000000008B, 0x8000000000008089, 0x8000000000008003,
        0x8000000000008002, 0x8000000000000080, 0x000000000000800A, 0x800000008000000A,
        0x8000000080008081, 0x8000000000008080, 0x0000000080000001, 0x8000000080008008,
    ]
    ROT = [[0,36,3,41,18],[1,44,10,45,2],[62,6,43,15,61],[28,55,25,21,56],[27,20,39,8,14]]
    M = (1 << 64) - 1
    def rol(x, n): return ((x << n) | (x >> (64 - n))) & M
    def keccak_f(st):
        for rnd in range(24):
            c = [st[x][0] ^ st[x][1] ^ st[x][2] ^ st[x][3] ^ st[x][4] for x in range(5)]
            d = [c[(i-1)%5] ^ rol(c[(i+1)%5], 1) for i in range(5)]
            for x in range(5):
                for y in range(5):
                    st[x][y] ^= d[x]
            b = [[0]*5 for _ in range(5)]
            for x in range(5):
                for y in range(5):
                    b[y][(2*x+3*y)%5] = rol(st[x][y], ROT[x][y])
            for x in range(5):
                for y in range(5):
                    st[x][y] = b[x][y] ^ ((~b[(x+1)%5][y]) & b[(x+2)%5][y])
            st[0][0] ^= RC[rnd]
        return st
    st = [[0]*5 for _ in range(5)]
    rate = 136  # 1088-bit rate for 256-bit output
    padded = bytearray(data)
    padded.append(0x01)
    while len(padded) % rate != 0:
        padded.append(0x00)
    padded[-1] |= 0x80
    for off in range(0, len(padded), rate):
        block = padded[off:off+rate]
        for i in range(rate // 8):
            lane = int.from_bytes(block[i*8:(i+1)*8], 'little')
            x, y = i % 5, i // 5
            st[x][y] ^= lane
        keccak_f(st)
    out = bytearray()
    for i in range(4):
        x, y = i % 5, i // 5
        out += st[x][y].to_bytes(8, 'little')
    return bytes(out)

def sel(sig): return "0x" + keccak256(sig.encode()).hex()[:8]

assert keccak256(b"").hex() == "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
assert sel("claimFees()") == "0xd294f093", sel("claimFees()")
assert sel("transfer(address,uint256)") == "0xa9059cbb"
assert sel("balanceOf(address)") == "0x70a08231"
print("keccak OK; modifyLiquidities(bytes,uint256) =", sel("modifyLiquidities(bytes,uint256)"))

def rpc(method, params):
    body = json.dumps({"jsonrpc": "2.0", "method": method, "params": params, "id": 1}).encode()
    req = urllib.request.Request(RPC, data=body, headers={"Content-Type": "application/json", "User-Agent": "nocknock-verify/0.2.0"})
    with urllib.request.urlopen(req, timeout=15) as r:
        res = json.loads(r.read())
    if "error" in res:
        return ("ERROR", res["error"])
    return ("OK", res.get("result"))

def call(to, data, frm):
    return rpc("eth_call", [{"to": to, "data": data, "from": frm}, "latest"])

WALLET = "0xeE114cC97eC5adeB34bA5645e315B497D8487a45"
POSM = "0x7c5f5a4bbd8fd63184577525326123b519429bdc"
TOKEN_ID = 3126000
USDC = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
NOCK = "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722"

def addr(w): return w.lower().replace("0x", "").rjust(64, "0")
def u256(v): return format(v, "064x")

# 1) getPoolAndPositionInfo(uint256) -> ((address,address,uint24,int24,address), uint256)
gppi = sel("getPoolAndPositionInfo(uint256)")
st, res = call(POSM, gppi + u256(TOKEN_ID), WALLET)
print("getPoolAndPositionInfo:", st)
if st == "OK" and isinstance(res, str):
    raw = res[2:]
    c0 = "0x" + raw[24:64]; c1 = "0x" + raw[88:128]
    fee = int(raw[128:192], 16); ts = int(raw[192:256], 16)
    hook = "0x" + raw[280:320]
    print(f"  currency0={c0} currency1={c1} fee={fee} tickSpacing={ts} hooks={hook}")

def abi_concat(*words): return "".join(words)
def encode_tuple(*words): return u256(32) + abi_concat(*words)

def encode_bytes_field(b_hex):
    # pad to a multiple of 32 BYTES = 64 hex chars (b_hex length is in chars)
    pairs = ((64 - (len(b_hex) % 64)) % 64) // 2
    padded = b_hex + "00" * pairs
    return u256(len(b_hex) // 2) + padded

def encode_bytes_array(items):
    # bytes[] array layout at its own position: [length, n offset words, contents]
    # offsets are in BYTES, relative to the start of the array (i.e. after the length word)
    n = len(items)
    offsets_area = n * 32
    offs = ""
    acc = 0
    for it in items:
        offs += u256(offsets_area + acc)
        acc += 32 + ((len(it) // 2 + 31) // 32) * 32
    contents = ""
    for it in items:
        contents += u256(len(it) // 2)
        contents += it + "00" * (((64 - (len(it) % 64)) % 64) // 2)
    return u256(n) + offs + contents

def ab_bytes(h): return [int(h[i:i+2], 16) for i in range(0, len(h), 2)]

SEL_MOD2 = sel("modifyLiquidities(bytes,uint256)")
SEL_MOD1 = sel("modifyLiquidities(bytes)")
DEADLINE = 0xFFFFFFFFFFFF  # far future; simulation only

def outer_two_arg(unlock_hex):
    # (bytes unlockData, uint256 deadline): word0 = 0x40 offset, word1 = deadline
    return SEL_MOD2 + u256(0x40) + u256(DEADLINE) + encode_bytes_field(unlock_hex)

def outer_one_arg(unlock_hex):
    # (bytes unlockData): offset 0x20 correct for a single dynamic arg
    return SEL_MOD1 + encode_bytes_field(unlock_hex)

def build_strict(params, actions_hex, outer):
    # decodeActions expects: unlockData = abi.encode(bytes actions, bytes[] params) STRICT:
    # word0 = 0x40 (offset to actions bytes), word1 = offset to params array, then fields
    actions_len_padded = ((len(actions_hex) + 63) // 64) * 32
    params_offset = 0x40 + 32 + actions_len_padded
    unlock = u256(0x40) + u256(params_offset) + encode_bytes_field(actions_hex) + encode_bytes_array(params)
    return outer(unlock)

def build_packed(params, actions_hex, outer):
    # packed candidate: concat(action byte + abi.encode(tuple)) wrapped as single bytes
    unlock = "".join(format(b, "02x") + p for b, p in zip(ab_bytes(actions_hex), params))
    return outer(encode_bytes_field(unlock))

def decrease_new(token_id, hook_data=""):
    # main branch, flat member encoding (no tuple offset), dynamic bytes offset is
    # head-relative INCLUDING the offset word itself (4 statics + 1 offset = 0xa0):
    # [tokenId u256, liquidity u256, amount0Min u128, amount1Min u128, hookOff, hookLen]
    return (u256(token_id) + u256(0) + u256(0) + u256(0)
            + u256(0xa0) + u256(len(hook_data) // 2)
            + hook_data + "00" * (((64 - (len(hook_data) % 64)) % 64) // 2))

def decrease_old(token_id, deadline):
    # v1.0.0 flat: (uint256 tokenId, int256 liquidityDelta, uint256 amount0Min,
    #               uint256 amount1Min, uint256 deadline) — all static, 5 words
    return u256(token_id) + u256(0) + u256(0) + u256(0) + u256(deadline)

def take_pair(recipient, c0, c1):
    # flat, all static: [currency0, currency1, recipient] — 3 words, no offset wrapper
    return addr(c0) + addr(c1) + addr(recipient)

cands = [
    ("L strict 0x01/0x07 OLD tuples (deadline-in-params)", build_strict(
        [decrease_old(TOKEN_ID, DEADLINE), take_pair(WALLET, USDC, NOCK)], "0107", outer_two_arg)),
    ("M strict 0x01/0x11 NEW tuples (hookData)", build_strict(
        [decrease_new(TOKEN_ID), take_pair(WALLET, USDC, NOCK)], "0111", outer_two_arg)),
    ("N strict 0x01/0x07 NEW tuples", build_strict(
        [decrease_new(TOKEN_ID), take_pair(WALLET, USDC, NOCK)], "0107", outer_two_arg)),
    ("O strict 0x01/0x11 OLD tuples", build_strict(
        [decrease_old(TOKEN_ID, DEADLINE), take_pair(WALLET, USDC, NOCK)], "0111", outer_two_arg)),
]

for name, data in cands:
    st, res = call(POSM, data, WALLET)
    if isinstance(res, str):
        print(f"{name}: {st} -> {res[:140]}")
    else:
        print(f"{name}: {st} -> {json.dumps(res)[:240]}")
