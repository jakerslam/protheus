
const CHANNEL_REGISTRY_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/channel_registry.json";
const CHANNEL_QR_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/channel_qr_sessions.json";

fn clean_text(raw: &str, max_len: usize) -> String {
    lane_utils::clean_text(Some(raw), max_len.max(1))
}

fn parse_json(raw: &[u8]) -> Value {
    serde_json::from_slice::<Value>(raw).unwrap_or_else(|_| json!({}))
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
        let _ = fs::write(path, raw);
    }
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

fn parse_non_negative_i64(value: Option<&Value>, fallback: i64) -> i64 {
    value.and_then(Value::as_i64).unwrap_or(fallback).max(0)
}

fn error_text_from_value(value: &Value) -> String {
    if let Some(text) = value.get("error").and_then(Value::as_str) {
        return clean_text(text, 280);
    }
    if let Some(text) = value
        .get("error")
        .and_then(Value::as_object)
        .and_then(|row| row.get("message"))
        .and_then(Value::as_str)
    {
        return clean_text(text, 280);
    }
    if let Some(text) = value.get("message").and_then(Value::as_str) {
        return clean_text(text, 280);
    }
    clean_text(&value.to_string(), 280)
}

fn config_text(channel: &Value, keys: &[&str], max_len: usize) -> String {
    let Some(config) = channel.get("config").and_then(Value::as_object) else {
        return String::new();
    };
    for key in keys {
        let value = clean_text(
            config.get(*key).and_then(Value::as_str).unwrap_or(""),
            max_len,
        );
        if !value.is_empty() {
            return value;
        }
    }
    String::new()
}

fn channel_flag(channel: &Value, key: &str, fallback: bool) -> bool {
    channel
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or(fallback)
}

fn channel_adapter(channel: &Value) -> String {
    clean_text(
        channel
            .get("runtime_adapter")
            .and_then(Value::as_str)
            .unwrap_or("generic_http"),
        64,
    )
}

fn channel_probe_method(channel: &Value) -> String {
    clean_text(
        channel
            .get("probe_method")
            .and_then(Value::as_str)
            .unwrap_or("get"),
        12,
    )
    .to_lowercase()
}

fn channel_token(channel: &Value) -> String {
    config_text(
        channel,
        &[
            "bot_token",
            "private_integration_token",
            "access_token",
            "api_key",
            "token",
            "secret",
            "key",
        ],
        600,
    )
}

fn channel_endpoint(channel: &Value) -> String {
    let endpoint = config_text(
        channel,
        &[
            "webhook_url",
            "endpoint",
            "base_url",
            "api_url",
            "url",
            "host",
        ],
        1200,
    );
    if endpoint.eq_ignore_ascii_case("default") {
        String::new()
    } else {
        endpoint
    }
}

fn normalize_url(raw: &str) -> String {
    let mut url = clean_text(raw, 1200);
    while url.ends_with('/') {
        url.pop();
    }
    url
}

fn error_response(message: &str) -> CompatApiResponse {
    CompatApiResponse {
        status: 200,
        payload: json!({"ok": true, "status": "error", "message": clean_text(message, 320)}),
    }
}

fn ok_response(message: &str, details: Value) -> CompatApiResponse {
    CompatApiResponse {
        status: 200,
        payload: json!({
            "ok": true,
            "status": "ok",
            "message": clean_text(message, 320),
            "details": details
        }),
    }
}
