fn fetch_retry_blocking_kind_for_reason(reason: &str) -> &'static str {
    if reason == "fetch_url_required" || reason == "fetch_url_invalid_scheme" {
        "input_adjustment_required"
    } else if reason == "non_fetch_meta_query" {
        "direct_answer_required"
    } else if reason == "unknown_fetch_provider" {
        "provider_configuration_required"
    } else if reason == "ssrf_blocked" || reason == "policy_denied" {
        "policy_or_target_change_required"
    } else if reason == "web_fetch_duplicate_attempt_suppressed" {
        "cooldown_required"
    } else if reason.starts_with("web_fetch_tool_surface_") {
        "tool_surface_restore_required"
    } else {
        "none"
    }
}
