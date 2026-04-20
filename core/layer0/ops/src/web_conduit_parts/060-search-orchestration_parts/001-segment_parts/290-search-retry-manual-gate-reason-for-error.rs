fn search_retry_manual_gate_reason_for_error(error: &str) -> &'static str {
    match search_retry_blocking_kind_for_error(error) {
        "input_adjustment_required" => "input_adjustment_required",
        "direct_answer_required" => "direct_answer_required",
        "provider_configuration_required" => "provider_configuration_required",
        "tool_surface_restore_required" => "tool_surface_restore_required",
        _ => "none",
    }
}
