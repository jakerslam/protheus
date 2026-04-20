
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
