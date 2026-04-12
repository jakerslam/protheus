fn print_json_error_and_exit(kind: &str, error: &str, reason: &str) -> i32 {
    println!(
        "{}",
        json!({
            "ok": false,
            "type": kind,
            "error": error,
            "reason": clean_text(reason, 240)
        })
    );
    1
}

fn build_pdf_extract_cli_request(parsed: &crate::ParsedArgs) -> Value {
    let mut request = cli_media_request_from_parsed(parsed);
    if let Some(obj) = request.as_object_mut() {
        obj.insert(
            "max_pages".to_string(),
            json!(parse_u64(
                parsed
                    .flags
                    .get("max-pages")
                    .or_else(|| parsed.flags.get("max_pages")),
                DEFAULT_PDF_EXTRACT_MAX_PAGES as u64,
                1,
                32
            )),
        );
        obj.insert(
            "min_text_chars".to_string(),
            json!(parse_u64(
                parsed
                    .flags
                    .get("min-text-chars")
                    .or_else(|| parsed.flags.get("min_text_chars")),
                DEFAULT_PDF_MIN_TEXT_CHARS as u64,
                0,
                20_000
            )),
        );
        obj.insert(
            "extract_images".to_string(),
            json!(
                parse_bool(parsed.flags.get("extract-images"))
                    || parse_bool(parsed.flags.get("extract_images"))
            ),
        );
        obj.insert(
            "page_numbers".to_string(),
            json!(clean_text(
                parsed
                    .flags
                    .get("page-numbers")
                    .or_else(|| parsed.flags.get("page_numbers"))
                    .map(String::as_str)
                    .unwrap_or(""),
                200
            )),
        );
    }
    request
}

fn build_pdf_native_cli_request(parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let sources = parse_json_flag(
        parsed
            .flags
            .get("sources-json")
            .or_else(|| parsed.flags.get("sources_json")),
    )?;
    let mut request = cli_media_request_from_parsed(parsed);
    if let Some(obj) = request.as_object_mut() {
        obj.insert(
            "provider".to_string(),
            json!(clean_text(
                parsed
                    .flags
                    .get("provider")
                    .map(String::as_str)
                    .unwrap_or(""),
                40
            )),
        );
        obj.insert(
            "model_id".to_string(),
            json!(clean_text(
                parsed
                    .flags
                    .get("model-id")
                    .or_else(|| parsed.flags.get("model_id"))
                    .map(String::as_str)
                    .unwrap_or(""),
                160
            )),
        );
        obj.insert(
            "prompt".to_string(),
            json!(clean_text(
                parsed.flags.get("prompt").map(String::as_str).unwrap_or(""),
                4000
            )),
        );
        obj.insert(
            "api_key".to_string(),
            json!(clean_text(
                parsed
                    .flags
                    .get("api-key")
                    .or_else(|| parsed.flags.get("api_key"))
                    .map(String::as_str)
                    .unwrap_or(""),
                600
            )),
        );
        obj.insert(
            "api_key_env".to_string(),
            json!(clean_text(
                parsed
                    .flags
                    .get("api-key-env")
                    .or_else(|| parsed.flags.get("api_key_env"))
                    .map(String::as_str)
                    .unwrap_or(""),
                160
            )),
        );
        obj.insert(
            "base_url".to_string(),
            json!(clean_text(
                parsed
                    .flags
                    .get("base-url")
                    .or_else(|| parsed.flags.get("base_url"))
                    .map(String::as_str)
                    .unwrap_or(""),
                2200
            )),
        );
        obj.insert(
            "max_tokens".to_string(),
            json!(parse_u64(
                parsed
                    .flags
                    .get("max-tokens")
                    .or_else(|| parsed.flags.get("max_tokens")),
                4096,
                1,
                32_000
            )),
        );
        obj.insert("sources".to_string(), sources);
    }
    Ok(request)
}

fn run_document_cli_command(
    root: &Path,
    command: &str,
    parsed: &crate::ParsedArgs,
) -> Result<Option<Value>, i32> {
    match command {
        "audio-probe" => Ok(Some(api_audio_probe(root, &cli_media_request_from_parsed(parsed)))),
        "pdf-extract" => Ok(Some(api_pdf_extract(root, &build_pdf_extract_cli_request(parsed)))),
        "pdf-native-analyze" => match build_pdf_native_cli_request(parsed) {
            Ok(request) => Ok(Some(api_pdf_native_analyze(root, &request))),
            Err(err) => Err(print_json_error_and_exit(
                "web_conduit_pdf_native_provider",
                "invalid_sources_json",
                &err,
            )),
        },
        _ => Ok(None),
    }
}
