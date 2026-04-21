
fn preflight(payload: &Map<String, Value>) -> Value {
    let mut checks = Vec::<Value>::new();
    let mut failures = Vec::<Value>::new();
    let max_items = clamp_u64(payload, "max_items", 20, 0, 200);
    let secret_present = payload
        .get("secret_present")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let host = resolved_host(payload);
    let requested = requested_host(payload);
    let api_base_host = host_from_urlish(&resolve_api_base(payload));

    if !secret_present {
        failures.push(json!({
            "code": "auth_missing",
            "message": "missing_moltbook_api_key"
        }));
    } else {
        checks.push(json!({
            "name": "api_key_present",
            "ok": true
        }));
    }

    if max_items == 0 {
        failures.push(json!({
            "code": "invalid_budget",
            "message": "budgets.max_items must be > 0"
        }));
    } else {
        checks.push(json!({
            "name": "max_items_valid",
            "ok": true,
            "value": max_items
        }));
    }

    if !requested.is_empty() && !api_base_host.is_empty() && requested != api_base_host {
        failures.push(json!({
            "code": "api_base_host_mismatch",
            "message": format!("api_base resolves to {api_base_host} not {requested}")
        }));
    }

    let allowlist = parse_allowlist(payload);
    if allowlist.is_empty() || !host_allowed(&host, &allowlist) {
        failures.push(json!({
            "code": "domain_not_allowlisted",
            "message": format!("collector host not allowlisted: {host}")
        }));
    } else {
        checks.push(json!({
            "name": "allowlisted_host",
            "ok": true,
            "host": host
        }));
    }

    json!({
        "ok": failures.is_empty(),
        "parser_type": "moltbook_hot",
        "checks": checks,
        "failures": failures
    })
}

fn classify_fetch_error(payload: &Map<String, Value>) -> Value {
    let code =
        clean_text(payload.get("error_code").and_then(Value::as_str), 80).to_ascii_lowercase();
    let fallback_codes = [
        "dns_unreachable",
        "connection_refused",
        "connection_reset",
        "timeout",
        "tls_error",
        "network_error",
        "http_5xx",
        "rate_limited",
        "env_blocked",
    ];
    json!({
        "ok": true,
        "error_code": code,
        "fallback_allowed": fallback_codes.contains(&code.as_str())
    })
}

fn map_posts(payload: &Map<String, Value>) -> Value {
    let max_items = clamp_u64(payload, "max_items", 20, 1, 50) as usize;
    let topics = normalize_topics(payload);
    let posts = extract_posts(payload);
    let mut items = Vec::<Value>::new();
    for post in posts {
        if items.len() >= max_items {
            break;
        }
        let obj = match post.as_object() {
            Some(v) => v,
            None => continue,
        };
        let title = clean_text(obj.get("title").and_then(Value::as_str), 200);
        let pid = post_id(obj);
        let url = post_url(obj, &pid);
        if title.is_empty() || url.is_empty() {
            continue;
        }
        let id = if pid.is_empty() { sha16(&url) } else { pid };
        items.push(json!({
            "collected_at": now_iso(),
            "id": id,
            "url": url.clone(),
            "title": title,
            "topics": topics,
            "bytes": std::cmp::min(1024_usize, title.len() + url.len() + 64)
        }));
    }
    json!({
        "ok": true,
        "items": items
    })
}

fn command_collect(payload: &Map<String, Value>) -> Result<Value, String> {
    let pre = preflight(payload);
    if pre.get("ok").and_then(Value::as_bool) != Some(true) {
        return Err(preflight_error(&pre));
    }

    let fetch_error = clean_text(payload.get("fetch_error").and_then(Value::as_str), 80);
    if !fetch_error.is_empty() {
        let policy = classify_fetch_error(
            &json!({
                "error_code": fetch_error
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        return Ok(json!({
            "ok": true,
            "success": false,
            "fallback_allowed": policy.get("fallback_allowed").cloned().unwrap_or(Value::Bool(false)),
            "error_code": clean_text(policy.get("error_code").and_then(Value::as_str), 80)
        }));
    }

    let mapped = map_posts(
        &json!({
            "max_items": payload.get("max_items").cloned().unwrap_or(Value::from(20)),
            "topics": payload.get("topics").cloned().unwrap_or(Value::Array(Vec::new())),
            "posts": payload.get("posts").cloned().unwrap_or(Value::Null)
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    );
    let items = mapped
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(json!({
        "ok": true,
        "success": true,
        "items": items
    }))
}

fn command_run(payload: &Map<String, Value>) -> Result<Value, String> {
    let pre = preflight(payload);
    if pre.get("ok").and_then(Value::as_bool) != Some(true) {
        return Err(preflight_error(&pre));
    }
    let max_items = clamp_u64(payload, "max_items", 20, 1, 50);
    let started_at_ms = Utc::now().timestamp_millis().max(0) as u64;
    let timeout_ms = payload
        .get("timeout_ms")
        .and_then(Value::as_u64)
        .or_else(|| {
            std::env::var("MOLTBOOK_HTTP_TIMEOUT_MS")
                .ok()
                .and_then(|v| v.trim().parse::<u64>().ok())
        })
        .unwrap_or(12_000)
        .clamp(2_000, 30_000);
    let api_base = resolve_api_base(payload);
    let fetch_url = format!("{api_base}/v1/posts/hot?limit={max_items}");
    let auth = auth_headers(payload);

    let (posts, bytes, requests, fetch_error) =
        match curl_fetch_with_status(&fetch_url, timeout_ms, &auth, "application/json") {
            Ok((status, body, body_bytes)) => {
                if status >= 400 {
                    (
                        Value::Null,
                        0_u64,
                        0_u64,
                        Some(http_status_to_code(status).to_string()),
                    )
                } else {
                    (parse_json_or_null(&body), body_bytes, 1_u64, None)
                }
            }
            Err(err) => {
                let code = clean_text(Some(&err), 120)
                    .split(':')
                    .next()
                    .unwrap_or("collector_error")
                    .to_string();
                (Value::Null, 0_u64, 0_u64, Some(code))
            }
        };

    let duration_ms = Utc::now()
        .timestamp_millis()
        .max(0)
        .saturating_sub(started_at_ms as i64) as u64;

    let mut out = command_collect(
        &json!({
            "secret_present": payload.get("secret_present").cloned().unwrap_or(Value::Bool(false)),
            "host": Value::String(resolved_host(payload)),
            "allowed_domains": payload.get("allowed_domains").cloned().unwrap_or(Value::Array(Vec::new())),
            "max_items": max_items,
            "posts": posts,
            "fetch_error": fetch_error,
            "topics": payload.get("topics").cloned().unwrap_or(Value::Array(Vec::new()))
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    )?;
    if let Some(obj) = out.as_object_mut() {
        obj.insert("bytes".to_string(), Value::from(bytes));
        obj.insert("requests".to_string(), Value::from(requests));
        obj.insert("duration_ms".to_string(), Value::from(duration_ms));
    }
    Ok(out)
}

fn dispatch(command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command { "run" => command_run(payload), "preflight" => Ok(preflight(payload)), "classify-fetch-error" => Ok(classify_fetch_error(payload)), "map-posts" => Ok(map_posts(payload)), "collect" => command_collect(payload), _ => Err("moltbook_hot_collector_kernel_unknown_command".to_string()) }
}
