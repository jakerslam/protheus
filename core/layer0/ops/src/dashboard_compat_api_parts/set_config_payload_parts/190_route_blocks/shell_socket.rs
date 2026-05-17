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

fn shell_socket_runtime_status(_root: &Path, snapshot: &Value, _request_host: &str) -> Value {
    let connected = snapshot.get("ok").and_then(Value::as_bool).unwrap_or(true)
        && snapshot
            .get("connected")
            .and_then(Value::as_bool)
            .unwrap_or(true);
    let degraded = snapshot
        .get("degraded")
        .and_then(Value::as_bool)
        .unwrap_or(!connected);
    let state = if connected && !degraded { "ready" } else { "degraded" };
    let label = if connected && !degraded {
        "Runtime connected"
    } else if connected {
        "Runtime degraded"
    } else {
        "Runtime unavailable"
    };
    json!({
        "state": state,
        "label": label,
        "source": "gateway.dashboard_compat_api",
        "source_sequence": snapshot.get("receipt_hash").cloned().unwrap_or(Value::Null),
        "age_seconds": 0,
        "stale": false,
        "degraded_reason": clean_text(snapshot.get("warning").and_then(Value::as_str).unwrap_or(""), 160),
        "next_retry_hint": Value::Null,
        "receipt_ref": shell_socket_receipt_ref("get_runtime_status", snapshot),
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
    if detail_ref.starts_with("session_artifact:") {
        let legacy = session_artifact_detail_payload(root, detail_ref)?;
        return Some(json!({
            "detail_id": clean_text(legacy.get("detail_id").and_then(Value::as_str).unwrap_or(detail_ref), 180),
            "detail_kind": clean_text(legacy.get("detail_kind").and_then(Value::as_str).unwrap_or("session_artifact"), 80),
            "requested_view": requested_view,
            "detail_projection": legacy.get("detail_projection").cloned().unwrap_or_else(|| json!({})),
            "size_bound": legacy.get("size_bound").cloned().unwrap_or_else(|| json!({"max_response_bytes": 65536})),
            "next_cursor": Value::Null,
            "receipt_ref": legacy.get("receipt_ref").cloned().unwrap_or_else(|| json!(shell_socket_receipt_ref("get_message_detail", &legacy))),
            "correlation_id": legacy.get("correlation_id").cloned().unwrap_or_else(|| json!("shell_socket.message_detail"))
        }));
    }
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
    if parts[2].starts_with("session_artifact:") {
        let legacy = session_artifact_detail_payload(root, &parts[2])?;
        return Some(json!({
            "detail_id": clean_text(legacy.get("detail_id").and_then(Value::as_str).unwrap_or(&parts[2]), 180),
            "detail_kind": clean_text(legacy.get("detail_kind").and_then(Value::as_str).unwrap_or("session_artifact"), 80),
            "requested_view": requested_view,
            "detail_projection": legacy.get("detail_projection").cloned().unwrap_or_else(|| json!({})),
            "size_bound": legacy.get("size_bound").cloned().unwrap_or_else(|| json!({"max_response_bytes": 65536})),
            "next_cursor": Value::Null,
            "receipt_ref": legacy.get("receipt_ref").cloned().unwrap_or_else(|| json!(shell_socket_receipt_ref("get_message_detail", &legacy))),
            "correlation_id": legacy.get("correlation_id").cloned().unwrap_or_else(|| json!("shell_socket.message_detail"))
        }));
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

fn shell_socket_searchable_message_text(row: &Value) -> String {
    let mut parts = Vec::<String>::new();
    for key in [
        "text",
        "content_preview",
        "search_text",
        "notice_label",
        "notice_type",
        "role",
        "status",
        "meta",
    ] {
        if let Some(value) = row.get(key).and_then(Value::as_str) {
            let cleaned = clean_text(value, 2_000);
            if !cleaned.is_empty() {
                parts.push(cleaned);
            }
        }
    }
    if let Some(tools) = row.get("tools").and_then(Value::as_array) {
        for tool in tools.iter().take(24) {
            for key in ["name", "tool", "status", "summary"] {
                if let Some(value) = tool.get(key).and_then(Value::as_str) {
                    let cleaned = clean_text(value, 240);
                    if !cleaned.is_empty() {
                        parts.push(cleaned);
                    }
                }
            }
        }
    }
    parts.join("\n")
}

fn shell_socket_search_matches(text: &str, query: &str, terms: &[String]) -> bool {
    let haystack = text.to_ascii_lowercase();
    let needle = query.to_ascii_lowercase();
    if !needle.is_empty() && haystack.contains(&needle) {
        return true;
    }
    !terms.is_empty() && terms.iter().all(|term| haystack.contains(term))
}

fn shell_socket_message_hit(agent_id: &str, session_id: &str, message: &Value, absolute_index: usize) -> Value {
    let projected = project_session_message_row(agent_id, message, absolute_index);
    let message_id = projected
        .get("id")
        .and_then(Value::as_str)
        .map(|value| clean_text(value, 180))
        .unwrap_or_else(|| format!("idx-{absolute_index}"));
    let preview = clean_text(
        projected
            .get("text")
            .or_else(|| projected.get("content_preview"))
            .or_else(|| projected.get("notice_label"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        360,
    );
    let detail_ref = format!("/api/agents/{}/details/message/{}", clean_agent_id(agent_id), message_id);
    json!({
        "id": message_id,
        "kind": "message",
        "agent_id": clean_agent_id(agent_id),
        "session_id": shell_socket_session_ref(agent_id, session_id),
        "message_id": message_id,
        "message_index": absolute_index,
        "role": projected.get("role").cloned().unwrap_or_else(|| json!("")),
        "label": projected.get("agent_name").or_else(|| projected.get("role")).cloned().unwrap_or_else(|| json!("message")),
        "snippet": preview,
        "preview": preview,
        "ts": projected.get("ts").or_else(|| projected.get("timestamp")).cloned().unwrap_or(Value::Null),
        "detail_ref": detail_ref
    })
}

fn shell_socket_search_indexed_session_messages(
    root: &Path,
    agent_id: &str,
    session_id: &str,
    query: &str,
    terms: &[String],
    limit: usize,
    offset: usize,
) -> Option<(Vec<Value>, usize, usize)> {
    use std::io::BufRead;

    let path = session_index_messages_path(root, agent_id, session_id);
    let file = fs::File::open(&path).ok()?;
    let reader = std::io::BufReader::new(file);
    let mut hits = Vec::<Value>::new();
    let mut total_hits = 0usize;
    let mut scanned = 0usize;
    for (idx, line) in reader.lines().enumerate() {
        let Ok(line) = line else {
            continue;
        };
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        scanned = scanned.saturating_add(1);
        let searchable = shell_socket_searchable_message_text(&message);
        if !shell_socket_search_matches(&searchable, query, terms) {
            continue;
        }
        total_hits = total_hits.saturating_add(1);
        if total_hits <= offset {
            continue;
        }
        if hits.len() < limit {
            hits.push(shell_socket_message_hit(agent_id, session_id, &message, idx));
        }
    }
    Some((hits, total_hits, scanned))
}

fn shell_socket_search_legacy_session_messages(
    root: &Path,
    agent_id: &str,
    session_id: &str,
    query: &str,
    terms: &[String],
    limit: usize,
    offset: usize,
) -> (Vec<Value>, usize, usize) {
    let state = load_session_state(root, agent_id);
    let messages = shell_socket_session_messages(&state, session_id);
    let mut hits = Vec::<Value>::new();
    let mut total_hits = 0usize;
    for (idx, message) in messages.iter().enumerate() {
        let searchable = shell_socket_searchable_message_text(message);
        if !shell_socket_search_matches(&searchable, query, terms) {
            continue;
        }
        total_hits = total_hits.saturating_add(1);
        if total_hits <= offset {
            continue;
        }
        if hits.len() < limit {
            hits.push(shell_socket_message_hit(agent_id, session_id, message, idx));
        }
    }
    (hits, total_hits, messages.len())
}

fn shell_socket_session_search(root: &Path, path: &str, cleaned_query: &str, terms: &[String]) -> Value {
    let agent_id = clean_agent_id(
        query_value(path, "agent_id")
            .or_else(|| query_value(path, "agent"))
            .unwrap_or_default()
            .as_str(),
    );
    let limit = shell_socket_limit(path, 20, 80);
    let offset = shell_socket_cursor_offset(path);
    if agent_id.is_empty() {
        return json!({
            "ok": false,
            "error": "agent_id_required",
            "query_id": shell_socket_receipt_ref("search", &json!({"q": cleaned_query, "limit": limit, "agent_id": agent_id})),
            "hits": [],
            "hit_ids": [],
            "counts": json!({"hits": 0, "total_hits": 0, "scanned": 0}),
            "next_cursor": Value::Null,
            "detail_refs": json!({}),
            "receipt_ref": shell_socket_receipt_ref("search", &json!({"q": cleaned_query, "limit": limit, "agent_id": agent_id, "error": "agent_id_required"})),
            "correlation_id": "shell_socket.search"
        });
    }
    let session_id = query_value(path, "session_id")
        .or_else(|| query_value(path, "session"))
        .map(|value| clean_session_id(&value))
        .or_else(|| load_session_index_meta(root, &agent_id).map(|meta| indexed_active_session_id(&meta)))
        .unwrap_or_else(|| {
            clean_session_id(
                load_session_state(root, &agent_id)
                    .get("active_session_id")
                    .and_then(Value::as_str)
                    .unwrap_or("default"),
            )
        });
    let (hits, total_hits, scanned, storage_source) =
        if let Some((hits, total_hits, scanned)) = shell_socket_search_indexed_session_messages(
            root,
            &agent_id,
            &session_id,
            cleaned_query,
            terms,
            limit,
            offset,
        ) {
            (hits, total_hits, scanned, "indexed_session_jsonl")
        } else {
            let (hits, total_hits, scanned) = shell_socket_search_legacy_session_messages(
                root,
                &agent_id,
                &session_id,
                cleaned_query,
                terms,
                limit,
                offset,
            );
            (hits, total_hits, scanned, "legacy_session_json_fallback")
        };
    let mut hit_ids = Vec::<Value>::new();
    let mut snippets = Map::<String, Value>::new();
    let mut labels = Map::<String, Value>::new();
    let mut detail_refs = Map::<String, Value>::new();
    for hit in &hits {
        let id = clean_text(hit.get("id").and_then(Value::as_str).unwrap_or(""), 180);
        if id.is_empty() {
            continue;
        }
        hit_ids.push(json!(id.clone()));
        snippets.insert(id.clone(), hit.get("snippet").cloned().unwrap_or_else(|| json!("")));
        labels.insert(id.clone(), hit.get("label").cloned().unwrap_or_else(|| json!("message")));
        detail_refs.insert(id.clone(), hit.get("detail_ref").cloned().unwrap_or(Value::Null));
    }
    json!({
        "ok": true,
        "type": "shell_socket_message_search_projection_v1",
        "agent_id": agent_id,
        "session_id": shell_socket_session_ref(&agent_id, &session_id),
        "query_id": shell_socket_receipt_ref("search", &json!({"q": cleaned_query, "limit": limit, "agent_id": agent_id, "session_id": session_id})),
        "hits": hits,
        "hit_ids": hit_ids,
        "snippets": snippets,
        "labels": labels,
        "counts": json!({"hits": hit_ids.len(), "total_hits": total_hits, "scanned": scanned}),
        "next_cursor": if offset + limit < total_hits { json!(format!("offset={}", offset + limit)) } else { Value::Null },
        "detail_refs": detail_refs,
        "storage_source": storage_source,
        "receipt_ref": shell_socket_receipt_ref("search", &json!({"q": cleaned_query, "limit": limit, "agent_id": agent_id, "session_id": session_id, "hits": hit_ids.len(), "total_hits": total_hits})),
        "correlation_id": "shell_socket.search"
    })
}

fn shell_socket_search(root: &Path, path: &str) -> Value {
    let query = query_value(path, "q")
        .or_else(|| query_value(path, "query"))
        .unwrap_or_default();
    let limit = shell_socket_limit(path, 20, 80);
    let cleaned_query = clean_text(&query, 260);
    let terms = cleaned_query
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|term| term.len() >= 2)
        .map(|term| term.to_string())
        .collect::<Vec<String>>();
    let mut hits = Vec::<Value>::new();
    let mut hit_ids = Vec::<Value>::new();
    let mut snippets = Map::<String, Value>::new();
    let mut labels = Map::<String, Value>::new();
    let mut detail_refs = Map::<String, Value>::new();
    if cleaned_query.is_empty() || terms.is_empty() {
        return json!({
            "query_id": shell_socket_receipt_ref("search", &json!({"q": cleaned_query, "limit": limit})),
            "hits": hits,
            "hit_ids": hit_ids,
            "snippets": snippets,
            "labels": labels,
            "counts": json!({"hits": 0}),
            "next_cursor": Value::Null,
            "detail_refs": detail_refs,
            "receipt_ref": shell_socket_receipt_ref("search", &json!({"q": cleaned_query, "limit": limit, "empty": true})),
            "correlation_id": "shell_socket.search"
        });
    }
    if query_value(path, "agent_id").is_some() || query_value(path, "agent").is_some() {
        return shell_socket_session_search(root, path, &cleaned_query, &terms);
    }
    let rows = compact_sidebar_roster_rows(build_sidebar_agent_roster_fast(root, &json!({}), true));
    let mut scored = Vec::<(i64, String, Value)>::new();
    for row in rows {
        let id = clean_agent_id(row.get("agent_id").and_then(Value::as_str).unwrap_or(""));
        let id = if id.is_empty() {
            clean_agent_id(row.get("id").and_then(Value::as_str).unwrap_or(""))
        } else {
            id
        };
        if id.is_empty() {
            continue;
        }
        let label = clean_text(row.get("name").and_then(Value::as_str).unwrap_or(&id), 120);
        let state = clean_text(
            row.get("sidebar_status_state")
                .or_else(|| row.get("state"))
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            80,
        );
        let preview = clean_text(
            row.get("sidebar_status_label")
                .or_else(|| row.get("role"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            180,
        );
        let haystack = format!("{id} {label} {state} {preview}").to_ascii_lowercase();
        let mut score = 0i64;
        if haystack.contains(&cleaned_query.to_ascii_lowercase()) {
            score += 100;
        }
        for term in &terms {
            if haystack.contains(term) {
                score += 20;
            }
        }
        if score <= 0 {
            continue;
        }
        scored.push((
            score,
            clean_text(row.get("updated_at").and_then(Value::as_str).unwrap_or(""), 80),
            json!({
                "id": id,
                "kind": "agent",
                "label": label,
                "snippet": if preview.is_empty() { state } else { preview },
                "detail_ref": format!("agent:{id}")
            }),
        ));
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
    for (_, _, row) in scored.into_iter().take(limit) {
        let id = clean_agent_id(row.get("id").and_then(Value::as_str).unwrap_or(""));
        if id.is_empty() {
            continue;
        }
        hit_ids.push(json!(id.clone()));
        snippets.insert(id.clone(), row.get("snippet").cloned().unwrap_or_else(|| json!("")));
        labels.insert(id.clone(), row.get("label").cloned().unwrap_or_else(|| json!(id.clone())));
        detail_refs.insert(id.clone(), row.get("detail_ref").cloned().unwrap_or_else(|| json!(format!("agent:{id}"))));
        hits.push(row);
    }
    json!({
        "query_id": shell_socket_receipt_ref("search", &json!({"q": cleaned_query, "limit": limit})),
        "hits": hits,
        "hit_ids": hit_ids,
        "snippets": snippets,
        "labels": labels,
        "counts": json!({"hits": hit_ids.len()}),
        "next_cursor": Value::Null,
        "detail_refs": detail_refs,
        "receipt_ref": shell_socket_receipt_ref("search", &json!({"q": cleaned_query, "limit": limit, "hits": hit_ids.len()})),
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

fn shell_socket_message_tool_projection(tool: &Value, idx: usize) -> Value {
    let row = tool.as_object();
    let read_text = |key: &str, limit: usize| -> String {
        clean_text(row.and_then(|map| map.get(key)).and_then(Value::as_str).unwrap_or(""), limit)
    };
    let name = read_text("name", 120);
    let tool_name = read_text("tool", 120);
    json!({
        "id": read_text("id", 160),
        "name": if name.is_empty() { tool_name.clone() } else { name },
        "tool": tool_name,
        "status": read_text("status", 80),
        "input_preview": clean_text(
            row.and_then(|map| map.get("input_preview").or_else(|| map.get("arguments_preview")).or_else(|| map.get("args_preview")))
                .and_then(Value::as_str)
                .unwrap_or(""),
            2000,
        ),
        "result_preview": clean_text(
            row.and_then(|map| map.get("result_preview").or_else(|| map.get("output_preview")).or_else(|| map.get("summary")))
                .and_then(Value::as_str)
                .unwrap_or(""),
            4000,
        ),
        "is_error": row.and_then(|map| map.get("is_error")).and_then(Value::as_bool).unwrap_or(false),
        "blocked": row.and_then(|map| map.get("blocked")).and_then(Value::as_bool).unwrap_or(false),
        "attempt_id": read_text("attempt_id", 160),
        "attempt_sequence": row
            .and_then(|map| map.get("attempt_sequence"))
            .and_then(Value::as_u64)
            .unwrap_or((idx + 1) as u64)
    })
}

fn shell_socket_message_result_projection(legacy: CompatApiResponse) -> CompatApiResponse {
    let payload = legacy.payload;
    let ok = legacy.status < 400 && payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out = Map::<String, Value>::new();
    out.insert("ok".to_string(), json!(ok));
    for key in [
        "response",
        "content",
        "input_tokens",
        "output_tokens",
        "cost_usd",
        "iterations",
        "agent_id",
        "agent_name",
        "auto_route",
        "route",
        "context_tokens",
        "context_used_tokens",
        "context_total_tokens",
        "context_window",
        "context_window_tokens",
        "context_ratio",
        "context_pressure",
        "context_pool",
    ] {
        if let Some(value) = payload.get(key) {
            out.insert(key.to_string(), value.clone());
        }
    }
    let tools = payload
        .get("tools")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .take(16)
                .enumerate()
                .map(|(idx, tool)| shell_socket_message_tool_projection(tool, idx))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    out.insert("tools".to_string(), json!(tools));
    out.insert(
        "detail_refs".to_string(),
        json!({"message_result": shell_socket_receipt_ref("submit_message_result_detail", &payload)}),
    );
    out.insert(
        "receipt_ref".to_string(),
        json!(shell_socket_receipt_ref("submit_message_result", &payload)),
    );
    out.insert(
        "correlation_id".to_string(),
        json!("shell_socket.submit_message_result"),
    );
    CompatApiResponse {
        status: if ok { 200 } else { legacy.status.max(400) },
        payload: Value::Object(out),
    }
}

fn shell_socket_agent_mutation_projection(capability: &str, legacy: CompatApiResponse) -> CompatApiResponse {
    let payload = legacy.payload;
    let ok = legacy.status < 400 && payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out = Map::<String, Value>::new();
    out.insert("ok".to_string(), json!(ok));
    for key in ["agent_id", "mode", "provider", "model", "runtime_model", "rename_notice"] {
        if let Some(value) = payload.get(key) {
            out.insert(key.to_string(), value.clone());
        }
    }
    if let Some(filters) = payload.get("tool_filters") {
        out.insert("tool_filters".to_string(), filters.clone());
    }
    if let Some(error) = payload.get("error").and_then(Value::as_str) {
        out.insert("error".to_string(), json!(clean_text(error, 240)));
    }
    out.insert(
        "receipt_ref".to_string(),
        json!(shell_socket_receipt_ref(capability, &payload)),
    );
    out.insert(
        "correlation_id".to_string(),
        json!(format!("shell_socket.{capability}")),
    );
    CompatApiResponse {
        status: if ok { 200 } else { legacy.status.max(400) },
        payload: Value::Object(out),
    }
}

fn shell_socket_agent_lifecycle_projection(capability: &str, legacy: CompatApiResponse) -> CompatApiResponse {
    let payload = legacy.payload;
    let ok = legacy.status < 400 && payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out = Map::<String, Value>::new();
    out.insert("ok".to_string(), json!(ok));
    for key in [
        "id",
        "agent_id",
        "name",
        "role",
        "state",
        "type",
        "archived",
        "reason",
        "removed_history_entries",
        "deleted_archived_agents",
    ] {
        if let Some(value) = payload.get(key) {
            out.insert(key.to_string(), value.clone());
        }
    }
    if let Some(error) = payload.get("error").and_then(Value::as_str) {
        out.insert("error".to_string(), json!(clean_text(error, 240)));
    }
    out.insert(
        "receipt_ref".to_string(),
        json!(shell_socket_receipt_ref(capability, &payload)),
    );
    out.insert(
        "correlation_id".to_string(),
        json!(format!("shell_socket.{capability}")),
    );
    CompatApiResponse {
        status: if ok { 200 } else { legacy.status.max(400) },
        payload: Value::Object(out),
    }
}

fn shell_socket_session_lifecycle_projection(capability: &str, legacy: CompatApiResponse) -> CompatApiResponse {
    let payload = legacy.payload;
    let ok = legacy.status < 400 && payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out = Map::<String, Value>::new();
    out.insert("ok".to_string(), json!(ok));
    for key in ["agent_id", "session_id", "active_session_id", "label", "title", "type"] {
        if let Some(value) = payload.get(key) {
            out.insert(key.to_string(), value.clone());
        }
    }
    if let Some(error) = payload.get("error").and_then(Value::as_str) {
        out.insert("error".to_string(), json!(clean_text(error, 240)));
    }
    out.insert(
        "receipt_ref".to_string(),
        json!(shell_socket_receipt_ref(capability, &payload)),
    );
    out.insert(
        "correlation_id".to_string(),
        json!(format!("shell_socket.{capability}")),
    );
    CompatApiResponse {
        status: if ok { 200 } else { legacy.status.max(400) },
        payload: Value::Object(out),
    }
}

fn shell_socket_suggestion_projection(capability: &str, legacy: CompatApiResponse) -> CompatApiResponse {
    let payload = legacy.payload;
    let ok = legacy.status < 400 && payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out = Map::<String, Value>::new();
    out.insert("ok".to_string(), json!(ok));
    if let Some(agent_id) = payload.get("agent_id") {
        out.insert("agent_id".to_string(), agent_id.clone());
    }
    let suggestions = payload
        .get("suggestions")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .take(12)
                .enumerate()
                .map(|(idx, row)| {
                    if let Some(text) = row.as_str() {
                        return json!({
                            "id": format!("suggestion-{idx}"),
                            "label": clean_text(text, 220)
                        });
                    }
                    json!({
                        "id": clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 80),
                        "label": clean_text(
                            row.get("label")
                                .and_then(Value::as_str)
                                .or_else(|| row.get("title").and_then(Value::as_str))
                                .or_else(|| row.get("text").and_then(Value::as_str))
                                .unwrap_or(""),
                            220
                        ),
                        "detail_ref": row.get("detail_ref").cloned().unwrap_or(Value::Null)
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    out.insert("suggestions".to_string(), Value::Array(suggestions));
    if let Some(error) = payload.get("error").and_then(Value::as_str) {
        out.insert("error".to_string(), json!(clean_text(error, 240)));
    }
    out.insert(
        "receipt_ref".to_string(),
        json!(shell_socket_receipt_ref(capability, &payload)),
    );
    out.insert(
        "correlation_id".to_string(),
        json!(format!("shell_socket.{capability}")),
    );
    CompatApiResponse {
        status: if ok { 200 } else { legacy.status.max(400) },
        payload: Value::Object(out),
    }
}

fn shell_socket_artifact_projection(capability: &str, legacy: CompatApiResponse) -> CompatApiResponse {
    let payload = legacy.payload;
    let ok = legacy.status < 400 && payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out = Map::<String, Value>::new();
    out.insert("ok".to_string(), json!(ok));
    for key in ["file", "folder", "archive", "routing"] {
        if let Some(value) = payload.get(key) {
            out.insert(key.to_string(), value.clone());
        }
    }
    if let Some(error) = payload.get("error").and_then(Value::as_str) {
        out.insert("error".to_string(), json!(clean_text(error, 240)));
    }
    out.insert(
        "receipt_ref".to_string(),
        json!(shell_socket_receipt_ref(capability, &payload)),
    );
    out.insert(
        "correlation_id".to_string(),
        json!(format!("shell_socket.{capability}")),
    );
    CompatApiResponse {
        status: if ok { 200 } else { legacy.status.max(400) },
        payload: Value::Object(out),
    }
}

include!("shell_socket_parts/020-routes.rs");
