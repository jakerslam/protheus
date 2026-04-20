
fn registry_rows(root: &Path, snapshot: &Value) -> Vec<ModelRow> {
    let mut rows = Vec::<ModelRow>::new();
    for provider_row in crate::dashboard_provider_runtime::provider_rows(root, snapshot) {
        let provider = clean_text(
            provider_row.get("id").and_then(Value::as_str).unwrap_or(""),
            80,
        );
        if provider.is_empty() {
            continue;
        }
        let is_provider_local = parse_bool(provider_row.get("is_local"), false);
        let supports_chat = parse_bool(provider_row.get("supports_chat"), true);
        let needs_key = parse_bool(provider_row.get("needs_key"), false);
        let auth_status = clean_text(
            provider_row
                .get("auth_status")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            40,
        );
        let reachable = parse_bool(provider_row.get("reachable"), is_provider_local);

        let profiles = provider_row
            .get("model_profiles")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();

        for (model_name, profile) in profiles {
            let model = clean_text(&model_name, 140);
            if model.is_empty() || model_id_is_placeholder(&model) {
                continue;
            }
            let specialty = clean_text(
                profile
                    .get("specialty")
                    .and_then(Value::as_str)
                    .unwrap_or("general"),
                40,
            );
            let specialty_tags = profile
                .get("specialty_tags")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|v| v.as_str().map(|s| clean_text(s, 40)))
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>();
            let param_count_billion = parse_i64(profile.get("param_count_billion"), 0).max(0);
            let context_size = parse_i64(
                profile
                    .get("context_size")
                    .or_else(|| profile.get("context_window"))
                    .or_else(|| profile.get("context_tokens")),
                0,
            )
            .max(0);
            let deployment_kind = clean_text(
                profile
                    .get("deployment_kind")
                    .and_then(Value::as_str)
                    .unwrap_or("api"),
                40,
            )
            .to_ascii_lowercase();
            let local_download_path = clean_text(
                profile
                    .get("local_download_path")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                400,
            );
            let download_available = parse_bool(profile.get("download_available"), false)
                || !local_download_path.is_empty()
                || deployment_kind.contains("ollama")
                || deployment_kind.contains("local");
            let max_output_tokens = parse_i64(profile.get("max_output_tokens"), 0).max(0);
            let is_local = is_provider_local
                || deployment_kind.contains("local")
                || deployment_kind.contains("ollama");
            let power_signal =
                parse_i64(profile.get("power_rating"), 0)
                    .max(0)
                    .max(if param_count_billion > 0 {
                        ((param_count_billion as f64).log10() * 2.0).round() as i64
                    } else {
                        0
                    });
            let cost_signal = parse_i64(profile.get("cost_rating"), 0)
                .max(0)
                .max(if is_local {
                    ((param_count_billion as f64 / 20.0).ceil() as i64).clamp(1, 5)
                } else {
                    0
                });
            let tier = clean_text(
                profile
                    .get("tier")
                    .or_else(|| profile.get("specialty"))
                    .and_then(Value::as_str)
                    .unwrap_or("general"),
                40,
            );
            rows.push(ModelRow {
                provider: provider.clone(),
                model,
                display_name: clean_text(
                    profile
                        .get("display_name")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    160,
                ),
                specialty,
                specialty_tags,
                is_local,
                supports_chat,
                needs_key,
                auth_status: auth_status.clone(),
                reachable,
                power_signal,
                cost_signal,
                param_count_billion,
                context_size,
                deployment_kind,
                local_download_path,
                download_available,
                max_output_tokens,
                tier,
            });
        }
    }

    if rows.is_empty() {
        let provider = clean_text(
            snapshot
                .pointer("/app/settings/provider")
                .and_then(Value::as_str)
                .unwrap_or("auto"),
            80,
        );
        let model = clean_text(
            snapshot
                .pointer("/app/settings/model")
                .and_then(Value::as_str)
                .unwrap_or(""),
            140,
        );
        if !model.is_empty() && !model_id_is_placeholder(&model) {
            rows.push(ModelRow {
                provider,
                model,
                display_name: String::new(),
                specialty: "general".to_string(),
                specialty_tags: vec!["general".to_string()],
                is_local: false,
                supports_chat: false,
                needs_key: false,
                auth_status: "unknown".to_string(),
                reachable: false,
                power_signal: 3,
                cost_signal: 3,
                param_count_billion: 0,
                context_size: 0,
                deployment_kind: "api".to_string(),
                local_download_path: String::new(),
                download_available: false,
                max_output_tokens: 0,
                tier: "general".to_string(),
            });
        }
    }
    rows
}
