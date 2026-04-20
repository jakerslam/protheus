
const HANDS_STATE_REL: &str = "client/runtime/local/state/ui/infring_dashboard/hands_state.json";
const BROWSER_PLACEHOLDER_SCREENSHOT_BASE64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO9f3n8AAAAASUVORK5CYII=";

fn clean_text(raw: &str, max_len: usize) -> String {
    lane_utils::clean_text(Some(raw), max_len.max(1))
}

fn clean_id(raw: &str, max_len: usize) -> String {
    let mut out = String::new();
    for ch in clean_text(raw, max_len).to_ascii_lowercase().chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        } else if ch == ' ' {
            out.push('-');
        }
    }
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    out.trim_matches('-').to_string()
}

fn state_path(root: &Path, rel: &str) -> PathBuf {
    root.join(rel)
}

fn read_json(path: &Path) -> Option<Value> {
    lane_utils::read_json(path)
}

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(value) {
        let _ = fs::write(path, format!("{raw}\n"));
    }
}

fn parse_json(body: &[u8]) -> Value {
    serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}))
}

fn make_id(prefix: &str, seed: &Value) -> String {
    let hash = crate::deterministic_receipt_hash(seed);
    format!(
        "{}-{}",
        clean_id(prefix, 24),
        hash.chars().take(10).collect::<String>()
    )
}

fn as_bool(value: Option<&Value>, fallback: bool) -> bool {
    value.and_then(Value::as_bool).unwrap_or(fallback)
}

fn parse_rfc3339(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|v| v.with_timezone(&Utc))
}

fn server_platform() -> String {
    match std::env::consts::OS {
        "macos" => "macos".to_string(),
        "windows" => "windows".to_string(),
        _ => "linux".to_string(),
    }
}

fn command_available(command: &str) -> bool {
    let cmd = clean_id(command, 60);
    if cmd.is_empty() {
        return false;
    }
    if cfg!(windows) {
        Command::new("cmd")
            .args(["/C", "where", &cmd])
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    } else {
        Command::new("sh")
            .args(["-lc", &format!("command -v {cmd} >/dev/null 2>&1")])
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }
}

fn env_present(key: &str) -> bool {
    let cleaned = clean_text(key, 120);
    if cleaned.is_empty() {
        return false;
    }
    std::env::var(cleaned)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
}

fn load_state(root: &Path) -> Value {
    read_json(&state_path(root, HANDS_STATE_REL)).unwrap_or_else(|| {
        json!({
            "type": "infring_dashboard_hands_state",
            "updated_at": crate::now_iso(),
            "instances": [],
            "hand_config": {}
        })
    })
}

fn save_state(root: &Path, mut state: Value) {
    state["updated_at"] = Value::String(crate::now_iso());
    write_json(&state_path(root, HANDS_STATE_REL), &state);
}

fn hand_config(state: &Value, hand_id: &str) -> Map<String, Value> {
    state
        .pointer(&format!("/hand_config/{}", clean_id(hand_id, 120)))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

fn set_hand_config(state: &mut Value, hand_id: &str, config: &Map<String, Value>) {
    if !state
        .get("hand_config")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["hand_config"] = Value::Object(Map::new());
    }
    if let Some(configs) = state.get_mut("hand_config").and_then(Value::as_object_mut) {
        configs.insert(clean_id(hand_id, 120), Value::Object(config.clone()));
    }
}

fn normalize_instance(instance: &Value) -> Value {
    let now = crate::now_iso();
    let instance_id = {
        let raw = clean_id(
            instance
                .get("instance_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        if raw.is_empty() {
            make_id(
                "inst",
                &json!({"hand_id": instance.get("hand_id").cloned().unwrap_or(Value::Null), "ts": now}),
            )
        } else {
            raw
        }
    };
    let hand_id = clean_id(
        instance
            .get("hand_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    let agent_id = clean_id(
        instance
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    let agent_name = clean_text(
        instance
            .get("agent_name")
            .and_then(Value::as_str)
            .unwrap_or(""),
        140,
    );
    let status = clean_text(
        instance
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("Active"),
        40,
    );
    let activated_at = clean_text(
        instance
            .get("activated_at")
            .and_then(Value::as_str)
            .unwrap_or(&now),
        80,
    );
    json!({
        "instance_id": instance_id,
        "hand_id": hand_id,
        "agent_id": agent_id,
        "agent_name": agent_name,
        "status": if status.is_empty() { "Active" } else { &status },
        "activated_at": if activated_at.is_empty() { &now } else { &activated_at },
        "updated_at": clean_text(instance.get("updated_at").and_then(Value::as_str).unwrap_or(&now), 80),
        "config": instance.get("config").cloned().unwrap_or_else(|| json!({}))
    })
}
