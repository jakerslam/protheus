
fn split_model_ref(model_ref: &str) -> (String, String) {
    let cleaned = clean_text(model_ref, 240);
    if let Some((prefix, tail)) = cleaned.split_once('/') {
        return (
            normalize_provider_id(prefix),
            clean_text(tail, 200).trim().to_string(),
        );
    }
    (String::new(), cleaned)
}

pub fn discover_models(root: &Path, input: &str) -> Value {
    let cleaned = clean_text(input, 4096);
    let auto_refresh = matches!(
        cleaned.to_ascii_lowercase().as_str(),
        "__auto__" | "auto" | "refresh" | "discover"
    );
    if auto_refresh {
        let rows = crate::dashboard_provider_runtime::provider_rows(root, &json!({}));
        let mut probed = Vec::<Value>::new();
        let mut discovered_provider_count = 0usize;
        for row in rows {
            let provider = clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 80);
            if provider.is_empty() {
                continue;
            }
            let is_local = row.get("is_local").and_then(Value::as_bool).unwrap_or(false);
            let auth_configured = crate::dashboard_provider_runtime::auth_status_configured(
                row.get("auth_status").and_then(Value::as_str).unwrap_or(""),
            );
            if !is_local && !auth_configured {
                continue;
            }
            let probe = crate::dashboard_provider_runtime::test_provider(root, &provider);
            if probe.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                discovered_provider_count += 1;
            }
            probed.push(json!({
                "provider": provider,
                "ok": probe.get("ok").and_then(Value::as_bool).unwrap_or(false),
                "status": clean_text(probe.get("status").and_then(Value::as_str).unwrap_or(""), 40),
                "error": clean_text(probe.get("error").and_then(Value::as_str).unwrap_or(""), 220)
            }));
        }
        let catalog = crate::dashboard_model_catalog::catalog_payload(root, &json!({}));
        let models = catalog
            .get("models")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let available_models = models
            .iter()
            .filter(|row| row.get("available").and_then(Value::as_bool).unwrap_or(false))
            .count();
        return json!({
            "ok": true,
            "input_kind": "auto_discovery",
            "provider_count": discovered_provider_count,
            "probed": probed,
            "model_count": models.len(),
            "available_model_count": available_models
        });
    }
    if cleaned.is_empty() {
        return json!({"ok": false, "error": "discover_input_required"});
    }
    let candidate_path = PathBuf::from(&cleaned);
    if candidate_path.exists() {
        let provider = "local";
        let mut profiles = Map::<String, Value>::new();
        let mut local_paths = Vec::<Value>::new();
        if candidate_path.is_dir() {
            if let Ok(entries) = fs::read_dir(&candidate_path) {
                for entry in entries.flatten().take(128) {
                    let name = clean_text(&entry.file_name().to_string_lossy(), 140);
                    if name.is_empty() || model_id_is_placeholder(&name) {
                        continue;
                    }
                    let mut profile = inferred_model_profile(provider, &name, true);
                    if let Some(profile_obj) = profile.as_object_mut() {
                        profile_obj.insert(
                            "local_download_path".to_string(),
                            json!(entry.path().to_string_lossy().to_string()),
                        );
                        profile_obj.insert("download_available".to_string(), json!(true));
                        profile_obj.insert("updated_at".to_string(), json!(crate::now_iso()));
                    }
                    profiles.insert(name.clone(), profile);
                    local_paths.push(json!(entry.path().to_string_lossy().to_string()));
                }
            }
        }
        let mut registry = load_registry(root);
        let row = ensure_provider_row_mut(&mut registry, provider);
        row["is_local"] = json!(true);
        row["needs_key"] = json!(false);
        row["auth_status"] = json!("configured");
        row["reachable"] = json!(true);
        row["local_model_root"] = json!(candidate_path.to_string_lossy().to_string());
        row["local_model_paths"] = json!(local_paths);
        row["model_profiles"] = Value::Object(profiles.clone());
        row["updated_at"] = json!(crate::now_iso());
        save_registry(root, registry);
        return json!({
            "ok": true,
            "provider": provider,
            "input_kind": "local_path",
            "model_count": profiles.len(),
            "models": profiles.keys().cloned().collect::<Vec<_>>()
        });
    }

    let provider = guess_provider_from_key(&cleaned);
    let saved = save_provider_key(root, &provider, &cleaned);
    if !saved.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return saved;
    }
    let row = provider_row(root, &provider);
    let models = row
        .get("model_profiles")
        .and_then(Value::as_object)
        .map(|obj| obj.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    json!({
        "ok": true,
        "provider": provider,
        "input_kind": "api_key",
        "model_count": models.len(),
        "models": models
    })
}

pub fn add_custom_model(
    root: &Path,
    provider_id: &str,
    model_id: &str,
    context_window: i64,
    max_output_tokens: i64,
) -> Value {
    let provider = normalize_provider_id(provider_id);
    let mut model = clean_text(model_id, 240);
    let (maybe_provider, maybe_model) = split_model_ref(&model);
    if provider != "openrouter" && !maybe_provider.is_empty() && !maybe_model.is_empty() {
        model = maybe_model;
    }
    if provider.is_empty() || model.is_empty() {
        return json!({"ok": false, "error": "custom_model_invalid"});
    }
    if model_id_is_placeholder(&model) {
        return json!({"ok": false, "error": "custom_model_invalid"});
    }
    let mut registry = load_registry(root);
    let row = ensure_provider_row_mut(&mut registry, &provider);
    if row.get("model_profiles").is_none()
        || !row
            .get("model_profiles")
            .map(Value::is_object)
            .unwrap_or(false)
    {
        row["model_profiles"] = json!({});
    }
    let mut profile = inferred_model_profile(&provider, &model, provider_is_local(&provider));
    if let Some(profile_obj) = profile.as_object_mut() {
        if context_window.max(0) > 0 {
            profile_obj.insert("context_window".to_string(), json!(context_window.max(0)));
        }
        profile_obj.insert(
            "max_output_tokens".to_string(),
            json!(max_output_tokens.max(0)),
        );
        profile_obj.insert(
            "download_available".to_string(),
            json!(provider_is_local(&provider)),
        );
        profile_obj.insert("local_download_path".to_string(), json!(""));
        profile_obj.insert("custom".to_string(), json!(true));
        profile_obj.insert("updated_at".to_string(), json!(crate::now_iso()));
    }
    row["model_profiles"][model.clone()] = profile;
    row["updated_at"] = json!(crate::now_iso());
    save_registry(root, registry);
    let ensured = ensure_model_profile(root, &provider, &model);
    json!({
        "ok": true,
        "provider": provider,
        "model": model,
        "metadata_researched": ensured
            .get("metadata_researched")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "profile": ensured.get("profile").cloned().unwrap_or_else(|| json!({}))
    })
}
