    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_decision_lane_token_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_decision_lane_token",
            &json!({}),
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
    fn dashboard_troubleshooting_outbox_pressure_contract_decision_dispatch_mode_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_decision_dispatch_mode",
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
    fn dashboard_troubleshooting_summary_pressure_contract_decision_dispatch_mode_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_decision_dispatch_mode",
            &json!({}),
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
    fn dashboard_troubleshooting_outbox_pressure_contract_decision_manual_ack_required_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_decision_manual_ack_required",
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
    fn dashboard_troubleshooting_summary_pressure_contract_decision_manual_ack_required_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_decision_manual_ack_required",
            &json!({}),
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
    fn dashboard_troubleshooting_outbox_pressure_contract_decision_execution_guard_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_decision_execution_guard",
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
    fn dashboard_troubleshooting_summary_pressure_contract_decision_execution_guard_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_decision_execution_guard",
            &json!({}),
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
    fn dashboard_troubleshooting_outbox_pressure_contract_decision_followup_required_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_decision_followup_required",
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
    fn dashboard_troubleshooting_summary_pressure_contract_decision_followup_required_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_decision_followup_required",
            &json!({}),
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
    fn dashboard_troubleshooting_outbox_pressure_contract_deadline_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_deadline",
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
    fn dashboard_troubleshooting_summary_pressure_contract_deadline_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_deadline",
            &json!({}),
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
    fn dashboard_troubleshooting_outbox_pressure_contract_breach_reason_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_breach_reason",
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
    fn dashboard_troubleshooting_summary_pressure_contract_breach_reason_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_breach_reason",
            &json!({}),
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
    fn dashboard_troubleshooting_outbox_pressure_snapshot_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.snapshot",
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
    fn dashboard_troubleshooting_summary_pressure_snapshot_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.snapshot",
            &json!({}),
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
    fn dashboard_troubleshooting_summary_outbox_health_reports_pressure_deadline_and_breach() {
        let root = tempfile::tempdir().expect("tempdir");
        let now_epoch = dashboard_troubleshooting_now_epoch_seconds();
        let outbox_path = root.path().join(DASHBOARD_TROUBLESHOOTING_ISSUE_OUTBOX_REL);
        if let Some(parent) = outbox_path.parent() {
            fs::create_dir_all(parent).expect("mkdir outbox");
        }
        fs::write(
            &outbox_path,
            serde_json::to_string_pretty(&json!({
                "type": "dashboard_troubleshooting_issue_outbox",
                "items": [
                    {
                        "id": "blocked-stale-item",
                        "attempts": 6,
                        "queued_at_epoch_s": now_epoch - 4_200,
                        "next_retry_after_epoch_s": now_epoch + 600,
                        "error_bucket": "github_issue_auth_missing"
                    }
                ]
            }))
            .expect("json"),
        )
        .expect("write");

        let lane = run_action(root.path(), "dashboard.troubleshooting.summary", &json!({}));
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_tier")
                .and_then(Value::as_str),
            Some("high")
        );
        assert!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_deadline_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= now_epoch
        );
        assert!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_deadline_remaining_seconds")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                > 0
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_breach")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_breach_reason")
                .and_then(Value::as_str),
            Some("outbox_oldest_age_seconds>1800")
        );
        assert!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_breach_detected_at_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= now_epoch
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
    }

