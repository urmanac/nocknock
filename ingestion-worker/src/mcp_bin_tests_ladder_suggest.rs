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
