
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
