const SESSION_DETAIL_TEXT_BOUND_CHARS: usize = 12_000;
const SESSION_DETAIL_ARRAY_BOUND: usize = 20;

fn session_message_projection_id(message: &Value, absolute_index: usize) -> String {
    for key in ["id", "message_id", "turn_id", "receipt_ref", "detail_ref"] {
        let value = clean_text(message.get(key).and_then(Value::as_str).unwrap_or(""), 160);
        if !value.is_empty() {
            return value;
        }
    }
    format!("message-{absolute_index}")
}

fn session_detail_ref(agent_id: &str, detail_kind: &str, detail_id: &str) -> String {
    format!(
        "/api/agents/{}/details/{}/{}",
        clean_agent_id(agent_id),
        clean_text(detail_kind, 80),
        clean_text(detail_id, 180)
    )
}

fn session_projection_text(message: &Value) -> String {
    for key in ["content_preview", "text", "content", "message", "assistant", "user"] {
        if let Some(value) = message.get(key) {
            if let Some(text) = value.as_str() {
                let compact = clean_text(text, SESSION_DETAIL_TEXT_BOUND_CHARS);
                if !compact.is_empty() {
                    return compact;
                }
            } else if value.is_array() || value.is_object() {
                let compact = clean_text(&value.to_string(), SESSION_DETAIL_TEXT_BOUND_CHARS);
                if !compact.is_empty() {
                    return compact;
                }
            }
        }
    }
    String::new()
}

fn session_projection_line_count(text: &str) -> usize {
    if text.is_empty() {
        0
    } else {
        text.lines().count().max(1)
    }
}

fn session_tool_detail_id(message_id: &str, tool: &Value, idx: usize) -> String {
    for key in ["tool_result_id", "id", "receipt_ref", "receipt_id", "detail_ref"] {
        let value = clean_text(tool.get(key).and_then(Value::as_str).unwrap_or(""), 160);
        if !value.is_empty() && !value.contains('/') {
            return value;
        }
    }
    format!("{message_id}-tool-{idx}")
}

fn session_artifact_detail_id(message_id: &str, artifact: &Value, idx: usize) -> String {
    for key in ["artifact_id", "id", "detail_ref", "receipt_ref"] {
        let value = clean_text(artifact.get(key).and_then(Value::as_str).unwrap_or(""), 160);
        if !value.is_empty() && !value.contains('/') {
            return value;
        }
    }
    format!("{message_id}-artifact-{idx}")
}

fn session_trace_detail_id(message_id: &str, message: &Value) -> String {
    for key in ["trace_id", "correlation_id", "receipt_ref", "id"] {
        let value = clean_text(message.get(key).and_then(Value::as_str).unwrap_or(""), 160);
        if !value.is_empty() && !value.contains('/') {
            return value;
        }
    }
    format!("{message_id}-trace")
}

fn session_workflow_detail_id(message_id: &str, message: &Value) -> String {
    for key in ["workflow_id", "workflow_ref", "correlation_id", "receipt_ref", "id"] {
        let value = clean_text(message.get(key).and_then(Value::as_str).unwrap_or(""), 160);
        if !value.is_empty() && !value.contains('/') {
            return value;
        }
    }
    format!("{message_id}-workflow")
}

fn session_project_tool_summary(agent_id: &str, message_id: &str, tool: &Value, idx: usize) -> Value {
    let detail_id = session_tool_detail_id(message_id, tool, idx);
    let name = clean_text(
        tool.get("name")
            .or_else(|| tool.get("tool"))
            .or_else(|| tool.get("tool_name"))
            .and_then(Value::as_str)
            .unwrap_or("tool"),
        120,
    );
    let status = clean_text(
        tool.get("status")
            .or_else(|| tool.get("display_state"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    let summary = clean_text(
        tool.get("summary")
            .or_else(|| tool.get("display_text"))
            .or_else(|| tool.get("label"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        800,
    );
    let detail_ref = session_detail_ref(agent_id, "tool-result", &detail_id);
    json!({
        "id": detail_id,
        "name": name,
        "status": status,
        "summary": summary,
        "detail_ref": detail_ref,
        "input_ref": detail_ref,
        "result_ref": detail_ref,
        "is_error": tool.get("is_error").and_then(Value::as_bool).unwrap_or(false)
    })
}

fn session_project_artifact_summary(
    agent_id: &str,
    message_id: &str,
    artifact: &Value,
    idx: usize,
) -> Value {
    let detail_id = session_artifact_detail_id(message_id, artifact, idx);
    let detail_ref = session_detail_ref(agent_id, "artifact", &detail_id);
    json!({
        "id": detail_id,
        "name": clean_text(artifact.get("name").or_else(|| artifact.get("filename")).and_then(Value::as_str).unwrap_or("artifact"), 160),
        "kind": clean_text(artifact.get("kind").or_else(|| artifact.get("type")).and_then(Value::as_str).unwrap_or("artifact"), 80),
        "summary": clean_text(artifact.get("summary").or_else(|| artifact.get("preview")).and_then(Value::as_str).unwrap_or(""), 800),
        "detail_ref": detail_ref
    })
}

fn session_message_tools(message: &Value) -> Vec<Value> {
    message
        .get("tools")
        .or_else(|| message.get("tool_calls"))
        .or_else(|| message.get("response_tools"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn session_message_artifacts(message: &Value) -> Vec<Value> {
    message
        .get("artifacts")
        .or_else(|| message.get("artifact_rows"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn project_session_message_row(agent_id: &str, message: &Value, absolute_index: usize) -> Value {
    let message_id = session_message_projection_id(message, absolute_index);
    let text = session_projection_text(message);
    let tools = session_message_tools(message)
        .into_iter()
        .take(SESSION_DETAIL_ARRAY_BOUND)
        .enumerate()
        .map(|(idx, tool)| session_project_tool_summary(agent_id, &message_id, &tool, idx))
        .collect::<Vec<Value>>();
    let artifacts = session_message_artifacts(message)
        .into_iter()
        .take(SESSION_DETAIL_ARRAY_BOUND)
        .enumerate()
        .map(|(idx, artifact)| session_project_artifact_summary(agent_id, &message_id, &artifact, idx))
        .collect::<Vec<Value>>();
    let mut row = Map::<String, Value>::new();
    row.insert("id".to_string(), json!(message_id));
    row.insert(
        "role".to_string(),
        json!(clean_text(
            message
                .get("role")
                .or_else(|| message.get("type"))
                .and_then(Value::as_str)
                .unwrap_or("agent"),
            80
        )),
    );
    row.insert("text".to_string(), json!(text));
    row.insert("content_preview".to_string(), row.get("text").cloned().unwrap_or(Value::Null));
    row.insert("line_count".to_string(), json!(session_projection_line_count(row.get("text").and_then(Value::as_str).unwrap_or(""))));
    row.insert("detail_ref".to_string(), json!(session_detail_ref(agent_id, "message", row.get("id").and_then(Value::as_str).unwrap_or(""))));
    row.insert("tools".to_string(), Value::Array(tools.clone()));
    row.insert("tool_summary_count".to_string(), json!(tools.len()));
    row.insert("artifacts".to_string(), Value::Array(artifacts.clone()));
    row.insert("artifact_summary_count".to_string(), json!(artifacts.len()));
    for key in [
        "ts",
        "timestamp",
        "created_at",
        "status",
        "agent_id",
        "agent_name",
        "terminal",
        "is_notice",
        "notice_label",
        "notice_type",
        "notice_icon",
        "notice_action",
        "progress_percent",
        "progress_label",
    ] {
        if let Some(value) = message.get(key) {
            row.insert(key.to_string(), value.clone());
        }
    }
    if message.get("trace_id").is_some() || message.get("decision_trace").is_some() {
        let id = session_trace_detail_id(row.get("id").and_then(Value::as_str).unwrap_or("message"), message);
        row.insert("trace_detail_ref".to_string(), json!(session_detail_ref(agent_id, "trace", &id)));
    }
    if message.get("workflow_id").is_some() || message.get("workflow_graph").is_some() {
        let id = session_workflow_detail_id(row.get("id").and_then(Value::as_str).unwrap_or("message"), message);
        row.insert("workflow_detail_ref".to_string(), json!(session_detail_ref(agent_id, "workflow", &id)));
    }
    Value::Object(row)
}

fn session_message_window(
    agent_id: &str,
    messages: &[Value],
    total_messages: usize,
    offset: usize,
) -> Value {
    let end_exclusive = total_messages.saturating_sub(offset);
    let start = end_exclusive.saturating_sub(messages.len());
    let end_index = end_exclusive.saturating_sub(1);
    let has_more_before = start > 0;
    let has_more_after = end_exclusive < total_messages;
    let projected_rows = messages
        .iter()
        .enumerate()
        .map(|(idx, row)| project_session_message_row(agent_id, row, start + idx))
        .collect::<Vec<Value>>();
    let message_ids = projected_rows
        .iter()
        .filter_map(|row| row.get("id").and_then(Value::as_str).map(|id| json!(id)))
        .collect::<Vec<Value>>();
    let window_start_id = projected_rows
        .first()
        .and_then(|row| row.get("id"))
        .cloned()
        .unwrap_or(Value::Null);
    let window_end_id = projected_rows
        .last()
        .and_then(|row| row.get("id"))
        .cloned()
        .unwrap_or(Value::Null);
    json!({
        "rows": projected_rows,
        "message_ids": message_ids,
        "window_start_id": window_start_id,
        "window_end_id": window_end_id,
        "before_cursor": if has_more_before { json!(format!("offset={}", total_messages.saturating_sub(start))) } else { Value::Null },
        "after_cursor": if has_more_after { json!(format!("offset={}", total_messages.saturating_sub(end_exclusive))) } else { Value::Null },
        "offset": offset,
        "limit": messages.len(),
        "total_count": total_messages,
        "has_more": has_more_before
    })
}

fn session_payload(root: &Path, agent_id: &str) -> Value {
    session_payload_paged(root, agent_id, 80, 0)
}

fn session_payload_paged(root: &Path, agent_id: &str, limit: usize, offset: usize) -> Value {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let state = load_session_state(root, &id);
    let bounded_limit = if limit == 0 { 80 } else { limit.min(80) };
    let (messages, total_messages) = session_messages_paged(&state, bounded_limit, offset);
    let message_window = session_message_window(&id, &messages, total_messages, offset);
    let has_more = message_window
        .get("has_more")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let sessions = session_rows_payload(&state);
    let mut state_slim = state.clone();
    if let Some(arr) = state_slim.get_mut("sessions").and_then(Value::as_array_mut) {
        for s in arr.iter_mut() {
            if let Some(obj) = s.as_object_mut() {
                obj.remove("messages");
            }
        }
    }
    json!({
        "ok": true,
        "agent_id": id,
        "active_session_id": state.get("active_session_id").cloned().unwrap_or_else(|| json!("default")),
        "message_window": message_window,
        "message_count": total_messages,
        "total_count": total_messages,
        "has_more": has_more,
        "sessions": sessions,
        "session": state_slim,
        "detail_refs": json!({ "message_window": format!("agent_session:{id}:window:{offset}") }),
        "receipt_ref": format!("agent_session_window:{id}:{offset}"),
        "correlation_id": format!("agent_session_window:{id}:{offset}")
    })
}


fn bounded_detail_envelope(
    agent_id: &str,
    detail_kind: &str,
    detail_id: &str,
    capability_scope: &str,
    detail_projection: Value,
    found: bool,
) -> Value {
    let id = clean_text(detail_id, 180);
    json!({
        "ok": found,
        "agent_id": clean_agent_id(agent_id),
        "detail_kind": clean_text(detail_kind, 80),
        "detail_id": id,
        "capability_scope": capability_scope,
        "size_bound": {
            "max_response_bytes": 65536,
            "max_string_chars": SESSION_DETAIL_TEXT_BOUND_CHARS,
            "max_array_items": SESSION_DETAIL_ARRAY_BOUND
        },
        "window_bound": {
            "max_rows": SESSION_DETAIL_ARRAY_BOUND,
            "cursor_required_for_overflow": true
        },
        "detail_projection": detail_projection,
        "receipt_ref": format!("detail_fetch:{}:{}", clean_agent_id(agent_id), id),
        "correlation_id": format!("detail_fetch:{}:{}", clean_agent_id(agent_id), id),
        "audit_receipt": true,
        "nexus_checkpoint": true
    })
}

fn session_detail_message_payload(agent_id: &str, messages: &[Value], detail_id: &str) -> Value {
    for (idx, message) in messages.iter().enumerate() {
        let message_id = session_message_projection_id(message, idx);
        if message_id == detail_id {
            let projection = project_session_message_row(agent_id, message, idx);
            return bounded_detail_envelope(
                agent_id,
                "message_detail",
                detail_id,
                "shell.message.detail.read",
                projection,
                true,
            );
        }
    }
    bounded_detail_envelope(agent_id, "message_detail", detail_id, "shell.message.detail.read", json!({}), false)
}

fn session_detail_tool_payload(agent_id: &str, messages: &[Value], detail_id: &str) -> Value {
    for (message_idx, message) in messages.iter().enumerate() {
        let message_id = session_message_projection_id(message, message_idx);
        for (tool_idx, tool) in session_message_tools(message).iter().enumerate() {
            let tool_id = session_tool_detail_id(&message_id, tool, tool_idx);
            if tool_id == detail_id {
                let projection = json!({
                    "id": tool_id,
                    "message_id": message_id,
                    "name": clean_text(tool.get("name").or_else(|| tool.get("tool")).and_then(Value::as_str).unwrap_or("tool"), 120),
                    "summary": clean_text(tool.get("summary").or_else(|| tool.get("display_text")).and_then(Value::as_str).unwrap_or(""), 1200),
                    "input": clean_text(tool.get("input").and_then(Value::as_str).unwrap_or(""), SESSION_DETAIL_TEXT_BOUND_CHARS),
                    "result": clean_text(tool.get("result").or_else(|| tool.get("output")).and_then(Value::as_str).unwrap_or(""), SESSION_DETAIL_TEXT_BOUND_CHARS),
                    "is_error": tool.get("is_error").and_then(Value::as_bool).unwrap_or(false)
                });
                return bounded_detail_envelope(
                    agent_id,
                    "tool_result_detail",
                    detail_id,
                    "shell.tool.detail.read",
                    projection,
                    true,
                );
            }
        }
    }
    bounded_detail_envelope(agent_id, "tool_result_detail", detail_id, "shell.tool.detail.read", json!({}), false)
}

fn session_detail_artifact_payload(agent_id: &str, messages: &[Value], detail_id: &str) -> Value {
    for (message_idx, message) in messages.iter().enumerate() {
        let message_id = session_message_projection_id(message, message_idx);
        for (artifact_idx, artifact) in session_message_artifacts(message).iter().enumerate() {
            let artifact_id = session_artifact_detail_id(&message_id, artifact, artifact_idx);
            if artifact_id == detail_id {
                let projection = json!({
                    "id": artifact_id,
                    "message_id": message_id,
                    "name": clean_text(artifact.get("name").or_else(|| artifact.get("filename")).and_then(Value::as_str).unwrap_or("artifact"), 160),
                    "kind": clean_text(artifact.get("kind").or_else(|| artifact.get("type")).and_then(Value::as_str).unwrap_or("artifact"), 80),
                    "body": clean_text(artifact.get("body").or_else(|| artifact.get("content")).and_then(Value::as_str).unwrap_or(""), SESSION_DETAIL_TEXT_BOUND_CHARS)
                });
                return bounded_detail_envelope(
                    agent_id,
                    "artifact_detail",
                    detail_id,
                    "shell.artifact.detail.read",
                    projection,
                    true,
                );
            }
        }
    }
    bounded_detail_envelope(agent_id, "artifact_detail", detail_id, "shell.artifact.detail.read", json!({}), false)
}

fn session_detail_trace_payload(agent_id: &str, messages: &[Value], detail_id: &str) -> Value {
    for (idx, message) in messages.iter().enumerate() {
        let message_id = session_message_projection_id(message, idx);
        if session_trace_detail_id(&message_id, message) == detail_id {
            let projection = json!({
                "id": detail_id,
                "message_id": message_id,
                "trace": clean_text(message.get("decision_trace").or_else(|| message.get("trace_body")).map(Value::to_string).as_deref().unwrap_or(""), SESSION_DETAIL_TEXT_BOUND_CHARS)
            });
            return bounded_detail_envelope(agent_id, "trace_detail", detail_id, "shell.trace.detail.read", projection, true);
        }
    }
    bounded_detail_envelope(agent_id, "trace_detail", detail_id, "shell.trace.detail.read", json!({}), false)
}

fn session_detail_workflow_payload(agent_id: &str, messages: &[Value], detail_id: &str) -> Value {
    for (idx, message) in messages.iter().enumerate() {
        let message_id = session_message_projection_id(message, idx);
        if session_workflow_detail_id(&message_id, message) == detail_id {
            let projection = json!({
                "id": detail_id,
                "message_id": message_id,
                "workflow": clean_text(message.get("workflow_graph").or_else(|| message.get("workflow")).map(Value::to_string).as_deref().unwrap_or(""), SESSION_DETAIL_TEXT_BOUND_CHARS)
            });
            return bounded_detail_envelope(agent_id, "workflow_detail", detail_id, "shell.workflow.detail.read", projection, true);
        }
    }
    bounded_detail_envelope(agent_id, "workflow_detail", detail_id, "shell.workflow.detail.read", json!({}), false)
}

fn session_detail_payload(root: &Path, agent_id: &str, detail_kind: &str, detail_id: &str) -> Value {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let state = load_session_state(root, &id);
    let messages = session_messages(&state);
    let kind = clean_text(detail_kind, 80).to_ascii_lowercase();
    let detail = clean_text(detail_id, 180);
    match kind.as_str() {
        "message" | "message-detail" | "message_detail" => {
            session_detail_message_payload(&id, &messages, &detail)
        }
        "tool-result" | "tool_result" | "tool" => {
            session_detail_tool_payload(&id, &messages, &detail)
        }
        "artifact" | "artifact-detail" | "artifact_detail" => {
            session_detail_artifact_payload(&id, &messages, &detail)
        }
        "trace" | "trace-detail" | "trace_detail" => {
            session_detail_trace_payload(&id, &messages, &detail)
        }
        "workflow" | "workflow-detail" | "workflow_detail" => {
            session_detail_workflow_payload(&id, &messages, &detail)
        }
        _ => json!({
            "ok": false,
            "error": "unsupported_detail_kind",
            "agent_id": id,
            "detail_kind": kind,
            "detail_id": detail
        }),
    }
}
