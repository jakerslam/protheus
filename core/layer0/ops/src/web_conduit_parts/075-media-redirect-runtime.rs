const MEDIA_REMOTE_MAX_REDIRECTS: usize = 5;
const MEDIA_CROSS_ORIGIN_REDIRECT_SAFE_HEADERS: &[&str] = &[
    "accept",
    "accept-encoding",
    "accept-language",
    "cache-control",
    "content-language",
    "content-type",
    "if-match",
    "if-modified-since",
    "if-none-match",
    "if-unmodified-since",
    "pragma",
    "range",
    "user-agent",
];

struct MediaFetchOnceResult {
    run_ok: bool,
    status_code: i64,
    content_type: String,
    effective_url: String,
    location: String,
    headers_raw: String,
    body: Vec<u8>,
    stderr: String,
    curl_error: String,
    declared_content_length: Option<usize>,
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

fn media_cleanup_temp_file(path: &Path) {
    let _ = fs::remove_file(path);
}

fn media_bounded_error_snippet(bytes: &[u8]) -> String {
    let probe = &bytes[..bytes.len().min(4096)];
    if media_bytes_look_binary(probe) {
        return String::new();
    }
    clean_text(&String::from_utf8_lossy(probe), 220)
}

fn parse_media_header_value(headers: &str, key: &str) -> String {
    header_value_from_block(headers, key)
}

fn parse_media_content_length(raw: &str) -> Option<usize> {
    clean_text(raw, 40)
        .parse::<u64>()
        .ok()
        .map(|row| row.min(usize::MAX as u64) as usize)
}

fn media_request_headers(request: &Value) -> Vec<(String, String)> {
    let candidate = request
        .get("headers")
        .or_else(|| request.get("http_headers"))
        .or_else(|| request.get("request_headers"));
    let Some(Value::Object(obj)) = candidate else {
        return Vec::new();
    };
    let mut deduped = std::collections::BTreeMap::<String, (String, String)>::new();
    for (raw_key, raw_value) in obj {
        let key = raw_key
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
            .take(80)
            .collect::<String>();
        let value = match raw_value {
            Value::String(row) => clean_text(row, 600),
            Value::Number(row) => clean_text(&row.to_string(), 600),
            Value::Bool(row) => clean_text(&row.to_string(), 600),
            _ => String::new(),
        };
        if key.is_empty() || value.is_empty() {
            continue;
        }
        deduped.insert(key.to_ascii_lowercase(), (key, value));
    }
    deduped.into_values().collect()
}

fn media_header_is_present(headers: &[(String, String)], key: &str) -> bool {
    headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case(key))
}

fn retain_media_safe_headers_for_cross_origin_redirect(
    headers: &[(String, String)],
) -> Vec<(String, String)> {
    headers
        .iter()
        .filter(|(name, _)| {
            MEDIA_CROSS_ORIGIN_REDIRECT_SAFE_HEADERS
                .iter()
                .any(|safe| name.eq_ignore_ascii_case(safe))
        })
        .cloned()
        .collect()
}

fn run_curl_media_fetch_once(
    root: &Path,
    url: &str,
    timeout_ms: u64,
    idle_timeout_ms: u64,
    max_bytes: usize,
    request_headers: &[(String, String)],
) -> Result<MediaFetchOnceResult, Value> {
    let header_path = media_temp_file(root, "headers");
    let body_path = media_temp_file(root, "body");
    let timeout_sec = ((timeout_ms as f64) / 1000.0).ceil() as u64;
    let idle_timeout_sec = ((idle_timeout_ms as f64) / 1000.0).ceil() as u64;
    let max_probe_bytes = max_bytes.saturating_add(1);
    let mut command = Command::new("curl");
    command
        .arg("-sS")
        .arg("--compressed")
        .arg("--proto")
        .arg("=http,https")
        .arg("--max-redirs")
        .arg("0")
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
        .arg("-e")
        .arg(DEFAULT_REFERER)
        .arg("-D")
        .arg(&header_path)
        .arg("-o")
        .arg(&body_path)
        .arg("-w")
        .arg("__STATUS__:%{http_code}\n__CTYPE__:%{content_type}\n__EFFECTIVE__:%{url_effective}\n__CLEN__:%header{content-length}\n__ERR__:%{errormsg}");
    if media_header_is_present(request_headers, "User-Agent") {
        for (name, value) in request_headers {
            command.arg("-H").arg(format!("{name}: {value}"));
        }
    } else {
        command.arg("-A").arg(DEFAULT_WEB_USER_AGENTS[0]);
        for (name, value) in request_headers {
            command.arg("-H").arg(format!("{name}: {value}"));
        }
    }
    if !media_header_is_present(request_headers, "Accept-Language") {
        command
            .arg("-H")
            .arg(format!("Accept-Language: {DEFAULT_ACCEPT_LANGUAGE}"));
    }
    let output = command.arg(url).output();
    let run = match output {
        Ok(row) => row,
        Err(err) => {
            media_cleanup_temp_file(&header_path);
            media_cleanup_temp_file(&body_path);
            return Err(json!({
                "ok": false,
                "error": "fetch_failed",
                "reason": clean_text(&format!("curl_spawn_failed:{err}"), 240),
                "resolved_url": redact_media_locator(url)
            }));
        }
    };
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let headers_raw = fs::read_to_string(&header_path).unwrap_or_default();
    let body = fs::read(&body_path).unwrap_or_default();
    media_cleanup_temp_file(&header_path);
    media_cleanup_temp_file(&body_path);
    let status_code = stdout
        .lines()
        .find_map(|line| line.strip_prefix("__STATUS__:"))
        .and_then(|row| clean_text(row, 12).parse::<i64>().ok())
        .unwrap_or(0);
    let content_type = stdout
        .lines()
        .find_map(|line| line.strip_prefix("__CTYPE__:"))
        .map(|row| clean_text(row, 180))
        .filter(|row| !row.is_empty())
        .unwrap_or_else(|| parse_media_header_value(&headers_raw, "content-type"));
    let effective_url = stdout
        .lines()
        .find_map(|line| line.strip_prefix("__EFFECTIVE__:"))
        .map(|row| clean_text(row, 2200))
        .filter(|row| !row.is_empty())
        .unwrap_or_else(|| clean_text(url, 2200));
    let location = parse_media_header_value(&headers_raw, "location");
    let curl_error = stdout
        .lines()
        .find_map(|line| line.strip_prefix("__ERR__:"))
        .map(|row| clean_text(row, 240))
        .unwrap_or_default();
    let declared_content_length = stdout
        .lines()
        .find_map(|line| line.strip_prefix("__CLEN__:"))
        .and_then(parse_media_content_length)
        .or_else(|| {
            parse_media_content_length(&parse_media_header_value(&headers_raw, "content-length"))
        });
    Ok(MediaFetchOnceResult {
        run_ok: run.status.success(),
        status_code,
        content_type,
        effective_url,
        location,
        headers_raw,
        body,
        stderr: clean_text(&String::from_utf8_lossy(&run.stderr), 320),
        curl_error,
        declared_content_length,
    })
}

fn fetch_remote_media_binary(root: &Path, request: &Value) -> Result<LoadedMedia, Value> {
    let raw_requested_url = clean_text(request.get("url").and_then(Value::as_str).unwrap_or(""), 2200);
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
    let requested_timeout_ms = request
        .get("timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or(9000)
        .clamp(1000, 120_000);
    let requested_idle_timeout_ms = request
        .get("idle_timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or(requested_timeout_ms.min(5000))
        .clamp(1000, 30_000);
    let max_bytes = media_prefetch_max_bytes(request);
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
    let selected_provider = fetch_provider_chain
        .first()
        .cloned()
        .unwrap_or_else(|| "direct_http".to_string());
    let policy_eval = crate::infring_layer1_security_bridge::evaluate_web_conduit_policy(
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
    let reason = clean_text(
        policy_eval
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("policy_denied"),
        180,
    );
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

    let mut current_url = resolved_url.clone();
    let mut request_headers = media_request_headers(request);
    let mut redirect_count = 0usize;
    for _ in 0..=MEDIA_REMOTE_MAX_REDIRECTS {
        let fetched = run_curl_media_fetch_once(
            root,
            &current_url,
            requested_timeout_ms,
            requested_idle_timeout_ms,
            max_bytes,
            &request_headers,
        )?;
        if matches!(fetched.status_code, 301 | 302 | 303 | 307 | 308) {
            if fetched.location.is_empty() {
                return Err(json!({
                    "ok": false,
                    "error": "invalid_redirect",
                    "reason": "redirect_missing_location",
                    "requested_url": redact_media_locator(&raw_requested_url),
                    "resolved_url": redact_media_locator(&current_url),
                    "provider": selected_provider,
                    "provider_hint": provider_hint,
                    "redirect_count": redirect_count
                }));
            }
            let Some(next_url) = resolve_fetch_redirect_url(&current_url, &fetched.location) else {
                return Err(json!({
                    "ok": false,
                    "error": "invalid_redirect_target",
                    "requested_url": redact_media_locator(&raw_requested_url),
                    "resolved_url": redact_media_locator(&current_url),
                    "redirect_location": clean_text(&fetched.location, 2200),
                    "provider": selected_provider,
                    "provider_hint": provider_hint,
                    "redirect_count": redirect_count
                }));
            };
            request_headers = if fetch_url_origin(&next_url)
                .eq_ignore_ascii_case(fetch_url_origin(&current_url).as_str())
            {
                request_headers
            } else {
                retain_media_safe_headers_for_cross_origin_redirect(&request_headers)
            };
            current_url = next_url;
            redirect_count += 1;
            continue;
        }
        if !fetched.run_ok {
            let lowered_stderr = format!("{} {}", fetched.stderr, fetched.curl_error).to_ascii_lowercase();
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
                "resolved_url": redact_media_locator(&current_url),
                "provider": selected_provider,
                "provider_hint": provider_hint,
                "stderr": fetched.stderr,
                "curl_error": fetched.curl_error,
                "declared_size": fetched.declared_content_length,
                "redirect_count": redirect_count
            }));
        }
        if fetched.status_code < 200 || fetched.status_code >= 400 {
            return Err(json!({
                "ok": false,
                "error": "http_error",
                "requested_url": redact_media_locator(&raw_requested_url),
                "resolved_url": redact_media_locator(&fetched.effective_url),
                "provider": selected_provider,
                "provider_hint": provider_hint,
                "status_code": fetched.status_code,
                "body_snippet": media_bounded_error_snippet(&fetched.body),
                "redirect_count": redirect_count
            }));
        }
        if fetched
            .declared_content_length
            .is_some_and(|row| row > max_bytes)
            || fetched.body.len() > max_bytes
        {
            return Err(json!({
                "ok": false,
                "error": "max_bytes",
                "requested_url": redact_media_locator(&raw_requested_url),
                "resolved_url": redact_media_locator(&fetched.effective_url),
                "status_code": fetched.status_code,
                "declared_size": fetched.declared_content_length,
                "redirect_count": redirect_count
            }));
        }
        let disposition = parse_media_header_value(&fetched.headers_raw, "content-disposition");
        let mut file_name = media_content_disposition_filename(&disposition)
            .or_else(|| media_file_name_from_url(&fetched.effective_url))
            .unwrap_or_else(|| "download.bin".to_string());
        let final_content_type =
            media_guess_content_type(Some(&file_name), &fetched.body, Some(&fetched.content_type));
        if Path::new(&file_name).extension().is_none() {
            if let Some(ext) = media_extension_for_content_type(&final_content_type) {
                file_name.push('.');
                file_name.push_str(ext);
            }
        }
        return Ok(LoadedMedia {
            buffer: fetched.body,
            content_type: final_content_type.clone(),
            kind: media_kind_from_content_type(&final_content_type),
            file_name,
            resolved_source: fetched.effective_url,
            source_kind: "remote".to_string(),
            status_code: fetched.status_code,
            provider: selected_provider,
            provider_hint,
            citation_redirect_resolved: redirect_resolved,
            redirect_count,
        });
    }
    Err(json!({
        "ok": false,
        "error": "too_many_redirects",
        "requested_url": redact_media_locator(&raw_requested_url),
        "resolved_url": redact_media_locator(&current_url),
        "provider": selected_provider,
        "provider_hint": provider_hint,
        "redirect_count": redirect_count
    }))
}
