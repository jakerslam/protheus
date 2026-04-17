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
