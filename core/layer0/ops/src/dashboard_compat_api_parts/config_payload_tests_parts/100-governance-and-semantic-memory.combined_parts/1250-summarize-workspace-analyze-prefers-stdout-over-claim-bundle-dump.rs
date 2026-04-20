fn summarize_workspace_analyze_prefers_stdout_over_claim_bundle_dump() {
    let summary = summarize_tool_payload(
        "workspace_analyze",
        &json!({
            "ok": true,
            "stdout": "docs/workspace/SRS.md:42: response_workflow\\ndocs/workspace/SRS.md:43: complex_prompt_chain_v1\\nclient/runtime/config/runtime.json:4: resident IPC",
            "tool_pipeline": {
                "claim_bundle": {
                    "claims": [
                        {
                            "status": "supported",
                            "text": "{\"command_translated\":false,\"cwd\":\"/Users/jay/.openclaw/workspace\",\"executed_command\":\"rg -n ...\"}"
                        }
                    ]
                }
            }
        }),
    );
    assert!(summary.contains("response_workflow"), "{summary}");
    assert!(summary.contains("complex_prompt_chain_v1"), "{summary}");
    assert!(!summary.contains("{\"command_translated\""), "{summary}");
}

#[test]
