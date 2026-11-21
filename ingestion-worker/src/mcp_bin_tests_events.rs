//! stdio MCP binary: the calls to action and the ladder maintenance (issue #169) in
//! get_shot_advice. Included from src/bin/mcp.rs. Parity is checked against the text the
//! worker's own golden test wrote (testdata/ladder_maintenance/golden_events_text.json),
//! through the stdio's own format_shot_advice path.

use super::*;

#[test]
fn test_stdio_prints_the_events_the_worker_prints() {
    let g: Value = serde_json::from_str(include_str!("../testdata/ladder_maintenance/golden_events_text.json")).expect("the golden file parses");
    assert!(g.as_object().unwrap().len() >= 4);
    for (case, v) in g.as_object().unwrap() {
        let data = &v["data"];
        let health = v["health"].as_str();
        let expected = v["text"].as_str().unwrap();
        assert_eq!(events_text::events_lines(data, health), expected, "{case}");
        let advice = format_shot_advice(data);
        let full = events_text::with_events(&advice, data, health);
        assert_eq!(full, format!("{advice}\n{}", expected.trim_end()), "{case}");
    }
    assert!(g["near_move"]["text"].as_str().unwrap().contains("LADDER: MOVE THE NEAR RUNG"));
}

#[test]
fn test_stdio_composes_events_inside_the_ladder_suggestion() {
    let src = include_str!("bin/mcp.rs");
    let events = src.find("events_text::with_events(&format_shot_advice(data), data, health.as_deref())").unwrap();
    let suggest = src.find("ladder_suggest_text::with_ladder_suggestion(").unwrap();
    assert!(suggest < events, "the events wrap the advice inside the ladder suggestion's wrapper");
}
