struct PreparedMessageRouteContext {
    provider: String, model: String, auto_route: Option<Value>,
    requested_provider: String, requested_model: String, virtual_key_id: String, virtual_key_gate: Value,
    state: Value, messages: Vec<Value>, active_messages: Vec<Value>,
    context_pool_limit_tokens: i64, context_pool_tokens: i64, pooled_messages_len: usize, sessions_total: usize,
    fallback_window: i64, memory_kv_entries: usize, active_context_target_tokens: i64, active_context_min_recent: usize,
    include_all_sessions_context: bool, context_active_tokens: i64, context_ratio: f64, context_pressure: String,
    pre_generation_pruned: bool, recent_floor_enforced: bool, recent_floor_injected: usize, history_trim_confirmed: bool,
    emergency_compact: Value, workspace_hints: Value, latent_tool_candidates: Value, inline_tools_allowed: bool,
    system_prompt: String,
}
fn latest_assistant_process_summary(active_messages: &[Value]) -> Option<Value> {
    active_messages.iter().rev().find_map(|row| {
        let role = clean_text(row.get("role").and_then(Value::as_str).unwrap_or(""), 24);
        if !role.eq_ignore_ascii_case("assistant") {
            return None;
        }
        let summary = row.get("process_summary")?;
        if !summary.is_object() {
            return None;
        }
        Some(summary.clone())
    })
}
fn process_summary_prompt_context(active_messages: &[Value]) -> String {
    let Some(summary) = latest_assistant_process_summary(active_messages) else {
        return String::new();
    };
    let compact_summary =
        clean_text(&serde_json::to_string(&summary).unwrap_or_else(|_| "{}".to_string()), 1_600);
    if compact_summary.is_empty() || compact_summary == "{}" {
        return String::new();
    }
    clean_text(
        &format!(
            "Previous-turn process summary (short-term debug memory, single-turn only): {compact_summary}\nUse this as continuity context for diagnostics, but prioritize the current user request and current-turn receipts."
        ),
        2_000,
    )
}

fn reply_scope_messages_from_request(request: &Value) -> Vec<Value> {
    if !request
        .get("reply_scope_enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Vec::new();
    }
    let Some(rows) = request.get("reply_scope_messages").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut out = Vec::<Value>::new();
    for row in rows.iter().take(32) {
        let mut role = clean_text(
            row.get("role").and_then(Value::as_str).unwrap_or("assistant"),
            24,
        )
        .to_ascii_lowercase();
        if role == "agent" {
            role = "assistant".to_string();
        }
        if role != "assistant" && role != "user" && role != "system" {
            role = "assistant".to_string();
        }
        let text = clean_chat_text(row.get("text").and_then(Value::as_str).unwrap_or(""), 64_000);
        if text.is_empty() {
            continue;
        }
        out.push(json!({"role": role, "text": text, "ts": row.get("ts").cloned().unwrap_or(Value::Null)}));
    }
    out
}

fn message_timestamp_millis_for_retry(row: &Value) -> i64 {
    if let Some(ms) = row.get("ts").and_then(Value::as_i64) {
        return ms;
    }
    if let Some(raw) = row.get("ts").and_then(Value::as_str) {
        let cleaned = clean_text(raw, 120);
        if let Ok(ms) = cleaned.parse::<i64>() { return ms; }
        if let Some(parsed) = parse_rfc3339_utc(&cleaned) { return parsed.timestamp_millis(); }
    }
    0
}

fn remove_retry_drop_message_from_state(state: &mut Value, request: &Value) -> Option<Value> {
    let target_id = clean_text(request.get("retry_drop_message_id").and_then(Value::as_str).unwrap_or(""), 180);
    let target_ts = request.get("retry_drop_message_ts").and_then(Value::as_i64).unwrap_or(0);
    let target_text = clean_chat_text(request.get("retry_drop_message_text").and_then(Value::as_str).unwrap_or(""), 64_000);
    if target_id.is_empty() && target_ts <= 0 && target_text.is_empty() {
        return None;
    }
    let active_id = clean_text(state.get("active_session_id").and_then(Value::as_str).unwrap_or("default"), 120);
    let mut removed: Option<Value> = None;
    if let Some(sessions) = state.get_mut("sessions").and_then(Value::as_array_mut) {
        for session in sessions.iter_mut() {
            let sid = clean_text(session.get("session_id").and_then(Value::as_str).unwrap_or(""), 120);
            if sid != active_id {
                continue;
            }
            let Some(messages) = session.get_mut("messages").and_then(Value::as_array_mut) else {
                break;
            };
            let mut remove_idx: Option<usize> = None;
            for idx in (0..messages.len()).rev() {
                let probe = &messages[idx];
                let role = clean_text(probe.get("role").and_then(Value::as_str).unwrap_or(""), 24)
                    .to_ascii_lowercase();
                if role != "assistant" && role != "agent" {
                    continue;
                }
                let probe_id = clean_text(probe.get("id").and_then(Value::as_str).unwrap_or(""), 180);
                let probe_ts = message_timestamp_millis_for_retry(probe);
                let probe_text = message_text(probe);
                let id_match = !target_id.is_empty() && !probe_id.is_empty() && probe_id == target_id;
                let ts_match = target_ts > 0 && probe_ts > 0 && (probe_ts - target_ts).unsigned_abs() <= 2_500;
                let text_match = !target_text.is_empty() && !probe_text.is_empty() && probe_text == target_text;
                if id_match || ts_match || text_match {
                    remove_idx = Some(idx);
                    break;
                }
            }
            if remove_idx.is_none() {
                remove_idx = (0..messages.len()).rev().find(|idx| {
                    let role = clean_text(messages[*idx].get("role").and_then(Value::as_str).unwrap_or(""), 24).to_ascii_lowercase();
                    role == "assistant" || role == "agent"
                });
            }
            if let Some(idx) = remove_idx {
                removed = Some(messages.remove(idx));
                session["updated_at"] = Value::String(crate::now_iso());
            }
            break;
        }
    }
    removed
}

fn prepare_message_route_context(
    root: &Path,
    snapshot: &Value,
    row: &Value,
    request: &Value,
    message: &str,
    route_request: &Value,
    requested_provider: &str,
    requested_model: &str,
    virtual_key_id: &str,
    agent_id: &str,
    workspace_hints: &Value,
    latent_tool_candidates: &Value,
) -> Result<PreparedMessageRouteContext, CompatApiResponse> {
    let (provider, model, auto_route) = crate::dashboard_model_catalog::resolve_model_selection(
        root,
        snapshot,
        requested_provider,
        requested_model,
        route_request,
    );
    let mut provider = provider;
    let mut model = model;
    let mut virtual_key_gate = Value::Null;
    if !virtual_key_id.is_empty() {
        let gate =
            crate::dashboard_provider_runtime::reserve_virtual_key_slot(root, virtual_key_id);
        if !gate.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            let error_code = clean_text(
                gate.get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("virtual_key_denied"),
                80,
            );
            let status = if error_code == "virtual_key_budget_exceeded" {
                402
            } else if error_code == "virtual_key_rate_limited" {
                429
            } else {
                400
            };
            return Err(CompatApiResponse {
                status,
                payload: json!({
                    "ok": false,
                    "agent_id": agent_id,
                    "error": error_code,
                    "virtual_key_id": virtual_key_id,
                    "virtual_key": gate
                }),
            });
        }
        let route_hint =
            crate::dashboard_provider_runtime::resolve_virtual_key_route(root, virtual_key_id);
        let key_provider = clean_text(
            route_hint
                .get("provider")
                .or_else(|| gate.get("provider"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        );
        let key_model = clean_text(
            route_hint
                .get("model")
                .or_else(|| gate.get("model"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            240,
        );
        if !key_provider.is_empty() && !key_provider.eq_ignore_ascii_case("auto") {
            provider = key_provider;
        }
        if !key_model.is_empty() && !key_model.eq_ignore_ascii_case("auto") {
            model = key_model;
        }
        virtual_key_gate = gate;
    }
    let mut state = load_session_state(root, agent_id);
    let retry_drop_removed = remove_retry_drop_message_from_state(&mut state, request);
    if retry_drop_removed.is_some() {
        save_session_state(root, agent_id, &state);
    }
    let sessions_total = state
        .get("sessions")
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    let row_context_window = row
        .get("context_window_tokens")
        .or_else(|| row.get("context_window"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let fallback_window = if row_context_window > 0 {
        row_context_window
    } else {
        128_000
    };
    let active_context_target_tokens = request
        .get("active_context_target_tokens")
        .or_else(|| request.get("target_context_window"))
        .and_then(Value::as_i64)
        .unwrap_or_else(|| ((fallback_window as f64) * 0.68).round() as i64)
        .clamp(4_096, 512_000);
    let active_context_min_recent = request
        .get("active_context_min_recent_messages")
        .or_else(|| request.get("min_recent_messages"))
        .and_then(Value::as_u64)
        .unwrap_or(ACTIVE_CONTEXT_MIN_RECENT_FLOOR as u64)
        .clamp(ACTIVE_CONTEXT_MIN_RECENT_FLOOR as u64, 256)
        as usize;
    let include_all_sessions_context = request
        .get("include_all_sessions_context")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let row_system_context_limit = row
        .get("system_context_tokens")
        .or_else(|| row.get("context_pool_limit_tokens"))
        .and_then(Value::as_i64)
        .unwrap_or(1_000_000);
    let row_auto_compact_threshold_ratio = row
        .get("auto_compact_threshold_ratio")
        .and_then(Value::as_f64)
        .unwrap_or(0.95);
    let row_auto_compact_target_ratio = row
        .get("auto_compact_target_ratio")
        .and_then(Value::as_f64)
        .unwrap_or(0.72);
    let context_pool_limit_tokens = request
        .get("context_pool_limit_tokens")
        .and_then(Value::as_i64)
        .unwrap_or(row_system_context_limit)
        .clamp(32_000, 2_000_000);
    let auto_compact_threshold_ratio = request
        .get("auto_compact_threshold_ratio")
        .and_then(Value::as_f64)
        .unwrap_or(row_auto_compact_threshold_ratio)
        .clamp(0.75, 0.99);
    let auto_compact_target_ratio = request
        .get("auto_compact_target_ratio")
        .and_then(Value::as_f64)
        .unwrap_or(row_auto_compact_target_ratio)
        .clamp(0.40, 0.90);
    // Conversation history is authoritative and must not be rewritten as a side effect
    // of normal message execution. Manual compaction remains available through explicit
    // compaction routes only.
    let history_trim_confirmed = false;
    let persist_system_prune = false;
    let persist_auto_compact = false;
    let mut messages = context_source_messages(&state, include_all_sessions_context);
    let all_session_history_count = context_source_messages(&state, true).len();
    let mut pooled_messages = trim_context_pool(&messages, context_pool_limit_tokens);
    let pre_generation_pruned = pooled_messages.len() != messages.len();
    if pre_generation_pruned && persist_system_prune {
        set_active_session_messages(&mut state, &pooled_messages);
        save_session_state(root, agent_id, &state);
        state = load_session_state(root, agent_id);
        messages = context_source_messages(&state, include_all_sessions_context);
        pooled_messages = trim_context_pool(&messages, context_pool_limit_tokens);
    }
    let (pooled_messages_with_floor, recent_floor_injected) =
        enforce_recent_context_floor(&messages, &pooled_messages, active_context_min_recent);
    let recent_floor_enforced = recent_floor_injected > 0;
    pooled_messages = pooled_messages_with_floor;
    if all_session_history_count > 0 && messages.is_empty() {
        return Err(CompatApiResponse {
            status: 503,
            payload: crate::dashboard_tool_turn_loop::hydration_failed_payload(agent_id),
        });
    }
    let mut active_messages = select_active_context_window(
        &pooled_messages,
        active_context_target_tokens,
        active_context_min_recent,
    );
    let mut context_pool_tokens = total_message_tokens(&pooled_messages);
    let mut context_active_tokens = total_message_tokens(&active_messages);
    let mut context_ratio = if fallback_window > 0 {
        (context_active_tokens as f64 / fallback_window as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let mut context_pressure = context_pressure_label(context_ratio).to_string();
    let mut emergency_compact = json!({
        "triggered": false,
        "threshold_ratio": auto_compact_threshold_ratio,
        "target_ratio": auto_compact_target_ratio,
        "removed_messages": 0
    });
    if context_ratio >= auto_compact_threshold_ratio && fallback_window > 0 {
        let emergency_target_tokens =
            ((fallback_window as f64) * auto_compact_target_ratio).round() as i64;
        let emergency_min_recent = request
            .get("emergency_min_recent_messages")
            .or_else(|| request.get("min_recent_messages"))
            .and_then(Value::as_u64)
            .unwrap_or(active_context_min_recent as u64)
            .clamp(ACTIVE_CONTEXT_MIN_RECENT_FLOOR as u64, 256)
            as usize;
        let emergency_messages = select_active_context_window(
            &pooled_messages,
            emergency_target_tokens,
            emergency_min_recent,
        );
        let emergency_tokens = total_message_tokens(&emergency_messages);
        let removed_messages = pooled_messages
            .len()
            .saturating_sub(emergency_messages.len()) as u64;
        emergency_compact = json!({
            "triggered": true,
            "threshold_ratio": auto_compact_threshold_ratio,
            "target_ratio": auto_compact_target_ratio,
            "removed_messages": removed_messages,
            "before_tokens": context_active_tokens,
            "after_tokens": emergency_tokens,
            "persisted_to_history": false
        });
        if removed_messages > 0 {
            active_messages = emergency_messages;
            context_pool_tokens = total_message_tokens(&pooled_messages);
            context_active_tokens = emergency_tokens;
            context_ratio = if fallback_window > 0 {
                (context_active_tokens as f64 / fallback_window as f64).clamp(0.0, 1.0)
            } else {
                0.0
            };
            context_pressure = context_pressure_label(context_ratio).to_string();
            if persist_auto_compact {
                let compact_request = json!({
                    "target_context_window": fallback_window,
                    "target_ratio": auto_compact_target_ratio,
                    "min_recent_messages": emergency_min_recent,
                    "max_messages": request
                        .get("max_messages")
                        .and_then(Value::as_u64)
                        .unwrap_or(220)
                        .clamp(20, 800)
                });
                let compact_result = compact_active_session(root, agent_id, &compact_request);
                emergency_compact["persisted_to_history"] = json!(true);
                emergency_compact["persist_result"] = compact_result;
            }
        }
    }
    let reply_scope_messages = reply_scope_messages_from_request(request);
    if !reply_scope_messages.is_empty() {
        active_messages = reply_scope_messages;
        context_active_tokens = total_message_tokens(&active_messages);
        context_ratio = if fallback_window > 0 {
            (context_active_tokens as f64 / fallback_window as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };
        context_pressure = context_pressure_label(context_ratio).to_string();
    }
    let memory_kv_entries = memory_kv_pairs_from_state(&state).len();
    let memory_prompt_context = memory_kv_prompt_context(&state, 24);
    let instinct_prompt_context = agent_instinct_prompt_context(root, 6_000);
    let plugin_prompt_context =
        dashboard_skills_marketplace::skills_prompt_context(root, 12, 4_000);
    let passive_memory_context = passive_attention_context_for_message(root, agent_id, message, 6);
    let keyframe_context = context_keyframes_prompt_context(&state, 8, 2_400);
    let overflow_keyframes_context =
        historical_context_keyframes_prompt_context(&messages, &active_messages, 10, 2_400);
    let relevant_recall_context =
        historical_relevant_recall_prompt_context(&messages, &active_messages, message, 8, 2_800);
    let identity_hydration_prompt = agent_identity_hydration_prompt(row);
    let custom_system_prompt = clean_text(
        row.get("system_prompt")
            .and_then(Value::as_str)
            .unwrap_or(""),
        12_000,
    );
    let tool_gate = workflow_turn_tool_decision_tree(message);
    let gate_should_call_tools = tool_gate
        .get("should_call_tools")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let inline_tools_allowed =
        inline_tool_calls_allowed_for_user_message(message) && gate_should_call_tools;
    let mut prompt_parts = Vec::<String>::new();
    if !identity_hydration_prompt.is_empty() {
        prompt_parts.push(identity_hydration_prompt);
    }
    prompt_parts.push(AGENT_RUNTIME_SYSTEM_PROMPT.to_string());
    let workflow_prompt_context = workflow_library_prompt_context(
        message,
        latent_tool_candidates
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or(&[]),
    );
    if !workflow_prompt_context.is_empty() {
        prompt_parts.push(workflow_prompt_context);
    }
    let previous_process_summary_context = process_summary_prompt_context(&active_messages);
    if !previous_process_summary_context.is_empty() {
        prompt_parts.push(previous_process_summary_context);
    }
    if !inline_tools_allowed {
        prompt_parts.push("Direct-answer guard: default to natural conversational answers. Do not emit `<function=...>` tool calls unless the user explicitly requested web retrieval, file/terminal operations, memory operations, or agent management in this turn.".to_string());
    }
    if !instinct_prompt_context.is_empty() {
        prompt_parts.push(instinct_prompt_context);
    }
    if !plugin_prompt_context.is_empty() {
        prompt_parts.push(plugin_prompt_context);
    }
    if !passive_memory_context.is_empty() {
        prompt_parts.push(passive_memory_context);
    }
    if !keyframe_context.is_empty() {
        prompt_parts.push(keyframe_context);
    }
    if !overflow_keyframes_context.is_empty() {
        prompt_parts.push(overflow_keyframes_context);
    }
    if !relevant_recall_context.is_empty() {
        prompt_parts.push(relevant_recall_context);
    }
    if !custom_system_prompt.is_empty() {
        prompt_parts.push(custom_system_prompt);
    }
    if !memory_prompt_context.is_empty() {
        prompt_parts.push(memory_prompt_context);
    }
    let system_prompt = clean_text(&prompt_parts.join("\n\n"), 12_000);
    Ok(PreparedMessageRouteContext {
        provider,
        model,
        auto_route,
        requested_provider: requested_provider.to_string(),
        requested_model: requested_model.to_string(),
        virtual_key_id: virtual_key_id.to_string(),
        virtual_key_gate,
        state,
        messages,
        active_messages,
        context_pool_limit_tokens,
        context_pool_tokens,
        pooled_messages_len: pooled_messages.len(),
        sessions_total,
        fallback_window,
        memory_kv_entries,
        active_context_target_tokens,
        active_context_min_recent,
        include_all_sessions_context,
        context_active_tokens,
        context_ratio,
        context_pressure,
        pre_generation_pruned,
        recent_floor_enforced,
        recent_floor_injected,
        history_trim_confirmed,
        emergency_compact,
        workspace_hints: workspace_hints.clone(),
        latent_tool_candidates: latent_tool_candidates.clone(),
        inline_tools_allowed,
        system_prompt,
    })
}