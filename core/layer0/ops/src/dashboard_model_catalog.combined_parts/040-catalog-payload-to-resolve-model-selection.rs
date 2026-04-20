
pub fn catalog_payload(root: &Path, snapshot: &Value) -> Value {
    let rows = registry_rows(root, snapshot);
    let power_min = rows.iter().map(|r| r.power_signal).min().unwrap_or(1);
    let power_max = rows.iter().map(|r| r.power_signal).max().unwrap_or(5);
    let cost_min = rows.iter().map(|r| r.cost_signal).min().unwrap_or(1);
    let cost_max = rows.iter().map(|r| r.cost_signal).max().unwrap_or(5);
    let context_min = rows.iter().map(|r| r.context_size).min().unwrap_or(0);
    let context_max = rows.iter().map(|r| r.context_size).max().unwrap_or(1);

    let mut models = rows
        .into_iter()
        .map(|row| {
            let power_rating = scale_to_five(row.power_signal, power_min, power_max);
            let cost_rating = scale_to_five(row.cost_signal, cost_min, cost_max);
            let context_rating = scale_to_five(row.context_size, context_min, context_max);
            let available = row.supports_chat
                && if row.is_local {
                    row.reachable
                } else {
                    !row.needs_key
                        || row.reachable
                        || crate::dashboard_provider_runtime::auth_status_configured(
                            &row.auth_status,
                        )
                };
            let display_name = if row.display_name.is_empty() {
                row.model.clone()
            } else {
                row.display_name.clone()
            };
            json!({
                "id": format!("{}/{}", row.provider, row.model),
                "provider": row.provider,
                "model": row.model,
                "model_name": row.model,
                "runtime_model": row.model,
                "display_name": display_name,
                "is_local": row.is_local,
                "supports_chat": row.supports_chat,
                "available": available,
                "reachable": row.reachable,
                "specialty": row.specialty,
                "specialty_tags": row.specialty_tags,
                "tier": row.tier,
                "params_billion": row.param_count_billion,
                "context_size": row.context_size,
                "context_window": row.context_size,
                "context_window_tokens": row.context_size,
                "power_scale": power_rating,
                "power_rating": power_rating,
                "cost_scale": cost_rating,
                "cost_rating": cost_rating,
                "context_scale": context_rating,
                "needs_key": row.needs_key,
                "auth_status": row.auth_status,
                "deployment_kind": row.deployment_kind,
                "local_download_path": row.local_download_path,
                "download_available": row.download_available,
                "max_output_tokens": row.max_output_tokens
            })
        })
        .collect::<Vec<_>>();
    models.sort_by(|a, b| {
        clean_text(a.get("provider").and_then(Value::as_str).unwrap_or(""), 80)
            .cmp(&clean_text(
                b.get("provider").and_then(Value::as_str).unwrap_or(""),
                80,
            ))
            .then(
                clean_text(a.get("model").and_then(Value::as_str).unwrap_or(""), 140).cmp(
                    &clean_text(b.get("model").and_then(Value::as_str).unwrap_or(""), 140),
                ),
            )
    });
    json!({"ok": true, "models": models})
}

pub fn model_ref_available(
    root: &Path,
    snapshot: &Value,
    provider_id: &str,
    model_name: &str,
) -> bool {
    let provider = clean_text(provider_id, 80).to_ascii_lowercase();
    let model = clean_text(model_name, 240);
    if provider.is_empty() || model.is_empty() {
        return false;
    }
    catalog_payload(root, snapshot)
        .get("models")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter().any(|row| {
                clean_text(
                    row.get("provider").and_then(Value::as_str).unwrap_or(""),
                    80,
                )
                .eq_ignore_ascii_case(&provider)
                    && clean_text(row.get("model").and_then(Value::as_str).unwrap_or(""), 240)
                        == model
                    && parse_bool(row.get("available"), false)
            })
        })
        .unwrap_or(false)
}

pub fn resolve_model_selection(
    root: &Path,
    snapshot: &Value,
    preferred_provider: &str,
    preferred_model: &str,
    request: &Value,
) -> (String, String, Option<Value>) {
    let provider = clean_text(preferred_provider, 80);
    let model = clean_text(preferred_model, 240);
    let needs_route = provider.is_empty()
        || provider.eq_ignore_ascii_case("auto")
        || model.is_empty()
        || model.eq_ignore_ascii_case("auto")
        || !model_ref_available(root, snapshot, &provider, &model);
    if !needs_route {
        return (provider, model, None);
    }

    let route = route_decision_payload(root, snapshot, request);
    let routed_provider = clean_text(
        route
            .pointer("/route/provider")
            .or_else(|| route.pointer("/selected/provider"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    let routed_model = clean_text(
        route
            .pointer("/route/model")
            .or_else(|| route.pointer("/selected/model"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        240,
    );
    if routed_provider.is_empty() || routed_model.is_empty() {
        return (provider, model, None);
    }
    (routed_provider, routed_model, Some(route))
}
