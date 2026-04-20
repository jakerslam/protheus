
const SESSION_ANALYTICS_TUNING_REL: &str =
    "local/state/ops/session_command_tracking/nightly_tuning.json";

#[cfg(test)]
const PROVIDER_REGISTRY_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/provider_registry.json";

fn clean_text(raw: &str, max_len: usize) -> String {
    lane_utils::clean_text(Some(raw), max_len.max(1))
}

fn bool_env(name: &str, fallback: bool) -> bool {
    lane_utils::parse_bool(std::env::var(name).ok().as_deref(), fallback)
}

fn read_json(path: &Path) -> Option<Value> {
    lane_utils::read_json(path)
}

fn load_session_analytics_tuning(root: &Path) -> Value {
    if !bool_env("INFRING_SESSION_ANALYTICS_ROUTING_ENABLED", true) {
        return json!({});
    }
    read_json(&root.join(SESSION_ANALYTICS_TUNING_REL)).unwrap_or_else(|| json!({}))
}

fn parse_f64_value(value: Option<&Value>) -> f64 {
    value
        .and_then(|row| {
            row.as_f64()
                .or_else(|| row.as_i64().map(|num| num as f64))
                .or_else(|| row.as_u64().map(|num| num as f64))
                .or_else(|| {
                    row.as_str()
                        .and_then(|text| clean_text(text, 40).parse::<f64>().ok())
                })
        })
        .unwrap_or(0.0)
}

fn parse_i64(value: Option<&Value>, fallback: i64) -> i64 {
    value.and_then(Value::as_i64).unwrap_or(fallback)
}

fn parse_bool(value: Option<&Value>, fallback: bool) -> bool {
    value.and_then(Value::as_bool).unwrap_or(fallback)
}

fn model_id_is_placeholder(model_id: &str) -> bool {
    matches!(
        clean_text(model_id, 240).to_ascii_lowercase().as_str(),
        "model" | "<model>" | "(model)"
    )
}

#[cfg(test)]
fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(value) {
        let _ = fs::write(path, raw);
    }
}

#[derive(Clone)]
struct ModelRow {
    provider: String,
    model: String,
    display_name: String,
    specialty: String,
    specialty_tags: Vec<String>,
    is_local: bool,
    supports_chat: bool,
    needs_key: bool,
    auth_status: String,
    reachable: bool,
    power_signal: i64,
    cost_signal: i64,
    param_count_billion: i64,
    context_size: i64,
    deployment_kind: String,
    local_download_path: String,
    download_available: bool,
    max_output_tokens: i64,
    tier: String,
}

fn scale_to_five(value: i64, min: i64, max: i64) -> i64 {
    if max <= min {
        return 3;
    }
    let ratio = (value - min) as f64 / (max - min) as f64;
    (1.0 + ratio * 4.0).round().clamp(1.0, 5.0) as i64
}
