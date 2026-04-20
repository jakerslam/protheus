fn fetch_retry_operator_owner_for_reason(reason: &str) -> &'static str {
    if reason == "fetch_url_required"
        || reason == "fetch_url_invalid_scheme"
        || reason == "non_fetch_meta_query"
    {
        "user"
    } else if reason == "unknown_fetch_provider" {
        "operator"
    } else if reason == "ssrf_blocked" || reason == "policy_denied" {
        "security_operator"
    } else if reason == "web_fetch_duplicate_attempt_suppressed"
        || reason.starts_with("web_fetch_tool_surface_")
    {
        "system_operator"
    } else {
        "operator"
    }
}
