fn trim_preserve_text(raw: &str, max_len: usize) -> String {
    raw.trim().chars().take(max_len.max(1)).collect::<String>()
}

fn attachment_request_string(request: &Value, keys: &[&str], max_len: usize) -> String {
    keys.iter()
        .filter_map(|key| request.get(*key).and_then(Value::as_str))
        .map(|value| trim_preserve_text(value, max_len))
        .find(|value| !value.is_empty())
        .unwrap_or_default()
}

fn attachment_request_array(request: &Value, keys: &[&str], max_len: usize) -> Vec<String> {
    for key in keys {
        let Some(rows) = request.get(*key).and_then(Value::as_array) else {
            continue;
        };
        return rows
            .iter()
            .filter_map(Value::as_str)
            .map(|value| trim_preserve_text(value, max_len))
            .filter(|value| !value.is_empty())
            .collect();
    }
    Vec::new()
}

fn attachment_request_index_set(request: &Value) -> std::collections::BTreeSet<usize> {
    for key in [
        "already_transcribed_indices",
        "alreadyTranscribedIndices",
        "AlreadyTranscribedIndices",
    ] {
        if let Some(rows) = request.get(key).and_then(Value::as_array) {
            return rows
                .iter()
                .filter_map(Value::as_u64)
                .map(|value| value as usize)
                .collect();
        }
        if let Some(raw) = request.get(key).and_then(Value::as_str) {
            return raw
                .split(',')
                .filter_map(|part| part.trim().parse::<usize>().ok())
                .collect();
        }
    }
    std::collections::BTreeSet::new()
}

fn normalize_attachment_path(raw: &str) -> Option<String> {
    let value = trim_preserve_text(raw, 4000);
    if value.is_empty() {
        return None;
    }
    let path = if value.starts_with("file://") {
        media_safe_file_url_to_path(&value).ok()?
    } else {
        if media_is_windows_network_path(&value) {
            return None;
        }
        PathBuf::from(&value)
    };
    let display = path.to_string_lossy().to_string();
    if display.is_empty() || media_is_windows_network_path(&display) {
        return None;
    }
    Some(display)
}

fn normalize_attachment_url(raw: &str) -> Option<String> {
    let value = trim_preserve_text(raw, 4000);
    if value.is_empty() { None } else { Some(value) }
}

fn resolve_attachment_kind_value(path: Option<&str>, url: Option<&str>, mime: Option<&str>) -> String {
    let mime_kind = mime
        .map(normalize_media_content_type)
        .map(|value| media_kind_from_content_type(&value))
        .unwrap_or_else(|| "unknown".to_string());
    if mime_kind != "unknown" {
        return mime_kind;
    }
    let locator = path.or(url).unwrap_or("");
    let extension_kind = media_mime_type_from_file_name(Some(locator))
        .map(media_kind_from_content_type)
        .unwrap_or_else(|| "unknown".to_string());
    if extension_kind != "unknown" {
        return extension_kind;
    }
    let lowered = locator.to_ascii_lowercase();
    if lowered.ends_with(".mp4")
        || lowered.ends_with(".mov")
        || lowered.ends_with(".mkv")
        || lowered.ends_with(".webm")
        || lowered.ends_with(".avi")
        || lowered.ends_with(".m4v")
    {
        return "video".to_string();
    }
    "unknown".to_string()
}

fn normalize_attachment_record(value: &Value, index_hint: usize) -> Option<Value> {
    let index = value
        .get("index")
        .and_then(Value::as_u64)
        .map(|row| row as usize)
        .unwrap_or(index_hint);
    let path = value
        .get("path")
        .or_else(|| value.get("MediaPath"))
        .and_then(Value::as_str)
        .and_then(normalize_attachment_path);
    let url = value
        .get("url")
        .or_else(|| value.get("MediaUrl"))
        .and_then(Value::as_str)
        .and_then(normalize_attachment_url);
    let mime = value
        .get("mime")
        .or_else(|| value.get("MediaType"))
        .and_then(Value::as_str)
        .map(|row| trim_preserve_text(row, 160))
        .filter(|row| !row.is_empty());
    let already_transcribed = value
        .get("already_transcribed")
        .or_else(|| value.get("alreadyTranscribed"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if path.is_none() && url.is_none() {
        return None;
    }
    let kind = resolve_attachment_kind_value(path.as_deref(), url.as_deref(), mime.as_deref());
    Some(json!({
        "index": index,
        "path": path,
        "url": url,
        "mime": mime,
        "already_transcribed": already_transcribed,
        "kind": kind
    }))
}

fn normalize_attachments_from_request(request: &Value) -> Vec<Value> {
    if let Some(rows) = request
        .get("attachments")
        .or_else(|| request.get("Attachments"))
        .and_then(Value::as_array)
    {
        return rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| normalize_attachment_record(row, index))
            .collect();
    }

    let already_transcribed = attachment_request_index_set(request);
    let media_paths = attachment_request_array(
        request,
        &["MediaPaths", "media_paths", "mediaPaths"],
        4000,
    );
    let media_urls = attachment_request_array(
        request,
        &["MediaUrls", "media_urls", "mediaUrls"],
        4000,
    );
    let media_types = attachment_request_array(
        request,
        &["MediaTypes", "media_types", "mediaTypes"],
        160,
    );
    let single_media_type = attachment_request_string(
        request,
        &["MediaType", "media_type", "mediaType"],
        160,
    );

    let resolve_mime = |count: usize, index: usize| -> Option<String> {
        media_types
            .get(index)
            .map(|value| trim_preserve_text(value, 160))
            .filter(|value| !value.is_empty())
            .or_else(|| {
                if count == 1 && !single_media_type.is_empty() {
                    Some(single_media_type.clone())
                } else {
                    None
                }
            })
    };

    if !media_paths.is_empty() {
        let count = media_paths.len();
        return media_paths
            .iter()
            .enumerate()
            .filter_map(|(index, value)| {
                let path = normalize_attachment_path(value);
                let url = media_urls.get(index).and_then(|value| normalize_attachment_url(value));
                if path.is_none() && url.is_none() {
                    return None;
                }
                let mime = resolve_mime(count, index);
                let kind = resolve_attachment_kind_value(path.as_deref(), url.as_deref(), mime.as_deref());
                Some(json!({
                    "index": index,
                    "path": path,
                    "url": url,
                    "mime": mime,
                    "already_transcribed": already_transcribed.contains(&index),
                    "kind": kind
                }))
            })
            .collect();
    }

    if !media_urls.is_empty() {
        let count = media_urls.len();
        return media_urls
            .iter()
            .enumerate()
            .filter_map(|(index, value)| {
                let url = normalize_attachment_url(value);
                url.as_ref()?;
                let mime = resolve_mime(count, index);
                let kind = resolve_attachment_kind_value(None, url.as_deref(), mime.as_deref());
                Some(json!({
                    "index": index,
                    "path": Value::Null,
                    "url": url,
                    "mime": mime,
                    "already_transcribed": already_transcribed.contains(&index),
                    "kind": kind
                }))
            })
            .collect();
    }

    let path = attachment_request_string(request, &["MediaPath", "media_path", "mediaPath"], 4000);
    let url = attachment_request_string(request, &["MediaUrl", "media_url", "mediaUrl"], 4000);
    let path = normalize_attachment_path(&path);
    let url = normalize_attachment_url(&url);
    if path.is_none() && url.is_none() {
        return Vec::new();
    }
    let mime = if single_media_type.is_empty() {
        None
    } else {
        Some(single_media_type)
    };
    vec![json!({
        "index": 0,
        "path": path,
        "url": url,
        "mime": mime,
        "already_transcribed": already_transcribed.contains(&0),
        "kind": resolve_attachment_kind_value(
            path.as_deref(),
            url.as_deref(),
            mime.as_deref()
        )
    })]
}

fn media_attachments_prefer(request: &Value) -> String {
    attachment_request_string(request, &["prefer"], 24).to_ascii_lowercase()
}

fn media_attachments_mode(request: &Value) -> String {
    attachment_request_string(request, &["mode"], 24).to_ascii_lowercase()
}

fn media_attachments_capability(request: &Value) -> String {
    attachment_request_string(request, &["capability"], 24).to_ascii_lowercase()
}

fn media_attachment_matches_capability(row: &Value, capability: &str) -> bool {
    let kind = row.get("kind").and_then(Value::as_str).unwrap_or("");
    if capability == "audio" {
        if row
            .get("already_transcribed")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return false;
        }
        return kind == "audio";
    }
    kind == capability
}

fn order_media_attachments(attachments: &[Value], prefer: &str) -> Vec<Value> {
    let list = attachments.to_vec();
    match prefer {
        "last" => list.into_iter().rev().collect(),
        "path" => {
            let mut with_path = list
                .iter()
                .filter(|row| row.get("path").and_then(Value::as_str).is_some())
                .cloned()
                .collect::<Vec<_>>();
            with_path.extend(
                list.iter()
                    .filter(|row| row.get("path").and_then(Value::as_str).is_none())
                    .cloned(),
            );
            with_path
        }
        "url" => {
            let mut with_url = list
                .iter()
                .filter(|row| row.get("url").and_then(Value::as_str).is_some())
                .cloned()
                .collect::<Vec<_>>();
            with_url.extend(
                list.iter()
                    .filter(|row| row.get("url").and_then(Value::as_str).is_none())
                    .cloned(),
            );
            with_url
        }
        _ => list,
    }
}

fn web_media_attachments_contract() -> Value {
    json!({
        "context_fields": {
            "paths": ["MediaPath", "MediaPaths"],
            "urls": ["MediaUrl", "MediaUrls"],
            "types": ["MediaType", "MediaTypes"]
        },
        "attachment_shape": ["index", "path", "url", "mime", "kind", "already_transcribed"],
        "capabilities": ["image", "audio", "video"],
        "selection_policy": {
            "default_mode": "first",
            "default_prefer": "first",
            "supported_modes": ["first", "all"],
            "supported_prefer": ["first", "last", "path", "url"],
            "default_max_attachments": 1
        },
        "path_normalization_contract": {
            "supports_file_url_hosts": ["", "localhost"],
            "rejects_remote_file_url_hosts": true,
            "rejects_windows_network_paths": true
        }
    })
}

fn api_media_attachments(request: &Value) -> Value {
    let normalized = normalize_attachments_from_request(request);
    let capability = media_attachments_capability(request);
    let prefer = media_attachments_prefer(request);
    let mode = media_attachments_mode(request);
    let max_attachments = parse_fetch_u64(
        request.get("max_attachments").or_else(|| request.get("maxAttachments")),
        1,
        1,
        32,
    ) as usize;
    if !capability.is_empty() && !matches!(capability.as_str(), "image" | "audio" | "video") {
        return json!({
            "ok": false,
            "type": "web_conduit_media_attachments",
            "error": "invalid_capability",
            "capability": capability,
            "attachments_contract": web_media_attachments_contract()
        });
    }
    let matched = if capability.is_empty() {
        Vec::new()
    } else {
        normalized
            .iter()
            .filter(|row| media_attachment_matches_capability(row, &capability))
            .cloned()
            .collect::<Vec<_>>()
    };
    let ordered = order_media_attachments(&matched, &prefer);
    let selected = if mode == "all" {
        ordered.into_iter().take(max_attachments.max(1)).collect::<Vec<_>>()
    } else {
        ordered.into_iter().take(1).collect::<Vec<_>>()
    };
    let summary_only = media_tool_read_boolean_param(request, "summary_only").unwrap_or(false);
    json!({
        "ok": true,
        "type": "web_conduit_media_attachments",
        "capability": if capability.is_empty() { Value::Null } else { json!(capability) },
        "selection_policy": {
            "prefer": if prefer.is_empty() { "first" } else { prefer.as_str() },
            "mode": if mode.is_empty() { "first" } else { mode.as_str() },
            "max_attachments": max_attachments
        },
        "attachment_count": normalized.len(),
        "selected_count": selected.len(),
        "attachments": if summary_only { Value::Null } else { Value::Array(normalized.clone()) },
        "selected_attachments": selected,
        "summary": format!(
            "Normalized {} attachment(s) and selected {} {} attachment(s).",
            normalized.len(),
            if capability.is_empty() {
                0
            } else {
                matched
                    .len()
                    .min(if mode == "all" { max_attachments.max(1) } else { 1 })
            },
            if capability.is_empty() {
                "candidate".to_string()
            } else {
                capability
            }
        ),
        "attachments_contract": web_media_attachments_contract()
    })
}
