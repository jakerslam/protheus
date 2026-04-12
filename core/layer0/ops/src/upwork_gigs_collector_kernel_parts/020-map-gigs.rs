fn map_gigs(payload: &Map<String, Value>) -> Value {
    let max_items = clamp_u64(payload, "max_items", 10, 1, 200) as usize;
    let date = date_seed(payload);
    let collected_at = now_iso();
    let mut seen = normalize_seen_ids(payload);
    let rows = payload
        .get("gigs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut items = Vec::<Value>::new();
    for row in rows {
        if items.len() >= max_items {
            break;
        }
        let obj = match row.as_object() {
            Some(v) => v,
            None => continue,
        };
        let title = clean_text(obj.get("title").and_then(Value::as_str), 220);
        let url = canonical_upwork_gig_url(obj.get("url").and_then(Value::as_str));
        if title.is_empty() || url.is_empty() {
            continue;
        }
        let description = clean_text(obj.get("description").and_then(Value::as_str), 420);
        let budget = clean_text(obj.get("budget").and_then(Value::as_str), 120);
        let pub_date = clean_text(obj.get("pubDate").and_then(Value::as_str), 120);
        let id = sha16(&format!("gig-{title}-{url}-{date}"));
        if seen.contains(&id) {
            continue;
        }
        seen.insert(id.clone());

        let value_score = score_gig_value(&title, &description);
        let is_high_value = value_score >= 4;
        items.push(json!({
            "id": id,
            "collected_at": collected_at,
            "url": url,
            "title": title,
            "description": if description.is_empty() {
                Value::String(format!("Upwork gig value score: {value_score}"))
            } else {
                Value::String(description)
            },
            "budget": if budget.is_empty() { Value::Null } else { Value::String(budget) },
            "pubDate": if pub_date.is_empty() { Value::Null } else { Value::String(pub_date) },
            "value_score": value_score,
            "signal_type": if is_high_value { "high_value_gig" } else { "freelance_opportunity" },
            "signal": is_high_value,
            "source": "upwork_gigs",
            "tags": ["freelance", if is_high_value { "high-value" } else { "standard" }, "gig"],
            "topics": ["revenue", "freelance", "gigs", "opportunities"],
            "bytes": 0
        }));
    }

    let mut seen_ids = seen.into_iter().collect::<Vec<_>>();
    seen_ids.sort();
    if seen_ids.len() > 2000 {
        let drop = seen_ids.len() - 2000;
        seen_ids.drain(0..drop);
    }
    json!({
        "ok": true,
        "items": items,
        "seen_ids": seen_ids
    })
}

fn canonical_upwork_gig_url(raw: Option<&str>) -> String {
    let cleaned = clean_text(raw, 600);
    for prefix in [
        "https://www.upwork.com/",
        "https://upwork.com/",
        "http://www.upwork.com/",
        "http://upwork.com/",
    ] {
        if let Some(rest) = cleaned.strip_prefix(prefix) {
            return format!("https://www.upwork.com/{}", clean_text(Some(rest), 560));
        }
    }
    String::new()
}

fn fallback_gigs(payload: &Map<String, Value>) -> Value {
    let max_items = clamp_u64(payload, "max_items", 10, 1, 200) as usize;
    let date = date_seed(payload);
    let collected_at = now_iso();
    let mut seen = normalize_seen_ids(payload);
    let seed = [
        (
            "AI Automation Specialist - Workflow Optimization",
            "https://www.upwork.com/jobs/ai-automation-workflow",
            "Looking for expert to build AI agent workflows using n8n and OpenAI API. Budget: $5,000+",
            "$5,000+",
        ),
        (
            "Chrome Extension Developer - AI Assistant",
            "https://www.upwork.com/jobs/chrome-extension-ai",
            "Build browser extension that integrates with Claude API for content summarization. Budget: $2,000-$5,000",
            "$2,000-$5,000",
        ),
        (
            "No-Code SaaS MVP Builder",
            "https://www.upwork.com/jobs/nocode-saas-mvp",
            "Create functional MVP using Bubble or Webflow with database integration. Budget: $3,000-$8,000",
            "$3,000-$8,000",
        ),
    ];

    let mut items = Vec::<Value>::new();
    for (title, url, description, budget) in seed {
        if items.len() >= max_items {
            break;
        }
        let id = sha16(&format!("gig-{title}-{date}"));
        if seen.contains(&id) {
            continue;
        }
        seen.insert(id.clone());
        let value_score = score_gig_value(title, description);
        items.push(json!({
            "id": id,
            "collected_at": collected_at,
            "url": url,
            "title": format!("{title} — Freelance Opportunity"),
            "description": format!("{description} Value score: {value_score}. Fallback data."),
            "budget": budget,
            "value_score": value_score,
            "signal_type": "high_value_gig",
            "signal": true,
            "source": "upwork_gigs",
            "tags": ["freelance", "high-value", "gig", "fallback"],
            "topics": ["revenue", "freelance", "gigs"],
            "bytes": 0
        }));
    }

    let mut seen_ids = seen.into_iter().collect::<Vec<_>>();
    seen_ids.sort();
    if seen_ids.len() > 2000 {
        let drop = seen_ids.len() - 2000;
        seen_ids.drain(0..drop);
    }
    json!({
        "ok": true,
        "items": items,
        "seen_ids": seen_ids
    })
}

fn command_prepare_run(root: &Path, payload: &Map<String, Value>) -> Value {
    let force = as_bool(payload.get("force"), false);
    let min_hours = as_f64(payload.get("min_hours"), 4.0).clamp(0.0, 24.0 * 365.0);
    let meta_path = meta_path_for(root, payload);
    let meta = normalize_meta_value(Some(&read_json(&meta_path, normalize_meta_value(None))));
    let last_run_ms = meta
        .get("last_run")
        .and_then(Value::as_str)
        .and_then(parse_iso_ms);
    let hours_since_last =
        last_run_ms.map(|ms| ((Utc::now().timestamp_millis() - ms) as f64 / 3_600_000.0).max(0.0));
    let skipped = !force && hours_since_last.map(|h| h < min_hours).unwrap_or(false);
    json!({
        "ok": true,
        "collector_id": COLLECTOR_ID,
        "force": force,
        "min_hours": min_hours,
        "hours_since_last": hours_since_last,
        "skipped": skipped,
        "reason": if skipped { Value::String("cadence".to_string()) } else { Value::Null },
        "meta": meta,
        "meta_path": meta_path.display().to_string()
    })
}

fn command_build_fetch_plan(payload: &Map<String, Value>) -> Value {
    let query = clean_text(
        payload
            .get("search_query")
            .and_then(Value::as_str)
            .or_else(|| payload.get("q").and_then(Value::as_str)),
        240,
    );
    let query = if query.is_empty() {
        "automation OR ai OR nocode OR chatbot OR agent".to_string()
    } else {
        query
    };
    let encoded = encode_query_component(&query);
    json!({
        "ok": true,
        "collector_id": COLLECTOR_ID,
        "search_query": query,
        "requests": [
            {
                "key": "rss",
                "url": format!("https://www.upwork.com/ab/feed/jobs/rss?q={encoded}&sort=recency&paging=0-10"),
                "required": true,
                "accept": "application/rss+xml,application/xml,text/xml,*/*"
            }
        ]
    })
}

fn finalize_success(
    root: &Path,
    payload: &Map<String, Value>,
    min_hours: f64,
    max_items: usize,
    bytes: u64,
    requests: u64,
    duration_ms: u64,
) -> Result<Value, String> {
    let mut meta = normalize_meta_value(payload.get("meta"));
    let today = date_seed(payload);
    let initial_seen = meta
        .get("seen_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let gigs = payload
        .get("gigs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut degraded = false;
    let mapped = if !gigs.is_empty() {
        map_gigs(&json!({
            "date": today,
            "max_items": max_items,
            "seen_ids": initial_seen,
            "gigs": gigs
        }).as_object().cloned().unwrap_or_default())
    } else {
        degraded = true;
        fallback_gigs(&json!({
            "date": today,
            "max_items": max_items,
            "seen_ids": initial_seen
        }).as_object().cloned().unwrap_or_default())
    };

    let items = mapped
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let seen_ids = mapped
        .get("seen_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    meta["seen_ids"] = Value::Array(seen_ids);
    meta["last_run"] = Value::String(now_iso());
    if !items.is_empty() {
        meta["last_success"] = Value::String(now_iso());
        write_json_atomic(&cache_path_for(root, payload), &json!({ "items": items }))?;
    }
    write_json_atomic(&meta_path_for(root, payload), &meta)?;

    let sample = items
        .first()
        .and_then(Value::as_object)
        .and_then(|o| o.get("title"))
        .and_then(Value::as_str)
        .map(|s| clean_text(Some(s), 80))
        .filter(|s| !s.is_empty())
        .map(Value::String)
        .unwrap_or(Value::Null);

    Ok(json!({
        "ok": true,
        "success": true,
        "eye": COLLECTOR_ID,
        "items": items,
        "bytes": bytes,
        "duration_ms": duration_ms,
        "requests": requests.max(1),
        "cadence_hours": min_hours,
        "degraded": degraded,
        "sample": sample
    }))
}

fn finalize_error(
    root: &Path,
    payload: &Map<String, Value>,
    min_hours: f64,
    max_items: usize,
    bytes: u64,
    requests: u64,
    duration_ms: u64,
    error: &str,
) -> Result<Value, String> {
    let mut meta = normalize_meta_value(payload.get("meta"));
    let cached = load_cache_items(root, payload);
    if !cached.is_empty() {
        return Ok(json!({
            "ok": true,
            "success": true,
            "eye": COLLECTOR_ID,
            "cache_hit": true,
            "degraded": true,
            "error": clean_text(Some(error), 120),
            "items": cached.into_iter().take(max_items).collect::<Vec<_>>(),
            "bytes": bytes,
            "requests": requests,
            "duration_ms": duration_ms,
            "cadence_hours": min_hours
        }));
    }

    let initial_seen = meta
        .get("seen_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let fallback = fallback_gigs(&json!({
        "date": date_seed(payload),
        "max_items": max_items,
        "seen_ids": initial_seen
    }).as_object().cloned().unwrap_or_default());
    let fallback_items = fallback
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    meta["last_run"] = Value::String(now_iso());
    if let Some(seen_ids) = fallback.get("seen_ids").and_then(Value::as_array) {
        meta["seen_ids"] = Value::Array(seen_ids.clone());
    }
    write_json_atomic(&meta_path_for(root, payload), &meta)?;

    let sample = fallback_items
        .first()
        .and_then(Value::as_object)
        .and_then(|o| o.get("title"))
        .and_then(Value::as_str)
        .map(|s| clean_text(Some(s), 80))
        .filter(|s| !s.is_empty())
        .map(Value::String)
        .unwrap_or(Value::Null);

    Ok(json!({
        "ok": true,
        "success": true,
        "eye": COLLECTOR_ID,
        "items": fallback_items,
        "bytes": bytes,
        "duration_ms": duration_ms,
        "requests": requests.max(1),
        "cadence_hours": min_hours,
        "degraded": true,
        "error": clean_text(Some(error), 120),
        "sample": sample
    }))
}

fn command_finalize_run(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let min_hours = as_f64(payload.get("min_hours"), 4.0).clamp(0.0, 24.0 * 365.0);
    let max_items = clamp_u64(payload, "max_items", 10, 1, 200) as usize;
    let bytes = clamp_u64(payload, "bytes", 0, 0, u64::MAX);
    let requests = clamp_u64(payload, "requests", 0, 0, u64::MAX);
    let duration_ms = clamp_u64(payload, "duration_ms", 0, 0, u64::MAX);
    let fetch_error = clean_text(payload.get("fetch_error").and_then(Value::as_str), 220);
    if !fetch_error.is_empty() {
        return finalize_error(
            root,
            payload,
            min_hours,
            max_items,
            bytes,
            requests,
            duration_ms,
            &fetch_error,
        );
    }
    finalize_success(root, payload, min_hours, max_items, bytes, requests, duration_ms)
}

fn command_collect(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let min_hours = as_f64(payload.get("min_hours"), 4.0).clamp(0.0, 24.0 * 365.0);
    let max_items = clamp_u64(payload, "max_items", 10, 1, 200) as usize;
    let force = as_bool(payload.get("force"), false);
    let prepared = command_prepare_run(
        root,
        &json!({
            "eyes_state_dir": payload.get("eyes_state_dir").cloned().unwrap_or(Value::Null),
            "force": force,
            "min_hours": min_hours
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    );

    if prepared.get("skipped").and_then(Value::as_bool) == Some(true) {
        return Ok(json!({
            "ok": true,
            "success": true,
            "eye": COLLECTOR_ID,
            "skipped": true,
            "reason": "cadence",
            "hours_since_last": prepared.get("hours_since_last").cloned().unwrap_or(Value::Null),
            "min_hours": min_hours,
            "items": []
        }));
    }

    let rss_xml = payload
        .get("rss_xml")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let gigs = if rss_xml.trim().is_empty() {
        Vec::<Value>::new()
    } else {
        parse_rss(rss_xml.as_str())
    };
    let bytes = clamp_u64(payload, "bytes", 0, 0, u64::MAX);
    let requests = clamp_u64(payload, "requests", 0, 0, u64::MAX);
    let duration_ms = clamp_u64(payload, "duration_ms", 0, 0, u64::MAX);
    let fetch_error = clean_text(payload.get("fetch_error").and_then(Value::as_str), 220);

    command_finalize_run(
        root,
        &json!({
            "eyes_state_dir": payload.get("eyes_state_dir").cloned().unwrap_or(Value::Null),
            "meta": prepared.get("meta").cloned().unwrap_or_else(|| normalize_meta_value(None)),
            "min_hours": min_hours,
            "max_items": max_items,
            "bytes": bytes,
            "requests": requests,
            "duration_ms": duration_ms,
            "gigs": gigs,
            "fetch_error": if fetch_error.is_empty() { Value::Null } else { Value::String(fetch_error) }
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    )
}
