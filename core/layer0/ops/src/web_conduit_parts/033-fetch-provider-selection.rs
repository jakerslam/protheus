fn resolved_fetch_provider_selection(
    root: &Path,
    policy: &Value,
    request: &Value,
    provider_hint: &str,
) -> (Value, Vec<String>, String) {
    let provider_resolution = crate::web_conduit_provider_runtime::fetch_provider_resolution_snapshot(
        root,
        policy,
        request,
        provider_hint,
    );
    let fetch_provider_chain = provider_resolution
        .get("provider_chain")
        .and_then(Value::as_array)
        .map(|rows: &Vec<Value>| {
            rows.iter()
                .filter_map(|row: &Value| row.as_str().map(ToString::to_string))
                .collect::<Vec<_>>()
        })
        .filter(|rows: &Vec<String>| !rows.is_empty())
        .unwrap_or_else(|| fetch_provider_chain_from_request(provider_hint, request, policy));
    let selected_provider = provider_resolution
        .get("selected_provider")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| "direct_http".to_string());
    (provider_resolution, fetch_provider_chain, selected_provider)
}
