fn governance_temp_root() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

fn governance_ok_snapshot() -> Value {
    json!({"ok": true})
}

fn governance_test_chat_script_path(root: &Path) -> PathBuf {
    state_path(
        root,
        "client/runtime/local/state/ui/infring_dashboard/test_chat_script.json",
    )
}

fn governance_test_tool_script_path(root: &Path) -> PathBuf {
    state_path(
        root,
        "client/runtime/local/state/ui/infring_dashboard/test_tool_script.json",
    )
}

#[test]
fn terminal_tools_run_without_signoff_and_still_enforce_command_policy() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let allowed = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        "agent-terminal",
        None,
        "terminal_exec",
        &json!({"command":"echo hi"}),
    );
    assert_ne!(
        allowed.get("error").and_then(Value::as_str),
        Some("tool_explicit_signoff_required")
    );
    let allow_verdict = allowed
        .pointer("/permission_gate/verdict")
        .and_then(Value::as_str)
        .unwrap_or("allow");
    assert_ne!(allow_verdict, "deny");

    let risky = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        "agent-terminal",
        None,
        "terminal_exec",
        &json!({"command":"git reset --hard HEAD"}),
    );
    assert_ne!(
        risky.get("error").and_then(Value::as_str),
        Some("tool_explicit_signoff_required")
    );
}

#[test]
fn workspace_analyze_alias_routes_into_terminal_exec_surface() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let routed = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        "agent-terminal",
        None,
        "workspace_analyze",
        &json!({"query":"effective loc"}),
    );
    assert_ne!(
        routed.get("error").and_then(Value::as_str),
        Some("unsupported_tool")
    );
    assert_ne!(
        routed.get("error").and_then(Value::as_str),
        Some("command_required")
    );
}

#[test]
fn spawn_tools_run_without_confirmation_gate() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let out = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        "agent-spawn",
        None,
        "spawn_subagents",
        &json!({
            "count": 2,
            "objective": "parallelize architecture diagnostics"
        }),
    );
    let error = out.get("error").and_then(Value::as_str).unwrap_or("");
    assert_ne!(error, "tool_explicit_signoff_required");
    assert_ne!(error, "tool_confirmation_required");
    assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
}

#[test]
fn parent_can_archive_descendant_without_signoff_and_reason_is_persisted() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();

    let parent_create = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"parent-gate-test","role":"operator"}"#,
        &snapshot,
    )
    .expect("create parent");
    let parent_id = clean_agent_id(
        parent_create
            .payload
            .get("agent_id")
            .or_else(|| parent_create.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    assert!(!parent_id.is_empty());

    let child_payload = serde_json::to_vec(&json!({
        "name": "child-gate-test",
        "role": "analyst",
        "parent_agent_id": parent_id
    }))
    .expect("serialize child create");
    let child_create = handle(
        root.path(),
        "POST",
        "/api/agents",
        &child_payload,
        &snapshot,
    )
    .expect("create child");
    let child_id = clean_agent_id(
        child_create
            .payload
            .get("agent_id")
            .or_else(|| child_create.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    assert!(!child_id.is_empty());

    let archived = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        &parent_id,
        None,
        "manage_agent",
        &json!({"action":"archive", "agent_id": child_id}),
    );
    assert_eq!(archived.get("ok").and_then(Value::as_bool), Some(true));
    assert_ne!(
        archived.get("error").and_then(Value::as_str),
        Some("tool_explicit_signoff_required")
    );
    assert_eq!(
        archived.get("reason").and_then(Value::as_str),
        Some("Archived by parent agent")
    );

    let archived_state = read_json_loose(
        &root
            .path()
            .join("client/runtime/local/state/ui/infring_dashboard/archived_agents.json"),
    )
    .unwrap_or_else(|| json!({}));
    let reason = archived_state
        .pointer(&format!("/agents/{child_id}/reason"))
        .and_then(Value::as_str)
        .unwrap_or("");
    assert_eq!(reason, "Archived by parent agent");

    let blocked = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        "agent-unrelated",
        None,
        "manage_agent",
        &json!({"action":"archive", "agent_id": child_id}),
    );
    assert_eq!(
        blocked.get("error").and_then(Value::as_str),
        Some("tool_explicit_signoff_required")
    );
}

#[test]
fn semantic_memory_query_route_returns_matches() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
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
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
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
    assert_eq!(child_contract.get("merge_strategy").and_then(Value::as_str), Some("reduce"));
    assert!(child_contract.get("spawn_guard").and_then(Value::as_object).is_some());
    let context_slice = child_contract.get("context_slice").expect("context slice persisted");
    assert_eq!(context_slice.get("strategy").and_then(Value::as_str), Some("objective_scoped_recent_window"));
    assert!(context_slice.get("objective_tokens").and_then(Value::as_array).unwrap_or(&Vec::new()).iter().any(|token| token.as_str() == Some("workflow")));
    let selected = context_slice.get("selected_messages").and_then(Value::as_array).expect("selected messages");
    assert!(selected.len() <= 12);
    assert!(selected.len() < 16);
    assert!(selected.iter().any(|row| {
        row.get("text").and_then(Value::as_str).unwrap_or("").contains("file workflow")
    }));
}

#[test]
fn web_tool_fallback_can_use_semantic_memory_matches() {
    let root = governance_temp_root();
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

#[test]
fn web_search_summary_strips_search_engine_chrome_noise() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "query": "agentic ai systems architecture best practices 2024",
            "summary": "agentic AI systems architecture best practices 2024 at DuckDuckGo All Regions Argentina Australia Austria arxiv.org/abs/2601.00123"
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(!lowered.contains("duckduckgo all regions"));
    assert!(lowered.contains("web search for"));
    assert!(lowered.contains("arxiv.org"));
}

#[test]
fn web_fetch_summary_suppresses_example_domain_placeholder_dump() {
    let summary = summarize_tool_payload(
        "web_fetch",
        &json!({
            "ok": true,
            "requested_url": "https://example.com",
            "summary": "Example Domain This domain is for use in documentation examples without needing permission."
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("placeholder"));
    assert!(lowered.contains("example.com"));
    assert!(!lowered.contains("without needing permission"));
}

#[test]
fn web_fetch_summary_converts_navigation_chrome_into_actionable_hint() {
    let summary = summarize_tool_payload(
        "web_fetch",
        &json!({
            "ok": true,
            "requested_url": "https://www.bbc.com/",
            "summary": "BBC News - Breaking news. Skip to content. Home News Sport Business Technology Health Culture Arts Travel Audio Video Live."
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("navigation/chrome"));
    assert!(lowered.contains("batch_query") || lowered.contains("web_search"));
    assert!(!lowered.contains("skip to content"));
}

#[test]
fn natural_web_intent_does_not_auto_route_peer_comparisons_without_explicit_web_request() {
    let route = natural_web_intent_from_user_message(
        "Compare Infring to its competitors and rank it among peers in a table",
    );
    assert!(route.is_none());
}
