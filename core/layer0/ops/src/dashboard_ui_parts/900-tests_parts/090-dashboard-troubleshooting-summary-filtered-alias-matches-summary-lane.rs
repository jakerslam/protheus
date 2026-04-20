    #[test]
    fn dashboard_troubleshooting_summary_filtered_alias_matches_summary_lane() {
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
                        "stale": false,
                        "workflow": {
                            "classification": "tool_not_found",
                            "error_code": "web_tool_not_found"
                        }
                    },
                    {
                        "stale": false,
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
            "dashboard.troubleshooting.summary.filtered",
            &json!({
                "error_filter": ["web_tool_not_found"]
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.get("type").and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
        assert_eq!(
            payload.pointer("/filters/applied").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload.pointer("/recent/entry_count").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            payload
                .pointer("/recent/error_histogram/0/error")
                .and_then(Value::as_str),
            Some("web_tool_not_found")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_supports_wildcard_filters_and_top_cluster() {
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
                        "stale": false,
                        "workflow": {
                            "classification": "tool_surface_degraded",
                            "error_code": "web_tool_surface_degraded"
                        }
                    },
                    {
                        "stale": false,
                        "workflow": {
                            "classification": "tool_surface_degraded",
                            "error_code": "web_tool_surface_unavailable"
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
                "classification_filter": ["tool_surface_*"]
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.pointer("/recent/entry_count").and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(
            payload
                .pointer("/top_failure_cluster/top_classification")
                .and_then(Value::as_str),
            Some("tool_surface_degraded")
        );
        assert_eq!(
            payload
                .pointer("/top_failure_cluster/top_classification_count")
                .and_then(Value::as_i64),
            Some(2)
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_reports_no_match_on_filtered_empty_result() {
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
                        "stale": false,
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
            "dashboard.troubleshooting.summary",
            &json!({
                "error_filter": ["web_tool_surface_*"]
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.pointer("/recent/entry_count").and_then(Value::as_u64),
            Some(0)
        );
        assert_eq!(
            payload.pointer("/filters/no_match").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_by_error_alias_routes_to_summary() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.by_error",
            &json!({
                "error_filter": ["web_tool_not_found"]
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
    fn dashboard_troubleshooting_outbox_state_reports_age_and_max_attempts() {
        let root = tempfile::tempdir().expect("tempdir");
        let outbox_path = root.path().join(DASHBOARD_TROUBLESHOOTING_ISSUE_OUTBOX_REL);
        if let Some(parent) = outbox_path.parent() {
            fs::create_dir_all(parent).expect("mkdir outbox");
        }
        let now_epoch = dashboard_troubleshooting_now_epoch_seconds();
        fs::write(
            &outbox_path,
            serde_json::to_string_pretty(&json!({
                "type": "dashboard_troubleshooting_issue_outbox",
                "items": [
                    {
                        "id": "a",
                        "attempts": 2,
                        "queued_at_epoch_s": now_epoch - 120,
                        "next_retry_after_epoch_s": now_epoch - 5
                    },
                    {
                        "id": "b",
                        "attempts": 5,
                        "queued_at_epoch_s": now_epoch - 60,
                        "next_retry_after_epoch_s": now_epoch + 60
                    }
                ]
            }))
            .expect("json"),
        )
        .expect("write");

        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.state",
            &json!({}),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.get("max_attempts_observed").and_then(Value::as_i64),
            Some(5)
        );
        assert!(
            payload
                .get("oldest_age_seconds")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= 120
        );
        assert_eq!(
            payload.get("ready_ratio").and_then(Value::as_f64),
            Some(0.5)
        );
        assert_eq!(
            payload.get("blocked_ratio").and_then(Value::as_f64),
            Some(0.5)
        );
        assert_eq!(
            payload.get("oldest_item_id").and_then(Value::as_str),
            Some("a")
        );
        assert_eq!(
            payload.get("next_retry_item_id").and_then(Value::as_str),
            Some("b")
        );
        assert_eq!(
            payload
                .get("retry_due_within_900s_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(payload.get("stale_count").and_then(Value::as_u64), Some(0));
        assert_eq!(payload.get("stale_ratio").and_then(Value::as_f64), Some(0.0));
        assert_eq!(payload.get("fresh_count").and_then(Value::as_u64), Some(2));
        assert_eq!(payload.get("fresh_ratio").and_then(Value::as_f64), Some(1.0));
        assert_eq!(payload.get("aging_count").and_then(Value::as_u64), Some(0));
        assert_eq!(payload.get("aging_ratio").and_then(Value::as_f64), Some(0.0));
        assert_eq!(
            payload.get("queue_action_hint").and_then(Value::as_str),
            Some("increase_flush_frequency_and_monitor_auth")
        );
        assert_eq!(
            payload
                .get("queue_pressure_escalation_required")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload
                .get("queue_pressure_escalation_reason")
                .and_then(Value::as_str),
            Some("none")
        );
        assert_eq!(
            payload
                .get("queue_pressure_runbook_id")
                .and_then(Value::as_str),
            Some("runbook.troubleshooting.queue_pressure.medium")
        );
        assert_eq!(
            payload
                .get("queue_pressure_escalation_owner")
                .and_then(Value::as_str),
            Some("runtime_owner")
        );
        assert_eq!(
            payload
                .get("queue_pressure_sla_minutes")
                .and_then(Value::as_i64),
            Some(15)
        );
        assert_eq!(
            payload
                .get("queue_pressure_escalation_lane")
                .and_then(Value::as_str),
            Some("dashboard.troubleshooting.eval.drain")
        );
        assert!(
            payload
                .get("queue_pressure_deadline_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= now_epoch
        );
        assert!(
            payload
                .get("queue_pressure_deadline_remaining_seconds")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                > 0
        );
        assert_eq!(
            payload
                .get("queue_pressure_breach")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload
                .get("queue_pressure_breach_reason")
                .and_then(Value::as_str),
            Some("none")
        );
        assert_eq!(
            payload
                .get("queue_pressure_breach_detected_at_epoch_s")
                .and_then(Value::as_i64),
            Some(0)
        );
        assert_eq!(
            payload
                .get("queue_pressure_contract_version")
                .and_then(Value::as_str),
            Some("v1")
        );
        assert!(
            payload
                .get("queue_pressure_snapshot_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= now_epoch
        );
        assert_eq!(
            payload.get("health_reason").and_then(Value::as_str),
            Some("ready_ratio>=0.40_with_some_cooldown_pressure")
        );
        assert_eq!(
            payload.pointer("/items/0/source_sequence").and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            payload.pointer("/items/0/stale").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload.pointer("/items/0/freshness_tier").and_then(Value::as_str),
            Some("fresh")
        );
        assert_eq!(
            payload.pointer("/items/0/source").and_then(Value::as_str),
            Some("issue_outbox")
        );
        assert!(
            payload
                .pointer("/items/0/age_seconds")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= 60
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_health_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.health",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_overview_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.overview",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

