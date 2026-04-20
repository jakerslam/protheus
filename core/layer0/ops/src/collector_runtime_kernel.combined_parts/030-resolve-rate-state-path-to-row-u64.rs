
fn resolve_rate_state_path(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    if let Some(raw) = payload.get("rate_state_path").and_then(Value::as_str) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let candidate = PathBuf::from(trimmed);
            if candidate.is_absolute() {
                return candidate;
            }
            return root.join(candidate);
        }
    }

    if let Ok(raw) = std::env::var("EYES_STATE_DIR") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed).join("collector_rate_state.json");
        }
    }

    root.join(DEFAULT_RATE_STATE_REL)
}

fn resolve_eyes_state_dir(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    if let Some(raw) = payload.get("eyes_state_dir").and_then(Value::as_str) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let candidate = PathBuf::from(trimmed);
            if candidate.is_absolute() {
                return candidate;
            }
            return root.join(candidate);
        }
    }
    if let Ok(raw) = std::env::var("EYES_STATE_DIR") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    root.join(EYES_STATE_DEFAULT_REL)
}

fn meta_path_for(root: &Path, payload: &Map<String, Value>, collector_id: &str) -> PathBuf {
    resolve_eyes_state_dir(root, payload)
        .join("collector_meta")
        .join(format!("{collector_id}.json"))
}

fn cache_path_for(root: &Path, payload: &Map<String, Value>, collector_id: &str) -> PathBuf {
    resolve_eyes_state_dir(root, payload)
        .join("collector_meta")
        .join(format!("{collector_id}.cache.json"))
}

fn read_json(path: &Path, fallback: Value) -> Value {
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str::<Value>(&raw).unwrap_or(fallback),
        Err(_) => fallback,
    }
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("collector_runtime_kernel_create_dir_failed:{err}"))?;
    }
    let body = format!(
        "{}\n",
        serde_json::to_string_pretty(value)
            .map_err(|err| format!("collector_runtime_kernel_encode_failed:{err}"))?
    );
    fs::write(path, body).map_err(|err| format!("collector_runtime_kernel_write_failed:{err}"))
}

fn clean_seen_id(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if out.len() >= 120 {
            break;
        }
        let keep = ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.');
        if keep {
            out.push(ch);
        }
    }
    out
}

fn normalize_meta_value(collector_id: &str, raw: Option<&Value>) -> Value {
    let obj = raw.and_then(Value::as_object);
    let last_run = lane_utils::clean_text(
        obj.and_then(|o| o.get("last_run")).and_then(Value::as_str),
        80,
    );
    let last_success = lane_utils::clean_text(
        obj.and_then(|o| o.get("last_success"))
            .and_then(Value::as_str),
        80,
    );
    let mut seen_ids = Vec::new();
    if let Some(items) = obj
        .and_then(|o| o.get("seen_ids"))
        .and_then(Value::as_array)
    {
        for entry in items {
            if let Some(raw_id) = entry.as_str() {
                let cleaned = clean_seen_id(raw_id);
                if !cleaned.is_empty() {
                    seen_ids.push(Value::String(cleaned));
                }
            }
        }
    }
    if seen_ids.len() > 2000 {
        let split = seen_ids.len() - 2000;
        seen_ids = seen_ids.into_iter().skip(split).collect::<Vec<_>>();
    }
    json!({
        "collector_id": collector_id,
        "last_run": if last_run.is_empty() { Value::Null } else { Value::String(last_run) },
        "last_success": if last_success.is_empty() { Value::Null } else { Value::String(last_success) },
        "seen_ids": seen_ids
    })
}

fn parse_iso_ms(raw: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

fn default_state_json() -> Value {
    json!({
        "schema_id": RATE_SCHEMA_ID,
        "collectors": {}
    })
}

fn read_state(path: &Path) -> Value {
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| default_state_json()),
        Err(_) => default_state_json(),
    }
}

fn write_state(path: &Path, state: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("collector_runtime_kernel_create_dir_failed:{err}"))?;
    }
    let pretty = serde_json::to_string_pretty(state)
        .map_err(|err| format!("collector_runtime_kernel_encode_failed:{err}"))?;
    fs::write(path, format!("{pretty}\n"))
        .map_err(|err| format!("collector_runtime_kernel_write_failed:{err}"))
}

fn ensure_collectors_mut(state: &mut Value) -> Result<&mut Map<String, Value>, String> {
    if !state.is_object() {
        *state = default_state_json();
    }
    let state_obj = state
        .as_object_mut()
        .ok_or_else(|| "collector_runtime_kernel_state_not_object".to_string())?;
    if state_obj
        .get("schema_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        != RATE_SCHEMA_ID
    {
        state_obj.insert(
            "schema_id".to_string(),
            Value::String(RATE_SCHEMA_ID.to_string()),
        );
    }
    if !state_obj
        .get("collectors")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state_obj.insert("collectors".to_string(), Value::Object(Map::new()));
    }
    state_obj
        .get_mut("collectors")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "collector_runtime_kernel_collectors_not_object".to_string())
}

fn ensure_row_mut<'a>(
    collectors: &'a mut Map<String, Value>,
    collector_id: &str,
) -> Result<&'a mut Map<String, Value>, String> {
    if !collectors
        .get(collector_id)

        .map(Value::is_object)
        .unwrap_or(false)
    {
        collectors.insert(
            collector_id.to_string(),
            json!({
                "last_attempt_ms": 0,
                "last_success_ms": 0,
                "failure_streak": 0,
                "next_allowed_ms": 0,
                "circuit_open_until_ms": 0,
                "last_error_code": null
            }),
        );
    }
    collectors
        .get_mut(collector_id)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "collector_runtime_kernel_row_not_object".to_string())
}

fn row_u64(row: &Map<String, Value>, key: &str) -> u64 {
    row.get(key).and_then(Value::as_u64).unwrap_or(0)
}
