fn fetch_retry_manual_gate_timeout_seconds_for_reason(reason: &str) -> i64 {
    match fetch_retry_manual_gate_reason_for_reason(reason) {
        "input_adjustment_required" => 1800,
        "direct_answer_required" => 900,
        "provider_configuration_required" => 3600,
        "policy_or_target_change_required" => 2400,
        "tool_surface_restore_required" => 1200,
        _ => 0,
    }
}
