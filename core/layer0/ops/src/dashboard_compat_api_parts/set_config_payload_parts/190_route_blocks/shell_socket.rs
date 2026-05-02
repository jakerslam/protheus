const SHELL_SOCKET_PREFIX: &str = "/api/shell-socket";

fn shell_socket_receipt_ref(capability: &str, seed: &Value) -> String {
    let hash = crate::deterministic_receipt_hash(&json!({
        "capability": capability,
        "seed": seed,
        "ts": crate::now_iso()
    }));
    format!(
        "shell_socket:{}:{}",
        clean_text(capability, 80),
        hash.chars().take(16).collect::<String>()
    )
}

fn shell_socket_cursor_offset(path: &str) -> usize {
    let raw = query_value(path, "cursor").or_else(|| query_value(path, "offset"));
    let Some(cursor) = raw else {
        return 0;
    };
    cursor
        .strip_prefix("offset=")
        .unwrap_or(cursor.as_str())
        .parse::<usize>()
        .unwrap_or(0)
}

fn shell_socket_limit(path: &str, default_limit: usize, max_limit: usize) -> usize {
    query_value(path, "limit")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default_limit)
        .clamp(1, max_limit)
}

fn shell_socket_path_parts(path_only: &str) -> Vec<String> {
    path_only
        .trim_start_matches(SHELL_SOCKET_PREFIX)
        .trim_matches('/')
        .split('/')
        .filter(|part| !part.is_empty())
        .map(decode_path_segment)
        .collect::<Vec<_>>()
}

fn shell_socket_session_ref(agent_id: &str, session_id: &str) -> String {
    format!("{}::{}", clean_agent_id(agent_id), clean_text(session_id, 120))
}

fn shell_socket_decode_session_ref(raw: &str) -> (String, String) {
    let cleaned = clean_text(raw, 260);
    if let Some((agent_id, session_id)) = cleaned.split_once("::") {
        return (clean_agent_id(agent_id), clean_text(session_id, 120));
    }
    (clean_agent_id(&cleaned), "default".to_string())
}

fn shell_socket_session_rows(state: &Value) -> Vec<Value> {
    state
        .get("sessions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn shell_socket_session_messages(state: &Value, session_id: &str) -> Vec<Value> {
    let cleaned = clean_text(session_id, 120);
    for row in shell_socket_session_rows(state) {
        let row_id = clean_text(row.get("session_id").and_then(Value::as_str).unwrap_or(""), 120);
        if row_id == cleaned {
            return row
                .get("messages")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
        }
    }
    session_messages(state)
}

fn shell_socket_messages_paged(messages: &[Value], limit: usize, offset: usize) -> Vec<Value> {
    let total = messages.len();
    let end = total.saturating_sub(offset);
    let start = end.saturating_sub(limit);
    messages[start..end].to_vec()
}

fn shell_socket_runtime_status(root: &Path, snapshot: &Value, request_host: &str) -> Value {
    let legacy = status_payload(root, snapshot, request_host);
    let connected = legacy
        .get("connected")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let degraded = legacy
        .get("degraded")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let state = if connected && !degraded { "ready" } else { "degraded" };
    let label = if connected { "Runtime connected" } else { "Runtime unavailable" };
    json!({
        "state": state,
        "label": label,
        "source": "gateway.dashboard_compat_api",
        "source_sequence": legacy.get("receipt_hash").cloned().unwrap_or(Value::Null),
        "age_seconds": 0,
        "stale": false,
        "degraded_reason": clean_text(legacy.get("warning").and_then(Value::as_str).unwrap_or(""), 160),
        "next_retry_hint": Value::Null,
        "receipt_ref": shell_socket_receipt_ref("get_runtime_status", &legacy),
        "correlation_id": "shell_socket.runtime_status"
    })
}

fn shell_socket_agent_roster(root: &Path, path: &str, snapshot: &Value) -> Value {
    let include_terminated = query_value(path, "include_terminated")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let limit = shell_socket_limit(path, 50, 100);
    let offset = shell_socket_cursor_offset(path);
    let rows = compact_sidebar_roster_rows(build_sidebar_agent_roster_fast(
        root,
        snapshot,
        include_terminated,
    ));
    let total = rows.len();
    let window = rows
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<Value>>();
    let mut labels = Map::<String, Value>::new();
    let mut previews = Map::<String, Value>::new();
    let mut detail_refs = Map::<String, Value>::new();
    let mut status_counts = Map::<String, Value>::new();
    let mut agent_ids = Vec::<Value>::new();
    for row in &window {
        let id = clean_agent_id(row.get("id").and_then(Value::as_str).unwrap_or(""));
        if id.is_empty() {
            continue;
        }
        let state = clean_text(
            row.get("sidebar_status_state")
                .or_else(|| row.get("state"))
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            60,
        );
        let count = status_counts.get(&state).and_then(Value::as_i64).unwrap_or(0) + 1;
        status_counts.insert(state, json!(count));
        agent_ids.push(json!(id.clone()));
        labels.insert(
            id.clone(),
            json!(clean_text(row.get("name").and_then(Value::as_str).unwrap_or(&id), 120)),
        );
        previews.insert(
            id.clone(),
            json!(clean_text(
                row.get("sidebar_status_label")
                    .or_else(|| row.get("role"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                180
            )),
        );
        detail_refs.insert(id.clone(), json!(format!("agent:{id}")));
    }
    json!({
        "agents": window,
        "agent_ids": agent_ids,
        "active_agent_id": Value::Null,
        "labels": labels,
        "status_counts": status_counts,
        "last_activity_preview": previews,
        "next_cursor": if offset + limit < total { json!(format!("offset={}", offset + limit)) } else { Value::Null },
        "detail_refs": detail_refs,
        "receipt_ref": shell_socket_receipt_ref("list_agents", &json!({"offset": offset, "limit": limit})),
        "correlation_id": "shell_socket.agent_roster"
    })
}

fn shell_socket_session_list(root: &Path, agent_id: &str, path: &str) -> Value {
    let id = clean_agent_id(agent_id);
    let state = load_session_state(root, &id);
    let active = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    let limit = shell_socket_limit(path, 40, 100);
    let offset = shell_socket_cursor_offset(path);
    let rows = shell_socket_session_rows(&state);
    let total = rows.len();
    let mut sessions = Vec::<Value>::new();
    let mut session_ids = Vec::<Value>::new();
    let mut previews = Map::<String, Value>::new();
    let mut counts = Map::<String, Value>::new();
    let mut detail_refs = Map::<String, Value>::new();
    for row in rows.into_iter().skip(offset).take(limit) {
        let sid = clean_text(row.get("session_id").and_then(Value::as_str).unwrap_or("default"), 120);
        let session_ref = shell_socket_session_ref(&id, &sid);
        let messages = row
            .get("messages")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let preview = messages
            .last()
            .map(session_projection_text)
            .unwrap_or_default();
        session_ids.push(json!(session_ref.clone()));
        previews.insert(session_ref.clone(), json!(clean_text(&preview, 240)));
        counts.insert(session_ref.clone(), json!(messages.len()));
        detail_refs.insert(session_ref.clone(), json!(format!("session:{session_ref}")));
        sessions.push(json!({
            "id": session_ref,
            "session_id": sid,
            "label": clean_text(row.get("label").and_then(Value::as_str).unwrap_or("Session"), 120),
            "updated_at": row.get("updated_at").cloned().unwrap_or(Value::Null),
            "message_count": messages.len(),
            "detail_ref": format!("session:{}", shell_socket_session_ref(&id, &sid))
        }));
    }
    json!({
        "sessions": sessions,
        "session_ids": session_ids,
        "active_session_id": shell_socket_session_ref(&id, &active),
        "last_message_previews": previews,
        "message_counts": counts,
        "next_cursor": if offset + limit < total { json!(format!("offset={}", offset + limit)) } else { Value::Null },
        "detail_refs": detail_refs,
        "receipt_ref": shell_socket_receipt_ref("list_sessions", &json!({"agent_id": id, "offset": offset, "limit": limit})),
        "correlation_id": "shell_socket.session_list"
    })
}

fn shell_socket_message_window(root: &Path, session_ref: &str, path: &str) -> Value {
    let (agent_id, session_id) = shell_socket_decode_session_ref(session_ref);
    let state = load_session_state(root, &agent_id);
    let active = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    let all = shell_socket_session_messages(&state, &session_id);
    let limit = shell_socket_limit(path, 80, 80);
    let offset = shell_socket_cursor_offset(path);
    let window_rows = shell_socket_messages_paged(&all, limit, offset);
    let message_window = session_message_window(&agent_id, &window_rows, all.len(), offset);
    json!({
        "ok": true,
        "agent_id": agent_id,
        "session_id": shell_socket_session_ref(&agent_id, &session_id),
        "active_session_id": shell_socket_session_ref(&agent_id, &active),
        "message_window": message_window,
        "message_count": all.len(),
        "total_count": all.len(),
        "has_more": offset + limit < all.len(),
        "before_cursor": message_window.get("before_cursor").cloned().unwrap_or(Value::Null),
        "after_cursor": message_window.get("after_cursor").cloned().unwrap_or(Value::Null),
        "detail_refs": json!({"message_window": format!("agent_session:{}:window:{}", agent_id, offset)}),
        "receipt_ref": shell_socket_receipt_ref("get_message_window", &json!({"agent_id": agent_id, "session_id": session_id, "offset": offset, "limit": limit})),
        "correlation_id": "shell_socket.message_window"
    })
}

fn shell_socket_detail_projection(root: &Path, detail_ref: &str, path: &str) -> Option<Value> {
    let requested_view = query_value(path, "view").unwrap_or_else(|| "summary".to_string());
    if detail_ref.starts_with("agent:") || detail_ref.starts_with("session:") {
        let id = clean_text(detail_ref.split_once(':').map(|(_, value)| value).unwrap_or(""), 180);
        return Some(json!({
            "detail_id": id,
            "detail_kind": clean_text(detail_ref.split_once(':').map(|(kind, _)| kind).unwrap_or("detail"), 80),
            "requested_view": requested_view,
            "detail_projection": json!({}),
            "size_bound": json!({"max_response_bytes": 65536, "max_string_chars": SESSION_DETAIL_TEXT_BOUND_CHARS}),
            "next_cursor": Value::Null,
            "receipt_ref": shell_socket_receipt_ref("get_message_detail", &json!({"detail_ref": detail_ref})),
            "correlation_id": "shell_socket.message_detail"
        }));
    }
    let parsed = if detail_ref.starts_with("/api/agents/") {
        parse_agent_route(detail_ref)
    } else {
        None
    }?;
    let (agent_id, parts) = parsed;
    if parts.len() != 3 || parts[0] != "details" {
        return None;
    }
    let legacy = session_detail_payload(root, &agent_id, &parts[1], &parts[2]);
    Some(json!({
        "detail_id": clean_text(legacy.get("detail_id").and_then(Value::as_str).unwrap_or(&parts[2]), 180),
        "detail_kind": clean_text(legacy.get("detail_kind").and_then(Value::as_str).unwrap_or(&parts[1]), 80),
        "requested_view": requested_view,
        "detail_projection": legacy.get("detail_projection").cloned().unwrap_or_else(|| json!({})),
        "size_bound": legacy.get("size_bound").cloned().unwrap_or_else(|| json!({"max_response_bytes": 65536})),
        "next_cursor": Value::Null,
        "receipt_ref": legacy.get("receipt_ref").cloned().unwrap_or_else(|| json!(shell_socket_receipt_ref("get_message_detail", &legacy))),
        "correlation_id": legacy.get("correlation_id").cloned().unwrap_or_else(|| json!("shell_socket.message_detail"))
    }))
}

fn shell_socket_event_projection(root: &Path, session_ref: &str) -> Value {
    let (agent_id, session_id) = shell_socket_decode_session_ref(session_ref);
    let state = load_session_state(root, &agent_id);
    let count = shell_socket_session_messages(&state, &session_id).len();
    json!({
        "event_id": format!("{}:{}:snapshot", agent_id, session_id),
        "event_kind": "session_snapshot",
        "agent_id": agent_id,
        "session_id": shell_socket_session_ref(&agent_id, &session_id),
        "display_projection": json!({"status": "ready", "message_count": count}),
        "status_label": "ready",
        "cursor_refs": json!({"next": Value::Null}),
        "detail_refs": json!({"message_window": format!("{}::{}", agent_id, session_id)}),
        "receipt_refs": json!([shell_socket_receipt_ref("subscribe_events", &json!({"agent_id": agent_id, "session_id": session_id}))]),
        "correlation_id": "shell_socket.events"
    })
}

fn shell_socket_search(root: &Path, path: &str) -> Value {
    let query = query_value(path, "q")
        .or_else(|| query_value(path, "query"))
        .unwrap_or_default();
    let limit = shell_socket_limit(path, 20, 80);
    let legacy = crate::dashboard_internal_search::search_conversations(root, &query, limit);
    let results = legacy
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut hits = Vec::<Value>::new();
    let mut hit_ids = Vec::<Value>::new();
    let mut snippets = Map::<String, Value>::new();
    let mut labels = Map::<String, Value>::new();
    let mut detail_refs = Map::<String, Value>::new();
    for row in results {
        let id = clean_agent_id(row.get("agent_id").and_then(Value::as_str).unwrap_or(""));
        if id.is_empty() {
            continue;
        }
        let snippet = clean_text(row.get("snippet").and_then(Value::as_str).unwrap_or(""), 260);
        let label = clean_text(row.get("name").and_then(Value::as_str).unwrap_or(&id), 120);
        hit_ids.push(json!(id.clone()));
        snippets.insert(id.clone(), json!(snippet));
        labels.insert(id.clone(), json!(label.clone()));
        detail_refs.insert(id.clone(), json!(format!("agent:{id}")));
        hits.push(json!({"id": id, "kind": "agent", "label": label, "snippet": snippet, "detail_ref": format!("agent:{id}")}));
    }
    json!({
        "query_id": shell_socket_receipt_ref("search", &json!({"q": query, "limit": limit})),
        "hits": hits,
        "hit_ids": hit_ids,
        "snippets": snippets,
        "labels": labels,
        "counts": json!({"hits": hit_ids.len()}),
        "next_cursor": Value::Null,
        "detail_refs": detail_refs,
        "receipt_ref": shell_socket_receipt_ref("search", &legacy),
        "correlation_id": "shell_socket.search"
    })
}

fn shell_socket_ingress_ack(capability: &str, accepted: bool, reason_code: &str, seed: &Value) -> Value {
    let receipt_ref = shell_socket_receipt_ref(capability, seed);
    json!({
        "accepted": accepted,
        "rejected": !accepted,
        "reason_code": clean_text(reason_code, 120),
        "receipt_ref": receipt_ref,
        "follow_up_ref": if accepted { json!(format!("follow_up:{receipt_ref}")) } else { Value::Null },
        "correlation_id": format!("shell_socket.{capability}")
    })
}

fn shell_socket_ack_from_legacy(capability: &str, legacy: CompatApiResponse) -> CompatApiResponse {
    let ok = legacy.status < 400
        && legacy
            .payload
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(true);
    let reason = if ok {
        "accepted"
    } else {
        legacy
            .payload
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("gateway_rejected")
    };
    CompatApiResponse {
        status: if ok { 202 } else { 200 },
        payload: shell_socket_ingress_ack(capability, ok, reason, &legacy.payload),
    }
}

include!("shell_socket_parts/020-routes.rs");
