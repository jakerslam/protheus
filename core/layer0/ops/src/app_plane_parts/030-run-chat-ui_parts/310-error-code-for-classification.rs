fn chat_ui_error_code_for_classification(classification: &str) -> &'static str {
    match classification {
        "tool_surface_unavailable" => "web_tool_surface_unavailable",
        "tool_surface_degraded" => "web_tool_surface_degraded",
        "workflow_gate_blocked" => "workflow_gate_blocked_web_tooling",
        "tool_not_invoked" => "web_tool_not_invoked",
        "policy_blocked" => "web_tool_policy_blocked",
        "tool_not_found" => "web_tool_not_found",
        "low_signal" => "web_tool_low_signal",
        _ => "",
    }
}
