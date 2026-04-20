fn fetch_retry_auto_retry_allowed_for_reason(reason: &str) -> bool {
    matches!(
        fetch_retry_blocking_kind_for_reason(reason),
        "provider_configuration_required" | "cooldown_required" | "none"
    )
}
