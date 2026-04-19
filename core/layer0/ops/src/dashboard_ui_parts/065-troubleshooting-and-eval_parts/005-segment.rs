fn dashboard_troubleshooting_now_epoch_seconds() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| dur.as_secs() as i64)
        .unwrap_or(0)
}

fn dashboard_payload_truthy_flag(payload: &Value, key: &str) -> bool {
    payload.get(key).is_some_and(|value| {
        value.as_bool().unwrap_or_else(|| {
            value
                .as_str()
                .map(|raw| {
                    let lowered = clean_text(raw, 24).to_ascii_lowercase();
                    matches!(lowered.as_str(), "1" | "true" | "yes" | "on")
                })
                .or_else(|| value.as_i64().map(|raw| raw != 0))
                .unwrap_or(false)
        })
    })
}

fn dashboard_payload_string_list(payload: &Value, key: &str, max_items: usize, max_len: usize) -> Vec<String> {
    payload
        .get(key)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|raw| clean_text(raw, max_len))
                .filter(|raw| !raw.is_empty())
                .take(max_items)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn dashboard_payload_first_string_filter(
    payload: &Value,
    keys: &[&str],
    max_items: usize,
    max_len: usize,
) -> Vec<String> {
    for key in keys {
        let values = dashboard_payload_string_list(payload, key, max_items, max_len);
        if !values.is_empty() {
            return values
                .into_iter()
                .map(|value| value.to_ascii_lowercase())
                .collect::<Vec<_>>();
        }
        if let Some(raw) = payload.get(*key).and_then(Value::as_str) {
            let parsed = raw
                .split(',')
                .map(|value| clean_text(value, max_len))
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .take(max_items)
                .map(|value| value.to_ascii_lowercase())
                .collect::<Vec<_>>();
            if !parsed.is_empty() {
                return parsed;
            }
        }
    }
    Vec::new()
}

fn dashboard_troubleshooting_filter_match(candidate: &str, filter: &str) -> bool {
    let normalized_candidate = clean_text(candidate, 160).to_ascii_lowercase();
    let normalized_filter = clean_text(filter, 160).to_ascii_lowercase();
    if normalized_filter.is_empty() {
        return false;
    }
    if normalized_filter == "*" {
        return true;
    }
    if let Some(prefix) = normalized_filter.strip_suffix('*') {
        return !prefix.is_empty() && normalized_candidate.starts_with(prefix);
    }
    normalized_candidate == normalized_filter
}

fn dashboard_payload_i64_with_bounds(
    payload: &Value,
    key: &str,
    default: i64,
    min: i64,
    max: i64,
) -> i64 {
    let value = payload
        .get(key)
        .and_then(Value::as_i64)
        .unwrap_or(default);
    value.clamp(min, max)
}

fn dashboard_summary_window_seconds(payload: &Value) -> i64 {
    let window_seconds =
        dashboard_payload_i64_with_bounds(payload, "window_seconds", 0, 0, 7 * 24 * 60 * 60);
    if window_seconds > 0 {
        return window_seconds;
    }
    let window_minutes =
        dashboard_payload_i64_with_bounds(payload, "window_minutes", 0, 0, 7 * 24 * 60);
    if window_minutes > 0 {
        return window_minutes.saturating_mul(60);
    }
    0
}

fn dashboard_outbox_health_tier(depth: usize, ready_ratio: f64, blocked_ratio: f64) -> &'static str {
    if depth == 0 {
        "empty"
    } else if ready_ratio >= 0.7 && blocked_ratio <= 0.3 {
        "healthy"
    } else if ready_ratio >= 0.4 {
        "degraded"
    } else {
        "blocked"
    }
}

fn dashboard_outbox_health_reason(depth: usize, ready_ratio: f64, blocked_ratio: f64) -> &'static str {
    if depth == 0 {
        "outbox_empty"
    } else if ready_ratio >= 0.7 && blocked_ratio <= 0.3 {
        "ready_ratio>=0.70_and_blocked_ratio<=0.30"
    } else if ready_ratio >= 0.4 {
        "ready_ratio>=0.40_with_some_cooldown_pressure"
    } else {
        "ready_ratio<0.40_or_cooldown_dominant"
    }
}

fn dashboard_outbox_queue_action_hint(health_tier: &str) -> &'static str {
    match health_tier {
        "empty" => "no_action",
        "healthy" => "drain_normally",
        "degraded" => "increase_flush_frequency_and_monitor_auth",
        _ => "resolve_auth_or_transport_blockers_then_force_flush",
    }
}

fn dashboard_outbox_health_score(
    depth: usize,
    ready_ratio: f64,
    blocked_ratio: f64,
    stale_ratio: f64,
) -> f64 {
    if depth == 0 {
        return 100.0;
    }
    let score = (ready_ratio * 0.65 + (1.0 - blocked_ratio) * 0.25 + (1.0 - stale_ratio) * 0.10)
        * 100.0;
    (score.clamp(0.0, 100.0) * 100.0).round() / 100.0
}

fn dashboard_outbox_retry_pressure_tier(
    depth: usize,
    due_within_60s: usize,
    due_within_300s: usize,
    due_within_900s: usize,
) -> &'static str {
    if depth == 0 {
        "none"
    } else if due_within_60s >= depth.saturating_div(2).max(1)
        || due_within_300s >= depth.saturating_mul(3).saturating_div(4).max(1)
    {
        "high"
    } else if due_within_300s > 0 || due_within_900s >= depth.saturating_div(2).max(1) {
        "medium"
    } else {
        "low"
    }
}

fn dashboard_outbox_pressure_score(
    depth: usize,
    blocked_ratio: f64,
    stale_ratio: f64,
    retry_pressure_tier: &str,
) -> f64 {
    if depth == 0 {
        return 0.0;
    }
    let retry_weight = match retry_pressure_tier {
        "high" => 1.0,
        "medium" => 0.6,
        "low" => 0.25,
        _ => 0.0,
    };
    let score = (blocked_ratio * 0.45 + stale_ratio * 0.35 + retry_weight * 0.20) * 100.0;
    (score.clamp(0.0, 100.0) * 100.0).round() / 100.0
}

fn dashboard_outbox_pressure_tier(pressure_score: f64) -> &'static str {
    if pressure_score >= 70.0 {
        "high"
    } else if pressure_score >= 35.0 {
        "medium"
    } else if pressure_score > 0.0 {
        "low"
    } else {
        "none"
    }
}

fn dashboard_outbox_pressure_action_hint(pressure_tier: &str) -> &'static str {
    match pressure_tier {
        "high" => "run_outbox_flush_and_triage_deadletter_immediately",
        "medium" => "increase_eval_drain_frequency_and_monitor_retry_backlog",
        "low" => "continue_normal_drain_with_periodic_health_checks",
        _ => "no_action",
    }
}

fn dashboard_outbox_pressure_priority(pressure_tier: &str) -> &'static str {
    match pressure_tier {
        "high" => "p0",
        "medium" => "p1",
        "low" => "p2",
        _ => "none",
    }
}

fn dashboard_outbox_pressure_recommended_lane(pressure_tier: &str) -> &'static str {
    match pressure_tier {
        "high" => "dashboard.troubleshooting.outbox.flush",
        "medium" => "dashboard.troubleshooting.eval.drain",
        "low" => "dashboard.troubleshooting.summary",
        _ => "none",
    }
}

fn dashboard_outbox_pressure_runbook_id(pressure_tier: &str) -> &'static str {
    match pressure_tier {
        "high" => "runbook.troubleshooting.queue_pressure.high",
        "medium" => "runbook.troubleshooting.queue_pressure.medium",
        "low" => "runbook.troubleshooting.queue_pressure.low",
        _ => "none",
    }
}

fn dashboard_outbox_pressure_escalation_owner(pressure_tier: &str) -> &'static str {
    match pressure_tier {
        "high" => "ops_oncall",
        "medium" => "runtime_owner",
        "low" => "none",
        _ => "none",
    }
}

fn dashboard_outbox_pressure_sla_minutes(pressure_tier: &str) -> i64 {
    match pressure_tier {
        "high" => 5,
        "medium" => 15,
        "low" => 60,
        _ => 0,
    }
}

fn dashboard_outbox_pressure_escalation_lane(pressure_tier: &str) -> &'static str {
    match pressure_tier {
        "high" => "dashboard.troubleshooting.outbox.flush",
        "medium" => "dashboard.troubleshooting.eval.drain",
        _ => "none",
    }
}

fn dashboard_outbox_pressure_deadline_breach(
    now_epoch: i64,
    pressure_tier: &str,
    oldest_age_seconds: i64,
    blocked_ratio: f64,
    sla_minutes: i64,
    oldest_age_breach_reason: &'static str,
    blocked_ratio_breach_reason: &'static str,
) -> (i64, bool, &'static str) {
    let deadline_epoch_s = if sla_minutes > 0 {
        now_epoch.saturating_add(sla_minutes.saturating_mul(60))
    } else {
        0
    };
    if pressure_tier != "high" {
        return (deadline_epoch_s, false, "none");
    }
    if oldest_age_seconds > 1800 {
        return (deadline_epoch_s, true, oldest_age_breach_reason);
    }
    if blocked_ratio >= 0.7 {
        return (deadline_epoch_s, true, blocked_ratio_breach_reason);
    }
    (deadline_epoch_s, false, "none")
}

fn dashboard_outbox_pressure_deadline_remaining_seconds(
    now_epoch: i64,
    deadline_epoch_s: i64,
) -> i64 {
    if deadline_epoch_s <= 0 || deadline_epoch_s <= now_epoch {
        0
    } else {
        deadline_epoch_s.saturating_sub(now_epoch)
    }
}

fn dashboard_outbox_pressure_breach_detected_at_epoch_s(now_epoch: i64, breach: bool) -> i64 {
    if breach {
        now_epoch
    } else {
        0
    }
}

fn dashboard_outbox_pressure_contract_version() -> &'static str {
    "v1"
}

fn dashboard_outbox_pressure_blocking_kind(tier: &str, breach: bool) -> &'static str {
    if breach || tier == "critical" {
        "manual_intervention_required"
    } else if tier == "high" {
        "operator_review_required"
    } else {
        "none"
    }
}

fn dashboard_outbox_pressure_auto_retry_allowed(blocking_kind: &str) -> bool {
    blocking_kind == "none"
}

fn dashboard_outbox_pressure_execution_policy(
    blocking_kind: &str,
    auto_retry_allowed: bool,
) -> &'static str {
    if auto_retry_allowed {
        "auto_retry"
    } else if blocking_kind == "none" {
        "deferred_auto_retry"
    } else {
        "manual_gate_required"
    }
}

fn dashboard_outbox_pressure_manual_gate_required(blocking_kind: &str) -> bool {
    blocking_kind != "none"
}

fn dashboard_outbox_pressure_manual_gate_reason(blocking_kind: &str) -> &'static str {
    match blocking_kind {
        "manual_intervention_required" => "manual_intervention_required",
        "operator_review_required" => "operator_review_required",
        _ => "none",
    }
}

fn dashboard_outbox_pressure_requeue_strategy(execution_policy: &str) -> &'static str {
    match execution_policy {
        "auto_retry" => "immediate",
        "deferred_auto_retry" => "deferred",
        _ => "manual",
    }
}

fn dashboard_outbox_pressure_can_execute_without_human(execution_policy: &str) -> bool {
    matches!(execution_policy, "auto_retry" | "deferred_auto_retry")
}

fn dashboard_outbox_pressure_execution_window(requeue_strategy: &str) -> &'static str {
    match requeue_strategy {
        "immediate" => "now",
        "deferred" => "deferred",
        _ => "after_manual_gate",
    }
}

fn dashboard_outbox_pressure_manual_gate_timeout_seconds(manual_gate_reason: &str) -> i64 {
    match manual_gate_reason {
        "manual_intervention_required" => 1200,
        "operator_review_required" => 1800,
        _ => 0,
    }
}

fn dashboard_outbox_pressure_next_action_after_seconds(
    execution_window: &str,
    manual_gate_timeout_seconds: i64,
) -> i64 {
    match execution_window {
        "now" => 0,
        "deferred" => 60,
        _ => manual_gate_timeout_seconds.max(0),
    }
}

fn dashboard_outbox_pressure_readiness_state(
    can_execute_without_human: bool,
    next_action_after_seconds: i64,
) -> &'static str {
    if !can_execute_without_human {
        "manual_gate_pending"
    } else if next_action_after_seconds > 0 {
        "deferred_retry_pending"
    } else {
        "ready_now"
    }
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
    readiness_state: &str,
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
        "readiness_state": clean_text(readiness_state, 80),
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

fn dashboard_troubleshooting_outbox_state_lane(root: &Path, payload: &Value) -> LaneResult {
    let limit = dashboard_payload_usize(payload, "limit", 20, 1, 200);
    let now_epoch = dashboard_troubleshooting_now_epoch_seconds();
    let rows = dashboard_troubleshooting_read_issue_outbox(root);
    let ready_count = rows
        .iter()
        .filter(|row| {
            row.get("next_retry_after_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                <= now_epoch
        })
        .count();
    let cooldown_blocked_count = rows.len().saturating_sub(ready_count);
    let next_retry_after_epoch_s = rows
        .iter()
        .filter_map(|row| row.get("next_retry_after_epoch_s").and_then(Value::as_i64))
        .filter(|epoch| *epoch > now_epoch)
        .min()
        .unwrap_or(0);
    let next_retry_after_seconds = if next_retry_after_epoch_s > now_epoch {
        next_retry_after_epoch_s - now_epoch
    } else {
        0
    };
    let mut display_rows = rows.clone();
    if display_rows.len() > limit {
        display_rows = display_rows.split_off(display_rows.len().saturating_sub(limit));
    }
    let mut sequence_cursor = 0i64;
    let display_rows = display_rows
        .into_iter()
        .map(|row| {
            sequence_cursor += 1;
            let epoch = dashboard_troubleshooting_epoch_hint(&row);
            let age_seconds = if epoch > 0 && epoch <= now_epoch {
                now_epoch - epoch
            } else {
                0
            };
            let stale = age_seconds > 900;
            let freshness_tier = if age_seconds <= 300 {
                "fresh"
            } else if age_seconds <= 900 {
                "aging"
            } else {
                "stale"
            };
            let mut out = row.clone();
            if let Some(obj) = out.as_object_mut() {
                obj.insert("source_sequence".to_string(), json!(sequence_cursor));
                obj.insert("age_seconds".to_string(), json!(age_seconds));
                obj.insert("stale".to_string(), json!(stale));
                obj.insert("freshness_tier".to_string(), json!(freshness_tier));
                obj.insert("source".to_string(), json!("issue_outbox"));
            }
            out
        })
        .collect::<Vec<_>>();
    let oldest_epoch = rows
        .iter()
        .map(dashboard_troubleshooting_epoch_hint)
        .filter(|epoch| *epoch > 0)
        .min()
        .unwrap_or(0);
    let oldest_age_seconds = if oldest_epoch > 0 && oldest_epoch <= now_epoch {
        now_epoch - oldest_epoch
    } else {
        0
    };
    let age_p95_seconds = dashboard_troubleshooting_age_p95_seconds(&rows, now_epoch);
    let max_attempts_observed = rows
        .iter()
        .filter_map(|row| row.get("attempts").and_then(Value::as_i64))
        .max()
        .unwrap_or(0);
    let blocked_ratio = if rows.is_empty() {
        0.0
    } else {
        ((cooldown_blocked_count as f64) / (rows.len() as f64) * 10_000.0).round() / 10_000.0
    };
    let oldest_item_id = rows
        .iter()
        .filter_map(|row| {
            let epoch = dashboard_troubleshooting_epoch_hint(row);
            if epoch <= 0 {
                return None;
            }
            Some((
                epoch,
                clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 80),
            ))
        })
        .min_by_key(|(epoch, _)| *epoch)
        .map(|(_, id)| id)
        .unwrap_or_default();
    let next_retry_item_id = rows
        .iter()
        .filter_map(|row| {
            let epoch = row
                .get("next_retry_after_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            if epoch <= now_epoch {
                return None;
            }
            Some((
                epoch,
                clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 80),
            ))
        })
        .min_by_key(|(epoch, _)| *epoch)
        .map(|(_, id)| id)
        .unwrap_or_default();
    let retry_due_within_60s_count = rows
        .iter()
        .filter(|row| {
            let epoch = row
                .get("next_retry_after_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            epoch > now_epoch && epoch <= now_epoch + 60
        })
        .count();
    let retry_due_within_300s_count = rows
        .iter()
        .filter(|row| {
            let epoch = row
                .get("next_retry_after_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            epoch > now_epoch && epoch <= now_epoch + 300
        })
        .count();
    let retry_due_within_900s_count = rows
        .iter()
        .filter(|row| {
            let epoch = row
                .get("next_retry_after_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            epoch > now_epoch && epoch <= now_epoch + 900
        })
        .count();
    let fresh_count = rows
        .iter()
        .filter(|row| {
            let epoch = dashboard_troubleshooting_epoch_hint(row);
            epoch > 0 && epoch <= now_epoch && (now_epoch - epoch) <= 300
        })
        .count();
    let aging_count = rows
        .iter()
        .filter(|row| {
            let epoch = dashboard_troubleshooting_epoch_hint(row);
            epoch > 0 && epoch <= now_epoch && (now_epoch - epoch) > 300 && (now_epoch - epoch) <= 900
        })
        .count();
    let stale_count = rows
        .iter()
        .filter(|row| {
            let epoch = dashboard_troubleshooting_epoch_hint(row);
            epoch > 0 && epoch <= now_epoch && (now_epoch - epoch) > 900
        })
        .count();
    let stale_oldest_age_seconds = rows
        .iter()
        .filter_map(|row| {
            let epoch = dashboard_troubleshooting_epoch_hint(row);
            if epoch > 0 && epoch <= now_epoch && (now_epoch - epoch) > 900 {
                Some(now_epoch - epoch)
            } else {
                None
            }
        })
        .max()
        .unwrap_or(0);
    let fresh_ratio = if rows.is_empty() {
        0.0
    } else {
        ((fresh_count as f64) / (rows.len() as f64) * 10_000.0).round() / 10_000.0
    };
    let aging_ratio = if rows.is_empty() {
        0.0
    } else {
        ((aging_count as f64) / (rows.len() as f64) * 10_000.0).round() / 10_000.0
    };
    let stale_ratio = if rows.is_empty() {
        0.0
    } else {
        ((stale_count as f64) / (rows.len() as f64) * 10_000.0).round() / 10_000.0
    };
    let ready_ratio = if rows.is_empty() {
        0.0
    } else {
        ((ready_count as f64) / (rows.len() as f64) * 10_000.0).round() / 10_000.0
    };
    let health_tier = dashboard_outbox_health_tier(rows.len(), ready_ratio, blocked_ratio);
    let health_reason = dashboard_outbox_health_reason(rows.len(), ready_ratio, blocked_ratio);
    let queue_action_hint = dashboard_outbox_queue_action_hint(health_tier);
    let health_score =
        dashboard_outbox_health_score(rows.len(), ready_ratio, blocked_ratio, stale_ratio);
    let retry_pressure_tier = dashboard_outbox_retry_pressure_tier(
        rows.len(),
        retry_due_within_60s_count,
        retry_due_within_300s_count,
        retry_due_within_900s_count,
    );
    let queue_pressure_score = dashboard_outbox_pressure_score(
        rows.len(),
        blocked_ratio,
        stale_ratio,
        retry_pressure_tier,
    );
    let queue_pressure_tier = dashboard_outbox_pressure_tier(queue_pressure_score);
    let queue_pressure_action_hint =
        dashboard_outbox_pressure_action_hint(queue_pressure_tier);
    let queue_pressure_priority = dashboard_outbox_pressure_priority(queue_pressure_tier);
    let queue_pressure_recommended_lane =
        dashboard_outbox_pressure_recommended_lane(queue_pressure_tier);
    let queue_pressure_escalation_required = queue_pressure_priority == "p0";
    let queue_pressure_escalation_reason = if queue_pressure_escalation_required {
        "high_queue_pressure"
    } else {
        "none"
    };
    let queue_pressure_runbook_id = dashboard_outbox_pressure_runbook_id(queue_pressure_tier);
    let queue_pressure_escalation_owner =
        dashboard_outbox_pressure_escalation_owner(queue_pressure_tier);
    let queue_pressure_sla_minutes =
        dashboard_outbox_pressure_sla_minutes(queue_pressure_tier);
    let queue_pressure_escalation_lane =
        dashboard_outbox_pressure_escalation_lane(queue_pressure_tier);
    let (queue_pressure_deadline_epoch_s, queue_pressure_breach, queue_pressure_breach_reason) =
        dashboard_outbox_pressure_deadline_breach(
            now_epoch,
            queue_pressure_tier,
            stale_oldest_age_seconds,
            blocked_ratio,
            queue_pressure_sla_minutes,
            "stale_oldest_age_seconds>1800",
            "blocked_ratio>=0.70",
        );
    let queue_pressure_deadline_remaining_seconds =
        dashboard_outbox_pressure_deadline_remaining_seconds(
            now_epoch,
            queue_pressure_deadline_epoch_s,
        );
    let queue_pressure_breach_detected_at_epoch_s =
        dashboard_outbox_pressure_breach_detected_at_epoch_s(now_epoch, queue_pressure_breach);
    let queue_pressure_blocking_kind =
        dashboard_outbox_pressure_blocking_kind(queue_pressure_tier, queue_pressure_breach);
    let queue_pressure_auto_retry_allowed =
        dashboard_outbox_pressure_auto_retry_allowed(queue_pressure_blocking_kind);
    let queue_pressure_execution_policy = dashboard_outbox_pressure_execution_policy(
        queue_pressure_blocking_kind,
        queue_pressure_auto_retry_allowed,
    );
    let queue_pressure_manual_gate_required =
        dashboard_outbox_pressure_manual_gate_required(queue_pressure_blocking_kind);
    let queue_pressure_manual_gate_reason =
        dashboard_outbox_pressure_manual_gate_reason(queue_pressure_blocking_kind);
    let queue_pressure_requeue_strategy =
        dashboard_outbox_pressure_requeue_strategy(queue_pressure_execution_policy);
    let queue_pressure_can_execute_without_human =
        dashboard_outbox_pressure_can_execute_without_human(queue_pressure_execution_policy);
    let queue_pressure_execution_window =
        dashboard_outbox_pressure_execution_window(queue_pressure_requeue_strategy);
    let queue_pressure_manual_gate_timeout_seconds =
        dashboard_outbox_pressure_manual_gate_timeout_seconds(queue_pressure_manual_gate_reason);
    let queue_pressure_next_action_after_seconds = dashboard_outbox_pressure_next_action_after_seconds(
        queue_pressure_execution_window,
        queue_pressure_manual_gate_timeout_seconds,
    );
    let queue_pressure_readiness_state = dashboard_outbox_pressure_readiness_state(
        queue_pressure_can_execute_without_human,
        queue_pressure_next_action_after_seconds,
    );
    let compaction_recommended = stale_ratio >= 0.4 || oldest_age_seconds > 86_400;
    let compaction_reason = if rows.is_empty() {
        "none"
    } else if stale_ratio >= 0.4 {
        "stale_ratio>=0.40"
    } else if oldest_age_seconds > 86_400 {
        "oldest_age_seconds>86400"
    } else {
        "none"
    };
    LaneResult {
        ok: true,
        status: 0,
        argv: vec!["dashboard.troubleshooting.outbox.state".to_string()],
        payload: Some(json!({
            "ok": true,
            "type": "dashboard_troubleshooting_outbox_state",
            "depth": rows.len(),
            "ready_count": ready_count,
            "ready_ratio": ready_ratio,
            "cooldown_blocked_count": cooldown_blocked_count,
            "blocked_ratio": blocked_ratio,
            "health_tier": health_tier,
            "health_reason": health_reason,
            "oldest_age_seconds": oldest_age_seconds,
            "age_p95_seconds": age_p95_seconds,
            "oldest_item_id": oldest_item_id,
            "max_attempts_observed": max_attempts_observed,
            "next_retry_after_epoch_s": next_retry_after_epoch_s,
            "next_retry_after_seconds": next_retry_after_seconds,
            "next_retry_item_id": next_retry_item_id,
            "retry_due_within_60s_count": retry_due_within_60s_count,
            "retry_due_within_300s_count": retry_due_within_300s_count,
            "retry_due_within_900s_count": retry_due_within_900s_count,
            "fresh_count": fresh_count,
            "fresh_ratio": fresh_ratio,
            "aging_count": aging_count,
            "aging_ratio": aging_ratio,
            "stale_count": stale_count,
            "stale_ratio": stale_ratio,
            "stale_oldest_age_seconds": stale_oldest_age_seconds,
            "health_score": health_score,
            "queue_action_hint": queue_action_hint,
            "retry_pressure_tier": retry_pressure_tier,
            "queue_pressure_score": queue_pressure_score,
            "queue_pressure_tier": queue_pressure_tier,
            "queue_pressure_action_hint": queue_pressure_action_hint,
            "queue_pressure_priority": queue_pressure_priority,
            "queue_pressure_recommended_lane": queue_pressure_recommended_lane,
            "queue_pressure_escalation_required": queue_pressure_escalation_required,
            "queue_pressure_escalation_reason": queue_pressure_escalation_reason,
            "queue_pressure_runbook_id": queue_pressure_runbook_id,
            "queue_pressure_escalation_owner": queue_pressure_escalation_owner,
            "queue_pressure_sla_minutes": queue_pressure_sla_minutes,
            "queue_pressure_escalation_lane": queue_pressure_escalation_lane,
            "queue_pressure_deadline_epoch_s": queue_pressure_deadline_epoch_s,
            "queue_pressure_deadline_remaining_seconds": queue_pressure_deadline_remaining_seconds,
            "queue_pressure_breach": queue_pressure_breach,
            "queue_pressure_breach_reason": queue_pressure_breach_reason,
            "queue_pressure_breach_detected_at_epoch_s": queue_pressure_breach_detected_at_epoch_s,
            "queue_pressure_blocking_kind": queue_pressure_blocking_kind,
            "queue_pressure_auto_retry_allowed": queue_pressure_auto_retry_allowed,
            "queue_pressure_execution_policy": queue_pressure_execution_policy,
            "queue_pressure_manual_gate_required": queue_pressure_manual_gate_required,
            "queue_pressure_manual_gate_reason": queue_pressure_manual_gate_reason,
            "queue_pressure_requeue_strategy": queue_pressure_requeue_strategy,
            "queue_pressure_can_execute_without_human": queue_pressure_can_execute_without_human,
            "queue_pressure_execution_window": queue_pressure_execution_window,
            "queue_pressure_manual_gate_timeout_seconds": queue_pressure_manual_gate_timeout_seconds,
            "queue_pressure_next_action_after_seconds": queue_pressure_next_action_after_seconds,
            "queue_pressure_readiness_state": queue_pressure_readiness_state,
            "queue_pressure_contract_version": dashboard_outbox_pressure_contract_version(),
            "queue_pressure_contract_family": "dashboard_queue_pressure_contract_v1",
            "queue_pressure_snapshot_epoch_s": now_epoch,
            "queue_pressure_contract": dashboard_outbox_pressure_contract(
                now_epoch,
                queue_pressure_priority,
                queue_pressure_action_hint,
                queue_pressure_escalation_lane,
                queue_pressure_runbook_id,
                queue_pressure_escalation_owner,
                queue_pressure_blocking_kind,
                queue_pressure_auto_retry_allowed,
                queue_pressure_execution_policy,
                queue_pressure_manual_gate_required,
                queue_pressure_manual_gate_reason,
                queue_pressure_requeue_strategy,
                queue_pressure_can_execute_without_human,
                queue_pressure_execution_window,
                queue_pressure_manual_gate_timeout_seconds,
                queue_pressure_next_action_after_seconds,
                queue_pressure_readiness_state,
                queue_pressure_deadline_epoch_s,
                queue_pressure_breach_reason
            ),
            "compaction_recommended": compaction_recommended,
            "compaction_reason": compaction_reason,
            "error_histogram": dashboard_troubleshooting_outbox_reason_histogram(&rows),
            "items": display_rows
        })),
    }
}

fn dashboard_troubleshooting_summary_lane(root: &Path, payload: &Value) -> LaneResult {
    let limit = dashboard_payload_usize(payload, "limit", 20, 1, 200);
    let now_epoch = dashboard_troubleshooting_now_epoch_seconds();
    let window_seconds = dashboard_summary_window_seconds(payload);
    let explicit_since_epoch = dashboard_payload_i64_with_bounds(
        payload,
        "since_epoch_s",
        0,
        0,
        i64::MAX,
    );
    let effective_since_epoch = if explicit_since_epoch > 0 {
        explicit_since_epoch
    } else if window_seconds > 0 {
        now_epoch.saturating_sub(window_seconds)
    } else {
        0
    };
    let window_filter_applied = effective_since_epoch > 0;
    let recent_entries = dashboard_troubleshooting_read_recent_entries(root);
    let classification_filter = dashboard_payload_first_string_filter(
        payload,
        &["classification_filter", "class_filter", "classifications"],
        12,
        80,
    );
    let error_filter =
        dashboard_payload_first_string_filter(payload, &["error_filter", "errors", "error_codes"], 12, 120);
    let filters_applied = !classification_filter.is_empty() || !error_filter.is_empty();
    let filtered_entries = recent_entries
        .iter()
        .filter(|row| {
            let error_code = clean_text(
                row.pointer("/workflow/error_code")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120,
            )
            .to_ascii_lowercase();
            let class = clean_text(
                row.pointer("/workflow/classification")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
                80,
            )
            .to_ascii_lowercase();
            let class_match = classification_filter.is_empty()
                || classification_filter
                    .iter()
                    .any(|value| dashboard_troubleshooting_filter_match(&class, value));
            let error_match = error_filter.is_empty()
                || error_filter
                    .iter()
                    .any(|value| dashboard_troubleshooting_filter_match(&error_code, value));
            let window_match = if window_filter_applied {
                let epoch = dashboard_troubleshooting_epoch_hint(row);
                epoch > 0 && epoch >= effective_since_epoch
            } else {
                true
            };
            class_match && error_match && window_match
        })
        .cloned()
        .collect::<Vec<_>>();
    let entry_count = filtered_entries.len();
    let filtered_out_count = recent_entries.len().saturating_sub(filtered_entries.len());
    let failure_count = filtered_entries
        .iter()
        .filter(|row| dashboard_troubleshooting_exchange_failed(row))
        .count();
    let stale_count = filtered_entries
        .iter()
        .filter(|row| row.get("stale").and_then(Value::as_bool).unwrap_or(false))
        .count();
    let failure_rate = if entry_count == 0 {
        0.0
    } else {
        ((failure_count as f64) / (entry_count as f64) * 10_000.0).round() / 10_000.0
    };
    let stale_rate = if entry_count == 0 {
        0.0
    } else {
        ((stale_count as f64) / (entry_count as f64) * 10_000.0).round() / 10_000.0
    };
    let mut error_counts = HashMap::<String, i64>::new();
    let mut class_counts = HashMap::<String, i64>::new();
    for row in &filtered_entries {
        let error_code = clean_text(
            row.pointer("/workflow/error_code")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        )
        .to_ascii_lowercase();
        if !error_code.is_empty() {
            *error_counts.entry(error_code).or_insert(0) += 1;
        }
        let class = clean_text(
            row.pointer("/workflow/classification")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            80,
        )
        .to_ascii_lowercase();
        *class_counts.entry(class).or_insert(0) += 1;
    }
    let error_hist = dashboard_troubleshooting_sorted_histogram(error_counts, "error");
    let class_hist = dashboard_troubleshooting_sorted_histogram(class_counts, "classification");
    let top_error = error_hist
        .first()
        .and_then(|row| row.get("error"))
        .and_then(Value::as_str)
        .unwrap_or("none");
    let top_class = class_hist
        .first()
        .and_then(|row| row.get("classification"))
        .and_then(Value::as_str)
        .unwrap_or("none");
    let recommendations = dashboard_troubleshooting_eval_recommendations(top_error, top_class);
    let outbox = dashboard_troubleshooting_read_issue_outbox(root);
    let deadletter = dashboard_troubleshooting_read_deadletter_all(root);
    let eval_queue = dashboard_troubleshooting_read_eval_queue(root);
    let outbox_ready_count = outbox
        .iter()
        .filter(|row| {
            row.get("next_retry_after_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                <= now_epoch
        })
        .count();
    let outbox_ready_ratio = if outbox.is_empty() {
        0.0
    } else {
        ((outbox_ready_count as f64) / (outbox.len() as f64) * 10_000.0).round() / 10_000.0
    };
    let outbox_blocked_ratio = if outbox.is_empty() {
        0.0
    } else {
        (((outbox.len().saturating_sub(outbox_ready_count)) as f64) / (outbox.len() as f64) * 10_000.0)
            .round()
            / 10_000.0
    };
    let outbox_stale_count = outbox
        .iter()
        .filter(|row| {
            let epoch = dashboard_troubleshooting_epoch_hint(row);
            epoch > 0 && epoch <= now_epoch && (now_epoch - epoch) > 900
        })
        .count();
    let outbox_fresh_count = outbox
        .iter()
        .filter(|row| {
            let epoch = dashboard_troubleshooting_epoch_hint(row);
            epoch > 0 && epoch <= now_epoch && (now_epoch - epoch) <= 300
        })
        .count();
    let outbox_aging_count = outbox
        .iter()
        .filter(|row| {
            let epoch = dashboard_troubleshooting_epoch_hint(row);
            epoch > 0 && epoch <= now_epoch && (now_epoch - epoch) > 300 && (now_epoch - epoch) <= 900
        })
        .count();
    let outbox_fresh_ratio = if outbox.is_empty() {
        0.0
    } else {
        ((outbox_fresh_count as f64) / (outbox.len() as f64) * 10_000.0).round() / 10_000.0
    };
    let outbox_aging_ratio = if outbox.is_empty() {
        0.0
    } else {
        ((outbox_aging_count as f64) / (outbox.len() as f64) * 10_000.0).round() / 10_000.0
    };
    let outbox_stale_ratio = if outbox.is_empty() {
        0.0
    } else {
        ((outbox_stale_count as f64) / (outbox.len() as f64) * 10_000.0).round() / 10_000.0
    };
    let outbox_oldest_epoch = outbox
        .iter()
        .map(dashboard_troubleshooting_epoch_hint)
        .filter(|epoch| *epoch > 0)
        .min()
        .unwrap_or(0);
    let outbox_oldest_age_seconds = if outbox_oldest_epoch > 0 && outbox_oldest_epoch <= now_epoch {
        now_epoch - outbox_oldest_epoch
    } else {
        0
    };
    let outbox_age_p95_seconds = dashboard_troubleshooting_age_p95_seconds(&outbox, now_epoch);
    let outbox_retry_due_within_60s_count = outbox
        .iter()
        .filter(|row| {
            let epoch = row
                .get("next_retry_after_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            epoch > now_epoch && epoch <= now_epoch + 60
        })
        .count();
    let outbox_retry_due_within_300s_count = outbox
        .iter()
        .filter(|row| {
            let epoch = row
                .get("next_retry_after_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            epoch > now_epoch && epoch <= now_epoch + 300
        })
        .count();
    let outbox_retry_due_within_900s_count = outbox
        .iter()
        .filter(|row| {
            let epoch = row
                .get("next_retry_after_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            epoch > now_epoch && epoch <= now_epoch + 900
        })
        .count();
    let outbox_health_tier =
        dashboard_outbox_health_tier(outbox.len(), outbox_ready_ratio, outbox_blocked_ratio);
    let outbox_health_reason =
        dashboard_outbox_health_reason(outbox.len(), outbox_ready_ratio, outbox_blocked_ratio);
    let outbox_queue_action_hint = dashboard_outbox_queue_action_hint(outbox_health_tier);
    let outbox_health_score = dashboard_outbox_health_score(
        outbox.len(),
        outbox_ready_ratio,
        outbox_blocked_ratio,
        outbox_stale_ratio,
    );
    let outbox_retry_pressure_tier = dashboard_outbox_retry_pressure_tier(
        outbox.len(),
        outbox_retry_due_within_60s_count,
        outbox_retry_due_within_300s_count,
        outbox_retry_due_within_900s_count,
    );
    let outbox_queue_pressure_score = dashboard_outbox_pressure_score(
        outbox.len(),
        outbox_blocked_ratio,
        outbox_stale_ratio,
        outbox_retry_pressure_tier,
    );
    let outbox_queue_pressure_tier =
        dashboard_outbox_pressure_tier(outbox_queue_pressure_score);
    let outbox_queue_pressure_action_hint =
        dashboard_outbox_pressure_action_hint(outbox_queue_pressure_tier);
    let outbox_queue_pressure_priority =
        dashboard_outbox_pressure_priority(outbox_queue_pressure_tier);
    let outbox_queue_pressure_recommended_lane =
        dashboard_outbox_pressure_recommended_lane(outbox_queue_pressure_tier);
    let outbox_queue_pressure_escalation_required = outbox_queue_pressure_priority == "p0";
    let outbox_queue_pressure_escalation_reason = if outbox_queue_pressure_escalation_required {
        "high_queue_pressure"
    } else {
        "none"
    };
    let outbox_queue_pressure_runbook_id =
        dashboard_outbox_pressure_runbook_id(outbox_queue_pressure_tier);
    let outbox_queue_pressure_escalation_owner =
        dashboard_outbox_pressure_escalation_owner(outbox_queue_pressure_tier);
    let outbox_queue_pressure_sla_minutes =
        dashboard_outbox_pressure_sla_minutes(outbox_queue_pressure_tier);
    let outbox_queue_pressure_escalation_lane =
        dashboard_outbox_pressure_escalation_lane(outbox_queue_pressure_tier);
    let (
        outbox_queue_pressure_deadline_epoch_s,
        outbox_queue_pressure_breach,
        outbox_queue_pressure_breach_reason,
    ) = dashboard_outbox_pressure_deadline_breach(
        now_epoch,
        outbox_queue_pressure_tier,
        outbox_oldest_age_seconds,
        outbox_blocked_ratio,
        outbox_queue_pressure_sla_minutes,
        "outbox_oldest_age_seconds>1800",
        "outbox_blocked_ratio>=0.70",
    );
    let outbox_queue_pressure_deadline_remaining_seconds =
        dashboard_outbox_pressure_deadline_remaining_seconds(
            now_epoch,
            outbox_queue_pressure_deadline_epoch_s,
        );
    let outbox_queue_pressure_breach_detected_at_epoch_s =
        dashboard_outbox_pressure_breach_detected_at_epoch_s(
            now_epoch,
            outbox_queue_pressure_breach,
        );
    let outbox_queue_pressure_blocking_kind = dashboard_outbox_pressure_blocking_kind(
        outbox_queue_pressure_tier,
        outbox_queue_pressure_breach,
    );
    let outbox_queue_pressure_auto_retry_allowed =
        dashboard_outbox_pressure_auto_retry_allowed(outbox_queue_pressure_blocking_kind);
    let outbox_queue_pressure_execution_policy = dashboard_outbox_pressure_execution_policy(
        outbox_queue_pressure_blocking_kind,
        outbox_queue_pressure_auto_retry_allowed,
    );
    let outbox_queue_pressure_manual_gate_required =
        dashboard_outbox_pressure_manual_gate_required(outbox_queue_pressure_blocking_kind);
    let outbox_queue_pressure_manual_gate_reason =
        dashboard_outbox_pressure_manual_gate_reason(outbox_queue_pressure_blocking_kind);
    let outbox_queue_pressure_requeue_strategy =
        dashboard_outbox_pressure_requeue_strategy(outbox_queue_pressure_execution_policy);
    let outbox_queue_pressure_can_execute_without_human =
        dashboard_outbox_pressure_can_execute_without_human(outbox_queue_pressure_execution_policy);
    let outbox_queue_pressure_execution_window =
        dashboard_outbox_pressure_execution_window(outbox_queue_pressure_requeue_strategy);
    let outbox_queue_pressure_manual_gate_timeout_seconds =
        dashboard_outbox_pressure_manual_gate_timeout_seconds(outbox_queue_pressure_manual_gate_reason);
    let outbox_queue_pressure_next_action_after_seconds =
        dashboard_outbox_pressure_next_action_after_seconds(
            outbox_queue_pressure_execution_window,
            outbox_queue_pressure_manual_gate_timeout_seconds,
        );
    let outbox_queue_pressure_readiness_state = dashboard_outbox_pressure_readiness_state(
        outbox_queue_pressure_can_execute_without_human,
        outbox_queue_pressure_next_action_after_seconds,
    );
    let outbox_compaction_recommended =
        outbox_stale_ratio >= 0.4 || outbox_oldest_age_seconds > 86_400;
    let outbox_compaction_reason = if outbox.is_empty() {
        "none"
    } else if outbox_stale_ratio >= 0.4 {
        "stale_ratio>=0.40"
    } else if outbox_oldest_age_seconds > 86_400 {
        "oldest_age_seconds>86400"
    } else {
        "none"
    };
    let mut recent = filtered_entries.clone();
    if recent.len() > limit {
        recent = recent.split_off(recent.len().saturating_sub(limit));
    }
    let no_filter_matches = filters_applied && filtered_entries.is_empty();
    let top_error_count = error_hist
        .first()
        .and_then(|row| row.get("count"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let top_class_count = class_hist
        .first()
        .and_then(|row| row.get("count"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let top_error_share = if entry_count == 0 {
        0.0
    } else {
        ((top_error_count as f64) / (entry_count as f64) * 10_000.0).round() / 10_000.0
    };
    let top_classification_share = if entry_count == 0 {
        0.0
    } else {
        ((top_class_count as f64) / (entry_count as f64) * 10_000.0).round() / 10_000.0
    };
    let severity_tier = if entry_count == 0 {
        "none"
    } else if failure_rate >= 0.6 || top_error_share >= 0.6 || top_classification_share >= 0.6 {
        "high"
    } else if failure_rate >= 0.3 || top_error_share >= 0.3 || top_classification_share >= 0.3 {
        "medium"
    } else {
        "low"
    };
    LaneResult {
        ok: true,
        status: 0,
        argv: vec!["dashboard.troubleshooting.summary".to_string()],
        payload: Some(json!({
            "ok": true,
            "type": "dashboard_troubleshooting_summary",
            "filters": {
                "applied": filters_applied,
                "classification": classification_filter,
                "error": error_filter,
                "no_match": no_filter_matches
            },
            "window": {
                "applied": window_filter_applied,
                "window_seconds": window_seconds,
                "since_epoch_s": effective_since_epoch,
                "filtered_out_count": filtered_out_count
            },
            "top_failure_cluster": {
                "top_error": top_error,
                "top_error_count": top_error_count,
                "top_error_share": top_error_share,
                "top_classification": top_class,
                "top_classification_count": top_class_count,
                "top_classification_share": top_classification_share,
                "severity_tier": severity_tier
            },
            "recent": {
                "entry_count": entry_count,
                "total_entry_count": recent_entries.len(),
                "failure_count": failure_count,
                "failure_rate": failure_rate,
                "stale_count": stale_count,
                "stale_rate": stale_rate,
                "error_histogram": error_hist,
                "classification_histogram": class_hist,
                "entries": recent
            },
            "queues": {
                "eval_depth": eval_queue.len(),
                "outbox_depth": outbox.len(),
                "deadletter_depth": deadletter.len(),
                "outbox_health": {
                    "ready_count": outbox_ready_count,
                    "ready_ratio": outbox_ready_ratio,
                    "blocked_ratio": outbox_blocked_ratio,
                    "fresh_count": outbox_fresh_count,
                    "fresh_ratio": outbox_fresh_ratio,
                    "aging_count": outbox_aging_count,
                    "aging_ratio": outbox_aging_ratio,
                    "stale_count": outbox_stale_count,
                    "stale_ratio": outbox_stale_ratio,
                    "oldest_age_seconds": outbox_oldest_age_seconds,
                    "age_p95_seconds": outbox_age_p95_seconds,
                    "health_score": outbox_health_score,
                    "health_tier": outbox_health_tier,
                    "health_reason": outbox_health_reason,
                    "queue_action_hint": outbox_queue_action_hint,
                    "retry_due_within_60s_count": outbox_retry_due_within_60s_count,
                    "retry_due_within_300s_count": outbox_retry_due_within_300s_count,
                    "retry_due_within_900s_count": outbox_retry_due_within_900s_count,
                    "retry_pressure_tier": outbox_retry_pressure_tier,
                    "queue_pressure_score": outbox_queue_pressure_score,
                    "queue_pressure_tier": outbox_queue_pressure_tier,
                    "queue_pressure_action_hint": outbox_queue_pressure_action_hint,
                    "queue_pressure_priority": outbox_queue_pressure_priority,
                    "queue_pressure_recommended_lane": outbox_queue_pressure_recommended_lane,
                    "queue_pressure_escalation_required": outbox_queue_pressure_escalation_required,
                    "queue_pressure_escalation_reason": outbox_queue_pressure_escalation_reason,
                    "queue_pressure_runbook_id": outbox_queue_pressure_runbook_id,
                    "queue_pressure_escalation_owner": outbox_queue_pressure_escalation_owner,
                    "queue_pressure_sla_minutes": outbox_queue_pressure_sla_minutes,
                    "queue_pressure_escalation_lane": outbox_queue_pressure_escalation_lane,
                    "queue_pressure_deadline_epoch_s": outbox_queue_pressure_deadline_epoch_s,
                    "queue_pressure_deadline_remaining_seconds": outbox_queue_pressure_deadline_remaining_seconds,
                    "queue_pressure_breach": outbox_queue_pressure_breach,
                    "queue_pressure_breach_reason": outbox_queue_pressure_breach_reason,
                    "queue_pressure_breach_detected_at_epoch_s": outbox_queue_pressure_breach_detected_at_epoch_s,
                    "queue_pressure_blocking_kind": outbox_queue_pressure_blocking_kind,
                    "queue_pressure_auto_retry_allowed": outbox_queue_pressure_auto_retry_allowed,
                    "queue_pressure_execution_policy": outbox_queue_pressure_execution_policy,
                    "queue_pressure_manual_gate_required": outbox_queue_pressure_manual_gate_required,
                    "queue_pressure_manual_gate_reason": outbox_queue_pressure_manual_gate_reason,
                    "queue_pressure_requeue_strategy": outbox_queue_pressure_requeue_strategy,
                    "queue_pressure_can_execute_without_human": outbox_queue_pressure_can_execute_without_human,
                    "queue_pressure_execution_window": outbox_queue_pressure_execution_window,
                    "queue_pressure_manual_gate_timeout_seconds": outbox_queue_pressure_manual_gate_timeout_seconds,
                    "queue_pressure_next_action_after_seconds": outbox_queue_pressure_next_action_after_seconds,
                    "queue_pressure_readiness_state": outbox_queue_pressure_readiness_state,
                    "queue_pressure_contract_version": dashboard_outbox_pressure_contract_version(),
                    "queue_pressure_contract_family": "dashboard_queue_pressure_contract_v1",
                    "queue_pressure_snapshot_epoch_s": now_epoch,
                    "queue_pressure_contract": dashboard_outbox_pressure_contract(
                        now_epoch,
                        outbox_queue_pressure_priority,
                        outbox_queue_pressure_action_hint,
                        outbox_queue_pressure_escalation_lane,
                        outbox_queue_pressure_runbook_id,
                        outbox_queue_pressure_escalation_owner,
                        outbox_queue_pressure_blocking_kind,
                        outbox_queue_pressure_auto_retry_allowed,
                        outbox_queue_pressure_execution_policy,
                        outbox_queue_pressure_manual_gate_required,
                        outbox_queue_pressure_manual_gate_reason,
                        outbox_queue_pressure_requeue_strategy,
                        outbox_queue_pressure_can_execute_without_human,
                        outbox_queue_pressure_execution_window,
                        outbox_queue_pressure_manual_gate_timeout_seconds,
                        outbox_queue_pressure_next_action_after_seconds,
                        outbox_queue_pressure_readiness_state,
                        outbox_queue_pressure_deadline_epoch_s,
                        outbox_queue_pressure_breach_reason
                    ),
                    "compaction_recommended": outbox_compaction_recommended,
                    "compaction_reason": outbox_compaction_reason
                },
                "outbox_error_histogram": dashboard_troubleshooting_outbox_reason_histogram(&outbox),
                "deadletter_reason_histogram": dashboard_troubleshooting_reason_histogram(&deadletter)
            },
            "recommendations": recommendations
        })),
    }
}

fn dashboard_troubleshooting_read_deadletter_rows(root: &Path, limit: usize) -> Vec<Value> {
    let path = root.join(DASHBOARD_TROUBLESHOOTING_ISSUE_DEADLETTER_REL);
    let raw = fs::read_to_string(path).unwrap_or_default();
    if raw.is_empty() {
        return Vec::new();
    }
    let mut rows = Vec::<Value>::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
            rows.push(value);
        }
    }
    if rows.len() > limit {
        rows.split_off(rows.len().saturating_sub(limit))
    } else {
        rows
    }
}

fn dashboard_troubleshooting_read_deadletter_all(root: &Path) -> Vec<Value> {
    dashboard_troubleshooting_read_deadletter_rows(root, usize::MAX)
}

fn dashboard_troubleshooting_write_deadletter_rows(root: &Path, rows: &[Value]) {
    let path = root.join(DASHBOARD_TROUBLESHOOTING_ISSUE_DEADLETTER_REL);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if rows.is_empty() {
        let _ = fs::write(path, "");
        return;
    }
    let mut out = String::new();
    for row in rows {
        if let Ok(line) = serde_json::to_string(row) {
            out.push_str(&line);
            out.push('\n');
        }
    }
    let _ = fs::write(path, out);
}

fn dashboard_troubleshooting_outbox_flush_lane(root: &Path, payload: &Value) -> LaneResult {
    let max_items = dashboard_payload_usize(payload, "max_items", 10, 1, 50);
    let force = dashboard_payload_truthy_flag(payload, "force")
        || dashboard_payload_truthy_flag(payload, "ignore_cooldown");
    let dry_run = dashboard_payload_truthy_flag(payload, "dry_run")
        || dashboard_payload_truthy_flag(payload, "preview_only")
        || dashboard_payload_truthy_flag(payload, "preview");
    let max_attempts = dashboard_payload_usize(payload, "max_attempts", 6, 1, 20) as i64;
    let now_epoch = dashboard_troubleshooting_now_epoch_seconds();
    let mut items = dashboard_troubleshooting_read_issue_outbox(root);
    if dry_run {
        let mut ready = Vec::<Value>::new();
        let mut cooldown_blocked = Vec::<Value>::new();
        let mut max_attempt_blocked = Vec::<Value>::new();
        for row in items.iter() {
            if ready.len() + cooldown_blocked.len() + max_attempt_blocked.len() >= max_items {
                break;
            }
            let attempts = row.get("attempts").and_then(Value::as_i64).unwrap_or(0);
            if attempts >= max_attempts {
                max_attempt_blocked.push(json!({
                    "item_id": clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 80),
                    "attempts": attempts,
                    "status": "would_quarantine_max_attempts"
                }));
                continue;
            }
            let next_retry_epoch = row
                .get("next_retry_after_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            if !force && next_retry_epoch > now_epoch {
                cooldown_blocked.push(json!({
                    "item_id": clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 80),
                    "next_retry_after_epoch_s": next_retry_epoch,
                    "next_retry_after_seconds": next_retry_epoch - now_epoch,
                    "status": "cooldown_blocked"
                }));
                continue;
            }
            ready.push(json!({
                "item_id": clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 80),
                "attempts": attempts,
                "status": "ready"
            }));
        }
        return LaneResult {
            ok: true,
            status: 0,
            argv: vec!["dashboard.troubleshooting.outbox.flush".to_string()],
            payload: Some(json!({
                "ok": true,
                "type": "dashboard_troubleshooting_outbox_flush_preview",
                "dry_run": true,
                "force": force,
                "max_attempts": max_attempts,
                "ready_count": ready.len(),
                "cooldown_blocked_count": cooldown_blocked.len(),
                "max_attempt_blocked_count": max_attempt_blocked.len(),
                "remaining_depth": items.len(),
                "error_histogram": dashboard_troubleshooting_outbox_reason_histogram(&items),
                "ready": ready,
                "cooldown_blocked": cooldown_blocked,
                "max_attempt_blocked": max_attempt_blocked
            })),
        };
    }
    let mut remaining = Vec::<Value>::new();
    let mut submitted = Vec::<Value>::new();
    let mut failed = Vec::<Value>::new();
    let mut quarantined = Vec::<Value>::new();
    let mut skipped_due_cooldown = 0usize;
    let mut auth_blocked_count = 0usize;
    let mut transport_failed_count = 0usize;
    let mut validation_failed_count = 0usize;
    let mut failed_error_counts = HashMap::<String, i64>::new();
    for row in items.drain(..) {
        if submitted.len() + failed.len() >= max_items {
            remaining.push(row);
            continue;
        }
        let attempts = row.get("attempts").and_then(Value::as_i64).unwrap_or(0);
        if attempts >= max_attempts {
            let quarantine_row = json!({
                "item_id": clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 80),
                "attempts": attempts,
                "status": "quarantined_max_attempts"
            });
            let deadletter_inserted = dashboard_troubleshooting_append_deadletter_row(
                root,
                &json!({
                    "type": "dashboard_troubleshooting_issue_deadletter",
                    "ts": now_iso(),
                    "reason": "quarantined_max_attempts",
                    "row": row.clone()
                }),
            );
            *failed_error_counts
                .entry("quarantined_max_attempts".to_string())
                .or_insert(0) += 1;
            if let Some(obj) = quarantine_row.as_object_mut() {
                obj.insert("deadletter_inserted".to_string(), json!(deadletter_inserted));
            }
            quarantined.push(quarantine_row);
            continue;
        }
        let next_retry_epoch = row
            .get("next_retry_after_epoch_s")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        if !force && next_retry_epoch > now_epoch {
            skipped_due_cooldown += 1;
            remaining.push(row);
            continue;
        }
        let request = row
            .get("issue_request")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let lane = run_action(root, "dashboard.github.issue.create", &request);
        if lane.ok {
            submitted.push(json!({
                "item_id": clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 80),
                "issue": lane.payload.clone().unwrap_or_else(|| json!({}))
            }));
            dashboard_troubleshooting_clear_active_context(root, "issue_outbox_submission_succeeded");
            continue;
        }
        let issue_error = clean_text(
            lane.payload
                .as_ref()
                .and_then(|value| value.get("error"))
                .and_then(Value::as_str)
                .unwrap_or("github_issue_transport_error"),
            120,
        )
        .to_ascii_lowercase();
        *failed_error_counts.entry(issue_error.clone()).or_insert(0) += 1;
        let (error_bucket, retry_lane, retry_after_seconds) = if issue_error == "github_issue_auth_missing" {
            auth_blocked_count += 1;
            ("auth_missing", "auth_required", 3600)
        } else if issue_error.starts_with("github_issue_http_4") {
            validation_failed_count += 1;
            ("http_client", "request_fix_required", 900)
        } else if issue_error.starts_with("github_issue_http_5") {
            transport_failed_count += 1;
            ("http_server", "transport_retry", (attempts + 1) * 45)
        } else if issue_error == "github_issue_transport_error" {
            transport_failed_count += 1;
            ("transport", "transport_retry", (attempts + 1) * 45)
        } else {
            validation_failed_count += 1;
            ("other", "request_fix_required", 600)
        };
        let mut updated = row.clone();
        if let Some(obj) = updated.as_object_mut() {
            let attempts = obj.get("attempts").and_then(Value::as_i64).unwrap_or(0) + 1;
            let retry_after_seconds = retry_after_seconds.clamp(30, 3600);
            obj.insert("attempts".to_string(), json!(attempts));
            obj.insert("last_attempt_at".to_string(), json!(now_iso()));
            obj.insert(
                "retry_after_seconds".to_string(),
                json!(retry_after_seconds),
            );
            obj.insert(
                "next_retry_after_epoch_s".to_string(),
                json!(now_epoch + retry_after_seconds),
            );
            obj.insert(
                "last_error".to_string(),
                lane.payload.clone().unwrap_or_else(|| json!({})),
            );
            obj.insert("error_bucket".to_string(), json!(error_bucket));
            obj.insert("retry_lane".to_string(), json!(retry_lane));
        }
        failed.push(updated.clone());
        remaining.push(updated);
    }
    dashboard_troubleshooting_write_issue_outbox(root, &remaining);
    let next_retry_after_epoch_s = remaining
        .iter()
        .filter_map(|row| row.get("next_retry_after_epoch_s").and_then(Value::as_i64))
        .filter(|epoch| *epoch > now_epoch)
        .min()
        .unwrap_or(0);
    let next_retry_after_seconds = if next_retry_after_epoch_s > now_epoch {
        next_retry_after_epoch_s - now_epoch
    } else {
        0
    };
    let deadletter_depth = dashboard_troubleshooting_read_deadletter_rows(root, usize::MAX).len();
    LaneResult {
        ok: true,
        status: 0,
        argv: vec!["dashboard.troubleshooting.outbox.flush".to_string()],
        payload: Some(json!({
            "ok": true,
            "type": "dashboard_troubleshooting_outbox_flush",
            "force": force,
            "max_attempts": max_attempts,
            "skipped_due_cooldown_count": skipped_due_cooldown,
            "auth_blocked_count": auth_blocked_count,
            "transport_failed_count": transport_failed_count,
            "validation_failed_count": validation_failed_count,
            "submitted_count": submitted.len(),
            "failed_count": failed.len(),
            "quarantined_count": quarantined.len(),
            "remaining_depth": remaining.len(),
            "deadletter_depth": deadletter_depth,
            "next_retry_after_epoch_s": next_retry_after_epoch_s,
            "next_retry_after_seconds": next_retry_after_seconds,
            "failed_error_histogram": dashboard_troubleshooting_sorted_histogram(failed_error_counts, "error"),
            "submitted": submitted,
            "failed": failed,
            "quarantined": quarantined
        })),
    }
}

fn dashboard_troubleshooting_deadletter_state_lane(root: &Path, payload: &Value) -> LaneResult {
    let limit = dashboard_payload_usize(payload, "limit", 20, 1, 200);
    let reason_filter = clean_text(
        payload.get("reason").and_then(Value::as_str).unwrap_or(""),
        120,
    )
    .to_ascii_lowercase();
    let all_rows = dashboard_troubleshooting_read_deadletter_all(root);
    let mut rows = all_rows.clone();
    if !reason_filter.is_empty() {
        rows.retain(|row| dashboard_troubleshooting_deadletter_reason(row) == reason_filter);
    }
    if rows.len() > limit {
        rows = rows.split_off(rows.len().saturating_sub(limit));
    }
    let oldest_ts = all_rows
        .first()
        .and_then(|row| row.get("ts").and_then(Value::as_str))
        .map(|raw| clean_text(raw, 80))
        .unwrap_or_default();
    let latest_ts = all_rows
        .last()
        .and_then(|row| row.get("ts").and_then(Value::as_str))
        .map(|raw| clean_text(raw, 80))
        .unwrap_or_default();
    LaneResult {
        ok: true,
        status: 0,
        argv: vec!["dashboard.troubleshooting.deadletter.state".to_string()],
        payload: Some(json!({
            "ok": true,
            "type": "dashboard_troubleshooting_deadletter_state",
            "reason_filter": reason_filter,
            "depth": all_rows.len(),
            "filtered_depth": rows.len(),
            "oldest_ts": oldest_ts,
            "latest_ts": latest_ts,
            "reason_histogram": dashboard_troubleshooting_reason_histogram(&all_rows),
            "items": rows
        })),
    }
}

fn dashboard_troubleshooting_deadletter_requeue_lane(root: &Path, payload: &Value) -> LaneResult {
    let max_items = dashboard_payload_usize(payload, "max_items", 10, 1, 100);
    let mut deadletter = dashboard_troubleshooting_read_deadletter_all(root);
    if deadletter.is_empty() {
        return LaneResult {
            ok: true,
            status: 0,
            argv: vec!["dashboard.troubleshooting.deadletter.requeue".to_string()],
            payload: Some(json!({
                "ok": true,
                "type": "dashboard_troubleshooting_deadletter_requeue",
                "requeued_count": 0,
                "deadletter_depth_after": 0,
                "outbox_depth_after": dashboard_troubleshooting_read_issue_outbox(root).len()
            })),
        };
    }
    let mut outbox = dashboard_troubleshooting_read_issue_outbox(root);
    let outbox_capacity = DASHBOARD_TROUBLESHOOTING_MAX_OUTBOX.saturating_sub(outbox.len());
    let target = max_items.min(outbox_capacity);
    let dry_run = dashboard_payload_truthy_flag(payload, "dry_run")
        || dashboard_payload_truthy_flag(payload, "preview_only")
        || dashboard_payload_truthy_flag(payload, "preview");
    let selected_ids = dashboard_payload_string_list(payload, "item_ids", 100, 120);
    let selected_filter_applied = !selected_ids.is_empty();
    let mut outbox_signatures = outbox
        .iter()
        .map(dashboard_troubleshooting_issue_request_signature)
        .collect::<Vec<_>>();
    let mut requeued = Vec::<Value>::new();
    let mut keep = Vec::<Value>::new();
    let mut skipped_not_selected = 0usize;
    let mut skipped_duplicate = 0usize;
    if dry_run {
        let mut would_requeue = 0usize;
        let mut would_skip_duplicate = 0usize;
        let mut would_skip_not_selected = 0usize;
        for row in deadletter.iter().rev() {
            let issue_id = clean_text(
                row.pointer("/row/id").and_then(Value::as_str).unwrap_or(""),
                80,
            );
            if selected_filter_applied && !selected_ids.iter().any(|id| id == &issue_id) {
                would_skip_not_selected += 1;
                continue;
            }
            if let Some(issue_row) = row.get("row").cloned() {
                let issue_signature = dashboard_troubleshooting_issue_request_signature(&issue_row);
                if !issue_signature.is_empty()
                    && outbox_signatures.iter().any(|existing| existing == &issue_signature)
                {
                    would_skip_duplicate += 1;
                    continue;
                }
                if would_requeue < target {
                    would_requeue += 1;
                }
            }
        }
        return LaneResult {
            ok: true,
            status: 0,
            argv: vec!["dashboard.troubleshooting.deadletter.requeue".to_string()],
            payload: Some(json!({
                "ok": true,
                "type": "dashboard_troubleshooting_deadletter_requeue_preview",
                "dry_run": true,
                "selected_filter_applied": selected_filter_applied,
                "selected_item_count": selected_ids.len(),
                "target_capacity": target,
                "would_requeue_count": would_requeue,
                "would_skip_not_selected_count": would_skip_not_selected,
                "would_skip_duplicate_count": would_skip_duplicate,
                "deadletter_depth": deadletter.len(),
                "outbox_depth": outbox.len()
            })),
        };
    }

    for row in deadletter.into_iter().rev() {
        let issue_id = clean_text(
            row.pointer("/row/id").and_then(Value::as_str).unwrap_or(""),
            80,
        );
        if selected_filter_applied && !selected_ids.iter().any(|id| id == &issue_id) {
            skipped_not_selected += 1;
            keep.push(row);
            continue;
        }
        if requeued.len() < target {
            if let Some(issue_row) = row.get("row").cloned() {
                let issue_signature = dashboard_troubleshooting_issue_request_signature(&issue_row);
                if !issue_signature.is_empty()
                    && outbox_signatures.iter().any(|existing| existing == &issue_signature)
                {
                    skipped_duplicate += 1;
                    keep.push(row);
                    continue;
                }
                let mut restored = issue_row;
                if let Some(obj) = restored.as_object_mut() {
                    obj.insert("attempts".to_string(), json!(0));
                    obj.insert("retry_after_seconds".to_string(), json!(0));
                    obj.insert("next_retry_after_epoch_s".to_string(), json!(0));
                    obj.insert("last_requeued_at".to_string(), json!(now_iso()));
                }
                outbox.push(restored.clone());
                if !issue_signature.is_empty() {
                    outbox_signatures.push(issue_signature);
                }
                requeued.push(json!({
                    "item_id": clean_text(
                        restored.get("id").and_then(Value::as_str).unwrap_or(""),
                        80
                    ),
                    "status": "requeued"
                }));
                continue;
            }
        }
        keep.push(row);
    }

    keep.reverse();
    dashboard_troubleshooting_write_deadletter_rows(root, &keep);
    dashboard_troubleshooting_write_issue_outbox(root, &outbox);

    LaneResult {
        ok: true,
        status: 0,
        argv: vec!["dashboard.troubleshooting.deadletter.requeue".to_string()],
        payload: Some(json!({
            "ok": true,
            "type": "dashboard_troubleshooting_deadletter_requeue",
            "selected_filter_applied": selected_filter_applied,
            "selected_item_count": selected_ids.len(),
            "skipped_not_selected_count": skipped_not_selected,
            "skipped_duplicate_count": skipped_duplicate,
            "requeued_count": requeued.len(),
            "deadletter_depth_after": keep.len(),
            "outbox_depth_after": outbox.len(),
            "deadletter_reason_histogram_after": dashboard_troubleshooting_reason_histogram(&keep),
            "outbox_error_histogram_after": dashboard_troubleshooting_outbox_reason_histogram(&outbox),
            "requeued": requeued
        })),
    }
}

fn dashboard_troubleshooting_deadletter_purge_lane(root: &Path, payload: &Value) -> LaneResult {
    let remove_all = dashboard_payload_truthy_flag(payload, "all");
    let reason_filter = clean_text(
        payload.get("reason").and_then(Value::as_str).unwrap_or(""),
        120,
    )
    .to_ascii_lowercase();
    let selected_ids = dashboard_payload_string_list(payload, "item_ids", 300, 120);
    let dry_run = dashboard_payload_truthy_flag(payload, "dry_run")
        || dashboard_payload_truthy_flag(payload, "preview_only")
        || dashboard_payload_truthy_flag(payload, "preview");
    if !remove_all && reason_filter.is_empty() && selected_ids.is_empty() {
        return LaneResult {
            ok: true,
            status: 0,
            argv: vec!["dashboard.troubleshooting.deadletter.purge".to_string()],
            payload: Some(json!({
                "ok": false,
                "type": "dashboard_troubleshooting_deadletter_purge",
                "error": "deadletter_purge_selector_required",
                "summary": "Specify all=true, reason, or item_ids for deadletter purge."
            })),
        };
    }
    let mut rows = dashboard_troubleshooting_read_deadletter_all(root);
    let mut removed = Vec::<Value>::new();
    let mut keep = Vec::<Value>::new();
    for row in rows.drain(..) {
        let issue_id = clean_text(
            row.pointer("/row/id").and_then(Value::as_str).unwrap_or(""),
            80,
        );
        let row_reason = dashboard_troubleshooting_deadletter_reason(&row);
        let reason_match = !reason_filter.is_empty() && row_reason == reason_filter;
        let id_match = !selected_ids.is_empty() && selected_ids.iter().any(|id| id == &issue_id);
        let should_remove = remove_all || reason_match || id_match;
        if should_remove {
            removed.push(row);
        } else {
            keep.push(row);
        }
    }
    if dry_run {
        return LaneResult {
            ok: true,
            status: 0,
            argv: vec!["dashboard.troubleshooting.deadletter.purge".to_string()],
            payload: Some(json!({
                "ok": true,
                "type": "dashboard_troubleshooting_deadletter_purge_preview",
                "dry_run": true,
                "all": remove_all,
                "reason_filter": reason_filter,
                "selected_item_count": selected_ids.len(),
                "removed_count": removed.len(),
                "remaining_depth": keep.len(),
                "removed_reason_histogram": dashboard_troubleshooting_reason_histogram(&removed),
                "remaining_reason_histogram": dashboard_troubleshooting_reason_histogram(&keep)
            })),
        };
    }
    dashboard_troubleshooting_write_deadletter_rows(root, &keep);
    LaneResult {
        ok: true,
        status: 0,
        argv: vec!["dashboard.troubleshooting.deadletter.purge".to_string()],
        payload: Some(json!({
            "ok": true,
            "type": "dashboard_troubleshooting_deadletter_purge",
            "all": remove_all,
            "reason_filter": reason_filter,
            "selected_item_count": selected_ids.len(),
            "removed_count": removed.len(),
            "remaining_depth": keep.len(),
            "removed_reason_histogram": dashboard_troubleshooting_reason_histogram(&removed),
            "remaining_reason_histogram": dashboard_troubleshooting_reason_histogram(&keep)
        })),
    }
}

fn dashboard_troubleshooting_report_message_lane(root: &Path, payload: &Value) -> LaneResult {
    let (eval_model, _) = dashboard_troubleshooting_resolve_eval_model(Some(payload));
    let snapshot = dashboard_troubleshooting_capture_snapshot(
        root,
        "user_report",
        &json!({
            "source": clean_text(payload.get("source").and_then(Value::as_str).unwrap_or("dashboard_report_message"), 80),
            "session_id": clean_text(payload.get("session_id").or_else(|| payload.get("sessionId")).and_then(Value::as_str).unwrap_or(""), 160),
            "message_id": clean_text(payload.get("message_id").or_else(|| payload.get("messageId")).and_then(Value::as_str).unwrap_or(""), 160)
        }),
    );
    let queue_item = dashboard_troubleshooting_enqueue_eval(
        root,
        &snapshot,
        "user_report",
        Some(&eval_model),
    );
    let eval_drain = dashboard_troubleshooting_eval_drain_internal(root, 1, "user_report");
    let eval_report = eval_drain
        .get("reports")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .cloned()
        .or_else(|| read_json_file(&root.join(DASHBOARD_TROUBLESHOOTING_EVAL_LATEST_REL)))
        .unwrap_or_else(|| json!({}));
    let issue_request = dashboard_troubleshooting_issue_request_from_report(payload, &snapshot, &eval_report);
    let issue_lane = run_action(root, "dashboard.github.issue.create", &issue_request);
    if issue_lane.ok {
        dashboard_troubleshooting_clear_active_context(root, "issue_submission_succeeded");
        return LaneResult {
            ok: true,
            status: 0,
            argv: vec!["dashboard.troubleshooting.report_message".to_string()],
            payload: Some(json!({
                "ok": true,
                "type": "dashboard_troubleshooting_report",
                "submitted": true,
                "queued": false,
                "snapshot_id": snapshot.get("snapshot_id").cloned().unwrap_or(Value::Null),
                "eval_report_id": eval_report.get("report_id").cloned().unwrap_or(Value::Null),
                "issue": issue_lane.payload.unwrap_or_else(|| json!({}))
            })),
        };
    }
    let outbox_item =
        dashboard_troubleshooting_enqueue_outbox(root, &issue_request, &issue_lane, &snapshot, &eval_report);
    let issue_error = issue_lane
        .payload
        .as_ref()
        .and_then(|row| row.get("error"))
        .and_then(Value::as_str)
        .unwrap_or("github_issue_transport_error")
        .to_string();
    let issue_error_hint = if issue_error == "github_issue_auth_missing" {
        "no github auth token, please input your token first"
    } else {
        "Issue pipeline queued locally; run dashboard.troubleshooting.outbox.flush after auth/pipeline recovery."
    };
    LaneResult {
        ok: true,
        status: 0,
        argv: vec!["dashboard.troubleshooting.report_message".to_string()],
        payload: Some(json!({
            "ok": true,
            "type": "dashboard_troubleshooting_report",
            "submitted": false,
            "queued": true,
            "snapshot_id": snapshot.get("snapshot_id").cloned().unwrap_or(Value::Null),
            "eval_report_id": eval_report.get("report_id").cloned().unwrap_or(Value::Null),
            "queue_item": queue_item,
            "eval_drain": eval_drain,
            "issue_error": issue_error,
            "issue_error_hint": issue_error_hint,
            "outbox_item": outbox_item
        })),
    }
}
