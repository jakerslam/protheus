use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn parse_flag(args: &[String], key: &str) -> Option<String> {
    let inline = format!("--{key}=");
    for (idx, arg) in args.iter().enumerate() {
        if let Some(value) = arg.strip_prefix(&inline) {
            return Some(value.to_string());
        }
        if arg == &format!("--{key}") {
            return args.get(idx + 1).cloned();
        }
    }
    None
}

pub(super) fn parse_bool_flag(args: &[String], key: &str, default: bool) -> bool {
    parse_flag(args, key)
        .map(|raw| matches!(raw.trim(), "1" | "true" | "TRUE" | "yes" | "on"))
        .unwrap_or(default)
}

pub(super) fn read_json(path: &str) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| json!({}))
}

pub(super) fn write_json(path: impl AsRef<Path>, value: &Value) -> io::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))
}

pub(super) fn write_jsonl(path: impl AsRef<Path>, rows: &[Value]) -> io::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    let mut out = String::new();
    for row in rows {
        out.push_str(&serde_json::to_string(row)?);
        out.push('\n');
    }
    fs::write(path, out)
}

pub(super) fn print_json_line(value: &Value) {
    let _ = writeln!(
        io::stdout(),
        "{}",
        serde_json::to_string(value).unwrap_or_default()
    );
}

pub(super) fn str_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut cursor = value;
    for segment in path {
        cursor = cursor.get(*segment)?;
    }
    cursor.as_str().map(str::trim).filter(|raw| !raw.is_empty())
}

pub(super) fn required_str(value: &Value, path: &[&str], default: &str) -> String {
    clean_text(str_at(value, path).unwrap_or(default), 2_000)
}

pub(super) fn array_at(value: &Value, path: &[&str]) -> Vec<Value> {
    let mut cursor = value;
    for segment in path {
        let Some(next) = cursor.get(*segment) else {
            return Vec::new();
        };
        cursor = next;
    }
    cursor.as_array().cloned().unwrap_or_default()
}

pub(super) fn string_array_at(value: &Value, path: &[&str]) -> Vec<String> {
    array_at(value, path)
        .iter()
        .filter_map(Value::as_str)
        .map(|raw| clean_text(raw, 500))
        .filter(|raw| !raw.is_empty())
        .collect()
}

pub(super) fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len)
        .collect()
}

pub(super) fn normalize_agent_id(raw: &str) -> String {
    clean_text(raw, 160)
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect()
}

pub(super) fn normalized_severity(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "critical" => "critical".to_string(),
        "high" => "high".to_string(),
        "warn" | "warning" => "warn".to_string(),
        _ => "info".to_string(),
    }
}

pub(super) fn agent_id_from_source_event(raw: &str) -> Option<String> {
    let tail = if let Some(idx) = raw.find("agent:") {
        &raw[idx + "agent:".len()..]
    } else if let Some(idx) = raw.find("agent-") {
        &raw[idx..]
    } else {
        return None;
    };
    let token = tail
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'))
        .next()
        .unwrap_or("");
    let agent = normalize_agent_id(token);
    if agent.is_empty() {
        None
    } else {
        Some(agent)
    }
}

pub(super) fn chat_monitor_next_action(row: &Value) -> String {
    let direct = required_str(row, &["next_action"], "");
    if !direct.is_empty() {
        return direct;
    }
    let body = str_at(row, &["body"]).unwrap_or("");
    section_after_heading(body, "Next action").unwrap_or_default()
}

pub(super) fn chat_monitor_suggested_test(row: &Value) -> String {
    string_array_at(row, &["acceptance_criteria"])
        .into_iter()
        .next()
        .unwrap_or_else(|| {
            "Replay the eval chat-monitor evidence and verify the issue no longer recurs."
                .to_string()
        })
}

pub(super) fn evidence_summary(row: &Value) -> String {
    for path in [
        ["exact_evidence", "prompt"].as_slice(),
        ["evidence", "sanitized_user_text"].as_slice(),
        ["actual_behavior"].as_slice(),
    ] {
        let raw = required_str(row, path, "");
        if !raw.is_empty() {
            return clean_text(&raw, 240);
        }
    }
    for evidence in array_at(row, &["evidence"]) {
        for path in [["snippet"].as_slice(), ["turn_id"].as_slice()] {
            let raw = required_str(&evidence, path, "");
            if !raw.is_empty() {
                return clean_text(&raw, 240);
            }
        }
    }
    String::new()
}

fn section_after_heading(body: &str, heading: &str) -> Option<String> {
    let marker = format!("{heading}:");
    let (_, tail) = body.split_once(&marker)?;
    let section = tail
        .lines()
        .map(str::trim)
        .skip_while(|line| line.is_empty())
        .take_while(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let cleaned = clean_text(&section, 500);
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

pub(super) fn stable_hash_hex(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    let digest = hasher.finalize();
    digest
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(super) fn now_iso_like() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("unix_ms:{ms}")
}
