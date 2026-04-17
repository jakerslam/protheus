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
