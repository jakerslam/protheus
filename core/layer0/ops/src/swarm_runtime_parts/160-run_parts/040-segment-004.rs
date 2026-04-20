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
                            parse_u64_flag(argv, "ttl-ms", DEFAULT_MESSAGE_TTL_MS),
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
