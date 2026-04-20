
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
