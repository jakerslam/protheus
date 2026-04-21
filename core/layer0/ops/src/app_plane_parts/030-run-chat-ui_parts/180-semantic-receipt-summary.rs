fn chat_ui_semantic_receipt_summary(
    diagnostics: &Value,
    requires_live_web: bool,
    message: &str,
) -> String {
    let total_calls = diagnostics
        .get("total_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let successful_calls = diagnostics
        .get("successful_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let failed_calls = diagnostics
        .get("failed_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let blocked_calls = diagnostics
        .get("blocked_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let not_found_calls = diagnostics
        .get("not_found_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let low_signal_calls = diagnostics
        .get("low_signal_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let silent_failure_calls = diagnostics
        .get("silent_failure_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let surface_unavailable_calls = diagnostics
        .get("surface_unavailable_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let surface_degraded_calls = diagnostics
        .get("surface_degraded_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let error_codes = diagnostics
        .get("error_codes")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let has_surface_unavailable =
        surface_unavailable_calls > 0 || error_codes.contains_key("web_tool_surface_unavailable");
    let has_surface_degraded =
        surface_degraded_calls > 0 || error_codes.contains_key("web_tool_surface_degraded");
    let gate_blocked_calls = diagnostics
        .pointer("/error_codes/workflow_gate_blocked_web_tooling")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let status = if requires_live_web && has_surface_unavailable {
        "failed"
    } else if requires_live_web && has_surface_degraded {
        "degraded"
    } else if gate_blocked_calls > 0 {
        "blocked"
    } else if requires_live_web && total_calls <= 0 {
        "failed"
    } else if failed_calls == 0 && silent_failure_calls == 0 {
        "complete"
    } else if successful_calls > 0 {
        "degraded"
    } else {
        "failed"
    };
    let intent = if requires_live_web {
        chat_ui_extract_web_query(message)
    } else {
        clean(message, 140)
    };
    clean(
        &format!(
            "Tool transaction {} for intent \"{}\": total={} success={} failed={} blocked={} gate_blocked={} not_found={} low_signal={} surface_unavailable={} surface_degraded={} silent_failure={}.",
            status,
            intent,
            total_calls,
            successful_calls,
            failed_calls,
            blocked_calls,
            gate_blocked_calls,
            not_found_calls,
            low_signal_calls,
            surface_unavailable_calls,
            surface_degraded_calls,
            silent_failure_calls
        ),
        600,
    )
}
