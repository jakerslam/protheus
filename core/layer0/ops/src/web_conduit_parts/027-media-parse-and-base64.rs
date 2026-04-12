fn estimate_base64_decoded_bytes(base64: &str) -> usize {
    let mut effective_len = 0usize;
    for ch in base64.bytes() {
        if ch <= 0x20 {
            continue;
        }
        effective_len += 1;
    }
    if effective_len == 0 {
        return 0;
    }
    let mut padding = 0usize;
    let mut end = base64.len();
    while end > 0 && base64.as_bytes()[end - 1] <= 0x20 {
        end -= 1;
    }
    if end > 0 && base64.as_bytes()[end - 1] == b'=' {
        padding = 1;
        end -= 1;
        while end > 0 && base64.as_bytes()[end - 1] <= 0x20 {
            end -= 1;
        }
        if end > 0 && base64.as_bytes()[end - 1] == b'=' {
            padding = 2;
        }
    }
    (((effective_len * 3) / 4) as isize - padding as isize).max(0) as usize
}

fn is_base64_data_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/')
}

fn canonicalize_base64(base64: &str) -> Option<String> {
    let mut cleaned = String::new();
    let mut padding = 0usize;
    let mut saw_padding = false;
    for byte in base64.bytes() {
        if byte <= 0x20 {
            continue;
        }
        if byte == b'=' {
            padding += 1;
            if padding > 2 {
                return None;
            }
            saw_padding = true;
            cleaned.push('=');
            continue;
        }
        if saw_padding || !is_base64_data_char(byte) {
            return None;
        }
        cleaned.push(byte as char);
    }
    if cleaned.is_empty() || cleaned.len() % 4 != 0 {
        return None;
    }
    Some(cleaned)
}

fn media_redacted_inline_source(raw: &str) -> String {
    let trimmed = clean_text(raw, 240);
    if let Some((prefix, _)) = trimmed.split_once(',') {
        format!("{prefix},[redacted]")
    } else {
        "data:[redacted]".to_string()
    }
}

fn parse_inline_media_data_url(raw: &str, max_bytes: usize) -> Result<(Vec<u8>, String), Value> {
    let Some(rest) = raw.strip_prefix("data:") else {
        return Err(media_json_error("invalid-inline-media", "data URL required"));
    };
    let Some((meta, payload)) = rest.split_once(',') else {
        return Err(media_json_error(
            "invalid-inline-media",
            "data URL missing payload separator",
        ));
    };
    let meta = clean_text(meta, 240);
    if !meta.to_ascii_lowercase().contains(";base64") {
        return Err(media_json_error(
            "invalid-inline-media",
            "data URL must use base64 encoding",
        ));
    }
    let estimated = estimate_base64_decoded_bytes(payload);
    if estimated > max_bytes {
        return Err(json!({
            "ok": false,
            "error": "max_bytes",
            "declared_size": estimated
        }));
    }
    let canonical = canonicalize_base64(payload).ok_or_else(|| {
        media_json_error(
            "invalid-inline-media",
            "base64 payload is invalid or non-canonical",
        )
    })?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(canonical)
        .map_err(|_| media_json_error("invalid-inline-media", "base64 payload could not be decoded"))?;
    let mime = meta
        .split(';')
        .next()
        .map(|row| clean_text(row, 120).to_ascii_lowercase())
        .filter(|row| !row.is_empty())
        .unwrap_or_else(|| "application/octet-stream".to_string());
    Ok((bytes, mime))
}

fn media_default_inline_file_name(content_type: &str) -> String {
    media_extension_for_content_type(content_type)
        .map(|ext| format!("inline.{ext}"))
        .unwrap_or_else(|| "inline.bin".to_string())
}

fn load_inline_media_binary(request: &Value) -> Result<LoadedMedia, Value> {
    let raw_source = media_request_source(request);
    let max_bytes = media_prefetch_max_bytes(request);
    let (bytes, header_content_type) = parse_inline_media_data_url(&raw_source, max_bytes)?;
    let file_name = clean_text(
        request
            .get("file_name")
            .or_else(|| request.get("fileName"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        240,
    );
    let content_type = media_guess_content_type(
        if file_name.is_empty() { None } else { Some(&file_name) },
        &bytes,
        Some(&header_content_type),
    );
    Ok(LoadedMedia {
        buffer: bytes,
        content_type: content_type.clone(),
        kind: media_kind_from_content_type(&content_type),
        file_name: if file_name.is_empty() {
            media_default_inline_file_name(&content_type)
        } else {
            file_name
        },
        resolved_source: media_redacted_inline_source(&raw_source),
        source_kind: "inline".to_string(),
        status_code: 200,
        provider: "inline_base64".to_string(),
        provider_hint: "inline".to_string(),
        citation_redirect_resolved: false,
        redirect_count: 0,
    })
}

fn media_unwrap_quoted(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.len() < 2 {
        return None;
    }
    let first = trimmed.chars().next()?;
    let last = trimmed.chars().last()?;
    if first != last || !matches!(first, '"' | '\'' | '`') {
        return None;
    }
    Some(trimmed[1..trimmed.len() - 1].trim().to_string())
}

fn media_has_traversal_or_home_prefix(candidate: &str) -> bool {
    candidate.starts_with("../")
        || candidate == ".."
        || candidate.starts_with("~")
        || candidate
            .replace('\\', "/")
            .split('/')
            .any(|segment| segment == "..")
}

fn media_windows_drive_path(candidate: &str) -> bool {
    let bytes = candidate.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\')
}

fn media_scheme_path(candidate: &str) -> bool {
    let mut chars = candidate.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    chars.any(|ch| ch == ':')
}

fn normalize_media_parse_source(src: &str) -> String {
    src.strip_prefix("file://")
        .unwrap_or(src)
        .trim()
        .to_string()
}

fn media_valid_candidate(candidate: &str, allow_spaces: bool, allow_bare_filename: bool) -> bool {
    if candidate.is_empty() || candidate.len() > 4096 {
        return false;
    }
    if !allow_spaces && candidate.chars().any(char::is_whitespace) {
        return false;
    }
    if candidate.starts_with("http://") || candidate.starts_with("https://") || candidate.starts_with("data:") {
        return true;
    }
    if media_has_traversal_or_home_prefix(candidate) {
        return false;
    }
    if candidate.starts_with('/')
        || candidate.starts_with("./")
        || media_windows_drive_path(candidate)
        || candidate.starts_with("\\\\")
        || (!media_scheme_path(candidate) && (candidate.contains('/') || candidate.contains('\\')))
    {
        return true;
    }
    allow_bare_filename
        && !media_scheme_path(candidate)
        && Regex::new(r"\.\w{1,10}$").unwrap().is_match(candidate)
}

fn strip_audio_as_voice_tag(text: &str) -> (String, bool) {
    let tag_re = Regex::new(r"\[\[\s*audio_as_voice\s*\]\]").unwrap();
    let had = tag_re.is_match(text);
    if !had {
        return (text.to_string(), false);
    }
    let cleaned = tag_re
        .replace_all(text, |caps: &regex::Captures<'_>| {
            let full = caps.get(0).expect("full match");
            let start = full.start();
            let end = full.end();
            let before = text[..start].chars().next_back();
            let after = text[end..].chars().next();
            if before.is_some_and(|ch| !ch.is_whitespace())
                && after.is_some_and(|ch| !ch.is_whitespace())
            {
                " ".to_string()
            } else {
                String::new()
            }
        })
        .to_string()
        .replace("\r\n", "\n");
    let normalized = cleaned
        .split('\n')
        .map(|line| {
            Regex::new(r"[ \t]{2,}")
                .unwrap()
                .replace_all(line.trim_end(), " ")
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();
    (normalized, true)
}

fn split_media_from_output(raw: &str) -> Value {
    let trimmed = raw.trim_end();
    if trimmed.trim().is_empty() {
        return json!({"text": ""});
    }
    let mut media_urls: Vec<String> = Vec::new();
    let mut kept_lines: Vec<String> = Vec::new();
    let mut segments: Vec<Value> = Vec::new();
    let mut in_fence = false;
    for line in trimmed.split('\n') {
        let trimmed_start = line.trim_start();
        if trimmed_start.starts_with("```") || trimmed_start.starts_with("~~~") {
            in_fence = !in_fence;
            kept_lines.push(line.to_string());
            continue;
        }
        if in_fence || !trimmed_start.starts_with("MEDIA:") {
            kept_lines.push(line.to_string());
            continue;
        }
        let payload = trimmed_start.trim_start_matches("MEDIA:").trim();
        let unwrapped = media_unwrap_quoted(payload);
        let payload_value = unwrapped.clone().unwrap_or_else(|| payload.to_string());
        let mut candidates = if let Some(value) = unwrapped.clone() {
            vec![value]
        } else {
            payload
                .split_whitespace()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        };
        let mut parsed = candidates
            .drain(..)
            .map(|part| normalize_media_parse_source(part.trim_matches(|ch| matches!(ch, '`' | '"' | '\'' | '[' | ']' | '{' | '}' | '(' | ')' | ',' | '\\')).trim()))
            .filter(|part| media_valid_candidate(part, unwrapped.is_some(), false))
            .collect::<Vec<_>>();
        if parsed.is_empty() && payload_value.contains(' ') {
            let fallback = normalize_media_parse_source(payload_value.trim());
            if media_valid_candidate(&fallback, true, true) {
                parsed.push(fallback);
            }
        }
        if parsed.is_empty() {
            kept_lines.push(line.to_string());
            continue;
        }
        media_urls.extend(parsed.iter().cloned());
        segments.extend(parsed.into_iter().map(|url| json!({"type": "media", "url": url})));
    }
    let (text, audio_as_voice) = strip_audio_as_voice_tag(&kept_lines.join("\n"));
    if !text.trim().is_empty() {
        segments.insert(0, json!({"type": "text", "text": text}));
    }
    json!({
        "text": text,
        "media_urls": if media_urls.is_empty() { Value::Null } else { json!(media_urls) },
        "media_url": media_urls.first().cloned(),
        "audio_as_voice": if audio_as_voice { Value::Bool(true) } else { Value::Null },
        "had_audio_tag": audio_as_voice,
        "audio_delivery_mode": if audio_as_voice { "voice" } else { "file" },
        "segments": if segments.is_empty() { Value::Null } else { json!(segments) }
    })
}

fn api_parse_media(request: &Value) -> Value {
    let text = request
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or("");
    let parsed = split_media_from_output(text);
    json!({
        "ok": true,
        "type": "web_conduit_parse_media",
        "text": parsed.get("text").cloned().unwrap_or(Value::String(String::new())),
        "media_urls": parsed.get("media_urls").cloned().unwrap_or(Value::Null),
        "media_url": parsed.get("media_url").cloned().unwrap_or(Value::Null),
        "audio_as_voice": parsed.get("audio_as_voice").cloned().unwrap_or(Value::Null),
        "had_audio_tag": parsed.get("had_audio_tag").cloned().unwrap_or(Value::Bool(false)),
        "audio_delivery_mode": parsed.get("audio_delivery_mode").cloned().unwrap_or(json!("file")),
        "segments": parsed.get("segments").cloned().unwrap_or(Value::Null)
    })
}

#[cfg(test)]
mod openclaw_media_parse_base64_tests {
    use super::*;

    const TINY_PNG_BASE64_INLINE: &str =
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/woAAn8B9FD5fHAAAAAASUVORK5CYII=";

    #[test]
    fn openclaw_media_base64_helpers_normalize_and_estimate() {
        assert_eq!(canonicalize_base64(" SGV s bG8= \n").as_deref(), Some("SGVsbG8="));
        assert_eq!(canonicalize_base64("SGVsbG8=\"bad"), None);
        assert_eq!(estimate_base64_decoded_bytes("SGV s bG8= \n"), 5);
        assert_eq!(estimate_base64_decoded_bytes(""), 0);
    }

    #[test]
    fn openclaw_media_inline_data_url_loads_png() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_media(
            tmp.path(),
            &json!({
                "url": format!("data:image/png;base64,{}", TINY_PNG_BASE64_INLINE),
                "summary_only": true
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("source_kind").and_then(Value::as_str), Some("inline"));
        assert_eq!(out.get("kind").and_then(Value::as_str), Some("image"));
        assert_eq!(out.get("content_type").and_then(Value::as_str), Some("image/png"));
    }

    #[test]
    fn openclaw_media_parse_extracts_tokens_and_strips_audio_tag() {
        let out = api_parse_media(&json!({
            "text": "Hello [[audio_as_voice]]\nMEDIA:\"/Users/pete/My File.png\""
        }));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("audio_as_voice").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("media_urls").and_then(Value::as_array).and_then(|rows| rows.first()).and_then(Value::as_str),
            Some("/Users/pete/My File.png")
        );
    }

    #[test]
    fn openclaw_media_parse_rejects_traversal_and_ignores_fenced_media_lines() {
        let out = api_parse_media(&json!({
            "text": "Before\nMEDIA:../../.env\n```text\nMEDIA:https://example.com/ignored.png\n```\nAfter"
        }));
        let media = out.get("media_urls").cloned().unwrap_or(Value::Null);
        assert!(media.is_null() || media.as_array().is_some_and(|rows| rows.is_empty()));
        assert_eq!(out.get("text").and_then(Value::as_str), Some("Before\nMEDIA:../../.env\n```text\nMEDIA:https://example.com/ignored.png\n```\nAfter"));
    }
}
use base64::Engine;
