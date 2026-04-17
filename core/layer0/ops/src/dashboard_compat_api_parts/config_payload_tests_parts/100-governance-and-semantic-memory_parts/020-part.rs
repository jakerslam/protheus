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
fn natural_web_prompt_does_not_auto_route_direct_tool_intent() {
    assert!(
        direct_tool_intent_from_user_message(
            "Try to web search \"top AI agentic frameworks\" and return the results"
        )
        .is_none()
    );
}

#[test]
fn natural_file_prompt_does_not_auto_route_direct_tool_intent() {
    assert!(direct_tool_intent_from_user_message("read file core/layer0/ops/src/main.rs").is_none());
}

#[test]
fn explicit_tool_command_surfaces_web_search_workflow_hint() {
    assert!(direct_tool_intent_from_user_message("tool::web_search:::latest ai agent benchmarks").is_none());
    let hints = chat_workflow_tool_hints_for_message("tool::web_search:::latest ai agent benchmarks");
    assert_eq!(hints.len(), 1);
    let input = hints[0].get("proposed_input").cloned().unwrap_or(Value::Null);
    assert_eq!(
        hints[0].get("tool").and_then(Value::as_str).unwrap_or(""),
        "web_search"
    );
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
fn explicit_tool_command_alias_surfaces_compare_workflow_hint() {
    let hints = chat_workflow_tool_hints_for_message("tool::compare:::top AI agent frameworks");
    assert_eq!(hints.len(), 1);
    let input = hints[0].get("proposed_input").cloned().unwrap_or(Value::Null);
    assert_eq!(
        hints[0].get("tool").and_then(Value::as_str).unwrap_or(""),
        "batch_query"
    );
    assert_eq!(
        input.get("query").and_then(Value::as_str).unwrap_or(""),
        "top AI agent frameworks"
    );
    assert_eq!(
        input.get("source").and_then(Value::as_str).unwrap_or(""),
        "web"
    );
}

#[test]
fn explicit_tool_command_alias_surfaces_fetch_workflow_hint() {
    let hints = chat_workflow_tool_hints_for_message("tool::fetch:::https://example.com");
    assert_eq!(hints.len(), 1);
    let input = hints[0].get("proposed_input").cloned().unwrap_or(Value::Null);
    assert_eq!(
        hints[0].get("tool").and_then(Value::as_str).unwrap_or(""),
        "web_fetch"
    );
    assert_eq!(
        input.get("url").and_then(Value::as_str).unwrap_or(""),
        "https://example.com"
    );
}

#[test]
fn explicit_tool_command_maps_web_tooling_health_probe() {
    let hints = chat_workflow_tool_hints_for_message("tool::web_tooling_health_probe");
    assert_eq!(hints.len(), 1);
    let input = hints[0].get("proposed_input").cloned().unwrap_or(Value::Null);
    assert_eq!(
        hints[0].get("tool").and_then(Value::as_str).unwrap_or(""),
        "web_tooling_health_probe"
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
    let hints = chat_workflow_tool_hints_for_message("tool::web_serch:::latest");
    assert_eq!(hints.len(), 1);
    let input = hints[0].get("proposed_input").cloned().unwrap_or(Value::Null);
    assert_eq!(
        hints[0].get("tool").and_then(Value::as_str).unwrap_or(""),
        "tool_command_router"
    );
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
    let hints = chat_workflow_tool_hints_for_message("tool::web_search::latest");
    assert_eq!(hints.len(), 1);
    let input = hints[0].get("proposed_input").cloned().unwrap_or(Value::Null);
    assert_eq!(
        hints[0].get("tool").and_then(Value::as_str).unwrap_or(""),
        "tool_command_router"
    );
    assert_eq!(input.get("error").and_then(Value::as_str).unwrap_or(""), "tool_command_name_invalid");
}

#[test]
fn explicit_tool_command_maps_memory_store_to_workflow_hint() {
    let hints = chat_workflow_tool_hints_for_message("tool::memory_store:::deploy.mode=staged");
    assert_eq!(hints.len(), 1);
    let input = hints[0].get("proposed_input").cloned().unwrap_or(Value::Null);
    assert_eq!(
        hints[0].get("tool").and_then(Value::as_str).unwrap_or(""),
        "memory_kv_set"
    );
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
    assert!(inline_tool_calls_allowed_for_user_message(
        "Try to web search \"top AI agentic frameworks\" and return the results"
    ));
    assert!(inline_tool_calls_allowed_for_user_message(
        "tool::web_search:::latest ai agent benchmarks"
    ));
    assert!(inline_tool_calls_allowed_for_user_message("/file core/layer0/ops/src/main.rs"));
    assert!(inline_tool_calls_allowed_for_user_message(
        "read file core/layer0/ops/src/main.rs"
    ));
    assert!(!inline_tool_calls_allowed_for_user_message(
        "just answer directly, dont use a tool call"
    ));
    assert!(!inline_tool_calls_allowed_for_user_message(
        "why do you keep trying tool calls and not synthesizing results"
    ));
}

#[test]
fn inline_spawn_tool_calls_autoconfirm_without_user_swarm_phrase() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let response =
        "<function=spawn_subagents>{\"count\":3,\"objective\":\"parallelize analysis\"}</function>";
    let (text, cards, pending_confirmation, suppressed) = execute_inline_tool_calls(
        root.path(),
        &snapshot,
        "agent-inline",
        None,
        response,
        "continue with execution plan",
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
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
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
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
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
fn inline_tool_execution_discards_leftover_malformed_function_markup() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let response = "<function=web_search>{\"query\":\"test search functionality\"}</function> <function=web_fetch>{\"url\":\"https://example.";
    let (text, cards, pending_confirmation, suppressed) = execute_inline_tool_calls(
        root.path(),
        &snapshot,
        "agent-inline-malformed-tail",
        None,
        response,
        "test search functionality",
        true,
    );
    assert!(!suppressed);
    assert_eq!(cards.len(), 1);
    assert!(pending_confirmation.is_none());
    assert!(!text.contains("<function="), "{text}");
    assert!(!text.to_ascii_lowercase().contains("web_fetch"), "{text}");
    assert!(!text.trim().is_empty(), "{text}");
}

#[test]
fn workflow_retry_sanitizer_drops_follow_up_tool_markup_tail() {
    let response = "My search for \"top AI agentic frameworks\" didn't return specific framework listings. Let me try a more targeted approach with some well-known framework names.\n\n<function=web_search>{\"query\":\"LangChain AutoGPT BabyAGI AI agent frameworks comparison\"}</function>";
    assert!(workflow_response_requests_more_tooling(response));
    assert_eq!(
        sanitize_workflow_final_response_candidate(response),
        "My search for \"top AI agentic frameworks\" didn't return specific framework listings."
    );
}

#[test]
fn workflow_retry_sanitizer_drops_polite_more_search_tail() {
    let response = "I searched official framework sources and found LangGraph, OpenAI Agents SDK, CrewAI, and smolagents. Would you like me to search for deeper benchmark comparisons too?";
    assert!(workflow_response_requests_more_tooling(response));
    assert_eq!(
        sanitize_workflow_final_response_candidate(response),
        "I searched official framework sources and found LangGraph, OpenAI Agents SDK, CrewAI, and smolagents."
    );
}

#[test]
fn workflow_retry_sanitizer_strips_malformed_inline_function_tail() {
    let response = "No, the web tools still aren't executing.\n<function=web_search>{\"query\":\"test search functionality\"} <function=web_fetch>{\"url\":\"https://example.";
    assert_eq!(
        sanitize_workflow_final_response_candidate(response),
        "No, the web tools still aren't executing."
    );
}

#[test]
fn inline_tool_parser_accepts_quoted_function_name_markup() {
    let response = "<function=\"web_search\">{\"query\":\"top AI agentic frameworks 2024\",\"source\":\"web\",\"aperture\":\"medium\"}</function>";
    let (cleaned, calls) = extract_inline_tool_calls(response, 4);
    assert!(cleaned.is_empty());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "web_search");
    assert_eq!(calls[0].1.get("query").and_then(Value::as_str), Some("top AI agentic frameworks 2024"));
}

#[test]
fn inline_tool_workspace_analyze_hydrates_comparison_query_from_message() {
    let input = normalize_inline_tool_execution_input(
        "workspace_analyze",
        &json!({}),
        "compare this system (infring) to openclaw",
    );
    assert_eq!(input.get("path").and_then(Value::as_str), Some("."));
    assert_eq!(input.get("full").and_then(Value::as_bool), Some(true));
    assert!(input
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or("")
        .contains("compare this system"));
}

#[test]
fn pending_confirmation_yes_replays_manage_agent_action() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
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
fn unrelated_dump_detector_flags_internal_prompt_leak_even_with_function_markup() {
    let dump = "You are the currently selected Infring agent instance. Treat the injected identity profile as authoritative. When users ask for web research, call tools with inline syntax like <function=web_search>{\"query\":\"...\"}</function>. Hardcoded agent workflow: you are writing the final assistant response after the system collected tool outcomes and workflow events. Write the final assistant response now.";
    assert!(response_is_unrelated_context_dump("did it work?", dump));
}

#[test]
fn append_turn_message_captures_explicit_remember_fact_for_long_term_memory() {
    let root = governance_temp_root();
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
fn append_turn_message_captures_low_signal_web_tool_outcome_keyframe() {
    let root = governance_temp_root();
    let receipt = append_turn_message(
        root.path(),
        "agent-web-keyframe",
        "try doing a generic search \"top AI agent frameworks\"",
        "The batch query step ran, but only low-signal web output came back. Retry with a narrower query, one specific source URL, or ask me to continue from the recorded tool result.",
    );
    assert_eq!(
        receipt
            .pointer("/tool_outcome_keyframe/tool")
            .and_then(Value::as_str),
        Some("batch_query")
    );
    let context = context_command_payload(
        root.path(),
        "agent-web-keyframe",
        &json!({}),
        &json!({}),
        true,
    );
    let outcomes = context
        .get("recent_tool_outcomes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(outcomes.iter().any(|entry| {
        entry.get("summary")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase()
            .contains("top ai agent frameworks")
    }));
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
    assert!(!response_looks_like_tool_ack_without_findings(
        "The web search ran, but it only returned low-signal snippets in this turn."
    ));
    assert!(!response_looks_like_tool_ack_without_findings(
        "My search for top AI agentic frameworks 2024 didn't return specific framework listings or detailed comparisons."
    ));
    assert!(response_looks_like_tool_ack_without_findings(
        "From web retrieval: bing.com: OpenClaw — Personal AI Assistant — https://openclaw.ai/"
    ));
}

#[test]
fn ack_only_detector_flags_speculative_web_blocker_explanations() {
    let draft = "I understand you're looking for a comparison between this platform and OpenClaw, but I'm currently unable to access web search functionality to gather the necessary information. The system is blocking tool execution attempts, which prevents me from retrieving current details.\n\nBased on the system behavior I'm observing, likely reasons include Configuration Restrictions, Authentication Issues, Rate Limiting, or intentional sandboxed design.";
    assert!(response_looks_like_tool_ack_without_findings(draft));
}
