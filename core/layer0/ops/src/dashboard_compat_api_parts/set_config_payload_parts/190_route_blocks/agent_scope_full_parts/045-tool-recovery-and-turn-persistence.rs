fn tool_turn_needs_llm_recovery(response_text: &str, response_tools: &[Value]) -> bool {
    if response_tools.is_empty() {
        return false;
    }
    let cleaned = clean_text(response_text, 32_000);
    if cleaned.is_empty()
        || response_is_no_findings_placeholder(&cleaned)
        || response_looks_like_tool_ack_without_findings(&cleaned)
    {
        return true;
    }
    response_tools.iter().any(|tool| {
        tool.get("is_error").and_then(Value::as_bool).unwrap_or(false)
            || tool.get("blocked").and_then(Value::as_bool).unwrap_or(false)
    })
}

fn tool_rows_for_llm_recovery(response_tools: &[Value], limit: usize) -> Value {
    let mut rows = Vec::<Value>::new();
    for tool in response_tools.iter().take(limit.clamp(1, 8)) {
        let name = normalize_tool_name(tool.get("name").and_then(Value::as_str).unwrap_or("tool"));
        let input = clean_text(tool.get("input").and_then(Value::as_str).unwrap_or(""), 800);
        let result = clean_text(tool.get("result").and_then(Value::as_str).unwrap_or(""), 2_000);
        let status = clean_text(tool.get("status").and_then(Value::as_str).unwrap_or(""), 120);
        rows.push(json!({
            "name": if name.is_empty() { "tool" } else { &name },
            "input": input,
            "status": status,
            "blocked": tool.get("blocked").and_then(Value::as_bool).unwrap_or(false),
            "is_error": tool.get("is_error").and_then(Value::as_bool).unwrap_or(false),
            "result": result
        }));
    }
    Value::Array(rows)
}

fn maybe_synthesize_tool_turn_response(
    root: &Path,
    provider: &str,
    model: &str,
    active_messages: &[Value],
    message: &str,
    response_tools: &[Value],
    draft_response: &str,
) -> Option<String> {
    if cfg!(test) {
        return None;
    }
    if !tool_turn_needs_llm_recovery(draft_response, response_tools) {
        return None;
    }
    let tool_rows = tool_rows_for_llm_recovery(response_tools, 6);
    let findings = clean_text(&response_tools_summary_for_user(response_tools, 4), 2_000);
    let failure_reason = clean_text(
        &response_tools_failure_reason_for_user(response_tools, 4),
        2_000,
    );
    let tool_rows_json = serde_json::to_string(&tool_rows).ok()?;
    let system_prompt = clean_text(
        &format!(
            "{}\n\nTool recovery guard: write the final assistant reply in natural language. If the recorded tool steps failed, were blocked, or returned low-signal output, say that plainly in your own words. Never pretend a failed tool succeeded. Do not emit raw tool telemetry or placeholder copy.",
            AGENT_RUNTIME_SYSTEM_PROMPT
        ),
        12_000,
    );
    let user_prompt = clean_text(
        &format!(
            "User request:\n{message}\n\nCurrent draft response:\n{}\n\nRecorded tool outcomes:\n{tool_rows_json}\n\nReadable findings summary:\n{}\n\nReadable failure summary:\n{}\n\nWrite the final assistant reply now.",
            if clean_text(draft_response, 2_000).is_empty() {
                "(empty)"
            } else {
                draft_response
            },
            if findings.is_empty() { "(none)" } else { &findings },
            if failure_reason.is_empty() { "(none)" } else { &failure_reason }
        ),
        20_000,
    );
    let retried = crate::dashboard_provider_runtime::invoke_chat(
        root,
        provider,
        model,
        &system_prompt,
        active_messages,
        &user_prompt,
    )
    .ok()?;
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
    if retried_text.is_empty() || response_is_unrelated_context_dump(message, &retried_text) {
        return None;
    }
    Some(retried_text)
}

fn ensure_tool_turn_response_text(response_text: &str, response_tools: &[Value]) -> String {
    let cleaned = clean_chat_text(response_text, 32_000);
    if !cleaned.is_empty() || response_tools.is_empty() {
        return cleaned;
    }
    let failure_reason = clean_text(
        &response_tools_failure_reason_for_user(response_tools, 4),
        4_000,
    );
    if !failure_reason.is_empty() {
        return failure_reason;
    }
    let findings = clean_text(&response_tools_summary_for_user(response_tools, 4), 4_000);
    if !findings.is_empty() {
        let partial = format!(
            "I completed tool steps, but only partial recorded output is available so far: {findings}"
        );
        return clean_chat_text(
            &partial,
            32_000,
        );
    }
    "I couldn't finish a readable answer from the tool steps, but the failure details were recorded in this turn."
        .to_string()
}

fn persist_last_assistant_turn_metadata(
    root: &Path,
    agent_id: &str,
    assistant_text: &str,
    metadata: &Value,
) -> Value {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let mut state = load_session_state(root, &id);
    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    let assistant = clean_chat_text(assistant_text, 64_000);
    let mut updated = false;
    if let Some(sessions) = state.get_mut("sessions").and_then(Value::as_array_mut) {
        for session in sessions.iter_mut() {
            let sid = clean_text(
                session.get("session_id").and_then(Value::as_str).unwrap_or(""),
                120,
            );
            if sid != active_id {
                continue;
            }
            if !session.get("messages").map(Value::is_array).unwrap_or(false) {
                session["messages"] = Value::Array(Vec::new());
            }
            let messages = session
                .get_mut("messages")
                .and_then(Value::as_array_mut)
                .expect("messages");
            let target_idx = messages.iter().rposition(|row| {
                clean_text(row.get("role").and_then(Value::as_str).unwrap_or(""), 24)
                    .eq_ignore_ascii_case("assistant")
            });
            let idx = if let Some(found) = target_idx {
                found
            } else {
                messages.push(json!({"role": "assistant", "text": assistant, "ts": crate::now_iso()}));
                messages.len().saturating_sub(1)
            };
            if let Some(target) = messages.get_mut(idx) {
                if !assistant.is_empty() {
                    target["text"] = Value::String(assistant.clone());
                }
                if let Some(object) = metadata.as_object() {
                    for (key, value) in object {
                        target[key] = value.clone();
                    }
                }
                if target.get("ts").and_then(Value::as_str).unwrap_or("").is_empty() {
                    target["ts"] = Value::String(crate::now_iso());
                }
            }
            session["updated_at"] = Value::String(crate::now_iso());
            updated = true;
            break;
        }
    }
    save_session_state(root, &id, &state);
    json!({"ok": true, "updated": updated, "agent_id": id})
}
