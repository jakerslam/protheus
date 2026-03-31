fn sessions_terminate(
    state: &mut SwarmState,
    session_id: &str,
    graceful: bool,
    now_ms: u64,
) -> Result<Value, String> {
    let Some(session) = state.sessions.get_mut(session_id) else {
        return Err(format!("unknown_session:{session_id}"));
    };
    if session.persistent.is_none() {
        return Err(format!("session_not_persistent:{session_id}"));
    }

    let final_report = if graceful {
        Some(perform_persistent_check_in(
            session,
            "terminated_graceful",
            true,
        )?)
    } else {
        None
    };
    session.status = if graceful {
        "terminated_graceful".to_string()
    } else {
        "terminated".to_string()
    };
    if let Some(runtime) = session.persistent.as_mut() {
        runtime.terminated_at_ms = Some(now_ms);
        runtime.terminated_reason = Some(if graceful {
            "terminated_graceful".to_string()
        } else {
            "terminated".to_string()
        });
    }
    mark_service_instance_unhealthy(state, session_id);
    settle_budget_reservation(state, session_id);

    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_terminate",
        "session_id": session_id,
        "graceful": graceful,
        "final_report": final_report,
    }))
}

fn sessions_resume(state: &mut SwarmState, session_id: &str, now_ms: u64) -> Result<Value, String> {
    let Some(session) = state.sessions.get_mut(session_id) else {
        return Err(format!("unknown_session:{session_id}"));
    };
    if session.persistent.is_none() {
        return Err(format!("session_not_persistent:{session_id}"));
    }
    session.reachable = true;
    if let Some(runtime) = session.persistent.as_mut() {
        runtime.last_check_in_ms = Some(now_ms);
        if runtime.terminated_at_ms.is_none() {
            session.status = if session.background_worker {
                "background_running".to_string()
            } else {
                "persistent_running".to_string()
            };
            runtime.next_check_in_ms =
                now_ms.saturating_add(runtime.config.check_in_interval_sec.saturating_mul(1000));
        }
    }
    append_event(
        state,
        json!({
            "type": "swarm_session_resumed",
            "session_id": session_id,
            "timestamp": now_iso(),
        }),
    );
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_resume",
        "session_id": session_id,
        "status": state.sessions.get(session_id).map(|row| row.status.clone()).unwrap_or_default(),
    }))
}

fn sessions_metrics(
    state: &SwarmState,
    session_id: &str,
    include_timeline: bool,
) -> Result<Value, String> {
    let Some(session) = state.sessions.get(session_id) else {
        return Err(format!("unknown_session:{session_id}"));
    };
    let started_at_ms = session
        .persistent
        .as_ref()
        .map(|runtime| runtime.started_at_ms)
        .unwrap_or_else(now_epoch_ms);
    let latest = session.metrics_timeline.last().cloned();
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_metrics",
        "session_id": session_id,
        "started_at_ms": started_at_ms,
        "snapshot_count": session.metrics_timeline.len(),
        "latest": latest,
        "timeline": if include_timeline { Value::Array(session.metrics_timeline.iter().cloned().map(|row| json!(row)).collect::<Vec<_>>()) } else { Value::Null },
    }))
}

fn sessions_anomalies(state: &SwarmState, session_id: &str) -> Result<Value, String> {
    let Some(session) = state.sessions.get(session_id) else {
        return Err(format!("unknown_session:{session_id}"));
    };
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_anomalies",
        "session_id": session_id,
        "anomalies": session.anomalies,
    }))
}

fn sessions_state(
    state: &SwarmState,
    session_id: &str,
    include_timeline: bool,
    tool_history_limit: usize,
) -> Result<Value, String> {
    let Some(session) = state.sessions.get(session_id) else {
        return Err(format!("unknown_session:{session_id}"));
    };

    let registered_roles = state
        .service_registry
        .iter()
        .filter_map(|(role, instances)| {
            instances
                .iter()
                .any(|instance| instance.session_id == session_id)
                .then(|| role.clone())
        })
        .collect::<Vec<_>>();
    let advertised_capabilities = state
        .service_registry
        .values()
        .flat_map(|rows| rows.iter())
        .find(|row| row.session_id == session_id)
        .map(|row| row.capabilities.clone())
        .unwrap_or_default();

    let results = state
        .results_by_session
        .get(session_id)
        .cloned()
        .unwrap_or_default();

    let mut tool_history = session
        .budget_telemetry
        .as_ref()
        .map(|telemetry| {
            telemetry
                .usage_over_time
                .iter()
                .rev()
                .take(tool_history_limit.max(1))
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    tool_history.reverse();

    let metrics = session
        .metrics
        .as_ref()
        .map(SpawnMetrics::as_json)
        .unwrap_or(Value::Null);
    let latest_snapshot = session.metrics_timeline.last().cloned();
    let context_utilization = session
        .budget_telemetry
        .as_ref()
        .map(BudgetTelemetry::utilization)
        .unwrap_or(0.0);
    let mailbox = state.mailboxes.get(session_id);
    let tool_manifest = session_tool_manifest(state, session);
    let dead_letter_count = state
        .dead_letters
        .iter()
        .filter(|row| row.message.recipient_session_id == session_id)
        .count();
    let handoffs = session
        .handoff_ids
        .iter()
        .filter_map(|row| state.handoff_registry.get(row).cloned())
        .collect::<Vec<_>>();
    let registered_tools = session
        .registered_tool_ids
        .iter()
        .filter_map(|tool_id| {
            state
                .tool_registry
                .values()
                .find(|row| {
                    row.get("manifest_id").and_then(Value::as_str) == Some(tool_id.as_str())
                })
                .cloned()
        })
        .collect::<Vec<_>>();
    let networks = session
        .network_ids
        .iter()
        .filter_map(|row| state.network_registry.get(row).cloned())
        .collect::<Vec<_>>();

    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_session_state",
        "session_id": session_id,
        "session": {
            "status": session.status.clone(),
            "task": session.task.clone(),
            "created_at": session.created_at.clone(),
            "depth": session.depth,
            "parent_id": session.parent_id.clone(),
            "children": session.children.clone(),
            "reachable": session.reachable,
            "byzantine": session.byzantine,
            "corruption_type": session.corruption_type.clone(),
            "role": session.role.clone(),
            "registered_roles": registered_roles,
            "capabilities": advertised_capabilities,
            "tool_access": session.tool_access.clone(),
            "results_published": results.len(),
            "result_ids": results,
            "background_worker": session.background_worker,
            "persistent": session.persistent.clone(),
            "agent_label": session.agent_label.clone(),
            "report": session.report.clone(),
            "anomalies": session.anomalies.clone(),
            "tool_manifest": tool_manifest,
            "budget": session.budget_telemetry.as_ref().map(BudgetTelemetry::generate_report).unwrap_or(Value::Null),
            "budget_parent_session_id": session.budget_parent_session_id.clone(),
            "budget_reservation_tokens": session.budget_reservation_tokens,
            "budget_reservation_settled": session.budget_reservation_settled,
            "handoff_count": session.handoff_ids.len(),
            "registered_tool_count": session.registered_tool_ids.len(),
            "network_count": session.network_ids.len(),
            "turn_run_count": session.turn_run_ids.len(),
            "stream_turn_count": session.stream_turn_ids.len(),
        },
        "context": {
            "variables": session_context_json(session),
            "mode": session.context_mode.clone(),
            "utilization_ratio": context_utilization,
            "utilization_pct": context_utilization * 100.0,
            "latest_snapshot": latest_snapshot,
            "snapshot_count": session.metrics_timeline.len(),
        },
        "metrics": metrics,
        "tool_call_history": tool_history,
        "check_ins": {
            "count": session.check_ins.len(),
            "latest": session.check_ins.last().cloned(),
            "timeline": if include_timeline { Value::Array(session.check_ins.clone()) } else { Value::Null },
        },
        "mailbox": {
            "unread": mailbox.map(|row| row.unread.len()).unwrap_or(0),
            "read": mailbox.map(|row| row.read.len()).unwrap_or(0),
            "dead_lettered": dead_letter_count,
        },
        "handoffs": handoffs,
        "registered_tools": registered_tools,
        "networks": networks,
    }))
}

fn sessions_dead_letters(
    state: &SwarmState,
    session_id: Option<&str>,
    retryable_only: bool,
) -> Value {
    let entries = state
        .dead_letters
        .iter()
        .filter(|row| {
            session_id
                .map(|id| {
                    row.message.recipient_session_id == id || row.message.sender_session_id == id
                })
                .unwrap_or(true)
        })
        .filter(|row| !retryable_only || row.retryable)
        .cloned()
        .collect::<Vec<_>>();
    json!({
        "ok": true,
        "type": "swarm_runtime_dead_letters",
        "dead_letter_count": entries.len(),
        "dead_letters": entries,
    })
}

fn sessions_retry_dead_letter(state: &mut SwarmState, message_id: &str) -> Result<Value, String> {
    let Some(index) = state
        .dead_letters
        .iter()
        .position(|row| row.message.message_id == message_id)
    else {
        return Err(format!("unknown_dead_letter_message:{message_id}"));
    };
    let entry = state.dead_letters.remove(index);
    if !entry.retryable {
        state.dead_letters.push(entry);
        return Err(format!("dead_letter_not_retryable:{message_id}"));
    }
    let sent = send_session_message(
        state,
        &entry.message.sender_session_id,
        &entry.message.recipient_session_id,
        &entry.message.payload,
        entry.message.delivery.clone(),
        false,
        entry.message.ttl_ms.max(DEFAULT_MESSAGE_TTL_MS),
    )?;
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_dead_letter_retry",
        "message_id": message_id,
        "retry_result": sent,
    }))
}
fn queue_metrics_snapshot(state: &SwarmState) -> Value {
    let mut queue_wait_values = Vec::new();
    let mut execution_values = Vec::new();
    let mut total_latency_values = Vec::new();

    for session in state.sessions.values() {
        if let Some(metrics) = session.metrics.as_ref() {
            queue_wait_values.push(metrics.queue_wait_ms);
            execution_values.push(metrics.execution_time_ms());
            total_latency_values.push(metrics.total_latency_ms());
        }
    }

    let queue_wait_sum: u64 = queue_wait_values.iter().copied().sum();
    let execution_sum: u64 = execution_values.iter().copied().sum();
    let total_latency_sum: u64 = total_latency_values.iter().copied().sum();
    let sample_count = queue_wait_values.len() as u64;
    let denom = sample_count.max(1) as f64;

    let mut queue_wait_sorted = queue_wait_values.clone();
    let mut execution_sorted = execution_values.clone();
    let mut total_latency_sorted = total_latency_values.clone();
    queue_wait_sorted.sort_unstable();
    execution_sorted.sort_unstable();
    total_latency_sorted.sort_unstable();

    let p95_idx = |len: usize| -> usize {
        if len == 0 {
            return 0;
        }
        (((len as f64) * 0.95).ceil() as usize).saturating_sub(1)
    };

    let queue_wait_p95 = queue_wait_sorted
        .get(p95_idx(queue_wait_sorted.len()))
        .copied()
        .unwrap_or(0);
    let execution_p95 = execution_sorted
        .get(p95_idx(execution_sorted.len()))
        .copied()
        .unwrap_or(0);
    let total_latency_p95 = total_latency_sorted
        .get(p95_idx(total_latency_sorted.len()))
        .copied()
        .unwrap_or(0);

    let unread_messages: usize = state
        .mailboxes
        .values()
        .map(|mailbox| mailbox.unread.len())
        .sum();
    let dead_letter_count = state.dead_letters.len();
    let backpressure_count = state
        .dead_letters
        .iter()
        .filter(|row| row.reason == "mailbox_backpressure")
        .count();
    let persistent_sessions = state
        .sessions
        .values()
        .filter(|session| session.persistent.is_some())
        .count();
    let active_sessions = state
        .sessions
        .values()
        .filter(|session| {
            matches!(
                session.status.as_str(),
                "running" | "persistent_running" | "background_running"
            )
        })
        .count();

    json!({
        "sample_count": sample_count,
        "queue_wait_ms": {
            "sum": queue_wait_sum,
            "avg": queue_wait_sum as f64 / denom,
            "p95": queue_wait_p95,
        },
        "execution_ms": {
            "sum": execution_sum,
            "avg": execution_sum as f64 / denom,
            "p95": execution_p95,
        },
        "total_latency_ms": {
            "sum": total_latency_sum,
            "avg": total_latency_sum as f64 / denom,
            "p95": total_latency_p95,
        },
        "session_counts": {
            "total": state.sessions.len(),
            "active": active_sessions,
            "persistent": persistent_sessions,
        },
        "mailbox": {
            "unread_total": unread_messages,
            "mailbox_count": state.mailboxes.len(),
            "dead_letter_total": dead_letter_count,
            "backpressure_total": backpressure_count,
        }
    })
}
