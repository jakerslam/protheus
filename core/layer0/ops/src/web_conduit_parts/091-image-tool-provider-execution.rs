const IMAGE_TOOL_PROVIDER_SECRETS_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/provider_secrets.json";
const IMAGE_TOOL_PROVIDER_NETWORK_POLICY_REL: &str = "client/runtime/config/provider_network_policy.json";
const IMAGE_TOOL_DEFAULT_MAX_TOKENS: u64 = 4096;
const IMAGE_TOOL_DEFAULT_TELEMETRY_BLOCKLIST: &[&str] = &[
    "segment.io",
    "sentry.io",
    "mixpanel.com",
    "amplitude.com",
    "datadoghq.com",
    "newrelic.com",
];
const IMAGE_TOOL_DEFAULT_DENY_DOMAINS: &[&str] = &["metadata.google.internal", "169.254.169.254"];

fn image_tool_provider_key_env_candidates(provider: &str) -> &'static [&'static str] {
    match provider {
        "openai" => &["OPENAI_API_KEY"],
        "frontier_provider" => &["ANTHROPIC_API_KEY", "FRONTIER_PROVIDER_API_KEY", "CLAUDE_API_KEY"],
        "google" => &["GEMINI_API_KEY", "GOOGLE_API_KEY"],
        "groq" => &["GROQ_API_KEY"],
        "moonshot" => &["MOONSHOT_API_KEY", "KIMI_API_KEY"],
        "xai" => &["XAI_API_KEY"],
        "openrouter" => &["OPENROUTER_API_KEY"],
        "deepseek" => &["DEEPSEEK_API_KEY"],
        "together" => &["TOGETHER_API_KEY"],
        "fireworks" => &["FIREWORKS_API_KEY"],
        "mistral" => &["MISTRAL_API_KEY"],
        _ => &[],
    }
}

fn image_tool_provider_is_local(provider: &str) -> bool {
    matches!(provider, "ollama" | "local" | "llama.cpp" | "claude-code")
}

fn image_tool_provider_secrets_path(root: &Path) -> PathBuf {
    root.join(IMAGE_TOOL_PROVIDER_SECRETS_REL)
}

fn image_tool_provider_network_policy_path(root: &Path) -> PathBuf {
    root.join(IMAGE_TOOL_PROVIDER_NETWORK_POLICY_REL)
}

fn image_tool_provider_key(root: &Path, provider: &str) -> Option<String> {
    for key in image_tool_provider_key_env_candidates(provider) {
        if let Ok(value) = std::env::var(key) {
            let cleaned = clean_text(&value, 4096);
            if !cleaned.is_empty() {
                return Some(cleaned);
            }
        }
    }
    read_json_or(&image_tool_provider_secrets_path(root), json!({}))
        .get("providers")
        .and_then(Value::as_object)
        .and_then(|providers| providers.get(provider))
        .and_then(Value::as_object)
        .and_then(|row| row.get("key").and_then(Value::as_str))
        .map(|raw| clean_text(raw, 4096))
        .filter(|value| !value.is_empty())
}

fn image_tool_provider_row(root: &Path, provider: &str) -> Value {
    crate::dashboard_provider_runtime::provider_rows(root, &json!({}))
        .into_iter()
        .find(|row| row.get("id").and_then(Value::as_str) == Some(provider))
        .unwrap_or_else(|| json!({}))
}

fn image_tool_provider_base_url(root: &Path, provider: &str) -> String {
    clean_text(
        image_tool_provider_row(root, provider)
            .get("base_url")
            .and_then(Value::as_str)
            .unwrap_or(""),
        2200,
    )
}

fn image_tool_default_network_policy() -> Value {
    json!({
        "type": "infring_provider_network_policy",
        "version": "v1",
        "local_first_default": true,
        "require_explicit_provider_consent": true,
        "telemetry_blocklist_enabled": true,
        "telemetry_blocklist_domains": IMAGE_TOOL_DEFAULT_TELEMETRY_BLOCKLIST,
        "deny_domains": IMAGE_TOOL_DEFAULT_DENY_DOMAINS,
        "allow_provider_ids": []
    })
}

fn image_tool_host_matches_domain(host: &str, domain: &str) -> bool {
    let host_clean = clean_text(host, 220).to_ascii_lowercase();
    let domain_clean = clean_text(domain, 220).to_ascii_lowercase();
    !host_clean.is_empty()
        && !domain_clean.is_empty()
        && (host_clean == domain_clean || host_clean.ends_with(&format!(".{domain_clean}")))
}

fn image_tool_host_is_loopback(host: &str) -> bool {
    matches!(
        clean_text(host, 220).to_ascii_lowercase().as_str(),
        "localhost" | "0.0.0.0" | "::1" | "127.0.0.1"
    ) || clean_text(host, 220).starts_with("127.")
}

fn image_tool_provider_network_guard(
    root: &Path,
    provider: &str,
    base_url: &str,
) -> Result<Value, String> {
    let host = extract_domain(base_url);
    let host_is_loopback = image_tool_host_is_loopback(&host);
    let policy = read_json_or(
        &image_tool_provider_network_policy_path(root),
        image_tool_default_network_policy(),
    );
    let denied = policy
        .get("deny_domains")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|value| clean_text(value, 220)))
        .filter(|value| !value.is_empty())
        .any(|domain| !host_is_loopback && image_tool_host_matches_domain(&host, &domain));
    if denied {
        return Err("provider_network_policy_blocked:denied_domain".to_string());
    }
    let telemetry_blocked = policy
        .get("telemetry_blocklist_enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
        && policy
            .get("telemetry_blocklist_domains")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| row.as_str().map(|value| clean_text(value, 220)))
            .filter(|value| !value.is_empty())
            .any(|domain| image_tool_host_matches_domain(&host, &domain));
    if telemetry_blocked {
        return Err("provider_network_policy_blocked:telemetry_blocklist_domain".to_string());
    }
    let local_first = policy
        .get("local_first_default")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let require_explicit_provider_consent = policy
        .get("require_explicit_provider_consent")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let allow_provider_ids = policy
        .get("allow_provider_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|value| clean_text(value, 80)))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let has_provider_key = image_tool_provider_key(root, provider).is_some();
    let provider_explicitly_allowed = allow_provider_ids.iter().any(|value| value == provider);
    let needs_cloud_opt_in = local_first
        && require_explicit_provider_consent
        && !image_tool_provider_is_local(provider)
        && !has_provider_key
        && !provider_explicitly_allowed;
    if needs_cloud_opt_in {
        return Err("provider_network_policy_blocked:local_first_opt_in_required".to_string());
    }
    Ok(json!({
        "allowed": true,
        "provider": provider,
        "host": host,
        "host_is_loopback": host_is_loopback,
        "local_first_default": local_first,
        "consent_via_provider_key": has_provider_key,
        "consent_via_allowlist": provider_explicitly_allowed
    }))
}

fn image_tool_extract_text_rows(value: &Value, pointer: &str, max_len: usize) -> String {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            row.get("text")
                .and_then(Value::as_str)
                .map(|text| clean_text(text, max_len))
        })
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn image_tool_extract_openai_text(value: &Value) -> String {
    if let Some(text) = value
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(|text| clean_text(text, 32_000))
    {
        return text;
    }
    value
        .pointer("/choices/0/message/content")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            row.get("text")
                .and_then(Value::as_str)
                .map(|text| clean_text(text, 32_000))
        })
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn image_tool_error_text(value: &Value) -> String {
    value
        .get("error")
        .and_then(Value::as_str)
        .map(|text| clean_text(text, 240))
        .or_else(|| {
            value.get("error")
                .and_then(Value::as_object)
                .and_then(|row| row.get("message").and_then(Value::as_str))
                .map(|text| clean_text(text, 240))
        })
        .or_else(|| {
            value
                .get("message")
                .and_then(Value::as_str)
                .map(|text| clean_text(text, 240))
        })
        .unwrap_or_else(|| clean_text(&value.to_string(), 240))
}

fn image_tool_parse_json_response(raw: &str) -> Value {
    serde_json::from_str::<Value>(raw).unwrap_or_else(|_| json!({"raw": clean_text(raw, 12_000)}))
}

fn invoke_image_tool_provider(
    root: &Path,
    provider: &str,
    model: &str,
    prompt: &str,
    images: &[LoadedMedia],
    timeout_ms: u64,
    max_tokens: u64,
) -> Result<Value, String> {
    use base64::Engine;

    let base_url = image_tool_provider_base_url(root, provider);
    if base_url.is_empty() {
        return Err("provider_base_url_missing".to_string());
    }
    let policy_decision = image_tool_provider_network_guard(root, provider, &base_url)?;
    let started = std::time::Instant::now();
    let api_key = image_tool_provider_key(root, provider);
    let image_data = images
        .iter()
        .map(|image| {
            (
                image.content_type.clone(),
                base64::engine::general_purpose::STANDARD.encode(&image.buffer),
            )
        })
        .collect::<Vec<_>>();

    let (status_code, response_body, parser) = if provider == "frontier_provider" {
        let key = api_key.ok_or_else(|| "provider_key_missing".to_string())?;
        let content = std::iter::once(json!({"type": "text", "text": prompt}))
            .chain(image_data.iter().map(|(mime, data)| {
                json!({
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": mime,
                        "data": data
                    }
                })
            }))
            .collect::<Vec<_>>();
        let body = json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": [{"role": "user", "content": content}]
        });
        let headers = vec![
            "Content-Type: application/json".to_string(),
            format!("x-api-key: {key}"),
            "anthropic-version: 2023-06-01".to_string(),
        ];
        let raw_body =
            serde_json::to_string(&body).map_err(|err| format!("image_tool_encode_failed:{err}"))?;
        let (status_code, response_body) = run_curl_json_post(
            &format!("{}/v1/messages", base_url.trim_end_matches('/')),
            &headers,
            &raw_body,
            timeout_ms,
        )?;
        (status_code, response_body, "frontier_provider")
    } else if provider == "google" {
        let key = api_key.ok_or_else(|| "provider_key_missing".to_string())?;
        let parts = std::iter::once(json!({"text": prompt}))
            .chain(image_data.iter().map(|(mime, data)| {
                json!({
                    "inline_data": {
                        "mime_type": mime,
                        "data": data
                    }
                })
            }))
            .collect::<Vec<_>>();
        let body = json!({
            "contents": [{"role": "user", "parts": parts}]
        });
        let headers = vec!["Content-Type: application/json".to_string()];
        let raw_body =
            serde_json::to_string(&body).map_err(|err| format!("image_tool_encode_failed:{err}"))?;
        let (status_code, response_body) = run_curl_json_post(
            &format!(
                "{}/models/{}:generateContent?key={}",
                normalize_google_pdf_native_base_url(&base_url),
                urlencoding::encode(model),
                key
            ),
            &headers,
            &raw_body,
            timeout_ms,
        )?;
        (status_code, response_body, "google")
    } else if provider == "ollama" {
        let body = json!({
            "model": model,
            "stream": false,
            "messages": [{
                "role": "user",
                "content": prompt,
                "images": image_data.iter().map(|(_, data)| data.clone()).collect::<Vec<_>>()
            }]
        });
        let headers = vec!["Content-Type: application/json".to_string()];
        let raw_body =
            serde_json::to_string(&body).map_err(|err| format!("image_tool_encode_failed:{err}"))?;
        let (status_code, response_body) = run_curl_json_post(
            &format!("{}/api/chat", base_url.trim_end_matches('/')),
            &headers,
            &raw_body,
            timeout_ms,
        )?;
        (status_code, response_body, "ollama")
    } else {
        let key = api_key.ok_or_else(|| "provider_key_missing".to_string())?;
        let content = std::iter::once(json!({"type": "text", "text": prompt}))
            .chain(image_data.iter().map(|(mime, data)| {
                json!({
                    "type": "image_url",
                    "image_url": {
                        "url": format!("data:{mime};base64,{data}")
                    }
                })
            }))
            .collect::<Vec<_>>();
        let body = json!({
            "model": model,
            "stream": false,
            "messages": [{"role": "user", "content": content}]
        });
        let headers = vec![
            "Content-Type: application/json".to_string(),
            format!("Authorization: Bearer {key}"),
        ];
        let raw_body =
            serde_json::to_string(&body).map_err(|err| format!("image_tool_encode_failed:{err}"))?;
        let (status_code, response_body) = run_curl_json_post(
            &format!("{}/chat/completions", base_url.trim_end_matches('/')),
            &headers,
            &raw_body,
            timeout_ms,
        )?;
        (status_code, response_body, "openai_compat")
    };

    let value = image_tool_parse_json_response(&response_body);
    if !(200..300).contains(&(status_code as u16)) {
        return Err(format!("{provider}:{}", image_tool_error_text(&value)));
    }
    let analysis = match parser {
        "frontier_provider" => image_tool_extract_text_rows(&value, "/content", 32_000),
        "google" => image_tool_extract_text_rows(&value, "/candidates/0/content/parts", 32_000),
        "ollama" => value
            .pointer("/message/content")
            .and_then(Value::as_str)
            .map(|text| clean_text(text, 32_000))
            .unwrap_or_default(),
        _ => image_tool_extract_openai_text(&value),
    };
    Ok(json!({
        "ok": true,
        "provider": provider,
        "model": model,
        "analysis": normalize_block_text(&analysis),
        "status_code": status_code,
        "latency_ms": started.elapsed().as_millis() as u64,
        "policy_decision": policy_decision
    }))
}
