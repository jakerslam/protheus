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
    let _ = writeln!(io::stdout(), "{}", serde_json::to_string(value).unwrap_or_default());
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
        "high" | "warn" | "warning" => "warn".to_string(),
        _ => "info".to_string(),
    }
}

pub(super) fn stable_hash_hex(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    let digest = hasher.finalize();
    digest.iter().take(8).map(|byte| format!("{byte:02x}")).collect()
}

pub(super) fn now_iso_like() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("unix_ms:{ms}")
}
