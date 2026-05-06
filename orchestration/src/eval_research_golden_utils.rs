use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn load_responses_by_case(path: &str) -> BTreeMap<String, Value> {
    let input = read_json(path);
    let rows = input
        .get("responses")
        .or_else(|| input.get("cases"))
        .or_else(|| input.get("turns"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut by_case = BTreeMap::new();
    for row in rows {
        let case_id = str_at(&row, &["case_id"], &str_at(&row, &["id"], ""));
        if case_id.is_empty() {
            continue;
        }
        let payload = row
            .get("response_payload")
            .or_else(|| row.get("payload"))
            .or_else(|| row.get("mock_response"))
            .cloned()
            .unwrap_or(row);
        by_case.insert(case_id, payload);
    }
    by_case
}

pub(super) fn response_sequence_payload(source: &Value, index: usize) -> Option<Value> {
    source
        .get("response_sequence")
        .or_else(|| source.get("__research_golden_response_sequence"))
        .and_then(Value::as_array)
        .and_then(|rows| rows.get(index))
        .cloned()
}

pub(super) fn payload_has_pending_tool_confirmation(payload: &Value) -> bool {
    [
        "/pending_tool_request/status",
        "/response_workflow/pending_tool_request/status",
        "/response_workflow/manual_toolbox_pending_tool_request/status",
        "/response_finalization/pending_tool_request/status",
    ]
    .iter()
    .any(|pointer| {
        payload
            .pointer(pointer)
            .and_then(Value::as_str)
            .map(|status| status == "pending_confirmation")
            .unwrap_or(false)
    })
}

pub(super) fn create_live_agent(
    base_url: &str,
    case_id: &str,
    model_ref: Option<&str>,
    timeout_seconds: u64,
) -> Option<String> {
    let name = format!("Research Golden {}", clean_text(case_id, 80));
    let payload = post_json(
        base_url,
        "/api/agents",
        &json!({
            "name": name,
            "role": "analyst"
        }),
        timeout_seconds,
    );
    str_opt(&payload, &["agent_id"])
        .or_else(|| str_opt(&payload, &["id"]))
        .map(normalize_agent_id)
        .filter(|agent_id| !agent_id.is_empty())
        .and_then(|agent_id| {
            if set_live_agent_model(base_url, &agent_id, model_ref, timeout_seconds) {
                Some(agent_id)
            } else {
                None
            }
        })
}

fn set_live_agent_model(
    base_url: &str,
    agent_id: &str,
    model_ref: Option<&str>,
    timeout_seconds: u64,
) -> bool {
    let Some(model_ref) = model_ref.map(|raw| clean_text(raw, 240)) else {
        return true;
    };
    if model_ref.is_empty() {
        return true;
    }
    let response = curl_json(
        "PUT",
        base_url,
        &format!("/api/agents/{agent_id}/model"),
        &json!({ "model": model_ref }),
        timeout_seconds,
    );
    response.get("ok").and_then(Value::as_bool).unwrap_or(false)
}

pub(super) fn delete_live_agent(base_url: &str, agent_id: &str, timeout_seconds: u64) -> Value {
    curl_json(
        "DELETE",
        base_url,
        &format!("/api/agents/{agent_id}"),
        &json!({}),
        timeout_seconds,
    )
}

pub(super) fn post_agent_message(
    base_url: &str,
    agent_id: &str,
    request: &Value,
    timeout_seconds: u64,
) -> Value {
    let path = format!("/api/agents/{agent_id}/message");
    let response = post_json(base_url, &path, request, timeout_seconds);
    if is_retryable_curl_timeout(&response) {
        let retry_timeout_seconds = timeout_seconds.saturating_add(15).min(120);
        return post_json(base_url, &path, request, retry_timeout_seconds);
    }
    response
}

fn post_json(base_url: &str, path: &str, request: &Value, timeout_seconds: u64) -> Value {
    curl_json("POST", base_url, path, request, timeout_seconds)
}

fn is_retryable_curl_timeout(payload: &Value) -> bool {
    payload.get("transport_error").and_then(Value::as_str) == Some("curl_failed")
        && payload
            .get("stderr")
            .and_then(Value::as_str)
            .map(|stderr| stderr.to_ascii_lowercase().contains("timed out"))
            .unwrap_or(false)
}

fn curl_json(
    method: &str,
    base_url: &str,
    path: &str,
    request: &Value,
    timeout_seconds: u64,
) -> Value {
    let url = format!("{}{}", base_url.trim_end_matches('/'), path);
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
            method,
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

pub(super) fn assistant_text(payload: &Value) -> String {
    for path in [
        &["response"][..],
        &["text"][..],
        &["message"][..],
        &["content"][..],
        &["assistant", "text"][..],
    ] {
        if let Some(raw) = str_opt(payload, path) {
            return clean_text(raw, 12_000);
        }
    }
    String::new()
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

pub(super) fn append_jsonl(path: &str, rows: &[Value]) -> io::Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    if let Some(parent) = Path::new(path).parent() {
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
    clean_text(&raw.to_ascii_lowercase(), 4_000)
}

pub(super) fn normalize_agent_id(raw: &str) -> String {
    clean_text(raw, 160)
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect()
}

pub(super) fn is_local_dashboard_url(raw: &str) -> bool {
    let lower = raw.trim().to_ascii_lowercase();
    lower.starts_with("http://127.0.0.1")
        || lower.starts_with("http://localhost")
        || lower.starts_with("http://[::1]")
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

#[cfg(test)]
mod eval_research_golden_utils_tests {
    use super::*;

    #[test]
    fn retryable_curl_timeout_matches_timeout_stderr() {
        let payload = json!({
            "ok": false,
            "transport_error": "curl_failed",
            "stderr": "curl: (28) Operation timed out after 45010 milliseconds with 0 bytes received"
        });
        assert!(is_retryable_curl_timeout(&payload));
    }

    #[test]
    fn retryable_curl_timeout_rejects_non_timeout_failures() {
        let payload = json!({
            "ok": false,
            "transport_error": "curl_failed",
            "stderr": "curl: (7) Failed to connect to 127.0.0.1 port 4173"
        });
        assert!(!is_retryable_curl_timeout(&payload));
    }
}
