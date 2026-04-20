fn compare_platform_wording_clusters_workspace_and_web_tools() {
    let hints = latent_tool_candidates_for_message("compare this platform to openclaw", &[]);
    let tool_names = hints
        .iter()
        .filter_map(|row| row.get("tool").and_then(Value::as_str))
        .collect::<Vec<_>>();
    assert!(tool_names.contains(&"workspace_analyze"), "{tool_names:?}");
    assert!(tool_names.contains(&"batch_query"), "{tool_names:?}");
}

#[test]
