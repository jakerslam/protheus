
fn context_stacks_node_spike(root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let mut state = load_context_stacks_state(root);
    let stack_id = stack_id_from(parsed);
    if stack_id.is_empty() {
        return json!({"ok": false, "status": "blocked", "error": "stack_id_required"});
    }
    if find_manifest_index(&state, &stack_id).is_none() {
        return json!({"ok": false, "status": "blocked", "error": "stack_not_found", "stack_id": stack_id});
    }
    if !state.node_spike_metrics.is_object() {
        state.node_spike_metrics = json!({
            "queue_limit": 128u64,
            "dropped_non_critical": 0u64,
            "critical_retained": 0u64,
            "critical_journaled": 0u64,
            "critical_dropped": 0u64,
            "last_overload_at": Value::Null
        });
    }
    let node_id = clean(
        parsed
            .flags
            .get("node-id")
            .or_else(|| parsed.flags.get("node"))
            .map(String::as_str)
            .unwrap_or("root"),
        120,
    );
    let delta = parsed
        .flags
        .get("delta")
        .and_then(|row| clean(row, 32).parse::<f64>().ok())
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let staleness_seconds = parse_u64_flag(parsed, "staleness-seconds", 0).min(172_800);
    let staleness_norm = (staleness_seconds as f64 / 3600.0).clamp(0.0, 1.0);
    let external_trigger = parsed
        .flags
        .get("external-trigger")
        .map(|row| bool_like(row))
        .unwrap_or(false);
    let queue_limit_default = state
        .node_spike_metrics
        .get("queue_limit")
        .and_then(Value::as_u64)
        .unwrap_or(128);
    let queue_limit = parse_u64_flag(parsed, "queue-limit", queue_limit_default)
        .clamp(8, 4096) as usize;
    let queue_depth_before = state.node_spike_queue.len();
    let inferred_load = if queue_limit == 0 {
        0.0
    } else {
        queue_depth_before as f64 / queue_limit as f64
    };
    let load_signal = parsed
        .flags
        .get("load-signal")
        .and_then(|row| clean(row, 32).parse::<f64>().ok())
        .unwrap_or(inferred_load)
        .clamp(0.0, 1.0);
    let success_signal = parsed
        .flags
        .get("success-signal")
        .and_then(|row| clean(row, 32).parse::<f64>().ok())
        .unwrap_or(0.5)
        .clamp(0.0, 1.0);
    let utility = (delta * 0.6 + staleness_norm * 0.3 + if external_trigger { 0.35 } else { 0.0 })
        .clamp(0.0, 1.0);
    let threshold_before = state
        .node_spike_thresholds
        .get(&node_id)
        .copied()
        .unwrap_or(0.35);
    let mut threshold_after = threshold_before + (load_signal - 0.5) * 0.2 - (success_signal - 0.5) * 0.15;
    threshold_after = threshold_after.clamp(0.05, 0.95);
    let should_fire = utility >= threshold_after;
    let critical = external_trigger || utility >= 0.9;
    let mut backpressure_action = "none".to_string();

    let mut dropped_non_critical = state
        .node_spike_metrics
        .get("dropped_non_critical")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let mut critical_retained = state
        .node_spike_metrics
        .get("critical_retained")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let mut critical_journaled = state
        .node_spike_metrics
        .get("critical_journaled")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let critical_dropped = state
        .node_spike_metrics
        .get("critical_dropped")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let mut event = Value::Null;
    let mut enqueued = false;
    if should_fire {
        let spike_event = json!({
            "event_id": receipt_hash(&json!({
                "stack_id": stack_id,
                "node_id": node_id,
                "delta": delta,
                "staleness_seconds": staleness_seconds,
                "external_trigger": external_trigger,
                "utility": utility,
                "threshold_after": threshold_after,
                "ts": now_iso()
            })),
            "stack_id": stack_id,
            "node_id": node_id,
            "critical": critical,
            "delta": delta,
            "staleness_seconds": staleness_seconds,
            "external_trigger": external_trigger,
            "utility": utility,
            "threshold_before": threshold_before,
            "threshold_after": threshold_after,
            "ts": now_iso()
        });
        event = spike_event.clone();
        state.node_spike_events.push(spike_event.clone());
        if state.node_spike_events.len() > 256 {
            let trim = state.node_spike_events.len().saturating_sub(256);
            state.node_spike_events.drain(0..trim);
        }
        state.node_spike_queue.push(spike_event);
        enqueued = true;
        if state.node_spike_queue.len() > queue_limit {
            if let Some(non_critical_idx) = state
                .node_spike_queue
                .iter()
                .position(|row| !row.get("critical").and_then(Value::as_bool).unwrap_or(false))
            {
                state.node_spike_queue.remove(non_critical_idx);
                dropped_non_critical = dropped_non_critical.saturating_add(1);
                backpressure_action = "drop_non_critical".to_string();
            } else {
                let _ = state.node_spike_queue.pop();
                enqueued = false;
                critical_journaled = critical_journaled.saturating_add(1);
                backpressure_action = "critical_journaled".to_string();
            }
            state.node_spike_metrics["last_overload_at"] = json!(now_iso());
        }
        if critical {
            critical_retained = critical_retained.saturating_add(1);
        }
    }
    let queue_depth_after = state.node_spike_queue.len();
    let queue_pressure_after = if queue_limit == 0 {
        0.0
    } else {
        queue_depth_after as f64 / queue_limit as f64
    };
    threshold_after = (threshold_after + (queue_pressure_after - 0.5) * 0.1).clamp(0.05, 0.95);
    state
        .node_spike_thresholds
        .insert(node_id.clone(), threshold_after);
    state.node_spike_metrics["queue_limit"] = json!(queue_limit as u64);
    state.node_spike_metrics["queue_depth"] = json!(queue_depth_after as u64);
    state.node_spike_metrics["queue_pressure"] = json!(queue_pressure_after);
    state.node_spike_metrics["dropped_non_critical"] = json!(dropped_non_critical);
    state.node_spike_metrics["critical_retained"] = json!(critical_retained);
    state.node_spike_metrics["critical_journaled"] = json!(critical_journaled);
    state.node_spike_metrics["critical_dropped"] = json!(critical_dropped);
    state.node_spike_metrics["last_backpressure_action"] = json!(backpressure_action.clone());
    state.node_spike_metrics["last_threshold_after"] = json!(threshold_after);
    state.node_spike_metrics["last_utility"] = json!(utility);

    let _ = persist_context_stacks_state(root, &state);

    let receipt = json!({
        "type": "context_stack_node_spike",
        "stack_id": stack_id,
        "node_id": node_id,
        "should_fire": should_fire,
        "critical": critical,
        "utility": utility,
        "threshold_before": threshold_before,
        "threshold_after": threshold_after,
        "enqueued": enqueued,
        "queue_depth_before": queue_depth_before,
        "queue_depth_after": queue_depth_after,
        "queue_limit": queue_limit,
        "backpressure_action": backpressure_action,
        "event": event,
        "ts": now_iso()
    });
    let mut receipt_with_id = receipt.clone();
    receipt_with_id["receipt_id"] = json!(receipt_hash(&receipt));
    let _ = append_context_stacks_receipt(root, &receipt_with_id);
    let _ = append_context_stacks_digestion_log(
        root,
        &stack_id,
        &[format!(
            "node_spike node={} fire={} utility={:.4} threshold={:.4} action={} queue={}/{}",
            node_id, should_fire, utility, threshold_after, backpressure_action, queue_depth_after, queue_limit
        )],
    );
    json!({
        "ok": true,
        "type": "context_stacks_node_spike",
        "stack_id": stack_id,
        "node_id": node_id,
        "should_fire": should_fire,
        "critical": critical,
        "utility": utility,
        "threshold_before": threshold_before,
        "threshold_after": threshold_after,
        "enqueued": enqueued,
        "queue": {
            "depth_before": queue_depth_before,
            "depth_after": queue_depth_after,
            "limit": queue_limit
        },
        "metrics": state.node_spike_metrics,
        "backpressure_action": backpressure_action,
        "event": event,
        "receipt_id": receipt_with_id.get("receipt_id").cloned().unwrap_or(Value::Null)
    })
}
