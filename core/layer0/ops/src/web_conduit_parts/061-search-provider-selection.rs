fn resolved_search_provider_selection(
    root: &Path,
    policy: &Value,
    request: &Value,
    provider_hint: &str,
) -> (Value, Vec<String>, String, bool) {
    let provider_resolution =
        crate::web_conduit_provider_runtime::search_provider_resolution_snapshot(
            root,
            policy,
            request,
            provider_hint,
        );
    let search_provider_chain = provider_resolution
        .get("provider_chain")
        .and_then(Value::as_array)
        .map(|rows: &Vec<Value>| {
            rows.iter()
                .filter_map(|row: &Value| row.as_str().map(ToString::to_string))
                .collect::<Vec<_>>()
        })
        .filter(|rows: &Vec<String>| !rows.is_empty())
        .unwrap_or_else(|| {
            crate::web_conduit_provider_runtime::resolved_search_provider_chain(
                provider_hint,
                request,
                policy,
            )
        });
    let selected_provider = provider_resolution
        .get("selected_provider")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| "none".to_string());
    let allow_fallback = provider_resolution
        .get("allow_fallback")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    (
        provider_resolution,
        search_provider_chain,
        selected_provider,
        allow_fallback,
    )
}
