fn normalize_session_state(agent_id: &str, mut state: Value) -> Value {
    let id = clean_agent_id(agent_id);
    if !state.is_object() {
        state = default_session_state(&id);
    }
    state["agent_id"] = Value::String(id);
    if !state
        .get("active_session_id")
        .and_then(Value::as_str)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
    {
        state["active_session_id"] = Value::String("default".to_string());
    }
    if !state.get("sessions").map(Value::is_array).unwrap_or(false) {
        state["sessions"] = Value::Array(Vec::new());
    }
    if state
        .get("sessions")
        .and_then(Value::as_array)
        .map(|rows| rows.is_empty())
        .unwrap_or(true)
    {
        state["sessions"] = Value::Array(vec![json!({
            "session_id": "default",
            "label": "Session",
            "created_at": crate::now_iso(),
            "updated_at": crate::now_iso(),
            "messages": []
        })]);
    }
    if !state
        .get("memory_kv")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["memory_kv"] = json!({});
    }
    state
}

fn load_session_state(root: &Path, agent_id: &str) -> Value {
    let path = session_path(root, agent_id);
    let state = read_json_loose(&path).unwrap_or_else(|| default_session_state(agent_id));
    normalize_session_state(agent_id, state)
}

fn save_session_state(root: &Path, agent_id: &str, state: &Value) {
    let path = session_path(root, agent_id);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let bounded_state = bound_session_state_for_persistence(root, agent_id, state);
    write_json_pretty(&path, &bounded_state);
}

const SESSION_MESSAGE_TEXT_MAX_CHARS: usize = 64_000;
const SESSION_MESSAGE_PREVIEW_MAX_CHARS: usize = 4_000;
const SESSION_TOOL_PREVIEW_MAX_CHARS: usize = 1_200;
const SESSION_TERMINAL_PREVIEW_MAX_CHARS: usize = 1_000;
const SESSION_FORBIDDEN_PROJECTION_KEYS: [&str; 9] = [
    "raw",
    "root",
    "trace_body",
    "decision_trace",
    "workflow_graph",
    "execution_observation",
    "response_workflow",
    "response_finalization",
    "process_summary",
];

fn session_artifact_dir(root: &Path, agent_id: &str) -> PathBuf {
    state_path(root, "client/runtime/local/state/ui/infring_dashboard/session_artifacts")
        .join(clean_agent_id(agent_id))
}

fn persist_session_artifact_ref(root: &Path, agent_id: &str, kind: &str, value: &Value) -> Value {
    if value.is_null() {
        return Value::Null;
    }
    let id = clean_agent_id(agent_id);
    let artifact_kind = clean_text(kind, 80)
        .replace('/', "_")
        .replace('\\', "_");
    if id.is_empty() || artifact_kind.is_empty() {
        return Value::Null;
    }
    let hash = crate::deterministic_receipt_hash(value);
    let dir = session_artifact_dir(root, &id);
    let _ = fs::create_dir_all(&dir);
    let path = dir.join(format!("{artifact_kind}-{hash}.json"));
    if !path.exists() {
        write_json_pretty(&path, value);
    }
    json!({
        "kind": artifact_kind,
        "ref": format!("session_artifact:{id}:{artifact_kind}:{hash}"),
        "sha256": hash,
        "bytes": value.to_string().len()
    })
}

fn compact_session_tool_rows(root: &Path, agent_id: &str, tools: &Value) -> Value {
    let rows = tools.as_array().cloned().unwrap_or_default();
    Value::Array(rows.into_iter().take(24).enumerate().map(|(idx, tool)| {
        let existing_ref = tool.get("detail_ref").and_then(Value::as_str).unwrap_or("");
        let detail_ref = if existing_ref.is_empty() {
            persist_session_artifact_ref(root, agent_id, &format!("tool_{idx}"), &tool)
        } else {
            json!({"ref": existing_ref})
        };
        let name = clean_text(tool.get("name").or_else(|| tool.get("tool")).and_then(Value::as_str).unwrap_or("tool"), 120);
        let status = clean_text(tool.get("status").and_then(Value::as_str).unwrap_or(""), 80);
        let is_error = tool.get("is_error").and_then(Value::as_bool).unwrap_or(false);
        let blocked = tool.get("blocked").and_then(Value::as_bool).unwrap_or(false);
        let result_preview = clean_text(
            tool.get("summary")
                .or_else(|| tool.get("result"))
                .or_else(|| tool.get("output"))
                .map(Value::to_string)
                .as_deref()
                .unwrap_or(""),
            SESSION_TOOL_PREVIEW_MAX_CHARS,
        );
        let input_preview = clean_text(
            tool.get("input")
                .or_else(|| tool.get("arguments"))
                .or_else(|| tool.get("payload"))
                .map(Value::to_string)
                .as_deref()
                .unwrap_or(""),
            SESSION_TOOL_PREVIEW_MAX_CHARS,
        );
        json!({
            "name": name,
            "tool": name,
            "status": status,
            "is_error": is_error,
            "blocked": blocked,
            "result_preview": result_preview,
            "input_preview": input_preview,
            "detail_ref": detail_ref.get("ref").cloned().unwrap_or(Value::Null)
        })
    }).collect::<Vec<_>>())
}

fn compact_workflow_visibility(value: &Value) -> Value {
    json!({
        "contract": value.get("contract").cloned().unwrap_or_else(|| json!("workflow_visibility_payload_v1")),
        "current_stage": clean_text(value.get("current_stage").and_then(Value::as_str).unwrap_or(""), 120),
        "current_stage_status": clean_text(value.get("current_stage_status").and_then(Value::as_str).unwrap_or(""), 120),
        "ui_status": clean_text(value.get("ui_status").and_then(Value::as_str).unwrap_or(""), 240),
        "agent_process_status": clean_text(value.get("agent_process_status").and_then(Value::as_str).unwrap_or(""), 240),
        "debug_status": clean_text(value.get("debug_status").and_then(Value::as_str).unwrap_or(""), 240),
        "selected_workflow_id": clean_text(value.get("selected_workflow_id").and_then(Value::as_str).unwrap_or(""), 160),
        "visible_response_source": clean_text(value.get("visible_response_source").and_then(Value::as_str).unwrap_or(""), 120),
        "visible_chat_text_authority": clean_text(value.get("visible_chat_text_authority").and_then(Value::as_str).unwrap_or(""), 120),
        "system_chat_injection_used": value.get("system_chat_injection_used").and_then(Value::as_bool).unwrap_or(false)
    })
}

fn compact_terminal_transcript(root: &Path, agent_id: &str, transcript: &Value) -> Value {
    let rows = transcript.as_array().cloned().unwrap_or_default();
    Value::Array(rows.into_iter().take(12).enumerate().map(|(idx, row)| {
        let detail_ref = persist_session_artifact_ref(root, agent_id, &format!("terminal_{idx}"), &row);
        json!({
            "tool": clean_text(row.get("tool").and_then(Value::as_str).unwrap_or("terminal"), 120),
            "command": clean_text(row.get("command").and_then(Value::as_str).unwrap_or(""), 500),
            "cwd": clean_text(row.get("cwd").and_then(Value::as_str).unwrap_or(""), 500),
            "output_preview": clean_text(row.get("output").and_then(Value::as_str).unwrap_or(""), SESSION_TERMINAL_PREVIEW_MAX_CHARS),
            "is_error": row.get("is_error").and_then(Value::as_bool).unwrap_or(false),
            "detail_ref": detail_ref.get("ref").cloned().unwrap_or(Value::Null)
        })
    }).collect::<Vec<_>>())
}

fn session_safe_turn_metadata(root: &Path, agent_id: &str, metadata: &Value) -> Value {
    let mut out = Map::<String, Value>::new();
    let mut refs = Map::<String, Value>::new();
    if let Some(tools) = metadata.get("tools") {
        refs.insert("tools".to_string(), persist_session_artifact_ref(root, agent_id, "tools", tools));
        out.insert("tools".to_string(), compact_session_tool_rows(root, agent_id, tools));
    }
    for key in ["response_workflow", "response_finalization", "process_summary"] {
        if let Some(value) = metadata.get(key) {
            refs.insert(key.to_string(), persist_session_artifact_ref(root, agent_id, key, value));
        }
    }
    if let Some(value) = metadata.get("workflow_visibility") {
        refs.insert("workflow_visibility".to_string(), persist_session_artifact_ref(root, agent_id, "workflow_visibility", value));
        out.insert("workflow_visibility".to_string(), compact_workflow_visibility(value));
    }
    if let Some(value) = metadata.get("turn_transaction") {
        refs.insert("turn_transaction".to_string(), persist_session_artifact_ref(root, agent_id, "turn_transaction", value));
        out.insert("turn_transaction".to_string(), json!({
            "contract_version": value.get("contract_version").cloned().unwrap_or(Value::Null),
            "complete": value.get("complete").cloned().unwrap_or(Value::Null),
            "first_incomplete_stage": clean_text(value.get("first_incomplete_stage").and_then(Value::as_str).unwrap_or(""), 120),
            "receipt_id": clean_text(value.get("receipt_id").and_then(Value::as_str).unwrap_or(""), 160)
        }));
    }
    if let Some(value) = metadata.get("terminal_transcript") {
        refs.insert("terminal_transcript".to_string(), persist_session_artifact_ref(root, agent_id, "terminal_transcript", value));
        out.insert("terminal_transcript".to_string(), compact_terminal_transcript(root, agent_id, value));
    }
    if !refs.is_empty() {
        out.insert("detail_refs".to_string(), Value::Object(refs));
        out.insert("session_projection_contract".to_string(), json!("session_message_projection_v1"));
    }
    Value::Object(out)
}

fn strip_forbidden_session_projection_keys(value: &mut Value) {
    match value {
        Value::Array(rows) => {
            for row in rows {
                strip_forbidden_session_projection_keys(row);
            }
        }
        Value::Object(obj) => {
            let keys = obj.keys().cloned().collect::<Vec<_>>();
            for key in keys {
                if SESSION_FORBIDDEN_PROJECTION_KEYS.contains(&key.as_str()) {
                    obj.remove(&key);
                    continue;
                }
                if key == "detail_refs" {
                    continue;
                }
                if let Some(nested) = obj.get_mut(&key) {
                    strip_forbidden_session_projection_keys(nested);
                }
            }
        }
        _ => {}
    }
}

fn bound_session_message_for_persistence(root: &Path, agent_id: &str, message: &Value) -> Value {
    let Some(obj) = message.as_object() else {
        return json!({});
    };
    let mut out = obj.clone();
    for key in SESSION_FORBIDDEN_PROJECTION_KEYS {
        if let Some(value) = out.remove(key) {
            let detail = persist_session_artifact_ref(root, agent_id, key, &value);
            out.entry("detail_refs".to_string())
                .or_insert_with(|| json!({}))
                .as_object_mut()
                .map(|refs| { refs.insert(key.to_string(), detail); });
        }
    }
    if let Some(text) = out.get("text").and_then(Value::as_str) {
        out.insert("text".to_string(), Value::String(clean_text(text, SESSION_MESSAGE_TEXT_MAX_CHARS)));
    }
    if let Some(tools) = out.get("tools").cloned() {
        out.insert("tools".to_string(), compact_session_tool_rows(root, agent_id, &tools));
    }
    if let Some(visibility) = out.get("workflow_visibility").cloned() {
        out.insert("workflow_visibility".to_string(), compact_workflow_visibility(&visibility));
    }
    if let Some(transcript) = out.get("terminal_transcript").cloned() {
        out.insert("terminal_transcript".to_string(), compact_terminal_transcript(root, agent_id, &transcript));
    }
    let mut projected = Value::Object(out);
    strip_forbidden_session_projection_keys(&mut projected);
    projected
}

fn bound_session_state_for_persistence(root: &Path, agent_id: &str, state: &Value) -> Value {
    let mut bounded = state.clone();
    if let Some(sessions) = bounded.get_mut("sessions").and_then(Value::as_array_mut) {
        for session in sessions.iter_mut() {
            if let Some(messages) = session.get_mut("messages").and_then(Value::as_array_mut) {
                *messages = messages
                    .iter()
                    .map(|message| bound_session_message_for_persistence(root, agent_id, message))
                    .collect::<Vec<_>>();
            }
            if let Some(keyframes) = session.get_mut("context_keyframes").and_then(Value::as_array_mut) {
                keyframes.truncate(24);
                for keyframe in keyframes.iter_mut() {
                    if let Some(obj) = keyframe.as_object_mut() {
                        if let Some(summary) = obj.get("summary").and_then(Value::as_str) {
                            obj.insert("summary".to_string(), Value::String(clean_text(summary, SESSION_MESSAGE_PREVIEW_MAX_CHARS)));
                        }
                    }
                }
            }
        }
    }
    bounded
}

#[cfg(test)]
mod session_output_boundary_tests {
    use super::*;

    fn collect_forbidden_projection_hits(value: &Value, path: &str, hits: &mut Vec<String>) {
        match value {
            Value::Array(rows) => {
                for (idx, row) in rows.iter().enumerate() {
                    collect_forbidden_projection_hits(row, &format!("{path}.{idx}"), hits);
                }
            }
            Value::Object(obj) => {
                for (key, nested) in obj {
                    let next_path = format!("{path}.{key}");
                    if key != "detail_refs" && SESSION_FORBIDDEN_PROJECTION_KEYS.contains(&key.as_str()) {
                        hits.push(next_path.clone());
                    }
                    if key != "detail_refs" {
                        collect_forbidden_projection_hits(nested, &next_path, hits);
                    }
                }
            }
            _ => {}
        }
    }

    #[test]
    fn session_persistence_projects_messages_and_routes_raw_details_to_refs() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let agent_id = "agent-boundary";
        let state = json!({
            "agent_id": agent_id,
            "active_session_id": "default",
            "sessions": [{
                "session_id": "default",
                "messages": [{
                    "id": "msg-1",
                    "role": "assistant",
                    "text": "visible text",
                    "raw": {"secret": "raw-runtime-state"},
                    "root": {"store": "full-root-state"},
                    "trace_body": {"trace": ["too", "large"]},
                    "metadata": {
                        "raw": {"nested": "raw-runtime-state"},
                        "workflow_graph": {"nodes": [1, 2, 3]}
                    },
                    "tools": [{
                        "name": "search",
                        "status": "done",
                        "input": {"query": "full input payload"},
                        "result": {"very": "large result payload"},
                        "raw": {"tool": "raw detail"}
                    }],
                    "workflow_visibility": {
                        "current_stage": "Searching",
                        "workflow_graph": {"nodes": ["not for shell"]}
                    }
                }]
            }]
        });

        let bounded = bound_session_state_for_persistence(root, agent_id, &state);
        let message = bounded
            .pointer("/sessions/0/messages/0")
            .expect("projected message");
        let mut hits = Vec::<String>::new();
        collect_forbidden_projection_hits(message, "message", &mut hits);
        assert!(hits.is_empty(), "forbidden projection keys leaked: {hits:?}");

        assert_eq!(message.pointer("/tools/0/name").and_then(Value::as_str), Some("search"));
        assert!(message.pointer("/tools/0/input").is_none());
        assert!(message.pointer("/tools/0/result").is_none());
        assert!(message.pointer("/tools/0/raw").is_none());
        assert!(message.pointer("/tools/0/input_preview").and_then(Value::as_str).unwrap_or("").contains("query"));
        assert!(message.pointer("/tools/0/result_preview").and_then(Value::as_str).unwrap_or("").contains("large result"));
        assert!(message.pointer("/tools/0/detail_ref").and_then(Value::as_str).unwrap_or("").starts_with("session_artifact:agent-boundary:tool_0:"));
        let raw_ref = message.pointer("/detail_refs/raw/ref").and_then(Value::as_str).unwrap_or("");
        assert!(raw_ref.starts_with("session_artifact:agent-boundary:raw:"));

        let artifact_dir = session_artifact_dir(root, agent_id);
        let artifact_count = fs::read_dir(&artifact_dir).expect("artifact dir").count();
        assert!(artifact_count >= 2, "expected raw/tool detail artifacts in {artifact_dir:?}");

        let lazy_detail = shell_socket_detail_projection(root, raw_ref, "/api/shell/details/session_artifact?view=full")
            .expect("session artifact detail projection");
        assert_eq!(
            lazy_detail.pointer("/detail_projection/value/secret").and_then(Value::as_str),
            Some("raw-runtime-state")
        );
        assert_eq!(
            lazy_detail.pointer("/detail_kind").and_then(Value::as_str),
            Some("session_artifact_detail")
        );
    }

    #[test]
    fn turn_metadata_projection_keeps_workflow_status_but_not_raw_workflow_payloads() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let metadata = json!({
            "response_workflow": {"plan_graph": {"nodes": [1, 2, 3]}},
            "response_finalization": {"execution_observation": {"raw": true}},
            "process_summary": {"trace_body": "hidden"},
            "workflow_visibility": {
                "current_stage": "Searching the web",
                "current_stage_status": "running",
                "ui_status": "Searching the web..."
            },
            "tools": [{
                "name": "web",
                "input": {"q": "payload"},
                "result": {"items": ["payload"]}
            }]
        });

        let projected = session_safe_turn_metadata(root, "agent-meta", &metadata);
        let mut hits = Vec::<String>::new();
        collect_forbidden_projection_hits(&projected, "metadata", &mut hits);
        assert!(hits.is_empty(), "forbidden metadata projection keys leaked: {hits:?}");
        assert_eq!(
            projected.pointer("/workflow_visibility/current_stage").and_then(Value::as_str),
            Some("Searching the web")
        );
        assert!(projected.pointer("/response_workflow").is_none());
        assert!(projected.pointer("/response_finalization").is_none());
        assert!(projected.pointer("/process_summary").is_none());
        assert!(projected.pointer("/detail_refs/response_workflow/ref").and_then(Value::as_str).unwrap_or("").starts_with("session_artifact:agent-meta:response_workflow:"));
    }
}

fn estimate_tokens(text: &str) -> i64 {
    ((clean_text(text, 20_000).chars().count() as i64) / 4).max(1)
}

fn active_session_row(state: &Value) -> Value {
    active_session_row_ref(state)
        .cloned()
        .unwrap_or_else(|| json!({"messages": []}))
}

fn active_session_row_ref(state: &Value) -> Option<&Value> {
    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    let rows = state
        .get("sessions")
        .and_then(Value::as_array)
        .map(|rows| rows.as_slice())
        .unwrap_or(&[]);
    if let Some(found) = rows.iter().find(|row| {
        row.get("session_id")
            .and_then(Value::as_str)
            .map(|v| clean_text(v, 120) == active_id)
            .unwrap_or(false)
    }) {
        return Some(found);
    }
    rows.first()
}

fn session_messages(state: &Value) -> Vec<Value> {
    active_session_row(state)
        .get("messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn session_messages_paged(state: &Value, limit: usize, offset: usize) -> (Vec<Value>, usize) {
    let messages = active_session_row_ref(state)
        .and_then(|row| row.get("messages"))
        .and_then(Value::as_array);
    let Some(all) = messages else {
        return (Vec::new(), 0);
    };
    let total = all.len();
    if limit == 0 {
        return (all.clone(), total);
    }
    let end = total.saturating_sub(offset);
    let start = end.saturating_sub(limit);
    (all[start..end].to_vec(), total)
}

fn all_session_messages(state: &Value) -> Vec<Value> {
    let sessions = state
        .get("sessions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut rows = Vec::<Value>::new();
    for session in sessions {
        let messages = session
            .get("messages")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        rows.extend(messages);
    }
    rows.sort_by_key(message_timestamp_iso);
    rows
}

const ACTIVE_CONTEXT_MIN_RECENT_FLOOR: usize = 28;

fn active_session_messages_sorted(state: &Value) -> Vec<Value> {
    let mut rows = session_messages(state);
    rows.sort_by_key(message_timestamp_iso);
    rows
}

fn context_source_messages(state: &Value, include_all_sessions: bool) -> Vec<Value> {
    if include_all_sessions {
        all_session_messages(state)
    } else {
        active_session_messages_sorted(state)
    }
}

fn recall_prefers_earliest(user_message: &str) -> bool {
    let lowered = clean_text(user_message, 800).to_ascii_lowercase();
    lowered.contains("first chat")
        || lowered.contains("first conversation")
        || lowered.contains("first message")
        || lowered.contains("very first")
        || lowered.contains("earliest")
        || lowered.contains("at the start")
        || lowered.contains("from the beginning")
}

fn recall_message_candidate(row: &Value, require_remember_term: bool) -> Option<String> {
    let role =
        clean_text(row.get("role").and_then(Value::as_str).unwrap_or(""), 20).to_ascii_lowercase();
    if role != "user" {
        return None;
    }
    let text = message_text(row);
    if text.is_empty() {
        return None;
    }
    if require_remember_term && !text.to_ascii_lowercase().contains("remember") {
        return None;
    }
    Some(text)
}

fn collect_user_recall_messages(
    messages: &[Value],
    prefer_earliest: bool,
    require_remember_term: bool,
    limit: usize,
) -> Vec<String> {
    let take_limit = limit.clamp(1, 8);
    let mut out = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    if prefer_earliest {
        for row in messages {
            let Some(text) = recall_message_candidate(row, require_remember_term) else {
                continue;
            };
            let key = clean_text(&text, 320).to_ascii_lowercase();
            if key.is_empty() || !seen.insert(key) {
                continue;
            }
            out.push(text);
            if out.len() >= take_limit {
                break;
            }
        }
        return out;
    }
    for row in messages.iter().rev() {
        let Some(text) = recall_message_candidate(row, require_remember_term) else {
            continue;
        };
        let key = clean_text(&text, 320).to_ascii_lowercase();
        if key.is_empty() || !seen.insert(key) {
            continue;
        }
        out.push(text);
        if out.len() >= take_limit {
            break;
        }
    }
    out
}

fn build_memory_recall_response(
    state: &Value,
    history_messages: &[Value],
    message: &str,
) -> String {
    let prefer_earliest = recall_prefers_earliest(message);
    let active_history_messages = active_session_messages_sorted(state);
    let mut remembered =
        collect_user_recall_messages(&active_history_messages, prefer_earliest, true, 4);
    if remembered.is_empty() {
        remembered =
            collect_user_recall_messages(&active_history_messages, prefer_earliest, false, 4);
    }
    if remembered.is_empty() {
        remembered = collect_user_recall_messages(history_messages, prefer_earliest, true, 3);
    }
    if remembered.is_empty() {
        remembered = collect_user_recall_messages(history_messages, prefer_earliest, false, 3);
    }
    if remembered.is_empty() {
        "I don't have enough earlier context to reference yet. Share what you want me to track, and I'll carry it forward.".to_string()
    } else {
        format!(
            "Here's what I remember from earlier: {}",
            remembered.join(" | ")
        )
    }
}

fn memory_kv_pairs_from_state(state: &Value) -> Vec<Value> {
    let mut out = state
        .get("memory_kv")
        .and_then(Value::as_object)
        .map(|rows| {
            rows.iter()
                .map(|(key, value)| {
                    json!({
                        "key": clean_text(key, 200),
                        "value": value
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    out.sort_by_key(|row| clean_text(row.get("key").and_then(Value::as_str).unwrap_or(""), 200));
    out
}

fn memory_value_timestamp(value: &Value) -> Option<DateTime<Utc>> {
    if let Some(raw) = value
        .get("captured_at")
        .or_else(|| value.get("updated_at"))
        .or_else(|| value.get("ts"))
    {
        if let Some(text) = raw.as_str() {
            if let Some(parsed) = parse_rfc3339_utc(text) {
                return Some(parsed);
            }
        } else if let Some(ms) = raw.as_i64() {
            if let Some(parsed) = DateTime::<Utc>::from_timestamp_millis(ms) {
                return Some(parsed);
            }
        }
    }
    None
}

fn memory_bucket_for_kv(key: &str, value: &Value) -> (&'static str, bool) {
    let key_lc = clean_text(key, 200).to_ascii_lowercase();
    let mut pinned = key_lc.starts_with("pin.")
        || key_lc.contains(".pin.")
        || key_lc.contains(".pinned")
        || key_lc.starts_with("fact.")
        || key_lc.starts_with("profile.")
        || key_lc.starts_with("preference.")
        || key_lc.starts_with("identity.")
        || key_lc.starts_with("user.");

    let mut memory_type = String::new();
    if let Some(obj) = value.as_object() {
        if obj.get("pinned").and_then(Value::as_bool).unwrap_or(false) {
            pinned = true;
        }
        memory_type = clean_text(
            obj.get("memory_type")
                .or_else(|| obj.get("kind"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            60,
        );
        if memory_type.eq_ignore_ascii_case("semantic") {
            pinned = true;
        }
    }

    let bucket = if pinned || memory_type.eq_ignore_ascii_case("semantic") {
        "semantic"
    } else {
        "episodic"
    };
    (bucket, pinned)
}

fn episodic_memory_is_stale(value: &Value, max_age_days: i64) -> bool {
    let Some(captured_at) = memory_value_timestamp(value) else {
        return false;
    };
    let age_days = Utc::now()
        .signed_duration_since(captured_at)
        .num_days()
        .max(0);
    age_days > max_age_days.max(1)
}

fn memory_kv_prompt_context(state: &Value, max_entries: usize) -> String {
    let mut semantic_lines = Vec::<String>::new();
    let mut episodic_lines = Vec::<String>::new();
    let kv_pairs = memory_kv_pairs_from_state(state);
    for row in kv_pairs.into_iter().take(max_entries.max(1)) {
        let key = clean_text(row.get("key").and_then(Value::as_str).unwrap_or(""), 120);
        if key.is_empty() {
            continue;
        }
        let value = row.get("value").cloned().unwrap_or(Value::Null);
        let rendered = if value.is_string() {
            clean_text(value.as_str().unwrap_or(""), 280)
        } else {
            clean_text(&value.to_string(), 280)
        };
        if rendered.is_empty() {
            continue;
        }
        if internal_context_metadata_phrase(&rendered)
            || persistent_memory_denied_phrase(&rendered)
            || runtime_access_denied_phrase(&rendered)
        {
            continue;
        }
        let (bucket, pinned) = memory_bucket_for_kv(&key, &value);
        if bucket == "episodic" && !pinned && episodic_memory_is_stale(&value, 14) {
            continue;
        }
        let line = format!("- {key}: {rendered}");
        if bucket == "semantic" {
            semantic_lines.push(line);
        } else {
            episodic_lines.push(line);
        }
    }
    semantic_lines.truncate(16);
    episodic_lines.truncate(8);

    let mut sections = Vec::<String>::new();
    if !semantic_lines.is_empty() {
        sections.push(format!(
            "Pinned semantic memory (stable facts/preferences):\n{}",
            semantic_lines.join("\n")
        ));
    }
    if !episodic_lines.is_empty() {
        sections.push(format!(
            "Recent episodic memory (working context):\n{}",
            episodic_lines.join("\n")
        ));
    }
    sections.join("\n\n")
}

fn session_rows_payload(state: &Value) -> Vec<Value> {
    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    state
        .get("sessions")
        .and_then(Value::as_array)
        .map(|rows| rows.as_slice())
        .unwrap_or(&[])
        .iter()
        .map(|row| {
            let sid = clean_text(
                row.get("session_id").and_then(Value::as_str).unwrap_or(""),
                120,
            );
            let label = clean_text(
                row.get("label")
                    .and_then(Value::as_str)
                    .unwrap_or("Session"),
                80,
            );
            let updated_at = clean_text(
                row.get("updated_at").and_then(Value::as_str).unwrap_or(""),
                80,
            );
            let message_count = row
                .get("messages")
                .and_then(Value::as_array)
                .map(|rows| rows.len() as i64)
                .unwrap_or(0);
            json!({
                "id": sid,
                "session_id": sid,
                "label": if label.is_empty() { "Session" } else { &label },
                "updated_at": updated_at,
                "message_count": message_count,
                "active": sid == active_id
            })
        })
        .collect::<Vec<_>>()
}

fn split_model_ref(
    model_ref: &str,
    fallback_provider: &str,
    fallback_model: &str,
) -> (String, String) {
    fn normalize_runtime_provider_id(raw: &str) -> String {
        let lowered = clean_text(raw, 80)
            .replace('_', "-")
            .to_ascii_lowercase();
        match lowered.as_str() {
            "google" => "gemini".to_string(),
            "xai" => "grok".to_string(),
            "moonshot" => "kimi".to_string(),
            "azure-openai-responses" => "openai".to_string(),
            _ => lowered,
        }
    }

    let cleaned = clean_text(model_ref, 200);
    if cleaned.contains('/') {
        let mut parts = cleaned.splitn(2, '/');
        let provider = normalize_runtime_provider_id(parts.next().unwrap_or(""));
        let model = clean_text(parts.next().unwrap_or(""), 120);
        if !provider.is_empty() && !model.is_empty() {
            return (provider, model);
        }
    }
    let provider = if fallback_provider.is_empty() {
        "auto".to_string()
    } else {
        normalize_runtime_provider_id(fallback_provider)
    };
    let model = if cleaned.is_empty() {
        clean_text(fallback_model, 120)
    } else {
        cleaned
    };
    (provider, model)
}

fn parse_i64_loose(value: Option<&Value>) -> i64 {
    value
        .and_then(|row| {
            row.as_i64()
                .or_else(|| row.as_u64().map(|num| num as i64))
                .or_else(|| {
                    row.as_str()
                        .and_then(|text| clean_text(text, 40).parse::<i64>().ok())
                })
        })
        .unwrap_or(0)
        .max(0)
}
