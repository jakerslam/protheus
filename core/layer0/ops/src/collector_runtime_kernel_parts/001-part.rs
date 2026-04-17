        .map(Value::is_object)
        .unwrap_or(false)
    {
        collectors.insert(
            collector_id.to_string(),
            json!({
                "last_attempt_ms": 0,
                "last_success_ms": 0,
                "failure_streak": 0,
                "next_allowed_ms": 0,
                "circuit_open_until_ms": 0,
                "last_error_code": null
            }),
        );
    }
    collectors
        .get_mut(collector_id)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "collector_runtime_kernel_row_not_object".to_string())
}

fn row_u64(row: &Map<String, Value>, key: &str) -> u64 {
    row.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn set_row_u64(row: &mut Map<String, Value>, key: &str, value: u64) {
    row.insert(key.to_string(), Value::Number(value.into()));
}

fn render_row(row: &Map<String, Value>) -> Value {
    json!({
        "last_attempt_ms": row_u64(row, "last_attempt_ms"),
        "last_success_ms": row_u64(row, "last_success_ms"),
        "failure_streak": row_u64(row, "failure_streak"),
        "next_allowed_ms": row_u64(row, "next_allowed_ms"),
        "circuit_open_until_ms": row_u64(row, "circuit_open_until_ms"),
        "last_error_code": row.get("last_error_code").cloned().unwrap_or(Value::Null)
    })
}

fn handle_prepare_attempt(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let collector_id = clean_collector_id(payload);
    let min_interval_ms = json_u64(
        payload,
        "min_interval_ms",
        default_u64_from_env("EYES_COLLECTOR_MIN_INTERVAL_MS", 300, 50, 30_000),
        50,
        30_000,
    );
    let state_path = resolve_rate_state_path(root, payload);

    let mut state = read_state(&state_path);
    let collectors = ensure_collectors_mut(&mut state)?;
    let row = ensure_row_mut(collectors, &collector_id)?;

    let now = now_ms_u64();
    let circuit_open_until_ms = row_u64(row, "circuit_open_until_ms");
    if circuit_open_until_ms > now {
        return Ok(json!({
            "ok": true,
            "collector_id": collector_id,
            "circuit_open": true,
            "retry_after_ms": circuit_open_until_ms.saturating_sub(now),
            "row": render_row(row),
            "rate_state_path": state_path.display().to_string()
        }));
    }

    let last_attempt_ms = row_u64(row, "last_attempt_ms");
    let next_allowed_ms = row_u64(row, "next_allowed_ms");
    let ready_at = max(
        next_allowed_ms,
        last_attempt_ms.saturating_add(min_interval_ms),
    );
    let wait_ms = ready_at.saturating_sub(now);
    if wait_ms > 0 {
        thread::sleep(Duration::from_millis(wait_ms));
    }

    let attempted_at_ms = now_ms_u64();
    set_row_u64(row, "last_attempt_ms", attempted_at_ms);
    let row_snapshot = render_row(row);
    write_state(&state_path, &state)?;

    Ok(json!({
        "ok": true,
        "collector_id": collector_id,
        "circuit_open": false,
        "wait_ms": wait_ms,
        "attempted_at_ms": attempted_at_ms,
        "row": row_snapshot,
        "rate_state_path": state_path.display().to_string()
    }))
}

fn handle_mark_success(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let collector_id = clean_collector_id(payload);
    let min_interval_ms = json_u64(
        payload,
        "min_interval_ms",
        default_u64_from_env("EYES_COLLECTOR_MIN_INTERVAL_MS", 300, 50, 30_000),
        50,
        30_000,
    );
    let state_path = resolve_rate_state_path(root, payload);

    let mut state = read_state(&state_path);
    let collectors = ensure_collectors_mut(&mut state)?;
    let row = ensure_row_mut(collectors, &collector_id)?;
    let now = now_ms_u64();

    set_row_u64(row, "last_success_ms", now);
    set_row_u64(row, "failure_streak", 0);
    set_row_u64(row, "next_allowed_ms", now.saturating_add(min_interval_ms));
    set_row_u64(row, "circuit_open_until_ms", 0);
    row.insert("last_error_code".to_string(), Value::Null);
    let row_snapshot = render_row(row);

    write_state(&state_path, &state)?;

    Ok(json!({
        "ok": true,
        "collector_id": collector_id,
        "row": row_snapshot,
        "rate_state_path": state_path.display().to_string()
    }))
}

fn handle_mark_failure(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let collector_id = clean_collector_id(payload);
    let last_error_code = lane_utils::clean_token(
        payload.get("code").and_then(Value::as_str),
        "collector_error",
    );
    let retryable = is_retryable_code(&last_error_code);

    let base_backoff_ms = json_u64(
        payload,
        "base_backoff_ms",
        default_u64_from_env("EYES_COLLECTOR_BACKOFF_BASE_MS", 300, 50, 30_000),
        50,
        30_000,
    );
    let max_backoff_ms = json_u64(
        payload,
        "max_backoff_ms",
        default_u64_from_env("EYES_COLLECTOR_BACKOFF_MAX_MS", 8_000, 200, 120_000),
        200,
        120_000,
    );
    let circuit_open_ms = json_u64(
        payload,
        "circuit_open_ms",
        default_u64_from_env("EYES_COLLECTOR_CIRCUIT_MS", 30_000, 500, 300_000),
        500,
        300_000,
    );
    let circuit_after_failures = json_u64(
        payload,
        "circuit_after_failures",
        default_u64_from_env("EYES_COLLECTOR_CIRCUIT_AFTER", 3, 1, 10),
        1,
        10,
    );

    let state_path = resolve_rate_state_path(root, payload);

    let mut state = read_state(&state_path);
    let collectors = ensure_collectors_mut(&mut state)?;
    let row = ensure_row_mut(collectors, &collector_id)?;
    let now = now_ms_u64();

    let next_failure_streak = row_u64(row, "failure_streak").saturating_add(1);
    set_row_u64(row, "failure_streak", next_failure_streak);

    if retryable {
        let exp = next_failure_streak.saturating_sub(1).min(16);
        let backoff_ms = std::cmp::min(
            max_backoff_ms,
            base_backoff_ms.saturating_mul(2_u64.pow(exp as u32)),
        );
        set_row_u64(row, "next_allowed_ms", now.saturating_add(backoff_ms));
        if next_failure_streak >= circuit_after_failures {
            set_row_u64(
                row,
                "circuit_open_until_ms",
                now.saturating_add(circuit_open_ms),
            );
        }
    } else {
        set_row_u64(
            row,
            "next_allowed_ms",
            now.saturating_add(std::cmp::min(max_backoff_ms, base_backoff_ms)),
        );
    }

    row.insert(
        "last_error_code".to_string(),
        Value::String(last_error_code.clone()),
    );
    let row_snapshot = render_row(row);

    write_state(&state_path, &state)?;

    Ok(json!({
        "ok": true,
        "collector_id": collector_id,
        "retryable": retryable,
        "last_error_code": last_error_code,
        "row": row_snapshot,
        "rate_state_path": state_path.display().to_string()
    }))
}

fn handle_prepare_run(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let collector_id = clean_collector_id(payload);
    let force = json_bool(payload, "force", false);
    let min_hours = json_f64(payload, "min_hours", 4.0, 0.0, 24.0 * 365.0);

    let meta_path = meta_path_for(root, payload, &collector_id);
    let meta = normalize_meta_value(
        &collector_id,
        Some(&read_json(
            &meta_path,
            normalize_meta_value(&collector_id, None),
        )),
    );
    let last_run_ms = meta
        .get("last_run")
        .and_then(Value::as_str)
        .and_then(parse_iso_ms);
    let hours_since_last = last_run_ms
        .map(|ms| ((chrono::Utc::now().timestamp_millis() - ms) as f64 / 3_600_000.0).max(0.0));
    let skipped = !force && hours_since_last.map(|h| h < min_hours).unwrap_or(false);

    Ok(json!({
        "ok": true,
        "collector_id": collector_id,
        "force": force,
        "min_hours": min_hours,
        "hours_since_last": hours_since_last,
        "skipped": skipped,
        "reason": if skipped { Value::String("cadence".to_string()) } else { Value::Null },
        "meta": meta,
        "meta_path": meta_path.display().to_string()
    }))
}

fn handle_begin_collection(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let controls = crate::collector_runtime_kernel_support::resolve_controls(payload);
    let collector_id = lane_utils::clean_token(
        controls.get("collector_id").and_then(Value::as_str),
        "collector",
    );
    let min_hours = controls
        .get("min_hours")
        .and_then(Value::as_f64)
        .unwrap_or(4.0)
        .clamp(0.0, 24.0 * 365.0);
    let force = controls
        .get("force")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let prepared = handle_prepare_run(
        root,
        payload_obj(&json!({
            "collector_id": collector_id,
            "eyes_state_dir": payload.get("eyes_state_dir").cloned().unwrap_or(Value::Null),
            "force": force,
            "min_hours": min_hours
        })),
    )?;
    if prepared.get("skipped").and_then(Value::as_bool) == Some(true) {
        return Ok(json!({
            "ok": true,
            "success": true,
            "eye": collector_id,
            "skipped": true,
            "reason": "cadence",
            "hours_since_last": prepared.get("hours_since_last").cloned().unwrap_or(Value::Null),
            "min_hours": min_hours,
            "items": [],
            "controls": controls,
            "meta": prepared.get("meta").cloned().unwrap_or(Value::Object(Map::new()))
        }));
    }
    Ok(json!({
        "ok": true,
        "success": true,
        "skipped": false,
        "eye": collector_id,
        "min_hours": min_hours,
        "max_items": controls.get("max_items").cloned().unwrap_or(Value::from(20)),
        "controls": controls,
        "meta": prepared.get("meta").cloned().unwrap_or(Value::Object(Map::new()))
    }))
}

fn handle_classify_error(payload: &Map<String, Value>) -> Value {
    let message = lane_utils::clean_text(payload.get("message").and_then(Value::as_str), 200);
    let status_from_err = payload
        .get("http_status")
        .or_else(|| payload.get("status"))
        .and_then(Value::as_u64)
        .filter(|v| *v > 0);
    let status_from_msg = parse_http_status_from_message(&message);
    let http_status = status_from_err.or(status_from_msg);

    let mut code = normalize_node_code(payload.get("code").and_then(Value::as_str).unwrap_or(""));
    if code.is_empty() {
        if let Some(status) = http_status {
            code = http_status_to_code(status).to_string();
        }
    }
    if code.is_empty() {
        code = classify_message(&message);
    }
    if code.is_empty() {
        code = "collector_error".to_string();
    }

    json!({
        "ok": true,
        "code": code.clone(),
        "message": message,
        "http_status": http_status,
        "transport": is_transport_failure_code(&code),
        "retryable": is_retryable_code(&code)
    })
}

fn compute_bytes_from_items(items: &[Value]) -> u64 {
    items
        .iter()
        .filter_map(Value::as_object)
        .map(|row| row.get("bytes").and_then(Value::as_u64).unwrap_or(0))
        .sum()
}

fn sample_title(items: &[Value]) -> Value {
    items
        .first()
        .and_then(Value::as_object)
        .and_then(|o| o.get("title"))
        .and_then(Value::as_str)
        .map(|v| lane_utils::clean_text(Some(v), 120))
        .filter(|v| !v.is_empty())
        .map(Value::String)
        .unwrap_or(Value::Null)
}

fn handle_finalize_run(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let collector_id = clean_collector_id(payload);
    let min_hours = json_f64(payload, "min_hours", 4.0, 0.0, 24.0 * 365.0);
    let max_items = json_u64(payload, "max_items", 20, 1, 200) as usize;
    let use_cache_when_empty = json_bool(payload, "use_cache_when_empty", false);
    let bytes = json_u64(payload, "bytes", 0, 0, u64::MAX);
    let requests = json_u64(payload, "requests", 1, 0, u64::MAX);
    let duration_ms = json_u64(payload, "duration_ms", 0, 0, u64::MAX);
    let fetch_error_code =
        lane_utils::clean_text(payload.get("fetch_error_code").and_then(Value::as_str), 80);
    let fetch_error = if fetch_error_code.is_empty() {
        String::new()
    } else {
        fetch_error_code
    };
    let http_status = payload.get("http_status").and_then(Value::as_u64);

    let mut items = payload
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if items.len() > max_items {
        items = items.into_iter().take(max_items).collect::<Vec<_>>();
    }

    if items.is_empty() && (use_cache_when_empty || !fetch_error.is_empty()) {
        let cache = read_json(
            &cache_path_for(root, payload, &collector_id),
            json!({ "items": [] }),
        );
        let mut cached = cache
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if !cached.is_empty() {
            if cached.len() > max_items {
                cached = cached.into_iter().take(max_items).collect::<Vec<_>>();
            }
            return Ok(json!({
                "ok": true,
                "success": true,
                "eye": collector_id,
                "cache_hit": true,
                "degraded": !fetch_error.is_empty(),
                "error": if fetch_error.is_empty() { Value::Null } else { Value::String(fetch_error.clone()) },
                "items": cached,
                "bytes": compute_bytes_from_items(&cached),
                "requests": requests,
                "duration_ms": duration_ms,
                "cadence_hours": min_hours,
                "sample": sample_title(&cached)
            }));
        }
    }

    let mut meta = normalize_meta_value(&collector_id, payload.get("meta"));
    let now_iso = chrono::Utc::now().to_rfc3339();
    meta["last_run"] = Value::String(now_iso.clone());
    if !items.is_empty() {
        meta["last_success"] = Value::String(now_iso);
        write_json(
            &cache_path_for(root, payload, &collector_id),
            &json!({ "items": items.clone() }),
        )?;
    }
    write_json(&meta_path_for(root, payload, &collector_id), &meta)?;
