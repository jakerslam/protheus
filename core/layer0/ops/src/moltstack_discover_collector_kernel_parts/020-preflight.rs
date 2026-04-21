
fn preflight(payload: &Map<String, Value>) -> Value {
    let mut checks = Vec::<Value>::new();
    let mut failures = Vec::<Value>::new();
    let url = resolved_api_url(payload);
    let max_items = payload
        .get("max_items")
        .and_then(Value::as_u64)
        .unwrap_or(20);
    let max_seconds = payload
        .get("max_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(10);

    match split_scheme_host(&url) {
        Some((scheme, host)) => {
            if scheme != "https" {
                failures.push(json!({
                    "code": "invalid_config",
                    "message": format!("URL must use https: {url}")
                }));
            }
            let allowlist = parse_allowlist(payload);
            let allowed = if allowlist.is_empty() {
                host == "moltstack.net" || host.ends_with(".moltstack.net")
            } else {
                allowlist
                    .iter()
                    .any(|d| host == *d || host.ends_with(&format!(".{d}")))
            };
            if !allowed {
                failures.push(json!({
                    "code": "domain_not_allowlisted",
                    "message": format!("host not allowlisted: {host}")
                }));
            } else {
                checks.push(json!({
                    "name": "allowlisted_url",
                    "ok": true,
                    "host": host
                }));
            }
        }
        None => failures.push(json!({
            "code": "invalid_config",
            "message": format!("Invalid URL: {url}")
        })),
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

    if max_seconds == 0 {
        failures.push(json!({
            "code": "invalid_budget",
            "message": "budgets.max_seconds must be > 0"
        }));
    } else {
        checks.push(json!({
            "name": "max_seconds_valid",
            "ok": true,
            "value": max_seconds
        }));
    }

    json!({
        "ok": failures.is_empty(),
        "parser_type": "moltstack_discover",
        "checks": checks,
        "failures": failures
    })
}

fn extract_posts(raw: &Value) -> Vec<Value> {
    if let Some(rows) = raw.as_array() {
        return rows.clone();
    }
    if let Some(rows) = raw.get("posts").and_then(Value::as_array) {
        return rows.clone();
    }
    raw.get("data")
        .and_then(|d| d.get("posts"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn map_posts(payload: &Map<String, Value>) -> Value {
    let max_items = clamp_u64(payload, "max_items", 20, 1, 200) as usize;
    let topics_cfg = configured_topics(payload);
    let posts = extract_posts(payload.get("posts").unwrap_or(&Value::Null));
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
        let slug = clean_text(obj.get("slug").and_then(Value::as_str), 120);
        let agent_slug = clean_text(
            obj.get("agent")
                .and_then(Value::as_object)
                .and_then(|a| a.get("slug"))
                .and_then(Value::as_str),
            120,
        );
        if title.is_empty() || slug.is_empty() {
            continue;
        }
        let explicit_url = clean_text(obj.get("url").and_then(Value::as_str), 600);
        let url = if !explicit_url.is_empty() {
            explicit_url
        } else if !agent_slug.is_empty() {
            format!("https://moltstack.net/{agent_slug}/{slug}")
        } else {
            format!("https://moltstack.net/discover/{slug}")
        };
        if host_from_url(&url).is_none() {
            continue;
        }
        let url_len = url.len();
        let topics = keyword_topics(&title, &topics_cfg)
            .into_iter()
            .map(Value::String)
            .collect::<Vec<_>>();
        items.push(json!({
            "collected_at": now_iso(),
            "id": sha16(&url),
            "url": url,
            "title": title,
            "topics": topics,
            "bytes": std::cmp::min(512_usize, title.len() + 64 + url_len)
        }));
    }
    json!({
        "ok": true,
        "items": items
    })
}

fn build_fetch_plan(payload: &Map<String, Value>) -> Value {
    let max_items = clamp_u64(payload, "max_items", 20, 1, 50);
    let max_seconds = clamp_u64(payload, "max_seconds", 10, 1, 30);
    let timeout_ms = (max_seconds.saturating_mul(1000)).min(15_000);
    let url = resolved_api_url(payload);
    json!({
        "ok": true,
        "max_items": max_items,
        "timeout_ms": timeout_ms,
        "requests": [
            {
                "key": "posts_json",
                "url": url,
                "required": true,
                "accept": "application/json"
            }
        ]
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
    let fallback_allowed = fallback_codes.contains(&code.as_str());
    json!({
        "ok": true,
        "error_code": code,
        "fallback_allowed": fallback_allowed
    })
}

fn finalize_run(payload: &Map<String, Value>) -> Value {
    let max_items = clamp_u64(payload, "max_items", 20, 1, 50);
    let topics = payload
        .get("topics")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let posts = payload.get("posts").cloned().unwrap_or(Value::Null);
    map_posts(&object_from_value(json!({
        "max_items": max_items,
        "topics": topics,
        "posts": posts
    })))
}

fn command_collect(payload: &Map<String, Value>) -> Result<Value, String> {
    let pre = preflight(payload);
    if pre.get("ok").and_then(Value::as_bool) != Some(true) {
        return Err(first_preflight_error(&pre));
    }

    let fetch_error = clean_text(payload.get("fetch_error").and_then(Value::as_str), 80);
    if !fetch_error.is_empty() {
        let policy = classify_fetch_error(&object_from_value(json!({
            "error_code": fetch_error
        })));
        return Ok(json!({
            "ok": true,
            "success": false,
            "fallback_allowed": policy.get("fallback_allowed").cloned().unwrap_or(Value::Bool(false)),
            "error_code": clean_text(policy.get("error_code").and_then(Value::as_str), 80)
        }));
    }

    let mapped = finalize_run(&object_from_value(json!({
        "max_items": payload.get("max_items").cloned().unwrap_or(Value::from(20)),
        "topics": payload.get("topics").cloned().unwrap_or(Value::Array(Vec::new())),
        "posts": payload.get("posts_json").cloned().unwrap_or_else(|| payload.get("posts").cloned().unwrap_or(Value::Null))
    })));
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
