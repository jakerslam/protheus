
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
