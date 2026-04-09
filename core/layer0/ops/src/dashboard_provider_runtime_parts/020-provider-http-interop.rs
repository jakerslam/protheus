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

fn clean_chat_text(raw: &str, max_len: usize) -> String {
    raw.replace("\r\n", "\n")
        .replace('\r', "\n")
        .chars()
        .filter(|ch| *ch == '\n' || *ch == '\t' || !ch.is_control())
        .take(max_len)
        .collect::<String>()
}

fn extract_openai_text(value: &Value) -> String {
    value
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(|text| clean_chat_text(text, 32_000))
        .or_else(|| {
            value
                .pointer("/choices/0/text")
                .and_then(Value::as_str)
                .map(|text| clean_chat_text(text, 32_000))
        })
        .unwrap_or_default()
}

fn extract_text_rows(value: &Value, pointer: &str, max_len: usize) -> String {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            row.get("text")
                .and_then(Value::as_str)
                .map(|v| clean_chat_text(v, max_len))
        })
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn extract_frontier_provider_text(value: &Value) -> String {
    extract_text_rows(value, "/content", 12_000)
}

fn extract_google_text(value: &Value) -> String {
    extract_text_rows(value, "/candidates/0/content/parts", 12_000)
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

fn model_ref_from_probe(provider_id: &str, raw: &str) -> String {
    let mut model = clean_text(raw, 240);
    if provider_id == "google" {
        if let Some((_, tail)) = model.rsplit_once('/') {
            model = clean_text(tail, 240);
        }
    }
    if model_id_is_placeholder(&model) {
        String::new()
    } else {
        model
    }
}

fn models_from_probe_response(provider_id: &str, value: &Value) -> Vec<String> {
    let provider = normalize_provider_id(provider_id);
    let mut out = Vec::<String>::new();
    let mut push = |candidate: &str| {
        let cleaned = model_ref_from_probe(&provider, candidate);
        if cleaned.is_empty() || out.iter().any(|row| row == &cleaned) {
            return;
        }
        out.push(cleaned);
    };

    if provider == "ollama" {
        if let Some(rows) = value.get("models").and_then(Value::as_array) {
            for row in rows {
                if let Some(name) = row
                    .get("model")
                    .and_then(Value::as_str)
                    .or_else(|| row.get("name").and_then(Value::as_str))
                {
                    push(name);
                }
            }
        }
        return out;
    }

    if provider == "google" {
        if let Some(rows) = value.get("models").and_then(Value::as_array) {
            for row in rows {
                if let Some(name) = row
                    .get("name")
                    .and_then(Value::as_str)
                    .or_else(|| row.get("model").and_then(Value::as_str))
                {
                    push(name);
                }
            }
        }
        return out;
    }

    if let Some(rows) = value.get("data").and_then(Value::as_array) {
        for row in rows {
            if let Some(name) = row
                .get("id")
                .and_then(Value::as_str)
                .or_else(|| row.get("model").and_then(Value::as_str))
            {
                push(name);
            }
        }
    }
    out
}

fn parse_ollama_list_models(raw: &str) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for line in raw.lines() {
        let trimmed = clean_text(line, 320);
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.to_ascii_lowercase().starts_with("name ") {
            continue;
        }
        let first = clean_text(trimmed.split_whitespace().next().unwrap_or(""), 240);
        let model = model_ref_from_probe("ollama", &first);
        if model.is_empty() || out.iter().any(|row| row == &model) {
            continue;
        }
        out.push(model);
    }
    out
}

fn parse_ollama_list_models_json(raw: &str) -> Vec<String> {
    let mut out = Vec::<String>::new();
    if let Ok(value) = serde_json::from_str::<Value>(raw) {
        let rows = if let Some(array) = value.as_array() {
            array.clone()
        } else if let Some(array) = value.get("models").and_then(Value::as_array) {
            array.clone()
        } else {
            Vec::new()
        };
        for row in rows {
            if let Some(name) = row
                .get("model")
                .and_then(Value::as_str)
                .or_else(|| row.get("name").and_then(Value::as_str))
            {
                let cleaned = model_ref_from_probe("ollama", name);
                if !cleaned.is_empty() && !out.iter().any(|existing| existing == &cleaned) {
                    out.push(cleaned);
                }
            }
        }
        if !out.is_empty() {
            return out;
        }
    }
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
            if let Some(name) = value
                .get("model")
                .and_then(Value::as_str)
                .or_else(|| value.get("name").and_then(Value::as_str))
            {
                let cleaned = model_ref_from_probe("ollama", name);
                if !cleaned.is_empty() && !out.iter().any(|existing| existing == &cleaned) {
                    out.push(cleaned);
                }
            }
        }
    }
    out
}

fn canonical_ollama_base_url(raw: &str) -> String {
    let cleaned = clean_text(raw, 400);
    if cleaned.is_empty() {
        return String::new();
    }
    if cleaned.starts_with("http://") || cleaned.starts_with("https://") {
        return cleaned.trim_end_matches('/').to_string();
    }
    format!("http://{}", cleaned.trim_end_matches('/'))
}

fn ollama_base_url_candidates(base_url: &str) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let mut push = |raw: &str| {
        let candidate = canonical_ollama_base_url(raw);
        if candidate.is_empty() || out.iter().any(|existing| existing == &candidate) {
            return;
        }
        out.push(candidate);
    };
    push(base_url);
    if let Ok(env_host) = std::env::var("OLLAMA_HOST") {
        push(&env_host);
    }
    push("http://127.0.0.1:11434");
    push("http://localhost:11434");
    out
}

fn probe_ollama_runtime_online(base_url: &str) -> bool {
    let cleaned = canonical_ollama_base_url(base_url);
    if cleaned.is_empty() {
        return false;
    }
    for endpoint in ["api/tags", "api/version"] {
        if let Ok((status, _)) = curl_json(
            &format!("{}/{}", cleaned.trim_end_matches('/'), endpoint),
            "GET",
            &["Content-Type: application/json".to_string()],
            None,
            8,
        ) {
            if (200..300).contains(&status) {
                return true;
            }
        }
    }
    false
}

fn resolve_ollama_runtime_base_url(base_url: &str) -> String {
    for candidate in ollama_base_url_candidates(base_url) {
        if probe_ollama_runtime_online(&candidate) {
            return candidate;
        }
    }
    canonical_ollama_base_url(base_url)
}

fn probe_ollama_runtime_models(base_url: &str) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for candidate in ollama_base_url_candidates(base_url) {
        let tags_url = format!("{}/api/tags", candidate.trim_end_matches('/'));
        if let Ok((status, value)) = curl_json(
            &tags_url,
            "GET",
            &["Content-Type: application/json".to_string()],
            None,
            12,
        ) {
            if (200..300).contains(&status) {
                out = models_from_probe_response("ollama", &value);
                if !out.is_empty() {
                    return out;
                }
            }
        }
    }
    if !out.is_empty() {
        return out;
    }
    if !command_exists("ollama") {
        return out;
    }
    let cli_json_output = Command::new("ollama").arg("list").arg("--json").output();
    if let Ok(output) = cli_json_output {
        if output.status.success() {
            let parsed = parse_ollama_list_models_json(&String::from_utf8_lossy(&output.stdout));
            if !parsed.is_empty() {
                return parsed;
            }
        }
    }
    let cli_output = Command::new("ollama").arg("list").output();
    if let Ok(output) = cli_output {
        if output.status.success() {
            let parsed = parse_ollama_list_models(&String::from_utf8_lossy(&output.stdout));
            if !parsed.is_empty() {
                return parsed;
            }
        }
    }
    out
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
        let mut base_url = clean_text(
            row.get("base_url").and_then(Value::as_str).unwrap_or(""),
            400,
        );
        if provider_id == "ollama" {
            let resolved = resolve_ollama_runtime_base_url(&base_url);
            if !resolved.is_empty() {
                base_url = resolved;
                row["base_url"] = json!(base_url.clone());
            }
        }
        let key_present = provider_key(root, &provider_id).is_some();
        let mut local_reachable = if provider_is_local(&provider_id) {
            if provider_id == "ollama" {
                probe_ollama_runtime_online(&base_url)
            } else {
                local_provider_reachable(&provider_id, row)
            }
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
        if provider_id == "ollama" {
            let discovered_models = probe_ollama_runtime_models(&base_url);
            if !discovered_models.is_empty() {
                if !local_reachable {
                    // When CLI model discovery succeeds, treat the local runtime as reachable
                    // even if the initial HTTP probe path failed.
                    local_reachable = true;
                    row["reachable"] = json!(true);
                    row["auth_status"] = json!("configured");
                }
                if !row.get("model_profiles").map(Value::is_object).unwrap_or(false) {
                    row["model_profiles"] = json!({});
                }
                if let Some(profiles) = row.get_mut("model_profiles").and_then(Value::as_object_mut)
                {
                    for model in &discovered_models {
                        if profiles.contains_key(model) {
                            continue;
                        }
                        profiles.insert(
                            model.clone(),
                            inferred_model_profile("ollama", model, true),
                        );
                    }
                }
                row["detected_models"] = Value::Array(
                    discovered_models
                        .into_iter()
                        .map(Value::String)
                        .collect::<Vec<_>>(),
                );
                row["updated_at"] = json!(crate::now_iso());
            }
        }
        let mut profiles = row
            .get("model_profiles")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let profile_count_before = profiles.len();
        profiles.retain(|model_name, _| {
            let cleaned = clean_text(model_name, 240);
            !cleaned.is_empty() && !model_id_is_placeholder(&cleaned)
        });
        let profiles_sanitized = profiles.len() != profile_count_before;
        let profiles_missing = profiles.is_empty();
        if profiles_missing {
            profiles = model_profiles_for_provider(&provider_id);
        }
        let mut profiles_seeded = false;
        let provider_defaults = model_profiles_for_provider(&provider_id);
        if !provider_defaults.is_empty() {
            for (model, profile) in provider_defaults {
                if profiles.contains_key(&model) {
                    continue;
                }
                profiles.insert(model, profile);
                profiles_seeded = true;
            }
        }
        profiles.retain(|model_name, _| {
            let cleaned = clean_text(model_name, 240);
            !cleaned.is_empty() && !model_id_is_placeholder(&cleaned)
        });
        let profiles_enriched = enrich_model_profiles_for_provider(&provider_id, &mut profiles);
        if profiles_missing || profiles_enriched || profiles_sanitized || profiles_seeded {
            row["model_profiles"] = Value::Object(profiles);
        }
        let detected = row
            .get("model_profiles")
            .and_then(Value::as_object)
            .map(|obj| {
                obj.keys()
                    .filter_map(|model_name| {
                        let cleaned = clean_text(model_name, 240);
                        if cleaned.is_empty() || model_id_is_placeholder(&cleaned) {
                            None
                        } else {
                            Some(Value::String(cleaned))
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        row["detected_models"] = Value::Array(detected);
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
        "network_policy": provider_network_policy(root),
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

    if provider == "ollama" {
        let row = provider_row(root, &provider);
        let base_url = clean_text(
            row.get("base_url")
                .and_then(Value::as_str)
                .unwrap_or(&provider_base_url_default(&provider)),
            400,
        );
        let resolved_base = resolve_ollama_runtime_base_url(&base_url);
        let mut online = probe_ollama_runtime_online(&resolved_base);
        let discovered_models = probe_ollama_runtime_models(&resolved_base);
        if !online && !discovered_models.is_empty() {
            online = true;
        }
        let mut registry = load_registry(root);
        let row = ensure_provider_row_mut(&mut registry, &provider);
        row["base_url"] = json!(resolved_base.clone());
        row["reachable"] = json!(online);
        if !discovered_models.is_empty() {
            if !row.get("model_profiles").map(Value::is_object).unwrap_or(false) {
                row["model_profiles"] = json!({});
            }
            if let Some(profiles) = row.get_mut("model_profiles").and_then(Value::as_object_mut) {
                profiles.retain(|model_name, _| {
                    let cleaned = clean_text(model_name, 240);
                    !cleaned.is_empty() && !model_id_is_placeholder(&cleaned)
                });
                for model in &discovered_models {
                    if profiles.contains_key(model) {
                        continue;
                    }
                    profiles.insert(model.clone(), inferred_model_profile("ollama", model, true));
                }
            }
            row["detected_models"] = Value::Array(
                discovered_models
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect::<Vec<_>>(),
            );
        }
        row["updated_at"] = json!(crate::now_iso());
        save_registry(root, registry);
        return if online {
            json!({
                "ok": true,
                "status": "ok",
                "provider": provider,
                "latency_ms": started.elapsed().as_millis() as i64,
                "detail": {
                    "base_url": resolved_base,
                    "discovered_models": discovered_models,
                }
            })
        } else {
            json!({
                "ok": false,
                "status": "error",
                "provider": provider,
                "error": "ollama_runtime_unreachable",
                "detail": {"base_url": resolved_base}
            })
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
        "frontier_provider" => {
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
            let discovered_models = models_from_probe_response(&provider, &value);
            if !discovered_models.is_empty() {
                if !row.get("model_profiles").map(Value::is_object).unwrap_or(false) {
                    row["model_profiles"] = json!({});
                }
                if let Some(profiles) = row.get_mut("model_profiles").and_then(Value::as_object_mut) {
                    profiles.retain(|model_name, _| {
                        let cleaned = clean_text(model_name, 240);
                        !cleaned.is_empty() && !model_id_is_placeholder(&cleaned)
                    });
                    for model in &discovered_models {
                        if profiles.contains_key(model) {
                            continue;
                        }
                        profiles.insert(
                            model.clone(),
                            inferred_model_profile(&provider, model, provider_is_local(&provider)),
                        );
                    }
                }
                row["detected_models"] = Value::Array(
                    discovered_models
                        .iter()
                        .cloned()
                        .map(Value::String)
                        .collect::<Vec<_>>(),
                );
            }
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

#[cfg(test)]
mod provider_http_interop_tests {
    use super::*;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn extract_openai_text_preserves_multiline_list_layout() {
        let payload = json!({
            "choices": [{
                "message": {
                    "content": "1. First item\n2. Second item\n   - nested detail"
                }
            }]
        });
        let text = extract_openai_text(&payload);
        assert!(text.contains("1. First item\n2. Second item"));
        assert!(text.contains("\n   - nested detail"));
    }

    #[test]
    fn parse_ollama_list_models_reads_name_column() {
        let raw = "\
NAME                             ID              SIZE      MODIFIED
qwen3:8b                         500a1f067a9f    5.2 GB    7 weeks ago
kimi-k2.5:cloud                  6d1c3246c608    -         7 weeks ago
";
        let rows = parse_ollama_list_models(raw);
        assert_eq!(
            rows,
            vec!["qwen3:8b".to_string(), "kimi-k2.5:cloud".to_string()]
        );
    }

    #[test]
    fn parse_ollama_list_models_json_reads_array_rows() {
        let raw = r#"
[
  {"name":"qwen3:8b","model":"qwen3:8b"},
  {"name":"smallthinker:latest","model":"smallthinker:latest"}
]
"#;
        let rows = parse_ollama_list_models_json(raw);
        assert_eq!(
            rows,
            vec!["qwen3:8b".to_string(), "smallthinker:latest".to_string()]
        );
    }

    #[test]
    fn ollama_base_url_candidates_include_default_loopback() {
        let rows = ollama_base_url_candidates("127.0.0.1:11434");
        assert!(rows.iter().any(|row| row == "http://127.0.0.1:11434"));
        assert!(rows.iter().any(|row| row == "http://localhost:11434"));
    }

    #[test]
    fn provider_rows_marks_ollama_reachable_when_cli_lists_models() {
        let root = tempfile::tempdir().expect("tempdir");
        let bin_dir = tempfile::tempdir().expect("tempdir");
        let ollama_path = bin_dir.path().join("ollama");
        let script = r#"#!/bin/sh
if [ "$1" = "list" ] && [ "$2" = "--json" ]; then
  printf '[{"name":"qwen3:4b","model":"qwen3:4b"},{"name":"smallthinker:latest","model":"smallthinker:latest"}]\n'
  exit 0
fi
if [ "$1" = "list" ]; then
  printf 'NAME ID SIZE MODIFIED\nqwen3:4b deadbeef 3.2GB now\n'
  exit 0
fi
exit 1
"#;
        fs::write(&ollama_path, script).expect("write ollama stub");
        #[cfg(unix)]
        {
            fs::set_permissions(&ollama_path, fs::Permissions::from_mode(0o755))
                .expect("chmod ollama stub");
        }
        let old_path = std::env::var("PATH").unwrap_or_default();
        let new_path = format!("{}:{}", bin_dir.path().display(), old_path);
        std::env::set_var("PATH", new_path);

        let rows = provider_rows(root.path(), &json!({}));
        let ollama = rows
            .iter()
            .find(|row| row.get("id").and_then(Value::as_str) == Some("ollama"))
            .cloned()
            .unwrap_or_else(|| json!({}));

        std::env::set_var("PATH", old_path);

        assert_eq!(ollama.get("reachable").and_then(Value::as_bool), Some(true));
        assert!(ollama
            .get("detected_models")
            .and_then(Value::as_array)
            .map(|models| {
                models
                    .iter()
                    .any(|row| row.as_str() == Some("qwen3:4b"))
            })
            .unwrap_or(false));
    }
}
