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

    fn calc_result(
        result_id: &str,
        session_id: &str,
        agent_label: &str,
        value: f64,
        timestamp_ms: u64,
    ) -> AgentResult {
        AgentResult {
            result_id: result_id.to_string(),
            session_id: session_id.to_string(),
            agent_label: agent_label.to_string(),
            agent_role: "worker".to_string(),
            task_id: "task-1".to_string(),
            payload: ResultPayload::Calculation { value },
            data: json!({"value": value}),
            confidence: 0.9,
            verification_status: "verified".to_string(),
            timestamp_ms,
            created_at: "2026-01-01T00:00:00Z".to_string(),
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
            calc_result("r1", "s1", "a1", 10.0, 1),
            calc_result("r2", "s2", "a2", 10.1, 2),
            calc_result("r3", "s3", "a3", 42.0, 3),
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

    #[test]
    fn spawn_fails_closed_on_parent_lineage_cycle() {
        let mut state = SwarmState::default();
        let mut a = session_metadata_base(
            "cycle-a".to_string(),
            Some("cycle-b".to_string()),
            0,
            "cycle-a-task".to_string(),
            "running".to_string(),
        );
        let mut b = session_metadata_base(
            "cycle-b".to_string(),
            Some("cycle-a".to_string()),
            1,
            "cycle-b-task".to_string(),
            "running".to_string(),
        );
        a.reachable = true;
        b.reachable = true;
        state.sessions.insert("cycle-a".to_string(), a);
        state.sessions.insert("cycle-b".to_string(), b);

        let err = spawn_single(&mut state, Some("cycle-a"), "child", 8, &spawn_options())
            .expect_err("cycle should be blocked");
        assert_eq!(err, "lineage_cycle_detected");
    }

    #[test]
    fn tick_uses_should_terminate_contract_for_goal_met() {
        let mut state = SwarmState::default();
        let mut options = spawn_options();
        options.role = Some("worker".to_string());
        let cfg = PersistentAgentConfig {
            lifespan_sec: 3600,
            check_in_interval_sec: 30,
            report_mode: ReportMode::Always,
        };

        let session_id = spawn_persistent_session(
            &mut state,
            None,
            "goal-tracking-task",
            8,
            &options,
            &cfg,
            false,
        )
        .expect("persistent spawn")
        .get("session_id")
        .and_then(Value::as_str)
        .expect("session id")
        .to_string();

        if let Some(session) = state.sessions.get_mut(&session_id) {
            session
                .context_vars
                .insert("goal_met".to_string(), Value::Bool(true));
        }

        let tick = tick_persistent_sessions(&mut state, now_epoch_ms(), 4).expect("tick");
        let report_row = tick
            .get("reports")
            .and_then(Value::as_array)
            .and_then(|rows| {
                rows.iter().find(|row| {
                    row.get("session_id")
                        .and_then(Value::as_str)
                        .map(|value| value == session_id)
                        .unwrap_or(false)
                })
            })
            .cloned()
            .unwrap_or(Value::Null);

        assert_eq!(
            report_row
                .get("should_terminate")
                .and_then(|row| row.get("reason"))
                .and_then(Value::as_str),
            Some("goal_met")
        );
        let terminated_reason = state
            .sessions
            .get(&session_id)
            .and_then(|session| session.persistent.as_ref())
            .and_then(|runtime| runtime.terminated_reason.as_deref());
        assert_eq!(terminated_reason, Some("goal_met"));
    }

    fn argv(rows: &[&str]) -> Vec<String> {
        rows.iter().map(|row| (*row).to_string()).collect::<Vec<_>>()
    }

    #[test]
    fn plans_start_creates_supervisor_and_task_graph() {
        let mut state = SwarmState::default();
        let output = run_plans_command(
            &mut state,
            &argv(&[
                "plans",
                "start",
                "--goal=ship reliable workflow gates, add eval traces, harden retries",
                "--plan-max-depth=4",
            ]),
        )
        .expect("plan start");
        assert_eq!(
            output.get("type").and_then(Value::as_str),
            Some("swarm_runtime_plan_start")
        );
        let plan = output.get("plan").cloned().unwrap_or(Value::Null);
        assert_eq!(plan.get("status").and_then(Value::as_str), Some("running"));
        assert!(
            plan.get("nodes")
                .and_then(Value::as_object)
                .map(|nodes| nodes.len() >= 2)
                .unwrap_or(false)
        );
    }

    #[test]
    fn plans_advance_supports_recursive_replan_loop() {
        let mut state = SwarmState::default();
        let started = run_plans_command(
            &mut state,
            &argv(&[
                "plans",
                "start",
                "--goal=resolve blocked dependency and continue execution",
                "--plan-max-depth=4",
            ]),
        )
        .expect("plan start");
        let plan_id = started
            .get("plan")
            .and_then(|row| row.get("plan_id"))
            .and_then(Value::as_str)
            .expect("plan id");

        let advanced = run_plans_command(
            &mut state,
            &argv(&[
                "plans",
                "advance",
                &format!("--plan-id={plan_id}"),
                "--max-steps=2",
                "--allow-replan=1",
                "--simulate-blocked=1",
            ]),
        )
        .expect("plan advance");
        assert_eq!(
            advanced.get("steps_executed").and_then(Value::as_u64),
            Some(2)
        );
        assert!(
            advanced
                .get("replan_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                >= 1
        );
    }

    #[test]
    fn plans_checkpoint_supports_save_and_resume() {
        let mut state = SwarmState::default();
        let started = run_plans_command(
            &mut state,
            &argv(&[
                "plans",
                "start",
                "--goal=checkpoint test goal",
                "--plan-max-depth=3",
            ]),
        )
        .expect("plan start");
        let plan = started.get("plan").cloned().unwrap_or(Value::Null);
        let plan_id = plan
            .get("plan_id")
            .and_then(Value::as_str)
            .expect("plan id")
            .to_string();
        let node_id = plan
            .get("nodes")
            .and_then(Value::as_object)
            .and_then(|nodes| nodes.keys().find(|row| row.ends_with("-root")).cloned())
            .expect("root node id");

        let checkpoint = run_plans_command(
            &mut state,
            &argv(&[
                "plans",
                "checkpoint",
                &format!("--plan-id={plan_id}"),
                &format!("--node-id={node_id}"),
                "--state-json={\"progress\":0.5}",
            ]),
        )
        .expect("checkpoint save");
        let checkpoint_id = checkpoint
            .get("checkpoint")
            .and_then(|row| row.get("checkpoint_id"))
            .and_then(Value::as_str)
            .expect("checkpoint id")
            .to_string();

        let resumed = run_plans_command(
            &mut state,
            &argv(&[
                "plans",
                "checkpoint",
                &format!("--plan-id={plan_id}"),
                &format!("--checkpoint-id={checkpoint_id}"),
            ]),
        )
        .expect("checkpoint resume");
        assert_eq!(
            resumed.get("type").and_then(Value::as_str),
            Some("swarm_runtime_plan_checkpoint_resume")
        );
    }

    #[test]
    fn plans_branch_gate_waits_or_approves_deterministically() {
        let mut state = SwarmState::default();
        let started = run_plans_command(
            &mut state,
            &argv(&["plans", "start", "--goal=branch gate policy test"]),
        )
        .expect("plan start");
        let plan = started.get("plan").cloned().unwrap_or(Value::Null);
        let plan_id = plan
            .get("plan_id")
            .and_then(Value::as_str)
            .expect("plan id")
            .to_string();
        let node_id = plan
            .get("nodes")
            .and_then(Value::as_object)
            .and_then(|nodes| nodes.keys().find(|row| row.ends_with("-root")).cloned())
            .expect("node id");

        let waiting = run_plans_command(
            &mut state,
            &argv(&[
                "plans",
                "branch-gate",
                &format!("--plan-id={plan_id}"),
                &format!("--node-id={node_id}"),
                "--wait-user=1",
                "--decision=auto",
            ]),
        )
        .expect("branch gate waiting");
        assert_eq!(
            waiting
                .get("gate")
                .and_then(|row| row.get("status"))
                .and_then(Value::as_str),
            Some("waiting_user")
        );

        let approved = run_plans_command(
            &mut state,
            &argv(&[
                "plans",
                "branch-gate",
                &format!("--plan-id={plan_id}"),
                &format!("--node-id={node_id}"),
                "--decision=approve",
            ]),
        )
        .expect("branch gate approve");
        assert_eq!(
            approved
                .get("gate")
                .and_then(|row| row.get("status"))
                .and_then(Value::as_str),
            Some("approved")
        );
    }

    #[test]
    fn plans_speaker_selection_prefers_matching_expertise() {
        let mut state = SwarmState::default();
        let mut options = spawn_options();
        options.role = Some("analyst".to_string());
        options.capabilities = vec!["analyze".to_string(), "audit".to_string()];
        let analyst = spawn_single(&mut state, None, "analyst worker", 8, &options)
            .expect("analyst")
            .get("session_id")
            .and_then(Value::as_str)
            .expect("analyst id")
            .to_string();

        let mut options2 = spawn_options();
        options2.role = Some("researcher".to_string());
        options2.capabilities = vec!["research".to_string(), "search".to_string()];
        let researcher = spawn_single(&mut state, None, "research worker", 8, &options2)
            .expect("researcher")
            .get("session_id")
            .and_then(Value::as_str)
            .expect("researcher id")
            .to_string();

        let started = run_plans_command(
            &mut state,
            &argv(&[
                "plans",
                "start",
                "--goal=select speaker for research-heavy user request",
                &format!("--session-id={analyst}"),
            ]),
        )
        .expect("plan start");
        let plan_id = started
            .get("plan")
            .and_then(|row| row.get("plan_id"))
            .and_then(Value::as_str)
            .expect("plan id");

        let selected = run_plans_command(
            &mut state,
            &argv(&[
                "plans",
                "speaker-select",
                &format!("--plan-id={plan_id}"),
                "--message=Need research and search synthesis for this topic",
                &format!("--candidate-session-ids={analyst},{researcher}"),
            ]),
        )
        .expect("speaker select");
        assert_eq!(
            selected
                .get("selected")
                .and_then(|row| row.get("session_id"))
                .and_then(Value::as_str),
            Some(researcher.as_str())
        );
    }
}
