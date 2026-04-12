use crate::web_conduit_provider_runtime::{
    DEFAULT_IMAGE_TOOL_MAX_BYTES, DEFAULT_IMAGE_TOOL_MAX_IMAGES, DEFAULT_IMAGE_TOOL_PROMPT,
    DEFAULT_IMAGE_TOOL_TIMEOUT_SECONDS,
};

fn resolve_image_tool_inputs(request: &Value, max_images: usize) -> Result<Vec<String>, Value> {
    let mut combined =
        normalize_media_reference_inputs(request, "image", "images", max_images, "image inputs")?;
    for key in ["path", "url"] {
        let raw = normalize_request_string(request, key, &[], 4000);
        if !raw.is_empty() {
            combined.push(raw);
        }
    }
    normalize_media_reference_candidates(combined, max_images, "image inputs")
}

fn build_image_tool_source_request(request: &Value, raw_source: &str, max_bytes: u64) -> Value {
    let mut next = request.clone();
    if let Some(obj) = next.as_object_mut() {
        obj.remove("image");
        obj.remove("images");
        obj.insert("path".to_string(), Value::String(String::new()));
        obj.insert("url".to_string(), Value::String(String::new()));
        obj.insert("max_bytes".to_string(), json!(max_bytes));
        if raw_source.starts_with("http://")
            || raw_source.starts_with("https://")
            || raw_source.starts_with("data:")
        {
            obj.insert("url".to_string(), Value::String(raw_source.to_string()));
        } else {
            obj.insert("path".to_string(), Value::String(raw_source.to_string()));
        }
    }
    next
}

fn load_image_tool_sources(
    root: &Path,
    request: &Value,
    runtime: &Value,
) -> Result<Vec<LoadedMedia>, Value> {
    let max_images = runtime
        .get("max_images")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_IMAGE_TOOL_MAX_IMAGES) as usize;
    let max_bytes = runtime
        .get("max_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_IMAGE_TOOL_MAX_BYTES);
    let inputs = resolve_image_tool_inputs(request, max_images)?;
    if inputs.is_empty() {
        return Err(json!({
            "ok": false,
            "type": "web_conduit_image_tool",
            "error": "image_required"
        }));
    }
    let mut loaded = Vec::<LoadedMedia>::new();
    for raw_source in inputs {
        let source_request = build_image_tool_source_request(request, &raw_source, max_bytes);
        let image = load_media_binary_for_request(root, &source_request)?;
        if image.kind != "image" {
            return Err(json!({
                "ok": false,
                "type": "web_conduit_image_tool",
                "error": "unsupported_media_type",
                "requested_source": if raw_source.starts_with("data:") {
                    media_redacted_inline_source(&raw_source)
                } else {
                    raw_source
                },
                "resolved_source": image.resolved_source,
                "kind": image.kind,
                "content_type": image.content_type
            }));
        }
        loaded.push(image);
    }
    Ok(loaded)
}

fn image_tool_attempt_plan(runtime: &Value) -> Vec<(String, String)> {
    let mut plan = Vec::<(String, String)>::new();
    let mut seen = std::collections::BTreeSet::<String>::new();
    let push =
        |plan: &mut Vec<(String, String)>,
         seen: &mut std::collections::BTreeSet<String>,
         provider: String,
         model: String| {
            if provider.is_empty() || model.is_empty() {
                return;
            }
            let key = format!("{provider}/{model}");
            if seen.insert(key) {
                plan.push((provider, model));
            }
        };
    let selected_provider = clean_text(
        runtime
            .get("selected_provider")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    let selected_model = clean_text(
        runtime
            .get("selected_model")
            .and_then(Value::as_str)
            .unwrap_or(""),
        240,
    );
    push(&mut plan, &mut seen, selected_provider, selected_model);
    if !runtime
        .get("allow_fallback")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return plan;
    }
    let provider_catalog = runtime
        .get("provider_catalog")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for provider in runtime
        .get("ready_provider_order")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|value| clean_text(value, 80)))
    {
        let model = provider_catalog
            .iter()
            .find(|row| row.get("provider").and_then(Value::as_str) == Some(provider.as_str()))
            .and_then(|row| row.get("default_model").and_then(Value::as_str))
            .map(|value| clean_text(value, 240))
            .unwrap_or_default();
        push(&mut plan, &mut seen, provider, model);
    }
    plan
}

fn build_image_tool_cli_request(parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let images = parse_json_flag(
        parsed
            .flags
            .get("images-json")
            .or_else(|| parsed.flags.get("images_json")),
    )?;
    let mut request = cli_media_request_from_parsed(parsed);
    if let Some(obj) = request.as_object_mut() {
        obj.insert(
            "image".to_string(),
            json!(clean_text(
                parsed.flags.get("image").map(String::as_str).unwrap_or(""),
                4000
            )),
        );
        obj.insert("images".to_string(), images);
        obj.insert(
            "prompt".to_string(),
            json!(clean_text(
                parsed.flags.get("prompt").map(String::as_str).unwrap_or(""),
                4000
            )),
        );
        obj.insert(
            "provider".to_string(),
            json!(clean_text(
                parsed.flags.get("provider").map(String::as_str).unwrap_or(""),
                80
            )),
        );
        obj.insert(
            "model".to_string(),
            json!(clean_text(
                parsed.flags.get("model").map(String::as_str).unwrap_or(""),
                240
            )),
        );
        obj.insert(
            "max_images".to_string(),
            json!(parse_u64(
                parsed
                    .flags
                    .get("max-images")
                    .or_else(|| parsed.flags.get("max_images")),
                DEFAULT_IMAGE_TOOL_MAX_IMAGES,
                1,
                64
            )),
        );
        obj.insert(
            "timeout_seconds".to_string(),
            json!(parse_u64(
                parsed
                    .flags
                    .get("timeout-seconds")
                    .or_else(|| parsed.flags.get("timeout_seconds")),
                DEFAULT_IMAGE_TOOL_TIMEOUT_SECONDS,
                1,
                600
            )),
        );
        obj.insert(
            "max_tokens".to_string(),
            json!(parse_u64(
                parsed
                    .flags
                    .get("max-tokens")
                    .or_else(|| parsed.flags.get("max_tokens")),
                IMAGE_TOOL_DEFAULT_MAX_TOKENS,
                1,
                32_000
            )),
        );
    }
    Ok(request)
}

fn api_image_tool(root: &Path, request: &Value) -> Value {
    let (policy, _) = load_policy(root);
    let contract = crate::web_conduit_provider_runtime::web_image_tool_contract(root, &policy);
    let runtime =
        crate::web_conduit_provider_runtime::image_tool_runtime_resolution_snapshot(
            root, &policy, request,
        );
    let prompt = normalize_request_string(request, "prompt", &[], 4000);
    let effective_prompt = if prompt.is_empty() {
        runtime
            .get("default_prompt")
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_IMAGE_TOOL_PROMPT)
            .to_string()
    } else {
        prompt
    };
    let summary_only = media_tool_read_boolean_param(request, "summary_only").unwrap_or(false);
    let requested_sources = match resolve_image_tool_inputs(
        request,
        runtime
            .get("max_images")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_IMAGE_TOOL_MAX_IMAGES) as usize,
    ) {
        Ok(rows) => rows,
        Err(mut err) => {
            err["image_tool_contract"] = contract;
            err["image_tool_runtime"] = runtime;
            return err;
        }
    };
    if requested_sources.is_empty() {
        return json!({
            "ok": false,
            "type": "web_conduit_image_tool",
            "error": "image_required",
            "image_tool_contract": contract,
            "image_tool_runtime": runtime
        });
    }
    let loaded = match load_image_tool_sources(root, request, &runtime) {
        Ok(rows) => rows,
        Err(mut err) => {
            err["image_tool_contract"] = contract;
            err["image_tool_runtime"] = runtime;
            return err;
        }
    };
    let attempts_plan = image_tool_attempt_plan(&runtime);
    if attempts_plan.is_empty() {
        return json!({
            "ok": false,
            "type": "web_conduit_image_tool",
            "error": "image_tool_execution_unavailable",
            "reason": "no executable provider/model pair is currently selected",
            "requested_sources": requested_sources,
            "image_tool_contract": contract,
            "image_tool_runtime": runtime
        });
    }
    let timeout_ms = runtime
        .get("timeout_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_IMAGE_TOOL_TIMEOUT_SECONDS)
        .saturating_mul(1000);
    let max_tokens = parse_fetch_u64(
        request.get("max_tokens").or_else(|| request.get("maxTokens")),
        IMAGE_TOOL_DEFAULT_MAX_TOKENS,
        1,
        32_000,
    );
    let mut attempts = Vec::<Value>::new();
    let requested_descriptor = clean_text(&requested_sources.join(", "), 2200);
    for (provider, model) in attempts_plan {
        match invoke_image_tool_provider(
            root,
            &provider,
            &model,
            &effective_prompt,
            &loaded,
            timeout_ms,
            max_tokens,
        ) {
            Ok(result) => {
                attempts.push(json!({
                    "provider": provider,
                    "model": model,
                    "ok": true
                }));
                let analysis = result
                    .get("analysis")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let response_hash = sha256_hex(&analysis);
                let receipt = build_receipt(
                    &requested_descriptor,
                    "allow",
                    Some(&response_hash),
                    result.get("status_code").and_then(Value::as_i64).unwrap_or(200),
                    "image_tool_completed",
                    None,
                );
                let _ = append_jsonl(&receipts_path(root), &receipt);
                return json!({
                    "ok": true,
                    "type": "web_conduit_image_tool",
                    "prompt": effective_prompt,
                    "analysis": if summary_only {
                        clean_text(&analysis, 4000)
                    } else {
                        analysis.clone()
                    },
                    "summary": format!(
                        "Image tool analyzed {} image(s) with {}/{}.",
                        loaded.len(),
                        result.get("provider").and_then(Value::as_str).unwrap_or(""),
                        result.get("model").and_then(Value::as_str).unwrap_or("")
                    ),
                    "provider": result.get("provider").cloned().unwrap_or(Value::Null),
                    "model": result.get("model").cloned().unwrap_or(Value::Null),
                    "image_count": loaded.len(),
                    "images": loaded.iter().map(|image| {
                        json!({
                            "resolved_source": image.resolved_source,
                            "source_kind": image.source_kind,
                            "content_type": image.content_type,
                            "file_name": image.file_name,
                            "bytes": image.buffer.len()
                        })
                    }).collect::<Vec<_>>(),
                    "attempts": attempts,
                    "provider_resolution": {
                        "selection_scope": runtime.get("selection_scope").cloned().unwrap_or(Value::Null),
                        "allow_fallback": runtime.get("allow_fallback").cloned().unwrap_or(Value::Null),
                        "selected_provider": runtime.get("selected_provider").cloned().unwrap_or(Value::Null),
                        "selected_model": runtime.get("selected_model").cloned().unwrap_or(Value::Null)
                    },
                    "policy_decision": result.get("policy_decision").cloned().unwrap_or(Value::Null),
                    "image_tool_contract": contract,
                    "image_tool_runtime": runtime,
                    "receipt": receipt
                });
            }
            Err(err) => {
                attempts.push(json!({
                    "provider": provider,
                    "model": model,
                    "ok": false,
                    "error": clean_text(&err, 240)
                }));
            }
        }
    }
    let last_error = attempts
        .last()
        .and_then(|row| row.get("error").and_then(Value::as_str))
        .unwrap_or("image_tool_failed");
    let receipt = build_receipt(
        &requested_descriptor,
        "deny",
        None,
        500,
        "image_tool_failed",
        Some(last_error),
    );
    let _ = append_jsonl(&receipts_path(root), &receipt);
    json!({
        "ok": false,
        "type": "web_conduit_image_tool",
        "error": "image_tool_provider_failed",
        "reason": clean_text(last_error, 240),
        "prompt": effective_prompt,
        "image_count": loaded.len(),
        "attempts": attempts,
        "requested_sources": requested_sources,
        "image_tool_contract": contract,
        "image_tool_runtime": runtime,
        "receipt": receipt
    })
}
