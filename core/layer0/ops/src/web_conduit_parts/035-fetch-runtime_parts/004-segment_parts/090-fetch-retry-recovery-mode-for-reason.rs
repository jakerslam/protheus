fn fetch_retry_recovery_mode_for_reason(reason: &str) -> &'static str {
    if reason == "fetch_url_required" || reason == "fetch_url_invalid_scheme" {
        "fix_url_input"
    } else if reason == "non_fetch_meta_query" {
        "answer_directly"
    } else if reason == "unknown_fetch_provider" {
        "switch_provider"
    } else if reason == "ssrf_blocked" || reason == "policy_denied" {
        "change_target_or_policy"
    } else if reason == "web_fetch_duplicate_attempt_suppressed" {
        "adjust_query_or_provider"
    } else if reason.starts_with("web_fetch_tool_surface_") {
        "restore_tool_surface"
    } else {
        "adjust_request"
    }
}
