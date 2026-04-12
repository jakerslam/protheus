fn normalize_pdf_native_provider(raw: &str) -> String {
    match clean_text(raw, 80).to_ascii_lowercase().as_str() {
        "anthropic" | "claude" => "anthropic".to_string(),
        "google" | "gemini" => "google".to_string(),
        other => other.to_string(),
    }
}

fn pdf_native_provider_env_candidates(provider: &str) -> &'static [&'static str] {
    match provider {
        "anthropic" => &["ANTHROPIC_API_KEY"],
        "google" => &["GEMINI_API_KEY", "GOOGLE_API_KEY"],
        _ => &[],
    }
}

fn web_media_pdf_native_provider_contract() -> Value {
    json!({
        "providers": ["anthropic", "google"],
        "supports_multiple_pdfs": true,
        "anthropic_request_path": "/v1/messages",
        "google_request_path_template": "/v1beta/models/{model_id}:generateContent?key=<redacted>",
        "api_key_env_contract": {
            "anthropic": pdf_native_provider_env_candidates("anthropic"),
            "google": pdf_native_provider_env_candidates("google")
        },
        "returns": ["provider", "model_id", "analysis", "pdf_count"]
    })
}

fn append_pdf_native_provider_tool_entry(tool_catalog: &mut Value, policy: &Value) {
    if let Some(rows) = tool_catalog.as_array_mut() {
        rows.push(json!({
            "tool": "web_media_pdf_native_provider",
            "label": "Web Media PDF Native Provider",
            "family": "media",
            "enabled": policy.pointer("/web_conduit/enabled").and_then(Value::as_bool).unwrap_or(true),
            "request_contract": web_media_pdf_native_provider_contract()
        }));
    }
}

fn resolve_pdf_native_api_key(request: &Value, provider: &str) -> (String, String) {
    let direct = clean_text(
        request
            .get("api_key")
            .or_else(|| request.get("apiKey"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        600,
    );
    if !direct.is_empty() {
        return (direct, "request".to_string());
    }
    let explicit_env = clean_text(
        request
            .get("api_key_env")
            .or_else(|| request.get("apiKeyEnv"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        160,
    );
    if !explicit_env.is_empty() {
        return (
            std::env::var(&explicit_env).unwrap_or_default(),
            format!("env:{explicit_env}"),
        );
    }
    for candidate in pdf_native_provider_env_candidates(provider) {
        if let Ok(value) = std::env::var(candidate) {
            if !value.trim().is_empty() {
                return (value, format!("env:{candidate}"));
            }
        }
    }
    (String::new(), "missing".to_string())
}

fn build_pdf_native_sources(root: &Path, request: &Value) -> Result<Vec<Value>, Value> {
    let mut sources = Vec::new();
    if let Some(array) = request.get("sources").and_then(Value::as_array) {
        for item in array.iter().take(4) {
            let mut source_request = request.clone();
            if let Some(obj) = source_request.as_object_mut() {
                obj.remove("path");
                obj.remove("url");
                match item {
                    Value::String(raw) if raw.starts_with("http://") || raw.starts_with("https://") || raw.starts_with("data:") => {
                        obj.insert("url".to_string(), Value::String(raw.clone()));
                    }
                    Value::String(raw) => {
                        obj.insert("path".to_string(), Value::String(raw.clone()));
                    }
                    Value::Object(map) => {
                        if let Some(url) = map.get("url") {
                            obj.insert("url".to_string(), url.clone());
                        }
                        if let Some(path) = map.get("path") {
                            obj.insert("path".to_string(), path.clone());
                        }
                    }
                    _ => {}
                }
            }
            let loaded = load_media_binary_for_request(root, &source_request)?;
            if normalize_media_content_type(&loaded.content_type) != "application/pdf" {
                return Err(json!({
                    "ok": false,
                    "type": "web_conduit_pdf_native_provider",
                    "error": "unsupported_content_type",
                    "resolved_source": loaded.resolved_source,
                    "content_type": loaded.content_type,
                    "pdf_native_provider_contract": web_media_pdf_native_provider_contract()
                }));
            }
            use base64::Engine;
            sources.push(json!({
                "file_name": loaded.file_name,
                "resolved_source": loaded.resolved_source,
                "base64": base64::engine::general_purpose::STANDARD.encode(&loaded.buffer)
            }));
        }
    }
    if sources.is_empty() {
        let loaded = load_media_binary_for_request(root, request)?;
        if normalize_media_content_type(&loaded.content_type) != "application/pdf" {
            return Err(json!({
                "ok": false,
                "type": "web_conduit_pdf_native_provider",
                "error": "unsupported_content_type",
                "resolved_source": loaded.resolved_source,
                "content_type": loaded.content_type,
                "pdf_native_provider_contract": web_media_pdf_native_provider_contract()
            }));
        }
        use base64::Engine;
        sources.push(json!({
            "file_name": loaded.file_name,
            "resolved_source": loaded.resolved_source,
            "base64": base64::engine::general_purpose::STANDARD.encode(&loaded.buffer)
        }));
    }
    Ok(sources)
}

fn normalize_google_pdf_native_base_url(raw: &str) -> String {
    let cleaned = clean_text(raw, 2200);
    let chosen = if cleaned.is_empty() {
        "https://generativelanguage.googleapis.com/v1beta".to_string()
    } else {
        cleaned
    };
    let trimmed = chosen.trim_end_matches('/');
    if trimmed.ends_with("/v1beta") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/v1beta")
    }
}

fn run_curl_json_post(url: &str, headers: &[String], body: &str, timeout_ms: u64) -> Result<(i64, String), String> {
    let header_path = curl_fetch_temp_path("web-conduit-json-header", ".tmp");
    let body_path = curl_fetch_temp_path("web-conduit-json-body", ".json");
    let response_path = curl_fetch_temp_path("web-conduit-json-response", ".tmp");
    fs::write(&body_path, body).map_err(|err| format!("write_json_body_failed:{err}"))?;
    let timeout_sec = ((timeout_ms as f64) / 1000.0).ceil() as u64;
    let mut cmd = Command::new("curl");
    cmd.arg("-sS")
        .arg("--proto")
        .arg("=http,https")
        .arg("--connect-timeout")
        .arg(timeout_sec.max(1).to_string())
        .arg("--max-time")
        .arg(timeout_sec.max(1).to_string())
        .arg("-X")
        .arg("POST")
        .arg("-D")
        .arg(&header_path)
        .arg("-o")
        .arg(&response_path)
        .arg("-w")
        .arg("__STATUS__:%{http_code}")
        .arg("--data-binary")
        .arg(format!("@{}", body_path.display()));
    for header in headers {
        cmd.arg("-H").arg(header);
    }
    let output = cmd.arg(url).output().map_err(|err| format!("curl_spawn_failed:{err}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = clean_text(&String::from_utf8_lossy(&output.stderr), 240);
    let response_body = fs::read_to_string(&response_path).unwrap_or_default();
    let _ = fs::remove_file(&header_path);
    let _ = fs::remove_file(&body_path);
    let _ = fs::remove_file(&response_path);
    if !output.status.success() {
        return Err(if stderr.is_empty() { "curl_failed".to_string() } else { format!("curl_failed:{stderr}") });
    }
    let status_code = stdout
        .lines()
        .find_map(|line| line.strip_prefix("__STATUS__:"))
        .and_then(|row| clean_text(row, 12).parse::<i64>().ok())
        .unwrap_or(0);
    Ok((status_code, response_body))
}

fn api_pdf_native_analyze(root: &Path, request: &Value) -> Value {
    let provider = normalize_pdf_native_provider(
        request.get("provider").and_then(Value::as_str).unwrap_or(""),
    );
    if !matches!(provider.as_str(), "anthropic" | "google") {
        return json!({
            "ok": false,
            "type": "web_conduit_pdf_native_provider",
            "error": "unknown_pdf_native_provider",
            "provider": provider,
            "pdf_native_provider_contract": web_media_pdf_native_provider_contract()
        });
    }
    let model_id = clean_text(
        request
            .get("model_id")
            .or_else(|| request.get("modelId"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        160,
    );
    let prompt = clean_text(request.get("prompt").and_then(Value::as_str).unwrap_or(""), 4000);
    let timeout_ms =
        parse_fetch_u64(request.get("timeout_ms").or_else(|| request.get("timeoutMs")), MEDIA_FFMPEG_TIMEOUT_MS, 1000, 120_000);
    let (api_key, api_key_source) = resolve_pdf_native_api_key(request, &provider);
    if api_key.is_empty() {
        return json!({
            "ok": false,
            "type": "web_conduit_pdf_native_provider",
            "error": "api_key_required",
            "provider": provider,
            "api_key_source": api_key_source,
            "pdf_native_provider_contract": web_media_pdf_native_provider_contract()
        });
    }
    let sources = match build_pdf_native_sources(root, request) {
        Ok(row) => row,
        Err(err) => return err,
    };
    if model_id.is_empty() || prompt.is_empty() {
        return json!({
            "ok": false,
            "type": "web_conduit_pdf_native_provider",
            "error": "model_and_prompt_required",
            "provider": provider,
            "pdf_native_provider_contract": web_media_pdf_native_provider_contract()
        });
    }
    let (url, headers, body, response_parser) = if provider == "anthropic" {
        let base_url = clean_text(
            request.get("base_url").or_else(|| request.get("baseUrl")).and_then(Value::as_str).unwrap_or("https://api.anthropic.com"),
            2200,
        )
        .trim_end_matches('/')
        .to_string();
        let mut content = sources
            .iter()
            .map(|row| {
                json!({
                    "type": "document",
                    "source": {
                        "type": "base64",
                        "media_type": "application/pdf",
                        "data": row.get("base64").cloned().unwrap_or(Value::String(String::new()))
                    }
                })
            })
            .collect::<Vec<_>>();
        content.push(json!({"type": "text", "text": prompt}));
        let body = json!({
            "model": model_id,
            "max_tokens": parse_fetch_u64(request.get("max_tokens"), 4096, 1, 32000),
            "messages": [{"role": "user", "content": content}]
        })
        .to_string();
        let headers = vec![
            "Content-Type: application/json".to_string(),
            format!("x-api-key: {api_key}"),
            "anthropic-version: 2023-06-01".to_string(),
            "anthropic-beta: pdfs-2024-09-25".to_string(),
        ];
        (format!("{base_url}/v1/messages"), headers, body, "anthropic")
    } else {
        let base_url = normalize_google_pdf_native_base_url(
            request.get("base_url").or_else(|| request.get("baseUrl")).and_then(Value::as_str).unwrap_or(""),
        );
        let mut parts = sources
            .iter()
            .map(|row| {
                json!({
                    "inline_data": {
                        "mime_type": "application/pdf",
                        "data": row.get("base64").cloned().unwrap_or(Value::String(String::new()))
                    }
                })
            })
            .collect::<Vec<_>>();
        parts.push(json!({"text": prompt}));
        let body = json!({"contents": [{"role": "user", "parts": parts}]}).to_string();
        let headers = vec!["Content-Type: application/json".to_string()];
        (
            format!(
                "{}/models/{}:generateContent?key={}",
                base_url,
                urlencoding::encode(&model_id),
                urlencoding::encode(&api_key)
            ),
            headers,
            body,
            "google",
        )
    };
    let (status_code, response_body) = match run_curl_json_post(&url, &headers, &body, timeout_ms) {
        Ok(row) => row,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "web_conduit_pdf_native_provider",
                "error": "provider_request_failed",
                "provider": provider,
                "reason": clean_text(&err, 240),
                "pdf_native_provider_contract": web_media_pdf_native_provider_contract()
            });
        }
    };
    if !(200..300).contains(&status_code) {
        return json!({
            "ok": false,
            "type": "web_conduit_pdf_native_provider",
            "error": "provider_http_error",
            "provider": provider,
            "status_code": status_code,
            "body_snippet": clean_text(&response_body, 400),
            "pdf_native_provider_contract": web_media_pdf_native_provider_contract()
        });
    }
    let parsed = match serde_json::from_str::<Value>(&response_body) {
        Ok(row) => row,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "web_conduit_pdf_native_provider",
                "error": "provider_json_invalid",
                "provider": provider,
                "reason": clean_text(&err.to_string(), 240),
                "pdf_native_provider_contract": web_media_pdf_native_provider_contract()
            });
        }
    };
    let analysis = if response_parser == "anthropic" {
        parsed
            .get("content")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter(|row| row.get("type").and_then(Value::as_str) == Some("text"))
                    .filter_map(|row| row.get("text").and_then(Value::as_str))
                    .collect::<String>()
            })
            .unwrap_or_default()
    } else {
        parsed
            .get("candidates")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(|row| row.pointer("/content/parts"))
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(|row| row.get("text").and_then(Value::as_str))
                    .collect::<String>()
            })
            .unwrap_or_default()
    };
    if analysis.trim().is_empty() {
        return json!({
            "ok": false,
            "type": "web_conduit_pdf_native_provider",
            "error": "provider_no_text",
            "provider": provider,
            "status_code": status_code,
            "pdf_native_provider_contract": web_media_pdf_native_provider_contract()
        });
    }
    json!({
        "ok": true,
        "type": "web_conduit_pdf_native_provider",
        "provider": provider,
        "model_id": model_id,
        "api_key_source": api_key_source,
        "pdf_count": sources.len(),
        "analysis": analysis.trim(),
        "summary": format!("Native PDF analysis returned {} characters across {} PDF input(s).", analysis.trim().chars().count(), sources.len()),
        "pdf_native_provider_contract": web_media_pdf_native_provider_contract()
    })
}
