const MANAGED_CANVAS_MEDIA_PREFIX: &str = "/canvas/documents/";

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

fn media_default_local_roots(root: &Path, workspace_dir: &Path) -> Vec<PathBuf> {
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
    let mut deduped = Vec::new();
    for root_path in roots {
        let canonical = media_canonicalize_or_resolve(&root_path);
        if !deduped.iter().any(|existing: &PathBuf| existing == &canonical) {
            deduped.push(canonical);
        }
    }
    deduped
}

fn media_request_local_roots(
    root: &Path,
    request: &Value,
    workspace_dir: &Path,
) -> Result<Option<Vec<PathBuf>>, Value> {
    let Some(value) = request.get("local_roots") else {
        return Ok(Some(media_default_local_roots(root, workspace_dir)));
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
        return Ok(Some(media_default_local_roots(root, workspace_dir)));
    }
    let mut resolved_roots = Vec::new();
    for raw_root in raw_roots {
        let mut candidate = if raw_root.starts_with("file://") {
            media_safe_file_url_to_path(&raw_root)?
        } else {
            media_expand_user_path(&raw_root)
        };
        if !candidate.is_absolute() {
            candidate = root.join(candidate);
        }
        let canonical = media_canonicalize_or_resolve(&candidate);
        if canonical.parent().is_none() {
            return Err(json!({
                "ok": false,
                "error": "invalid-root",
                "message": format!(
                    "Invalid localRoots entry (refuses filesystem root): {}. Pass a narrower directory.",
                    raw_root
                )
            }));
        }
        if !resolved_roots.iter().any(|existing: &PathBuf| existing == &canonical) {
            resolved_roots.push(canonical);
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
        let allowed = allowed_roots.iter().any(|base| {
            let canonical = media_canonicalize_or_resolve(base);
            candidate == canonical || candidate.starts_with(&canonical)
        });
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
