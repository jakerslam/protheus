
fn command_map_results(payload: &Map<String, Value>) -> Value {
    let max_items = support::clamp_u64(payload, "max_items", 10, 1, 200) as usize;
    let collector_id = support::clean_text(payload.get("collector_id").and_then(Value::as_str), 64);
    let collector_id = if collector_id.is_empty() {
        support::COLLECTOR_ID.to_string()
    } else {
        collector_id
    };

    let results = payload
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut seen = support::normalize_seen_ids(payload);
    let mut items = Vec::<Value>::new();
    for row in results {
        if items.len() >= max_items {
            break;
        }
        let obj = match row.as_object() {
            Some(v) => v,
            None => continue,
        };
        let tweet_id = support::clean_text(
            obj.get("id")
                .and_then(Value::as_str)
                .or_else(|| obj.get("tweet_id").and_then(Value::as_str)),
            120,
        );
        if tweet_id.is_empty() || seen.contains(&tweet_id) {
            continue;
        }

        let content = support::clean_text(
            obj.get("text")
                .and_then(Value::as_str)
                .or_else(|| obj.get("content").and_then(Value::as_str)),
            1200,
        );
        let (author_handle, author_name) = support::extract_author_parts(obj);
        let title = support::first_line_title(&content, &author_handle);
        let likes = support::as_i64(obj, &["likes", "favorite_count"]);
        let retweets = support::as_i64(obj, &["retweets", "retweet_count"]);
        let url = if author_handle.is_empty() || author_handle == "unknown" {
            format!("https://x.com/i/web/status/{tweet_id}")
        } else {
            format!("https://x.com/{author_handle}/status/{tweet_id}")
        };
        let topics = support::infer_topics(&content)
            .into_iter()
            .map(Value::String)
            .collect::<Vec<_>>();
        let id = support::sha16(&format!(
            "{}|{}",
            tweet_id,
            support::clean_text(Some(&content), 200)
        ));
        let tags = vec![
            Value::String(support::clean_text(Some(&author_name), 120)),
            Value::String(format!("likes:{likes}")),
            Value::String(format!("rt:{retweets}")),
        ];
        let item = json!({
            "collected_at": support::now_iso(),
            "eye_id": collector_id,
            "id": id,
            "tweet_id": tweet_id,
            "title": title,
            "description": content,
            "url": url,
            "author": author_handle,
            "tags": tags,
            "topics": topics,
            "bytes": std::cmp::min(4096_usize, title.len() + support::clean_text(obj.get("text").and_then(Value::as_str), 1200).len() + 160)
        });
        items.push(item);
        seen.insert(support::clean_seen_id(&tweet_id));
    }

    let mut seen_ids = seen.into_iter().collect::<Vec<_>>();
    seen_ids.sort();
    if seen_ids.len() > 4000 {
        let drop = seen_ids.len() - 4000;
        seen_ids.drain(0..drop);
    }

    json!({
        "ok": true,
        "items": items,
        "seen_ids": seen_ids
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
    let mut meta = support::normalize_meta_value(payload.get("meta"));
    let items = payload
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut seen_ids = payload
        .get("seen_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if seen_ids.len() > 2000 {
        let drop = seen_ids.len() - 2000;
        seen_ids = seen_ids.into_iter().skip(drop).collect::<Vec<_>>();
    }

    meta["seen_ids"] = Value::Array(seen_ids);
    meta["last_run"] = Value::String(support::now_iso());
    if !items.is_empty() {
        meta["last_success"] = Value::String(support::now_iso());
        support::write_json_atomic(
            &support::cache_path_for(root, payload),
            &json!({ "items": items }),
        )?;
    }
    support::write_json_atomic(&support::meta_path_for(root, payload), &meta)?;

    Ok(json!({
        "ok": true,
        "success": true,
        "eye": support::COLLECTOR_ID,
        "items": items.into_iter().take(max_items).collect::<Vec<_>>(),
        "bytes": bytes,
        "duration_ms": duration_ms,
        "requests": requests,
        "cadence_hours": min_hours
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
) -> Result<Value, String> {
    let mut meta = support::normalize_meta_value(payload.get("meta"));
    meta["last_run"] = Value::String(support::now_iso());
    support::write_json_atomic(&support::meta_path_for(root, payload), &meta)?;

    let failures = support::query_failures(payload);
    if !failures.is_empty() {
        let cached = support::load_cache_items(root, payload);
        if !cached.is_empty() {
            return Ok(json!({
                "ok": true,
                "success": true,
                "eye": support::COLLECTOR_ID,
                "cache_hit": true,
                "degraded": true,
                "items": cached.into_iter().take(max_items).collect::<Vec<_>>(),
                "bytes": bytes,
                "duration_ms": duration_ms,
                "requests": requests,
                "failure_count": failures.len(),
                "failures": failures.into_iter().take(3).collect::<Vec<_>>(),
                "cadence_hours": min_hours
            }));
        }

        let primary = failures
            .first()
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let code = support::clean_text(primary.get("code").and_then(Value::as_str), 80);
        let message = support::clean_text(primary.get("message").and_then(Value::as_str), 220);
        let http_status = primary.get("http_status").and_then(Value::as_i64);
        return Ok(json!({
            "ok": false,
            "success": false,
            "eye": support::COLLECTOR_ID,
            "items": [],
            "bytes": bytes,
            "duration_ms": duration_ms,
            "requests": requests,
            "error": if message.is_empty() { Value::String("bird_x_all_queries_failed".to_string()) } else { Value::String(message) },
            "error_code": if code.is_empty() { Value::String("collector_error".to_string()) } else { Value::String(code) },
            "error_http_status": http_status,
            "failure_count": failures.len(),
            "failures": failures.into_iter().take(3).collect::<Vec<_>>(),
            "cadence_hours": min_hours
        }));
    }

    Ok(json!({
        "ok": false,
        "success": false,
        "eye": support::COLLECTOR_ID,
        "items": [],
        "bytes": bytes,
        "duration_ms": duration_ms,
        "requests": requests,
        "error": "bird_x_no_results",
        "error_code": "collector_error",
        "error_http_status": Value::Null,
        "cadence_hours": min_hours
    }))
}

fn command_finalize_run(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let min_hours = support::as_f64(payload.get("min_hours"), 0.0).clamp(0.0, 24.0 * 365.0);
    let max_items = support::clamp_u64(payload, "max_items", 15, 1, 200) as usize;
    let bytes = support::clamp_u64(payload, "bytes", 0, 0, u64::MAX);
    let requests = support::clamp_u64(payload, "requests", 0, 0, u64::MAX);
    let duration_ms = support::clamp_u64(payload, "duration_ms", 0, 0, u64::MAX);
    let items = payload
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !items.is_empty() {
        return finalize_success(
            root,
            payload,
            min_hours,
            max_items,
            bytes,
            requests,
            duration_ms,
        );
    }
    finalize_error(
        root,
        payload,
        min_hours,
        max_items,
        bytes,
        requests,
        duration_ms,
    )
}

fn dispatch(root: &Path, command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "preflight" => Ok(command_preflight(payload)),
        "prepare-run" => Ok(command_prepare_run(root, payload)),
        "map-results" => Ok(command_map_results(payload)),
        "finalize-run" => command_finalize_run(root, payload),
        "collect" => command_collect(root, payload),
        _ => Err("bird_x_collector_kernel_unknown_command".to_string()),
    }
}
