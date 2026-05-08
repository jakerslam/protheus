fn latent_tool_candidates_preserve_explicit_live_web_research_turns() {
    let candidates = latent_tool_candidates_for_message(
        "try to web search \"top AI agent frameworks\"",
        &[],
    );
    assert_eq!(
        candidates.len(),
        1,
        "{candidates:?}"
    );
    assert_eq!(
        candidates[0].get("tool").and_then(Value::as_str),
        Some("batch_query")
    );
}

#[test]
