pub fn provider_rows(root: &Path, _snapshot: &Value) -> Vec<Value> {
    let registry = load_registry(root);
    let mut provider_ids = DEFAULT_PROVIDER_IDS
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>();
    if let Some(obj) = registry.get("providers").and_then(Value::as_object) {
        for key in obj.keys() {
            let provider_id = normalize_provider_id(key);
            if provider_id.is_empty() || provider_ids.iter().any(|row| row == &provider_id) {
                continue;
            }
            provider_ids.push(provider_id);
        }
    }
    let mut rows = provider_ids
        .into_iter()
        .map(|provider_id| provider_row(root, &provider_id))
        .collect::<Vec<_>>();
    for row in &mut rows {
        let provider_id =
            normalize_provider_id(row.get("id").and_then(Value::as_str).unwrap_or(""));
        if provider_id.is_empty() {
            continue;
        }
        row["id"] = json!(provider_id.clone());
        if provider_has_builtin_defaults(&provider_id)
            || row
                .get("display_name")
                .and_then(Value::as_str)
                .map(|value| clean_text(value, 120).is_empty())
                .unwrap_or(true)
        {
            row["display_name"] = json!(provider_display_name(&provider_id));
        }
        if provider_has_builtin_defaults(&provider_id) {
            row["is_local"] = json!(provider_is_local(&provider_id));
            row["needs_key"] = json!(provider_needs_key(&provider_id));
        }
        if row
            .get("api_key_env")
            .and_then(Value::as_str)
            .map(|value| clean_text(value, 120).is_empty())
            .unwrap_or(true)
        {
            row["api_key_env"] = json!(provider_api_key_env(&provider_id));
        }
        if row
            .get("base_url")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .is_empty()
        {
            row["base_url"] = json!(provider_base_url_default(&provider_id));
        }
        let mut base_url = clean_text(
            row.get("base_url").and_then(Value::as_str).unwrap_or(""),
            400,
        );
        if provider_id == "ollama" {
            let resolved = resolve_ollama_runtime_base_url(&base_url);
            if !resolved.is_empty() {
                base_url = resolved;
                row["base_url"] = json!(base_url.clone());
            }
        }
        let key_present = provider_key(root, &provider_id).is_some();
        let mut local_reachable = if provider_is_local(&provider_id) {
            if provider_id == "ollama" {
                probe_ollama_runtime_online(&base_url)
            } else {
                local_provider_reachable(&provider_id, row)
            }
        } else {
            row.get("reachable")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        };
        if provider_is_local(&provider_id) {
            row["auth_status"] = json!(if provider_id == "claude-code" && !local_reachable {
                "not_set"
            } else {
                "configured"
            });
            row["reachable"] = json!(local_reachable);
        } else if key_present {
            row["auth_status"] = json!("configured");
        } else if !row
            .get("auth_status")
            .and_then(Value::as_str)
            .map(auth_status_configured)
            .unwrap_or(false)
        {
            row["auth_status"] = json!("not_set");
        }
        row["supports_chat"] = json!(provider_supports_chat(&provider_id, &base_url));
        if provider_id == "ollama" {
            let discovered_models = probe_ollama_runtime_models(&base_url);
            if !discovered_models.is_empty() {
                if !local_reachable {
                    // When CLI model discovery succeeds, treat the local runtime as reachable
                    // even if the initial HTTP probe path failed.
                    local_reachable = true;
                    row["reachable"] = json!(true);
                    row["auth_status"] = json!("configured");
                }
                if !row.get("model_profiles").map(Value::is_object).unwrap_or(false) {
                    row["model_profiles"] = json!({});
                }
                if let Some(profiles) = row.get_mut("model_profiles").and_then(Value::as_object_mut)
                {
                    for model in &discovered_models {
                        if profiles.contains_key(model) {
                            continue;
                        }
                        profiles.insert(
                            model.clone(),
                            inferred_model_profile("ollama", model, true),
                        );
                    }
                }
                row["detected_models"] = Value::Array(
                    discovered_models
                        .into_iter()
                        .map(Value::String)
                        .collect::<Vec<_>>(),
                );
                row["updated_at"] = json!(crate::now_iso());
            }
        }
        let mut profiles = row
            .get("model_profiles")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let profile_count_before = profiles.len();
        profiles.retain(|model_name, _| {
            let cleaned = clean_text(model_name, 240);
            !cleaned.is_empty() && !model_id_is_placeholder(&cleaned)
        });
        let profiles_sanitized = profiles.len() != profile_count_before;
        let profiles_missing = profiles.is_empty();
        if profiles_missing {
            profiles = model_profiles_for_provider(&provider_id);
        }
        let mut profiles_seeded = false;
        let provider_defaults = model_profiles_for_provider(&provider_id);
        if !provider_defaults.is_empty() {
            for (model, profile) in provider_defaults {
                if profiles.contains_key(&model) {
                    continue;
                }
                profiles.insert(model, profile);
                profiles_seeded = true;
            }
        }
        profiles.retain(|model_name, _| {
            let cleaned = clean_text(model_name, 240);
            !cleaned.is_empty() && !model_id_is_placeholder(&cleaned)
        });
        let profiles_enriched = enrich_model_profiles_for_provider(&provider_id, &mut profiles);
        if profiles_missing || profiles_enriched || profiles_sanitized || profiles_seeded {
            row["model_profiles"] = Value::Object(profiles);
        }
        let detected = row
            .get("model_profiles")
            .and_then(Value::as_object)
            .map(|obj| {
                obj.keys()
                    .filter_map(|model_name| {
                        let cleaned = clean_text(model_name, 240);
                        if cleaned.is_empty() || model_id_is_placeholder(&cleaned) {
                            None
                        } else {
                            Some(Value::String(cleaned))
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        row["detected_models"] = Value::Array(detected);
        if provider_id == "google" {
            row["aliases"] = json!(["gemini"]);
        } else if provider_id == "moonshot" {
            row["aliases"] = json!(["kimi", "moonshot-ai"]);
        }
    }
    rows.sort_by(|a, b| {
        clean_text(a.get("id").and_then(Value::as_str).unwrap_or(""), 120).cmp(&clean_text(
            b.get("id").and_then(Value::as_str).unwrap_or(""),
            120,
        ))
    });
    rows
}

pub fn providers_payload(root: &Path, snapshot: &Value) -> Value {
    let virtual_keys = virtual_keys_payload(root);
    json!({
        "ok": true,
        "providers": provider_rows(root, snapshot),
        "routing": routing_policy(root),
        "network_policy": provider_network_policy(root),
        "virtual_keys_count": virtual_keys.get("count").cloned().unwrap_or_else(|| json!(0))
    })
}

pub fn save_provider_key(root: &Path, provider_id: &str, key: &str) -> Value {
    let provider = normalize_provider_id(provider_id);
    let secret = clean_text(key, 4096);
    if provider.is_empty() || secret.is_empty() || provider == "auto" {
        return json!({"ok": false, "error": "provider_key_invalid"});
    }
    let mut secrets = load_secrets(root);
    if secrets.get("providers").is_none()
        || !secrets
            .get("providers")
            .map(Value::is_object)
            .unwrap_or(false)
    {
        secrets["providers"] = json!({});
    }
    secrets["providers"][provider.clone()] = json!({"key": secret, "updated_at": crate::now_iso()});
    save_secrets(root, secrets);

    let mut registry = load_registry(root);
    let row = ensure_provider_row_mut(&mut registry, &provider);
    row["auth_status"] = json!("configured");
    row["key_prefix"] = json!(masked_prefix(key));
    row["key_last4"] = json!(masked_last4(key));
    row["key_hash"] = json!(crate::deterministic_receipt_hash(
        &json!({"provider": provider, "key": key})
    ));
    row["key_set_at"] = json!(crate::now_iso());
    row["updated_at"] = json!(crate::now_iso());
    save_registry(root, registry);
    json!({
        "ok": true,
        "provider": provider,
        "auth_status": "configured",
        "switched_default": false
    })
}

pub fn remove_provider_key(root: &Path, provider_id: &str) -> Value {
    let provider = normalize_provider_id(provider_id);
    let mut secrets = load_secrets(root);
    if let Some(obj) = secrets.get_mut("providers").and_then(Value::as_object_mut) {
        obj.remove(&provider);
    }
    save_secrets(root, secrets);
    let mut registry = load_registry(root);
    let row = ensure_provider_row_mut(&mut registry, &provider);
    row["auth_status"] = json!(if provider_is_local(&provider) {
        "configured"
    } else {
        "not_set"
    });
    row["key_prefix"] = json!("");
    row["key_last4"] = json!("");
    row["key_hash"] = json!("");
    row["key_set_at"] = json!("");
    row["updated_at"] = json!(crate::now_iso());
    save_registry(root, registry);
    json!({"ok": true, "provider": provider})
}

pub fn set_provider_url(root: &Path, provider_id: &str, base_url: &str) -> Value {
    let provider = normalize_provider_id(provider_id);
    let cleaned = clean_text(base_url, 400);
    if provider.is_empty() || cleaned.is_empty() {
        return json!({"ok": false, "error": "provider_url_invalid"});
    }
    let mut registry = load_registry(root);
    let row = ensure_provider_row_mut(&mut registry, &provider);
    row["base_url"] = json!(cleaned);
    row["updated_at"] = json!(crate::now_iso());
    save_registry(root, registry);
    let probe = test_provider(root, &provider);
    json!({
        "ok": probe.get("status").and_then(Value::as_str) == Some("ok"),
        "provider": provider,
        "reachable": probe.get("status").and_then(Value::as_str) == Some("ok"),
        "latency_ms": probe.get("latency_ms").cloned().unwrap_or_else(|| json!(0)),
        "error": probe.get("error").cloned().unwrap_or(Value::Null)
    })
}
