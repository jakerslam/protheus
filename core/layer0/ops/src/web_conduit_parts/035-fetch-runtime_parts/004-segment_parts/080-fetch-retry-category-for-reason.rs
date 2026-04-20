fn fetch_retry_category_for_reason(reason: &str) -> &'static str {
    if reason == "fetch_url_required" || reason == "fetch_url_invalid_scheme" {
        "input_contract"
    } else if reason == "non_fetch_meta_query" {
        "intent_contract"
    } else if reason == "unknown_fetch_provider" {
        "provider_contract"
    } else if reason == "ssrf_blocked" || reason == "policy_denied" {
        "security_policy"
    } else if reason == "web_fetch_duplicate_attempt_suppressed" {
        "replay_guard"
    } else if reason.starts_with("web_fetch_tool_surface_") {
        "tool_surface"
    } else {
        "request_contract"
    }
}
