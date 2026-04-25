// SRS: V12-SYNTHETIC-USER-HARNESS-001
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn post_agent_message(
    base_url: &str,
    agent_id: &str,
    request: &Value,
    timeout_seconds: u64,
) -> Value {
    let url = format!(
        "{}/api/agents/{}/message",
        base_url.trim_end_matches('/'),
        agent_id
    );
    let Ok(body) = serde_json::to_string(request) else {
        return json!({"ok": false, "transport_error": "request_json_encode_failed"});
    };
    let mut child = match Command::new("curl")
        .args([
            "-sS",
            "--max-time",
            &timeout_seconds.to_string(),
            "-H",
            "Content-Type: application/json",
            "-X",
            "POST",
            "--data-binary",
            "@-",
            &url,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            return json!({"ok": false, "transport_error": format!("curl_spawn_failed:{err}")})
        }
    };
    if let Some(stdin) = child.stdin.as_mut() {
        let _ = stdin.write_all(body.as_bytes());
    }
    match child.wait_with_output() {
        Ok(output) if output.status.success() => serde_json::from_slice(&output.stdout)
            .unwrap_or_else(
                |_| json!({"ok": false, "transport_error": "response_json_decode_failed"}),
            ),
        Ok(output) => json!({
            "ok": false,
            "transport_error": "curl_failed",
            "stderr": clean_text(&String::from_utf8_lossy(&output.stderr), 500),
        }),
        Err(err) => json!({"ok": false, "transport_error": format!("curl_wait_failed:{err}")}),
    }
}

pub(super) fn write_attention_events(dir: &Path, events: &[Value]) -> io::Result<()> {
    for event in events {
        let agent = str_opt(event, &["raw_event", "agent_id"]).unwrap_or("unknown");
        append_jsonl(
            dir.join(format!("{}.attention.jsonl", normalize_agent_id(agent))),
            std::slice::from_ref(event),
        )?;
    }
    Ok(())
}

pub(super) fn append_jsonl(path: impl AsRef<Path>, rows: &[Value]) -> io::Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    for row in rows {
        writeln!(file, "{}", serde_json::to_string(row)?)?;
    }
    Ok(())
}

pub(super) fn assistant_text(payload: &Value) -> String {
    for path in [
        &["response"][..],
        &["text"][..],
        &["message"][..],
        &["content"][..],
        &["assistant", "text"][..],
    ] {
        if let Some(raw) = str_opt(payload, path) {
            return clean_text(raw, 4_000);
        }
    }
    String::new()
}

pub(super) fn route_error_code(payload: &Value) -> Option<String> {
    if payload.get("ok").and_then(Value::as_bool) != Some(false) {
        return None;
    }
    str_opt(payload, &["error_code"])
        .or_else(|| str_opt(payload, &["error"]))
        .map(|raw| clean_text(raw, 160))
        .filter(|raw| !raw.is_empty())
}

pub(super) fn workflow_visible(payload: &Value) -> bool {
    payload.get("response_workflow").is_some()
        || payload.get("workflow_state").is_some()
        || payload.get("workflow_trace").is_some()
        || payload.get("workflow_events").is_some()
        || payload.get("workflow_visibility").is_some()
}

pub(super) fn is_local_dashboard_url(raw: &str) -> bool {
    let lower = raw.trim().to_ascii_lowercase();
    lower.starts_with("http://127.0.0.1")
        || lower.starts_with("http://localhost")
        || lower.starts_with("http://[::1]")
}

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

pub(super) fn parse_u64_flag(args: &[String], key: &str, default: u64) -> u64 {
    parse_flag(args, key)
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(default)
}

pub(super) fn read_json(path: &str) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| json!({}))
}

pub(super) fn write_json(path: &str, value: &Value) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))
}

pub(super) fn write_text(path: &str, content: &str) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}

pub(super) fn str_opt<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut cursor = value;
    for segment in path {
        cursor = cursor.get(*segment)?;
    }
    cursor.as_str().map(str::trim).filter(|raw| !raw.is_empty())
}

pub(super) fn str_at(value: &Value, path: &[&str], default: &str) -> String {
    str_opt(value, path).unwrap_or(default).to_string()
}

pub(super) fn string_array_at(value: &Value, path: &[&str]) -> Vec<String> {
    let mut cursor = value;
    for segment in path {
        let Some(next) = cursor.get(*segment) else {
            return Vec::new();
        };
        cursor = next;
    }
    cursor
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(|raw| clean_text(raw, 500))
        .collect()
}

pub(super) fn bool_at(value: &Value, path: &[&str], default: bool) -> bool {
    value
        .pointer(&format!("/{}", path.join("/")))
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

pub(super) fn u64_at(value: &Value, path: &[&str], default: u64) -> u64 {
    value
        .pointer(&format!("/{}", path.join("/")))
        .and_then(Value::as_u64)
        .unwrap_or(default)
}

pub(super) fn f64_at(value: &Value, path: &[&str], default: f64) -> f64 {
    value
        .pointer(&format!("/{}", path.join("/")))
        .and_then(Value::as_f64)
        .unwrap_or(default)
}

pub(super) fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

pub(super) fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len)
        .collect()
}

pub(super) fn normalize_for_compare(raw: &str) -> String {
    clean_text(&raw.to_ascii_lowercase(), 1_000)
}

pub(super) fn normalize_agent_id(raw: &str) -> String {
    clean_text(raw, 160)
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect()
}

pub(super) fn stable_hash_hex(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    format!("{:x}", hasher.finalize())
        .chars()
        .take(20)
        .collect()
}

pub(super) fn now_iso_like() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("unix_ms:{ms}")
}

pub(super) fn print_json_line(value: &Value) {
    let _ = writeln!(
        io::stdout(),
        "{}",
        serde_json::to_string(value).unwrap_or_default()
    );
}
