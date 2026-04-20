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
