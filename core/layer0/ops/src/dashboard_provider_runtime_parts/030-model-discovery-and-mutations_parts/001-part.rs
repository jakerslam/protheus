#[cfg(test)]
fn scripted_chat_harness_response(
    root: &Path,
    provider_id: &str,
    model_name: &str,
    system_prompt: &str,
    user_message: &str,
) -> Option<Result<Value, String>> {
    let path = scripted_chat_harness_path(root);
    let script_exists = path.exists();
    let mut script = read_json(&path).unwrap_or_else(|| json!({}));
    let step = script
        .get_mut("queue")
        .and_then(Value::as_array_mut)
        .and_then(|queue| {
            if queue.is_empty() {
                None
            } else {
                Some(queue.remove(0))
            }
        });
    let inferred = step.and_then(|row| {
        if let Some(error) = row.get("error").and_then(Value::as_str) {
            Some(Err(clean_text(error, 240)))
        } else {
            let response = clean_chat_text(row.get("response").and_then(Value::as_str).unwrap_or(""), 32_000);
            let scripted_provider = row
                .get("provider")
                .and_then(Value::as_str)
                .unwrap_or(provider_id);
            let scripted_model = row
                .get("runtime_model")
                .or_else(|| row.get("model"))
                .and_then(Value::as_str)
                .unwrap_or(model_name);
            Some(Ok(json!({
                "ok": true,
                "provider": normalize_provider_id(scripted_provider),
                "model": clean_text(scripted_model, 240),
                "runtime_model": clean_text(scripted_model, 240),
                "response": response,
                "input_tokens": ((user_message.len() as i64) / 4).max(1),
                "output_tokens": ((response.len() as i64) / 4).max(1),
                "cost_usd": 0.0,
                "context_window": 0,
                "latency_ms": 1,
                "tools": row.get("tools").cloned().unwrap_or_else(|| json!([]))
            })))
        }
    });
    let response = if script_exists && inferred.is_none() {
        Some(Ok(json!({
            "ok": true,
            "provider": normalize_provider_id(provider_id),
            "model": clean_text(model_name, 240),
            "runtime_model": clean_text(model_name, 240),
            "response": "",
            "input_tokens": ((user_message.len() as i64) / 4).max(1),
            "output_tokens": 0,
            "cost_usd": 0.0,
            "context_window": 0,
            "latency_ms": 1,
            "tools": []
        })))
    } else {
        inferred.or_else(|| {
        infer_test_inline_tool_response(user_message).map(|response| {
            Ok(json!({
                "ok": true,
                "provider": normalize_provider_id(provider_id),
                "model": clean_text(model_name, 240),
                "runtime_model": clean_text(model_name, 240),
                "response": response,
                "input_tokens": ((user_message.len() as i64) / 4).max(1),
                "output_tokens": ((response.len() as i64) / 4).max(1),
                "cost_usd": 0.0,
                "context_window": 0,
                "latency_ms": 1,
                "tools": []
            }))
        })
    })};
    let response_excerpt = match &response {
        Some(Ok(value)) => clean_chat_text(
            value
                .get("response")
                .and_then(Value::as_str)
                .unwrap_or(""),
            20_000,
        ),
        Some(Err(error)) => clean_text(error, 1_000),
        None => String::new(),
    };
    if script_exists {
        if let Some(obj) = script.as_object_mut() {
            let calls = obj.entry("calls".to_string()).or_insert_with(|| json!([]));
            if let Some(rows) = calls.as_array_mut() {
                rows.push(json!({
                    "provider": normalize_provider_id(provider_id),
                    "model": clean_text(model_name, 240),
                    "system_prompt": clean_text(system_prompt, 4_000),
                    "user_message": clean_text(user_message, 20_000),
                    "response": response_excerpt
                }));
            }
        }
        write_json_pretty(&path, &script);
    }
    response
}

#[cfg(test)]
fn invoke_chat_impl(
    root: &Path,
    provider_id: &str,
    model_name: &str,
    system_prompt: &str,
    _session_messages: &[Value],
    user_message: &str,
    _assistant_prefill: &str,
) -> Result<Value, String> {
    if let Some(scripted) =
        scripted_chat_harness_response(root, provider_id, model_name, system_prompt, user_message)
    {
        return scripted;
    }
    if live_web_tooling_smoke_enabled() {
        return invoke_chat_live(
            root,
            provider_id,
            model_name,
            system_prompt,
            _session_messages,
            user_message,
            _assistant_prefill,
        );
    }
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
