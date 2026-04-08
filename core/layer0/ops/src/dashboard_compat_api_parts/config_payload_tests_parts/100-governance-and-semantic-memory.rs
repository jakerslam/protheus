#[test]
fn terminal_tools_run_without_signoff_and_still_enforce_command_policy() {
    let root = tempfile::tempdir().expect("tempdir");
    let snapshot = json!({"ok": true});
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
    let root = tempfile::tempdir().expect("tempdir");
    let snapshot = json!({"ok": true});
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
    let root = tempfile::tempdir().expect("tempdir");
    let snapshot = json!({"ok": true});
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
    let root = tempfile::tempdir().expect("tempdir");
    let snapshot = json!({"ok": true});

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

#[test]
fn natural_web_intent_does_not_treat_tool_route_mapping_prompts_as_web_queries() {
    let route =
        natural_web_intent_from_user_message("Map `tool::web_search` into a supported route");
    assert!(route.is_none());
}

#[test]
fn conversational_prompt_does_not_auto_route_direct_tool_intent() {
    assert!(direct_tool_intent_from_user_message("what do you think of infring?").is_none());
}

#[test]
fn explicit_tool_command_routes_web_search_with_defaults() {
    let (tool, input) =
        direct_tool_intent_from_user_message("tool::web_search:::latest ai agent benchmarks")
            .expect("explicit tool command");
    assert_eq!(tool, "web_search");
    assert_eq!(
        input.get("query").and_then(Value::as_str).unwrap_or(""),
        "latest ai agent benchmarks"
    );
    assert_eq!(
        input.get("source").and_then(Value::as_str).unwrap_or(""),
        "web"
    );
    assert_eq!(
        input.get("aperture").and_then(Value::as_str).unwrap_or(""),
        "medium"
    );
}

#[test]
fn explicit_tool_command_rejects_unknown_names_with_suggestion() {
    let (tool, input) =
        direct_tool_intent_from_user_message("tool::web_serch:::latest").expect("router reply");
    assert_eq!(tool, "tool_command_router");
    assert_eq!(
        input.get("error").and_then(Value::as_str).unwrap_or(""),
        "unsupported_tool_command"
    );
    assert_eq!(
        input.get("suggestion").and_then(Value::as_str).unwrap_or(""),
        "web_search"
    );
}

#[test]
fn explicit_tool_command_rejects_malformed_shape_before_routing() {
    let (tool, input) = direct_tool_intent_from_user_message("tool::web_search::latest").expect("router reply");
    assert_eq!(tool, "tool_command_router");
    assert_eq!(input.get("error").and_then(Value::as_str).unwrap_or(""), "tool_command_name_invalid");
}

#[test]
fn explicit_tool_command_maps_memory_store_to_kv_set() {
    let (tool, input) =
        direct_tool_intent_from_user_message("tool::memory_store:::deploy.mode=staged")
            .expect("memory store command");
    assert_eq!(tool, "memory_kv_set");
    assert_eq!(
        input.get("key").and_then(Value::as_str).unwrap_or(""),
        "deploy.mode"
    );
    assert_eq!(
        input.get("value").and_then(Value::as_str).unwrap_or(""),
        "staged"
    );
}

#[test]
fn inline_tool_policy_requires_explicit_tooling_request() {
    assert!(!inline_tool_calls_allowed_for_user_message(
        "what do you think of infring?"
    ));
    assert!(inline_tool_calls_allowed_for_user_message(
        "search the web for latest ai agent benchmarks"
    ));
    assert!(!inline_tool_calls_allowed_for_user_message(
        "just answer directly, dont use a tool call"
    ));
    assert!(!inline_tool_calls_allowed_for_user_message(
        "why do you keep trying tool calls and not synthesizing results"
    ));
}

#[test]
fn inline_tool_calls_hide_signoff_error_codes_from_chat_text() {
    let root = tempfile::tempdir().expect("tempdir");
    let snapshot = json!({"ok": true});
    let response =
        "<function=spawn_subagents>{\"count\":3,\"objective\":\"parallelize analysis\"}</function>";
    let (text, cards, pending_confirmation, suppressed) = execute_inline_tool_calls(
        root.path(),
        &snapshot,
        "agent-inline",
        None,
        response,
        "parallelize this with a swarm",
        true,
    );
    assert!(!suppressed);
    assert_eq!(cards.len(), 1);
    assert_eq!(
        cards[0].get("is_error").and_then(Value::as_bool),
        Some(false)
    );
    assert!(pending_confirmation.is_none());
    let lowered = text.to_ascii_lowercase();
    assert!(!lowered.contains("tool_explicit_signoff_required"));
    assert!(!lowered.contains("spawn_subagents failed"));
    assert!(!lowered.contains("confirmation"));
}

#[test]
fn inline_tool_execution_is_suppressed_for_plain_conversation_turns() {
    let root = tempfile::tempdir().expect("tempdir");
    let snapshot = json!({"ok": true});
    let response = "<function=web_search>{\"query\":\"latest technology news\"}</function>";
    let (text, cards, pending_confirmation, suppressed) = execute_inline_tool_calls(
        root.path(),
        &snapshot,
        "agent-inline-suppressed",
        None,
        response,
        "what do you think of infring?",
        false,
    );
    assert!(suppressed);
    assert!(cards.is_empty());
    assert!(pending_confirmation.is_none());
    assert!(text.trim().is_empty());
}

#[test]
fn inline_tool_execution_replaces_low_signal_cleaned_text_with_tool_fallback_lines() {
    let root = tempfile::tempdir().expect("tempdir");
    let snapshot = json!({"ok": true});
    let response = "<function=spawn_subagents>{\"count\":2,\"objective\":\"parallelize analysis\"}</function>\nFrom web retrieval: bing.com: compare [A with B] vs compare A [with B]";
    let (text, cards, pending_confirmation, suppressed) = execute_inline_tool_calls(
        root.path(),
        &snapshot,
        "agent-inline-low-signal",
        None,
        response,
        "parallelize this with a swarm",
        true,
    );
    assert!(!suppressed);
    assert_eq!(cards.len(), 1);
    assert!(pending_confirmation.is_none());
    let lowered = text.to_ascii_lowercase();
    assert!(lowered.contains("spawned"));
    assert!(!lowered.contains("from web retrieval:"));
    assert!(!lowered.contains("bing.com: compare"));
}

#[test]
fn pending_confirmation_yes_replays_manage_agent_action() {
    let root = tempfile::tempdir().expect("tempdir");
    let snapshot = json!({"ok": true});
    let parent = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"parent-runtime","role":"operator"}"#,
        &snapshot,
    )
    .expect("parent create");
    let parent_id = clean_agent_id(
        parent
            .payload
            .get("agent_id")
            .or_else(|| parent.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    assert!(!parent_id.is_empty());
    let child_payload = serde_json::to_vec(&json!({
        "name": "child-runtime",
        "role": "analyst",
        "parent_agent_id": parent_id
    }))
    .expect("serialize child");
    let child = handle(
        root.path(),
        "POST",
        "/api/agents",
        &child_payload,
        &snapshot,
    )
    .expect("child create");
    let child_id = clean_agent_id(
        child
            .payload
            .get("agent_id")
            .or_else(|| child.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    assert!(!child_id.is_empty());
    let _ = update_profile_patch(
        root.path(),
        &parent_id,
        &json!({
            "pending_tool_confirmation": {
                "tool_name": "manage_agent",
                "input": {"action": "archive", "agent_id": child_id}
            }
        }),
    );
    let yes_body = serde_json::to_vec(&json!({"message":"yes"})).expect("serialize yes");
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{parent_id}/message"),
        &yes_body,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    assert_eq!(
        response.payload.get("ok").and_then(Value::as_bool),
        Some(true)
    );
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    assert!(!response_text.contains("need your confirmation"));
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
    let profile = profiles_map(root.path())
        .get(&parent_id)
        .cloned()
        .unwrap_or_else(|| json!({}));
    assert!(profile
        .get("pending_tool_confirmation")
        .map(Value::is_null)
        .unwrap_or(true));
}

#[test]
fn telemetry_dump_detector_flags_duckduckgo_noise_and_tool_error_codes() {
    let dump = "agentic AI systems architecture at DuckDuckGo All Regions Argentina Australia. spawn_subagents failed: tool_explicit_signoff_required";
    assert!(response_is_unrelated_context_dump(
        "improve this system",
        dump
    ));
}

#[test]
fn unrelated_dump_detector_flags_peer_review_template_leaks() {
    let dump = "AIFFEL Campus Online 5th Code Peer Review Templete - 코더 : 최연석 - 리뷰어 : 김연 # PRT(PeerReviewTemplate) 각 항목을 스스로 확인하고 토의하여 작성한 코드에 적용합니다. 코드가 정상적으로 동작하고 주어진 문제를 해결했나요?";
    assert!(response_is_unrelated_context_dump(
        "did you format that as a list?",
        dump
    ));
}

#[test]
fn append_turn_message_captures_explicit_remember_fact_for_long_term_memory() {
    let root = tempfile::tempdir().expect("tempdir");
    let captured =
        parse_memory_capture_text("remember this: the fallback phrase is cobalt sunrise")
            .expect("memory capture text");
    assert!(captured.to_ascii_lowercase().contains("cobalt sunrise"));
    assert!(parse_memory_capture_text("just answer normally").is_none());

    let _ = crate::dashboard_agent_state::memory_kv_set(
        root.path(),
        "agent-memory-capture",
        "explicit_memory.test",
        &json!({"text": captured, "captured_at": crate::now_iso()}),
    );
    let memory_state = crate::dashboard_agent_state::memory_kv_semantic_query(
        root.path(),
        "agent-memory-capture",
        "cobalt sunrise",
        5,
    );
    assert!(memory_state
        .get("matches")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));
}

#[test]
fn response_tools_summary_rewrites_ack_only_text_into_user_facing_findings() {
    let synthesized = response_tools_summary_for_user(
        &[json!({
            "name": "web_search",
            "is_error": false,
            "result": "Web search findings for \"agent reliability\": - arxiv.org/abs/2601.12345 - github.com/org/repo/issues"
        })],
        4,
    );
    assert!(!synthesized.is_empty());
    assert!(synthesized
        .to_ascii_lowercase()
        .contains("here's what i found"));
    assert!(synthesized.to_ascii_lowercase().contains("web search"));
}

#[test]
fn ack_only_detector_flags_generic_tool_acknowledgements() {
    assert!(response_looks_like_tool_ack_without_findings(
        "I searched the internet and executed the tool call."
    ));
    assert!(response_looks_like_tool_ack_without_findings(
        "Web search completed. I called the tools and processed your request."
    ));
    assert!(!response_looks_like_tool_ack_without_findings(
        "Here are the findings: 1. https://arxiv.org/abs/2601.12345 2. https://github.com/org/repo"
    ));
}

#[test]
fn ack_only_detector_flags_explicit_no_findings_failure_copy() {
    assert!(response_looks_like_tool_ack_without_findings(
        "I couldn't extract usable findings from the search response yet."
    ));
    assert!(response_looks_like_tool_ack_without_findings(
        "From web retrieval: bing.com: OpenClaw — Personal AI Assistant — https://openclaw.ai/"
    ));
}

#[test]
fn ack_only_detector_flags_key_findings_source_scaffold_copy() {
    assert!(response_looks_like_tool_ack_without_findings(
        "Key findings for \"Infring AI vs competitors comparison 2024\": - Potential sources: hai.stanford.edu, artificialanalysis.ai, epoch.ai."
    ));
}

#[test]
fn ack_only_detector_flags_duckduckgo_findings_placeholder_copy() {
    assert!(response_looks_like_tool_ack_without_findings(
        "I couldn't extract usable findings for this yet. The search response came from https://duckduckgo.com/html/?q=agent+systems"
    ));
}

#[test]
fn response_tools_summary_drops_ack_only_tool_rows() {
    let synthesized = response_tools_summary_for_user(
        &[json!({
            "name": "web_search",
            "is_error": false,
            "result": "Web search completed."
        })],
        4,
    );
    assert!(synthesized.is_empty());
}

#[test]
fn response_tools_summary_drops_key_findings_source_scaffold_rows() {
    let synthesized = response_tools_summary_for_user(
        &[json!({
            "name": "batch_query",
            "is_error": false,
            "result": "Key findings for \"Infring AI vs competitors comparison 2024\": - Potential sources: hai.stanford.edu, artificialanalysis.ai, epoch.ai."
        })],
        4,
    );
    assert!(synthesized.is_empty());
}

#[test]
fn finalize_user_facing_response_replaces_ack_with_findings() {
    let finalized = finalize_user_facing_response(
        "Web search completed.".to_string(),
        Some("Here's what I found:\n- arxiv.org/abs/2601.12345".to_string()),
    );
    let lowered = finalized.to_ascii_lowercase();
    assert!(!lowered.contains("web search completed"));
    assert!(lowered.contains("here's what i found"));
    assert!(!response_looks_like_tool_ack_without_findings(&finalized));
}

#[test]
fn finalize_user_facing_response_replaces_ack_without_findings() {
    let finalized = finalize_user_facing_response("Web search completed.".to_string(), None);
    let lowered = finalized.to_ascii_lowercase();
    assert!(!lowered.contains("web search completed"));
    assert!(lowered.contains("no relevant results"));
    assert!(!response_looks_like_tool_ack_without_findings(&finalized));
}

#[test]
fn finalize_user_facing_response_rewrites_generic_tool_failure_placeholder() {
    let finalized = finalize_user_facing_response(
        "I couldn't complete system_diagnostic right now.".to_string(),
        None,
    );
    let lowered = finalized.to_ascii_lowercase();
    assert!(lowered.contains("doctor --json"));
    assert!(!lowered.contains("couldn't complete system_diagnostic right now"));
}

#[test]
fn finalize_user_facing_response_never_leaks_tool_status_text() {
    let finalized = finalize_user_facing_response(
        "Tool call finished.".to_string(),
        Some("Tool call finished.".to_string()),
    );
    let lowered = finalized.to_ascii_lowercase();
    assert!(!lowered.contains("tool call finished"));
    assert!(!response_looks_like_tool_ack_without_findings(&finalized));
}

#[test]
fn comparative_detector_matches_peer_ranking_language() {
    assert!(message_requests_comparative_answer(
        "find out how Infring ranks among its peers"
    ));
    assert!(message_requests_comparative_answer(
        "compare infring versus top competitors"
    ));
}

#[test]
fn comparative_no_findings_fallback_is_actionable() {
    let fallback = comparative_no_findings_fallback("rank infring among peers");
    let lowered = fallback.to_ascii_lowercase();
    assert!(lowered.contains("infring"));
    assert!(lowered.contains("strongest"));
    assert!(lowered.contains("batch_query"));
    assert!(!response_is_no_findings_placeholder(&fallback));
}

#[test]
fn tooling_failure_fallback_triggers_for_diagnostic_prompt() {
    let fallback = maybe_tooling_failure_fallback(
        "so the tooling isnt working at all?",
        "I couldn't extract usable findings for \"current technology news\" yet.",
        "",
    )
    .expect("fallback");
    let lowered = fallback.to_ascii_lowercase();
    assert!(lowered.contains("partially working"));
    assert!(lowered.contains("batch_query"));
    assert!(lowered.contains("doctor --json"));
}

#[test]
fn tooling_failure_fallback_triggers_for_repeated_placeholder_loop() {
    let repeated = "I couldn't extract usable findings for \"current technology news\" yet.";
    let fallback = maybe_tooling_failure_fallback("?", repeated, repeated).expect("fallback");
    assert!(fallback.to_ascii_lowercase().contains("parse miss"));
}

#[test]
fn system_diagnostic_failure_summary_is_not_generic_dead_end() {
    let summary = user_facing_tool_failure_summary(
        "system_diagnostic",
        &json!({"ok": false, "error": "request_read_failed"}),
    )
    .expect("summary");
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("diagnose manually"));
    assert!(!lowered.contains("couldn't complete `system_diagnostic` right now"));
}

#[test]
fn web_search_request_read_failed_summary_is_actionable() {
    let summary = user_facing_tool_failure_summary(
        "web_search",
        &json!({"ok": false, "error": "request_read_failed:Resource temporarily unavailable (os error 35)"}),
    )
    .expect("summary");
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("retry transient failures"));
    assert!(lowered.contains("doctor --json"));
    assert!(lowered.contains("request_read_failed"));
}

#[test]
fn transient_tool_failure_detects_request_read_failed_signature() {
    assert!(transient_tool_failure(&json!({
        "ok": false,
        "error": "request_read_failed:Resource temporarily unavailable (os error 35)"
    })));
}

#[test]
fn web_search_summary_avoids_completed_placeholder_copy() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "query": "agent reliability",
            "summary": "safe search region picker noise only"
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(!lowered.contains("web search completed"));
    assert!(!lowered.contains("completed."));
    assert!(!lowered.trim().is_empty());
}

#[test]
fn web_search_summary_reports_root_cause_without_legacy_no_findings_template() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "query": "ai assistant systems comparison 2024 capabilities landscape",
            "requested_url": "https://duckduckgo.com/html/?q=ai+assistant+systems+comparison",
            "domain": "duckduckgo.com",
            "summary": "AI assistant systems comparison 2024 capabilities landscape at DuckDuckGo All Regions Argentina Australia Safe search Any time"
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("low-signal"));
    assert!(lowered.contains("batch_query"));
    assert!(!lowered.contains("search response came from"));
    assert!(!lowered.contains("couldn't extract usable findings"));
}

#[test]
fn web_search_summary_discards_potential_sources_scaffold_output() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "query": "Infring AI agent platform capabilities features 2024",
            "summary": "Key findings for \"Infring AI agent platform capabilities features 2024\":\n- Potential sources: nlplogix.com, gartner.com, insightpartners.com.\n- Potential sources: salesforce.com, microsoft.com, lyzr.ai."
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(!lowered.contains("potential sources:"));
    assert!(!lowered.contains("key findings for"));
    assert!(lowered.contains("low-signal") || lowered.contains("no extractable findings"));
}

#[test]
fn web_search_summary_uses_content_domains_when_summary_is_search_chrome() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "query": "latest technology news today",
            "requested_url": "https://duckduckgo.com/html/?q=latest+technology+news+today",
            "summary": "latest technology news today at DuckDuckGo All Regions Argentina Australia Safe Search Any Time",
            "content": "latest technology news today at DuckDuckGo All Regions Any Time Tech News | Today's Latest Technology News | Reuters www.reuters.com/technology/ Find latest technology news from every corner of the globe. Technology: Latest Tech News Articles Today | AP News apnews.com/technology Don't miss an update on the latest tech news. The Latest News in Technology | PCMag www.pcmag.com/news Get the latest technology news and in-depth analysis."
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(
        lowered.contains("key web findings"),
        "unexpected summary: {summary}"
    );
    assert!(lowered.contains("reuters.com"));
    assert!(!lowered.contains("couldn't extract usable findings"));
    assert!(!lowered.contains("search response came from"));
}

#[test]
fn batch_query_summary_rewrites_unsynthesized_domain_dump_to_structured_evidence() {
    let summary = summarize_tool_payload(
        "batch_query",
        &json!({
            "ok": true,
            "status": "ok",
            "summary": "Web benchmark synthesis: bing.com: compare [A with B] vs compare A [with B] | WordReference Forums — https://forum.wordreference.com/threads/compare-a-with-b-vs-compare-a-with-b.4047424/",
            "evidence_refs": [
                {
                    "title": "OpenClaw — Personal AI Assistant",
                    "locator": "https://openclaw.ai/"
                }
            ]
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("batch query evidence"));
    assert!(!lowered.contains("bing.com: compare"));
}

#[test]
fn unsynthesized_web_snippet_detector_flags_domain_dump_copy() {
    assert!(response_looks_like_unsynthesized_web_snippet_dump(
        "From web retrieval: bing.com: OpenClaw — Personal AI Assistant — https://openclaw.ai/ bing.com: OpenClaw docs — https://openclaw.ai/docs"
    ));
    assert!(!response_looks_like_unsynthesized_web_snippet_dump(
        "In short: OpenClaw focuses on cross-platform local execution, while Infring emphasizes policy-gated orchestration and receipts."
    ));
}

#[test]
fn finalize_user_facing_response_rewrites_raw_placeholder_dump() {
    let finalized = finalize_user_facing_response(
        "Example Domain This domain is for use in documentation examples without needing permission."
            .to_string(),
        None,
    );
    let lowered = finalized.to_ascii_lowercase();
    assert!(lowered.contains("raw web output"));
    assert!(lowered.contains("batch_query"));
    assert!(!lowered.contains("without needing permission"));
}

#[test]
fn finalize_user_facing_response_unwraps_internal_payload_json_response() {
    let raw = json!({
        "agent_id": "agent-83ed64e07515",
        "response": "From web retrieval: benchmark summary with sources. https://example.com/benchmarks",
        "response_finalization": {"tool_completion": {"completion_state": "reported_findings"}},
        "tools": [{"name": "batch_query", "is_error": false, "result": "raw tool output"}],
        "turn_loop_tracking": {"ok": true},
        "turn_transaction": {"tool_execute": "complete"}
    })
    .to_string();
    let finalized = finalize_user_facing_response(raw, None);
    assert_eq!(
        finalized,
        "From web retrieval: benchmark summary with sources. https://example.com/benchmarks"
    );
    assert!(!finalized.contains("agent_id"));
    assert!(!finalized.starts_with('{'));
}

#[test]
fn finalize_user_facing_response_unwraps_wrapped_internal_payload_json_response() {
    let raw = format!(
        "tool output follows:\n{}\nend",
        json!({
            "agent_id": "agent-83ed64e07515",
            "response": "Synthesized answer with linked sources.",
            "response_finalization": {"tool_completion": {"completion_state": "reported_findings"}},
            "tools": [{"name": "batch_query", "is_error": false, "result": "raw tool output"}],
            "turn_loop_tracking": {"ok": true},
            "turn_transaction": {"tool_execute": "complete"}
        })
    );
    let finalized = finalize_user_facing_response(raw, None);
    assert_eq!(finalized, "Synthesized answer with linked sources.");
    assert!(!finalized.contains("agent_id"));
}

#[test]
fn finalize_user_facing_response_blocks_internal_payload_json_without_response() {
    let raw = json!({
        "agent_id": "agent-83ed64e07515",
        "response_finalization": {"tool_completion": {"completion_state": "reported_reason"}},
        "tools": [{"name": "manage_agent", "is_error": false, "result": "{\"ok\":true}"}],
        "turn_loop_tracking": {"ok": true},
        "turn_transaction": {"tool_execute": "complete"}
    })
    .to_string();
    let finalized = finalize_user_facing_response(raw, None);
    let lowered = finalized.to_ascii_lowercase();
    assert!(lowered.contains("no synthesized response") || lowered.contains("couldn't produce source-backed findings in this turn"));
    assert!(!lowered.contains("agent_id"));
    assert!(!finalized.starts_with('{'));
}

#[test]
fn summarize_tool_payload_unknown_tool_avoids_raw_json_dump() {
    let payload = json!({
        "ok": true,
        "agent_id": "agent-raw-dump",
        "runtime_model": "tool-router",
        "turn_loop_tracking": {"ok": true},
        "response_finalization": {"tool_completion": {"completion_state": "reported_findings"}},
        "result_count": 3,
        "source": "web"
    });
    let summary = summarize_tool_payload("manage_agent", &payload);
    let lowered = summary.to_ascii_lowercase();
    assert!(!summary.trim_start().starts_with('{'));
    assert!(!lowered.contains("\"agent_id\""));
    assert!(lowered.contains("completed"));
}

#[test]
fn enforce_user_facing_finalization_contract_unwraps_internal_payload_dump() {
    let cards = vec![json!({
        "name": "web_search",
        "is_error": false,
        "result": "From web retrieval: benchmark summary with sources."
    })];
    let raw = json!({
        "agent_id": "agent-raw-dump",
        "response": "From web retrieval: benchmark summary with sources.",
        "turn_loop_tracking": {"ok": true},
        "turn_transaction": {"tool_execute": "complete"}
    })
    .to_string();
    let (finalized, report, outcome) = enforce_user_facing_finalization_contract(raw, &cards);
    let lowered = finalized.to_ascii_lowercase();
    assert!(!finalized.trim_start().starts_with('{'));
    assert!(!lowered.contains("agent_id"));
    assert!(lowered.contains("benchmark summary") || lowered.contains("couldn't produce source-backed findings in this turn"));
    assert_eq!(
        report.get("completion_state").and_then(Value::as_str),
        Some("reported_no_findings")
    );
    assert!(
        outcome.contains("normalized_raw_payload_json") || outcome.contains("reported_no_findings"),
        "unexpected outcome={outcome}"
    );
}

#[test]
fn tool_completion_contract_rewrites_ack_to_findings_from_tool_cards() {
    let cards = vec![json!({
        "name": "web_search",
        "is_error": false,
        "result": "Web search findings for \"runtime reliability\": https://example.com/reliability-overview"
    })];
    let (finalized, report) = enforce_tool_completion_contract("Web search completed.".to_string(), &cards);
    let lowered = finalized.to_ascii_lowercase();
    assert!(lowered.contains("here's what i found"));
    assert!(!lowered.contains("web search completed"));
    assert!(!lowered.contains("couldn't produce source-backed findings in this turn"));
    assert_eq!(
        report.get("completion_state").and_then(Value::as_str),
        Some("reported_findings")
    );
    assert_eq!(
        report.get("final_ack_only").and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn tool_completion_contract_rewrites_ack_to_explicit_no_findings_when_results_are_low_signal() {
    let cards = vec![json!({
        "name": "web_search",
        "is_error": false,
        "result": "Web search completed."
    })];
    let (finalized, report) =
        enforce_tool_completion_contract("Web search completed.".to_string(), &cards);
    assert_eq!(finalized, no_findings_user_facing_response());
    assert_eq!(
        report.get("completion_state").and_then(Value::as_str),
        Some("reported_no_findings")
    );
    assert_eq!(
        report.get("final_no_findings").and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn tool_completion_contract_preserves_actionable_failure_reason_when_no_findings_exist() {
    let cards = vec![json!({
        "name": "web_search",
        "is_error": true,
        "result": "I need your confirmation before running `web_search`."
    })];
    let (finalized, report) = enforce_tool_completion_contract(
        "I need your confirmation before running `web_search`.".to_string(),
        &cards,
    );
    let lowered = finalized.to_ascii_lowercase();
    assert!(lowered.contains("confirmation"));
    assert!(!lowered.contains("web search completed"));
    assert_eq!(
        report.get("completion_state").and_then(Value::as_str),
        Some("reported_reason")
    );
}

#[test]
fn relevant_recall_context_surfaces_older_thread_facts_for_continuity() {
    let pooled_messages = vec![
        json!({"role":"user","text":"Remember that cobalt sunrise is our fallback phrase.","ts":"2026-04-01T00:00:00Z"}),
        json!({"role":"assistant","text":"Stored. Cobalt sunrise is the fallback phrase.","ts":"2026-04-01T00:00:01Z"}),
        json!({"role":"user","text":"Also track dashboard reconnect reliability fixes.","ts":"2026-04-01T00:00:02Z"}),
        json!({"role":"assistant","text":"I will keep reconnect fixes and fallback phrase in scope.","ts":"2026-04-01T00:00:03Z"}),
    ];
    let active_messages = vec![
        json!({"role":"user","text":"How do we improve reconnect reliability next?","ts":"2026-04-01T00:05:00Z"}),
    ];
    let context = historical_relevant_recall_prompt_context(
        &pooled_messages,
        &active_messages,
        "Use the fallback phrase and reconnect plan from earlier",
        8,
        2400,
    );
    let lowered = context.to_ascii_lowercase();
    assert!(lowered.contains("relevant long-thread recall"));
    assert!(lowered.contains("fallback phrase") || lowered.contains("cobalt sunrise"));
    assert!(lowered.contains("reconnect"));
}

#[test]
fn recent_floor_enforcement_rehydrates_tail_after_pool_trim() {
    let messages = (0..36)
        .map(|idx| {
            json!({
                "id": idx + 1,
                "role": if idx % 2 == 0 { "user" } else { "agent" },
                "text": format!("context-floor-{idx} {}", "token ".repeat(180)),
                "ts": format!("2026-04-01T00:{idx:02}:00Z")
            })
        })
        .collect::<Vec<_>>();
    let pooled = trim_context_pool(&messages, 2048);
    let floor = 14usize;
    assert!(
        pooled.len() < floor,
        "pool should trim below floor for this fixture"
    );
    let (rehydrated, injected) = enforce_recent_context_floor(&messages, &pooled, floor);
    assert!(injected > 0, "expected floor reinjection");
    assert!(rehydrated.len() >= floor, "recent floor should be restored");
    let required_tail_ids = messages
        .iter()
        .rev()
        .take(floor)
        .filter_map(|row| row.get("id").and_then(Value::as_i64))
        .collect::<Vec<_>>();
    for id in required_tail_ids {
        assert!(
            rehydrated
                .iter()
                .any(|row| row.get("id").and_then(Value::as_i64) == Some(id)),
            "missing reinjected tail message id={id}"
        );
    }
}

#[test]
fn relevant_recall_uses_full_history_even_when_pool_drops_older_facts() {
    let mut history = vec![json!({
        "id": 1,
        "role": "user",
        "text": "Remember the nebula ledger anchor phrase for later continuity.",
        "ts": "2026-04-01T00:00:00Z"
    })];
    for idx in 0..32 {
        history.push(json!({
            "id": idx + 2,
            "role": if idx % 2 == 0 { "agent" } else { "user" },
            "text": format!("filler-{idx} {}", "alpha ".repeat(180)),
            "ts": format!("2026-04-01T00:{:02}:00Z", (idx + 1) % 60)
        }));
    }
    let pooled = trim_context_pool(&history, 2048);
    assert!(
        !pooled.iter().any(|row| message_text(row)
            .to_ascii_lowercase()
            .contains("nebula ledger")),
        "fixture failed: pooled context still contains the anchor fact"
    );
    let (pooled_with_floor, _) = enforce_recent_context_floor(&history, &pooled, 14);
    let active = select_active_context_window(&pooled_with_floor, 1536, 14);
    let recall = historical_relevant_recall_prompt_context(
        &history,
        &active,
        "Recall the nebula ledger anchor from earlier.",
        8,
        2400,
    );
    let lowered = recall.to_ascii_lowercase();
    assert!(lowered.contains("relevant long-thread recall"));
    assert!(lowered.contains("nebula ledger"), "recall={recall}");
}

#[test]
fn execute_tool_recovery_applies_turn_loop_tracking_metadata() {
    let root = tempfile::tempdir().expect("tempdir");
    let mut out = json!({
        "ok": true,
        "summary": "Web search completed."
    });
    crate::dashboard_tool_turn_loop::annotate_tool_payload_tracking(
        root.path(),
        "agent-turnloop-tracking",
        "web_search",
        &mut out,
    );
    let lowered = out
        .get("summary")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    assert!(lowered.contains("no relevant results"));
    assert!(out.get("turn_loop_post_filter").is_some());
    assert!(out.get("turn_loop_tracking").is_some());
}

#[test]
fn execute_tool_recovery_blocks_when_pre_gate_requires_confirmation() {
    let root = tempfile::tempdir().expect("tempdir");
    let snapshot = json!({"ok": true});
    let policy_path = root
        .path()
        .join("client/runtime/config/terminal_command_permission_policy.json");
    std::fs::create_dir_all(
        policy_path
            .parent()
            .expect("terminal permission policy parent"),
    )
    .expect("mkdir");
    std::fs::write(&policy_path, r#"{"ask_rules":["Bash(echo *)"]}"#).expect("write policy");
    let out = execute_tool_call_with_recovery(
        root.path(),
        &snapshot,
        "agent-turnloop-pre-gate",
        None,
        "terminal_exec",
        &json!({"command":"echo hello"}),
    );
    assert_eq!(
        out.get("error").and_then(Value::as_str),
        Some("tool_confirmation_required")
    );
    assert_eq!(
        out.pointer("/permission_gate/verdict")
            .and_then(Value::as_str),
        Some("ask")
    );
}

#[test]
fn execute_tool_recovery_emits_nexus_connection_metadata() {
    let root = tempfile::tempdir().expect("tempdir");
    let snapshot = json!({"ok": true});
    let out = execute_tool_call_with_recovery(
        root.path(),
        &snapshot,
        "agent-nexus-route",
        None,
        "file_read",
        &json!({"path":"README.md"}),
    );
    assert!(out.get("nexus_connection").is_some());
    assert_eq!(
        out.pointer("/nexus_connection/source")
            .and_then(Value::as_str),
        Some("client_ingress")
    );
    assert_eq!(
        out.pointer("/nexus_connection/delivery/allowed")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        out.pointer("/tool_pipeline/normalized_result/tool_name")
            .and_then(Value::as_str),
        Some("file_read")
    );
}

#[test]
fn summarize_tool_payload_prefers_claim_bundle_findings_when_available() {
    let payload = json!({
        "ok": true,
        "summary": "raw summary should not win",
        "tool_pipeline": {
            "claim_bundle": {
                "claims": [
                    {"status":"supported","text":"Framework A shows higher task completion consistency under constrained retries."},
                    {"status":"partial","text":"Framework B has better ecosystem coverage but weaker deterministic controls."},
                    {"status":"unsupported","text":"ignore me"}
                ]
            }
        }
    });
    let summary = summarize_tool_payload("web_search", &payload);
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.starts_with("key findings:"));
    assert!(lowered.contains("framework a"));
    assert!(lowered.contains("framework b"));
    assert!(!lowered.contains("ignore me"));
}
