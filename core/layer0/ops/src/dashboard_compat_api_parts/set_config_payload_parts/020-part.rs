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
    write_json_pretty(&path, state);
}

fn estimate_tokens(text: &str) -> i64 {
    ((clean_text(text, 20_000).chars().count() as i64) / 4).max(1)
}

fn active_session_row(state: &Value) -> Value {
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
        .cloned()
        .unwrap_or_default();
    if let Some(found) = rows.iter().find(|row| {
        row.get("session_id")
            .and_then(Value::as_str)
            .map(|v| clean_text(v, 120) == active_id)
            .unwrap_or(false)
    }) {
        return found.clone();
    }
    rows.first()
        .cloned()
        .unwrap_or_else(|| json!({"messages": []}))
}

fn session_messages(state: &Value) -> Vec<Value> {
    active_session_row(state)
        .get("messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
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
        .cloned()
        .unwrap_or_default()
        .into_iter()
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
    let cleaned = clean_text(model_ref, 200);
    if cleaned.contains('/') {
        let mut parts = cleaned.splitn(2, '/');
        let provider = clean_text(parts.next().unwrap_or(""), 80);
        let model = clean_text(parts.next().unwrap_or(""), 120);
        if !provider.is_empty() && !model.is_empty() {
            return (provider, model);
        }
    }
    let provider = if fallback_provider.is_empty() {
        "auto".to_string()
    } else {
        clean_text(fallback_provider, 80)
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

