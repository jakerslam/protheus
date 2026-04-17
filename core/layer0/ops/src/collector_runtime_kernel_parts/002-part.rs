
    if items.is_empty() && !fetch_error.is_empty() {
        return Ok(json!({
            "ok": true,
            "success": false,
            "eye": collector_id,
            "items": [],
            "bytes": bytes,
            "requests": requests,
            "duration_ms": duration_ms,
            "error": fetch_error,
            "http_status": http_status,
            "cadence_hours": min_hours
        }));
    }

    Ok(json!({
        "ok": true,
        "success": true,
        "eye": collector_id,
        "items": items,
        "bytes": bytes,
        "requests": requests,
        "duration_ms": duration_ms,
        "cadence_hours": min_hours,
        "sample": sample_title(&items)
    }))
}

fn handle_fetch_text(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let collector_id = clean_collector_id(payload);
    let url = lane_utils::clean_text(payload.get("url").and_then(Value::as_str), 800);
    if url.is_empty() {
        return Err("collector_runtime_kernel_fetch_text_missing_url".to_string());
    }
    let attempts = json_u64(payload, "attempts", 3, 1, 5);
    let timeout_ms = json_u64(payload, "timeout_ms", 15_000, 1_000, 120_000);
    let headers = crate::collector_runtime_kernel_support::parse_headers(payload);
    let mut last_error = "collector_error".to_string();

    for attempt in 1..=attempts {
        let prep = handle_prepare_attempt(
            root,
            payload_obj(&json!({
                "collector_id": collector_id.clone(),
                "rate_state_path": payload.get("rate_state_path").cloned().unwrap_or(Value::Null),
                "min_interval_ms": payload.get("min_interval_ms").cloned().unwrap_or(Value::Null)
            })),
        )?;
        if prep.get("circuit_open").and_then(Value::as_bool) == Some(true) {
            let retry_after = prep
                .get("retry_after_ms")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            return Err(format!("rate_limited:circuit_open:{retry_after}"));
        }

        match crate::collector_runtime_kernel_support::curl_fetch_with_status(
            &url, timeout_ms, &headers,
        ) {
            Ok((status, body, bytes)) if status < 400 => {
                let _ = handle_mark_success(
                    root,
                    payload_obj(&json!({
                        "collector_id": collector_id.clone(),
                        "rate_state_path": payload.get("rate_state_path").cloned().unwrap_or(Value::Null),
                        "min_interval_ms": payload.get("min_interval_ms").cloned().unwrap_or(Value::Null)
                    })),
                );
                return Ok(json!({
                    "ok": true,
                    "collector_id": collector_id.clone(),
                    "status": status,
                    "text": body,
                    "bytes": bytes,
                    "attempt": attempt
                }));
            }
            Ok((status, _, _)) => {
                last_error = http_status_to_code(status).to_string();
            }
            Err(err) => {
                last_error = crate::collector_runtime_kernel_support::split_error_code(&err);
            }
        }

        let mark = handle_mark_failure(
            root,
            payload_obj(&json!({
                "collector_id": collector_id.clone(),
                "rate_state_path": payload.get("rate_state_path").cloned().unwrap_or(Value::Null),
                "code": last_error.clone(),
                "base_backoff_ms": payload.get("base_backoff_ms").cloned().unwrap_or(Value::Null),
                "max_backoff_ms": payload.get("max_backoff_ms").cloned().unwrap_or(Value::Null),
                "circuit_open_ms": payload.get("circuit_open_ms").cloned().unwrap_or(Value::Null),
                "circuit_after_failures": payload.get("circuit_after_failures").cloned().unwrap_or(Value::Null)
            })),
        )?;
        let retryable = mark
            .get("retryable")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !retryable || attempt >= attempts {
            break;
        }
    }

    Err(format!(
        "collector_runtime_kernel_fetch_text_failed:{last_error}"
    ))
}

fn dispatch(root: &Path, command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "classify-error" => Ok(handle_classify_error(payload)),
        "resolve-controls" => Ok(crate::collector_runtime_kernel_support::resolve_controls(
            payload,
        )),
        "begin-collection" => handle_begin_collection(root, payload),
        "prepare-run" => handle_prepare_run(root, payload),
        "finalize-run" => handle_finalize_run(root, payload),
        "fetch-text" => handle_fetch_text(root, payload),
        "prepare-attempt" => handle_prepare_attempt(root, payload),
        "mark-success" => handle_mark_success(root, payload),
        "mark-failure" => handle_mark_failure(root, payload),
        _ => Err("collector_runtime_kernel_unknown_command".to_string()),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let command = argv[0].trim().to_ascii_lowercase();
    let payload = match lane_utils::payload_json(&argv[1..], "collector_runtime_kernel") {
        Ok(value) => value,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "collector_runtime_kernel_error",
                &err,
            ));
            return 1;
        }
    };
    let payload_obj = payload_obj(&payload);

    match dispatch(root, &command, payload_obj) {
        Ok(value) => {
            lane_utils::print_json_line(&lane_utils::cli_receipt(
                "collector_runtime_kernel",
                value,
            ));
            0
        }
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "collector_runtime_kernel_error",
                &err,
            ));
            1
        }
    }
}

#[cfg(test)]
#[path = "collector_runtime_kernel_tests.rs"]
mod collector_runtime_kernel_tests;
