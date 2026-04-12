fn media_content_disposition_filename(header: &str) -> Option<String> {
    let raw = clean_text(header, 600);
    if raw.is_empty() {
        return None;
    }
    let lowered = raw.to_ascii_lowercase();
    if let Some(idx) = lowered.find("filename*=") {
        let encoded = raw[idx + "filename*=".len()..]
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .trim_matches('"')
            .trim_matches('\'');
        let decoded = encoded.split("''").nth(1).unwrap_or(encoded);
        let name = percent_decode_urlish(decoded);
        let base = Path::new(name.trim())
            .file_name()?
            .to_str()?
            .trim()
            .to_string();
        if !base.is_empty() {
            return Some(base);
        }
    }
    if let Some(idx) = lowered.find("filename=") {
        let name = raw[idx + "filename=".len()..]
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .trim_matches('"')
            .trim_matches('\'');
        let base = Path::new(name).file_name()?.to_str()?.trim().to_string();
        if !base.is_empty() {
            return Some(base);
        }
    }
    None
}

fn media_bytes_look_binary(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    let probe_len = bytes.len().min(4096);
    let sample = &bytes[..probe_len];
    if sample.iter().any(|byte| *byte == 0) {
        return true;
    }
    let control_count = sample
        .iter()
        .filter(|byte| {
            let b = **byte;
            b < 9 || (b > 13 && b < 32)
        })
        .count();
    let control_ratio = control_count as f64 / probe_len as f64;
    if control_ratio > 0.12 {
        return true;
    }
    std::str::from_utf8(sample).is_err() && control_ratio > 0.04
}

fn normalize_media_content_type(raw: &str) -> String {
    clean_text(raw, 120)
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
}

fn media_is_generic_content_type(content_type: &str) -> bool {
    matches!(
        normalize_media_content_type(content_type).as_str(),
        "" | "application/octet-stream" | "application/zip"
    )
}

fn media_mime_type_from_extension(ext: &str) -> Option<&'static str> {
    match ext {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "svg" => Some("image/svg+xml"),
        "heic" => Some("image/heic"),
        "heif" => Some("image/heif"),
        "mp3" => Some("audio/mpeg"),
        "wav" => Some("audio/wav"),
        "ogg" | "oga" => Some("audio/ogg"),
        "flac" => Some("audio/flac"),
        "aac" => Some("audio/aac"),
        "opus" => Some("audio/opus"),
        "m4a" => Some("audio/x-m4a"),
        "mp4" => Some("video/mp4"),
        "mov" => Some("video/quicktime"),
        "pdf" => Some("application/pdf"),
        "doc" => Some("application/msword"),
        "docx" => Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
        "xls" => Some("application/vnd.ms-excel"),
        "xlsx" => Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
        "ppt" => Some("application/vnd.ms-powerpoint"),
        "pptx" => Some("application/vnd.openxmlformats-officedocument.presentationml.presentation"),
        "csv" => Some("text/csv"),
        "html" | "htm" => Some("text/html"),
        "txt" => Some("text/plain"),
        "md" => Some("text/markdown"),
        "json" => Some("application/json"),
        "xml" => Some("text/xml"),
        "css" => Some("text/css"),
        "js" => Some("text/javascript"),
        "zip" => Some("application/zip"),
        "gz" => Some("application/gzip"),
        "tar" => Some("application/x-tar"),
        "7z" => Some("application/x-7z-compressed"),
        "rar" => Some("application/vnd.rar"),
        _ => None,
    }
}

fn media_mime_type_from_file_name(file_name: Option<&str>) -> Option<&'static str> {
    let ext = file_name
        .and_then(|row| Path::new(row).extension().and_then(|ext| ext.to_str()))
        .unwrap_or("")
        .to_ascii_lowercase();
    media_mime_type_from_extension(&ext)
}

fn media_sniff_content_type(bytes: &[u8]) -> String {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return "image/png".to_string();
    }
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        return "image/jpeg".to_string();
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return "image/gif".to_string();
    }
    if bytes.len() > 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return "image/webp".to_string();
    }
    if bytes.starts_with(b"%PDF-") {
        return "application/pdf".to_string();
    }
    if bytes.len() > 12 && bytes.get(4..8) == Some(b"ftyp") {
        return "video/mp4".to_string();
    }
    if bytes.starts_with(b"OggS") {
        return "audio/ogg".to_string();
    }
    if bytes.starts_with(b"ID3") {
        return "audio/mpeg".to_string();
    }
    if bytes.len() > 4 && &bytes[..4] == b"RIFF" && bytes.get(8..12) == Some(b"WAVE") {
        return "audio/wav".to_string();
    }
    let probe = String::from_utf8_lossy(&bytes[..bytes.len().min(512)]).to_ascii_lowercase();
    if probe.contains("<svg") {
        return "image/svg+xml".to_string();
    }
    if media_bytes_look_binary(bytes) {
        "application/octet-stream".to_string()
    } else {
        "text/plain".to_string()
    }
}

fn media_guess_content_type(
    file_name: Option<&str>,
    bytes: &[u8],
    header_content_type: Option<&str>,
) -> String {
    let header = header_content_type
        .map(normalize_media_content_type)
        .unwrap_or_default();
    if !media_is_generic_content_type(&header) {
        return header;
    }
    let sniffed = media_sniff_content_type(bytes);
    if !media_is_generic_content_type(&sniffed) && sniffed != "text/plain" {
        return sniffed;
    }
    if let Some(ext_mime) = media_mime_type_from_file_name(file_name) {
        return ext_mime.to_string();
    }
    if !header.is_empty() {
        return header;
    }
    sniffed
}

fn media_kind_from_content_type(content_type: &str) -> String {
    let normalized = normalize_media_content_type(content_type);
    if normalized.starts_with("image/") {
        "image".to_string()
    } else if normalized.starts_with("audio/") {
        "audio".to_string()
    } else if normalized.starts_with("video/") {
        "video".to_string()
    } else if normalized == "application/pdf"
        || normalized == "application/json"
        || normalized == "application/xml"
        || normalized == "text/javascript"
        || normalized.contains("msword")
        || normalized.contains("ms-excel")
        || normalized.contains("ms-powerpoint")
        || normalized.contains("spreadsheetml")
        || normalized.contains("presentationml")
        || normalized.contains("wordprocessingml")
        || normalized.starts_with("text/")
    {
        "document".to_string()
    } else {
        "unknown".to_string()
    }
}

fn media_extension_for_content_type(content_type: &str) -> Option<&'static str> {
    match normalize_media_content_type(content_type).as_str() {
        "image/png" => Some("png"),
        "image/jpeg" => Some("jpg"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        "image/svg+xml" => Some("svg"),
        "image/heic" => Some("heic"),
        "image/heif" => Some("heif"),
        "audio/mpeg" => Some("mp3"),
        "audio/wav" => Some("wav"),
        "audio/ogg" => Some("ogg"),
        "audio/flac" => Some("flac"),
        "audio/aac" => Some("aac"),
        "audio/opus" => Some("opus"),
        "audio/x-m4a" | "audio/mp4" => Some("m4a"),
        "video/mp4" => Some("mp4"),
        "video/quicktime" => Some("mov"),
        "application/pdf" => Some("pdf"),
        "application/json" => Some("json"),
        "application/xml" => Some("xml"),
        "application/zip" => Some("zip"),
        "application/gzip" => Some("gz"),
        "application/x-tar" => Some("tar"),
        "application/x-7z-compressed" => Some("7z"),
        "application/vnd.rar" => Some("rar"),
        "text/html" => Some("html"),
        "text/plain" => Some("txt"),
        "text/markdown" => Some("md"),
        "text/csv" => Some("csv"),
        "text/xml" => Some("xml"),
        "text/css" => Some("css"),
        "text/javascript" => Some("js"),
        _ => None,
    }
}

fn media_file_name_from_url(raw_url: &str) -> Option<String> {
    let clean = clean_text(raw_url, 2200);
    let tail = clean
        .split(['?', '#'])
        .next()
        .unwrap_or("")
        .trim_end_matches('/');
    let base = tail.rsplit('/').next().unwrap_or("").trim();
    if base.is_empty() {
        None
    } else {
        Some(percent_decode_urlish(base))
    }
}

fn redact_media_locator(raw: &str) -> String {
    let mut out = clean_text(raw, 2200);
    for key in ["token=", "api_key=", "apikey=", "key=", "auth="] {
        loop {
            let lowered = out.to_ascii_lowercase();
            let Some(idx) = lowered.find(key) else {
                break;
            };
            let start = idx + key.len();
            let end = out[start..]
                .find(['&', '#'])
                .map(|offset| start + offset)
                .unwrap_or(out.len());
            out.replace_range(start..end, "[REDACTED]");
        }
    }
    out
}

fn host_read_media_allowed(sniffed_content_type: &str) -> bool {
    let normalized = normalize_media_content_type(sniffed_content_type);
    normalized.starts_with("image/")
        || normalized.starts_with("audio/")
        || normalized.starts_with("video/")
        || matches!(
            normalized.as_str(),
            "application/pdf"
                | "application/msword"
                | "application/vnd.ms-excel"
                | "application/vnd.ms-powerpoint"
                | "application/vnd.openxmlformats-officedocument.presentationml.presentation"
                | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
                | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        )
}

fn media_policy_denies_read(policy: &Value) -> bool {
    policy
        .get("read_allowed")
        .and_then(Value::as_bool)
        .is_some_and(|allowed| !allowed)
        || policy
            .get("deny")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(Value::as_str)
                    .any(|row| row.eq_ignore_ascii_case("read"))
            })
            .unwrap_or(false)
}

fn media_request_read_denied_by_policy(request: &Value) -> bool {
    ["sender_tool_policy", "group_tool_policy", "tool_policy"]
        .iter()
        .filter_map(|key| request.get(*key))
        .any(media_policy_denies_read)
}

fn media_request_host_read_capability(request: &Value) -> bool {
    let requested = media_tool_read_boolean_param(request, "host_read_capability")
        .or_else(|| media_tool_read_boolean_param(request, "allow_host_read"))
        .or_else(|| {
            request
                .pointer("/media_access/host_read_capability")
                .and_then(Value::as_bool)
        })
        .or_else(|| {
            request
                .pointer("/media_access/allow_host_read")
                .and_then(Value::as_bool)
        })
        .or_else(|| {
            request
                .pointer("/mediaAccess/host_read_capability")
                .and_then(Value::as_bool)
        })
        .or_else(|| request.pointer("/mediaAccess/allowHostRead").and_then(Value::as_bool))
        .unwrap_or(false);
    requested && !media_request_read_denied_by_policy(request)
}

fn web_media_request_contract() -> Value {
    json!({
        "max_bytes_default": Value::Null,
        "default_max_bytes_rule": "kind_budget_when_unspecified",
        "max_bytes_by_kind": media_kind_budget_contract(),
        "supported_source_schemes": ["http", "https", "file", "local_path", "data"],
        "supported_file_url_hosts": ["", "localhost"],
        "workspace_relative_paths": true,
        "managed_canvas_media_prefix": "/canvas/documents/",
        "default_local_root_suffixes": media_default_local_root_suffixes(),
        "supports_wildcard_local_roots": true,
        "local_root_pattern_contract": {
            "wildcard_segment": "*",
            "requires_absolute_paths": true,
            "rejects_double_star": true
        },
        "channel_attachment_root_contract": media_channel_attachment_root_contract(),
        "workspace_dir_resolution_contract": {
            "top_level_fields": ["workspace_dir", "workspaceDir"],
            "nested_media_access_fields": ["media_access.workspace_dir", "media_access.workspaceDir", "mediaAccess.workspace_dir", "mediaAccess.workspaceDir"],
            "precedence": "top_level_over_media_access"
        },
        "host_read_policy_contract": {
            "request_fields": ["host_read_capability", "allow_host_read", "media_access.host_read_capability", "media_access.allow_host_read"],
            "deny_policy_fields": ["sender_tool_policy", "group_tool_policy", "tool_policy"],
            "deny_rule": "deny.read_or_read_allowed_false_disables_host_read_and_unbounded_local_roots"
        },
        "fail_closed_error_codes": [
            "invalid-file-url",
            "invalid-path",
            "invalid-root",
            "network-path-not-allowed",
            "not-file",
            "not-found",
            "path-not-allowed",
            "unsafe-bypass"
        ],
        "rejects_windows_network_paths": true,
        "host_read_capability_requires_sniffed_binary_or_office_document": true,
        "summary_only_default": false,
        "tool_shared_contract": web_media_tool_shared_contract(),
        "remote_request_headers_contract": {
            "top_level_fields": ["headers", "http_headers", "request_headers"],
            "string_map_only": true,
            "cross_origin_retained_headers": MEDIA_CROSS_ORIGIN_REDIRECT_SAFE_HEADERS
        },
        "remote_redirect_contract": {
            "max_redirects": MEDIA_REMOTE_MAX_REDIRECTS,
            "preserve_headers_on_same_origin": true,
            "strip_sensitive_headers_on_cross_origin": true,
            "missing_location_error": "invalid_redirect"
        },
        "prompt_image_order_contract": web_media_prompt_image_order_contract(),
        "voice_audio_contract": web_media_voice_contract(),
        "audio_probe_contract": web_media_audio_probe_contract(),
        "pdf_extract_contract": web_media_pdf_extract_contract(),
        "pdf_native_provider_contract": web_media_pdf_native_provider_contract(),
        "pdf_tool_contract": web_media_pdf_tool_contract(),
        "media_store_contract": media_store_contract(),
        "hosting_contract": web_media_host_contract(),
        "outbound_attachment_contract": web_media_outbound_attachment_contract(),
        "file_context_contract": web_media_file_context_contract(),
        "qr_image_contract": web_media_qr_contract(),
        "image_ops_contract": web_media_image_ops_contract(),
        "image_optimization_contract": web_media_image_optimization_contract()
    })
}

fn web_media_host_contract() -> Value {
    json!({
        "route_prefix": "/api/web/media/",
        "default_ttl_seconds": 120,
        "max_ttl_seconds": 3600,
        "single_use_delivery": true,
        "cleanup_after_delivery": true,
        "delivery_shape": "json_data_url",
        "absolute_url_requires_base_url": true,
        "source_contract": "same_as_web_media"
    })
}

fn web_media_parse_contract() -> Value {
    json!({
        "marker": "MEDIA:",
        "supports_audio_as_voice_tag": true,
        "audio_tag_contract": web_media_audio_tag_contract(),
        "rejects_traversal_and_home_dir_paths": true,
        "ignores_fenced_media_lines": true,
        "supports_quoted_paths_with_spaces": true,
        "returns": ["text", "media_urls", "media_url", "audio_as_voice", "had_audio_tag", "audio_delivery_mode", "segments"]
    })
}

fn append_web_media_tool_entry(tool_catalog: &mut Value, policy: &Value) {
    if let Some(rows) = tool_catalog.as_array_mut() {
        rows.push(json!({
            "tool": "web_media",
            "label": "Web Media",
            "family": "media",
            "enabled": policy.pointer("/web_conduit/enabled").and_then(Value::as_bool).unwrap_or(true),
            "default_provider": "direct_http",
            "default_provider_chain": fetch_provider_chain_from_request("", &json!({}), policy),
            "request_contract": web_media_request_contract()
        }));
        rows.push(json!({
            "tool": "web_media_host",
            "label": "Web Media Host",
            "family": "media",
            "enabled": policy.pointer("/web_conduit/enabled").and_then(Value::as_bool).unwrap_or(true),
            "request_contract": web_media_host_contract()
        }));
        rows.push(json!({
            "tool": "web_media_parse",
            "label": "Web Media Parse",
            "family": "media",
            "enabled": true,
            "request_contract": web_media_parse_contract()
        }));
        rows.push(json!({
            "tool": "web_media_file_context",
            "label": "Web Media File Context",
            "family": "media",
            "enabled": true,
            "request_contract": web_media_file_context_contract()
        }));
    }
    append_web_media_outbound_tool_entry(tool_catalog, policy);
    append_web_media_qr_tool_entry(tool_catalog, policy);
    append_web_media_image_ops_tool_entry(tool_catalog, policy);
    append_openclaw_pdf_tool_entries(tool_catalog, policy);
}
