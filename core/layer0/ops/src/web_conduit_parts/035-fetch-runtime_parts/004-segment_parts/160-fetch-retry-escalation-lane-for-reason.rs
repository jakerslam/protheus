fn fetch_retry_escalation_lane_for_reason(reason: &str) -> &'static str {
    match fetch_retry_blocking_kind_for_reason(reason) {
        "input_adjustment_required" | "direct_answer_required" => "user_input",
        "provider_configuration_required" => "operations",
        "policy_or_target_change_required" => "security",
        "cooldown_required" => "automation",
        "tool_surface_restore_required" => "platform",
        _ => "none",
    }
}
