include!("autotest_doctor_parts/010-parse-cli.rs");
include!("autotest_doctor_parts/020-load-policy.rs");
include!("autotest_doctor_parts/030-doctor-runtime.rs");

pub fn web_tooling_repair_plan(root: &std::path::Path) -> serde_json::Value {
    let report = crate::network_protocol::web_tooling_health_report(root, false);
    let missing_policy = report
        .get("missing_policy_codes")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let auth_present = report
        .get("auth_present")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let mut recommended = Vec::new();
    if !auth_present {
        recommended.push(serde_json::json!({
            "id": "web_tool_auth",
            "priority": "high",
            "command": "export WEB_SEARCH_API_KEY=<token>",
            "reason": "network_web_tool_auth_missing"
        }));
    }
    if !missing_policy.is_empty() {
        recommended.push(serde_json::json!({
            "id": "web_tool_policy",
            "priority": "high",
            "command": "protheus-ops directive-kernel prime-sign --directive=allow:web-search --signer=operator",
            "reason": "directive_allow_web_search_missing_or_related"
        }));
        recommended.push(serde_json::json!({
            "id": "web_tool_policy_fetch",
            "priority": "high",
            "command": "protheus-ops directive-kernel prime-sign --directive=allow:web-fetch --signer=operator",
            "reason": "directive_allow_web_fetch_missing_or_related"
        }));
    }
    recommended.push(serde_json::json!({
        "id": "runtime_refresh",
        "priority": "medium",
        "command": "protheus-ops network-protocol web-tooling-status --activate=1 --strict=1",
        "reason": "refresh_receipted_runtime_state"
    }));

    serde_json::json!({
        "ok": true,
        "type": "autotest_doctor_web_tooling_plan",
        "auth_present": auth_present,
        "missing_policy_codes": missing_policy,
        "recommended_actions": recommended,
        "health_report": report
    })
}
