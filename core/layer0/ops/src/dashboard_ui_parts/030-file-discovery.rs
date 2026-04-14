
fn to_iso(ts: SystemTime) -> String {
    DateTime::<Utc>::from(ts).to_rfc3339()
}

fn file_rows(
    root: &Path,
    dir: &Path,
    max_depth: usize,
    limit: usize,
    include: &dyn Fn(&Path) -> bool,
) -> Vec<FileRow> {
    let mut rows = Vec::<FileRow>::new();
    for entry in WalkDir::new(dir)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if !include(path) {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let modified = meta.modified().unwrap_or(UNIX_EPOCH);
        let mtime_ms = modified
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let rel = path
            .strip_prefix(root)
            .ok()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        rows.push(FileRow {
            rel_path: rel,
            full_path: path.to_path_buf(),
            mtime_ms,
            mtime: to_iso(modified),
            size_bytes: meta.len(),
        });
    }
    rows.sort_by_key(|row| Reverse(row.mtime_ms));
    rows.truncate(limit);
    rows
}

fn read_tail_lines(path: &Path, max_lines: usize) -> Vec<String> {
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return Vec::new(),
    };

    let len = file.metadata().ok().map(|meta| meta.len()).unwrap_or(0);
    if len == 0 {
        return Vec::new();
    }

    let take = len.min(LOG_TAIL_MAX_READ_BYTES as u64);
    if len > take {
        let _ = file.seek(SeekFrom::End(-(take as i64)));
    }

    let mut buf = Vec::<u8>::with_capacity(take as usize);
    if file.read_to_end(&mut buf).is_err() {
        return Vec::new();
    }

    let mut raw = String::from_utf8_lossy(&buf).to_string();
    if len > take {
        if let Some((_, rest)) = raw.split_once('\n') {
            raw = rest.to_string();
        }
    }

    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .rev()
        .take(max_lines)
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn collect_log_events(root: &Path) -> Vec<Value> {
    let roots = [
        root.join("core/local/state/ops"),
        root.join("client/runtime/local/state"),
    ];
    let mut rows = Vec::<Value>::new();
    for base in roots {
        let files = file_rows(root, &base, 4, 8, &|path| {
            let rel = path.to_string_lossy();
            rel.ends_with(".jsonl")
        });
        for file in files {
            for line in read_tail_lines(&file.full_path, 8) {
                let payload = parse_json_loose(&line).unwrap_or(Value::Null);
                let ts = payload
                    .get("ts")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                    .unwrap_or_else(|| file.mtime.clone());
                let message = payload
                    .get("type")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                    .unwrap_or_else(|| clean_text(&line, 220));
                rows.push(json!({
                    "ts": ts,
                    "source": file.rel_path,
                    "message": message
                }));
            }
        }
    }
    rows.sort_by(|a, b| {
        b.get("ts")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(a.get("ts").and_then(Value::as_str).unwrap_or(""))
    });
    rows.truncate(40);
    rows
}

fn collect_receipts(root: &Path) -> Vec<Value> {
    let roots = [
        root.join("core/local/state/ops"),
        root.join("client/runtime/local/state"),
    ];
    let mut files = Vec::<FileRow>::new();
    for base in roots {
        files.extend(file_rows(root, &base, 4, 30, &|path| {
            let rel = path.to_string_lossy();
            rel.ends_with("latest.json")
                || rel.ends_with("history.jsonl")
                || rel.ends_with(".receipt.json")
        }));
    }
    files.sort_by_key(|row| Reverse(row.mtime_ms));
    files.truncate(32);
    files
        .into_iter()
        .map(|row| {
            json!({
                "kind": if row.rel_path.ends_with(".jsonl") { "timeline" } else { "receipt" },
                "path": row.rel_path,
                "mtime": row.mtime,
                "size_bytes": row.size_bytes
            })
        })
        .collect()
}

fn memory_artifact_source_fingerprint(root: &Path) -> String {
    let roots = [
        root.join("client/runtime/local/state"),
        root.join("core/local/state/ops"),
    ];
    let mut rows = Vec::<Value>::new();
    for base in roots {
        let meta = fs::metadata(&base).ok();
        rows.push(json!({
            "path": base.to_string_lossy().to_string(),
            "exists": meta.is_some(),
            "len": meta.as_ref().map(|m| m.len()).unwrap_or(0),
            "modified_ms": meta
                .and_then(|m| m.modified().ok())
                .and_then(|ts| ts.duration_since(UNIX_EPOCH).ok())
                .map(|dur| dur.as_millis() as i64)
                .unwrap_or(0)
        }));
    }
    crate::deterministic_receipt_hash(&Value::Array(rows))
}

fn collect_memory_artifacts_uncached(root: &Path) -> Vec<Value> {
    let roots = [
        root.join("client/runtime/local/state"),
        root.join("core/local/state/ops"),
    ];
    let mut rows = Vec::<Value>::new();
    for base in roots {
        for row in file_rows(root, &base, 3, 20, &|path| {
            let rel = path.to_string_lossy();
            rel.ends_with("latest.json") || rel.ends_with(".jsonl") || rel.ends_with("queue.json")
        }) {
            rows.push(json!({
                "scope": if row.rel_path.contains("memory") { "memory" } else { "state" },
                "kind": if row.rel_path.ends_with(".jsonl") { "timeline" } else { "snapshot" },
                "path": row.rel_path,
                "mtime": row.mtime
            }));
        }
    }
    rows.sort_by(|a, b| {
        b.get("mtime")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(a.get("mtime").and_then(Value::as_str).unwrap_or(""))
    });
    rows.truncate(30);
    rows
}

fn collect_memory_artifacts(root: &Path) -> Vec<Value> {
    static CACHE: OnceLock<Mutex<HashMap<String, Value>>> = OnceLock::new();
    let key = root.to_string_lossy().to_string();
    let fingerprint = memory_artifact_source_fingerprint(root);
    let now_ms = Utc::now().timestamp_millis();
    if let Ok(guard) = CACHE.get_or_init(|| Mutex::new(HashMap::new())).lock() {
        if let Some(row) = guard.get(&key) {
            let cached_fingerprint = clean_text(
                row.get("fingerprint").and_then(Value::as_str).unwrap_or(""),
                120,
            );
            let cached_ts = row.get("cached_at_ms").and_then(Value::as_i64).unwrap_or(0);
            let within_window = now_ms.saturating_sub(cached_ts) <= 3000;
            if cached_fingerprint == fingerprint && within_window {
                return row
                    .get("rows")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
            }
        }
    }
    let rows = collect_memory_artifacts_uncached(root);
    if let Ok(mut guard) = CACHE.get_or_init(|| Mutex::new(HashMap::new())).lock() {
        guard.insert(
            key,
            json!({
                "fingerprint": fingerprint,
                "cached_at_ms": now_ms,
                "rows": rows
            }),
        );
    }
    rows
}

const DASHBOARD_CHANNEL_REGISTRY_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/channel_registry.json";
const DASHBOARD_PROVIDER_REGISTRY_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/provider_registry.json";

fn increment_counter(map: &mut serde_json::Map<String, Value>, key: &str) {
    let normalized = clean_text(key, 80).to_ascii_lowercase();
    if normalized.is_empty() {
        return;
    }
    let next = map
        .get(&normalized)
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .saturating_add(1);
    map.insert(normalized, json!(next));
}

fn channel_connected(row: &Value) -> bool {
    if let Some(connected) = row.get("connected").and_then(Value::as_bool) {
        return connected;
    }
    let configured = row
        .get("configured")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let has_token = row
        .get("has_token")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let requires_token = row
        .get("requires_token")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let runtime_supported = row
        .get("runtime_supported")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let probe_required = row
        .get("live_probe_required_for_ready")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let config_ready = if requires_token {
        configured && has_token
    } else {
        configured
    };
    let live_ok = row
        .get("live_probe")
        .and_then(Value::as_object)
        .and_then(|probe| probe.get("status"))
        .and_then(Value::as_str)
        .map(|status| status.eq_ignore_ascii_case("ok"))
        .unwrap_or(false);
    runtime_supported && if probe_required { config_ready && live_ok } else { config_ready }
}

fn object_values(path: &Path, key: &str) -> Vec<Value> {
    read_json_file(path)
        .and_then(|value| value.get(key).and_then(Value::as_object).cloned())
        .map(|rows| rows.values().cloned().collect::<Vec<_>>())
        .unwrap_or_default()
}

fn collect_web_tooling_summary(root: &Path) -> Value {
    let channel_rows = object_values(&root.join(DASHBOARD_CHANNEL_REGISTRY_REL), "channels");
    let provider_rows = object_values(&root.join(DASHBOARD_PROVIDER_REGISTRY_REL), "providers");

    let mut channels_configured = 0i64;
    let mut channels_connected = 0i64;
    let mut transport_counts = serde_json::Map::<String, Value>::new();
    for row in &channel_rows {
        if row
            .get("configured")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            channels_configured += 1;
        }
        if channel_connected(row) {
            channels_connected += 1;
        }
        increment_counter(
            &mut transport_counts,
            row.get("transport_kind")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
        );
    }

    let mut providers_reachable = 0i64;
    let mut providers_auth_configured = 0i64;
    for row in &provider_rows {
        if row
            .get("reachable")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            providers_reachable += 1;
        }
        if matches!(
            clean_text(
                row.get("auth_status")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                32
            )
            .to_ascii_lowercase()
            .as_str(),
            "configured" | "set" | "ok"
        ) {
            providers_auth_configured += 1;
        }
    }

    let channel_total = channel_rows.len() as i64;
    let provider_total = provider_rows.len() as i64;
    let status = if channel_total == 0 && provider_total == 0 {
        "empty"
    } else if (channels_configured > 0 && channels_connected == 0)
        || (provider_total > 0 && providers_auth_configured == 0)
    {
        "degraded"
    } else {
        "ok"
    };

    json!({
        "status": status,
        "generated_at": now_iso(),
        "channels": {
            "total": channel_total,
            "configured": channels_configured,
            "connected": channels_connected,
            "transport_counts": transport_counts
        },
        "providers": {
            "total": provider_total,
            "reachable": providers_reachable,
            "auth_configured": providers_auth_configured
        }
    })
}

fn metric_rows(health: &Value) -> Vec<Value> {
    let Some(metrics) = health.get("dashboard_metrics").and_then(Value::as_object) else {
        return Vec::new();
    };
    metrics
        .iter()
        .map(|(name, row)| {
            let status = row
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let target = row
                .get("target_max")
                .map(|v| format!("<= {v}"))
                .or_else(|| row.get("target_min").map(|v| format!(">= {v}")))
                .unwrap_or_else(|| "n/a".to_string());
            json!({
                "name": name,
                "status": status,
                "value": row.get("value").cloned().unwrap_or(Value::Null),
                "target": target
            })
        })
        .collect()
}

fn i64_from_value(value: Option<&Value>, fallback: i64) -> i64 {
    let parsed = value
        .and_then(|row| {
            row.as_i64()
                .or_else(|| row.as_u64().and_then(|n| i64::try_from(n).ok()))
                .or_else(|| row.as_f64().map(|n| n.round() as i64))
                .or_else(|| row.as_str().and_then(|s| s.trim().parse::<i64>().ok()))
        })
        .unwrap_or(fallback);
    parsed.max(0)
}

fn recommended_conduit_signals(
    queue_depth: i64,
    queue_utilization: f64,
    active_conduit_channels: i64,
    active_agents: i64,
) -> i64 {
    let depth = queue_depth.max(0);
    let util = queue_utilization.clamp(0.0, 1.0);
    let mut baseline = 4;
    if depth >= 95 || util >= 0.90 {
        baseline = 16;
    } else if depth >= 85 || util >= 0.82 {
        baseline = 14;
    } else if depth >= 65 || util >= 0.68 {
        baseline = 12;
    } else if depth >= RUNTIME_SYNC_WARN_DEPTH || util >= 0.58 {
        baseline = 8;
    } else if depth >= RUNTIME_SYNC_SOFT_SCALE_DEPTH || util >= 0.40 {
        baseline = 6;
    }

    let channels = active_conduit_channels.max(0);
    let conduit_floor = if channels > 0 {
        let bonus = if depth >= RUNTIME_SYNC_DRAIN_TRIGGER_DEPTH || util >= 0.65 {
            2
        } else if depth >= RUNTIME_SYNC_SOFT_SCALE_DEPTH || util >= 0.40 {
            1
        } else {
            0
        };
        (channels + bonus).clamp(4, 16)
    } else {
        4
    };

    let agents = active_agents.max(0);
    let agent_scale = if depth >= RUNTIME_SYNC_DRAIN_TRIGGER_DEPTH || util >= 0.65 {
        40
    } else if depth >= RUNTIME_SYNC_SOFT_SCALE_DEPTH || util >= 0.40 {
        120
    } else {
        400
    };
    let agent_floor = if agents > 0 {
        (4 + ((agents + agent_scale - 1) / agent_scale)).clamp(4, 24)
    } else {
        4
    };

    baseline.max(conduit_floor).max(agent_floor)
}
