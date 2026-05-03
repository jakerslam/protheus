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
        "What kind of work is this? What kind of work is this?",
        "What kind of work is this? What kind of work is this?",
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

#[test]
fn live_eval_monitor_flags_required_web_tool_without_attempt() {
    let root = governance_temp_root();
    let report = live_eval_monitor_turn(
        root.path(),
        "misty-required-tool-eval",
        "Use web search to find one current source.",
        "I would choose the web search tool.",
        "",
        "",
        &json!({
            "final_llm_response": {"status": "synthesized"},
            "web_invariant": {
                "requires_live_web": true,
                "tool_attempted": false,
                "failure_code": "web_route_parse_failed"
            }
        }),
    );
    let classes = live_eval_issue_classes(&report);
    assert!(
        classes
            .iter()
            .any(|class| class == "required_tool_without_attempt"),
        "{classes:?}"
    );
    assert_eq!(
        report
            .get("chat_injection_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn live_eval_monitor_flags_unresolved_tool_intent_final_answer() {
    let root = governance_temp_root();
    let report = live_eval_monitor_turn(
        root.path(),
        "misty-unresolved-tool-intent-eval",
        "compare infring to top agentic frameworks",
        "I would choose to run a batch_query to collect external evidence about top agentic frameworks for comparison.",
        "",
        "",
        &json!({"final_llm_response": {"status": "synthesized"}}),
    );
    let classes = live_eval_issue_classes(&report);
    assert!(
        classes
            .iter()
            .any(|class| class == "unresolved_tool_intent_final"),
        "{classes:?}"
    );
    assert_eq!(
        report
            .get("chat_injection_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn live_eval_monitor_flags_unsupported_low_signal_tool_claim_without_evidence() {
    let guard = final_response_guard_report(
        "compare infring to top agentic frameworks",
        "Live web retrieval was low-signal in this turn. Provisional comparison: Infring is strongest in identity persistence.",
        &[],
        false,
    );
    assert_eq!(
        guard
            .get("unsupported_tool_success_claim")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        guard
            .pointer("/contamination_guard/current_turn_tool_evidence")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn runtime_system_prompt_uses_workflow_gate_not_direct_repo_tool_claims() {
    let prompt = AGENT_RUNTIME_SYSTEM_PROMPT.to_ascii_lowercase();
    assert!(prompt.contains("workflow gates"), "{prompt}");
    assert!(!prompt.contains("workspace files"), "{prompt}");
    assert!(!prompt.contains("<function="), "{prompt}");
    assert!(!prompt.contains("use those capabilities directly"), "{prompt}");
}

#[test]
fn live_eval_monitor_flags_short_stale_code_dump() {
    let root = governance_temp_root();
    let report = live_eval_monitor_turn(
        root.path(),
        "misty-stale-code-eval",
        "Use web search to find one current source.",
        "<?php\nnamespace app\\api\\model;\nclass ProductProperty extends BaseModel\n{\n    protected $hidden = ['product_id'];\n}",
        "",
        "",
        &json!({"final_llm_response": {"status": "synthesized"}}),
    );
    let classes = live_eval_issue_classes(&report);
    assert!(
        classes
            .iter()
            .any(|class| class == "visible_stale_code_context_dump"),
        "{classes:?}"
    );
}
