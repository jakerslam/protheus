fn chat_ui_retry_profile_for_guard(error_code: &str, classification: &str) -> (bool, &'static str, &'static str) {
    let code = clean(error_code, 120).to_ascii_lowercase();
    let class = clean(classification, 80).to_ascii_lowercase();
    if code == "web_tool_low_signal" || class == "low_signal" {
        return (true, "narrow_query", "immediate");
    }
    if code == "web_tool_not_invoked" || class == "tool_not_invoked" {
        return (true, "rerun_with_tool_call", "immediate");
    }
    if code == "web_tool_timeout" || code == "web_tool_http_429" || code == "web_tool_surface_degraded"
    {
        return (true, "retry_with_backoff", "delayed");
    }
    if code == "web_tool_policy_blocked" || class == "policy_blocked" {
        return (false, "operator_policy_action", "blocked");
    }
    if code == "web_tool_auth_missing" {
        return (false, "provide_auth", "blocked");
    }
    if code == "web_tool_surface_unavailable" {
        return (false, "restore_tool_surface", "blocked");
    }
    if code == "web_tool_not_found" || class == "tool_not_found" {
        return (false, "adjust_tool_selection", "blocked");
    }
    (false, "none", "none")
}
