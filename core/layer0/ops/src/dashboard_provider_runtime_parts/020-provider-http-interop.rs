fn curl_json(
    url: &str,
    method: &str,
    headers: &[String],
    body: Option<&Value>,
    timeout_secs: u64,
) -> Result<(u16, Value), String> {
    let mut cmd = Command::new("curl");
    cmd.arg("-sS")
        .arg("-L")
        .arg("-X")
        .arg(method)
        .arg("--connect-timeout")
        .arg("8")
        .arg("--max-time")
        .arg(timeout_secs.to_string());
    for header in headers {
        cmd.arg("-H").arg(header);
    }
    if body.is_some() {
        cmd.arg("--data-binary").arg("@-");
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::null());
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    cmd.arg("-w").arg("\n__HTTP_STATUS__:%{http_code}").arg(url);
    let mut child = cmd
        .spawn()
        .map_err(|err| format!("curl_spawn_failed:{err}"))?;
    if let Some(payload) = body {
        let encoded =
            serde_json::to_vec(payload).map_err(|err| format!("http_body_encode_failed:{err}"))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(&encoded)
                .map_err(|err| format!("curl_stdin_write_failed:{err}"))?;
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|err| format!("curl_wait_failed:{err}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = clean_text(&String::from_utf8_lossy(&output.stderr), 600);
    let marker = "\n__HTTP_STATUS__:";
    let Some(index) = stdout.rfind(marker) else {
        return Err(if stderr.is_empty() {
            "curl_http_status_missing".to_string()
        } else {
            stderr
        });
    };
    let body_raw = stdout[..index].trim();
    let status_raw = stdout[index + marker.len()..].trim();
    let status = status_raw.parse::<u16>().unwrap_or(0);
    let value = serde_json::from_str::<Value>(body_raw)
        .unwrap_or_else(|_| json!({"raw": clean_text(body_raw, 12_000)}));
    if !output.status.success() && status == 0 {
        return Err(if stderr.is_empty() {
            "curl_failed".to_string()
        } else {
            stderr
        });
    }
    Ok((status, value))
}

fn error_text_from_value(value: &Value) -> String {
    if let Some(text) = value.get("error").and_then(Value::as_str) {
        return clean_text(text, 280);
    }
    if let Some(text) = value
        .get("error")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get("message").and_then(Value::as_str))
    {
        return clean_text(text, 280);
    }
    if let Some(text) = value.get("message").and_then(Value::as_str) {
        return clean_text(text, 280);
    }
    clean_text(&value.to_string(), 280)
}

fn extract_openai_text(value: &Value) -> String {
    value
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(|text| clean_text(text, 32_000))
        .or_else(|| {
            value
                .pointer("/choices/0/text")
                .and_then(Value::as_str)
                .map(|text| clean_text(text, 32_000))
        })
        .unwrap_or_default()
}

fn extract_anthropic_text(value: &Value) -> String {
    value
        .get("content")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            row.get("text")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 12_000))
        })
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn extract_google_text(value: &Value) -> String {
    value
        .pointer("/candidates/0/content/parts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            row.get("text")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 12_000))
        })
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn model_context_window(root: &Path, provider_id: &str, model_name: &str) -> i64 {
    provider_row(root, provider_id)
        .get("model_profiles")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get(model_name))
        .and_then(|row| {
            row.get("context_window")
                .or_else(|| row.get("context_size"))
                .or_else(|| row.get("context_tokens"))
                .and_then(Value::as_i64)
        })
        .unwrap_or(0)
}

pub fn provider_rows(root: &Path, _snapshot: &Value) -> Vec<Value> {
    let registry = load_registry(root);
    let mut provider_ids = DEFAULT_PROVIDER_IDS
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>();
    if let Some(obj) = registry.get("providers").and_then(Value::as_object) {
        for key in obj.keys() {
            let provider_id = normalize_provider_id(key);
            if provider_id.is_empty() || provider_ids.iter().any(|row| row == &provider_id) {
                continue;
            }
            provider_ids.push(provider_id);
        }
    }
    let mut rows = provider_ids
        .into_iter()
        .map(|provider_id| provider_row(root, &provider_id))
        .collect::<Vec<_>>();
    for row in &mut rows {
        let provider_id =
            normalize_provider_id(row.get("id").and_then(Value::as_str).unwrap_or(""));
        if provider_id.is_empty() {
            continue;
        }
        row["id"] = json!(provider_id.clone());
        if provider_has_builtin_defaults(&provider_id)
            || row
                .get("display_name")
                .and_then(Value::as_str)
                .map(|value| clean_text(value, 120).is_empty())
                .unwrap_or(true)
        {
            row["display_name"] = json!(provider_display_name(&provider_id));
        }
        if provider_has_builtin_defaults(&provider_id) {
            row["is_local"] = json!(provider_is_local(&provider_id));
            row["needs_key"] = json!(provider_needs_key(&provider_id));
        }
        if row
            .get("api_key_env")
            .and_then(Value::as_str)
            .map(|value| clean_text(value, 120).is_empty())
            .unwrap_or(true)
        {
            row["api_key_env"] = json!(provider_api_key_env(&provider_id));
        }
        if row
            .get("base_url")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .is_empty()
        {
            row["base_url"] = json!(provider_base_url_default(&provider_id));
        }
        let base_url = clean_text(
            row.get("base_url").and_then(Value::as_str).unwrap_or(""),
            400,
        );
        let key_present = provider_key(root, &provider_id).is_some();
        let local_reachable = if provider_is_local(&provider_id) {
            local_provider_reachable(&provider_id, row)
        } else {
            row.get("reachable")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        };
        if provider_is_local(&provider_id) {
            row["auth_status"] = json!(if provider_id == "claude-code" && !local_reachable {
                "not_set"
            } else {
                "configured"
            });
            row["reachable"] = json!(local_reachable);
        } else if key_present {
            row["auth_status"] = json!("configured");
        } else if !row
            .get("auth_status")
            .and_then(Value::as_str)
            .map(auth_status_configured)
            .unwrap_or(false)
        {
            row["auth_status"] = json!("not_set");
        }
        row["supports_chat"] = json!(provider_supports_chat(&provider_id, &base_url));
        let mut profiles = row
            .get("model_profiles")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let profiles_missing = profiles.is_empty();
        if profiles_missing {
            profiles = model_profiles_for_provider(&provider_id);
        }
        let profiles_enriched = enrich_model_profiles_for_provider(&provider_id, &mut profiles);
        if profiles_missing || profiles_enriched {
            row["model_profiles"] = Value::Object(profiles);
        }
        if row
            .get("detected_models")
            .and_then(Value::as_array)
            .map(|rows| rows.is_empty())
            .unwrap_or(true)
        {
            let detected = row
                .get("model_profiles")
                .and_then(Value::as_object)
                .map(|obj| obj.keys().cloned().map(Value::String).collect::<Vec<_>>())
                .unwrap_or_default();
            row["detected_models"] = Value::Array(detected);
        }
        if provider_id == "google" {
            row["aliases"] = json!(["gemini"]);
        } else if provider_id == "moonshot" {
            row["aliases"] = json!(["kimi", "moonshot-ai"]);
        }
    }
    rows.sort_by(|a, b| {
        clean_text(a.get("id").and_then(Value::as_str).unwrap_or(""), 120).cmp(&clean_text(
            b.get("id").and_then(Value::as_str).unwrap_or(""),
            120,
        ))
    });
    rows
}

pub fn providers_payload(root: &Path, snapshot: &Value) -> Value {
    let virtual_keys = virtual_keys_payload(root);
    json!({
        "ok": true,
        "providers": provider_rows(root, snapshot),
        "routing": routing_policy(root),
        "virtual_keys_count": virtual_keys.get("count").cloned().unwrap_or_else(|| json!(0))
    })
}

pub fn save_provider_key(root: &Path, provider_id: &str, key: &str) -> Value {
    let provider = normalize_provider_id(provider_id);
    let secret = clean_text(key, 4096);
    if provider.is_empty() || secret.is_empty() || provider == "auto" {
        return json!({"ok": false, "error": "provider_key_invalid"});
    }
    let mut secrets = load_secrets(root);
    if secrets.get("providers").is_none()
        || !secrets
            .get("providers")
            .map(Value::is_object)
            .unwrap_or(false)
    {
        secrets["providers"] = json!({});
    }
    secrets["providers"][provider.clone()] = json!({"key": secret, "updated_at": crate::now_iso()});
    save_secrets(root, secrets);

    let mut registry = load_registry(root);
    let row = ensure_provider_row_mut(&mut registry, &provider);
    row["auth_status"] = json!("configured");
    row["key_prefix"] = json!(masked_prefix(key));
    row["key_last4"] = json!(masked_last4(key));
    row["key_hash"] = json!(crate::deterministic_receipt_hash(
        &json!({"provider": provider, "key": key})
    ));
    row["key_set_at"] = json!(crate::now_iso());
    row["updated_at"] = json!(crate::now_iso());
    save_registry(root, registry);
    json!({
        "ok": true,
        "provider": provider,
        "auth_status": "configured",
        "switched_default": false
    })
}

pub fn remove_provider_key(root: &Path, provider_id: &str) -> Value {
    let provider = normalize_provider_id(provider_id);
    let mut secrets = load_secrets(root);
    if let Some(obj) = secrets.get_mut("providers").and_then(Value::as_object_mut) {
        obj.remove(&provider);
    }
    save_secrets(root, secrets);
    let mut registry = load_registry(root);
    let row = ensure_provider_row_mut(&mut registry, &provider);
    row["auth_status"] = json!(if provider_is_local(&provider) {
        "configured"
    } else {
        "not_set"
    });
    row["key_prefix"] = json!("");
    row["key_last4"] = json!("");
    row["key_hash"] = json!("");
    row["key_set_at"] = json!("");
    row["updated_at"] = json!(crate::now_iso());
    save_registry(root, registry);
    json!({"ok": true, "provider": provider})
}

pub fn set_provider_url(root: &Path, provider_id: &str, base_url: &str) -> Value {
    let provider = normalize_provider_id(provider_id);
    let cleaned = clean_text(base_url, 400);
    if provider.is_empty() || cleaned.is_empty() {
        return json!({"ok": false, "error": "provider_url_invalid"});
    }
    let mut registry = load_registry(root);
    let row = ensure_provider_row_mut(&mut registry, &provider);
    row["base_url"] = json!(cleaned);
    row["updated_at"] = json!(crate::now_iso());
    save_registry(root, registry);
    let probe = test_provider(root, &provider);
    json!({
        "ok": probe.get("status").and_then(Value::as_str) == Some("ok"),
        "provider": provider,
        "reachable": probe.get("status").and_then(Value::as_str) == Some("ok"),
        "latency_ms": probe.get("latency_ms").cloned().unwrap_or_else(|| json!(0)),
        "error": probe.get("error").cloned().unwrap_or(Value::Null)
    })
}

pub fn test_provider(root: &Path, provider_id: &str) -> Value {
    let provider = normalize_provider_id(provider_id);
    let started = Instant::now();
    if provider == "claude-code" {
        let ok = Command::new("sh")
            .arg("-lc")
            .arg("command -v claude >/dev/null 2>&1")
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        return if ok {
            json!({"ok": true, "status": "ok", "provider": provider, "latency_ms": started.elapsed().as_millis() as i64})
        } else {
            json!({"ok": false, "status": "error", "provider": provider, "error": "claude_code_cli_not_detected"})
        };
    }

    if provider == "auto" {
        let providers = provider_rows(root, &json!({}));
        let ready = providers.into_iter().any(|row| {
            row.get("is_local")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || auth_status_configured(
                    row.get("auth_status").and_then(Value::as_str).unwrap_or(""),
                )
        });
        return if ready {
            json!({"ok": true, "status": "ok", "provider": provider, "latency_ms": started.elapsed().as_millis() as i64})
        } else {
            json!({"ok": false, "status": "error", "provider": provider, "error": "no_configured_provider"})
        };
    }

    let row = provider_row(root, &provider);
    let base_url = clean_text(
        row.get("base_url")
            .and_then(Value::as_str)
            .unwrap_or(&provider_base_url_default(&provider)),
        400,
    );
    let mut headers = vec!["Content-Type: application/json".to_string()];
    let url = match provider.as_str() {
        "ollama" => format!("{base_url}/api/tags"),
        "google" => {
            let Some(key) = provider_key(root, &provider) else {
                return json!({"ok": false, "status": "error", "provider": provider, "error": "provider_key_missing"});
            };
            format!("{base_url}/v1beta/models?key={key}")
        }
        "anthropic" => {
            let Some(key) = provider_key(root, &provider) else {
                return json!({"ok": false, "status": "error", "provider": provider, "error": "provider_key_missing"});
            };
            headers.push(format!("x-api-key: {key}"));
            headers.push("anthropic-version: 2023-06-01".to_string());
            format!("{base_url}/v1/models")
        }
        _ => {
            let Some(key) = provider_key(root, &provider) else {
                return json!({"ok": false, "status": "error", "provider": provider, "error": "provider_key_missing"});
            };
            headers.push(format!("Authorization: Bearer {key}"));
            format!("{base_url}/models")
        }
    };

    match curl_json(&url, "GET", &headers, None, 20) {
        Ok((status, value)) if status >= 200 && status < 300 => {
            let mut registry = load_registry(root);
            let row = ensure_provider_row_mut(&mut registry, &provider);
            row["reachable"] = json!(true);
            row["updated_at"] = json!(crate::now_iso());
            save_registry(root, registry);
            json!({
                "ok": true,
                "status": "ok",
                "provider": provider,
                "latency_ms": started.elapsed().as_millis() as i64,
                "detail": value
            })
        }
        Ok((status, value)) => {
            let mut registry = load_registry(root);
            let row = ensure_provider_row_mut(&mut registry, &provider);
            row["reachable"] = json!(false);
            row["updated_at"] = json!(crate::now_iso());
            save_registry(root, registry);
            json!({
                "ok": false,
                "status": "error",
                "provider": provider,
                "error": format!("http_{status}:{}", error_text_from_value(&value))
            })
        }
        Err(err) => {
            let mut registry = load_registry(root);
            let row = ensure_provider_row_mut(&mut registry, &provider);
            row["reachable"] = json!(false);
            row["updated_at"] = json!(crate::now_iso());
            save_registry(root, registry);
            json!({"ok": false, "status": "error", "provider": provider, "error": clean_text(&err, 280)})
        }
    }
}
