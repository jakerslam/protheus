const MANAGED_CANVAS_MEDIA_PREFIX: &str = "/canvas/documents/";
const DEFAULT_IMESSAGE_ATTACHMENT_ROOT_PATTERNS: &[&str] =
    &["/Users/*/Library/Messages/Attachments"];

fn media_default_local_root_suffixes() -> Vec<&'static str> {
    vec![
        "client/runtime/local/config/media",
        "client/runtime/local/state/media",
        "client/runtime/local/state/canvas",
        "client/runtime/local/state/workspace",
        "client/runtime/local/state/sandboxes",
    ]
}

fn media_local_state_dir(root: &Path) -> PathBuf {
    root.join("client/runtime/local/state")
}

fn media_local_config_dir(root: &Path) -> PathBuf {
    root.join("client/runtime/local/config")
}

fn media_canonicalize_or_resolve(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(path)
        }
    })
}

fn media_is_windows_network_path(raw: &str) -> bool {
    let clean = raw.trim();
    clean.starts_with("\\\\") || clean.starts_with("//")
}

fn media_json_error(code: &str, message: &str) -> Value {
    json!({
        "ok": false,
        "error": code,
        "message": clean_text(message, 260)
    })
}

fn media_is_windows_drive_absolute(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && bytes[2] == b'/'
}

fn media_normalize_root_pattern(raw: &str) -> Option<String> {
    let normalized = clean_text(raw, 2200).replace('\\', "/");
    let trimmed = normalized.trim_end_matches('/');
    if trimmed.is_empty() || trimmed == "/" {
        return None;
    }
    let (prefix, body) = if media_is_windows_drive_absolute(trimmed) {
        (trimmed[..2].to_string(), &trimmed[2..])
    } else if trimmed.starts_with('/') {
        (String::new(), trimmed)
    } else {
        return None;
    };
    let segments = body
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if segments.is_empty() {
        return None;
    }
    let mut normalized_segments = Vec::new();
    for segment in segments {
        if segment == "." || segment == ".." {
            return None;
        }
        if segment == "*" {
            normalized_segments.push(segment.to_string());
            continue;
        }
        if segment.contains('*') {
            return None;
        }
        normalized_segments.push(segment.to_string());
    }
    if normalized_segments.is_empty() {
        return None;
    }
    if let Some(wildcard_index) = normalized_segments.iter().position(|segment| segment == "*") {
        if wildcard_index > 0 {
            let concrete_prefix = if prefix.is_empty() {
                format!("/{}", normalized_segments[..wildcard_index].join("/"))
            } else {
                format!("{}/{}", prefix, normalized_segments[..wildcard_index].join("/"))
            };
            let canonical_prefix = media_canonicalize_or_resolve(Path::new(&concrete_prefix));
            let canonical_prefix = canonical_prefix
                .to_string_lossy()
                .replace('\\', "/")
                .trim_end_matches('/')
                .to_string();
            if canonical_prefix.is_empty() || canonical_prefix == "/" {
                return None;
            }
            return Some(format!(
                "{}/{}",
                canonical_prefix,
                normalized_segments[wildcard_index..].join("/")
            ));
        }
    }
    Some(if prefix.is_empty() {
        format!("/{}", normalized_segments.join("/"))
    } else {
        format!("{}/{}", prefix, normalized_segments.join("/"))
    })
}

fn media_normalize_resolved_root_path(path: &Path) -> Result<String, Value> {
    let canonical = media_canonicalize_or_resolve(path);
    if canonical.parent().is_none() {
        return Err(media_json_error(
            "invalid-root",
            "Local root cannot be a filesystem root.",
        ));
    }
    media_normalize_root_pattern(&canonical.to_string_lossy())
        .ok_or_else(|| media_json_error("invalid-root", "Local root pattern is invalid."))
}

fn media_match_root_pattern(candidate_path: &str, root_pattern: &str) -> bool {
    let candidate_segments = candidate_path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let root_segments = root_pattern
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if candidate_segments.len() < root_segments.len() {
        return false;
    }
    root_segments
        .iter()
        .zip(candidate_segments.iter())
        .all(|(expected, actual)| *expected == "*" || expected == actual)
}

fn media_merge_root_patterns(lists: &[Vec<String>]) -> Vec<String> {
    let mut merged = Vec::new();
    for list in lists {
        for pattern in list {
            if !merged.iter().any(|existing| existing == pattern) {
                merged.push(pattern.clone());
            }
        }
    }
    merged
}

fn media_channel_surface_id(request: &Value) -> String {
    for key in ["channel_surface", "channel", "message_provider", "messageProvider"] {
        let value = clean_text(request.get(key).and_then(Value::as_str).unwrap_or(""), 80)
            .to_ascii_lowercase();
        if !value.is_empty() {
            return value;
        }
    }
    String::new()
}

fn media_channel_attachment_root_patterns(request: &Value) -> Vec<String> {
    match media_channel_surface_id(request).as_str() {
        "imessage" | "messages" => DEFAULT_IMESSAGE_ATTACHMENT_ROOT_PATTERNS
            .iter()
            .filter_map(|row| media_normalize_root_pattern(row))
            .collect(),
        _ => Vec::new(),
    }
}

fn media_channel_attachment_root_contract() -> Value {
    json!({
        "supported_channels": ["imessage"],
        "channels": {
            "imessage": {
                "default_attachment_roots": DEFAULT_IMESSAGE_ATTACHMENT_ROOT_PATTERNS,
                "default_remote_attachment_roots": DEFAULT_IMESSAGE_ATTACHMENT_ROOT_PATTERNS
            }
        }
    })
}

fn media_safe_file_url_to_path(raw: &str) -> Result<PathBuf, Value> {
    let Some(rest) = raw.strip_prefix("file://") else {
        return Ok(PathBuf::from(raw));
    };
    if rest.is_empty() {
        return Err(media_json_error(
            "invalid-file-url",
            "file URL missing path",
        ));
    }
    let resolved = if let Some(path) = rest.strip_prefix("localhost/") {
        PathBuf::from(format!("/{}", percent_decode_urlish(path)))
    } else if rest.starts_with('/') {
        PathBuf::from(percent_decode_urlish(rest))
    } else {
        return Err(media_json_error(
            "invalid-file-url",
            "Remote hosts are not allowed in file URLs.",
        ));
    };
    if media_is_windows_network_path(&resolved.to_string_lossy()) {
        return Err(media_json_error(
            "network-path-not-allowed",
            "Local file URL cannot use Windows network paths.",
        ));
    }
    Ok(resolved)
}

fn media_expand_user_path(raw: &str) -> PathBuf {
    if let Some(rest) = raw.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(raw)
}

fn resolve_media_workspace_dir(root: &Path, request: &Value) -> PathBuf {
    let workspace_dir = clean_text(
        request
            .get("workspace_dir")
            .or_else(|| request.get("workspaceDir"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        2200,
    );
    if workspace_dir.is_empty() {
        return root.to_path_buf();
    }
    let expanded = media_expand_user_path(&workspace_dir);
    if expanded.is_absolute() {
        expanded
    } else {
        root.join(expanded)
    }
}

fn media_default_local_root_patterns(root: &Path, workspace_dir: &Path, request: &Value) -> Vec<String> {
    let mut roots = vec![
        media_local_config_dir(root).join("media"),
        media_local_state_dir(root).join("media"),
        media_local_state_dir(root).join("canvas"),
        media_local_state_dir(root).join("workspace"),
        media_local_state_dir(root).join("sandboxes"),
    ];
    if !workspace_dir.as_os_str().is_empty() {
        roots.push(workspace_dir.to_path_buf());
    }
    let concrete = roots
        .into_iter()
        .filter_map(|root_path| media_normalize_resolved_root_path(&root_path).ok())
        .collect::<Vec<_>>();
    media_merge_root_patterns(&[concrete, media_channel_attachment_root_patterns(request)])
}

fn media_request_local_roots(
    root: &Path,
    request: &Value,
    workspace_dir: &Path,
) -> Result<Option<Vec<String>>, Value> {
    let Some(value) = request.get("local_roots") else {
        return Ok(Some(media_default_local_root_patterns(root, workspace_dir, request)));
    };
    if value
        .as_str()
        .map(|row| row.trim().eq_ignore_ascii_case("any"))
        .unwrap_or(false)
    {
        return Ok(None);
    }
    let raw_roots = if let Some(rows) = value.as_array() {
        rows.iter()
            .filter_map(Value::as_str)
            .map(|row| clean_text(row, 2200))
            .collect::<Vec<_>>()
    } else {
        clean_text(value.as_str().unwrap_or(""), 4000)
            .split(',')
            .map(|row| clean_text(row, 2200))
            .filter(|row| !row.is_empty())
            .collect::<Vec<_>>()
    };
    if raw_roots.is_empty() {
        return Ok(Some(media_default_local_root_patterns(root, workspace_dir, request)));
    }
    let mut resolved_roots = Vec::new();
    for raw_root in raw_roots {
        if raw_root.contains('*') {
            let normalized = media_normalize_root_pattern(&raw_root).ok_or_else(|| {
                media_json_error(
                    "invalid-root",
                    "Local root wildcard patterns must be absolute and may use only '*' path segments.",
                )
            })?;
            if !resolved_roots.iter().any(|existing| existing == &normalized) {
                resolved_roots.push(normalized);
            }
            continue;
        }
        let candidate = if raw_root.starts_with("file://") {
            media_safe_file_url_to_path(&raw_root)?
        } else {
            let expanded = media_expand_user_path(&raw_root);
            if expanded.is_absolute() {
                expanded
            } else {
                root.join(expanded)
            }
        };
        let normalized = media_normalize_resolved_root_path(&candidate).map_err(|_| {
            json!({
                "ok": false,
                "error": "invalid-root",
                "message": format!(
                    "Invalid localRoots entry (refuses filesystem root or invalid pattern): {}. Pass a narrower absolute directory or wildcard pattern.",
                    raw_root
                )
            })
        })?;
        if !resolved_roots.iter().any(|existing| existing == &normalized) {
            resolved_roots.push(normalized);
        }
    }
    Ok(Some(resolved_roots))
}

fn media_normalize_canvas_document_id(raw: &str) -> Result<String, Value> {
    let normalized = clean_text(raw, 160);
    let valid = !normalized.is_empty()
        && normalized != "."
        && normalized != ".."
        && normalized
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'));
    if valid {
        Ok(normalized)
    } else {
        Err(media_json_error(
            "invalid-path",
            "canvas document id invalid",
        ))
    }
}

fn media_normalize_canvas_logical_path(raw: &str) -> Result<String, Value> {
    let normalized = raw.replace('\\', "/");
    let parts = normalized
        .trim_start_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() || parts.iter().any(|segment| *segment == "." || *segment == "..") {
        return Err(media_json_error(
            "invalid-path",
            "canvas document logicalPath invalid",
        ));
    }
    Ok(parts.join("/"))
}

fn resolve_canvas_media_http_path(root: &Path, raw_source: &str) -> Result<PathBuf, Value> {
    let trimmed = raw_source.trim();
    if !trimmed.starts_with(MANAGED_CANVAS_MEDIA_PREFIX) {
        return Err(media_json_error(
            "invalid-path",
            "managed canvas media path must begin with /canvas/documents/",
        ));
    }
    let without_query = trimmed
        .split(['?', '#'])
        .next()
        .unwrap_or("")
        .trim();
    let relative = without_query
        .strip_prefix(MANAGED_CANVAS_MEDIA_PREFIX)
        .unwrap_or("");
    let segments = relative
        .split('/')
        .map(percent_decode_urlish)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if segments.len() < 2 {
        return Err(media_json_error(
            "invalid-path",
            "managed canvas media path must include a document id and entry path",
        ));
    }
    let document_id = media_normalize_canvas_document_id(&segments[0])?;
    let logical_path = media_normalize_canvas_logical_path(&segments[1..].join("/"))?;
    let documents_dir = media_local_state_dir(root).join("canvas").join("documents");
    let candidate = documents_dir.join(document_id).join(logical_path);
    let canonical = media_canonicalize_or_resolve(&candidate);
    let documents_root = media_canonicalize_or_resolve(&documents_dir);
    if !(canonical == documents_root || canonical.starts_with(&documents_root)) {
        return Err(media_json_error(
            "invalid-path",
            "managed canvas media path escapes the documents root",
        ));
    }
    Ok(candidate)
}

fn resolve_local_media_source_path(
    root: &Path,
    request: &Value,
    raw_source: &str,
    workspace_dir: &Path,
) -> Result<PathBuf, Value> {
    if media_is_windows_network_path(raw_source) {
        return Err(media_json_error(
            "network-path-not-allowed",
            "Local media path cannot use Windows network paths.",
        ));
    }
    let mut resolved = if raw_source.starts_with(MANAGED_CANVAS_MEDIA_PREFIX) {
        resolve_canvas_media_http_path(root, raw_source)?
    } else if raw_source.starts_with("file://") {
        media_safe_file_url_to_path(raw_source)?
    } else {
        media_expand_user_path(raw_source)
    };
    if !resolved.is_absolute() {
        resolved = workspace_dir.join(resolved);
    }
    let roots = media_request_local_roots(root, request, workspace_dir)?;
    if let Some(allowed_roots) = roots {
        let candidate = media_canonicalize_or_resolve(&resolved);
        let candidate_pattern = media_normalize_resolved_root_path(&candidate).map_err(|_| {
            json!({
                "ok": false,
                "error": "invalid-path",
                "resolved_path": resolved.display().to_string(),
                "message": format!("Local media path is invalid: {}", raw_source)
            })
        })?;
        let default_state_dir = media_canonicalize_or_resolve(&media_local_state_dir(root));
        let uses_default_roots = request.get("local_roots").is_none();
        if uses_default_roots {
            if let Ok(relative) = candidate.strip_prefix(&default_state_dir) {
                if let Some(first) = relative.components().next() {
                    let segment = first.as_os_str().to_string_lossy();
                    if segment.starts_with("workspace-") {
                        return Err(json!({
                            "ok": false,
                            "error": "path-not-allowed",
                            "resolved_path": resolved.display().to_string(),
                            "message": format!(
                                "Local media path is not under an allowed directory: {}",
                                raw_source
                            )
                        }));
                    }
                }
            }
        }
        let allowed = allowed_roots
            .iter()
            .any(|root_pattern| media_match_root_pattern(&candidate_pattern, root_pattern));
        if !allowed {
            return Err(json!({
                "ok": false,
                "error": "path-not-allowed",
                "resolved_path": resolved.display().to_string(),
                "message": format!(
                    "Local media path is not under an allowed directory: {}",
                    raw_source
                )
            }));
        }
    }
    Ok(resolved)
}
