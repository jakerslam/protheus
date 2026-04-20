fn search_retry_manual_gate_timeout_seconds_for_error(error: &str) -> i64 {
    match search_retry_manual_gate_reason_for_error(error) {
        "input_adjustment_required" => 1800,
        "direct_answer_required" => 900,
        "provider_configuration_required" => 3600,
        "tool_surface_restore_required" => 1200,
        _ => 0,
    }
}
