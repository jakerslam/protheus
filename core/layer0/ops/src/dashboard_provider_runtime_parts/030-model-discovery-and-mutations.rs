include!("035-model-metadata-research.rs");
include!("022-provider-adapters.rs");
include!("025-prompt-optimization.rs");

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
    if model.contains('/') && provider != "openrouter" {
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

include!("040-routing-policy.rs");
include!("050-virtual-keys.rs");
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

fn infer_auto_route_request(
    system_prompt: &str,
    session_messages: &[Value],
    user_message: &str,
) -> Value {
    let system = clean_chat_text(system_prompt, 2_000).to_ascii_lowercase();
    let user = clean_chat_text(user_message, 4_000).to_ascii_lowercase();
    let transcript = content_from_message_rows(session_messages)
        .into_iter()
        .rev()
        .take(6)
        .map(|(_, text)| clean_chat_text(&text, 320))
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    let merged = format!("{system} {user} {transcript}");
    let complexity = if merged.contains("deep")
        || merged.contains("in-depth")
        || merged.contains("comprehensive")
        || merged.contains("thorough")
        || merged.contains("analyze")
    {
        "high"
    } else {
        "general"
    };
    let task_type = if merged.contains("code")
        || merged.contains("bug")
        || merged.contains("test")
        || merged.contains("refactor")
    {
        "code"
    } else if merged.contains("research")
        || merged.contains("docs")
        || merged.contains("cite")
    {
        "research"
    } else {
        "general"
    };
    let budget_mode = if merged.contains("cheap")
        || merged.contains("low cost")
        || merged.contains("save tokens")
        || merged.contains("fast")
    {
        "cheap"
    } else {
        "balanced"
    };
    let prefer_local = merged.contains("local")
        || merged.contains("offline")
        || merged.contains("private")
        || merged.contains("air-gapped");
    let token_count = ((system.len() + user.len() + transcript.len()) / 4).max(1) as i64;
    json!({
        "task_type": task_type,
        "complexity": complexity,
        "budget_mode": budget_mode,
        "prefer_local": prefer_local,
        "token_count": token_count
    })
}

#[cfg(test)]
fn invoke_chat_impl(
    _root: &Path,
    provider_id: &str,
    model_name: &str,
    system_prompt: &str,
    _session_messages: &[Value],
    user_message: &str,
    _assistant_prefill: &str,
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
    assistant_prefill: &str,
) -> Result<Value, String> {
    invoke_chat_live(
        root,
        provider_id,
        model_name,
        system_prompt,
        session_messages,
        user_message,
        assistant_prefill,
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
    let requested_provider = normalize_provider_id(provider_id);
    let requested_model = clean_text(model_name, 240);
    let snapshot = read_json(&PathBuf::from(root).join(
        "client/runtime/local/state/ui/infring_dashboard/latest_snapshot.json",
    ))
    .unwrap_or_else(|| json!({}));
    let route_request = infer_auto_route_request(system_prompt, session_messages, user_message);
    let (resolved_provider, resolved_model, auto_route_decision) =
        crate::dashboard_model_catalog::resolve_model_selection(
            root,
            &snapshot,
            &requested_provider,
            &requested_model,
            &route_request,
        );
    if let Some(decision) = auto_route_decision.clone() {
        append_routing_event(
            root,
            &json!({
                "ts": crate::now_iso(),
                "ok": true,
                "provider": clean_text(&resolved_provider, 80),
                "model": clean_text(&resolved_model, 240),
                "auto_route_decision": decision,
                "route_request": route_request
            }),
        );
    }
    let primary_provider = normalize_provider_id(&resolved_provider);
    let primary_model = clean_text(&resolved_model, 240);
    if primary_provider.is_empty() || primary_model.is_empty() {
        return Err("provider_or_model_required".to_string());
    }
    let routes = fallback_routes(root, &primary_provider, &primary_model);
    let (max_attempts_per_route, max_total_attempts, base_backoff_ms, max_backoff_ms, factor) =
        retry_policy_limits(root);
    let mut attempts = Vec::<Value>::new();
    let mut total_attempts = 0usize;
    let mut last_error = "model backend unavailable".to_string();
    let mut last_prompt_metadata = json!({});

    for (route_index, (provider, model)) in routes.iter().enumerate() {
        let optimized = optimize_prompt_request(
            root,
            provider,
            model,
            system_prompt,
            session_messages,
            user_message,
        );
        let optimized_system_prompt = optimized.system_prompt.clone();
        let optimized_session_messages = optimized.session_messages.clone();
        let optimized_user_message = optimized.user_message.clone();
        let optimized_prefill = optimized.assistant_prefill.clone();
        let route_prompt_metadata = optimized.metadata.clone();
        last_prompt_metadata = route_prompt_metadata.clone();
        let mut route_attempt_index = 0usize;
        while route_attempt_index < max_attempts_per_route && total_attempts < max_total_attempts {
            route_attempt_index += 1;
            total_attempts += 1;
            let attempt_started = Instant::now();
            match invoke_chat_impl(
                root,
                provider,
                model,
                &optimized_system_prompt,
                &optimized_session_messages,
                &optimized_user_message,
                &optimized_prefill,
            ) {
                Ok(mut response) => {
                    let input_tokens = response
                        .get("input_tokens")
                        .and_then(Value::as_i64)
                        .unwrap_or(0);
                    let output_tokens = response
                        .get("output_tokens")
                        .and_then(Value::as_i64)
                        .unwrap_or(0);
                    let provider_cost =
                        parse_f64_like(response.get("cost_usd"), 0.0, 0.0, 1_000_000_000.0);
                    let estimated_cost = if provider_cost > 0.0 {
                        provider_cost
                    } else {
                        estimated_chat_cost_usd(root, provider, model, input_tokens, output_tokens)
                    };
                    response["cost_usd"] = json!(round_usd(estimated_cost));
                    response["provider"] = json!(provider);
                    response["model"] = json!(model);
                    response["runtime_model"] = json!(model);
                    let response_hash = crate::deterministic_receipt_hash(&json!({
                        "provider": provider,
                        "model": model,
                        "response": response.get("response").and_then(Value::as_str).unwrap_or("")
                    }));
                    response["response_hash"] = json!(response_hash.clone());
                    response["prompt_optimization"] = route_prompt_metadata.clone();
                    if let Some(decision) = auto_route_decision.clone() {
                        response["auto_route_decision"] = decision;
                    }
                    let mut trace_rows = attempts.clone();
                    trace_rows.push(json!({
                        "provider": provider,
                        "model": model,
                        "route_index": route_index,
                        "route_attempt": route_attempt_index,
                        "global_attempt": total_attempts,
                        "ok": true,
                        "latency_ms": attempt_started.elapsed().as_millis() as i64
                    }));
                    response["routing_trace"] = json!({
                        "attempts": trace_rows,
                        "fallback_applied": route_index > 0,
                        "total_attempts": total_attempts,
                        "retry_policy": {
                            "max_attempts_per_route": max_attempts_per_route,
                            "max_total_attempts": max_total_attempts,
                            "base_backoff_ms": base_backoff_ms,
                            "max_backoff_ms": max_backoff_ms,
                            "factor": factor
                        }
                    });
                    append_routing_event(
                        root,
                        &json!({
                            "ts": crate::now_iso(),
                            "ok": true,
                            "provider": provider,
                            "model": model,
                            "fallback_applied": route_index > 0,
                            "total_attempts": total_attempts,
                            "input_tokens": input_tokens,
                            "output_tokens": output_tokens,
                            "cost_usd": round_usd(estimated_cost),
                            "prompt_optimization": route_prompt_metadata.clone()
                        }),
                    );
                    append_provider_inference_receipt(
                        root,
                        json!({
                            "ok": true,
                            "provider": provider,
                            "model": model,
                            "input_tokens": input_tokens,
                            "output_tokens": output_tokens,
                            "cost_usd": round_usd(estimated_cost),
                            "response_hash": response_hash
                        }),
                    );
                    return Ok(response);
                }
                Err(error) => {
                    let retryable = is_retryable_model_error(&error);
                    last_error = error.clone();
                    attempts.push(json!({
                        "provider": provider,
                        "model": model,
                        "route_index": route_index,
                        "route_attempt": route_attempt_index,
                        "global_attempt": total_attempts,
                        "ok": false,
                        "retryable": retryable,
                        "error": clean_text(&error, 240),
                        "latency_ms": attempt_started.elapsed().as_millis() as i64,
                        "prompt_optimization": route_prompt_metadata.clone()
                    }));
                    append_provider_inference_receipt(
                        root,
                        json!({
                            "ok": false,
                            "provider": provider,
                            "model": model,
                            "error": clean_text(&error, 240),
                            "response_hash": Value::Null
                        }),
                    );
                    if !retryable {
                        break;
                    }
                    if route_attempt_index < max_attempts_per_route
                        && total_attempts < max_total_attempts
                    {
                        let _backoff_ms = backoff_for_attempt(
                            base_backoff_ms,
                            max_backoff_ms,
                            factor,
                            route_attempt_index,
                        );
                        #[cfg(not(test))]
                        std::thread::sleep(std::time::Duration::from_millis(_backoff_ms));
                    }
                }
            }
        }
    }

    append_routing_event(
        root,
        &json!({
            "ts": crate::now_iso(),
            "ok": false,
            "provider": primary_provider,
            "model": primary_model,
            "error": clean_text(&last_error, 240),
            "total_attempts": total_attempts,
            "attempts": attempts,
            "prompt_optimization": last_prompt_metadata
        }),
    );
    Err(format!(
        "model backend unavailable: routing_exhausted:{}",
        clean_text(&last_error, 240)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn routing_policy_requires_signature_on_update() {
        let root = tempfile::tempdir().expect("tempdir");
        let rejected = update_routing_policy(root.path(), &json!({"mode":"simulation"}));
        assert_eq!(
            rejected.get("ok").and_then(Value::as_bool),
            Some(false),
            "unsigned routing updates must fail closed"
        );
        assert_eq!(
            rejected.get("error").and_then(Value::as_str),
            Some("routing_policy_signature_required")
        );
    }

    #[test]
    fn routing_policy_update_and_fallback_chain_round_trip() {
        let root = tempfile::tempdir().expect("tempdir");
        let updated = update_routing_policy(
            root.path(),
            &json!({
                "signature": "sig:test-routing-v1",
                "retry": {"max_attempts_per_route": 3, "max_total_attempts": 6},
                "fallback_chain": [
                    {"provider":"moonshot","model":"kimi-k2.5"},
                    {"provider":"openrouter","model":"deepseek/deepseek-chat-v3-0324:free"}
                ]
            }),
        );
        assert_eq!(updated.get("ok").and_then(Value::as_bool), Some(true));
        let chain = routing_fallback_chain(root.path(), "openai", "gpt-5");
        assert!(chain.len() >= 3);
        assert_eq!(
            chain[0].get("provider").and_then(Value::as_str),
            Some("openai")
        );
        assert_eq!(
            chain[1].get("provider").and_then(Value::as_str),
            Some("moonshot")
        );
    }

    #[test]
    fn virtual_key_budget_is_fail_closed() {
        let root = tempfile::tempdir().expect("tempdir");
        let upsert = upsert_virtual_key(
            root.path(),
            "team-alpha",
            &json!({
                "provider": "openai",
                "model": "gpt-5",
                "team_id": "alpha",
                "budget_limit_usd": 0.000001,
                "rate_limit_rpm": 100
            }),
        );
        assert_eq!(upsert.get("ok").and_then(Value::as_bool), Some(true));
        let reserve = reserve_virtual_key_slot(root.path(), "team-alpha");
        assert_eq!(reserve.get("ok").and_then(Value::as_bool), Some(true));
        let spend = record_virtual_key_usage(root.path(), "team-alpha", 0.01);
        assert_eq!(spend.get("ok").and_then(Value::as_bool), Some(true));
        let blocked = reserve_virtual_key_slot(root.path(), "team-alpha");
        assert_eq!(blocked.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            blocked.get("error").and_then(Value::as_str),
            Some("virtual_key_budget_exceeded")
        );
    }

    #[test]
    fn invoke_chat_emits_routing_trace_and_cost_estimate() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = invoke_chat(
            root.path(),
            "openai",
            "gpt-5",
            "You are a helper",
            &[],
            "hello",
        )
        .expect("invoke chat");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(out
            .get("routing_trace")
            .and_then(|row| row.get("attempts"))
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        assert!(out
            .get("cost_usd")
            .and_then(Value::as_f64)
            .map(|value| value >= 0.0)
            .unwrap_or(false));
        assert!(out
            .get("prompt_optimization")
            .and_then(|row| row.get("cache_control"))
            .and_then(|row| row.get("lane"))
            .and_then(Value::as_str)
            .map(|lane| !lane.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn optimize_prompt_request_tracks_cache_hits_and_context_summary() {
        let root = tempfile::tempdir().expect("tempdir");
        let mut rows = Vec::<Value>::new();
        for idx in 0..20usize {
            rows.push(json!({
                "role": if idx % 2 == 0 { "user" } else { "assistant" },
                "text": format!("long conversation turn {idx}: {}", "detail ".repeat(40))
            }));
        }
        let first = optimize_prompt_request(
            root.path(),
            "openai",
            "gpt-5",
            "You are helpful.",
            &rows,
            "Return JSON schema for concise cache test output.",
        );
        let second = optimize_prompt_request(
            root.path(),
            "openai",
            "gpt-5",
            "You are helpful.",
            &rows,
            "Return JSON schema for concise cache test output.",
        );
        assert_eq!(
            first
                .metadata
                .pointer("/cache_control/cache_hit")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            second
                .metadata
                .pointer("/cache_control/cache_hit")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            second
                .metadata
                .pointer("/context/summary_applied")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            second
                .metadata
                .pointer("/output_contract/type")
                .and_then(Value::as_str),
            Some("json")
        );
        assert_eq!(second.assistant_prefill, "{");
    }

    #[test]
    fn ensure_model_profile_backfills_metadata_for_new_model_ref() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = ensure_model_profile(root.path(), "moonshot", "kimi-k2.5-preview");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        let profile = out.get("profile").cloned().unwrap_or_else(|| json!({}));
        assert!(
            profile
                .get("context_window")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= 131_072
        );
        assert!(
            profile
                .get("max_output_tokens")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                > 0
        );
    }

    #[test]
    fn ensure_model_profile_keeps_openrouter_namespaced_model_ids() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = ensure_model_profile(root.path(), "openrouter", "moonshotai/kimi-k2.5");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("model").and_then(Value::as_str), Some("moonshotai/kimi-k2.5"));
    }

    #[test]
    fn missing_model_errors_are_not_retryable() {
        assert!(!is_retryable_model_error(
            "model backend unavailable: model 'llama3.2:latest' not found"
        ));
        assert!(!is_retryable_model_error("ollama: no such model"));
    }

    #[test]
    fn fallback_chain_skips_unavailable_local_models() {
        let root = tempfile::tempdir().expect("tempdir");
        let updated = update_routing_policy(
            root.path(),
            &json!({
                "signature": "sig:test-local-fallback-skip",
                "fallback_chain": [
                    {"provider":"ollama","model":"definitely-missing-local-model-xyz"}
                ]
            }),
        );
        assert_eq!(updated.get("ok").and_then(Value::as_bool), Some(true));
        let chain = routing_fallback_chain(root.path(), "openai", "gpt-5");
        let has_missing_local = chain.iter().any(|row| {
            row.get("provider").and_then(Value::as_str) == Some("ollama")
                && row.get("model").and_then(Value::as_str)
                    == Some("definitely-missing-local-model-xyz")
        });
        assert!(
            !has_missing_local,
            "unavailable local fallback models should be filtered out"
        );
    }

    #[test]
    fn invoke_chat_auto_route_emits_decision_and_inference_receipt() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = add_custom_model(root.path(), "openai", "gpt-4o-mini", 128_000, 4_096);
        let _ = save_provider_key(root.path(), "openai", "sk-test-openai");
        let out = invoke_chat(
            root.path(),
            "auto",
            "auto",
            "You are a coding assistant.",
            &[],
            "Write a concise code review checklist.",
        )
        .expect("auto route invoke");
        assert!(
            out.get("provider")
                .and_then(Value::as_str)
                .map(|row| !row.is_empty() && row != "auto")
                .unwrap_or(false),
            "auto route should resolve a concrete provider"
        );
        assert!(
            out.get("model")
                .and_then(Value::as_str)
                .map(|row| !row.is_empty() && row != "auto")
                .unwrap_or(false),
            "auto route should resolve a concrete model"
        );
        assert!(
            out.get("auto_route_decision")
                .and_then(Value::as_object)
                .is_some(),
            "resolved auto route decision should be visible in response"
        );
        assert!(
            out.get("response_hash")
                .and_then(Value::as_str)
                .map(|row| !row.is_empty())
                .unwrap_or(false),
            "response hash should be attached for deterministic receipts"
        );
        let receipts = fs::read_to_string(root.path().join(PROVIDER_INFERENCE_RECEIPTS_REL))
            .expect("provider inference receipts");
        assert!(
            receipts.contains("\"type\":\"infring_provider_inference_receipt\"")
                && receipts.contains("\"provider\"")
                && receipts.contains("\"response_hash\""),
            "inference receipts should be persisted with provider and response hash fields"
        );
    }
}
