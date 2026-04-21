
pub fn run(_root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let command = argv[0].trim().to_ascii_lowercase();
    let payload = match lane_utils::payload_json(&argv[1..], "moltbook_hot_collector_kernel") {
        Ok(v) => v,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "moltbook_hot_collector_kernel_error",
                &err,
            ));
            return 1;
        }
    };
    let payload_obj = lane_utils::payload_obj(&payload);
    match dispatch(&command, payload_obj) {
        Ok(out) => {
            lane_utils::print_json_line(&lane_utils::cli_receipt(
                "moltbook_hot_collector_kernel",
                out,
            ));
            0
        }
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "moltbook_hot_collector_kernel_error",
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
    fn preflight_uses_api_base_host_for_allowlist_checks() {
        let out = preflight(lane_utils::payload_obj(&json!({"secret_present": false, "allowed_domains": ["api.moltbook.com"], "host": "www.moltbook.com", "api_base": "https://api.moltbook.com", "max_items": 10})));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        let codes = out.get("failures").and_then(Value::as_array).cloned().unwrap_or_default().into_iter().filter_map(|row| row.get("code").and_then(Value::as_str).map(str::to_string)).collect::<Vec<_>>();
        assert!(codes.contains(&"auth_missing".to_string()));
        assert!(codes.contains(&"api_base_host_mismatch".to_string()));
        assert_eq!(out.get("checks").and_then(Value::as_array).and_then(|rows| rows.iter().find(|row| row.get("name").and_then(Value::as_str) == Some("allowlisted_host"))).and_then(|row| row.get("host")).and_then(Value::as_str), Some("api.moltbook.com"));
    }

    #[test]
    fn classify_fetch_error_allows_timeout() {
        let out = classify_fetch_error(lane_utils::payload_obj(&json!({
            "error_code": "timeout"
        })));
        assert_eq!(
            out.get("fallback_allowed").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn map_posts_outputs_items() {
        let out = map_posts(lane_utils::payload_obj(&json!({
            "max_items": 2,
            "topics": ["ai", "agents"],
            "posts": [
                {"id":"p1","title":"Ship agents","url":"https://www.moltbook.com/p/p1"}
            ]
        })));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("items")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
    }

    #[test]
    fn collect_returns_fallback_signal_on_fetch_error() {
        let out = command_collect(lane_utils::payload_obj(&json!({
            "secret_present": true,
            "allowed_domains": ["moltbook.com", "www.moltbook.com"],
            "host": "www.moltbook.com",
            "max_items": 10,
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
