
pub fn test_provider(root: &Path, provider_id: &str) -> Value {
    let provider = normalize_provider_id(provider_id);
    let started = Instant::now();
    if provider == "claude-code" {
        let ok = Command::new("sh")
            .arg("-lc")
            .arg("command -v claude >/dev/null 2>&1")
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        return if ok {
            json!({"ok": true, "status": "ok", "provider": provider, "latency_ms": started.elapsed().as_millis() as i64})
        } else {
            json!({"ok": false, "status": "error", "provider": provider, "error": "claude_code_cli_not_detected"})
        };
    }

    if provider == "auto" {
        let providers = provider_rows(root, &json!({}));
        let ready = providers.into_iter().any(|row| {
            row.get("is_local")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || auth_status_configured(
                    row.get("auth_status").and_then(Value::as_str).unwrap_or(""),
                )
        });
        return if ready {
            json!({"ok": true, "status": "ok", "provider": provider, "latency_ms": started.elapsed().as_millis() as i64})
        } else {
            json!({"ok": false, "status": "error", "provider": provider, "error": "no_configured_provider"})
        };
    }

    if provider == "ollama" {
        let row = provider_row(root, &provider);
        let base_url = clean_text(
            row.get("base_url")
                .and_then(Value::as_str)
                .unwrap_or(&provider_base_url_default(&provider)),
            400,
        );
        let resolved_base = resolve_ollama_runtime_base_url(&base_url);
        let mut online = probe_ollama_runtime_online(&resolved_base);
        let discovered_models = probe_ollama_runtime_models(&resolved_base);
        if !online && !discovered_models.is_empty() {
            online = true;
        }
        let mut registry = load_registry(root);
        let row = ensure_provider_row_mut(&mut registry, &provider);
        row["base_url"] = json!(resolved_base.clone());
        row["reachable"] = json!(online);
        if !discovered_models.is_empty() {
            if !row.get("model_profiles").map(Value::is_object).unwrap_or(false) {
                row["model_profiles"] = json!({});
            }
            if let Some(profiles) = row.get_mut("model_profiles").and_then(Value::as_object_mut) {
                profiles.retain(|model_name, _| {
                    let cleaned = clean_text(model_name, 240);
                    !cleaned.is_empty() && !model_id_is_placeholder(&cleaned)
                });
                for model in &discovered_models {
                    if profiles.contains_key(model) {
                        continue;
                    }
                    profiles.insert(model.clone(), inferred_model_profile("ollama", model, true));
                }
            }
            row["detected_models"] = Value::Array(
                discovered_models
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect::<Vec<_>>(),
            );
        }
        row["updated_at"] = json!(crate::now_iso());
        save_registry(root, registry);
        return if online {
            json!({
                "ok": true,
                "status": "ok",
                "provider": provider,
                "latency_ms": started.elapsed().as_millis() as i64,
                "detail": {
                    "base_url": resolved_base,
                    "discovered_models": discovered_models,
                }
            })
        } else {
            json!({
                "ok": false,
                "status": "error",
                "provider": provider,
                "error": "ollama_runtime_unreachable",
                "detail": {"base_url": resolved_base}
            })
        };
    }

    let row = provider_row(root, &provider);
    let base_url = clean_text(
        row.get("base_url")
            .and_then(Value::as_str)
            .unwrap_or(&provider_base_url_default(&provider)),
        400,
    );
    let mut headers = vec!["Content-Type: application/json".to_string()];
    let url = match provider.as_str() {
        "ollama" => format!("{base_url}/api/tags"),
        "google" => {
            let Some(key) = provider_key(root, &provider) else {
                return json!({"ok": false, "status": "error", "provider": provider, "error": "provider_key_missing"});
            };
            format!("{base_url}/v1beta/models?key={key}")
        }
        "frontier_provider" => {
            let Some(key) = provider_key(root, &provider) else {
                return json!({"ok": false, "status": "error", "provider": provider, "error": "provider_key_missing"});
            };
            headers.push(format!("x-api-key: {key}"));
            headers.push("anthropic-version: 2023-06-01".to_string());
            format!("{base_url}/v1/models")
        }
        _ => {
            let Some(key) = provider_key(root, &provider) else {
                return json!({"ok": false, "status": "error", "provider": provider, "error": "provider_key_missing"});
            };
            headers.push(format!("Authorization: Bearer {key}"));
            format!("{base_url}/models")
        }
    };

    match curl_json(&url, "GET", &headers, None, 20) {
        Ok((status, value)) if status >= 200 && status < 300 => {
            let mut registry = load_registry(root);
            let row = ensure_provider_row_mut(&mut registry, &provider);
            row["reachable"] = json!(true);
            let discovered_models = models_from_probe_response(&provider, &value);
            if !discovered_models.is_empty() {
                if !row.get("model_profiles").map(Value::is_object).unwrap_or(false) {
                    row["model_profiles"] = json!({});
                }
                if let Some(profiles) = row.get_mut("model_profiles").and_then(Value::as_object_mut) {
                    profiles.retain(|model_name, _| {
                        let cleaned = clean_text(model_name, 240);
                        !cleaned.is_empty() && !model_id_is_placeholder(&cleaned)
                    });
                    for model in &discovered_models {
                        if profiles.contains_key(model) {
                            continue;
                        }
                        profiles.insert(
                            model.clone(),
                            inferred_model_profile(&provider, model, provider_is_local(&provider)),
                        );
                    }
                }
                row["detected_models"] = Value::Array(
                    discovered_models
                        .iter()
                        .cloned()
                        .map(Value::String)
                        .collect::<Vec<_>>(),
                );
            }
            row["updated_at"] = json!(crate::now_iso());
            save_registry(root, registry);
            json!({
                "ok": true,
                "status": "ok",
                "provider": provider,
                "latency_ms": started.elapsed().as_millis() as i64,
                "detail": value
            })
        }
        Ok((status, value)) => {
            let mut registry = load_registry(root);
            let row = ensure_provider_row_mut(&mut registry, &provider);
            row["reachable"] = json!(false);
            row["updated_at"] = json!(crate::now_iso());
            save_registry(root, registry);
            json!({
                "ok": false,
                "status": "error",
                "provider": provider,
                "error": format!("http_{status}:{}", error_text_from_value(&value))
            })
        }
        Err(err) => {
            let mut registry = load_registry(root);
            let row = ensure_provider_row_mut(&mut registry, &provider);
            row["reachable"] = json!(false);
            row["updated_at"] = json!(crate::now_iso());
            save_registry(root, registry);
            json!({"ok": false, "status": "error", "provider": provider, "error": clean_text(&err, 280)})
        }
    }
}
