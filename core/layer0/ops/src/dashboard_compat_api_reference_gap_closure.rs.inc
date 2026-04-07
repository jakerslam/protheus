// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use super::CompatApiResponse;
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::Sha256;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

const GAP_STATE_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/reference_runtime_gap_closure_state.json";
const AGENT_PROFILES_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/agent_profiles.json";
const NONCE_REPLAY_WINDOW_MS: i64 = 10 * 60 * 1000;
const HANDSHAKE_CLOCK_SKEW_MS: i64 = 5 * 60 * 1000;

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
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

fn parse_json(body: &[u8]) -> Value {
    serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}))
}

fn query_value(path: &str, key: &str) -> Option<String> {
    let query = path.split_once('?').map(|(_, q)| q).unwrap_or("");
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        if clean_text(k, 80).eq_ignore_ascii_case(key) {
            let decoded = urlencoding::decode(v)
                .ok()
                .map(|s| s.to_string())
                .unwrap_or_default();
            let cleaned = clean_text(&decoded, 200);
            if !cleaned.is_empty() {
                return Some(cleaned);
            }
        }
    }
    None
}

fn state_path(root: &Path) -> PathBuf {
    root.join(GAP_STATE_REL)
}

fn load_state(root: &Path) -> Value {
    read_json(&state_path(root)).unwrap_or_else(|| {
        json!({
            "type": "reference_runtime_gap_closure_state",
            "updated_at": crate::now_iso(),
            "a2a": {
                "agents": [],
                "tasks": [],
                "messages": []
            },
            "network": {
                "shared_secret": "",
                "nonce_log": [],
                "peers": []
            },
            "pairing": {
                "requests": [],
                "devices": [],
                "notifications": []
            },
            "manifest_signing": {
                "algorithm": "Ed25519",
                "secret_seed_b64": "",
                "public_key_b64": ""
            }
        })
    })
}

fn save_state(root: &Path, mut state: Value) {
    state["updated_at"] = Value::String(crate::now_iso());
    write_json(&state_path(root), &state);
}

fn as_array_mut<'a>(value: &'a mut Value, key: &str) -> &'a mut Vec<Value> {
    if !value.get(key).map(Value::is_array).unwrap_or(false) {
        value[key] = Value::Array(Vec::new());
    }
    value
        .get_mut(key)
        .and_then(Value::as_array_mut)
        .expect("array must exist")
}

fn parse_profiles(root: &Path) -> Vec<Value> {
    read_json(&root.join(AGENT_PROFILES_REL))
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
}

fn local_a2a_agents(root: &Path) -> Vec<Value> {
    parse_profiles(root)
        .into_iter()
        .map(|row| {
            let id = clean_text(
                row.get("agent_id")
                    .or_else(|| row.get("id"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120,
            );
            let name = clean_text(
                row.get("name").and_then(Value::as_str).unwrap_or("agent"),
                120,
            );
            let role = clean_text(
                row.get("role").and_then(Value::as_str).unwrap_or("analyst"),
                80,
            );
            let state = clean_text(
                row.get("state").and_then(Value::as_str).unwrap_or("Running"),
                40,
            );
            json!({
                "agent_id": id,
                "name": name,
                "role": role,
                "state": state,
                "transport": "local_dashboard",
                "origin": "local"
            })
        })
        .filter(|row| {
            !clean_text(
                row.get("agent_id").and_then(Value::as_str).unwrap_or(""),
                120,
            )
            .is_empty()
        })
        .collect::<Vec<_>>()
}

fn merge_discovered_agents(root: &Path, state: &mut Value, extra_agents: &[Value]) -> Vec<Value> {
    let mut merged = local_a2a_agents(root);
    let known = state
        .pointer("/a2a/agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for row in known.iter().chain(extra_agents.iter()) {
        let id = clean_text(
            row.get("agent_id")
                .or_else(|| row.get("id"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        if id.is_empty() {
            continue;
        }
        if merged
            .iter()
            .any(|existing| existing.get("agent_id").and_then(Value::as_str) == Some(id.as_str()))
        {
            continue;
        }
        merged.push(json!({
            "agent_id": id,
            "name": clean_text(row.get("name").and_then(Value::as_str).unwrap_or("remote_agent"), 120),
            "role": clean_text(row.get("role").and_then(Value::as_str).unwrap_or("remote"), 80),
            "state": clean_text(row.get("state").and_then(Value::as_str).unwrap_or("available"), 40),
            "transport": clean_text(row.get("transport").and_then(Value::as_str).unwrap_or("a2a"), 60),
            "origin": clean_text(row.get("origin").and_then(Value::as_str).unwrap_or("remote"), 80),
            "last_seen_ms": now_ms()
        }));
    }
    state["a2a"]["agents"] = Value::Array(merged.clone());
    merged
}

fn lookup_task(state: &Value, task_id: &str) -> Option<Value> {
    state
        .pointer("/a2a/tasks")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter()
                .find(|row| row.get("task_id").and_then(Value::as_str) == Some(task_id))
                .cloned()
        })
}

fn parse_model(model: &str, request: &Value) -> (String, String, String) {
    let model = clean_text(model, 240);
    let requested_provider = clean_text(
        request.get("provider").and_then(Value::as_str).unwrap_or("auto"),
        80,
    );
    if let Some((provider, short_model)) = model.split_once('/') {
        let provider = clean_text(provider, 80);
        let short_model = clean_text(short_model, 240);
        let combined = if short_model.is_empty() {
            model.clone()
        } else {
            format!("{provider}/{short_model}")
        };
        return (provider, short_model, combined);
    }
    (
        if requested_provider.is_empty() {
            "auto".to_string()
        } else {
            requested_provider
        },
        model.clone(),
        model,
    )
}

fn extract_message_content(message: &Value) -> String {
    if let Some(text) = message.get("content").and_then(Value::as_str) {
        return clean_text(text, 16_000);
    }
    if let Some(parts) = message.get("content").and_then(Value::as_array) {
        let mut out = Vec::<String>::new();
        for part in parts {
            let part_type = clean_text(part.get("type").and_then(Value::as_str).unwrap_or(""), 40);
            if part_type == "text" || part_type.is_empty() {
                let text = clean_text(part.get("text").and_then(Value::as_str).unwrap_or(""), 2000);
                if !text.is_empty() {
                    out.push(text);
                }
            }
        }
        return clean_text(&out.join(" "), 16_000);
    }
    String::new()
}

fn a2a_agents_payload(root: &Path, state: &mut Value) -> CompatApiResponse {
    let merged = merge_discovered_agents(root, state, &[]);
    save_state(root, state.clone());
    CompatApiResponse {
        status: 200,
        payload: json!({
            "ok": true,
            "type": "a2a_agents",
            "agents": merged,
            "count": merged.len()
        }),
    }
}

fn a2a_discover_payload(root: &Path, path: &str, body: &[u8], state: &mut Value) -> CompatApiResponse {
    let request = parse_json(body);
    let extra_agents = request
        .get("agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let merged = merge_discovered_agents(root, state, &extra_agents);
    let query_from_path = query_value(path, "query").or_else(|| query_value(path, "q"));
    let query = clean_text(
        request
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or_else(|| query_from_path.as_deref().unwrap_or("")),
        120,
    )
    .to_ascii_lowercase();
    let filtered = if query.is_empty() {
        merged
    } else {
        merged
            .into_iter()
            .filter(|row| {
                let hay = format!(
                    "{} {} {}",
                    clean_text(row.get("agent_id").and_then(Value::as_str).unwrap_or(""), 120),
                    clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 120),
                    clean_text(row.get("role").and_then(Value::as_str).unwrap_or(""), 120)
                )
                .to_ascii_lowercase();
                hay.contains(&query)
            })
            .collect::<Vec<_>>()
    };
    save_state(root, state.clone());
    CompatApiResponse {
        status: 200,
        payload: json!({
            "ok": true,
            "type": "a2a_discover",
            "query": query,
            "agents": filtered,
            "count": filtered.len()
        }),
    }
}

fn a2a_send_payload(root: &Path, body: &[u8], state: &mut Value) -> CompatApiResponse {
    let request = parse_json(body);
    let to_agent_id = clean_text(
        request
            .get("to_agent_id")
            .or_else(|| request.get("agent_id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    let message = clean_text(
        request
            .get("message")
            .or_else(|| request.get("content"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        16_000,
    );
    if to_agent_id.is_empty() || message.is_empty() {
        return CompatApiResponse {
            status: 400,
            payload: json!({
                "ok": false,
                "error": "a2a_send_requires_to_agent_id_and_message"
            }),
        };
    }
    let from_agent_id = clean_text(
        request
            .get("from_agent_id")
            .and_then(Value::as_str)
            .unwrap_or("system"),
        120,
    );
    let task_id = format!(
        "a2atask_{}",
        crate::deterministic_receipt_hash(&json!({
            "from": from_agent_id,
            "to": to_agent_id,
            "message": message,
            "ts_ms": now_ms()
        }))
        .chars()
        .take(16)
        .collect::<String>()
    );
    let delivery = json!({
        "task_id": task_id,
        "from_agent_id": from_agent_id,
        "to_agent_id": to_agent_id,
        "message": message,
        "status": "completed",
        "created_at": crate::now_iso(),
        "updated_at": crate::now_iso(),
        "transport": "a2a_http",
        "result": "delivered"
    });
    as_array_mut(&mut state["a2a"], "tasks").push(delivery.clone());
    as_array_mut(&mut state["a2a"], "messages").push(json!({
        "task_id": task_id,
        "from_agent_id": delivery.get("from_agent_id").cloned().unwrap_or(Value::Null),
        "to_agent_id": delivery.get("to_agent_id").cloned().unwrap_or(Value::Null),
        "message": delivery.get("message").cloned().unwrap_or(Value::Null),
        "ts": crate::now_iso()
    }));
    save_state(root, state.clone());
    CompatApiResponse {
        status: 200,
        payload: json!({
            "ok": true,
            "type": "a2a_send",
            "task": delivery
        }),
    }
}

fn a2a_task_status_payload(path: &str, state: &Value) -> CompatApiResponse {
    let task_id = clean_text(
        query_value(path, "task_id")
            .or_else(|| query_value(path, "id"))
            .as_deref()
            .unwrap_or(""),
        120,
    );
    if task_id.is_empty() {
        return CompatApiResponse {
            status: 400,
            payload: json!({"ok": false, "error": "task_id_required"}),
        };
    }
    if let Some(task) = lookup_task(state, &task_id) {
        return CompatApiResponse {
            status: 200,
            payload: json!({
                "ok": true,
                "type": "a2a_task_status",
                "task": task
            }),
        };
    }
    CompatApiResponse {
        status: 404,
        payload: json!({"ok": false, "error": "task_not_found", "task_id": task_id}),
    }
}

fn purge_old_nonces(state: &mut Value, now: i64) {
    let keep_after = now.saturating_sub(NONCE_REPLAY_WINDOW_MS);
    let rows = state
        .pointer("/network/nonce_log")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let filtered = rows
        .into_iter()
        .filter(|row| {
            row.get("ts_ms")
                .and_then(Value::as_i64)
                .unwrap_or_default()
                >= keep_after
        })
        .collect::<Vec<_>>();
    state["network"]["nonce_log"] = Value::Array(filtered);
}

fn network_shared_secret(state: &Value) -> String {
    let configured = clean_text(
        state
            .pointer("/network/shared_secret")
            .and_then(Value::as_str)
            .unwrap_or(""),
        240,
    );
    if !configured.is_empty() {
        return configured;
    }
    std::env::var("INFRING_PEER_SHARED_SECRET")
        .ok()
        .map(|raw| clean_text(&raw, 240))
        .filter(|raw| !raw.is_empty())
        .unwrap_or_else(|| "infring-reference_runtime-peer-shared-secret".to_string())
}

fn hmac_signature(secret: &str, payload: &str) -> Result<String, String> {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).map_err(|err| format!("hmac_init:{err}"))?;
    mac.update(payload.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

fn peer_handshake_payload(root: &Path, body: &[u8], state: &mut Value) -> CompatApiResponse {
    let request = parse_json(body);
    let peer_id = clean_text(request.get("peer_id").and_then(Value::as_str).unwrap_or(""), 120);
    let nonce = clean_text(request.get("nonce").and_then(Value::as_str).unwrap_or(""), 240);
    let signature = clean_text(
        request.get("signature").and_then(Value::as_str).unwrap_or(""),
        240,
    )
    .to_ascii_lowercase();
    let ts_ms = request
        .get("ts_ms")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    let address = clean_text(
        request
            .get("address")
            .or_else(|| request.get("host"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        240,
    );
    let capabilities = request
        .get("capabilities")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if peer_id.is_empty() || nonce.is_empty() || signature.is_empty() || ts_ms <= 0 {
        return CompatApiResponse {
            status: 400,
            payload: json!({
                "ok": false,
                "error": "peer_handshake_requires_peer_id_nonce_signature_ts_ms"
            }),
        };
    }
    let now = now_ms();
    if (now - ts_ms).abs() > HANDSHAKE_CLOCK_SKEW_MS {
        return CompatApiResponse {
            status: 401,
            payload: json!({
                "ok": false,
                "error": "peer_handshake_clock_skew",
                "allowed_skew_ms": HANDSHAKE_CLOCK_SKEW_MS
            }),
        };
    }
    purge_old_nonces(state, now);
    let replayed = state
        .pointer("/network/nonce_log")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter().any(|row| {
                row.get("nonce").and_then(Value::as_str) == Some(nonce.as_str())
                    && row.get("peer_id").and_then(Value::as_str) == Some(peer_id.as_str())
            })
        })
        .unwrap_or(false);
    if replayed {
        return CompatApiResponse {
            status: 409,
            payload: json!({
                "ok": false,
                "error": "peer_handshake_replay_denied",
                "peer_id": peer_id
            }),
        };
    }
    let secret = network_shared_secret(state);
    let payload = format!("{peer_id}|{nonce}|{ts_ms}");
    let expected = match hmac_signature(&secret, &payload) {
        Ok(value) => value,
        Err(err) => {
            return CompatApiResponse {
                status: 500,
                payload: json!({"ok": false, "error": err}),
            };
        }
    };
    if signature != expected {
        return CompatApiResponse {
            status: 401,
            payload: json!({
                "ok": false,
                "error": "peer_handshake_signature_invalid",
                "peer_id": peer_id
            }),
        };
    }
    as_array_mut(&mut state["network"], "nonce_log").push(json!({
        "peer_id": peer_id,
        "nonce": nonce,
        "ts_ms": ts_ms
    }));
    let peers = as_array_mut(&mut state["network"], "peers");
    let index = peers
        .iter()
        .position(|row| row.get("peer_id").and_then(Value::as_str) == Some(peer_id.as_str()));
    let peer_row = json!({
        "peer_id": peer_id,
        "address": address,
        "connected": true,
        "auth": "hmac_sha256",
        "last_handshake_ms": now,
        "session_token": format!(
            "peer_{}",
            crate::deterministic_receipt_hash(&json!({"peer_id": peer_id, "nonce": nonce, "ts_ms": ts_ms}))
                .chars()
                .take(18)
                .collect::<String>()
        ),
        "capabilities": capabilities
    });
    if let Some(existing_idx) = index {
        peers[existing_idx] = peer_row.clone();
    } else {
        peers.push(peer_row.clone());
    }
    save_state(root, state.clone());
    CompatApiResponse {
        status: 200,
        payload: json!({
            "ok": true,
            "type": "peer_handshake",
            "peer": peer_row
        }),
    }
}

fn peers_payload(state: &Value) -> Value {
    let peers = state
        .pointer("/network/peers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let connected = peers
        .iter()
        .filter(|row| row.get("connected").and_then(Value::as_bool).unwrap_or(false))
        .count();
    json!({
        "ok": true,
        "peers": peers,
        "connected": connected,
        "total": peers.len()
    })
}

fn network_status_payload(state: &Value, snapshot: &Value) -> Value {
    let peers = state
        .pointer("/network/peers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let connected = peers
        .iter()
        .filter(|row| row.get("connected").and_then(Value::as_bool).unwrap_or(false))
        .count();
    json!({
        "ok": true,
        "enabled": true,
        "connected_peers": connected,
        "total_peers": peers.len(),
        "runtime_sync": snapshot.pointer("/runtime_sync/summary").cloned().unwrap_or_else(|| json!({})),
        "auth": {
            "handshake": "hmac_sha256",
            "nonce_replay_defense": true
        }
    })
}

fn pairing_request_payload(root: &Path, body: &[u8], state: &mut Value) -> CompatApiResponse {
    let request = parse_json(body);
    let device_name = clean_text(
        request
            .get("device_name")
            .or_else(|| request.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("device"),
        120,
    );
    let platform = clean_text(
        request
            .get("platform")
            .or_else(|| request.get("device_type"))
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        80,
    );
    let pairing_id = format!(
        "pair_{}",
        crate::deterministic_receipt_hash(&json!({
            "device_name": device_name,
            "platform": platform,
            "ts_ms": now_ms()
        }))
        .chars()
        .take(14)
        .collect::<String>()
    );
    let code = crate::deterministic_receipt_hash(&json!({"pairing_id": pairing_id, "ts_ms": now_ms()}))
        .chars()
        .take(6)
        .collect::<String>()
        .to_ascii_uppercase();
    let row = json!({
        "pairing_id": pairing_id,
        "device_name": device_name,
        "platform": platform,
        "status": "requested",
        "code": code,
        "created_at_ms": now_ms(),
        "updated_at_ms": now_ms()
    });
    as_array_mut(&mut state["pairing"], "requests").push(row.clone());
    save_state(root, state.clone());
    CompatApiResponse {
        status: 200,
        payload: json!({"ok": true, "type": "pairing_request", "pairing": row}),
    }
}

fn pairing_complete_payload(root: &Path, body: &[u8], state: &mut Value) -> CompatApiResponse {
    let request = parse_json(body);
    let pairing_id = clean_text(
        request
            .get("pairing_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    if pairing_id.is_empty() {
        return CompatApiResponse {
            status: 400,
            payload: json!({"ok": false, "error": "pairing_id_required"}),
        };
    }
    let requests = as_array_mut(&mut state["pairing"], "requests");
    let index = requests
        .iter()
        .position(|row| row.get("pairing_id").and_then(Value::as_str) == Some(pairing_id.as_str()));
    let Some(request_index) = index else {
        return CompatApiResponse {
            status: 404,
            payload: json!({"ok": false, "error": "pairing_not_found", "pairing_id": pairing_id}),
        };
    };
    let mut request_row = requests[request_index].clone();
    request_row["status"] = Value::String("completed".to_string());
    request_row["updated_at_ms"] = json!(now_ms());
    requests[request_index] = request_row.clone();
    let device_id = format!(
        "dev_{}",
        crate::deterministic_receipt_hash(&json!({
            "pairing_id": pairing_id,
            "device_name": request_row.get("device_name").cloned().unwrap_or(Value::Null)
        }))
        .chars()
        .take(16)
        .collect::<String>()
    );
    let device = json!({
        "device_id": device_id,
        "pairing_id": pairing_id,
        "name": request_row.get("device_name").cloned().unwrap_or_else(|| json!("device")),
        "platform": request_row.get("platform").cloned().unwrap_or_else(|| json!("unknown")),
        "status": "paired",
        "paired_at_ms": now_ms(),
        "last_notified_ms": 0
    });
    as_array_mut(&mut state["pairing"], "devices").push(device.clone());
    save_state(root, state.clone());
    CompatApiResponse {
        status: 200,
        payload: json!({"ok": true, "type": "pairing_complete", "device": device, "pairing": request_row}),
    }
}

fn pairing_devices_payload(state: &Value) -> CompatApiResponse {
    let devices = state
        .pointer("/pairing/devices")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    CompatApiResponse {
        status: 200,
        payload: json!({"ok": true, "devices": devices, "count": devices.len()}),
    }
}

fn pairing_notify_payload(root: &Path, body: &[u8], state: &mut Value) -> CompatApiResponse {
    let request = parse_json(body);
    let device_id = clean_text(
        request
            .get("device_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    let message = clean_text(
        request
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or(""),
        2000,
    );
    if device_id.is_empty() || message.is_empty() {
        return CompatApiResponse {
            status: 400,
            payload: json!({"ok": false, "error": "pairing_notify_requires_device_id_and_message"}),
        };
    }
    let severity = clean_text(
        request
            .get("severity")
            .and_then(Value::as_str)
            .unwrap_or("info"),
        40,
    );
    let row = json!({
        "notification_id": format!(
            "pairnotify_{}",
            crate::deterministic_receipt_hash(&json!({"device_id": device_id, "message": message, "ts_ms": now_ms()}))
                .chars()
                .take(14)
                .collect::<String>()
        ),
        "device_id": device_id,
        "message": message,
        "severity": severity,
        "ts_ms": now_ms()
    });
    as_array_mut(&mut state["pairing"], "notifications").push(row.clone());
    if let Some(devices) = state
        .pointer_mut("/pairing/devices")
        .and_then(Value::as_array_mut)
    {
        if let Some(device) = devices.iter_mut().find(|device| {
            device.get("device_id").and_then(Value::as_str) == Some(device_id.as_str())
        })
        {
            device["last_notified_ms"] = json!(now_ms());
        }
    }
    save_state(root, state.clone());
    CompatApiResponse {
        status: 200,
        payload: json!({"ok": true, "type": "pairing_notify", "notification": row}),
    }
}

fn ensure_signing_key(state: &mut Value) -> Result<SigningKey, String> {
    let existing_seed_b64 = clean_text(
        state
            .pointer("/manifest_signing/secret_seed_b64")
            .and_then(Value::as_str)
            .unwrap_or(""),
        800,
    );
    if !existing_seed_b64.is_empty() {
        let decoded =
            base64::engine::general_purpose::STANDARD
                .decode(existing_seed_b64)
                .map_err(|err| format!("manifest_signing_seed_decode_failed:{err}"))?;
        if decoded.len() == 32 {
            let mut seed = [0u8; 32];
            seed.copy_from_slice(&decoded[..32]);
            return Ok(SigningKey::from_bytes(&seed));
        }
    }
    let seed_hash =
        crate::deterministic_receipt_hash(&json!({"ts_ms": now_ms(), "nonce": crate::now_iso()}));
    let mut seed = [0u8; 32];
    let seed_bytes = seed_hash.as_bytes();
    for idx in 0..32usize {
        seed[idx] = seed_bytes[idx % seed_bytes.len()];
    }
    let signing = SigningKey::from_bytes(&seed);
    state["manifest_signing"]["secret_seed_b64"] =
        Value::String(base64::engine::general_purpose::STANDARD.encode(seed));
    state["manifest_signing"]["public_key_b64"] = Value::String(
        base64::engine::general_purpose::STANDARD.encode(signing.verifying_key().to_bytes()),
    );
    Ok(signing)
}

fn manifest_payload_text(request: &Value) -> String {
    if let Some(raw) = request.get("manifest").and_then(Value::as_str) {
        return clean_text(raw, 32_000);
    }
    if let Some(manifest_value) = request.get("manifest") {
        return serde_json::to_string(manifest_value).unwrap_or_else(|_| "{}".to_string());
    }
    serde_json::to_string(request).unwrap_or_else(|_| "{}".to_string())
}

fn manifest_sign_payload(root: &Path, body: &[u8], state: &mut Value) -> CompatApiResponse {
    let request = parse_json(body);
    let manifest_text = manifest_payload_text(&request);
    let signing = match ensure_signing_key(state) {
        Ok(key) => key,
        Err(err) => {
            return CompatApiResponse {
                status: 500,
                payload: json!({"ok": false, "error": err}),
            };
        }
    };
    let signature = signing.sign(manifest_text.as_bytes());
    let public_key_b64 = base64::engine::general_purpose::STANDARD.encode(
        signing.verifying_key().to_bytes(),
    );
    let signature_b64 = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());
    save_state(root, state.clone());
    CompatApiResponse {
        status: 200,
        payload: json!({
            "ok": true,
            "type": "manifest_signing_receipt",
            "algorithm": "Ed25519",
            "manifest_hash": crate::deterministic_receipt_hash(&json!({"manifest": manifest_text})),
            "public_key_b64": public_key_b64,
            "signature_b64": signature_b64
        }),
    }
}

fn manifest_verify_payload(body: &[u8], state: &mut Value) -> CompatApiResponse {
    let request = parse_json(body);
    let manifest_text = manifest_payload_text(&request);
    let signature_b64 = clean_text(
        request
            .get("signature_b64")
            .or_else(|| request.get("signature"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        1200,
    );
    let public_key_b64 = clean_text(
        request
            .get("public_key_b64")
            .or_else(|| request.get("public_key"))
            .and_then(Value::as_str)
            .or_else(|| state.pointer("/manifest_signing/public_key_b64").and_then(Value::as_str))
            .unwrap_or(""),
        1200,
    );
    if signature_b64.is_empty() || public_key_b64.is_empty() {
        return CompatApiResponse {
            status: 400,
            payload: json!({"ok": false, "error": "manifest_verify_requires_signature_and_public_key"}),
        };
    }
    let signature_bytes = match base64::engine::general_purpose::STANDARD.decode(signature_b64) {
        Ok(bytes) => bytes,
        Err(err) => {
            return CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": format!("signature_decode_failed:{err}")}),
            };
        }
    };
    let signature = match Signature::from_slice(&signature_bytes) {
        Ok(sig) => sig,
        Err(err) => {
            return CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": format!("signature_invalid:{err}")}),
            };
        }
    };
    let public_key_bytes = match base64::engine::general_purpose::STANDARD.decode(public_key_b64) {
        Ok(bytes) => bytes,
        Err(err) => {
            return CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": format!("public_key_decode_failed:{err}")}),
            };
        }
    };
    if public_key_bytes.len() != 32 {
        return CompatApiResponse {
            status: 400,
            payload: json!({"ok": false, "error": "public_key_length_invalid"}),
        };
    }
    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&public_key_bytes[..32]);
    let public_key = match VerifyingKey::from_bytes(&key_bytes) {
        Ok(key) => key,
        Err(err) => {
            return CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": format!("public_key_invalid:{err}")}),
            };
        }
    };
    let valid = public_key
        .verify(manifest_text.as_bytes(), &signature)
        .is_ok();
    CompatApiResponse {
        status: if valid { 200 } else { 400 },
        payload: json!({
            "ok": valid,
            "type": "manifest_verify_receipt",
            "algorithm": "Ed25519",
            "valid": valid,
            "manifest_hash": crate::deterministic_receipt_hash(&json!({"manifest": manifest_text}))
        }),
    }
}

fn taint_type_rows() -> Value {
    json!([
        {"label":"trusted","description":"Input was sourced from trusted internal state."},
        {"label":"untrusted","description":"Input originated from untrusted external/user-provided source."},
        {"label":"secret","description":"Input contains credential-like material and must not be echoed."},
        {"label":"pii","description":"Input contains personal data and requires redaction controls."}
    ])
}

fn taint_analyze_payload(body: &[u8]) -> CompatApiResponse {
    let request = parse_json(body);
    let source = clean_text(
        request
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("untrusted"),
        80,
    )
    .to_ascii_lowercase();
    let input = request.get("input").cloned().unwrap_or_else(|| json!({}));
    let input_text = if input.is_string() {
        clean_text(input.as_str().unwrap_or(""), 16_000)
    } else {
        serde_json::to_string(&input).unwrap_or_else(|_| "{}".to_string())
    }
    .to_ascii_lowercase();
    let mut labels = Vec::<String>::new();
    if source != "trusted" && source != "internal" {
        labels.push("untrusted".to_string());
    } else {
        labels.push("trusted".to_string());
    }
    if ["token", "password", "secret", "api_key", "sk-"]
        .iter()
        .any(|needle| input_text.contains(needle))
    {
        labels.push("secret".to_string());
    }
    if ["email", "phone", "ssn", "address", "dob"]
        .iter()
        .any(|needle| input_text.contains(needle))
    {
        labels.push("pii".to_string());
    }
    labels.sort();
    labels.dedup();
    let risk = if labels.iter().any(|label| label == "secret") {
        "high"
    } else if labels.iter().any(|label| label == "untrusted")
        || labels.iter().any(|label| label == "pii")
    {
        "medium"
    } else {
        "low"
    };
    CompatApiResponse {
        status: 200,
        payload: json!({
            "ok": true,
            "type": "taint_analysis",
            "source": source,
            "labels": labels,
            "risk": risk,
            "tainted": risk != "low"
        }),
    }
}

fn openai_chat_completions_payload(root: &Path, body: &[u8]) -> CompatApiResponse {
    let request = parse_json(body);
    let model_input = clean_text(request.get("model").and_then(Value::as_str).unwrap_or(""), 240);
    let (provider, model, rendered_model) = parse_model(&model_input, &request);
    let messages = request
        .get("messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let system_prompt = messages
        .iter()
        .filter(|row| row.get("role").and_then(Value::as_str) == Some("system"))
        .map(extract_message_content)
        .find(|text| !text.is_empty())
        .unwrap_or_default();
    let user_message = messages
        .iter()
        .rev()
        .find(|row| row.get("role").and_then(Value::as_str) == Some("user"))
        .map(extract_message_content)
        .unwrap_or_default();
    if user_message.is_empty() {
        return CompatApiResponse {
            status: 400,
            payload: json!({
                "error": {
                    "message": "last user message is required",
                    "type": "invalid_request_error"
                }
            }),
        };
    }
    let reply = match crate::dashboard_provider_runtime::invoke_chat(
        root,
        &provider,
        &model,
        &system_prompt,
        &messages,
        &user_message,
    ) {
        Ok(value) => value,
        Err(err) => {
            return CompatApiResponse {
                status: 500,
                payload: json!({
                    "error": {
                        "message": clean_text(&err, 300),
                        "type": "runtime_error"
                    }
                }),
            };
        }
    };
    let content = clean_text(
        reply.get("response").and_then(Value::as_str).unwrap_or(""),
        32_000,
    );
    let completion_id = format!(
        "chatcmpl_{}",
        crate::deterministic_receipt_hash(&json!({
            "model": rendered_model,
            "user_message": user_message,
            "response": content,
            "ts_ms": now_ms()
        }))
        .chars()
        .take(22)
        .collect::<String>()
    );
    let prompt_tokens = reply.get("input_tokens").and_then(Value::as_i64).unwrap_or(0).max(0);
    let completion_tokens = reply
        .get("output_tokens")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    CompatApiResponse {
        status: 200,
        payload: json!({
            "id": completion_id,
            "object": "chat.completion",
            "created": now_ms() / 1000,
            "model": if rendered_model.is_empty() { model_input } else { rendered_model },
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": content
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": prompt_tokens,
                "completion_tokens": completion_tokens,
                "total_tokens": prompt_tokens + completion_tokens
            }
        }),
    }
}

pub fn handle(
    root: &Path,
    method: &str,
    path: &str,
    path_only: &str,
    body: &[u8],
    snapshot: &Value,
) -> Option<CompatApiResponse> {
    if method == "POST" && path_only == "/v1/chat/completions" {
        return Some(openai_chat_completions_payload(root, body));
    }

    let mut state = load_state(root);

    if method == "GET" && path_only == "/api/a2a/agents" {
        return Some(a2a_agents_payload(root, &mut state));
    }
    if (method == "GET" || method == "POST") && path_only == "/api/a2a/discover" {
        return Some(a2a_discover_payload(root, path, body, &mut state));
    }
    if method == "POST" && path_only == "/api/a2a/send" {
        return Some(a2a_send_payload(root, body, &mut state));
    }
    if method == "GET" && path_only == "/api/a2a/task-status" {
        return Some(a2a_task_status_payload(path, &state));
    }
    if method == "GET" {
        if let Some(task_id) = path_only.strip_prefix("/api/a2a/tasks/") {
            let clean_id = clean_text(task_id, 120);
            if let Some(task) = lookup_task(&state, &clean_id) {
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({"ok": true, "task": task}),
                });
            }
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "task_not_found", "task_id": clean_id}),
            });
        }
    }

    if method == "POST" && path_only == "/api/peers/handshake" {
        return Some(peer_handshake_payload(root, body, &mut state));
    }
    if method == "GET" && path_only == "/api/peers" {
        return Some(CompatApiResponse {
            status: 200,
            payload: peers_payload(&state),
        });
    }
    if method == "GET" && path_only == "/api/network/status" {
        return Some(CompatApiResponse {
            status: 200,
            payload: network_status_payload(&state, snapshot),
        });
    }

    if method == "POST" && path_only == "/api/pairing/request" {
        return Some(pairing_request_payload(root, body, &mut state));
    }
    if method == "POST" && path_only == "/api/pairing/complete" {
        return Some(pairing_complete_payload(root, body, &mut state));
    }
    if method == "GET" && path_only == "/api/pairing/devices" {
        return Some(pairing_devices_payload(&state));
    }
    if method == "POST" && path_only == "/api/pairing/notify" {
        return Some(pairing_notify_payload(root, body, &mut state));
    }

    if method == "GET" && path_only == "/api/security/taint/types" {
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "type": "taint_types", "labels": taint_type_rows()}),
        });
    }
    if method == "POST" && path_only == "/api/security/taint/analyze" {
        return Some(taint_analyze_payload(body));
    }
    if method == "POST" && path_only == "/api/security/manifest/sign" {
        return Some(manifest_sign_payload(root, body, &mut state));
    }
    if method == "POST" && path_only == "/api/security/manifest/verify" {
        return Some(manifest_verify_payload(body, &mut state));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a2a_discover_send_and_task_status_flow() {
        let root = tempfile::tempdir().expect("tempdir");
        let discover = handle(
            root.path(),
            "GET",
            "/api/a2a/discover",
            "/api/a2a/discover",
            &[],
            &json!({}),
        )
        .expect("discover");
        assert_eq!(discover.status, 200);
        let send = handle(
            root.path(),
            "POST",
            "/api/a2a/send",
            "/api/a2a/send",
            br#"{"from_agent_id":"AG1","to_agent_id":"AG2","message":"sync now"}"#,
            &json!({}),
        )
        .expect("send");
        assert_eq!(send.status, 200);
        let task_id = clean_text(
            send.payload
                .pointer("/task/task_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        assert!(!task_id.is_empty());
        let status = handle(
            root.path(),
            "GET",
            &format!("/api/a2a/task-status?task_id={task_id}"),
            "/api/a2a/task-status",
            &[],
            &json!({}),
        )
        .expect("task status");
        assert_eq!(status.status, 200);
        assert_eq!(
            status
                .payload
                .pointer("/task/status")
                .and_then(Value::as_str)
                .unwrap_or(""),
            "completed"
        );
    }

    #[test]
    fn peer_handshake_denies_replay_nonce() {
        let root = tempfile::tempdir().expect("tempdir");
        let ts_ms = now_ms();
        let payload = format!("peer-alpha|nonce-1|{ts_ms}");
        let signature = hmac_signature("infring-reference_runtime-peer-shared-secret", &payload)
            .expect("signature");
        let body = json!({
            "peer_id": "peer-alpha",
            "nonce": "nonce-1",
            "ts_ms": ts_ms,
            "signature": signature,
            "address": "10.0.0.5:9000"
        });
        let first = handle(
            root.path(),
            "POST",
            "/api/peers/handshake",
            "/api/peers/handshake",
            serde_json::to_string(&body).expect("serialize").as_bytes(),
            &json!({}),
        )
        .expect("first");
        assert_eq!(first.status, 200);
        let second = handle(
            root.path(),
            "POST",
            "/api/peers/handshake",
            "/api/peers/handshake",
            serde_json::to_string(&body).expect("serialize").as_bytes(),
            &json!({}),
        )
        .expect("second");
        assert_eq!(second.status, 409);
    }

    #[test]
    fn pairing_request_complete_and_devices() {
        let root = tempfile::tempdir().expect("tempdir");
        let requested = handle(
            root.path(),
            "POST",
            "/api/pairing/request",
            "/api/pairing/request",
            br#"{"device_name":"Jay iPhone","platform":"ios"}"#,
            &json!({}),
        )
        .expect("pair request");
        assert_eq!(requested.status, 200);
        let pairing_id = clean_text(
            requested
                .payload
                .pointer("/pairing/pairing_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        assert!(!pairing_id.is_empty());
        let completed = handle(
            root.path(),
            "POST",
            "/api/pairing/complete",
            "/api/pairing/complete",
            format!(r#"{{"pairing_id":"{pairing_id}"}}"#).as_bytes(),
            &json!({}),
        )
        .expect("pair complete");
        assert_eq!(completed.status, 200);
        let devices = handle(
            root.path(),
            "GET",
            "/api/pairing/devices",
            "/api/pairing/devices",
            &[],
            &json!({}),
        )
        .expect("devices");
        assert_eq!(devices.status, 200);
        assert!(
            devices
                .payload
                .get("count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                >= 1
        );
    }

    #[test]
    fn v1_chat_completions_returns_openai_shape() {
        let root = tempfile::tempdir().expect("tempdir");
        let response = handle(
            root.path(),
            "POST",
            "/v1/chat/completions",
            "/v1/chat/completions",
            br#"{"model":"auto","messages":[{"role":"system","content":"be concise"},{"role":"user","content":"hello"}]}"#,
            &json!({}),
        )
        .expect("chat completions");
        assert_eq!(response.status, 200);
        assert_eq!(
            response
                .payload
                .get("object")
                .and_then(Value::as_str)
                .unwrap_or(""),
            "chat.completion"
        );
        assert!(
            response
                .payload
                .get("choices")
                .and_then(Value::as_array)
                .map(|rows| !rows.is_empty())
                .unwrap_or(false)
        );
    }

    #[test]
    fn manifest_sign_and_verify_round_trip() {
        let root = tempfile::tempdir().expect("tempdir");
        let sign = handle(
            root.path(),
            "POST",
            "/api/security/manifest/sign",
            "/api/security/manifest/sign",
            br#"{"manifest":{"name":"demo","version":"1.0.0"}}"#,
            &json!({}),
        )
        .expect("sign");
        assert_eq!(sign.status, 200);
        let signature_b64 = clean_text(
            sign.payload
                .get("signature_b64")
                .and_then(Value::as_str)
                .unwrap_or(""),
            1200,
        );
        let public_key_b64 = clean_text(
            sign.payload
                .get("public_key_b64")
                .and_then(Value::as_str)
                .unwrap_or(""),
            1200,
        );
        let verify_body = json!({
            "manifest": {"name":"demo","version":"1.0.0"},
            "signature_b64": signature_b64,
            "public_key_b64": public_key_b64
        });
        let verify = handle(
            root.path(),
            "POST",
            "/api/security/manifest/verify",
            "/api/security/manifest/verify",
            serde_json::to_string(&verify_body)
                .expect("serialize")
                .as_bytes(),
            &json!({}),
        )
        .expect("verify");
        assert_eq!(verify.status, 200);
        assert!(
            verify
                .payload
                .get("valid")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        );
    }

    #[test]
    fn peers_and_network_status_reflect_handshake_state() {
        let root = tempfile::tempdir().expect("tempdir");
        let ts_ms = now_ms();
        let payload = format!("peer-bravo|nonce-peers|{ts_ms}");
        let signature = hmac_signature("infring-reference_runtime-peer-shared-secret", &payload)
            .expect("signature");
        let handshake = handle(
            root.path(),
            "POST",
            "/api/peers/handshake",
            "/api/peers/handshake",
            serde_json::to_string(&json!({
                "peer_id": "peer-bravo",
                "nonce": "nonce-peers",
                "ts_ms": ts_ms,
                "signature": signature,
                "address": "127.0.0.1:9777"
            }))
            .expect("serialize")
            .as_bytes(),
            &json!({}),
        )
        .expect("handshake");
        assert_eq!(handshake.status, 200);

        let peers = handle(
            root.path(),
            "GET",
            "/api/peers",
            "/api/peers",
            &[],
            &json!({}),
        )
        .expect("peers");
        assert_eq!(peers.status, 200);
        assert_eq!(
            peers
                .payload
                .get("connected")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            1
        );

        let status = handle(
            root.path(),
            "GET",
            "/api/network/status",
            "/api/network/status",
            &[],
            &json!({"runtime_sync":{"summary":{"queue_depth":0}}}),
        )
        .expect("network status");
        assert_eq!(status.status, 200);
        assert_eq!(
            status
                .payload
                .get("connected_peers")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            1
        );
    }

    #[test]
    fn pairing_notify_records_notification_and_updates_device() {
        let root = tempfile::tempdir().expect("tempdir");
        let requested = handle(
            root.path(),
            "POST",
            "/api/pairing/request",
            "/api/pairing/request",
            br#"{"device_name":"Ops Mac","platform":"macos"}"#,
            &json!({}),
        )
        .expect("pair request");
        let pairing_id = clean_text(
            requested
                .payload
                .pointer("/pairing/pairing_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let completed = handle(
            root.path(),
            "POST",
            "/api/pairing/complete",
            "/api/pairing/complete",
            format!(r#"{{"pairing_id":"{pairing_id}"}}"#).as_bytes(),
            &json!({}),
        )
        .expect("pair complete");
        let device_id = clean_text(
            completed
                .payload
                .pointer("/device/device_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        assert!(!device_id.is_empty());

        let notify = handle(
            root.path(),
            "POST",
            "/api/pairing/notify",
            "/api/pairing/notify",
            format!(
                r#"{{"device_id":"{device_id}","message":"background sync complete","severity":"info"}}"#
            )
            .as_bytes(),
            &json!({}),
        )
        .expect("notify");
        assert_eq!(notify.status, 200);
        assert_eq!(
            notify
                .payload
                .pointer("/notification/device_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            device_id
        );
    }

    #[test]
    fn taint_types_and_analysis_surface_runtime_labels() {
        let root = tempfile::tempdir().expect("tempdir");
        let types = handle(
            root.path(),
            "GET",
            "/api/security/taint/types",
            "/api/security/taint/types",
            &[],
            &json!({}),
        )
        .expect("taint types");
        assert_eq!(types.status, 200);
        assert!(
            types
                .payload
                .pointer("/labels")
                .and_then(Value::as_array)
                .map(|rows| !rows.is_empty())
                .unwrap_or(false)
        );

        let analysis = handle(
            root.path(),
            "POST",
            "/api/security/taint/analyze",
            "/api/security/taint/analyze",
            br#"{"source":"user","input":"my secret token is sk-123"}"#,
            &json!({}),
        )
        .expect("taint analyze");
        assert_eq!(analysis.status, 200);
        let labels = analysis
            .payload
            .get("labels")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| row.as_str().map(|value| value.to_string()))
            .collect::<Vec<_>>();
        assert!(labels.iter().any(|label| label == "secret"));
        assert!(labels.iter().any(|label| label == "untrusted"));
    }
}
