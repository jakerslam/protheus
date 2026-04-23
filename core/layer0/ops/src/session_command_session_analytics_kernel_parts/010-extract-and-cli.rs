fn usage() {
    println!("session-command-session-analytics-kernel commands:");
    println!(
        "  infring-ops session-command-session-analytics-kernel <extract-jsonl|classify-jsonl|adoption-report> [--payload=<json>|--payload-base64=<base64_json>]"
    );
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.chars()
        .take(max_len)
        .collect::<String>()
        .trim()
        .to_string()
}

fn strip_trailing_question_marks(raw: &str) -> String {
    clean_text(raw, 220)
        .trim_end_matches(|ch: char| matches!(ch, '?' | '？' | '﹖' | '⸮' | '؟' | '՞'))
        .trim()
        .to_string()
}

fn normalize_follow_up_suggestion(raw: &str) -> String {
    let mut normalized = strip_trailing_question_marks(raw);
    if normalized.is_empty() {
        return String::new();
    }
    let lowered = normalized.to_ascii_lowercase();
    let prefixes = [
        "should i",
        "should we",
        "want me to",
        "do you want me to",
        "would you like me to",
        "do you want us to",
        "would you like us to",
        "can i",
        "could i",
        "can we",
        "could we",
        "i can",
        "i could",
        "i will",
        "i'll",
        "we can",
        "we could",
        "we will",
        "we'll",
        "let me",
        "let us",
    ];
    for prefix in prefixes {
        if lowered == prefix || lowered.starts_with(&format!("{prefix} ")) {
            normalized = normalized.chars().skip(prefix.len()).collect::<String>();
            normalized = normalized
                .trim_start_matches(|ch: char| {
                    ch.is_whitespace() || matches!(ch, ':' | ';' | ',' | '-' | '.')
                })
                .to_string();
            break;
        }
    }
    normalized = strip_trailing_question_marks(&normalized);
    if normalized.is_empty() {
        return String::new();
    }
    let first = normalized
        .split_whitespace()
        .next()
        .map(|row| row.to_ascii_lowercase())
        .unwrap_or_default();
    let starts_with_action = matches!(
        first.as_str(),
        "add"
            | "build"
            | "check"
            | "compare"
            | "convert"
            | "create"
            | "debug"
            | "execute"
            | "fix"
            | "generate"
            | "implement"
            | "inspect"
            | "map"
            | "optimize"
            | "repair"
            | "replace"
            | "run"
            | "summarize"
            | "test"
            | "validate"
            | "verify"
    );
    if !starts_with_action {
        normalized = format!("Run {normalized}");
    }
    let normalized = strip_trailing_question_marks(&clean_text(&normalized, 220));
    let mut chars = normalized.chars();
    let first = match chars.next() {
        Some(ch) => ch.to_ascii_uppercase(),
        None => return String::new(),
    };
    let rest = chars.collect::<String>();
    format!("{first}{rest}")
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    crate::contract_lane_utils::cli_receipt(kind, payload)
}

fn cli_error(kind: &str, error: &str) -> Value {
    crate::contract_lane_utils::cli_error(kind, error)
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("session_analytics_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("session_analytics_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("session_analytics_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("session_analytics_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn normalize_tool_content_type(value: Option<&str>) -> String {
    value.unwrap_or("").trim().to_ascii_lowercase()
}

fn is_tool_call_content_type(value: &str) -> bool {
    matches!(value, "toolcall" | "tool_call" | "tooluse" | "tool_use")
}

fn is_tool_result_content_type(value: &str) -> bool {
    matches!(value, "toolresult" | "tool_result" | "toolresponse" | "tool_response")
}

fn resolve_tool_use_id(block: &Value) -> Option<String> {
    for key in ["id", "tool_use_id", "toolUseId", "tool_call_id", "toolCallId"] {
        let Some(raw) = block.get(key).and_then(Value::as_str) else {
            continue;
        };
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

fn resolve_tool_call_command(block: &Value) -> Option<String> {
    [
        block.pointer("/input/command").and_then(Value::as_str),
        block.pointer("/input/cmd").and_then(Value::as_str),
        block.pointer("/args/command").and_then(Value::as_str),
        block.pointer("/args/cmd").and_then(Value::as_str),
        block.pointer("/arguments/command").and_then(Value::as_str),
        block.pointer("/arguments/cmd").and_then(Value::as_str),
        block.get("command").and_then(Value::as_str),
        block.get("cmd").and_then(Value::as_str),
    ]
    .into_iter()
    .flatten()
    .map(|raw| clean_text(raw, 2000))
    .find(|value| !value.is_empty())
}

fn resolve_tool_result_text(block: &Value) -> String {
    if let Some(text) = block.get("content").and_then(Value::as_str) {
        return clean_text(text, 1000);
    }
    if let Some(text) = block.get("output_text").and_then(Value::as_str) {
        return clean_text(text, 1000);
    }
    if let Some(text) = block.get("outputText").and_then(Value::as_str) {
        return clean_text(text, 1000);
    }
    if let Some(items) = block.get("content").and_then(Value::as_array) {
        let mut parts = Vec::<String>::new();
        for item in items {
            let text = item
                .as_str()
                .or_else(|| item.get("text").and_then(Value::as_str))
                .or_else(|| item.get("content").and_then(Value::as_str));
            if let Some(raw) = text {
                let cleaned = clean_text(raw, 400);
                if !cleaned.is_empty() {
                    parts.push(cleaned);
                }
            }
        }
        if !parts.is_empty() {
            return clean_text(&parts.join(" "), 1000);
        }
    }
    String::new()
}

fn jsonl_line_might_include_tool_blocks(line: &str) -> bool {
    let lowered = line.to_ascii_lowercase();
    lowered.contains("\"bash\"")
        || lowered.contains("\"tool_use\"")
        || lowered.contains("\"tooluse\"")
        || lowered.contains("\"tool_call\"")
        || lowered.contains("\"toolcall\"")
        || lowered.contains("\"tool_result\"")
        || lowered.contains("\"toolresult\"")
}

fn extract_commands_from_jsonl(session_id: &str, jsonl: &str) -> Vec<ExtractedCommand> {
    let mut pending_tool_uses = Vec::<(String, String, usize)>::new();
    let mut tool_results = HashMap::<String, (usize, String, bool)>::new();
    let mut sequence_counter = 0usize;

    for line in jsonl.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !jsonl_line_might_include_tool_blocks(trimmed) {
            continue;
        }
        let parsed = match serde_json::from_str::<Value>(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let entry_type = parsed
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        let content_blocks = parsed
            .pointer("/message/content")
            .or_else(|| parsed.get("content"))
            .and_then(Value::as_array);
        match entry_type.as_str() {
            "assistant" => {
                let Some(content) = content_blocks else {
                    continue;
                };
                for block in content {
                    let content_type =
                        normalize_tool_content_type(block.get("type").and_then(Value::as_str));
                    if !is_tool_call_content_type(&content_type) {
                        continue;
                    }
                    let is_bash = block
                        .get("name")
                        .and_then(Value::as_str)
                        .map(|value| value.trim().eq_ignore_ascii_case("bash"))
                        .unwrap_or(false);
                    if !is_bash {
                        continue;
                    }
                    let Some(tool_id) = resolve_tool_use_id(block) else {
                        continue;
                    };
                    let Some(normalized) = resolve_tool_call_command(block) else {
                        continue;
                    };
                    pending_tool_uses.push((tool_id, normalized, sequence_counter));
                    sequence_counter += 1;
                }
            }
            "user" | "tool" | "tool_result" => {
                let Some(content) = content_blocks else {
                    continue;
                };
                for block in content {
                    let content_type =
                        normalize_tool_content_type(block.get("type").and_then(Value::as_str));
                    if !is_tool_result_content_type(&content_type) {
                        continue;
                    }
                    let Some(tool_id) = resolve_tool_use_id(block) else {
                        continue;
                    };
                    let text = resolve_tool_result_text(block);
                    let is_error = block
                        .get("is_error")
                        .and_then(Value::as_bool)
                        .or_else(|| block.get("isError").and_then(Value::as_bool))
                        .unwrap_or(false);
                    tool_results.insert(tool_id, (text.len(), text, is_error));
                }
            }
            _ => {}
        }
    }

    let mut out = Vec::<ExtractedCommand>::new();
    for (tool_id, command, sequence_index) in pending_tool_uses {
        let (output_len, output_preview, is_error) = tool_results
            .get(&tool_id)
            .map(|row| (Some(row.0), Some(row.1.clone()), row.2))
            .unwrap_or((None, None, false));
        let _ = session_id;
        out.push(ExtractedCommand {
            command,
            output_len,
            output_preview,
            is_error,
            sequence_index,
        });
    }
    out
}
