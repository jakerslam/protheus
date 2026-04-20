fn fetch_retry_execution_policy_for_reason(reason: &str) -> &'static str {
    let blocking_kind = fetch_retry_blocking_kind_for_reason(reason);
    if fetch_retry_requires_manual_confirmation_for_reason(reason) {
        "manual_gate_required"
    } else if blocking_kind == "cooldown_required" {
        "deferred_auto_retry"
    } else if fetch_retry_auto_retry_allowed_for_reason(reason) {
        "auto_retry"
    } else {
        "manual_gate_required"
    }
}
