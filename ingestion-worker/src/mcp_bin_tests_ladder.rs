//! stdio MCP binary: the ladder simulator (issue #166). Included from src/bin/mcp.rs.
//! This binary cannot run the worker's simulation, and never computes one: its fallback
//! formats the worker's own GET /v1/simulate/ladder body with the same ladder_text.rs.
//! Parity is checked against what the worker's tests wrote into
//! testdata/ladder/golden_ladder.json: the bodies and the HTTP transport's text.

use super::*;

fn golden() -> Value {
    serde_json::from_str(include_str!("../testdata/ladder/golden_ladder.json")).expect("the golden file parses")
}

#[test]
fn test_fallback_prints_the_text_the_http_transport_prints() {
    let g = golden();
    let mut ok = 0;
    let mut errors = 0;
    for (case, v) in g.as_object().unwrap() {
        let Some(status) = v.get("status").and_then(|s| s.as_u64()) else { continue };
        let (text, is_error) = ladder_fallback_text(status as u16, &v["body"]).expect("an endpoint answer");
        assert_eq!(text, v["mcp_text"].as_str().unwrap(), "{case}");
        assert_eq!(is_error, status != 200, "{case}");
        if is_error { errors += 1 } else { ok += 1 }
    }
    assert!(ok >= 4 && errors >= 3, "{ok} ok, {errors} errors");
    // 503 is the worker's own no-snapshot text; anything else is not the endpoint's.
    let (t, e) = ladder_fallback_text(503, &json!({"error": "NO_SNAPSHOT", "detail": "boom"})).unwrap();
    assert_eq!((t.as_str(), e), ("No snapshot available yet. Last ingestion error: boom", true));
    assert_eq!(ladder_fallback_text(502, &json!({})), None);
}

#[test]
fn test_stdio_lists_the_same_tool_and_forwards_arguments_and_is_error() {
    assert_eq!(ladder_text::simulate_ladder_tool(), golden()["tool"]);
    let args = json!({"usdc": 741, "low": 0.01, "high_at_spot": true});
    let call = forwarded_call("simulate_ladder", Some(&args));
    assert_eq!(call["params"]["name"], "simulate_ladder");
    assert_eq!(call["params"]["arguments"], args);
    // The other tools still send no arguments.
    assert!(forwarded_call("get_portfolio_state", None)["params"].get("arguments").is_none());
    let answer = json!({"result": {"content": [{"type": "text", "text": "x"}], "isError": true}});
    assert_eq!(forwarded_result(&answer), Some(("x".to_string(), true)));
    let answer = json!({"result": {"content": [{"type": "text", "text": "y"}]}});
    assert_eq!(forwarded_result(&answer), Some(("y".to_string(), false)));
    // The fallback's query is the endpoint's own parameters.
    let q = ladder_text::ladder_query_from_args(Some(&args)).unwrap();
    assert_eq!(q, vec![("usdc".into(), "741".into()), ("low".into(), "0.01".into()), ("high".into(), "spot".into())]);
    // Unreachable: said so, and nothing computed.
    let text = zero_db_fallback_text("simulate_ladder", "http://127.0.0.1:1");
    assert!(text.starts_with("🧮 Ladder Simulation (worker unreachable)"), "{text}");
    assert!(text.contains("Nothing is reported that the worker has not read"));
}

/// Ladders of rungs and the configured shape (2.10.0): the same file, the same text.
#[test]
fn test_fallback_prints_the_rung_texts_the_http_transport_prints() {
    let g: Value = serde_json::from_str(include_str!("../testdata/ladder/golden_ladder_rungs.json")).expect("the golden file parses");
    let mut cases = 0;
    for (case, v) in g.as_object().unwrap() {
        let Some(status) = v.get("status").and_then(|s| s.as_u64()) else { continue };
        let (text, is_error) = ladder_fallback_text(status as u16, &v["body"]).expect("an endpoint answer");
        assert_eq!(text, v["mcp_text"].as_str().unwrap(), "{case}");
        assert_eq!(is_error, status != 200, "{case}");
        // The stdio fallback sends the rungs array as the endpoint's own indexed query.
        let q = ladder_text::ladder_query_from_args(Some(&v["arguments"])).expect("valid arguments");
        assert!(!q.is_empty(), "{case}");
        if v["arguments"].get("rungs").is_some() {
            assert!(q.iter().any(|(k, _)| k == "r1_usdc"), "{case}: {q:?}");
        }
        cases += 1;
    }
    assert!(cases >= 6, "{cases}");
}
