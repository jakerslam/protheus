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

fn media_temp_file(root: &Path, label: &str) -> PathBuf {
    let dir = artifacts_dir_path(root).join(".tmp-media");
    let _ = fs::create_dir_all(&dir);
    dir.join(format!(
        "{}-{}-{}",
        label,
        std::process::id(),
        Utc::now().timestamp_millis()
    ))
}

fn media_bounded_error_snippet(bytes: &[u8]) -> String {
    let probe = &bytes[..bytes.len().min(4096)];
    if media_bytes_look_binary(probe) {
        return String::new();
    }
    clean_text(&String::from_utf8_lossy(probe), 220)
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

fn parse_media_header_value(headers: &str, key: &str) -> String {
    headers
        .lines()
        .rev()
        .find_map(|line| {
            let trimmed = line.trim();
            let lowered = trimmed.to_ascii_lowercase();
            let prefix = format!("{}:", key.to_ascii_lowercase());
            lowered
                .strip_prefix(&prefix)
                .map(|_| clean_text(trimmed.split_once(':').map(|row| row.1).unwrap_or(""), 400))
        })
        .unwrap_or_default()
}

fn parse_media_content_length(raw: &str) -> Option<usize> {
    clean_text(raw, 40)
        .parse::<u64>()
        .ok()
        .map(|row| row.min(usize::MAX as u64) as usize)
}

fn fetch_remote_media_binary(
    root: &Path,
    request: &Value,
) -> Result<LoadedMedia, Value> {
    let raw_requested_url = clean_text(
        request.get("url").and_then(Value::as_str).unwrap_or(""),
        2200,
    );
    let provider_hint = clean_text(
        request
            .get("provider")
            .or_else(|| request.get("fetch_provider"))
            .and_then(Value::as_str)
            .unwrap_or("auto"),
        40,
    )
    .to_ascii_lowercase();
    if let Some(unknown_provider) = validate_explicit_fetch_provider_hint(&provider_hint) {
        return Err(json!({
            "ok": false,
            "error": "unknown_fetch_provider",
            "requested_url": raw_requested_url,
            "requested_provider": unknown_provider
        }));
    }
    let human_approved = request.get("human_approved").and_then(Value::as_bool).unwrap_or(false);
    let approval_id = clean_text(request.get("approval_id").and_then(Value::as_str).unwrap_or(""), 160);
    let requested_timeout_ms = request.get("timeout_ms").and_then(Value::as_u64).unwrap_or(9000).clamp(1000, 120_000);
    let requested_idle_timeout_ms = request
        .get("idle_timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or(requested_timeout_ms.min(5000))
        .clamp(1000, 30_000);
    let max_bytes = request.get("max_bytes").and_then(Value::as_u64).unwrap_or(8 * 1024 * 1024).clamp(256, 32 * 1024 * 1024) as usize;
    let resolve_redirect = request
        .get("resolve_citation_redirect")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let (resolved_url, redirect_resolved) = if resolve_redirect {
        resolve_citation_redirect_url(&raw_requested_url, requested_timeout_ms)
    } else {
        (raw_requested_url.clone(), false)
    };
    let approval_state = approval_state_for_request(root, &approval_id, &resolved_url);
    let effective_human_approved = human_approved || approval_state.as_deref() == Some("approved");
    let (policy, _) = load_policy(root);
    let fetch_provider_chain = fetch_provider_chain_from_request(&provider_hint, request, &policy);
    let selected_provider = fetch_provider_chain.first().cloned().unwrap_or_else(|| "direct_http".to_string());
    let policy_eval = infring_layer1_security::evaluate_web_conduit_policy(
        root,
        &json!({
            "requested_url": resolved_url,
            "domain": extract_domain(&resolved_url),
            "human_approved": effective_human_approved,
            "requests_last_minute": requests_last_minute(root)
        }),
        &policy,
    );
    let allow = policy_eval.get("allow").and_then(Value::as_bool).unwrap_or(false);
    let reason = clean_text(policy_eval.get("reason").and_then(Value::as_str).unwrap_or("policy_denied"), 180);
    if !allow {
        let approval = if reason == "human_approval_required_for_sensitive_domain" {
            ensure_sensitive_web_approval(root, &resolved_url, &policy_eval)
        } else {
            None
        };
        return Err(json!({
            "ok": false,
            "error": "web_conduit_policy_denied",
            "requested_url": raw_requested_url,
            "resolved_url": resolved_url,
            "citation_redirect_resolved": redirect_resolved,
            "policy_decision": policy_eval,
            "approval_required": approval.is_some(),
            "approval": approval,
            "approval_state": approval_state
        }));
    }

    let header_path = media_temp_file(root, "headers");
    let body_path = media_temp_file(root, "body");
    let timeout_sec = ((requested_timeout_ms as f64) / 1000.0).ceil() as u64;
    let idle_timeout_sec = ((requested_idle_timeout_ms as f64) / 1000.0).ceil() as u64;
    let max_probe_bytes = max_bytes.saturating_add(1);
    let output = Command::new("curl")
        .arg("-sS")
        .arg("-L")
        .arg("--compressed")
        .arg("--proto")
        .arg("=http,https")
        .arg("--connect-timeout")
        .arg(timeout_sec.max(1).to_string())
        .arg("--max-time")
        .arg(timeout_sec.max(1).to_string())
        .arg("--speed-limit")
        .arg("1")
        .arg("--speed-time")
        .arg(idle_timeout_sec.to_string())
        .arg("-r")
        .arg(format!("0-{}", max_bytes))
        .arg("--max-filesize")
        .arg(max_probe_bytes.to_string())
        .arg("-A")
        .arg(DEFAULT_WEB_USER_AGENTS[0])
        .arg("-H")
        .arg(format!("Accept-Language: {DEFAULT_ACCEPT_LANGUAGE}"))
        .arg("-e")
        .arg(DEFAULT_REFERER)
        .arg("-D")
        .arg(&header_path)
        .arg("-o")
        .arg(&body_path)
        .arg("-w")
        .arg("__STATUS__:%{http_code}\n__CTYPE__:%{content_type}\n__EFFECTIVE__:%{url_effective}\n__CLEN__:%header{content-length}\n__ERR__:%{errormsg}")
        .arg(&resolved_url)
        .output();
    let cleanup = || {
        let _ = fs::remove_file(&header_path);
        let _ = fs::remove_file(&body_path);
    };
    let run = match output {
        Ok(row) => row,
        Err(err) => {
            cleanup();
            return Err(json!({
                "ok": false,
                "error": "fetch_failed",
                "reason": clean_text(&format!("curl_spawn_failed:{err}"), 240),
                "requested_url": redact_media_locator(&raw_requested_url)
            }));
        }
    };
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let stderr = clean_text(&String::from_utf8_lossy(&run.stderr), 320);
    let headers = fs::read_to_string(&header_path).unwrap_or_default();
    let bytes = fs::read(&body_path).unwrap_or_default();
    cleanup();
    let curl_error = stdout
        .lines()
        .find_map(|line| line.strip_prefix("__ERR__:"))
        .map(|row| clean_text(row, 240))
        .unwrap_or_default();
    let declared_content_length = stdout
        .lines()
        .find_map(|line| line.strip_prefix("__CLEN__:"))
        .and_then(parse_media_content_length)
        .or_else(|| parse_media_content_length(&parse_media_header_value(&headers, "content-length")));
    if !run.status.success() {
        let lowered_stderr = format!("{stderr} {curl_error}").to_ascii_lowercase();
        let code = if lowered_stderr.contains("maximum file size exceeded") {
            "max_bytes"
        } else if lowered_stderr.contains("operation too slow")
            || lowered_stderr.contains("timed out")
            || lowered_stderr.contains("timeout")
        {
            "fetch_stalled"
        } else {
            "fetch_failed"
        };
        return Err(json!({
            "ok": false,
            "error": code,
            "requested_url": redact_media_locator(&raw_requested_url),
            "resolved_url": redact_media_locator(&resolved_url),
            "provider": selected_provider,
            "provider_hint": provider_hint,
            "stderr": stderr,
            "curl_error": curl_error,
            "declared_size": declared_content_length
        }));
    }
    let status_code = stdout
        .lines()
        .find_map(|line| line.strip_prefix("__STATUS__:"))
        .and_then(|row| row.trim().parse::<i64>().ok())
        .unwrap_or(0);
    let content_type = stdout
        .lines()
        .find_map(|line| line.strip_prefix("__CTYPE__:"))
        .map(|row| clean_text(row, 180))
        .filter(|row| !row.is_empty())
        .unwrap_or_else(|| parse_media_header_value(&headers, "content-type"));
    let effective_url = stdout
        .lines()
        .find_map(|line| line.strip_prefix("__EFFECTIVE__:"))
        .map(|row| clean_text(row, 2200))
        .filter(|row| !row.is_empty())
        .unwrap_or_else(|| resolved_url.clone());
    if status_code < 200 || status_code >= 400 {
        let snippet = media_bounded_error_snippet(&bytes);
        return Err(json!({
            "ok": false,
            "error": "http_error",
            "requested_url": redact_media_locator(&raw_requested_url),
            "resolved_url": redact_media_locator(&effective_url),
            "provider": selected_provider,
            "provider_hint": provider_hint,
            "status_code": status_code,
            "body_snippet": snippet
        }));
    }
    if declared_content_length.is_some_and(|row| row > max_bytes) {
        return Err(json!({
            "ok": false,
            "error": "max_bytes",
            "requested_url": redact_media_locator(&raw_requested_url),
            "resolved_url": redact_media_locator(&effective_url),
            "status_code": status_code,
            "declared_size": declared_content_length
        }));
    }
    if bytes.len() > max_bytes {
        return Err(json!({
            "ok": false,
            "error": "max_bytes",
            "requested_url": redact_media_locator(&raw_requested_url),
            "resolved_url": redact_media_locator(&effective_url),
            "status_code": status_code,
            "declared_size": declared_content_length
        }));
    }
    let disposition = parse_media_header_value(&headers, "content-disposition");
    let mut file_name = media_content_disposition_filename(&disposition)
        .or_else(|| media_file_name_from_url(&effective_url))
        .unwrap_or_else(|| "download.bin".to_string());
    let final_content_type = media_guess_content_type(Some(&file_name), &bytes, Some(&content_type));
    if Path::new(&file_name).extension().is_none() {
        if let Some(ext) = media_extension_for_content_type(&final_content_type) {
            file_name.push('.');
            file_name.push_str(ext);
        }
    }
    Ok(LoadedMedia {
        buffer: bytes,
        content_type: final_content_type.clone(),
        kind: media_kind_from_content_type(&final_content_type),
        file_name,
        resolved_source: effective_url,
        source_kind: "remote".to_string(),
        status_code,
        provider: selected_provider,
        provider_hint,
        citation_redirect_resolved: redirect_resolved,
    })
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
    })
}

pub fn api_media(root: &Path, request: &Value) -> Value {
    let raw = media_request_source(request);
    if raw.is_empty() {
        return json!({"ok": false, "error": "media_source_required"});
    }
    let summary_only = request.get("summary_only").and_then(Value::as_bool).unwrap_or(false);
    let loaded = if raw.starts_with("http://") || raw.starts_with("https://") {
        fetch_remote_media_binary(root, request)
    } else {
        load_local_media_binary(root, request)
    };
    match loaded {
        Ok(loaded) => {
            let artifact = persist_media_artifact(root, &loaded).unwrap_or(Value::Null);
            let response_hash = sha256_hex(&String::from_utf8_lossy(&loaded.buffer));
            let receipt = build_receipt(
                &raw,
                "allow",
                Some(&response_hash),
                loaded.status_code,
                if loaded.source_kind == "remote" { "media_loaded" } else { "local_media_loaded" },
                None,
            );
            let _ = append_jsonl(&receipts_path(root), &receipt);
            let include_inline = !summary_only && loaded.buffer.len() <= 512_000;
            json!({
                "ok": true,
                "type": "web_conduit_media",
                "requested_source": raw,
                "resolved_source": loaded.resolved_source,
                "source_kind": loaded.source_kind,
                "provider": loaded.provider,
                "provider_hint": loaded.provider_hint,
                "citation_redirect_resolved": loaded.citation_redirect_resolved,
                "status_code": loaded.status_code,
                "content_type": loaded.content_type,
                "kind": loaded.kind,
                "file_name": loaded.file_name,
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
