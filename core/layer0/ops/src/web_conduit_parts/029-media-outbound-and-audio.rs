const DEFAULT_OUTBOUND_MEDIA_SUBDIR: &str = "outbound";
const TELEGRAM_VOICE_AUDIO_EXTENSIONS: &[&str] = &["oga", "ogg", "opus", "mp3", "m4a"];
const TELEGRAM_VOICE_MIME_TYPES: &[&str] = &[
    "audio/ogg",
    "audio/opus",
    "audio/mpeg",
    "audio/mp3",
    "audio/mp4",
    "audio/x-m4a",
    "audio/m4a",
];

fn normalize_request_string(request: &Value, key: &str, aliases: &[&str], max_len: usize) -> String {
    request
        .get(key)
        .or_else(|| aliases.iter().find_map(|alias| request.get(*alias)))
        .and_then(Value::as_str)
        .map(|row| clean_text(row, max_len))
        .unwrap_or_default()
}

fn normalize_request_bool(request: &Value, key: &str, aliases: &[&str]) -> bool {
    request
        .get(key)
        .or_else(|| aliases.iter().find_map(|alias| request.get(*alias)))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn outbound_media_local_roots_value(request: &Value) -> Value {
    let top_level = request
        .get("local_roots")
        .or_else(|| request.get("localRoots"))
        .or_else(|| request.get("local_root"))
        .or_else(|| request.get("localRoot"));
    if top_level.is_some() {
        return top_level.cloned().unwrap_or(Value::Null);
    }
    request
        .pointer("/media_access/local_roots")
        .or_else(|| request.pointer("/media_access/localRoots"))
        .or_else(|| request.pointer("/mediaAccess/local_roots"))
        .or_else(|| request.pointer("/mediaAccess/localRoots"))
        .cloned()
        .unwrap_or(Value::Null)
}

fn outbound_media_workspace_dir(request: &Value) -> String {
    let top_level = normalize_request_string(request, "workspace_dir", &["workspaceDir"], 2200);
    if !top_level.is_empty() {
        return top_level;
    }
    request
        .pointer("/media_access/workspace_dir")
        .or_else(|| request.pointer("/media_access/workspaceDir"))
        .or_else(|| request.pointer("/mediaAccess/workspace_dir"))
        .or_else(|| request.pointer("/mediaAccess/workspaceDir"))
        .and_then(Value::as_str)
        .map(|row| clean_text(row, 2200))
        .unwrap_or_default()
}

fn outbound_media_host_read_capability(request: &Value) -> bool {
    let top_level = normalize_request_bool(
        request,
        "host_read_capability",
        &["allow_host_read", "hostReadCapability", "allowHostRead"],
    );
    if top_level {
        return true;
    }
    request
        .pointer("/media_access/host_read_capability")
        .or_else(|| request.pointer("/media_access/allow_host_read"))
        .or_else(|| request.pointer("/mediaAccess/host_read_capability"))
        .or_else(|| request.pointer("/mediaAccess/allowHostRead"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn build_outbound_media_request(request: &Value) -> Value {
    json!({
        "url": normalize_request_string(request, "url", &[], 4000),
        "path": normalize_request_string(request, "path", &[], 4000),
        "workspace_dir": outbound_media_workspace_dir(request),
        "local_roots": outbound_media_local_roots_value(request),
        "host_read_capability": outbound_media_host_read_capability(request),
        "human_approved": normalize_request_bool(request, "human_approved", &["humanApproved"]),
        "approval_id": normalize_request_string(request, "approval_id", &["approvalId"], 160),
        "provider": normalize_request_string(request, "provider", &["fetch_provider", "fetchProvider"], 40),
        "resolve_citation_redirect": request
            .get("resolve_citation_redirect")
            .or_else(|| request.get("resolveCitationRedirect"))
            .and_then(Value::as_bool)
            .unwrap_or(true),
        "timeout_ms": request.get("timeout_ms").cloned().unwrap_or(Value::Null),
        "max_bytes": request.get("max_bytes").cloned().unwrap_or(Value::Null),
        "summary_only": true
    })
}

fn normalized_audio_file_extension(file_name: Option<&str>) -> String {
    file_name
        .and_then(|row| Path::new(row).extension().and_then(|ext| ext.to_str()))
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn is_telegram_voice_compatible_audio(content_type: Option<&str>, file_name: Option<&str>) -> bool {
    let normalized_mime = content_type
        .map(normalize_media_content_type)
        .unwrap_or_default();
    if TELEGRAM_VOICE_MIME_TYPES
        .iter()
        .any(|row| normalized_mime == *row)
    {
        return true;
    }
    TELEGRAM_VOICE_AUDIO_EXTENSIONS
        .iter()
        .any(|row| normalized_audio_file_extension(file_name) == *row)
}

fn web_media_voice_contract() -> Value {
    json!({
        "default_delivery_mode": "file",
        "voice_compatible_field": "voice_compatible_audio",
        "telegram_voice_extensions": TELEGRAM_VOICE_AUDIO_EXTENSIONS,
        "telegram_voice_mime_types": TELEGRAM_VOICE_MIME_TYPES
    })
}

fn web_media_outbound_attachment_contract() -> Value {
    json!({
        "request_fields": [
            "url",
            "path",
            "workspace_dir",
            "local_roots",
            "host_read_capability",
            "media_access.workspace_dir",
            "media_access.local_roots",
            "media_access.host_read_capability"
        ],
        "workspace_dir_precedence": "top_level_over_media_access",
        "default_store_subdir": DEFAULT_OUTBOUND_MEDIA_SUBDIR,
        "saved_id_shape": media_store_contract().get("saved_id_shape").cloned().unwrap_or(Value::Null),
        "media_store_contract": media_store_contract(),
        "returns": ["path", "content_type", "file_name", "saved_media.id", "saved_media.bytes"],
        "voice_audio_contract": web_media_voice_contract()
    })
}

fn append_web_media_outbound_tool_entry(tool_catalog: &mut Value, policy: &Value) {
    if let Some(rows) = tool_catalog.as_array_mut() {
        rows.push(json!({
            "tool": "web_media_outbound_attachment",
            "label": "Web Media Outbound Attachment",
            "family": "media",
            "enabled": policy.pointer("/web_conduit/enabled").and_then(Value::as_bool).unwrap_or(true),
            "request_contract": web_media_outbound_attachment_contract()
        }));
    }
}

fn cli_media_request_from_parsed(parsed: &crate::ParsedArgs) -> Value {
    json!({
        "url": clean_text(parsed.flags.get("url").map(String::as_str).unwrap_or(""), 4000),
        "path": clean_text(parsed.flags.get("path").map(String::as_str).unwrap_or_else(|| parsed.positional.get(1).map(String::as_str).unwrap_or("")), 4000),
        "workspace_dir": clean_text(parsed.flags.get("workspace-dir").or_else(|| parsed.flags.get("workspace_dir")).map(String::as_str).unwrap_or(""), 2200),
        "local_roots": clean_text(parsed.flags.get("local-roots").or_else(|| parsed.flags.get("local_roots")).or_else(|| parsed.flags.get("local-root")).or_else(|| parsed.flags.get("local_root")).map(String::as_str).unwrap_or(""), 4000),
        "host_read_capability": parse_bool(parsed.flags.get("host-read-capability"))
            || parse_bool(parsed.flags.get("host_read_capability"))
            || parse_bool(parsed.flags.get("allow-host-read"))
            || parse_bool(parsed.flags.get("allow_host_read")),
        "human_approved": parse_bool(parsed.flags.get("human-approved")) || parse_bool(parsed.flags.get("human_approved")),
        "approval_id": clean_text(parsed.flags.get("approval-id").or_else(|| parsed.flags.get("approval_id")).map(String::as_str).unwrap_or(""), 160),
        "summary_only": parse_bool(parsed.flags.get("summary-only")) || parse_bool(parsed.flags.get("summary_only")),
        "provider": clean_text(
            parsed
                .flags
                .get("provider")
                .or_else(|| parsed.flags.get("fetch-provider"))
                .or_else(|| parsed.flags.get("fetch_provider"))
                .map(String::as_str)
                .unwrap_or("auto"),
            40
        ),
        "resolve_citation_redirect": parsed.flags.get("resolve-citation-redirect")
            .or_else(|| parsed.flags.get("resolve_citation_redirect"))
            .map(|raw| !matches!(raw.trim().to_ascii_lowercase().as_str(), "0" | "false" | "no" | "off"))
            .unwrap_or(true),
        "timeout_ms": parse_u64(parsed.flags.get("timeout-ms").or_else(|| parsed.flags.get("timeout_ms")), 9000, 1000, 120000),
        "max_bytes": parse_u64(parsed.flags.get("max-bytes").or_else(|| parsed.flags.get("max_bytes")), 8 * 1024 * 1024, 4096, 32 * 1024 * 1024)
    })
}

fn cli_media_host_request_from_parsed(parsed: &crate::ParsedArgs) -> Value {
    let mut request = cli_media_request_from_parsed(parsed);
    if let Some(obj) = request.as_object_mut() {
        obj.insert(
            "ttl_seconds".to_string(),
            json!(parse_u64(
                parsed.flags.get("ttl-seconds").or_else(|| parsed.flags.get("ttl_seconds")),
                DEFAULT_HOSTED_MEDIA_TTL_SECONDS,
                1,
                MAX_HOSTED_MEDIA_TTL_SECONDS
            )),
        );
        obj.insert(
            "base_url".to_string(),
            json!(clean_text(
                parsed.flags.get("base-url").or_else(|| parsed.flags.get("base_url")).map(String::as_str).unwrap_or(""),
                2200
            )),
        );
    }
    request
}

fn cli_outbound_attachment_request_from_parsed(parsed: &crate::ParsedArgs) -> Value {
    let mut request = cli_media_request_from_parsed(parsed);
    if let Some(obj) = request.as_object_mut() {
        obj.insert(
            "media_access".to_string(),
            json!({
                "workspace_dir": clean_text(parsed.flags.get("media-access-workspace-dir").or_else(|| parsed.flags.get("media_access_workspace_dir")).map(String::as_str).unwrap_or(""), 2200),
                "local_roots": clean_text(parsed.flags.get("media-access-local-roots").or_else(|| parsed.flags.get("media_access_local_roots")).map(String::as_str).unwrap_or(""), 4000),
                "host_read_capability": parse_bool(parsed.flags.get("media-access-host-read-capability"))
                    || parse_bool(parsed.flags.get("media_access_host_read_capability"))
            }),
        );
    }
    request
}

pub fn api_outbound_attachment(root: &Path, request: &Value) -> Value {
    let media_request = build_outbound_media_request(request);
    let media_out = api_media(root, &media_request);
    if !media_out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        let mut raw_source = normalize_request_string(request, "path", &[], 4000);
        if raw_source.is_empty() {
            raw_source = normalize_request_string(request, "url", &[], 4000);
        }
        let mut payload = map_media_store_source_error(&raw_source, &media_out);
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("type".to_string(), json!("web_conduit_outbound_attachment"));
            obj.insert(
                "outbound_attachment_contract".to_string(),
                web_media_outbound_attachment_contract(),
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
        return json!({
            "ok": false,
            "type": "web_conduit_outbound_attachment",
            "error": "artifact_unavailable",
            "outbound_attachment_contract": web_media_outbound_attachment_contract()
        });
    }
    let file_name = clean_text(
        media_out.get("file_name").and_then(Value::as_str).unwrap_or("attachment.bin"),
        220,
    );
    let content_type = clean_text(
        media_out.get("content_type").and_then(Value::as_str).unwrap_or("application/octet-stream"),
        120,
    );
    let bytes = media_out.get("bytes").and_then(Value::as_u64).unwrap_or(0) as usize;
    let saved_media = match store_media_artifact_copy(
        root,
        &PathBuf::from(&artifact_path),
        &file_name,
        &content_type,
        bytes,
        DEFAULT_OUTBOUND_MEDIA_SUBDIR,
    ) {
        Ok(value) => value,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "web_conduit_outbound_attachment",
                "error": clean_text(err.get("error").and_then(Value::as_str).unwrap_or("stored_media_copy_failed"), 120),
                "detail": err.get("detail").cloned().unwrap_or(Value::Null),
                "outbound_attachment_contract": web_media_outbound_attachment_contract()
            })
        }
    };
    json!({
        "ok": true,
        "type": "web_conduit_outbound_attachment",
        "requested_source": media_out.get("requested_source").cloned().unwrap_or(Value::Null),
        "resolved_source": media_out.get("resolved_source").cloned().unwrap_or(Value::Null),
        "content_type": content_type,
        "file_name": file_name,
        "bytes": bytes,
        "voice_compatible_audio": is_telegram_voice_compatible_audio(Some(&content_type), Some(&file_name)),
        "path": saved_media.get("path").cloned().unwrap_or(Value::Null),
        "saved_media": saved_media,
        "artifact": media_out.get("artifact").cloned().unwrap_or(Value::Null),
        "outbound_attachment_contract": web_media_outbound_attachment_contract(),
        "receipt": media_out.get("receipt").cloned().unwrap_or(Value::Null)
    })
}
