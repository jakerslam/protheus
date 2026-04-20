
const CORE_SKILLS_REGISTRY_REL: &str = "core/local/state/ops/skills_plane/registry.json";
const DASHBOARD_SKILLS_STATE_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/skills_registry.json";

fn clean_text(raw: &str, max_len: usize) -> String {
    lane_utils::clean_text(Some(raw), max_len.max(1))
}

fn normalize_name(raw: &str) -> String {
    clean_text(raw, 120)
        .to_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn state_path(root: &Path, rel: &str) -> PathBuf {
    root.join(rel)
}

fn read_json(path: &Path) -> Option<Value> {
    lane_utils::read_json(path)
}

fn write_json(path: &Path, value: &Value) {
    let _ = lane_utils::write_json(path, value);
}

fn parse_json(raw: &[u8]) -> Value {
    serde_json::from_slice::<Value>(raw).unwrap_or_else(|_| json!({}))
}

fn parse_query(path: &str) -> Map<String, Value> {
    let mut out = Map::new();
    let query = path.split_once('?').map(|(_, q)| q).unwrap_or("");
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let key = clean_text(k, 80).to_lowercase();
        if key.is_empty() {
            continue;
        }
        let value = urlencoding::decode(v)
            .ok()
            .map(|s| s.to_string())
            .unwrap_or_default();
        out.insert(key, Value::String(clean_text(&value, 400)));
    }
    out
}

fn parse_u64(value: Option<&Value>, fallback: u64) -> u64 {
    value
        .and_then(|v| match v {
            Value::Number(_) => v.as_u64(),
            Value::String(s) => clean_text(s, 40).parse::<u64>().ok(),
            _ => None,
        })
        .unwrap_or(fallback)
}

fn as_object_mut<'a>(value: &'a mut Value, key: &str) -> &'a mut Map<String, Value> {
    if !value.get(key).map(Value::is_object).unwrap_or(false) {
        value[key] = Value::Object(Map::new());
    }
    value
        .get_mut(key)
        .and_then(Value::as_object_mut)
        .expect("object must exist")
}

fn load_dashboard_state(root: &Path) -> Value {
    read_json(&state_path(root, DASHBOARD_SKILLS_STATE_REL)).unwrap_or_else(|| {
        json!({
            "type": "infring_dashboard_skills_registry",
            "updated_at": crate::now_iso(),
            "installed": {},
            "created": {}
        })
    })
}

fn save_dashboard_state(root: &Path, mut state: Value) {
    state["updated_at"] = Value::String(crate::now_iso());
    write_json(&state_path(root, DASHBOARD_SKILLS_STATE_REL), &state);
}

fn load_core_registry(root: &Path) -> Value {
    read_json(&state_path(root, CORE_SKILLS_REGISTRY_REL)).unwrap_or_else(|| {
        json!({
            "kind": "skills_registry",
            "installed": {}
        })
    })
}

fn save_core_registry(root: &Path, mut state: Value) {
    if !state.get("kind").map(Value::is_string).unwrap_or(false) {
        state["kind"] = Value::String("skills_registry".to_string());
    }
    write_json(&state_path(root, CORE_SKILLS_REGISTRY_REL), &state);
}

fn core_record_from_skill_row(skill_id: &str, row: &Value) -> Value {
    let source = row
        .get("source")
        .cloned()
        .unwrap_or_else(|| json!({"type":"local"}));
    let source_type = clean_text(
        source
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("local"),
        40,
    );
    let source_slug = clean_text(
        source
            .get("slug")
            .and_then(Value::as_str)
            .unwrap_or(skill_id),
        120,
    );
    let fallback_path = if source_type.eq_ignore_ascii_case("clawhub") {
        format!("clawhub://{source_slug}")
    } else {
        format!("dashboard://{skill_id}")
    };
    json!({
        "name": clean_text(row.get("name").and_then(Value::as_str).unwrap_or(skill_id), 120),
        "description": clean_text(row.get("description").and_then(Value::as_str).unwrap_or(""), 300),
        "version": clean_text(row.get("version").and_then(Value::as_str).unwrap_or("v1"), 40),
        "author": clean_text(row.get("author").and_then(Value::as_str).unwrap_or("Unknown"), 120),
        "runtime": clean_text(row.get("runtime").and_then(Value::as_str).unwrap_or("prompt_only"), 40),
        "tools_count": parse_u64(row.get("tools_count"), 0),
        "tags": row.get("tags").cloned().filter(|v| v.is_array()).unwrap_or_else(|| Value::Array(default_tags())),
        "enabled": row.get("enabled").and_then(Value::as_bool).unwrap_or(true),
        "has_prompt_context": row.get("has_prompt_context").and_then(Value::as_bool).unwrap_or(false),
        "prompt_context": clean_text(row.get("prompt_context").and_then(Value::as_str).unwrap_or(""), 4000),
        "source": source,
        "path": clean_text(row.get("path").and_then(Value::as_str).unwrap_or(&fallback_path), 512),
        "installed_at": row
            .get("installed_at")
            .cloned()
            .unwrap_or_else(|| Value::String(crate::now_iso())),
    })
}

fn upsert_core_installed_skill(root: &Path, skill_id: &str, row: &Value) {
    let key = normalize_name(skill_id);
    if key.is_empty() {
        return;
    }
    let mut state = load_core_registry(root);
    let installed = as_object_mut(&mut state, "installed");
    installed.insert(key.clone(), core_record_from_skill_row(&key, row));
    save_core_registry(root, state);
}

fn remove_core_installed_skill(root: &Path, skill_id: &str) {
    let key = normalize_name(skill_id);
    if key.is_empty() {
        return;
    }
    let mut state = load_core_registry(root);
    let installed = as_object_mut(&mut state, "installed");
    installed.remove(&key);
    save_core_registry(root, state);
}

fn default_tags() -> Vec<Value> {
    vec![Value::String("general".to_string())]
}
