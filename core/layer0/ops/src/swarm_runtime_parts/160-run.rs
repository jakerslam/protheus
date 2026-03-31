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
            "state_path": state_file,
        })),
        "spawn" => {
            let task = parse_flag(argv, "task").unwrap_or_else(|| "swarm-task".to_string());
            let parent_id = parse_flag(argv, "session-id");
            let recursive = parse_bool_flag(argv, "recursive", false);
            let max_depth = parse_u8_flag(argv, "max-depth", 8).max(1);
            let levels = parse_u8_flag(argv, "levels", max_depth).max(1);
            let options = build_spawn_options(argv);
            let mode = options.execution_mode.clone();

            let payload_result = if recursive {
                if !matches!(mode, ExecutionMode::TaskOriented) {
                    Err("recursive_mode_requires_task_execution_mode".to_string())
                } else {
                    recursive_spawn_with_tracking(
                        &mut state,
                        parent_id.as_deref(),
                        &task,
                        levels,
                        max_depth,
                        &options,
                    )
                }
            } else {
                match mode {
                    ExecutionMode::TaskOriented => {
                        spawn_single(&mut state, parent_id.as_deref(), &task, max_depth, &options)
                    }
                    ExecutionMode::Persistent(cfg) => spawn_persistent_session(
                        &mut state,
                        parent_id.as_deref(),
                        &task,
                        max_depth,
                        &options,
                        &cfg,
                        false,
                    ),
                    ExecutionMode::Background(cfg) => spawn_persistent_session(
                        &mut state,
                        parent_id.as_deref(),
                        &task,
                        max_depth,
                        &options,
                        &cfg,
                        true,
                    ),
                }
            };
            payload_result.map(|payload| {
                json!({
                    "ok": true,
                    "type": "swarm_runtime_spawn",
                    "recursive": recursive,
                    "mode": match options.execution_mode {
                        ExecutionMode::TaskOriented => "task",
                        ExecutionMode::Persistent(_) => "persistent",
                        ExecutionMode::Background(_) => "background",
                    },
                    "payload": payload,
                })
            })
        }
        "tick" => {
            let now_ms = now_epoch_ms().saturating_add(parse_u64_flag(argv, "advance-ms", 0));
            let max_check_ins = parse_u64_flag(argv, "max-check-ins", 16).max(1);
            tick_persistent_sessions(&mut state, now_ms, max_check_ins).map(|payload| {
                json!({
                    "ok": true,
                    "type": "swarm_runtime_tick",
                    "payload": payload,
                })
            })
        }
        "byzantine-test" => {
            let action = argv
                .get(1)
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            match action.as_str() {
                "enable" => {
                    state.byzantine_test_mode = true;
                    Ok(json!({
                        "ok": true,
                        "type": "swarm_runtime_byzantine_test",
                        "enabled": true,
                    }))
                }
                "disable" => {
                    state.byzantine_test_mode = false;
                    Ok(json!({
                        "ok": true,
                        "type": "swarm_runtime_byzantine_test",
                        "enabled": false,
                    }))
                }
                _ => Ok(json!({
                    "ok": true,
                    "type": "swarm_runtime_byzantine_test",
                    "enabled": state.byzantine_test_mode,
                })),
            }
        }
        "consensus-check" => {
            let task_id = parse_flag(argv, "task-id");
            let threshold = parse_f64_flag(argv, "threshold", 0.6).clamp(0.0, 1.0);
            let report_flag = parse_flag(argv, "reports-json");
            let mut reports = parse_reports_from_flag(report_flag);
            if reports.is_empty() {
                reports = reports_from_state(&state, task_id.as_deref());
            }
            let fields = normalize_fields(parse_flag(argv, "fields"), &reports);
            let consensus = evaluate_consensus(&reports, &fields, threshold);
            Ok(json!({
                "ok": true,
                "type": "swarm_runtime_consensus",
                "task_id": task_id,
                "consensus": consensus,
                "sample_size": reports.len(),
            }))
        }
        "budget-report" => {
            if let Some(session_id) =
                parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty())
            {
                budget_report_for_session(&state, &session_id)
            } else {
                Err("session_id_required".to_string())
            }
        }
        "background" => run_background_command(&mut state, argv),
        "scheduled" => {
            let sub = argv
                .get(1)
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            match sub.as_str() {
                "add" => scheduled_add(&mut state, argv, now_epoch_ms()),
                "status" => Ok(scheduled_status(&state)),
                "run-due" => {
                    let now_ms =
                        now_epoch_ms().saturating_add(parse_u64_flag(argv, "advance-ms", 0));
                    let max_runs = parse_u64_flag(argv, "max-runs", 8).max(1);
                    scheduled_run_due(&mut state, now_ms, max_runs)
                }
                _ => Err(format!("unknown_scheduled_subcommand:{sub}")),
            }
        }
        "sessions" => {
            let sub = argv
                .get(1)
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            match sub.as_str() {
                "budget-report" => {
                    if let Some(session_id) =
                        parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty())
                    {
                        budget_report_for_session(&state, &session_id)
                    } else {
                        Err("session_id_required".to_string())
                    }
                }
                "wake" => {
                    if let Some(session_id) =
                        parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty())
                    {
                        sessions_wake(&mut state, &session_id, now_epoch_ms())
                    } else {
                        Err("session_id_required".to_string())
                    }
                }
                "resume" => {
                    if let Some(session_id) =
                        parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty())
                    {
                        sessions_resume(&mut state, &session_id, now_epoch_ms())
                    } else {
                        Err("session_id_required".to_string())
                    }
                }
                "terminate" => {
                    if let Some(session_id) =
                        parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty())
                    {
                        sessions_terminate(
                            &mut state,
                            &session_id,
                            parse_bool_flag(argv, "graceful", true),
                            now_epoch_ms(),
                        )
                    } else {
                        Err("session_id_required".to_string())
                    }
                }
                "metrics" => {
                    if let Some(session_id) =
                        parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty())
                    {
                        sessions_metrics(
                            &state,
                            &session_id,
                            parse_bool_flag(argv, "timeline", false),
                        )
                    } else {
                        Err("session_id_required".to_string())
                    }
                }
                "state" => {
                    if let Some(session_id) =
                        parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty())
                    {
                        sessions_state(
                            &state,
                            &session_id,
                            parse_bool_flag(argv, "timeline", false),
                            parse_u64_flag(argv, "tool-history-limit", 50) as usize,
                        )
                    } else {
                        Err("session_id_required".to_string())
                    }
                }
                "bootstrap" => {
                    if let Some(session_id) =
                        parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty())
                    {
                        sessions_bootstrap(&state, &session_id)
                    } else {
                        Err("session_id_required".to_string())
                    }
                }
                "anomalies" => {
                    if let Some(session_id) =
                        parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty())
                    {
                        sessions_anomalies(&state, &session_id)
                    } else {
                        Err("session_id_required".to_string())
                    }
                }
                "handoff" => {
                    let sender_id =
                        parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty());
                    let recipient_id = parse_flag(argv, "target-session-id")
                        .or_else(|| parse_flag(argv, "recipient-session-id"))
                        .filter(|value| !value.trim().is_empty());
                    let reason =
                        parse_flag(argv, "reason").filter(|value| !value.trim().is_empty());
                    match (sender_id, recipient_id, reason) {
                        (Some(sender_id), Some(recipient_id), Some(reason)) => register_handoff(
                            &mut state,
                            &sender_id,
                            &recipient_id,
                            &reason,
                            parse_f64_flag(argv, "importance", 0.5).clamp(0.0, 1.0),
                            parse_json_flag(argv, "context-json"),
                            parse_flag(argv, "network-id")
                                .filter(|value| !value.trim().is_empty()),
                        ),
                        (None, _, _) => Err("session_id_required".to_string()),
                        (_, None, _) => Err("target_session_id_required".to_string()),
                        (_, _, None) => Err("reason_required".to_string()),
                    }
                }
                "context-put" => {
                    let session_id =
                        parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty());
                    let context = parse_json_flag(argv, "context-json");
                    match (session_id, context) {
                        (Some(session_id), Some(context)) => match state.sessions.get_mut(&session_id)
                        {
                            Some(session) => match apply_context_update(
                                session,
                                context,
                                parse_bool_flag(argv, "merge", true),
                                "sessions_context_put",
                            ) {
                                Ok(receipt) => {
                                    append_event(
                                        &mut state,
                                        json!({
                                            "type": "swarm_context_put",
                                            "session_id": session_id,
                                            "timestamp": now_iso(),
                                        }),
                                    );
                                    Ok(json!({
                                        "ok": true,
                                        "type": "swarm_runtime_context_put",
                                        "session_id": session_id,
                                        "receipt": receipt,
                                    }))
                                }
                                Err(err) => Err(err),
                            },
                            None => Err(format!("unknown_session:{session_id}")),
                        },
                        (None, _) => Err("session_id_required".to_string()),
                        (_, None) => Err("context_json_required".to_string()),
                    }
                }
                "context-get" => {
                    if let Some(session_id) =
                        parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty())
                    {
                        match state.sessions.get(&session_id) {
                            Some(session) => Ok(json!({
                                "ok": true,
                                "type": "swarm_runtime_context_get",
                                "session_id": session_id,
                                "context": session_context_json(session),
                                "mode": session.context_mode.clone(),
                            })),
                            None => Err(format!("unknown_session:{session_id}")),
                        }
                    } else {
                        Err("session_id_required".to_string())
                    }
                }
                "send" => {
                    let sender_id = parse_flag(argv, "sender-id")
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or_else(|| "coordinator".to_string());
                    let recipient_id =
                        parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty());
                    let message =
                        parse_flag(argv, "message").filter(|value| !value.trim().is_empty());
                    match (recipient_id, message) {
                        (Some(recipient_id), Some(message)) => send_session_message(
                            &mut state,
                            &sender_id,
                            &recipient_id,
                            &message,
                            DeliveryGuarantee::from_flag(parse_flag(argv, "delivery")),
                            parse_bool_flag(argv, "simulate-first-attempt-fail", false),
                            parse_u64_flag(argv, "ttl-ms", DEFAULT_MESSAGE_TTL_MS),
                        ),
                        (None, _) => Err("session_id_required".to_string()),
                        (_, None) => Err("message_required".to_string()),
                    }
                }
                "receive" => {
                    if let Some(session_id) =
                        parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty())
                    {
                        receive_session_messages(
                            &mut state,
                            &session_id,
                            parse_u64_flag(argv, "limit", 50) as usize,
                            parse_bool_flag(argv, "mark-read", true),
                        )
                    } else {
                        Err("session_id_required".to_string())
                    }
                }
                "ack" => {
                    let session_id =
                        parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty());
                    let message_id =
                        parse_flag(argv, "message-id").filter(|value| !value.trim().is_empty());
                    match (session_id, message_id) {
                        (Some(session_id), Some(message_id)) => {
                            acknowledge_session_message(&mut state, &session_id, &message_id)
                        }
                        (None, _) => Err("session_id_required".to_string()),
                        (_, None) => Err("message_id_required".to_string()),
                    }
                }
                "dead-letter" => Ok(sessions_dead_letters(
                    &state,
                    parse_flag(argv, "session-id")
                        .filter(|value| !value.trim().is_empty())
                        .as_deref(),
                    parse_bool_flag(argv, "retryable", false),
                )),
                "retry-dead-letter" => {
                    if let Some(message_id) =
                        parse_flag(argv, "message-id").filter(|value| !value.trim().is_empty())
                    {
                        sessions_retry_dead_letter(&mut state, &message_id)
                    } else {
                        Err("message_id_required".to_string())
                    }
                }
                "discover" => {
                    if let Some(role) =
                        parse_flag(argv, "role").filter(|value| !value.trim().is_empty())
                    {
                        Ok(json!({
                            "ok": true,
                            "type": "swarm_runtime_sessions_discover",
                            "role": role,
                            "instances": discover_services(&state, &role),
                        }))
                    } else {
                        Err("role_required".to_string())
                    }
                }
                "send-role" => {
                    let sender_id = parse_flag(argv, "sender-id")
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or_else(|| "coordinator".to_string());
                    let role = parse_flag(argv, "role").filter(|value| !value.trim().is_empty());
                    let message =
                        parse_flag(argv, "message").filter(|value| !value.trim().is_empty());
                    match (role, message) {
                        (Some(role), Some(message)) => send_to_role(
                            &mut state,
                            &sender_id,
                            &role,
                            &message,
                            DeliveryGuarantee::from_flag(parse_flag(argv, "delivery")),
                        ),
                        (None, _) => Err("role_required".to_string()),
                        (_, None) => Err("message_required".to_string()),
                    }
                }
                _ => Err(format!("unknown_sessions_subcommand:{sub}")),
            }
        }
        "tools" => {
            let sub = argv
                .get(1)
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            match sub.as_str() {
                "register-json-schema" => {
                    let session_id =
                        parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty());
                    let tool_name =
                        parse_flag(argv, "tool-name").filter(|value| !value.trim().is_empty());
                    let schema = parse_json_flag(argv, "schema-json");
                    let bridge_path =
                        parse_flag(argv, "bridge-path").filter(|value| !value.trim().is_empty());
                    let entrypoint =
                        parse_flag(argv, "entrypoint").filter(|value| !value.trim().is_empty());
                    match (session_id, tool_name, schema, bridge_path, entrypoint) {
                        (Some(session_id), Some(tool_name), Some(schema), Some(bridge_path), Some(entrypoint)) =>
                            register_json_schema_tool(
                                &mut state,
                                &session_id,
                                &tool_name,
                                schema,
                                &bridge_path,
                                &entrypoint,
                                parse_flag(argv, "description")
                                    .filter(|value| !value.trim().is_empty()),
                            ),
                        (None, _, _, _, _) => Err("session_id_required".to_string()),
                        (_, None, _, _, _) => Err("tool_name_required".to_string()),
                        (_, _, None, _, _) => Err("schema_json_required".to_string()),
                        (_, _, _, None, _) => Err("bridge_path_required".to_string()),
                        (_, _, _, _, None) => Err("entrypoint_required".to_string()),
                    }
                }
                "invoke" => {
                    let session_id =
                        parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty());
                    let tool_name =
                        parse_flag(argv, "tool-name").filter(|value| !value.trim().is_empty());
                    match (session_id, tool_name) {
                        (Some(session_id), Some(tool_name)) => invoke_registered_tool(
                            &mut state,
                            &session_id,
                            &tool_name,
                            parse_json_flag(argv, "args-json")
                                .unwrap_or_else(|| Value::Object(Map::new())),
                        ),
                        (None, _) => Err("session_id_required".to_string()),
                        (_, None) => Err("tool_name_required".to_string()),
                    }
                }
                _ => Err(format!("unknown_tools_subcommand:{sub}")),
            }
        }
        "stream" => {
            let sub = argv
                .get(1)
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "render".to_string());
            match sub.as_str() {
                "emit" => {
                    let session_id =
                        parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty());
                    let chunks = parse_json_flag(argv, "chunks-json")
                        .and_then(|value| value.as_array().cloned())
                        .unwrap_or_default();
                    if let Some(session_id) = session_id {
                        stream_emit(
                            &mut state,
                            &session_id,
                            parse_flag(argv, "turn-id").filter(|value| !value.trim().is_empty()),
                            parse_flag(argv, "agent-label")
                                .filter(|value| !value.trim().is_empty()),
                            chunks,
                        )
                    } else {
                        Err("session_id_required".to_string())
                    }
                }
                "render" => {
                    if let Some(session_id) =
                        parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty())
                    {
                        stream_render(
                            &state,
                            &session_id,
                            parse_flag(argv, "turn-id")
                                .filter(|value| !value.trim().is_empty())
                                .as_deref(),
                        )
                    } else {
                        Err("session_id_required".to_string())
                    }
                }
                _ => Err(format!("unknown_stream_subcommand:{sub}")),
            }
        }
        "turns" => {
            let sub = argv
                .get(1)
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "run".to_string());
            match sub.as_str() {
                "run" => {
                    let session_id =
                        parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty());
                    let turns = parse_json_flag(argv, "turns-json")
                        .and_then(|value| value.as_array().cloned())
                        .unwrap_or_default();
                    if let Some(session_id) = session_id {
                        execute_turns(
                            &mut state,
                            &session_id,
                            turns,
                            parse_flag(argv, "label").filter(|value| !value.trim().is_empty()),
                        )
                    } else {
                        Err("session_id_required".to_string())
                    }
                }
                "show" => {
                    let session_id =
                        parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty());
                    let run_id =
                        parse_flag(argv, "run-id").filter(|value| !value.trim().is_empty());
                    match (session_id, run_id) {
                        (Some(session_id), Some(run_id)) => show_turn_run(&state, &session_id, &run_id),
                        (None, _) => Err("session_id_required".to_string()),
                        (_, None) => Err("run_id_required".to_string()),
                    }
                }
                _ => Err(format!("unknown_turns_subcommand:{sub}")),
            }
        }
        "networks" => {
            let sub = argv
                .get(1)
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            match sub.as_str() {
                "create" => {
                    let spec = parse_json_flag(argv, "spec-json");
                    if let Some(spec) = spec {
                        create_agent_network(
                            &mut state,
                            parse_flag(argv, "session-id")
                                .filter(|value| !value.trim().is_empty())
                                .as_deref(),
                            spec,
                        )
                    } else {
                        Err("spec_json_required".to_string())
                    }
                }
                "status" => {
                    let network_id =
                        parse_flag(argv, "network-id").filter(|value| !value.trim().is_empty());
                    if let Some(network_id) = network_id {
                        network_status(
                            &state,
                            parse_flag(argv, "session-id")
                                .filter(|value| !value.trim().is_empty())
                                .as_deref(),
                            &network_id,
                        )
                    } else {
                        Err("network_id_required".to_string())
                    }
                }
                _ => Err(format!("unknown_networks_subcommand:{sub}")),
            }
        }
        "channels" => {
            let sub = argv
                .get(1)
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            match sub.as_str() {
                "create" => {
                    let name = parse_flag(argv, "name")
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or_else(|| "swarm-channel".to_string());
                    let participants = parse_flag(argv, "participants")
                        .map(|raw| {
                            raw.split(',')
                                .map(|value| value.trim().to_string())
                                .filter(|value| !value.is_empty())
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    create_channel(&mut state, &name, participants)
                }
                "publish" => {
                    let channel_id =
                        parse_flag(argv, "channel-id").filter(|value| !value.trim().is_empty());
                    let sender_id = parse_flag(argv, "sender-id")
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or_else(|| "coordinator".to_string());
                    let message =
                        parse_flag(argv, "message").filter(|value| !value.trim().is_empty());
                    match (channel_id, message) {
                        (Some(channel_id), Some(message)) => publish_channel_message(
                            &mut state,
                            &channel_id,
                            &sender_id,
                            &message,
                            DeliveryGuarantee::from_flag(parse_flag(argv, "delivery")),
                        ),
                        (None, _) => Err("channel_id_required".to_string()),
                        (_, None) => Err("message_required".to_string()),
                    }
                }
                "poll" | "monitor" => {
                    let channel_id =
                        parse_flag(argv, "channel-id").filter(|value| !value.trim().is_empty());
                    let session_id =
                        parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty());
                    match (channel_id, session_id) {
                        (Some(channel_id), Some(session_id)) => poll_channel_messages(
                            &state,
                            &channel_id,
                            &session_id,
                            parse_flag(argv, "since-ms")
                                .and_then(|value| value.trim().parse::<u64>().ok()),
                        ),
                        (None, _) => Err("channel_id_required".to_string()),
                        (_, None) => Err("session_id_required".to_string()),
                    }
                }
                _ => Err(format!("unknown_channels_subcommand:{sub}")),
            }
        }
        "results" => {
            let sub = argv
                .get(1)
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "query".to_string());
            match sub.as_str() {
                "publish" => {
                    match parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty()) {
                        Some(session_id) => {
                            let confidence =
                                parse_f64_flag(argv, "confidence", 1.0).clamp(0.0, 1.0);
                            let verification_status = parse_flag(argv, "verification-status")
                                .filter(|value| !value.trim().is_empty())
                                .unwrap_or_else(|| "not_verified".to_string());
                            let payload_result: Result<ResultPayload, String> = if let Some(value) =
                                parse_flag(argv, "value")
                                    .and_then(|value| value.parse::<f64>().ok())
                            {
                                Ok(ResultPayload::Calculation { value })
                            } else if let Some(text) =
                                parse_flag(argv, "text").filter(|value| !value.trim().is_empty())
                            {
                                Ok(ResultPayload::Text { content: text })
                            } else if let Some(data_json) = parse_flag(argv, "data-json")
                                .filter(|value| !value.trim().is_empty())
                            {
                                match serde_json::from_str::<Value>(&data_json) {
                                    Ok(data) => Ok(ResultPayload::Structured {
                                        schema: parse_flag(argv, "schema")
                                            .unwrap_or_else(|| "result_v1".to_string()),
                                        data,
                                    }),
                                    Err(err) => Err(format!("invalid_data_json:{err}")),
                                }
                            } else if let Some(session) = state.sessions.get(&session_id) {
                                Ok(ResultPayload::Structured {
                                    schema: "swarm_runtime_report_v1".to_string(),
                                    data: session.report.clone().unwrap_or(Value::Null),
                                })
                            } else {
                                Err(format!("unknown_session:{session_id}"))
                            };
                            match payload_result {
                                Ok(payload) => publish_result(
                                    &mut state,
                                    &session_id,
                                    parse_flag(argv, "label")
                                        .filter(|value| !value.trim().is_empty()),
                                    parse_flag(argv, "task-id")
                                        .filter(|value| !value.trim().is_empty()),
                                    payload,
                                    json!({"source": "results_publish_command"}),
                                    confidence,
                                    verification_status,
                                ),
                                Err(err) => Err(err),
                            }
                        }
                        None => Err("session_id_required".to_string()),
                    }
                }
                "query" => {
                    let filters = parse_result_filters(argv);
                    let results = query_results(&state, &filters);
                    Ok(json!({
                        "ok": true,
                        "type": "swarm_runtime_results_query",
                        "filters": {
                            "label_pattern": filters.label_pattern,
                            "role": filters.role,
                            "task_id": filters.task_id,
                            "session_id": filters.session_id,
                        },
                        "result_count": results.len(),
                        "results": results,
                    }))
                }
                "wait" => {
                    let filters = parse_result_filters(argv);
                    let min_count = parse_u64_flag(argv, "min-count", 1) as usize;
                    let timeout_ms =
                        (parse_f64_flag(argv, "timeout-sec", 30.0).max(0.1) * 1000.0) as u64;
                    match wait_for_results(&state_file, &state, &filters, min_count, timeout_ms) {
                        Ok(results) => Ok(json!({
                            "ok": true,
                            "type": "swarm_runtime_results_wait",
                            "min_count": min_count.max(1),
                            "timeout_ms": timeout_ms,
                            "result_count": results.len(),
                            "results": results,
                        })),
                        Err(err) => Err(err),
                    }
                }
                "show" => {
                    if let Some(result_id) =
                        parse_flag(argv, "result-id").filter(|value| !value.trim().is_empty())
                    {
                        if let Some(result) = state.result_registry.get(&result_id).cloned() {
                            Ok(json!({
                                "ok": true,
                                "type": "swarm_runtime_results_show",
                                "result": result,
                            }))
                        } else {
                            Err(format!("unknown_result:{result_id}"))
                        }
                    } else {
                        Err("result_id_required".to_string())
                    }
                }
                "consensus" => {
                    let filters = parse_result_filters(argv);
                    let field = parse_flag(argv, "field")
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or_else(|| "value".to_string());
                    let threshold = parse_f64_flag(argv, "threshold", 1.0).clamp(0.0, 1.0);
                    let results = query_results(&state, &filters);
                    let consensus = analyze_result_consensus(&results, &field, threshold);
                    append_event(
                        &mut state,
                        json!({
                            "type": "swarm_results_consensus",
                            "field": field,
                            "threshold": threshold,
                            "result_count": results.len(),
                            "status": consensus.get("status").cloned().unwrap_or(Value::Null),
                            "consensus_reached": consensus
                                .get("consensus_reached")
                                .cloned()
                                .unwrap_or(Value::Bool(false)),
                            "timestamp": now_iso(),
                        }),
                    );
                    Ok(json!({
                        "ok": true,
                        "type": "swarm_runtime_results_consensus",
                        "field": field,
                        "result_count": results.len(),
                        "consensus": consensus,
                    }))
                }
                "outliers" => {
                    let filters = parse_result_filters(argv);
                    let field = parse_flag(argv, "field")
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or_else(|| "value".to_string());
                    let results = query_results(&state, &filters);
                    let analysis = analyze_result_outliers(&results, &field);
                    append_event(
                        &mut state,
                        json!({
                            "type": "swarm_results_outliers",
                            "field": field,
                            "result_count": results.len(),
                            "status": analysis.get("status").cloned().unwrap_or(Value::Null),
                            "outlier_count": analysis.get("outlier_count").cloned().unwrap_or(json!(0)),
                            "timestamp": now_iso(),
                        }),
                    );
                    Ok(json!({
                        "ok": true,
                        "type": "swarm_runtime_results_outliers",
                        "field": field,
                        "result_count": results.len(),
                        "analysis": analysis,
                    }))
                }
                _ => Err(format!("unknown_results_subcommand:{sub}")),
            }
        }
        "metrics" => {
            let sub = argv
                .get(1)
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "queue".to_string());
            match sub.as_str() {
                "queue" => {
                    let snapshot = queue_metrics_snapshot(&state);
                    let format = parse_flag(argv, "format")
                        .unwrap_or_else(|| "json".to_string())
                        .to_ascii_lowercase();
                    let prometheus = if format == "prometheus" {
                        Some(queue_metrics_prometheus(&state, &snapshot))
                    } else {
                        None
                    };
                    Ok(json!({
                        "ok": true,
                        "type": "swarm_runtime_metrics_queue",
                        "format": format,
                        "snapshot": snapshot,
                        "prometheus": prometheus,
                    }))
                }
                _ => Err(format!("unknown_metrics_subcommand:{sub}")),
            }
        }
        "test" => {
            let suite = argv
                .get(1)
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "recursive".to_string());
            match suite.as_str() {
                "recursive" => run_test_recursive(&mut state, argv),
                "byzantine" => run_test_byzantine(&mut state, argv),
                "concurrency" => run_test_concurrency(&mut state, argv),
                "budget" => run_test_budget(&mut state, argv),
                "persistent" => run_test_persistent(&mut state, argv),
                "communication" => run_test_communication(&mut state, argv),
                "heterogeneous" => run_test_heterogeneous(&mut state, &state_file, argv),
                _ => Err(format!("unknown_test_suite:{suite}")),
            }
        }
        "thorn" => run_thorn_contract_in_state(&mut state, &argv[1..]).map(|mut payload| {
            payload["claim_evidence"] = json!([{
                "id": "V6-SEC-THORN-001",
                "claim": "thorn_cells_quarantine_compromised_sessions_with_restricted_capabilities_and_receipted_reroute_self_destruct_flow",
                "evidence": {
                    "command": argv.get(1).cloned().unwrap_or_else(|| "status".to_string()),
                    "state_path": state_file.display().to_string(),
                }
            }]);
            payload["receipt_hash"] = Value::String(deterministic_receipt_hash(&payload));
            payload
        }),
        _ => Err(format!("unknown_command:{cmd}")),
    };

    state.updated_at = now_iso();
    let save_result = save_state(&state_file, &state);

    match result {
        Ok(payload) => {
            if let Err(err) = save_result {
                print_receipt(json!({
                    "ok": false,
                    "type": "swarm_runtime_error",
                    "command": cmd,
                    "error": err,
                    "state_path": state_file,
                }));
                return 2;
            }

            append_event(
                &mut state,
                json!({
                    "type": "swarm_runtime_command",
                    "command": cmd,
                    "timestamp": now_iso(),
                    "ok": true,
                }),
            );
            let _ = save_state(&state_file, &state);
            print_receipt(payload);
            0
        }
        Err(err) => {
            print_receipt(json!({
                "ok": false,
                "type": "swarm_runtime_error",
                "command": cmd,
                "error": err,
                "state_path": state_file,
            }));
            2
        }
    }
}
