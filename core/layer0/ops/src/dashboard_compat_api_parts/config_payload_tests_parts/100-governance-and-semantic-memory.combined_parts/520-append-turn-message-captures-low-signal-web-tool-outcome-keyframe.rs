fn append_turn_message_captures_low_signal_web_tool_outcome_keyframe() {
    let root = governance_temp_root();
    let receipt = append_turn_message(
        root.path(),
        "agent-web-keyframe",
        "try doing a generic search \"top AI agent frameworks\"",
        "The batch query step ran, but only low-signal web output came back. Retry with a narrower query, one specific source URL, or ask me to continue from the recorded tool result.",
    );
    assert_eq!(
        receipt
            .pointer("/tool_outcome_keyframe/tool")
            .and_then(Value::as_str),
        Some("batch_query")
    );
    let context = context_command_payload(
        root.path(),
        "agent-web-keyframe",
        &json!({}),
        &json!({}),
        true,
    );
    let outcomes = context
        .get("recent_tool_outcomes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(outcomes.iter().any(|entry| {
        entry.get("summary")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase()
            .contains("top ai agent frameworks")
    }));
}

#[test]
