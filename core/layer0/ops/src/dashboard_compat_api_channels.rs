// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use base64::Engine;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use super::CompatApiResponse;

const CHANNEL_REGISTRY_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/channel_registry.json";
const CHANNEL_QR_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/channel_qr_sessions.json";

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

fn state_path(root: &Path, rel: &str) -> PathBuf {
    root.join(rel)
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
        let k = clean_text(key, 80).to_lowercase();
        if k.is_empty() {
            continue;
        }
        let text = clean_text(value.as_str().unwrap_or(""), 2000);
        if text.is_empty() {
            continue;
        }
        if k.contains("token") || k.contains("secret") || k.contains("key") {
            has_token = true;
        }
        saved.insert(k, Value::String(text));
    }
    channel["configured"] = Value::Bool(!saved.is_empty());
    channel["has_token"] = Value::Bool(has_token);
    channel["config"] = Value::Object(saved.clone());
    channel["live_probe"] = json!({
        "status": "unknown",
        "checked_at": Value::Null,
        "message": "Run live test to verify connectivity."
    });
    channel["connected"] = Value::Bool(false);
    if let Some(fields_rows) = channel.get_mut("fields").and_then(Value::as_array_mut) {
        for row in fields_rows.iter_mut() {
            let key =
                clean_text(row.get("key").and_then(Value::as_str).unwrap_or(""), 80).to_lowercase();
            if key.is_empty() {
                continue;
            }
            if let Some(value) = saved.get(&key).and_then(Value::as_str) {
                let is_secret = row
                    .get("type")
                    .and_then(Value::as_str)
                    .map(|v| v == "secret")
                    .unwrap_or(false);
                row["value"] = Value::String(if is_secret {
                    "••••••".to_string()
                } else {
                    value.to_string()
                });
            }
        }
    }
}

fn configure_channel(root: &Path, name: &str, body: &Value) -> CompatApiResponse {
    let mut state = load_channel_registry(root);
    let channel = {
        let channels = as_object_mut(&mut state, "channels");
        if !channels.contains_key(name) {
            return CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "channel_not_found"}),
            };
        }
        let fields = body
            .get("fields")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        if let Some(channel) = channels.get_mut(name) {
            apply_channel_config(channel, &fields);
        }
        channels.get(name).cloned().unwrap_or_else(|| json!({}))
    };
    save_channel_registry(root, state);
    CompatApiResponse {
        status: 200,
        payload: json!({"ok": true, "status": "ok", "channel": channel}),
    }
}

fn remove_channel_config(root: &Path, name: &str) -> CompatApiResponse {
    let mut state = load_channel_registry(root);
    let channels = as_object_mut(&mut state, "channels");
    if let Some(channel) = channels.get_mut(name) {
        channel["configured"] = Value::Bool(false);
        channel["has_token"] = Value::Bool(false);
        channel["config"] = Value::Object(Map::new());
        channel["live_probe"] = json!({
            "status": "unknown",
            "checked_at": Value::Null,
            "message": "Channel is not configured."
        });
        channel["connected"] = Value::Bool(false);
    }
    save_channel_registry(root, state);
    CompatApiResponse {
        status: 200,
        payload: json!({"ok": true, "status": "ok"}),
    }
}

fn run_http_probe(
    method: &str,
    url: &str,
    headers: &[String],
    body: Option<Value>,
    adapter: &str,
) -> CompatApiResponse {
    let body_ref = body.as_ref();
    match curl_json_request(method, url, headers, body_ref, 20) {
        Ok((status, response)) if (200..400).contains(&status) => ok_response(
            "Live probe succeeded.",
            json!({
                "adapter": adapter,
                "method": method,
                "url": url,
                "http_status": status,
                "response": response
            }),
        ),
        Ok((status, response)) => {
            let err = error_text_from_value(&response);
            error_response(&if err.is_empty() {
                format!("Live probe failed with HTTP {status}.")
            } else {
                format!("Live probe failed with HTTP {status}: {err}")
            })
        }
        Err(err) => error_response(&format!(
            "Live probe request failed: {}",
            clean_text(&err, 280)
        )),
    }
}

fn run_live_probe_whatsapp(root: &Path) -> CompatApiResponse {
    let qr = load_qr_state(root);
    let sessions = qr
        .get("sessions")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let latest = sessions
        .values()
        .max_by_key(|row| parse_non_negative_i64(row.get("created_at_ms"), 0))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let connected = latest
        .get("connected")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if connected {
        ok_response(
            "WhatsApp QR session is connected.",
            json!({"adapter":"whatsapp_qr", "connected": true}),
        )
    } else {
        error_response("WhatsApp is not connected yet. Start QR pairing and scan from mobile.")
    }
}

fn run_live_probe_gohighlevel(channel: &Value) -> CompatApiResponse {
    let pit = config_text(
        channel,
        &[
            "private_integration_token",
            "pit",
            "token",
            "access_token",
            "api_key",
        ],
        500,
    );
    if pit.is_empty() {
        return error_response("Missing Private Integration Token (PIT).");
    }
    let location_id = config_text(
        channel,
        &[
            "location_id",
            "locationid",
            "sub_account_id",
            "subaccount_id",
        ],
        160,
    );
    if location_id.is_empty() {
        return error_response(
            "Missing location_id. Add a HighLevel location (sub-account) ID to run live verification.",
        );
    }
    let endpoint = {
        let configured_endpoint = config_text(channel, &["endpoint", "base_url", "api_url"], 400);
        let fallback = "https://services.leadconnectorhq.com".to_string();
        let raw = if configured_endpoint.is_empty() {
            fallback
        } else {
            configured_endpoint
        };
        normalize_url(&raw)
    };
    let api_version = {
        let configured_version = config_text(channel, &["api_version", "version"], 40);
        if configured_version.is_empty() {
            "2021-07-28".to_string()
        } else {
            configured_version
        }
    };
    let location_id_encoded = urlencoding::encode(&location_id).to_string();
    let url = format!("{endpoint}/locations/{location_id_encoded}");
    let headers = vec![
        "Accept: application/json".to_string(),
        format!("Authorization: Bearer {pit}"),
        format!("Version: {api_version}"),
    ];
    let outcome = run_http_probe("GET", &url, &headers, None, "gohighlevel_api");
    if outcome.payload.get("status").and_then(Value::as_str) == Some("ok") {
        return ok_response(
            "GoHighLevel connection verified.",
            json!({
                "adapter":"gohighlevel_api",
                "endpoint": endpoint,
                "location_id": location_id
            }),
        );
    }
    outcome
}

fn run_live_probe_slack(channel: &Value) -> CompatApiResponse {
    let token = channel_token(channel);
    if token.is_empty() {
        return error_response("Missing Slack token.");
    }
    let base = channel_endpoint(channel);
    let endpoint = if base.is_empty() {
        "https://slack.com/api".to_string()
    } else {
        normalize_url(&base)
    };
    let url = format!("{endpoint}/auth.test");
    let headers = vec![
        "Accept: application/json".to_string(),
        format!("Authorization: Bearer {token}"),
    ];
    match curl_json_request("GET", &url, &headers, None, 20) {
        Ok((status, body))
            if (200..300).contains(&status)
                && body.get("ok").and_then(Value::as_bool).unwrap_or(false) =>
        {
            ok_response(
                "Slack token verified.",
                json!({"adapter":"slack_bot", "http_status": status, "team": body.get("team").cloned().unwrap_or(Value::Null)}),
            )
        }
        Ok((status, body)) => {
            let err = error_text_from_value(&body);
            error_response(&if err.is_empty() {
                format!("Slack auth.test failed with HTTP {status}.")
            } else {
                format!("Slack auth.test failed: {err}")
            })
        }
        Err(err) => error_response(&format!(
            "Slack connectivity probe failed: {}",
            clean_text(&err, 280)
        )),
    }
}

fn run_live_probe_discord(channel: &Value) -> CompatApiResponse {
    let token = channel_token(channel);
    if token.is_empty() {
        return error_response("Missing Discord bot token.");
    }
    let base = channel_endpoint(channel);
    let endpoint = if base.is_empty() {
        "https://discord.com/api/v10".to_string()
    } else {
        normalize_url(&base)
    };
    let url = format!("{endpoint}/users/@me");
    let headers = vec![
        "Accept: application/json".to_string(),
        format!("Authorization: Bot {token}"),
    ];
    match curl_json_request("GET", &url, &headers, None, 20) {
        Ok((status, body)) if (200..300).contains(&status) => ok_response(
            "Discord bot token verified.",
            json!({
                "adapter":"discord_bot",
                "http_status": status,
                "bot_id": body.get("id").cloned().unwrap_or(Value::Null),
                "username": body.get("username").cloned().unwrap_or(Value::Null)
            }),
        ),
        Ok((status, body)) => {
            let err = error_text_from_value(&body);
            error_response(&if err.is_empty() {
                format!("Discord probe failed with HTTP {status}.")
            } else {
                format!("Discord probe failed: {err}")
            })
        }
        Err(err) => error_response(&format!(
            "Discord connectivity probe failed: {}",
            clean_text(&err, 280)
        )),
    }
}

fn run_live_probe_telegram(channel: &Value) -> CompatApiResponse {
    let token = channel_token(channel);
    if token.is_empty() {
        return error_response("Missing Telegram bot token.");
    }
    let url = format!("https://api.telegram.org/bot{token}/getMe");
    let headers = vec!["Accept: application/json".to_string()];
    match curl_json_request("GET", &url, &headers, None, 20) {
        Ok((status, body))
            if (200..300).contains(&status)
                && body.get("ok").and_then(Value::as_bool).unwrap_or(false) =>
        {
            ok_response(
                "Telegram bot token verified.",
                json!({"adapter":"telegram_bot", "http_status": status, "result": body.get("result").cloned().unwrap_or(Value::Null)}),
            )
        }
        Ok((status, body)) => {
            let err = error_text_from_value(&body);
            error_response(&if err.is_empty() {
                format!("Telegram getMe failed with HTTP {status}.")
            } else {
                format!("Telegram getMe failed: {err}")
            })
        }
        Err(err) => error_response(&format!(
            "Telegram connectivity probe failed: {}",
            clean_text(&err, 280)
        )),
    }
}

fn run_live_probe_generic(name: &str, channel: &Value) -> CompatApiResponse {
    let endpoint = channel_endpoint(channel);
    if endpoint.is_empty() {
        return error_response(
            "Missing endpoint/webhook_url for live probe. Add endpoint in channel config.",
        );
    }
    let url = normalize_url(&endpoint);
    let method = {
        let raw = channel_probe_method(channel);
        if raw == "post" || raw == "put" || raw == "patch" {
            raw.to_uppercase()
        } else {
            "GET".to_string()
        }
    };
    let token = channel_token(channel);
    let mut headers = vec![
        "Accept: application/json".to_string(),
        "X-Infring-Probe: live".to_string(),
    ];
    if !token.is_empty() {
        headers.push(format!("Authorization: Bearer {token}"));
    }
    let body = if method == "GET" {
        None
    } else {
        Some(json!({
            "type": "channel_live_probe",
            "channel": name,
            "source": "infring",
            "timestamp": crate::now_iso()
        }))
    };
    run_http_probe(&method, &url, &headers, body, "generic_http")
}

fn run_live_probe(root: &Path, name: &str, channel: &Value) -> CompatApiResponse {
    match channel_adapter(channel).as_str() {
        "whatsapp_qr" => run_live_probe_whatsapp(root),
        "gohighlevel_api" => run_live_probe_gohighlevel(channel),
        "slack_bot" => run_live_probe_slack(channel),
        "discord_bot" => run_live_probe_discord(channel),
        "telegram_bot" => run_live_probe_telegram(channel),
        "webchat_internal" => ok_response(
            "Web chat channel is always available.",
            json!({"adapter":"webchat_internal", "local": true}),
        ),
        _ => run_live_probe_generic(name, channel),
    }
}

fn test_channel(root: &Path, name: &str, body: &Value) -> CompatApiResponse {
    let mut state = load_channel_registry(root);
    let channel = state
        .get("channels")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get(name))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let configured = channel
        .get("configured")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let has_token = channel
        .get("has_token")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let requires_token = channel_flag(&channel, "requires_token", true);
    if !configured {
        return error_response("Channel is not configured yet.");
    }
    if requires_token && !has_token {
        return error_response("Missing token/secret in channel config.");
    }

    let force_live = body
        .get("force_live")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !force_live {
        return ok_response(
            "Configuration is valid. Run live test to verify real connectivity.",
            json!({
                "adapter": channel_adapter(&channel),
                "live_probe_required": true,
                "live_probe_hint": "Send {\"force_live\":true} to /api/channels/<name>/test",
                "last_live_probe": channel.get("live_probe").cloned().unwrap_or(Value::Null)
            }),
        );
    }
    let response = run_live_probe(root, name, &channel);
    let probe_status = response
        .payload
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("error");
    let message = clean_text(
        response
            .payload
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("Live probe completed."),
        280,
    );
    let connected = probe_status == "ok";
    let checked_at = crate::now_iso();

    if let Some(entry) = as_object_mut(&mut state, "channels").get_mut(name) {
        entry["live_probe"] = json!({
            "status": if connected { "ok" } else { "error" },
            "checked_at": checked_at,
            "message": message,
            "details": response.payload.get("details").cloned().unwrap_or(Value::Null)
        });
        entry["connected"] = Value::Bool(connected);
    }
    save_channel_registry(root, state);
    response
}

fn start_whatsapp_qr(root: &Path) -> CompatApiResponse {
    let session_id = format!("wa-{}", now_ms());
    let qr_svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='256' height='256'><rect width='256' height='256' fill='white'/><rect x='12' y='12' width='232' height='232' fill='black'/><rect x='24' y='24' width='208' height='208' fill='white'/><text x='128' y='126' font-size='14' text-anchor='middle' fill='black'>WhatsApp QR</text><text x='128' y='146' font-size='10' text-anchor='middle' fill='black'>{}</text></svg>",
        session_id
    );
    let encoded = base64::engine::general_purpose::STANDARD.encode(qr_svg.as_bytes());
    let mut qr = load_qr_state(root);
    let sessions = as_object_mut(&mut qr, "sessions");
    sessions.insert(
        session_id.clone(),
        json!({
            "session_id": session_id,
            "created_at_ms": now_ms(),
            "connected": false,
            "expired": false,
            "message": "Scan with WhatsApp mobile app to connect."
        }),
    );
    save_qr_state(root, qr);
    CompatApiResponse {
        status: 200,
        payload: json!({
            "ok": true,
            "available": true,
            "session_id": session_id,
            "qr_data_url": format!("data:image/svg+xml;base64,{}", encoded),
            "connected": false,
            "message": "Scan the QR code with WhatsApp.",
            "help": "Open WhatsApp -> Linked devices -> Link a device"
        }),
    }
}

fn whatsapp_qr_status(root: &Path) -> CompatApiResponse {
    let mut qr = load_qr_state(root);
    let sessions = as_object_mut(&mut qr, "sessions");
    let maybe_latest = sessions
        .iter_mut()
        .max_by_key(|(_, row)| parse_non_negative_i64(row.get("created_at_ms"), 0));
    let Some((_, row)) = maybe_latest else {
        return CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "connected": false, "expired": true, "message": "No active QR session."}),
        };
    };
    let age_ms = now_ms() - parse_non_negative_i64(row.get("created_at_ms"), now_ms());
    if age_ms > 5 * 60 * 1000 {
        row["expired"] = Value::Bool(true);
    }
    let connected = row
        .get("connected")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let expired = row.get("expired").and_then(Value::as_bool).unwrap_or(false);
    save_qr_state(root, qr);
    CompatApiResponse {
        status: 200,
        payload: json!({
            "ok": true,
            "connected": connected,
            "expired": expired,
            "message": if connected { "Connected." } else if expired { "QR code expired." } else { "Waiting for scan..." }
        }),
    }
}

pub fn channels_payload(root: &Path) -> Value {
    let state = load_channel_registry(root);
    save_channel_registry(root, state.clone());
    json!({"ok": true, "channels": channel_rows(&state)})
}

pub fn handle(
    root: &Path,
    method: &str,
    path_only: &str,
    body: &[u8],
) -> Option<CompatApiResponse> {
    if method == "GET" {
        return match path_only {
            "/api/channels" => Some(CompatApiResponse {
                status: 200,
                payload: channels_payload(root),
            }),
            "/api/channels/whatsapp/qr/status" => Some(whatsapp_qr_status(root)),
            _ => None,
        };
    }

    if method == "POST" {
        if path_only == "/api/channels/whatsapp/qr/start" {
            return Some(start_whatsapp_qr(root));
        }
        if path_only.starts_with("/api/channels/") && path_only.ends_with("/configure") {
            if let Some(name) = channel_name_from_path(path_only) {
                return Some(configure_channel(root, &name, &parse_json(body)));
            }
        }
        if path_only.starts_with("/api/channels/") && path_only.ends_with("/test") {
            if let Some(name) = channel_name_from_path(path_only) {
                return Some(test_channel(root, &name, &parse_json(body)));
            }
        }
    }

    if method == "DELETE"
        && path_only.starts_with("/api/channels/")
        && path_only.ends_with("/configure")
    {
        if let Some(name) = channel_name_from_path(path_only) {
            return Some(remove_channel_config(root, &name));
        }
    }

    None
}
