
pub fn delete_custom_model(root: &Path, model_ref: &str) -> Value {
    let cleaned = clean_text(model_ref, 240);
    if cleaned.is_empty() {
        return json!({"ok": false, "error": "custom_model_ref_required"});
    }
    if model_id_is_placeholder(&cleaned) {
        return json!({"ok": false, "error": "custom_model_ref_required"});
    }
    let mut registry = load_registry(root);
    let mut removed = false;
    if let Some(providers) = registry.get_mut("providers").and_then(Value::as_object_mut) {
        for (provider_id, row) in providers.iter_mut() {
            let provider_id_clean = normalize_provider_id(provider_id);
            let target = if cleaned.starts_with(&(provider_id_clean.clone() + "/")) {
                clean_text(
                    cleaned.split_once('/').map(|(_, tail)| tail).unwrap_or(""),
                    200,
                )
            } else {
                cleaned.clone()
            };
            if let Some(models) = row.get_mut("model_profiles").and_then(Value::as_object_mut) {
                if models.remove(&target).is_some() {
                    removed = true;
                    row["updated_at"] = json!(crate::now_iso());
                    break;
                }
            }
        }
    }
    save_registry(root, registry);
    json!({"ok": removed, "removed": removed, "model": cleaned})
}

pub fn download_model(root: &Path, provider_id: &str, model_ref: &str) -> Value {
    let provider = normalize_provider_id(provider_id);
    let mut model = clean_text(model_ref, 240);
    if model.is_empty() || model_id_is_placeholder(&model) {
        return json!({"ok": false, "error": "model_download_path_missing"});
    }
    let (maybe_provider, maybe_model) = split_model_ref(&model);
    if !maybe_provider.is_empty() {
        if maybe_provider == "ollama" {
            return download_model(root, "ollama", &maybe_model);
        }
        if !maybe_model.is_empty() {
            model = maybe_model;
        }
    }
    if model_id_is_placeholder(&model) {
        return json!({"ok": false, "error": "model_download_path_missing"});
    }
    if provider == "ollama" {
        let output = Command::new("ollama").arg("pull").arg(&model).output();
        return match output {
            Ok(out) if out.status.success() => json!({
                "ok": true,
                "provider": provider,
                "model": model,
                "method": "ollama_pull",
                "download_path": format!("ollama://{}", model)
            }),
            Ok(out) => json!({
                "ok": false,
                "error": clean_text(
                    &format!(
                        "{} {}",
                        String::from_utf8_lossy(&out.stdout),
                        String::from_utf8_lossy(&out.stderr)
                    ),
                    280
                )
            }),
            Err(err) => json!({"ok": false, "error": clean_text(&err.to_string(), 280)}),
        };
    }

    if command_exists("ollama") {
        let mut bridged_model = if model.contains('/') {
            clean_text(model.split('/').next_back().unwrap_or(""), 200)
        } else {
            model.clone()
        };
        if let Some((head, _tail)) = bridged_model.split_once(":free") {
            bridged_model = clean_text(head, 200);
        }
        if let Some((head, _tail)) = bridged_model.split_once(":online") {
            bridged_model = clean_text(head, 200);
        }
        if !bridged_model.is_empty() && !model_id_is_placeholder(&bridged_model) {
            let output = Command::new("ollama").arg("pull").arg(&bridged_model).output();
            if let Ok(out) = output {
                if out.status.success() {
                    return json!({
                        "ok": true,
                        "provider": "ollama",
                        "requested_provider": provider,
                        "model": bridged_model,
                        "requested_model": model,
                        "method": "ollama_pull_bridge",
                        "download_path": format!("ollama://{}", bridged_model)
                    });
                }
            }
        }
    }

    let row = provider_row(root, &provider);
    let path = row
        .get("model_profiles")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get(&model))
        .and_then(|profile| profile.get("local_download_path").and_then(Value::as_str))
        .map(|raw| clean_text(raw, 4000))
        .unwrap_or_default();
    if path.is_empty() {
        return json!({"ok": false, "error": "model_download_path_missing"});
    }
    let download_path = PathBuf::from(&path);
    let _ = fs::create_dir_all(&download_path);
    json!({
        "ok": true,
        "provider": provider,
        "model": model,
        "method": "prepare_local_path",
        "download_path": download_path.to_string_lossy().to_string()
    })
}

fn invoke_chat_live(
    root: &Path,
    provider_id: &str,
    model_name: &str,
    system_prompt: &str,
    session_messages: &[Value],
    user_message: &str,
    assistant_prefill: &str,
) -> Result<Value, String> {
    let provider = normalize_provider_id(provider_id);
    let model = clean_text(model_name, 240);
    let system = clean_chat_text(system_prompt, 12_000);
    let mut messages = content_from_message_rows(session_messages);
    let user = clean_chat_text(user_message, 16_000);
    let prefill = clean_chat_text(assistant_prefill, 320);
    if user.trim().is_empty() {
        return Err("message_required".to_string());
    }
    messages.push(("user".to_string(), user.clone()));
    if !prefill.trim().is_empty() {
        messages.push(("assistant".to_string(), prefill.clone()));
    }
    let base_url = clean_text(
        provider_row(root, &provider)
            .get("base_url")
            .and_then(Value::as_str)
            .unwrap_or(&provider_base_url_default(&provider)),
        400,
    );
    let started = Instant::now();
    let context_window = model_context_window(root, &provider, &model);
    let input = ProviderInvokeInput {
        root,
        provider: &provider,
        model: &model,
        base_url: &base_url,
        system: &system,
        messages: &messages,
        prefill: &prefill,
        user: &user,
        context_window,
        started,
    };
    let response = invoke_provider_via_adapter(&input)?;
    let text = clean_chat_text(
        response
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or(""),
        32_000,
    );
    if text.trim().is_empty() {
        return Err("model backend unavailable: empty_response".to_string());
    }
    Ok(response)
}
