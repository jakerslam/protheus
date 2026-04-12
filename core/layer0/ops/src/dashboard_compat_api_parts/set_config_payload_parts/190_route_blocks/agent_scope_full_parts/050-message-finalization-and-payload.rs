fn finalize_message_finalization_and_payload(
    root: &Path,
    agent_id: &str,
    message: &str,
    result: &Value,
    mut response_text: String,
    response_tools: Vec<Value>,
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
    history_trim_confirmed: bool,
    emergency_compact: Value,
    workspace_hints: Value,
    latent_tool_candidates: Value,
    inline_tools_allowed: bool,
) -> CompatApiResponse {
    let response_workflow = run_turn_workflow_final_response(
        root,
        &provider,
        &model,
        &active_messages,
        message,
        &workflow_mode,
        &response_tools,
        &workflow_system_events,
        &response_text,
    );
    if let Some(synthesized) = response_workflow.get("response").and_then(Value::as_str) {
        response_text = synthesized.to_string();
    }
    let (mut finalized_response, mut tool_completion, seed_outcome) =
        enforce_user_facing_finalization_contract(response_text, &response_tools);
    let mut finalization_outcome = clean_text(&seed_outcome, 200);
    let workflow_status = clean_text(
        response_workflow
            .pointer("/final_llm_response/status")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    if !workflow_status.is_empty() {
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            &format!("workflow:{workflow_status}"),
            200,
        );
    }
    let mut tool_synthesis_retry_used = false;
    let initial_ack_only = tool_completion
        .get("initial_ack_only")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut retry_attempted = false;
    let mut retry_used = false;
    if initial_ack_only
        && tool_completion
            .get("final_ack_only")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        retry_attempted = true;
        let strict_tool_prompt = clean_text(
            &format!(
                "{}\n\nOutput guard: Return synthesized findings or an explicit no-findings reason. Do not output tool status text like 'Web search completed' or 'Tool call finished'.",
                AGENT_RUNTIME_SYSTEM_PROMPT
            ),
            12_000,
        );
        if let Ok(retried) = crate::dashboard_provider_runtime::invoke_chat(
            root,
            &provider,
            &model,
            &strict_tool_prompt,
            &active_messages,
            message,
        ) {
            let mut retried_text = clean_chat_text(
                retried
                    .get("response")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                32_000,
            );
            retried_text = strip_internal_context_metadata_prefix(&retried_text);
            retried_text = strip_internal_cache_control_markup(&retried_text);
            if !user_requested_internal_runtime_details(message) {
                retried_text = abstract_runtime_mechanics_terms(&retried_text);
            }
            let (retry_finalized, _retry_report, retry_outcome) =
                enforce_user_facing_finalization_contract(retried_text, &response_tools);
            finalized_response = retry_finalized;
            finalization_outcome = merge_response_outcomes(
                &finalization_outcome,
                &format!("retry:{retry_outcome}"),
                200,
            );
            retry_used = true;
        }
    }
    if let Some(retried_text) = maybe_synthesize_tool_turn_response(
        root,
        &provider,
        &model,
        &active_messages,
        message,
        &response_tools,
        &finalized_response,
    ) {
        let (retry_finalized, _retry_report, retry_outcome) =
            enforce_user_facing_finalization_contract(retried_text, &response_tools);
        if !response_is_no_findings_placeholder(&retry_finalized)
            || response_tools_failure_reason_for_user(&response_tools, 4).is_empty()
        {
            finalized_response = retry_finalized;
            finalization_outcome = merge_response_outcomes(
                &finalization_outcome,
                &format!("tool_synthesis_retry:{retry_outcome}"),
                200,
            );
            tool_synthesis_retry_used = true;
        }
    }
    let mut synthesis_retry_used = false;
    if response_is_no_findings_placeholder(&finalized_response)
        && message_requests_comparative_answer(message)
    {
        let synthesis_prompt = clean_text(
            &format!(
                "{}\n\nFallback guard: if tool extraction failed or returned no usable findings, still answer the user directly using stable knowledge. Prioritize relevance to the latest request and return usable content in the requested format.",
                AGENT_RUNTIME_SYSTEM_PROMPT
            ),
            12_000,
        );
        if let Ok(retried) = crate::dashboard_provider_runtime::invoke_chat(
            root,
            &provider,
            &model,
            &synthesis_prompt,
            &active_messages,
            message,
        ) {
            let mut retried_text = clean_chat_text(
                retried
                    .get("response")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                32_000,
            );
            retried_text = strip_internal_context_metadata_prefix(&retried_text);
            retried_text = strip_internal_cache_control_markup(&retried_text);
            if !user_requested_internal_runtime_details(message) {
                retried_text = abstract_runtime_mechanics_terms(&retried_text);
            }
            if !response_is_unrelated_context_dump(message, &retried_text) {
                let (retry_finalized, _retry_report, retry_outcome) =
                    enforce_user_facing_finalization_contract(retried_text, &response_tools);
                if !response_is_no_findings_placeholder(&retry_finalized) {
                    finalized_response = retry_finalized;
                    finalization_outcome = merge_response_outcomes(
                        &finalization_outcome,
                        &format!("synthesis_retry:{retry_outcome}"),
                        200,
                    );
                    synthesis_retry_used = true;
                }
            }
        }
    }
    if response_is_no_findings_placeholder(&finalized_response)
        && message_requests_comparative_answer(message)
    {
        finalized_response = comparative_no_findings_fallback(message);
        finalization_outcome = format!("{finalization_outcome}+comparative_fallback");
    }
    if response_is_no_findings_placeholder(&finalized_response) && !response_tools.is_empty() {
        let tool_failure_reason = response_tools_failure_reason_for_user(&response_tools, 4);
        if !tool_failure_reason.is_empty() {
            finalized_response = tool_failure_reason;
            finalization_outcome =
                merge_response_outcomes(&finalization_outcome, "tool_failure_reason", 200);
        }
    }
    if response_tools.is_empty()
        && !inline_tools_allowed
        && response_is_no_findings_placeholder(&finalized_response)
    {
        let direct_chat_repair_prompt = clean_text(
            &format!(
                "{}\n\nConversational recovery: answer directly in natural language without tools. Do not mention missing findings unless the user explicitly requested a tool call.",
                AGENT_RUNTIME_SYSTEM_PROMPT
            ),
            12_000,
        );
        if let Ok(retried) = crate::dashboard_provider_runtime::invoke_chat(
            root,
            &provider,
            &model,
            &direct_chat_repair_prompt,
            &active_messages,
            message,
        ) {
            let mut retried_text = clean_chat_text(
                retried
                    .get("response")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                32_000,
            );
            retried_text = strip_internal_context_metadata_prefix(&retried_text);
            retried_text = strip_internal_cache_control_markup(&retried_text);
            if !user_requested_internal_runtime_details(message) {
                retried_text = abstract_runtime_mechanics_terms(&retried_text);
            }
            if !response_is_unrelated_context_dump(message, &retried_text) {
                let (retry_finalized, _retry_report, retry_outcome) =
                    enforce_user_facing_finalization_contract(retried_text, &response_tools);
                if !response_is_no_findings_placeholder(&retry_finalized) {
                    finalized_response = retry_finalized;
                    finalization_outcome = merge_response_outcomes(
                        &finalization_outcome,
                        &format!("conversation_retry:{retry_outcome}"),
                        200,
                    );
                }
            }
        }
        if response_is_no_findings_placeholder(&finalized_response) {
            finalized_response =
                "I can answer directly without tool calls. Ask your question naturally and I’ll respond conversationally unless you explicitly request a tool run.".to_string();
            finalization_outcome = format!("{finalization_outcome}+conversation_fallback");
        }
    }
    let mut tooling_fallback_used = false;
    if let Some(tooling_fallback) = maybe_tooling_failure_fallback(
        message,
        &finalized_response,
        &latest_assistant_message_text(&active_messages),
    ) {
        finalized_response = tooling_fallback;
        finalization_outcome = format!("{finalization_outcome}+tooling_failure_fallback");
        tooling_fallback_used = true;
    }
    let (contract_finalized, contract_report, contract_outcome) =
        enforce_user_facing_finalization_contract(finalized_response, &response_tools);
    finalized_response = contract_finalized;
    tool_completion = contract_report;
    tool_completion = enrich_tool_completion_receipt(tool_completion, &response_tools);
    finalization_outcome = merge_response_outcomes(&finalization_outcome, &contract_outcome, 200);
    response_text = finalized_response;
    if memory_recall_requested(message)
        && (response_is_no_findings_placeholder(&response_text)
            || response_looks_like_tool_ack_without_findings(&response_text))
    {
        response_text = build_memory_recall_response(&state, &messages, message);
    }
    response_text = ensure_tool_turn_response_text(&response_text, &response_tools);
    let final_ack_only = response_looks_like_tool_ack_without_findings(&response_text);
    let response_finalization = json!({
        "applied": finalization_outcome != "unchanged",
        "outcome": finalization_outcome,
        "initial_ack_only": initial_ack_only,
        "final_ack_only": final_ack_only,
        "findings_available": tool_completion
            .get("findings_available")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "tool_completion": tool_completion,
        "retry_attempted": retry_attempted,
        "retry_used": retry_used,
        "tool_synthesis_retry_used": tool_synthesis_retry_used,
        "synthesis_retry_used": synthesis_retry_used,
        "tooling_fallback_used": tooling_fallback_used
    });
    let turn_transaction = crate::dashboard_tool_turn_loop::turn_transaction_payload(
        "complete",
        if response_tools.is_empty() {
            "none"
        } else {
            "complete"
        },
        "complete",
        "complete",
    );
    let terminal_transcript = tool_terminal_transcript(&response_tools);
    let mut turn_receipt = append_turn_message(root, agent_id, message, &response_text);
    turn_receipt["assistant_turn_patch"] = persist_last_assistant_turn_metadata(
        root,
        agent_id,
        &response_text,
        &json!({
            "tools": response_tools.clone(),
            "response_workflow": response_workflow.clone(),
            "response_finalization": response_finalization.clone(),
            "turn_transaction": turn_transaction.clone(),
            "terminal_transcript": terminal_transcript.clone()
        }),
    );
    turn_receipt["response_finalization"] = response_finalization.clone();
    let runtime_model = clean_text(
        result
            .get("runtime_model")
            .and_then(Value::as_str)
            .unwrap_or(&model),
        240,
    );
    let mut runtime_patch = json!({
        "runtime_model": runtime_model,
        "context_window": result.get("context_window").cloned().unwrap_or_else(|| json!(0)),
        "context_window_tokens": result.get("context_window").cloned().unwrap_or_else(|| json!(0)),
        "updated_at": crate::now_iso()
    });
    if auto_route.is_some() {
        runtime_patch["runtime_provider"] = json!(provider.clone());
        if !requested_provider.eq_ignore_ascii_case("auto")
            && !requested_model.is_empty()
            && !requested_model.eq_ignore_ascii_case("auto")
        {
            runtime_patch["model_provider"] = json!(provider.clone());
            runtime_patch["model_name"] = json!(model.clone());
            runtime_patch["model_override"] = json!(format!("{provider}/{model}"));
        }
    }
    let _ = update_profile_patch(root, agent_id, &runtime_patch);
    let mut payload = result.clone();
    payload["ok"] = json!(true);
    payload["agent_id"] = json!(agent_id);
    payload["provider"] = json!(provider);
    payload["model"] = json!(model);
    payload["iterations"] = json!(1);
    payload["response"] = json!(response_text);
    payload["runtime_sync"] = runtime_summary;
    payload["tools"] = Value::Array(response_tools);
    payload["response_workflow"] = response_workflow;
    payload["terminal_transcript"] = Value::Array(terminal_transcript);
    payload["response_finalization"] = response_finalization;
    payload["turn_transaction"] = turn_transaction;
    payload["context_window"] = json!(fallback_window.max(0));
    payload["context_tokens"] = json!(context_active_tokens.max(0));
    payload["context_used_tokens"] = json!(context_active_tokens.max(0));
    payload["context_ratio"] = json!(context_ratio);
    payload["context_pressure"] = json!(context_pressure.clone());
    payload["attention_queue"] = turn_receipt
        .get("attention_queue")
        .cloned()
        .unwrap_or_else(|| json!({}));
    payload["memory_capture"] = turn_receipt
        .get("memory_capture")
        .cloned()
        .unwrap_or_else(|| json!({}));
    payload["context_pool"] = json!({
        "pool_limit_tokens": context_pool_limit_tokens,
        "pool_tokens": context_pool_tokens,
        "pool_messages": pooled_messages_len,
        "session_count": sessions_total,
        "system_context_enabled": true,
        "system_context_limit_tokens": context_pool_limit_tokens,
        "llm_context_window_tokens": fallback_window.max(0),
        "cross_session_memory_enabled": true,
        "memory_kv_entries": memory_kv_entries,
        "active_target_tokens": active_context_target_tokens,
        "active_tokens": context_active_tokens,
        "active_messages": active_messages.len(),
        "min_recent_messages": active_context_min_recent,
        "include_all_sessions_context": include_all_sessions_context,
        "context_window": fallback_window.max(0),
        "context_ratio": context_ratio,
        "context_pressure": context_pressure,
        "pre_generation_pruning_enabled": true,
        "pre_generation_pruned": pre_generation_pruned,
        "recent_floor_enforced": recent_floor_enforced,
        "recent_floor_injected": recent_floor_injected,
        "history_trim_confirmed": history_trim_confirmed,
        "emergency_compact_enabled": true,
        "emergency_compact": emergency_compact
    });
    payload["workspace_hints"] = workspace_hints;
    payload["latent_tool_candidates"] = latent_tool_candidates;
    if let Some(route) = auto_route {
        payload["auto_route"] = route.get("route").cloned().unwrap_or_else(|| route.clone());
    }
    if !virtual_key_id.is_empty() {
        let spend_receipt = crate::dashboard_provider_runtime::record_virtual_key_usage(
            root,
            &virtual_key_id,
            payload
                .get("cost_usd")
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
        );
        payload["virtual_key"] = json!({
            "id": virtual_key_id,
            "reservation": virtual_key_gate,
            "spend": spend_receipt
        });
    }
    CompatApiResponse {
        status: 200,
        payload,
    }
}
