#!/usr/bin/env python3
"""Bisect the deployed V4 posm collect: which (action, layout, tuple-shape) combo succeeds."""
import json, urllib.request, sys
sys.path.insert(0, ".")
from v4collect_sim import (  # reuse keccak/encoders/rpc
    keccak256, sel, rpc, call, WALLET, POSM, TOKEN_ID, USDC, NOCK,
    addr, u256, encode_tuple, encode_bytes_field, encode_bytes_array, ab_bytes,
    outer_two_arg, decrease_new, decrease_old, take_pair, DEADLINE,
)

def build_strict(params, actions_hex):
    actions_len_padded = ((len(actions_hex) + 63) // 64) * 32
    params_offset = 0x40 + 32 + actions_len_padded
    unlock = u256(0x40) + u256(params_offset) + encode_bytes_field(actions_hex) + encode_bytes_array(params)
    return outer_two_arg(unlock)

def build_packed(params, actions_hex):
    unlock = "".join(format(b, "02x") + p for b, p in zip(ab_bytes(actions_hex), params))
    return outer_two_arg(encode_bytes_field(unlock))

def take_single(currency, recipient, amount):
    return encode_tuple(addr(currency), addr(recipient), u256(amount))

def take_all(currency, recipient):
    return encode_tuple(addr(currency), addr(recipient))

def run(name, data):
    st, res = call(POSM, data, WALLET)
    ok = st == "OK" and isinstance(res, str)
    print(f"{name}: {'SUCCESS' if ok else 'revert ' + json.dumps(res)[:140]}")
    return ok

print("== bisect: DECREASE only ==")
run("strict DECREASE-only OLD tuple", build_strict([decrease_old(TOKEN_ID, DEADLINE)], "01"))
run("strict DECREASE-only NEW tuple", build_strict([decrease_new(TOKEN_ID)], "01"))
run("packed  DECREASE-only OLD tuple", build_packed([decrease_old(TOKEN_ID, DEADLINE)], "01"))
run("packed  DECREASE-only NEW tuple", build_packed([decrease_new(TOKEN_ID)], "01"))

print("== bisect: TAKE variants (no decrease; expect revert if zero credit) ==")
run("strict TAKE 0x0e single-amount=1", build_strict([take_single(USDC, WALLET, 1)], "0e"))
run("strict TAKE_ALL 0x0f", build_strict([take_all(USDC, WALLET)], "0f"))
run("strict TAKE_PAIR 0x07 old", build_strict([take_pair(WALLET, USDC, NOCK)], "07"))
run("strict TAKE_PAIR 0x11 new", build_strict([take_pair(WALLET, USDC, NOCK)], "11"))

print("== bisect: DECREASE + TAKE singles (pairs) ==")
for name, actions, params in [
    ("dec-old + TAKE0x0e", "010e", [decrease_old(TOKEN_ID, DEADLINE), take_single(USDC, WALLET, 1)]),
    ("dec-old + TAKE0x0e (nock)", "010e", [decrease_old(TOKEN_ID, DEADLINE), take_single(NOCK, WALLET, 1)]),
    ("dec-new + TAKE0x0e", "010e", [decrease_new(TOKEN_ID), take_single(USDC, WALLET, 1)]),
    ("dec-old + TAKE_ALL0x0f", "010f", [decrease_old(TOKEN_ID, DEADLINE), take_all(USDC, WALLET)]),
    ("dec-new + TAKE_ALL0x0f", "010f", [decrease_new(TOKEN_ID), take_all(USDC, WALLET)]),
]:
    run("strict " + name, build_strict(params, actions))

print("== full combos again ==")
run("strict dec-old + TAKE_PAIR 0x07", build_strict(
    [decrease_old(TOKEN_ID, DEADLINE), take_pair(WALLET, USDC, NOCK)], "0107"))
run("strict dec-new + TAKE_PAIR 0x11", build_strict(
    [decrease_new(TOKEN_ID), take_pair(WALLET, USDC, NOCK)], "0111"))
run("strict dec-old + TAKE_PAIR 0x11", build_strict(
    [decrease_old(TOKEN_ID, DEADLINE), take_pair(WALLET, USDC, NOCK)], "0111"))
run("strict dec-new + TAKE_PAIR 0x07", build_strict(
    [decrease_new(TOKEN_ID), take_pair(WALLET, USDC, NOCK)], "0107"))
