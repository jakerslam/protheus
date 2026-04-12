fn command_run(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let max_items = clamp_u64(payload, "max_items", 10, 1, 200) as usize;
    let min_hours = as_f64(payload.get("min_hours"), 4.0).clamp(0.0, 24.0 * 365.0);
    let force = as_bool(payload.get("force"), false);
    let timeout_ms = clamp_u64(payload, "timeout_ms", 15_000, 1_000, 120_000);
    let search_query = clean_text(payload.get("search_query").and_then(Value::as_str), 240);
    let started_at_ms = Utc::now().timestamp_millis().max(0) as u64;

    let plan = command_build_fetch_plan(
        &json!({ "search_query": search_query })
            .as_object()
            .cloned()
            .unwrap_or_default(),
    );
    let request = plan
        .get("requests")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let fetch_url = clean_text(request.get("url").and_then(Value::as_str), 800);
    let accept = clean_text(request.get("accept").and_then(Value::as_str), 120);

    let (rss_xml, bytes, requests, fetch_error) =
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
            "rss_xml": rss_xml,
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
        "parse-rss" => {
            let xml = payload.get("xml").and_then(Value::as_str).unwrap_or("");
            Ok(json!({ "ok": true, "gigs": parse_rss(xml) }))
        }
        "map-gigs" => Ok(map_gigs(payload)),
        "fallback-gigs" => Ok(fallback_gigs(payload)),
        _ => Err("upwork_gigs_collector_kernel_unknown_command".to_string()),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let command = argv[0].trim().to_ascii_lowercase();
    let payload = match lane_utils::payload_json(&argv[1..], "upwork_gigs_collector_kernel") {
        Ok(v) => v,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error("upwork_gigs_collector_kernel_error", &err));
            return 1;
        }
    };
    let payload_obj = lane_utils::payload_obj(&payload);

    match dispatch(root, &command, payload_obj) {
        Ok(out) => {
            lane_utils::print_json_line(&lane_utils::cli_receipt("upwork_gigs_collector_kernel", out));
            0
        }
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error("upwork_gigs_collector_kernel_error", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_root() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "upwork_gigs_collector_kernel_test_{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn parse_rss_extracts_items() {
        let xml = r#"<rss><channel><item><title>AI gig</title><link>https://x/jobs/1</link><description>Automation</description></item></channel></rss>"#;
        let rows = parse_rss(xml);
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn map_gigs_sets_signal_for_high_value() {
        let payload = json!({
            "date": "2026-03-27",
            "max_items": 10,
            "seen_ids": [],
            "gigs": [
                {
                    "title": "OpenAI automation workflow",
                    "url": "https://x/jobs/1",
                    "description": "Build GPT automation with API integration"
                }
            ]
        });
        let out = map_gigs(lane_utils::payload_obj(&payload));
        assert_eq!(
            out.get("items")
                .and_then(Value::as_array)
                .and_then(|rows| rows.first())
                .and_then(|row| row.get("signal"))
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn map_gigs_rejects_non_upwork_urls_and_canonicalizes_host() {
        let payload = json!({
            "date": "2026-04-11",
            "max_items": 10,
            "seen_ids": [],
            "gigs": [
                {"title": "bad", "url": "javascript:alert(1)"},
                {"title": "good", "url": "http://upwork.com/jobs/good", "description": "AI workflow build"}
            ]
        });
        let out = map_gigs(lane_utils::payload_obj(&payload));
        let items = out.get("items").and_then(Value::as_array).cloned().unwrap_or_default();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].get("url").and_then(Value::as_str), Some("https://www.upwork.com/jobs/good"));
    }

    #[test]
    fn build_fetch_plan_returns_rss_request() {
        let out = command_build_fetch_plan(&Map::new());
        let reqs = out.get("requests").and_then(Value::as_array).cloned().unwrap_or_default();
        assert_eq!(reqs.len(), 1);
        assert_eq!(
            reqs.first()
                .and_then(Value::as_object)
                .and_then(|o| o.get("key"))
                .and_then(Value::as_str),
            Some("rss")
        );
    }

    #[test]
    fn prepare_run_skips_when_recent() {
        let root = temp_root();
        let payload = json!({ "min_hours": 48.0, "force": false });
        let meta_path = meta_path_for(&root, lane_utils::payload_obj(&payload));
        let _ = write_json_atomic(
            &meta_path,
            &json!({
                "collector_id": COLLECTOR_ID,
                "last_run": now_iso(),
                "last_success": now_iso(),
                "seen_ids": []
            }),
        );
        let out = command_prepare_run(&root, lane_utils::payload_obj(&payload));
        assert_eq!(out.get("skipped").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn collect_returns_skip_payload_when_cadence_not_met() {
        let root = temp_root();
        let payload = json!({ "min_hours": 48.0, "force": false });
        let meta_path = meta_path_for(&root, lane_utils::payload_obj(&payload));
        let _ = write_json_atomic(
            &meta_path,
            &json!({
                "collector_id": COLLECTOR_ID,
                "last_run": now_iso(),
                "last_success": now_iso(),
                "seen_ids": []
            }),
        );

        let out = command_collect(&root, lane_utils::payload_obj(&payload)).expect("collect");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("skipped").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("reason").and_then(Value::as_str), Some("cadence"));
    }

    #[test]
    fn prepare_run_roots_relative_env_state_dir_under_root() {
        let root = temp_root();
        let previous = std::env::var("EYES_STATE_DIR").ok();
        std::env::set_var("EYES_STATE_DIR", "relative/upwork-eyes");

        let out = command_prepare_run(&root, &Map::new());
        assert_eq!(
            out.get("meta_path").and_then(Value::as_str),
            Some(
                root.join("relative/upwork-eyes")
                    .join("collector_meta")
                    .join("upwork_gigs.json")
                    .to_string_lossy()
                    .as_ref()
            )
        );

        if let Some(value) = previous {
            std::env::set_var("EYES_STATE_DIR", value);
        } else {
            std::env::remove_var("EYES_STATE_DIR");
        }
    }
}
