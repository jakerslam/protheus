
fn command_collect(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let min_hours = as_f64(payload.get("min_hours"), 1.0).clamp(0.0, 24.0 * 365.0);
    let max_items = clamp_u64(payload, "max_items", 20, 1, 200) as usize;
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

    let market_html = payload
        .get("market_html")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let quotes = if market_html.trim().is_empty() {
        Vec::<Value>::new()
    } else {
        extract_quotes_from_html(market_html.as_str())
            .iter()
            .map(quote_to_value)
            .collect::<Vec<_>>()
    };
    let bytes = clamp_u64(payload, "bytes", 0, 0, u64::MAX);
    let requests = clamp_u64(payload, "requests", 0, 0, u64::MAX);
    let duration_ms = clamp_u64(payload, "duration_ms", 0, 0, u64::MAX);
    let mut fetch_error = clean_text(payload.get("fetch_error").and_then(Value::as_str), 220);
    if fetch_error.is_empty() && market_html.trim().is_empty() {
        fetch_error = "collector_error_no_market_html".to_string();
    }

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
            "quotes": quotes,
            "fetch_error": if fetch_error.is_empty() { Value::Null } else { Value::String(fetch_error) }
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    )
}

fn command_run(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let max_items = clamp_u64(payload, "max_items", 20, 1, 200) as usize;
    let min_hours = as_f64(payload.get("min_hours"), 1.0).clamp(0.0, 24.0 * 365.0);
    let force = as_bool(payload.get("force"), false);
    let timeout_ms = clamp_u64(payload, "timeout_ms", 15_000, 1_000, 120_000);
    let started_at_ms = Utc::now().timestamp_millis().max(0) as u64;

    let plan = command_build_fetch_plan(&Map::new());
    let request = plan
        .get("requests")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let fetch_url = {
        let override_url = clean_text(payload.get("url").and_then(Value::as_str), 800);
        if override_url.is_empty() {
            clean_text(request.get("url").and_then(Value::as_str), 800)
        } else {
            override_url
        }
    };
    let accept = clean_text(request.get("accept").and_then(Value::as_str), 160);

    let (market_html, bytes, requests, fetch_error) =
        match curl_fetch_with_status(&fetch_url, timeout_ms, &accept) {
            Ok((status, body, body_bytes)) => {
                if status >= 400 {
                    (
                        String::new(),
                        0_u64,
                        0_u64,
                        Some(http_status_to_code(status).to_string()),
                    )
                } else {
                    (body, body_bytes, 1_u64, None)
                }
            }
            Err(err) => {
                let code = clean_text(Some(&err), 120)
                    .split(':')
                    .next()
                    .unwrap_or("collector_error")
                    .to_string();
                (String::new(), 0_u64, 0_u64, Some(code))
            }
        };

    let duration_ms = Utc::now()
        .timestamp_millis()
        .max(0)
        .saturating_sub(started_at_ms as i64) as u64;

    command_collect(
        root,
        &json!({
            "eyes_state_dir": payload.get("eyes_state_dir").cloned().unwrap_or(Value::Null),
            "force": force,
            "min_hours": min_hours,
            "max_items": max_items,
            "bytes": bytes,
            "requests": requests,
            "duration_ms": duration_ms,
            "market_html": market_html,
            "fetch_error": fetch_error
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    )
}

fn dispatch(root: &Path, command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "run" => command_run(root, payload),
        "prepare-run" => Ok(command_prepare_run(root, payload)),
        "build-fetch-plan" => Ok(command_build_fetch_plan(payload)),
        "finalize-run" => command_finalize_run(root, payload),
        "collect" => command_collect(root, payload),
        "extract-quotes" => {
            let html = payload.get("html").and_then(Value::as_str).unwrap_or("");
            let quotes = extract_quotes_from_html(html)
                .into_iter()
                .map(|q| quote_to_value(&q))
                .collect::<Vec<_>>();
            Ok(json!({ "ok": true, "quotes": quotes }))
        }
        "map-quotes" => Ok(map_quotes(payload)),
        "fallback-indices" => Ok(fallback_indices(payload)),
        _ => Err("stock_market_collector_kernel_unknown_command".to_string()),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let command = argv[0].trim().to_ascii_lowercase();
    let payload = match lane_utils::payload_json(&argv[1..], "stock_market_collector_kernel") {
        Ok(v) => v,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "stock_market_collector_kernel_error",
                &err,
            ));
            return 1;
        }
    };
    let payload_obj = lane_utils::payload_obj(&payload);

    match dispatch(root, &command, payload_obj) {
        Ok(out) => {
            lane_utils::print_json_line(&lane_utils::cli_receipt(
                "stock_market_collector_kernel",
                out,
            ));
            0
        }
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "stock_market_collector_kernel_error",
                &err,
            ));
            1
        }
    }
}

#[cfg(test)]
#[path = "stock_market_collector_kernel_tests.rs"]
mod stock_market_collector_kernel_tests;

