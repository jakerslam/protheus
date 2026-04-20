fn search_retry_auto_retry_allowed_for_error(error: &str) -> bool {
    matches!(
        search_retry_blocking_kind_for_error(error),
        "provider_configuration_required" | "cooldown_required" | "none"
    )
}
