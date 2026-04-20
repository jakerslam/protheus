fn rewrite_chat_ui_placeholder_with_tool_diagnostics(
    assistant: &str,
    diagnostics: &Value,
) -> (String, String) {
    let current = clean(assistant, 16_000);
    if current.is_empty() || !crate::tool_output_match_filter::matches_ack_placeholder(&current) {
        return (current, "unchanged".to_string());
    }
    let errors = diagnostics
        .get("error_codes")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let has_error = !errors.is_empty();
    let total_calls = diagnostics
        .get("total_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let has_surface_unavailable = errors.contains_key("web_tool_surface_unavailable");
    let has_surface_degraded = errors.contains_key("web_tool_surface_degraded");
    let has_auth_missing = errors.contains_key("web_tool_auth_missing");
    let has_policy_blocked = errors.contains_key("web_tool_policy_blocked");
    let has_invalid_response = errors.contains_key("web_tool_invalid_response");
    let has_not_found = errors.contains_key("web_tool_not_found");
    let has_silent_failure = errors.contains_key("web_tool_silent_failure");

    if has_surface_unavailable {
        return (
            chat_ui_tool_surface_fail_closed_copy("web_tool_surface_unavailable").to_string(),
            "placeholder_replaced_surface_unavailable".to_string(),
        );
    }
    if has_surface_degraded {
        return (
            chat_ui_tool_surface_fail_closed_copy("web_tool_surface_degraded").to_string(),
            "placeholder_replaced_surface_degraded".to_string(),
        );
    }
    if has_auth_missing {
        return (
            crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                "auth_missing",
                "web_tool_auth_missing",
                None,
            ),
            "placeholder_replaced_auth".to_string(),
        );
    }
    if has_policy_blocked {
        return (
            crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                "policy_blocked",
                "web_tool_policy_blocked",
                None,
            ),
            "placeholder_replaced_policy".to_string(),
        );
    }
    if has_invalid_response {
        return (
            crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                "parse_failed",
                "web_tool_invalid_response",
                None,
            ),
            "placeholder_replaced_invalid_response".to_string(),
        );
    }
    if has_not_found {
        return (
            crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                "failed",
                "web_tool_not_found",
                None,
            ),
            "placeholder_replaced_not_found".to_string(),
        );
    }
    if has_silent_failure {
        return (
            crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                "failed",
                "web_tool_silent_failure",
                None,
            ),
            "placeholder_replaced_silent_failure".to_string(),
        );
    }
    if has_error {
        return (
            crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                "failed",
                "web_tool_error",
                None,
            ),
            "placeholder_replaced_error".to_string(),
        );
    }
    if total_calls > 0 {
        return (
            crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                "provider_low_signal",
                "web_tool_low_signal",
                None,
            ),
            "placeholder_replaced_low_signal".to_string(),
        );
    }
    (current, "unchanged".to_string())
}
