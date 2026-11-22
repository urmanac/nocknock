//! stdio MCP binary: the WETH/NOCK bet card (issue #181) in get_shot_advice. Included from
//! src/bin/mcp.rs. Parity is checked against the text the worker's own golden test wrote
//! (testdata/bets/golden_bet_suggestion.json), through the stdio's own format_shot_advice path.

use super::*;

fn golden() -> Value {
    serde_json::from_str(include_str!("../testdata/bets/golden_bet_suggestion.json")).expect("the golden file parses")
}

#[test]
fn test_stdio_prints_the_bet_card_the_worker_prints() {
    let g = golden();
    assert!(g.as_object().unwrap().len() >= 5);
    for (case, v) in g.as_object().unwrap() {
        let data = json!({ "derived_metrics": { "bet_suggestion": v["bet_suggestion"] } });
        let health = v["health"].as_str();
        let expected = v["text"].as_str().unwrap();
        assert_eq!(bet_suggest_text::bet_suggestion_lines(&data, health), expected, "{case}");
        let advice = format_shot_advice(&data);
        let full = bet_suggest_text::with_bet_suggestion(&advice, &data, health);
        assert_eq!(full, format!("{advice}\n{}", expected.trim_end()), "{case}");
    }
    assert!(g["ready"]["text"].as_str().unwrap().contains("Amounts to enter in the Aero app: "));
    assert!(g["ready"]["text"].as_str().unwrap().contains("https://app.aero.xyz/deposit/8453-"));
    assert!(g["ready_stale"]["text"].as_str().unwrap().contains("not fresh"));
}

#[test]
fn test_stdio_without_a_bet_card_prints_nothing_more() {
    let data = json!({ "derived_metrics": {} });
    assert_eq!(bet_suggest_text::bet_suggestion_lines(&data, Some("ok")), "");
    let advice = format_shot_advice(&data);
    assert_eq!(bet_suggest_text::with_bet_suggestion(&advice, &data, Some("ok")), advice);
}
