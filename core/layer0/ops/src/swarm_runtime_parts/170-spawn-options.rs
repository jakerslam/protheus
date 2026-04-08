#[cfg(test)]
mod tests {
    use super::*;

    fn spawn_options() -> SpawnOptions {
        SpawnOptions {
            verify: true,
            timeout_ms: 100,
            metrics_detailed: true,
            simulate_unreachable: false,
            byzantine: false,
            corruption_type: "data_falsification".to_string(),
            token_budget: None,
            token_warning_threshold: 0.8,
            budget_exhaustion_action: BudgetAction::FailHard,
            adaptive_complexity: false,
            execution_mode: ExecutionMode::TaskOriented,
            role: None,
            capabilities: Vec::new(),
            auto_publish_results: false,
            agent_label: None,
            result_value: None,
            result_text: None,
            result_confidence: 1.0,
            verification_status: "not_verified".to_string(),
        }
    }

    #[test]
    fn recursive_spawn_tracks_parent_and_children() {
        let mut state = SwarmState::default();
        let options = spawn_options();
        let result = recursive_spawn_with_tracking(&mut state, None, "task", 3, 6, &options)
            .expect("recursive spawn should succeed");
        assert_eq!(
            result
                .get("lineage")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(3)
        );

        let lineage = result
            .get("lineage")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let first = lineage
            .first()
            .and_then(|row| row.get("session_id"))
            .and_then(Value::as_str)
            .expect("first session id");
        let second = lineage
            .get(1)
            .and_then(|row| row.get("session_id"))
            .and_then(Value::as_str)
            .expect("second session id");
        let first_session = state.sessions.get(first).expect("first session exists");
        assert_eq!(first_session.children, vec![second.to_string()]);
    }

    #[test]
    fn spawn_verify_fails_when_child_is_unreachable() {
        let mut state = SwarmState::default();
        let mut options = spawn_options();
        options.simulate_unreachable = true;
        let err = spawn_single(&mut state, None, "task", 4, &options).expect_err("must fail");
        assert!(
            err.contains("session_unreachable_timeout"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn consensus_detector_marks_outliers() {
        let reports = vec![
            AgentReport {
                agent_id: "a1".to_string(),
                values: BTreeMap::from([
                    ("file_size".to_string(), json!(1847)),
                    ("word_count".to_string(), json!(292)),
                ]),
            },
            AgentReport {
                agent_id: "a2".to_string(),
                values: BTreeMap::from([
                    ("file_size".to_string(), json!(1847)),
                    ("word_count".to_string(), json!(292)),
                ]),
            },
            AgentReport {
                agent_id: "a3".to_string(),
                values: BTreeMap::from([
                    ("file_size".to_string(), json!(9999)),
                    ("word_count".to_string(), json!(5000)),
                ]),
            },
        ];
        let fields = vec!["file_size".to_string(), "word_count".to_string()];
        let result = evaluate_consensus(&reports, &fields, 0.6);
        assert_eq!(
            result.get("consensus_reached").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            result
                .get("outliers")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
        assert_eq!(
            result.get("reason_code").and_then(Value::as_str),
            Some("majority_with_outliers")
        );
        assert_eq!(
            result.get("recommended_action").and_then(Value::as_str),
            Some("accept_with_outlier_review")
        );
        assert_eq!(
            result.get("confidence_band").and_then(Value::as_str),
            Some("medium")
        );
    }

    #[test]
    fn numeric_outlier_analysis_emits_robust_stats() {
        let results = vec![
            AgentResult {
                result_id: "r1".to_string(),
                session_id: "s1".to_string(),
                agent_label: "a1".to_string(),
                agent_role: "worker".to_string(),
                task_id: "task-1".to_string(),
                payload: ResultPayload::Calculation { value: 10.0 },
                data: json!({"value": 10.0}),
                confidence: 0.9,
                verification_status: "verified".to_string(),
                timestamp_ms: 1,
                created_at: "2026-01-01T00:00:00Z".to_string(),
            },
            AgentResult {
                result_id: "r2".to_string(),
                session_id: "s2".to_string(),
                agent_label: "a2".to_string(),
                agent_role: "worker".to_string(),
                task_id: "task-1".to_string(),
                payload: ResultPayload::Calculation { value: 10.1 },
                data: json!({"value": 10.1}),
                confidence: 0.9,
                verification_status: "verified".to_string(),
                timestamp_ms: 2,
                created_at: "2026-01-01T00:00:00Z".to_string(),
            },
            AgentResult {
                result_id: "r3".to_string(),
                session_id: "s3".to_string(),
                agent_label: "a3".to_string(),
                agent_role: "worker".to_string(),
                task_id: "task-1".to_string(),
                payload: ResultPayload::Calculation { value: 42.0 },
                data: json!({"value": 42.0}),
                confidence: 0.9,
                verification_status: "verified".to_string(),
                timestamp_ms: 3,
                created_at: "2026-01-01T00:00:00Z".to_string(),
            },
        ];
        let outliers = analyze_result_outliers(&results, "value");
        assert_eq!(
            outliers.get("status").and_then(Value::as_str),
            Some("outliers_detected")
        );
        assert!(outliers.get("median").is_some());
        assert!(outliers.get("mad").is_some());
        let first_outlier = outliers
            .get("outliers")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .cloned()
            .unwrap_or(Value::Null);
        assert!(first_outlier.get("robust_z_score").is_some());
    }

    #[test]
    fn byzantine_requires_test_mode() {
        let mut state = SwarmState::default();
        let mut options = spawn_options();
        options.byzantine = true;
        let err = spawn_single(&mut state, None, "task", 5, &options)
            .expect_err("byzantine must fail without test mode");
        assert_eq!(err, "byzantine_test_mode_required");

        state.byzantine_test_mode = true;
        let ok = spawn_single(&mut state, None, "task", 5, &options)
            .expect("byzantine should pass in test mode");
        assert_eq!(
            ok.get("report")
                .and_then(|v| v.get("corrupted"))
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn detailed_metrics_emit_breakdown_fields() {
        let mut state = SwarmState::default();
        let options = spawn_options();
        let ok = spawn_single(&mut state, None, "task", 5, &options).expect("spawn ok");
        let metrics = ok.get("metrics").cloned().unwrap_or(Value::Null);
        assert!(metrics.get("queue_wait_ms").is_some());
        assert!(metrics.get("execution_time_ms").is_some());
        assert!(metrics.get("total_latency_ms").is_some());
    }

    #[test]
    fn token_budget_fail_hard_enforced() {
        let mut state = SwarmState::default();
        let mut options = spawn_options();
        options.token_budget = Some(100);
        options.budget_exhaustion_action = BudgetAction::FailHard;
        let err = spawn_single(
            &mut state,
            None,
            "write detailed and exhaustive analysis with examples and references",
            5,
            &options,
        )
        .expect_err("budget should hard fail");
        assert!(
            err.contains("token_budget_exceeded"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn adaptive_task_scaling_applies_for_small_budget() {
        let mut state = SwarmState::default();
        let mut options = spawn_options();
        options.token_budget = Some(200);
        options.adaptive_complexity = true;
        options.budget_exhaustion_action = BudgetAction::AllowWithWarning;

        let ok = spawn_single(&mut state, None, "Analyze file", 5, &options).expect("spawn ok");
        let report_task = ok
            .get("report")
            .and_then(|value| value.get("task"))
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(
            report_task.contains("ultra-concise"),
            "expected scaled task annotation, got: {report_task}"
        );
    }

    #[test]
    fn sessions_send_allows_sibling_delivery() {
        let mut state = SwarmState::default();
        let parent = spawn_single(&mut state, None, "parent", 8, &spawn_options())
            .expect("parent spawn")
            .get("session_id")
            .and_then(Value::as_str)
            .expect("parent id")
            .to_string();
        let child_a = spawn_single(&mut state, Some(&parent), "child-a", 8, &spawn_options())
            .expect("child-a")
            .get("session_id")
            .and_then(Value::as_str)
            .expect("child-a id")
            .to_string();
        let child_b = spawn_single(&mut state, Some(&parent), "child-b", 8, &spawn_options())
            .expect("child-b")
            .get("session_id")
            .and_then(Value::as_str)
            .expect("child-b id")
            .to_string();

        let delivered = send_session_message(
            &mut state,
            &child_a,
            &child_b,
            "hello",
            DeliveryGuarantee::AtMostOnce,
            false,
            DEFAULT_MESSAGE_TTL_MS,
        )
        .expect("sibling message should be delivered");
        assert_eq!(
            delivered
                .get("recipient_session_id")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            Some(child_b.clone())
        );
        let inbox = state.mailboxes.get(&child_b).expect("mailbox").unread.len();
        assert_eq!(inbox, 1);
    }

    #[test]
    fn exactly_once_dedupes_repeat_messages() {
        let mut state = SwarmState::default();
        let a = spawn_single(&mut state, None, "sender", 8, &spawn_options())
            .expect("sender spawn")
            .get("session_id")
            .and_then(Value::as_str)
            .expect("sender id")
            .to_string();
        let b = spawn_single(&mut state, None, "receiver", 8, &spawn_options())
            .expect("receiver spawn")
            .get("session_id")
            .and_then(Value::as_str)
            .expect("receiver id")
            .to_string();

        let first = send_session_message(
            &mut state,
            "coordinator",
            &b,
            "dedupe-message",
            DeliveryGuarantee::ExactlyOnce,
            false,
            DEFAULT_MESSAGE_TTL_MS,
        )
        .expect("first send");
        let second = send_session_message(
            &mut state,
            "coordinator",
            &b,
            "dedupe-message",
            DeliveryGuarantee::ExactlyOnce,
            false,
            DEFAULT_MESSAGE_TTL_MS,
        )
        .expect("second send");
        assert_eq!(
            second.get("dedupe_hit").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            first.get("message_id").and_then(Value::as_str),
            second.get("message_id").and_then(Value::as_str)
        );
        let inbox_len = state.mailboxes.get(&b).expect("mailbox").unread.len();
        assert_eq!(inbox_len, 1, "exactly-once must avoid duplicate inbox rows");
        assert!(state.sessions.contains_key(&a));
    }

    #[test]
    fn wildcard_label_match_supports_prefix_suffix_patterns() {
        assert!(wildcard_matches(
            "swarm-test-7-*",
            "swarm-test-7-agent-fast"
        ));
        assert!(wildcard_matches("*agent-fast", "swarm-test-7-agent-fast"));
        assert!(!wildcard_matches(
            "swarm-test-8-*",
            "swarm-test-7-agent-fast"
        ));
    }

    #[test]
    fn auto_publish_results_are_queryable_and_consensus_detects_mismatch() {
        let mut state = SwarmState::default();
        let mut first = spawn_options();
        first.auto_publish_results = true;
        first.result_value = Some(5050.0);
        first.agent_label = Some("swarm-test-7-het-agent-fast".to_string());
        first.role = Some("calculator".to_string());
        spawn_single(&mut state, None, "calc-fast", 8, &first).expect("first spawn");

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
}
