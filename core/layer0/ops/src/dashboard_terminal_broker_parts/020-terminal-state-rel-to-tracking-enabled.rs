
const TERMINAL_STATE_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/terminal_broker.json";
const TERMINAL_PERMISSION_POLICY_REL: &str =
    "client/runtime/config/terminal_command_permission_policy.json";
const OUTPUT_MAX_BYTES: usize = 32 * 1024;
const OUTPUT_TRUNCATION_MARKER: &str = "\n... (output truncated) ...\n";

#[derive(Debug, Clone)]
pub struct CommandResolution {
    pub requested_command: String,
    pub resolved_command: String,
    pub translated: bool,
    pub translation_reason: String,
    pub suggestions: Vec<String>,
}

fn now_iso() -> String {
    crate::now_iso()
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn parse_json(raw: &[u8]) -> Value {
    serde_json::from_slice::<Value>(raw).unwrap_or_else(|_| json!({}))
}

fn read_json(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(value) {
        let _ = fs::write(path, raw);
    }
}

fn as_object_mut<'a>(root: &'a mut Value, key: &str) -> &'a mut Map<String, Value> {
    if !root.get(key).map(Value::is_object).unwrap_or(false) {
        root[key] = json!({});
    }
    root[key]
        .as_object_mut()
        .unwrap_or_else(|| unreachable!("object shape"))
}

fn as_array_mut<'a>(root: &'a mut Value, key: &str) -> &'a mut Vec<Value> {
    if !root.get(key).map(Value::is_array).unwrap_or(false) {
        root[key] = Value::Array(Vec::new());
    }
    root[key]
        .as_array_mut()
        .unwrap_or_else(|| unreachable!("array shape"))
}

fn state_path(root: &Path) -> PathBuf {
    root.join(TERMINAL_STATE_REL)
}

fn normalize_session_id(raw: &str) -> String {
    let mut out = String::new();
    for ch in clean_text(raw, 120).chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        }
    }
    out
}

fn default_state() -> Value {
    json!({
        "type": "infring_dashboard_terminal_broker",
        "updated_at": now_iso(),
        "sessions": {},
        "history": []
    })
}

fn load_state(root: &Path) -> Value {
    let mut state = read_json(&state_path(root)).unwrap_or_else(default_state);
    if !state.is_object() {
        state = default_state();
    }
    let _ = as_object_mut(&mut state, "sessions");
    let _ = as_array_mut(&mut state, "history");
    state
}

fn save_state(root: &Path, mut state: Value) {
    state["updated_at"] = Value::String(now_iso());
    write_json(&state_path(root), &state);
}

fn resolve_cwd(root: &Path, requested: &str) -> PathBuf {
    let text = clean_text(requested, 240);
    if text.is_empty() {
        return root.to_path_buf();
    }
    if text == "/workspace" || text == "/workspace/" {
        return root.to_path_buf();
    }
    if let Some(rest) = text.strip_prefix("/workspace/") {
        let mut normalized = PathBuf::new();
        for component in Path::new(rest).components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    let _ = normalized.pop();
                }
                Component::Normal(part) => normalized.push(part),
                Component::Prefix(_) | Component::RootDir => {}
            }
        }
        return root.join(normalized);
    }
    let candidate = PathBuf::from(&text);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn cwd_allowed(root: &Path, cwd: &Path) -> bool {
    cwd.starts_with(root)
}

fn utf8_prefix_by_bytes(text: &str, max_bytes: usize) -> &str {
    if text.as_bytes().len() <= max_bytes {
        return text;
    }
    let mut end = max_bytes.min(text.len());
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn utf8_suffix_by_bytes(text: &str, max_bytes: usize) -> &str {
    if text.as_bytes().len() <= max_bytes {
        return text;
    }
    let mut start = text.len().saturating_sub(max_bytes);
    while start < text.len() && !text.is_char_boundary(start) {
        start += 1;
    }
    &text[start..]
}

fn truncate_output(text: &str) -> String {
    if text.as_bytes().len() <= OUTPUT_MAX_BYTES {
        return text.to_string();
    }
    let marker = OUTPUT_TRUNCATION_MARKER;
    let marker_len = marker.as_bytes().len();
    if OUTPUT_MAX_BYTES <= marker_len + 8 {
        return utf8_suffix_by_bytes(text, OUTPUT_MAX_BYTES).to_string();
    }
    let budget = OUTPUT_MAX_BYTES - marker_len;
    let head_budget = budget / 2;
    let tail_budget = budget - head_budget;
    let head = utf8_prefix_by_bytes(text, head_budget);
    let tail = utf8_suffix_by_bytes(text, tail_budget);
    if head.len() + tail.len() >= text.len() {
        return text.to_string();
    }
    let mut truncated = String::with_capacity(OUTPUT_MAX_BYTES);
    truncated.push_str(head);
    truncated.push_str(marker);
    truncated.push_str(tail);
    if truncated.as_bytes().len() <= OUTPUT_MAX_BYTES {
        return truncated;
    }
    let strict_budget = OUTPUT_MAX_BYTES - marker_len;
    let strict_head = utf8_prefix_by_bytes(text, strict_budget / 2);
    let strict_tail = utf8_suffix_by_bytes(text, strict_budget - strict_head.as_bytes().len());
    format!("{strict_head}{marker}{strict_tail}")
}

fn bool_env(name: &str, fallback: bool) -> bool {
    match std::env::var(name) {
        Ok(raw) => matches!(
            clean_text(&raw, 40).to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => fallback,
    }
}

fn primitives_enabled() -> bool {
    bool_env("INFRING_TURN_LOOP_PRIMITIVES_ENABLED", true)
}

fn pre_tool_gate_enabled() -> bool {
    primitives_enabled() && bool_env("INFRING_TOOL_PRE_GATE_ENABLED", true)
}

fn post_tool_filter_enabled() -> bool {
    primitives_enabled() && bool_env("INFRING_TOOL_POST_FILTER_ENABLED", true)
}

fn tracking_enabled() -> bool {
    primitives_enabled() && bool_env("INFRING_TOOL_TRACKING_ENABLED", true)
}
