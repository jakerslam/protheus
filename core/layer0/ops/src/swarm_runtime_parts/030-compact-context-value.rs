fn compact_context_value(value: &Value) -> Value {
    match value {
        Value::String(text) => Value::String(clean_text(text, 160)),
        Value::Array(rows) => Value::Array(
            rows.iter()
                .take(8)
                .map(compact_context_value)
                .collect::<Vec<_>>(),
        ),
        Value::Object(map) => {
            let mut out = Map::new();
            for (key, value) in map.iter().take(12) {
                out.insert(clean_text(key, 64), compact_context_value(value));
            }
            Value::Object(out)
        }
        _ => value.clone(),
    }
}
fn normalize_context_map(input: Value) -> Map<String, Value> {
    match input {
        Value::Object(map) => map,
        other => {
            let mut out = Map::new();
            out.insert("value".to_string(), other);
            out
        }
    }
}

fn safe_registry_slug(raw: &str, max_len: usize) -> String {
    let mut out = String::new();
    for ch in raw.trim().chars() {
        if out.len() >= max_len {
            break;
        }
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if matches!(ch, '-' | '_' | '.' | ':') {
            out.push(ch);
        } else if ch.is_whitespace() && !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

fn state_path(root: &Path, argv: &[String]) -> PathBuf {
    parse_flag(argv, "state-path")
        .filter(|v| !v.trim().is_empty())
        .map(|v| {
            let candidate = PathBuf::from(v.trim());
            if candidate.is_absolute() {
                candidate
            } else {
                root.join(candidate)
            }
        })
        .unwrap_or_else(|| root.join(DEFAULT_STATE_PATH))
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    fs::create_dir_all(parent).map_err(|err| format!("mkdir_failed:{}:{err}", parent.display()))
}

fn load_state(path: &Path) -> Result<SwarmState, String> {
    if !path.exists() {
        return Ok(SwarmState::default());
    }
    let raw = fs::read_to_string(path).map_err(|err| format!("state_read_failed:{err}"))?;
    if raw.trim().is_empty() {
        return Ok(SwarmState::default());
    }
    serde_json::from_str::<SwarmState>(&raw).map_err(|err| format!("state_parse_failed:{err}"))
}

fn save_state(path: &Path, state: &SwarmState) -> Result<(), String> {
    ensure_parent(path)?;
    let encoded =
        serde_json::to_string_pretty(state).map_err(|err| format!("state_encode_failed:{err}"))?;
    fs::write(path, encoded).map_err(|err| format!("state_write_failed:{err}"))
}

fn now_epoch_ms() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => u64::try_from(duration.as_millis()).unwrap_or(0),
        Err(_) => 0,
    }
}

fn print_receipt(mut payload: Value) {
    payload["receipt_hash"] = Value::String(deterministic_receipt_hash(&payload));
    println!(
        "{}",
        serde_json::to_string(&payload)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn append_event(state: &mut SwarmState, event: Value) {
    state.events.push(event);
    if state.events.len() > MAX_EVENT_ROWS {
        let excess = state.events.len() - MAX_EVENT_ROWS;
        state.events.drain(0..excess);
    }
}

fn append_dead_letter(
    state: &mut SwarmState,
    message: AgentMessage,
    reason: &str,
    retryable: bool,
) {
    let dead_letter = DeadLetterMessage {
        dead_letter_id: format!(
            "dlq-{}",
            &deterministic_receipt_hash(&json!({
                "message_id": message.message_id,
                "reason": reason,
                "moved_at_ms": now_epoch_ms(),
            }))[..12]
        ),
        message,
        reason: reason.to_string(),
        moved_at: now_iso(),
        moved_at_ms: now_epoch_ms(),
        retryable,
        retry_count: 0,
    };
    state.dead_letters.push(dead_letter);
    if state.dead_letters.len() > MAX_DEAD_LETTER_ROWS {
        let excess = state.dead_letters.len() - MAX_DEAD_LETTER_ROWS;
        state.dead_letters.drain(0..excess);
    }
}

fn session_key(session_id: &str) -> String {
    format!("agent:main:subagent:{session_id}")
}

fn session_capabilities(state: &SwarmState, session_id: &str) -> Vec<String> {
    state
        .service_registry
        .values()
        .flat_map(|rows| rows.iter())
        .find(|row| row.session_id == session_id)
        .map(|row| row.capabilities.clone())
        .unwrap_or_default()
}

fn session_transport_contract(session_id: &str) -> Value {
    let session_key = session_key(session_id);
    json!({
        "bridge_path": "client/runtime/systems/autonomy/swarm_sessions_bridge.ts",
        "sessions_send": format!("node client/runtime/systems/autonomy/swarm_sessions_bridge.ts sessions_send --sender={session_key} --sessionKey=<target> --message=<text>"),
        "sessions_receive": format!("node client/runtime/systems/autonomy/swarm_sessions_bridge.ts sessions_receive --sessionKey={session_key}"),
        "sessions_ack": format!("node client/runtime/systems/autonomy/swarm_sessions_bridge.ts sessions_ack --sessionKey={session_key} --message-id=<id>"),
        "sessions_handoff": format!("node client/runtime/systems/autonomy/swarm_sessions_bridge.ts sessions_handoff --sessionKey={session_key} --targetSessionKey=<target> --reason=<text>"),
        "sessions_context_put": format!("node client/runtime/systems/autonomy/swarm_sessions_bridge.ts sessions_context_put --sessionKey={session_key} --context-json=<json>"),
        "sessions_context_get": format!("node client/runtime/systems/autonomy/swarm_sessions_bridge.ts sessions_context_get --sessionKey={session_key}"),
        "sessions_state": format!("node client/runtime/systems/autonomy/swarm_sessions_bridge.ts sessions_state --sessionKey={session_key}"),
        "sessions_bootstrap": format!("node client/runtime/systems/autonomy/swarm_sessions_bridge.ts sessions_bootstrap --sessionKey={session_key}"),
        "sessions_tick": "node client/runtime/systems/autonomy/swarm_sessions_bridge.ts sessions_tick".to_string(),
        "tools_register_json_schema": format!("node client/runtime/systems/autonomy/swarm_sessions_bridge.ts tools_register_json_schema --sessionKey={session_key} --toolName=<name> --schema-json=<json> --bridgePath=<path> --entrypoint=<name>"),
        "tools_invoke": format!("node client/runtime/systems/autonomy/swarm_sessions_bridge.ts tools_invoke --sessionKey={session_key} --toolName=<name> --args-json=<json>"),
        "stream_emit": format!("node client/runtime/systems/autonomy/swarm_sessions_bridge.ts stream_emit --sessionKey={session_key} --agentLabel=<label> --chunks-json=<json>"),
        "stream_render": format!("node client/runtime/systems/autonomy/swarm_sessions_bridge.ts stream_render --sessionKey={session_key}"),
        "turns_run": format!("node client/runtime/systems/autonomy/swarm_sessions_bridge.ts turns_run --sessionKey={session_key} --turns-json=<json>"),
        "turns_show": format!("node client/runtime/systems/autonomy/swarm_sessions_bridge.ts turns_show --sessionKey={session_key} --runId=<id>"),
        "networks_create": format!("node client/runtime/systems/autonomy/swarm_sessions_bridge.ts networks_create --sessionKey={session_key} --spec-json=<json>"),
        "networks_status": format!("node client/runtime/systems/autonomy/swarm_sessions_bridge.ts networks_status --sessionKey={session_key} --networkId=<id>"),
    })
}

fn session_budget_bootstrap(session: &SessionMetadata) -> Value {
    if let Some(telemetry) = session.budget_telemetry.as_ref() {
        json!({
            "configured": true,
            "budget": telemetry.budget_config.max_tokens,
            "warning_threshold": telemetry.budget_config.warning_threshold,
            "on_budget_exhausted": telemetry.budget_config.exhaustion_action.as_label(),
            "remaining_tokens": telemetry.remaining_tokens(),
            "final_usage": telemetry.final_usage,
            "budget_exhausted": telemetry.budget_exhausted,
            "reserved_for_children": telemetry.reserved_for_children,
            "settled_child_tokens": telemetry.settled_child_tokens,
        })
    } else {
        json!({
            "configured": false,
            "note": "set --token-budget or --max-tokens to enable fail-closed swarm budget enforcement"
        })
    }
}

fn session_bootstrap_contract(_state: &SwarmState, session: &SessionMetadata) -> Value {
    let session_id = session.session_id.as_str();
    let session_key = session_key(session_id);
    let transport = session_transport_contract(session_id);
    let budget = session_budget_bootstrap(session);
    let send_cmd = transport
        .get("sessions_send")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let receive_cmd = transport
        .get("sessions_receive")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let ack_cmd = transport
        .get("sessions_ack")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let state_cmd = transport
        .get("sessions_state")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let budget_summary = if budget
        .get("configured")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        format!(
            "Budget is hard-governed at {} tokens with '{}' exhaustion handling and {} remaining tokens.",
            budget
                .get("budget")
                .and_then(Value::as_u64)
                .unwrap_or_default(),
            budget
                .get("on_budget_exhausted")
                .and_then(Value::as_str)
                .unwrap_or("fail"),
            budget
                .get("remaining_tokens")
                .and_then(Value::as_u64)
                .unwrap_or_default()
        )
    } else {
        "No explicit token budget is configured for this session yet.".to_string()
    };
    json!({
        "version": "swarm-agent-bootstrap/v1",
        "session_id": session_id,
        "session_key": session_key,
        "commands": transport,
        "budget": budget,
        "fallback_policy": "use direct swarm bridge commands first; only use file relay if the bridge is unavailable",
        "prompt": format!(
            "You are swarm session {session_key}. Use direct swarm bridge commands instead of file relays. Send messages with `{send_cmd}`. Read inbound messages with `{receive_cmd}`. Ack processed messages with `{ack_cmd}`. Inspect live state with `{state_cmd}`. {budget_summary}"
        ),
    })
}

fn session_tool_manifest(state: &SwarmState, session: &SessionMetadata) -> Value {
    let session_id = session.session_id.as_str();
    json!({
        "version": "swarm-tool-manifest/v1",
        "session_id": session_id,
        "session_key": session_key(session_id),
        "tool_access": session.tool_access.clone(),
        "role": session.role.clone(),
        "capabilities": session_capabilities(state, session_id),
        "transport": session_transport_contract(session_id),
        "handoff_registry_size": state.handoff_registry.len(),
        "tool_registry_size": state.tool_registry.len(),
        "network_registry_size": state.network_registry.len(),
        "resumption": {
            "persistent": session.persistent.is_some(),
            "resume_command": format!("protheus-ops swarm-runtime sessions resume --session-id={session_id}"),
        },
        "agent_bootstrap": session_bootstrap_contract(state, session),
    })
}

fn sessions_bootstrap(state: &SwarmState, session_id: &str) -> Result<Value, String> {
    let Some(session) = state.sessions.get(session_id) else {
        return Err(format!("unknown_session:{session_id}"));
    };
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_session_bootstrap",
        "session_id": session_id,
        "session_key": session_key(session_id),
        "bootstrap": session_bootstrap_contract(state, session),
    }))
}

fn reserve_budget_from_parent(
    state: &mut SwarmState,
    parent_id: Option<&str>,
    child_id: &str,
    requested_budget: Option<u32>,
) -> Result<(Option<u32>, Option<String>, u32), String> {
    let Some(parent_id) = parent_id else {
        return Ok((requested_budget, None, 0));
    };
    let Some(parent_session) = state.sessions.get_mut(parent_id) else {
        return Err(format!("parent_session_missing:{parent_id}"));
    };
    let Some(telemetry) = parent_session.budget_telemetry.as_mut() else {
        return Ok((requested_budget, None, 0));
    };
    let remaining = telemetry.remaining_tokens();
    if remaining == 0 {
        return Err(format!(
            "parent_token_budget_exceeded:parent={parent_id}:remaining=0"
        ));
    }
    let child_budget = requested_budget.unwrap_or(remaining);
    if child_budget > remaining {
        return Err(format!(
            "parent_token_budget_exceeded:parent={parent_id}:requested={child_budget}:remaining={remaining}"
        ));
    }
    telemetry.reserved_for_children = telemetry.reserved_for_children.saturating_add(child_budget);
    telemetry
        .child_reservations
        .insert(child_id.to_string(), child_budget);
    Ok((
        Some(child_budget),
        Some(parent_id.to_string()),
        child_budget,
    ))
}

fn settle_budget_reservation(state: &mut SwarmState, session_id: &str) {
    let settlement = {
        let Some(session) = state.sessions.get(session_id) else {
            return;
        };
        if session.budget_reservation_settled || session.budget_reservation_tokens == 0 {
            return;
        }
        let Some(parent_id) = session.budget_parent_session_id.clone() else {
            return;
        };
        let child_usage = session
            .budget_telemetry
            .as_ref()
            .map(|telemetry| telemetry.final_usage)
            .unwrap_or(0);
        (
            parent_id,
            session.budget_reservation_tokens,
            child_usage,
            format!("child_session:{session_id}"),
        )
    };

    let (parent_id, reserved_tokens, child_usage, child_tool) = settlement;
    if let Some(parent_session) = state.sessions.get_mut(&parent_id) {
        if let Some(telemetry) = parent_session.budget_telemetry.as_mut() {
            telemetry.reserved_for_children = telemetry
                .reserved_for_children
                .saturating_sub(reserved_tokens);
            telemetry.child_reservations.remove(session_id);
            telemetry.settled_child_tokens =
                telemetry.settled_child_tokens.saturating_add(child_usage);
            if child_usage > 0 {
                telemetry.push_usage(&child_tool, child_usage);
                if !telemetry.warning_emitted
                    && telemetry.final_usage >= telemetry.warning_at_tokens
                {
                    telemetry.warning_emitted = true;
                }
            }
        }
    }
    if let Some(session) = state.sessions.get_mut(session_id) {
        session.budget_reservation_settled = true;
    }
}

fn drain_expired_messages(state: &mut SwarmState, now_ms: u64) {
    let mut expired = Vec::new();
    for mailbox in state.mailboxes.values_mut() {
        let mut remaining = Vec::new();
        for message in mailbox.unread.drain(..) {
            if message.expires_at_ms <= now_ms {
                expired.push(message);
            } else {
                remaining.push(message);
            }
        }
        mailbox.unread = remaining;
    }
    for message in expired {
        let recipient_session_id = message.recipient_session_id.clone();
        let message_id = message.message_id.clone();
        append_dead_letter(state, message, "message_ttl_expired", true);
        append_event(
            state,
            json!({
                "type": "swarm_dead_letter",
                "message_id": message_id,
                "recipient_session_id": recipient_session_id,
                "reason": "message_ttl_expired",
                "timestamp": now_iso(),
            }),
        );
    }
}

fn recover_persistent_sessions_after_reload(state: &mut SwarmState, now_ms: u64) {
    let mut resumed = Vec::new();
    for (session_id, session) in state.sessions.iter_mut() {
        let Some(runtime) = session.persistent.as_mut() else {
            continue;
        };
        if runtime.terminated_at_ms.is_some() {
            continue;
        }
        if !matches!(
            session.status.as_str(),
            "persistent_running" | "background_running" | "running"
        ) {
            continue;
        }
        session.reachable = true;
        if runtime.next_check_in_ms < now_ms {
            runtime.next_check_in_ms = now_ms;
        }
        resumed.push(session_id.clone());
    }
    if !resumed.is_empty() {
        append_event(
            state,
            json!({
                "type": "swarm_restart_recovery",
                "resumed_sessions": resumed,
                "timestamp": now_iso(),
            }),
        );
    }
}

fn next_session_id(state: &SwarmState, task: &str, depth: u8) -> String {
    let mut salt = 0u64;
    loop {
        let candidate_seed = json!({
            "task": task,
            "depth": depth,
            "salt": salt,
            "ts": now_epoch_ms()
        });
        let digest = deterministic_receipt_hash(&candidate_seed);
        let candidate = format!("swarm-{}", &digest[..12]);
        if !state.sessions.contains_key(&candidate) {
            return candidate;
        }
        salt = salt.saturating_add(1);
    }
}

fn thorn_cell_limit(state: &SwarmState) -> usize {
    (((state.sessions.len().max(1) as f64) * 0.10).ceil() as usize).max(1)
}
