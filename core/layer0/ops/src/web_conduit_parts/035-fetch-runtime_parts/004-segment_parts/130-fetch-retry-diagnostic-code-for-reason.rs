fn fetch_retry_diagnostic_code_for_reason(reason: &str) -> &'static str {
    if reason == "fetch_url_required" {
        "fetch_retry_fetch_url_required"
    } else if reason == "fetch_url_invalid_scheme" {
        "fetch_retry_fetch_url_invalid_scheme"
    } else if reason == "non_fetch_meta_query" {
        "fetch_retry_non_fetch_meta_query"
    } else if reason == "unknown_fetch_provider" {
        "fetch_retry_unknown_fetch_provider"
    } else if reason == "ssrf_blocked" {
        "fetch_retry_ssrf_blocked"
    } else if reason == "policy_denied" {
        "fetch_retry_policy_denied"
    } else if reason == "web_fetch_duplicate_attempt_suppressed" {
        "fetch_retry_duplicate_attempt_suppressed"
    } else if reason.starts_with("web_fetch_tool_surface_") {
        "fetch_retry_tool_surface"
    } else {
        "fetch_retry_request_contract_adjustment_required"
    }
}
