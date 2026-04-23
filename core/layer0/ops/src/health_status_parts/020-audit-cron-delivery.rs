fn audit_cron_delivery(root: &Path) -> Value {
    let cron_path = root.join(CRON_JOBS_REL);
    let parsed = match read_json(&cron_path) {
        Ok(v) => v,
        Err(err) => {
            return json!({
                "ok": false,
                "path": CRON_JOBS_REL,
                "error": err,
                "issues": [
                    {
                        "reason": "cron_jobs_unreadable"
                    }
                ]
            })
        }
    };

    let jobs = parsed
        .get("jobs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut enabled_jobs = 0usize;
    let mut isolated_jobs = 0usize;
    let mut jobs_with_delivery = 0usize;
    let mut issues = Vec::<Value>::new();
    let web_tooling_auth_sources = cron_runtime_web_tooling_auth_sources();
    let web_tooling_auth_present = !web_tooling_auth_sources.is_empty();
    let mut web_tooling_jobs = 0usize;

    for job in jobs {
        let name = job
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let id = job
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let enabled = job.get("enabled").and_then(Value::as_bool).unwrap_or(true);
        if !enabled {
            continue;
        }
        enabled_jobs += 1;

        let session_target = job
            .get("sessionTarget")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        if session_target == "isolated" {
            isolated_jobs += 1;
        }

        let delivery = job.get("delivery").and_then(Value::as_object);
        let payload_text = job
            .get("payload")
            .and_then(Value::as_object)
            .and_then(|payload| payload.get("text").or_else(|| payload.get("message")))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        let command_text = job
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        let name_lower = name.to_ascii_lowercase();
        let uses_web_tooling = name_lower.contains("web")
            || payload_text.contains("web_search")
            || payload_text.contains("web search")
            || payload_text.contains("web_fetch")
            || payload_text.contains("web fetch")
            || command_text.contains("web-conduit")
            || command_text.contains("web search")
            || command_text.contains("web fetch");
        if uses_web_tooling {
            web_tooling_jobs += 1;
            if !web_tooling_auth_present {
                issues.push(json!({
                    "id": id,
                    "name": name,
                    "reason": "web_tooling_auth_missing_for_web_job"
                }));
            }
        }
        if delivery.is_none() {
            issues.push(json!({
                "id": id,
                "name": name,
                "reason": "missing_delivery_for_enabled_job",
                "session_target": session_target
            }));
            continue;
        }

        jobs_with_delivery += 1;
        let delivery = delivery.expect("checked");
        let mode = delivery
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        let channel = delivery
            .get("channel")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();

        if mode.is_empty() {
            issues.push(json!({
                "id": id,
                "name": name,
                "reason": "missing_delivery_mode"
            }));
            continue;
        }

        if mode == "none" {
            issues.push(json!({
                "id": id,
                "name": name,
                "reason": "delivery_mode_none_forbidden",
                "mode": mode,
                "channel": channel
            }));
            continue;
        }

        if mode == "announce" {
            if channel.is_empty() {
                issues.push(json!({
                    "id": id,
                    "name": name,
                    "reason": "announce_missing_channel",
                    "mode": mode
                }));
                continue;
            }
            if !allowed_delivery_channel(&channel) {
                issues.push(json!({
                    "id": id,
                    "name": name,
                    "reason": "unsupported_delivery_channel",
                    "mode": mode,
                    "channel": channel,
                    "allowed_channels": ALLOWED_DELIVERY_CHANNELS
                }));
            }
        }

        if session_target == "isolated" && mode != "announce" {
            issues.push(json!({
                "id": id,
                "name": name,
                "reason": "isolated_requires_announce_delivery",
                "mode": mode,
                "channel": channel
            }));
        }
    }

    json!({
        "ok": issues.is_empty(),
        "path": CRON_JOBS_REL,
        "total_jobs": parsed.get("jobs").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0),
        "enabled_jobs": enabled_jobs,
        "isolated_jobs": isolated_jobs,
        "jobs_with_delivery": jobs_with_delivery,
        "web_tooling_jobs": web_tooling_jobs,
        "web_tooling_auth_present": web_tooling_auth_present,
        "web_tooling_auth_sources": web_tooling_auth_sources,
        "issues": issues
    })
}

fn cron_runtime_web_tooling_auth_sources() -> Vec<String> {
    let env_candidates = [
        "BRAVE_API_KEY",
        "EXA_API_KEY",
        "TAVILY_API_KEY",
        "PERPLEXITY_API_KEY",
        "SERPAPI_API_KEY",
        "GOOGLE_SEARCH_API_KEY",
        "GOOGLE_CSE_ID",
        "FIRECRAWL_API_KEY",
        "XAI_API_KEY",
        "MOONSHOT_API_KEY",
        "OPENAI_API_KEY",
    ];
    let mut sources = Vec::<String>::new();
    for env_name in env_candidates {
        let present = env::var(env_name)
            .ok()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);
        if present {
            sources.push(format!("env:{env_name}"));
        }
    }
    sources
}

fn percentile(values: &[f64], q: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let quantile = if q.is_finite() {
        q.clamp(0.0, 1.0)
    } else {
        0.5
    };
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((sorted.len() as f64) * quantile).ceil() as usize;
    let idx = idx.saturating_sub(1).min(sorted.len().saturating_sub(1));
    sorted.get(idx).copied()
}

fn percentile_95(values: &[f64]) -> Option<f64> {
    percentile(values, 0.95)
}

fn percentile_99(values: &[f64]) -> Option<f64> {
    percentile(values, 0.99)
}

fn parse_row_ts_ms(row: &Value) -> Option<i64> {
    row.get("ts")
        .and_then(Value::as_str)
        .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
        .map(|dt| dt.timestamp_millis())
}

fn parse_ts_ms(raw: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

fn age_seconds(now_ms: i64, ts_ms: Option<i64>) -> Option<i64> {
    ts_ms.map(|ts| ((now_ms - ts).max(0)) / 1000)
}

fn default_secrets_dir() -> PathBuf {
    if let Ok(raw) = env::var("SECRET_BROKER_SECRETS_DIR") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("infring")
        .join("secrets")
}

fn expand_provider_path(root: &Path, raw: &str) -> PathBuf {
    let expanded = raw
        .replace(
            "${DEFAULT_SECRETS_DIR}",
            &default_secrets_dir().to_string_lossy(),
        )
        .replace("${HOME}", &env::var("HOME").unwrap_or_default())
        .replace("${INFRING_WORKSPACE}", &root.to_string_lossy());
    let candidate = PathBuf::from(expanded.trim());
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn read_secret_value_from_json(path: &Path, field: &str) -> Option<String> {
    let value = read_json(path).ok()?;
    let token = value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("");
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

fn collect_moltbook_credentials_dashboard_metric(root: &Path) -> Value {
    let cron_path = root.join(CRON_JOBS_REL);
    let cron = read_json(&cron_path).ok();
    let mut monitored_jobs = Vec::<String>::new();
    if let Some(rows) = cron
        .as_ref()
        .and_then(|v| v.get("jobs"))
        .and_then(Value::as_array)
    {
        for row in rows {
            if row.get("enabled").and_then(Value::as_bool) == Some(false) {
                continue;
            }
            let name = row
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .trim()
                .to_string();
            let payload_text = row
                .get("payload")
                .and_then(Value::as_object)
                .and_then(|payload| payload.get("text").or_else(|| payload.get("message")))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            let is_moltbook = name.to_ascii_lowercase().contains("moltbook")
                || payload_text.contains("moltcheck")
                || payload_text.contains("moltbook");
            if is_moltbook {
                monitored_jobs.push(name);
            }
        }
    }

    let policy_path = root.join("client/runtime/config/secret_broker_policy.json");
    let policy = read_json(&policy_path).ok();
    let providers = policy
        .as_ref()
        .and_then(|v| v.pointer("/secrets/moltbook_api_key/providers"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut providers_checked = 0u64;
    let mut env_hits = 0u64;
    let mut json_hits = 0u64;
    let mut command_enabled = 0u64;
    let mut availability_sources = Vec::<String>::new();

    for provider in providers {
        let provider_type = provider
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        if provider.get("enabled").and_then(Value::as_bool) == Some(false) {
            continue;
        }
        providers_checked += 1;
        match provider_type.as_str() {
            "env" => {
                let env_name = provider
                    .get("env")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if env_name.is_empty() {
                    continue;
                }
                let present = env::var(&env_name)
                    .ok()
                    .map(|v| !v.trim().is_empty())
                    .unwrap_or(false);
                if present {
                    env_hits += 1;
                    availability_sources.push(format!("env:{env_name}"));
                }
            }
            "json_file" => {
                let field = provider
                    .get("field")
                    .and_then(Value::as_str)
                    .unwrap_or("api_key")
                    .trim();
                let paths = provider
                    .get("paths")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                for raw in paths {
                    let Some(raw_path) = raw.as_str() else {
                        continue;
                    };
                    let path = expand_provider_path(root, raw_path);
                    if read_secret_value_from_json(&path, field).is_some() {
                        json_hits += 1;
                        availability_sources.push(format!("file:{}", path.to_string_lossy()));
                        break;
                    }
                }
            }
            "command" => {
                if provider
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    command_enabled += 1;
                }
            }
            _ => {}
        }
    }

    let credentials_available = env_hits > 0 || json_hits > 0;
    let jobs_requiring_credentials = monitored_jobs.len() as u64;
    let suppression_recommended = jobs_requiring_credentials > 0 && !credentials_available;
    let status = if suppression_recommended {
        "warn"
    } else {
        "pass"
    };

    json!({
        "moltbook_credentials_surface": {
            "value": if credentials_available { 1.0 } else { 0.0 },
            "target_min": 1.0,
            "status": status,
            "credentials_available": credentials_available,
            "jobs_requiring_credentials": jobs_requiring_credentials,
            "monitored_jobs": monitored_jobs,
            "suppression_recommended": suppression_recommended,
            "providers_checked": providers_checked,
            "env_hits": env_hits,
            "json_file_hits": json_hits,
            "command_providers_enabled": command_enabled,
            "availability_sources": availability_sources,
            "source": format!("{CRON_JOBS_REL} + client/runtime/config/secret_broker_policy.json")
        }
    })
}
