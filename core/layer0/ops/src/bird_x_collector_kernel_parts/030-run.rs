
pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let command = argv[0].trim().to_ascii_lowercase();
    let payload = match lane_utils::payload_json(&argv[1..], "bird_x_collector_kernel") {
        Ok(v) => v,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "bird_x_collector_kernel_error",
                &err,
            ));
            return 1;
        }
    };
    let payload_obj = lane_utils::payload_obj(&payload);
    match dispatch(root, &command, payload_obj) {
        Ok(out) => {
            lane_utils::print_json_line(&lane_utils::cli_receipt("bird_x_collector_kernel", out));
            0
        }
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "bird_x_collector_kernel_error",
                &err,
            ));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_root(name: &str) -> PathBuf {
        let mut root = std::env::temp_dir();
        let nonce = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        root.push(format!("protheus_bird_x_kernel_{name}_{nonce}"));
        fs::create_dir_all(&root).expect("mkdir temp root");
        root
    }

    #[test]
    fn preflight_reflects_missing_cli() {
        let payload = json!({"bird_cli_present": false});
        let out = command_preflight(lane_utils::payload_obj(&payload));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn map_results_dedupes_and_infers_topics() {
        let payload = json!({
            "max_items": 10,
            "seen_ids": [],
            "results": [
                { "id": "123", "text": "AI agent launch update", "author": { "handle": "ax", "name": "Axiom" }, "likes": 10, "retweets": 3 },
                { "id": "123", "text": "AI agent launch update", "author": { "handle": "ax", "name": "Axiom" }, "likes": 10, "retweets": 3 },
                { "id": "456", "text": "LLM news update", "author_name": "Anon" }
            ]
        });
        let out = command_map_results(lane_utils::payload_obj(&payload));
        let rows = out.get("items").and_then(Value::as_array).cloned().unwrap_or_default();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].get("url").and_then(Value::as_str), Some("https://x.com/i/web/status/456"));
        assert_eq!(rows[0].get("topics").and_then(Value::as_array).map(|topics| !topics.is_empty()), Some(true));
    }

    #[test]
    fn prepare_run_respects_cadence() {
        let root = temp_root("cadence");
        let payload = json!({"force":false,"min_hours":4.0});
        let meta_path = support::meta_path_for(&root, lane_utils::payload_obj(&payload));
        support::write_json_atomic(
            &meta_path,
            &json!({"last_run": support::now_iso(), "seen_ids": []}),
        )
        .expect("write meta");
        let out = command_prepare_run(&root, lane_utils::payload_obj(&payload));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("skipped").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn finalize_run_returns_cache_on_failures() {
        let root = temp_root("cache_fallback");
        let payload = json!({});
        let cache_path = support::cache_path_for(&root, lane_utils::payload_obj(&payload));
        support::write_json_atomic(
            &cache_path,
            &json!({"items":[{"id":"cached","title":"cached title","bytes":11}]}),
        )
        .expect("write cache");
        let out = command_finalize_run(
            &root,
            lane_utils::payload_obj(&json!({
                "meta": {"seen_ids":[]},
                "items": [],
                "query_failures": [{"code":"timeout","message":"request timed out","http_status":null}],
                "bytes": 0,
                "requests": 1,
                "duration_ms": 10
            })),
        )
        .expect("finalize");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("cache_hit").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("items").and_then(Value::as_array).map(|rows| rows.len()), Some(1));
    }

    #[test]
    fn collect_returns_preflight_error_when_bird_missing() {
        let root = temp_root("collect_preflight_missing");
        let out = command_collect(
            &root,
            lane_utils::payload_obj(&json!({
                "bird_cli_present": false
            })),
        )
        .expect("collect");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(out.get("error").and_then(Value::as_str), Some("env_blocked"));
    }
}
