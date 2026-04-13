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
