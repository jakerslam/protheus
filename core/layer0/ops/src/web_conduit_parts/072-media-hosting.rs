const HOSTED_MEDIA_DIR_REL: &str = "client/runtime/local/state/web_conduit/hosted_media";
const DEFAULT_HOSTED_MEDIA_TTL_SECONDS: u64 = 120;
const MAX_HOSTED_MEDIA_TTL_SECONDS: u64 = 3600;
const MAX_HOSTED_MEDIA_ID_CHARS: usize = 200;

fn hosted_media_dir_path(root: &Path) -> PathBuf {
    root.join(HOSTED_MEDIA_DIR_REL)
}

fn hosted_media_manifest_path(root: &Path, hosted_id: &str) -> PathBuf {
    hosted_media_dir_path(root).join(format!("{hosted_id}.json"))
}

fn hosted_media_route_path(hosted_id: &str) -> String {
    format!("/api/web/media/{hosted_id}")
}

fn is_valid_hosted_media_id(hosted_id: &str) -> bool {
    if hosted_id.is_empty()
        || hosted_id.len() > MAX_HOSTED_MEDIA_ID_CHARS
        || hosted_id == "."
        || hosted_id == ".."
    {
        return false;
    }
    hosted_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
}

fn hosted_media_error(error: &str, extra: Value) -> Value {
    let mut payload = json!({
        "ok": false,
        "type": "web_conduit_media_host",
        "error": clean_text(error, 120)
    });
    if let (Some(dst), Some(src)) = (payload.as_object_mut(), extra.as_object()) {
        for (key, value) in src {
            dst.insert(key.clone(), value.clone());
        }
    }
    payload
}

fn cleanup_hosted_media_entry(root: &Path, hosted_id: &str, manifest: &Value) {
    let data_path = clean_text(
        manifest.get("path").and_then(Value::as_str).unwrap_or(""),
        2200,
    );
    if !data_path.is_empty() {
        let _ = fs::remove_file(PathBuf::from(data_path));
    }
    let _ = fs::remove_file(hosted_media_manifest_path(root, hosted_id));
}

fn prune_expired_hosted_media(root: &Path) {
    let dir = hosted_media_dir_path(root);
    let entries = match fs::read_dir(&dir) {
        Ok(rows) => rows,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|row| row.to_str()) != Some("json") {
            continue;
        }
        let manifest = read_json_or(&path, Value::Null);
        let hosted_id = clean_text(
            manifest.get("id").and_then(Value::as_str).unwrap_or(""),
            MAX_HOSTED_MEDIA_ID_CHARS,
        );
        let expires_at = clean_text(
            manifest.get("expires_at").and_then(Value::as_str).unwrap_or(""),
            80,
        );
        let expired = DateTime::parse_from_rfc3339(&expires_at)
            .ok()
            .map(|row| row.with_timezone(&Utc) <= Utc::now())
            .unwrap_or(true);
        if expired {
            cleanup_hosted_media_entry(root, &hosted_id, &manifest);
        }
    }
}

fn store_hosted_media_entry(
    root: &Path,
    artifact_path: &Path,
    file_name: &str,
    content_type: &str,
    bytes: usize,
    ttl_seconds: u64,
) -> Result<Value, Value> {
    let dir = hosted_media_dir_path(root);
    fs::create_dir_all(&dir).map_err(|err| {
        hosted_media_error(
            "hosted_media_dir_create_failed",
            json!({"detail": clean_text(&err.to_string(), 240)}),
        )
    })?;
    let ext = artifact_path
        .extension()
        .and_then(|row| row.to_str())
        .map(|row| row.to_ascii_lowercase())
        .or_else(|| media_extension_for_content_type(content_type).map(|row| row.to_string()))
        .unwrap_or_else(|| "bin".to_string());
    let seed = format!(
        "{}:{}:{}",
        artifact_path.display(),
        file_name,
        Utc::now().timestamp_millis()
    );
    let hosted_id = format!("hosted-media-{}", &sha256_hex(&seed)[..16]);
    let hosted_path = dir.join(format!("{hosted_id}.{ext}"));
    fs::copy(artifact_path, &hosted_path).map_err(|err| {
        hosted_media_error(
            "hosted_media_copy_failed",
            json!({"detail": clean_text(&err.to_string(), 240)}),
        )
    })?;
    let expires_at = (Utc::now() + chrono::Duration::seconds(ttl_seconds as i64)).to_rfc3339();
    let manifest = json!({
        "id": hosted_id,
        "path": hosted_path.display().to_string(),
        "file_name": clean_text(file_name, 220),
        "content_type": normalize_media_content_type(content_type),
        "bytes": bytes,
        "expires_at": expires_at,
        "ttl_seconds": ttl_seconds
    });
    write_json_atomic(
        &hosted_media_manifest_path(
            root,
            manifest.get("id").and_then(Value::as_str).unwrap_or(""),
        ),
        &manifest,
    )
    .map_err(|err| {
        hosted_media_error(
            "hosted_media_manifest_write_failed",
            json!({"detail": clean_text(&err, 240)}),
        )
    })?;
    Ok(manifest)
}

pub fn api_media_host(root: &Path, request: &Value) -> Value {
    prune_expired_hosted_media(root);
    let ttl_seconds = request
        .get("ttl_seconds")
        .or_else(|| request.get("ttlSeconds"))
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_HOSTED_MEDIA_TTL_SECONDS)
        .clamp(1, MAX_HOSTED_MEDIA_TTL_SECONDS);
    let base_url = clean_text(
        request
            .get("base_url")
            .or_else(|| request.get("baseUrl"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        2200,
    );
    let media_out = api_media(
        root,
        &json!({
            "url": request.get("url").cloned().unwrap_or(Value::Null),
            "path": request.get("path").cloned().unwrap_or(Value::Null),
            "workspace_dir": request.get("workspace_dir").cloned().unwrap_or(Value::Null),
            "local_roots": request.get("local_roots").cloned().unwrap_or(Value::Null),
            "host_read_capability": request.get("host_read_capability").cloned().unwrap_or(Value::Null),
            "human_approved": request.get("human_approved").cloned().unwrap_or(Value::Null),
            "approval_id": request.get("approval_id").cloned().unwrap_or(Value::Null),
            "provider": request.get("provider").cloned().unwrap_or(Value::Null),
            "resolve_citation_redirect": request.get("resolve_citation_redirect").cloned().unwrap_or(Value::Bool(true)),
            "timeout_ms": request.get("timeout_ms").cloned().unwrap_or(Value::Null),
            "max_bytes": request.get("max_bytes").cloned().unwrap_or(Value::Null),
            "summary_only": true
        }),
    );
    if !media_out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        let mut payload = media_out;
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("type".to_string(), json!("web_conduit_media_host"));
            obj.insert(
                "media_host_contract".to_string(),
                web_media_host_contract(),
            );
        }
        return payload;
    }
    let artifact_path = clean_text(
        media_out
            .pointer("/artifact/path")
            .and_then(Value::as_str)
            .unwrap_or(""),
        2200,
    );
    if artifact_path.is_empty() {
        return hosted_media_error(
            "artifact_unavailable",
            json!({"media_host_contract": web_media_host_contract()}),
        );
    }
    let artifact_bytes = media_out
        .pointer("/artifact/bytes")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let file_name = clean_text(
        media_out
            .get("file_name")
            .and_then(Value::as_str)
            .unwrap_or("media.bin"),
        220,
    );
    let content_type = clean_text(
        media_out
            .get("content_type")
            .and_then(Value::as_str)
            .unwrap_or("application/octet-stream"),
        120,
    );
    let manifest = match store_hosted_media_entry(
        root,
        &PathBuf::from(&artifact_path),
        &file_name,
        &content_type,
        artifact_bytes,
        ttl_seconds,
    ) {
        Ok(row) => row,
        Err(err) => return err,
    };
    let hosted_id = manifest.get("id").and_then(Value::as_str).unwrap_or("");
    let route_path = hosted_media_route_path(hosted_id);
    let url = if base_url.is_empty() {
        route_path.clone()
    } else {
        format!("{}/api/web/media/{}", base_url.trim_end_matches('/'), hosted_id)
    };
    json!({
        "ok": true,
        "type": "web_conduit_media_host",
        "id": hosted_id,
        "route_path": route_path,
        "url": url,
        "expires_at": manifest.get("expires_at").cloned().unwrap_or(Value::Null),
        "ttl_seconds": ttl_seconds,
        "bytes": artifact_bytes,
        "file_name": file_name,
        "content_type": content_type,
        "artifact": media_out.get("artifact").cloned().unwrap_or(Value::Null),
        "requested_source": media_out.get("requested_source").cloned().unwrap_or(Value::Null),
        "resolved_source": media_out.get("resolved_source").cloned().unwrap_or(Value::Null),
        "media_host_contract": web_media_host_contract()
    })
}

pub fn api_media_host_read(root: &Path, hosted_id: &str) -> Value {
    let hosted_key = clean_text(hosted_id, MAX_HOSTED_MEDIA_ID_CHARS);
    if !is_valid_hosted_media_id(&hosted_key) {
        return hosted_media_error("invalid-path", json!({"hosted_id": hosted_key}));
    }
    let manifest_path = hosted_media_manifest_path(root, &hosted_key);
    if !manifest_path.exists() {
        return hosted_media_error("not-found", json!({"hosted_id": hosted_key}));
    }
    let manifest = read_json_or(&manifest_path, Value::Null);
    let expires_at = clean_text(
        manifest.get("expires_at").and_then(Value::as_str).unwrap_or(""),
        80,
    );
    let expired = DateTime::parse_from_rfc3339(&expires_at)
        .ok()
        .map(|row| row.with_timezone(&Utc) <= Utc::now())
        .unwrap_or(true);
    if expired {
        cleanup_hosted_media_entry(root, &hosted_key, &manifest);
        return hosted_media_error("expired", json!({"hosted_id": hosted_key}));
    }
    let data_path = clean_text(
        manifest.get("path").and_then(Value::as_str).unwrap_or(""),
        2200,
    );
    let hosted_dir_real = match fs::canonicalize(hosted_media_dir_path(root)) {
        Ok(row) => row,
        Err(_) => hosted_media_dir_path(root),
    };
    let real_path = match fs::canonicalize(PathBuf::from(&data_path)) {
        Ok(row) => row,
        Err(_) => return hosted_media_error("not-found", json!({"hosted_id": hosted_key})),
    };
    if !real_path.starts_with(&hosted_dir_real) {
        cleanup_hosted_media_entry(root, &hosted_key, &manifest);
        return hosted_media_error("outside-workspace", json!({"hosted_id": hosted_key}));
    }
    let bytes = match fs::read(&real_path) {
        Ok(row) => row,
        Err(_) => return hosted_media_error("not-found", json!({"hosted_id": hosted_key})),
    };
    if bytes.len() > 8 * 1024 * 1024 {
        cleanup_hosted_media_entry(root, &hosted_key, &manifest);
        return hosted_media_error(
            "too-large",
            json!({"hosted_id": hosted_key, "bytes": bytes.len()}),
        );
    }
    let content_type = clean_text(
        manifest
            .get("content_type")
            .and_then(Value::as_str)
            .unwrap_or("application/octet-stream"),
        120,
    );
    let file_name = clean_text(
        manifest
            .get("file_name")
            .and_then(Value::as_str)
            .unwrap_or("media.bin"),
        220,
    );
    use base64::Engine;
    let content_base64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let data_url = format!(
        "data:{};base64,{}",
        if content_type.is_empty() {
            "application/octet-stream"
        } else {
            content_type.as_str()
        },
        content_base64
    );
    cleanup_hosted_media_entry(root, &hosted_key, &manifest);
    json!({
        "ok": true,
        "type": "web_conduit_media_delivery",
        "id": hosted_key,
        "file_name": file_name,
        "content_type": content_type,
        "bytes": bytes.len(),
        "content_base64": content_base64,
        "data_url": data_url,
        "single_use": true
    })
}
