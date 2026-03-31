
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
    let collab_agents = collab_payload
        .pointer("/dashboard/agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut active_count = 0i64;
    let mut idle_agents = 0i64;
    for row in &collab_agents {
        let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 40)
            .to_ascii_lowercase();
        if status == "active" || status == "running" {
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
                "runtime_sync": "protheus-ops dashboard-ui runtime-sync"
            }
        },
        "health": health_payload,
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
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn write_snapshot_receipt(root: &Path, snapshot: &Value) {
    let latest = root.join(SNAPSHOT_LATEST_REL);
    let history = root.join(SNAPSHOT_HISTORY_REL);
    write_json(&latest, snapshot);
    append_jsonl(&history, snapshot);
}
