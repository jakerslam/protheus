fn now_iso() -> String {
    crate::now_iso()
}

fn clean_text(value: &str, max_len: usize) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn normalize_agent_id(raw: &str) -> String {
    let mut out = String::new();
    for ch in clean_text(raw, 140).chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        }
    }
    out
}

fn parse_json_loose(text: &str) -> Option<Value> {
    if text.trim().is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<Value>(text) {
        return Some(value);
    }
    for line in text.lines().rev() {
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

fn read_json_file(path: &Path) -> Option<Value> {
    let body = fs::read_to_string(path).ok()?;
    parse_json_loose(&body)
}

fn ensure_dir(path: &Path) {
    let _ = fs::create_dir_all(path);
}

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        ensure_dir(parent);
    }
    if let Ok(body) = serde_json::to_string_pretty(value) {
        let _ = fs::write(path, format!("{body}\n"));
    }
}

fn as_object_mut<'a>(root: &'a mut Value, key: &str) -> &'a mut Map<String, Value> {
    if !root.get(key).map(Value::is_object).unwrap_or(false) {
        root[key] = json!({});
    }
    root.get_mut(key)
        .and_then(Value::as_object_mut)
        .expect("object shape")
}

fn as_array_mut<'a>(root: &'a mut Value, key: &str) -> &'a mut Vec<Value> {
    if !root.get(key).map(Value::is_array).unwrap_or(false) {
        root[key] = Value::Array(Vec::new());
    }
    root.get_mut(key)
        .and_then(Value::as_array_mut)
        .expect("array shape")
}

fn profiles_path(root: &Path) -> PathBuf {
    root.join(AGENT_PROFILES_REL)
}

fn archived_path(root: &Path) -> PathBuf {
    root.join(ARCHIVED_AGENTS_REL)
}

fn contracts_path(root: &Path) -> PathBuf {
    root.join(AGENT_CONTRACTS_REL)
}

fn session_path(root: &Path, agent_id: &str) -> PathBuf {
    root.join(AGENT_SESSIONS_DIR_REL)
        .join(format!("{}.json", normalize_agent_id(agent_id)))
}

fn default_profiles_state() -> Value {
    json!({
        "type": "infring_dashboard_agent_profiles",
        "updated_at": now_iso(),
        "agents": {}
    })
}

fn load_profiles_state(root: &Path) -> Value {
    let mut state = read_json_file(&profiles_path(root)).unwrap_or_else(default_profiles_state);
    if !state.is_object() {
        state = default_profiles_state();
    }
    let _ = as_object_mut(&mut state, "agents");
    state
}

fn save_profiles_state(root: &Path, mut state: Value) {
    state["updated_at"] = Value::String(now_iso());
    write_json(&profiles_path(root), &state);
}

fn default_archived_state() -> Value {
    json!({
        "type": "infring_dashboard_archived_agents",
        "updated_at": now_iso(),
        "agents": {}
    })
}

fn load_archived_state(root: &Path) -> Value {
    let mut state = read_json_file(&archived_path(root)).unwrap_or_else(default_archived_state);
    if !state.is_object() {
        state = default_archived_state();
    }
    let _ = as_object_mut(&mut state, "agents");
    state
}

fn save_archived_state(root: &Path, mut state: Value) {
    state["updated_at"] = Value::String(now_iso());
    write_json(&archived_path(root), &state);
}

fn default_contract(agent_id: &str) -> Value {
    let now = now_iso();
    json!({
        "contract_id": format!(
            "contract-{}",
            crate::deterministic_receipt_hash(&json!({"agent_id": agent_id, "ts": now}))
                .chars()
                .take(16)
                .collect::<String>()
        ),
        "agent_id": agent_id,
        "mission": format!("Assist with assigned mission for {}.", agent_id),
        "owner": "dashboard_session",
        "status": "active",
        "termination_condition": "task_or_timeout",
        "expiry_seconds": DEFAULT_EXPIRY_SECONDS,
        "created_at": now,
        "updated_at": now,
        "expires_at": "",
        "auto_terminate_allowed": true,
        "idle_terminate_allowed": true,
        "idle_timeout_seconds": DEFAULT_IDLE_TIMEOUT_SECONDS
    })
}

fn default_contracts_state() -> Value {
    json!({
        "type": "infring_dashboard_agent_contracts",
        "updated_at": now_iso(),
        "contracts": {},
        "terminated_history": []
    })
}

fn load_contracts_state(root: &Path) -> Value {
    let mut state = read_json_file(&contracts_path(root)).unwrap_or_else(default_contracts_state);
    if !state.is_object() {
        state = default_contracts_state();
    }
    let _ = as_object_mut(&mut state, "contracts");
    let _ = as_array_mut(&mut state, "terminated_history");
    state
}

fn save_contracts_state(root: &Path, mut state: Value) {
    state["updated_at"] = Value::String(now_iso());
    write_json(&contracts_path(root), &state);
}

fn parse_expiry_seconds(value: Option<&Value>) -> i64 {
    value
        .and_then(Value::as_i64)
        .unwrap_or(DEFAULT_EXPIRY_SECONDS)
        .clamp(1, MAX_EXPIRY_SECONDS)
}

fn parse_idle_timeout_seconds(value: Option<&Value>) -> i64 {
    value
        .and_then(Value::as_i64)
        .unwrap_or(DEFAULT_IDLE_TIMEOUT_SECONDS)
        .clamp(30, MAX_IDLE_TIMEOUT_SECONDS)
}

fn parse_ts(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|v| v.with_timezone(&Utc))
}

fn session_last_activity_ts(root: &Path, agent_id: &str) -> Option<DateTime<Utc>> {
    let state = read_json_file(&session_path(root, agent_id))?;
    let sessions = state.get("sessions").and_then(Value::as_array)?;
    let mut latest = None::<DateTime<Utc>>;

    for session in sessions {
        if let Some(updated) = session
            .get("updated_at")
            .and_then(Value::as_str)
            .and_then(parse_ts)
        {
            latest = Some(latest.map(|ts| ts.max(updated)).unwrap_or(updated));
        }
        let messages = session
            .get("messages")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for row in messages {
            for field in ["ts", "updated_at", "created_at"] {
                if let Some(ts) = row.get(field).and_then(Value::as_str).and_then(parse_ts) {
                    latest = Some(latest.map(|seen| seen.max(ts)).unwrap_or(ts));
                }
            }
        }
    }
    latest
}

pub fn archived_agent_ids(root: &Path) -> HashSet<String> {
    let state = load_archived_state(root);
    state
        .get("agents")
        .and_then(Value::as_object)
        .map(|rows| {
            rows.keys()
                .map(|row| normalize_agent_id(row))
                .filter(|row| !row.is_empty())
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default()
}

pub fn merge_profiles_into_collab(root: &Path, collab_payload: &mut Value, default_team: &str) {
    let profiles_state = load_profiles_state(root);
    let profiles = profiles_state
        .get("agents")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if profiles.is_empty() {
        return;
    }
    let archived = archived_agent_ids(root);
    if !collab_payload
        .get("dashboard")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        collab_payload["dashboard"] = json!({
            "version": "v1",
            "team": default_team,
            "agents": [],
            "tasks": [],
            "handoff_history": []
        });
    }
    if !collab_payload["dashboard"]
        .get("agents")
        .map(Value::is_array)
        .unwrap_or(false)
    {
        collab_payload["dashboard"]["agents"] = Value::Array(Vec::new());
    }
    let rows = collab_payload["dashboard"]["agents"]
        .as_array_mut()
        .expect("agents array");
    let mut existing = rows
        .iter()
        .filter_map(|row| row.get("shadow").and_then(Value::as_str))
        .map(normalize_agent_id)
        .collect::<HashSet<_>>();

    for (raw_id, profile) in profiles {
        let agent_id = normalize_agent_id(&raw_id);
        if agent_id.is_empty() || archived.contains(&agent_id) || existing.contains(&agent_id) {
            continue;
        }
        let role = profile
            .get("role")
            .and_then(Value::as_str)
            .map(|v| clean_text(v, 60))
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "analyst".to_string());
        let status = profile
            .get("state")
            .and_then(Value::as_str)
            .map(|v| clean_text(v, 40))
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "inactive".to_string());
        rows.push(json!({
            "shadow": agent_id,
            "role": role,
            "status": status,
            "activated_at": profile
                .get("updated_at")
                .cloned()
                .unwrap_or_else(|| Value::String(String::new())),
            "source": "profile_state"
        }));
        existing.insert(agent_id);
    }
}

pub fn upsert_profile(root: &Path, agent_id: &str, patch: &Value) -> Value {
    let id = normalize_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let mut state = load_profiles_state(root);
    let agents = as_object_mut(&mut state, "agents");
    let mut current = agents.get(&id).cloned().unwrap_or_else(|| json!({}));
    if !current.is_object() {
        current = json!({});
    }
    let mut model_patch_seen = false;
    if let Some(obj) = patch.as_object() {
        for (key, value) in obj {
            if matches!(
                key.as_str(),
                "role"
                    | "name"
                    | "emoji"
                    | "avatar_url"
                    | "state"
                    | "description"
                    | "lifespan"
                    | "identity"
                    | "color"
                    | "archetype"
                    | "vibe"
                    | "model_override"
                    | "model_provider"
                    | "model_name"
                    | "runtime_model"
                    | "fallback_models"
                    | "system_prompt"
                    | "context_window"
                    | "context_window_tokens"
                    | "git_branch"
                    | "git_tree_kind"
                    | "workspace_dir"
                    | "workspace_rel"
                    | "git_tree_ready"
                    | "git_tree_error"
                    | "is_master_agent"
                    | "parent_agent_id"
                    | "mode"
            ) {
                current[key] = value.clone();
                if matches!(
                    key.as_str(),
                    "model_override" | "model_provider" | "model_name" | "runtime_model"
                ) {
                    model_patch_seen = true;
                }
            }
        }
    }
    if !current
        .get("created_at")
        .and_then(Value::as_str)
        .map(|v| !v.is_empty())
        .unwrap_or(false)
    {
        current["created_at"] = Value::String(now_iso());
    }
    current["updated_at"] = Value::String(now_iso());
    if model_patch_seen {
        let mut provider = clean_text(
            current
                .get("model_provider")
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        )
        .to_ascii_lowercase();
        let mut model = clean_text(
            current
                .get("model_name")
                .or_else(|| current.get("runtime_model"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            240,
        );
        if provider.is_empty() || model.is_empty() {
            if let Some(raw) = current.get("model_override").and_then(Value::as_str) {
                let cleaned_override = clean_text(raw, 280);
                if let Some((left, right)) = cleaned_override.split_once('/') {
                    if provider.is_empty() {
                        provider = clean_text(left, 80).to_ascii_lowercase();
                    }
                    if model.is_empty() {
                        model = clean_text(right, 240);
                    }
                }
            }
        }
        if !provider.is_empty() && !model.is_empty() {
            let _ =
                crate::dashboard_provider_runtime::ensure_model_profile(root, &provider, &model);
        }
    }
    agents.insert(id.clone(), current.clone());
    save_profiles_state(root, state);
    json!({"ok": true, "type": "dashboard_agent_profile", "agent_id": id, "profile": current})
}

pub fn archive_agent(root: &Path, agent_id: &str, reason: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let mut state = load_archived_state(root);
    let agents = as_object_mut(&mut state, "agents");
    agents.insert(
        id.clone(),
        json!({
            "reason": clean_text(reason, 120),
            "archived_at": now_iso()
        }),
    );
    save_archived_state(root, state);
    json!({"ok": true, "type": "dashboard_agent_archive", "agent_id": id})
}

pub fn unarchive_agent(root: &Path, agent_id: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let mut state = load_archived_state(root);
    let agents = as_object_mut(&mut state, "agents");
    let removed = agents.remove(&id).is_some();
    save_archived_state(root, state);
    json!({"ok": true, "type": "dashboard_agent_unarchive", "agent_id": id, "removed": removed})
}

