pub fn run(root: &Path, argv: &[String]) -> i32 {
    let cmd = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if matches!(cmd.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let state_file = state_path(root, argv);
    let mut registry = match load_registry(&state_file) {
        Ok(v) => v,
        Err(err) => {
            print_json_line(&error_receipt(
                "state_load_failed",
                &err,
                &cmd,
                argv,
                &state_file,
                2,
            ));
            return 2;
        }
    };

    let now_ms = now_epoch_ms();
    let exit_code = match cmd.as_str() {
        "list" | "status" => {
            let target = session_id_from_args(&cmd, argv);
            let payload = if let Some(session_id) = target {
                if let Some(session) = registry.sessions.get(&session_id) {
                    json!({
                        "session": session_view(session, now_ms),
                        "session_count": registry.sessions.len()
                    })
                } else {
                    print_json_line(&error_receipt(
                        "unknown_session",
                        "session_id not found",
                        &cmd,
                        argv,
                        &state_file,
                        3,
                    ));
                    return 3;
                }
            } else {
                let sessions = registry
                    .sessions
                    .values()
                    .map(|session| session_view(session, now_ms))
                    .collect::<Vec<_>>();
                json!({
                    "sessions": sessions,
                    "session_count": registry.sessions.len()
                })
            };
            print_json_line(&success_receipt(
                "command_center_session_status",
                &cmd,
                argv,
                &state_file,
                payload,
            ));
            0
        }
        "register" | "start" => {
            let Some(session_id) = session_id_from_args(&cmd, argv) else {
                print_json_line(&error_receipt(
                    "missing_session_id",
                    "expected --session-id=<id> or positional session id",
                    &cmd,
                    argv,
                    &state_file,
                    2,
                ));
                return 2;
            };
            let lineage_id = parse_cli_flag(argv, "lineage-id")
                .unwrap_or_else(|| lineage_seed(&session_id, now_ms));
            let status = parse_cli_flag(argv, "status").unwrap_or_else(|| "running".to_string());
            let task = parse_cli_flag(argv, "task");

            let existing_started = registry
                .sessions
                .get(&session_id)
                .map(|row| row.started_epoch_ms)
                .unwrap_or(now_ms);
            let mut metadata = registry
                .sessions
                .get(&session_id)
                .map(|row| row.metadata.clone())
                .unwrap_or_else(|| json!({}));
            if let Some(task_name) = task {
                metadata["task"] = Value::String(task_name);
            }

            let mut session = SessionState {
                session_id: session_id.clone(),
                lineage_id: lineage_id.clone(),
                status: status.clone(),
                started_epoch_ms: existing_started,
                last_attach_epoch_ms: None,
                terminated_epoch_ms: if status == "terminated" {
                    Some(now_ms)
                } else {
                    None
                },
                attach_count: 0,
                steering_count: 0,
                token_count: 0,
                cost_usd: 0.0,
                health: normalized_health(&status),
                last_steering_hash: None,
                recent_steering: Vec::new(),
                events: Vec::new(),
                metadata,
            };
            push_event(
                &mut session,
                now_ms,
                "register",
                json!({
                  "status": status,
                  "lineage_id": lineage_id
                }),
            );
            registry.sessions.insert(session_id.clone(), session);
            registry.updated_epoch_ms = now_ms;
            if let Err(err) = save_registry(&state_file, &registry) {
                print_json_line(&error_receipt(
                    "state_write_failed",
                    &err,
                    &cmd,
                    argv,
                    &state_file,
                    2,
                ));
                return 2;
            }

            let payload = json!({
                "session_id": session_id,
                "lineage_id": lineage_id,
                "status": status
            });
            print_json_line(&success_receipt(
                "command_center_session_register",
                &cmd,
                argv,
                &state_file,
                payload,
            ));
            0
        }
        "resume" | "attach" => {
            let Some(session_id) = session_id_from_args(&cmd, argv) else {
                print_json_line(&error_receipt(
                    "missing_session_id",
                    "expected session id for resume/attach",
                    &cmd,
                    argv,
                    &state_file,
                    2,
                ));
                return 2;
            };
            let payload = {
                let Some(session) = registry.sessions.get_mut(&session_id) else {
                    print_json_line(&error_receipt(
                        "unknown_session",
                        "session_id not found",
                        &cmd,
                        argv,
                        &state_file,
                        3,
                    ));
                    return 3;
                };
                if session.status == "terminated" {
                    print_json_line(&error_receipt(
                        "stale_session",
                        "cannot resume terminated session",
                        &cmd,
                        argv,
                        &state_file,
                        4,
                    ));
                    return 4;
                }
                session.status = "running".to_string();
                session.health = normalized_health(&session.status);
                session.attach_count = session.attach_count.saturating_add(1);
                session.last_attach_epoch_ms = Some(now_ms);
                session.terminated_epoch_ms = None;
                push_event(&mut *session, now_ms, "resume", json!({}));
                registry.updated_epoch_ms = now_ms;

                json!({
                    "session_id": session.session_id,
                    "lineage_id": session.lineage_id,
                    "status": session.status,
                    "attach_count": session.attach_count,
                    "attached_epoch_ms": session.last_attach_epoch_ms,
                    "steering_contract": format!("infring session send {} --message=\"...\"", session_id),
                    "lineage_receipt_key": format!("{}::{}", session.lineage_id, session.session_id)
                })
            };

            if let Err(err) = save_registry(&state_file, &registry) {
                print_json_line(&error_receipt(
                    "state_write_failed",
                    &err,
                    &cmd,
                    argv,
                    &state_file,
                    2,
                ));
                return 2;
            }
            print_json_line(&success_receipt(
                "command_center_session_resume",
                &cmd,
                argv,
                &state_file,
                payload,
            ));
            0
        }
        "send" | "steer" => {
            let Some(session_id) = session_id_from_args(&cmd, argv) else {
                print_json_line(&error_receipt(
                    "missing_session_id",
                    "expected session id for send/steer",
                    &cmd,
                    argv,
                    &state_file,
                    2,
                ));
                return 2;
            };
            let message = parse_cli_flag(argv, "message")
                .or_else(|| parse_cli_flag(argv, "steer"))
                .or_else(|| first_free_positional(argv, 2))
                .unwrap_or_default();
            if message.trim().is_empty() {
                print_json_line(&error_receipt(
                    "missing_message",
                    "expected --message=<text> for send/steer",
                    &cmd,
                    argv,
                    &state_file,
                    2,
                ));
                return 2;
            }
            let token_delta = parse_cli_flag(argv, "token-delta")
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0);
            let cost_delta = parse_cli_flag(argv, "cost-delta")
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.0)
                .max(0.0);
            let payload = {
                let Some(session) = registry.sessions.get_mut(&session_id) else {
                    print_json_line(&error_receipt(
                        "unknown_session",
                        "session_id not found",
                        &cmd,
                        argv,
                        &state_file,
                        3,
                    ));
                    return 3;
                };
                if session.status == "terminated" {
                    print_json_line(&error_receipt(
                        "stale_session",
                        "cannot steer terminated session",
                        &cmd,
                        argv,
                        &state_file,
                        4,
                    ));
                    return 4;
                }

                let message_hash = sha256_hex(&message);
                let event = SteeringEvent {
                    ts_epoch_ms: now_ms,
                    message: message.clone(),
                    message_hash: message_hash.clone(),
                };
                session.steering_count = session.steering_count.saturating_add(1);
                session.token_count = session.token_count.saturating_add(token_delta);
                session.cost_usd += cost_delta;
                session.last_steering_hash = Some(message_hash.clone());
                session.recent_steering.push(event);
                if session.recent_steering.len() > 20 {
                    let excess = session.recent_steering.len() - 20;
                    session.recent_steering.drain(0..excess);
                }
                push_event(
                    &mut *session,
                    now_ms,
                    "steer",
                    json!({
                        "message_hash": format!("sha256:{message_hash}"),
                        "token_delta": token_delta,
                        "cost_delta": cost_delta
                    }),
                );
                registry.updated_epoch_ms = now_ms;

                json!({
                    "session_id": session.session_id,
                    "lineage_id": session.lineage_id,
                    "intervention_id": format!("{}-{}", session.session_id, session.steering_count),
                    "steering_count": session.steering_count,
                    "message_hash": format!("sha256:{message_hash}"),
                    "lineage_receipt_key": format!("{}::{}", session.lineage_id, session.session_id)
                })
            };

            if let Err(err) = save_registry(&state_file, &registry) {
                print_json_line(&error_receipt(
                    "state_write_failed",
                    &err,
                    &cmd,
                    argv,
                    &state_file,
                    2,
                ));
                return 2;
            }
            print_json_line(&success_receipt(
                "command_center_session_steer",
                &cmd,
                argv,
                &state_file,
                payload,
            ));
            0
        }
        "kill" | "terminate" => {
            let Some(session_id) = session_id_from_args(&cmd, argv) else {
                print_json_line(&error_receipt(
                    "missing_session_id",
                    "expected session id for kill/terminate",
                    &cmd,
                    argv,
                    &state_file,
                    2,
                ));
                return 2;
            };
            let payload = {
                let Some(session) = registry.sessions.get_mut(&session_id) else {
                    print_json_line(&error_receipt(
                        "unknown_session",
                        "session_id not found",
                        &cmd,
                        argv,
                        &state_file,
                        3,
                    ));
                    return 3;
                };
                session.status = "terminated".to_string();
                session.health = normalized_health(&session.status);
                session.terminated_epoch_ms = Some(now_ms);
                push_event(&mut *session, now_ms, "kill", json!({}));
                registry.updated_epoch_ms = now_ms;
                json!({
                  "session_id": session.session_id,
                  "lineage_id": session.lineage_id,
                  "status": session.status,
                  "terminated_epoch_ms": session.terminated_epoch_ms
                })
            };
            if let Err(err) = save_registry(&state_file, &registry) {
                print_json_line(&error_receipt(
                    "state_write_failed",
                    &err,
                    &cmd,
                    argv,
                    &state_file,
                    2,
                ));
                return 2;
            }
            print_json_line(&success_receipt(
                "command_center_session_kill",
                &cmd,
                argv,
                &state_file,
                payload,
            ));
            0
        }
        "tail" => {
            let Some(session_id) = session_id_from_args(&cmd, argv) else {
                print_json_line(&error_receipt(
                    "missing_session_id",
                    "expected session id for tail",
                    &cmd,
                    argv,
                    &state_file,
                    2,
                ));
                return 2;
            };
            let lines = parse_cli_flag(argv, "lines")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(10)
                .clamp(1, 100);
            let payload = {
                let Some(session) = registry.sessions.get(&session_id) else {
                    print_json_line(&error_receipt(
                        "unknown_session",
                        "session_id not found",
                        &cmd,
                        argv,
                        &state_file,
                        3,
                    ));
                    return 3;
                };
                let events = session
                    .events
                    .iter()
                    .rev()
                    .take(lines)
                    .cloned()
                    .collect::<Vec<_>>();
                let steering = session
                    .recent_steering
                    .iter()
                    .rev()
                    .take(lines)
                    .cloned()
                    .collect::<Vec<_>>();
                json!({
                    "session": session_view(session, now_ms),
                    "events": events,
                    "steering": steering,
                    "lines": lines
                })
            };
            print_json_line(&success_receipt(
                "command_center_session_tail",
                &cmd,
                argv,
                &state_file,
                payload,
            ));
            0
        }
        "inspect" => {
            let Some(session_id) = session_id_from_args(&cmd, argv) else {
                print_json_line(&error_receipt(
                    "missing_session_id",
                    "expected session id for inspect",
                    &cmd,
                    argv,
                    &state_file,
                    2,
                ));
                return 2;
            };
            let payload = {
                let Some(session) = registry.sessions.get(&session_id) else {
                    print_json_line(&error_receipt(
                        "unknown_session",
                        "session_id not found",
                        &cmd,
                        argv,
                        &state_file,
                        3,
                    ));
                    return 3;
                };
                json!({
                    "session": session,
                    "summary": session_view(session, now_ms)
                })
            };
            print_json_line(&success_receipt(
                "command_center_session_inspect",
                &cmd,
                argv,
                &state_file,
                payload,
            ));
            0
        }
        _ => {
            usage();
            print_json_line(&error_receipt(
                "unknown_command",
                "unsupported command",
                &cmd,
                argv,
                &state_file,
                2,
            ));
            2
        }
    };
    exit_code
}
