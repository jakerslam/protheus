    #[test]
    fn dashboard_troubleshooting_outbox_pressure_reason_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.reason",
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
    fn dashboard_troubleshooting_summary_pressure_reason_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.reason",
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
    fn dashboard_troubleshooting_outbox_pressure_breach_reason_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.breach_reason",
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
    fn dashboard_troubleshooting_summary_pressure_breach_reason_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.breach_reason",
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
    fn dashboard_troubleshooting_outbox_pressure_blocking_kind_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.blocking_kind",
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
    fn dashboard_troubleshooting_summary_pressure_blocking_kind_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.blocking_kind",
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
    fn dashboard_troubleshooting_outbox_pressure_auto_retry_allowed_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.auto_retry_allowed",
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
    fn dashboard_troubleshooting_summary_pressure_auto_retry_allowed_alias_routes_to_summary_lane()
    {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.auto_retry_allowed",
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
    fn dashboard_troubleshooting_outbox_pressure_execution_policy_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.execution_policy",
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
    fn dashboard_troubleshooting_summary_pressure_execution_policy_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.execution_policy",
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
    fn dashboard_troubleshooting_outbox_pressure_manual_gate_required_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.manual_gate_required",
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
    fn dashboard_troubleshooting_summary_pressure_manual_gate_required_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.manual_gate_required",
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
    fn dashboard_troubleshooting_outbox_pressure_manual_gate_reason_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.manual_gate_reason",
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
    fn dashboard_troubleshooting_summary_pressure_manual_gate_reason_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.manual_gate_reason",
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
    fn dashboard_troubleshooting_outbox_pressure_requeue_strategy_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.requeue_strategy",
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
    fn dashboard_troubleshooting_summary_pressure_requeue_strategy_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.requeue_strategy",
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
    fn dashboard_troubleshooting_outbox_pressure_can_execute_without_human_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.can_execute_without_human",
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
    fn dashboard_troubleshooting_summary_pressure_can_execute_without_human_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.can_execute_without_human",
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
    fn dashboard_troubleshooting_outbox_pressure_execution_window_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.execution_window",
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
    fn dashboard_troubleshooting_summary_pressure_execution_window_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.execution_window",
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
    fn dashboard_troubleshooting_outbox_pressure_manual_gate_timeout_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.manual_gate_timeout",
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
    fn dashboard_troubleshooting_summary_pressure_manual_gate_timeout_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.manual_gate_timeout",
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

