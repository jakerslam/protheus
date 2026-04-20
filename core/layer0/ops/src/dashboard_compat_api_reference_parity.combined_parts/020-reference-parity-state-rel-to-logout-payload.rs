
const REFERENCE_PARITY_STATE_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/reference_runtime_parity_state.json";
const REFERENCE_UPLOADS_DIR_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/reference_runtime_uploads";
const ACTION_HISTORY_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/actions/history.jsonl";

fn clean_text(raw: &str, max_len: usize) -> String {
    lane_utils::clean_text(Some(raw), max_len.max(1))
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

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn parse_json(raw: &[u8]) -> Value {
    serde_json::from_slice::<Value>(raw).unwrap_or_else(|_| json!({}))
}

fn query_value(path: &str, key: &str) -> Option<String> {
    let query = path.split_once('?').map(|(_, q)| q).unwrap_or("");
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        if clean_text(k, 80).eq_ignore_ascii_case(key) {
            let decoded = urlencoding::decode(v)
                .ok()
                .map(|s| s.to_string())
                .unwrap_or_default();
            let value = clean_text(&decoded, 160);
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn header_value(headers: &[(&str, &str)], key: &str) -> String {
    headers
        .iter()
        .find(|(name, _)| clean_text(name, 80).eq_ignore_ascii_case(key))
        .map(|(_, value)| clean_text(value, 1024))
        .unwrap_or_default()
}

fn stable_hash(seed: &str, len: usize) -> String {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    let digest = format!("{:016x}", hasher.finish());
    digest.chars().take(len.max(1).min(digest.len())).collect()
}

fn load_parity_state(root: &Path) -> Value {
    read_json(&state_path(root, REFERENCE_PARITY_STATE_REL)).unwrap_or_else(|| {
        json!({
            "type": "reference_runtime_parity_state",
            "updated_at": crate::now_iso(),
            "auth": {
                "token": "",
                "user": "operator",
                "login_at_ms": 0
            },
            "pairing": {
                "pairing_id": "",
                "status": "idle",
                "code": "",
                "started_at_ms": 0,
                "updated_at_ms": 0
            },
            "uploads": []
        })
    })
}

fn save_parity_state(root: &Path, mut state: Value) {
    state["updated_at"] = Value::String(crate::now_iso());
    write_json(&state_path(root, REFERENCE_PARITY_STATE_REL), &state);
}

fn as_array_mut<'a>(value: &'a mut Value, key: &str) -> &'a mut Vec<Value> {
    if !value.get(key).map(Value::is_array).unwrap_or(false) {
        value[key] = Value::Array(Vec::new());
    }
    value
        .get_mut(key)
        .and_then(Value::as_array_mut)
        .expect("array must exist")
}

fn models_v1_payload(root: &Path, snapshot: &Value) -> Value {
    let catalog = crate::dashboard_model_catalog::catalog_payload(root, snapshot);
    let data = catalog
        .get("models")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let provider = clean_text(
                        row.get("provider")
                            .and_then(Value::as_str)
                            .unwrap_or("auto"),
                        80,
                    );
                    let model =
                        clean_text(row.get("model").and_then(Value::as_str).unwrap_or(""), 160);
                    if model.is_empty() {
                        return None;
                    }
                    Some(json!({
                        "id": model,
                        "object": "model",
                        "owned_by": provider,
                        "provider": provider,
                        "available": row.get("available").and_then(Value::as_bool).unwrap_or(false),
                        "context_window": row
                            .get("context_window_tokens")
                            .and_then(Value::as_u64)
                            .unwrap_or(0)
                    }))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    json!({
        "object": "list",
        "data": data
    })
}

fn login_payload(root: &Path, body: &[u8], headers: &[(&str, &str)]) -> CompatApiResponse {
    let request = parse_json(body);
    let user = clean_text(
        request
            .get("email")
            .and_then(Value::as_str)
            .or_else(|| request.get("username").and_then(Value::as_str))
            .or_else(|| request.get("user").and_then(Value::as_str))
            .unwrap_or("operator"),
        120,
    );
    let user = if user.is_empty() {
        "operator".to_string()
    } else {
        user
    };
    let host = header_value(headers, "host");
    let seed = format!("{user}|{}|{host}", now_ms());
    let token = format!("ofg_{}", stable_hash(&seed, 28));
    let mut state = load_parity_state(root);
    state["auth"] = json!({
        "token": token,
        "user": user,
        "login_at_ms": now_ms()
    });
    save_parity_state(root, state.clone());
    CompatApiResponse {
        status: 200,
        payload: json!({
            "ok": true,
            "token": state.pointer("/auth/token").cloned().unwrap_or(Value::String(String::new())),
            "user": {
                "id": state.pointer("/auth/user").cloned().unwrap_or(Value::String("operator".to_string())),
                "email": state.pointer("/auth/user").cloned().unwrap_or(Value::String("operator".to_string()))
            },
            "expires_in": 86400
        }),
    }
}

fn logout_payload(root: &Path) -> CompatApiResponse {
    let mut state = load_parity_state(root);
    state["auth"] = json!({
        "token": "",
        "user": "operator",
        "login_at_ms": 0
    });
    save_parity_state(root, state);
    CompatApiResponse {
        status: 200,
        payload: json!({"ok": true, "logged_out": true}),
    }
}
