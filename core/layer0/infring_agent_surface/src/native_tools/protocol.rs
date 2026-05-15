use crate::native_tools::receipts::NativeToolReceipt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NativeToolCall {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub args: Value,
}

pub fn parse_native_tool_calls(raw: &str) -> Vec<NativeToolCall> {
    let cleaned = strip_ansi(raw);
    let candidates = json_candidates(&cleaned);
    for candidate in candidates {
        if let Ok(value) = serde_json::from_str::<Value>(&candidate) {
            let calls = tool_calls_from_value(&value);
            if !calls.is_empty() {
                return calls;
            }
        }
    }
    Vec::new()
}

pub fn native_tool_observation_prompt(receipts: &[NativeToolReceipt]) -> String {
    json!({
        "native_tool_observations": receipts,
        "instruction": "Use these receipts as authoritative. Continue with another tool call if needed, otherwise provide the final answer."
    })
    .to_string()
}

fn tool_calls_from_value(value: &Value) -> Vec<NativeToolCall> {
    if let Some(items) = value.get("tool_calls").and_then(Value::as_array) {
        return items
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| tool_call_from_value(item, idx))
            .collect();
    }
    if value.get("tool").is_some() || value.get("name").is_some() {
        return tool_call_from_value(value, 0).into_iter().collect();
    }
    Vec::new()
}

fn tool_call_from_value(value: &Value, idx: usize) -> Option<NativeToolCall> {
    let name = value
        .get("name")
        .or_else(|| value.get("tool"))
        .or_else(|| value.get("tool_name"))
        .and_then(Value::as_str)?
        .trim()
        .to_string();
    let id = value
        .get("id")
        .or_else(|| value.get("call_id"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| format!("call_{}", idx + 1));
    let args = value
        .get("args")
        .or_else(|| value.get("arguments"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    Some(NativeToolCall { id, name, args })
}

fn json_candidates(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = raw;
    while let Some(start) = rest.find("```") {
        rest = &rest[start + 3..];
        if let Some(newline) = rest.find('\n') {
            rest = &rest[newline + 1..];
        }
        let Some(end) = rest.find("```") else {
            break;
        };
        out.push(rest[..end].trim().to_string());
        rest = &rest[end + 3..];
    }
    if let Some(object) = first_balanced_object(raw) {
        out.push(object);
    }
    out.push(raw.trim().to_string());
    out
}

fn first_balanced_object(raw: &str) -> Option<String> {
    let mut start = None;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (idx, ch) in raw.char_indices() {
        if start.is_none() {
            if ch == '{' {
                start = Some(idx);
                depth = 1;
            }
            continue;
        }
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return start.map(|start_idx| raw[start_idx..=idx].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

fn strip_ansi(raw: &str) -> String {
    let mut out = String::new();
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}
