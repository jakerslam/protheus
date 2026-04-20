fn latent_tool_candidates_normalize_try_to_web_search_query() {
    let candidates = latent_tool_candidates_for_message(
        "try to web search \"top AI agent frameworks\"",
        &[],
    );
    let batch = candidates
        .iter()
        .find(|row| row.get("tool").and_then(Value::as_str) == Some("batch_query"))
        .cloned()
        .expect("batch query candidate");
    assert_eq!(
        batch.pointer("/proposed_input/query").and_then(Value::as_str),
        Some("top AI agent frameworks")
    );
}

#[test]
