fn normalize_job(job: &Value) -> Value {
    let now = crate::now_iso();
    let id = {
        let raw = clean_id(job.get("id").and_then(Value::as_str).unwrap_or(""), 120);
        if raw.is_empty() {
            make_id(
                "cron",
                &json!({"name": job.get("name").cloned().unwrap_or(Value::Null), "ts": now}),
            )
        } else {
            raw
        }
    };
    let schedule = normalize_schedule(job.get("schedule").unwrap_or(&json!({})));
    let name = clean_text(
        job.get("name")
            .and_then(Value::as_str)
            .unwrap_or("scheduled-job"),
        180,
    );
    let agent_id = clean_text(
        job.get("agent_id").and_then(Value::as_str).unwrap_or(""),
        140,
    );
    let action_message = clean_text(
        job.pointer("/action/message")
            .and_then(Value::as_str)
            .unwrap_or("Scheduled task execution."),
        2000,
    );
    let enabled = as_bool(job.get("enabled"), true);
    let created_at = clean_text(
        job.get("created_at").and_then(Value::as_str).unwrap_or(""),
        80,
    );
    let updated_at = clean_text(
        job.get("updated_at").and_then(Value::as_str).unwrap_or(""),
        80,
    );
    let last_run = job.get("last_run").cloned().unwrap_or(Value::Null);
    let next_run = if enabled {
        job.get("next_run")
            .cloned()
            .filter(|v| !v.is_null())
            .unwrap_or_else(|| schedule_next_run(&schedule))
    } else {
        Value::Null
    };
    json!({
        "id": id,
        "name": if name.is_empty() {
            "scheduled-job".to_string()
        } else {
            name.clone()
        },
        "agent_id": agent_id,
        "enabled": enabled,
        "schedule": schedule,
        "action": {
            "kind": clean_text(job.pointer("/action/kind").and_then(Value::as_str).unwrap_or("agent_turn"), 40),
            "message": action_message
        },
        "delivery": {
            "kind": clean_text(job.pointer("/delivery/kind").and_then(Value::as_str).unwrap_or("last_channel"), 40)
        },
        "run_count": as_i64(job.get("run_count"), 0).max(0),
        "last_run": last_run,
        "next_run": next_run,
        "created_at": if created_at.is_empty() { &now } else { &created_at },
        "updated_at": if updated_at.is_empty() { &now } else { &updated_at }
    })
}

fn load_jobs(root: &Path) -> Vec<Value> {
    let raw = read_json(&state_path(root, CRON_JOBS_REL)).unwrap_or_else(|| json!({"jobs": []}));
    let mut rows = array_from_value(&raw, "jobs")
        .iter()
        .map(normalize_job)
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| {
        std::cmp::Reverse(clean_text(
            row.get("updated_at").and_then(Value::as_str).unwrap_or(""),
            80,
        ))
    });
    rows
}

fn save_jobs(root: &Path, rows: &[Value]) {
    write_json(
        &state_path(root, CRON_JOBS_REL),
        &json!({
            "type": "infring_dashboard_cron_jobs",
            "updated_at": crate::now_iso(),
            "jobs": rows
        }),
    );
}

fn normalize_trigger(trigger: &Value) -> Value {
    let now = crate::now_iso();
    let id = {
        let raw = clean_id(trigger.get("id").and_then(Value::as_str).unwrap_or(""), 120);
        if raw.is_empty() {
            make_id(
                "trigger",
                &json!({"agent_id": trigger.get("agent_id").cloned().unwrap_or(Value::Null), "ts": now}),
            )
        } else {
            raw
        }
    };
    json!({
        "id": id,
        "agent_id": clean_text(trigger.get("agent_id").and_then(Value::as_str).unwrap_or(""), 140),
        "pattern": trigger.get("pattern").cloned().unwrap_or_else(|| json!({"all": true})),
        "prompt_template": clean_text(trigger.get("prompt_template").and_then(Value::as_str).unwrap_or(""), 2000),
        "enabled": as_bool(trigger.get("enabled"), true),
        "fire_count": as_i64(trigger.get("fire_count"), 0).max(0),
        "max_fires": as_i64(trigger.get("max_fires"), 0).max(0),
        "created_at": clean_text(trigger.get("created_at").and_then(Value::as_str).unwrap_or(&now), 80),
        "updated_at": clean_text(trigger.get("updated_at").and_then(Value::as_str).unwrap_or(&now), 80)
    })
}

fn load_triggers(root: &Path) -> Vec<Value> {
    let raw = read_json(&state_path(root, TRIGGERS_REL)).unwrap_or_else(|| json!([]));
    let mut rows = array_from_value(&raw, "triggers")
        .iter()
        .map(normalize_trigger)
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| {
        std::cmp::Reverse(clean_text(
            row.get("updated_at").and_then(Value::as_str).unwrap_or(""),
            80,
        ))
    });
    rows
}

fn save_triggers(root: &Path, rows: &[Value]) {
    write_json(
        &state_path(root, TRIGGERS_REL),
        &json!({
            "type": "infring_dashboard_triggers",
            "updated_at": crate::now_iso(),
            "triggers": rows
        }),
    );
}

fn normalize_approval(row: &Value) -> Value {
    let now = crate::now_iso();
    let id = {
        let raw = clean_id(row.get("id").and_then(Value::as_str).unwrap_or(""), 120);
        if raw.is_empty() {
            make_id("approval", &json!({"created_at": now}))
        } else {
            raw
        }
    };
    json!({
        "id": id,
        "action": clean_text(row.get("action").and_then(Value::as_str).unwrap_or("Sensitive action"), 180),
        "description": clean_text(row.get("description").and_then(Value::as_str).unwrap_or("Approval required before continuing."), 400),
        "agent_name": clean_text(row.get("agent_name").and_then(Value::as_str).unwrap_or("runtime"), 120),
        "status": clean_text(row.get("status").and_then(Value::as_str).unwrap_or("pending"), 40).to_ascii_lowercase(),
        "created_at": clean_text(row.get("created_at").and_then(Value::as_str).unwrap_or(&now), 80),
        "updated_at": clean_text(row.get("updated_at").and_then(Value::as_str).unwrap_or(&now), 80)
    })
}

fn load_approvals(root: &Path) -> Vec<Value> {
    let raw =
        read_json(&state_path(root, APPROVALS_REL)).unwrap_or_else(|| json!({"approvals": []}));
    let mut rows = array_from_value(&raw, "approvals")
        .iter()
        .map(normalize_approval)
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| {
        std::cmp::Reverse(clean_text(
            row.get("created_at").and_then(Value::as_str).unwrap_or(""),
            80,
        ))
    });
    rows
}

fn save_approvals(root: &Path, rows: &[Value]) {
    write_json(
        &state_path(root, APPROVALS_REL),
        &json!({
            "type": "infring_dashboard_approvals",
            "updated_at": crate::now_iso(),
            "approvals": rows
        }),
    );
}

fn eyes_store_path(root: &Path) -> PathBuf {
    for rel in EYES_CATALOG_STATE_PATHS {
        let path = state_path(root, rel);
        if path.exists() {
            return path;
        }
    }
    state_path(root, EYES_CATALOG_STATE_PATHS[0])
}

fn normalize_eye(eye: &Value) -> Value {
    let now = crate::now_iso();
    let mut name = clean_text(eye.get("name").and_then(Value::as_str).unwrap_or(""), 120);
    let endpoint_url = clean_text(
        eye.get("endpoint_url")
            .and_then(Value::as_str)
            .or_else(|| eye.get("url").and_then(Value::as_str))
            .unwrap_or(""),
        500,
    );
    if name.is_empty() && !endpoint_url.is_empty() {
        name = host_from_url(&endpoint_url);
    }
    if name.is_empty() {
        name = "eye".to_string();
    }
    let id = {
        let raw = clean_id(
            eye.get("id")
                .and_then(Value::as_str)
                .unwrap_or(&name.to_ascii_lowercase()),
            120,
        );
        if raw.is_empty() {
            make_id("eye", &json!({"name": name, "url": endpoint_url}))
        } else {
            raw
        }
    };
    let status = {
        let raw = clean_text(
            eye.get("status")
                .and_then(Value::as_str)
                .unwrap_or("active"),
            24,
        )
        .to_ascii_lowercase();
        match raw.as_str() {
            "active" | "paused" | "dormant" | "disabled" => raw,
            _ => "active".to_string(),
        }
    };
    let mut topics = Vec::<String>::new();
    if let Some(rows) = eye.get("topics").and_then(Value::as_array) {
        for row in rows {
            let topic = clean_text(row.as_str().unwrap_or(""), 80);
            if !topic.is_empty() {
                topics.push(topic);
            }
        }
    } else if let Some(raw) = eye.get("topics").and_then(Value::as_str) {
        for part in raw.split(',') {
            let topic = clean_text(part, 80);
            if !topic.is_empty() {
                topics.push(topic);
            }
        }
    }
    json!({
        "uid": clean_text(eye.get("uid").and_then(Value::as_str).unwrap_or(&id), 160),
        "id": id,
        "name": name,
        "status": status,
        "endpoint_url": endpoint_url,
        "endpoint_host": clean_text(
            eye.get("endpoint_host")
                .and_then(Value::as_str)
                .unwrap_or(&host_from_url(
                    eye.get("endpoint_url")
                        .and_then(Value::as_str)
                        .or_else(|| eye.get("url").and_then(Value::as_str))
                        .unwrap_or("")
                )),
            120
        ),
        "api_key_present": as_bool(eye.get("api_key_present"), eye.get("api_key_hash").is_some()),
        "api_key_hash": clean_text(eye.get("api_key_hash").and_then(Value::as_str).unwrap_or(""), 160),
        "cadence_hours": as_i64(eye.get("cadence_hours"), 4).clamp(1, 168),
        "topics": topics,
        "updated_ts": clean_text(
            eye.get("updated_ts")
                .and_then(Value::as_str)
                .or_else(|| eye.get("updated_at").and_then(Value::as_str))
                .unwrap_or(&now),
            80
        ),
        "source": clean_text(eye.get("source").and_then(Value::as_str).unwrap_or("system"), 40)
    })
}

fn load_eyes(root: &Path) -> Vec<Value> {
    let raw = read_json(&eyes_store_path(root)).unwrap_or_else(|| json!({"eyes": []}));
    let mut rows =
        if let Some(catalog_rows) = raw.pointer("/catalog/eyes").and_then(Value::as_array) {
            catalog_rows.clone()
        } else {
            array_from_value(&raw, "eyes")
        };
    if rows.is_empty() && raw.is_array() {
        rows = raw.as_array().cloned().unwrap_or_default();
    }
    let mut normalized = rows.iter().map(normalize_eye).collect::<Vec<_>>();
    normalized.sort_by(|a, b| {
        clean_text(a.get("name").and_then(Value::as_str).unwrap_or(""), 120).cmp(&clean_text(
            b.get("name").and_then(Value::as_str).unwrap_or(""),
            120,
        ))
    });
    normalized
}

fn save_eyes(root: &Path, eyes: &[Value]) {
    write_json(
        &eyes_store_path(root),
        &json!({
            "type": "eyes_catalog",
            "updated_at": crate::now_iso(),
            "eyes": eyes
        }),
    );
}

fn workflow_path_segments(path_only: &str) -> Option<Vec<String>> {
    if path_only == "/api/workflows" {
        return Some(Vec::new());
    }
    if let Some(rest) = path_only.strip_prefix("/api/workflows/") {
        let segs = rest
            .split('/')
            .filter_map(|v| {
                let cleaned = clean_text(v, 200);
                if cleaned.is_empty() {
                    None
                } else {
                    Some(cleaned)
                }
            })
            .collect::<Vec<_>>();
        return Some(segs);
    }
    None
}

const WEB_TOOLING_PROFILE_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/web_tooling_profile.json";

fn normalize_web_provider_id(raw: &str) -> String {
    clean_text(raw, 80)
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn normalize_domain_value(raw: &str) -> String {
    let lowered = clean_text(raw, 200).to_ascii_lowercase();
    if lowered.is_empty() {
        return String::new();
    }
    let without_scheme = lowered
        .strip_prefix("https://")
        .or_else(|| lowered.strip_prefix("http://"))
        .unwrap_or(&lowered)
        .to_string();
    clean_text(without_scheme.split('/').next().unwrap_or(""), 140)
        .trim_matches('.')
        .to_string()
}

fn normalized_string_list(
    value: Option<&Value>,
    max_items: usize,
    max_len: usize,
    id_mode: bool,
) -> Vec<String> {
    let mut set = BTreeSet::<String>::new();
    let mut ingest = |raw: &str| {
        let mut item = if id_mode {
            normalize_web_provider_id(raw)
        } else {
            clean_text(raw, max_len)
        };
        if item.is_empty() {
            return;
        }
        if !id_mode {
            item = item.to_ascii_lowercase();
        }
        if set.len() < max_items {
            set.insert(item);
        }
    };
    if let Some(rows) = value.and_then(Value::as_array) {
        for row in rows {
            ingest(row.as_str().unwrap_or(""));
        }
    } else if let Some(raw) = value.and_then(Value::as_str) {
        for part in raw.split(',') {
            ingest(part);
        }
    }
    set.into_iter().collect()
}

fn default_web_tooling_profile() -> Value {
    json!({
        "type": "infring_dashboard_web_tooling_profile",
        "version": "v1",
        "provider_order": ["auto"],
        "query_policy": {
            "mode": "balanced",
            "max_queries": 4,
            "prefer_official_docs": true
        },
        "allowed_domains": [],
        "blocked_terms": [],
        "updated_at": crate::now_iso()
    })
}

fn web_tooling_profile_path(root: &Path) -> PathBuf {
    state_path(root, WEB_TOOLING_PROFILE_REL)
}

fn normalize_web_tooling_profile(profile: &Value) -> Value {
    let mut provider_order = normalized_string_list(profile.get("provider_order"), 8, 80, true);
    if provider_order.is_empty() {
        provider_order.push("auto".to_string());
    }
    let mut allowed_domains = normalized_string_list(profile.get("allowed_domains"), 24, 140, false)
        .into_iter()
        .map(|domain| normalize_domain_value(&domain))
        .filter(|domain| !domain.is_empty())
        .collect::<Vec<_>>();
    allowed_domains.sort();
    allowed_domains.dedup();

    let blocked_terms = normalized_string_list(profile.get("blocked_terms"), 32, 100, false);
    let mode = clean_text(
        profile
            .pointer("/query_policy/mode")
            .and_then(Value::as_str)
            .unwrap_or("balanced"),
        40,
    )
    .to_ascii_lowercase();
    let normalized_mode = match mode.as_str() {
        "domain_first" | "narrow_first" | "balanced" => mode,
        _ => "balanced".to_string(),
    };
    let max_queries = as_i64(profile.pointer("/query_policy/max_queries"), 4).clamp(1, 8);
    let prefer_official_docs =
        as_bool(profile.pointer("/query_policy/prefer_official_docs"), true);

    json!({
        "type": "infring_dashboard_web_tooling_profile",
        "version": "v1",
        "provider_order": provider_order,
        "query_policy": {
            "mode": normalized_mode,
            "max_queries": max_queries,
            "prefer_official_docs": prefer_official_docs
        },
        "allowed_domains": allowed_domains,
        "blocked_terms": blocked_terms,
        "updated_at": crate::now_iso()
    })
}

pub(crate) fn load_web_tooling_profile(root: &Path) -> Value {
    let raw = read_json(&web_tooling_profile_path(root)).unwrap_or_else(default_web_tooling_profile);
    normalize_web_tooling_profile(&raw)
}

pub(crate) fn save_web_tooling_profile(root: &Path, profile: &Value) {
    write_json(
        &web_tooling_profile_path(root),
        &normalize_web_tooling_profile(profile),
    );
}

pub(crate) fn merge_web_tooling_profile(existing: &Value, patch: &Value) -> Value {
    let merged = json!({
        "provider_order": patch.get("provider_order").cloned().unwrap_or_else(|| existing.get("provider_order").cloned().unwrap_or(Value::Null)),
        "allowed_domains": patch.get("allowed_domains").cloned().unwrap_or_else(|| existing.get("allowed_domains").cloned().unwrap_or(Value::Null)),
        "blocked_terms": patch.get("blocked_terms").cloned().unwrap_or_else(|| existing.get("blocked_terms").cloned().unwrap_or(Value::Null)),
        "query_policy": {
            "mode": patch.pointer("/query_policy/mode").cloned().unwrap_or_else(|| existing.pointer("/query_policy/mode").cloned().unwrap_or_else(|| json!("balanced"))),
            "max_queries": patch.pointer("/query_policy/max_queries").cloned().unwrap_or_else(|| existing.pointer("/query_policy/max_queries").cloned().unwrap_or_else(|| json!(4))),
            "prefer_official_docs": patch.pointer("/query_policy/prefer_official_docs").cloned().unwrap_or_else(|| existing.pointer("/query_policy/prefer_official_docs").cloned().unwrap_or_else(|| json!(true)))
        }
    });
    normalize_web_tooling_profile(&merged)
}

pub(crate) fn preferred_web_tooling_provider(profile: &Value) -> String {
    profile
        .get("provider_order")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(Value::as_str)
        .map(|raw| normalize_web_provider_id(raw))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "auto".to_string())
}

pub(crate) fn web_tooling_auth_presence(profile: &Value) -> Value {
    let mut sources = BTreeSet::<String>::new();
    for (label, env_var) in [
        ("openai", "OPENAI_API_KEY"),
        ("github", "GITHUB_TOKEN"),
        ("github_app", "GITHUB_APP_INSTALLATION_TOKEN"),
        ("brave", "BRAVE_API_KEY"),
        ("tavily", "TAVILY_API_KEY"),
        ("perplexity", "PERPLEXITY_API_KEY"),
        ("exa", "EXA_API_KEY"),
    ] {
        if !clean_text(&std::env::var(env_var).unwrap_or_default(), 4000).is_empty() {
            sources.insert(label.to_string());
        }
    }
    if as_bool(profile.pointer("/auth/token_present"), false) {
        sources.insert("profile_token".to_string());
    }
    let source_rows = sources.into_iter().collect::<Vec<_>>();
    json!({
        "any_present": !source_rows.is_empty(),
        "sources": source_rows
    })
}
