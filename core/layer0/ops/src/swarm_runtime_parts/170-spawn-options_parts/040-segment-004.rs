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

