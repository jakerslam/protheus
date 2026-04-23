fn web_tooling_ops_from_receipts(receipts: &[Value]) -> Value {
    let mut turn_count = 0i64;
    let mut intent_detected = 0i64;
    let mut tool_attempted = 0i64;
    let mut tool_blocked = 0i64;
    let mut low_signal = 0i64;
    let mut final_repaired = 0i64;
    let mut no_response = 0i64;
    for row in receipts {
        let response = clean_text(
            row.get("response")
                .or_else(|| row.pointer("/turn/assistant"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            4_000,
        );
        let web_invariant = row
            .pointer("/response_finalization/web_invariant")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let detected = web_invariant
            .get("requires_live_web")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let attempted = web_invariant
            .get("tool_attempted")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let blocked = web_invariant
            .get("tool_blocked")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let low = web_invariant
            .get("low_signal")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let repaired = row
            .pointer("/response_finalization/visible_response_repaired")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || web_invariant
                .get("invariant_repair_used")
                .and_then(Value::as_bool)
                .unwrap_or(false);
        let no_resp = response.trim().is_empty()
            || row
                .pointer("/response_finalization/final_ack_only")
                .and_then(Value::as_bool)
                .unwrap_or(false);
        if detected || attempted || blocked || low || repaired || no_resp || !response.is_empty() {
            turn_count += 1;
        }
        if detected {
            intent_detected += 1;
        }
        if attempted {
            tool_attempted += 1;
        }
        if blocked {
            tool_blocked += 1;
        }
        if low {
            low_signal += 1;
        }
        if repaired {
            final_repaired += 1;
        }
        if no_resp {
            no_response += 1;
        }
    }
    let denominator = if turn_count > 0 { turn_count as f64 } else { 1.0 };
    let no_response_rate = (no_response as f64) / denominator;
    json!({
        "window_turns": turn_count,
        "intent_detected": intent_detected,
        "tool_attempted": tool_attempted,
        "tool_blocked": tool_blocked,
        "low_signal": low_signal,
        "final_repaired": final_repaired,
        "no_response": no_response,
        "no_response_rate": no_response_rate
    })
}

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
    let runtime_freshness = runtime_sync_payload
        .get("freshness")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let runtime_freshness_stale_surfaces =
        i64_from_value(runtime_freshness.pointer("/summary/stale_surfaces"), 0);
    let runtime_freshness_stale = runtime_freshness
        .pointer("/summary/stale")
        .and_then(Value::as_bool)
        .unwrap_or(runtime_freshness_stale_surfaces > 0);
    let runtime_freshness_attention = runtime_freshness
        .get("attention_status")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let runtime_freshness_attention_next = runtime_freshness
        .get("attention_next")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let runtime_freshness_primary = if runtime_freshness_attention
        .get("source")
        .and_then(Value::as_str)
        .unwrap_or("")
        .is_empty()
    {
        runtime_freshness_attention_next.clone()
    } else {
        runtime_freshness_attention.clone()
    };
    let runtime_freshness_source = clean_text(
        runtime_freshness_primary
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("runtime_sync.freshness"),
        120,
    );
    let runtime_freshness_sequence = clean_text(
        runtime_freshness_primary
            .get("sequence")
            .and_then(Value::as_str)
            .unwrap_or(""),
        160,
    );
    let runtime_freshness_age_seconds =
        (i64_from_value(runtime_freshness_primary.get("age_ms"), 0)).max(0) / 1000;
    let runtime_freshness_primary_stale = runtime_freshness_primary
        .get("stale")
        .and_then(Value::as_bool)
        .unwrap_or(runtime_freshness_stale);
    let runtime_backpressure_level = clean_text(
        runtime_summary
            .get("backpressure_level")
            .and_then(Value::as_str)
            .unwrap_or("normal"),
        24,
    )
    .to_ascii_lowercase();
    let runtime_backpressure_degraded =
        runtime_backpressure_level == "high" || runtime_backpressure_level == "critical";
    let conduit_lifecycle = read_json_file(
        &root.join("core/local/state/ops/agency_plane/conduit/lifecycle.json"),
    )
    .or_else(|| read_json_file(&root.join("local/state/ops/agency_plane/conduit/lifecycle.json")))
    .or_else(|| read_json_file(&root.join("local/state/agency_plane/conduit/lifecycle.json")))
    .unwrap_or_else(|| {
        json!({
            "state": "healthy",
            "transition_count": 0,
            "updated_at": now_iso()
        })
    });
    let conduit_lifecycle_state = clean_text(
        conduit_lifecycle
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or("healthy"),
        40,
    )
    .to_ascii_lowercase();
    let conduit_lifecycle_degraded = matches!(
        conduit_lifecycle_state.as_str(),
        "degraded" | "reconnecting" | "quarantined" | "failed_closed"
    );
    let conduit_lifecycle_source = clean_text(
        conduit_lifecycle
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("agency_plane.conduit.lifecycle"),
        120,
    );
    let conduit_lifecycle_sequence = crate::deterministic_receipt_hash(&conduit_lifecycle);
    let conduit_lifecycle_age_seconds =
        (i64_from_value(conduit_lifecycle.get("age_ms"), 0)).max(0) / 1000;
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
    let web_tooling_runtime_status = clean_text(
        web_tooling_summary
            .pointer("/runtime/status")
            .and_then(Value::as_str)
            .unwrap_or(""),
        40,
    )
    .to_ascii_lowercase();
    let web_tooling_runtime = web_tooling_summary
        .get("runtime")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let web_tooling_runtime_source = clean_text(
        web_tooling_runtime
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("web_tooling.runtime.status"),
        120,
    );
    let web_tooling_runtime_sequence = crate::deterministic_receipt_hash(&web_tooling_runtime);
    let web_tooling_runtime_age_seconds =
        (i64_from_value(web_tooling_runtime.get("age_ms"), 0)).max(0) / 1000;
    let web_tooling_runtime_stale = web_tooling_runtime
        .get("stale")
        .and_then(Value::as_bool)
        .unwrap_or(
            web_tooling_runtime_status == "degraded" || web_tooling_runtime_status == "blocked_auth",
        );
    let web_tooling_degraded =
        web_tooling_runtime_status == "degraded" || web_tooling_runtime_status == "blocked_auth";
    let receipt_rows = collect_receipts(root);
    let web_tooling_ops = web_tooling_ops_from_receipts(&receipt_rows);
    let web_ops_no_response_rate = web_tooling_ops
        .get("no_response_rate")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let web_ops_blocked = i64_from_value(web_tooling_ops.get("tool_blocked"), 0);
    let web_ops_low_signal = i64_from_value(web_tooling_ops.get("low_signal"), 0);
    let web_ops_degraded = web_ops_no_response_rate > 0.05 || web_ops_blocked > 0 || web_ops_low_signal > 0;
    let web_tooling_ops_sequence = crate::deterministic_receipt_hash(&web_tooling_ops);
    let mut ui_controller_payload = read_json_file(
        &root.join("client/runtime/local/state/ui/infring_dashboard/ui_controller_state.json"),
    )
    .unwrap_or_else(|| {
        json!({
            "type": "dashboard_ui_controller_state",
            "initialized": false,
            "initialization_count": 0,
            "last_initialized_at": "",
            "terminal_execution_mode": "vscodeTerminal",
            "subscriptions": {
                "add_to_input": { "count": 0, "last_event_at": "", "last_payload": "" },
                "chat_button_clicked": { "count": 0, "last_event_at": "", "last_payload": "" },
                "history_button_clicked": { "count": 0, "last_event_at": "", "last_payload": "" }
            },
            "source": "dashboard.ui.controller",
            "source_sequence": "",
            "age_seconds": 0,
            "stale": false,
            "updated_at": ""
        })
    });
    if ui_controller_payload
        .get("source")
        .and_then(Value::as_str)
        .unwrap_or("")
        .is_empty()
    {
        ui_controller_payload["source"] = Value::String("dashboard.ui.controller".to_string());
    }
    if ui_controller_payload
        .get("source_sequence")
        .and_then(Value::as_str)
        .unwrap_or("")
        .is_empty()
    {
        let mut seq_seed = ui_controller_payload.clone();
        seq_seed["source_sequence"] = Value::String(String::new());
        ui_controller_payload["source_sequence"] =
            Value::String(crate::deterministic_receipt_hash(&seq_seed));
    }
    if ui_controller_payload.get("age_seconds").is_none() {
        ui_controller_payload["age_seconds"] = Value::from(0);
    }
    if ui_controller_payload.get("stale").is_none() {
        ui_controller_payload["stale"] = Value::Bool(false);
    }
    let runtime_stall_detected =
        runtime_freshness_stale || runtime_backpressure_degraded || web_tooling_degraded;
    let normal_cadence_ms = flags.refresh_ms.max(500);
    let emergency_cadence_ms = (flags.refresh_ms / 2).max(500);
    let runtime_autoheal_result = if web_tooling_degraded {
        "web_tooling_degraded"
    } else if runtime_stall_detected {
        "watching_backpressure"
    } else {
        "healthy"
    };
    let runtime_autoheal_stage = if web_tooling_degraded {
        "web_tooling_recovery"
    } else if runtime_stall_detected {
        "monitor"
    } else {
        "steady"
    };
    let mut runtime_blocks = vec![
        json!({
            "id": "runtime_freshness",
            "title": "Runtime Freshness",
            "state": if runtime_freshness_stale { "degraded" } else { "healthy" },
            "degraded": runtime_freshness_stale,
            "source": runtime_freshness_source.clone(),
            "source_sequence": if runtime_freshness_sequence.is_empty() {
                crate::deterministic_receipt_hash(&runtime_freshness)
            } else {
                runtime_freshness_sequence.clone()
            },
            "age_seconds": runtime_freshness_age_seconds,
            "stale": runtime_freshness_primary_stale,
            "details": {
                "stale_surfaces": runtime_freshness_stale_surfaces,
                "backpressure_level": runtime_backpressure_level.clone(),
                "source": runtime_freshness_source.clone(),
                "source_sequence": if runtime_freshness_sequence.is_empty() {
                    crate::deterministic_receipt_hash(&runtime_freshness)
                } else {
                    runtime_freshness_sequence.clone()
                },
                "age_seconds": runtime_freshness_age_seconds,
                "stale": runtime_freshness_primary_stale
            }
        }),
        json!({
            "id": "conduit_lifecycle",
            "title": "Conduit Lifecycle",
            "state": conduit_lifecycle_state.clone(),
            "degraded": conduit_lifecycle_degraded,
            "source": conduit_lifecycle_source.clone(),
            "source_sequence": conduit_lifecycle_sequence.clone(),
            "age_seconds": conduit_lifecycle_age_seconds,
            "stale": conduit_lifecycle_degraded,
            "details": {
                "source": conduit_lifecycle_source.clone(),
                "source_sequence": conduit_lifecycle_sequence.clone(),
                "age_seconds": conduit_lifecycle_age_seconds,
                "stale": conduit_lifecycle_degraded,
                "payload": conduit_lifecycle.clone()
            }
        }),
        json!({
            "id": "web_tooling_runtime",
            "title": "Web Tooling Runtime",
            "state": if web_tooling_degraded { "degraded" } else { "healthy" },
            "degraded": web_tooling_degraded,
            "source": web_tooling_runtime_source.clone(),
            "source_sequence": web_tooling_runtime_sequence.clone(),
            "age_seconds": web_tooling_runtime_age_seconds,
            "stale": web_tooling_runtime_stale,
            "details": {
                "runtime_status": web_tooling_runtime_status.clone(),
                "source": web_tooling_runtime_source.clone(),
                "source_sequence": web_tooling_runtime_sequence.clone(),
                "age_seconds": web_tooling_runtime_age_seconds,
                "stale": web_tooling_runtime_stale
            }
        }),
    ];
    runtime_blocks.push(json!({
        "id": "web_tooling_ops",
        "title": "Web Tooling Ops",
        "state": if web_ops_degraded { "degraded" } else { "healthy" },
        "degraded": web_ops_degraded,
        "source": "receipts.recent.response_finalization.web_invariant",
        "source_sequence": web_tooling_ops_sequence.clone(),
        "age_seconds": 0,
        "stale": web_ops_degraded,
        "details": {
            "source": "receipts.recent.response_finalization.web_invariant",
            "source_sequence": web_tooling_ops_sequence.clone(),
            "age_seconds": 0,
            "stale": web_ops_degraded,
            "ops": web_tooling_ops.clone()
        }
    }));
    for row in runtime_blocks.iter_mut() {
        if let Some(obj) = row.as_object_mut() {
            let source_value = clean_text(
                obj.get("source")
                    .and_then(Value::as_str)
                    .unwrap_or("dashboard.runtime.unknown"),
                120,
            );
            let id_value = clean_text(obj.get("id").and_then(Value::as_str).unwrap_or("unknown"), 80);
            let mut source_sequence = clean_text(
                obj.get("source_sequence").and_then(Value::as_str).unwrap_or(""),
                160,
            );
            if source_sequence.is_empty() {
                let sequence_seed = json!({
                    "id": id_value,
                    "source": source_value
                });
                source_sequence = crate::deterministic_receipt_hash(&sequence_seed);
                obj.insert(
                    "source_sequence".to_string(),
                    Value::String(source_sequence.clone()),
                );
            }
            let age_seconds_from_top = obj
                .get("age_seconds")
                .and_then(Value::as_i64)
                .unwrap_or(-1);
            let age_seconds = if age_seconds_from_top >= 0 {
                age_seconds_from_top
            } else {
                let detail_age_seconds = obj
                    .get("details")
                    .and_then(|v| v.get("age_seconds"))
                    .and_then(Value::as_i64)
                    .unwrap_or(-1);
                if detail_age_seconds >= 0 {
                    detail_age_seconds
                } else {
                    (obj.get("details")
                        .and_then(|v| v.get("age_ms"))
                        .and_then(Value::as_i64)
                        .unwrap_or(0)
                        .max(0))
                        / 1000
                }
            };
            obj.insert("age_seconds".to_string(), json!(age_seconds.max(0)));
            let stale_value = match obj.get("stale").and_then(Value::as_bool) {
                Some(value) => value,
                None => obj
                    .get("details")
                    .and_then(|v| v.get("stale"))
                    .and_then(Value::as_bool)
                    .unwrap_or_else(|| obj.get("degraded").and_then(Value::as_bool).unwrap_or(false)),
            };
            obj.insert("stale".to_string(), Value::Bool(stale_value));
            if let Some(details_obj) = obj.get_mut("details").and_then(Value::as_object_mut) {
                if !details_obj.contains_key("source") {
                    details_obj.insert("source".to_string(), Value::String(source_value.clone()));
                }
                if !details_obj.contains_key("source_sequence") {
                    details_obj.insert(
                        "source_sequence".to_string(),
                        Value::String(source_sequence.clone()),
                    );
                }
                if !details_obj.contains_key("age_seconds") {
                    details_obj.insert("age_seconds".to_string(), json!(age_seconds.max(0)));
                }
                if !details_obj.contains_key("stale") {
                    details_obj.insert("stale".to_string(), Value::Bool(stale_value));
                }
            }
        }
    }

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
                "runtime_sync": "infring-ops dashboard-ui runtime-sync",
                "ui_controller": "client/runtime/local/state/ui/infring_dashboard/ui_controller_state.json",
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
        "ui_controller": ui_controller_payload,
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
            "last_result": runtime_autoheal_result,
            "last_stage": runtime_autoheal_stage,
            "stall_detected": runtime_stall_detected,
            "web_tooling_degraded": web_tooling_degraded,
            "web_tooling_runtime_status": web_tooling_runtime_status,
            "freshness": runtime_freshness,
            "stale_surfaces": runtime_freshness_stale_surfaces,
            "freshness_stale": runtime_freshness_stale,
            "backpressure_level": runtime_backpressure_level,
            "conduit_lifecycle": conduit_lifecycle,
            "conduit_lifecycle_state": conduit_lifecycle_state,
            "conduit_lifecycle_degraded": conduit_lifecycle_degraded,
            "cadence_ms": {
                "normal": normal_cadence_ms,
                "emergency": emergency_cadence_ms
            }
        },
        "dashboard_blocks": {
            "runtime": runtime_blocks
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
            "recent": receipt_rows,
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
    out["web_tooling"]["ops"] = web_tooling_ops;
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
        "web_tooling": crate::deterministic_receipt_hash(&out["web_tooling"]),
        "ui_controller": crate::deterministic_receipt_hash(&out["ui_controller"])
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
    dashboard_troubleshooting_bootstrap_runtime_activation(root, snapshot);
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
