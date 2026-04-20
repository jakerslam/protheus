fn fetch_retry_operator_action_hint_for_reason(reason: &str) -> &'static str {
    if reason == "fetch_url_required" {
        "provide_valid_http_or_https_url"
    } else if reason == "fetch_url_invalid_scheme" {
        "use_http_or_https_scheme"
    } else if reason == "non_fetch_meta_query" {
        "answer_without_web_fetch_or_set_force_web_fetch"
    } else if reason == "unknown_fetch_provider" {
        "set_fetch_provider_auto_or_supported_provider"
    } else if reason == "ssrf_blocked" {
        "use_public_non_local_target"
    } else if reason == "policy_denied" {
        "adjust_policy_or_target_and_retry"
    } else if reason == "web_fetch_duplicate_attempt_suppressed" {
        "adjust_query_or_wait_for_retry_window"
    } else if reason.starts_with("web_fetch_tool_surface_") {
        "restore_web_tool_surface_and_retry"
    } else {
        "adjust_fetch_request_and_retry"
    }
}
