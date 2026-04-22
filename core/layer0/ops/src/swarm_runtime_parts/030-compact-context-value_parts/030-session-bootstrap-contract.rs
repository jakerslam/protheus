
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
        let digest = crate::deterministic_receipt_hash(&candidate_seed);
        let candidate = format!("swarm-{}", &digest[..12]);
        if !state.sessions.contains_key(&candidate) {
            return candidate;
        }
        salt = salt.saturating_add(1);
    }
}
