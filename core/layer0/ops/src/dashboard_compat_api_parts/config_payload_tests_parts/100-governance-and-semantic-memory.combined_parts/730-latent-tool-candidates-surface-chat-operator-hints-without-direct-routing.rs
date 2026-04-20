fn latent_tool_candidates_surface_chat_operator_hints_without_direct_routing() {
    let slash_candidates = latent_tool_candidates_for_message("/search top AI agentic frameworks", &[]);
    assert!(slash_candidates.iter().any(|row| {
        row.get("tool").and_then(Value::as_str) == Some("batch_query")
            && row.get("selection_source").and_then(Value::as_str) == Some("slash_search_hint")
    }));

    let explicit_candidates =
        latent_tool_candidates_for_message("tool::fetch:::https://example.com", &[]);
    assert!(explicit_candidates.iter().any(|row| {
        row.get("tool").and_then(Value::as_str) == Some("web_fetch")
            && row.get("selection_source").and_then(Value::as_str)
                == Some("explicit_tool_command")
    }));
}

#[test]
