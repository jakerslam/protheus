
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
