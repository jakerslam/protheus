fn chat_ui_fallback_status_error_for_diagnostics(
    diagnostics: &Value,
    requires_live_web: bool,
    web_search_calls: i64,
) -> (&'static str, &'static str) {
    match chat_ui_expected_classification_from_diagnostics(
        diagnostics,
        requires_live_web,
        web_search_calls,
    ) {
        "tool_surface_unavailable" => ("failed", "web_tool_surface_unavailable"),
        "tool_surface_degraded" => ("failed", "web_tool_surface_degraded"),
        "workflow_gate_blocked" => ("policy_blocked", "workflow_gate_blocked_web_tooling"),
        "tool_not_invoked" => ("tool_not_invoked", "web_tool_not_invoked"),
        "policy_blocked" => ("policy_blocked", "web_tool_policy_blocked"),
        "tool_not_found" => ("failed", "web_tool_not_found"),
        "low_signal" => ("provider_low_signal", "web_tool_low_signal"),
        _ => ("parse_failed", "web_tool_invalid_response"),
    }
}
