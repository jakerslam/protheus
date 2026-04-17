include!("autotest_controller_parts/010-parse-cli.rs");
include!("autotest_controller_parts/020-runtime-paths.rs");
include!("autotest_controller_parts/030-score-module-test-pair.rs");
include!("autotest_controller_parts/040-is-spine-hot.rs");

pub fn web_tooling_runtime_autotest(root: &std::path::Path, strict: bool) -> serde_json::Value {
    let report = crate::network_protocol::web_tooling_health_report(root, strict);
    let auth_present = report
        .get("auth_present")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let policy_ready = report
        .get("policy_ready")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let passed = auth_present && policy_ready;
    serde_json::json!({
        "ok": if strict { passed } else { true },
        "strict": strict,
        "type": "autotest_web_tooling_runtime_gate",
        "status": if passed { "pass" } else if strict { "fail" } else { "warn" },
        "auth_present": auth_present,
        "policy_ready": policy_ready,
        "errors": report.get("errors").cloned().unwrap_or_else(|| serde_json::json!([])),
        "health_report": report,
        "claim_evidence": [
            {
                "id": "V8-AUTOTEST-003.1",
                "claim": "autotest_controller_surfaces_web_tooling_runtime_gate_before_execution",
                "evidence": {
                    "auth_present": auth_present,
                    "policy_ready": policy_ready
                }
            }
        ]
    })
}
