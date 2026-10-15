#!/usr/bin/env python3
"""
NockNock MCP Playtesting & Verification Suite
Tests both STDIO and Stateless HTTP transports according to the Model Context Protocol (2024-11-05).
"""

import sys
import json
import subprocess
import urllib.request
import urllib.error

API_URL = os.environ.get("NOCKNOCK_API_URL", f"http://127.0.0.1:{os.environ.get('PORT', '8081')}/v1/mcp")

def test_stdio_mcp():
    print("==================================================")
    print("1. Playtesting STDIO MCP Server (`nocknock-mcp`)")
    print("==================================================")

    binary_path = "./ingestion-worker/target/release/nocknock-mcp"
    try:
        proc = subprocess.Popen(
            [binary_path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )
    except FileNotFoundError:
        print(f"❌ Binary not found at {binary_path}. Run `make build-worker` first.")
        return False

    requests = [
        {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}},
        {"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}},
        {"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "get_portfolio_state"}},
        {"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "get_shot_advice"}},
        {"jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": {"name": "simulate_claim_fees"}},
    ]

    input_payload = "\n".join(json.dumps(r) for r in requests) + "\n"
    stdout, stderr = proc.communicate(input=input_payload, timeout=10)

    lines = [l.strip() for l in stdout.strip().split("\n") if l.strip()]
    if len(lines) < len(requests):
        print(f"❌ Expected {len(requests)} responses, got {len(lines)}")
        if stderr:
            print("STDERR:", stderr)
        return False

    responses = [json.loads(l) for l in lines]

    # Verify initialize
    init_res = responses[0].get("result", {})
    assert init_res.get("protocolVersion") == "2024-11-05", "Protocol version mismatch"
    assert init_res.get("serverInfo", {}).get("name") == "nocknock-mcp", "Server name mismatch"
    print("  ✓ STDIO initialize handshake passed")

    # Verify tools/list
    tools = responses[1].get("result", {}).get("tools", [])
    tool_names = [t["name"] for t in tools]
    assert "get_portfolio_state" in tool_names, "Missing get_portfolio_state"
    assert "get_shot_advice" in tool_names, "Missing get_shot_advice"
    assert "simulate_claim_fees" in tool_names, "Missing simulate_claim_fees"
    print(f"  ✓ STDIO tools/list returned {len(tools)} tools: {', '.join(tool_names)}")

    # Verify tool calls
    state_text = responses[2]["result"]["content"][0]["text"]
    assert "NockNock Sovereign Portfolio State" in state_text or "NockNock Sovereign State" in state_text
    print("  ✓ STDIO get_portfolio_state call passed")

    advice_text = responses[3]["result"]["content"][0]["text"]
    assert "Tactical Shot Advice Engine" in advice_text
    print("  ✓ STDIO get_shot_advice call passed")

    fees_text = responses[4]["result"]["content"][0]["text"]
    assert "Fee Claim Simulation" in fees_text
    print("  ✓ STDIO simulate_claim_fees call passed")

    return True

def test_http_mcp():
    print("\n==================================================")
    print("2. Playtesting Stateless HTTP MCP (`/v1/mcp`)")
    print("==================================================")

    def post_mcp(method, params=None, id_val=1):
        payload = {"jsonrpc": "2.0", "id": id_val, "method": method}
        if params:
            payload["params"] = params
        req = urllib.request.Request(
            API_URL,
            data=json.dumps(payload).encode("utf-8"),
            headers={"Content-Type": "application/json"}
        )
        try:
            with urllib.request.urlopen(req, timeout=5) as resp:
                return json.loads(resp.read().decode("utf-8"))
        except urllib.error.URLError as e:
            print(f"  ⚠️ HTTP request failed: {e}. Is the worker running on port 8081?")
            return None

    init_res = post_mcp("initialize")
    if not init_res:
        return False
    assert init_res.get("result", {}).get("serverInfo", {}).get("name") == "nocknock-mcp"
    print("  ✓ HTTP /v1/mcp initialize handshake passed")

    tools_res = post_mcp("tools/list")
    tools = tools_res.get("result", {}).get("tools", [])
    assert len(tools) >= 3
    print(f"  ✓ HTTP /v1/mcp tools/list returned {len(tools)} tools")

    state_res = post_mcp("tools/call", {"name": "get_portfolio_state"})
    assert "NockNock Sovereign" in state_res["result"]["content"][0]["text"]
    print("  ✓ HTTP /v1/mcp get_portfolio_state passed")

    advice_res = post_mcp("tools/call", {"name": "get_shot_advice"})
    assert "Tactical Shot Advice" in advice_res["result"]["content"][0]["text"]
    print("  ✓ HTTP /v1/mcp get_shot_advice passed")

    claim_res = post_mcp("tools/call", {"name": "simulate_claim_fees"})
    claim_text = claim_res["result"]["content"][0]["text"]
    assert "Fee Claim Simulation" in claim_text
    print("  ✓ HTTP /v1/mcp simulate_claim_fees passed")

    return True

def main():
    stdio_ok = test_stdio_mcp()
    http_ok = test_http_mcp()

    print("\n==================================================")
    if stdio_ok and http_ok:
        print("🎉 ALL MCP PLAYTEST CHECKS PASSED (STDIO & HTTP)")
        print("==================================================")
        sys.exit(0)
    else:
        print("⚠️ SOME MCP CHECKS FAILED OR SKIPPED")
        print("==================================================")
        sys.exit(1)

if __name__ == "__main__":
    main()
