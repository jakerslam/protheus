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

fn dashboard_troubleshooting_recent_lane(row: &Value) -> &'static str {
    let classification = clean_text(
        row.pointer("/workflow/classification")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    )
    .to_ascii_lowercase();
    let error_code = clean_text(
        row.pointer("/workflow/error_code")
            .and_then(Value::as_str)
            .unwrap_or(""),
        160,
    )
    .to_ascii_lowercase();
    let stale = row.get("stale").and_then(Value::as_bool).unwrap_or(false);
    if classification.contains("context")
        || classification.contains("halluc")
        || error_code.contains("context")
        || error_code.contains("halluc")
    {
        return "continuity";
    }
    if stale || classification.contains("stale") || error_code.contains("stale") {
        return "liveness";
    }
    if classification.contains("lifecycle")
        || error_code.starts_with("agent_")
        || error_code.starts_with("gateway_")
    {
        return "lifecycle";
    }
    if classification.contains("tool")
        || classification.contains("provider")
        || error_code.starts_with("web_")
        || error_code.contains("provider")
    {
        return "tool_completion";
    }
    "e2e"
}

fn dashboard_troubleshooting_recent_lane_health(rows: &[Value]) -> Value {
    let lanes = ["continuity", "tool_completion", "liveness", "lifecycle", "e2e"];
    let mut totals = HashMap::<&'static str, i64>::new();
    let mut failed = HashMap::<&'static str, i64>::new();
    for lane in lanes {
        totals.insert(lane, 0);
        failed.insert(lane, 0);
    }
    for row in rows {
        let lane = dashboard_troubleshooting_recent_lane(row);
        *totals.entry(lane).or_insert(0) += 1;
        if dashboard_troubleshooting_exchange_failed(row) {
            *failed.entry(lane).or_insert(0) += 1;
        }
    }
    let mut out = serde_json::Map::<String, Value>::new();
    for lane in lanes {
        let total = totals.get(lane).copied().unwrap_or(0).max(0);
        let failed_count = failed.get(lane).copied().unwrap_or(0).max(0);
        let passed = total.saturating_sub(failed_count);
        out.insert(
            lane.to_string(),
            json!({
                "total": total,
                "failed": failed_count,
                "passed": passed,
                "ok": failed_count == 0
            }),
        );
    }
    Value::Object(out)
}

fn dashboard_troubleshooting_recent_recovery_hints(
    lane_health: &Value,
    severity_tier: &str,
) -> Vec<Value> {
    let lane_failed = |lane: &str| -> bool {
        lane_health
            .pointer(&format!("/{lane}/failed"))
            .and_then(Value::as_i64)
            .unwrap_or(0)
            > 0
    };
    let mut hints = Vec::<Value>::new();
    if lane_failed("continuity") {
        hints.push(Value::String(
            "Continuity lane degraded: tighten recent-context compatibility checks before synthesis."
                .to_string(),
        ));
    }
    if lane_failed("tool_completion") {
        hints.push(Value::String(
            "Tool completion lane degraded: verify web tool request/response contracts and retry reasons."
                .to_string(),
        ));
    }
    if lane_failed("liveness") {
        hints.push(Value::String(
            "Liveness lane degraded: inspect stale/freshness contract propagation and roster visibility."
                .to_string(),
        ));
    }
    if lane_failed("lifecycle") {
        hints.push(Value::String(
            "Lifecycle lane degraded: re-check agent permission and terminal policy gates.".to_string(),
        ));
    }
    if lane_failed("e2e") {
        hints.push(Value::String(
            "E2E lane degraded: inspect recent exchange traces and replay fail-closed paths."
                .to_string(),
        ));
    }
    if severity_tier == "high" {
        hints.push(Value::String(
            "High severity cluster: capture snapshot + enqueue eval before next retry wave."
                .to_string(),
        ));
    }
    if hints.is_empty() {
        hints.push(Value::String(
            "Recent troubleshooting health is stable: no failing lanes detected.".to_string(),
        ));
    }
    hints
}

fn dashboard_troubleshooting_recent_health_checks(
    lane_health: &Value,
    latest_loop_level: &str,
    entry_count: usize,
    filtered_out_count: usize,
    total_entry_count: usize,
    stale_rate: f64,
    queue_pressure_tier: &str,
    tooling_gate_ok: bool,
    provider_resolution_ok: bool,
    tooling_watchdog_not_triggered: bool,
    tooling_completion_signal_ok: bool,
    tooling_manual_intervention_not_required: bool,
    tooling_contract_version_supported: bool,
    tooling_next_action_routable: bool,
    tooling_llm_reliability_not_low: bool,
    tooling_hallucination_pattern_not_detected: bool,
    tooling_placeholder_output_not_detected: bool,
    tooling_final_response_contract_ok: bool,
    tooling_no_result_pattern_not_detected: bool,
    tooling_answer_contract_ok: bool,
    response_gate: &Value,
) -> Value {
    let lanes = ["continuity", "tool_completion", "liveness", "lifecycle", "e2e"];
    let lane_health_ok = lanes.iter().all(|lane| {
        lane_health
            .pointer(&format!("/{lane}/ok"))
            .and_then(Value::as_bool)
            .unwrap_or(true)
    });
    let window_consistent = entry_count.saturating_add(filtered_out_count) == total_entry_count;
    let mut checks = json!({
        "lane_health_ok": lane_health_ok,
        "critical_loop_absent": clean_text(latest_loop_level, 40) != "critical",
        "window_consistent": window_consistent,
        "stale_rate_ok": stale_rate <= 0.6,
        "queue_pressure_not_high": clean_text(queue_pressure_tier, 40) != "high",
        "tooling_gate_ok": tooling_gate_ok,
        "provider_resolution_ok": provider_resolution_ok,
        "tooling_watchdog_not_triggered": tooling_watchdog_not_triggered,
        "tooling_completion_signal_ok": tooling_completion_signal_ok,
        "tooling_manual_intervention_not_required": tooling_manual_intervention_not_required,
        "tooling_contract_version_supported": tooling_contract_version_supported,
        "tooling_next_action_routable": tooling_next_action_routable,
        "tooling_llm_reliability_not_low": tooling_llm_reliability_not_low,
        "tooling_hallucination_pattern_not_detected": tooling_hallucination_pattern_not_detected,
        "tooling_placeholder_output_not_detected": tooling_placeholder_output_not_detected,
        "tooling_final_response_contract_ok": tooling_final_response_contract_ok,
        "tooling_no_result_pattern_not_detected": tooling_no_result_pattern_not_detected,
        "tooling_answer_contract_ok": tooling_answer_contract_ok
    });
    if let Some(dest) = checks.as_object_mut() {
        if let Some(src) = dashboard_response_gate_checks(response_gate).as_object() {
            for (key, value) in src {
                dest.insert(key.clone(), value.clone());
            }
        }
    }
    checks
}
