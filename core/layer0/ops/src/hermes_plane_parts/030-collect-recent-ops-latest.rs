fn collect_recent_ops_latest(root: &Path, max_blocks: usize) -> Vec<Value> {
    let ops_root = root.join("core").join("local").join("state").join("ops");
    let mut rows = Vec::<Value>::new();
    if !ops_root.exists() {
        return rows;
    }
    let Ok(entries) = fs::read_dir(&ops_root) else {
        return rows;
    };
    for entry in entries.flatten() {
        let lane = entry.file_name().to_string_lossy().to_string();
        let latest = entry.path().join("latest.json");
        if !latest.exists() {
            continue;
        }
        let latest_mtime_ms = file_mtime_epoch_ms(&latest);
        let conduit_history = entry.path().join("conduit").join("history.jsonl");
        let conduit_history_mtime_ms = if conduit_history.exists() {
            file_mtime_epoch_ms(&conduit_history)
        } else {
            0
        };
        if let Some(payload) = read_json(&latest) {
            let ok = payload
                .get("ok")
                .and_then(Value::as_bool)
                .unwrap_or_else(|| {
                    let status = payload
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .trim()
                        .to_ascii_lowercase();
                    matches!(
                        status.as_str(),
                        "ok" | "pass" | "healthy" | "running" | "active" | "success"
                    )
                });
            let ty = clean(
                payload
                    .get("type")
                    .and_then(Value::as_str)
                    .or_else(|| payload.get("event_type").and_then(Value::as_str))
                    .unwrap_or("unknown"),
                120,
            );
            let ts = clean(
                payload
                    .get("ts")
                    .and_then(Value::as_str)
                    .or_else(|| payload.get("generated_at").and_then(Value::as_str))
                    .unwrap_or(""),
                80,
            );
            let tool_class = classify_tool_call(&ty).to_string();
            let provider_runtime_posture = provider_runtime_contract_posture(&payload);
            rows.push(json!({
                "lane": lane,
                "type": ty,
                "class": tool_class,
                "ok": ok,
                "ts": ts,
                "latest_path": latest.display().to_string(),
                "latest_mtime_ms": latest_mtime_ms,
                "has_conduit_history": conduit_history.exists(),
                "conduit_history_mtime_ms": conduit_history_mtime_ms,
                "provider_runtime_posture": provider_runtime_posture,
                "payload": payload
            }));
        }
    }
    rows.sort_by(|a, b| {
        let left_mtime = a
            .get("latest_mtime_ms")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let right_mtime = b
            .get("latest_mtime_ms")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        right_mtime.cmp(&left_mtime).then_with(|| {
            let left_lane = a.get("lane").and_then(Value::as_str).unwrap_or_default();
            let right_lane = b.get("lane").and_then(Value::as_str).unwrap_or_default();
            left_lane.cmp(right_lane)
        })
    });
    rows.truncate(max_blocks);
    rows
}

fn default_reclaim_protected_lanes() -> BTreeSet<String> {
    [
        "app_plane",
        "skills_plane",
        "collab_plane",
        "hermes_plane",
        "security_plane",
        "attention_queue",
        "dashboard_ui",
    ]
    .iter()
    .map(|value| value.to_string())
    .collect()
}

fn reclaim_protected_lanes(contract: &Value) -> BTreeSet<String> {
    let mut out = default_reclaim_protected_lanes();
    if let Some(rows) = contract
        .get("reclaim_protected_lanes")
        .and_then(Value::as_array)
    {
        for row in rows {
            let lane = clean(row.as_str().unwrap_or_default(), 80).to_ascii_lowercase();
            if !lane.is_empty() {
                out.insert(lane);
            }
        }
    }
    out
}

fn reclaim_stale_latest(
    root: &Path,
    stale_threshold_ms: u64,
    max_reclaims: usize,
    protected_lanes: &BTreeSet<String>,
    dry_run: bool,
) -> Value {
    let ops_root = root.join("core").join("local").join("state").join("ops");
    if !ops_root.exists() {
        return json!({
            "ok": true,
            "type": "hermes_plane_reclaim_stale",
            "dry_run": dry_run,
            "stale_threshold_ms": stale_threshold_ms,
            "scanned_lanes": 0,
            "candidate_count": 0,
            "reclaimed_count": 0,
            "skipped_protected_count": 0,
            "rows": [],
            "errors": []
        });
    }
    let mut scanned_lanes: usize = 0;
    let mut skipped_protected_count: usize = 0;
    let mut candidates = Vec::<(u64, String, String, PathBuf, String)>::new();
    let mut errors = Vec::<String>::new();
    let entries = match fs::read_dir(&ops_root) {
        Ok(rows) => rows,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "hermes_plane_reclaim_stale",
                "dry_run": dry_run,
                "stale_threshold_ms": stale_threshold_ms,
                "scanned_lanes": 0,
                "candidate_count": 0,
                "reclaimed_count": 0,
                "skipped_protected_count": 0,
                "rows": [],
                "errors": [clean(&format!("read_dir_failed:{err}"), 320)]
            });
        }
    };
    for entry in entries.flatten() {
        let lane = entry.file_name().to_string_lossy().to_ascii_lowercase();
        let latest_path = entry.path().join("latest.json");
        if !latest_path.exists() {
            continue;
        }
        scanned_lanes += 1;
        if protected_lanes.contains(&lane) {
            skipped_protected_count += 1;
            continue;
        }
        let payload = read_json(&latest_path).unwrap_or(Value::Null);
        let ts = clean(
            payload
                .get("ts")
                .and_then(Value::as_str)
                .or_else(|| payload.get("generated_at").and_then(Value::as_str))
                .unwrap_or(""),
            80,
        );
        let latest_mtime_ms = file_mtime_epoch_ms(&latest_path);
        let (duration_ms, source) = duration_from_ts_or_mtime_ms(&ts, latest_mtime_ms);
        if duration_ms < stale_threshold_ms {
            continue;
        }
        candidates.push((
            duration_ms,
            lane,
            latest_path.display().to_string(),
            latest_path,
            source.to_string(),
        ));
    }
    candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    let mut reclaimed_count: usize = 0;
    let mut rows = Vec::<Value>::new();
    let max_apply = max_reclaims.max(1);
    let archive_dir = state_root(root).join("cockpit").join("reclaimed");
    if !dry_run {
        let _ = fs::create_dir_all(&archive_dir);
    }
    for (idx, (duration_ms, lane, from_display, from_path, source)) in candidates.iter().enumerate() {
        if idx >= max_apply {
            break;
        }
        if dry_run {
            rows.push(json!({
                "lane": lane,
                "duration_ms": duration_ms,
                "duration_source": source,
                "from": from_display,
                "reclaimed": false,
                "dry_run": true
            }));
            continue;
        }
        let lane_token = clean_id(lane, "lane");
        let stamp = Utc::now().timestamp_millis().max(0) as u64;
        let archive_path = archive_dir.join(format!("{lane_token}-{stamp}.json"));
        let moved = fs::rename(from_path, &archive_path)
            .or_else(|_| fs::copy(from_path, &archive_path).map(|_| ()))
            .and_then(|_| fs::remove_file(from_path).or(Ok(())));
        if moved.is_ok() {
            reclaimed_count += 1;
            rows.push(json!({
                "lane": lane,
                "duration_ms": duration_ms,
                "duration_source": source,
                "from": from_display,
                "to": archive_path.display().to_string(),
                "reclaimed": true
            }));
        } else if let Err(err) = moved {
            errors.push(clean(&format!("reclaim_failed:{lane}:{err}"), 320));
        }
    }
    json!({
        "ok": errors.is_empty(),
        "type": "hermes_plane_reclaim_stale",
        "dry_run": dry_run,
        "stale_threshold_ms": stale_threshold_ms,
        "max_reclaims": max_apply,
        "scanned_lanes": scanned_lanes,
        "candidate_count": candidates.len(),
        "reclaimed_count": reclaimed_count,
        "skipped_protected_count": skipped_protected_count,
        "rows": rows,
        "errors": errors
    })
}

fn classify_tool_call(ty: &str) -> &'static str {
    let lower = ty.to_ascii_lowercase();
    if lower.contains("memory") || lower.contains("embedding") || lower.contains("provider") {
        "capability_runtime"
    } else if lower.contains("research") {
        "research"
    } else if lower.contains("parse") {
        "parse"
    } else if lower.contains("mcp") {
        "mcp"
    } else if lower.contains("skills") {
        "skills"
    } else if lower.contains("binary") {
        "security"
    } else if lower.contains("vbrowser") {
        "browser"
    } else {
        "runtime"
    }
}

fn provider_runtime_contract_posture(payload: &Value) -> Value {
    let compact = payload.to_string().to_ascii_lowercase();
    let families = ["openai", "openrouter", "xai"];
    let family_rows = families
        .into_iter()
        .map(|family| {
            json!({
                "family": family,
                "present": compact.contains(&format!("\"{family}\""))
            })
        })
        .collect::<Vec<_>>();
    json!({
        "resolution_mode": "registered_first_capability_fallback",
        "required_contracts": [
            "memoryEmbeddingProviders",
            "speechProviders",
            "realtimeVoiceProviders"
        ],
        "memory_embedding_contract_present": compact.contains("memoryembeddingproviders"),
        "capability_provider_runtime_present": compact.contains("capability-provider-runtime")
            || compact.contains("capability_provider_runtime"),
        "provider_families": family_rows
    })
}

fn status_color(ok: bool, class: &str) -> &'static str {
    if !ok {
        "red"
    } else if class == "security" {
        "amber"
    } else if class == "browser" {
        "blue"
    } else {
        "green"
    }
}

fn parse_ts_epoch_ms(ts: &str) -> Option<u64> {
    let parsed = DateTime::parse_from_rfc3339(ts).ok();
    parsed
        .map(|value| value.with_timezone(&Utc).timestamp_millis())
        .and_then(|ms| u64::try_from(ms).ok())
}

fn file_mtime_epoch_ms(path: &Path) -> u64 {
    fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

fn duration_from_epoch_ms(epoch_ms: u64) -> u64 {
    let now_ms = Utc::now().timestamp_millis();
    let now_ms = u64::try_from(now_ms).unwrap_or(0);
    now_ms.saturating_sub(epoch_ms)
}

fn duration_from_ts_or_mtime_ms(ts: &str, latest_mtime_ms: u64) -> (u64, &'static str) {
    if let Some(ts_ms) = parse_ts_epoch_ms(ts) {
        return (duration_from_epoch_ms(ts_ms), "event_ts");
    }
    if latest_mtime_ms > 0 {
        return (duration_from_epoch_ms(latest_mtime_ms), "latest_mtime");
    }
    (0, "unknown")
}
