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
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            for key in keys.into_iter().take(12) {
                if let Some(value) = map.get(&key) {
                    out.insert(clean_text(&key, 64), compact_context_value(value));
                }
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

#[derive(Debug, Clone)]
struct StateCacheEntry {
    modified_ms: u128,
    byte_len: u64,
    state: SwarmState,
}

fn state_cache() -> &'static Mutex<BTreeMap<String, StateCacheEntry>> {
    static STATE_CACHE: OnceLock<Mutex<BTreeMap<String, StateCacheEntry>>> = OnceLock::new();
    STATE_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn state_file_fingerprint(path: &Path) -> Option<(u128, u64)> {
    let metadata = fs::metadata(path).ok()?;
    let modified_ms = metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis();
    Some((modified_ms, metadata.len()))
}

fn load_cached_state(path: &Path, modified_ms: u128, byte_len: u64) -> Option<SwarmState> {
    let key = path.to_string_lossy().to_string();
    let guard = state_cache().lock().ok()?;
    guard.get(&key).and_then(|entry| {
        (entry.modified_ms == modified_ms && entry.byte_len == byte_len)
            .then(|| entry.state.clone())
    })
}

fn store_cached_state(path: &Path, modified_ms: u128, byte_len: u64, state: &SwarmState) {
    let key = path.to_string_lossy().to_string();
    if let Ok(mut guard) = state_cache().lock() {
        guard.insert(
            key,
            StateCacheEntry {
                modified_ms,
                byte_len,
                state: state.clone(),
            },
        );
    }
}

fn clear_cached_state(path: &Path) {
    let key = path.to_string_lossy().to_string();
    if let Ok(mut guard) = state_cache().lock() {
        guard.remove(&key);
    }
}

fn total_mailbox_message_count(state: &SwarmState) -> usize {
    state.mailboxes.values().fold(0usize, |acc, mailbox| {
        acc.saturating_add(mailbox.unread.len().saturating_add(mailbox.read.len()))
    })
}

fn should_pretty_encode_state(state: &SwarmState) -> bool {
    state.sessions.len() <= STATE_PRETTY_MAX_SESSIONS
        && total_mailbox_message_count(state) <= STATE_PRETTY_MAX_MAILBOX_MESSAGES
        && state.events.len() <= STATE_PRETTY_MAX_EVENT_ROWS
        && state.dead_letters.len() <= STATE_PRETTY_MAX_DEAD_LETTERS
}

fn load_state(path: &Path) -> Result<SwarmState, String> {
    if !path.exists() {
        clear_cached_state(path);
        return Ok(SwarmState::default());
    }
    if let Some((modified_ms, byte_len)) = state_file_fingerprint(path) {
        if let Some(state) = load_cached_state(path, modified_ms, byte_len) {
            return Ok(state);
        }
    }
    let raw = fs::read_to_string(path).map_err(|err| format!("state_read_failed:{err}"))?;
    if raw.trim().is_empty() {
        if let Some((modified_ms, byte_len)) = state_file_fingerprint(path) {
            store_cached_state(path, modified_ms, byte_len, &SwarmState::default());
        }
        return Ok(SwarmState::default());
    }
    let parsed = serde_json::from_str::<SwarmState>(&raw)
        .map_err(|err| format!("state_parse_failed:{err}"))?;
    if let Some((modified_ms, byte_len)) = state_file_fingerprint(path) {
        store_cached_state(path, modified_ms, byte_len, &parsed);
    }
    Ok(parsed)
}

fn save_state(path: &Path, state: &SwarmState) -> Result<(), String> {
    ensure_parent(path)?;
    let encoded = if should_pretty_encode_state(state) {
        serde_json::to_string_pretty(state).map_err(|err| format!("state_encode_failed:{err}"))?
    } else {
        serde_json::to_string(state).map_err(|err| format!("state_encode_failed:{err}"))?
    };
    fs::write(path, encoded).map_err(|err| format!("state_write_failed:{err}"))?;
    if let Some((modified_ms, byte_len)) = state_file_fingerprint(path) {
        store_cached_state(path, modified_ms, byte_len, state);
    } else {
        clear_cached_state(path);
    }
    Ok(())
}

fn effective_spawn_max_depth(state: &SwarmState, requested_max_depth: u8) -> u8 {
    requested_max_depth
        .max(1)
        .min(state.scale_policy.max_depth_hard.max(1))
}

fn recommended_manager_fanout_for_target(target_agents: usize) -> usize {
    if target_agents >= 100_000 {
        32
    } else if target_agents >= 10_000 {
        24
    } else if target_agents >= 1_000 {
        12
    } else if target_agents >= 500 {
        10
    } else if target_agents >= 100 {
        8
    } else {
        5
    }
}

fn compute_hierarchy_topology(target_agents: usize, fanout: usize) -> Value {
    let target_agents = target_agents.max(1);
    let fanout = fanout.max(2);

    let mut remaining = target_agents;
    let mut level_capacity = 1usize;
    let mut level = 0usize;
    let mut level_counts: Vec<usize> = Vec::new();
    let mut levels = Vec::new();

    while remaining > 0 {
        let count = remaining.min(level_capacity);
        level_counts.push(count);
        levels.push(json!({
            "level": level,
            "agents": count,
            "capacity": level_capacity,
        }));
        remaining = remaining.saturating_sub(count);
        if remaining == 0 {
            break;
        }
        level = level.saturating_add(1);
        level_capacity = level_capacity.saturating_mul(fanout);
        if level > 512 {
            break;
        }
    }

    let mut managers_by_level = Vec::new();
    let mut manager_count = 0usize;
    for idx in 1..level_counts.len() {
        let children_at_level = level_counts[idx];
        let manager_agents = (children_at_level.saturating_add(fanout).saturating_sub(1)) / fanout;
        manager_count = manager_count.saturating_add(manager_agents);
        managers_by_level.push(json!({
            "level": idx - 1,
            "manager_agents": manager_agents,
        }));
    }
    let leaf_count = target_agents.saturating_sub(manager_count);
    let required_depth = level_counts.len().saturating_sub(1);

    json!({
        "target_agents": target_agents,
        "fanout": fanout,
        "required_depth": required_depth,
        "levels": levels,
        "managers_by_level": managers_by_level,
        "manager_count": manager_count,
        "leaf_count": leaf_count,
        "manager_ratio": if target_agents == 0 { 0.0 } else { manager_count as f64 / target_agents as f64 },
    })
}

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
