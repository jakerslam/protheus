const PDF_TOOL_DEFAULT_PROMPT: &str = "Analyze this PDF document.";
const PDF_TOOL_MAX_PDFS: usize = 10;
const PDF_TOOL_DEFAULT_MAX_BYTES_MB: u64 = 10;
const PDF_TOOL_DEFAULT_MAX_PAGES: usize = 20;
const PDF_TOOL_DEFAULT_ANTHROPIC_MODEL: &str = "anthropic/claude-opus-4-6";
const PDF_TOOL_DEFAULT_GOOGLE_MODEL: &str = "google/gemini-2.5-pro";
const PDF_TOOL_DEFAULT_OPENAI_MODEL: &str = "openai/gpt-5.4-mini";

fn pdf_tool_default_model_preferences() -> [&'static str; 3] {
    [
        PDF_TOOL_DEFAULT_ANTHROPIC_MODEL,
        PDF_TOOL_DEFAULT_GOOGLE_MODEL,
        PDF_TOOL_DEFAULT_OPENAI_MODEL,
    ]
}

fn pdf_tool_provider_env_candidates(provider: &str) -> &'static [&'static str] {
    match normalize_pdf_tool_provider(provider).as_str() {
        "anthropic" => pdf_native_provider_env_candidates("anthropic"),
        "google" => pdf_native_provider_env_candidates("google"),
        "openai" => &["OPENAI_API_KEY"],
        _ => &[],
    }
}

fn normalize_pdf_tool_provider(raw: &str) -> String {
    match clean_text(raw, 80).to_ascii_lowercase().as_str() {
        "anthropic" | "claude" => "anthropic".to_string(),
        "google" | "gemini" => "google".to_string(),
        "openai" | "gpt" => "openai".to_string(),
        other => other.to_string(),
    }
}

fn pdf_tool_provider_supports_native_pdf(provider: &str) -> bool {
    matches!(
        normalize_pdf_tool_provider(provider).as_str(),
        "anthropic" | "google"
    )
}

fn pdf_tool_model_supports_document_input(provider: &str) -> bool {
    pdf_tool_provider_supports_native_pdf(provider)
}

fn parse_pdf_tool_model_ref(raw: &str) -> Option<(String, String)> {
    let cleaned = clean_text(raw, 160);
    let (provider_raw, model_raw) = cleaned.split_once('/')?;
    let provider = normalize_pdf_tool_provider(provider_raw);
    let model_id = clean_text(model_raw, 160);
    if provider.is_empty() || model_id.is_empty() {
        None
    } else {
        Some((provider, model_id))
    }
}

fn request_has_explicit_api_key(request: &Value) -> bool {
    let direct = normalize_request_string(request, "api_key", &["apiKey"], 600);
    if !direct.is_empty() {
        return true;
    }
    let env_name = normalize_request_string(request, "api_key_env", &["apiKeyEnv"], 160);
    !env_name.is_empty()
        && std::env::var(&env_name)
            .ok()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
}

fn provider_has_pdf_tool_env_auth(provider: &str) -> bool {
    pdf_tool_provider_env_candidates(provider).iter().any(|candidate| {
        std::env::var(candidate)
            .ok()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
    })
}

fn provider_has_pdf_tool_auth_for_request(provider: &str, request: &Value) -> bool {
    request_has_explicit_api_key(request) || provider_has_pdf_tool_env_auth(provider)
}

fn default_pdf_tool_model_candidates() -> Vec<String> {
    let mut rows = Vec::<String>::new();
    if provider_has_pdf_tool_env_auth("anthropic") {
        rows.push(PDF_TOOL_DEFAULT_ANTHROPIC_MODEL.to_string());
    }
    if provider_has_pdf_tool_env_auth("google") {
        rows.push(PDF_TOOL_DEFAULT_GOOGLE_MODEL.to_string());
    }
    if provider_has_pdf_tool_env_auth("openai") {
        rows.push(PDF_TOOL_DEFAULT_OPENAI_MODEL.to_string());
    }
    rows
}

fn request_string_list(
    request: &Value,
    key: &str,
    aliases: &[&str],
    max_len: usize,
) -> Vec<String> {
    let mut rows = Vec::<String>::new();
    for name in std::iter::once(key).chain(aliases.iter().copied()) {
        let Some(value) = request.get(name) else {
            continue;
        };
        match value {
            Value::Array(items) => {
                for item in items {
                    if let Some(raw) = item.as_str() {
                        let cleaned = clean_text(raw, max_len);
                        if !cleaned.is_empty() {
                            rows.push(cleaned);
                        }
                    }
                }
            }
            Value::String(raw) => {
                for part in raw.split(',') {
                    let cleaned = clean_text(part, max_len);
                    if !cleaned.is_empty() {
                        rows.push(cleaned);
                    }
                }
            }
            _ => {}
        }
        if !rows.is_empty() {
            break;
        }
    }
    rows
}

fn dedupe_trimmed_rows(rows: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::<String>::new();
    let mut deduped = Vec::<String>::new();
    for row in rows {
        let trimmed = row.trim().to_string();
        if trimmed.is_empty() || seen.contains(&trimmed) {
            continue;
        }
        seen.insert(trimmed.clone());
        deduped.push(trimmed);
    }
    deduped
}

fn resolve_pdf_tool_inputs(request: &Value) -> Result<Vec<String>, Value> {
    let mut inputs = normalize_media_reference_inputs(request, "pdf", "pdfs", PDF_TOOL_MAX_PDFS, "PDF inputs")?;
    inputs.extend(request_string_list(request, "sources", &[], 4000));
    let path = normalize_request_string(request, "path", &[], 4000);
    if !path.is_empty() {
        inputs.push(path);
    }
    let url = normalize_request_string(request, "url", &[], 4000);
    if !url.is_empty() {
        inputs.push(url);
    }
    let deduped = normalize_media_reference_candidates(inputs, PDF_TOOL_MAX_PDFS, "PDF inputs")?;
    if deduped.is_empty() {
        Err(json!({
            "ok": false,
            "type": "web_conduit_pdf_tool",
            "error": "pdf_required",
            "reason": "provide pdf, pdfs, path, url, or sources"
        }))
    } else {
        Ok(deduped)
    }
}

fn parse_pdf_tool_page_range(raw: &str, max_pages: usize) -> Result<Vec<u32>, String> {
    let mut pages = std::collections::BTreeSet::<u32>::new();
    for token in raw.split(',').map(str::trim).filter(|token| !token.is_empty()) {
        if let Some((start_raw, end_raw)) = token.split_once('-') {
            let start = start_raw
                .trim()
                .parse::<u32>()
                .map_err(|_| format!("Invalid page range: \"{token}\""))?;
            let end = end_raw
                .trim()
                .parse::<u32>()
                .map_err(|_| format!("Invalid page range: \"{token}\""))?;
            if start == 0 || end < start {
                return Err(format!("Invalid page range: \"{token}\""));
            }
            let bounded_end = end.min(max_pages as u32);
            for page in start..=bounded_end {
                pages.insert(page);
            }
        } else {
            let page = token
                .parse::<u32>()
                .map_err(|_| format!("Invalid page number: \"{token}\""))?;
            if page == 0 {
                return Err(format!("Invalid page number: \"{token}\""));
            }
            if page <= max_pages as u32 {
                pages.insert(page);
            }
        }
    }
    Ok(pages.into_iter().collect())
}

fn resolve_pdf_tool_model_plan(request: &Value) -> Value {
    let explicit_model = normalize_request_string(request, "model", &[], 160);
    let explicit_provider =
        normalize_request_string(request, "provider", &["model_provider", "modelProvider"], 40);
    let explicit_model_id =
        normalize_request_string(request, "model_id", &["modelId"], 160);
    let fallback_models =
        request_string_list(request, "fallback_models", &["fallbackModels"], 160);
    let mut candidates = Vec::<String>::new();
    let selection_scope = if !explicit_model.is_empty() {
        candidates.push(explicit_model.clone());
        "request_model"
    } else if !explicit_provider.is_empty() || !explicit_model_id.is_empty() {
        if !explicit_provider.is_empty() && !explicit_model_id.is_empty() {
            candidates.push(format!(
                "{}/{}",
                normalize_pdf_tool_provider(&explicit_provider),
                explicit_model_id
            ));
        }
        "request_provider_model_id"
    } else {
        candidates.extend(default_pdf_tool_model_candidates());
        "auto_env"
    };
    candidates.extend(fallback_models.clone());
    let candidates = dedupe_trimmed_rows(candidates);
    let allow_fallback = selection_scope == "auto_env" || !fallback_models.is_empty();
    let mut parsed_candidates = Vec::<Value>::new();
    let mut selected_index = None::<usize>;
    for candidate in &candidates {
        if let Some((provider, model_id)) = parse_pdf_tool_model_ref(candidate) {
            let credential_available = if selection_scope == "auto_env" {
                provider_has_pdf_tool_env_auth(&provider)
            } else {
                provider_has_pdf_tool_auth_for_request(&provider, request)
            };
            let row = json!({
                "model": candidate,
                "provider": provider,
                "model_id": model_id,
                "credential_available": credential_available,
                "native_supported": pdf_tool_provider_supports_native_pdf(candidate.split('/').next().unwrap_or("")),
                "supports_document_input": pdf_tool_model_supports_document_input(candidate.split('/').next().unwrap_or(""))
            });
            if selected_index.is_none() && (!allow_fallback || credential_available) {
                selected_index = Some(parsed_candidates.len());
            }
            parsed_candidates.push(row);
        } else {
            parsed_candidates.push(json!({
                "model": candidate,
                "error": "invalid_model_ref"
            }));
        }
    }
    if selected_index.is_none() && !parsed_candidates.is_empty() {
        selected_index = parsed_candidates.iter().position(|row| row.get("provider").is_some());
    }
    let selected = selected_index
        .and_then(|index| parsed_candidates.get(index).cloned())
        .unwrap_or(Value::Null);
    let selection_fallback_reason = if allow_fallback {
        selected_index
            .filter(|index| *index > 0)
            .map(|_| Value::String("credential_unavailable".to_string()))
            .unwrap_or(Value::Null)
    } else {
        Value::Null
    };
    let selected_provider = selected
        .get("provider")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let selected_model_id = selected
        .get("model_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let selected_model = selected
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let native_supported = selected
        .get("native_supported")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let credential_available = selected
        .get("credential_available")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    json!({
        "selection_scope": selection_scope,
        "allow_fallback": allow_fallback,
        "default_model_preferences": pdf_tool_default_model_preferences(),
        "candidates": parsed_candidates,
        "primary": selected_model,
        "provider": selected_provider,
        "model_id": selected_model_id,
        "credential_available": credential_available,
        "native_supported": native_supported,
        "supports_document_input": native_supported,
        "execution_mode": if native_supported { "native_provider" } else { "extraction_only_fallback" },
        "selection_fallback_reason": selection_fallback_reason
    })
}
