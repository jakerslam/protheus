fn build_snapshot(root: &Path, flags: &Flags) -> Value {
    let team = if flags.team.trim().is_empty() {
        DEFAULT_TEAM.to_string()
    } else {
        clean_text(&flags.team, 80)
    };
    let contract_enforcement = dashboard_agent_state::enforce_expired_contracts(root);
    let app_payload = read_json_file(&root.join("core/local/state/ops/app_plane/latest.json"))
        .or_else(|| read_cached_snapshot_component(root, "app"))
        .unwrap_or_else(|| json!({}));

    let mut collab_payload = read_json_file(&root.join(format!(
        "core/local/state/ops/collab_plane/dashboard/{team}.json"
    )))
    .map(|dashboard| {
        json!({
            "ok": true,
            "type": "collab_plane_dashboard",
            "dashboard": dashboard
        })
    })
    .or_else(|| read_cached_snapshot_component(root, "collab"))
    .unwrap_or_else(|| json!({}));
    dashboard_agent_state::merge_profiles_into_collab(root, &mut collab_payload, &team);

    let skills_payload =
        read_json_file(&root.join("core/local/state/ops/skills_plane/latest.json"))
            .or_else(|| read_cached_snapshot_component(root, "skills"))
            .unwrap_or_else(|| json!({}));
    let web_tooling_summary = collect_web_tooling_summary(root);

    let health_payload = read_cached_snapshot_component(root, "health").unwrap_or_else(|| {
        json!({
            "ok": true,
            "type": "health_status_dashboard_cache_fallback",
            "checks": {},
            "alerts": {},
            "dashboard_metrics": {}
        })
    });
    let runtime_sync_payload = build_runtime_sync(root, flags);
    let cockpit_runtime = runtime_sync_payload
        .get("cockpit")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let attention_runtime = runtime_sync_payload
        .get("attention_queue")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let runtime_summary = runtime_sync_payload
        .get("summary")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let memory_entries = collect_memory_artifacts(root);
    let memory_seq = memory_entries.len() as i64;
    let queue_depth = i64_from_value(attention_runtime.get("queue_depth"), 0);
    let memory_pause_threshold = 80i64;
    let memory_resume_threshold = 50i64;
    let memory_entry_threshold = 25i64;
    let memory_ingest_paused =
        queue_depth >= memory_pause_threshold || memory_seq >= memory_entry_threshold;
    let profiles = read_json_file(
        &root.join("client/runtime/local/state/ui/infring_dashboard/agent_profiles.json"),
    )
    .and_then(|value| value.get("agents").and_then(Value::as_object).cloned())
    .unwrap_or_default();
    let contracts = read_json_file(
        &root.join("client/runtime/local/state/ui/infring_dashboard/agent_contracts.json"),
    )
    .and_then(|value| value.get("contracts").and_then(Value::as_object).cloned())
    .unwrap_or_default();
    let archived = read_json_file(
        &root.join("client/runtime/local/state/ui/infring_dashboard/archived_agents.json"),
    )
    .and_then(|value| value.get("agents").and_then(Value::as_object).cloned())
    .unwrap_or_default();
    let mut roster_ids = std::collections::HashSet::<String>::new();
    for id in profiles.keys() {
        let normalized = clean_text(id, 140);
        if !normalized.is_empty() {
            roster_ids.insert(normalized);
        }
    }
    for id in contracts.keys() {
        let normalized = clean_text(id, 140);
        if !normalized.is_empty() {
            roster_ids.insert(normalized);
        }
    }
    let mut active_count = 0i64;
    let mut idle_agents = 0i64;
    for agent_id in roster_ids {
        if archived.contains_key(&agent_id) {
            continue;
        }
        let profile = profiles.get(&agent_id);
        let profile_state = clean_text(
            profile
                .and_then(|row| row.get("state").and_then(Value::as_str))
                .unwrap_or(""),
            40,
        )
        .to_ascii_lowercase();
        if profile_state == "archived" {
            continue;
        }
        let contract_status = clean_text(
            contracts
                .get(&agent_id)
                .and_then(|row| row.get("status").and_then(Value::as_str))
                .unwrap_or("active"),
            40,
        )
        .to_ascii_lowercase();
        if contract_status == "terminated" {
            continue;
        }
        if profile_state == "running" || profile_state == "active" {
            active_count += 1;
        } else {
            idle_agents += 1;
        }
    }
    let idle_threshold = 3i64;
    let terminated_recent = dashboard_agent_state::terminated_entries(root)
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let cockpit_stale_actionable =
        i64_from_value(cockpit_runtime.pointer("/metrics/stale_block_count"), 0);
    let runtime_stall_detected =
        queue_depth >= RUNTIME_SYNC_WARN_DEPTH || cockpit_stale_actionable > 0;
    let normal_cadence_ms = flags.refresh_ms.max(500);
    let emergency_cadence_ms = (flags.refresh_ms / 2).max(500);

    let mut out = json!({
        "ok": true,
        "type": "infring_dashboard_snapshot",
        "ts": now_iso(),
        "metadata": {
            "root": root.to_string_lossy().to_string(),
            "team": team,
            "refresh_ms": flags.refresh_ms,
            "authority": "rust_core_cached_runtime_state",
            "sources": {
                "app": "core/local/state/ops/app_plane/latest.json",
                "collab": format!("core/local/state/ops/collab_plane/dashboard/{team}.json"),
                "skills": "core/local/state/ops/skills_plane/latest.json",
                "health": "client/runtime/local/state/ui/infring_dashboard/latest_snapshot.json#health",
                "runtime_sync": "protheus-ops dashboard-ui runtime-sync",
                "channel_registry": "client/runtime/local/state/ui/infring_dashboard/channel_registry.json",
                "provider_registry": "client/runtime/local/state/ui/infring_dashboard/provider_registry.json"
            }
        },
        "health": health_payload,
        "web_tooling": web_tooling_summary,
        "runtime_sync": runtime_summary,
        "cockpit": cockpit_runtime,
        "attention_queue": attention_runtime,
        "app": app_payload,
        "collab": collab_payload,
        "skills": skills_payload,
        "agents": {
            "session_summaries": dashboard_agent_state::session_summaries(root, 200),
            "contract_enforcement": contract_enforcement
        },
        "agent_lifecycle": {
            "active_count": active_count,
            "idle_agents": idle_agents,
            "idle_threshold": idle_threshold,
            "idle_alert": idle_agents >= idle_threshold,
            "terminated_recent": terminated_recent
        },
        "runtime_autoheal": {
            "last_result": if runtime_stall_detected { "watching_backpressure" } else { "healthy" },
            "last_stage": if runtime_stall_detected { "monitor" } else { "steady" },
            "stall_detected": runtime_stall_detected,
            "cadence_ms": {
                "normal": normal_cadence_ms,
                "emergency": emergency_cadence_ms
            }
        },
        "memory": {
            "entries": memory_entries,
            "stream": {
                "enabled": true,
                "changed": false,
                "seq": memory_seq,
                "index_strategy": "hour_bucket_time_series"
            },
            "ingest_control": {
                "paused": memory_ingest_paused,
                "pause_threshold": memory_pause_threshold,
                "resume_threshold": memory_resume_threshold,
                "memory_entry_threshold": memory_entry_threshold
            }
        },
        "receipts": {
            "recent": collect_receipts(root),
            "action_history_path": ACTION_HISTORY_REL
        },
        "logs": {
            "recent": collect_log_events(root)
        },
        "apm": {
            "metrics": [],
            "checks": {},
            "alerts": {}
        }
    });
    out["apm"]["metrics"] = Value::Array(metric_rows(&out["health"]));
    out["apm"]["checks"] = out["health"]
        .get("checks")
        .cloned()
        .unwrap_or_else(|| json!({}));
    out["apm"]["alerts"] = out["health"]
        .get("alerts")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let component_checksums = json!({
        "app": crate::deterministic_receipt_hash(&app_payload),
        "collab": crate::deterministic_receipt_hash(&collab_payload),
        "skills": crate::deterministic_receipt_hash(&skills_payload),
        "health": crate::deterministic_receipt_hash(&health_payload),
        "runtime_sync": crate::deterministic_receipt_hash(&runtime_summary),
        "attention_queue": crate::deterministic_receipt_hash(&attention_runtime),
        "cockpit": crate::deterministic_receipt_hash(&cockpit_runtime),
        "memory": crate::deterministic_receipt_hash(&out["memory"]),
        "agent_lifecycle": crate::deterministic_receipt_hash(&out["agent_lifecycle"]),
        "web_tooling": crate::deterministic_receipt_hash(&out["web_tooling"])
    });
    let composite_checksum = crate::deterministic_receipt_hash(&component_checksums);
    let previous_composite = read_json_file(&root.join(SNAPSHOT_LATEST_REL))
        .and_then(|row| {
            row.pointer("/sync/composite_checksum")
                .and_then(Value::as_str)
                .map(|raw| clean_text(raw, 160))
        })
        .unwrap_or_default();
    let sync_changed = previous_composite != composite_checksum;
    let previous_checksum_value = if previous_composite.is_empty() {
        Value::Null
    } else {
        Value::String(previous_composite)
    };
    out["sync"] = json!({
        "strategy": "component_receipt_hash_v1",
        "component_checksums": component_checksums,
        "composite_checksum": composite_checksum,
        "previous_composite_checksum": previous_checksum_value,
        "changed": sync_changed,
        "checkpoint_ts": now_iso()
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn write_snapshot_receipt(root: &Path, snapshot: &Value) {
    let latest = root.join(SNAPSHOT_LATEST_REL);
    let history = root.join(SNAPSHOT_HISTORY_REL);
    write_json(&latest, snapshot);
    append_jsonl(&history, snapshot);
    let force = fs::metadata(&history)
        .map(|meta| meta.len() > SNAPSHOT_HISTORY_MAX_BYTES)
        .unwrap_or(false);
    if should_prune_snapshot_history(&history, force) {
        trim_snapshot_history_with_policy(
            &history,
            SNAPSHOT_HISTORY_MAX_BYTES,
            SNAPSHOT_HISTORY_MAX_LINES,
            SNAPSHOT_HISTORY_MAX_AGE_DAYS,
        );
    }
}

fn should_prune_snapshot_history(path: &Path, force: bool) -> bool {
    static LAST_PRUNE_SECONDS: OnceLock<Mutex<HashMap<String, i64>>> = OnceLock::new();
    let now = Utc::now().timestamp();
    let key = path.to_string_lossy().to_string();
    let mut guard = match LAST_PRUNE_SECONDS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    {
        Ok(locked) => locked,
        Err(_) => return force,
    };
    let last = *guard.get(&key).unwrap_or(&0);
    if force || (now - last) >= SNAPSHOT_HISTORY_PRUNE_INTERVAL_SECONDS {
        guard.insert(key, now);
        return true;
    }
    false
}

fn parse_snapshot_timestamp(row: &Value) -> Option<DateTime<Utc>> {
    row.get("ts")
        .and_then(Value::as_str)
        .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
        .map(|ts| ts.with_timezone(&Utc))
}

fn trim_snapshot_history_with_policy(
    path: &Path,
    max_bytes: u64,
    max_lines: usize,
    max_age_days: i64,
) {
    let meta = match fs::metadata(path) {
        Ok(meta) => meta,
        Err(_) => return,
    };
    if meta.len() == 0 {
        return;
    }

    let byte_cap = max_bytes.max(1);
    let line_cap = max_lines.max(1);
    let cutoff = Utc::now() - chrono::Duration::days(max_age_days.max(0));
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return,
    };

    let tail_window = meta
        .len()
        .min(byte_cap.saturating_add(LOG_TAIL_MAX_READ_BYTES as u64));
    if meta.len() > tail_window {
        let _ = file.seek(SeekFrom::End(-(tail_window as i64)));
    }

    let mut reader = BufReader::new(file);
    let mut raw = String::new();
    if reader.read_to_string(&mut raw).is_err() {
        return;
    }
    if meta.len() > tail_window {
        if let Some((_, rest)) = raw.split_once('\n') {
            raw = rest.to_string();
        }
    }

    let mut kept = VecDeque::<String>::new();
    let mut kept_bytes = 0u64;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parsed = parse_json_loose(trimmed).unwrap_or(Value::Null);
        let keep_by_age = parse_snapshot_timestamp(&parsed)
            .map(|ts| ts >= cutoff)
            .unwrap_or(true);
        if !keep_by_age {
            continue;
        }
        kept.push_back(trimmed.to_string());
        kept_bytes = kept_bytes.saturating_add((trimmed.len() + 1) as u64);
        while kept.len() > line_cap || kept_bytes > byte_cap {
            if let Some(removed) = kept.pop_front() {
                kept_bytes = kept_bytes.saturating_sub((removed.len() + 1) as u64);
            } else {
                break;
            }
        }
    }

    let mut out = String::new();
    while let Some(line) = kept.pop_front() {
        out.push_str(&line);
        out.push('\n');
    }
    let _ = fs::write(path, out);
}
