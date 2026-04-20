
fn clear_isolation_for_task(state: &mut Value, task: &str) {
    let clean_task = clean(task, 80);
    let mut quarantine = state["failure_isolation"]["quarantine"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    quarantine.retain(|entry| entry.get("task").and_then(Value::as_str) != Some(clean_task.as_str()));
    state["failure_isolation"]["quarantine"] = Value::Array(quarantine);
}

fn choose_recovery_strategy(failure_class: &str, prior_attempts: u64) -> &'static str {
    match failure_class {
        "resource_pressure" => "resync",
        "runtime_fault" => {
            if prior_attempts == 0 {
                "retry"
            } else {
                "rollback"
            }
        }
        "transport_fault" => {
            if prior_attempts == 0 {
                "retry"
            } else {
                "escalate"
            }
        }
        "isolation_active" => "resync",
        _ => "escalate",
    }
}

fn record_recovery_strategy(
    state: &mut Value,
    intent: &Value,
    reason: &str,
    now_ms: u64,
) -> Value {
    if !state["recovery_matrix"]["attempts_by_task"].is_object() {
        state["recovery_matrix"]["attempts_by_task"] = json!({});
    }
    let task = clean(
        intent
            .get("task")
            .and_then(Value::as_str)
            .map(|v| v.to_string())
            .unwrap_or_default()
            .as_str(),
        80,
    );
    let failure_class = classify_proactive_failure(reason);
    let prior_attempts = state["recovery_matrix"]["attempts_by_task"]
        .get(task.as_str())
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let strategy = choose_recovery_strategy(failure_class, prior_attempts);
    state["recovery_matrix"]["attempts_by_task"][task.as_str()] =
        json!(prior_attempts.saturating_add(1));
    let strategy_outcome = match strategy {
        "retry" => "scheduled_retry",
        "rollback" => "rollback_to_last_receipt",
        "resync" => "state_resync",
        _ => "operator_escalation",
    };
    let row = json!({
        "event_id": receipt_hash(&json!({"task":task,"reason":reason,"attempt":prior_attempts,"now_ms":now_ms})),
        "task": task,
        "reason": clean(reason, 64),
        "failure_class": failure_class,
        "attempt": prior_attempts.saturating_add(1),
        "strategy": strategy,
        "outcome": strategy_outcome,
        "rollback_target": state["tool_surfaces"]["last_receipt_id"].clone(),
        "ts": now_iso(),
    });
    let mut history = state["recovery_matrix"]["history"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    history.push(row.clone());
    if history.len() > 128 {
        let trim = history.len().saturating_sub(128);
        history.drain(0..trim);
    }
    state["recovery_matrix"]["history"] = Value::Array(history);
    state["recovery_matrix"]["last_strategy"] = json!(strategy);
    state["recovery_matrix"]["last_outcome"] = json!(strategy_outcome);
    row
}

fn mark_recovery_success(state: &mut Value, task: &str) {
    clear_isolation_for_task(state, task);
    if let Some(map) = state["recovery_matrix"]["attempts_by_task"].as_object_mut() {
        map.remove(task);
    }
}

fn persist_proactive_daemon_state(
    root: &Path,
    state: &mut Value,
    strict: bool,
) -> Result<(), String> {
    let path = proactive_daemon_state_path(root);
    state["write_discipline"]["state_write_confirmed"] = json!(false);
    state["write_discipline"]["last_state_write_at"] = json!(now_iso());
    state["write_discipline"]["state_path"] = json!(path.display().to_string());
    write_json(&path, state)?;
    let persisted = read_json(&path).unwrap_or(Value::Null);
    let confirmed = persisted.get("updated_at") == state.get("updated_at");
    state["write_discipline"]["state_write_confirmed"] = json!(confirmed);
    if strict && !confirmed {
        return Err("proactive_daemon_state_write_confirm_failed".to_string());
    }
    write_json(&path, state)?;
    Ok(())
}
