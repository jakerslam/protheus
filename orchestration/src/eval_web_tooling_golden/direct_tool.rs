use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};

use super::super::eval_research_golden_utils::{clean_text, normalize_for_compare, str_at};

pub(super) fn invoke_direct_tool(
    base_url: &str,
    tool_name: &str,
    request: &Value,
    timeout_seconds: u64,
) -> Value {
    let path = match tool_name {
        "web_fetch" => "/api/web/fetch",
        "web_search" => "/api/web/search",
        _ => "/api/batch-query",
    };
    curl_json("POST", base_url, path, request, timeout_seconds)
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
            return json!({"ok": false, "transport_error": format!("curl_spawn_failed:{err}")});
        }
    };
    if let Some(stdin) = child.stdin.as_mut() {
        let _ = stdin.write_all(body.as_bytes());
    }
    match child.wait_with_output() {
        Ok(output) if output.status.success() => serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|_| {
                json!({
                    "ok": false,
                    "transport_error": "response_json_decode_failed",
                    "stdout": clean_text(&String::from_utf8_lossy(&output.stdout), 4_000)
                })
            }),
        Ok(output) => json!({
            "ok": false,
            "transport_error": "curl_failed",
            "stderr": clean_text(&String::from_utf8_lossy(&output.stderr), 4_000),
            "stdout": clean_text(&String::from_utf8_lossy(&output.stdout), 4_000),
        }),
        Err(err) => json!({"ok": false, "transport_error": format!("curl_wait_failed:{err}")}),
    }
}

pub(super) fn payload_is_transport_failure(payload: &Value) -> bool {
    if payload
        .as_object()
        .map(|map| map.is_empty())
        .unwrap_or(false)
    {
        return true;
    }
    let transport_error = str_at(payload, &["transport_error"], "");
    if !transport_error.is_empty() {
        return true;
    }
    let error = normalize_for_compare(&str_at(payload, &["error"], ""));
    [
        "socket hang up",
        "connection reset",
        "connection refused",
        "failed to connect",
        "couldn't connect",
        "response_json_decode_failed",
        "curl_failed",
        "network error",
        "econnreset",
        "econnrefused",
        "timed out",
    ]
    .iter()
    .any(|needle| error.contains(*needle))
        || payload
            .get("stderr")
            .and_then(Value::as_str)
            .map(|stderr| normalize_for_compare(stderr).contains("timed out"))
            .unwrap_or(false)
}

pub(super) fn direct_tool_status(tool_name: &str, direct_payload: &Value) -> &'static str {
    if direct_payload.get("status").and_then(Value::as_str) == Some("blocked") {
        return "blocked";
    }
    if payload_is_transport_failure(direct_payload) {
        return "failed";
    }
    if tool_name == "batch_query"
        && direct_payload
            .get("status")
            .and_then(Value::as_str)
            .map(|raw| raw == "ok" || raw == "success" || raw == "done")
            .unwrap_or(false)
    {
        return "ok";
    }
    if direct_payload
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        "ok"
    } else {
        "failed"
    }
}

pub(super) fn direct_tool_payload_diagnostics(payload: &Value) -> Value {
    json!({
        "top_keys": payload
            .as_object()
            .map(|obj| obj.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default(),
        "status": payload.get("status").and_then(Value::as_str),
        "ok": payload.get("ok").and_then(Value::as_bool),
        "error": payload.get("error").and_then(Value::as_str),
        "transport_error": payload.get("transport_error").and_then(Value::as_str),
        "stderr": payload.get("stderr").and_then(Value::as_str).map(|raw| clean_text(raw, 500)),
    })
}

pub(super) fn is_local_dashboard_url(base_url: &str) -> bool {
    let lowered = base_url.trim().to_ascii_lowercase();
    lowered.starts_with("http://127.0.0.1")
        || lowered.starts_with("http://localhost")
        || lowered.starts_with("http://[::1]")
}
