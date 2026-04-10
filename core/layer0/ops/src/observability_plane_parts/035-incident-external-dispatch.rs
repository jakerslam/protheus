#[derive(Debug)]
struct IncidentExternalDispatchResult {
    requested: bool,
    mode: String,
    receipts: Vec<Value>,
    hard_fail: bool,
}

fn run_incident_external_dispatch(
    parsed: &crate::ParsedArgs,
    incident_id: &str,
    runbook: &str,
    requested_actions: &[String],
) -> IncidentExternalDispatchResult {
    let force_dispatch = parse_bool(parsed.flags.get("dispatch-external"), false);
    let requested = force_dispatch || requested_actions.iter().any(|row| row == "page-oncall");
    if !requested {
        return IncidentExternalDispatchResult {
            requested,
            mode: "off".to_string(),
            receipts: Vec::new(),
            hard_fail: false,
        };
    }
    let mode = incident_external_dispatch_mode(parsed);
    let require_success = parse_bool(parsed.flags.get("require-external-dispatch"), false);
    let providers = incident_external_dispatch_providers(parsed);
    let mut receipts = Vec::<Value>::new();
    for provider in providers {
        let receipt = match provider.as_str() {
            "pagerduty" => run_incident_dispatch_pagerduty(
                &mode,
                parsed,
                incident_id,
                runbook,
                requested_actions,
            ),
            "datadog" => run_incident_dispatch_datadog(
                &mode,
                parsed,
                incident_id,
                runbook,
                requested_actions,
            ),
            _ => json!({
                "provider": provider,
                "status": "skipped",
                "mode": mode,
                "reason": "unsupported_provider",
                "ts": crate::now_iso()
            }),
        };
        receipts.push(receipt);
    }
    let all_succeeded = receipts
        .iter()
        .all(|row| incident_dispatch_receipt_is_success(row, &mode));
    IncidentExternalDispatchResult {
        requested,
        mode,
        hard_fail: require_success && !all_succeeded,
        receipts,
    }
}

fn incident_external_dispatch_mode(parsed: &crate::ParsedArgs) -> String {
    let mode = clean(
        parsed
            .flags
            .get("external-dispatch-mode")
            .cloned()
            .unwrap_or_else(|| "live".to_string()),
        24,
    )
    .to_ascii_lowercase();
    match mode.as_str() {
        "live" | "dry-run" | "off" => mode,
        _ => "live".to_string(),
    }
}

fn incident_external_dispatch_providers(parsed: &crate::ParsedArgs) -> Vec<String> {
    let raw = parsed
        .flags
        .get("dispatch-providers")
        .cloned()
        .unwrap_or_else(|| "pagerduty+datadog".to_string());
    let mut out = Vec::<String>::new();
    for provider in split_actions(&raw) {
        if !matches!(provider.as_str(), "pagerduty" | "datadog") {
            continue;
        }
        if !out.iter().any(|existing| existing == &provider) {
            out.push(provider);
        }
    }
    if out.is_empty() {
        out.push("pagerduty".to_string());
        out.push("datadog".to_string());
    }
    out
}

fn incident_dispatch_receipt_is_success(receipt: &Value, mode: &str) -> bool {
    let status = receipt.get("status").and_then(Value::as_str).unwrap_or("");
    status == "success" || (mode == "dry-run" && status == "simulated")
}

fn run_incident_dispatch_pagerduty(
    mode: &str,
    parsed: &crate::ParsedArgs,
    incident_id: &str,
    runbook: &str,
    requested_actions: &[String],
) -> Value {
    let endpoint = incident_dispatch_endpoint(
        parsed,
        "pagerduty-events-url",
        "PAGERDUTY_EVENTS_URL",
        "https://events.pagerduty.com/v2/enqueue",
    );
    let routing_key =
        incident_dispatch_secret(parsed, "pagerduty-routing-key", "PAGERDUTY_ROUTING_KEY");
    let redacted_payload = json!({
        "routing_key": "[redacted]",
        "event_action": "trigger",
        "dedup_key": incident_id,
        "payload": {
            "summary": format!("InfRing incident {}", incident_id),
            "source": "infring-observability-plane",
            "severity": "error",
            "component": "observability_plane",
            "custom_details": {
                "runbook": runbook,
                "actions": requested_actions
            }
        }
    });
    let request_hash = sha256_hex_str(&redacted_payload.to_string());
    if mode == "off" {
        return json!({
            "provider": "pagerduty",
            "status": "skipped",
            "mode": mode,
            "reason": "dispatch_mode_off",
            "endpoint": endpoint,
            "request_hash": request_hash,
            "ts": crate::now_iso()
        });
    }
    let Some(routing_key) = routing_key else {
        return json!({
            "provider": "pagerduty",
            "status": "blocked",
            "mode": mode,
            "reason": "missing_pagerduty_routing_key",
            "endpoint": endpoint,
            "request_hash": request_hash,
            "ts": crate::now_iso()
        });
    };
    if mode == "dry-run" {
        return json!({
            "provider": "pagerduty",
            "status": "simulated",
            "mode": mode,
            "endpoint": endpoint,
            "request_hash": request_hash,
            "ts": crate::now_iso()
        });
    }
    let payload = json!({
        "routing_key": routing_key,
        "event_action": "trigger",
        "dedup_key": incident_id,
        "payload": {
            "summary": format!("InfRing incident {}", incident_id),
            "source": "infring-observability-plane",
            "severity": "error",
            "component": "observability_plane",
            "custom_details": {
                "runbook": runbook,
                "actions": requested_actions
            }
        }
    });
    match incident_dispatch_post_json(&endpoint, &[], &payload, 15) {
        Ok((http_status, body, latency_ms)) if (200..300).contains(&http_status) => json!({
            "provider": "pagerduty",
            "status": "success",
            "mode": mode,
            "endpoint": endpoint,
            "request_hash": request_hash,
            "http_status": http_status,
            "response_sha256": sha256_hex_str(&body),
            "latency_ms": latency_ms,
            "ts": crate::now_iso()
        }),
        Ok((http_status, body, latency_ms)) => json!({
            "provider": "pagerduty",
            "status": "failed",
            "mode": mode,
            "reason": "pagerduty_http_non_success",
            "endpoint": endpoint,
            "request_hash": request_hash,
            "http_status": http_status,
            "response_sha256": sha256_hex_str(&body),
            "latency_ms": latency_ms,
            "ts": crate::now_iso()
        }),
        Err(err) => json!({
            "provider": "pagerduty",
            "status": "failed",
            "mode": mode,
            "reason": "pagerduty_transport_error",
            "error": clean(err, 200),
            "endpoint": endpoint,
            "request_hash": request_hash,
            "ts": crate::now_iso()
        }),
    }
}

fn run_incident_dispatch_datadog(
    mode: &str,
    parsed: &crate::ParsedArgs,
    incident_id: &str,
    runbook: &str,
    requested_actions: &[String],
) -> Value {
    let endpoint = incident_datadog_events_url(parsed);
    let api_key = incident_dispatch_secret(parsed, "datadog-api-key", "DATADOG_API_KEY");
    let payload = json!({
        "title": format!("InfRing incident {}", incident_id),
        "text": format!(
            "runbook={} actions={}",
            runbook,
            requested_actions.join(",")
        ),
        "alert_type": "error",
        "source_type_name": "infring",
        "tags": [
            "source:infring-observability-plane",
            format!("incident_id:{}", incident_id),
            format!("runbook:{}", clean(runbook, 80))
        ]
    });
    let request_hash = sha256_hex_str(&payload.to_string());
    if mode == "off" {
        return json!({
            "provider": "datadog",
            "status": "skipped",
            "mode": mode,
            "reason": "dispatch_mode_off",
            "endpoint": endpoint,
            "request_hash": request_hash,
            "ts": crate::now_iso()
        });
    }
    let Some(api_key) = api_key else {
        return json!({
            "provider": "datadog",
            "status": "blocked",
            "mode": mode,
            "reason": "missing_datadog_api_key",
            "endpoint": endpoint,
            "request_hash": request_hash,
            "ts": crate::now_iso()
        });
    };
    if mode == "dry-run" {
        return json!({
            "provider": "datadog",
            "status": "simulated",
            "mode": mode,
            "endpoint": endpoint,
            "request_hash": request_hash,
            "ts": crate::now_iso()
        });
    }
    let headers = vec![format!("DD-API-KEY: {api_key}")];
    match incident_dispatch_post_json(&endpoint, &headers, &payload, 15) {
        Ok((http_status, body, latency_ms)) if (200..300).contains(&http_status) => json!({
            "provider": "datadog",
            "status": "success",
            "mode": mode,
            "endpoint": endpoint,
            "request_hash": request_hash,
            "http_status": http_status,
            "response_sha256": sha256_hex_str(&body),
            "latency_ms": latency_ms,
            "ts": crate::now_iso()
        }),
        Ok((http_status, body, latency_ms)) => json!({
            "provider": "datadog",
            "status": "failed",
            "mode": mode,
            "reason": "datadog_http_non_success",
            "endpoint": endpoint,
            "request_hash": request_hash,
            "http_status": http_status,
            "response_sha256": sha256_hex_str(&body),
            "latency_ms": latency_ms,
            "ts": crate::now_iso()
        }),
        Err(err) => json!({
            "provider": "datadog",
            "status": "failed",
            "mode": mode,
            "reason": "datadog_transport_error",
            "error": clean(err, 200),
            "endpoint": endpoint,
            "request_hash": request_hash,
            "ts": crate::now_iso()
        }),
    }
}

fn incident_dispatch_secret(
    parsed: &crate::ParsedArgs,
    flag_key: &str,
    env_key: &str,
) -> Option<String> {
    parsed
        .flags
        .get(flag_key)
        .cloned()
        .map(|row| clean(row, 200))
        .filter(|row| !row.is_empty())
        .or_else(|| {
            std::env::var(env_key)
                .ok()
                .map(|row| clean(row, 200))
                .filter(|row| !row.is_empty())
        })
}

fn incident_dispatch_endpoint(
    parsed: &crate::ParsedArgs,
    flag_key: &str,
    env_key: &str,
    fallback: &str,
) -> String {
    parsed
        .flags
        .get(flag_key)
        .cloned()
        .or_else(|| std::env::var(env_key).ok())
        .map(|row| clean(row, 240))
        .filter(|row| row.starts_with("https://"))
        .unwrap_or_else(|| fallback.to_string())
}

fn incident_datadog_events_url(parsed: &crate::ParsedArgs) -> String {
    if let Some(explicit) = parsed
        .flags
        .get("datadog-events-url")
        .cloned()
        .or_else(|| std::env::var("DATADOG_EVENTS_URL").ok())
        .map(|row| clean(row, 240))
        .filter(|row| row.starts_with("https://"))
    {
        return explicit;
    }
    let site = parsed
        .flags
        .get("datadog-site")
        .cloned()
        .or_else(|| std::env::var("DATADOG_SITE").ok())
        .unwrap_or_else(|| "datadoghq.com".to_string());
    let cleaned_site = site
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '.')
        .collect::<String>();
    let final_site = if cleaned_site.is_empty() {
        "datadoghq.com".to_string()
    } else {
        cleaned_site
    };
    format!("https://api.{final_site}/api/v1/events")
}

fn incident_dispatch_post_json(
    url: &str,
    headers: &[String],
    body: &Value,
    timeout_secs: u64,
) -> Result<(u16, String, u64), String> {
    let started = std::time::Instant::now();
    let mut cmd = std::process::Command::new("curl");
    cmd.arg("-sS")
        .arg("-X")
        .arg("POST")
        .arg("--max-time")
        .arg(timeout_secs.to_string())
        .arg("-H")
        .arg("Content-Type: application/json");
    for header in headers {
        cmd.arg("-H").arg(header);
    }
    cmd.arg("--data-binary")
        .arg(body.to_string())
        .arg("-w")
        .arg("\n__HTTP_STATUS__:%{http_code}")
        .arg(url);
    let output = cmd
        .output()
        .map_err(|err| format!("curl_spawn_failed:{err}"))?;
    let latency_ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let (body_out, status_raw) = stdout
        .rsplit_once("\n__HTTP_STATUS__:")
        .ok_or_else(|| "curl_http_status_missing".to_string())?;
    if !output.status.success() {
        let err = clean(stderr, 240);
        return if err.is_empty() {
            Err("curl_failed".to_string())
        } else {
            Err(format!("curl_failed:{err}"))
        };
    }
    let status = status_raw
        .trim()
        .parse::<u16>()
        .map_err(|_| "curl_http_status_invalid".to_string())?;
    Ok((status, body_out.to_string(), latency_ms))
}
