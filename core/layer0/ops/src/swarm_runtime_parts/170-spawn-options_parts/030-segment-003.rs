        let mut second = spawn_options();
        second.auto_publish_results = true;
        second.result_value = Some(5000.0);
        second.agent_label = Some("swarm-test-7-het-agent-thorough".to_string());
        second.role = Some("calculator".to_string());
        spawn_single(&mut state, None, "calc-thorough", 8, &second).expect("second spawn");

        let results = query_results(
            &state,
            &ResultFilters {
                label_pattern: Some("swarm-test-7-het-agent-*".to_string()),
                role: Some("calculator".to_string()),
                task_id: None,
                session_id: None,
            },
        );
        assert_eq!(results.len(), 2);

        let consensus = analyze_result_consensus(&results, "value", 1.0);
        assert_eq!(
            consensus
                .get("consensus_reached")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            false
        );
        let outliers = consensus
            .get("outliers")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(outliers.len(), 1);
    }

    #[test]
    fn sessions_state_includes_context_and_tool_history() {
        let mut state = SwarmState::default();
        let mut options = spawn_options();
        options.token_budget = Some(1000);
        options.budget_exhaustion_action = BudgetAction::AllowWithWarning;
        options.role = Some("calculator".to_string());
        options.capabilities = vec!["calculate".to_string(), "verify".to_string()];

        let session_id = spawn_single(&mut state, None, "state-introspection", 8, &options)
            .expect("spawn")
            .get("session_id")
            .and_then(Value::as_str)
            .expect("session_id")
            .to_string();

        let snapshot = sessions_state(&state, &session_id, true, 8).expect("session state");
        assert_eq!(
            snapshot.get("type").and_then(Value::as_str),
            Some("swarm_runtime_session_state")
        );
        assert!(
            snapshot
                .get("context")
                .and_then(|row| row.get("utilization_pct"))
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                >= 0.0
        );
        assert!(
            snapshot
                .get("tool_call_history")
                .and_then(Value::as_array)
                .map(|rows| !rows.is_empty())
                .unwrap_or(false),
            "expected tool call history for spawned session"
        );
    }

    #[test]
    fn queue_metrics_prometheus_export_contains_expected_fields() {
        let mut state = SwarmState::default();
        for idx in 0..3 {
            let task = format!("queue-metrics-{idx}");
            let _ = spawn_single(&mut state, None, &task, 8, &spawn_options()).expect("spawn");
        }

        let snapshot = queue_metrics_snapshot(&state);
        let exported = queue_metrics_prometheus(&state, &snapshot);
        assert!(exported.contains("swarm_runtime_queue_wait_ms_avg"));
        assert!(exported.contains("swarm_runtime_execution_ms_p95"));
        assert!(exported.contains("swarm_runtime_sessions_total"));
    }

    #[test]
    fn scale_plan_reports_100k_ready_topology_under_default_policy() {
        let state = SwarmState::default();
        let fanout = recommended_manager_fanout_for_target(100_000);
        let readiness = evaluate_scale_policy_readiness(&state, 100_000, fanout);
        assert_eq!(
            readiness
                .get("readiness")
                .and_then(|row| row.get("within_session_cap"))
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            readiness
                .get("readiness")
                .and_then(|row| row.get("within_depth_cap"))
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            readiness
                .get("topology")
                .and_then(|row| row.get("target_agents"))
                .and_then(Value::as_u64),
            Some(100_000)
        );
    }

    #[test]
    fn spawn_enforces_parent_capacity_when_policy_enabled() {
        let mut state = SwarmState::default();
        state.scale_policy.max_children_per_parent = 2;
        state.scale_policy.enforce_parent_capacity = true;

        let root = spawn_single(&mut state, None, "root", 8, &spawn_options())
            .expect("root spawn")
            .get("session_id")
            .and_then(Value::as_str)
            .expect("root id")
            .to_string();

        spawn_single(&mut state, Some(&root), "child-1", 8, &spawn_options()).expect("child 1");
        spawn_single(&mut state, Some(&root), "child-2", 8, &spawn_options()).expect("child 2");

        let err = spawn_single(&mut state, Some(&root), "child-3", 8, &spawn_options())
            .expect_err("third child should exceed capacity");
        assert!(
            err.contains("parent_capacity_exceeded"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn spawn_emits_role_card_and_enforces_capability_envelope() {
        let mut state = SwarmState::default();
        let mut options = spawn_options();
        options.role = Some("validator".to_string());
        options.capabilities = vec![
            "validate".to_string(),
            "audit".to_string(),
            "invalid capability!".to_string(),
            "validate".to_string(),
        ];

        let spawned = spawn_single(&mut state, None, "validate deployment plan", 8, &options)
            .expect("spawn should succeed");
        let role_card = spawned
            .get("role_card")
            .cloned()
            .unwrap_or(Value::Null);
        assert_eq!(role_card.get("role").and_then(Value::as_str), Some("validator"));
        assert_eq!(
            role_card.get("goal").and_then(Value::as_str),
            Some("validate deployment plan")
        );
        assert_eq!(
            role_card
                .get("capability_envelope")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(2)
        );

        let service_caps = state
            .service_registry
            .get("validator")
            .and_then(|rows| rows.first())
            .map(|instance| instance.capabilities.clone())
            .unwrap_or_default();
        assert_eq!(service_caps, vec!["audit".to_string(), "validate".to_string()]);
    }

