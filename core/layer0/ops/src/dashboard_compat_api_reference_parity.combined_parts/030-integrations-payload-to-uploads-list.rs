
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
