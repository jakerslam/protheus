// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use super::CompatApiResponse;
use base64::Engine;
use serde_json::{json, Value};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const REFERENCE_PARITY_STATE_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/reference_runtime_parity_state.json";
const REFERENCE_UPLOADS_DIR_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/reference_runtime_uploads";
const ACTION_HISTORY_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/actions/history.jsonl";

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
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

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn parse_json(raw: &[u8]) -> Value {
    serde_json::from_slice::<Value>(raw).unwrap_or_else(|_| json!({}))
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
            let value = clean_text(&decoded, 160);
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn header_value(headers: &[(&str, &str)], key: &str) -> String {
    headers
        .iter()
        .find(|(name, _)| clean_text(name, 80).eq_ignore_ascii_case(key))
        .map(|(_, value)| clean_text(value, 1024))
        .unwrap_or_default()
}

fn stable_hash(seed: &str, len: usize) -> String {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    let digest = format!("{:016x}", hasher.finish());
    digest.chars().take(len.max(1).min(digest.len())).collect()
}

fn load_parity_state(root: &Path) -> Value {
    read_json(&state_path(root, REFERENCE_PARITY_STATE_REL)).unwrap_or_else(|| {
        json!({
            "type": "reference_runtime_parity_state",
            "updated_at": crate::now_iso(),
            "auth": {
                "token": "",
                "user": "operator",
                "login_at_ms": 0
            },
            "pairing": {
                "pairing_id": "",
                "status": "idle",
                "code": "",
                "started_at_ms": 0,
                "updated_at_ms": 0
            },
            "uploads": []
        })
    })
}

fn save_parity_state(root: &Path, mut state: Value) {
    state["updated_at"] = Value::String(crate::now_iso());
    write_json(&state_path(root, REFERENCE_PARITY_STATE_REL), &state);
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

fn models_v1_payload(root: &Path, snapshot: &Value) -> Value {
    let catalog = crate::dashboard_model_catalog::catalog_payload(root, snapshot);
    let data = catalog
        .get("models")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let provider = clean_text(
                        row.get("provider")
                            .and_then(Value::as_str)
                            .unwrap_or("auto"),
                        80,
                    );
                    let model =
                        clean_text(row.get("model").and_then(Value::as_str).unwrap_or(""), 160);
                    if model.is_empty() {
                        return None;
                    }
                    Some(json!({
                        "id": model,
                        "object": "model",
                        "owned_by": provider,
                        "provider": provider,
                        "available": row.get("available").and_then(Value::as_bool).unwrap_or(false),
                        "context_window": row
                            .get("context_window_tokens")
                            .and_then(Value::as_u64)
                            .unwrap_or(0)
                    }))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    json!({
        "object": "list",
        "data": data
    })
}

fn login_payload(root: &Path, body: &[u8], headers: &[(&str, &str)]) -> CompatApiResponse {
    let request = parse_json(body);
    let user = clean_text(
        request
            .get("email")
            .and_then(Value::as_str)
            .or_else(|| request.get("username").and_then(Value::as_str))
            .or_else(|| request.get("user").and_then(Value::as_str))
            .unwrap_or("operator"),
        120,
    );
    let user = if user.is_empty() {
        "operator".to_string()
    } else {
        user
    };
    let host = header_value(headers, "host");
    let seed = format!("{user}|{}|{host}", now_ms());
    let token = format!("ofg_{}", stable_hash(&seed, 28));
    let mut state = load_parity_state(root);
    state["auth"] = json!({
        "token": token,
        "user": user,
        "login_at_ms": now_ms()
    });
    save_parity_state(root, state.clone());
    CompatApiResponse {
        status: 200,
        payload: json!({
            "ok": true,
            "token": state.pointer("/auth/token").cloned().unwrap_or(Value::String(String::new())),
            "user": {
                "id": state.pointer("/auth/user").cloned().unwrap_or(Value::String("operator".to_string())),
                "email": state.pointer("/auth/user").cloned().unwrap_or(Value::String("operator".to_string()))
            },
            "expires_in": 86400
        }),
    }
}

fn logout_payload(root: &Path) -> CompatApiResponse {
    let mut state = load_parity_state(root);
    state["auth"] = json!({
        "token": "",
        "user": "operator",
        "login_at_ms": 0
    });
    save_parity_state(root, state);
    CompatApiResponse {
        status: 200,
        payload: json!({"ok": true, "logged_out": true}),
    }
}

fn integrations_payload(root: &Path) -> Value {
    let channels = super::dashboard_compat_api_channels::channels_payload(root)
        .get("channels")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut items = channels
        .iter()
        .map(|row| {
            json!({
                "id": clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 120),
                "name": clean_text(row.get("display_name").and_then(Value::as_str).unwrap_or(""), 160),
                "category": clean_text(row.get("category").and_then(Value::as_str).unwrap_or(""), 80),
                "adapter": clean_text(row.get("runtime_adapter").and_then(Value::as_str).unwrap_or(""), 120),
                "connected": row.get("configured").and_then(Value::as_bool).unwrap_or(false),
                "has_token": row.get("has_token").and_then(Value::as_bool).unwrap_or(false),
                "ready": row.get("ready").and_then(Value::as_bool).unwrap_or(false),
                "real_channel": row.get("real_channel").and_then(Value::as_bool).unwrap_or(false)
            })
        })
        .collect::<Vec<_>>();
    items.sort_by(|a, b| {
        clean_text(a.get("id").and_then(Value::as_str).unwrap_or(""), 120).cmp(&clean_text(
            b.get("id").and_then(Value::as_str).unwrap_or(""),
            120,
        ))
    });
    json!({"ok": true, "items": items, "total": items.len()})
}

fn integration_detail_payload(root: &Path, integration_id: &str) -> CompatApiResponse {
    let needle = clean_text(integration_id, 120).to_ascii_lowercase();
    let Some(row) = integrations_payload(root)
        .get("items")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter().find(|row| {
                clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 120)
                    .eq_ignore_ascii_case(&needle)
            })
        })
        .cloned()
    else {
        return CompatApiResponse {
            status: 404,
            payload: json!({"ok": false, "error": "integration_not_found", "integration_id": needle}),
        };
    };
    CompatApiResponse {
        status: 200,
        payload: json!({"ok": true, "integration": row}),
    }
}

fn rewrite_integration_to_channel(path_only: &str) -> Option<String> {
    let suffix = path_only.strip_prefix("/api/integrations/")?;
    let mut parts = suffix.split('/');
    let name = clean_text(parts.next().unwrap_or(""), 120);
    let action = clean_text(parts.next().unwrap_or(""), 80);
    if name.is_empty() || action.is_empty() {
        return None;
    }
    if action == "configure" || action == "test" {
        return Some(format!("/api/channels/{name}/{action}"));
    }
    None
}

fn pairing_start_payload(root: &Path) -> CompatApiResponse {
    let pairing_id = format!("pair_{}", stable_hash(&format!("{}|pairing", now_ms()), 16));
    let code_raw = stable_hash(&format!("{pairing_id}|{}", now_ms()), 12);
    let code = code_raw
        .chars()
        .take(6)
        .collect::<String>()
        .to_ascii_uppercase();
    let mut state = load_parity_state(root);
    state["pairing"] = json!({
        "pairing_id": pairing_id,
        "status": "pending",
        "code": code,
        "started_at_ms": now_ms(),
        "updated_at_ms": now_ms()
    });
    save_parity_state(root, state.clone());
    CompatApiResponse {
        status: 200,
        payload: json!({
            "ok": true,
            "pairing_id": state.pointer("/pairing/pairing_id").cloned().unwrap_or(Value::String(String::new())),
            "status": "pending",
            "code": state.pointer("/pairing/code").cloned().unwrap_or(Value::String(String::new()))
        }),
    }
}

fn pairing_status_payload(root: &Path, path: &str) -> CompatApiResponse {
    let state = load_parity_state(root);
    let requested = query_value(path, "pairing_id").unwrap_or_default();
    let current = clean_text(
        state
            .pointer("/pairing/pairing_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    if !requested.is_empty() && !requested.eq_ignore_ascii_case(&current) {
        return CompatApiResponse {
            status: 404,
            payload: json!({"ok": false, "error": "pairing_not_found", "pairing_id": requested}),
        };
    }
    CompatApiResponse {
        status: 200,
        payload: json!({
            "ok": true,
            "pairing": state.get("pairing").cloned().unwrap_or_else(|| json!({}))
        }),
    }
}

fn pairing_transition_payload(root: &Path, body: &[u8], status: &str) -> CompatApiResponse {
    let request = parse_json(body);
    let requested = clean_text(
        request
            .get("pairing_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    let mut state = load_parity_state(root);
    let current = clean_text(
        state
            .pointer("/pairing/pairing_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    if current.is_empty() || (!requested.is_empty() && !requested.eq_ignore_ascii_case(&current)) {
        return CompatApiResponse {
            status: 404,
            payload: json!({"ok": false, "error": "pairing_not_found"}),
        };
    }
    state["pairing"]["status"] = Value::String(clean_text(status, 40));
    state["pairing"]["updated_at_ms"] = json!(now_ms());
    save_parity_state(root, state.clone());
    CompatApiResponse {
        status: 200,
        payload: json!({
            "ok": true,
            "pairing_id": current,
            "status": clean_text(status, 40),
            "pairing": state.get("pairing").cloned().unwrap_or_else(|| json!({}))
        }),
    }
}

fn upload_bytes_from_request(request: &Value, body: &[u8]) -> Vec<u8> {
    if let Some(text) = request.get("content_base64").and_then(Value::as_str) {
        if let Ok(bytes) =
            base64::engine::general_purpose::STANDARD.decode(clean_text(text, 1_000_000))
        {
            return bytes;
        }
    }
    if let Some(text) = request.get("content").and_then(Value::as_str) {
        return text.as_bytes().to_vec();
    }
    body.to_vec()
}

fn uploads_list(root: &Path) -> Vec<Value> {
    load_parity_state(root)
        .get("uploads")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn upload_create_payload(root: &Path, body: &[u8]) -> CompatApiResponse {
    let request = parse_json(body);
    let bytes = upload_bytes_from_request(&request, body);
    if bytes.is_empty() {
        return CompatApiResponse {
            status: 400,
            payload: json!({"ok": false, "error": "upload_empty"}),
        };
    }
    let filename = clean_text(
        request
            .get("filename")
            .and_then(Value::as_str)
            .unwrap_or("upload.bin"),
        200,
    );
    let upload_id = format!(
        "upl_{}",
        stable_hash(&format!("{}|{}|{}", filename, bytes.len(), now_ms()), 18)
    );
    let digest = stable_hash(&String::from_utf8_lossy(&bytes), 16);
    let uploads_dir = state_path(root, REFERENCE_UPLOADS_DIR_REL);
    let _ = fs::create_dir_all(&uploads_dir);
    let file_path = uploads_dir.join(format!("{upload_id}.bin"));
    let _ = fs::write(&file_path, &bytes);

    let mut state = load_parity_state(root);
    let uploads = as_array_mut(&mut state, "uploads");
    uploads.retain(|row| {
        clean_text(
            row.get("upload_id").and_then(Value::as_str).unwrap_or(""),
            120,
        ) != upload_id
    });
    uploads.push(json!({
        "upload_id": upload_id,
        "filename": filename,
        "size_bytes": bytes.len(),
        "hash": digest,
        "path": clean_text(file_path.to_string_lossy().as_ref(), 500),
        "created_at": crate::now_iso(),
        "created_at_ms": now_ms()
    }));
    save_parity_state(root, state.clone());
    let upload = state
        .get("uploads")
        .and_then(Value::as_array)
        .and_then(|rows| rows.last())
        .cloned()
        .unwrap_or_else(|| json!({}));
    CompatApiResponse {
        status: 200,
        payload: json!({"ok": true, "upload": upload}),
    }
}

fn upload_detail_payload(root: &Path, upload_id: &str) -> CompatApiResponse {
    let needle = clean_text(upload_id, 120);
    let Some(row) = uploads_list(root).into_iter().find(|row| {
        clean_text(
            row.get("upload_id").and_then(Value::as_str).unwrap_or(""),
            120,
        ) == needle
    }) else {
        return CompatApiResponse {
            status: 404,
            payload: json!({"ok": false, "error": "upload_not_found", "upload_id": needle}),
        };
    };
    CompatApiResponse {
        status: 200,
        payload: json!({"ok": true, "upload": row}),
    }
}

fn upload_delete_payload(root: &Path, upload_id: &str) -> CompatApiResponse {
    let needle = clean_text(upload_id, 120);
    let mut state = load_parity_state(root);
    let rows = as_array_mut(&mut state, "uploads");
    let mut removed = None;
    rows.retain(|row| {
        let keep = clean_text(
            row.get("upload_id").and_then(Value::as_str).unwrap_or(""),
            120,
        ) != needle;
        if !keep {
            removed = row
                .get("path")
                .and_then(Value::as_str)
                .map(|v| v.to_string());
        }
        keep
    });
    if let Some(path) = removed {
        let _ = fs::remove_file(path);
        save_parity_state(root, state);
        return CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "deleted": true, "upload_id": needle}),
        };
    }
    CompatApiResponse {
        status: 404,
        payload: json!({"ok": false, "error": "upload_not_found", "upload_id": needle}),
    }
}

fn recent_log_events(root: &Path, limit: usize) -> Vec<Value> {
    let mut rows = fs::read_to_string(state_path(root, ACTION_HISTORY_REL))
        .ok()
        .map(|raw| {
            raw.lines()
                .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if rows.len() > limit {
        rows = rows.split_off(rows.len() - limit);
    }
    rows
}

fn stream_payload(root: &Path, stream: &str) -> CompatApiResponse {
    let events = recent_log_events(root, 60);
    CompatApiResponse {
        status: 200,
        payload: json!({
            "ok": true,
            "stream": clean_text(stream, 80),
            "events": events,
            "count": events.len(),
            "cursor": stable_hash(&format!("{}|{}", stream, now_ms()), 12)
        }),
    }
}

fn handle_marketplace_aliases(
    root: &Path,
    method: &str,
    path: &str,
    path_only: &str,
    snapshot: &Value,
    body: &[u8],
) -> Option<CompatApiResponse> {
    if method == "GET"
        && (path_only == "/api/marketplace" || path_only == "/api/marketplace/browse")
    {
        let query = path.split_once('?').map(|(_, q)| q).unwrap_or("limit=25");
        let forwarded = format!("/api/clawhub/browse?{query}");
        return super::dashboard_skills_marketplace::handle(root, "GET", &forwarded, snapshot, &[]);
    }
    if method == "GET" && path_only == "/api/marketplace/search" {
        let query = path.split_once('?').map(|(_, q)| q).unwrap_or("q=");
        let forwarded = format!("/api/clawhub/search?{query}");
        return super::dashboard_skills_marketplace::handle(root, "GET", &forwarded, snapshot, &[]);
    }
    if method == "GET" {
        if let Some(slug) = path_only.strip_prefix("/api/marketplace/skill/") {
            let forwarded = format!("/api/clawhub/skill/{}", clean_text(slug, 160));
            return super::dashboard_skills_marketplace::handle(
                root,
                "GET",
                &forwarded,
                snapshot,
                &[],
            );
        }
    }
    if method == "POST" && path_only == "/api/skills/install" {
        return super::dashboard_skills_marketplace::handle(
            root,
            "POST",
            "/api/clawhub/install",
            snapshot,
            body,
        );
    }
    if method == "POST" && path_only == "/api/skills/reload" {
        if let Some(listed) =
            super::dashboard_skills_marketplace::handle(root, "GET", "/api/skills", snapshot, &[])
        {
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "reloaded": true, "skills": listed.payload.get("skills").cloned().unwrap_or_else(|| Value::Array(Vec::new()))}),
            });
        }
    }
    None
}

pub fn handle(
    root: &Path,
    method: &str,
    path: &str,
    path_only: &str,
    headers: &[(&str, &str)],
    body: &[u8],
    snapshot: &Value,
) -> Option<CompatApiResponse> {
    if method == "GET" && path_only == "/v1/models" {
        return Some(CompatApiResponse {
            status: 200,
            payload: models_v1_payload(root, snapshot),
        });
    }

    if method == "POST" && path_only == "/api/auth/login" {
        return Some(login_payload(root, body, headers));
    }
    if method == "POST" && path_only == "/api/auth/logout" {
        return Some(logout_payload(root));
    }

    if method == "GET"
        && (path_only == "/api/integrations" || path_only == "/api/integrations/catalog")
    {
        return Some(CompatApiResponse {
            status: 200,
            payload: integrations_payload(root),
        });
    }
    if method == "GET" {
        if let Some(id) = path_only.strip_prefix("/api/integrations/") {
            if !id.contains('/') {
                return Some(integration_detail_payload(root, id));
            }
        }
    }
    if let Some(channel_path) = rewrite_integration_to_channel(path_only) {
        return super::dashboard_compat_api_channels::handle(root, method, &channel_path, body);
    }

    if method == "POST" && path_only == "/api/pairing/start" {
        return Some(pairing_start_payload(root));
    }
    if method == "GET" && path_only == "/api/pairing/status" {
        return Some(pairing_status_payload(root, path));
    }
    if method == "POST" && path_only == "/api/pairing/confirm" {
        return Some(pairing_transition_payload(root, body, "confirmed"));
    }
    if method == "POST" && path_only == "/api/pairing/cancel" {
        return Some(pairing_transition_payload(root, body, "cancelled"));
    }

    if method == "GET" && path_only == "/api/uploads" {
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "uploads": uploads_list(root)}),
        });
    }
    if method == "POST" && path_only == "/api/uploads" {
        return Some(upload_create_payload(root, body));
    }
    if method == "GET" {
        if let Some(upload_id) = path_only.strip_prefix("/api/uploads/") {
            return Some(upload_detail_payload(root, upload_id));
        }
    }
    if method == "DELETE" {
        if let Some(upload_id) = path_only.strip_prefix("/api/uploads/") {
            return Some(upload_delete_payload(root, upload_id));
        }
    }

    if method == "GET" && path_only == "/api/logs/stream" {
        return Some(stream_payload(root, "logs"));
    }
    if method == "GET" && path_only == "/api/comms/events/stream" {
        return Some(stream_payload(root, "comms_events"));
    }

    if let Some(response) =
        handle_marketplace_aliases(root, method, path, path_only, snapshot, body)
    {
        return Some(response);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_models_endpoint_returns_list_shape() {
        let root = tempfile::tempdir().expect("tempdir");
        let response = handle(
            root.path(),
            "GET",
            "/v1/models",
            "/v1/models",
            &[],
            &[],
            &json!({"ok": true}),
        )
        .expect("models response");
        assert_eq!(response.status, 200);
        assert_eq!(
            response
                .payload
                .get("object")
                .and_then(Value::as_str)
                .unwrap_or(""),
            "list"
        );
        assert!(response
            .payload
            .get("data")
            .map(Value::is_array)
            .unwrap_or(false));
    }

    #[test]
    fn auth_login_logout_round_trip() {
        let root = tempfile::tempdir().expect("tempdir");
        let login = handle(
            root.path(),
            "POST",
            "/api/auth/login",
            "/api/auth/login",
            &[("host", "localhost:4200")],
            br#"{"email":"ops@example.com"}"#,
            &json!({}),
        )
        .expect("login");
        assert_eq!(login.status, 200);
        let token = clean_text(
            login
                .payload
                .get("token")
                .and_then(Value::as_str)
                .unwrap_or(""),
            200,
        );
        assert!(!token.is_empty());

        let logout = handle(
            root.path(),
            "POST",
            "/api/auth/logout",
            "/api/auth/logout",
            &[],
            &[],
            &json!({}),
        )
        .expect("logout");
        assert_eq!(logout.status, 200);
        assert!(logout
            .payload
            .get("logged_out")
            .and_then(Value::as_bool)
            .unwrap_or(false));
    }

    #[test]
    fn integrations_aliases_to_channels() {
        let root = tempfile::tempdir().expect("tempdir");
        let list = handle(
            root.path(),
            "GET",
            "/api/integrations",
            "/api/integrations",
            &[],
            &[],
            &json!({}),
        )
        .expect("integrations");
        assert_eq!(list.status, 200);
        let items = list
            .payload
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!items.is_empty());

        let configure = handle(
            root.path(),
            "POST",
            "/api/integrations/telegram/configure",
            "/api/integrations/telegram/configure",
            &[],
            br#"{"token":"abc","endpoint":"https://api.telegram.org"}"#,
            &json!({}),
        )
        .expect("configure");
        assert_eq!(configure.status, 200);
        assert!(configure
            .payload
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(false));
    }

    #[test]
    fn pairing_start_status_confirm_flow() {
        let root = tempfile::tempdir().expect("tempdir");
        let started = handle(
            root.path(),
            "POST",
            "/api/pairing/start",
            "/api/pairing/start",
            &[],
            &[],
            &json!({}),
        )
        .expect("pair start");
        let pairing_id = clean_text(
            started
                .payload
                .get("pairing_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        assert!(!pairing_id.is_empty());

        let status = handle(
            root.path(),
            "GET",
            &format!("/api/pairing/status?pairing_id={pairing_id}"),
            "/api/pairing/status",
            &[],
            &[],
            &json!({}),
        )
        .expect("pair status");
        assert_eq!(status.status, 200);

        let confirmed = handle(
            root.path(),
            "POST",
            "/api/pairing/confirm",
            "/api/pairing/confirm",
            &[],
            format!(r#"{{"pairing_id":"{pairing_id}"}}"#).as_bytes(),
            &json!({}),
        )
        .expect("pair confirm");
        assert_eq!(
            confirmed
                .payload
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or(""),
            "confirmed"
        );
    }

    #[test]
    fn uploads_create_list_detail_delete() {
        let root = tempfile::tempdir().expect("tempdir");
        let created = handle(
            root.path(),
            "POST",
            "/api/uploads",
            "/api/uploads",
            &[],
            br#"{"filename":"notes.txt","content":"hello"}"#,
            &json!({}),
        )
        .expect("upload create");
        assert_eq!(created.status, 200);
        let upload_id = clean_text(
            created
                .payload
                .pointer("/upload/upload_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        assert!(!upload_id.is_empty());

        let listed = handle(
            root.path(),
            "GET",
            "/api/uploads",
            "/api/uploads",
            &[],
            &[],
            &json!({}),
        )
        .expect("uploads list");
        let rows = listed
            .payload
            .get("uploads")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(rows.len(), 1);

        let detail = handle(
            root.path(),
            "GET",
            &format!("/api/uploads/{upload_id}"),
            &format!("/api/uploads/{upload_id}"),
            &[],
            &[],
            &json!({}),
        )
        .expect("upload detail");
        assert_eq!(
            clean_text(
                detail
                    .payload
                    .pointer("/upload/upload_id")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120
            ),
            upload_id
        );

        let deleted = handle(
            root.path(),
            "DELETE",
            &format!("/api/uploads/{upload_id}"),
            &format!("/api/uploads/{upload_id}"),
            &[],
            &[],
            &json!({}),
        )
        .expect("upload delete");
        assert!(deleted
            .payload
            .get("deleted")
            .and_then(Value::as_bool)
            .unwrap_or(false));
    }

    #[test]
    fn marketplace_aliases_and_reload_work() {
        let root = tempfile::tempdir().expect("tempdir");
        let browse = handle(
            root.path(),
            "GET",
            "/api/marketplace?limit=5",
            "/api/marketplace",
            &[],
            &[],
            &json!({}),
        )
        .expect("marketplace browse");
        assert_eq!(browse.status, 200);
        assert!(browse
            .payload
            .get("items")
            .map(Value::is_array)
            .unwrap_or(false));

        let reload = handle(
            root.path(),
            "POST",
            "/api/skills/reload",
            "/api/skills/reload",
            &[],
            &[],
            &json!({}),
        )
        .expect("skills reload");
        assert_eq!(reload.status, 200);
        assert!(reload
            .payload
            .get("reloaded")
            .and_then(Value::as_bool)
            .unwrap_or(false));
    }
}
