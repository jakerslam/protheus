fn fetch_retry_priority_for_reason(reason: &str) -> &'static str {
    if reason == "fetch_url_required"
        || reason == "fetch_url_invalid_scheme"
        || reason == "ssrf_blocked"
        || reason == "policy_denied"
        || reason.starts_with("web_fetch_tool_surface_")
    {
        "high"
    } else if reason == "non_fetch_meta_query" {
        "low"
    } else {
        "medium"
    }
}
