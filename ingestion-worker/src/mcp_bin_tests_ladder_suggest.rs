//! stdio MCP binary: the ladder suggestion (issue #168) in get_shot_advice. Included from
//! src/bin/mcp.rs. Parity is checked against the text the worker's own golden test wrote
//! (testdata/ladder_suggest/golden_ladder_suggest.json), through the stdio's own
//! format_shot_advice path.

use super::*;

fn golden() -> Value {
    serde_json::from_str(include_str!("../testdata/ladder_suggest/golden_ladder_suggest.json")).expect("the golden file parses")
}

#[test]
fn test_stdio_prints_the_ladder_suggestion_the_worker_prints() {
    let g = golden();
    assert!(g.as_object().unwrap().len() >= 5);
    for (case, v) in g.as_object().unwrap() {
        let data = json!({ "derived_metrics": { "ladder_suggestion": v["ladder_suggestion"] } });
        let health = v["health"].as_str();
        let expected = v["text"].as_str().unwrap();
        assert_eq!(ladder_suggest_text::ladder_suggestion_lines(&data, health), expected, "{case}");
        // The stdio's get_shot_advice carries it after the directive, as the worker's does.
        let advice = format_shot_advice(&data);
        let full = ladder_suggest_text::with_ladder_suggestion(&advice, &data, health);
        assert_eq!(full, format!("{advice}\n{}", expected.trim_end()), "{case}");
    }
    assert!(g["ready"]["text"].as_str().unwrap().contains("Deposit 1 of 1: "));
    assert!(g["ready_stale"]["text"].as_str().unwrap().contains("not fresh"));
}

/// The same parity for a configured shape (the operator's bottle of three deposits): the
/// text tests_ladder_bottle.rs wrote for the worker, through the stdio's own modules.
#[test]
fn test_stdio_prints_the_bottle_the_worker_prints() {
    let g: Value = serde_json::from_str(include_str!("../testdata/ladder_suggest/golden_ladder_bottle.json")).expect("the golden file parses");
    assert!(g.as_object().unwrap().len() >= 9);
    for (case, v) in g.as_object().unwrap() {
        let data = json!({ "derived_metrics": { "ladder_suggestion": v["ladder_suggestion"] } });
        let health = v["health"].as_str();
        let expected = v["text"].as_str().unwrap();
        assert_eq!(ladder_suggest_text::ladder_suggestion_lines(&data, health), expected, "{case}");
        let advice = format_shot_advice(&data);
        let full = ladder_suggest_text::with_ladder_suggestion(&advice, &data, health);
        assert_eq!(full, format!("{advice}\n{}", expected.trim_end()), "{case}");
    }
    // The three deposits, numbered the same in both transports, with the cap's limit.
    let bottle = g["bottle"]["text"].as_str().unwrap();
    for needle in [
        "Deposit 1 of 3 (cap): about 86.79 NOCK + $1.22 USDC, in range now.",
        "Deposit 2 of 3 (near rung): $114.86 USDC, no NOCK, below the price now.",
        "Deposit 3 of 3 (deep rung): $650.89 USDC, no NOCK, below the price now.",
        "The NOCK side must not exceed 86.7891 NOCK, your LP NOCK budget, which binds now;",
        "After these deposits your wallet holds about $779 USDC;",
        "Exact mint: liquidity 14125167068081104, 650899400 raw USDC and 0 raw NOCK.",
    ] {
        assert!(bottle.contains(needle), "{needle}");
    }
    assert_eq!(bottle.matches("Fees while in range (an estimate): $").count(), 3);
    assert!(g["bottle_stale"]["text"].as_str().unwrap().contains("not fresh"));
    assert!(g["deep_deployed"]["text"].as_str().unwrap().contains("Deposit 3 of 3 (deep rung): done."));
    assert!(g["all_deployed"]["text"].as_str().unwrap().contains("Ladder deployed: all 3 deposits are done."));
    assert!(g["near_passed"]["text"].as_str().unwrap().contains("Deposit 2 of 3 (near rung): passed."));
}

#[test]
fn test_stdio_tools_list_names_the_ladder_suggestion() {
    let src = include_str!("bin/mcp.rs");
    assert!(src.contains("Also states the ladder suggestion (#168)"));
    let worker = include_str!("mcp_http.rs");
    assert!(worker.contains("Also states the ladder suggestion (#168)"));
}

#[test]
fn test_an_older_snapshot_prints_no_ladder_section() {
    let data = json!({ "derived_metrics": {} });
    assert_eq!(ladder_suggest_text::ladder_suggestion_lines(&data, Some("ok")), "");
    assert_eq!(ladder_suggest_text::with_ladder_suggestion("advice", &data, Some("ok")), "advice");
}
