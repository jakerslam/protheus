
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
