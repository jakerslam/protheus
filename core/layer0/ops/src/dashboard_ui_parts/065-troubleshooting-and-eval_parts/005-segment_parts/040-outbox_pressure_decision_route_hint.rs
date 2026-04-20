fn dashboard_outbox_pressure_decision_route_hint(next_action_kind: &str) -> &'static str {
    match next_action_kind {
        "manual_gate" => "manual_review_lane",
        "deferred_retry" => "deferred_retry_lane",
        _ => "auto_execute_lane",
    }
}

fn dashboard_outbox_pressure_decision_urgency_tier(
    retry_window_class: &str,
    automation_safe: bool,
) -> &'static str {
    if !automation_safe {
        return "manual";
    }
    match retry_window_class {
        "immediate" => "high",
        "short" => "medium",
        "medium" => "low",
        _ => "deferred",
    }
}

fn dashboard_outbox_pressure_decision_retry_budget_class(
    retry_window_class: &str,
    automation_safe: bool,
) -> &'static str {
    if !automation_safe {
        return "manual_only";
    }
    match retry_window_class {
        "immediate" => "single_attempt",
        "short" => "bounded_backoff_short",
        "medium" => "bounded_backoff_medium",
        _ => "bounded_backoff_long",
    }
}

fn dashboard_outbox_pressure_decision_lane_token(route_hint: &str, urgency_tier: &str) -> String {
    format!(
        "{}::{}",
        clean_text(route_hint, 80),
        clean_text(urgency_tier, 40)
    )
}

fn dashboard_outbox_pressure_decision_dispatch_mode(next_action_kind: &str) -> &'static str {
    match next_action_kind {
        "manual_gate" => "manual_review",
        "deferred_retry" => "scheduled_retry",
        _ => "immediate_execute",
    }
}

fn dashboard_outbox_pressure_decision_manual_ack_required(
    next_action_kind: &str,
    automation_safe: bool,
) -> bool {
    !automation_safe || next_action_kind == "manual_gate"
}

fn dashboard_outbox_pressure_decision_execution_guard(
    next_action_kind: &str,
    automation_safe: bool,
) -> &'static str {
    if !automation_safe || next_action_kind == "manual_gate" {
        "manual_gate_guard"
    } else if next_action_kind == "deferred_retry" {
        "retry_window_guard"
    } else {
        "none"
    }
}

fn dashboard_outbox_pressure_decision_followup_required(next_action_kind: &str) -> bool {
    next_action_kind != "execute_now"
}

fn dashboard_outbox_pressure_contract(
    snapshot_epoch_s: i64,
    priority: &str,
    action_hint: &str,
    escalation_lane: &str,
    runbook_id: &str,
    escalation_owner: &str,
    blocking_kind: &str,
    auto_retry_allowed: bool,
    execution_policy: &str,
    manual_gate_required: bool,
    manual_gate_reason: &str,
    requeue_strategy: &str,
    can_execute_without_human: bool,
    execution_window: &str,
    manual_gate_timeout_seconds: i64,
    next_action_after_seconds: i64,
    next_action_kind: &str,
    retry_window_class: &str,
    readiness_state: &str,
    readiness_reason: &str,
    automation_safe: bool,
    decision_vector_key: &str,
    deadline_epoch_s: i64,
    breach_reason: &str,
) -> Value {
    json!({
        "version": dashboard_outbox_pressure_contract_version(),
        "family": "dashboard_queue_pressure_contract_v1",
        "producer": "dashboard.troubleshooting",
        "priority": clean_text(priority, 40),
        "action_hint": clean_text(action_hint, 160),
        "escalation_lane": clean_text(escalation_lane, 80),
        "runbook_id": clean_text(runbook_id, 120),
        "escalation_owner": clean_text(escalation_owner, 120),
        "blocking_kind": clean_text(blocking_kind, 80),
        "auto_retry_allowed": auto_retry_allowed,
        "execution_policy": clean_text(execution_policy, 80),
        "manual_gate_required": manual_gate_required,
        "manual_gate_reason": clean_text(manual_gate_reason, 80),
        "requeue_strategy": clean_text(requeue_strategy, 40),
        "can_execute_without_human": can_execute_without_human,
        "execution_window": clean_text(execution_window, 80),
        "manual_gate_timeout_seconds": manual_gate_timeout_seconds.max(0),
        "next_action_after_seconds": next_action_after_seconds.max(0),
        "next_action_kind": clean_text(next_action_kind, 80),
        "retry_window_class": clean_text(retry_window_class, 40),
        "readiness_state": clean_text(readiness_state, 80),
        "readiness_reason": clean_text(readiness_reason, 80),
        "automation_safe": automation_safe,
        "decision_route_hint": dashboard_outbox_pressure_decision_route_hint(next_action_kind),
        "decision_urgency_tier": dashboard_outbox_pressure_decision_urgency_tier(
            retry_window_class,
            automation_safe
        ),
        "decision_retry_budget_class": dashboard_outbox_pressure_decision_retry_budget_class(
            retry_window_class,
            automation_safe
        ),
        "decision_lane_token": dashboard_outbox_pressure_decision_lane_token(
            dashboard_outbox_pressure_decision_route_hint(next_action_kind),
            dashboard_outbox_pressure_decision_urgency_tier(retry_window_class, automation_safe)
        ),
        "decision_dispatch_mode": dashboard_outbox_pressure_decision_dispatch_mode(next_action_kind),
        "decision_manual_ack_required": dashboard_outbox_pressure_decision_manual_ack_required(
            next_action_kind,
            automation_safe
        ),
        "decision_execution_guard": dashboard_outbox_pressure_decision_execution_guard(
            next_action_kind,
            automation_safe
        ),
        "decision_followup_required": dashboard_outbox_pressure_decision_followup_required(
            next_action_kind
        ),
        "decision_vector_version": "v1",
        "decision_vector_key": clean_text(decision_vector_key, 200),
        "decision_vector": dashboard_outbox_pressure_decision_vector(
            next_action_after_seconds,
            next_action_kind,
            retry_window_class,
            readiness_state,
            readiness_reason,
            automation_safe
        ),
        "deadline_epoch_s": if deadline_epoch_s > 0 { deadline_epoch_s } else { 0 },
        "breach_reason": clean_text(breach_reason, 160),
        "snapshot_epoch_s": snapshot_epoch_s
    })
}

fn dashboard_troubleshooting_age_p95_seconds(rows: &[Value], now_epoch: i64) -> i64 {
    let mut ages = rows
        .iter()
        .filter_map(|row| {
            let epoch = dashboard_troubleshooting_epoch_hint(row);
            if epoch > 0 && epoch <= now_epoch {
                Some(now_epoch - epoch)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if ages.is_empty() {
        return 0;
    }
    ages.sort_unstable();
    let idx = ((ages.len().saturating_sub(1)) as f64 * 0.95).round() as usize;
    ages[idx.min(ages.len().saturating_sub(1))]
}

fn dashboard_troubleshooting_epoch_hint(row: &Value) -> i64 {
    row.get("queued_at_epoch_s")
        .or_else(|| row.get("created_at_epoch_s"))
        .or_else(|| row.get("captured_at_epoch_s"))
        .or_else(|| row.pointer("/snapshot/created_at_epoch_s"))
        .and_then(Value::as_i64)
        .unwrap_or(0)
}

fn dashboard_troubleshooting_deadletter_reason(row: &Value) -> String {
    clean_text(
        row.get("reason")
            .and_then(Value::as_str)
            .or_else(|| row.pointer("/row/last_error/error").and_then(Value::as_str))
            .unwrap_or("unknown"),
        120,
    )
    .to_ascii_lowercase()
}

fn dashboard_troubleshooting_issue_request_signature(row: &Value) -> String {
    let issue_request = row
        .get("issue_request")
        .cloned()
        .or_else(|| row.pointer("/row/issue_request").cloned())
        .unwrap_or_else(|| json!({}));
    let stable = json!({
        "title": clean_text(issue_request.get("title").and_then(Value::as_str).unwrap_or(""), 180),
        "body": clean_text(issue_request.get("body").and_then(Value::as_str).unwrap_or(""), 2000),
        "source": clean_text(issue_request.get("source").and_then(Value::as_str).unwrap_or(""), 80),
        "owner": clean_text(issue_request.get("owner").and_then(Value::as_str).unwrap_or(""), 120),
        "repo": clean_text(issue_request.get("repo").and_then(Value::as_str).unwrap_or(""), 120)
    });
    crate::deterministic_receipt_hash(&stable)
}

fn dashboard_troubleshooting_deadletter_row_signature(row: &Value) -> String {
    let reason = dashboard_troubleshooting_deadletter_reason(row);
    let request_sig = dashboard_troubleshooting_issue_request_signature(row);
    crate::v8_kernel::sha256_hex_str(&format!("{}:{}", reason, request_sig))
}

fn dashboard_troubleshooting_append_deadletter_row(root: &Path, row: &Value) -> bool {
    let mut rows = dashboard_troubleshooting_read_deadletter_all(root);
    let signature = dashboard_troubleshooting_deadletter_row_signature(row);
    let already_exists = rows.iter().any(|existing| {
        dashboard_troubleshooting_deadletter_row_signature(existing) == signature
    });
    if already_exists {
        return false;
    }
    rows.push(row.clone());
    if rows.len() > DASHBOARD_TROUBLESHOOTING_MAX_DEADLETTER {
        let keep_from = rows.len() - DASHBOARD_TROUBLESHOOTING_MAX_DEADLETTER;
        rows = rows.split_off(keep_from);
    }
    dashboard_troubleshooting_write_deadletter_rows(root, &rows);
    true
}

fn dashboard_troubleshooting_reason_histogram(rows: &[Value]) -> Vec<Value> {
    let mut counts = HashMap::<String, i64>::new();
    for row in rows {
        let reason = dashboard_troubleshooting_deadletter_reason(row);
        *counts.entry(reason).or_insert(0) += 1;
    }
    dashboard_troubleshooting_sorted_histogram(counts, "reason")
}

fn dashboard_troubleshooting_outbox_error_bucket(row: &Value) -> String {
    let error = clean_text(
        row.get("error_bucket")
            .and_then(Value::as_str)
            .or_else(|| row.pointer("/last_error/error").and_then(Value::as_str))
            .unwrap_or("unknown"),
        120,
    )
    .to_ascii_lowercase();
    if error.is_empty() {
        "unknown".to_string()
    } else {
        error
    }
}

fn dashboard_troubleshooting_outbox_reason_histogram(rows: &[Value]) -> Vec<Value> {
    let mut counts = HashMap::<String, i64>::new();
    for row in rows {
        let bucket = dashboard_troubleshooting_outbox_error_bucket(row);
        *counts.entry(bucket).or_insert(0) += 1;
    }
    dashboard_troubleshooting_sorted_histogram(counts, "error_bucket")
}
