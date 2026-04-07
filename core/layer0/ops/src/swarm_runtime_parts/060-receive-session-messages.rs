fn receive_session_messages(
    state: &mut SwarmState,
    session_id: &str,
    limit: usize,
    mark_read: bool,
) -> Result<Value, String> {
    if !session_exists(state, session_id) {
        return Err(format!("unknown_session:{session_id}"));
    }
    let mailbox = ensure_mailbox(state, session_id);
    let take = mailbox.unread.len().min(limit.max(1));
    let mut messages = mailbox
        .unread
        .iter()
        .take(take)
        .cloned()
        .collect::<Vec<_>>();
    if mark_read && !messages.is_empty() {
        let remaining = mailbox.unread.split_off(take);
        mailbox.read.extend(messages.iter().cloned());
        mailbox.unread = remaining;
    }
    messages.sort_by_key(|message| message.timestamp_ms);
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_sessions_receive",
        "session_id": session_id,
        "message_count": messages.len(),
        "messages": messages,
    }))
}

fn acknowledge_session_message(
    state: &mut SwarmState,
    session_id: &str,
    message_id: &str,
) -> Result<Value, String> {
    if !session_exists(state, session_id) {
        return Err(format!("unknown_session:{session_id}"));
    }
    let mailbox = ensure_mailbox(state, session_id);
    let mark_ack = |message: &mut AgentMessage| {
        message.acknowledged = true;
        message.acked_at_ms = Some(now_epoch_ms());
    };
    if let Some(message) = mailbox
        .unread
        .iter_mut()
        .find(|row| row.message_id == message_id)
    {
        mark_ack(message);
    } else if let Some(message) = mailbox
        .read
        .iter_mut()
        .find(|row| row.message_id == message_id)
    {
        mark_ack(message);
    } else {
        return Err(format!("unknown_message:{message_id}"));
    }
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_sessions_ack",
        "session_id": session_id,
        "message_id": message_id,
        "acknowledged": true,
    }))
}

fn register_service_instance(
    state: &mut SwarmState,
    session_id: &str,
    role: Option<String>,
    capabilities: Vec<String>,
) {
    let Some(role_name) = role else {
        return;
    };
    let entries = state.service_registry.entry(role_name.clone()).or_default();
    entries.retain(|row| row.session_id != session_id);
    entries.push(ServiceInstance {
        session_id: session_id.to_string(),
        role: role_name,
        capabilities,
        healthy: true,
        registered_at: now_iso(),
    });
}
fn mark_service_instance_unhealthy(state: &mut SwarmState, session_id: &str) {
    for instances in state.service_registry.values_mut() {
        for instance in instances.iter_mut() {
            if instance.session_id == session_id {
                instance.healthy = false;
            }
        }
    }
}

fn discover_services(state: &SwarmState, role: &str) -> Vec<ServiceInstance> {
    state
        .service_registry
        .get(role)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|instance| {
            instance.healthy
                && state
                    .sessions
                    .get(&instance.session_id)
                    .map(|session| session.reachable)
                    .unwrap_or(false)
        })
        .collect::<Vec<_>>()
}

fn send_to_role(
    state: &mut SwarmState,
    sender_session_id: &str,
    role: &str,
    payload: &str,
    delivery: DeliveryGuarantee,
    ttl_ms: u64,
) -> Result<Value, String> {
    let services = discover_services(state, role);
    if services.is_empty() {
        return Err(format!("no_healthy_instances_for_role:{role}"));
    }
    let cursor = state
        .role_dispatch_cursor
        .entry(role.to_string())
        .or_insert(0usize);
    let start_index = *cursor % services.len();
    let mut selected: Option<&ServiceInstance> = None;
    for offset in 0..services.len() {
        let index = (start_index + offset) % services.len();
        let candidate = &services[index];
        let unread_depth = state
            .mailboxes
            .get(&candidate.session_id)
            .map(|mailbox| mailbox.unread.len())
            .unwrap_or(0);
        if unread_depth < MAX_MAILBOX_UNREAD {
            selected = Some(candidate);
            *cursor = index.wrapping_add(1);
            break;
        }
    }

    let Some(recipient) = selected else {
        return Err(format!("role_backpressure_exhausted:{role}"));
    };
    send_session_message(
        state,
        sender_session_id,
        &recipient.session_id,
        payload,
        delivery,
        false,
        ttl_ms.max(1),
    )
}

fn session_context_json(session: &SessionMetadata) -> Value {
    Value::Object(
        session
            .context_vars
            .clone()
            .into_iter()
            .collect::<Map<String, Value>>(),
    )
}

include!("055-handoff-context-isolation.rs");

fn apply_context_update(
    session: &mut SessionMetadata,
    context: Value,
    merge: bool,
    source: &str,
) -> Result<Value, String> {
    let normalized = Value::Object(normalize_context_map(context));
    let initial_size = json_size_bytes(&normalized);
    let max_bytes = session
        .budget_telemetry
        .as_ref()
        .map(|telemetry| ((telemetry.remaining_tokens().max(16) as usize) * 24).clamp(256, 4096))
        .unwrap_or(2048);
    let mut degraded_mode: Option<&'static str> = None;
    let effective = if initial_size > max_bytes {
        degraded_mode = Some("context_compacted");
        compact_context_value(&normalized)
    } else {
        normalized
    };
    let effective_size = json_size_bytes(&effective);
    let requested_tokens = u32::try_from(((effective_size as f64) / 32.0).ceil() as u64)
        .unwrap_or(u32::MAX)
        .max(1);

    if let Some(telemetry) = session.budget_telemetry.as_mut() {
        match telemetry.record_tool_usage("context_propagation", requested_tokens) {
            BudgetUsageOutcome::Ok => {}
            BudgetUsageOutcome::Warning(event) => session.check_ins.push(event),
            BudgetUsageOutcome::ExhaustedAllowed { event, action } => {
                session.check_ins.push(event);
                session.budget_action_taken = Some(action);
            }
            BudgetUsageOutcome::ExceededDenied(reason) => return Err(reason),
        }
    }

    let effective_map = normalize_context_map(effective.clone());
    if !merge {
        session.context_vars.clear();
    }
    for (key, value) in effective_map {
        session.context_vars.insert(clean_text(&key, 64), value);
    }
    session.context_mode = Some(degraded_mode.unwrap_or("full").to_string());

    let receipt = json!({
        "source": source,
        "merge": merge,
        "degraded_mode": degraded_mode,
        "applied_keys": session.context_vars.keys().cloned().collect::<Vec<_>>(),
        "context": session_context_json(session),
        "requested_tokens": requested_tokens,
    });
    session.check_ins.push(json!({
        "type": "swarm_context_update",
        "source": source,
        "merge": merge,
        "degraded_mode": degraded_mode,
        "timestamp": now_iso(),
    }));
    Ok(receipt)
}

fn session_lineage(state: &SwarmState, session_id: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = Some(session_id.to_string());
    let mut seen = BTreeMap::<String, bool>::new();
    while let Some(current) = cursor {
        if seen.insert(current.clone(), true).is_some() {
            break;
        }
        out.push(current.clone());
        cursor = state
            .sessions
            .get(&current)
            .and_then(|session| session.parent_id.clone());
    }
    out.reverse();
    out
}

fn register_handoff(
    state: &mut SwarmState,
    sender_session_id: &str,
    recipient_session_id: &str,
    reason: &str,
    importance: f64,
    context_override: Option<Value>,
    network_id: Option<String>,
) -> Result<Value, String> {
    if !session_exists(state, sender_session_id) {
        return Err(format!("unknown_sender_session:{sender_session_id}"));
    }
    if !session_exists(state, recipient_session_id) {
        return Err(format!("unknown_recipient_session:{recipient_session_id}"));
    }
    if !(0.0..=1.0).contains(&importance) {
        return Err(format!("invalid_importance:{importance}"));
    }

    let sender_context = state
        .sessions
        .get(sender_session_id)
        .map(session_context_json)
        .unwrap_or(Value::Object(Map::new()));
    let sender_lineage = session_lineage(state, sender_session_id);
    let recipient_lineage = session_lineage(state, recipient_session_id);
    let requested_context = context_override.unwrap_or(sender_context);
    let (effective_context, context_isolation_receipt) =
        isolate_handoff_context(requested_context, reason, importance);
    let context_receipt = {
        let recipient = state
            .sessions
            .get_mut(recipient_session_id)
            .ok_or_else(|| format!("unknown_recipient_session:{recipient_session_id}"))?;
        apply_context_update(recipient, effective_context, true, "handoff")?
    };
    let handoff_message = json!({
        "kind": "handoff",
        "from": sender_session_id,
        "to": recipient_session_id,
        "reason": clean_text(reason, 240),
        "importance": importance,
        "network_id": network_id,
    });
    let message_result = send_session_message(
        state,
        sender_session_id,
        recipient_session_id,
        &handoff_message.to_string(),
        DeliveryGuarantee::AtLeastOnce,
        false,
        DEFAULT_MESSAGE_TTL_MS,
    )?;
    let handoff_id = format!(
        "handoff-{}",
        &deterministic_receipt_hash(&json!({
            "sender": sender_session_id,
            "recipient": recipient_session_id,
            "reason": reason,
            "importance": importance,
            "ts": now_epoch_ms(),
        }))[..12]
    );
    let receipt = json!({
        "handoff_id": handoff_id,
        "sender_session_id": sender_session_id,
        "recipient_session_id": recipient_session_id,
        "reason": clean_text(reason, 240),
        "importance": importance,
        "lineage": {
            "sender": sender_lineage,
            "recipient": recipient_lineage,
        },
        "context_isolation_receipt": context_isolation_receipt,
        "context_receipt": context_receipt,
        "message": message_result,
        "network_id": network_id,
        "created_at": now_iso(),
    });
    state
        .handoff_registry
        .insert(handoff_id.clone(), receipt.clone());
    if let Some(sender) = state.sessions.get_mut(sender_session_id) {
        if !sender.handoff_ids.iter().any(|row| row == &handoff_id) {
            sender.handoff_ids.push(handoff_id.clone());
        }
    }
    if let Some(recipient) = state.sessions.get_mut(recipient_session_id) {
        if !recipient.handoff_ids.iter().any(|row| row == &handoff_id) {
            recipient.handoff_ids.push(handoff_id.clone());
        }
    }
    append_event(
        state,
        json!({
            "type": "swarm_handoff",
            "handoff_id": handoff_id,
            "sender_session_id": sender_session_id,
            "recipient_session_id": recipient_session_id,
            "reason": clean_text(reason, 240),
            "importance": importance,
            "context_isolation_hash": receipt
                .get("context_isolation_receipt")
                .and_then(|row| row.get("context_hash"))
                .cloned()
                .unwrap_or(Value::Null),
            "timestamp": now_iso(),
        }),
    );
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_session_handoff",
        "handoff": receipt,
    }))
}

fn safe_tool_bridge_path(raw: &str) -> Result<String, String> {
    let normalized = clean_text(raw, 260).replace('\\', "/");
    if normalized.is_empty() {
        return Err("tool_bridge_path_required".to_string());
    }
    if normalized.starts_with('/') || normalized.contains("..") {
        return Err(format!("unsafe_tool_bridge:{normalized}"));
    }
    if normalized == "client/runtime/systems/autonomy/swarm_sessions_bridge.ts"
        || normalized.starts_with("adapters/")
    {
        Ok(normalized)
    } else {
        Err(format!("unsupported_tool_bridge:{normalized}"))
    }
}

fn tool_manifest_storage_key(session_id: &str, tool_name: &str) -> String {
    format!("{session_id}:{}", safe_registry_slug(tool_name, 80))
}

fn register_json_schema_tool(
    state: &mut SwarmState,
    session_id: &str,
    tool_name: &str,
    schema: Value,
    bridge_path: &str,
    entrypoint: &str,
    description: Option<String>,
) -> Result<Value, String> {
    if !session_exists(state, session_id) {
        return Err(format!("unknown_session:{session_id}"));
    }
    if !matches!(schema, Value::Object(_)) {
        return Err("tool_schema_object_required".to_string());
    }
    let safe_path = safe_tool_bridge_path(bridge_path)?;
    let tool_name = clean_text(tool_name, 120);
    if tool_name.is_empty() {
        return Err("tool_name_required".to_string());
    }
    let entrypoint = clean_text(entrypoint, 120);
    if entrypoint.is_empty() {
        return Err("tool_entrypoint_required".to_string());
    }
    let manifest_id = format!(
        "tool-{}",
        &deterministic_receipt_hash(&json!({
            "session_id": session_id,
            "tool_name": tool_name,
            "entrypoint": entrypoint,
            "bridge_path": safe_path,
        }))[..12]
    );
    let manifest = json!({
        "manifest_id": manifest_id,
        "session_id": session_id,
        "tool_name": tool_name,
        "entrypoint": entrypoint,
        "bridge_path": safe_path,
        "schema": schema,
        "description": description,
        "registered_at": now_iso(),
        "invocation_count": 0u64,
        "policy": {
            "fail_closed": true,
            "unsafe_bridge_denied": true,
        }
    });
    state.tool_registry.insert(
        tool_manifest_storage_key(session_id, &tool_name),
        manifest.clone(),
    );
    if let Some(session) = state.sessions.get_mut(session_id) {
        if !session
            .registered_tool_ids
            .iter()
            .any(|row| row == &manifest_id)
        {
            session.registered_tool_ids.push(manifest_id.clone());
        }
    }
    append_event(
        state,
        json!({
            "type": "swarm_tool_registered",
            "manifest_id": manifest_id,
            "session_id": session_id,
            "tool_name": tool_name,
            "entrypoint": entrypoint,
            "bridge_path": bridge_path,
            "timestamp": now_iso(),
        }),
    );
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_tool_register",
        "tool_manifest": manifest,
    }))
}
