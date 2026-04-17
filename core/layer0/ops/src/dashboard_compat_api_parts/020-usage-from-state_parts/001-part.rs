fn dashboard_runtime_version_candidate(root: &Path) -> Option<Value> {
    let path = root
        .join("client")
        .join("runtime")
        .join("config")
        .join("runtime_version.json");
    let payload = read_json(&path)?;
    let source = clean_text(
        payload
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("runtime_version_contract"),
        80,
    );
    dashboard_version_candidate(
        payload.get("version").and_then(Value::as_str).unwrap_or(""),
        payload.get("tag").and_then(Value::as_str).unwrap_or(""),
        if source.is_empty() {
            "runtime_version_contract"
        } else {
            &source
        },
    )
}

fn dashboard_package_version_candidate(root: &Path) -> Option<Value> {
    let payload = read_json(&root.join("package.json"))?;
    dashboard_version_candidate(
        payload.get("version").and_then(Value::as_str).unwrap_or(""),
        "",
        "package_json",
    )
}

fn dashboard_runtime_version_info(root: &Path) -> Value {
    let mut best = None;
    best = pick_dashboard_version_candidate(best, dashboard_runtime_version_candidate(root));
    best = pick_dashboard_version_candidate(best, dashboard_package_version_candidate(root));
    best = pick_dashboard_version_candidate(best, dashboard_installed_release_candidate(root));
    best = pick_dashboard_version_candidate(best, dashboard_git_latest_tag_candidate(root));
    best.unwrap_or_else(|| {
        json!({
            "version": "0.0.0",
            "tag": "v0.0.0",
            "source": "fallback_default"
        })
    })
}

fn status_payload_cache() -> &'static Mutex<Option<StatusPayloadCacheEntry>> {
    static STATUS_PAYLOAD_CACHE: OnceLock<Mutex<Option<StatusPayloadCacheEntry>>> = OnceLock::new();
    STATUS_PAYLOAD_CACHE.get_or_init(|| Mutex::new(None))
}

fn status_payload(root: &Path, snapshot: &Value, host_header: &str) -> Value {
    let cache_key = format!(
        "{}|{}|{}",
        clean_text(host_header, 200),
        clean_text(
            snapshot
                .get("receipt_hash")
                .and_then(Value::as_str)
                .unwrap_or(""),
            128
        ),
        parse_non_negative_i64(
            snapshot
                .pointer("/runtime_sync/uptime_seconds")
                .or_else(|| snapshot.pointer("/runtime_sync/uptime_sec")),
            0
        )
    );
    let now_ms = monotonic_now_ms();
    if let Ok(guard) = status_payload_cache().lock() {
        if let Some(entry) = guard.as_ref() {
            if entry.key == cache_key && now_ms.saturating_sub(entry.built_at_ms) <= 900 {
                return entry.payload.clone();
            }
        }
    }
    let usage = usage_from_state(root, snapshot);
    let runtime = runtime_sync_summary(snapshot);
    let continuity = continuity_pending_payload(root, snapshot);
    let memory_hygiene = memory_hygiene_payload(root, &continuity);
    let task_runtime = task_runtime_summary(root);
    let worker_runtime = worker_runtime_summary(root);
    let hot_path_allocators = protheus_ops_core_v1::hot_path_allocators::snapshot_json();
    let web_conduit = crate::web_conduit::api_status(root);
    let (default_provider, default_model) = effective_app_settings(root, snapshot);
    let version_info = dashboard_runtime_version_info(root);
    let version = version_info
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("0.0.0")
        .to_string();
    let version_tag = version_info
        .get("tag")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let version_source = version_info
        .get("source")
        .and_then(Value::as_str)
        .unwrap_or("fallback_default")
        .to_string();
    let listen = {
        let cleaned = clean_text(host_header, 200);
        if cleaned.is_empty() {
            "127.0.0.1:4173".to_string()
        } else {
            cleaned
        }
    };
    let uptime_seconds = parse_non_negative_i64(
        snapshot
            .pointer("/runtime_sync/uptime_seconds")
            .or_else(|| snapshot.pointer("/runtime_sync/uptime_sec")),
        0,
    );
    let agent_count = usage
        .get("agents")
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    let out = json!({
        "ok": true,
        "version": version,
        "version_tag": version_tag,
        "version_source": version_source,
        "agent_count": agent_count,
        "connected": true,
        "uptime_sec": uptime_seconds,
        "uptime_seconds": uptime_seconds,
        "ws": true,
        "default_provider": default_provider,
        "default_model": default_model,
        "git_branch": crate::dashboard_git_runtime::git_current_branch(root, "main"),
        "api_listen": listen,
        "listen": listen,
        "home_dir": root.to_string_lossy().to_string(),
        "workspace_dir": root.to_string_lossy().to_string(),
        "log_level": clean_text(
            &std::env::var("RUST_LOG")
                .or_else(|_| std::env::var("LOG_LEVEL"))
                .unwrap_or_else(|_| "info".to_string()),
            32,
        ),
        "network_enabled": true,
        "runtime_sync": runtime,
        "task_runtime": task_runtime,
        "worker_runtime": worker_runtime,
        "hot_path_allocators": hot_path_allocators,
        "web_conduit": {
            "enabled": web_conduit.get("enabled").cloned().unwrap_or_else(|| json!(false)),
            "receipts_total": web_conduit.get("receipts_total").cloned().unwrap_or_else(|| json!(0)),
            "recent_denied": web_conduit.get("recent_denied").cloned().unwrap_or_else(|| json!(0)),
            "last_receipt": web_conduit.get("last_receipt").cloned().unwrap_or(Value::Null)
        },
        "memory_hygiene": memory_hygiene,
        "continuity": {
            "pending_total": continuity.get("pending_total").cloned().unwrap_or_else(|| json!(0)),
            "tasks_pending": continuity.pointer("/tasks/pending").cloned().unwrap_or_else(|| json!(0)),
            "stale_sessions": continuity.pointer("/sessions/stale_48h_count").cloned().unwrap_or_else(|| json!(0)),
            "channel_attention": continuity.pointer("/channels/attention_needed_count").cloned().unwrap_or_else(|| json!(0))
        }
    });
    if let Ok(mut guard) = status_payload_cache().lock() {
        *guard = Some(StatusPayloadCacheEntry {
            key: cache_key,
            built_at_ms: now_ms,
            payload: out.clone(),
        });
    }
    out
}

fn task_runtime_summary(root: &Path) -> Value {
    let path = root.join("local/state/runtime/task_runtime/registry.json");
    let registry = read_json(&path).unwrap_or_else(|| json!({}));
    let tasks = registry
        .get("tasks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut queued = 0i64;
    let mut running = 0i64;
    let mut done = 0i64;
    let mut cancelled = 0i64;
    for row in tasks {
        let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 40)
            .to_ascii_lowercase();
        match status.as_str() {
            "queued" => queued += 1,
            "running" => running += 1,
            "done" => done += 1,
            "cancelled" => cancelled += 1,
            _ => {}
        }
    }
    json!({
        "queued": queued,
        "running": running,
        "done": done,
        "cancelled": cancelled,
        "pending": queued + running
    })
}

fn worker_runtime_summary(root: &Path) -> Value {
    let path = root.join("local/state/runtime/task_runtime/worker_state.json");
    let state = read_json(&path).unwrap_or_else(|| json!({}));
    let active_workers = state
        .get("active_workers")
        .and_then(Value::as_object)
        .map(|rows| rows.len())
        .unwrap_or(0) as i64;
    json!({
        "active_workers": active_workers,
        "total_hibernations": state.get("total_hibernations").and_then(Value::as_i64).unwrap_or(0).max(0),
        "last_hibernated": state.get("last_hibernated").cloned().unwrap_or(Value::Null),
        "last_event": state.get("last_event").cloned().unwrap_or(Value::Null),
        "updated_at_ms": state.get("updated_at_ms").cloned().unwrap_or(Value::Null)
    })
}

fn session_pending_rows(root: &Path, snapshot: &Value, max_rows: usize) -> Vec<Value> {
    let now = Utc::now();
    let mut rows = Vec::<Value>::new();
    for row in session_summary_rows(root, snapshot).into_iter() {
        let message_count = row
            .get("message_count")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .max(0);
        if message_count <= 0 {
            continue;
        }
        let agent_id = clean_text(
            row.get("agent_id").and_then(Value::as_str).unwrap_or(""),
            140,
        );
        if agent_id.is_empty() {
            continue;
        }
        let updated_at = clean_text(
            row.get("updated_at").and_then(Value::as_str).unwrap_or(""),
            80,
        );
        let age_hours = parse_rfc3339_utc(&updated_at)
            .map(|ts| {
                let delta = now.signed_duration_since(ts).num_minutes().max(0);
                delta as f64 / 60.0
            })
            .unwrap_or(0.0);
        rows.push(json!({
            "agent_id": agent_id,
            "active_session_id": clean_text(row.get("active_session_id").and_then(Value::as_str).unwrap_or(""), 120),
            "message_count": message_count,
            "updated_at": updated_at,
            "age_hours": (age_hours * 10.0).round() / 10.0,
            "stale_48h": age_hours >= 48.0
        }));
    }
    rows.sort_by(|a, b| {
        let left = a.get("age_hours").and_then(Value::as_f64).unwrap_or(0.0);
        let right = b.get("age_hours").and_then(Value::as_f64).unwrap_or(0.0);
        right
            .partial_cmp(&left)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rows.truncate(max_rows.clamp(1, 100));
    rows
}

fn agent_continuity_markers(root: &Path, snapshot: &Value, max_rows: usize) -> Vec<Value> {
    let roster = build_agent_roster(root, snapshot, false);
    let mut rows = Vec::<Value>::new();
    for profile in roster {
        let agent_id = clean_agent_id(
            profile
                .get("agent_id")
                .or_else(|| profile.get("id"))
                .and_then(Value::as_str)
                .unwrap_or(""),
        );
        if agent_id.is_empty() {
            continue;
        }
        let state = load_session_state(root, &agent_id);
        let messages = session_messages(&state);
        let mut latest_user_text = String::new();
        let mut latest_user_ts = String::new();
        let mut latest_agent_ts = String::new();
        for row in messages.iter().rev() {
            let role = clean_text(row.get("role").and_then(Value::as_str).unwrap_or(""), 24)
                .to_ascii_lowercase();
            if role == "user" && latest_user_text.is_empty() {
                latest_user_text = clean_text(&message_text(row), 180);
                latest_user_ts = message_timestamp_iso(row);
            }
            if (role == "assistant" || role == "agent") && latest_agent_ts.is_empty() {
                latest_agent_ts = message_timestamp_iso(row);
            }
            if !latest_user_text.is_empty() && !latest_agent_ts.is_empty() {
                break;
            }
        }
        let objective = if latest_user_text.is_empty() {
            "No active objective.".to_string()
        } else {
            latest_user_text.clone()
        };
        let completion_percent = if latest_user_text.is_empty() {
            100
        } else if !latest_agent_ts.is_empty()
            && !latest_user_ts.is_empty()
            && latest_agent_ts >= latest_user_ts
        {
            100
        } else if !latest_agent_ts.is_empty() {
            60
        } else {
            20
        };
        rows.push(json!({
            "agent_id": agent_id,
            "name": clean_text(profile.get("name").and_then(Value::as_str).unwrap_or("Agent"), 120),
            "state": clean_text(profile.get("state").and_then(Value::as_str).unwrap_or("Idle"), 40),
            "objective": objective,
            "completion_percent": completion_percent,
            "updated_at": clean_text(profile.get("updated_at").and_then(Value::as_str).unwrap_or(""), 80)
        }));
    }
    rows.sort_by(|a, b| {
        clean_text(
            b.get("updated_at").and_then(Value::as_str).unwrap_or(""),
            80,
        )
        .cmp(&clean_text(
            a.get("updated_at").and_then(Value::as_str).unwrap_or(""),
            80,
        ))
    });
    rows.truncate(max_rows.clamp(1, 24));
    rows
}

fn continuity_pending_payload(root: &Path, snapshot: &Value) -> Value {
    let tasks = task_runtime_summary(root);
    let workers = worker_runtime_summary(root);
    let sessions = session_pending_rows(root, snapshot, 24);
    let continuity_agents = agent_continuity_markers(root, snapshot, 12);
    let stale_sessions = sessions
        .iter()
        .filter(|row| {
            row.get("stale_48h")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();
    let channel_rows = dashboard_compat_api_channels::channels_payload(root)
        .get("channels")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let channel_attention = channel_rows
        .into_iter()
        .filter(|row| {
            let configured = row.get("configured").and_then(Value::as_bool).unwrap_or(false);
            let connected = row.get("connected").and_then(Value::as_bool).unwrap_or(false);
            configured && !connected
        })
        .map(|row| {
            json!({
                "name": clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 80),
                "provider": clean_text(row.get("provider").and_then(Value::as_str).unwrap_or(""), 80),
                "status": clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 40)
            })
        })
        .collect::<Vec<_>>();

    let pending_total = tasks
        .get("pending")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0)
        + stale_sessions.len() as i64
        + channel_attention.len() as i64;
    json!({
        "ok": true,
        "type": "cross_channel_project_continuity",
        "pending_total": pending_total,
        "tasks": tasks,
        "workers": workers,
        "sessions": {
            "rows": sessions,
            "stale_48h_count": stale_sessions.len(),
            "stale_48h": stale_sessions
        },
        "active_agents": {
            "count": continuity_agents.len(),
            "rows": continuity_agents
        },
        "channels": {
            "attention_needed_count": channel_attention.len(),
            "attention_needed": channel_attention
        }
    })
}

const SNAPSHOT_HISTORY_SOFT_CAP_BYTES: i64 = 100 * 1024 * 1024;

fn memory_hygiene_payload(root: &Path, continuity: &Value) -> Value {
    let stale_48h_count = continuity
        .pointer("/sessions/stale_48h_count")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let stale_7d_count = continuity
