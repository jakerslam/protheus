fn summarize_unknown_workspace_analyze_payload_prefers_stdout_findings() {
    let summary = summarize_unknown_tool_payload(
        "workspace_analyze",
        &json!({
            "ok": true,
            "stdout": "docs/workspace/SRS.md:42: response_workflow\ndocs/workspace/SRS.md:43: complex_prompt_chain_v1\nclient/runtime/config/runtime.json:4: resident IPC"
        }),
    );
    assert!(summary.contains("response_workflow"), "{summary}");
    assert!(summary.contains("complex_prompt_chain_v1"), "{summary}");
    assert!(!summary.contains("`workspace_analyze` completed"), "{summary}");
}

#[test]
