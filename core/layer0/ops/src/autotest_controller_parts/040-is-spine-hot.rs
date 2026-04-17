// AUTO-FLATTENED wrapper: preserves ordered include content.
include!("040-is-spine-hot.combined.rs");

pub(super) fn web_tooling_spine_snapshot(root: &std::path::Path) -> serde_json::Value {
    let report = crate::network_protocol::web_tooling_health_report(root, false);
    let auth_present = report
        .get("auth_present")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let policy_ready = report
        .get("policy_ready")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    serde_json::json!({
        "ok": auth_present && policy_ready,
        "type": "autotest_web_tooling_spine_snapshot",
        "auth_present": auth_present,
        "policy_ready": policy_ready,
        "errors": report.get("errors").cloned().unwrap_or_else(|| serde_json::json!([])),
        "missing_policy_codes": report
            .get("missing_policy_codes")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([]))
    })
}
