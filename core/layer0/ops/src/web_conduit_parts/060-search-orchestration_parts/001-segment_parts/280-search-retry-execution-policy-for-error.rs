fn search_retry_execution_policy_for_error(error: &str) -> &'static str {
    let blocking_kind = search_retry_blocking_kind_for_error(error);
    if search_retry_requires_manual_confirmation_for_error(error) {
        "manual_gate_required"
    } else if blocking_kind == "cooldown_required" {
        "deferred_auto_retry"
    } else if search_retry_auto_retry_allowed_for_error(error) {
        "auto_retry"
    } else {
        "manual_gate_required"
    }
}
