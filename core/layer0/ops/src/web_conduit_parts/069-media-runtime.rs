struct LoadedMedia {
    buffer: Vec<u8>,
    content_type: String,
    kind: String,
    file_name: String,
    resolved_source: String,
    source_kind: String,
    status_code: i64,
    provider: String,
    provider_hint: String,
    citation_redirect_resolved: bool,
    redirect_count: usize,
}

fn media_request_source(request: &Value) -> String {
    for key in ["url", "path"] {
        let value = clean_text(
            request.get(key).and_then(Value::as_str).unwrap_or(""),
            4000,
        );
        if !value.is_empty() {
            return value;
        }
    }
    String::new()
}

fn persist_media_artifact(root: &Path, loaded: &LoadedMedia) -> Option<Value> {
    if loaded.buffer.is_empty() {
        return None;
    }
    let response_hash = sha256_hex(&String::from_utf8_lossy(&loaded.buffer));
    let ext = Path::new(&loaded.file_name)
        .extension()
        .and_then(|row| row.to_str())
        .map(|row| row.to_ascii_lowercase())
        .or_else(|| media_extension_for_content_type(&loaded.content_type).map(|row| row.to_string()))
        .unwrap_or_else(|| "bin".to_string());
    let artifact_id = format!("web-media-{}", &response_hash[..16]);
    let path = artifacts_dir_path(root).join(format!("{artifact_id}.{ext}"));
    fs::create_dir_all(path.parent()?).ok()?;
    fs::write(&path, &loaded.buffer).ok()?;
    Some(json!({
        "artifact_id": artifact_id,
        "path": path.to_string_lossy().to_string(),
        "bytes": loaded.buffer.len(),
        "content_type": loaded.content_type,
        "file_name": loaded.file_name
    }))
}

fn load_local_media_binary(root: &Path, request: &Value) -> Result<LoadedMedia, Value> {
    let raw_source = media_request_source(request)
        .replace("MEDIA:", "")
        .trim()
        .to_string();
    let workspace_dir = resolve_media_workspace_dir(root, request);
    let max_bytes = request
        .get("max_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(8 * 1024 * 1024)
        .clamp(256, 32 * 1024 * 1024) as usize;
    let host_read_capability = media_request_host_read_capability(request);
    let resolved = resolve_local_media_source_path(root, request, &raw_source, &workspace_dir)?;
    let bytes = match fs::read(&resolved) {
        Ok(row) => row,
        Err(_) => {
            return Err(json!({
                "ok": false,
                "error": "not-found",
                "resolved_path": resolved.display().to_string()
            }))
        }
    };
    if bytes.len() > max_bytes {
        return Err(json!({
            "ok": false,
            "error": "max_bytes",
            "resolved_path": resolved.display().to_string()
        }));
    }
    let file_name = resolved
        .file_name()
        .and_then(|row| row.to_str())
        .unwrap_or("local.bin")
        .to_string();
    let sniffed = media_sniff_content_type(&bytes);
    if host_read_capability && !host_read_media_allowed(&sniffed) {
        return Err(json!({
            "ok": false,
            "error": "path-not-allowed",
            "reason": "host_read_requires_binary_verified_media_or_office_document"
        }));
    }
    let content_type = media_guess_content_type(Some(&file_name), &bytes, None);
    Ok(LoadedMedia {
        buffer: bytes,
        content_type: content_type.clone(),
        kind: media_kind_from_content_type(&content_type),
        file_name,
        resolved_source: resolved.display().to_string(),
        source_kind: "local".to_string(),
        status_code: 200,
        provider: "direct_http".to_string(),
        provider_hint: "local".to_string(),
        citation_redirect_resolved: false,
        redirect_count: 0,
    })
}

pub fn api_media(root: &Path, request: &Value) -> Value {
    let raw = media_request_source(request);
    if raw.is_empty() {
        return json!({"ok": false, "error": "media_source_required"});
    }
    let summary_only = request.get("summary_only").and_then(Value::as_bool).unwrap_or(false);
    let requested_source = if raw.starts_with("data:") {
        media_redacted_inline_source(&raw)
    } else {
        raw.clone()
    };
    let loaded = if raw.starts_with("http://") || raw.starts_with("https://") {
        fetch_remote_media_binary(root, request)
    } else if raw.starts_with("data:") {
        load_inline_media_binary(request)
    } else {
        load_local_media_binary(root, request)
    };
    match loaded {
        Ok(loaded) => {
            let kind_max_bytes = max_bytes_for_media_kind(&loaded.kind);
            if loaded.buffer.len() > kind_max_bytes {
                let receipt = build_receipt(&requested_source, "deny", None, loaded.status_code, "kind_max_bytes", None);
                let _ = append_jsonl(&receipts_path(root), &receipt);
                return json!({
                    "ok": false,
                    "type": "web_conduit_media",
                    "error": "kind_max_bytes",
                    "requested_source": requested_source,
                "resolved_source": loaded.resolved_source,
                "detected_kind": loaded.kind,
                "kind_max_bytes": kind_max_bytes,
                "bytes": loaded.buffer.len(),
                "redirect_count": loaded.redirect_count,
                "media_request_contract": web_media_request_contract(),
                "receipt": receipt
            });
            }
            let artifact = persist_media_artifact(root, &loaded).unwrap_or(Value::Null);
            let response_hash = sha256_hex(&String::from_utf8_lossy(&loaded.buffer));
            let receipt = build_receipt(
                &requested_source,
                "allow",
                Some(&response_hash),
                loaded.status_code,
                if loaded.source_kind == "remote" {
                    "media_loaded"
                } else if loaded.source_kind == "inline" {
                    "inline_media_loaded"
                } else {
                    "local_media_loaded"
                },
                None,
            );
            let _ = append_jsonl(&receipts_path(root), &receipt);
            let include_inline = !summary_only && loaded.buffer.len() <= 512_000;
            json!({
                "ok": true,
                "type": "web_conduit_media",
                "requested_source": requested_source,
                "resolved_source": loaded.resolved_source,
                "source_kind": loaded.source_kind,
                "provider": loaded.provider,
                "provider_hint": loaded.provider_hint,
                "citation_redirect_resolved": loaded.citation_redirect_resolved,
                "redirect_count": loaded.redirect_count,
                "status_code": loaded.status_code,
                "content_type": loaded.content_type,
                "kind": loaded.kind,
                "kind_max_bytes": kind_max_bytes,
                "file_name": loaded.file_name,
                "voice_compatible_audio": is_telegram_voice_compatible_audio(Some(&loaded.content_type), Some(&loaded.file_name)),
                "bytes": loaded.buffer.len(),
                "content_base64": if include_inline {
                    use base64::Engine;
                    base64::engine::general_purpose::STANDARD.encode(&loaded.buffer)
                } else {
                    String::new()
                },
                "content_included": include_inline,
                "artifact": artifact,
                "summary": format!(
                    "Loaded {} {} ({} bytes).",
                    loaded.source_kind,
                    loaded.kind,
                    loaded.buffer.len()
                ),
                "media_request_contract": web_media_request_contract(),
                "receipt": receipt
            })
        }
        Err(mut err) => {
            let status_code = err.get("status_code").and_then(Value::as_i64).unwrap_or(0);
            let reason = clean_text(err.get("error").and_then(Value::as_str).unwrap_or("web_media_failed"), 180);
            let receipt = build_receipt(&raw, "deny", None, status_code, &reason, err.get("body_snippet").and_then(Value::as_str));
            let _ = append_jsonl(&receipts_path(root), &receipt);
            if let Some(obj) = err.as_object_mut() {
                obj.insert("type".to_string(), json!("web_conduit_media"));
                obj.insert("requested_source".to_string(), json!(raw));
                obj.insert("media_request_contract".to_string(), web_media_request_contract());
                obj.insert("receipt".to_string(), receipt);
            }
            err
        }
    }
}
