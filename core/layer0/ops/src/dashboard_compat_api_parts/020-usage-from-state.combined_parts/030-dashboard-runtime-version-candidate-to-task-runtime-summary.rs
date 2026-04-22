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
    let hot_path_allocators = crate::protheus_ops_core_v1_bridge::hot_path_allocators::snapshot_json();
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
