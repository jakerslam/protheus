#[test]
fn red_tier_tools_require_explicit_signoff() {
    let root = tempfile::tempdir().expect("tempdir");
    let snapshot = json!({"ok": true});
    let blocked = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        "agent-red",
        None,
        "terminal_exec",
        &json!({"command":"echo hi"}),
    );
    assert_eq!(
        blocked.get("error").and_then(Value::as_str),
        Some("tool_explicit_signoff_required")
    );

    let allowed = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        "agent-red",
        None,
        "terminal_exec",
        &json!({
            "command":"echo hi",
            "confirm": true,
            "approval_note": "user approved this terminal execution"
        }),
    );
    assert_ne!(
        allowed.get("error").and_then(Value::as_str),
        Some("tool_explicit_signoff_required")
    );
}

#[test]
fn semantic_memory_query_route_returns_matches() {
    let root = tempfile::tempdir().expect("tempdir");
    let snapshot = json!({"ok": true});
    let _ = crate::dashboard_agent_state::memory_kv_set(
        root.path(),
        "agent-memory",
        "fact.auth.flow",
        &json!("PKCE callback must include nonce binding."),
    );

    let body = serde_json::to_vec(&json!({"query":"auth nonce", "limit": 5})).expect("serialize");
    let response = handle(
        root.path(),
        "POST",
        "/api/memory/agents/agent-memory/semantic-query",
        &body,
        &snapshot,
    )
    .expect("semantic query");
    assert_eq!(response.status, 200);
    assert_eq!(
        response
            .payload
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        true
    );
    assert!(response
        .payload
        .get("matches")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));
}

#[test]
fn spawn_tool_applies_budget_circuit_breaker_and_merge_strategy() {
    let root = tempfile::tempdir().expect("tempdir");
    let snapshot = json!({"ok": true});
    let out = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        "agent-parent",
        None,
        "spawn_subagents",
        &json!({
            "count": 8,
            "objective": "Parallelize a large analysis task",
            "merge_strategy": "voting",
            "budget_tokens": 1_000_000,
            "confirm": true,
            "approval_note": "user requested bounded spawn for analysis"
        }),
    );
    let effective = out
        .get("effective_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    assert!(effective <= 1);
    assert_eq!(
        out.pointer("/directive/merge_strategy")
            .and_then(Value::as_str)
            .unwrap_or(""),
        "voting"
    );
    assert!(out
        .pointer("/circuit_breakers/degraded")
        .and_then(Value::as_bool)
        .unwrap_or(false));
}

#[test]
fn web_tool_fallback_can_use_semantic_memory_matches() {
    let root = tempfile::tempdir().expect("tempdir");
    let _ = crate::dashboard_agent_state::memory_kv_set(
        root.path(),
        "agent-fallback",
        "fact.verity",
        &json!("Verity plane enforces fidelity receipts and drift checks."),
    );
    let fallback = fallback_memory_query_payload(
        root.path(),
        "agent-fallback",
        "web_search",
        &json!({"query":"verity drift checks"}),
    )
    .expect("fallback payload");
    assert_eq!(
        fallback.get("fallback_used").and_then(Value::as_bool),
        Some(true)
    );
    assert!(fallback
        .get("matches")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));
}
