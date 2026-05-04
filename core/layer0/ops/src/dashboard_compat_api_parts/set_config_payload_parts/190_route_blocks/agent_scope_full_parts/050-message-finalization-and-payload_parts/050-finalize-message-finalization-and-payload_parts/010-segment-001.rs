
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
    _state: Value,
    _messages: Vec<Value>,
    active_messages: Vec<Value>,
    provider: String,
    model: String,
    _requested_provider: String,
    _requested_model: String,
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
    _inline_tools_allowed: bool,
) -> CompatApiResponse {
    let initial_draft_response = clean_chat_text(&response_text, 32_000);
    let initial_ack_only = response_looks_like_tool_ack_without_findings(&initial_draft_response)
        || response_is_no_findings_placeholder(&initial_draft_response);
    let web_intent_route = String::new();
    let web_intent_detected = false;
    let web_intent_source = "workflow_llm_manual_only";
    let web_intent_confidence = 0.0;
    let web_forced_fallback_attempted = false;
    let latest_assistant_text = latest_assistant_message_text(&active_messages);
    let workflow_provider = provider.clone();
    let workflow_model = model.clone();
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
    let mut response_text = response_workflow
        .get("response")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_default();
    let mut finalized_response = clean_chat_text(&response_text, 32_000);
    let mut tool_completion = json!({});
    let workflow_status = workflow_final_response_status(&response_workflow);
    let mut workflow_used = workflow_final_response_used(&response_workflow);
    let mut finalization_outcome = if workflow_used {
        "workflow_authored".to_string()
    } else {
        "workflow_llm_unavailable".to_string()
