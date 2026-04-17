include!("network_protocol_run_parts/010-contribution-history-path.rs");
include!("network_protocol_run_parts/020-placeholder.rs");

fn network_web_tooling_runtime_path(root: &std::path::Path) -> std::path::PathBuf {
    super::state_root(root).join("web_tooling_runtime.json")
}

fn read_network_web_tooling_runtime(root: &std::path::Path) -> serde_json::Value {
    let path = network_web_tooling_runtime_path(root);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

pub fn web_tooling_health_report(root: &std::path::Path, strict: bool) -> serde_json::Value {
    let runtime = read_network_web_tooling_runtime(root);
    let policy = crate::directive_kernel::web_tooling_policy_status(root);
    let missing_policy_codes = crate::directive_kernel::web_tooling_policy_missing(root);
    let auth_present = runtime
        .get("auth")
        .and_then(serde_json::Value::as_object)
        .map(|auth| {
            auth.values().any(|row| {
                row.get("present")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    let policy_ready = policy
        .get("ok")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);

    let mut errors = Vec::new();
    if !auth_present {
        errors.push("network_web_tool_auth_missing".to_string());
    }
    if !policy_ready {
        errors.push("network_web_tooling_policy_missing".to_string());
    }

    serde_json::json!({
        "ok": if strict { errors.is_empty() } else { true },
        "strict": strict,
        "type": "network_web_tooling_health_report",
        "runtime_path": network_web_tooling_runtime_path(root).display().to_string(),
        "auth_present": auth_present,
        "policy_ready": policy_ready,
        "errors": errors,
        "policy": policy,
        "missing_policy_codes": missing_policy_codes,
        "runtime": runtime,
        "recommended_actions": [
            "protheus-ops network-protocol web-tooling-status --activate=1 --strict=1",
            "protheus-ops directive-kernel repair-vault-signatures --apply=1 --allow-unsigned=1"
        ]
    })
}
