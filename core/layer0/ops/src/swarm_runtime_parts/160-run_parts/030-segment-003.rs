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
