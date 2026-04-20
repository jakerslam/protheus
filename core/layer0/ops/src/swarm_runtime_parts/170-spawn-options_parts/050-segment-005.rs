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
