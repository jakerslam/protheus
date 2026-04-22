
fn evaluate_scale_policy_readiness(
    state: &SwarmState,
    target_agents: usize,
    fanout: usize,
) -> Value {
    let topology = compute_hierarchy_topology(target_agents, fanout);
    let required_depth = topology
        .get("required_depth")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u8;
    let sessions_total = state.sessions.len();
    let max_children_current = state
        .sessions
        .values()
        .map(|session| session.children.len())
        .max()
        .unwrap_or(0);
    let utilization = if state.scale_policy.max_sessions_hard == 0 {
        0.0
    } else {
        sessions_total as f64 / state.scale_policy.max_sessions_hard as f64
    };

    json!({
        "policy": {
            "max_sessions_hard": state.scale_policy.max_sessions_hard,
            "max_children_per_parent": state.scale_policy.max_children_per_parent,
            "max_depth_hard": state.scale_policy.max_depth_hard,
            "target_ready_agents": state.scale_policy.target_ready_agents,
            "enforce_session_cap": state.scale_policy.enforce_session_cap,
            "enforce_parent_capacity": state.scale_policy.enforce_parent_capacity,
        },
        "current_load": {
            "sessions_total": sessions_total,
            "max_children_current": max_children_current,
            "session_cap_utilization": utilization,
            "utilization_alert": utilization >= SCALE_UTILIZATION_ALERT_THRESHOLD,
        },
        "readiness": {
            "target_agents": target_agents,
            "requested_fanout": fanout,
            "within_session_cap": target_agents <= state.scale_policy.max_sessions_hard,
            "within_depth_cap": required_depth <= state.scale_policy.max_depth_hard,
            "within_parent_capacity": fanout <= state.scale_policy.max_children_per_parent,
            "required_depth": required_depth,
            "recommended_fanout": recommended_manager_fanout_for_target(target_agents),
        },
        "topology": topology,
    })
}

fn validate_spawn_capacity(
    state: &SwarmState,
    parent_id: Option<&str>,
    depth: u8,
    requested_max_depth: u8,
) -> Result<u8, String> {
    let effective_max_depth = effective_spawn_max_depth(state, requested_max_depth);
    if depth >= effective_max_depth {
        return Err(format!(
            "max_depth_exceeded:{depth}>=max_depth:{effective_max_depth}"
        ));
    }

    if state.scale_policy.enforce_session_cap
        && state.sessions.len() >= state.scale_policy.max_sessions_hard
    {
        return Err(format!(
            "session_capacity_exceeded:current={}:cap={}",
            state.sessions.len(),
            state.scale_policy.max_sessions_hard
        ));
    }

    if state.scale_policy.enforce_parent_capacity {
        if let Some(parent) = parent_id {
            let parent_children = state
                .sessions
                .get(parent)
                .map(|session| session.children.len())
                .unwrap_or(0);
            if parent_children >= state.scale_policy.max_children_per_parent {
                return Err(format!(
                    "parent_capacity_exceeded:parent={parent}:children={parent_children}:cap={}",
                    state.scale_policy.max_children_per_parent
                ));
            }
        }
    }

    Ok(effective_max_depth)
}

fn resolve_spawn_depth(state: &SwarmState, parent_id: Option<&str>) -> Result<u8, String> {
    match parent_id {
        Some(parent) => state
            .sessions
            .get(parent)
            .map(|session| session.depth.saturating_add(1))
            .ok_or_else(|| format!("parent_session_missing:{parent}")),
        None => Ok(0),
    }
}

fn ensure_spawn_capacity(
    state: &SwarmState,
    parent_id: Option<&str>,
    requested_max_depth: u8,
) -> Result<u8, String> {
    let depth = resolve_spawn_depth(state, parent_id)?;
    let _ = validate_spawn_capacity(state, parent_id, depth, requested_max_depth)?;
    Ok(depth)
}

fn now_epoch_ms() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => u64::try_from(duration.as_millis()).unwrap_or(0),
        Err(_) => 0,
    }
}

fn print_receipt(mut payload: Value) {
    payload["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&payload));
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
            &crate::deterministic_receipt_hash(&json!({
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
