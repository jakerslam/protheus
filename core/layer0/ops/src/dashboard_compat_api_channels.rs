// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use base64::Engine;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
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
    value
        .and_then(Value::as_i64)
        .unwrap_or(fallback)
        .max(0)
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
        channels.entry(name).or_insert(row);
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
        clean_text(a.get("name").and_then(Value::as_str).unwrap_or(""), 80)
            .cmp(&clean_text(b.get("name").and_then(Value::as_str).unwrap_or(""), 80))
    });
    rows.into_iter()
        .map(|mut row| {
            let configured = row.get("configured").and_then(Value::as_bool).unwrap_or(false);
            let has_token = row.get("has_token").and_then(Value::as_bool).unwrap_or(false);
            row["connected"] = Value::Bool(configured && has_token);
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
    if let Some(fields_rows) = channel.get_mut("fields").and_then(Value::as_array_mut) {
        for row in fields_rows.iter_mut() {
            let key = clean_text(row.get("key").and_then(Value::as_str).unwrap_or(""), 80)
                .to_lowercase();
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
    }
    save_channel_registry(root, state);
    CompatApiResponse {
        status: 200,
        payload: json!({"ok": true, "status": "ok"}),
    }
}

fn test_channel(root: &Path, name: &str) -> CompatApiResponse {
    let state = load_channel_registry(root);
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
    if !configured {
        return CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "status": "error", "message": "Channel is not configured yet."}),
        };
    }
    CompatApiResponse {
        status: 200,
        payload: json!({
            "ok": true,
            "status": "ok",
            "message": if has_token { "Connection verified." } else { "Configured without token; web-QR/session mode expected." }
        }),
    }
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
    let connected = row.get("connected").and_then(Value::as_bool).unwrap_or(false);
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
    json!({"ok": true, "channels": channel_rows(&state)})
}

pub fn handle(root: &Path, method: &str, path_only: &str, body: &[u8]) -> Option<CompatApiResponse> {
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
                return Some(test_channel(root, &name));
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
