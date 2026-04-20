fn search_retry_escalation_lane_for_error(error: &str) -> &'static str {
    match search_retry_blocking_kind_for_error(error) {
        "input_adjustment_required" | "direct_answer_required" => "user_input",
        "provider_configuration_required" => "operations",
        "cooldown_required" => "automation",
        "tool_surface_restore_required" => "platform",
        _ => "none",
    }
}
