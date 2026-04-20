fn append_turn_message_captures_explicit_remember_fact_for_long_term_memory() {
    let root = governance_temp_root();
    let captured =
        parse_memory_capture_text("remember this: the fallback phrase is cobalt sunrise")
            .expect("memory capture text");
    assert!(captured.to_ascii_lowercase().contains("cobalt sunrise"));
    assert!(parse_memory_capture_text("just answer normally").is_none());

    let _ = crate::dashboard_agent_state::memory_kv_set(
        root.path(),
        "agent-memory-capture",
        "explicit_memory.test",
        &json!({"text": captured, "captured_at": crate::now_iso()}),
    );
    let memory_state = crate::dashboard_agent_state::memory_kv_semantic_query(
        root.path(),
        "agent-memory-capture",
        "cobalt sunrise",
        5,
    );
    assert!(memory_state
        .get("matches")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));
}

#[test]
