fn web_media_pdf_tool_contract() -> Value {
    json!({
        "input_fields": ["prompt", "pdf", "pdfs", "path", "url", "pages", "model", "provider", "model_id", "max_bytes_mb", "max_pages", "min_text_chars"],
        "default_prompt": PDF_TOOL_DEFAULT_PROMPT,
        "max_pdfs": PDF_TOOL_MAX_PDFS,
        "default_max_bytes_mb": PDF_TOOL_DEFAULT_MAX_BYTES_MB,
        "default_max_pages": PDF_TOOL_DEFAULT_MAX_PAGES,
        "native_providers": ["anthropic", "google"],
        "default_model_preferences": pdf_tool_default_model_preferences(),
        "page_range_contract": {
            "format": "1-3,5,7-9",
            "pages_with_native_provider_supported": false
        },
        "non_native_execution_mode": "extraction_only_fallback",
        "returns": ["analysis", "native", "analysis_mode", "model_selection", "source_count"]
    })
}

fn append_web_media_pdf_tool_entry(tool_catalog: &mut Value, policy: &Value) {
    if let Some(rows) = tool_catalog.as_array_mut() {
        rows.push(json!({
            "tool": "web_media_pdf_tool",
            "label": "Web Media PDF Tool",
            "family": "media",
            "enabled": policy.pointer("/web_conduit/enabled").and_then(Value::as_bool).unwrap_or(true),
            "request_contract": web_media_pdf_tool_contract()
        }));
    }
}

fn build_pdf_tool_source_request(request: &Value, raw_source: &str, max_bytes: u64) -> Value {
    let mut next = request.clone();
    if let Some(obj) = next.as_object_mut() {
        obj.remove("pdf");
        obj.remove("pdfs");
        obj.remove("sources");
        obj.remove("pages");
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

fn combine_pdf_extraction_text(rows: &[Value]) -> String {
    let mut combined = String::new();
    for (index, row) in rows.iter().enumerate() {
        let text = row.get("text").and_then(Value::as_str).unwrap_or("").trim();
        if text.is_empty() {
            continue;
        }
        if rows.len() > 1 {
            combined.push_str(&format!("[PDF {} text]\n", index + 1));
        }
        combined.push_str(text);
        if !combined.ends_with('\n') {
            combined.push('\n');
        }
        combined.push('\n');
    }
    normalize_block_text(&combined)
}

fn api_pdf_tool(root: &Path, request: &Value) -> Value {
    let contract = web_media_pdf_tool_contract();
    let (prompt, _model_override) =
        resolve_media_tool_prompt_and_model_override(request, PDF_TOOL_DEFAULT_PROMPT);
    let max_bytes_mb = parse_fetch_u64(
        request
            .get("max_bytes_mb")
            .or_else(|| request.get("maxBytesMb"))
            .or_else(|| request.get("max_bytes_mb_default")),
        PDF_TOOL_DEFAULT_MAX_BYTES_MB,
        1,
        50,
    );
    let max_pages = parse_fetch_u64(
        request
            .get("max_pages")
            .or_else(|| request.get("maxPages")),
        PDF_TOOL_DEFAULT_MAX_PAGES as u64,
        1,
        32,
    ) as usize;
    let min_text_chars = parse_fetch_u64(
        request
            .get("min_text_chars")
            .or_else(|| request.get("minTextChars")),
        DEFAULT_PDF_MIN_TEXT_CHARS as u64,
        0,
        20_000,
    ) as usize;
    let summary_only = media_tool_read_boolean_param(request, "summary_only").unwrap_or(false);
    let pdf_inputs = match resolve_pdf_tool_inputs(request) {
        Ok(rows) => rows,
        Err(mut err) => {
            err["pdf_tool_contract"] = contract;
            return err;
        }
    };
    if pdf_inputs.len() > PDF_TOOL_MAX_PDFS {
        return json!({
            "ok": false,
            "type": "web_conduit_pdf_tool",
            "error": "too_many_pdfs",
            "count": pdf_inputs.len(),
            "max": PDF_TOOL_MAX_PDFS,
            "pdf_tool_contract": contract
        });
    }
    let pages_raw = normalize_request_string(request, "pages", &[], 200);
    let page_numbers = if !pages_raw.is_empty() {
        match parse_pdf_tool_page_range(&pages_raw, max_pages) {
            Ok(rows) => rows,
            Err(err) => {
                return json!({
                    "ok": false,
                    "type": "web_conduit_pdf_tool",
                    "error": "invalid_page_range",
                    "reason": clean_text(&err, 240),
                    "pdf_tool_contract": contract
                });
            }
        }
    } else {
        parse_pdf_page_numbers(request, max_pages)
    };
    let model_selection = resolve_pdf_tool_model_plan(request);
    let provider = model_selection
        .get("provider")
        .and_then(Value::as_str)
        .unwrap_or("");
    let model_id = model_selection
        .get("model_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    let primary_model = model_selection
        .get("primary")
        .and_then(Value::as_str)
        .unwrap_or("");
    let native_supported = model_selection
        .get("native_supported")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let credential_available = model_selection
        .get("credential_available")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if primary_model.is_empty() {
        return json!({
            "ok": false,
            "type": "web_conduit_pdf_tool",
            "error": "model_unavailable",
            "reason": "no usable PDF tool model is configured or credentialed",
            "model_selection": model_selection,
            "pdf_tool_contract": contract
        });
    }
    if !credential_available {
        return json!({
            "ok": false,
            "type": "web_conduit_pdf_tool",
            "error": "api_key_required",
            "provider": provider,
            "model": primary_model,
            "model_selection": model_selection,
            "pdf_tool_contract": contract
        });
    }
    let max_bytes = max_bytes_mb.saturating_mul(1024 * 1024);
    if native_supported {
        if !page_numbers.is_empty() {
            return json!({
                "ok": false,
                "type": "web_conduit_pdf_tool",
                "error": "pages_not_supported_with_native_provider",
                "provider": provider,
                "model": primary_model,
                "page_numbers": page_numbers,
                "model_selection": model_selection,
                "pdf_tool_contract": contract
            });
        }
        let mut native_request = request.clone();
        if let Some(obj) = native_request.as_object_mut() {
            obj.insert("provider".to_string(), json!(provider));
            obj.insert("model_id".to_string(), json!(model_id));
            obj.insert("prompt".to_string(), json!(prompt));
            obj.insert("sources".to_string(), json!(pdf_inputs));
            obj.insert("summary_only".to_string(), json!(summary_only));
            obj.insert(
                "max_tokens".to_string(),
                json!(parse_fetch_u64(
                    request.get("max_tokens").or_else(|| request.get("maxTokens")),
                    4096,
                    1,
                    32_000
                )),
            );
        }
        let native_out = api_pdf_native_analyze(root, &native_request);
        if !native_out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            return json!({
                "ok": false,
                "type": "web_conduit_pdf_tool",
                "error": "native_provider_failed",
                "provider": provider,
                "model": primary_model,
                "source_count": pdf_inputs.len(),
                "model_selection": model_selection,
                "native_result": native_out,
                "pdf_tool_contract": contract
            });
        }
        let analysis = native_out
            .get("analysis")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        return json!({
            "ok": true,
            "type": "web_conduit_pdf_tool",
            "prompt": prompt,
            "analysis": analysis,
            "summary": native_out.get("summary").cloned().unwrap_or_else(|| json!(format!("Native PDF tool analysis returned {} characters across {} PDF input(s).", analysis.chars().count(), pdf_inputs.len()))),
            "native": true,
            "analysis_mode": "native_provider",
            "provider": provider,
            "model": primary_model,
            "model_id": model_id,
            "source_count": pdf_inputs.len(),
            "pdf_inputs": pdf_inputs,
            "model_selection": model_selection,
            "native_result": native_out,
            "pdf_tool_contract": contract
        });
    }
    let mut extracted_parts = Vec::<Value>::new();
    for source in &pdf_inputs {
        let mut extract_request = build_pdf_tool_source_request(request, source, max_bytes);
        if let Some(obj) = extract_request.as_object_mut() {
            obj.insert("summary_only".to_string(), json!(false));
            obj.insert("max_pages".to_string(), json!(max_pages));
            obj.insert("min_text_chars".to_string(), json!(min_text_chars));
            if !page_numbers.is_empty() {
                obj.insert("page_numbers".to_string(), json!(page_numbers));
            }
        }
        let extracted = api_pdf_extract(root, &extract_request);
        if !extracted.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            return json!({
                "ok": false,
                "type": "web_conduit_pdf_tool",
                "error": "pdf_extract_failed",
                "provider": provider,
                "model": primary_model,
                "source": source,
                "model_selection": model_selection,
                "extract_result": extracted,
                "pdf_tool_contract": contract
            });
        }
        extracted_parts.push(extracted);
    }
    let analysis_full = combine_pdf_extraction_text(&extracted_parts);
    if analysis_full.is_empty() {
        return json!({
            "ok": false,
            "type": "web_conduit_pdf_tool",
            "error": "no_extractable_text",
            "provider": provider,
            "model": primary_model,
            "model_selection": model_selection,
            "pdf_tool_contract": contract
        });
    }
    let analysis = if summary_only {
        analysis_full.chars().take(1200).collect::<String>()
    } else {
        analysis_full.clone()
    };
    let total_text_chars = extracted_parts
        .iter()
        .map(|row| row.get("text_chars").and_then(Value::as_u64).unwrap_or(0))
        .sum::<u64>();
    json!({
        "ok": true,
        "type": "web_conduit_pdf_tool",
        "prompt": prompt,
        "analysis": analysis,
        "summary": format!("PDF tool used extracted-text fallback for {} PDF input(s).", pdf_inputs.len()),
        "native": false,
        "analysis_mode": "extraction_only_fallback",
        "provider": provider,
        "model": primary_model,
        "model_id": model_id,
        "page_numbers": page_numbers,
        "source_count": pdf_inputs.len(),
        "pdf_inputs": pdf_inputs,
        "total_text_chars": total_text_chars,
        "model_selection": model_selection,
        "extract_results": extracted_parts,
        "pdf_tool_contract": contract
    })
}
