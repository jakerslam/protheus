fn response_requires_visible_repair(text: &str) -> bool {
    let cleaned = clean_chat_text(text, 32_000);
    cleaned.trim().is_empty()
        || response_is_no_findings_placeholder(&cleaned)
        || response_looks_like_tool_ack_without_findings(&cleaned)
        || response_looks_like_unsynthesized_web_snippet_dump(&cleaned)
        || response_looks_like_raw_web_artifact_dump(&cleaned)
}

fn repair_visible_response_after_workflow(
    message: &str,
    candidate_response: &str,
    initial_draft_response: &str,
    latest_assistant_text: &str,
    response_tools: &[Value],
    inline_tools_allowed: bool,
    memory_fallback: Option<&str>,
) -> (String, String, bool, bool) {
    let cleaned = clean_chat_text(candidate_response, 32_000);
    if !response_requires_visible_repair(&cleaned) {
        return (cleaned, "unchanged".to_string(), false, false);
    }

    let cleaned_initial_draft = clean_chat_text(initial_draft_response, 32_000);
    if !response_requires_visible_repair(&cleaned_initial_draft) {
        return (
            cleaned_initial_draft,
            "repaired_with_initial_draft".to_string(),
            false,
            false,
        );
    }

    let cleaned_latest_assistant = clean_chat_text(latest_assistant_text, 32_000);
    if !response_requires_visible_repair(&cleaned_latest_assistant) {
        return (
            cleaned_latest_assistant,
            "repaired_with_latest_assistant".to_string(),
            false,
            false,
        );
    }

    let findings_summary = clean_text(&response_tools_summary_for_user(response_tools, 4), 4_000);
    if !findings_summary.is_empty() {
        return (
            findings_summary,
            "repaired_with_tool_findings_summary".to_string(),
            false,
            false,
        );
    }

    let failure_reason = clean_text(
        &response_tools_failure_reason_for_user(response_tools, 4),
        4_000,
    );
    if !failure_reason.is_empty() {
        return (
            failure_reason,
            "repaired_with_tool_failure_reason".to_string(),
            false,
            false,
        );
    }

    if message_requests_comparative_answer(message) {
        return (
            comparative_no_findings_fallback(message),
            "repaired_with_comparative_guidance".to_string(),
            false,
            true,
        );
    }

    if let Some(tooling_guidance) =
        maybe_tooling_failure_fallback(message, initial_draft_response, latest_assistant_text)
    {
        return (
            tooling_guidance,
            "repaired_with_tooling_guidance".to_string(),
            true,
            false,
        );
    }

    if let Some(memory_response) = memory_fallback {
        let cleaned_memory = clean_chat_text(memory_response, 32_000);
        if !cleaned_memory.is_empty() {
            return (
                cleaned_memory,
                "repaired_with_memory_fallback".to_string(),
                false,
                false,
            );
        }
    }

    if !response_tools.is_empty() {
        let readability_guidance =
            clean_text(&ensure_tool_turn_response_text(initial_draft_response, response_tools), 4_000);
        if !readability_guidance.is_empty() {
            return (
                readability_guidance,
                "repaired_with_tool_readability_guidance".to_string(),
                false,
                false,
            );
        }
    }

    if response_tools.is_empty() && !inline_tools_allowed {
        return (
            "I can answer this directly without tool calls. Ask your question naturally and I’ll respond conversationally unless you explicitly request a tool run.".to_string(),
            "repaired_with_direct_answer_guard".to_string(),
            false,
            false,
        );
    }

    (
        "I completed the workflow gate, but the visible response stayed empty or low-signal. Please retry and I’ll rerun the chain and explain what worked or failed directly.".to_string(),
        "repaired_with_generic_workflow_failure".to_string(),
        false,
        false,
    )
}

fn initial_model_invoke_failure_response(message: &str, err: &str) -> String {
    let cleaned_error = clean_text(err, 220);
    let base = if message_requests_comparative_answer(message) {
        "I couldn’t start the first model step for this comparison turn, so I did not finish gathering workspace and web evidence yet. Retry and I’ll rerun the full chain."
            .to_string()
    } else {
        "I couldn’t start the first model step for this turn, so the workflow could not continue normally. Retry and I’ll rerun the chain."
            .to_string()
    };
    if cleaned_error.is_empty() {
        return base;
    }
    format!("{base} Backend error: {cleaned_error}.")
}

fn finalize_message_invoke_failure_and_payload(
    root: &Path,
    agent_id: &str,
    message: &str,
    provider: &str,
    model: &str,
    error_text: &str,
    active_messages: &[Value],
    workspace_hints: Value,
    latent_tool_candidates: Value,
) -> CompatApiResponse {
    let workflow_events = vec![turn_workflow_event(
        "initial_model_invoke_failed",
        json!({
            "error": clean_text(error_text, 240),
            "provider": clean_text(provider, 80),
            "model": clean_text(model, 240)
        }),
    )];
    let latest_assistant_text = latest_assistant_message_text(active_messages);
    let response_workflow = run_turn_workflow_final_response(
        root,
        provider,
        model,
        active_messages,
        message,
        "model_initial_invoke_failed",
        &[],
        &workflow_events,
        &initial_model_invoke_failure_response(message, error_text),
        &latest_assistant_text,
    );
    let workflow_status = workflow_final_response_status(&response_workflow);
    let workflow_used = workflow_final_response_used(&response_workflow);
    let workflow_fallback_allowed =
        workflow_final_response_allows_system_fallback(&response_workflow);
    let mut response_text = response_workflow
        .get("response")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_default();
    let mut finalization_outcome = if workflow_used {
        "workflow_authored".to_string()
    } else {
        "workflow_llm_unavailable".to_string()
    };
    if !workflow_status.is_empty() {
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            &format!("workflow:{workflow_status}"),
            220,
        );
    }
    let mut workflow_system_fallback_used = false;
    if !workflow_used && workflow_fallback_allowed {
        response_text = initial_model_invoke_failure_response(message, error_text);
        workflow_system_fallback_used = true;
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            "workflow_system_fallback",
            220,
        );
    }
    let (repaired_response, repair_outcome, _, comparative_repair_used) =
        repair_visible_response_after_workflow(
            message,
            &response_text,
            &response_text,
            &latest_assistant_text,
            &[],
            true,
            None,
        );
    let (finalized_response, tool_completion, contract_outcome) =
        enforce_user_facing_finalization_contract(repaired_response, &[]);
    finalization_outcome = merge_response_outcomes(&finalization_outcome, &repair_outcome, 220);
    finalization_outcome = merge_response_outcomes(&finalization_outcome, &contract_outcome, 220);
    let response_finalization = json!({
        "applied": true,
        "outcome": finalization_outcome,
        "initial_ack_only": false,
        "final_ack_only": response_looks_like_tool_ack_without_findings(&finalized_response),
        "findings_available": false,
        "tool_completion": tool_completion,
        "retry_attempted": false,
        "retry_used": false,
        "tool_synthesis_retry_used": false,
        "synthesis_retry_used": false,
        "tooling_fallback_used": false,
        "comparative_fallback_used": comparative_repair_used,
        "workflow_system_fallback_used": workflow_system_fallback_used,
        "visible_response_repaired": repair_outcome != "unchanged",
        "initial_model_invoke_failed": true
    });
    let turn_transaction = crate::dashboard_tool_turn_loop::turn_transaction_payload(
        "complete",
        "none",
        "invoke_failed",
        "complete",
    );
    let terminal_transcript = Vec::<Value>::new();
    let mut turn_receipt = append_turn_message(root, agent_id, message, &finalized_response);
    turn_receipt["assistant_turn_patch"] = persist_last_assistant_turn_metadata(
        root,
        agent_id,
        &finalized_response,
        &json!({
            "tools": [],
            "response_workflow": response_workflow.clone(),
            "response_finalization": response_finalization.clone(),
            "turn_transaction": turn_transaction.clone(),
            "terminal_transcript": terminal_transcript.clone()
        }),
    );
    turn_receipt["response_finalization"] = response_finalization.clone();
    CompatApiResponse {
        status: 200,
        payload: json!({
            "ok": true,
            "agent_id": agent_id,
            "provider": provider,
            "model": model,
            "runtime_model": model,
            "iterations": 1,
            "response": finalized_response,
            "tools": [],
            "response_workflow": response_workflow,
            "response_finalization": response_finalization,
            "turn_transaction": turn_transaction,
            "terminal_transcript": terminal_transcript,
            "workspace_hints": workspace_hints,
            "latent_tool_candidates": latent_tool_candidates,
            "attention_queue": turn_receipt
                .get("attention_queue")
                .cloned()
                .unwrap_or_else(|| json!({})),
            "memory_capture": turn_receipt
                .get("memory_capture")
                .cloned()
                .unwrap_or_else(|| json!({})),
            "error": clean_text(error_text, 280),
            "degraded": true,
            "initial_invoke_error": true
        }),
    }
}

fn finalize_message_finalization_and_payload(
    root: &Path,
    agent_id: &str,
    message: &str,
    result: &Value,
    response_text: String,
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
    let initial_draft_response = clean_chat_text(&response_text, 32_000);
    let initial_ack_only = response_looks_like_tool_ack_without_findings(&initial_draft_response)
        || response_is_no_findings_placeholder(&initial_draft_response);
    let memory_fallback = if memory_recall_requested(message) {
        Some(build_memory_recall_response(&state, &messages, message))
    } else {
        None
    };
    let latest_assistant_text = latest_assistant_message_text(&active_messages);
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
    let workflow_used = workflow_final_response_used(&response_workflow);
    let workflow_fallback_allowed =
        workflow_final_response_allows_system_fallback(&response_workflow);
    let mut finalization_outcome = if workflow_used {
        "workflow_authored".to_string()
    } else {
        "workflow_llm_unavailable".to_string()
    };
    if !workflow_status.is_empty() {
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            &format!("workflow:{workflow_status}"),
            200,
        );
    }
    let mut tooling_fallback_used = false;
    let mut comparative_fallback_used = false;
    let mut workflow_system_fallback_used = false;
    let mut visible_response_repaired = false;
    if workflow_used {
        tool_completion = tool_completion_report_for_response(
            &finalized_response,
            &response_tools,
            "workflow_authored",
        );
    } else if workflow_fallback_allowed {
        let mut fallback_response = maybe_tooling_failure_fallback(
            message,
            &initial_draft_response,
            &latest_assistant_text,
        )
        .unwrap_or_default();
        tooling_fallback_used = !fallback_response.is_empty();
        if fallback_response.is_empty()
            && !response_requires_visible_repair(&initial_draft_response)
        {
            fallback_response = initial_draft_response.clone();
        }
        if fallback_response.is_empty()
            && message_requests_comparative_answer(message)
            && (response_is_no_findings_placeholder(&initial_draft_response)
                || response_tools_failure_reason_for_user(&response_tools, 4).is_empty())
        {
            comparative_fallback_used = true;
            fallback_response = comparative_no_findings_fallback(message);
        }
        if fallback_response.is_empty() && memory_recall_requested(message) {
            fallback_response = build_memory_recall_response(&state, &messages, message);
        }
        if fallback_response.is_empty() && !response_tools.is_empty() {
            fallback_response = ensure_tool_turn_response_text(&initial_draft_response, &response_tools);
        }
        if fallback_response.is_empty() && response_tools.is_empty() && !inline_tools_allowed {
            fallback_response =
                "I can answer directly without tool calls. Ask your question naturally and I’ll respond conversationally unless you explicitly request a tool run.".to_string();
        }
        if fallback_response.is_empty() {
            fallback_response =
                "I hit a response-synthesis failure after collecting this turn. Please retry and I’ll explain what worked or failed directly.".to_string();
        }
        workflow_system_fallback_used = true;
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            "workflow_system_fallback",
            200,
        );
        let (contract_finalized, contract_report, contract_outcome) =
            enforce_user_facing_finalization_contract(fallback_response, &response_tools);
        finalized_response = contract_finalized;
        tool_completion = contract_report;
        finalization_outcome =
            merge_response_outcomes(&finalization_outcome, &contract_outcome, 200);
    } else {
        workflow_system_fallback_used = true;
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            "workflow_unexpected_state",
            200,
        );
        let (contract_finalized, contract_report, contract_outcome) =
            enforce_user_facing_finalization_contract(
                "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly."
                    .to_string(),
                &response_tools,
            );
        finalized_response = contract_finalized;
        tool_completion = contract_report;
        finalization_outcome =
            merge_response_outcomes(&finalization_outcome, &contract_outcome, 200);
    }
    let (repaired_response, repair_outcome, repair_tooling_used, repair_comparative_used) =
        repair_visible_response_after_workflow(
            message,
            &finalized_response,
            &initial_draft_response,
            &latest_assistant_text,
            &response_tools,
            inline_tools_allowed,
            memory_fallback.as_deref(),
        );
    if repair_outcome != "unchanged" {
        visible_response_repaired = true;
        tooling_fallback_used |= repair_tooling_used;
        comparative_fallback_used |= repair_comparative_used;
        let (contract_finalized, contract_report, contract_outcome) =
            enforce_user_facing_finalization_contract(repaired_response, &response_tools);
        finalized_response = contract_finalized;
        tool_completion = contract_report;
        finalization_outcome = merge_response_outcomes(&finalization_outcome, &repair_outcome, 200);
        finalization_outcome =
            merge_response_outcomes(&finalization_outcome, &contract_outcome, 200);
    }
    tool_completion = enrich_tool_completion_receipt(tool_completion, &response_tools);
    response_text = finalized_response;
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
        "retry_attempted": false,
        "retry_used": false,
        "tool_synthesis_retry_used": false,
        "synthesis_retry_used": false,
        "tooling_fallback_used": tooling_fallback_used,
        "comparative_fallback_used": comparative_fallback_used,
        "workflow_system_fallback_used": workflow_system_fallback_used,
        "visible_response_repaired": visible_response_repaired
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
