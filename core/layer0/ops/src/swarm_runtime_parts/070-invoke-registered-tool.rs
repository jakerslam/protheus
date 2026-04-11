fn tool_target_session_id<'a>(args_obj: &'a Map<String, Value>) -> Option<&'a str> {
    args_obj
        .get("target_session_id")
        .or_else(|| args_obj.get("session_id"))
        .or_else(|| args_obj.get("target"))
        .and_then(Value::as_str)
}

fn invoke_registered_tool(
    state: &mut SwarmState,
    session_id: &str,
    tool_name: &str,
    args: Value,
) -> Result<Value, String> {
    if !session_exists(state, session_id) {
        return Err(format!("unknown_session:{session_id}"));
    }
    let key = tool_manifest_storage_key(session_id, tool_name);
    let manifest = state
        .tool_registry
        .get(&key)
        .cloned()
        .ok_or_else(|| format!("unknown_tool_manifest:{tool_name}"))?;
    let bridge_path = manifest
        .get("bridge_path")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let entrypoint = manifest
        .get("entrypoint")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let safe_path = safe_tool_bridge_path(bridge_path)?;
    if safe_path != "client/runtime/systems/autonomy/swarm_sessions_bridge.ts"
        && safe_path != "adapters/runtime/swarm_bridge_modules.ts"
    {
        return Err(format!("unsupported_tool_bridge:{safe_path}"));
    }

    let args_obj = match args {
        Value::Object(map) => map,
        other => normalize_context_map(other),
    };
    let result = match entrypoint {
        "sessions_send" => {
            let target = tool_target_session_id(&args_obj)
                .ok_or_else(|| "tool_target_session_id_required".to_string())?;
            let message = args_obj
                .get("message")
                .and_then(Value::as_str)
                .ok_or_else(|| "tool_message_required".to_string())?;
            send_session_message(
                state,
                session_id,
                target,
                message,
                DeliveryGuarantee::AtLeastOnce,
                false,
                DEFAULT_MESSAGE_TTL_MS,
            )?
        }
        "sessions_state" => {
            let target = tool_target_session_id(&args_obj).unwrap_or(session_id);
            sessions_state(state, target, false, 16)?
        }
        "sessions_handoff" => {
            let target = tool_target_session_id(&args_obj)
                .ok_or_else(|| "tool_target_session_id_required".to_string())?;
            let reason = args_obj
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or("tool_handoff");
            let importance = args_obj
                .get("importance")
                .and_then(Value::as_f64)
                .unwrap_or(0.5);
            let context = args_obj.get("context").cloned();
            register_handoff(state, session_id, target, reason, importance, context, None)?
        }
        "sessions_context_put" => {
            let context = args_obj
                .get("context")
                .cloned()
                .or_else(|| args_obj.get("context_json").cloned())
                .unwrap_or_else(|| Value::Object(Map::new()));
            let merge = args_obj
                .get("merge")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let session = state
                .sessions
                .get_mut(session_id)
                .ok_or_else(|| format!("unknown_session:{session_id}"))?;
            let receipt = apply_context_update(session, context, merge, "tool_invoke")?;
            json!({
                "ok": true,
                "type": "swarm_runtime_context_put",
                "session_id": session_id,
                "receipt": receipt,
            })
        }
        _ => return Err(format!("unsupported_tool_entrypoint:{entrypoint}")),
    };

    if let Some(updated) = state.tool_registry.get_mut(&key) {
        let count = updated
            .get("invocation_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .saturating_add(1);
        updated["invocation_count"] = json!(count);
        updated["last_invoked_at"] = json!(now_iso());
    }
    append_event(
        state,
        json!({
            "type": "swarm_tool_invoked",
            "session_id": session_id,
            "tool_name": tool_name,
            "entrypoint": entrypoint,
            "timestamp": now_iso(),
        }),
    );
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_tool_invoke",
        "session_id": session_id,
        "tool_name": tool_name,
        "entrypoint": entrypoint,
        "result": result,
    }))
}

fn render_stream_chunks(chunks: &[Value]) -> String {
    chunks
        .iter()
        .map(|row| {
            let agent = row
                .get("agent_label")
                .and_then(Value::as_str)
                .unwrap_or("agent");
            let delimiter = row
                .get("delimiter")
                .and_then(Value::as_str)
                .unwrap_or("agent_delta");
            let content = row
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or_default();
            format!("<agent:{agent}:{delimiter}>{content}")
        })
        .collect::<Vec<_>>()
        .join("")
}

fn stream_emit(
    state: &mut SwarmState,
    session_id: &str,
    turn_id: Option<String>,
    agent_label: Option<String>,
    chunks: Vec<Value>,
) -> Result<Value, String> {
    if !session_exists(state, session_id) {
        return Err(format!("unknown_session:{session_id}"));
    }
    if chunks.is_empty() {
        return Err("stream_chunks_required".to_string());
    }
    let turn_id = turn_id.unwrap_or_else(|| {
        format!(
            "turn-{}",
            &deterministic_receipt_hash(&json!({
                "session_id": session_id,
                "ts": now_epoch_ms(),
                "kind": "stream",
            }))[..12]
        )
    });
    let label = agent_label
        .or_else(|| {
            state
                .sessions
                .get(session_id)
                .and_then(|session| session.agent_label.clone().or(session.role.clone()))
        })
        .unwrap_or_else(|| session_id.to_string());
    let key = format!("{session_id}:{turn_id}");
    let mut emitted = Vec::new();
    let total = chunks.len();
    for (idx, chunk) in chunks.into_iter().enumerate() {
        let raw = match chunk {
            Value::Object(map) => Value::Object(map),
            other => json!({ "content": other }),
        };
        let delimiter = raw
            .get("delimiter")
            .or_else(|| raw.get("boundary"))
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .unwrap_or_else(|| {
                if idx == 0 {
                    "agent_start".to_string()
                } else if idx + 1 == total {
                    "agent_end".to_string()
                } else {
                    "agent_delta".to_string()
                }
            });
        let content = raw
            .get("content")
            .or_else(|| raw.get("text"))
            .map(|value| match value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            })
            .unwrap_or_default();
        let chunk_receipt = json!({
            "chunk_id": format!("chunk-{}", &deterministic_receipt_hash(&json!({
                "session_id": session_id,
                "turn_id": turn_id,
                "idx": idx,
                "content": content,
            }))[..12]),
            "session_id": session_id,
            "turn_id": turn_id,
            "agent_label": label,
            "delimiter": delimiter,
            "content": content,
            "partial": idx + 1 != total,
            "sequence": idx,
            "timestamp_ms": now_epoch_ms(),
        });
        emitted.push(chunk_receipt.clone());
        state
            .stream_registry
            .entry(key.clone())
            .or_default()
            .push(chunk_receipt.clone());
        append_event(
            state,
            json!({
                "type": "swarm_stream_chunk",
                "session_id": session_id,
                "turn_id": turn_id,
                "agent_label": label,
                "delimiter": chunk_receipt.get("delimiter").cloned().unwrap_or(Value::Null),
                "timestamp": now_iso(),
            }),
        );
    }
    if let Some(session) = state.sessions.get_mut(session_id) {
        if !session.stream_turn_ids.iter().any(|row| row == &turn_id) {
            session.stream_turn_ids.push(turn_id.clone());
        }
    }
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_stream_emit",
        "session_id": session_id,
        "turn_id": turn_id,
        "chunk_count": emitted.len(),
        "chunks": emitted,
        "rendered": render_stream_chunks(state.stream_registry.get(&key).map(|rows| rows.as_slice()).unwrap_or(&[])),
    }))
}

fn stream_render(
    state: &SwarmState,
    session_id: &str,
    turn_id: Option<&str>,
) -> Result<Value, String> {
    if !session_exists(state, session_id) {
        return Err(format!("unknown_session:{session_id}"));
    }
    let mut rows = Vec::new();
    for (key, chunks) in &state.stream_registry {
        if !key.starts_with(&format!("{session_id}:")) {
            continue;
        }
        if let Some(expected_turn) = turn_id {
            if !key.ends_with(&format!(":{expected_turn}")) {
                continue;
            }
        }
        rows.extend(chunks.clone());
    }
    rows.sort_by_key(|row| row.get("sequence").and_then(Value::as_u64).unwrap_or(0));
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_stream_render",
        "session_id": session_id,
        "turn_id": turn_id,
        "chunk_count": rows.len(),
        "chunks": rows.clone(),
        "rendered": render_stream_chunks(&rows),
    }))
}

fn execute_turns(
    state: &mut SwarmState,
    session_id: &str,
    turns: Vec<Value>,
    run_label: Option<String>,
) -> Result<Value, String> {
    if !session_exists(state, session_id) {
        return Err(format!("unknown_session:{session_id}"));
    }
    if turns.is_empty() {
        return Err("turns_required".to_string());
    }
    let run_id = format!(
        "run-{}",
        &deterministic_receipt_hash(&json!({
            "session_id": session_id,
            "run_label": run_label,
            "turn_count": turns.len(),
            "ts": now_epoch_ms(),
        }))[..12]
    );
    let mut receipts = Vec::new();
    let mut failed = false;
    for (idx, turn) in turns.into_iter().enumerate() {
        let raw = match turn {
            Value::Object(map) => Value::Object(map),
            other => json!({ "message": other }),
        };
        let message = raw
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let tool_name = raw
            .get("tool_name")
            .or_else(|| raw.get("tool"))
            .and_then(Value::as_str)
            .map(ToString::to_string);
        let recovery = raw
            .get("recovery")
            .and_then(Value::as_str)
            .unwrap_or("fail_closed")
            .to_string();
        let fail_first_attempt = raw
            .get("fail_first_attempt")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let mut recovery_action: Option<String> = None;
        if let Some(context_patch) = raw
            .get("context_patch")
            .or_else(|| raw.get("context"))
            .cloned()
        {
            let session = state
                .sessions
                .get_mut(session_id)
                .ok_or_else(|| format!("unknown_session:{session_id}"))?;
            let _ = apply_context_update(session, context_patch, true, "turn_run")?;
        }
        if fail_first_attempt {
            let error_receipt = json!({
                "turn_index": idx,
                "status": "error",
                "error": "simulated_transient_error",
                "attempt": 1,
                "message": message,
            });
            receipts.push(error_receipt);
            if recovery == "retry_once" {
                recovery_action = Some("retry_once".to_string());
            } else {
                failed = true;
                break;
            }
        }
        let result = if let Some(tool_name) = tool_name.clone() {
            let args = raw
                .get("tool_args")
                .cloned()
                .unwrap_or_else(|| Value::Object(Map::new()));
            invoke_registered_tool(state, session_id, &tool_name, args)?
        } else {
            json!({
                "ok": true,
                "type": "swarm_runtime_turn_message",
                "session_id": session_id,
                "message": message,
            })
        };
        let emit_stream = raw
            .get("emit_stream")
            .and_then(Value::as_bool)
            .unwrap_or(!message.is_empty());
        let stream_receipt = if emit_stream {
            Some(stream_emit(
                state,
                session_id,
                Some(format!("{run_id}-turn-{}", idx + 1)),
                None,
                vec![
                    json!({ "delimiter": "agent_start", "content": format!("turn:{}:", idx + 1) }),
                    json!({ "delimiter": "agent_delta", "content": if message.is_empty() { tool_name.clone().unwrap_or_else(|| "tool".to_string()) } else { message.clone() } }),
                    json!({ "delimiter": "agent_end", "content": "" }),
                ],
            )?)
        } else {
            None
        };
        receipts.push(json!({
            "turn_index": idx,
            "status": "ok",
            "message": message,
            "tool_name": tool_name,
            "recovery_action": recovery_action,
            "result": result,
            "stream": stream_receipt,
        }));
    }
    let run_receipt = json!({
        "run_id": run_id,
        "session_id": session_id,
        "label": run_label,
        "status": if failed { "failed" } else { "completed" },
        "turns": receipts,
        "completed_at": now_iso(),
    });
    state
        .turn_registry
        .insert(run_id.clone(), run_receipt.clone());
    if let Some(session) = state.sessions.get_mut(session_id) {
        if !session.turn_run_ids.iter().any(|row| row == &run_id) {
            session.turn_run_ids.push(run_id.clone());
        }
    }
    append_event(
        state,
        json!({
            "type": "swarm_turn_run",
            "run_id": run_id,
            "session_id": session_id,
            "status": if failed { "failed" } else { "completed" },
            "timestamp": now_iso(),
        }),
    );
    Ok(json!({
        "ok": !failed,
        "type": "swarm_runtime_turn_run",
        "run": run_receipt,
    }))
}
