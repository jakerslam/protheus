fn build_image_tool_status_cli_request(parsed: &crate::ParsedArgs) -> Value {
    json!({
        "provider": clean_text(
            parsed
                .flags
                .get("provider")
                .map(String::as_str)
                .unwrap_or(""),
            80
        ),
        "model": clean_text(
            parsed
                .flags
                .get("model")
                .map(String::as_str)
                .unwrap_or(""),
            240
        ),
        "summary_only": parse_bool(parsed.flags.get("summary-only"))
            || parse_bool(parsed.flags.get("summary_only"))
    })
}

fn build_media_attachments_cli_request(parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let context = parse_json_flag(
        parsed
            .flags
            .get("context-json")
            .or_else(|| parsed.flags.get("context_json"))
            .or_else(|| parsed.flags.get("ctx-json"))
            .or_else(|| parsed.flags.get("ctx_json")),
    )?;
    let attachments = parse_json_flag(
        parsed
            .flags
            .get("attachments-json")
            .or_else(|| parsed.flags.get("attachments_json")),
    )?;
    let media_paths = parse_json_flag(
        parsed
            .flags
            .get("media-paths-json")
            .or_else(|| parsed.flags.get("media_paths_json")),
    )?;
    let media_urls = parse_json_flag(
        parsed
            .flags
            .get("media-urls-json")
            .or_else(|| parsed.flags.get("media_urls_json")),
    )?;
    let media_types = parse_json_flag(
        parsed
            .flags
            .get("media-types-json")
            .or_else(|| parsed.flags.get("media_types_json")),
    )?;
    let mut request = if context.is_null() { json!({}) } else { context };
    let Some(obj) = request.as_object_mut() else {
        return Err("context_json_must_be_object".to_string());
    };
    if !attachments.is_null() {
        obj.insert("attachments".to_string(), attachments);
    }
    if !media_paths.is_null() {
        obj.insert("MediaPaths".to_string(), media_paths);
    }
    if !media_urls.is_null() {
        obj.insert("MediaUrls".to_string(), media_urls);
    }
    if !media_types.is_null() {
        obj.insert("MediaTypes".to_string(), media_types);
    }
    let media_path = parsed
        .flags
        .get("media-path")
        .or_else(|| parsed.flags.get("media_path"))
        .or_else(|| parsed.flags.get("path"))
        .map(String::as_str)
        .unwrap_or("");
    if !media_path.is_empty() {
        obj.insert("MediaPath".to_string(), json!(trim_preserve_text(media_path, 4000)));
    }
    let media_url = parsed
        .flags
        .get("media-url")
        .or_else(|| parsed.flags.get("media_url"))
        .or_else(|| parsed.flags.get("url"))
        .map(String::as_str)
        .unwrap_or("");
    if !media_url.is_empty() {
        obj.insert("MediaUrl".to_string(), json!(trim_preserve_text(media_url, 4000)));
    }
    let media_type = parsed
        .flags
        .get("media-type")
        .or_else(|| parsed.flags.get("media_type"))
        .map(String::as_str)
        .unwrap_or("");
    if !media_type.is_empty() {
        obj.insert("MediaType".to_string(), json!(trim_preserve_text(media_type, 160)));
    }
    let already = parsed
        .flags
        .get("already-transcribed-indices")
        .or_else(|| parsed.flags.get("already_transcribed_indices"))
        .map(String::as_str)
        .unwrap_or("");
    if !already.is_empty() {
        obj.insert(
            "already_transcribed_indices".to_string(),
            json!(trim_preserve_text(already, 240)),
        );
    }
    obj.insert(
        "capability".to_string(),
        json!(clean_text(
            parsed
                .flags
                .get("capability")
                .map(String::as_str)
                .unwrap_or(""),
            24
        )),
    );
    obj.insert(
        "prefer".to_string(),
        json!(clean_text(
            parsed
                .flags
                .get("prefer")
                .map(String::as_str)
                .unwrap_or(""),
            24
        )),
    );
    obj.insert(
        "mode".to_string(),
        json!(clean_text(
            parsed
                .flags
                .get("mode")
                .map(String::as_str)
                .unwrap_or(""),
            24
        )),
    );
    obj.insert(
        "max_attachments".to_string(),
        json!(parse_u64(
            parsed
                .flags
                .get("max-attachments")
                .or_else(|| parsed.flags.get("max_attachments")),
            1,
            1,
            32
        )),
    );
    obj.insert(
        "summary_only".to_string(),
        json!(parse_bool(parsed.flags.get("summary-only"))
            || parse_bool(parsed.flags.get("summary_only"))),
    );
    Ok(request)
}
