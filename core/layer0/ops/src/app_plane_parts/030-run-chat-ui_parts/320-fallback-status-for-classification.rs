fn chat_ui_fallback_status_for_classification(classification: &str) -> &'static str {
    match classification {
        "tool_surface_unavailable" | "tool_surface_degraded" => "failed",
        "tool_not_invoked" => "tool_not_invoked",
        "policy_blocked" => "policy_blocked",
        "tool_not_found" => "failed",
        "low_signal" => "provider_low_signal",
        _ => "parse_failed",
    }
}
