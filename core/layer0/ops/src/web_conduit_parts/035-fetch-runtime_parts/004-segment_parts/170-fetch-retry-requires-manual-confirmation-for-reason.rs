fn fetch_retry_requires_manual_confirmation_for_reason(reason: &str) -> bool {
    matches!(
        fetch_retry_blocking_kind_for_reason(reason),
        "input_adjustment_required"
            | "direct_answer_required"
            | "provider_configuration_required"
            | "policy_or_target_change_required"
            | "tool_surface_restore_required"
    )
}
