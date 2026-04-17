fn web_tool_name_for_invariant(name: &str) -> bool {
    matches!(
        normalize_tool_name(name).as_str(),
        "web_search"
            | "search_web"
            | "search"
            | "web_query"
            | "batch_query"
            | "web_fetch"
            | "browse"
            | "web_conduit_fetch"
            | "web_tooling_health_probe"
    )
}

fn response_tools_include_web_attempt(response_tools: &[Value]) -> bool {
    response_tools.iter().any(|row| {
        let name = clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 80);
        web_tool_name_for_invariant(&name)
    })
}

fn response_tools_web_blocked(response_tools: &[Value]) -> bool {
    response_tools.iter().any(|row| {
        let name = clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 80);
        if !web_tool_name_for_invariant(&name) {
            return false;
        }
        let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 80)
            .to_ascii_lowercase();
        let error = clean_text(row.get("error").and_then(Value::as_str).unwrap_or(""), 160)
            .to_ascii_lowercase();
        row.get("blocked").and_then(Value::as_bool).unwrap_or(false)
            || matches!(status.as_str(), "blocked" | "policy_denied")
            || error.contains("nexus_delivery_denied")
            || error.contains("permission_denied")
    })
}

fn response_tools_web_low_signal(response_tools: &[Value]) -> bool {
    response_tools.iter().any(|row| {
        let name = clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 80);
        if !web_tool_name_for_invariant(&name) {
            return false;
        }
        let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 80)
            .to_ascii_lowercase();
        let result = clean_text(row.get("result").and_then(Value::as_str).unwrap_or(""), 2000);
        matches!(status.as_str(), "low_signal" | "no_results")
            || response_looks_like_tool_ack_without_findings(&result)
            || response_is_no_findings_placeholder(&result)
            || response_looks_like_unsynthesized_web_snippet_dump(&result)
            || response_looks_like_raw_web_artifact_dump(&result)
    })
}

fn web_failure_code_from_response_tools(response_tools: &[Value]) -> String {
    for row in response_tools {
        let name = clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 80);
        if !web_tool_name_for_invariant(&name) {
            continue;
        }
        let error = clean_text(row.get("error").and_then(Value::as_str).unwrap_or(""), 160)
            .to_ascii_lowercase();
        if error.is_empty() {
            let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 80)
                .to_ascii_lowercase();
            if matches!(status.as_str(), "blocked" | "policy_denied") {
                return "web_tool_policy_blocked".to_string();
            }
            if status == "timeout" {
                return "web_tool_timeout".to_string();
            }
            if matches!(status.as_str(), "low_signal" | "no_results") {
                return "web_tool_low_signal".to_string();
            }
            continue;
        }
        if error.contains("nexus_delivery_denied") || error.contains("permission_denied") {
            return "web_tool_policy_blocked".to_string();
        }
        if error.contains("invalid_response_attempt") {
            return "web_tool_invalid_response".to_string();
        }
        if error.contains("timeout") {
            return "web_tool_timeout".to_string();
        }
        if error.contains("401") {
            return "web_tool_http_401".to_string();
        }
        if error.contains("403") {
            return "web_tool_http_403".to_string();
        }
        if error.contains("404") {
            return "web_tool_http_404".to_string();
        }
        if error.contains("422") {
            return "web_tool_http_422".to_string();
        }
        if error.contains("429") {
            return "web_tool_http_429".to_string();
        }
        if error.contains("500")
            || error.contains("502")
            || error.contains("503")
            || error.contains("504")
        {
            return "web_tool_http_5xx".to_string();
        }
        return "web_tool_error".to_string();
    }
    String::new()
}

fn classify_web_turn_state(
    requires_live_web: bool,
    tool_attempted: bool,
    blocked: bool,
    low_signal: bool,
) -> String {
    if !requires_live_web {
        return "not_requested".to_string();
    }
    if !tool_attempted {
        return "parse_failed".to_string();
    }
    if blocked {
        return "policy_blocked".to_string();
    }
    if low_signal {
        return "provider_low_signal".to_string();
    }
    "healthy".to_string()
}

fn response_tools_any_blocked(response_tools: &[Value]) -> bool {
    response_tools.iter().any(|row| {
        let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 80)
            .to_ascii_lowercase();
        let error = clean_text(row.get("error").and_then(Value::as_str).unwrap_or(""), 160)
            .to_ascii_lowercase();
        row.get("blocked").and_then(Value::as_bool).unwrap_or(false)
            || matches!(status.as_str(), "blocked" | "policy_denied")
            || error.contains("nexus_delivery_denied")
            || error.contains("permission_denied")
    })
}

fn response_tools_any_low_signal(response_tools: &[Value]) -> bool {
    response_tools.iter().any(|row| {
        let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 80)
            .to_ascii_lowercase();
        let result = clean_text(row.get("result").and_then(Value::as_str).unwrap_or(""), 2_000);
        matches!(status.as_str(), "low_signal" | "no_results" | "partial_no_results")
            || response_looks_like_tool_ack_without_findings(&result)
            || response_is_no_findings_placeholder(&result)
    })
}

fn tool_failure_code_from_response_tools(response_tools: &[Value]) -> String {
    for row in response_tools {
        let normalized_name =
            normalize_tool_name(row.get("name").and_then(Value::as_str).unwrap_or("tool"));
        if normalized_name.eq_ignore_ascii_case("thought_process") {
            continue;
        }
        let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 80)
            .to_ascii_lowercase();
        let error = clean_text(row.get("error").and_then(Value::as_str).unwrap_or(""), 160)
            .to_ascii_lowercase();
        let blocked = row.get("blocked").and_then(Value::as_bool).unwrap_or(false)
            || matches!(status.as_str(), "blocked" | "policy_denied");
        let token = {
            let cleaned = clean_text(&normalized_name, 48);
            if cleaned.is_empty() {
                "tool".to_string()
            } else {
                cleaned
            }
        };
        if blocked {
            return format!("{token}_policy_blocked");
        }
        if status == "timeout" || error.contains("timeout") {
            return format!("{token}_timeout");
        }
        if matches!(status.as_str(), "low_signal" | "no_results" | "partial_no_results") {
            return format!("{token}_low_signal");
        }
        if error.contains("invalid_response_attempt") || error.contains("invalid_response") {
            return format!("{token}_invalid_response");
        }
        if error.contains("401") {
            return format!("{token}_http_401");
        }
        if error.contains("403") {
            return format!("{token}_http_403");
        }
        if error.contains("404") {
            return format!("{token}_http_404");
        }
        if error.contains("422") {
            return format!("{token}_http_422");
        }
        if error.contains("429") {
            return format!("{token}_http_429");
        }
        if error.contains("500")
            || error.contains("502")
            || error.contains("503")
            || error.contains("504")
        {
            return format!("{token}_http_5xx");
        }
        let errored = row.get("is_error").and_then(Value::as_bool).unwrap_or(false);
        if errored || matches!(status.as_str(), "error" | "failed" | "execution_error") {
            return format!("{token}_error");
        }
    }
    String::new()
}

fn process_summary_tool_rows(response_tools: &[Value], limit: usize) -> Value {
    let mut rows = Vec::<Value>::new();
    for row in response_tools.iter().take(limit.clamp(1, 8)) {
        let name = clean_text(row.get("name").and_then(Value::as_str).unwrap_or("tool"), 80);
        let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 80);
        let error = clean_text(row.get("error").and_then(Value::as_str).unwrap_or(""), 160);
        let result_excerpt = clean_text(
            &first_sentence(
                row.get("result").and_then(Value::as_str).unwrap_or(""),
                240,
            ),
            240,
        );
        rows.push(json!({
            "tool": if name.is_empty() { "tool" } else { &name },
            "status": status,
            "error": error,
            "is_error": row.get("is_error").and_then(Value::as_bool).unwrap_or(false),
            "blocked": row.get("blocked").and_then(Value::as_bool).unwrap_or(false),
            "result_excerpt": result_excerpt
        }));
    }
    Value::Array(rows)
}

fn build_turn_process_summary(
    message: &str,
    response_tools: &[Value],
    response_workflow: &Value,
    response_finalization: &Value,
) -> Value {
    json!({
        "contract": "turn_process_summary_v1",
        "generated_at": crate::now_iso(),
        "request_excerpt": clean_text(message, 240),
        "tool_gate": response_workflow
            .get("tool_gate")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "final_llm_status": clean_text(
            response_workflow
                .pointer("/final_llm_response/status")
                .and_then(Value::as_str)
                .unwrap_or(""),
            80
        ),
        "finalization_outcome": clean_text(
            response_finalization
                .get("outcome")
                .and_then(Value::as_str)
                .unwrap_or(""),
            220
        ),
        "final_answer_contract": response_finalization
            .get("final_answer_contract")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "quality_telemetry": response_workflow
            .get("quality_telemetry")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "tooling_invariant": response_finalization
            .get("tooling_invariant")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "web_invariant": response_finalization
            .get("web_invariant")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "tools": {
            "attempted_count": response_tools.len(),
            "attempts": process_summary_tool_rows(response_tools, 5)
        }
    })
}

fn response_message_is_actionable_for_next_steps(message: &str) -> bool {
    let lowered = clean_text(message, 1_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    lowered.contains("next step")
        || lowered.contains("what next")
        || lowered.contains("what should")
        || lowered.contains("how should")
        || lowered.contains("how can we")
        || lowered.contains("plan")
        || lowered.contains("improve")
        || lowered.contains("fix")
        || lowered.contains("implement")
        || lowered.contains("harden")
}

fn next_action_options_for_message(message: &str, response_tools: &[Value]) -> Vec<String> {
    let lowered = clean_text(message, 1_000).to_ascii_lowercase();
    if lowered.contains("web")
        || response_tools.iter().any(|row| {
            let name = clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 80);
            web_tool_name_for_invariant(&name)
        })
    {
        return vec![
            "retry with a narrower web query".to_string(),
            "target one trusted source URL".to_string(),
            "switch to local/workspace evidence only".to_string(),
        ];
    }
    if lowered.contains("implement") || lowered.contains("patch") || lowered.contains("fix") {
        return vec![
            "confirm exact acceptance criteria".to_string(),
            "apply the minimal code patch".to_string(),
            "run a targeted regression check".to_string(),
        ];
    }
    vec![
        "clarify the exact outcome you want".to_string(),
        "run one targeted tool call".to_string(),
        "return a concise answer from current context".to_string(),
    ]
}

fn append_next_actions_line_if_actionable(
    message: &str,
    response_text: &str,
    response_tools: &[Value],
) -> String {
    let cleaned = clean_chat_text(response_text, 32_000);
    if cleaned.is_empty() || !response_message_is_actionable_for_next_steps(message) {
        return cleaned;
    }
    if cleaned.to_ascii_lowercase().contains("next actions:") {
        return cleaned;
    }
    let options = next_action_options_for_message(message, response_tools);
    if options.is_empty() {
        return cleaned;
    }
    trim_text(
        &format!(
            "{}\n\nNext actions: 1) {} 2) {} 3) {}",
            cleaned,
            options.first().cloned().unwrap_or_default(),
            options.get(1).cloned().unwrap_or_default(),
            options.get(2).cloned().unwrap_or_default()
        ),
        32_000,
    )
}

fn response_requires_visible_repair(text: &str) -> bool {
    let cleaned = clean_chat_text(text, 32_000);
    cleaned.trim().is_empty()
        || response_is_no_findings_placeholder(&cleaned)
        || response_looks_like_tool_ack_without_findings(&cleaned)
        || response_is_deferred_execution_preamble(&cleaned)
        || response_is_deferred_retry_prompt(&cleaned)
        || workflow_response_requests_more_tooling(&cleaned)
        || response_contains_speculative_web_blocker_language(&cleaned)
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
    if !response_requires_visible_repair(&cleaned_initial_draft)
        && !response_contains_speculative_web_blocker_language(&cleaned_initial_draft)
    {
        return (
            cleaned_initial_draft,
            "repaired_with_initial_draft".to_string(),
            false,
            false,
        );
    }

    let cleaned_latest_assistant = clean_chat_text(latest_assistant_text, 32_000);
    if !response_requires_visible_repair(&cleaned_latest_assistant)
        && !response_contains_speculative_web_blocker_language(&cleaned_latest_assistant)
    {
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
        enforce_user_facing_finalization_contract(message, repaired_response, &[]);
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
    let process_summary =
        build_turn_process_summary(message, &[], &response_workflow, &response_finalization);
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
            "process_summary": process_summary.clone(),
            "turn_transaction": turn_transaction.clone(),
            "terminal_transcript": terminal_transcript.clone()
        }),
    );
    turn_receipt["process_summary"] = process_summary.clone();
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
            "process_summary": process_summary,
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
    let mut response_workflow = run_turn_workflow_final_response(
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
    let mut final_fallback_used = false;
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
        final_fallback_used = true;
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            "workflow_system_fallback",
            200,
        );
        let (contract_finalized, contract_report, contract_outcome) =
            enforce_user_facing_finalization_contract(message, fallback_response, &response_tools);
        finalized_response = contract_finalized;
        tool_completion = contract_report;
        finalization_outcome =
            merge_response_outcomes(&finalization_outcome, &contract_outcome, 200);
    } else {
        workflow_system_fallback_used = true;
        final_fallback_used = true;
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            "workflow_unexpected_state",
            200,
        );
        let (contract_finalized, contract_report, contract_outcome) =
            enforce_user_facing_finalization_contract(
                message,
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
        final_fallback_used = true;
        tooling_fallback_used |= repair_tooling_used;
        comparative_fallback_used |= repair_comparative_used;
        let (contract_finalized, contract_report, contract_outcome) =
            enforce_user_facing_finalization_contract(message, repaired_response, &response_tools);
        finalized_response = contract_finalized;
        tool_completion = contract_report;
        finalization_outcome = merge_response_outcomes(&finalization_outcome, &repair_outcome, 200);
        finalization_outcome =
            merge_response_outcomes(&finalization_outcome, &contract_outcome, 200);
    }
    tool_completion = enrich_tool_completion_receipt(tool_completion, &response_tools);
    response_text = finalized_response;
    let web_tool_attempted = response_tools_include_web_attempt(&response_tools);
    let web_tool_blocked = response_tools_web_blocked(&response_tools);
    let web_tool_low_signal = response_tools_web_low_signal(&response_tools);
    let web_turn_classification = classify_web_turn_state(
        web_intent_detected,
        web_tool_attempted,
        web_tool_blocked,
        web_tool_low_signal,
    );
    let mut web_failure_code = web_failure_code_from_response_tools(&response_tools);
    if web_intent_detected && !web_tool_attempted {
        web_failure_code = "web_route_parse_failed".to_string();
    } else if web_failure_code.is_empty() && web_tool_low_signal {
        web_failure_code = "web_tool_low_signal".to_string();
    }
    let tooling_attempted = !response_tools.is_empty();
    let tooling_blocked = response_tools_any_blocked(&response_tools);
    let tooling_low_signal = response_tools_any_low_signal(&response_tools);
    let tooling_failure_code = tool_failure_code_from_response_tools(&response_tools);
    let tooling_turn_classification = if !tooling_attempted {
        "not_attempted".to_string()
    } else if tooling_blocked {
        "policy_blocked".to_string()
    } else if tooling_low_signal {
        "low_signal".to_string()
    } else if !tooling_failure_code.is_empty() {
        "failed".to_string()
    } else {
        "healthy".to_string()
    };
    let mut tooling_invariant_repair_used = false;
    let mut web_invariant_repair_used = false;
    if web_intent_detected && !web_tool_attempted {
        response_text = format!(
            "I detected a live web request, but no web tool lane executed in this turn. web_status: parse_failed. error_code: {}. Retry with `tool::web_search:::<query>` or `tool::web_tooling_health_probe`.",
            web_failure_code
        );
        web_invariant_repair_used = true;
        final_fallback_used = true;
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            "web_invariant_missing_tool_attempt",
            200,
        );
    } else if web_tool_attempted
        && (web_tool_blocked || web_tool_low_signal || !web_failure_code.is_empty())
        && !web_failure_code.is_empty()
        && !response_text
            .to_ascii_lowercase()
            .contains(&web_failure_code.to_ascii_lowercase())
    {
        response_text = trim_text(
            &format!(
                "{}\n\nweb_status: {}\nerror_code: {}",
                response_text, web_turn_classification, web_failure_code
            ),
            32_000,
        );
        web_invariant_repair_used = true;
        final_fallback_used = true;
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            "web_failure_code_appended",
            200,
        );
    }
    if tooling_attempted
        && !tooling_failure_code.is_empty()
        && !response_text
            .to_ascii_lowercase()
            .contains(&tooling_failure_code.to_ascii_lowercase())
    {
        response_text = trim_text(
            &format!(
                "{}\n\ntool_status: {}\nerror_code: {}",
                response_text, tooling_turn_classification, tooling_failure_code
            ),
            32_000,
        );
        tooling_invariant_repair_used = true;
        final_fallback_used = true;
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            "tooling_failure_code_appended",
            200,
        );
    }
    let final_contract_violation = response_text.trim().is_empty()
        || response_is_no_findings_placeholder(&response_text)
        || response_looks_like_tool_ack_without_findings(&response_text)
        || response_is_deferred_execution_preamble(&response_text)
        || response_is_deferred_retry_prompt(&response_text)
        || workflow_response_requests_more_tooling(&response_text);
    if final_contract_violation {
        let mut deterministic_fallback = clean_text(
            &response_tools_failure_reason_for_user(&response_tools, 4),
            4_000,
        );
        if deterministic_fallback.is_empty() {
            deterministic_fallback =
                clean_text(&response_tools_summary_for_user(&response_tools, 4), 4_000);
        }
        if deterministic_fallback.is_empty() && web_intent_detected {
            let stable_error = if web_failure_code.is_empty() {
                "web_tool_error".to_string()
            } else {
                web_failure_code.clone()
            };
            deterministic_fallback = format!(
                "Web retrieval did not produce a usable final answer in this turn. web_status: {}. error_code: {}. Next step: retry with a narrower query or provide one trusted source URL.",
                web_turn_classification, stable_error
            );
        }
        if deterministic_fallback.is_empty() && tooling_attempted && !tooling_failure_code.is_empty()
        {
            deterministic_fallback = format!(
                "Tool execution did not produce a usable final answer in this turn. tool_status: {}. error_code: {}. Next step: run one targeted tool call with explicit scope.",
                tooling_turn_classification, tooling_failure_code
            );
        }
        if deterministic_fallback.is_empty() && response_tools.is_empty() && !inline_tools_allowed {
            deterministic_fallback =
                "I can answer directly without tool calls. Ask your question naturally and I’ll respond conversationally unless you explicitly request a tool run.".to_string();
        }
        if deterministic_fallback.is_empty() {
            deterministic_fallback = "I completed the workflow, but synthesis could not produce a valid final response in this turn. Please retry and I’ll rerun the chain with explicit failure details.".to_string();
        }
        response_text = clean_chat_text(&deterministic_fallback, 32_000);
        final_fallback_used = true;
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            "deterministic_final_fallback_enforced",
            200,
        );
    }
    response_text = append_next_actions_line_if_actionable(message, &response_text, &response_tools);
    let tool_gate_should_call_tools = response_workflow
        .pointer("/tool_gate/should_call_tools")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let direct_answer_rate =
        response_workflow_quality_rate(&response_workflow, "direct_answer_rate");
    let retry_rate = response_workflow_quality_rate(&response_workflow, "retry_rate");
    let off_topic_reject_rate =
        response_workflow_quality_rate(&response_workflow, "off_topic_reject_rate");
    let tool_overcall_rate = if !tool_gate_should_call_tools && tooling_attempted {
        1.0
    } else {
        0.0
    };
    response_workflow["quality_telemetry"]["final_fallback_used"] = Value::Bool(final_fallback_used);
    let off_topic_reject = response_workflow_quality_count(&response_workflow, "off_topic_reject");
    let deferred_reply_reject =
        response_workflow_quality_count(&response_workflow, "deferred_reply_reject");
    let alignment_reject = response_workflow_quality_count(&response_workflow, "alignment_reject");
    let prompt_echo_reject =
        response_workflow_quality_count(&response_workflow, "prompt_echo_reject");
    let unsourced_claim_reject =
        response_workflow_quality_count(&response_workflow, "unsourced_claim_reject");
    let direct_answer_reject =
        response_workflow_quality_count(&response_workflow, "direct_answer_reject");
    let meta_control_tool_block =
        response_workflow_quality_flag(&response_workflow, "meta_control_tool_block");
    let final_ack_only = response_looks_like_tool_ack_without_findings(&response_text);
    let response_quality_telemetry = json!({
        "off_topic_reject": off_topic_reject,
        "deferred_reply_reject": deferred_reply_reject,
        "alignment_reject": alignment_reject,
        "prompt_echo_reject": prompt_echo_reject,
        "unsourced_claim_reject": unsourced_claim_reject,
        "direct_answer_reject": direct_answer_reject,
        "meta_control_tool_block": meta_control_tool_block,
        "final_fallback_used": final_fallback_used,
        "tooling_contract_repair_used": tooling_invariant_repair_used,
        "tooling_failure_code_present": !tooling_failure_code.is_empty(),
        "direct_answer_rate": direct_answer_rate,
        "retry_rate": retry_rate,
        "tool_overcall_rate": tool_overcall_rate,
        "off_topic_reject_rate": off_topic_reject_rate
    });
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
        "final_answer_contract": tool_completion
            .get("final_answer_contract")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "retry_attempted": false,
        "retry_used": false,
        "tool_synthesis_retry_used": false,
        "synthesis_retry_used": false,
        "tooling_fallback_used": tooling_fallback_used,
        "comparative_fallback_used": comparative_fallback_used,
        "workflow_system_fallback_used": workflow_system_fallback_used,
        "visible_response_repaired": visible_response_repaired,
        "response_quality_telemetry": response_quality_telemetry.clone(),
        "tooling_invariant": {
            "tool_attempted": tooling_attempted,
            "tool_blocked": tooling_blocked,
            "low_signal": tooling_low_signal,
            "classification": tooling_turn_classification,
            "failure_code": tooling_failure_code,
            "invariant_repair_used": tooling_invariant_repair_used
        },
        "web_invariant": {
            "requires_live_web": web_intent_detected,
            "intent_source": web_intent_source,
            "intent_confidence": web_intent_confidence,
            "selected_route": web_intent_route.clone(),
            "tool_attempted": web_tool_attempted,
            "tool_blocked": web_tool_blocked,
            "low_signal": web_tool_low_signal,
            "classification": web_turn_classification,
            "failure_code": web_failure_code,
            "forced_fallback_attempted": web_forced_fallback_attempted,
            "invariant_repair_used": web_invariant_repair_used
        }
    });
    let process_summary =
        build_turn_process_summary(message, &response_tools, &response_workflow, &response_finalization);
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
            "process_summary": process_summary.clone(),
            "turn_transaction": turn_transaction.clone(),
            "terminal_transcript": terminal_transcript.clone()
        }),
    );
    turn_receipt["process_summary"] = process_summary.clone();
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
    payload["process_summary"] = process_summary;
    payload["response_quality_telemetry"] = response_quality_telemetry;
    payload["web_intent"] = json!({
        "detected": web_intent_detected,
        "source": web_intent_source,
        "confidence": web_intent_confidence,
        "selected_route": web_intent_route
    });
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
