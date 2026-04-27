#[test]
fn spawn_tool_persists_objective_scoped_context_slice_on_child_contract() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let parent_id = "agent-parent-slice";
    let mut parent_state = default_session_state(parent_id);
    let mut messages = vec![
        json!({"role":"user","text":"Please diagnose the file workflow duplicate response rendering failure.","ts":"2026-04-26T00:00:01Z"}),
        json!({"role":"assistant","text":"The file workflow issue may involve synthesis finalization.","ts":"2026-04-26T00:00:02Z"}),
    ];
    for idx in 0..14 {
        messages.push(json!({
            "role": if idx % 2 == 0 { "user" } else { "assistant" },
            "text": format!("generic unrelated status update {idx}"),
            "ts": format!("2026-04-26T00:00:{:02}Z", idx + 3)
        }));
    }
    parent_state["sessions"][0]["messages"] = Value::Array(messages);
    save_session_state(root.path(), parent_id, &parent_state);

    let out = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        parent_id,
        None,
        "spawn_subagents",
        &json!({
            "count": 1,
            "objective": "diagnose file workflow duplicate response rendering",
            "merge_strategy": "reduce",
            "budget_tokens": 800
        }),
    );
    assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
    let child_id = out
        .pointer("/children/0/agent_id")
        .and_then(Value::as_str)
        .expect("spawned child id");
    let contracts = contracts_map(root.path());
    let child_contract = contracts.get(child_id).expect("child contract persisted");
    assert_eq!(
        child_contract.get("merge_strategy").and_then(Value::as_str),
        Some("reduce")
    );
    assert!(child_contract.get("spawn_guard").and_then(Value::as_object).is_some());
    let context_slice = child_contract.get("context_slice").expect("context slice persisted");
    assert_eq!(
        context_slice.get("strategy").and_then(Value::as_str),
        Some("objective_scoped_recent_window")
    );
    let has_workflow_token = context_slice
        .get("objective_tokens")
        .and_then(Value::as_array)
        .map(|tokens| tokens.iter().any(|token| token.as_str() == Some("workflow")))
        .unwrap_or(false);
    assert!(has_workflow_token);
    let selected = context_slice
        .get("selected_messages")
        .and_then(Value::as_array)
        .expect("selected messages");
    assert!(selected.len() <= 12);
    assert!(selected.len() < 16);
    assert!(selected.iter().any(|row| {
        row.get("text").and_then(Value::as_str).unwrap_or("").contains("file workflow")
    }));
}
