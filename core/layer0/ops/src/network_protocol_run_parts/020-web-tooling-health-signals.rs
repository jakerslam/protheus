fn web_tooling_bool_flag(value: &serde_json::Value, path: &[&str]) -> bool {
    let mut cursor = value;
    for key in path {
        cursor = match cursor.get(*key) {
            Some(next) => next,
            None => return false,
        };
    }
    cursor.as_bool().unwrap_or(false)
}

fn web_tooling_array_len(value: &serde_json::Value, key: &str) -> usize {
    value
        .get(key)
        .and_then(serde_json::Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0)
}

pub(super) fn web_tooling_gate_hint_from_health(
    report: &serde_json::Value,
) -> serde_json::Value {
    let auth_present = report
        .get("auth_present")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let policy_ready = report
        .get("policy_ready")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let policy_search = web_tooling_bool_flag(report, &["policy", "coverage", "allow_search"]);
    let policy_fetch = web_tooling_bool_flag(report, &["policy", "coverage", "allow_fetch"]);
    let policy_runtime_channel =
        web_tooling_bool_flag(report, &["policy", "coverage", "allow_runtime_channel"]);
    let missing_codes = web_tooling_array_len(report, "missing_policy_codes");

    let severity = if !auth_present || !policy_ready {
        "critical"
    } else if missing_codes > 0 {
        "high"
    } else {
        "nominal"
    };

    serde_json::json!({
        "severity": severity,
        "auth_present": auth_present,
        "policy_ready": policy_ready,
        "policy_allow_search": policy_search,
        "policy_allow_fetch": policy_fetch,
        "policy_allow_runtime_channel": policy_runtime_channel,
        "missing_policy_codes_count": missing_codes,
        "actions": [
            "infring-ops network-protocol web-tooling-status --activate=1 --strict=1",
            "infring-ops directive-kernel repair-vault-signatures --apply=1 --allow-unsigned=1"
        ]
    })
}

pub(super) fn web_tooling_probe_signal_snapshot(raw: &str) -> serde_json::Value {
    let text = raw.trim().to_ascii_lowercase();
    let function_level_block = text.contains("blocked the function calls")
        || text.contains("preventing any web search operations")
        || text.contains("invalid response attempt");
    let low_signal = text.contains("low signal")
        || text.contains("low-signal")
        || text.contains("no-result")
        || text.contains("no result");
    let policy_filter = text.contains("security controls") || text.contains("content filtering");
    let auth_missing = text.contains("auth missing") || text.contains("token missing");

    let class = if function_level_block {
        "function_execution_blocked"
    } else if auth_missing {
        "auth_missing"
    } else if policy_filter {
        "policy_filtered"
    } else if low_signal {
        "low_signal"
    } else {
        "unknown"
    };

    serde_json::json!({
        "ok": true,
        "type": "network_web_tooling_probe_signal",
        "classification": class,
        "flags": {
            "function_level_block": function_level_block,
            "low_signal": low_signal,
            "policy_filter": policy_filter,
            "auth_missing": auth_missing
        },
        "raw_excerpt": crate::clean(text, 240)
    })
}
