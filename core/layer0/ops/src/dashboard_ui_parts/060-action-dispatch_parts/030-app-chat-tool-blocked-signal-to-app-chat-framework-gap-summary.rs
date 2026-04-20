
fn app_chat_tool_blocked_signal(row: &Value) -> bool {
    let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 120)
        .to_ascii_lowercase();
    let error = clean_text(row.get("error").and_then(Value::as_str).unwrap_or(""), 240)
        .to_ascii_lowercase();
    let ty = clean_text(row.get("type").and_then(Value::as_str).unwrap_or(""), 240)
        .to_ascii_lowercase();
    row.get("blocked").and_then(Value::as_bool).unwrap_or(false)
        || status.contains("blocked")
        || status.contains("policy")
        || error.contains("blocked")
        || error.contains("permission")
        || error.contains("denied")
        || ty.contains("blocked")
        || ty.contains("policy")
}

fn app_chat_speculative_blocker_copy(text: &str) -> bool {
    let lowered = clean_text(text, 4_000).to_ascii_lowercase();
    lowered.contains("security controls")
        || lowered.contains("allowlists")
        || lowered.contains("proper authorization")
        || lowered.contains("invalid response attempt")
        || lowered.contains("preventing any web search operations")
}

fn app_chat_deferred_terminal_copy(text: &str) -> bool {
    let lowered = clean_text(text, 4_000).to_ascii_lowercase();
    lowered.starts_with("i'll get you an update")
        || lowered.contains("i'll get you an update on")
        || lowered.contains("would you like me to retry with a narrower query")
        || lowered.contains("would you like me to try a more specific query")
}

fn canonical_web_tooling_error_code(raw: &str) -> String {
    let cleaned = clean_text(raw, 140).to_ascii_lowercase();
    if cleaned.is_empty() {
        return "web_tool_error".to_string();
    }
    if cleaned.starts_with("web_tool_") {
        return cleaned;
    }
    crate::tool_output_match_filter::normalize_web_tooling_error_code(&cleaned)
}

fn canonical_action_error_payload(
    kind: &str,
    error_code: &str,
    status: i32,
    message: Option<&str>,
) -> Value {
    let code = clean_text(error_code, 140);
    let code = if code.is_empty() {
        "action_error".to_string()
    } else {
        code
    };
    let mut payload = json!({
        "ok": false,
        "type": kind,
        "error": code,
        "error_code": code,
        "status": status.max(0)
    });
    if let Some(message) = message {
        let cleaned = clean_chat_text_preserve_layout(message, 400);
        if !cleaned.is_empty() {
            payload["message"] = Value::String(cleaned);
        }
    }
    payload
}

fn app_chat_framework_gap_summary(raw_input: &str, tools: &[Value]) -> Option<String> {
    let input_lower = clean_text(raw_input, 1_000).to_ascii_lowercase();
    let joined = tools
        .iter()
        .map(|row| {
            [
                clean_text(row.get("result").and_then(Value::as_str).unwrap_or(""), 1_000),
                clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 120),
                clean_text(row.get("error").and_then(Value::as_str).unwrap_or(""), 240),
            ]
            .join(" ")
        })
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    if !(input_lower.contains("framework") || joined.contains("framework")) {
        return None;
    }
    let known = [
        "langgraph",
        "crewai",
        "autogen",
        "openai agents sdk",
        "smolagents",
    ];
    let mut found = Vec::<String>::new();
    let mut missing = Vec::<String>::new();
    for name in known {
        if joined.contains(name) {
            found.push(name.to_string());
        } else {
            missing.push(name.to_string());
        }
    }
    if found.is_empty() && missing.is_empty() {
        return None;
    }
    Some(format!(
        "Found: {}. Missing in this pass: {}.",
        if found.is_empty() {
            "none".to_string()
        } else {
            found.join(", ")
        },
        if missing.is_empty() {
            "none".to_string()
        } else {
            missing.join(", ")
        }
    ))
}
