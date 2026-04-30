// SRS: V13-WORKFLOW-GATE-003

fn live_eval_issue_classes(report: &Value) -> Vec<String> {
    report
        .get("issues")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|issue| {
            issue
                .pointer("/raw_event/issue_class")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .collect()
}

#[test]
fn live_eval_monitor_flags_workflow_failure_modes_without_chat_injection() {
    let root = governance_temp_root();
    let config_dir = root.path().join("local/state/ops/eval_live_monitor");
    std::fs::create_dir_all(&config_dir).expect("config dir");
    write_json(&config_dir.join("config.json"), &json!({"enabled": true}));

    let report = live_eval_monitor_turn(
        root.path(),
        "misty-workflow-eval",
        "use tools if needed",
        "Need tools? Yes/No Need tools? Yes/No",
        "Need tools? Yes/No Need tools? Yes/No",
        "",
        &json!({
            "workflow_system_fallback_used": true,
            "duration_ms": 45_000,
            "pending_tool_request": {"status": "pending_confirmation"},
            "final_llm_response": {
                "status": "skipped",
                "attempt_count": 2,
                "fallback_guard_multi_stage": true
            },
            "tool_completion": {
                "status": "ok",
                "tool_attempts": [{"name": "batch_query", "status": "ok"}]
            }
        }),
    );

    let classes = live_eval_issue_classes(&report);
    for expected in [
        "repeated_gate_prompt",
        "gate_token_leakage",
        "system_fallback_in_chat",
        "hidden_second_pass_call",
        "pending_tool_stuck_too_long",
        "tool_result_without_synthesis",
    ] {
        assert!(
            classes.iter().any(|class| class == expected),
            "missing {expected}: {classes:?}"
        );
    }
    assert_eq!(
        report
            .get("chat_injection_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn live_eval_monitor_flags_empty_direct_reply() {
    let root = governance_temp_root();
    let report = live_eval_monitor_turn(
        root.path(),
        "misty-empty-eval",
        "hey",
        "",
        "",
        "",
        &json!({"final_llm_response": {"status": "synthesized"}}),
    );
    let classes = live_eval_issue_classes(&report);
    assert!(
        classes.iter().any(|class| class == "empty_direct_reply"),
        "{classes:?}"
    );
    assert_eq!(
        report
            .get("chat_injection_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
}
