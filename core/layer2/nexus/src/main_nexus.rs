include!("main_nexus_parts/010-types.rs");
include!("main_nexus_parts/020-control.rs");
include!("main_nexus_parts/030-delivery.rs");
include!("main_nexus_parts/040-internals.rs");

pub fn synthesize_web_tooling_runtime_diagnostic(
    network_health: &serde_json::Value,
    execution_signal: &serde_json::Value,
) -> serde_json::Value {
    let auth_present = network_health
        .get("auth_present")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let policy_ready = network_health
        .get("policy_ready")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let signal_class = execution_signal
        .get("payload")
        .and_then(|row| row.get("classification"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let critical = !auth_present
        || !policy_ready
        || matches!(
            signal_class.as_str(),
            "function_execution_blocked" | "auth_missing"
        );
    serde_json::json!({
        "ok": true,
        "type": "nexus_web_tooling_runtime_diagnostic",
        "severity": if critical { "critical" } else { "elevated" },
        "auth_present": auth_present,
        "policy_ready": policy_ready,
        "signal_classification": signal_class,
        "summary": if critical {
            "web tooling not yet reliable for end-to-end synthesis"
        } else {
            "web tooling lane healthy enough for synthesis retries"
        },
        "next_steps": [
            "refresh network protocol web tooling runtime snapshot",
            "retry with one provider + one source query to validate signal quality"
        ]
    })
}
