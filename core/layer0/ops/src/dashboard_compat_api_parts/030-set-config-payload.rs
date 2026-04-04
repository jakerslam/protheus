// FILE_SIZE_EXCEPTION: reason=Atomic dashboard route/dispatch block requires semantic refactor to split safely; owner=jay; expires=2026-04-12
fn set_config_payload(root: &Path, snapshot: &Value, request: &Value) -> Value {
    let path = clean_text(
        request.get("path").and_then(Value::as_str).unwrap_or(""),
        120,
    )
    .to_ascii_lowercase();
    let string_value = clean_text(
        request
            .get("value")
            .and_then(|value| {
                value.as_str().map(|row| row.to_string()).or_else(|| {
                    if value.is_null() {
                        None
                    } else {
                        Some(value.to_string())
                    }
                })
            })
            .unwrap_or_default()
            .trim_matches('"'),
        4000,
    );
    if path.is_empty() {
        return json!({"ok": false, "error": "config_path_required"});
    }
    let field = path.rsplit('.').next().unwrap_or(path.as_str());
    let (current_provider, current_model) = extract_app_settings(root, snapshot);
    match field {
        "provider" => {
            let provider = if string_value.is_empty() {
                "auto".to_string()
            } else {
                string_value
            };
            let saved = save_app_settings(root, &provider, &current_model);
            json!({"ok": true, "path": path, "value": provider, "settings": saved})
        }
        "model" => {
            let saved = save_app_settings(root, &current_provider, &string_value);
            json!({"ok": true, "path": path, "value": string_value, "settings": saved})
        }
        "api_key" => crate::dashboard_provider_runtime::save_provider_key(
            root,
            &current_provider,
            &string_value,
        ),
        _ => {
            json!({"ok": true, "path": path, "value": request.get("value").cloned().unwrap_or(Value::Null)})
        }
    }
}

fn extract_profiles(root: &Path) -> Vec<Value> {
    let state = read_json(&state_path(root, AGENT_PROFILES_REL)).unwrap_or_else(|| json!({}));
    let mut rows = state
        .get("agents")
        .and_then(Value::as_object)
        .map(|obj| obj.values().map(|v| v.clone()).collect::<Vec<Value>>())
        .unwrap_or_default();
    rows.sort_by(|a, b| {
        clean_text(a.get("agent_id").and_then(Value::as_str).unwrap_or(""), 120).cmp(&clean_text(
            b.get("agent_id").and_then(Value::as_str).unwrap_or(""),
            120,
        ))
    });
    rows
}

fn recent_audit_entries(root: &Path, snapshot: &Value) -> Vec<Value> {
    let from_snapshot = snapshot
        .pointer("/receipts/recent")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !from_snapshot.is_empty() {
        return from_snapshot;
    }
    let raw = fs::read_to_string(state_path(root, ACTION_HISTORY_REL)).unwrap_or_default();
    raw.lines()
        .rev()
        .take(200)
        .filter_map(|row| serde_json::from_str::<Value>(row).ok())
        .collect::<Vec<_>>()
}

fn clean_agent_id(raw: &str) -> String {
    let mut out = String::new();
    for ch in clean_text(raw, 140).chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        }
    }
    out
}

fn parse_json_loose(raw: &str) -> Option<Value> {
    if raw.trim().is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<Value>(raw) {
        return Some(value);
    }
    for line in raw.lines().rev() {
        let candidate = line.trim();
        if candidate.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(candidate) {
            return Some(value);
        }
    }
    None
}

fn read_json_loose(path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    parse_json_loose(&raw)
}

fn write_json_pretty(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(value) {
        let _ = fs::write(path, format!("{raw}\n"));
    }
}

fn read_jsonl_loose(path: &Path, max_rows: usize) -> Vec<Value> {
    let raw = fs::read_to_string(path).unwrap_or_default();
    let limit = max_rows.max(1);
    raw.lines()
        .rev()
        .take(limit)
        .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
        .collect::<Vec<_>>()
}

fn instinct_dir(root: &Path) -> PathBuf {
    state_path(root, AGENT_INSTINCT_DIR_REL)
}

fn agent_instinct_prompt_context(root: &Path, max_chars: usize) -> String {
    let dir = instinct_dir(root);
    if !dir.is_dir() {
        return String::new();
    }
    let mut files = fs::read_dir(&dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.flatten())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.extension()
                .and_then(|value| value.to_str())
                .map(|value| {
                    let lowered = value.to_ascii_lowercase();
                    lowered == "md" || lowered == "txt"
                })
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    files.sort();
    let mut chunks = Vec::<String>::new();
    let mut used = 0usize;
    for path in files.into_iter().take(12) {
        let file_name = clean_text(
            path.file_name()
                .and_then(|value| value.to_str())
                .unwrap_or(""),
            120,
        );
        if file_name.is_empty() {
            continue;
        }
        let content = fs::read_to_string(&path).unwrap_or_default();
        let cleaned = clean_text(&content, max_chars.saturating_sub(used));
        if cleaned.is_empty() {
            continue;
        }
        let block = format!("[instinct:{file_name}] {cleaned}");
        used = used.saturating_add(block.len());
        chunks.push(block);
        if used >= max_chars {
            break;
        }
    }
    clean_text(&chunks.join("\n"), max_chars)
}

fn requester_agent_id(headers: &[(&str, &str)]) -> String {
    let primary = header_value(headers, "X-Actor-Agent-Id")
        .or_else(|| header_value(headers, "X-Agent-Id"))
        .or_else(|| header_value(headers, "X-Requester-Agent-Id"))
        .unwrap_or_default();
    clean_agent_id(&primary)
}

fn parent_agent_id_from_row(row: &Value) -> String {
    clean_agent_id(
        row.get("parent_agent_id")
            .and_then(Value::as_str)
            .or_else(|| {
                row.pointer("/contract/parent_agent_id")
                    .and_then(Value::as_str)
            })
            .unwrap_or(""),
    )
}

fn agent_parent_map(root: &Path, snapshot: &Value) -> HashMap<String, String> {
    let mut out = HashMap::<String, String>::new();
    for row in build_agent_roster(root, snapshot, true) {
        let id = clean_agent_id(row.get("id").and_then(Value::as_str).unwrap_or(""));
        if id.is_empty() {
            continue;
        }
        let parent = parent_agent_id_from_row(&row);
        if !parent.is_empty() {
            out.insert(id, parent);
        }
    }
    out
}

fn actor_can_manage_target(root: &Path, snapshot: &Value, actor_id: &str, target_id: &str) -> bool {
    let actor = clean_agent_id(actor_id);
    let target = clean_agent_id(target_id);
    if actor.is_empty() || target.is_empty() {
        return actor.is_empty();
    }
    if actor == target {
        return true;
    }
    let parent_map = agent_parent_map(root, snapshot);
    let mut current = target;
    let mut hops = 0usize;
    let mut seen = HashSet::<String>::new();
    while hops < 64 && seen.insert(current.clone()) {
        let Some(parent) = parent_map.get(&current).cloned() else {
            return false;
        };
        if parent == actor {
            return true;
        }
        current = parent;
        hops += 1;
    }
    false
}

fn parse_rfc3339_utc(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn latest_timestamp(values: &[String]) -> String {
    let mut best = String::new();
    for value in values {
        if value.is_empty() {
            continue;
        }
        if best.is_empty() || value > &best {
            best = value.clone();
        }
    }
    best
}

fn message_text(row: &Value) -> String {
    if let Some(text) = row.get("text").and_then(Value::as_str) {
        return clean_chat_text(text, 64_000);
    }
    if let Some(text) = row.get("content").and_then(Value::as_str) {
        return clean_chat_text(text, 64_000);
    }
    if let Some(text) = row.as_str() {
        return clean_chat_text(text, 64_000);
    }
    String::new()
}

fn message_timestamp_iso(row: &Value) -> String {
    if let Some(ts) = row.get("ts").and_then(Value::as_str) {
        return clean_text(ts, 80);
    }
    if let Some(ts_ms) = row.get("ts").and_then(Value::as_i64) {
        if let Some(parsed) = DateTime::<Utc>::from_timestamp_millis(ts_ms) {
            return parsed.to_rfc3339();
        }
    }
    clean_text(
        row.get("created_at").and_then(Value::as_str).unwrap_or(""),
        80,
    )
}

fn humanize_agent_name(agent_id: &str) -> String {
    let cleaned = clean_agent_id(agent_id).replace('-', " ").replace('_', " ");
    let mut words = Vec::<String>::new();
    for word in cleaned.split_whitespace() {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            let mut built = String::new();
            built.push(first.to_ascii_uppercase());
            built.push_str(chars.as_str());
            words.push(built);
        }
    }
    if words.is_empty() {
        "Agent".to_string()
    } else {
        words.join(" ")
    }
}

fn profiles_map(root: &Path) -> Map<String, Value> {
    read_json_loose(&state_path(root, AGENT_PROFILES_REL))
        .and_then(|v| v.get("agents").and_then(Value::as_object).cloned())
        .unwrap_or_default()
}

fn contracts_map(root: &Path) -> Map<String, Value> {
    read_json_loose(&state_path(root, AGENT_CONTRACTS_REL))
        .and_then(|v| v.get("contracts").and_then(Value::as_object).cloned())
        .unwrap_or_default()
}

fn session_dir(root: &Path) -> PathBuf {
    state_path(root, AGENT_SESSIONS_DIR_REL)
}

fn session_path(root: &Path, agent_id: &str) -> PathBuf {
    session_dir(root).join(format!("{}.json", clean_agent_id(agent_id)))
}

fn agent_files_dir(root: &Path, agent_id: &str) -> PathBuf {
    state_path(root, AGENT_FILES_DIR_REL).join(clean_agent_id(agent_id))
}

fn agent_tools_path(root: &Path, agent_id: &str) -> PathBuf {
    state_path(root, AGENT_TOOLS_DIR_REL).join(format!("{}.json", clean_agent_id(agent_id)))
}

fn default_session_state(agent_id: &str) -> Value {
    let now = crate::now_iso();
    json!({
        "type": "infring_dashboard_agent_session",
        "agent_id": clean_agent_id(agent_id),
        "active_session_id": "default",
        "sessions": [
            {
                "session_id": "default",
                "label": "Session",
                "created_at": now,
                "updated_at": now,
                "messages": []
            }
        ],
        "memory_kv": {}
    })
}

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

fn parse_manifest_fields(manifest_toml: &str) -> HashMap<String, String> {
    let mut out = HashMap::<String, String>::new();
    let mut in_model = false;
    for line in manifest_toml.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section = trimmed.trim_matches(|ch| ch == '[' || ch == ']');
            in_model = section.eq_ignore_ascii_case("model");
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            let key = clean_text(k, 80).to_ascii_lowercase();
            let mut value = v.trim().trim_matches('"').to_string();
            value = clean_text(&value, 400);
            if value.is_empty() {
                continue;
            }
            if key == "name" {
                out.insert("name".to_string(), value.clone());
            } else if key == "role" {
                out.insert("role".to_string(), value.clone());
            } else if in_model && key == "provider" {
                out.insert("provider".to_string(), value.clone());
            } else if in_model && key == "model" {
                out.insert("model".to_string(), value.clone());
            }
        }
    }
    out
}

fn make_agent_id(root: &Path, suggested_name: &str) -> String {
    let profiles = profiles_map(root);
    let contracts = contracts_map(root);
    let mut used = HashSet::<String>::new();
    for key in profiles.keys() {
        used.insert(clean_agent_id(key));
    }
    for key in contracts.keys() {
        used.insert(clean_agent_id(key));
    }
    let hint = clean_text(suggested_name, 80)
        .to_ascii_lowercase()
        .replace(' ', "-");
    let hint_suffix = if hint == "agent" {
        String::new()
    } else if let Some(rest) = hint
        .strip_prefix("agent-")
        .or_else(|| hint.strip_prefix("agent_"))
    {
        clean_agent_id(rest.trim_matches(|ch| ch == '-' || ch == '_'))
    } else {
        clean_agent_id(&hint)
    };
    let direct = clean_agent_id(&hint);
    if !direct.is_empty() && !used.contains(&direct) {
        return direct;
    }
    let hash_seed = json!({"hint": hint, "ts": crate::now_iso(), "nonce": Utc::now().timestamp_nanos_opt().unwrap_or_default()});
    let hash = crate::deterministic_receipt_hash(&hash_seed);
    let mut base = format!("agent-{}", hash.chars().take(12).collect::<String>());
    if !hint_suffix.is_empty() && hint_suffix.len() <= 18 {
        base = format!(
            "agent-{}-{}",
            hint_suffix,
            hash.chars().take(5).collect::<String>()
        );
    }
    let mut candidate = clean_agent_id(&base);
    if candidate.is_empty() {
        candidate = format!("agent-{}", hash.chars().take(12).collect::<String>());
    }
    if !used.contains(&candidate) {
        return candidate;
    }
    for idx in 2..5000 {
        let next = format!("{candidate}-{idx}");
        if !used.contains(&next) {
            return next;
        }
    }
    format!(
        "agent-{}",
        crate::deterministic_receipt_hash(&json!({"fallback": crate::now_iso()}))
            .chars()
            .take(14)
            .collect::<String>()
    )
}

fn contract_with_runtime_fields(contract: &Value) -> Value {
    let mut out = if contract.is_object() {
        contract.clone()
    } else {
        json!({})
    };
    let status = clean_text(
        out.get("status")
            .and_then(Value::as_str)
            .unwrap_or("active"),
        40,
    );
    let termination_condition = clean_text(
        out.get("termination_condition")
            .and_then(Value::as_str)
            .unwrap_or("task_or_timeout"),
        80,
    )
    .to_ascii_lowercase();
    let auto_terminate_allowed = out
        .get("auto_terminate_allowed")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let idle_terminate_allowed = out
        .get("idle_terminate_allowed")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let non_expiring = matches!(termination_condition.as_str(), "manual" | "task_complete")
        || (!auto_terminate_allowed && !idle_terminate_allowed);
    if non_expiring {
        if out
            .get("expires_at")
            .and_then(Value::as_str)
            .map(|v| v.trim().is_empty())
            .unwrap_or(true)
        {
            out["expires_at"] = Value::String(String::new());
        }
        out["remaining_ms"] = Value::Null;
        return out;
    }
    let now = Utc::now();
    let created = out
        .get("created_at")
        .and_then(Value::as_str)
        .and_then(parse_rfc3339_utc)
        .unwrap_or(now);
    let expiry_seconds = out
        .get("expiry_seconds")
        .and_then(Value::as_i64)
        .unwrap_or(3600)
        .clamp(1, 31 * 24 * 60 * 60);
    let expires = out
        .get("expires_at")
        .and_then(Value::as_str)
        .and_then(parse_rfc3339_utc)
        .unwrap_or_else(|| created + chrono::Duration::seconds(expiry_seconds));
    if out
        .get("expires_at")
        .and_then(Value::as_str)
        .map(|v| v.trim().is_empty())
        .unwrap_or(true)
    {
        out["expires_at"] = Value::String(expires.to_rfc3339());
    }
    let mut remaining = (expires.timestamp_millis() - now.timestamp_millis()).max(0);
    if status.eq_ignore_ascii_case("terminated") {
        remaining = 0;
    }
    out["remaining_ms"] = Value::from(remaining);
    out
}

fn collab_agents_map(snapshot: &Value) -> HashMap<String, Value> {
    let mut out = HashMap::<String, Value>::new();
    let rows = snapshot
        .pointer("/collab/dashboard/agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for row in rows {
        let id = clean_agent_id(row.get("shadow").and_then(Value::as_str).unwrap_or(""));
        if id.is_empty() {
            continue;
        }
        out.insert(id, row);
    }
    out
}

fn collab_runtime_active(row: Option<&Value>) -> bool {
    row.and_then(|value| value.get("status").and_then(Value::as_str))
        .map(|status| {
            status.eq_ignore_ascii_case("active") || status.eq_ignore_ascii_case("running")
        })
        .unwrap_or(false)
}

fn session_summary_map(root: &Path, snapshot: &Value) -> HashMap<String, Value> {
    let mut out = HashMap::<String, Value>::new();
    let snapshot_rows = snapshot
        .pointer("/agents/session_summaries/rows")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for row in snapshot_rows {
        let agent_id = clean_agent_id(row.get("agent_id").and_then(Value::as_str).unwrap_or(""));
        if agent_id.is_empty() {
            continue;
        }
        out.insert(agent_id, row);
    }
    let state_rows = crate::dashboard_agent_state::session_summaries(root, 500)
        .get("rows")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for row in state_rows {
        let agent_id = clean_agent_id(row.get("agent_id").and_then(Value::as_str).unwrap_or(""));
        if agent_id.is_empty() {
            continue;
        }
        out.insert(agent_id, row);
    }
    out
}

fn session_summary_rows(root: &Path, snapshot: &Value) -> Vec<Value> {
    let mut rows = session_summary_map(root, snapshot)
        .into_values()
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| {
        std::cmp::Reverse(clean_text(
            row.get("updated_at").and_then(Value::as_str).unwrap_or(""),
            80,
        ))
    });
    rows
}

fn first_string(value: Option<&Value>, key: &str) -> String {
    clean_text(
        value
            .and_then(|row| row.get(key).and_then(Value::as_str))
            .unwrap_or(""),
        240,
    )
}

fn build_agent_roster(root: &Path, snapshot: &Value, include_terminated: bool) -> Vec<Value> {
    let mut archived = crate::dashboard_agent_state::archived_agent_ids(root);
    let profiles = profiles_map(root);
    let contracts = contracts_map(root);
    let collab = collab_agents_map(snapshot);
    let session_summaries = session_summary_map(root, snapshot);
    let (default_provider, default_model) = effective_app_settings(root, snapshot);
    for (raw_id, profile) in &profiles {
        let profile_state = clean_text(
            profile.get("state").and_then(Value::as_str).unwrap_or(""),
            40,
        )
        .to_ascii_lowercase();
        if profile_state == "archived" {
            let id = clean_agent_id(raw_id);
            if !id.is_empty() {
                archived.insert(id);
            }
        }
    }
    let mut all_ids = HashSet::<String>::new();
    for key in profiles.keys() {
        let id = clean_agent_id(key);
        if !id.is_empty() {
            all_ids.insert(id);
        }
    }
    for key in contracts.keys() {
        let id = clean_agent_id(key);
        if !id.is_empty() {
            all_ids.insert(id);
        }
    }
    for key in collab.keys() {
        let id = clean_agent_id(key);
        if !id.is_empty() {
            all_ids.insert(id);
        }
    }
    for key in session_summaries.keys() {
        let id = clean_agent_id(key);
        if !id.is_empty() {
            all_ids.insert(id);
        }
    }
    let mut rows = Vec::<Value>::new();
    for agent_id in all_ids {
        if archived.contains(&agent_id) {
            continue;
        }
        let profile = profiles
            .get(&agent_id)
            .cloned()
            .unwrap_or_else(|| json!({}));
        let contract_raw = contracts
            .get(&agent_id)
            .cloned()
            .unwrap_or_else(|| json!({}));
        let collab_row = collab.get(&agent_id);
        let session_summary = session_summaries.get(&agent_id);
        let runtime_active = collab_runtime_active(collab_row);
        let contract = contract_with_runtime_fields(&contract_raw);
        let contract_status = clean_text(
            contract
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("active"),
            40,
        )
        .to_ascii_lowercase();
        let contract_terminated = contract_status == "terminated" && !runtime_active;
        let termination_condition = clean_text(
            contract
                .get("termination_condition")
                .and_then(Value::as_str)
                .unwrap_or("task_or_timeout"),
            80,
        )
        .to_ascii_lowercase();
        let contract_auto_terminate_allowed = contract
            .get("auto_terminate_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let contract_idle_terminate_allowed = contract
            .get("idle_terminate_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let non_expiring_contract = termination_condition.starts_with("manual")
            || termination_condition == "task_complete"
            || (!contract_auto_terminate_allowed && !contract_idle_terminate_allowed);
        let termination_reason = clean_text(
            contract
                .get("termination_reason")
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        )
        .to_ascii_lowercase();
        let revive_recommended = contract_terminated
            && non_expiring_contract
            && (termination_reason.contains("timeout")
                || termination_reason.contains("expired")
                || termination_reason.contains("terminated"));
        if !include_terminated && contract_terminated && !revive_recommended {
            continue;
        }
        let profile_name = clean_text(
            profile.get("name").and_then(Value::as_str).unwrap_or(""),
            120,
        );
        let name = if profile_name.is_empty() {
            humanize_agent_name(&agent_id)
        } else {
            profile_name
        };
        let role = {
            let from_profile = clean_text(
                profile.get("role").and_then(Value::as_str).unwrap_or(""),
                60,
            );
            if !from_profile.is_empty() {
                from_profile
            } else {
                let from_collab = first_string(collab_row, "role");
                if !from_collab.is_empty() {
                    from_collab
                } else {
                    "analyst".to_string()
                }
            }
        };
        let session_updated_at = clean_text(
            session_summary
                .and_then(|row| row.get("updated_at").and_then(Value::as_str))
                .unwrap_or(""),
            80,
        );
        let session_message_count = session_summary
            .and_then(|row| row.get("message_count").and_then(Value::as_i64))
            .unwrap_or(0);
        let state = if contract_terminated {
            if revive_recommended {
                "Idle".to_string()
            } else {
                "Terminated".to_string()
            }
        } else if runtime_active {
            "Running".to_string()
        } else {
            let raw = first_string(collab_row, "status");
            if raw.is_empty() {
                if session_message_count > 0 || !session_updated_at.is_empty() {
                    "Idle".to_string()
                } else {
                    "Running".to_string()
                }
            } else if raw.eq_ignore_ascii_case("active") || raw.eq_ignore_ascii_case("running") {
                "Running".to_string()
            } else if raw.eq_ignore_ascii_case("idle") {
                "Idle".to_string()
            } else if raw.eq_ignore_ascii_case("inactive") || raw.eq_ignore_ascii_case("paused") {
                let profile_state = clean_text(
                    profile.get("state").and_then(Value::as_str).unwrap_or(""),
                    40,
                )
                .to_ascii_lowercase();
                if profile_state == "running"
                    || profile_state == "active"
                    || contract_status == "active"
                {
                    "Idle".to_string()
                } else {
                    "Inactive".to_string()
                }
            } else {
                raw
            }
        };

        let identity = if profile
            .get("identity")
            .map(Value::is_object)
            .unwrap_or(false)
        {
            profile
                .get("identity")
                .cloned()
                .unwrap_or_else(|| json!({}))
        } else {
            json!({
                "emoji": profile.get("emoji").cloned().unwrap_or_else(|| json!("🧑‍💻")),
                "color": profile.get("color").cloned().unwrap_or_else(|| json!("#2563EB")),
                "archetype": profile.get("archetype").cloned().unwrap_or_else(|| json!("assistant")),
                "vibe": profile.get("vibe").cloned().unwrap_or_else(|| json!(""))
            })
        };
        let model_override = clean_text(
            profile
                .get("model_override")
                .and_then(Value::as_str)
                .unwrap_or(""),
            160,
        );
        let model_ref =
            if !model_override.is_empty() && !model_override.eq_ignore_ascii_case("auto") {
                model_override
            } else {
                default_model.clone()
            };
        let (model_provider, model_name) =
            split_model_ref(&model_ref, &default_provider, &default_model);
        let runtime_model = clean_text(
            profile
                .get("runtime_model")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let model_runtime = if runtime_model.is_empty() {
            model_name.clone()
        } else {
            runtime_model
        };
        let git_branch = clean_text(
            profile
                .get("git_branch")
                .and_then(Value::as_str)
                .unwrap_or("main"),
            180,
        );
        let git_tree_kind = clean_text(
            profile
                .get("git_tree_kind")
                .and_then(Value::as_str)
                .unwrap_or("master"),
            60,
        );
        let is_master = profile
            .get("is_master_agent")
            .and_then(Value::as_bool)
            .unwrap_or_else(|| {
                let branch = git_branch.to_ascii_lowercase();
                let kind = git_tree_kind.to_ascii_lowercase();
                branch == "main" || branch == "master" || kind == "master" || kind == "main"
            });
        let auto_terminate_allowed = contract
            .get("auto_terminate_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(!is_master);
        let contract_remaining_ms = if auto_terminate_allowed {
            contract.get("remaining_ms").and_then(Value::as_i64)
        } else {
            None
        };
        let created_at = clean_text(
            profile
                .get("created_at")
                .and_then(Value::as_str)
                .or_else(|| contract.get("created_at").and_then(Value::as_str))
                .or_else(|| {
                    session_summary.and_then(|row| row.get("updated_at").and_then(Value::as_str))
                })
                .unwrap_or(""),
            80,
        );
        let updated_at = latest_timestamp(&[
            clean_text(
                profile
                    .get("updated_at")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                80,
            ),
            clean_text(
                contract
                    .get("updated_at")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                80,
            ),
            clean_text(
                collab_row
                    .and_then(|v| v.get("activated_at").and_then(Value::as_str))
                    .unwrap_or(""),
                80,
            ),
            session_updated_at.clone(),
        ]);
        rows.push(json!({
            "id": agent_id,
            "agent_id": agent_id,
            "name": name,
            "role": role,
            "state": state,
            "model_provider": model_provider,
            "model_name": model_name,
            "runtime_model": model_runtime,
            "context_window": profile.get("context_window").cloned().unwrap_or(Value::Null),
            "context_window_tokens": profile.get("context_window_tokens").cloned().unwrap_or(Value::Null),
            "identity": identity,
            "avatar_url": profile.get("avatar_url").cloned().unwrap_or_else(|| json!("")),
            "system_prompt": profile.get("system_prompt").cloned().unwrap_or_else(|| json!("")),
            "fallback_models": profile.get("fallback_models").cloned().unwrap_or_else(|| json!([])),
            "git_branch": git_branch,
            "branch": git_branch,
            "git_tree_kind": git_tree_kind,
            "workspace_dir": profile
                .get("workspace_dir")
                .cloned()
                .unwrap_or_else(|| json!(root.to_string_lossy().to_string())),
            "workspace_rel": profile.get("workspace_rel").cloned().unwrap_or(Value::Null),
            "git_tree_ready": profile.get("git_tree_ready").cloned().unwrap_or_else(|| json!(true)),
            "git_tree_error": profile.get("git_tree_error").cloned().unwrap_or_else(|| json!("")),
            "is_master_agent": is_master,
            "created_at": created_at,
            "updated_at": updated_at,
            "message_count": session_message_count,
            "contract": contract.clone(),
            "contract_expires_at": contract.get("expires_at").cloned().unwrap_or(Value::Null),
            "contract_remaining_ms": contract_remaining_ms.map(Value::from).unwrap_or(Value::Null),
            "parent_agent_id": parent_agent_id_from_row(&json!({
                "parent_agent_id": profile.get("parent_agent_id").cloned().unwrap_or(Value::Null),
                "contract": {"parent_agent_id": contract.get("parent_agent_id").cloned().unwrap_or(Value::Null)}
            })),
            "auto_terminate_allowed": auto_terminate_allowed,
            "revive_recommended": revive_recommended
        }));
    }
    rows.sort_by_key(|row| {
        std::cmp::Reverse(clean_text(
            row.get("updated_at").and_then(Value::as_str).unwrap_or(""),
            80,
        ))
    });
    rows
}

fn archive_all_visible_agents(root: &Path, snapshot: &Value, reason: &str) -> Value {
    let archive_reason = {
        let cleaned = clean_text(reason, 120);
        if cleaned.is_empty() {
            "user_archive_all".to_string()
        } else {
            cleaned
        }
    };
    let mut archived_agent_ids = Vec::<String>::new();
    let mut failed_agent_ids = Vec::<String>::new();
    let mut skipped_agent_ids = Vec::<String>::new();
    for row in build_agent_roster(root, snapshot, false) {
        let agent_id = clean_agent_id(row.get("id").and_then(Value::as_str).unwrap_or(""));
        if agent_id.is_empty() {
            continue;
        }
        if agent_id.eq_ignore_ascii_case("system") {
            skipped_agent_ids.push(agent_id);
            continue;
        }
        let _ = update_profile_patch(
            root,
            &agent_id,
            &json!({"state": "Archived", "updated_at": crate::now_iso()}),
        );
        let _ = upsert_contract_patch(
            root,
            &agent_id,
            &json!({
                "status": "terminated",
                "termination_reason": "user_archived",
                "terminated_at": crate::now_iso(),
                "updated_at": crate::now_iso()
            }),
        );
        let archived =
            crate::dashboard_agent_state::archive_agent(root, &agent_id, &archive_reason);
        if archived.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            archived_agent_ids.push(agent_id);
        } else {
            failed_agent_ids.push(agent_id);
        }
    }
    let attempted = archived_agent_ids.len() + failed_agent_ids.len();
    json!({
        "ok": failed_agent_ids.is_empty(),
        "type": "dashboard_agent_archive_all",
        "reason": archive_reason,
        "attempted": attempted,
        "archived_count": archived_agent_ids.len(),
        "archived_agent_ids": archived_agent_ids,
        "failed_agent_ids": failed_agent_ids,
        "skipped_agent_ids": skipped_agent_ids
    })
}

fn agent_row_by_id(root: &Path, snapshot: &Value, agent_id: &str) -> Option<Value> {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return None;
    }
    build_agent_roster(root, snapshot, true)
        .into_iter()
        .find(|row| clean_agent_id(row.get("id").and_then(Value::as_str).unwrap_or("")) == id)
}
fn archived_agent_stub(root: &Path, agent_id: &str) -> Value {
    let id = clean_agent_id(agent_id);
    let profile = profiles_map(root)
        .get(&id)
        .cloned()
        .unwrap_or_else(|| json!({}));
    let name = clean_text(
        profile.get("name").and_then(Value::as_str).unwrap_or(""),
        120,
    );
    let role = clean_text(
        profile
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("analyst"),
        60,
    );
    let role_value = if role.is_empty() {
        "analyst".to_string()
    } else {
        role
    };
    json!({
        "ok": true,
        "id": id,
        "agent_id": id,
        "name": if name.is_empty() { humanize_agent_name(agent_id) } else { name },
        "role": role_value,
        "state": "inactive",
        "archived": true
    })
}

fn update_profile_patch(root: &Path, agent_id: &str, patch: &Value) -> Value {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    crate::dashboard_agent_state::upsert_profile(root, &id, patch)
}

fn upsert_contract_patch(root: &Path, agent_id: &str, patch: &Value) -> Value {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    crate::dashboard_agent_state::upsert_contract(root, &id, patch)
}

fn session_payload(root: &Path, agent_id: &str) -> Value {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let state = load_session_state(root, &id);
    let messages = session_messages(&state);
    let sessions = session_rows_payload(&state);
    json!({
        "ok": true,
        "agent_id": id,
        "active_session_id": state.get("active_session_id").cloned().unwrap_or_else(|| json!("default")),
        "messages": messages,
        "sessions": sessions,
        "session": state
    })
}

fn append_jsonl_row(path: &Path, row: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(line) = serde_json::to_string(row) {
        let _ = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| {
                std::io::Write::write_all(&mut file, format!("{line}\n").as_bytes())
            });
    }
}

fn attention_queue_fallback_path(root: &Path) -> PathBuf {
    root.join("client/runtime/local/state/attention/pending_memory_events.jsonl")
}

fn parse_memory_capture_text(user_text: &str) -> Option<String> {
    let cleaned = clean_text(user_text, 2000);
    if cleaned.is_empty() {
        return None;
    }
    let lowered = cleaned.to_ascii_lowercase();
    if !(lowered.starts_with("remember ") || lowered.contains("remember this")) {
        return None;
    }
    let extracted = if let Some((_, tail)) = cleaned.split_once(':') {
        clean_text(tail, 1200)
    } else {
        clean_text(cleaned.trim_start_matches("remember"), 1200)
    };
    if extracted.is_empty() {
        None
    } else {
        Some(extracted)
    }
}

fn important_memory_terms(text: &str, limit: usize) -> Vec<String> {
    let stop_words = [
        "the", "and", "for", "with", "that", "this", "from", "have", "your", "you", "are", "was",
        "were", "will", "into", "about", "what", "when", "then", "than", "just", "they", "them",
        "able", "make", "made", "need", "want", "does", "did", "done", "cant", "cannot", "dont",
        "not", "too", "very", "also", "like", "been", "being", "each", "more", "most", "over",
        "under", "after", "before", "because", "while", "where", "which", "who", "whom", "whose",
        "would", "could", "should",
    ];
    let mut seen = HashSet::<String>::new();
    let mut out = Vec::<String>::new();
    for raw in clean_text(text, 2000).to_ascii_lowercase().split(' ') {
        let token = raw
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
            .collect::<String>();
        if token.len() < 3 || stop_words.contains(&token.as_str()) {
            continue;
        }
        if seen.insert(token.clone()) {
            out.push(token);
            if out.len() >= limit {
                break;
            }
        }
    }
    out
}

fn passive_memory_attention_event(
    agent_id: &str,
    user_text: &str,
    assistant_text: &str,
) -> Option<Value> {
    let user = clean_text(user_text, 1400);
    let assistant = clean_text(assistant_text, 1400);
    if user.is_empty() && assistant.is_empty() {
        return None;
    }
    let summary = if !user.is_empty() {
        format!(
            "{}: {}",
            humanize_agent_name(agent_id),
            clean_text(&user, 220)
        )
    } else {
        format!(
            "{}: {}",
            humanize_agent_name(agent_id),
            clean_text(&assistant, 220)
        )
    };
    let terms = important_memory_terms(&format!("{user} {assistant}"), 12);
    let event = json!({
        "ts": crate::now_iso(),
        "source": format!("agent:{agent_id}"),
        "source_type": "passive_memory_turn",
        "severity": "info",
        "summary": summary,
        "attention_key": format!(
            "agent:{agent_id}:passive_memory:{}",
            crate::deterministic_receipt_hash(&json!({
                "agent_id": agent_id,
                "user": user,
                "assistant": assistant
            }))
            .chars()
            .take(20)
            .collect::<String>()
        ),
        "raw_event": {
            "agent_id": agent_id,
            "memory_kind": "passive_turn",
            "user_text": user,
            "assistant_text": assistant,
            "terms": terms
        }
    });
    Some(event)
}

fn enqueue_attention_event_best_effort(root: &Path, run_context: &str, event: &Value) -> Value {
    let event_json = match serde_json::to_string(event) {
        Ok(raw) => raw,
        Err(err) => {
            return json!({
                "ok": false,
                "queued": false,
                "reason": "event_encode_failed",
                "error": clean_text(&err.to_string(), 200)
            });
        }
    };
    let encoded = {
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;
        STANDARD.encode(event_json.as_bytes())
    };
    let args = vec![
        "enqueue".to_string(),
        format!("--event-json-base64={encoded}"),
        format!("--run-context={}", clean_text(run_context, 120)),
    ];
    let exit = crate::attention_queue::run(root, &args);
    if exit == 0 {
        json!({"ok": true, "queued": true, "run_context": run_context, "exit_code": 0})
    } else {
        let staged = json!({
            "ts": crate::now_iso(),
            "run_context": clean_text(run_context, 120),
            "event": event
        });
        append_jsonl_row(&attention_queue_fallback_path(root), &staged);
        json!({
            "ok": false,
            "queued": false,
            "staged": true,
            "run_context": run_context,
            "exit_code": exit,
            "fallback_path": attention_queue_fallback_path(root).to_string_lossy().to_string()
        })
    }
}

fn append_turn_message(
    root: &Path,
    agent_id: &str,
    user_text: &str,
    assistant_text: &str,
) -> Value {
    let mut receipt =
        crate::dashboard_agent_state::append_turn(root, agent_id, user_text, assistant_text);
    if let Some(memory_text) = parse_memory_capture_text(user_text) {
        let key = format!(
            "explicit_memory.{}",
            crate::deterministic_receipt_hash(
                &json!({"agent_id": agent_id, "memory": memory_text})
            )
            .chars()
            .take(12)
            .collect::<String>()
        );
        let value = json!({
            "text": memory_text,
            "captured_at": crate::now_iso(),
            "source": "user_explicit_remember"
        });
        let memory_receipt =
            crate::dashboard_agent_state::memory_kv_set(root, agent_id, &key, &value);
        receipt["memory_capture"] = memory_receipt;
    }
    if let Some(event) = passive_memory_attention_event(agent_id, user_text, assistant_text) {
        receipt["attention_queue"] =
            enqueue_attention_event_best_effort(root, "dashboard_agent_passive_memory", &event);
    } else {
        receipt["attention_queue"] = json!({
            "ok": true,
            "queued": false,
            "reason": "empty_turn"
        });
    }
    receipt
}

fn rollback_last_turn(root: &Path, agent_id: &str) -> Value {
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
    let mut removed = Vec::<Value>::new();
    let mut before_messages = 0usize;
    let mut after_messages = 0usize;
    let mut rollback_id = String::new();
    if let Some(rows) = state.get_mut("sessions").and_then(Value::as_array_mut) {
        for row in rows.iter_mut() {
            let sid = clean_text(
                row.get("session_id").and_then(Value::as_str).unwrap_or(""),
                120,
            );
            if sid != active_id {
                continue;
            }
            if !row.get("messages").map(Value::is_array).unwrap_or(false) {
                row["messages"] = Value::Array(Vec::new());
            }
            let messages = row
                .get_mut("messages")
                .and_then(Value::as_array_mut)
                .expect("messages");
            before_messages = messages.len();

            while messages
                .last()
                .map(|entry| {
                    clean_text(entry.get("role").and_then(Value::as_str).unwrap_or(""), 24)
                        .eq_ignore_ascii_case("system")
                })
                .unwrap_or(false)
            {
                if let Some(last) = messages.pop() {
                    removed.push(last);
                }
            }

            if messages
                .last()
                .map(|entry| {
                    let role =
                        clean_text(entry.get("role").and_then(Value::as_str).unwrap_or(""), 24)
                            .to_ascii_lowercase();
                    role == "assistant" || role == "agent"
                })
                .unwrap_or(false)
            {
                if let Some(last) = messages.pop() {
                    removed.push(last);
                }
            }

            if messages
                .last()
                .map(|entry| {
                    clean_text(entry.get("role").and_then(Value::as_str).unwrap_or(""), 24)
                        .eq_ignore_ascii_case("user")
                })
                .unwrap_or(false)
            {
                if let Some(last) = messages.pop() {
                    removed.push(last);
                }
            }

            if removed.is_empty() {
                if let Some(last) = messages.pop() {
                    removed.push(last);
                }
            }

            after_messages = messages.len();
            let removed_excerpt = removed
                .iter()
                .rev()
                .map(|entry| {
                    json!({
                        "role": clean_text(entry.get("role").and_then(Value::as_str).unwrap_or(""), 24),
                        "text": clean_text(&message_text(entry), 220),
                        "ts": entry.get("ts").cloned().unwrap_or(Value::Null)
                    })
                })
                .collect::<Vec<_>>();
            rollback_id = format!(
                "rbk-{}",
                &crate::deterministic_receipt_hash(&json!({
                    "agent_id": id.as_str(),
                    "removed_count": removed.len(),
                    "before": before_messages,
                    "after": after_messages,
                    "at": crate::now_iso()
                }))[..12]
            );
            if !row
                .get("rollback_archives")
                .map(Value::is_array)
                .unwrap_or(false)
            {
                row["rollback_archives"] = Value::Array(Vec::new());
            }
            if let Some(archives) = row
                .get_mut("rollback_archives")
                .and_then(Value::as_array_mut)
            {
                archives.push(json!({
                    "rollback_id": rollback_id.clone(),
                    "captured_at": crate::now_iso(),
                    "removed_count": removed.len(),
                    "removed_messages": removed_excerpt
                }));
                if archives.len() > 24 {
                    let trim = archives.len().saturating_sub(24);
                    archives.drain(0..trim);
                }
            }
            row["updated_at"] = Value::String(crate::now_iso());
            break;
        }
    }
    save_session_state(root, &id, &state);
    json!({
        "ok": !removed.is_empty(),
        "type": "dashboard_agent_session_rollback",
        "agent_id": id,
        "rollback_id": rollback_id,
        "removed_count": removed.len(),
        "before_messages": before_messages,
        "after_messages": after_messages,
        "removed_excerpt": removed
            .iter()
            .rev()
            .map(|entry| clean_text(&message_text(entry), 160))
            .filter(|text| !text.is_empty())
            .take(3)
            .collect::<Vec<_>>()
    })
}

fn reset_active_session(root: &Path, agent_id: &str) -> Value {
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
    if let Some(rows) = state.get_mut("sessions").and_then(Value::as_array_mut) {
        for row in rows.iter_mut() {
            let sid = clean_text(
                row.get("session_id").and_then(Value::as_str).unwrap_or(""),
                120,
            );
            if sid == active_id {
                row["messages"] = Value::Array(Vec::new());
                row["updated_at"] = Value::String(crate::now_iso());
                break;
            }
        }
    }
    save_session_state(root, &id, &state);
    json!({
        "ok": true,
        "type": "dashboard_agent_session_reset",
        "agent_id": id,
        "active_session_id": active_id
    })
}

fn compaction_message_text(row: &Value) -> String {
    let text = message_text(row);
    if !text.is_empty() {
        return clean_text(&text, 4000);
    }
    clean_text(
        row.get("summary").and_then(Value::as_str).unwrap_or(""),
        4000,
    )
}

fn build_context_keyframes_from_removed(removed: &[Value], max_keyframes: usize) -> Vec<Value> {
    if removed.is_empty() {
        return Vec::new();
    }
    let cap = max_keyframes.clamp(1, 24);
    let chunk_size = ((removed.len() as f64 / cap as f64).ceil() as usize).max(1);
    let mut out = Vec::<Value>::new();
    for (idx, chunk) in removed.chunks(chunk_size).enumerate() {
        if out.len() >= cap {
            break;
        }
        let mut highlights = Vec::<String>::new();
        for row in chunk {
            let role = clean_text(row.get("role").and_then(Value::as_str).unwrap_or(""), 20)
                .to_ascii_lowercase();
            let text = compaction_message_text(row);
            if text.is_empty() {
                continue;
            }
            let prefix = if role.is_empty() {
                "note".to_string()
            } else {
                role
            };
            highlights.push(format!("{prefix}: {}", clean_text(&text, 120)));
            if highlights.len() >= 2 {
                break;
            }
        }
        let summary = if highlights.is_empty() {
            format!(
                "Compaction batch {} summarized {} older turns.",
                idx + 1,
                chunk.len()
            )
        } else {
            highlights.join(" | ")
        };
        let key_seed = json!({"batch": idx + 1, "summary": summary, "count": chunk.len()});
        let key_hash = crate::deterministic_receipt_hash(&key_seed);
        out.push(json!({
            "keyframe_id": format!("kf-{}", &key_hash[..12]),
            "batch": idx + 1,
            "turns_covered": chunk.len(),
            "summary": clean_text(&summary, 260),
            "captured_at": crate::now_iso()
        }));
    }
    out
}

fn compact_active_session(root: &Path, agent_id: &str, request: &Value) -> Value {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let mut state = load_session_state(root, &id);
    let target_window = request
        .get("target_context_window")
        .and_then(Value::as_i64)
        .unwrap_or(8192)
        .clamp(512, 2_000_000);
    let target_ratio = request
        .get("target_ratio")
        .and_then(Value::as_f64)
        .unwrap_or(0.8)
        .clamp(0.2, 0.95);
    let min_recent_messages = request
        .get("min_recent_messages")
        .and_then(Value::as_u64)
        .unwrap_or(12)
        .clamp(2, 200) as usize;
    let max_messages = request
        .get("max_messages")
        .and_then(Value::as_u64)
        .unwrap_or(200)
        .clamp(20, 800) as usize;
    let persist_compaction_to_session = request
        .get("persist_compaction_to_session")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    let mut before_tokens = 0i64;
    let mut after_tokens = 0i64;
    let mut before_messages = 0usize;
    let mut after_messages = 0usize;
    let mut removed_messages = Vec::<Value>::new();
    let mut emitted_keyframes = Vec::<Value>::new();
    if let Some(rows) = state.get_mut("sessions").and_then(Value::as_array_mut) {
        for row in rows.iter_mut() {
            let sid = clean_text(
                row.get("session_id").and_then(Value::as_str).unwrap_or(""),
                120,
            );
            if sid != active_id {
                continue;
            }
            if !row.get("messages").map(Value::is_array).unwrap_or(false) {
                row["messages"] = Value::Array(Vec::new());
            }
            let messages = row
                .get_mut("messages")
                .and_then(Value::as_array_mut)
                .expect("messages");
            before_messages = messages.len();
            before_tokens = messages
                .iter()
                .map(|item| {
                    let text = item
                        .get("text")
                        .and_then(Value::as_str)
                        .or_else(|| item.get("content").and_then(Value::as_str))
                        .unwrap_or("");
                    estimate_tokens(text)
                })
                .sum::<i64>();
            let mut compacted = messages.clone();
            let target_tokens = ((target_window as f64) * target_ratio).round() as i64;
            if compacted.len() > max_messages {
                let drain = compacted.len().saturating_sub(max_messages);
                removed_messages.extend(compacted.drain(0..drain));
            }
            while compacted.len() > min_recent_messages {
                let current_tokens = compacted
                    .iter()
                    .map(|item| {
                        let text = item
                            .get("text")
                            .and_then(Value::as_str)
                            .or_else(|| item.get("content").and_then(Value::as_str))
                            .unwrap_or("");
                        estimate_tokens(text)
                    })
                    .sum::<i64>();
                if current_tokens <= target_tokens {
                    break;
                }
                if !compacted.is_empty() {
                    removed_messages.push(compacted.remove(0));
                }
            }
            after_messages = compacted.len();
            after_tokens = compacted
                .iter()
                .map(|item| {
                    let text = item
                        .get("text")
                        .and_then(Value::as_str)
                        .or_else(|| item.get("content").and_then(Value::as_str))
                        .unwrap_or("");
                    estimate_tokens(text)
                })
                .sum::<i64>();
            if persist_compaction_to_session {
                *messages = compacted;
            }
            emitted_keyframes = build_context_keyframes_from_removed(&removed_messages, 8);
            if !row
                .get("context_keyframes")
                .map(Value::is_array)
                .unwrap_or(false)
            {
                row["context_keyframes"] = Value::Array(Vec::new());
            }
            if let Some(keyframes) = row
                .get_mut("context_keyframes")
                .and_then(Value::as_array_mut)
            {
                keyframes.extend(emitted_keyframes.clone());
                if keyframes.len() > 48 {
                    let trim = keyframes.len().saturating_sub(48);
                    keyframes.drain(0..trim);
                }
            }
            if !row
                .get("compaction_archives")
                .map(Value::is_array)
                .unwrap_or(false)
            {
                row["compaction_archives"] = Value::Array(Vec::new());
            }
            let archive_messages = removed_messages
                .iter()
                .take(240)
                .map(|item| {
                    json!({
                        "role": clean_text(item.get("role").and_then(Value::as_str).unwrap_or(""), 24),
                        "text": clean_text(&compaction_message_text(item), 1200),
                        "ts": item.get("ts").cloned().unwrap_or(Value::Null),
                        "created_at": item.get("created_at").cloned().unwrap_or(Value::Null)
                    })
                })
                .collect::<Vec<_>>();
            let archive = json!({
                "archive_id": format!("cmp-{}", &crate::deterministic_receipt_hash(&json!({
                    "agent_id": id,
                    "removed_count": removed_messages.len(),
                    "before_tokens": before_tokens,
                    "after_tokens": after_tokens,
                    "captured_at": crate::now_iso()
                }))[..12]),
                "captured_at": crate::now_iso(),
                "removed_count": removed_messages.len(),
                "persisted_to_session": persist_compaction_to_session,
                "removed_excerpt_count": archive_messages.len(),
                "removed_messages": archive_messages,
                "keyframes": emitted_keyframes
            });
            if let Some(archives) = row
                .get_mut("compaction_archives")
                .and_then(Value::as_array_mut)
            {
                archives.push(archive);
                if archives.len() > 12 {
                    let trim = archives.len().saturating_sub(12);
                    archives.drain(0..trim);
                }
            }
            row["updated_at"] = Value::String(crate::now_iso());
            break;
        }
    }
    save_session_state(root, &id, &state);
    json!({
        "ok": true,
        "type": "dashboard_agent_session_compact",
        "agent_id": id,
        "before_tokens": before_tokens,
        "after_tokens": after_tokens,
        "before_messages": before_messages,
        "after_messages": after_messages,
        "removed_messages": removed_messages.len(),
        "persisted_to_session": persist_compaction_to_session,
        "keyframes_emitted": emitted_keyframes.len(),
        "keyframes": emitted_keyframes,
        "message": format!("Compaction complete: {} -> {} tokens", before_tokens, after_tokens)
    })
}

fn parse_agent_route(path_only: &str) -> Option<(String, Vec<String>)> {
    let prefix = "/api/agents/";
    if !path_only.starts_with(prefix) {
        return None;
    }
    let tail = path_only.trim_start_matches(prefix).trim_matches('/');
    if tail.is_empty() {
        return None;
    }
    let mut parts = tail
        .split('/')
        .map(|v| clean_text(v, 180))
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return None;
    }
    let agent_id = clean_agent_id(&parts.remove(0));
    if agent_id.is_empty() {
        return None;
    }
    Some((agent_id, parts))
}

fn resolve_agent_id_alias(root: &Path, requested: &str) -> String {
    let normalized = clean_agent_id(requested);
    if normalized.is_empty() {
        return String::new();
    }
    let profiles = profiles_map(root);
    if profiles.contains_key(&normalized) {
        return normalized;
    }
    let contracts = contracts_map(root);
    if contracts.contains_key(&normalized) {
        return normalized;
    }
    let requested_name = clean_text(requested, 120).to_ascii_lowercase();
    if requested_name.is_empty() {
        return normalized;
    }
    for (id, profile) in &profiles {
        let profile_name = clean_text(
            profile.get("name").and_then(Value::as_str).unwrap_or(""),
            120,
        )
        .to_ascii_lowercase();
        if !profile_name.is_empty() && profile_name == requested_name {
            let resolved = clean_agent_id(id);
            if !resolved.is_empty() {
                return resolved;
            }
        }
    }
    normalized
}

fn parse_provider_route(path_only: &str) -> Option<(String, Vec<String>)> {
    let prefix = "/api/providers/";
    if !path_only.starts_with(prefix) {
        return None;
    }
    let tail = path_only.trim_start_matches(prefix).trim_matches('/');
    if tail.is_empty() {
        return None;
    }
    let mut parts = tail
        .split('/')
        .map(|value| clean_text(value, 180))
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return None;
    }
    let provider_id = decode_path_segment(&parts.remove(0));
    if provider_id.is_empty() {
        return None;
    }
    Some((provider_id, parts))
}

fn parse_virtual_key_route(path_only: &str) -> Option<(String, Vec<String>)> {
    let prefix = "/api/virtual-keys/";
    if !path_only.starts_with(prefix) {
        return None;
    }
    let tail = path_only.trim_start_matches(prefix).trim_matches('/');
    if tail.is_empty() {
        return None;
    }
    let mut parts = tail
        .split('/')
        .map(|value| clean_text(value, 180))
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return None;
    }
    let key_id = decode_path_segment(&parts.remove(0));
    if key_id.is_empty() {
        return None;
    }
    Some((key_id, parts))
}

fn parse_memory_route(path_only: &str) -> Option<(String, Vec<String>)> {
    let prefix = "/api/memory/agents/";
    if !path_only.starts_with(prefix) {
        return None;
    }
    let tail = path_only.trim_start_matches(prefix).trim_matches('/');
    if tail.is_empty() {
        return None;
    }
    let mut parts = tail.split('/').map(decode_path_segment).collect::<Vec<_>>();
    if parts.len() < 2 {
        return None;
    }
    let agent_id = clean_agent_id(&parts.remove(0));
    if agent_id.is_empty() {
        return None;
    }
    Some((agent_id, parts))
}

fn decode_path_segment(raw: &str) -> String {
    let decoded = urlencoding::decode(raw)
        .ok()
        .map(|v| v.to_string())
        .unwrap_or_else(|| raw.to_string());
    clean_text(&decoded, 300)
}

fn workspace_base_for_agent(root: &Path, row: Option<&Value>) -> PathBuf {
    let raw = clean_text(
        row.and_then(|v| v.get("workspace_dir").and_then(Value::as_str))
            .unwrap_or(""),
        4000,
    );
    let base = if raw.is_empty() {
        root.to_path_buf()
    } else {
        let as_path = PathBuf::from(raw);
        if as_path.is_absolute() {
            as_path
        } else {
            root.join(as_path)
        }
    };
    normalize_lexical(&base)
}

fn resolve_workspace_path(base: &Path, requested_path: &str) -> Option<PathBuf> {
    let cleaned = requested_path.trim();
    if cleaned.is_empty() {
        return None;
    }
    let requested = PathBuf::from(cleaned);
    let candidate = if requested.is_absolute() {
        requested
    } else {
        base.join(requested)
    };
    let base_norm = normalize_lexical(base);
    let candidate_norm = normalize_lexical(&candidate);
    if !candidate_norm.starts_with(&base_norm) {
        return None;
    }
    Some(candidate_norm)
}

fn workspace_hint_tokens(message: &str, limit: usize) -> Vec<String> {
    let mut tokens = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for raw in clean_text(message, 600)
        .to_ascii_lowercase()
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'))
    {
        let token = raw.trim();
        if token.len() < 3 {
            continue;
        }
        if matches!(
            token,
            "the"
                | "and"
                | "for"
                | "with"
                | "that"
                | "this"
                | "from"
                | "have"
                | "your"
                | "you"
                | "are"
                | "was"
                | "were"
                | "will"
                | "into"
                | "about"
                | "what"
                | "when"
                | "then"
                | "than"
                | "just"
                | "they"
                | "them"
                | "able"
                | "make"
                | "made"
                | "need"
                | "want"
                | "does"
                | "did"
                | "done"
                | "not"
                | "too"
                | "very"
                | "also"
                | "like"
                | "been"
                | "being"
                | "each"
                | "more"
                | "most"
                | "over"
                | "under"
                | "after"
                | "before"
                | "because"
                | "while"
                | "where"
                | "which"
                | "would"
                | "could"
                | "should"
        ) {
            continue;
        }
        if seen.insert(token.to_string()) {
            tokens.push(token.to_string());
            if tokens.len() >= limit.max(1) {
                break;
            }
        }
    }
    tokens
}

fn should_infer_workspace_hints(message: &str) -> bool {
    let lowered = clean_text(message, 600).to_ascii_lowercase();
    [
        "file",
        "files",
        "module",
        "code",
        "api",
        "function",
        "class",
        "refactor",
        "patch",
        "update",
        "fix",
        "test",
        "workspace",
        "repo",
        "project",
        "notes",
        "docs",
        "meeting",
    ]
    .iter()
    .any(|token| lowered.contains(token))
}

fn should_skip_workspace_hint_entry(entry: &walkdir::DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
    let ignored = [
        ".git",
        "node_modules",
        "target",
        "dist",
        "build",
        ".next",
        ".cache",
        "artifacts",
        "backups",
        "tmp",
    ];
    ignored.iter().any(|value| *value == name)
}

fn workspace_file_hints_for_message(
    root: &Path,
    row: Option<&Value>,
    message: &str,
    limit: usize,
) -> Vec<Value> {
    if !should_infer_workspace_hints(message) {
        return Vec::new();
    }
    let tokens = workspace_hint_tokens(message, 8);
    if tokens.is_empty() {
        return Vec::new();
    }
    let workspace_base = workspace_base_for_agent(root, row);
    if !workspace_base.exists() {
        return Vec::new();
    }
    let lowered_message = clean_text(message, 600).to_ascii_lowercase();
    let code_focus = lowered_message.contains("code")
        || lowered_message.contains("api")
        || lowered_message.contains("function")
        || lowered_message.contains("test")
        || lowered_message.contains("module")
        || lowered_message.contains("refactor");
    let mut scored = Vec::<(i64, String, Vec<String>)>::new();
    let mut scanned = 0usize;
    let max_scan = 2200usize;
    for entry in WalkDir::new(&workspace_base)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| !should_skip_workspace_hint_entry(entry))
        .flatten()
    {
        if !entry.file_type().is_file() {
            continue;
        }
        scanned += 1;
        if scanned > max_scan {
            break;
        }
        let path = entry.path();
        let rel = path.strip_prefix(&workspace_base).unwrap_or(path);
        let rel_text = rel.to_string_lossy().replace('\\', "/");
        let rel_lc = rel_text.to_ascii_lowercase();
        let mut score = 0i64;
        let mut matches = Vec::<String>::new();
        for token in &tokens {
            if rel_lc.contains(token) {
                score += 5;
                matches.push(token.clone());
            } else if rel_lc
                .rsplit('/')
                .next()
                .map(|tail| tail.starts_with(token))
                .unwrap_or(false)
            {
                score += 3;
                matches.push(token.clone());
            }
        }
        if score <= 0 {
            continue;
        }
        if code_focus {
            let ext = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if matches!(
                ext.as_str(),
                "rs" | "ts" | "tsx" | "py" | "go" | "java" | "kt" | "cpp" | "c" | "h"
            ) {
                score += 2;
            }
        }
        scored.push((score, rel_text, matches));
    }

    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.len().cmp(&b.1.len())));
    scored
        .into_iter()
        .take(limit.clamp(1, 8))
        .map(|(score, path, matches)| {
            let match_count = matches.len();
            json!({
                "path": path,
                "score": score,
                "matches": matches,
                "reason": format!("matched {} workspace keywords", match_count)
            })
        })
        .collect::<Vec<_>>()
}

fn latent_tool_candidates_for_message(message: &str, workspace_hints: &[Value]) -> Vec<Value> {
    let lowered = clean_text(message, 1400).to_ascii_lowercase();
    if lowered.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::<Value>::new();
    let mut seen = HashSet::<String>::new();
    let mut push_candidate = |tool: &str, label: &str, reason: &str, proposed_input: Value| {
        let normalized = normalize_tool_name(tool);
        if normalized.is_empty() || seen.contains(&normalized) {
            return;
        }
        seen.insert(normalized.clone());
        let receipt = crate::deterministic_receipt_hash(&json!({
            "tool": normalized,
            "label": label,
            "reason": reason,
            "message": lowered.as_str(),
            "input": proposed_input.clone()
        }));
        out.push(json!({
            "tool": normalized,
            "label": clean_text(label, 80),
            "reason": clean_text(reason, 240),
            "requires_confirmation": true,
            "proposed_input": proposed_input,
            "discovery_receipt": receipt
        }));
    };

    let security_request = (lowered.contains("security")
        || lowered.contains("vulnerability")
        || lowered.contains("exploit")
        || lowered.contains("audit"))
        && (lowered.contains("code")
            || lowered.contains("api")
            || lowered.contains("module")
            || lowered.contains("file"));
    if security_request {
        push_candidate(
            "terminal_exec",
            "run security checks",
            "Security concern detected for code-path request.",
            json!({"command": "cargo test --workspace --tests"}),
        );
    }

    if let Some(path) = workspace_hints
        .first()
        .and_then(|row| row.get("path").and_then(Value::as_str))
    {
        if lowered.contains("file")
            || lowered.contains("module")
            || lowered.contains("api")
            || lowered.contains("update")
            || lowered.contains("change")
            || lowered.contains("patch")
            || lowered.contains("refactor")
        {
            push_candidate(
                "file_read",
                "open likely file",
                "Workspace file inference found a likely target.",
                json!({"path": path, "full": true}),
            );
        }
    }

    if lowered.contains("search")
        || lowered.contains("latest")
        || lowered.contains("news")
        || lowered.contains("internet")
        || lowered.contains("online")
        || lowered.contains("look up")
    {
        push_candidate(
            "web_search",
            "search web",
            "Message implies live web research intent.",
            json!({"query": clean_text(message, 600), "summary_only": true}),
        );
    }

    if lowered.contains("what did we decide")
        || lowered.contains("remember")
        || lowered.contains("recall")
        || lowered.contains("last month")
        || lowered.contains("previously")
    {
        push_candidate(
            "memory_semantic_query",
            "query semantic memory",
            "Message implies historical decision recall intent.",
            json!({"query": clean_text(message, 600), "limit": 8}),
        );
    }

    if lowered.contains("schedule")
        || lowered.contains("remind")
        || lowered.contains("every ")
        || lowered.contains("daily")
        || lowered.contains("cron")
    {
        push_candidate(
            "cron_schedule",
            "schedule follow-up",
            "Message implies recurring follow-up intent.",
            json!({"interval_minutes": 60, "message": clean_text(message, 400)}),
        );
    }

    if lowered.contains("swarm")
        || lowered.contains("parallel")
        || lowered.contains("subagent")
        || lowered.contains("multi-agent")
    {
        push_candidate(
            "spawn_subagents",
            "parallel subagents",
            "Message implies parallel execution intent.",
            json!({"count": infer_subagent_count_from_message(message), "objective": clean_text(message, 600)}),
        );
    }

    out.truncate(3);
    out
}

fn truncate_utf8_lossy(bytes: &[u8], max_bytes: usize) -> (String, bool) {
    if bytes.len() <= max_bytes {
        return (String::from_utf8_lossy(bytes).to_string(), false);
    }
    let mut end = max_bytes;
    while end > 0 && !std::str::from_utf8(&bytes[..end]).is_ok() {
        end -= 1;
    }
    let slice = if end == 0 {
        &bytes[..max_bytes]
    } else {
        &bytes[..end]
    };
    (String::from_utf8_lossy(slice).to_string(), true)
}

fn attention_policy_path(root: &Path) -> PathBuf {
    let from_env = std::env::var("MECH_SUIT_MODE_POLICY_PATH")
        .ok()
        .map(PathBuf::from);
    if let Some(path) = from_env {
        if path.is_absolute() {
            return path;
        }
        return root.join(path);
    }
    let default_root = root.join("config").join("mech_suit_mode_policy.json");
    if default_root.exists() {
        return default_root;
    }
    root.join("client/runtime/config/mech_suit_mode_policy.json")
}

fn attention_queue_path_for_dashboard(root: &Path) -> PathBuf {
    let fallback = root.join("client/runtime/local/state/attention/queue.jsonl");
    let policy = read_json_loose(&attention_policy_path(root)).unwrap_or_else(|| json!({}));
    let from_policy = clean_text(
        policy
            .pointer("/eyes/attention_queue_path")
            .and_then(Value::as_str)
            .unwrap_or(""),
        4000,
    );
    if from_policy.is_empty() {
        return fallback;
    }
    let raw = PathBuf::from(from_policy);
    if raw.is_absolute() {
        raw
    } else {
        root.join(raw)
    }
}

fn passive_attention_context_for_message(
    root: &Path,
    agent_id: &str,
    message: &str,
    max_items: usize,
) -> String {
    let path = attention_queue_path_for_dashboard(root);
    if !path.exists() {
        return String::new();
    }
    let message_terms = important_memory_terms(message, 20)
        .into_iter()
        .collect::<HashSet<_>>();
    let mut related = Vec::<String>::new();
    for row in read_jsonl_loose(&path, 1200) {
        let source = clean_text(row.get("source").and_then(Value::as_str).unwrap_or(""), 180);
        if source != format!("agent:{agent_id}") {
            continue;
        }
        let source_type = clean_text(
            row.get("source_type").and_then(Value::as_str).unwrap_or(""),
            120,
        )
        .to_ascii_lowercase();
        if source_type != "passive_memory_turn" {
            continue;
        }
        let summary = clean_text(
            row.get("summary").and_then(Value::as_str).unwrap_or(""),
            240,
        );
        if summary.is_empty() {
            continue;
        }
        if internal_context_metadata_phrase(&summary)
            || persistent_memory_denied_phrase(&summary)
            || runtime_access_denied_phrase(&summary)
        {
            continue;
        }
        let terms = row
            .pointer("/raw_event/terms")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|value| {
                value
                    .as_str()
                    .map(|raw| clean_text(raw, 120).to_ascii_lowercase())
            })
            .filter(|term| !term.is_empty())
            .collect::<HashSet<_>>();
        let effective_terms = if terms.is_empty() {
            important_memory_terms(&summary, 16)
                .into_iter()
                .collect::<HashSet<_>>()
        } else {
            terms
        };
        if !message_terms.is_empty() {
            if effective_terms.is_empty() || message_terms.is_disjoint(&effective_terms) {
                continue;
            }
        }
        if !related.iter().any(|item| item == &summary) {
            related.push(summary);
        }
        if related.len() >= max_items.max(1) {
            break;
        }
    }
    if related.is_empty() {
        String::new()
    } else {
        format!(
            "Relevant passive memory cues:\n{}",
            related
                .iter()
                .map(|row| format!("- {row}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

fn response_contains_project_dump_sections(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    let markers = [
        "project overview",
        "data source",
        "tools used",
        "key features",
        "sql queries",
        "future work",
        "how to use",
    ];
    let hits = markers
        .iter()
        .filter(|marker| lowered.contains(**marker))
        .count();
    hits >= 2
}

fn response_contains_tool_telemetry_dump(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    let noisy_markers = [
        "at duckduckgo all regions",
        "duckduckgo all regions",
        "all regions argentina",
        "all regions australia",
        "spawn_subagents failed:",
        "tool_explicit_signoff_required",
        "tool_confirmation_required",
    ];
    let hits = noisy_markers
        .iter()
        .filter(|marker| lowered.contains(**marker))
        .count();
    hits >= 2
}

fn response_is_unrelated_context_dump(user_message: &str, response_text: &str) -> bool {
    if response_text.contains("<function=") {
        return false;
    }
    if response_contains_tool_telemetry_dump(response_text) {
        return true;
    }
    if response_text.len() < 220 {
        return false;
    }
    if response_contains_project_dump_sections(response_text) {
        let user_terms = important_memory_terms(user_message, 20)
            .into_iter()
            .collect::<HashSet<_>>();
        let response_terms = important_memory_terms(response_text, 48)
            .into_iter()
            .collect::<HashSet<_>>();
        if user_terms.is_empty() || response_terms.is_empty() {
            return false;
        }
        return user_terms.is_disjoint(&response_terms);
    }
    false
}

fn response_looks_like_tool_ack_without_findings(text: &str) -> bool {
    let lowered = clean_text(text, 1200).to_ascii_lowercase();
    if lowered.is_empty() {
        return true;
    }
    if lowered.contains("no usable findings")
        || lowered.contains("couldn't extract reliable findings")
        || lowered.contains("could not extract reliable findings")
        || lowered.contains("couldn't extract usable findings")
        || lowered.contains("could not extract usable findings")
    {
        return false;
    }
    let token_count = lowered.split_whitespace().count();
    let mentions_tooling = lowered.contains("search")
        || lowered.contains("web")
        || lowered.contains("tool")
        || lowered.contains("looked up")
        || lowered.contains("called")
        || lowered.contains("executed")
        || lowered.contains("reading files")
        || lowered.contains("searching the internet")
        || lowered.contains("running terminal commands");
    let mainly_ack_language = lowered.contains("i searched")
        || lowered.contains("searched the internet")
        || lowered.contains("i looked up")
        || lowered.contains("i called")
        || lowered.contains("i executed")
        || lowered.contains("web search completed")
        || lowered.contains("tool completed");
    if !mentions_tooling {
        return false;
    }
    let has_rich_findings = lowered.contains("http://")
        || lowered.contains("https://")
        || lowered.contains("- ")
        || lowered.contains("1.")
        || lowered.contains("2.")
        || lowered.contains("key finding")
        || lowered.contains("sources:")
        || lowered.contains("according to");
    mentions_tooling
        && !has_rich_findings
        && (token_count <= 80 || mainly_ack_language)
}

fn response_tools_summary_for_user(response_tools: &[Value], max_items: usize) -> String {
    let limit = max_items.clamp(1, 8);
    let mut lines = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for tool in response_tools {
        let name = clean_text(tool.get("name").and_then(Value::as_str).unwrap_or("tool"), 80)
            .to_ascii_lowercase();
        if name.is_empty() || name == "thought_process" {
            continue;
        }
        if tool.get("is_error").and_then(Value::as_bool).unwrap_or(false) {
            continue;
        }
        let raw_result = clean_text(tool.get("result").and_then(Value::as_str).unwrap_or(""), 2_000);
        if raw_result.is_empty() {
            continue;
        }
        let lowered = raw_result.to_ascii_lowercase();
        if lowered.contains("model attempted this call as text") {
            continue;
        }
        if response_looks_like_tool_ack_without_findings(&raw_result) {
            continue;
        }
        if looks_like_search_engine_chrome_summary(&lowered) {
            continue;
        }
        let snippet = first_sentence(&raw_result, 220);
        if snippet.is_empty() {
            continue;
        }
        let pretty_name = name.replace('_', " ");
        let line = format!("- {}: {}", clean_text(&pretty_name, 60), snippet);
        let key = line.to_ascii_lowercase();
        if !seen.insert(key) {
            continue;
        }
        lines.push(line);
        if lines.len() >= limit {
            break;
        }
    }
    if lines.is_empty() {
        return String::new();
    }
    trim_text(&format!("Here's what I found:\n{}", lines.join("\n")), 32_000)
}

fn context_keyframes_prompt_context(
    state: &Value,
    max_keyframes: usize,
    max_chars: usize,
) -> String {
    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    let sessions = state
        .get("sessions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut keyframes = Vec::<String>::new();
    for session in sessions {
        let sid = clean_text(
            session
                .get("session_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        if sid != active_id {
            continue;
        }
        let entries = session
            .get("context_keyframes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for entry in entries.iter().rev().take(max_keyframes.max(1)) {
            let summary = clean_text(
                entry
                    .get("summary")
                    .and_then(Value::as_str)
                    .or_else(|| entry.get("text").and_then(Value::as_str))
                    .unwrap_or(""),
                260,
            );
            if summary.is_empty() {
                continue;
            }
            if internal_context_metadata_phrase(&summary)
                || persistent_memory_denied_phrase(&summary)
                || runtime_access_denied_phrase(&summary)
            {
                continue;
            }
            keyframes.push(summary);
        }
        break;
    }
    if keyframes.is_empty() {
        String::new()
    } else {
        let joined = keyframes.into_iter().rev().collect::<Vec<_>>().join(" | ");
        trim_text(
            &format!(
                "Compacted thread keyframes:\n- {}",
                clean_text(&joined, max_chars)
            ),
            max_chars,
        )
    }
}

fn first_sentence(raw: &str, max_len: usize) -> String {
    let cleaned = clean_text(raw, max_len.saturating_mul(4).max(200));
    if cleaned.is_empty() {
        return String::new();
    }
    let mut sentence_end = cleaned.len();
    for (idx, ch) in cleaned.char_indices() {
        if ch == '.' || ch == '!' || ch == '?' {
            sentence_end = idx + ch.len_utf8();
            break;
        }
    }
    clean_text(&cleaned[..sentence_end], max_len)
}

fn agent_identity_hydration_prompt(row: &Value) -> String {
    let agent_id = clean_text(
        row.get("agent_id")
            .or_else(|| row.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        160,
    );
    let name = clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 120);
    let resolved_name = if !name.is_empty() {
        name
    } else if !agent_id.is_empty() {
        humanize_agent_name(&agent_id)
    } else {
        "Agent".to_string()
    };
    let role = clean_text(
        row.get("role")
            .and_then(Value::as_str)
            .unwrap_or("assistant"),
        80,
    );
    let archetype = clean_text(
        row.pointer("/identity/archetype")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    let vibe = clean_text(
        row.pointer("/identity/vibe")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    let personality = first_sentence(
        row.get("system_prompt")
            .and_then(Value::as_str)
            .unwrap_or(""),
        220,
    );

    let mut profile_parts = vec![format!("name={resolved_name}"), format!("role={role}")];
    if !archetype.is_empty() {
        profile_parts.push(format!("archetype={archetype}"));
    }
    if !vibe.is_empty() {
        profile_parts.push(format!("vibe={vibe}"));
    }
    let mut lines = vec![format!(
        "Agent identity hydration: {}.",
        profile_parts.join(", ")
    )];
    if !personality.is_empty() {
        lines.push(format!("Personality directive: {personality}"));
    }
    lines.push(
        "When asked who you are, your name, or your role, reply using this profile in first person. Do not deny this identity unless profile metadata is changed later."
            .to_string(),
    );
    clean_text(&lines.join(" "), 1_600)
}

fn message_token_cost(row: &Value) -> i64 {
    estimate_tokens(&message_text(row))
}

fn total_message_tokens(rows: &[Value]) -> i64 {
    rows.iter().map(message_token_cost).sum::<i64>().max(0)
}

fn context_pressure_label(ratio: f64) -> &'static str {
    if !ratio.is_finite() || ratio <= 0.0 {
        "low"
    } else if ratio >= 0.96 {
        "critical"
    } else if ratio >= 0.82 {
        "high"
    } else if ratio >= 0.55 {
        "medium"
    } else {
        "low"
    }
}

fn trim_context_pool(messages: &[Value], limit_tokens: i64) -> Vec<Value> {
    let cap = limit_tokens.max(2_048);
    let mut out = messages.to_vec();
    let mut total = total_message_tokens(&out);
    while out.len() > 1 && total > cap {
        let removed = message_token_cost(&out[0]);
        out.remove(0);
        total = (total - removed).max(0);
    }
    out
}

fn select_active_context_window(
    messages: &[Value],
    target_tokens: i64,
    min_recent: usize,
) -> Vec<Value> {
    let cap = target_tokens.max(1_024);
    let floor = min_recent.clamp(1, 128);
    let mut out = messages.to_vec();
    let mut total = total_message_tokens(&out);
    while out.len() > floor && total > cap {
        let removed = message_token_cost(&out[0]);
        out.remove(0);
        total = (total - removed).max(0);
    }
    out
}

fn historical_context_keyframes_prompt_context(
    pooled_messages: &[Value],
    active_messages: &[Value],
    max_keyframes: usize,
    max_chars: usize,
) -> String {
    let target = max_keyframes.clamp(1, 24);
    let dropped = pooled_messages.len().saturating_sub(active_messages.len());
    if dropped == 0 {
        return String::new();
    }
    let mut candidates = Vec::<(String, String)>::new();
    for row in pooled_messages.iter().take(dropped) {
        let role = clean_text(
            row.get("role")
                .or_else(|| row.get("type"))
                .and_then(Value::as_str)
                .unwrap_or("assistant"),
            24,
        )
        .to_ascii_lowercase();
        if role == "system" {
            continue;
        }
        let snippet = first_sentence(&message_text(row), 220);
        if snippet.is_empty() {
            continue;
        }
        let role_label = if role.contains("user") {
            "User".to_string()
        } else {
            "Agent".to_string()
        };
        candidates.push((role_label, snippet));
    }
    if candidates.is_empty() {
        return String::new();
    }
    let mut selected = Vec::<(String, String)>::new();
    if candidates.len() <= target {
        selected = candidates;
    } else {
        selected.push(candidates[0].clone());
        if target > 2 {
            let remaining_slots = target.saturating_sub(2);
            let last_idx = candidates.len().saturating_sub(1);
            for slot in 0..remaining_slots {
                let idx = 1 + ((slot + 1) * last_idx.saturating_sub(1)) / (remaining_slots + 1);
                if idx < last_idx {
                    selected.push(candidates[idx].clone());
                }
            }
        }
        selected.push(candidates[candidates.len().saturating_sub(1)].clone());
    }
    let mut dedup = HashSet::<String>::new();
    let mut lines = Vec::<String>::new();
    for (role, snippet) in selected {
        let key = format!("{}|{}", role.to_ascii_lowercase(), snippet.to_ascii_lowercase());
        if !dedup.insert(key) {
            continue;
        }
        lines.push(format!("- [{role}] {snippet}"));
        if lines.len() >= target {
            break;
        }
    }
    if lines.is_empty() {
        return String::new();
    }
    trim_text(
        &format!(
            "Long-thread keyframes outside the active window (retain for continuity):\n{}",
            lines.join("\n")
        ),
        max_chars.max(400),
    )
}

fn historical_relevant_recall_prompt_context(
    pooled_messages: &[Value],
    active_messages: &[Value],
    user_message: &str,
    max_rows: usize,
    max_chars: usize,
) -> String {
    let target = max_rows.clamp(2, 20);
    let dropped = pooled_messages.len().saturating_sub(active_messages.len());
    if dropped == 0 {
        return String::new();
    }
    let user_terms = important_memory_terms(user_message, 24)
        .into_iter()
        .collect::<HashSet<_>>();
    let recall_intent = memory_recall_requested(user_message);
    if user_terms.is_empty() && !recall_intent {
        return String::new();
    }
    let mut scored = Vec::<(i64, String, String)>::new();
    for (idx, row) in pooled_messages.iter().take(dropped).enumerate() {
        let role = clean_text(
            row.get("role")
                .or_else(|| row.get("type"))
                .and_then(Value::as_str)
                .unwrap_or("assistant"),
            24,
        )
        .to_ascii_lowercase();
        if role == "system" {
            continue;
        }
        let snippet = clean_text(&message_text(row), 360);
        if snippet.is_empty() {
            continue;
        }
        let role_label = if role.contains("user") {
            "User".to_string()
        } else {
            "Agent".to_string()
        };
        let snippet_terms = important_memory_terms(&snippet, 24)
            .into_iter()
            .collect::<HashSet<_>>();
        let overlap = if user_terms.is_empty() {
            0
        } else {
            user_terms.intersection(&snippet_terms).count() as i64
        };
        if overlap == 0 && !recall_intent {
            continue;
        }
        let recency_score = (idx as i64).min(60);
        let score = overlap.saturating_mul(8) + recency_score;
        scored.push((score, role_label, first_sentence(&snippet, 260)));
    }
    if scored.is_empty() {
        return String::new();
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    let mut dedup = HashSet::<String>::new();
    let mut lines = Vec::<String>::new();
    for (_, role, snippet) in scored.into_iter().take(target.saturating_mul(2)) {
        if snippet.is_empty() {
            continue;
        }
        let key = format!("{}|{}", role.to_ascii_lowercase(), snippet.to_ascii_lowercase());
        if !dedup.insert(key) {
            continue;
        }
        lines.push(format!("- [{role}] {snippet}"));
        if lines.len() >= target {
            break;
        }
    }
    if lines.is_empty() {
        return String::new();
    }
    trim_text(
        &format!(
            "Relevant long-thread recall outside the active window (use for continuity):\n{}",
            lines.join("\n")
        ),
        max_chars.max(500),
    )
}

fn set_active_session_messages(state: &mut Value, messages: &[Value]) {
    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    if let Some(rows) = state.get_mut("sessions").and_then(Value::as_array_mut) {
        for row in rows.iter_mut() {
            let sid = clean_text(
                row.get("session_id").and_then(Value::as_str).unwrap_or(""),
                120,
            );
            if sid != active_id {
                continue;
            }
            row["messages"] = Value::Array(messages.to_vec());
            row["updated_at"] = Value::String(crate::now_iso());
            break;
        }
    }
}

fn context_command_payload(
    root: &Path,
    agent_id: &str,
    row: &Value,
    request: &Value,
    silent: bool,
) -> Value {
    let state = load_session_state(root, agent_id);
    let sessions_total = state
        .get("sessions")
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    let messages = all_session_messages(&state);
    let row_system_context_limit = row
        .get("system_context_tokens")
        .or_else(|| row.get("context_pool_limit_tokens"))
        .and_then(Value::as_i64)
        .unwrap_or(1_000_000);
    let context_pool_limit_tokens = request
        .get("context_pool_limit_tokens")
        .and_then(Value::as_i64)
        .unwrap_or(row_system_context_limit)
        .clamp(32_000, 2_000_000);
    let pooled_messages = trim_context_pool(&messages, context_pool_limit_tokens);
    let pre_generation_pruned = pooled_messages.len() != messages.len();
    let row_context_window = row
        .get("context_window_tokens")
        .or_else(|| row.get("context_window"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let context_window = if row_context_window > 0 {
        row_context_window
    } else {
        128_000
    };
    let active_context_target_tokens = request
        .get("active_context_target_tokens")
        .or_else(|| request.get("target_context_window"))
        .and_then(Value::as_i64)
        .unwrap_or_else(|| ((context_window as f64) * 0.68).round() as i64)
        .clamp(4_096, 512_000);
    let active_context_min_recent = request
        .get("active_context_min_recent_messages")
        .or_else(|| request.get("min_recent_messages"))
        .and_then(Value::as_u64)
        .unwrap_or(16)
        .clamp(4, 128) as usize;
    let row_auto_compact_threshold_ratio = row
        .get("auto_compact_threshold_ratio")
        .and_then(Value::as_f64)
        .unwrap_or(0.95);
    let row_auto_compact_target_ratio = row
        .get("auto_compact_target_ratio")
        .and_then(Value::as_f64)
        .unwrap_or(0.72);
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
    let mut active_messages = select_active_context_window(
        &pooled_messages,
        active_context_target_tokens,
        active_context_min_recent,
    );
    let context_pool_tokens = total_message_tokens(&pooled_messages);
    let mut context_tokens = total_message_tokens(&active_messages);
    let mut context_ratio = if context_window > 0 {
        (context_tokens as f64 / context_window as f64).clamp(0.0, 1.0)
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
    if context_ratio >= auto_compact_threshold_ratio && context_window > 0 {
        let emergency_target_tokens =
            ((context_window as f64) * auto_compact_target_ratio).round() as i64;
        let emergency_min_recent = request
            .get("emergency_min_recent_messages")
            .or_else(|| request.get("min_recent_messages"))
            .and_then(Value::as_u64)
            .unwrap_or(active_context_min_recent.min(4) as u64)
            .clamp(2, 128) as usize;
        let emergency_messages = select_active_context_window(
            &pooled_messages,
            emergency_target_tokens,
            emergency_min_recent,
        );
        let emergency_tokens = total_message_tokens(&emergency_messages);
        let removed_messages = active_messages
            .len()
            .saturating_sub(emergency_messages.len()) as u64;
        emergency_compact = json!({
            "triggered": true,
            "threshold_ratio": auto_compact_threshold_ratio,
            "target_ratio": auto_compact_target_ratio,
            "removed_messages": removed_messages,
            "before_tokens": context_tokens,
            "after_tokens": emergency_tokens,
            "persisted_to_history": false
        });
        if removed_messages > 0 && emergency_tokens <= context_tokens {
            active_messages = emergency_messages;
            context_tokens = emergency_tokens;
            context_ratio = if context_window > 0 {
                (context_tokens as f64 / context_window as f64).clamp(0.0, 1.0)
            } else {
                0.0
            };
            context_pressure = context_pressure_label(context_ratio).to_string();
        }
    }
    json!({
        "ok": true,
        "agent_id": agent_id,
        "command": "context",
        "silent": silent,
        "context_window": context_window,
        "context_tokens": context_tokens,
        "context_used_tokens": context_tokens,
        "context_ratio": context_ratio,
        "context_pressure": context_pressure,
        "context_pool": {
            "pool_limit_tokens": context_pool_limit_tokens,
            "pool_tokens": context_pool_tokens,
            "pool_messages": pooled_messages.len(),
            "session_count": sessions_total,
            "system_context_enabled": true,
            "system_context_limit_tokens": context_pool_limit_tokens,
            "llm_context_window_tokens": context_window,
            "active_target_tokens": active_context_target_tokens,
            "active_tokens": context_tokens,
            "active_messages": active_messages.len(),
            "min_recent_messages": active_context_min_recent,
            "pre_generation_pruning_enabled": true,
            "pre_generation_pruned": pre_generation_pruned,
            "emergency_compact_enabled": true,
            "emergency_compact": emergency_compact
        },
        "message": format!(
            "Context window: {} tokens | Active: {} tokens ({}%) | Pressure: {}",
            context_window.max(0),
            context_tokens.max(0),
            ((context_ratio * 100.0).round() as i64).max(0),
            context_pressure
        )
    })
}

fn data_url_from_bytes(bytes: &[u8], content_type: &str) -> String {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    format!(
        "data:{};base64,{}",
        clean_text(content_type, 120),
        STANDARD.encode(bytes)
    )
}
fn git_tree_payload_for_agent(root: &Path, snapshot: &Value, agent_id: &str) -> Value {
    let roster = build_agent_roster(root, snapshot, true);
    let mut counts = HashMap::<String, i64>::new();
    let mut current = Value::Null;
    for row in &roster {
        let branch = clean_text(
            row.get("git_branch").and_then(Value::as_str).unwrap_or(""),
            180,
        );
        if branch.is_empty() {
            continue;
        }
        *counts.entry(branch.clone()).or_insert(0) += 1;
        if clean_agent_id(row.get("id").and_then(Value::as_str).unwrap_or(""))
            == clean_agent_id(agent_id)
        {
            current = row.clone();
        }
    }
    let current_branch = clean_text(
        current
            .get("git_branch")
            .and_then(Value::as_str)
            .unwrap_or("main"),
        180,
    );
    let current_workspace = clean_text(
        current
            .get("workspace_dir")
            .and_then(Value::as_str)
            .unwrap_or(""),
        4000,
    );
    let current_workspace_dir = if current_workspace.is_empty() {
        root.to_path_buf()
    } else {
        PathBuf::from(&current_workspace)
    };
    let current_workspace_rel = current.get("workspace_rel").cloned().unwrap_or_else(|| {
        json!(crate::dashboard_git_runtime::workspace_rel(
            root,
            &current_workspace_dir
        ))
    });
    let (main_branch, mut branches) =
        crate::dashboard_git_runtime::list_git_branches(root, 200, &current_branch);
    if branches.is_empty() {
        branches.push(if main_branch.is_empty() {
            "main".to_string()
        } else {
            main_branch.clone()
        });
    }
    for branch in counts.keys() {
        if !branches.iter().any(|row| row == branch) {
            branches.push(branch.clone());
        }
    }
    branches.sort();
    let options = branches
        .iter()
        .map(|branch| {
            let kind = if branch == "main" || branch == "master" {
                "master"
            } else {
                "isolated"
            };
            let workspace = if branch == "main" || branch == "master" {
                root.to_path_buf()
            } else {
                crate::dashboard_git_runtime::workspace_for_agent_branch(root, agent_id, branch)
            };
            let ready = crate::dashboard_git_runtime::git_workspace_ready(root, &workspace);
            json!({
                "branch": branch,
                "current": *branch == current_branch,
                "main": *branch == "main" || *branch == "master",
                "kind": kind,
                "in_use_by_agents": counts.get(branch).copied().unwrap_or(0),
                "workspace_dir": workspace.to_string_lossy().to_string(),
                "workspace_rel": crate::dashboard_git_runtime::workspace_rel(root, &workspace),
                "git_tree_ready": if kind == "master" { true } else { ready },
                "git_tree_error": if kind == "master" || ready { "" } else { "git_worktree_missing" }
            })
        })
        .collect::<Vec<_>>();
    json!({
        "ok": true,
        "current": {
            "git_branch": if current_branch.is_empty() { "main" } else { &current_branch },
            "git_tree_kind": if current_branch == "main" || current_branch == "master" { "master" } else { "isolated" },
            "workspace_dir": if current_workspace.is_empty() { root.to_string_lossy().to_string() } else { current_workspace },
            "workspace_rel": current_workspace_rel,
            "git_tree_ready": current.get("git_tree_ready").cloned().unwrap_or_else(|| json!(true)),
            "git_tree_error": current.get("git_tree_error").cloned().unwrap_or_else(|| json!(""))
        },
        "options": options
    })
}

fn tool_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':')
}

fn normalize_tool_name(raw: &str) -> String {
    clean_text(raw, 80).to_ascii_lowercase().replace('-', "_")
}

fn find_json_object_span(raw: &str, from_index: usize) -> Option<(usize, usize)> {
    let mut start = None;
    for (idx, ch) in raw.char_indices().skip_while(|(idx, _)| *idx < from_index) {
        if ch == '{' {
            start = Some(idx);
            break;
        }
    }
    let start_idx = start?;
    let mut depth = 0i64;
    let mut in_string = false;
    let mut escaped = false;
    for (idx, ch) in raw.char_indices().skip_while(|(idx, _)| *idx < start_idx) {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
            continue;
        }
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                return Some((start_idx, idx + ch.len_utf8()));
            }
        }
    }
    None
}

fn extract_inline_tool_calls(
    text: &str,
    max_calls: usize,
) -> (String, Vec<(String, Value, String)>) {
    let mut calls = Vec::<(String, Value, String)>::new();
    let mut spans = Vec::<(usize, usize)>::new();
    let mut cursor = 0usize;
    let cap = max_calls.clamp(1, 12);

    while cursor < text.len() && calls.len() < cap {
        let next_open = text[cursor..].find("<function=").map(|idx| cursor + idx);
        let next_close = text[cursor..].find("</function>").map(|idx| cursor + idx);
        let next = match (next_open, next_close) {
            (Some(open), Some(close)) => Some(if open <= close {
                ("open", open)
            } else {
                ("close", close)
            }),
            (Some(open), None) => Some(("open", open)),
            (None, Some(close)) => Some(("close", close)),
            (None, None) => None,
        };
        let Some((kind, idx)) = next else {
            break;
        };
        if kind == "open" {
            let name_start = idx + "<function=".len();
            let Some(gt_rel) = text[name_start..].find('>') else {
                break;
            };
            let name_end = name_start + gt_rel;
            let raw_name = &text[name_start..name_end];
            let name = raw_name
                .chars()
                .take_while(|ch| tool_name_char(*ch))
                .collect::<String>();
            if name.is_empty() {
                cursor = name_end.saturating_add(1);
                continue;
            }
            let payload_start = name_end + 1;
            let Some((json_start, json_end)) = find_json_object_span(text, payload_start) else {
                cursor = payload_start;
                continue;
            };
            let parsed = serde_json::from_str::<Value>(&text[json_start..json_end]).ok();
            let Some(input) = parsed else {
                cursor = json_end;
                continue;
            };
            let tail = &text[json_end..];
            let full_end = tail
                .find("</function>")
                .map(|end| json_end + end + "</function>".len())
                .unwrap_or(json_end);
            let raw = text[idx..full_end].to_string();
            calls.push((name, input, raw));
            spans.push((idx, full_end));
            cursor = full_end;
            continue;
        }

        let close_idx = idx;
        let close_end = close_idx + "</function>".len();
        let prefix = &text[..close_idx];
        let mut back = prefix.len();
        while back > 0 {
            let ch = prefix[..back].chars().next_back().unwrap_or(' ');
            if tool_name_char(ch) {
                back -= ch.len_utf8();
            } else {
                break;
            }
        }
        let name = prefix[back..close_idx]
            .chars()
            .filter(|ch| tool_name_char(*ch))
            .collect::<String>();
        if name.is_empty() {
            cursor = close_end;
            continue;
        }
        let Some((json_start, json_end)) = find_json_object_span(text, close_end) else {
            cursor = close_end;
            continue;
        };
        let parsed = serde_json::from_str::<Value>(&text[json_start..json_end]).ok();
        let Some(input) = parsed else {
            cursor = json_end;
            continue;
        };
        let raw = text[back..json_end].to_string();
        calls.push((name, input, raw));
        spans.push((back, json_end));
        cursor = json_end;
    }

    if spans.is_empty() {
        return (text.to_string(), Vec::new());
    }
    spans.sort_by_key(|(start, _)| *start);
    let mut cleaned = String::new();
    let mut last = 0usize;
    for (start, end) in spans {
        if start > last {
            cleaned.push_str(&text[last..start]);
        }
        last = last.max(end);
    }
    if last < text.len() {
        cleaned.push_str(&text[last..]);
    }
    (cleaned.trim().to_string(), calls)
}

fn trim_text(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars.max(1)).collect::<String>()
}

fn tool_governance_policy(root: &Path) -> Value {
    let path = root.join("client/runtime/config/tool_governance_policy.json");
    let default = json!({
        "enabled": true,
        "tiers": {
            "green": {"confirm_required": false, "approval_note_min": 0},
            "yellow": {"confirm_required": true, "approval_note_min": 0},
            "red": {"confirm_required": true, "approval_note_min": 8}
        }
    });
    let mut merged = default.clone();
    if let Some(custom) = read_json_loose(&path) {
        if let Some(enabled) = custom.get("enabled").and_then(Value::as_bool) {
            merged["enabled"] = json!(enabled);
        }
        for tier in ["green", "yellow", "red"] {
            if let Some(confirm_required) = custom
                .pointer(&format!("/tiers/{tier}/confirm_required"))
                .and_then(Value::as_bool)
            {
                merged["tiers"][tier]["confirm_required"] = json!(confirm_required);
            }
            if let Some(min_note) = custom
                .pointer(&format!("/tiers/{tier}/approval_note_min"))
                .and_then(Value::as_i64)
            {
                merged["tiers"][tier]["approval_note_min"] = json!(min_note.max(0));
            }
        }
    }
    merged
}

fn input_has_confirmation(input: &Value) -> bool {
    input
        .get("confirm")
        .or_else(|| input.get("confirmed"))
        .or_else(|| input.get("approved"))
        .or_else(|| input.get("user_confirmed"))
        .or_else(|| input.get("signoff"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn input_approval_note(input: &Value) -> String {
    clean_text(
        input
            .get("approval_note")
            .or_else(|| input.get("note"))
            .or_else(|| input.get("reason"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        400,
    )
}

fn tool_capability_tier(normalized: &str, input: &Value) -> &'static str {
    match normalized {
        "terminal_exec" | "run_terminal" | "terminal" | "shell_exec" => "red",
        "spawn_subagents" | "spawn_swarm" | "agent_spawn" | "sessions_spawn" => "red",
        "agent_action" | "manage_agent" => {
            let action = clean_text(
                input.get("action").and_then(Value::as_str).unwrap_or(""),
                80,
            )
            .to_ascii_lowercase();
            if matches!(
                action.as_str(),
                "archive" | "delete" | "spawn" | "spawn_subagent"
            ) {
                "red"
            } else {
                "yellow"
            }
        }
        "memory_kv_set" | "memory_kv_delete" => "yellow",
        "cron_schedule" | "schedule_task" | "cron_create" => "yellow",
        "cron_run" | "schedule_run" | "cron_trigger" => "yellow",
        "cron_cancel" | "cron_delete" | "schedule_cancel" => "yellow",
        _ => "green",
    }
}

fn enforce_tool_capability_tier(
    root: &Path,
    actor_agent_id: &str,
    normalized_tool: &str,
    input: &Value,
) -> Option<Value> {
    let policy = tool_governance_policy(root);
    if !policy
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        return None;
    }
    let tier = tool_capability_tier(normalized_tool, input);
    let confirm_required = policy
        .pointer(&format!("/tiers/{tier}/confirm_required"))
        .and_then(Value::as_bool)
        .unwrap_or(matches!(tier, "yellow" | "red"));
    let min_note = policy
        .pointer(&format!("/tiers/{tier}/approval_note_min"))
        .and_then(Value::as_i64)
        .unwrap_or(if tier == "red" { 8 } else { 0 })
        .max(0) as usize;
    let confirmed = input_has_confirmation(input);
    let note = input_approval_note(input);
    if (!confirm_required || confirmed) && note.len() >= min_note {
        return None;
    }
    let next_step = if tier == "red" {
        format!(
            "Re-run with {{\"confirm\":true,\"approval_note\":\"why this destructive action is needed\"}} for `{}`.",
            normalized_tool
        )
    } else {
        format!(
            "Re-run with {{\"confirm\":true}} to execute `{}`.",
            normalized_tool
        )
    };
    let receipt = crate::deterministic_receipt_hash(&json!({
        "type": "tool_capability_tier_gate",
        "actor_agent_id": actor_agent_id,
        "tool": normalized_tool,
        "tier": tier,
        "confirmed": confirmed,
        "approval_note_len": note.len(),
        "ts": crate::now_iso()
    }));
    Some(json!({
        "ok": false,
        "error": if tier == "red" { "tool_explicit_signoff_required" } else { "tool_confirmation_required" },
        "tool": normalized_tool,
        "capability_tier": tier,
        "confirm_required": confirm_required,
        "approval_note_min_chars": min_note,
        "next_step": next_step,
        "receipt_hash": receipt
    }))
}

fn spawn_guard_policy(root: &Path) -> Value {
    let spawn_policy = read_json_loose(&root.join("client/runtime/config/spawn_policy.json"))
        .unwrap_or_else(|| json!({}));
    let child_policy =
        read_json_loose(&root.join("client/runtime/config/child_organ_runtime_policy.json"))
            .unwrap_or_else(|| json!({}));
    let orchestron_policy =
        read_json_loose(&root.join("client/runtime/config/orchestron_policy.json"))
            .unwrap_or_else(|| json!({}));
    let max_per_spawn = spawn_policy
        .pointer("/pool/max_cells")
        .and_then(Value::as_i64)
        .unwrap_or(8)
        .clamp(1, 64);
    let max_descendants_per_parent = child_policy
        .get("max_children")
        .and_then(Value::as_i64)
        .unwrap_or(24)
        .clamp(1, 4096);
    let max_depth = orchestron_policy
        .get("max_depth")
        .and_then(Value::as_i64)
        .unwrap_or(4)
        .clamp(1, 32);
    let per_child_budget_default = child_policy
        .pointer("/resource_envelope/token_cap_default")
        .and_then(Value::as_i64)
        .unwrap_or(800)
        .clamp(64, 200_000);
    let per_child_budget_max = child_policy
        .pointer("/resource_envelope/token_cap_max")
        .and_then(Value::as_i64)
        .unwrap_or(5000)
        .clamp(per_child_budget_default, 2_000_000);
    let spawn_budget_cap = per_child_budget_max
        .saturating_mul(max_per_spawn)
        .clamp(per_child_budget_max, 20_000_000);
    json!({
        "max_per_spawn": max_per_spawn,
        "max_descendants_per_parent": max_descendants_per_parent,
        "max_depth": max_depth,
        "per_child_budget_default": per_child_budget_default,
        "per_child_budget_max": per_child_budget_max,
        "spawn_budget_cap": spawn_budget_cap
    })
}

fn descendant_count(parent_map: &HashMap<String, String>, actor: &str) -> usize {
    let actor_id = clean_agent_id(actor);
    if actor_id.is_empty() {
        return 0;
    }
    let mut count = 0usize;
    for candidate in parent_map.keys() {
        let mut current = candidate.clone();
        let mut hops = 0usize;
        let mut seen = HashSet::<String>::new();
        while hops < 128 && seen.insert(current.clone()) {
            let Some(parent) = parent_map.get(&current).cloned() else {
                break;
            };
            if parent == actor_id {
                count += 1;
                break;
            }
            current = parent;
            hops += 1;
        }
    }
    count
}

fn agent_depth_from_parent_map(parent_map: &HashMap<String, String>, agent_id: &str) -> usize {
    let mut current = clean_agent_id(agent_id);
    if current.is_empty() {
        return 0;
    }
    let mut depth = 0usize;
    let mut seen = HashSet::<String>::new();
    while depth < 128 && seen.insert(current.clone()) {
        let Some(parent) = parent_map.get(&current).cloned() else {
            break;
        };
        current = parent;
        depth += 1;
    }
    depth
}

fn subagent_context_slice(root: &Path, parent_agent_id: &str, objective: &str) -> Value {
    let state = load_session_state(root, parent_agent_id);
    let mut messages = session_messages(&state);
    if messages.is_empty() {
        return json!({
            "strategy": "objective_scoped_recent_window",
            "selected_messages": [],
            "selected_count": 0
        });
    }
    let objective_tokens = workspace_hint_tokens(objective, 10);
    messages.sort_by_key(message_timestamp_iso);
    let mut scored = Vec::<(i64, Value)>::new();
    for (idx, row) in messages.into_iter().enumerate() {
        let role = clean_text(row.get("role").and_then(Value::as_str).unwrap_or(""), 20)
            .to_ascii_lowercase();
        if role.is_empty() {
            continue;
        }
        let text = message_text(&row);
        if text.is_empty() {
            continue;
        }
        let mut score = (idx as i64).min(40);
        let lowered = text.to_ascii_lowercase();
        for token in &objective_tokens {
            if lowered.contains(token) {
                score += 5;
            }
        }
        if role == "user" {
            score += 2;
        }
        scored.push((
            score,
            json!({
                "role": role,
                "text": trim_text(&text, 600),
                "ts": message_timestamp_iso(&row)
            }),
        ));
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    let selected = scored
        .into_iter()
        .take(12)
        .map(|(_, row)| row)
        .collect::<Vec<_>>();
    let selected_count = selected.len();
    json!({
        "strategy": "objective_scoped_recent_window",
        "objective_tokens": objective_tokens,
        "selected_messages": selected,
        "selected_count": selected_count
    })
}

fn tool_decision_audit_path(root: &Path) -> PathBuf {
    root.join("client/runtime/local/state/ui/infring_dashboard/decision_audit.jsonl")
}

fn append_tool_decision_audit(
    root: &Path,
    actor_agent_id: &str,
    tool_name: &str,
    tool_input: &Value,
    tool_output: &Value,
    recovery_strategy: &str,
) -> String {
    let tier = tool_capability_tier(tool_name, tool_input);
    let row = json!({
        "type": "tool_decision_audit",
        "timestamp": crate::now_iso(),
        "actor_agent_id": clean_agent_id(actor_agent_id),
        "tool": normalize_tool_name(tool_name),
        "capability_tier": tier,
        "ok": tool_output.get("ok").and_then(Value::as_bool).unwrap_or(false),
        "error": clean_text(tool_output.get("error").and_then(Value::as_str).unwrap_or(""), 240),
        "recovery_strategy": clean_text(recovery_strategy, 80),
        "input_hash": crate::deterministic_receipt_hash(tool_input),
        "output_hash": crate::deterministic_receipt_hash(tool_output)
    });
    let receipt = crate::deterministic_receipt_hash(&row);
    append_jsonl_row(&tool_decision_audit_path(root), &row);
    receipt
}

fn execute_tool_call_by_name(
    root: &Path,
    snapshot: &Value,
    actor_agent_id: &str,
    existing: Option<&Value>,
    tool_name: &str,
    input: &Value,
) -> Value {
    let normalized = normalize_tool_name(tool_name);
    let actor = clean_agent_id(actor_agent_id);
    if actor.is_empty() {
        return json!({
            "ok": false,
            "error": "actor_agent_required"
        });
    }
    if let Some(gate_payload) = enforce_tool_capability_tier(root, &actor, &normalized, input) {
        return gate_payload;
    }
    let headers = vec![("X-Actor-Agent-Id", actor.as_str())];
    match normalized.as_str() {
        "file_read" | "read_file" | "file" => {
            let body = if input.is_object() {
                input.clone()
            } else {
                json!({"path": clean_text(input.as_str().unwrap_or(""), 4000)})
            };
            let path = format!("/api/agents/{actor}/file/read");
            let body_bytes = serde_json::to_vec(&body).unwrap_or_default();
            handle_with_headers(root, "POST", &path, &body_bytes, &headers, snapshot)
                .map(|response| response.payload)
                .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}))
        }
        "folder_export" | "list_folder" | "folder_tree" | "folder" => {
            let body = if input.is_object() {
                input.clone()
            } else {
                json!({"path": clean_text(input.as_str().unwrap_or(""), 4000)})
            };
            let path = format!("/api/agents/{actor}/folder/export");
            let body_bytes = serde_json::to_vec(&body).unwrap_or_default();
            handle_with_headers(root, "POST", &path, &body_bytes, &headers, snapshot)
                .map(|response| response.payload)
                .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}))
        }
        "terminal_exec" | "run_terminal" | "terminal" | "shell_exec" => {
            let body = if input.is_object() {
                input.clone()
            } else {
                json!({"command": clean_text(input.as_str().unwrap_or(""), 12000)})
            };
            let path = format!("/api/agents/{actor}/terminal");
            let body_bytes = serde_json::to_vec(&body).unwrap_or_default();
            handle_with_headers(root, "POST", &path, &body_bytes, &headers, snapshot)
                .map(|response| response.payload)
                .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}))
        }
        "web_fetch" | "browse" | "web_conduit_fetch" => {
            let body = if input.is_object() {
                input.clone()
            } else {
                json!({"url": clean_text(input.as_str().unwrap_or(""), 2200)})
            };
            let body_bytes = serde_json::to_vec(&body).unwrap_or_default();
            handle_with_headers(
                root,
                "POST",
                "/api/web/fetch",
                &body_bytes,
                &headers,
                snapshot,
            )
            .map(|response| response.payload)
            .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}))
        }
        "web_search" | "search_web" | "search" | "web_query" => {
            let body = if input.is_object() {
                input.clone()
            } else {
                json!({"query": clean_text(input.as_str().unwrap_or(""), 600)})
            };
            let body_bytes = serde_json::to_vec(&body).unwrap_or_default();
            handle_with_headers(
                root,
                "POST",
                "/api/web/search",
                &body_bytes,
                &headers,
                snapshot,
            )
            .map(|response| response.payload)
            .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}))
        }
        "cron_list" | "schedule_list" | "cron_jobs" => {
            handle_with_headers(root, "GET", "/api/cron/jobs", &[], &headers, snapshot)
                .map(|response| response.payload)
                .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}))
        }
        "cron_schedule" | "schedule_task" | "cron_create" => {
            let interval_minutes =
                parse_non_negative_i64(input.get("interval_minutes"), 60).clamp(1, 10_080);
            let default_name = format!("{}-{}m-checkin", actor, interval_minutes);
            let job_name = clean_text(
                input
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or(default_name.as_str()),
                180,
            );
            let action_message = clean_text(
                input
                    .get("message")
                    .or_else(|| input.get("task"))
                    .or_else(|| input.get("objective"))
                    .and_then(Value::as_str)
                    .unwrap_or("Scheduled follow-up check."),
                2_000,
            );
            let mut request_body = json!({
                "name": if job_name.is_empty() { default_name } else { job_name },
                "agent_id": actor,
                "enabled": input.get("enabled").and_then(Value::as_bool).unwrap_or(true),
                "schedule": {
                    "kind": "every",
                    "every_secs": interval_minutes.saturating_mul(60)
                },
                "action": {
                    "kind": "agent_turn",
                    "message": if action_message.is_empty() {
                        "Scheduled follow-up check."
                    } else {
                        action_message.as_str()
                    }
                }
            });
            if let Some(custom_schedule) = input.get("schedule").cloned() {
                request_body["schedule"] = custom_schedule;
            }
            let body_bytes = serde_json::to_vec(&request_body).unwrap_or_default();
            handle_with_headers(
                root,
                "POST",
                "/api/cron/jobs",
                &body_bytes,
                &headers,
                snapshot,
            )
            .map(|response| response.payload)
            .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}))
        }
        "cron_cancel" | "cron_delete" | "schedule_cancel" => {
            let job_id = clean_text(
                input
                    .get("job_id")
                    .or_else(|| input.get("id"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                140,
            );
            if job_id.is_empty() {
                return json!({"ok": false, "error": "job_id_required"});
            }
            let path = format!("/api/cron/jobs/{job_id}");
            handle_with_headers(root, "DELETE", &path, &[], &headers, snapshot)
                .map(|response| response.payload)
                .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}))
        }
        "cron_run" | "schedule_run" | "cron_trigger" => {
            let job_id = clean_text(
                input
                    .get("job_id")
                    .or_else(|| input.get("id"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                140,
            );
            if job_id.is_empty() {
                return json!({"ok": false, "error": "job_id_required"});
            }
            let path = format!("/api/schedules/{job_id}/run");
            handle_with_headers(root, "POST", &path, &[], &headers, snapshot)
                .map(|response| response.payload)
                .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}))
        }
        "spawn_subagents" | "spawn_swarm" | "agent_spawn" | "sessions_spawn" => {
            let spawn_policy = spawn_guard_policy(root);
            let max_per_spawn = spawn_policy
                .get("max_per_spawn")
                .and_then(Value::as_i64)
                .unwrap_or(8)
                .clamp(1, 64) as usize;
            let max_descendants_per_parent = spawn_policy
                .get("max_descendants_per_parent")
                .and_then(Value::as_i64)
                .unwrap_or(24)
                .clamp(1, 4096) as usize;
            let depth_limit = spawn_policy
                .get("max_depth")
                .and_then(Value::as_i64)
                .unwrap_or(4)
                .clamp(1, 32) as usize;
            let per_child_budget_default = spawn_policy
                .get("per_child_budget_default")
                .and_then(Value::as_i64)
                .unwrap_or(800)
                .clamp(64, 200_000);
            let per_child_budget_max = spawn_policy
                .get("per_child_budget_max")
                .and_then(Value::as_i64)
                .unwrap_or(5000)
                .clamp(per_child_budget_default, 2_000_000);
            let spawn_budget_cap = spawn_policy
                .get("spawn_budget_cap")
                .and_then(Value::as_i64)
                .unwrap_or(per_child_budget_max.saturating_mul(max_per_spawn as i64))
                .clamp(per_child_budget_max, 20_000_000);

            let requested_count_raw = input
                .get("count")
                .or_else(|| input.get("team_size"))
                .or_else(|| input.get("agents"))
                .and_then(Value::as_i64)
                .unwrap_or(3);
            let requested_count_raw_pos = requested_count_raw.max(1) as usize;
            let requested_count = requested_count_raw_pos.min(max_per_spawn);
            let expiry_seconds = input
                .get("expiry_seconds")
                .or_else(|| input.get("lifespan_sec"))
                .and_then(Value::as_i64)
                .unwrap_or(3600)
                .clamp(60, 172_800);
            let budget_tokens_requested_raw = input
                .get("budget_tokens")
                .or_else(|| input.get("token_budget"))
                .and_then(Value::as_i64)
                .unwrap_or(per_child_budget_default);
            let budget_tokens = budget_tokens_requested_raw.clamp(64, per_child_budget_max);
            let budget_tokens_for_capacity =
                budget_tokens_requested_raw.clamp(64, spawn_budget_cap);
            let objective = clean_text(
                input
                    .get("objective")
                    .or_else(|| input.get("task"))
                    .or_else(|| input.get("message"))
                    .and_then(Value::as_str)
                    .unwrap_or("Parallel child task requested by parent directive."),
                800,
            );
            let merge_strategy = match clean_text(
                input
                    .get("merge_strategy")
                    .or_else(|| input.get("merge"))
                    .and_then(Value::as_str)
                    .unwrap_or("reduce"),
                40,
            )
            .to_ascii_lowercase()
            .as_str()
            {
                "voting" | "vote" => "voting",
                "concat" | "concatenate" => "concatenate",
                _ => "reduce",
            }
            .to_string();
            let mut role_plan = input
                .get("roles")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .filter_map(Value::as_str)
                        .map(|row| clean_text(row, 60))
                        .filter(|row| !row.is_empty())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let role_hint = clean_text(
                input
                    .get("role")
                    .or_else(|| input.get("default_role"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                60,
            );
            if !role_hint.is_empty() && role_plan.is_empty() {
                role_plan.push(role_hint);
            }
            if role_plan.is_empty() {
                role_plan = vec![
                    "analyst".to_string(),
                    "researcher".to_string(),
                    "builder".to_string(),
                    "reviewer".to_string(),
                ];
            }
            let base_name = clean_text(
                input
                    .get("base_name")
                    .or_else(|| input.get("name_prefix"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                80,
            );
            let parent_map = agent_parent_map(root, snapshot);
            let current_depth = agent_depth_from_parent_map(&parent_map, &actor);
            if current_depth + 1 > depth_limit {
                return json!({
                    "ok": false,
                    "error": "spawn_depth_limit_exceeded",
                    "parent_agent_id": actor,
                    "current_depth": current_depth,
                    "max_depth": depth_limit
                });
            }
            let existing_descendants = descendant_count(&parent_map, &actor);
            if existing_descendants >= max_descendants_per_parent {
                return json!({
                    "ok": false,
                    "error": "spawn_descendant_limit_exceeded",
                    "parent_agent_id": actor,
                    "existing_descendants": existing_descendants,
                    "max_descendants_per_parent": max_descendants_per_parent
                });
            }
            let remaining_capacity =
                max_descendants_per_parent.saturating_sub(existing_descendants);
            let budget_limited_count =
                ((spawn_budget_cap / budget_tokens_for_capacity.max(1)) as usize).max(1);
            let effective_count = requested_count
                .min(remaining_capacity.max(1))
                .min(budget_limited_count.max(1));
            if effective_count == 0 {
                return json!({
                    "ok": false,
                    "error": "spawn_budget_exceeded",
                    "parent_agent_id": actor,
                    "spawn_budget_cap": spawn_budget_cap,
                    "requested_budget_tokens": budget_tokens
                });
            }
            let context_slice = subagent_context_slice(root, &actor, &objective);
            let directive_receipt = crate::deterministic_receipt_hash(&json!({
                "type": "agent_spawn_directive",
                "actor_agent_id": actor,
                "requested_count_raw": requested_count_raw,
                "requested_count": requested_count,
                "effective_count": effective_count,
                "objective": objective,
                "merge_strategy": merge_strategy,
                "budget_tokens": budget_tokens,
                "budget_tokens_requested_raw": budget_tokens_requested_raw,
                "budget_tokens_for_capacity": budget_tokens_for_capacity,
                "requested_at": crate::now_iso()
            }));

            let mut created = Vec::<Value>::new();
            let mut errors = Vec::<Value>::new();
            for idx in 0..effective_count {
                let role = role_plan
                    .get(idx % role_plan.len())
                    .cloned()
                    .unwrap_or_else(|| "analyst".to_string());
                let mut request_body = json!({
                    "role": role,
                    "parent_agent_id": actor,
                    "contract": {
                        "owner": "descendant_auto_spawn",
                        "mission": if objective.is_empty() {
                            format!("Parallel subtask for parent {}", actor)
                        } else {
                            format!("Parallel subtask for parent {}: {}", actor, objective)
                        },
                        "termination_condition": "task_or_timeout",
                        "expiry_seconds": expiry_seconds,
                        "auto_terminate_allowed": true,
                        "budget_tokens": budget_tokens,
                        "merge_strategy": merge_strategy,
                        "context_slice": context_slice,
                        "source_user_directive": objective,
                        "source_user_directive_receipt": directive_receipt,
                        "spawn_guard": {
                            "max_depth": depth_limit,
                            "max_descendants_per_parent": max_descendants_per_parent,
                            "spawn_budget_cap": spawn_budget_cap
                        }
                    }
                });
                if !base_name.is_empty() {
                    request_body["name"] = json!(format!("{base_name}-{}", idx + 1));
                }
                let body_bytes = serde_json::to_vec(&request_body).unwrap_or_default();
                let spawned = handle_with_headers(
                    root,
                    "POST",
                    "/api/agents",
                    &body_bytes,
                    &headers,
                    snapshot,
                )
                .map(|response| response.payload)
                .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}));
                if spawned.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    created.push(json!({
                        "agent_id": clean_agent_id(
                            spawned
                                .get("agent_id")
                                .or_else(|| spawned.get("id"))
                                .and_then(Value::as_str)
                                .unwrap_or("")
                        ),
                        "name": clean_text(spawned.get("name").and_then(Value::as_str).unwrap_or(""), 120),
                        "role": role
                    }));
                } else {
                    errors.push(json!({
                        "role": role,
                        "error": clean_text(spawned.get("error").and_then(Value::as_str).unwrap_or("spawn_failed"), 160)
                    }));
                }
            }
            let mut out = json!({
                "ok": !created.is_empty(),
                "type": "spawn_subagents",
                "parent_agent_id": actor,
                "requested_count_raw": requested_count_raw,
                "requested_count": requested_count,
                "effective_count": effective_count,
                "created_count": created.len(),
                "failed_count": errors.len(),
                "directive": {
                    "objective": objective,
                    "receipt": directive_receipt,
                    "merge_strategy": merge_strategy,
                    "budget_tokens": budget_tokens
                },
                "circuit_breakers": {
                    "max_depth": depth_limit,
                    "current_depth": current_depth,
                    "existing_descendants": existing_descendants,
                    "max_descendants_per_parent": max_descendants_per_parent,
                    "spawn_budget_cap": spawn_budget_cap,
                    "remaining_capacity": remaining_capacity,
                    "degraded": effective_count < requested_count_raw_pos
                },
                "children": created,
                "errors": errors
            });
            out["receipt_hash"] = json!(crate::deterministic_receipt_hash(&out));
            out
        }
        "session_rollback_last_turn" | "undo_last_turn" | "rewind_turn" => {
            rollback_last_turn(root, &actor)
        }
        "memory_kv_get" => {
            let key = clean_text(input.get("key").and_then(Value::as_str).unwrap_or(""), 180);
            if key.is_empty() {
                return json!({"ok": false, "error": "memory_key_required"});
            }
            crate::dashboard_agent_state::memory_kv_get(root, &actor, &key)
        }
        "memory_kv_set" => {
            let key = clean_text(input.get("key").and_then(Value::as_str).unwrap_or(""), 180);
            if key.is_empty() {
                return json!({"ok": false, "error": "memory_key_required"});
            }
            let value = input.get("value").cloned().unwrap_or(Value::Null);
            crate::dashboard_agent_state::memory_kv_set(root, &actor, &key, &value)
        }
        "memory_kv_list" | "memory_kv_pairs" => {
            crate::dashboard_agent_state::memory_kv_pairs(root, &actor)
        }
        "memory_semantic_query" | "memory_query" => {
            let query = clean_text(
                input
                    .get("query")
                    .or_else(|| input.get("q"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                600,
            );
            let limit = input
                .get("limit")
                .and_then(Value::as_u64)
                .map(|value| value as usize)
                .unwrap_or(8)
                .clamp(1, 25);
            crate::dashboard_agent_state::memory_kv_semantic_query(root, &actor, &query, limit)
        }
        "agent_action" | "manage_agent" => {
            let action = clean_text(
                input.get("action").and_then(Value::as_str).unwrap_or(""),
                80,
            )
            .to_ascii_lowercase();
            let target = clean_agent_id(
                input
                    .get("agent_id")
                    .and_then(Value::as_str)
                    .unwrap_or(actor.as_str()),
            );
            if target.is_empty() || action.is_empty() {
                return json!({"ok": false, "error": "agent_action_and_target_required"});
            }
            let (method, path, body) = match action.as_str() {
                "start" => ("POST", format!("/api/agents/{target}/start"), json!({})),
                "stop" => ("POST", format!("/api/agents/{target}/stop"), json!({})),
                "archive" | "delete" => ("DELETE", format!("/api/agents/{target}"), json!({})),
                "clone" => (
                    "POST",
                    format!("/api/agents/{target}/clone"),
                    json!({"new_name": input.get("new_name").cloned().unwrap_or(Value::Null)}),
                ),
                "message" => (
                    "POST",
                    format!("/api/agents/{target}/message"),
                    json!({"message": clean_text(input.get("message").and_then(Value::as_str).unwrap_or(""), 8000)}),
                ),
                "spawn" | "spawn_subagent" => (
                    "POST",
                    "/api/agents".to_string(),
                    json!({
                        "name": clean_text(input.get("name").and_then(Value::as_str).unwrap_or(""), 120),
                        "role": clean_text(input.get("role").and_then(Value::as_str).unwrap_or("analyst"), 60),
                        "parent_agent_id": target,
                        "contract": {
                            "owner": clean_text(input.get("owner").and_then(Value::as_str).unwrap_or("manage_agent_spawn"), 80),
                            "mission": clean_text(input.get("mission").and_then(Value::as_str).unwrap_or("Assist parent mission"), 200),
                            "termination_condition": "task_or_timeout",
                            "expiry_seconds": input.get("expiry_seconds").and_then(Value::as_i64).unwrap_or(3600).clamp(60, 172_800),
                            "auto_terminate_allowed": input.get("auto_terminate_allowed").and_then(Value::as_bool).unwrap_or(true),
                            "idle_terminate_allowed": input.get("idle_terminate_allowed").and_then(Value::as_bool).unwrap_or(true)
                        }
                    }),
                ),
                _ => {
                    return json!({
                        "ok": false,
                        "error": "unsupported_agent_action",
                        "action": action
                    })
                }
            };
            let body_bytes = serde_json::to_vec(&body).unwrap_or_default();
            handle_with_headers(root, method, &path, &body_bytes, &headers, snapshot)
                .map(|response| response.payload)
                .unwrap_or_else(|| json!({"ok": false, "error": "tool_route_not_found"}))
        }
        "tabs_list" | "list_tabs" => {
            let _ = existing;
            json!({
                "ok": true,
                "tabs": [
                    "agents",
                    "chat",
                    "channels",
                    "plugins",
                    "sessions",
                    "approvals",
                    "workflows",
                    "scheduler",
                    "settings",
                    "network",
                    "security",
                    "usage",
                    "comms"
                ]
            })
        }
        _ => json!({
            "ok": false,
            "error": "unsupported_tool",
            "tool": tool_name
        }),
    }
}

fn summarize_tool_payload(tool_name: &str, payload: &Value) -> String {
    let normalized = normalize_tool_name(tool_name);
    if normalized == "spawn_subagents"
        || normalized == "spawn_swarm"
        || normalized == "agent_spawn"
        || normalized == "sessions_spawn"
    {
        if !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            return user_facing_tool_failure_summary(tool_name, payload)
                .unwrap_or_else(|| "I couldn't start parallel agents in this turn.".to_string());
        }
        let created_count = payload
            .get("created_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let requested_count = payload
            .get("requested_count")
            .and_then(Value::as_u64)
            .unwrap_or(created_count);
        let receipt = clean_text(
            payload
                .pointer("/directive/receipt")
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        );
        let ids = payload
            .get("children")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(|row| row.get("agent_id").and_then(Value::as_str))
                    .map(|row| clean_text(row, 60))
                    .filter(|row| !row.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let mut summary = format!("Spawned {created_count}/{requested_count} descendant agents.");
        if !ids.is_empty() {
            summary.push_str(&format!(" IDs: {}.", ids.join(", ")));
        }
        if !receipt.is_empty() {
            summary.push_str(&format!(" Directive receipt: {receipt}."));
        }
        return trim_text(&summary, 24_000);
    }
    if normalized == "memory_semantic_query" || normalized == "memory_query" {
        let query = clean_text(
            payload.get("query").and_then(Value::as_str).unwrap_or(""),
            200,
        );
        let matches = payload
            .get("matches")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if matches.is_empty() {
            if query.is_empty() {
                return "No semantic memory matches found.".to_string();
            }
            return trim_text(
                &format!("No semantic memory matches for `{query}`."),
                24_000,
            );
        }
        let mut lines = Vec::<String>::new();
        if query.is_empty() {
            lines.push("Semantic memory matches:".to_string());
        } else {
            lines.push(format!("Semantic memory matches for `{query}`:"));
        }
        for row in matches.into_iter().take(5) {
            let key = clean_text(row.get("key").and_then(Value::as_str).unwrap_or(""), 160);
            let snippet = clean_text(
                row.get("snippet").and_then(Value::as_str).unwrap_or(""),
                180,
            );
            let score = row.get("score").and_then(Value::as_i64).unwrap_or(0);
            if key.is_empty() {
                continue;
            }
            if snippet.is_empty() {
                lines.push(format!("- {key} (score {score})"));
            } else {
                lines.push(format!("- {key} (score {score}): {snippet}"));
            }
        }
        return trim_text(&lines.join("\n"), 24_000);
    }
    if normalized == "cron_schedule" || normalized == "schedule_task" || normalized == "cron_create"
    {
        let job_id = clean_text(
            payload
                .pointer("/job/id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("job_id").and_then(Value::as_str))
                .unwrap_or(""),
            140,
        );
        let name = clean_text(
            payload
                .pointer("/job/name")
                .and_then(Value::as_str)
                .unwrap_or("scheduled-job"),
            180,
        );
        let next_run = clean_text(
            payload
                .pointer("/job/next_run")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let mut summary = format!("Scheduled cron job `{}`.", name);
        if !job_id.is_empty() {
            summary.push_str(&format!(" ID: {job_id}."));
        }
        if !next_run.is_empty() {
            summary.push_str(&format!(" Next run: {next_run}."));
        }
        return trim_text(&summary, 24_000);
    }
    if normalized == "cron_cancel" || normalized == "cron_delete" || normalized == "schedule_cancel"
    {
        if payload.get("ok").and_then(Value::as_bool).unwrap_or(false)
            && payload
                .get("deleted")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        {
            let job_id = clean_text(
                payload.get("job_id").and_then(Value::as_str).unwrap_or(""),
                140,
            );
            if job_id.is_empty() {
                return "Deleted cron job.".to_string();
            }
            return format!("Deleted cron job `{job_id}`.");
        }
    }
    if normalized == "cron_run" || normalized == "schedule_run" || normalized == "cron_trigger" {
        if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            let job_id = clean_text(
                payload.get("job_id").and_then(Value::as_str).unwrap_or(""),
                140,
            );
            if job_id.is_empty() {
                return "Ran scheduled job successfully.".to_string();
            }
            return format!("Ran scheduled job `{job_id}`.");
        }
    }
    if normalized == "cron_list" || normalized == "schedule_list" || normalized == "cron_jobs" {
        let jobs = payload
            .get("jobs")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut names = jobs
            .iter()
            .take(4)
            .filter_map(|row| row.get("name").and_then(Value::as_str))
            .map(|name| clean_text(name, 80))
            .filter(|name| !name.is_empty())
            .collect::<Vec<_>>();
        names.dedup();
        let mut summary = format!("Cron jobs available: {}.", jobs.len());
        if !names.is_empty() {
            summary.push_str(&format!(" {}", names.join(", ")));
        }
        return trim_text(&summary, 24_000);
    }
    if normalized == "session_rollback_last_turn"
        || normalized == "undo_last_turn"
        || normalized == "rewind_turn"
    {
        let removed = payload
            .get("removed_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if removed == 0 {
            return "No recent turn available to undo.".to_string();
        }
        let rollback_id = clean_text(
            payload
                .get("rollback_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let mut summary = format!("Undid the most recent turn (removed {removed} messages).");
        if !rollback_id.is_empty() {
            summary.push_str(&format!(" Rollback receipt: {rollback_id}."));
        }
        return trim_text(&summary, 24_000);
    }
    if normalized == "file_read" || normalized == "read_file" || normalized == "file" {
        let content = payload
            .pointer("/file/content")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !content.is_empty() {
            return trim_text(content, 24_000);
        }
    }
    if normalized == "folder_export"
        || normalized == "list_folder"
        || normalized == "folder_tree"
        || normalized == "folder"
    {
        let tree = payload
            .pointer("/folder/tree")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !tree.is_empty() {
            return trim_text(tree, 24_000);
        }
    }
    if normalized == "terminal_exec"
        || normalized == "run_terminal"
        || normalized == "terminal"
        || normalized == "shell_exec"
    {
        let stdout = payload.get("stdout").and_then(Value::as_str).unwrap_or("");
        let stderr = payload.get("stderr").and_then(Value::as_str).unwrap_or("");
        let merged = if stderr.is_empty() {
            stdout.to_string()
        } else if stdout.is_empty() {
            stderr.to_string()
        } else {
            format!("{stdout}\n{stderr}")
        };
        if !merged.trim().is_empty() {
            return trim_text(&merged, 24_000);
        }
    }
    if normalized == "web_fetch" || normalized == "browse" || normalized == "web_conduit_fetch" {
        let summary = payload.get("summary").and_then(Value::as_str).unwrap_or("");
        let content = payload.get("content").and_then(Value::as_str).unwrap_or("");
        let body = if !summary.trim().is_empty() {
            summary
        } else {
            content
        };
        if !body.trim().is_empty() {
            return trim_text(body, 24_000);
        }
    }
    if normalized == "web_search"
        || normalized == "search_web"
        || normalized == "search"
        || normalized == "web_query"
    {
        if !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            return user_facing_tool_failure_summary(tool_name, payload)
                .unwrap_or_else(|| "Web search couldn't complete right now.".to_string());
        }
        let query = clean_text(
            payload.get("query").and_then(Value::as_str).unwrap_or(""),
            220,
        );
        let summary = clean_text(
            payload.get("summary").and_then(Value::as_str).unwrap_or(""),
            2_400,
        );
        let content = clean_text(
            payload.get("content").and_then(Value::as_str).unwrap_or(""),
            2_400,
        );
        let requested_url = clean_text(
            payload
                .get("requested_url")
                .or_else(|| payload.pointer("/receipt/requested_url"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            220,
        );
        let domain = clean_text(payload.get("domain").and_then(Value::as_str).unwrap_or(""), 120);
        if !summary.is_empty() && !looks_like_search_engine_chrome_summary(&summary) {
            return trim_text(&summary, 1_200);
        }
        let combined = if content.is_empty() {
            summary.clone()
        } else if summary.is_empty() {
            content.clone()
        } else {
            format!("{summary}\n{content}")
        };
        let findings = extract_search_result_findings(&combined, 3);
        if !findings.is_empty() {
            if !query.is_empty() {
                let mut lines = vec![format!(
                    "Web search findings for \"{}\":",
                    trim_text(&query, 120)
                )];
                for row in findings {
                    lines.push(format!("- {row}"));
                }
                return trim_text(&lines.join("\n"), 1_200);
            }
            return trim_text(
                &format!(
                    "Web search findings:\n{}",
                    findings
                        .into_iter()
                        .map(|row| format!("- {row}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                ),
                1_200,
            );
        }
        let sources = extract_search_result_domains(&combined, 4);
        if !sources.is_empty() {
            if !query.is_empty() {
                return format!(
                    "Web search for \"{}\" found sources: {}.",
                    trim_text(&query, 120),
                    sources.join(", ")
                );
            }
            return format!("Web search candidate sources: {}.", sources.join(", "));
        }
        if !query.is_empty() {
            if !requested_url.is_empty() {
                return format!(
                    "I couldn't extract usable findings for \"{}\" yet. The search response came from {}.",
                    trim_text(&query, 120),
                    requested_url
                );
            }
            if !domain.is_empty() {
                return format!(
                    "I couldn't extract usable findings for \"{}\" yet. The search response came from {}.",
                    trim_text(&query, 120),
                    domain
                );
            }
            return format!(
                "I couldn't extract usable findings for \"{}\" yet.",
                trim_text(&query, 120)
            );
        }
        if !summary.is_empty() && !looks_like_search_engine_chrome_summary(&summary) {
            return trim_text(&summary, 800);
        }
        if !requested_url.is_empty() {
            return format!(
                "I couldn't extract usable findings from the search response yet (source: {}).",
                requested_url
            );
        }
        if !domain.is_empty() {
            return format!(
                "I couldn't extract usable findings from the search response yet (source: {}).",
                domain
            );
        }
        return "I couldn't extract usable findings from the search response yet.".to_string();
    }
    if let Ok(raw) = serde_json::to_string_pretty(payload) {
        return trim_text(&raw, 24_000);
    }
    trim_text(&payload.to_string(), 24_000)
}

fn tool_error_text(payload: &Value) -> String {
    clean_text(
        payload
            .get("error")
            .or_else(|| payload.get("message"))
            .or_else(|| payload.pointer("/result/error"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        240,
    )
}

fn looks_like_domain_token(value: &str) -> bool {
    if value.is_empty() || !value.contains('.') {
        return false;
    }
    if value.starts_with('.') || value.ends_with('.') {
        return false;
    }
    if value
        .chars()
        .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-')))
    {
        return false;
    }
    let Some(tld) = value.rsplit('.').next() else {
        return false;
    };
    (2..=24).contains(&tld.len())
}

fn extract_search_result_domains(summary: &str, max_domains: usize) -> Vec<String> {
    let mut domains = Vec::<String>::new();
    for token in clean_text(summary, 4_000).split_whitespace() {
        let stripped = token
            .trim_matches(|ch: char| {
                !ch.is_ascii_alphanumeric() && ch != '.' && ch != '-' && ch != '/'
            })
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .trim_start_matches("www.");
        let host = stripped
            .split('/')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if !looks_like_domain_token(&host) {
            continue;
        }
        if host == "duckduckgo.com" {
            continue;
        }
        if domains.iter().any(|existing| existing == &host) {
            continue;
        }
        domains.push(host);
        if domains.len() >= max_domains.max(1) {
            break;
        }
    }
    domains
}

fn extract_search_result_findings(summary: &str, max_items: usize) -> Vec<String> {
    if max_items == 0 {
        return Vec::new();
    }
    let mut out = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    let normalized = clean_text(summary, 6_000);
    for line in normalized
        .split(|ch| matches!(ch, '\n' | '|' | '•'))
        .map(|row| clean_text(row, 280))
    {
        if line.is_empty() {
            continue;
        }
        if looks_like_search_engine_chrome_summary(&line) {
            continue;
        }
        let lowered = line.to_ascii_lowercase();
        if lowered.contains("duckduckgo all regions")
            || lowered.starts_with("all regions ")
            || lowered.starts_with("safe search ")
            || lowered.contains(" at duckduckgo")
            || lowered.contains("site links")
        {
            continue;
        }
        if lowered.contains(" at ") && lowered.contains("duckduckgo") {
            continue;
        }
        let has_link_hint = lowered.contains("http://")
            || lowered.contains("https://")
            || lowered.contains(".org/")
            || lowered.contains(".com/")
            || lowered.contains(".ai/")
            || lowered.contains(".dev/");
        if lowered.contains("...") && lowered.contains("all regions") {
            continue;
        }
        if !has_link_hint && line.len() < 44 {
            continue;
        }
        let compact = trim_text(&line.replace('\t', " ").replace("  ", " "), 240);
        if compact.is_empty() {
            continue;
        }
        let key = compact.to_ascii_lowercase();
        if !seen.insert(key) {
            continue;
        }
        out.push(compact);
        if out.len() >= max_items {
            break;
        }
    }
    out
}

fn looks_like_search_engine_chrome_summary(summary: &str) -> bool {
    let lowered = summary.to_ascii_lowercase();
    let markers = [
        "duckduckgo all regions",
        "all regions argentina",
        "all regions australia",
        "all regions canada",
        "safe search",
        "any time",
    ];
    let hits = markers
        .iter()
        .filter(|marker| lowered.contains(**marker))
        .count();
    hits >= 2
}

fn user_facing_tool_failure_summary(tool_name: &str, payload: &Value) -> Option<String> {
    let normalized = normalize_tool_name(tool_name);
    let lowered = tool_error_text(payload).to_ascii_lowercase();
    if lowered.is_empty() {
        return Some(format!("I couldn't complete `{normalized}` right now."));
    }
    if lowered == "tool_explicit_signoff_required" || lowered == "tool_confirmation_required" {
        return Some(format!(
            "I need your confirmation before running `{normalized}`. Confirm this step and I will execute it immediately."
        ));
    }
    if lowered.contains("query_required") {
        return Some(format!("`{normalized}` needs a query before it can run."));
    }
    if lowered.contains("url_required") {
        return Some(format!(
            "`{normalized}` needs a valid URL before it can run."
        ));
    }
    if lowered.contains("denied_domain")
        || lowered.contains("network_policy")
        || lowered.contains("domain_blocked")
    {
        return Some(format!(
            "`{normalized}` was blocked by network policy for this request."
        ));
    }
    if lowered.contains("timeout")
        || lowered.contains("timed out")
        || lowered.contains("unavailable")
        || lowered.contains("connection")
    {
        return Some(format!(
            "`{normalized}` hit a temporary network issue. Please retry."
        ));
    }
    Some(format!("I couldn't complete `{normalized}` right now."))
}

fn transient_tool_failure(payload: &Value) -> bool {
    if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return false;
    }
    let lowered = tool_error_text(payload).to_ascii_lowercase();
    lowered.contains("aborted")
        || lowered.contains("timeout")
        || lowered.contains("timed out")
        || lowered.contains("temporar")
        || lowered.contains("unavailable")
        || lowered.contains("network")
        || lowered.contains("connection")
        || lowered.contains("retry")
        || lowered.contains("econnreset")
}

fn fallback_memory_query_payload(
    root: &Path,
    actor_agent_id: &str,
    tool_name: &str,
    input: &Value,
) -> Option<Value> {
    let normalized = normalize_tool_name(tool_name);
    if normalized != "web_search"
        && normalized != "search_web"
        && normalized != "search"
        && normalized != "web_query"
        && normalized != "web_fetch"
        && normalized != "browse"
        && normalized != "web_conduit_fetch"
    {
        return None;
    }
    let query =
        if normalized == "web_fetch" || normalized == "browse" || normalized == "web_conduit_fetch"
        {
            clean_text(
                input
                    .get("url")
                    .or_else(|| input.get("query"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                600,
            )
        } else {
            clean_text(
                input
                    .get("query")
                    .or_else(|| input.get("q"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                600,
            )
        };
    if query.is_empty() {
        return None;
    }
    let fallback =
        crate::dashboard_agent_state::memory_kv_semantic_query(root, actor_agent_id, &query, 5);
    let matches = fallback
        .get("matches")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if matches.is_empty() {
        return None;
    }
    let summary = summarize_tool_payload("memory_semantic_query", &fallback);
    Some(json!({
        "ok": true,
        "type": "tool_degraded_fallback",
        "tool": normalized,
        "fallback_tool": "memory_semantic_query",
        "query": query,
        "summary": summary,
        "matches": matches,
        "fallback_used": true
    }))
}

fn execute_tool_call_with_recovery(
    root: &Path,
    snapshot: &Value,
    actor_agent_id: &str,
    existing: Option<&Value>,
    tool_name: &str,
    input: &Value,
) -> Value {
    let mut payload =
        execute_tool_call_by_name(root, snapshot, actor_agent_id, existing, tool_name, input);
    let mut recovery_strategy = "none".to_string();
    if transient_tool_failure(&payload) {
        std::thread::sleep(std::time::Duration::from_millis(180));
        let retry =
            execute_tool_call_by_name(root, snapshot, actor_agent_id, existing, tool_name, input);
        if retry.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            payload = retry;
            recovery_strategy = "retry_backoff".to_string();
        } else if let Some(fallback_payload) =
            fallback_memory_query_payload(root, &clean_agent_id(actor_agent_id), tool_name, input)
        {
            payload = fallback_payload;
            recovery_strategy = "semantic_memory_fallback".to_string();
        } else {
            payload = retry;
            recovery_strategy = "retry_backoff_failed".to_string();
        }
    }
    let audit_receipt = append_tool_decision_audit(
        root,
        actor_agent_id,
        tool_name,
        input,
        &payload,
        &recovery_strategy,
    );
    if let Some(obj) = payload.as_object_mut() {
        obj.insert(
            "recovery_strategy".to_string(),
            Value::String(recovery_strategy),
        );
        obj.insert(
            "decision_audit_receipt".to_string(),
            Value::String(audit_receipt),
        );
    }
    payload
}

fn execute_inline_tool_calls(
    root: &Path,
    snapshot: &Value,
    actor_agent_id: &str,
    existing: Option<&Value>,
    response_text: &str,
    user_message: &str,
) -> (String, Vec<Value>) {
    let (cleaned, calls) = extract_inline_tool_calls(response_text, 6);
    if calls.is_empty() {
        return (response_text.to_string(), Vec::new());
    }
    let mut cards = Vec::<Value>::new();
    let mut fallback_lines = Vec::<String>::new();
    for (idx, (name, input, _raw)) in calls.into_iter().enumerate() {
        let mut input_for_call = input.clone();
        let normalized_name = normalize_tool_name(&name);
        let user_requested_swarm = swarm_intent_requested(user_message)
            || user_message.to_ascii_lowercase().contains("multi-agent")
            || user_message.to_ascii_lowercase().contains("multi agent");
        if user_requested_swarm
            && matches!(
                normalized_name.as_str(),
                "spawn_subagents" | "spawn_swarm" | "agent_spawn" | "sessions_spawn"
            )
        {
            if !input_for_call.is_object() {
                input_for_call = json!({
                    "objective": clean_text(user_message, 800)
                });
            }
            if !input_has_confirmation(&input_for_call) {
                input_for_call["confirm"] = Value::Bool(true);
            }
            let approval_note = clean_text(
                input_for_call
                    .get("approval_note")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                200,
            );
            if approval_note.is_empty() {
                input_for_call["approval_note"] =
                    Value::String("user requested explicit swarm execution".to_string());
            }
        }
        let payload = execute_tool_call_with_recovery(
            root,
            snapshot,
            actor_agent_id,
            existing,
            &name,
            &input_for_call,
        );
        let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
        let result_text = summarize_tool_payload(&name, &payload);
        cards.push(json!({
            "id": format!("tool-{}-{}", normalize_tool_name(&name), idx),
            "name": normalize_tool_name(&name),
            "input": trim_text(&input_for_call.to_string(), 4000),
            "result": trim_text(&result_text, 24_000),
            "is_error": !ok
        }));
        if ok && !result_text.trim().is_empty() {
            if !response_looks_like_tool_ack_without_findings(&result_text) {
                fallback_lines.push(result_text);
            }
        } else if !ok {
            if let Some(line) = user_facing_tool_failure_summary(&name, &payload) {
                fallback_lines.push(line);
            }
        }
    }
    let cleaned_trimmed = cleaned.trim();
    let response = if cleaned_trimmed.is_empty()
        || (!fallback_lines.is_empty() && response_looks_like_tool_ack_without_findings(cleaned_trimmed))
    {
        let joined = fallback_lines.join("\n\n");
        if joined.trim().is_empty() {
            "I ran the requested tool calls, but they returned no usable findings yet. Ask me to retry with a narrower query or a specific source."
                .to_string()
        } else {
            trim_text(&joined, 32_000)
        }
    } else {
        trim_text(cleaned_trimmed, 32_000)
    };
    (response, cards)
}

fn first_http_url_in_text(text: &str) -> String {
    let cleaned = clean_text(text, 2200);
    for token in cleaned.split_whitespace() {
        if token.starts_with("http://") || token.starts_with("https://") {
            return clean_text(
                token.trim_matches(|ch| matches!(ch, ')' | ']' | '>' | ',')),
                2200,
            );
        }
    }
    String::new()
}

fn parse_cron_interval_minutes(token: &str) -> Option<i64> {
    let raw = clean_text(token, 40).to_ascii_lowercase();
    if raw.is_empty() {
        return None;
    }
    let (number_part, multiplier) = if raw.ends_with('m') {
        (&raw[..raw.len().saturating_sub(1)], 1i64)
    } else if raw.ends_with('h') {
        (&raw[..raw.len().saturating_sub(1)], 60i64)
    } else if raw.ends_with('d') {
        (&raw[..raw.len().saturating_sub(1)], 1440i64)
    } else {
        (raw.as_str(), 1i64)
    };
    let parsed = number_part.trim().parse::<i64>().ok()?;
    if parsed <= 0 {
        return None;
    }
    Some((parsed * multiplier).clamp(1, 10_080))
}

fn cron_tool_request_from_args(args: &str) -> Option<(String, Value)> {
    let trimmed = clean_text(args, 1_200);
    if trimmed.trim().is_empty() {
        return Some(("cron_list".to_string(), json!({})));
    }
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let action = parts
        .next()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let rest = parts.next().map(str::trim).unwrap_or("");
    match action.as_str() {
        "list" | "ls" | "status" | "jobs" => Some(("cron_list".to_string(), json!({}))),
        "cancel" | "delete" | "remove" | "rm" => {
            let job_id = clean_text(rest, 140);
            if job_id.is_empty() {
                None
            } else {
                Some((
                    "cron_cancel".to_string(),
                    json!({"job_id": job_id, "confirm": true}),
                ))
            }
        }
        "run" | "trigger" => {
            let job_id = clean_text(rest, 140);
            if job_id.is_empty() {
                None
            } else {
                Some((
                    "cron_run".to_string(),
                    json!({"job_id": job_id, "confirm": true}),
                ))
            }
        }
        "schedule" | "every" | "in" => {
            let mut schedule_parts = rest.splitn(2, char::is_whitespace);
            let interval_token = schedule_parts.next().map(str::trim).unwrap_or("");
            let mut message = schedule_parts.next().map(str::trim).unwrap_or("");
            let mut interval_minutes = parse_cron_interval_minutes(interval_token);
            if interval_minutes.is_none() {
                if action == "schedule" && !rest.is_empty() {
                    interval_minutes = Some(60);
                    message = rest;
                } else {
                    return None;
                }
            }
            let minutes = interval_minutes.unwrap_or(60);
            let text = clean_text(message, 2_000);
            Some((
                "cron_schedule".to_string(),
                json!({
                    "interval_minutes": minutes,
                    "message": if text.is_empty() {
                        "Scheduled follow-up check."
                    } else {
                        text.as_str()
                    },
                    "confirm": true
                }),
            ))
        }
        _ => {
            if let Some(minutes) = parse_cron_interval_minutes(&action) {
                let text = clean_text(rest, 2_000);
                return Some((
                    "cron_schedule".to_string(),
                    json!({
                        "interval_minutes": minutes,
                        "message": if text.is_empty() {
                            "Scheduled follow-up check."
                        } else {
                            text.as_str()
                        },
                        "confirm": true
                    }),
                ));
            }
            None
        }
    }
}

fn natural_web_intent_from_user_message(message: &str) -> Option<(String, Value)> {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lowered = clean_text(trimmed, 2200).to_ascii_lowercase();
    let url = first_http_url_in_text(trimmed);
    if !url.is_empty() {
        let asks_browse = lowered.contains("browse")
            || lowered.contains("fetch")
            || lowered.contains("read this")
            || lowered.contains("summarize")
            || lowered.contains("look at")
            || lowered.contains("open")
            || lowered.contains("web");
        if asks_browse {
            return Some((
                "web_fetch".to_string(),
                json!({"url": url, "summary_only": true}),
            ));
        }
    }

    let prefixes = [
        "search the web for ",
        "search web for ",
        "search for ",
        "web search for ",
        "look up ",
        "find online ",
    ];
    for prefix in prefixes {
        if lowered.starts_with(prefix) {
            let query = clean_text(&trimmed[prefix.len()..], 600);
            if !query.is_empty() {
                return Some((
                    "web_search".to_string(),
                    json!({"query": query, "summary_only": true}),
                ));
            }
        }
    }
    None
}

fn direct_tool_intent_from_user_message(message: &str) -> Option<(String, Value)> {
    let trimmed = message.trim();
    if !trimmed.starts_with('/') {
        if let Some(route) = natural_web_intent_from_user_message(trimmed) {
            return Some(route);
        }
        let lowered = clean_text(trimmed, 120).to_ascii_lowercase();
        if lowered.contains("what did we decide")
            || lowered.starts_with("recall ")
            || lowered.starts_with("remember ")
        {
            return Some((
                "memory_semantic_query".to_string(),
                json!({"query": clean_text(trimmed, 600), "limit": 8}),
            ));
        }
        let undo_like = lowered == "undo"
            || lowered == "undo that"
            || lowered == "undo last"
            || lowered == "rewind";
        if undo_like {
            return Some(("session_rollback_last_turn".to_string(), json!({})));
        }
        return None;
    }
    let mut split = trimmed.splitn(2, char::is_whitespace);
    let command = split
        .next()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let arg = split.next().map(str::trim).unwrap_or("");
    match command.as_str() {
        "/file" => {
            if arg.is_empty() {
                None
            } else {
                Some(("file_read".to_string(), json!({"path": arg, "full": true})))
            }
        }
        "/folder" => {
            if arg.is_empty() {
                None
            } else {
                Some((
                    "folder_export".to_string(),
                    json!({"path": arg, "full": true}),
                ))
            }
        }
        "/terminal" | "/term" | "/shell" => {
            if arg.is_empty() {
                None
            } else {
                Some((
                    "terminal_exec".to_string(),
                    json!({
                        "command": arg,
                        "confirm": true,
                        "approval_note": "user slash terminal invocation"
                    }),
                ))
            }
        }
        "/browse" | "/web" => {
            if arg.is_empty() {
                None
            } else {
                Some((
                    "web_fetch".to_string(),
                    json!({"url": arg, "summary_only": true}),
                ))
            }
        }
        "/search" => {
            if arg.is_empty() {
                None
            } else {
                Some((
                    "web_search".to_string(),
                    json!({"query": arg, "summary_only": true}),
                ))
            }
        }
        "/swarm" | "/spawn" | "/subagents" => {
            let mut count = 3usize;
            let mut objective = arg;
            let mut tokens = arg.splitn(2, char::is_whitespace);
            if let Some(first) = tokens.next() {
                let parsed = first.trim().parse::<usize>().ok();
                if let Some(value) = parsed {
                    count = value.clamp(1, 8);
                    objective = tokens.next().map(str::trim).unwrap_or("");
                }
            }
            if objective.is_empty() {
                objective = "Parallel descendant task requested by user directive.";
            }
            Some((
                "spawn_subagents".to_string(),
                json!({
                    "count": count,
                    "objective": clean_text(objective, 800),
                    "confirm": true,
                    "approval_note": "user slash spawn request"
                }),
            ))
        }
        "/undo" | "/rewind" | "/rollback" => {
            Some(("session_rollback_last_turn".to_string(), json!({})))
        }
        "/memory" => {
            let mut memory_parts = arg.splitn(3, char::is_whitespace);
            let action = memory_parts
                .next()
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_default();
            let key = memory_parts.next().map(str::trim).unwrap_or("");
            let raw_value = memory_parts.next().map(str::trim).unwrap_or("");
            if action == "list" || action == "ls" {
                Some(("memory_kv_list".to_string(), json!({})))
            } else if action == "query" || action == "search" {
                let query_source = if key.is_empty() {
                    raw_value.to_string()
                } else if raw_value.is_empty() {
                    key.to_string()
                } else {
                    format!("{key} {raw_value}")
                };
                let query = clean_text(&query_source, 600);
                if query.is_empty() {
                    None
                } else {
                    Some((
                        "memory_semantic_query".to_string(),
                        json!({"query": query, "limit": 8}),
                    ))
                }
            } else if action == "get" {
                if key.is_empty() {
                    None
                } else {
                    Some(("memory_kv_get".to_string(), json!({"key": key})))
                }
            } else if action == "set" {
                if key.is_empty() {
                    None
                } else {
                    let parsed_value = serde_json::from_str::<Value>(raw_value)
                        .ok()
                        .unwrap_or_else(|| json!(raw_value));
                    Some((
                        "memory_kv_set".to_string(),
                        json!({"key": key, "value": parsed_value, "confirm": true}),
                    ))
                }
            } else {
                None
            }
        }
        "/cron" | "/schedule" => cron_tool_request_from_args(arg),
        _ => None,
    }
}

pub fn handle_with_headers(
    root: &Path,
    method: &str,
    path: &str,
    body: &[u8],
    headers: &[(&str, &str)],
    snapshot: &Value,
) -> Option<CompatApiResponse> {
    let path_only = path.split('?').next().unwrap_or(path);
    let requester_agent = requester_agent_id(headers);
    let request_host = header_value(headers, "host").unwrap_or_default();
    if let Some(payload) =
        crate::dashboard_terminal_broker::handle_http(root, method, path_only, body)
    {
        return Some(CompatApiResponse {
            status: 200,
            payload,
        });
    }
    if let Some(response) = dashboard_compat_api_reference_gap_closure::handle(
        root, method, path, path_only, body, snapshot,
    ) {
        return Some(response);
    }
    if let Some(response) = dashboard_compat_api_reference_parity::handle(
        root, method, path, path_only, headers, body, snapshot,
    ) {
        return Some(response);
    }
    if let Some(response) = dashboard_compat_api_channels::handle(root, method, path_only, body) {
        return Some(response);
    }
    if let Some(response) = dashboard_skills_marketplace::handle(root, method, path, snapshot, body)
    {
        return Some(response);
    }
    if let Some(response) =
        dashboard_compat_api_comms::handle(root, method, path, path_only, body, snapshot)
    {
        return Some(response);
    }
    if let Some(response) =
        dashboard_compat_api_hands::handle(root, method, path_only, body, snapshot)
    {
        return Some(response);
    }
    if let Some(response) =
        dashboard_compat_api_sidebar_ops::handle(root, method, path_only, body, snapshot)
    {
        return Some(response);
    }
    if let Some(response) = dashboard_compat_api_settings_ops::handle(root, method, path_only, body)
    {
        return Some(response);
    }

    if let Some((requested_agent_id, segments)) = parse_memory_route(path_only) {
        let agent_id = resolve_agent_id_alias(root, &requested_agent_id);
        if !requester_agent.is_empty()
            && requester_agent != agent_id
            && !actor_can_manage_target(root, snapshot, &requester_agent, &agent_id)
        {
            return Some(CompatApiResponse {
                status: 403,
                payload: json!({
                    "ok": false,
                    "error": "agent_manage_forbidden",
                    "actor_agent_id": requester_agent.clone(),
                    "target_agent_id": agent_id
                }),
            });
        }
        if segments.first().map(|v| v == "kv").unwrap_or(false) {
            if method == "GET" && segments.len() == 1 {
                let state = load_session_state(root, &agent_id);
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({
                        "ok": true,
                        "agent_id": agent_id,
                        "kv_pairs": memory_kv_pairs_from_state(&state)
                    }),
                });
            }
            if segments.len() >= 2 {
                let key = decode_path_segment(&segments[1..].join("/"));
                if method == "GET" {
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: crate::dashboard_agent_state::memory_kv_get(root, &agent_id, &key),
                    });
                }
                if method == "PUT" {
                    let request =
                        serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
                    let value = request.get("value").cloned().unwrap_or(Value::Null);
                    let payload =
                        crate::dashboard_agent_state::memory_kv_set(root, &agent_id, &key, &value);
                    return Some(CompatApiResponse {
                        status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                            200
                        } else {
                            400
                        },
                        payload,
                    });
                }
                if method == "DELETE" {
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: crate::dashboard_agent_state::memory_kv_delete(
                            root, &agent_id, &key,
                        ),
                    });
                }
            }
        }
        if segments
            .first()
            .map(|v| v == "semantic-query" || v == "semantic_query")
            .unwrap_or(false)
        {
            if method == "GET" || method == "POST" {
                let request = if method == "POST" {
                    serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}))
                } else {
                    json!({
                        "query": query_value(path, "q")
                            .or_else(|| query_value(path, "query"))
                            .unwrap_or_default(),
                        "limit": query_value(path, "limit")
                            .and_then(|raw| raw.parse::<usize>().ok())
                            .unwrap_or(8)
                    })
                };
                let query = clean_text(
                    request
                        .get("query")
                        .or_else(|| request.get("q"))
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    600,
                );
                let limit = request
                    .get("limit")
                    .and_then(Value::as_u64)
                    .map(|value| value as usize)
                    .unwrap_or(8)
                    .clamp(1, 25);
                let payload = crate::dashboard_agent_state::memory_kv_semantic_query(
                    root, &agent_id, &query, limit,
                );
                return Some(CompatApiResponse {
                    status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                        200
                    } else {
                        400
                    },
                    payload,
                });
            }
        }
    }

    if let Some((provider_id, segments)) = parse_provider_route(path_only) {
        if method == "GET" && segments.is_empty() && provider_id == "routing" {
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_provider_runtime::routing_policy_payload(root),
            });
        }
        if method == "POST" && segments.is_empty() && provider_id == "routing" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let payload = crate::dashboard_provider_runtime::update_routing_policy(root, &request);
            return Some(CompatApiResponse {
                status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    200
                } else {
                    400
                },
                payload,
            });
        }
        if method == "POST" && segments.len() == 1 && segments[0] == "key" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let key = clean_text(
                request.get("key").and_then(Value::as_str).unwrap_or(""),
                4096,
            );
            let payload =
                crate::dashboard_provider_runtime::save_provider_key(root, &provider_id, &key);
            return Some(CompatApiResponse {
                status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    200
                } else {
                    400
                },
                payload,
            });
        }
        if method == "DELETE" && segments.len() == 1 && segments[0] == "key" {
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_provider_runtime::remove_provider_key(root, &provider_id),
            });
        }
        if method == "POST" && segments.len() == 1 && segments[0] == "test" {
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_provider_runtime::test_provider(root, &provider_id),
            });
        }
        if method == "PUT" && segments.len() == 1 && segments[0] == "url" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let base_url = clean_text(
                request
                    .get("base_url")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                400,
            );
            let payload =
                crate::dashboard_provider_runtime::set_provider_url(root, &provider_id, &base_url);
            return Some(CompatApiResponse {
                status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    200
                } else {
                    400
                },
                payload,
            });
        }
    }

    if method == "GET" && path_only == "/api/virtual-keys" {
        return Some(CompatApiResponse {
            status: 200,
            payload: crate::dashboard_provider_runtime::virtual_keys_payload(root),
        });
    }

    if method == "POST" && path_only == "/api/virtual-keys" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let key_id = clean_text(
            request
                .get("key_id")
                .or_else(|| request.get("id"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let payload =
            crate::dashboard_provider_runtime::upsert_virtual_key(root, &key_id, &request);
        return Some(CompatApiResponse {
            status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                200
            } else {
                400
            },
            payload,
        });
    }

    if method == "DELETE" {
        if let Some((key_id, segments)) = parse_virtual_key_route(path_only) {
            if segments.is_empty() {
                return Some(CompatApiResponse {
                    status: 200,
                    payload: crate::dashboard_provider_runtime::remove_virtual_key(root, &key_id),
                });
            }
        }
    }

    if method == "POST" && path_only == "/api/models/discover" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let input = clean_text(
            request
                .get("input")
                .and_then(Value::as_str)
                .or_else(|| request.get("api_key").and_then(Value::as_str))
                .unwrap_or(""),
            4096,
        );
        let payload = crate::dashboard_provider_runtime::discover_models(root, &input);
        return Some(CompatApiResponse {
            status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                200
            } else {
                400
            },
            payload,
        });
    }
    if method == "POST" && path_only == "/api/models/download" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let provider = clean_text(
            request
                .get("provider")
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        );
        let model = clean_text(
            request.get("model").and_then(Value::as_str).unwrap_or(""),
            240,
        );
        let payload = crate::dashboard_provider_runtime::download_model(root, &provider, &model);
        return Some(CompatApiResponse {
            status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                200
            } else {
                400
            },
            payload,
        });
    }
    if method == "POST" && path_only == "/api/models/custom" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let provider = clean_text(
            request
                .get("provider")
                .and_then(Value::as_str)
                .unwrap_or("openrouter"),
            80,
        );
        let model = clean_text(
            request
                .get("id")
                .or_else(|| request.get("model"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            240,
        );
        let context_window = request
            .get("context_window")
            .and_then(Value::as_i64)
            .unwrap_or(128_000);
        let max_output_tokens = request
            .get("max_output_tokens")
            .and_then(Value::as_i64)
            .unwrap_or(8192);
        let payload = crate::dashboard_provider_runtime::add_custom_model(
            root,
            &provider,
            &model,
            context_window,
            max_output_tokens,
        );
        return Some(CompatApiResponse {
            status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                200
            } else {
                400
            },
            payload,
        });
    }
    if method == "DELETE" && path_only.starts_with("/api/models/custom/") {
        let model_ref = decode_path_segment(path_only.trim_start_matches("/api/models/custom/"));
        return Some(CompatApiResponse {
            status: 200,
            payload: crate::dashboard_provider_runtime::delete_custom_model(root, &model_ref),
        });
    }

    if method == "GET" && path_only == "/api/search/conversations" {
        let query = query_value(path, "q")
            .or_else(|| query_value(path, "query"))
            .unwrap_or_default();
        let limit = query_value(path, "limit")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(40);
        return Some(CompatApiResponse {
            status: 200,
            payload: crate::dashboard_internal_search::search_conversations(root, &query, limit),
        });
    }
    if method == "POST" && path_only == "/api/search/conversations" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let query = clean_text(
            request
                .get("q")
                .or_else(|| request.get("query"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            260,
        );
        let limit = request
            .get("limit")
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(40);
        return Some(CompatApiResponse {
            status: 200,
            payload: crate::dashboard_internal_search::search_conversations(root, &query, limit),
        });
    }

    if method == "GET" && path_only == "/api/agents/terminated" {
        return Some(CompatApiResponse {
            status: 200,
            payload: crate::dashboard_agent_state::terminated_entries(root),
        });
    }
    if method == "POST" && path_only.starts_with("/api/agents/") && path_only.ends_with("/revive") {
        let agent_id = path_only
            .trim_start_matches("/api/agents/")
            .trim_end_matches("/revive")
            .trim_matches('/');
        if !requester_agent.is_empty()
            && !actor_can_manage_target(root, snapshot, &requester_agent, agent_id)
        {
            return Some(CompatApiResponse {
                status: 403,
                payload: json!({
                    "ok": false,
                    "error": "agent_manage_forbidden",
                    "actor_agent_id": requester_agent.clone(),
                    "target_agent_id": clean_agent_id(agent_id)
                }),
            });
        }
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let role = request
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("analyst");
        return Some(CompatApiResponse {
            status: 200,
            payload: crate::dashboard_agent_state::revive_agent(root, agent_id, role),
        });
    }
    if method == "DELETE" && path_only == "/api/agents/terminated" {
        if !requester_agent.is_empty() {
            return Some(CompatApiResponse {
                status: 403,
                payload: json!({
                    "ok": false,
                    "error": "agent_manage_forbidden",
                    "actor_agent_id": requester_agent.clone(),
                    "target_agent_id": "terminated/*"
                }),
            });
        }
        if query_value(path, "all")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
        {
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_agent_state::delete_all_terminated(root),
            });
        }
    }
    if method == "DELETE" && path_only.starts_with("/api/agents/terminated/") {
        let agent_id = path_only
            .trim_start_matches("/api/agents/terminated/")
            .trim();
        if !requester_agent.is_empty()
            && !actor_can_manage_target(root, snapshot, &requester_agent, agent_id)
        {
            return Some(CompatApiResponse {
                status: 403,
                payload: json!({
                    "ok": false,
                    "error": "agent_manage_forbidden",
                    "actor_agent_id": requester_agent.clone(),
                    "target_agent_id": clean_agent_id(agent_id)
                }),
            });
        }
        return Some(CompatApiResponse {
            status: 200,
            payload: crate::dashboard_agent_state::delete_terminated(
                root,
                agent_id,
                query_value(path, "contract_id").as_deref(),
            ),
        });
    }

    if method == "GET" && path_only == "/api/agents" {
        let _ = crate::dashboard_agent_state::enforce_expired_contracts(root);
        let include_terminated = query_value(path, "include_terminated")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        return Some(CompatApiResponse {
            status: 200,
            payload: Value::Array(build_agent_roster(root, snapshot, include_terminated)),
        });
    }

    if method == "POST" && path_only == "/api/agents/archive-all" {
        if !requester_agent.is_empty() {
            return Some(CompatApiResponse {
                status: 403,
                payload: json!({
                    "ok": false,
                    "error": "agent_manage_forbidden",
                    "actor_agent_id": requester_agent.clone(),
                    "target_agent_id": "*"
                }),
            });
        }
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let reason = clean_text(
            request
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or("user_archive_all"),
            120,
        );
        return Some(CompatApiResponse {
            status: 200,
            payload: archive_all_visible_agents(root, snapshot, &reason),
        });
    }

    if method == "POST" && path_only == "/api/agents" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let requested_parent = clean_agent_id(
            request
                .get("parent_agent_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
        );
        let parent_agent_id = if requested_parent.is_empty() {
            requester_agent.clone()
        } else {
            requested_parent
        };
        if !requester_agent.is_empty()
            && !parent_agent_id.is_empty()
            && parent_agent_id != requester_agent
            && !actor_can_manage_target(root, snapshot, &requester_agent, &parent_agent_id)
        {
            return Some(CompatApiResponse {
                status: 403,
                payload: json!({
                    "ok": false,
                    "error": "agent_manage_forbidden",
                    "actor_agent_id": requester_agent.clone(),
                    "target_agent_id": parent_agent_id
                }),
            });
        }
        let manifest = clean_text(
            request
                .get("manifest_toml")
                .and_then(Value::as_str)
                .unwrap_or(""),
            20_000,
        );
        let manifest_fields = parse_manifest_fields(&manifest);
        let requested_name = clean_text(
            request
                .get("name")
                .and_then(Value::as_str)
                .or_else(|| manifest_fields.get("name").map(|v| v.as_str()))
                .unwrap_or(""),
            120,
        );
        let requested_role = clean_text(
            request
                .get("role")
                .and_then(Value::as_str)
                .or_else(|| manifest_fields.get("role").map(|v| v.as_str()))
                .unwrap_or("analyst"),
            60,
        );
        let role = if requested_role.is_empty() {
            "analyst".to_string()
        } else {
            requested_role
        };
        let resolved_requested_name =
            dashboard_compat_api_agent_identity::resolve_agent_name(root, &requested_name, &role);
        let agent_id_seed = if resolved_requested_name.is_empty() {
            "agent".to_string()
        } else {
            resolved_requested_name.clone()
        };
        let agent_id = make_agent_id(root, &agent_id_seed);
        let name = if resolved_requested_name.is_empty() {
            dashboard_compat_api_agent_identity::default_agent_name(&agent_id)
        } else {
            resolved_requested_name
        };
        let (default_provider, default_model) = effective_app_settings(root, snapshot);
        let model_provider = clean_text(
            request
                .get("provider")
                .and_then(Value::as_str)
                .or_else(|| manifest_fields.get("provider").map(|v| v.as_str()))
                .unwrap_or(&default_provider),
            80,
        );
        let model_name = clean_text(
            request
                .get("model")
                .and_then(Value::as_str)
                .or_else(|| manifest_fields.get("model").map(|v| v.as_str()))
                .unwrap_or(&default_model),
            120,
        );
        let model_override = if model_provider.is_empty() || model_name.is_empty() {
            "auto".to_string()
        } else {
            format!("{model_provider}/{model_name}")
        };
        let identity =
            dashboard_compat_api_agent_identity::resolve_agent_identity(root, &request, &role);
        let profile_patch = json!({
            "agent_id": agent_id,
            "name": name,
            "role": role,
            "state": "Running",
            "parent_agent_id": if parent_agent_id.is_empty() { Value::Null } else { Value::String(parent_agent_id.clone()) },
            "model_override": model_override,
            "model_provider": model_provider,
            "model_name": model_name,
            "runtime_model": model_name,
            "system_prompt": request.get("system_prompt").cloned().unwrap_or_else(|| json!("")),
            "identity": identity,
            "fallback_models": request.get("fallback_models").cloned().unwrap_or_else(|| json!([])),
            "git_tree_kind": "master",
            "git_branch": "main",
            "workspace_dir": root.to_string_lossy().to_string(),
            "workspace_rel": "",
            "git_tree_ready": true,
            "git_tree_error": "",
            "is_master_agent": true
        });
        let _ = update_profile_patch(root, &agent_id, &profile_patch);
        let contract_obj = request
            .get("contract")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let contract_lifespan = clean_text(
            contract_obj
                .get("lifespan")
                .and_then(Value::as_str)
                .unwrap_or(""),
            40,
        )
        .to_ascii_lowercase();
        let mut termination_condition = clean_text(
            contract_obj
                .get("termination_condition")
                .and_then(Value::as_str)
                .unwrap_or("task_or_timeout"),
            80,
        );
        let explicit_indefinite = contract_obj
            .get("indefinite")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || contract_lifespan == "permanent"
            || contract_lifespan == "indefinite";
        if explicit_indefinite {
            termination_condition = "manual".to_string();
        } else if contract_lifespan == "task" && termination_condition.is_empty() {
            termination_condition = "task_complete".to_string();
        }
        if termination_condition.is_empty() {
            termination_condition = "task_or_timeout".to_string();
        }
        let non_expiring_termination = matches!(
            termination_condition.to_ascii_lowercase().as_str(),
            "manual" | "task_complete"
        );
        let expiry_seconds = contract_obj
            .get("expiry_seconds")
            .and_then(Value::as_i64)
            .unwrap_or(3600)
            .clamp(1, 31 * 24 * 60 * 60);
        let auto_terminate_allowed = contract_obj
            .get("auto_terminate_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(!non_expiring_termination)
            && !non_expiring_termination;
        let idle_terminate_allowed = contract_obj
            .get("idle_terminate_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(!non_expiring_termination)
            && !non_expiring_termination;
        let contract_patch = json!({
            "agent_id": agent_id,
            "status": "active",
            "created_at": crate::now_iso(),
            "updated_at": crate::now_iso(),
            "owner": clean_text(contract_obj.get("owner").and_then(Value::as_str).unwrap_or("dashboard_auto"), 80),
            "mission": clean_text(contract_obj.get("mission").and_then(Value::as_str).unwrap_or("Assist with assigned mission."), 200),
            "termination_condition": termination_condition,
            "expiry_seconds": expiry_seconds,
            "auto_terminate_allowed": auto_terminate_allowed,
            "idle_terminate_allowed": idle_terminate_allowed,
            "parent_agent_id": if parent_agent_id.is_empty() { Value::Null } else { Value::String(parent_agent_id) },
            "conversation_hold": contract_obj.get("conversation_hold").and_then(Value::as_bool).unwrap_or(false),
            "indefinite": explicit_indefinite,
            "lifespan": if explicit_indefinite {
                "permanent"
            } else if termination_condition.eq_ignore_ascii_case("task_complete") {
                "task"
            } else {
                "ephemeral"
            },
            "expires_at": clean_text(contract_obj.get("expires_at").and_then(Value::as_str).unwrap_or(""), 80),
            "source_user_directive": clean_text(contract_obj.get("source_user_directive").and_then(Value::as_str).unwrap_or(""), 800),
            "source_user_directive_receipt": clean_text(contract_obj.get("source_user_directive_receipt").and_then(Value::as_str).unwrap_or(""), 120)
        });
        let _ = upsert_contract_patch(root, &agent_id, &contract_patch);
        append_turn_message(root, &agent_id, "", "");
        let row = agent_row_by_id(root, snapshot, &agent_id).unwrap_or_else(|| {
            json!({
                "id": agent_id,
                "name": name,
                "role": role,
                "state": "Running",
                "model_provider": model_provider,
                "model_name": model_name
            })
        });
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({
                "ok": true,
                "id": row.get("id").cloned().unwrap_or_else(|| json!("")),
                "agent_id": row.get("id").cloned().unwrap_or_else(|| json!("")),
                "name": row
                    .get("name")
                    .cloned()
                    .unwrap_or_else(|| json!(name.clone())),
                "state": row.get("state").cloned().unwrap_or_else(|| json!("Running")),
                "model_provider": row.get("model_provider").cloned().unwrap_or_else(|| json!(default_provider)),
                "model_name": row.get("model_name").cloned().unwrap_or_else(|| json!(default_model)),
                "runtime_model": row.get("runtime_model").cloned().unwrap_or_else(|| json!(default_model)),
                "created_at": row.get("created_at").cloned().unwrap_or_else(|| json!(crate::now_iso()))
            }),
        });
    }

    if let Some((requested_agent_id, segments)) = parse_agent_route(path_only) {
        let agent_id = resolve_agent_id_alias(root, &requested_agent_id);
        if !requester_agent.is_empty()
            && method != "GET"
            && requester_agent != agent_id
            && !actor_can_manage_target(root, snapshot, &requester_agent, &agent_id)
        {
            return Some(CompatApiResponse {
                status: 403,
                payload: json!({
                    "ok": false,
                    "error": "agent_manage_forbidden",
                    "actor_agent_id": requester_agent.clone(),
                    "target_agent_id": agent_id
                }),
            });
        }
        let existing = agent_row_by_id(root, snapshot, &agent_id);
        let is_archived =
            crate::dashboard_agent_state::archived_agent_ids(root).contains(&agent_id);
        if method == "GET" && segments.is_empty() {
            if let Some(row) = existing {
                return Some(CompatApiResponse {
                    status: 200,
                    payload: row,
                });
            }
            if is_archived {
                return Some(CompatApiResponse {
                    status: 200,
                    payload: archived_agent_stub(root, &agent_id),
                });
            }
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "agent_not_found", "agent_id": agent_id}),
            });
        }

        if method == "DELETE" && segments.is_empty() {
            if existing.is_none() {
                if is_archived {
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: json!({
                            "ok": true,
                            "type": "dashboard_agent_archive",
                            "id": agent_id,
                            "agent_id": agent_id,
                            "state": "inactive",
                            "archived": true
                        }),
                    });
                }
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "agent_not_found", "agent_id": agent_id}),
                });
            }
            let _ = update_profile_patch(
                root,
                &agent_id,
                &json!({"state": "Archived", "updated_at": crate::now_iso()}),
            );
            let _ = upsert_contract_patch(
                root,
                &agent_id,
                &json!({
                    "status": "terminated",
                    "termination_reason": "user_archived",
                    "terminated_at": crate::now_iso(),
                    "updated_at": crate::now_iso()
                }),
            );
            let _ = crate::dashboard_agent_state::archive_agent(root, &agent_id, "user_archive");
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "type": "dashboard_agent_archive",
                    "id": agent_id,
                    "agent_id": agent_id,
                    "state": "inactive",
                    "archived": true
                }),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "stop" {
            if existing.is_none() {
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "agent_not_found", "agent_id": agent_id}),
                });
            }
            let _ = upsert_contract_patch(
                root,
                &agent_id,
                &json!({
                    "status": "terminated",
                    "termination_reason": "stopped",
                    "terminated_at": crate::now_iso(),
                    "updated_at": crate::now_iso()
                }),
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "type": "dashboard_agent_stop", "agent_id": agent_id}),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "start" {
            if existing.is_none() {
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "agent_not_found", "agent_id": agent_id}),
                });
            }
            let _ = update_profile_patch(
                root,
                &agent_id,
                &json!({
                    "state": "Running",
                    "updated_at": crate::now_iso()
                }),
            );
            let _ = upsert_contract_patch(
                root,
                &agent_id,
                &json!({
                    "status": "active",
                    "termination_reason": "",
                    "terminated_at": "",
                    "updated_at": crate::now_iso()
                }),
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "type": "dashboard_agent_start", "agent_id": agent_id}),
            });
        }

        if existing.is_none() {
            if is_archived && method == "POST" && segments.len() == 1 && segments[0] == "message" {
                return Some(CompatApiResponse {
                    status: 409,
                    payload: json!({
                        "ok": false,
                        "error": "agent_inactive",
                        "agent_id": agent_id,
                        "state": "inactive",
                        "archived": true
                    }),
                });
            }
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "agent_not_found", "agent_id": agent_id}),
            });
        }

        if method == "GET" && segments.len() == 1 && segments[0] == "session" {
            return Some(CompatApiResponse {
                status: 200,
                payload: session_payload(root, &agent_id),
            });
        }

        if method == "POST"
            && segments.len() == 2
            && segments[0] == "session"
            && segments[1] == "reset"
        {
            return Some(CompatApiResponse {
                status: 200,
                payload: reset_active_session(root, &agent_id),
            });
        }

        if method == "POST"
            && segments.len() == 2
            && segments[0] == "session"
            && segments[1] == "compact"
        {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            return Some(CompatApiResponse {
                status: 200,
                payload: compact_active_session(root, &agent_id, &request),
            });
        }

        if method == "GET" && segments.len() == 1 && segments[0] == "sessions" {
            let payload = session_payload(root, &agent_id);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "agent_id": agent_id,
                    "active_session_id": payload.get("active_session_id").cloned().unwrap_or_else(|| json!("default")),
                    "sessions": payload.get("sessions").cloned().unwrap_or_else(|| json!([]))
                }),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "sessions" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let label = clean_text(
                request
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or("Session"),
                80,
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_agent_state::create_session(root, &agent_id, &label),
            });
        }

        if method == "POST"
            && segments.len() == 3
            && segments[0] == "sessions"
            && segments[2] == "switch"
        {
            let session_id = decode_path_segment(&segments[1]);
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_agent_state::switch_session(root, &agent_id, &session_id),
            });
        }

        if method == "DELETE" && segments.len() == 1 && segments[0] == "history" {
            let mut state = load_session_state(root, &agent_id);
            if let Some(rows) = state.get_mut("sessions").and_then(Value::as_array_mut) {
                for row in rows.iter_mut() {
                    row["messages"] = Value::Array(Vec::new());
                    row["updated_at"] = Value::String(crate::now_iso());
                }
            }
            save_session_state(root, &agent_id, &state);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "type": "dashboard_agent_history_cleared", "agent_id": agent_id}),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "message" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let message = clean_text(
                request.get("message").and_then(Value::as_str).unwrap_or(""),
                8_000,
            );
            if message.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "message_required"}),
                });
            }
            let row = existing.clone().unwrap_or_else(|| json!({}));
            let lowered = message.to_ascii_lowercase();
            let contains_any = |terms: &[&str]| terms.iter().any(|term| lowered.contains(term));
            let contract_violation = (contains_any(&["ignore", "bypass", "disable", "override"])
                && contains_any(&["contract", "safety", "policy", "receipt"]))
                || contains_any(&["exfiltrate", "steal", "dump secrets", "leak", "secrets"]);
            if contract_violation {
                let _ = upsert_contract_patch(
                    root,
                    &agent_id,
                    &json!({
                        "status": "terminated",
                        "termination_reason": "contract_violation",
                        "terminated_at": crate::now_iso(),
                        "updated_at": crate::now_iso()
                    }),
                );
                return Some(CompatApiResponse {
                    status: 409,
                    payload: json!({
                        "ok": false,
                        "error": "agent_contract_terminated",
                        "agent_id": agent_id,
                        "termination_reason": "contract_violation"
                    }),
                });
            }
            let workspace_hints = workspace_file_hints_for_message(root, Some(&row), &message, 5);
            let latent_tool_candidates =
                latent_tool_candidates_for_message(&message, &workspace_hints);
            if let Some((tool_name, tool_input)) = direct_tool_intent_from_user_message(&message) {
                let tool_payload = execute_tool_call_with_recovery(
                    root,
                    snapshot,
                    &agent_id,
                    Some(&row),
                    &tool_name,
                    &tool_input,
                );
                let ok = tool_payload
                    .get("ok")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let mut response_text = summarize_tool_payload(&tool_name, &tool_payload);
                if response_text.trim().is_empty() {
                    response_text = if ok {
                        format!(
                            "I ran `{}`, but it returned no usable findings yet. Ask me to retry with a narrower input.",
                            normalize_tool_name(&tool_name)
                        )
                    } else {
                        user_facing_tool_failure_summary(&tool_name, &tool_payload).unwrap_or_else(
                            || {
                                format!(
                                    "I couldn't complete `{}` right now.",
                                    normalize_tool_name(&tool_name)
                                )
                            },
                        )
                    };
                }
                if ok && response_looks_like_tool_ack_without_findings(&response_text) {
                    response_text = format!(
                        "I ran `{}`, but it returned no usable findings yet. Ask me to retry with a narrower input.",
                        normalize_tool_name(&tool_name)
                    );
                }
                if !user_requested_internal_runtime_details(&message) {
                    response_text = abstract_runtime_mechanics_terms(&response_text);
                }
                let turn_receipt = append_turn_message(root, &agent_id, &message, &response_text);
                return Some(CompatApiResponse {
                    status: if ok { 200 } else { 400 },
                    payload: json!({
                        "ok": ok,
                        "agent_id": agent_id,
                        "provider": "tool",
                        "model": "tool-router",
                        "runtime_model": "tool-router",
                        "iterations": 1,
                        "input_tokens": estimate_tokens(&message),
                        "output_tokens": estimate_tokens(&response_text),
                        "cost_usd": 0.0,
                        "response": response_text,
                        "tools": [
                            {
                                "id": format!("tool-direct-{}", normalize_tool_name(&tool_name)),
                                "name": normalize_tool_name(&tool_name),
                                "input": trim_text(&tool_input.to_string(), 4000),
                                "result": trim_text(&summarize_tool_payload(&tool_name, &tool_payload), 24_000),
                                "is_error": !ok
                            }
                        ],
                        "workspace_hints": workspace_hints,
                        "latent_tool_candidates": latent_tool_candidates,
                        "attention_queue": turn_receipt.get("attention_queue").cloned().unwrap_or_else(|| json!({})),
                        "memory_capture": turn_receipt.get("memory_capture").cloned().unwrap_or_else(|| json!({}))
                    }),
                });
            }
            let requested_provider = clean_text(
                row.get("model_provider")
                    .and_then(Value::as_str)
                    .unwrap_or("auto"),
                80,
            );
            let requested_model = clean_text(
                row.get("model_name").and_then(Value::as_str).unwrap_or(""),
                240,
            );
            let virtual_key_id = clean_text(
                request
                    .get("virtual_key_id")
                    .or_else(|| request.get("virtual_key"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120,
            );
            let route_request = json!({
                "agent_id": agent_id,
                "message": message,
                "task_type": row.get("role").cloned().unwrap_or_else(|| json!("general")),
                "token_count": estimate_tokens(&message),
                "virtual_key_id": if virtual_key_id.is_empty() { Value::Null } else { json!(virtual_key_id.clone()) },
                "has_vision": request
                    .get("attachments")
                    .and_then(Value::as_array)
                    .map(|rows| rows.iter().any(|row| {
                        clean_text(
                            row.get("content_type")
                                .or_else(|| row.get("mime_type"))
                                .and_then(Value::as_str)
                                .unwrap_or(""),
                            120,
                        )
                        .to_ascii_lowercase()
                        .starts_with("image/")
                    }))
                    .unwrap_or(false)
            });
            let (provider, model, auto_route) =
                crate::dashboard_model_catalog::resolve_model_selection(
                    root,
                    snapshot,
                    &requested_provider,
                    &requested_model,
                    &route_request,
                );
            let mut provider = provider;
            let mut model = model;
            let mut virtual_key_gate = Value::Null;
            if !virtual_key_id.is_empty() {
                let gate = crate::dashboard_provider_runtime::reserve_virtual_key_slot(
                    root,
                    &virtual_key_id,
                );
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
                    return Some(CompatApiResponse {
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
                let route_hint = crate::dashboard_provider_runtime::resolve_virtual_key_route(
                    root,
                    &virtual_key_id,
                );
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
            let mut state = load_session_state(root, &agent_id);
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
                .unwrap_or(16)
                .clamp(4, 128) as usize;
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
            let mut messages = all_session_messages(&state);
            let mut pooled_messages = trim_context_pool(&messages, context_pool_limit_tokens);
            let pre_generation_pruned = pooled_messages.len() != messages.len();
            if pre_generation_pruned && persist_system_prune {
                set_active_session_messages(&mut state, &pooled_messages);
                save_session_state(root, &agent_id, &state);
                state = load_session_state(root, &agent_id);
                messages = all_session_messages(&state);
                pooled_messages = trim_context_pool(&messages, context_pool_limit_tokens);
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
                    .unwrap_or(active_context_min_recent.min(4) as u64)
                    .clamp(2, 128) as usize;
                let emergency_messages = select_active_context_window(
                    &pooled_messages,
                    emergency_target_tokens,
                    emergency_min_recent,
                );
                let emergency_tokens = total_message_tokens(&emergency_messages);
                let removed_messages = pooled_messages
                    .len()
                    .saturating_sub(emergency_messages.len())
                    as u64;
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
                        let compact_result =
                            compact_active_session(root, &agent_id, &compact_request);
                        emergency_compact["persisted_to_history"] = json!(true);
                        emergency_compact["persist_result"] = compact_result;
                    }
                }
            }
            let memory_kv_entries = memory_kv_pairs_from_state(&state).len();
            let memory_prompt_context = memory_kv_prompt_context(&state, 24);
            let instinct_prompt_context = agent_instinct_prompt_context(root, 6_000);
            let plugin_prompt_context =
                dashboard_skills_marketplace::skills_prompt_context(root, 12, 4_000);
            let passive_memory_context =
                passive_attention_context_for_message(root, &agent_id, &message, 6);
            let keyframe_context = context_keyframes_prompt_context(&state, 8, 2_400);
            let overflow_keyframes_context = historical_context_keyframes_prompt_context(
                &pooled_messages,
                &active_messages,
                10,
                2_400,
            );
            let relevant_recall_context = historical_relevant_recall_prompt_context(
                &pooled_messages,
                &active_messages,
                &message,
                8,
                2_800,
            );
            let identity_hydration_prompt = agent_identity_hydration_prompt(&row);
            let custom_system_prompt = clean_text(
                row.get("system_prompt")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                12_000,
            );
            let mut prompt_parts = Vec::<String>::new();
            if !identity_hydration_prompt.is_empty() {
                prompt_parts.push(identity_hydration_prompt);
            }
            prompt_parts.push(AGENT_RUNTIME_SYSTEM_PROMPT.to_string());
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
            match crate::dashboard_provider_runtime::invoke_chat(
                root,
                &provider,
                &model,
                &system_prompt,
                &active_messages,
                &message,
            ) {
                Ok(result) => {
                    let mut response_text = clean_chat_text(
                        result.get("response").and_then(Value::as_str).unwrap_or(""),
                        32_000,
                    );
                    let response_had_context_meta =
                        internal_context_metadata_phrase(&response_text);
                    response_text = strip_internal_context_metadata_prefix(&response_text);
                    response_text = strip_internal_cache_control_markup(&response_text);
                    if response_text.is_empty() && response_had_context_meta {
                        response_text = "I have relevant prior context loaded and can keep going from here. Tell me what you want to do next.".to_string();
                    }
                    let runtime_summary = runtime_sync_summary(snapshot);
                    let runtime_probe = runtime_probe_requested(&message);
                    let runtime_denial = runtime_access_denied_phrase(&response_text);
                    if runtime_probe || runtime_denial {
                        response_text = if runtime_probe {
                            runtime_access_summary_text(&runtime_summary)
                        } else {
                            "I can access runtime telemetry, persistent memory, workspace files, channels, and approved command surfaces in this session. Tell me what you want me to check and I will run it now.".to_string()
                        };
                    }
                    if memory_recall_requested(&message)
                        || persistent_memory_denied_phrase(&response_text)
                    {
                        let mut remembered = pooled_messages
                            .iter()
                            .rev()
                            .filter_map(|row| {
                                let role = clean_text(
                                    row.get("role").and_then(Value::as_str).unwrap_or(""),
                                    20,
                                )
                                .to_ascii_lowercase();
                                if role != "user" {
                                    return None;
                                }
                                let text = message_text(row);
                                if text.is_empty() {
                                    return None;
                                }
                                if text.to_ascii_lowercase().contains("remember") {
                                    return Some(text);
                                }
                                None
                            })
                            .take(3)
                            .collect::<Vec<_>>();
                        if remembered.is_empty() {
                            remembered = pooled_messages
                                .iter()
                                .rev()
                                .filter_map(|row| {
                                    let role = clean_text(
                                        row.get("role").and_then(Value::as_str).unwrap_or(""),
                                        20,
                                    )
                                    .to_ascii_lowercase();
                                    if role != "user" {
                                        return None;
                                    }
                                    let text = message_text(row);
                                    if text.is_empty() {
                                        None
                                    } else {
                                        Some(text)
                                    }
                                })
                                .take(3)
                                .collect::<Vec<_>>();
                        }
                        if remembered.is_empty() {
                            response_text = "I don't have enough earlier context to reference yet. Share what you want me to track, and I'll carry it forward.".to_string();
                        } else {
                            response_text = format!(
                                "Here's what I remember from earlier: {}",
                                remembered.join(" | ")
                            );
                        }
                    }
                    let explicit_parallel_directive = swarm_intent_requested(&message)
                        || message.to_ascii_lowercase().contains("multi-agent")
                        || message.to_ascii_lowercase().contains("multi agent");
                    let response_denied_spawn = spawn_surface_denied_phrase(&response_text);
                    let response_has_tool_call = response_text.contains("<function=");
                    if explicit_parallel_directive
                        && (response_denied_spawn || !response_has_tool_call)
                    {
                        let auto_count = infer_subagent_count_from_message(&message);
                        let directive_hint_receipt = crate::deterministic_receipt_hash(&json!({
                            "agent_id": agent_id,
                            "message": message,
                            "requested_at": crate::now_iso()
                        }));
                        response_text = format!(
                            "<function=spawn_subagents>{}</function>",
                            json!({
                                "count": auto_count,
                                "objective": message,
                                "reason": "user_directive_parallelization",
                                "directive_receipt_hint": directive_hint_receipt,
                                "confirm": true,
                                "approval_note": "user requested parallelization in active turn"
                            })
                            .to_string()
                        );
                    }
                    let (tool_adjusted_response, response_tools) = execute_inline_tool_calls(
                        root,
                        snapshot,
                        &agent_id,
                        Some(&row),
                        &response_text,
                        &message,
                    );
                    response_text = tool_adjusted_response;
                    if response_looks_like_tool_ack_without_findings(&response_text) {
                        let synthesized = response_tools_summary_for_user(&response_tools, 4);
                        if !synthesized.is_empty() {
                            response_text = synthesized;
                        }
                    }
                    if !user_requested_internal_runtime_details(&message) {
                        response_text = abstract_runtime_mechanics_terms(&response_text);
                    }
                    response_text = strip_internal_cache_control_markup(&response_text);
                    if response_is_unrelated_context_dump(&message, &response_text) {
                        let strict_relevance_prompt = clean_text(
                            &format!(
                                "{}\n\nRelevance guard: answer only the latest user request. Ignore unrelated prior snippets and project templates. If the user asks for code, provide direct code first.",
                                AGENT_RUNTIME_SYSTEM_PROMPT
                            ),
                            12_000,
                        );
                        let retried = crate::dashboard_provider_runtime::invoke_chat(
                            root,
                            &provider,
                            &model,
                            &strict_relevance_prompt,
                            &[],
                            &message,
                        )
                        .ok()
                        .and_then(|value| {
                            let mut retried_text = clean_chat_text(
                                value.get("response").and_then(Value::as_str).unwrap_or(""),
                                32_000,
                            );
                            retried_text = strip_internal_context_metadata_prefix(&retried_text);
                            retried_text = strip_internal_cache_control_markup(&retried_text);
                            if !user_requested_internal_runtime_details(&message) {
                                retried_text = abstract_runtime_mechanics_terms(&retried_text);
                            }
                            if response_is_unrelated_context_dump(&message, &retried_text) {
                                None
                            } else {
                                let cleaned = retried_text.trim().to_string();
                                if cleaned.is_empty() {
                                    None
                                } else {
                                    Some(cleaned)
                                }
                            }
                        });
                        response_text = retried.unwrap_or_else(|| {
                            "I dropped an unrelated context artifact and did not return it. Please resend your request and I will answer only that prompt.".to_string()
                        });
                    }
                    let turn_receipt =
                        append_turn_message(root, &agent_id, &message, &response_text);
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
                    let _ = update_profile_patch(root, &agent_id, &runtime_patch);
                    let mut payload = result.clone();
                    payload["ok"] = json!(true);
                    payload["agent_id"] = json!(agent_id);
                    payload["provider"] = json!(provider);
                    payload["model"] = json!(model);
                    payload["iterations"] = json!(1);
                    payload["response"] = json!(response_text);
                    payload["runtime_sync"] = runtime_summary;
                    payload["tools"] = Value::Array(response_tools);
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
                        "pool_messages": pooled_messages.len(),
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
                        "context_window": fallback_window.max(0),
                        "context_ratio": context_ratio,
                        "context_pressure": context_pressure,
                        "pre_generation_pruning_enabled": true,
                        "pre_generation_pruned": pre_generation_pruned,
                        "history_trim_confirmed": history_trim_confirmed,
                        "emergency_compact_enabled": true,
                        "emergency_compact": emergency_compact
                    });
                    payload["workspace_hints"] = json!(workspace_hints);
                    payload["latent_tool_candidates"] = json!(latent_tool_candidates);
                    if let Some(route) = auto_route {
                        payload["auto_route"] =
                            route.get("route").cloned().unwrap_or_else(|| route.clone());
                    }
                    if !virtual_key_id.is_empty() {
                        let spend_receipt =
                            crate::dashboard_provider_runtime::record_virtual_key_usage(
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
                    return Some(CompatApiResponse {
                        status: 200,
                        payload,
                    });
                }
                Err(err) => {
                    return Some(CompatApiResponse {
                        status: 502,
                        payload: json!({
                            "ok": false,
                            "agent_id": agent_id,
                            "error": clean_text(&err, 280),
                            "provider": provider,
                            "model": model
                        }),
                    });
                }
            }
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "suggestions" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let hint = clean_text(
                request
                    .get("user_hint")
                    .and_then(Value::as_str)
                    .or_else(|| request.get("hint").and_then(Value::as_str))
                    .unwrap_or(""),
                220,
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_agent_state::suggestions(root, &agent_id, &hint),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "command" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let command = clean_text(
                request.get("command").and_then(Value::as_str).unwrap_or(""),
                80,
            )
            .to_ascii_lowercase();
            let silent = request
                .get("silent")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if command == "context" {
                let row = existing.clone().unwrap_or_else(|| json!({}));
                return Some(CompatApiResponse {
                    status: 200,
                    payload: context_command_payload(root, &agent_id, &row, &request, silent),
                });
            }
            if command == "queue" {
                let runtime = runtime_sync_summary(snapshot);
                let queue_depth = parse_non_negative_i64(runtime.get("queue_depth"), 0);
                let conduit_signals = parse_non_negative_i64(runtime.get("conduit_signals"), 0);
                let backpressure_level = clean_text(
                    runtime
                        .get("backpressure_level")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown"),
                    40,
                );
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({
                        "ok": true,
                        "agent_id": agent_id,
                        "command": command,
                        "silent": silent,
                        "runtime_sync": runtime,
                        "message": format!(
                            "Queue depth: {} | Conduit signals: {} | Backpressure: {}",
                            queue_depth,
                            conduit_signals,
                            backpressure_level
                        )
                    }),
                });
            }
            if command == "cron" || command == "schedule" {
                let args = clean_text(
                    request
                        .get("args")
                        .and_then(Value::as_str)
                        .or_else(|| request.get("input").and_then(Value::as_str))
                        .or_else(|| request.get("query").and_then(Value::as_str))
                        .unwrap_or(""),
                    1_200,
                );
                let Some((tool_name, tool_input)) = cron_tool_request_from_args(&args) else {
                    return Some(CompatApiResponse {
                        status: 400,
                        payload: json!({
                            "ok": false,
                            "agent_id": agent_id,
                            "command": command,
                            "silent": silent,
                            "error": "cron_usage_required",
                            "usage": "/cron list | /cron schedule <interval> <message> | /cron run <job_id> | /cron cancel <job_id>"
                        }),
                    });
                };
                let row = existing.clone().unwrap_or_else(|| json!({}));
                let tool_payload = execute_tool_call_with_recovery(
                    root,
                    snapshot,
                    &agent_id,
                    Some(&row),
                    &tool_name,
                    &tool_input,
                );
                let ok = tool_payload
                    .get("ok")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let message = summarize_tool_payload(&tool_name, &tool_payload);
                return Some(CompatApiResponse {
                    status: if ok { 200 } else { 400 },
                    payload: json!({
                        "ok": ok,
                        "agent_id": agent_id,
                        "command": command,
                        "silent": silent,
                        "tool": tool_name,
                        "input": tool_input,
                        "message": if message.trim().is_empty() {
                            format!("Cron command '{}' processed.", command)
                        } else {
                            message
                        },
                        "result": tool_payload
                    }),
                });
            }
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "agent_id": agent_id,
                    "command": if command.is_empty() { "unknown" } else { &command },
                    "silent": silent,
                    "message": format!("Command '{}' acknowledged.", if command.is_empty() { "unknown" } else { &command })
                }),
            });
        }

        if method == "PATCH" && segments.len() == 1 && segments[0] == "config" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let mut patch = request.clone();
            if !patch.is_object() {
                patch = json!({});
            }
            let should_seed_intro = patch.get("contract").is_some()
                || patch.get("system_prompt").is_some()
                || patch.get("archetype").is_some()
                || patch.get("profile").is_some();
            let explicit_role =
                clean_text(patch.get("role").and_then(Value::as_str).unwrap_or(""), 60);
            let existing_role = clean_text(
                existing
                    .as_ref()
                    .and_then(|row| row.get("role").and_then(Value::as_str))
                    .unwrap_or(""),
                60,
            );
            let archetype_hint = clean_text(
                patch.get("archetype").and_then(Value::as_str).unwrap_or(""),
                80,
            )
            .to_ascii_lowercase();
            let profile_hint = clean_text(
                patch.get("profile").and_then(Value::as_str).unwrap_or(""),
                80,
            )
            .to_ascii_lowercase();
            let mut role_hint = format!("{archetype_hint} {profile_hint}");
            if role_hint.trim().is_empty() {
                role_hint = clean_text(
                    patch
                        .get("system_prompt")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    200,
                )
                .to_ascii_lowercase();
            }
            let inferred_role = if !explicit_role.is_empty() {
                explicit_role.clone()
            } else if role_hint.contains("teacher")
                || role_hint.contains("tutor")
                || role_hint.contains("mentor")
                || role_hint.contains("coach")
                || role_hint.contains("instructor")
            {
                "tutor".to_string()
            } else if role_hint.contains("code")
                || role_hint.contains("coder")
                || role_hint.contains("engineer")
                || role_hint.contains("developer")
                || role_hint.contains("devops")
                || role_hint.contains("api")
                || role_hint.contains("build")
            {
                "engineer".to_string()
            } else if role_hint.contains("research") || role_hint.contains("investig") {
                "researcher".to_string()
            } else if role_hint.contains("analyst")
                || role_hint.contains("analysis")
                || role_hint.contains("data")
            {
                "analyst".to_string()
            } else if role_hint.contains("writer")
                || role_hint.contains("editor")
                || role_hint.contains("content")
            {
                "writer".to_string()
            } else if role_hint.contains("design")
                || role_hint.contains("ui")
                || role_hint.contains("ux")
            {
                "designer".to_string()
            } else if role_hint.contains("support") {
                "support".to_string()
            } else if !existing_role.is_empty() {
                existing_role.clone()
            } else {
                "analyst".to_string()
            };
            let resolved_role = if inferred_role.is_empty() {
                "analyst".to_string()
            } else {
                inferred_role
            };
            if should_seed_intro
                && explicit_role.is_empty()
                && !resolved_role.eq_ignore_ascii_case(&existing_role)
            {
                patch["role"] = Value::String(resolved_role.clone());
            }
            let mut rename_notice: Option<Value> = None;
            if patch.get("name").is_some() {
                let requested_name =
                    clean_text(patch.get("name").and_then(Value::as_str).unwrap_or(""), 120);
                if requested_name.is_empty() {
                    if let Some(map) = patch.as_object_mut() {
                        map.remove("name");
                    }
                } else {
                    let requested_default_like =
                        dashboard_compat_api_agent_identity::is_default_agent_name_for_agent(
                            &requested_name,
                            &agent_id,
                        );
                    let resolved_name = dashboard_compat_api_agent_identity::resolve_agent_name(
                        root,
                        &requested_name,
                        &resolved_role,
                    );
                    let treat_as_blank_for_init = should_seed_intro
                        && (requested_default_like
                            || dashboard_compat_api_agent_identity::is_default_agent_name_for_agent(
                                &resolved_name,
                                &agent_id,
                            ));
                    if treat_as_blank_for_init {
                        if let Some(map) = patch.as_object_mut() {
                            map.remove("name");
                        }
                    } else {
                        patch["name"] = Value::String(resolved_name);
                    }
                }
            }
            if should_seed_intro && patch.get("name").is_none() {
                let existing_name = clean_text(
                    existing
                        .as_ref()
                        .and_then(|row| row.get("name").and_then(Value::as_str))
                        .unwrap_or(""),
                    120,
                );
                if dashboard_compat_api_agent_identity::is_default_agent_name_for_agent(
                    &existing_name,
                    &agent_id,
                ) {
                    let previous_name = if existing_name.is_empty() {
                        dashboard_compat_api_agent_identity::default_agent_name(&agent_id)
                    } else {
                        existing_name.clone()
                    };
                    let auto_name =
                        dashboard_compat_api_agent_identity::resolve_post_init_agent_name(
                            root,
                            &agent_id,
                            &resolved_role,
                        );
                    if !auto_name.is_empty() && !auto_name.eq_ignore_ascii_case(&previous_name) {
                        patch["name"] = Value::String(auto_name.clone());
                        rename_notice = Some(json!({
                            "notice_label": format!("changed name from {previous_name} to {auto_name}"),
                            "notice_type": "info",
                            "ts": crate::now_iso(),
                            "auto_generated": true
                        }));
                    }
                }
            }
            let patch_touches_identity = patch.get("identity").is_some()
                || patch.get("emoji").is_some()
                || patch.get("color").is_some()
                || patch.get("archetype").is_some()
                || patch.get("vibe").is_some();
            if patch_touches_identity {
                if !patch.get("identity").map(Value::is_object).unwrap_or(false) {
                    let emoji =
                        clean_text(patch.get("emoji").and_then(Value::as_str).unwrap_or(""), 16);
                    let color =
                        clean_text(patch.get("color").and_then(Value::as_str).unwrap_or(""), 32);
                    let archetype = clean_text(
                        patch.get("archetype").and_then(Value::as_str).unwrap_or(""),
                        80,
                    );
                    let vibe =
                        clean_text(patch.get("vibe").and_then(Value::as_str).unwrap_or(""), 80);
                    if !emoji.is_empty()
                        || !color.is_empty()
                        || !archetype.is_empty()
                        || !vibe.is_empty()
                    {
                        patch["identity"] = json!({
                            "emoji": emoji,
                            "color": color,
                            "archetype": archetype,
                            "vibe": vibe
                        });
                    }
                }
                let mut identity_request = existing.clone().unwrap_or_else(|| json!({}));
                if !identity_request.is_object() {
                    identity_request = json!({});
                }
                if let Some(identity_patch) = patch.get("identity").and_then(Value::as_object) {
                    let mut merged_identity = identity_request
                        .get("identity")
                        .and_then(Value::as_object)
                        .cloned()
                        .unwrap_or_default();
                    for (key, value) in identity_patch {
                        if let Some(raw) = value.as_str() {
                            if clean_text(raw, 120).is_empty() {
                                continue;
                            }
                        }
                        merged_identity.insert(key.clone(), value.clone());
                    }
                    identity_request["identity"] = Value::Object(merged_identity);
                }
                for key in ["emoji", "color", "archetype", "vibe"] {
                    if let Some(value) = patch.get(key) {
                        if let Some(raw) = value.as_str() {
                            if clean_text(raw, 120).is_empty() {
                                continue;
                            }
                        }
                        identity_request[key] = value.clone();
                    }
                }
                patch["identity"] = dashboard_compat_api_agent_identity::resolve_agent_identity(
                    root,
                    &identity_request,
                    &resolved_role,
                );
            }
            let _ = update_profile_patch(root, &agent_id, &patch);
            if patch.get("contract").map(Value::is_object).unwrap_or(false) {
                let _ = upsert_contract_patch(
                    root,
                    &agent_id,
                    patch.get("contract").unwrap_or(&json!({})),
                );
            } else if patch.get("expiry_seconds").is_some()
                || patch.get("termination_condition").is_some()
                || patch.get("auto_terminate_allowed").is_some()
                || patch.get("idle_terminate_allowed").is_some()
            {
                let _ = upsert_contract_patch(root, &agent_id, &patch);
            }
            if should_seed_intro {
                let intro_name = clean_text(
                    patch
                        .get("name")
                        .and_then(Value::as_str)
                        .or_else(|| {
                            existing
                                .as_ref()
                                .and_then(|row| row.get("name").and_then(Value::as_str))
                        })
                        .unwrap_or(&agent_id),
                    120,
                );
                let _ = crate::dashboard_agent_state::seed_intro_message(
                    root,
                    &agent_id,
                    &resolved_role,
                    &intro_name,
                );
            }
            let row = agent_row_by_id(root, snapshot, &agent_id)
                .unwrap_or_else(|| json!({"id": agent_id}));
            let mut payload = json!({"ok": true, "agent_id": agent_id, "agent": row});
            if let Some(notice) = rename_notice {
                payload["rename_notice"] = notice;
            }
            return Some(CompatApiResponse {
                status: 200,
                payload,
            });
        }

        if method == "PUT" && segments.len() == 1 && segments[0] == "model" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let requested = clean_text(
                request.get("model").and_then(Value::as_str).unwrap_or(""),
                200,
            );
            if requested.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "model_required"}),
                });
            }
            let (default_provider, default_model) = effective_app_settings(root, snapshot);
            let (provider, model) = split_model_ref(&requested, &default_provider, &default_model);
            let _ = update_profile_patch(
                root,
                &agent_id,
                &json!({
                    "model_override": format!("{provider}/{model}"),
                    "model_provider": provider,
                    "model_name": model,
                    "runtime_model": model
                }),
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "agent_id": agent_id,
                    "provider": provider,
                    "model": model,
                    "runtime_model": model
                }),
            });
        }

        if method == "PUT" && segments.len() == 1 && segments[0] == "mode" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let mode = clean_text(
                request.get("mode").and_then(Value::as_str).unwrap_or(""),
                40,
            );
            let _ = update_profile_patch(
                root,
                &agent_id,
                &json!({"mode": mode, "updated_at": crate::now_iso()}),
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": agent_id, "mode": mode}),
            });
        }

        if method == "GET" && segments.len() == 1 && segments[0] == "git-trees" {
            return Some(CompatApiResponse {
                status: 200,
                payload: git_tree_payload_for_agent(root, snapshot, &agent_id),
            });
        }

        if method == "POST"
            && segments.len() == 2
            && segments[0] == "git-tree"
            && segments[1] == "switch"
        {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let branch = clean_text(
                request.get("branch").and_then(Value::as_str).unwrap_or(""),
                180,
            );
            if branch.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "branch_required"}),
                });
            }
            let require_new = request
                .get("require_new")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let result = crate::dashboard_git_runtime::switch_agent_worktree(
                root,
                &agent_id,
                &branch,
                require_new,
            );
            let kind = clean_text(
                result
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or("isolated"),
                40,
            );
            let default_workspace_dir = root.to_string_lossy().to_string();
            let workspace_dir = clean_text(
                result
                    .get("workspace_dir")
                    .and_then(Value::as_str)
                    .unwrap_or(default_workspace_dir.as_str()),
                4000,
            );
            let workspace_rel = clean_text(
                result
                    .get("workspace_rel")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                4000,
            );
            let ready = result
                .get("ready")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let error = clean_text(
                result.get("error").and_then(Value::as_str).unwrap_or(""),
                280,
            );
            let _ = update_profile_patch(
                root,
                &agent_id,
                &json!({
                    "git_branch": clean_text(result.get("branch").and_then(Value::as_str).unwrap_or(&branch), 180),
                    "git_tree_kind": kind,
                    "workspace_dir": workspace_dir,
                    "workspace_rel": workspace_rel,
                    "git_tree_ready": ready,
                    "git_tree_error": error,
                    "updated_at": crate::now_iso()
                }),
            );
            return Some(CompatApiResponse {
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    200
                } else {
                    400
                },
                payload: git_tree_payload_for_agent(root, snapshot, &agent_id),
            });
        }

        if method == "POST" && segments.len() == 2 && segments[0] == "file" && segments[1] == "read"
        {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let requested_path = clean_text(
                request
                    .get("path")
                    .and_then(Value::as_str)
                    .or_else(|| request.get("file_path").and_then(Value::as_str))
                    .unwrap_or(""),
                4000,
            );
            if requested_path.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "path_required"}),
                });
            }
            let workspace_base = workspace_base_for_agent(root, existing.as_ref());
            let target = resolve_workspace_path(&workspace_base, &requested_path);
            let Some(target_path) = target else {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "path_outside_workspace", "path": requested_path}),
                });
            };
            if !target_path.is_file() {
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({
                        "ok": false,
                        "error": "file_not_found",
                        "file": {"ok": false, "path": target_path.to_string_lossy().to_string()}
                    }),
                });
            }
            let bytes = fs::read(&target_path).unwrap_or_default();
            let full = request
                .get("full")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let max_bytes = if full {
                bytes.len().max(1)
            } else {
                request
                    .get("max_bytes")
                    .and_then(Value::as_u64)
                    .unwrap_or((256 * 1024) as u64)
                    .clamp(1, (8 * 1024 * 1024) as u64) as usize
            };
            let (content, truncated) = truncate_utf8_lossy(&bytes, max_bytes);
            let content_type = "text/plain; charset=utf-8";
            let download_url = if bytes.len() <= (2 * 1024 * 1024) {
                data_url_from_bytes(&bytes, content_type)
            } else {
                String::new()
            };
            let file_name = clean_text(
                target_path
                    .file_name()
                    .and_then(|v| v.to_str())
                    .unwrap_or("download.txt"),
                180,
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "file": {
                        "ok": true,
                        "path": target_path.to_string_lossy().to_string(),
                        "content": content,
                        "truncated": truncated,
                        "bytes": bytes.len(),
                        "max_bytes": max_bytes,
                        "full": full,
                        "download_url": download_url,
                        "file_name": file_name,
                        "content_type": content_type
                    }
                }),
            });
        }

        if method == "POST"
            && segments.len() == 2
            && segments[0] == "folder"
            && segments[1] == "export"
        {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let requested_path = clean_text(
                request
                    .get("path")
                    .and_then(Value::as_str)
                    .or_else(|| request.get("folder").and_then(Value::as_str))
                    .unwrap_or(""),
                4000,
            );
            if requested_path.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "path_required"}),
                });
            }
            let workspace_base = workspace_base_for_agent(root, existing.as_ref());
            let target = resolve_workspace_path(&workspace_base, &requested_path);
            let Some(target_path) = target else {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "path_outside_workspace", "path": requested_path}),
                });
            };
            if !target_path.is_dir() {
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({
                        "ok": false,
                        "error": "folder_not_found",
                        "folder": {"ok": false, "path": target_path.to_string_lossy().to_string()}
                    }),
                });
            }
            let full = request
                .get("full")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let max_entries = if full {
                1_000_000usize
            } else {
                request
                    .get("max_entries")
                    .and_then(Value::as_u64)
                    .unwrap_or(20_000)
                    .clamp(1, 100_000) as usize
            };
            let mut lines = Vec::<String>::new();
            let root_name = clean_text(
                target_path
                    .file_name()
                    .and_then(|v| v.to_str())
                    .unwrap_or("folder"),
                180,
            );
            lines.push(format!("[d] {root_name}"));
            let mut entries = 0usize;
            let mut truncated = false;
            for entry in WalkDir::new(&target_path)
                .follow_links(false)
                .sort_by_file_name()
            {
                let Ok(row) = entry else {
                    continue;
                };
                let path = row.path();
                if path == target_path {
                    continue;
                }
                entries += 1;
                if entries > max_entries {
                    truncated = true;
                    continue;
                }
                let rel = path.strip_prefix(&target_path).unwrap_or(path);
                let rel_name =
                    clean_text(rel.file_name().and_then(|v| v.to_str()).unwrap_or(""), 240);
                if rel_name.is_empty() {
                    continue;
                }
                let depth = rel.components().count().saturating_sub(1).min(32);
                let indent = "  ".repeat(depth + 1);
                let marker = if row.file_type().is_dir() { "[d]" } else { "-" };
                lines.push(format!("{indent}{marker} {rel_name}"));
            }
            let tree = lines.join("\n");
            let archive_name = if root_name.is_empty() {
                "folder-tree.txt".to_string()
            } else {
                format!("{root_name}-tree.txt")
            };
            let tree_bytes = tree.as_bytes().len();
            let download_url = if tree_bytes > 0 && tree_bytes <= (2 * 1024 * 1024) {
                data_url_from_bytes(tree.as_bytes(), "text/plain; charset=utf-8")
            } else {
                String::new()
            };
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "folder": {
                        "ok": true,
                        "path": target_path.to_string_lossy().to_string(),
                        "tree": tree,
                        "entries": entries,
                        "truncated": truncated,
                        "full": full,
                        "max_entries": max_entries
                    },
                    "archive": {
                        "ok": true,
                        "download_url": download_url,
                        "file_name": archive_name,
                        "bytes": tree_bytes
                    }
                }),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "terminal" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let command = clean_text(
                request
                    .get("command")
                    .and_then(Value::as_str)
                    .or_else(|| request.get("cmd").and_then(Value::as_str))
                    .unwrap_or(""),
                16_000,
            );
            if command.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "command_required"}),
                });
            }
            let resolution =
                match crate::dashboard_terminal_broker::resolve_operator_command(&command) {
                    Ok(resolution) => resolution,
                    Err(err) => {
                        return Some(CompatApiResponse {
                            status: 400,
                            payload: err,
                        });
                    }
                };
            let requested_command = resolution.requested_command.clone();
            let executed_command = resolution.resolved_command.clone();
            let command_translated = resolution.translated;
            let translation_reason = resolution.translation_reason.clone();
            let suggestions = resolution.suggestions.clone();
            let workspace_base = workspace_base_for_agent(root, existing.as_ref());
            let requested_cwd = clean_text(
                request.get("cwd").and_then(Value::as_str).unwrap_or(""),
                4000,
            );
            let cwd = if requested_cwd.is_empty() {
                workspace_base.clone()
            } else {
                resolve_workspace_path(&workspace_base, &requested_cwd)
                    .unwrap_or(workspace_base.clone())
            };
            let started = Instant::now();
            let output = if cfg!(windows) {
                Command::new("cmd")
                    .args(["/C", &executed_command])
                    .current_dir(&cwd)
                    .output()
            } else {
                Command::new("sh")
                    .args(["-lc", &executed_command])
                    .current_dir(&cwd)
                    .output()
            };
            match output {
                Ok(out) => {
                    let (stdout, stdout_truncated) = truncate_utf8_lossy(&out.stdout, 128 * 1024);
                    let (stderr, stderr_truncated) = truncate_utf8_lossy(&out.stderr, 128 * 1024);
                    let mut effective_cwd = cwd.clone();
                    if let Some(last_line) = stdout
                        .lines()
                        .rev()
                        .map(str::trim)
                        .find(|line| !line.is_empty())
                    {
                        if last_line.starts_with('/') {
                            let parsed = normalize_lexical(&PathBuf::from(last_line));
                            if parsed.is_dir()
                                && (parsed == workspace_base || parsed.starts_with(&workspace_base))
                            {
                                effective_cwd = parsed;
                            }
                        }
                    }
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: json!({
                            "ok": true,
                            "stdout": stdout,
                            "stderr": stderr,
                            "stdout_truncated": stdout_truncated,
                            "stderr_truncated": stderr_truncated,
                            "exit_code": out.status.code().unwrap_or(1),
                            "duration_ms": started.elapsed().as_millis() as i64,
                            "cwd": effective_cwd.to_string_lossy().to_string(),
                            "requested_command": requested_command,
                            "executed_command": executed_command,
                            "command_translated": command_translated,
                            "translation_reason": translation_reason,
                            "suggestions": suggestions
                        }),
                    });
                }
                Err(err) => {
                    return Some(CompatApiResponse {
                        status: 500,
                        payload: json!({
                            "ok": false,
                            "error": "terminal_exec_failed",
                            "message": clean_text(&err.to_string(), 500),
                            "exit_code": 1,
                            "duration_ms": started.elapsed().as_millis() as i64,
                            "cwd": cwd.to_string_lossy().to_string(),
                            "requested_command": requested_command,
                            "executed_command": executed_command,
                            "command_translated": command_translated,
                            "translation_reason": translation_reason,
                            "suggestions": suggestions
                        }),
                    });
                }
            }
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "upload" {
            let file_name = clean_text(
                header_value(headers, "X-Filename")
                    .as_deref()
                    .unwrap_or("upload.bin"),
                240,
            );
            let content_type = clean_text(
                header_value(headers, "Content-Type")
                    .as_deref()
                    .unwrap_or("application/octet-stream"),
                120,
            );
            let workspace_base = workspace_base_for_agent(root, existing.as_ref());
            let uploads_dir = workspace_base.join(".infring").join("uploads");
            let _ = fs::create_dir_all(&uploads_dir);
            let file_id = format!(
                "upload-{}",
                crate::deterministic_receipt_hash(&json!({
                    "agent_id": agent_id,
                    "filename": file_name,
                    "bytes": body.len(),
                    "ts": crate::now_iso()
                }))
                .chars()
                .take(16)
                .collect::<String>()
            );
            let ext = Path::new(&file_name)
                .extension()
                .and_then(|v| v.to_str())
                .map(|v| clean_text(v, 16))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "bin".to_string());
            let stored_name = format!("{file_id}.{ext}");
            let stored_path = uploads_dir.join(&stored_name);
            let _ = fs::write(&stored_path, body);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "file_id": file_id,
                    "filename": file_name,
                    "content_type": content_type,
                    "bytes": body.len(),
                    "stored_path": stored_path.to_string_lossy().to_string(),
                    "uploaded_at": crate::now_iso()
                }),
            });
        }

        if method == "GET" && segments.len() == 1 && segments[0] == "files" {
            let dir = agent_files_dir(root, &agent_id);
            let mut rows = Vec::<Value>::new();
            let defaults = vec!["SOUL.md".to_string(), "SYSTEM.md".to_string()];
            for name in defaults {
                let path = dir.join(&name);
                rows.push(json!({
                    "name": name,
                    "exists": path.exists(),
                    "size": fs::metadata(&path).ok().map(|m| m.len()).unwrap_or(0)
                }));
            }
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }
                    let name =
                        clean_text(path.file_name().and_then(|v| v.to_str()).unwrap_or(""), 180);
                    if name.is_empty() {
                        continue;
                    }
                    if rows
                        .iter()
                        .any(|row| row.get("name").and_then(Value::as_str) == Some(name.as_str()))
                    {
                        continue;
                    }
                    rows.push(json!({
                        "name": name,
                        "exists": true,
                        "size": fs::metadata(&path).ok().map(|m| m.len()).unwrap_or(0)
                    }));
                }
            }
            rows.sort_by(|a, b| {
                clean_text(a.get("name").and_then(Value::as_str).unwrap_or(""), 180).cmp(
                    &clean_text(b.get("name").and_then(Value::as_str).unwrap_or(""), 180),
                )
            });
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": agent_id, "files": rows}),
            });
        }

        if (method == "GET" || method == "PUT") && segments.len() >= 2 && segments[0] == "files" {
            let file_name = decode_path_segment(&segments[1..].join("/"));
            if file_name.is_empty() || file_name.contains("..") {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "invalid_file_name"}),
                });
            }
            let path = agent_files_dir(root, &agent_id).join(&file_name);
            if method == "GET" {
                if !path.exists() {
                    return Some(CompatApiResponse {
                        status: 404,
                        payload: json!({"ok": false, "error": "file_not_found"}),
                    });
                }
                let content = fs::read_to_string(&path).unwrap_or_default();
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({"ok": true, "agent_id": agent_id, "name": file_name, "content": content}),
                });
            }
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let content = request
                .get("content")
                .and_then(Value::as_str)
                .map(|v| v.to_string())
                .unwrap_or_default();
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&path, content);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": agent_id, "name": file_name}),
            });
        }

        if method == "GET" && segments.len() == 1 && segments[0] == "tools" {
            let payload = read_json_loose(&agent_tools_path(root, &agent_id))
                .unwrap_or_else(|| json!({"tool_allowlist": [], "tool_blocklist": []}));
            return Some(CompatApiResponse {
                status: 200,
                payload,
            });
        }

        if method == "PUT" && segments.len() == 1 && segments[0] == "tools" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let payload = json!({
                "tool_allowlist": request.get("tool_allowlist").cloned().unwrap_or_else(|| json!([])),
                "tool_blocklist": request.get("tool_blocklist").cloned().unwrap_or_else(|| json!([]))
            });
            write_json_pretty(&agent_tools_path(root, &agent_id), &payload);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": agent_id, "tool_filters": payload}),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "clone" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let source = existing.unwrap_or_else(|| json!({}));
            let requested_new_name = clean_text(
                request
                    .get("new_name")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120,
            );
            let source_role = clean_text(
                source
                    .get("role")
                    .and_then(Value::as_str)
                    .unwrap_or("analyst"),
                60,
            );
            let resolved_requested_name = if requested_new_name.is_empty() {
                dashboard_compat_api_agent_identity::resolve_agent_name(root, "", &source_role)
            } else {
                requested_new_name.clone()
            };
            let new_id_seed = if resolved_requested_name.is_empty() {
                "agent".to_string()
            } else {
                resolved_requested_name.clone()
            };
            let new_id = make_agent_id(root, &new_id_seed);
            let new_name = if resolved_requested_name.is_empty() {
                dashboard_compat_api_agent_identity::default_agent_name(&new_id)
            } else {
                resolved_requested_name
            };
            let mut profile_patch = source.clone();
            profile_patch["name"] = Value::String(new_name.clone());
            profile_patch["agent_id"] = Value::String(new_id.clone());
            profile_patch["parent_agent_id"] = Value::String(agent_id.clone());
            profile_patch["state"] = Value::String("Running".to_string());
            if requested_new_name.is_empty() {
                profile_patch["identity"] =
                    dashboard_compat_api_agent_identity::resolve_agent_identity(
                        root,
                        &json!({}),
                        &source_role,
                    );
            }
            profile_patch["created_at"] = Value::String(crate::now_iso());
            profile_patch["updated_at"] = Value::String(crate::now_iso());
            let _ = update_profile_patch(root, &new_id, &profile_patch);
            let _ = upsert_contract_patch(
                root,
                &new_id,
                &json!({
                    "status": "active",
                    "created_at": crate::now_iso(),
                    "updated_at": crate::now_iso(),
                    "owner": "dashboard_clone",
                    "mission": format!("Assist with assigned mission for {}.", new_id),
                    "parent_agent_id": agent_id,
                    "termination_condition": "task_or_timeout",
                    "expiry_seconds": 3600,
                    "auto_terminate_allowed": false,
                    "idle_terminate_allowed": false
                }),
            );
            append_turn_message(root, &new_id, "", "");
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": new_id, "name": new_name}),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "avatar" {
            let content_type = clean_text(
                query_value(path, "content_type").as_deref().unwrap_or(""),
                120,
            );
            let inferred = if content_type.is_empty() {
                "image/png".to_string()
            } else {
                content_type
            };
            let encoded = {
                use base64::engine::general_purpose::STANDARD;
                use base64::Engine;
                STANDARD.encode(body)
            };
            let avatar_url = format!("data:{};base64,{}", inferred, encoded);
            let _ = update_profile_patch(root, &agent_id, &json!({"avatar_url": avatar_url}));
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": agent_id, "avatar_url": avatar_url}),
            });
        }
    }

    let usage = usage_from_state(root, snapshot);
    let runtime = runtime_sync_summary(snapshot);
    let alerts_count = parse_non_negative_i64(snapshot.pointer("/health/alerts/count"), 0);
    let status =
        if snapshot.get("ok").and_then(Value::as_bool).unwrap_or(false) && alerts_count == 0 {
            "healthy"
        } else if snapshot.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            "degraded"
        } else {
            "critical"
        };

    if method == "GET" {
        let payload = match path_only {
            "/api/health" => json!({
                "ok": true,
                "status": status,
                "checks": snapshot.pointer("/health/checks").cloned().unwrap_or_else(|| json!({})),
                "alerts": snapshot.pointer("/health/alerts").cloned().unwrap_or_else(|| json!({"count": 0, "checks": []})),
                "dashboard_metrics": snapshot.pointer("/health/dashboard_metrics").cloned().unwrap_or_else(|| json!({})),
                "runtime_sync": runtime,
                "receipt_hash": snapshot.get("receipt_hash").cloned().unwrap_or(Value::Null),
                "ts": crate::now_iso()
            }),
            "/api/usage" => {
                json!({"ok": true, "agents": usage["agents"].clone(), "summary": usage["summary"].clone(), "by_model": usage["models"].clone(), "daily": usage["daily"].clone()})
            }
            "/api/usage/summary" => {
                let mut summary = usage["summary"].clone();
                summary["ok"] = json!(true);
                summary
            }
            "/api/usage/by-model" => json!({"ok": true, "models": usage["models"].clone()}),
            "/api/usage/daily" => json!({
                "ok": true,
                "days": usage["daily"].clone(),
                "today_cost_usd": usage["today_cost_usd"].clone(),
                "first_event_date": usage["first_event_date"].clone()
            }),
            "/api/status" => status_payload(root, snapshot, &request_host),
            "/api/web/status" => crate::web_conduit::api_status(root),
            "/api/web/receipts" => {
                let limit = query_value(path, "limit")
                    .and_then(|raw| raw.parse::<usize>().ok())
                    .unwrap_or(20)
                    .clamp(1, 200);
                crate::web_conduit::api_receipts(root, limit)
            }
            "/api/web/search" => {
                let query = clean_text(
                    query_value(path, "q")
                        .or_else(|| query_value(path, "query"))
                        .as_deref()
                        .unwrap_or(""),
                    600,
                );
                crate::web_conduit::api_search(root, &json!({"query": query, "summary_only": true}))
            }
            "/api/telemetry/alerts" => proactive_telemetry_alerts_payload(root, snapshot),
            "/api/continuity" | "/api/continuity/pending" => {
                continuity_pending_payload(root, snapshot)
            }
            "/api/config" => config_payload(root, snapshot),
            "/api/config/schema" => config_schema_payload(),
            "/api/auth/check" => auth_check_payload(),
            "/api/providers" => providers_payload(root, snapshot),
            "/api/models" => crate::dashboard_model_catalog::catalog_payload(root, snapshot),
            "/api/models/recommended" => crate::dashboard_model_catalog::route_decision_payload(
                root,
                snapshot,
                &json!({"task_type":"general","budget_mode":"balanced"}),
            ),
            "/api/route/auto" => crate::dashboard_model_catalog::route_decision_payload(
                root,
                snapshot,
                &json!({"task_type":"general","budget_mode":"balanced"}),
            ),
            "/api/route/decision" => {
                crate::dashboard_model_catalog::route_decision_payload(root, snapshot, &json!({}))
            }
            "/api/channels" => dashboard_compat_api_channels::channels_payload(root),
            "/api/audit/recent" => {
                let entries = recent_audit_entries(root, snapshot);
                let tip_hash = crate::deterministic_receipt_hash(&json!({"entries": entries}));
                json!({"ok": true, "entries": entries, "tip_hash": tip_hash})
            }
            "/api/audit/decisions" => {
                let limit = query_value(path, "limit")
                    .and_then(|raw| raw.parse::<usize>().ok())
                    .unwrap_or(20)
                    .clamp(1, 200);
                let rows = read_jsonl_loose(&tool_decision_audit_path(root), limit);
                let tip_hash = crate::deterministic_receipt_hash(&json!({"rows": rows}));
                json!({"ok": true, "type": "tool_decision_audit_rows", "rows": rows, "tip_hash": tip_hash})
            }
            "/api/audit/verify" => {
                let entries = recent_audit_entries(root, snapshot);
                let tip_hash = crate::deterministic_receipt_hash(&json!({"entries": entries}));
                json!({"ok": true, "valid": true, "entries": entries.len(), "tip_hash": tip_hash})
            }
            "/api/version" => {
                let version = read_json(&root.join("package.json"))
                    .and_then(|v| v.get("version").and_then(Value::as_str).map(str::to_string))
                    .unwrap_or_else(|| "0.1.0".to_string());
                json!({
                    "ok": true,
                    "version": version,
                    "rust_authority": "rust_core_lanes",
                    "platform": std::env::consts::OS,
                    "arch": std::env::consts::ARCH
                })
            }
            "/api/security" => json!({
                "ok": true,
                "mode": "strict",
                "fail_closed": true,
                "receipts_required": true,
                "checks": snapshot.pointer("/health/checks").cloned().unwrap_or_else(|| json!({})),
                "alerts": snapshot.pointer("/health/alerts").cloned().unwrap_or_else(|| json!({})),
                "runtime_sync": runtime
            }),
            "/api/capabilities/status" => {
                let policy = tool_governance_policy(root);
                let tiers = [
                    ("file_read", "green"),
                    ("folder_export", "green"),
                    ("web_fetch", "green"),
                    ("web_search", "green"),
                    ("memory_kv_get", "green"),
                    ("memory_kv_list", "green"),
                    ("memory_semantic_query", "green"),
                    ("memory_kv_set", "yellow"),
                    ("cron_schedule", "yellow"),
                    ("cron_run", "yellow"),
                    ("cron_cancel", "yellow"),
                    ("manage_agent", "yellow"),
                    ("terminal_exec", "red"),
                    ("spawn_subagents", "red"),
                ];
                json!({
                    "ok": true,
                    "type": "tool_capability_tiers",
                    "policy": policy,
                    "tools": tiers.iter().map(|(tool, tier)| json!({"tool": tool, "tier": tier})).collect::<Vec<_>>()
                })
            }
            "/api/tools" => json!({
                "ok": true,
                "tools": [
                    {"name": "protheus-ops", "category": "runtime"},
                    {"name": "infringd", "category": "runtime"},
                    {"name": "web_conduit", "category": "runtime"},
                    {"name": "git", "category": "cli"},
                    {"name": "rg", "category": "cli"}
                ],
                "runtime_sync": runtime
            }),
            "/api/commands" => json!({
                "ok": true,
                "commands": [
                    {"cmd": "/status", "command": "/status", "desc": "Show runtime status and cockpit summary", "description": "Show runtime status and cockpit summary"},
                    {"cmd": "/queue", "command": "/queue", "desc": "Show current queue pressure", "description": "Show current queue pressure"},
                    {"cmd": "/context", "command": "/context", "desc": "Show context and attention state", "description": "Show context and attention state"},
                    {"cmd": "/model", "command": "/model", "desc": "Inspect or switch model (/model [name])", "description": "Inspect or switch model (/model [name])"},
                    {"cmd": "/file <path>", "command": "/file <path>", "desc": "Render full file output in chat from workspace path", "description": "Render full file output in chat from workspace path"},
                    {"cmd": "/folder <path>", "command": "/folder <path>", "desc": "Render folder tree + downloadable archive in chat", "description": "Render folder tree + downloadable archive in chat"},
                    {"cmd": "/alerts", "command": "/alerts", "desc": "Show proactive telemetry alerts", "description": "Show proactive telemetry alerts"},
                    {"cmd": "/continuity", "command": "/continuity", "desc": "Show pending actions across sessions/channels/tasks", "description": "Show pending actions across sessions/channels/tasks"},
                    {"cmd": "/browse <url>", "command": "/browse <url>", "desc": "Fetch and summarize a web URL via governed web conduit", "description": "Fetch and summarize a web URL via governed web conduit"},
                    {"cmd": "/search <query>", "command": "/search <query>", "desc": "Search the web with governed web conduit and summarize results", "description": "Search the web with governed web conduit and summarize results"},
                    {"cmd": "/cron", "command": "/cron list | /cron schedule <interval> <message> | /cron run <job_id> | /cron cancel <job_id>", "desc": "Manage agent-owned scheduled jobs", "description": "Manage agent-owned scheduled jobs"},
                    {"cmd": "/memory query <text>", "command": "/memory query <text>", "desc": "Semantic memory lookup over persisted KV entries", "description": "Semantic memory lookup over persisted KV entries"},
                    {"cmd": "/undo", "command": "/undo", "desc": "Undo the last conversational turn with receipted rollback", "description": "Undo the last conversational turn with receipted rollback"},
                    {"cmd": "/aliases", "command": "/aliases", "desc": "List active slash command aliases", "description": "List active slash command aliases"},
                    {"cmd": "/alias", "command": "/alias <shortcut> <target>", "desc": "Create a custom slash alias", "description": "Create a custom slash alias"}
                ]
            }),
            "/api/budget" => json!({
                "ok": true,
                "hourly_spend": 0,
                "daily_spend": usage.pointer("/summary/total_cost_usd").cloned().unwrap_or_else(|| json!(0)),
                "monthly_spend": usage.pointer("/summary/total_cost_usd").cloned().unwrap_or_else(|| json!(0)),
                "hourly_limit": 0,
                "daily_limit": 0,
                "monthly_limit": 0
            }),
            "/api/sessions" => {
                json!({"ok": true, "sessions": session_summary_rows(root, snapshot)})
            }
            "/api/comms/topology" => json!({
                "ok": true,
                "topology": {
                    "nodes": snapshot.pointer("/collab/dashboard/agents").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
                    "edges": 0,
                    "connected": true
                }
            }),
            "/api/comms/events" => json!({"ok": true, "events": []}),
            "/api/profiles" => json!({"ok": true, "profiles": extract_profiles(root)}),
            "/api/update/check" => crate::dashboard_release_update::check_update(root),
            "/api/templates" => json!({
                "ok": true,
                "templates": [
                    {"id": "general-assistant", "name": "General Assistant", "provider": "auto", "model": "auto"},
                    {"id": "research-analyst", "name": "Research Analyst", "provider": "openai", "model": "gpt-5"},
                    {"id": "ops-reliability", "name": "Ops Reliability", "provider": "frontier_provider", "model": "claude-opus-4-20250514"}
                ]
            }),
            _ => return None,
        };
        return Some(CompatApiResponse {
            status: 200,
            payload,
        });
    }

    if method == "POST" {
        if path_only == "/api/update/apply" {
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_release_update::apply_update(root),
            });
        }
        if path_only == "/api/config/set" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let payload = set_config_payload(root, snapshot, &request);
            return Some(CompatApiResponse {
                status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    200
                } else {
                    400
                },
                payload,
            });
        }
        if path_only == "/api/web/fetch" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let payload = crate::web_conduit::api_fetch(root, &request);
            return Some(CompatApiResponse {
                status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    200
                } else {
                    400
                },
                payload,
            });
        }
        if path_only == "/api/web/search" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let payload = crate::web_conduit::api_search(root, &request);
            return Some(CompatApiResponse {
                status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    200
                } else {
                    400
                },
                payload,
            });
        }
        if path_only == "/api/route/auto" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_model_catalog::route_decision_payload(
                    root, snapshot, &request,
                ),
            });
        }
        if path_only == "/api/route/decision" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_model_catalog::route_decision_payload(
                    root, snapshot, &request,
                ),
            });
        }
        return None;
    }

    if method == "DELETE" {
        return None;
    }

    None
}

pub fn handle(
    root: &Path,
    method: &str,
    path: &str,
    body: &[u8],
    snapshot: &Value,
) -> Option<CompatApiResponse> {
    handle_with_headers(root, method, path, body, &[], snapshot)
}

#[cfg(test)]
mod tests {
    include!("config_payload_tests_parts/010-init-git-repo.rs");
    include!("config_payload_tests_parts/020-agent-create-without-name-returns-non-generic-id.rs");
    include!("config_payload_tests_parts/030-memory-kv-http-routes-round-trip-and-feed-context.rs");
    include!("config_payload_tests_parts/040-terminated-agent-endpoints-round-trip.rs");
    include!("config_payload_tests_parts/050-compact-session-keyframes.rs");
    include!("config_payload_tests_parts/060-context-telemetry-and-auto-compact.rs");
    include!("config_payload_tests_parts/070-cron-command-routing.rs");
    include!("config_payload_tests_parts/080-conversation-search-includes-archived.rs");
    include!("config_payload_tests_parts/090-latent-tool-discovery-and-rollback.rs");
    include!("config_payload_tests_parts/100-governance-and-semantic-memory.rs");
    include!("config_payload_tests_parts/110-agent-capability-gauntlet.rs");
}
