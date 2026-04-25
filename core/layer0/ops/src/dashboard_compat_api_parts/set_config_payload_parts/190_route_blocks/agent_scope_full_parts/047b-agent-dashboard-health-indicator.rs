// SRS: V12-MISTY-HEALTH-WAVE5-001

fn health_bucket(status: &str, reason: &str) -> Value {
    json!({
        "status": status,
        "reason": clean_text(reason, 240)
    })
}

fn health_bucket_status(degraded: bool) -> &'static str {
    if degraded {
        "degraded"
    } else {
        "healthy"
    }
}

fn invariant_degraded(value: &Value) -> bool {
    matches!(
        value
            .get("classification")
            .and_then(Value::as_str)
            .unwrap_or(""),
        "failed" | "policy_blocked" | "low_signal"
    ) || value
        .get("tool_blocked")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || value
            .get("low_signal")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        || !value
            .get("failure_code")
            .and_then(Value::as_str)
            .unwrap_or("")
            .is_empty()
}

fn agent_dashboard_health_indicator(
    response_workflow: &Value,
    response_finalization: &Value,
    process_summary: &Value,
    live_eval_monitor: &Value,
) -> Value {
    let final_llm_status = clean_text(
        response_workflow
            .pointer("/final_llm_response/status")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    let finalization_outcome = clean_text(
        response_finalization
            .get("outcome")
            .and_then(Value::as_str)
            .unwrap_or(""),
        240,
    );
    let workflow_degraded = response_workflow_quality_count(response_workflow, "repeated_fallback_loop_detected") > 0
        || response_workflow_quality_count(response_workflow, "current_turn_dominance_reject") > 0
        || response_workflow_quality_count(response_workflow, "contamination_reject") > 0;
    let model_degraded = matches!(final_llm_status.as_str(), "invoke_failed" | "unavailable" | "skipped")
        || response_finalization
            .get("visible_response_source")
            .and_then(Value::as_str)
            .unwrap_or("none")
            == "none";
    let tool_degraded = invariant_degraded(
        response_finalization
            .get("tooling_invariant")
            .unwrap_or(&Value::Null),
    ) || invariant_degraded(response_finalization.get("web_invariant").unwrap_or(&Value::Null));
    let finalization_degraded = finalization_outcome.contains("withheld")
        || finalization_outcome.contains("fallback")
        || finalization_outcome.contains("contamination")
        || response_finalization
            .pointer("/contamination_guard/detected")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        || response_finalization
            .get("system_chat_injection_used")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let telemetry_disabled = !live_eval_monitor
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let telemetry_degraded = live_eval_monitor
        .get("issue_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0
        || live_eval_monitor
            .get("chat_injection_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let telemetry_status = if telemetry_degraded {
        "degraded"
    } else if telemetry_disabled {
        "disabled"
    } else {
        "healthy"
    };
    let buckets = json!({
        "workflow": health_bucket(health_bucket_status(workflow_degraded), "workflow quality counters and fallback-loop telemetry"),
        "model": health_bucket(health_bucket_status(model_degraded), "final LLM status and visible response provenance"),
        "tool": health_bucket(health_bucket_status(tool_degraded), "tooling and web invariant classifications"),
        "finalization": health_bucket(health_bucket_status(finalization_degraded), "visible response finalization guard state"),
        "telemetry": health_bucket(telemetry_status, "live eval monitor issues and chat-injection policy")
    });
    let degraded_buckets = buckets
        .as_object()
        .into_iter()
        .flat_map(|map| map.iter())
        .filter_map(|(name, bucket)| {
            (bucket.get("status").and_then(Value::as_str) == Some("degraded"))
                .then(|| name.clone())
        })
        .collect::<Vec<_>>();
    json!({
        "contract": "agent_dashboard_health_indicator_v1",
        "overall": if degraded_buckets.is_empty() { "healthy" } else { "degraded" },
        "buckets": buckets,
        "degraded_buckets": degraded_buckets,
        "diagnostics_only": true,
        "chat_injection_allowed": false,
        "process_contract": process_summary.get("contract").cloned().unwrap_or(Value::Null)
    })
}
