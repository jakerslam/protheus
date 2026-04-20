    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_next_action_after_seconds_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_next_action_after_seconds",
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
    fn dashboard_troubleshooting_outbox_pressure_contract_next_action_kind_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_next_action_kind",
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
    fn dashboard_troubleshooting_summary_pressure_contract_next_action_kind_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_next_action_kind",
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
    fn dashboard_troubleshooting_outbox_pressure_contract_retry_window_class_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_retry_window_class",
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
    fn dashboard_troubleshooting_summary_pressure_contract_retry_window_class_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_retry_window_class",
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
    fn dashboard_troubleshooting_outbox_pressure_contract_readiness_state_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_readiness_state",
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
    fn dashboard_troubleshooting_summary_pressure_contract_readiness_state_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_readiness_state",
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
    fn dashboard_troubleshooting_outbox_pressure_contract_readiness_reason_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_readiness_reason",
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
    fn dashboard_troubleshooting_summary_pressure_contract_readiness_reason_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_readiness_reason",
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
    fn dashboard_troubleshooting_outbox_pressure_contract_automation_safe_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_automation_safe",
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
    fn dashboard_troubleshooting_summary_pressure_contract_automation_safe_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_automation_safe",
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
    fn dashboard_troubleshooting_outbox_pressure_contract_decision_vector_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_decision_vector",
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
    fn dashboard_troubleshooting_summary_pressure_contract_decision_vector_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_decision_vector",
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
    fn dashboard_troubleshooting_outbox_pressure_contract_decision_vector_key_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_decision_vector_key",
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
    fn dashboard_troubleshooting_summary_pressure_contract_decision_vector_key_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_decision_vector_key",
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
    fn dashboard_troubleshooting_outbox_pressure_contract_decision_route_hint_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_decision_route_hint",
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
    fn dashboard_troubleshooting_summary_pressure_contract_decision_route_hint_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_decision_route_hint",
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
    fn dashboard_troubleshooting_outbox_pressure_contract_decision_urgency_tier_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_decision_urgency_tier",
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
    fn dashboard_troubleshooting_summary_pressure_contract_decision_urgency_tier_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_decision_urgency_tier",
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
    fn dashboard_troubleshooting_outbox_pressure_contract_decision_retry_budget_class_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_decision_retry_budget_class",
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
    fn dashboard_troubleshooting_summary_pressure_contract_decision_retry_budget_class_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_decision_retry_budget_class",
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
    fn dashboard_troubleshooting_outbox_pressure_contract_decision_lane_token_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_decision_lane_token",
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

