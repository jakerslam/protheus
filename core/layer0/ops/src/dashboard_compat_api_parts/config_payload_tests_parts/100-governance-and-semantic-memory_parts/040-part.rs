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
    assert!(
        lowered.contains("benchmark summary")
            || lowered.contains("usable tool findings from this turn yet")
    );
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
fn follow_up_suggestion_tool_intent_requires_query_for_infring_web_search_prompt() {
    let (tool, payload) =
        follow_up_suggestion_tool_intent_from_message("Run `infring web search` as the next safe step.")
            .expect("route");
    assert_eq!(tool, "tool_command_router");
    let message = payload.get("message").and_then(Value::as_str).unwrap_or("");
    assert!(message.contains("needs a query"));
    assert!(message.contains("top AI agent frameworks"));
}

#[test]
fn maybe_tooling_failure_fallback_rewrites_safe_step_prompt() {
    let fallback = maybe_tooling_failure_fallback(
        "Run `infring web search` as the next safe step.",
        &no_findings_user_facing_response(),
        "",
    )
    .expect("fallback");
    let lowered = fallback.to_ascii_lowercase();
    assert!(lowered.contains("needs a query"));
    assert!(!response_is_no_findings_placeholder(&fallback));
}

#[test]
fn maybe_tooling_failure_fallback_rewrites_web_file_better_prompt() {
    let fallback = maybe_tooling_failure_fallback(
        "does the web or file tooling seem any better?",
        &no_findings_user_facing_response(),
        "",
    )
    .expect("fallback");
    let lowered = fallback.to_ascii_lowercase();
    assert!(lowered.contains("routing/finalization miss"));
    assert!(lowered.contains("web_search"));
    assert!(!response_is_no_findings_placeholder(&fallback));
}

#[test]
fn maybe_tooling_failure_fallback_rewrites_route_mapping_suggestion_prompt() {
    let fallback = maybe_tooling_failure_fallback(
        "Run Improve command-to-route mapping for higher supported tool hit rate",
        &no_findings_user_facing_response(),
        "",
    )
    .expect("fallback");
    let lowered = fallback.to_ascii_lowercase();
    assert!(lowered.contains("implementation task"));
    assert!(lowered.contains("command-to-route mapping"));
    assert!(!response_is_no_findings_placeholder(&fallback));
}

#[test]
fn maybe_tooling_failure_fallback_rewrites_spawn_route_prompt() {
    let fallback = maybe_tooling_failure_fallback(
        "Implement a supported Rust route for `tool::spawn_subagents`",
        &no_findings_user_facing_response(),
        "",
    )
    .expect("fallback");
    let lowered = fallback.to_ascii_lowercase();
    assert!(lowered.contains("runtime-route implementation task"));
    assert!(lowered.contains("spawn_subagents"));
    assert!(!response_is_no_findings_placeholder(&fallback));
}

#[test]
fn direct_message_safe_step_prompt_returns_actionable_query_required_copy() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let parent = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"follow-up-suggestion-agent","role":"operator"}"#,
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
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{parent_id}/message"),
        br#"{"message":"Run `infring web search` as the next safe step."}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(response_text.contains("needs a query"), "{response_text}");
    assert!(!response_is_no_findings_placeholder(response_text));
    let tool_name = response
        .payload
        .get("tools")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("");
    assert_eq!(tool_name, "tool_command_router");
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
    assert!(!lowered.contains("source-backed findings in this turn"));
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
fn tool_completion_contract_rewrites_unverified_execution_claim_when_no_tools_exist() {
    let (finalized, report) = enforce_tool_completion_contract(
        "Batch execution initiated - 5 concurrent searches running. This demonstrates the full pipeline."
            .to_string(),
        &[],
    );
    let lowered = finalized.to_ascii_lowercase();
    assert!(lowered.contains("usable tool findings from this turn yet"));
    assert!(!lowered.contains("batch execution initiated"));
    assert_eq!(
        report.get("completion_state").and_then(Value::as_str),
        Some("not_applicable")
    );
}

#[test]
fn response_ack_detector_flags_batch_execution_scaffold_copy() {
    assert!(response_looks_like_tool_ack_without_findings(
        "Batch execution initiated - 5 concurrent searches running with concurrency limiting. This demonstrates the full pipeline from command parsing to response formatting."
    ));
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
fn enforce_user_facing_finalization_contract_uses_tool_failure_reason_when_payload_is_unsynthesized() {
    let cards = vec![json!({
        "name": "web_search",
        "is_error": true,
        "status": "timeout",
        "result": "provider timeout after 30s"
    })];
    let (finalized, report, outcome) = enforce_user_facing_finalization_contract(
        "I completed the tool call, but no synthesized response was available yet. Check the tool details below.".to_string(),
        &cards,
    );
    let lowered = finalized.to_ascii_lowercase();
    assert!(
        lowered.contains("provider timeout"),
        "finalized={finalized} outcome={outcome} report={report}"
    );
    assert!(!response_is_no_findings_placeholder(&finalized));
    assert_eq!(
        report.get("completion_state").and_then(Value::as_str),
        Some("reported_reason")
    );
    assert!(outcome.contains("tool_failure_reason"));
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
    let root = governance_temp_root();
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
    assert!(lowered.contains("usable tool findings"));
    assert!(out.get("turn_loop_post_filter").is_some());
    assert!(out.get("turn_loop_tracking").is_some());
}

#[test]
fn execute_tool_recovery_blocks_when_pre_gate_requires_confirmation() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
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
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
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
