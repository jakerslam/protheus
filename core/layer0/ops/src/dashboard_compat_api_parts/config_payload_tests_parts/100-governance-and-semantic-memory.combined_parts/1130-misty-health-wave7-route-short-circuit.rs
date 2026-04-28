// SRS: V12-MISTY-HEALTH-WAVE7-001, V12-MISTY-HEALTH-WAVE7-002, V12-MISTY-HEALTH-WAVE7-003, V12-MISTY-HEALTH-WAVE7-004, V12-MISTY-HEALTH-WAVE7-005, V12-MISTY-HEALTH-WAVE7-006, V12-MISTY-HEALTH-WAVE8-003, V12-MISTY-HEALTH-WAVE8-004, V12-MISTY-HEALTH-WAVE8-005

#[test]
fn misty_wave7_route_short_circuit_reports_empty_finalization_as_diagnostics_not_blank_chat() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave7-no-model-agent","role":"assistant"}"#,
        &snapshot,
    )
    .expect("agent create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"hey"}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(response.payload.get("ok").and_then(Value::as_bool), Some(false));
    assert_eq!(
        response.payload.get("error_code").and_then(Value::as_str),
        Some("final_response_empty")
    );
    assert_eq!(
        response
            .payload
            .get("diagnostic_class")
            .and_then(Value::as_str),
        Some("application_finalization_failure")
    );
    assert_eq!(
        response.payload.get("retryable").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(response.payload.get("response").and_then(Value::as_str), Some(""));
    assert_eq!(
        response
            .payload
            .pointer("/turn_persistence/user_message_persisted")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        response
            .payload
            .pointer("/turn_persistence/assistant_message_persisted")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        response
            .payload
            .pointer("/turn_persistence/diagnostics_in_chat")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        response
            .payload
            .pointer("/workflow_visibility/diagnostics_only")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        response
            .payload
            .pointer("/workflow_visibility/chat_injection_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert!(
        response
            .payload
            .pointer("/response_workflow/stage_statuses")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false)
    );
    assert_eq!(
        response
            .payload
            .pointer("/live_eval_monitor/chat_injection_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        response
            .payload
            .pointer("/live_eval_monitor/issue_count")
            .and_then(Value::as_u64),
        Some(1)
    );
}

#[test]
fn misty_wave7_successful_direct_turn_exposes_visibility_stage_count() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave7-direct-agent","role":"assistant"}"#,
        &snapshot,
    )
    .expect("agent create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({"queue": [{"response": "Hey, I am here."}], "calls": []}),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"hey"}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(response.payload.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(
        response.payload.get("response").and_then(Value::as_str),
        Some("Hey, I am here.")
    );
    assert_eq!(
        response
            .payload
            .pointer("/workflow_visibility/selected_workflow_id")
            .and_then(Value::as_str),
        Some("simple_conversation_v1")
    );
    assert_eq!(
        response
            .payload
            .pointer("/workflow_visibility/stage_count")
            .and_then(Value::as_u64),
        Some(4)
    );
    assert!(
        response
            .payload
            .pointer("/workflow_visibility/finalization_status")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("workflow")
    );
}

#[test]
fn misty_wave7_recovery_turn_uses_single_minimal_llm_finalization() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave7-recovery-fast-agent","role":"assistant"}"#,
        &snapshot,
    )
    .expect("agent create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({"queue": [
            {"response": "No. I'll answer directly."},
            {"response": "You're right: I should answer directly and not repeat internal status text."}
        ], "calls": []}),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"what? why are you repeating the same fallback text?"}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(response.payload.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(
        response.payload.get("response").and_then(Value::as_str),
        Some(
            "You're right: I should answer directly and not repeat internal status text."
        )
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/attempted")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/attempt_count")
            .and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(
        response
            .payload
            .pointer("/workflow_visibility/stage_count")
            .and_then(Value::as_u64),
        Some(4)
    );
    assert_eq!(
        response
            .payload
            .get("system_chat_injection_used")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn misty_wave7_dry_run_no_tools_uses_minimal_no_tool_exit() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave7-dry-run-agent","role":"assistant"}"#,
        &snapshot,
    )
    .expect("agent create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({"queue": [
            {"response": "No. I would use workspace_search, but I will not run tools for this dry run."},
            {"response": "I would use workspace_search, but I will not run tools for this dry run."}
        ], "calls": []}),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Dry run only: tell me which file tool you would use, but do not run tools yet."}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(response.payload.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(
        response.payload.get("response").and_then(Value::as_str),
        Some("I would use workspace_search, but I will not run tools for this dry run.")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/selected_workflow/mode")
            .and_then(Value::as_str),
        Some("model_direct_answer")
    );
    assert_eq!(
        response
            .payload
            .pointer("/workflow_visibility/stage_count")
            .and_then(Value::as_u64),
        Some(4)
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/attempted")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        response.payload.get("tools").and_then(Value::as_array).map(Vec::len),
        Some(0)
    );
    assert!(
        !response
            .payload
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("")
            .starts_with("No."),
        "{:?}",
        response.payload.get("response")
    );
    assert_eq!(
        response
            .payload
            .get("system_chat_injection_used")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn misty_wave7_finalization_edge_fails_closed_without_system_chat_injection() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave7-empty-final-agent","role":"assistant"}"#,
        &snapshot,
    )
    .expect("agent create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({"queue": [{
            "response": "This is not a hard-coded system response. This turn hit a workflow finalization edge without a policy denial, and I can continue with a direct answer from current context."
        }], "calls": []}),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"what about now? any better?"}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(response.payload.get("ok").and_then(Value::as_bool), Some(false));
    assert_eq!(
        response.payload.get("error_code").and_then(Value::as_str),
        Some("final_response_empty")
    );
    assert_eq!(
        response
            .payload
            .get("diagnostic_class")
            .and_then(Value::as_str),
        Some("application_finalization_failure")
    );
    assert_eq!(
        response.payload.get("retryable").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        response
            .payload
            .get("transport_retryable")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        response
            .payload
            .get("infrastructure_failure")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_finalization/route_failure/diagnostic_class")
            .and_then(Value::as_str),
        Some("application_finalization_failure")
    );
    assert_eq!(response.payload.get("response").and_then(Value::as_str), Some(""));
    assert_eq!(
        response
            .payload
            .get("system_chat_injection_used")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        response
            .payload
            .pointer("/workflow_visibility/chat_injection_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        response
            .payload
            .pointer("/live_eval_monitor/issues/0/raw_event/issue_class")
            .and_then(Value::as_str),
        Some("message_route_error")
    );

    let session = handle(
        root.path(),
        "GET",
        &format!("/api/agents/{agent_id}/session"),
        b"",
        &snapshot,
    )
    .expect("session response");
    assert_eq!(session.status, 200);
    let messages = session
        .payload
        .get("messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        messages.iter().any(|row| {
            row.get("role").and_then(Value::as_str) == Some("user")
                && row.get("text").and_then(Value::as_str)
                    == Some("what about now? any better?")
        }),
        "{messages:?}"
    );
    assert!(
        !messages.iter().any(|row| {
            row.get("role").and_then(Value::as_str) == Some("assistant")
                && row
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .contains("workflow finalization edge")
        }),
        "{messages:?}"
    );
}

#[test]
fn misty_wave8_tool_success_claim_without_evidence_fails_closed() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave8-no-fake-tool-success-agent","role":"assistant"}"#,
        &snapshot,
    )
    .expect("agent create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    let fake_tool_claim =
        "I searched the workspace files and found the following files: routes/web.php and Cargo.toml. The file tooling succeeded.";
    assert!(response_claims_tool_success_without_current_turn_evidence(
        "Find the route files.",
        fake_tool_claim,
        &[]
    ));
    assert!(!response_claims_tool_success_without_current_turn_evidence(
        "Find the route files.",
        "I would use workspace search next, but I have not run it yet.",
        &[]
    ));
    assert!(!response_claims_tool_success_without_current_turn_evidence(
        "Find the route files.",
        fake_tool_claim,
        &[json!({
            "name": "workspace_search",
            "status": "ok",
            "result": "routes/web.php",
            "tool_attempt_receipt": {"receipt_hash": "receipt-workspace-search"}
        })]
    ));
    let guard = final_response_guard_report("Find the route files.", fake_tool_claim, &[], false);
    assert_eq!(
        guard
            .get("unsupported_tool_success_claim")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        final_response_guard_outcome(&guard),
        "unsupported_tool_success_claim_withheld"
    );
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({"queue": [{"response": fake_tool_claim}], "calls": []}),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Find the route files in the workspace."}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(response.payload.get("ok").and_then(Value::as_bool), Some(false));
    assert_eq!(
        response.payload.get("error_code").and_then(Value::as_str),
        Some("final_response_empty")
    );
    assert_eq!(response.payload.get("response").and_then(Value::as_str), Some(""));
    assert_eq!(
        response.payload.get("system_chat_injection_used").and_then(Value::as_bool),
        Some(false)
    );
    let session = handle(
        root.path(),
        "GET",
        &format!("/api/agents/{agent_id}/session"),
        b"",
        &snapshot,
    )
    .expect("session response");
    let messages = session
        .payload
        .get("messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        messages.iter().any(|row| row.get("role").and_then(Value::as_str) == Some("user")),
        "{messages:?}"
    );
    assert!(
        !messages.iter().any(|row| {
            row.get("role").and_then(Value::as_str) == Some("assistant")
                && row.get("text").and_then(Value::as_str).unwrap_or("").contains("file tooling succeeded")
        }),
        "{messages:?}"
    );
}

#[test]
fn misty_wave7_empty_initial_tool_request_can_finish_with_llm_menu_selection() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave7-manual-web-menu-agent","role":"assistant"}"#,
        &snapshot,
    )
    .expect("agent create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({"queue": [
            {"response": "", "provider": "ollama", "runtime_model": "deepseek-v3.1:671b-cloud"},
            {"response": "Yes. Tool family: Web Search / Fetch. Tool: Web search. Request payload: {\"source\":\"web\",\"query\":\"compare infring to other major agentic frameworks in April 2026\",\"aperture\":\"medium\"}.", "provider": "ollama", "runtime_model": "deepseek-v3.1:671b-cloud"},
            {"response": "I would choose web search for a current April 2026 framework comparison, then synthesize from the returned results.", "provider": "ollama", "runtime_model": "deepseek-v3.1:671b-cloud"}
        ], "calls": []}),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Use web search to compare infring to other major agentic frameworks in April 2026."}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(response.payload.get("ok").and_then(Value::as_bool), Some(true));
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(response_text.to_ascii_lowercase().contains("web search"), "{response_text}");
    assert!(!response_text.contains("Request payload"), "{response_text}");
    assert!(!response_text.starts_with("Yes."), "{response_text}");
    assert_eq!(
        response.payload.get("tools").and_then(Value::as_array).map(Vec::len),
        Some(0)
    );
    assert_eq!(
        response
            .payload
            .pointer("/pending_tool_request/status")
            .and_then(Value::as_str),
        Some("pending_confirmation")
    );
    assert_eq!(
        response
            .payload
            .pointer("/pending_tool_request/tool_name")
            .and_then(Value::as_str),
        Some("batch_query")
    );
    assert!(
        response
            .payload
            .pointer("/pending_tool_request/receipt_binding")
            .and_then(Value::as_str)
            .map(|value| !value.is_empty())
            .unwrap_or(false)
    );
    assert_eq!(
        response
            .payload
            .pointer("/pending_tool_request/execution_claim_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        response.payload.get("model").and_then(Value::as_str),
        Some("deepseek-v3.1:671b-cloud")
    );
    assert_eq!(
        response
            .payload
            .get("system_chat_injection_used")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/system_events/0/kind")
            .and_then(Value::as_str),
        Some("manual_toolbox_candidate_menu")
    );
    assert_eq!(
        response
            .payload
            .pointer("/workflow_visibility/chat_injection_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn misty_wave7_empty_finalization_recovers_with_llm_menu_choice() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave7-empty-recovery-agent","role":"assistant"}"#,
        &snapshot,
    )
    .expect("agent create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({"queue": [
            {"response": "", "provider": "ollama", "runtime_model": "deepseek-v3.1:671b-cloud"},
            {"response": "Yes. Tool family: Web Search / Fetch. Tool: Web search. Request payload: {\"source\":\"web\",\"query\":\"compare infring to other major agentic frameworks in April 2026\",\"aperture\":\"medium\"}.", "provider": "ollama", "runtime_model": "deepseek-v3.1:671b-cloud"},
            {"response": "I would choose web search for a current April 2026 framework comparison, then synthesize from the returned results.", "provider": "ollama", "runtime_model": "deepseek-v3.1:671b-cloud"}
        ], "calls": []}),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Use web search to compare infring to other major agentic frameworks in April 2026."}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(response.payload.get("ok").and_then(Value::as_bool), Some(true));
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(response_text.to_ascii_lowercase().contains("web search"), "{response_text}");
    assert!(!response_text.contains("Request payload"), "{response_text}");
    assert!(!response_text.starts_with("Yes."), "{response_text}");
    assert_eq!(
        response
            .payload
            .get("system_chat_injection_used")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        response
            .payload
            .pointer("/pending_tool_request/status")
            .and_then(Value::as_str),
        Some("pending_confirmation")
    );
    assert_eq!(
        response
            .payload
            .pointer("/pending_tool_request/tool_name")
            .and_then(Value::as_str),
        Some("batch_query")
    );
    let event_kinds = response
        .payload
        .pointer("/response_workflow/system_events")
        .and_then(Value::as_array)
        .map(|events| {
            events
                .iter()
                .filter_map(|event| event.get("kind").and_then(Value::as_str))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    assert!(
        event_kinds.contains(&"draft_response_invalid")
            || event_kinds.contains(&"empty_final_response_menu_recovery"),
        "{event_kinds:?}"
    );
}
