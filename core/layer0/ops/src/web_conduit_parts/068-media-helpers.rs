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
        let base = Path::new(name.trim()).file_name()?.to_str()?.trim().to_string();
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
    if !header.is_empty() && header != "application/octet-stream" {
        return header;
    }
    let sniffed = media_sniff_content_type(bytes);
    if sniffed != "application/octet-stream" && sniffed != "text/plain" {
        return sniffed;
    }
    let ext = file_name
        .and_then(|row| Path::new(row).extension().and_then(|ext| ext.to_str()))
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "mp4" => "video/mp4",
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "html" | "htm" => "text/html",
        "txt" | "md" => "text/plain",
        _ => sniffed.as_str(),
    }
    .to_string()
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
        "audio/mpeg" => Some("mp3"),
        "audio/wav" => Some("wav"),
        "audio/ogg" => Some("ogg"),
        "video/mp4" => Some("mp4"),
        "application/pdf" => Some("pdf"),
        "text/html" => Some("html"),
        "text/plain" => Some("txt"),
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

fn media_file_url_to_path(raw: &str) -> Result<String, (String, String)> {
    let Some(rest) = raw.strip_prefix("file://") else {
        return Ok(raw.to_string());
    };
    if rest.is_empty() {
        return Err(("invalid-file-url".to_string(), "file URL missing path".to_string()));
    }
    if let Some(path) = rest.strip_prefix("localhost/") {
        return Ok(format!("/{}", percent_decode_urlish(path)));
    }
    if rest.starts_with('/') {
        return Ok(percent_decode_urlish(rest));
    }
    Err((
        "invalid-file-url".to_string(),
        "Remote hosts are not allowed in file URLs.".to_string(),
    ))
}

fn media_is_windows_network_path(raw: &str) -> bool {
    let clean = raw.trim();
    clean.starts_with("\\\\") || clean.starts_with("//")
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

fn web_media_request_contract() -> Value {
    json!({
        "max_bytes_default": 8 * 1024 * 1024,
        "supported_source_schemes": ["http", "https", "file", "local_path"],
        "workspace_relative_paths": true,
        "managed_canvas_media_prefix": "/canvas/documents/",
        "host_read_capability_requires_sniffed_binary_or_office_document": true,
        "summary_only_default": false
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
    }
}
