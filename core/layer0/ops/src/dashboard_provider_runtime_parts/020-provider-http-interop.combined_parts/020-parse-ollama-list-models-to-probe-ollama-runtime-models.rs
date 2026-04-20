
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
