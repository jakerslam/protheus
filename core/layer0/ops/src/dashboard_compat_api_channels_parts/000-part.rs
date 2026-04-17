// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use base64::Engine;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use super::CompatApiResponse;
use crate::contract_lane_utils as lane_utils;

const CHANNEL_REGISTRY_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/channel_registry.json";
const CHANNEL_QR_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/channel_qr_sessions.json";

fn clean_text(raw: &str, max_len: usize) -> String {
    lane_utils::clean_text(Some(raw), max_len.max(1))
}

fn parse_json(raw: &[u8]) -> Value {
    serde_json::from_slice::<Value>(raw).unwrap_or_else(|_| json!({}))
}

fn state_path(root: &Path, rel: &str) -> PathBuf {
    root.join(rel)
}

fn read_json(path: &Path) -> Option<Value> {
    lane_utils::read_json(path)
}

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(value) {
        let _ = fs::write(path, raw);
    }
}

fn as_object_mut<'a>(value: &'a mut Value, key: &str) -> &'a mut Map<String, Value> {
    if !value.get(key).map(Value::is_object).unwrap_or(false) {
        value[key] = Value::Object(Map::new());
    }
    value
        .get_mut(key)
        .and_then(Value::as_object_mut)
        .expect("object must exist")
}

fn parse_non_negative_i64(value: Option<&Value>, fallback: i64) -> i64 {
    value.and_then(Value::as_i64).unwrap_or(fallback).max(0)
}

fn error_text_from_value(value: &Value) -> String {
    if let Some(text) = value.get("error").and_then(Value::as_str) {
        return clean_text(text, 280);
    }
    if let Some(text) = value
        .get("error")
        .and_then(Value::as_object)
        .and_then(|row| row.get("message"))
        .and_then(Value::as_str)
    {
        return clean_text(text, 280);
    }
    if let Some(text) = value.get("message").and_then(Value::as_str) {
        return clean_text(text, 280);
    }
    clean_text(&value.to_string(), 280)
}

fn config_text(channel: &Value, keys: &[&str], max_len: usize) -> String {
    let Some(config) = channel.get("config").and_then(Value::as_object) else {
        return String::new();
    };
    for key in keys {
        let value = clean_text(
            config.get(*key).and_then(Value::as_str).unwrap_or(""),
            max_len,
        );
        if !value.is_empty() {
            return value;
        }
    }
    String::new()
}

fn channel_flag(channel: &Value, key: &str, fallback: bool) -> bool {
    channel
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or(fallback)
}

fn channel_adapter(channel: &Value) -> String {
    clean_text(
        channel
            .get("runtime_adapter")
            .and_then(Value::as_str)
            .unwrap_or("generic_http"),
        64,
    )
}

fn channel_probe_method(channel: &Value) -> String {
    clean_text(
        channel
            .get("probe_method")
            .and_then(Value::as_str)
            .unwrap_or("get"),
        12,
    )
    .to_lowercase()
}

fn channel_token(channel: &Value) -> String {
    config_text(
        channel,
        &[
            "bot_token",
            "private_integration_token",
            "access_token",
            "api_key",
            "token",
            "secret",
            "key",
        ],
        600,
    )
}

fn channel_endpoint(channel: &Value) -> String {
    let endpoint = config_text(
        channel,
        &[
            "webhook_url",
            "endpoint",
            "base_url",
            "api_url",
            "url",
            "host",
        ],
        1200,
    );
    if endpoint.eq_ignore_ascii_case("default") {
        String::new()
    } else {
        endpoint
    }
}

fn normalize_url(raw: &str) -> String {
    let mut url = clean_text(raw, 1200);
    while url.ends_with('/') {
        url.pop();
    }
    url
}

fn error_response(message: &str) -> CompatApiResponse {
    CompatApiResponse {
        status: 200,
        payload: json!({"ok": true, "status": "error", "message": clean_text(message, 320)}),
    }
}

fn ok_response(message: &str, details: Value) -> CompatApiResponse {
    CompatApiResponse {
        status: 200,
        payload: json!({
            "ok": true,
            "status": "ok",
            "message": clean_text(message, 320),
            "details": details
        }),
    }
}

fn curl_json_request(
    method: &str,
    url: &str,
    headers: &[String],
    body_json: Option<&Value>,
    timeout_secs: u64,
) -> Result<(u16, Value), String> {
    let mut cmd = Command::new("curl");
    cmd.arg("-sS")
        .arg("-L")
        .arg("-X")
        .arg(clean_text(method, 12))
        .arg("--connect-timeout")
        .arg("8")
        .arg("--max-time")
        .arg(timeout_secs.to_string());
    for header in headers {
        cmd.arg("-H").arg(header);
    }
    if let Some(body) = body_json {
        let body_text = serde_json::to_string(body).unwrap_or_else(|_| "{}".to_string());
        cmd.arg("-H").arg("Content-Type: application/json");
        cmd.arg("--data").arg(body_text);
    }
    cmd.arg("-w").arg("\n__HTTP_STATUS__:%{http_code}").arg(url);
    let output = cmd
        .output()
        .map_err(|err| format!("curl_spawn_failed:{err}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = clean_text(&String::from_utf8_lossy(&output.stderr), 600);
    let marker = "\n__HTTP_STATUS__:";
    let Some(index) = stdout.rfind(marker) else {
        return Err(if stderr.is_empty() {
            "curl_http_status_missing".to_string()
        } else {
            stderr
        });
    };
    let body_raw = stdout[..index].trim();
    let status = stdout[index + marker.len()..]
        .trim()
        .parse::<u16>()
        .unwrap_or(0);
    let value = serde_json::from_str::<Value>(body_raw)
        .unwrap_or_else(|_| json!({"raw": clean_text(body_raw, 8_000)}));
    if !output.status.success() && status == 0 {
        return Err(if stderr.is_empty() {
            "curl_failed".to_string()
        } else {
            stderr
        });
    }
    Ok((status, value))
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn channel_defaults() -> Vec<Value> {
    crate::dashboard_channel_catalog::catalog()
}

fn load_channel_registry(root: &Path) -> Value {
    let path = state_path(root, CHANNEL_REGISTRY_REL);
    let mut state = read_json(&path).unwrap_or_else(|| {
        json!({
            "type": "infring_dashboard_channel_registry",
            "updated_at": crate::now_iso(),
            "channels": {}
        })
    });
    let channels = as_object_mut(&mut state, "channels");
    for row in channel_defaults() {
        let name = clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 80);
        if name.is_empty() {
            continue;
        }
        if let Some(existing) = channels.get_mut(&name) {
            let default_obj = row.as_object().cloned().unwrap_or_default();
            for key in [
                "runtime_adapter",
                "runtime_mode",
                "channel_tier",
                "real_channel",
                "runtime_supported",
                "requires_token",
                "supports_send",
                "probe_method",
                "live_probe_required_for_ready",
                "setup_type",
                "category",
                "display_name",
                "description",
                "quick_setup",
                "difficulty",
                "setup_time",
                "icon",
            ] {
                let should_fill = existing
                    .get(key)
                    .map(|value| value.is_null())
                    .unwrap_or(true);
                if should_fill {
                    if let Some(value) = default_obj.get(key) {
                        existing[key] = value.clone();
                    }
                }
            }
            if !existing.get("fields").map(Value::is_array).unwrap_or(false) {
                if let Some(value) = default_obj.get("fields") {
                    existing["fields"] = value.clone();
                }
            }
            if !existing
                .get("setup_steps")
                .map(Value::is_array)
                .unwrap_or(false)
            {
                if let Some(value) = default_obj.get("setup_steps") {
                    existing["setup_steps"] = value.clone();
                }
            }
            if existing
                .get("config_template")
                .map(|value| value.is_null())
                .unwrap_or(true)
            {
                if let Some(value) = default_obj.get("config_template") {
                    existing["config_template"] = value.clone();
                }
            }
        } else {
            channels.insert(name, row);
        }
    }
    state
}

fn save_channel_registry(root: &Path, mut state: Value) {
    state["updated_at"] = Value::String(crate::now_iso());
    write_json(&state_path(root, CHANNEL_REGISTRY_REL), &state);
}

fn load_qr_state(root: &Path) -> Value {
    read_json(&state_path(root, CHANNEL_QR_REL)).unwrap_or_else(|| {
        json!({
            "type": "infring_dashboard_channel_qr_sessions",
            "updated_at": crate::now_iso(),
            "sessions": {}
        })
    })
}

fn save_qr_state(root: &Path, mut state: Value) {
    state["updated_at"] = Value::String(crate::now_iso());
    write_json(&state_path(root, CHANNEL_QR_REL), &state);
}

fn channel_rows(state: &Value) -> Vec<Value> {
    let mut rows = state
        .get("channels")
        .and_then(Value::as_object)
        .map(|obj| obj.values().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    rows.sort_by(|a, b| {
        clean_text(a.get("name").and_then(Value::as_str).unwrap_or(""), 80).cmp(&clean_text(
            b.get("name").and_then(Value::as_str).unwrap_or(""),
            80,
        ))
    });
    rows.into_iter()
        .map(|mut row| {
            let configured = row
                .get("configured")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let has_token = row
                .get("has_token")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let requires_token = row
                .get("requires_token")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let runtime_supported = row
                .get("runtime_supported")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let probe_required = row
                .get("live_probe_required_for_ready")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let config_ready = if requires_token {
                configured && has_token
            } else {
                configured
            };
            let live_ok = row
                .get("live_probe")
                .and_then(Value::as_object)
                .and_then(|p| p.get("status"))
                .and_then(Value::as_str)
                .map(|status| status == "ok")
                .unwrap_or(false);
            let connected = if probe_required {
                config_ready && live_ok
            } else {
                config_ready
            };
            row["connected"] = Value::Bool(connected && runtime_supported);
            row
        })
        .collect()
}

fn channel_name_from_path(path: &str) -> Option<String> {
    let prefix = "/api/channels/";
    if !path.starts_with(prefix) {
        return None;
    }
    let tail = path.strip_prefix(prefix).unwrap_or_default();
    let name = tail.split('/').next().unwrap_or_default();
    let normalized = clean_text(name, 80).to_lowercase();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn apply_channel_config(channel: &mut Value, fields: &Map<String, Value>) {
    let mut saved = Map::<String, Value>::new();
    let mut has_token = false;
    for (key, value) in fields {
