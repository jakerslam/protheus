fn search_retry_requires_manual_confirmation_for_error(error: &str) -> bool {
    matches!(
        search_retry_blocking_kind_for_error(error),
        "input_adjustment_required"
            | "direct_answer_required"
            | "provider_configuration_required"
            | "tool_surface_restore_required"
    )
}
