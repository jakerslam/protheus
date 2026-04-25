
pub fn invoke_chat(
    root: &Path,
    provider_id: &str,
    model_name: &str,
    system_prompt: &str,
    session_messages: &[Value],
    user_message: &str,
) -> Result<Value, String> {
    let requested_provider = normalize_provider_id(provider_id);
    let requested_model_raw = clean_text(model_name, 240);
    let requested_model = if model_id_is_placeholder(&requested_model_raw) {
        String::new()
    } else {
        requested_model_raw
    };
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
                    let response_runtime_model = clean_text(
                        response
                            .get("runtime_model")
                            .or_else(|| response.get("model"))
                            .and_then(Value::as_str)
                            .unwrap_or(model),
                        240,
                    );
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
                    response["runtime_model"] = json!(response_runtime_model);
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
