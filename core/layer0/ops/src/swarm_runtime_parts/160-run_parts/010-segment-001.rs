pub fn run(root: &Path, argv: &[String]) -> i32 {
    let cmd = argv
        .first()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if matches!(cmd.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let state_file = state_path(root, argv);
    let mut state = match load_state(&state_file) {
        Ok(value) => value,
        Err(err) => {
            print_receipt(json!({
                "ok": false,
                "type": "swarm_runtime_error",
                "command": cmd,
                "error": err,
                "state_path": state_file,
            }));
            return 2;
        }
    };
    let now_ms = now_epoch_ms();
    recover_persistent_sessions_after_reload(&mut state, now_ms);
    drain_expired_messages(&mut state, now_ms);
    drain_expired_thorn_cells(&mut state, now_ms);

    let auto_tick_enabled = parse_bool_flag(argv, "auto-tick", true);
    if auto_tick_enabled && cmd != "tick" && !persistent_session_ids(&state).is_empty() {
        let auto_now_ms = now_epoch_ms();
        let auto_max_check_ins = parse_u64_flag(argv, "auto-max-check-ins", 16).max(1);
        let _ = tick_persistent_sessions(&mut state, auto_now_ms, auto_max_check_ins);
    }

    let result: Result<Value, String> = match cmd.as_str() {
        "status" => Ok(json!({
            "ok": true,
            "type": "swarm_runtime_status",
            "byzantine_test_mode": state.byzantine_test_mode,
            "session_count": state.sessions.len(),
            "result_count": state.result_registry.len(),
            "handoff_count": state.handoff_registry.len(),
            "tool_manifest_count": state.tool_registry.len(),
            "network_count": state.network_registry.len(),
            "dead_letter_count": state.dead_letters.len(),
            "active_thorn_cells": active_thorn_cell_ids(&state).len(),
            "event_count": state.events.len(),
            "max_depth": state
                .sessions
                .values()
                .map(|session| session.depth)
                .max()
                .unwrap_or(0),
            "scale": evaluate_scale_policy_readiness(
                &state,
                state.scale_policy.target_ready_agents,
                recommended_manager_fanout_for_target(state.scale_policy.target_ready_agents)
                    .min(state.scale_policy.max_children_per_parent.max(2)),
            ),
            "state_path": state_file,
        })),
        "scale" => {
            let sub = argv
                .get(1)
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let scale_args = || {
                let target_agents = parse_u64_flag(
                    argv,
                    "agents",
                    state.scale_policy.target_ready_agents as u64,
                )
                .max(1) as usize;
                let default_fanout = recommended_manager_fanout_for_target(target_agents)
                    .min(state.scale_policy.max_children_per_parent.max(2));
                let fanout = parse_u64_flag(argv, "fanout", default_fanout as u64).max(2) as usize;
                (target_agents, fanout)
            };
            match sub.as_str() {
                "status" | "plan" => {
                    let (target_agents, fanout) = scale_args();
                    Ok(json!({
                        "ok": true,
                        "type": if sub == "status" {
                            "swarm_runtime_scale_status"
                        } else {
                            "swarm_runtime_scale_plan"
                        },
                        "scale": evaluate_scale_policy_readiness(&state, target_agents, fanout),
                    }))
                }
                "set" => {
                    let parse_bool = |raw: &str| -> Result<bool, String> {
                        match raw.trim().to_ascii_lowercase().as_str() {
                            "1" | "true" | "yes" | "on" => Ok(true),
                            "0" | "false" | "no" | "off" => Ok(false),
                            _ => Err(format!("invalid_bool:{raw}")),
                        }
                    };
                    let mut apply_set = || -> Result<(), String> {
                        if let Some(raw) = parse_flag(argv, "max-sessions") {
                            let value = raw
                                .trim()
                                .parse::<usize>()
                                .map_err(|_| format!("invalid_max_sessions:{raw}"))?;
                            if value == 0 {
                                return Err("max_sessions_must_be_positive".to_string());
                            }
                            state.scale_policy.max_sessions_hard = value;
                        }
                        if let Some(raw) = parse_flag(argv, "max-children-per-parent") {
                            let value = raw
                                .trim()
                                .parse::<usize>()
                                .map_err(|_| format!("invalid_max_children_per_parent:{raw}"))?;
                            if value < 2 {
                                return Err(
                                    "max_children_per_parent_must_be_at_least_2".to_string(),
                                );
                            }
                            state.scale_policy.max_children_per_parent = value;
                        }
                        if let Some(raw) = parse_flag(argv, "max-depth-hard") {
                            let value = raw
                                .trim()
                                .parse::<u8>()
                                .map_err(|_| format!("invalid_max_depth_hard:{raw}"))?;
                            if value < 2 {
                                return Err("max_depth_hard_must_be_at_least_2".to_string());
                            }
                            state.scale_policy.max_depth_hard = value;
                        }
                        if let Some(raw) = parse_flag(argv, "target-ready-agents") {
                            let value = raw
                                .trim()
                                .parse::<usize>()
                                .map_err(|_| format!("invalid_target_ready_agents:{raw}"))?;
                            if value == 0 {
                                return Err("target_ready_agents_must_be_positive".to_string());
                            }
                            state.scale_policy.target_ready_agents = value;
                        }
                        if let Some(raw) = parse_flag(argv, "enforce-session-cap") {
                            state.scale_policy.enforce_session_cap = parse_bool(&raw)?;
                        }
                        if let Some(raw) = parse_flag(argv, "enforce-parent-capacity") {
                            state.scale_policy.enforce_parent_capacity = parse_bool(&raw)?;
                        }
                        Ok(())
                    };
                    match apply_set() {
                        Ok(()) => {
                            let target_agents = state.scale_policy.target_ready_agents;
                            let fanout = recommended_manager_fanout_for_target(target_agents)
                                .min(state.scale_policy.max_children_per_parent.max(2));
                            Ok(json!({
                                "ok": true,
                                "type": "swarm_runtime_scale_set",
                                "scale": evaluate_scale_policy_readiness(&state, target_agents, fanout),
                            }))
                        }
                        Err(err) => Err(err),
                    }
                }
                _ => Err(format!("unknown_scale_subcommand:{sub}")),
            }
        }
        "spawn" => {
            let task = parse_flag(argv, "task").unwrap_or_else(|| "swarm-task".to_string());
            let parent_id = parse_flag(argv, "session-id");
            let recursive = parse_bool_flag(argv, "recursive", false);
            let max_depth = parse_u8_flag(argv, "max-depth", 8).max(1);
            let levels = parse_u8_flag(argv, "levels", max_depth).max(1);
            let options = build_spawn_options(argv);
            let mode = options.execution_mode.clone();

