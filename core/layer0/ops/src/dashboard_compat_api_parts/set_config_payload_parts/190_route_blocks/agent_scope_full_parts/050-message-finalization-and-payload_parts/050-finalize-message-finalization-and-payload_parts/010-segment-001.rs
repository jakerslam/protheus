
fn finalize_message_finalization_and_payload(
    root: &Path,
    agent_id: &str,
    message: &str,
    result: &Value,
    response_text: String,
    mut response_tools: Vec<Value>,
    workflow_mode: String,
    workflow_system_events: Vec<Value>,
    runtime_summary: Value,
    state: Value,
    messages: Vec<Value>,
    active_messages: Vec<Value>,
    provider: String,
    model: String,
    requested_provider: String,
    requested_model: String,
    auto_route: Option<Value>,
    virtual_key_id: String,
    virtual_key_gate: Value,
    fallback_window: i64,
    context_active_tokens: i64,
    context_ratio: f64,
    context_pressure: String,
    context_pool_limit_tokens: i64,
    context_pool_tokens: i64,
    pooled_messages_len: usize,
    sessions_total: usize,
    memory_kv_entries: usize,
    active_context_target_tokens: i64,
    active_context_min_recent: usize,
    include_all_sessions_context: bool,
    pre_generation_pruned: bool,
    recent_floor_enforced: bool,
    recent_floor_injected: usize,
    recent_floor_target: usize,
    recent_floor_missing_before: usize,
    recent_floor_satisfied: bool,
    recent_floor_coverage_before: f64,
    recent_floor_coverage_after: f64,
    recent_floor_active_missing: usize,
    recent_floor_active_satisfied: bool,
    recent_floor_active_coverage: f64,
    recent_floor_continuity_status: String,
    recent_floor_continuity_action: String,
    recent_floor_continuity_message: String,
    history_trim_confirmed: bool,
    emergency_compact: Value,
    workspace_hints: Value,
    latent_tool_candidates: Value,
    inline_tools_allowed: bool,
) -> CompatApiResponse {
    let initial_draft_response = clean_chat_text(&response_text, 32_000);
    let initial_ack_only = response_looks_like_tool_ack_without_findings(&initial_draft_response)
        || response_is_no_findings_placeholder(&initial_draft_response);
    let web_intent = natural_web_intent_from_user_message(message);
    let finalization_tool_gate = workflow_turn_tool_decision_tree(message);
    let finalization_meta_control = finalization_tool_gate
        .get("meta_control_message")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let finalization_status_check = finalization_tool_gate
        .get("status_check_message")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let finalization_requires_live_web = finalization_tool_gate
        .get("requires_live_web")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let finalization_should_call_tools = finalization_tool_gate
        .get("should_call_tools")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let draft_retry_web_signal = draft_response_implies_retryable_web_failure(&initial_draft_response)
        && finalization_requires_live_web
        && finalization_should_call_tools
        && !finalization_meta_control
        && !finalization_status_check
        && !message_explicitly_disallows_tool_calls(message);
    let web_intent_route = web_intent
        .as_ref()
        .map(|(tool, _)| clean_text(tool, 80))
        .unwrap_or_default();
    let web_intent_detected = web_intent.is_some() || draft_retry_web_signal;
    let web_intent_source = if web_intent.is_some() {
        "message"
    } else if draft_retry_web_signal {
        "draft_retry_signal"
    } else {
        "none"
    };
    let web_intent_confidence = if web_intent.is_some() {
        0.92
    } else if draft_retry_web_signal {
        0.64
    } else {
        0.0
    };
    let mut web_forced_fallback_attempted = false;
    if web_intent_detected && !response_tools_include_web_attempt(&response_tools) {
        let fallback_query = web_intent
            .as_ref()
            .and_then(|(_, payload)| {
                payload
                    .get("query")
                    .or_else(|| payload.get("q"))
                    .and_then(Value::as_str)
                    .map(|raw| clean_text(raw, 600))
            })
            .filter(|query| !query.is_empty())
            .unwrap_or_else(|| {
                fallback_live_web_query_from_failed_draft(message, &initial_draft_response)
            });
        if !fallback_query.is_empty() {
            let forced_payload = execute_tool_call_with_recovery(
                root,
                &state,
                agent_id,
                None,
                "batch_query",
                &json!({
                    "source": "web",
                    "query": fallback_query.clone(),
                    "aperture": "medium",
                    "diagnostic": "forced_live_web_invariant"
                }),
            );
            let ok = forced_payload
                .get("ok")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let result_text = summarize_tool_payload("batch_query", &forced_payload);
            let status = tool_card_status_from_payload(&forced_payload);
            response_tools.push(json!({
                "id": format!("tool-batch_query-forced-{}", response_tools.len()),
                "name": "batch_query",
                "input": trim_text(
                    &json!({
                        "source": "web",
                        "query": fallback_query.clone(),
                        "aperture": "medium",
                        "diagnostic": "forced_live_web_invariant"
                    }).to_string(),
                    4000
                ),
                "result": trim_text(&result_text, 24_000),
                "is_error": !ok,
                "blocked": status == "blocked" || status == "policy_denied",
                "status": status,
                "tool_attempt_receipt": forced_payload
                    .pointer("/tool_pipeline/tool_attempt_receipt")
                    .cloned()
                    .unwrap_or(Value::Null)
            }));
            web_forced_fallback_attempted = true;
        }
    }
    let memory_fallback = if memory_recall_requested(message) {
        Some(build_memory_recall_response(&state, &messages, message))
    } else {
        None
    };
    let latest_assistant_text = latest_assistant_message_text(&active_messages);
    let final_synthesis_needs_recovery_model =
        response_text.trim().is_empty()
            || response_requires_visible_repair_for_message(message, &response_text, &response_tools);
    let (workflow_provider, workflow_model) = if final_synthesis_needs_recovery_model {
        visible_response_recovery_model(&provider, &model)
    } else {
        (provider.clone(), model.clone())
    };
    let mut response_workflow = run_turn_workflow_final_response(
        root,
        &workflow_provider,
        &workflow_model,
        &active_messages,
        message,
        &workflow_mode,
        &response_tools,
        &workflow_system_events,
        &response_text,
        &latest_assistant_text,
    );
    if final_synthesis_needs_recovery_model {
        response_workflow["visible_response_recovery_model"] = json!({
            "provider": workflow_provider,
            "model": workflow_model
        });
    }
    let mut response_text = response_workflow
        .get("response")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_default();
    let mut finalized_response = clean_chat_text(&response_text, 32_000);
    let mut tool_completion = json!({});
    let workflow_status = workflow_final_response_status(&response_workflow);
    let mut workflow_used = workflow_final_response_used(&response_workflow);
    let workflow_fallback_allowed =
        workflow_final_response_allows_system_fallback(&response_workflow);
    let mut finalization_outcome = if workflow_used {
        "workflow_authored".to_string()
    } else {
        "workflow_llm_unavailable".to_string()
