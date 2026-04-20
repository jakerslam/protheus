fn fetch_retry_manual_gate_reason_for_reason(reason: &str) -> &'static str {
    match fetch_retry_blocking_kind_for_reason(reason) {
        "input_adjustment_required" => "input_adjustment_required",
        "direct_answer_required" => "direct_answer_required",
        "provider_configuration_required" => "provider_configuration_required",
        "policy_or_target_change_required" => "policy_or_target_change_required",
        "tool_surface_restore_required" => "tool_surface_restore_required",
        _ => "none",
    }
}
