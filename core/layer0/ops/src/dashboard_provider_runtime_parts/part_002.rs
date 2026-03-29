pub fn discover_models(root: &Path, input: &str) -> Value {
    let cleaned = clean_text(input, 4096);
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
                    if name.is_empty() {
                        continue;
                    }
                    profiles.insert(
                        name.clone(),
                        json!({
                            "power_rating": 3,
                            "cost_rating": 1,
                            "param_count_billion": 0,
                            "specialty": "general",
                            "specialty_tags": ["general"],
                            "deployment_kind": "local",
                            "local_download_path": entry.path().to_string_lossy().to_string(),
                            "download_available": true,
                            "updated_at": crate::now_iso()
                        }),
                    );
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
    if model.contains('/') {
        let mut parts = model.splitn(2, '/');
        let maybe_provider = normalize_provider_id(parts.next().unwrap_or(""));
        let maybe_model = clean_text(parts.next().unwrap_or(""), 200);
        if !maybe_provider.is_empty() && !maybe_model.is_empty() {
            model = maybe_model;
        }
    }
    if provider.is_empty() || model.is_empty() {
        return json!({"ok": false, "error": "custom_model_invalid"});
    }
    let mut registry = load_registry(root);
    let row = ensure_provider_row_mut(&mut registry, &provider);
    if row.get("model_profiles").is_none() || !row.get("model_profiles").map(Value::is_object).unwrap_or(false) {
        row["model_profiles"] = json!({});
    }
    row["model_profiles"][model.clone()] = json!({
        "power_rating": 3,
        "cost_rating": if provider_is_local(&provider) { 1 } else { 3 },
        "param_count_billion": 0,
        "specialty": "general",
        "specialty_tags": ["general"],
        "deployment_kind": if provider_is_local(&provider) { "local" } else { "api" },
        "context_window": context_window.max(0),
        "max_output_tokens": max_output_tokens.max(0),
        "download_available": provider_is_local(&provider),
        "local_download_path": "",
        "custom": true,
        "updated_at": crate::now_iso()
    });
    row["updated_at"] = json!(crate::now_iso());
    save_registry(root, registry);
    json!({"ok": true, "provider": provider, "model": model})
}

pub fn delete_custom_model(root: &Path, model_ref: &str) -> Value {
    let cleaned = clean_text(model_ref, 240);
    if cleaned.is_empty() {
        return json!({"ok": false, "error": "custom_model_ref_required"});
    }
    let mut registry = load_registry(root);
    let mut removed = false;
    if let Some(providers) = registry.get_mut("providers").and_then(Value::as_object_mut) {
        for (provider_id, row) in providers.iter_mut() {
            let provider_id_clean = normalize_provider_id(provider_id);
            let target = if cleaned.starts_with(&(provider_id_clean.clone() + "/")) {
                clean_text(cleaned.split_once('/').map(|(_, tail)| tail).unwrap_or(""), 200)
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
    if model.contains('/') {
        let mut parts = model.splitn(2, '/');
        let maybe_provider = normalize_provider_id(parts.next().unwrap_or(""));
        let maybe_model = clean_text(parts.next().unwrap_or(""), 200);
        if maybe_provider == "ollama" {
            return download_model(root, "ollama", &maybe_model);
        }
        if !maybe_model.is_empty() {
            model = maybe_model;
        }
    }
    if provider == "ollama" {
        let output = Command::new("ollama")
            .arg("pull")
            .arg(&model)
            .output();
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
) -> Result<Value, String> {
    let provider = normalize_provider_id(provider_id);
    let model = clean_text(model_name, 240);
    let system = clean_text(system_prompt, 12_000);
    let mut messages = content_from_message_rows(session_messages);
    let user = clean_text(user_message, 16_000);
    if user.is_empty() {
        return Err("message_required".to_string());
    }
    messages.push(("user".to_string(), user.clone()));
    let base_url = clean_text(
        provider_row(root, &provider)
            .get("base_url")
            .and_then(Value::as_str)
            .unwrap_or(&provider_base_url_default(&provider)),
        400,
    );
    let started = Instant::now();
    let context_window = model_context_window(root, &provider, &model);

    let response = match provider.as_str() {
        "ollama" => {
            let mut rows = Vec::<Value>::new();
            if !system.is_empty() {
                rows.push(json!({"role":"system","content": system}));
            }
            for (role, text) in &messages {
                rows.push(json!({"role": if role == "assistant" { "assistant" } else { "user" }, "content": text}));
            }
            let payload = json!({
                "model": model,
                "stream": false,
                "messages": rows
            });
            let (status, value) = curl_json(
                &format!("{base_url}/api/chat"),
                "POST",
                &["Content-Type: application/json".to_string()],
                Some(&payload),
                180,
            )?;
            if !(200..300).contains(&status) {
                return Err(format!("model backend unavailable: {}", error_text_from_value(&value)));
            }
            let text = clean_text(
                value.pointer("/message/content").and_then(Value::as_str).unwrap_or(""),
                32_000,
            );
            json!({
                "ok": true,
                "provider": provider,
                "model": model,
                "runtime_model": model,
                "response": text,
                "input_tokens": value.get("prompt_eval_count").and_then(Value::as_i64).unwrap_or(((system.len() + user.len()) / 4) as i64),
                "output_tokens": value.get("eval_count").and_then(Value::as_i64).unwrap_or((text.len() / 4) as i64),
                "cost_usd": 0.0,
                "context_window": context_window,
                "latency_ms": started.elapsed().as_millis() as i64,
                "tools": []
            })
        }
        "anthropic" => {
            let Some(key) = provider_key(root, &provider) else {
                return Err("couldn't reach a chat model backend: provider key missing".to_string());
            };
            let payload = json!({
                "model": model,
                "system": system,
                "max_tokens": 4096,
                "messages": messages.iter().map(|(role, text)| {
                    json!({
                        "role": if role == "assistant" { "assistant" } else { "user" },
                        "content": text
                    })
                }).collect::<Vec<_>>()
            });
            let headers = vec![
                "Content-Type: application/json".to_string(),
                format!("x-api-key: {key}"),
                "anthropic-version: 2023-06-01".to_string(),
            ];
            let (status, value) = curl_json(&format!("{base_url}/v1/messages"), "POST", &headers, Some(&payload), 180)?;
            if !(200..300).contains(&status) {
                return Err(format!("model backend unavailable: {}", error_text_from_value(&value)));
            }
            let text = extract_anthropic_text(&value);
            json!({
                "ok": true,
                "provider": provider,
                "model": model,
                "runtime_model": model,
                "response": text,
                "input_tokens": value.pointer("/usage/input_tokens").and_then(Value::as_i64).unwrap_or(((system.len() + user.len()) / 4) as i64),
                "output_tokens": value.pointer("/usage/output_tokens").and_then(Value::as_i64).unwrap_or(1.max((extract_anthropic_text(&value).len() / 4) as i64)),
                "cost_usd": 0.0,
                "context_window": context_window,
                "latency_ms": started.elapsed().as_millis() as i64,
                "tools": []
            })
        }
        "google" => {
            let Some(key) = provider_key(root, &provider) else {
                return Err("couldn't reach a chat model backend: provider key missing".to_string());
            };
            let payload = json!({
                "system_instruction": if system.is_empty() { Value::Null } else { json!({"parts":[{"text": system}]}) },
                "contents": messages.iter().map(|(role, text)| {
                    json!({
                        "role": if role == "assistant" { "model" } else { "user" },
                        "parts": [{"text": text}]
                    })
                }).collect::<Vec<_>>()
            });
            let (status, value) = curl_json(
                &format!("{base_url}/v1beta/models/{}:generateContent?key={}", urlencoding::encode(&model), key),
                "POST",
                &["Content-Type: application/json".to_string()],
                Some(&payload),
                180,
            )?;
            if !(200..300).contains(&status) {
                return Err(format!("model backend unavailable: {}", error_text_from_value(&value)));
            }
            let text = extract_google_text(&value);
            json!({
                "ok": true,
                "provider": provider,
                "model": model,
                "runtime_model": model,
                "response": text,
                "input_tokens": value.pointer("/usageMetadata/promptTokenCount").and_then(Value::as_i64).unwrap_or(((system.len() + user.len()) / 4) as i64),
                "output_tokens": value.pointer("/usageMetadata/candidatesTokenCount").and_then(Value::as_i64).unwrap_or(1.max((text.len() / 4) as i64)),
                "cost_usd": 0.0,
                "context_window": context_window,
                "latency_ms": started.elapsed().as_millis() as i64,
                "tools": []
            })
        }
        _ => {
            let Some(key) = provider_key(root, &provider) else {
                return Err("couldn't reach a chat model backend: provider key missing".to_string());
            };
            let mut rows = Vec::<Value>::new();
            if !system.is_empty() {
                rows.push(json!({"role": "system", "content": system}));
            }
            for (role, text) in &messages {
                rows.push(json!({"role": if role == "assistant" { "assistant" } else { "user" }, "content": text}));
            }
            let payload = json!({
                "model": model,
                "stream": false,
                "messages": rows
            });
            let headers = vec![
                "Content-Type: application/json".to_string(),
                format!("Authorization: Bearer {key}"),
            ];
            let (status, value) = curl_json(
                &format!("{base_url}/chat/completions"),
                "POST",
                &headers,
                Some(&payload),
                180,
            )?;
            if !(200..300).contains(&status) {
                return Err(format!("model backend unavailable: {}", error_text_from_value(&value)));
            }
            let text = extract_openai_text(&value);
            json!({
                "ok": true,
                "provider": provider,
                "model": model,
                "runtime_model": model,
                "response": text,
                "input_tokens": value.pointer("/usage/prompt_tokens").and_then(Value::as_i64).unwrap_or(((system.len() + user.len()) / 4) as i64),
                "output_tokens": value.pointer("/usage/completion_tokens").and_then(Value::as_i64).unwrap_or(1.max((text.len() / 4) as i64)),
                "cost_usd": 0.0,
                "context_window": context_window,
                "latency_ms": started.elapsed().as_millis() as i64,
                "tools": []
            })
        }
    };
    let text = clean_text(
        response.get("response").and_then(Value::as_str).unwrap_or(""),
        32_000,
    );
    if text.is_empty() {
        return Err("model backend unavailable: empty_response".to_string());
    }
    Ok(response)
}

#[cfg(test)]
fn invoke_chat_impl(
    _root: &Path,
    provider_id: &str,
    model_name: &str,
    system_prompt: &str,
    _session_messages: &[Value],
    user_message: &str,
) -> Result<Value, String> {
    let provider = normalize_provider_id(provider_id);
    let model = clean_text(model_name, 240);
    let system = clean_text(system_prompt, 1_000);
    let user = clean_text(user_message, 16_000);
    if user.is_empty() {
        return Err("message_required".to_string());
    }
    let response = if system.is_empty() {
        format!("[{provider}/{model}] {user}")
    } else {
        format!("[{provider}/{model}] {system} | {user}")
    };
    Ok(json!({
        "ok": true,
        "provider": provider,
        "model": model,
        "runtime_model": model,
        "response": response,
        "input_tokens": ((user.len() as i64) / 4).max(1),
        "output_tokens": ((response.len() as i64) / 4).max(1),
        "cost_usd": 0.0,
        "context_window": 0,
        "latency_ms": 1,
        "tools": []
    }))
}

#[cfg(not(test))]
fn invoke_chat_impl(
    root: &Path,
    provider_id: &str,
    model_name: &str,
    system_prompt: &str,
    session_messages: &[Value],
    user_message: &str,
) -> Result<Value, String> {
    invoke_chat_live(
        root,
        provider_id,
        model_name,
        system_prompt,
        session_messages,
        user_message,
    )
}

pub fn invoke_chat(
    root: &Path,
    provider_id: &str,
    model_name: &str,
    system_prompt: &str,
    session_messages: &[Value],
    user_message: &str,
) -> Result<Value, String> {
    invoke_chat_impl(
        root,
        provider_id,
        model_name,
        system_prompt,
        session_messages,
        user_message,
    )
}

