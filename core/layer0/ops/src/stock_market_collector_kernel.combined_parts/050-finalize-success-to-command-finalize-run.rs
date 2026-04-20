
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
    let quotes = payload
        .get("quotes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut degraded = false;
    let mapped = if !quotes.is_empty() {
        map_quotes(
            &json!({
                "date": today,
                "max_items": max_items,
                "seen_ids": initial_seen,
                "quotes": quotes
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        )
    } else {
        degraded = true;
        fallback_indices(
            &json!({
                "date": today,
                "max_items": max_items,
                "seen_ids": initial_seen
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        )
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
        .and_then(|o| o.get("symbol"))
        .and_then(Value::as_str)
        .map(|s| clean_text(Some(s), 64))
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
    let fallback = fallback_indices(
        &json!({
            "date": date_seed(payload),
            "max_items": max_items,
            "seen_ids": initial_seen
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    );
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
        .and_then(|o| o.get("symbol"))
        .and_then(Value::as_str)
        .map(|s| clean_text(Some(s), 64))
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
    let min_hours = as_f64(payload.get("min_hours"), 1.0).clamp(0.0, 24.0 * 365.0);
    let max_items = clamp_u64(payload, "max_items", 20, 1, 200) as usize;
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
    finalize_success(
        root,
        payload,
        min_hours,
        max_items,
        bytes,
        requests,
        duration_ms,
    )
}
