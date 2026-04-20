    #[test]
    fn dashboard_troubleshooting_summary_pressure_decision_lane_token_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.decision_lane_token",
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
    fn dashboard_troubleshooting_outbox_pressure_decision_dispatch_mode_alias_routes_to_state_lane()
    {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.decision_dispatch_mode",
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
    fn dashboard_troubleshooting_summary_pressure_decision_dispatch_mode_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.decision_dispatch_mode",
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
    fn dashboard_troubleshooting_outbox_pressure_decision_manual_ack_required_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.decision_manual_ack_required",
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
    fn dashboard_troubleshooting_summary_pressure_decision_manual_ack_required_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.decision_manual_ack_required",
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
    fn dashboard_troubleshooting_outbox_pressure_decision_execution_guard_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.decision_execution_guard",
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
    fn dashboard_troubleshooting_summary_pressure_decision_execution_guard_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.decision_execution_guard",
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
    fn dashboard_troubleshooting_outbox_pressure_decision_followup_required_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.decision_followup_required",
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
    fn dashboard_troubleshooting_summary_pressure_decision_followup_required_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.decision_followup_required",
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
    fn dashboard_troubleshooting_outbox_pressure_contract_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract",
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
    fn dashboard_troubleshooting_summary_pressure_contract_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract",
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
    fn dashboard_troubleshooting_outbox_pressure_contract_version_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_version",
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
    fn dashboard_troubleshooting_summary_pressure_contract_version_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_version",
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
    fn dashboard_troubleshooting_outbox_pressure_contract_family_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_family",
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
    fn dashboard_troubleshooting_summary_pressure_contract_family_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_family",
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
    fn dashboard_troubleshooting_outbox_pressure_contract_priority_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_priority",
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
    fn dashboard_troubleshooting_summary_pressure_contract_priority_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_priority",
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
    fn dashboard_troubleshooting_outbox_pressure_contract_action_hint_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_action_hint",
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
    fn dashboard_troubleshooting_summary_pressure_contract_action_hint_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_action_hint",
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
    fn dashboard_troubleshooting_outbox_pressure_contract_escalation_lane_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_escalation_lane",
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
    fn dashboard_troubleshooting_summary_pressure_contract_escalation_lane_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_escalation_lane",
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
    fn dashboard_troubleshooting_outbox_pressure_contract_runbook_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_runbook",
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

