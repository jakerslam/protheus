pub(crate) fn fetch_provider_resolution_snapshot(
    root: &Path,
    policy: &Value,
    request: &Value,
    provider_hint: &str,
) -> Value {
    let mut runtime = runtime_web_family_metadata(root, policy, WebProviderFamily::Fetch);
    let requested_provider_hint = clean_text(provider_hint, 60).to_ascii_lowercase();
    let request_provider_chain = request
        .get("fetch_provider_chain")
        .or_else(|| request.get("provider_chain"))
        .map(|raw| parse_provider_list_for_family(raw, WebProviderFamily::Fetch))
        .unwrap_or_default();
    let provider_chain = fetch_provider_chain_from_request(provider_hint, request, policy);
    let selected_provider = provider_chain
        .first()
        .cloned()
        .unwrap_or_else(|| "direct_http".to_string());
    let selection_scope = if requested_provider_hint != "auto"
        && normalize_provider_token_for_family(&requested_provider_hint, WebProviderFamily::Fetch)
            .is_some()
    {
        "request_provider_hint"
    } else if !request_provider_chain.is_empty() {
        "request_provider_chain"
    } else if runtime
        .get("provider_source")
        .and_then(Value::as_str)
        .unwrap_or("none")
        == "configured"
    {
        "policy_configured"
    } else if provider_chain.is_empty() {
        "none"
    } else {
        "auto-detect"
    };
    if let Some(obj) = runtime.as_object_mut() {
        obj.insert("requested_provider_hint".to_string(), json!(requested_provider_hint));
        obj.insert("request_provider_chain".to_string(), json!(request_provider_chain));
        obj.insert("provider_chain".to_string(), json!(provider_chain));
        obj.insert("selected_provider".to_string(), json!(selected_provider));
        obj.insert("selection_scope".to_string(), json!(selection_scope));
    }
    runtime
}
