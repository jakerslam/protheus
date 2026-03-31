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
