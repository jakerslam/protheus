    #[test]
    fn dashboard_troubleshooting_summary_window_filter_excludes_old_entries() {
        let root = tempfile::tempdir().expect("tempdir");
        let recent_path = root.path().join(DASHBOARD_TROUBLESHOOTING_RECENT_REL);
        if let Some(parent) = recent_path.parent() {
            fs::create_dir_all(parent).expect("mkdir troubleshooting");
        }
        let now_epoch = dashboard_troubleshooting_now_epoch_seconds();
        fs::write(
            &recent_path,
            serde_json::to_string_pretty(&json!({
                "type": "dashboard_troubleshooting_recent_workflows",
                "entries": [
                    {
                        "captured_at_epoch_s": now_epoch - 40,
                        "workflow": {
                            "classification": "low_signal",
                            "error_code": "web_tool_low_signal"
                        }
                    },
                    {
                        "captured_at_epoch_s": now_epoch - 400,
                        "workflow": {
                            "classification": "tool_not_found",
                            "error_code": "web_tool_not_found"
                        }
                    }
                ]
            }))
            .expect("json"),
        )
        .expect("write");

        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.window",
            &json!({
                "window_seconds": 120
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.pointer("/window/applied").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload.pointer("/recent/entry_count").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            payload.pointer("/recent/failure_rate").and_then(Value::as_f64),
            Some(1.0)
        );
        assert_eq!(
            payload
                .pointer("/top_failure_cluster/severity_tier")
                .and_then(Value::as_str),
            Some("high")
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/health_tier")
                .and_then(Value::as_str),
            Some("empty")
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/stale_count")
                .and_then(Value::as_u64),
            Some(0)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/stale_ratio")
                .and_then(Value::as_f64),
            Some(0.0)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/fresh_count")
                .and_then(Value::as_u64),
            Some(0)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/fresh_ratio")
                .and_then(Value::as_f64),
            Some(0.0)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/aging_count")
                .and_then(Value::as_u64),
            Some(0)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/aging_ratio")
                .and_then(Value::as_f64),
            Some(0.0)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/health_reason")
                .and_then(Value::as_str),
            Some("outbox_empty")
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_deadline_epoch_s")
                .and_then(Value::as_i64),
            Some(0)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_deadline_remaining_seconds")
                .and_then(Value::as_i64),
            Some(0)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_breach")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_breach_reason")
                .and_then(Value::as_str),
            Some("none")
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_breach_detected_at_epoch_s")
                .and_then(Value::as_i64),
            Some(0)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract_version")
                .and_then(Value::as_str),
            Some("v1")
        );
        assert!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_snapshot_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= now_epoch
        );
        assert_eq!(
            payload
                .pointer("/recent/classification_histogram/0/classification")
                .and_then(Value::as_str),
            Some("low_signal")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_recent_alias_routes_to_summary() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.recent",
            &json!({
                "window_seconds": 3600
            }),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_metrics_alias_routes_to_summary() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.metrics",
            &json!({
                "window_seconds": 300
            }),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_health_alias_routes_to_summary() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.health",
            &json!({
                "window_seconds": 300
            }),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_queue_health_alias_routes_to_summary() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.queue_health",
            &json!({
                "window_seconds": 300
            }),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_accepts_comma_separated_error_filter() {
        let root = tempfile::tempdir().expect("tempdir");
        let recent_path = root.path().join(DASHBOARD_TROUBLESHOOTING_RECENT_REL);
        if let Some(parent) = recent_path.parent() {
            fs::create_dir_all(parent).expect("mkdir troubleshooting");
        }
        fs::write(
            &recent_path,
            serde_json::to_string_pretty(&json!({
                "type": "dashboard_troubleshooting_recent_workflows",
                "entries": [
                    {
                        "workflow": {
                            "classification": "tool_not_found",
                            "error_code": "web_tool_not_found"
                        }
                    },
                    {
                        "workflow": {
                            "classification": "low_signal",
                            "error_code": "web_tool_low_signal"
                        }
                    }
                ]
            }))
            .expect("json"),
        )
        .expect("write");

        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary",
            &json!({
                "error_filter": "web_tool_not_found,web_tool_low_signal"
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.pointer("/recent/entry_count").and_then(Value::as_u64),
            Some(2)
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_by_time_accepts_minutes_and_reports_filtered_out_count() {
        let root = tempfile::tempdir().expect("tempdir");
        let recent_path = root.path().join(DASHBOARD_TROUBLESHOOTING_RECENT_REL);
        if let Some(parent) = recent_path.parent() {
            fs::create_dir_all(parent).expect("mkdir troubleshooting");
        }
        let now_epoch = dashboard_troubleshooting_now_epoch_seconds();
        fs::write(
            &recent_path,
            serde_json::to_string_pretty(&json!({
                "type": "dashboard_troubleshooting_recent_workflows",
                "entries": [
                    {
                        "captured_at_epoch_s": now_epoch - 30,
                        "workflow": {
                            "classification": "low_signal",
                            "error_code": "web_tool_low_signal"
                        }
                    },
                    {
                        "captured_at_epoch_s": now_epoch - 900,
                        "workflow": {
                            "classification": "tool_not_found",
                            "error_code": "web_tool_not_found"
                        }
                    }
                ]
            }))
            .expect("json"),
        )
        .expect("write");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.by_time",
            &json!({
                "window_minutes": 2
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.pointer("/window/applied").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/window/filtered_out_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            payload.pointer("/window/window_seconds").and_then(Value::as_i64),
            Some(120)
        );
    }
