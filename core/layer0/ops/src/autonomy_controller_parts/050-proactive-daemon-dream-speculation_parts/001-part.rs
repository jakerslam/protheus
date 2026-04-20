fn proactive_daemon_default_state() -> Value {
    json!({
        "version": "v2",
        "paused": false,
        "cycles": 0u64,
        "last_intents": [],
        "last_executed_intents": [],
        "last_deferred_intents": [],
        "heartbeat": {
            "tick_ms": 5000u64,
            "jitter_ms": 400u64,
            "last_tick_ms": 0u64,
            "next_tick_after_ms": 0u64
        },
        "proactive": {
            "window_sec": 900u64,
            "max_messages": 2u64,
            "sent_in_window": 0u64,
            "window_started_at_ms": 0u64,
            "brief_mode": true
        },
        "budgets": {
            "blocking_ms": 15000u64
        },
        "tool_surfaces": {
            "policy_tier": "observe",
            "enabled": ["subscribe_pr", "push_notification", "send_user_file"],
            "receipts_written": 0u64,
            "last_receipt_id": Value::Null
        },
        "pattern_log": {
            "history": [],
            "last_cycle_patterns": [],
            "suggestions": [],
            "last_logged_at": Value::Null
        },
        "failure_isolation": {
            "history": [],
            "quarantine": [],
            "last_isolation": Value::Null
        },
        "recovery_matrix": {
            "history": [],
            "attempts_by_task": {},
            "last_strategy": Value::Null,
            "last_outcome": Value::Null
        },
        "dream": {
            "max_idle_ms": 6u64 * 60u64 * 60u64 * 1000u64,
            "max_without_dream_ms": 24u64 * 60u64 * 60u64 * 1000u64,
            "last_dream_at_ms": 0u64,
            "last_dream_reason": Value::Null,
            "last_dream_hand_id": Value::Null,
            "last_cleanup_ok": Value::Null
        },
        "write_discipline": {
            "state_write_confirmed": false,
            "last_state_write_at": Value::Null
        }
    })
}

fn ensure_proactive_daemon_state_shape(state: &mut Value) {
    if !state.is_object() {
        *state = proactive_daemon_default_state();
    }
    if !state
        .get("heartbeat")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["heartbeat"] = proactive_daemon_default_state()["heartbeat"].clone();
    }
    if !state
        .get("proactive")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["proactive"] = proactive_daemon_default_state()["proactive"].clone();
    }
    if !state.get("budgets").map(Value::is_object).unwrap_or(false) {
        state["budgets"] = proactive_daemon_default_state()["budgets"].clone();
    }
    if !state
        .get("tool_surfaces")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["tool_surfaces"] = proactive_daemon_default_state()["tool_surfaces"].clone();
    }
    if !state
        .get("pattern_log")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["pattern_log"] = proactive_daemon_default_state()["pattern_log"].clone();
    }
    if !state
        .get("failure_isolation")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["failure_isolation"] = proactive_daemon_default_state()["failure_isolation"].clone();
    }
    if !state
        .get("recovery_matrix")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["recovery_matrix"] = proactive_daemon_default_state()["recovery_matrix"].clone();
    }
    if !state.get("dream").map(Value::is_object).unwrap_or(false) {
        state["dream"] = proactive_daemon_default_state()["dream"].clone();
    }
    if !state
        .get("write_discipline")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["write_discipline"] = proactive_daemon_default_state()["write_discipline"].clone();
    }
    for key in [
        "last_intents",
        "last_executed_intents",
        "last_deferred_intents",
        "pattern_log/history",
        "pattern_log/last_cycle_patterns",
        "pattern_log/suggestions",
        "failure_isolation/history",
        "failure_isolation/quarantine",
        "recovery_matrix/history",
    ] {
        let exists = if key.contains('/') {
            state.pointer(&format!("/{key}"))
                .map(Value::is_array)
                .unwrap_or(false)
        } else {
            state.get(key).map(Value::is_array).unwrap_or(false)
        };
        if !exists {
            if key.contains('/') {
                let mut pieces = key.split('/');
                let head = pieces.next().unwrap_or_default();
                let tail = pieces.next().unwrap_or_default();
                if !head.is_empty() && !tail.is_empty() {
                    state[head][tail] = Value::Array(Vec::new());
                }
            } else {
                state[key] = Value::Array(Vec::new());
            }
        }
    }
}

fn intent_estimated_blocking_ms(intent: &Value) -> u64 {
    match intent
        .get("task")
        .and_then(Value::as_str)
        .unwrap_or_default()
    {
        "sweep_dead_letters" => 5_000,
        "autoscale_review" => 4_000,
        "dream_consolidation" => 2_500,
        "compact_hand_memory" => 800,
        "pattern_log" => 200,
        "subscribe_pr" => 220,
        "push_notification" => 180,
        "send_user_file" => 260,
        _ => 1_000,
    }
}

fn deterministic_jitter_ms(cycle: u64, jitter_ms: u64) -> u64 {
    if jitter_ms == 0 {
        return 0;
    }
    let seed = receipt_hash(&json!({"cycle": cycle, "jitter_ms": jitter_ms}));
    let n = u64::from_str_radix(seed.get(0..8).unwrap_or("0"), 16).unwrap_or(0);
    n % (jitter_ms.saturating_mul(2).saturating_add(1))
}

fn rollover_proactive_window(state: &mut Value, now_ms: u64) {
    let window_sec = state
        .pointer("/proactive/window_sec")
        .and_then(Value::as_u64)
        .unwrap_or(900);
    let window_started = state
        .pointer("/proactive/window_started_at_ms")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let window_ms = window_sec.saturating_mul(1000);
    if window_started == 0 || now_ms.saturating_sub(window_started) >= window_ms {
        state["proactive"]["window_started_at_ms"] = json!(now_ms);
        state["proactive"]["sent_in_window"] = json!(0u64);
    }
}

fn append_proactive_daemon_log(root: &Path, row: &Value, strict: bool) -> Result<(), String> {
    let path = proactive_daemon_daily_log_path(root, &proactive_daemon_today_ymd());
    append_jsonl(&path, row)?;
    if strict {
        let rows = read_jsonl(&path);
        if rows.is_empty() {
            return Err("proactive_daemon_log_append_verification_failed".to_string());
        }
    }
    Ok(())
}

fn proactive_daemon_tool_receipts_path(root: &Path) -> PathBuf {
    state_root(root)
        .join("proactive_daemon")
        .join("tool_receipts.jsonl")
}

fn parse_tool_surfaces(raw: Option<String>) -> Vec<String> {
    let parsed = raw
        .map(|value| {
            value
                .split(',')
                .map(|token| clean_id(Some(token.to_string()), ""))
                .filter(|token| !token.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut out = Vec::new();
    for item in parsed {
        if !out.contains(&item) {
            out.push(item);
        }
    }
    if out.is_empty() {
        vec![
            "subscribe_pr".to_string(),
            "push_notification".to_string(),
            "send_user_file".to_string(),
        ]
    } else {
        out
    }
}

fn tool_surface_allowed(policy_tier: &str, tool: &str) -> bool {
    match clean(policy_tier, 32).to_ascii_lowercase().as_str() {
        "execute" => matches!(tool, "subscribe_pr" | "push_notification" | "send_user_file"),
        "assist" => matches!(tool, "subscribe_pr" | "push_notification"),
        _ => false,
    }
}

fn append_tool_surface_receipt(
    root: &Path,
    task: &str,
    policy_tier: &str,
    intent: &Value,
    strict: bool,
) -> Result<Value, String> {
    let row = json!({
        "type": "proactive_tool_surface_receipt",
        "task": clean(task, 80),
        "policy_tier": clean(policy_tier, 32),
        "transport": "conduit",
        "intent": intent,
        "ts": now_iso(),
    });
    let mut out = row;
    out["receipt_id"] = json!(receipt_hash(&out));
    let path = proactive_daemon_tool_receipts_path(root);
    append_jsonl(&path, &out)?;
    if strict {
        let rows = read_jsonl(&path);
        if rows.is_empty() {
            return Err("proactive_tool_receipt_append_failed".to_string());
        }
    }
    Ok(out)
}

fn update_pattern_log_state(
    state: &mut Value,
    now_iso_ts: &str,
    patterns: &[String],
    evidence: Value,
) -> Value {
    let mut suggestions = Vec::new();
    for pattern in patterns {
        match pattern.as_str() {
            "dead_letter_recurrence" => suggestions.push(json!({
                "hint": "review dead-letter retry pressure and mailbox backpressure thresholds",
                "priority": "high",
                "scope": "swarm_runtime"
            })),
            "session_pressure_recurrence" => suggestions.push(json!({
                "hint": "evaluate autoscale policy and parent fanout ceilings",
                "priority": "medium",
                "scope": "swarm_scale"
            })),
            "deferred_budget_recurrence" => suggestions.push(json!({
                "hint": "raise proactive block budget or lower proactive tool fanout",
                "priority": "medium",
                "scope": "autonomy_proactive"
            })),
            _ => {}
        }
    }
    let summary = json!({
        "ts": now_iso_ts,
        "patterns": patterns,
        "suggestions": suggestions,
        "evidence": evidence
    });
    let history = state["pattern_log"]["history"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut next = history;
    next.push(summary.clone());
    if next.len() > 64 {
        let trim = next.len().saturating_sub(64);
        next.drain(0..trim);
    }
    state["pattern_log"]["history"] = Value::Array(next);
    state["pattern_log"]["last_cycle_patterns"] =
        Value::Array(patterns.iter().map(|v| Value::String(v.clone())).collect());
    state["pattern_log"]["suggestions"] = summary["suggestions"].clone();
    state["pattern_log"]["last_logged_at"] = Value::String(now_iso_ts.to_string());
    summary
}

fn classify_proactive_failure(reason: &str) -> &'static str {
    match reason {
        "rate_limit" | "blocking_budget" => "resource_pressure",
        "compact_failed" | "dream_failed" => "runtime_fault",
        "tool_surface_failed" => "transport_fault",
        "isolation_quarantine" => "isolation_active",
        _ => "unknown_fault",
    }
}

fn purge_expired_isolation_quarantine(state: &mut Value, now_ms: u64) {
    let mut active = Vec::new();
    for row in state["failure_isolation"]["quarantine"]
        .as_array()
        .cloned()
        .unwrap_or_default()
    {
        let until_ms = row
            .get("quarantine_until_ms")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if until_ms > now_ms {
            active.push(row);
        }
    }
    state["failure_isolation"]["quarantine"] = Value::Array(active);
}

fn task_isolation_active(state: &Value, task: &str, now_ms: u64) -> bool {
    let clean_task = clean(task, 80);
    state["failure_isolation"]["quarantine"]
        .as_array()
        .map(|rows| {
            rows.iter().any(|row| {
                row.get("task").and_then(Value::as_str) == Some(clean_task.as_str())
                    && row
                        .get("quarantine_until_ms")
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                        > now_ms
            })
        })
        .unwrap_or(false)
}

fn record_isolation_event(state: &mut Value, intent: &Value, reason: &str, now_ms: u64) -> Value {
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
    let quarantine_ms = match failure_class {
        "resource_pressure" => 60_000,
        "runtime_fault" => 300_000,
        "transport_fault" => 180_000,
        _ => 120_000,
    };
    let row = json!({
        "event_id": receipt_hash(&json!({"task":task,"reason":reason,"now_ms":now_ms})),
        "task": task,
        "reason": clean(reason, 64),
        "failure_class": failure_class,
        "quarantine_started_ms": now_ms,
        "quarantine_until_ms": now_ms.saturating_add(quarantine_ms),
        "intent": intent,
        "ts": now_iso(),
    });
    let mut history = state["failure_isolation"]["history"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    history.push(row.clone());
    if history.len() > 128 {
        let trim = history.len().saturating_sub(128);
        history.drain(0..trim);
    }
    state["failure_isolation"]["history"] = Value::Array(history);

    let mut quarantine = state["failure_isolation"]["quarantine"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    quarantine.retain(|entry| entry.get("task").and_then(Value::as_str) != Some(task.as_str()));
    quarantine.push(row.clone());
    if quarantine.len() > 64 {
        let trim = quarantine.len().saturating_sub(64);
        quarantine.drain(0..trim);
    }
    state["failure_isolation"]["quarantine"] = Value::Array(quarantine);
    state["failure_isolation"]["last_isolation"] = row.clone();
    row
}

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
