fn rewrite_workspace_analyze_raw_json_result_into_local_evidence_summary() {
    let rewritten = rewrite_tool_result_for_user_summary(
        "workspace_analyze",
        "Key findings: {\"stdout\":\"docs/workspace/SRS.md:42: response_workflow\\ndocs/workspace/SRS.md:43: complex_prompt_chain_v1\\nclient/runtime/config/runtime.json:4: resident IPC\"}",
    )
    .unwrap_or_default();
    assert!(rewritten.contains("Local workspace evidence"), "{rewritten}");
    assert!(rewritten.contains("response_workflow"), "{rewritten}");
    assert!(!rewritten.contains("{\"stdout\""), "{rewritten}");
}

#[test]
