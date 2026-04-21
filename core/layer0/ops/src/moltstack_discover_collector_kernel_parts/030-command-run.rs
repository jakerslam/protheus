
fn command_run(payload: &Map<String, Value>) -> Result<Value, String> {
    let max_items = clamp_u64(payload, "max_items", 20, 1, 50);
    let max_seconds = clamp_u64(payload, "max_seconds", 10, 1, 30);
    let started_at_ms = Utc::now().timestamp_millis().max(0) as u64;

    let plan = build_fetch_plan(payload);
    let request = plan
        .get("requests")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let fetch_url = {
        let override_url = clean_text(payload.get("api_url").and_then(Value::as_str), 800);
        if override_url.is_empty() {
            clean_text(request.get("url").and_then(Value::as_str), 800)
        } else {
            override_url
        }
    };
    let accept = clean_text(request.get("accept").and_then(Value::as_str), 160);
    let timeout_ms = payload
        .get("timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| {
            plan.get("timeout_ms")
                .and_then(Value::as_u64)
                .unwrap_or(10_000)
        })
        .clamp(1_000, 30_000);

    let (posts_json, bytes, requests, fetch_error) =
        match curl_fetch_with_status(&fetch_url, timeout_ms, &accept) {
            Ok((status, body, _)) => {
                if status >= 400 {
                    (
                        Value::Null,
                        0_u64,
                        0_u64,
                        Some(http_status_to_code(status).to_string()),
                    )
                } else {
                    let b = body.as_bytes().len() as u64;
                    (parse_json_or_null(&body), b, 1_u64, None)
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

    let mut out = command_collect(&object_from_value(json!({
        "api_url": fetch_url,
        "allowed_domains": payload.get("allowed_domains").cloned().unwrap_or(Value::Array(Vec::new())),
        "max_seconds": max_seconds,
        "topics": payload.get("topics").cloned().unwrap_or(Value::Array(Vec::new())),
        "max_items": max_items,
        "posts_json": posts_json,
        "fetch_error": fetch_error,
        "duration_ms": duration_ms
    })))?;
    if let Some(obj) = out.as_object_mut() {
        obj.insert("bytes".to_string(), Value::from(bytes));
        obj.insert("requests".to_string(), Value::from(requests));
        obj.insert("duration_ms".to_string(), Value::from(duration_ms));
    }
    Ok(out)
}

fn dispatch(command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "run" => command_run(payload),
        "preflight" => Ok(preflight(payload)),
        "build-fetch-plan" => Ok(build_fetch_plan(payload)),
        "classify-fetch-error" => Ok(classify_fetch_error(payload)),
        "finalize-run" => Ok(finalize_run(payload)),
        "map-posts" => Ok(map_posts(payload)),
        "collect" => command_collect(payload),
        _ => Err("moltstack_discover_collector_kernel_unknown_command".to_string()),
    }
}

pub fn run(_root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let command = argv[0].trim().to_ascii_lowercase();
    let payload = match lane_utils::payload_json(&argv[1..], "moltstack_discover_collector_kernel")
    {
        Ok(v) => v,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "moltstack_discover_collector_kernel_error",
                &err,
            ));
            return 1;
        }
    };
    let payload_obj = lane_utils::payload_obj(&payload);
    match dispatch(&command, payload_obj) {
        Ok(out) => {
            lane_utils::print_json_line(&lane_utils::cli_receipt(
                "moltstack_discover_collector_kernel",
                out,
            ));
            0
        }
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "moltstack_discover_collector_kernel_error",
                &err,
            ));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preflight_flags_non_https() {
        let payload = json!({
            "api_url": "http://moltstack.net/api/posts",
            "allowed_domains": ["moltstack.net"],
            "max_items": 10,
            "max_seconds": 5
        });
        let out = preflight(lane_utils::payload_obj(&payload));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn map_posts_emits_items() {
        let payload = json!({
            "max_items": 10,
            "topics": ["automation"],
            "posts": {
              "posts": [
                {"title":"AI workflow automation","slug":"ai-workflow","agent":{"slug":"agent-x"}}
              ]
            }
        });
        let out = map_posts(lane_utils::payload_obj(&payload));
        assert_eq!(
            out.get("items")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
    }

    #[test]
    fn build_fetch_plan_defaults_url() {
        let out = build_fetch_plan(&Map::new());
        let url = out
            .get("requests")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(Value::as_object)
            .and_then(|o| o.get("url"))
            .and_then(Value::as_str);
        assert_eq!(url, Some("https://moltstack.net/api/posts"));
    }

    #[test]
    fn classify_fetch_error_allows_fallback_for_rate_limited() {
        let out = classify_fetch_error(lane_utils::payload_obj(&json!({
            "error_code": "rate_limited"
        })));
        assert_eq!(
            out.get("fallback_allowed").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn collect_returns_fallback_signal_on_fetch_error() {
        let out = command_collect(lane_utils::payload_obj(&json!({
            "api_url": "https://moltstack.net/api/posts",
            "allowed_domains": ["moltstack.net"],
            "max_items": 10,
            "max_seconds": 5,
            "fetch_error": "rate_limited"
        })))
        .expect("collect");
        assert_eq!(out.get("success").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("fallback_allowed").and_then(Value::as_bool),
            Some(true)
        );
    }
}
