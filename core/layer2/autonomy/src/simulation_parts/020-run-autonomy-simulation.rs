fn normalize_queue_status(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "queued" | "todo" | "new" => "pending".to_string(),
        "running" | "in_progress" | "opened" => "open".to_string(),
        other => other.to_string(),
    }
}

fn parse_row_timestamp_ms(row: &Value) -> Option<i64> {
    for key in ["created_at", "createdAt", "ts", "timestamp", "created_ms"] {
        let Some(raw) = row.get(key) else {
            continue;
        };
        if let Some(v) = raw.as_i64() {
            if v > 1_000_000_000_000 {
                return Some(v);
            }
            if v > 1_000_000_000 {
                return Some(v * 1000);
            }
        }
        if let Some(v) = raw.as_u64() {
            let as_i64 = i64::try_from(v).ok()?;
            if as_i64 > 1_000_000_000_000 {
                return Some(as_i64);
            }
            if as_i64 > 1_000_000_000 {
                return Some(as_i64 * 1000);
            }
        }
        if let Some(text) = raw.as_str() {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(trimmed) {
                return Some(dt.timestamp_millis());
            }
            if let Ok(parsed) = trimmed.parse::<i64>() {
                if parsed > 1_000_000_000_000 {
                    return Some(parsed);
                }
                if parsed > 1_000_000_000 {
                    return Some(parsed * 1000);
                }
            }
        }
    }
    None
}

fn queue_snapshot(dates: &[String], proposals_dir: &Path) -> Value {
    let mut total = 0i64;
    let mut pending = 0i64;
    let mut stale = 0i64;
    let now_ms = chrono::Utc::now().timestamp_millis();

    for day in dates {
        let fp = proposals_dir.join(format!("{day}.json"));
        let raw = read_json(&fp);
        let rows = if let Some(arr) = raw.as_array() {
            arr.clone()
        } else {
            raw.get("proposals")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        };

        for row in rows {
            if !row.is_object() {
                continue;
            }
            total += 1;
            let status = normalize_queue_status(
                row
                .get("status")
                .or_else(|| row.get("state"))
                .and_then(Value::as_str)
                .unwrap_or("pending"),
            );
            if status == "pending" || status == "open" {
                pending += 1;
                let day_floor_ms =
                    chrono::DateTime::parse_from_rfc3339(&format!("{day}T00:00:00.000Z"))
                        .ok()
                        .map(|dt| dt.timestamp_millis())
                        .unwrap_or(now_ms);
                let created_ms = parse_row_timestamp_ms(&row).unwrap_or(day_floor_ms);
                if now_ms.saturating_sub(created_ms) >= 72 * 3600 * 1000 {
                    stale += 1;
                }
            }
        }
    }

    json!({
        "total": total,
        "pending": pending,
        "stale_pending_72h": stale
    })
}

pub fn run_autonomy_simulation(
    root: &Path,
    end_date: Option<&str>,
    days: i64,
    write_output: bool,
) -> Value {
    let end_date = parse_date_or_today(end_date);
    let days = days.clamp(
        1,
        to_int(
            std::env::var("AUTONOMY_SIM_MAX_DAYS").ok().as_deref(),
            180,
            1,
            365,
        ),
    );

    let runs_dir = std::env::var("AUTONOMY_SIM_RUNS_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            resolve_runtime_path(
                root,
                Some("local/state/autonomy/runs"),
                "local/state/autonomy/runs",
            )
        });
    let proposals_dir = std::env::var("AUTONOMY_SIM_PROPOSALS_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            resolve_runtime_path(
                root,
                Some("local/state/sensory/proposals"),
                "local/state/sensory/proposals",
            )
        });
    let output_dir = std::env::var("AUTONOMY_SIM_OUTPUT_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            resolve_runtime_path(
                root,
                Some("local/state/autonomy/simulations"),
                "local/state/autonomy/simulations",
            )
        });
    let budget_path = std::env::var("AUTONOMY_SIM_BUDGET_AUTOPAUSE_PATH")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            resolve_runtime_path(
                root,
                Some("local/state/autonomy/budget_autopause.json"),
                "local/state/autonomy/budget_autopause.json",
            )
        });

    let dates = date_window(&end_date, days);
    let mut run_rows = Vec::<Value>::new();
    for day in &dates {
        let fp = runs_dir.join(format!("{day}.jsonl"));
        run_rows.extend(read_jsonl(&fp));
    }
    run_rows.retain(|row| row.get("type").and_then(Value::as_str) == Some("autonomy_run"));

    let baseline_attempts_raw = run_rows.clone();
    let baseline_policy_holds: Vec<Value> = baseline_attempts_raw
        .iter()
        .filter(|row| is_policy_hold_event(row))
        .cloned()
        .collect();
    let baseline_budget_holds: Vec<Value> = baseline_policy_holds
        .iter()
        .filter(|row| is_budget_hold_event(row))
        .cloned()
        .collect();
    let baseline_attempts: Vec<Value> = baseline_attempts_raw
        .iter()
        .filter(|row| !is_policy_hold_event(row))
        .cloned()
        .collect();
    let baseline_executed: Vec<Value> = baseline_attempts
        .iter()
        .filter(|row| row.get("result").and_then(Value::as_str) == Some("executed"))
        .cloned()
        .collect();
    let baseline_shipped: Vec<Value> = baseline_executed
        .iter()
        .filter(|row| row.get("outcome").and_then(Value::as_str) == Some("shipped"))
        .cloned()
        .collect();
    let baseline_no_progress: Vec<Value> = baseline_attempts
        .iter()
        .filter(|row| is_no_progress(row))
        .cloned()
        .collect();
    let baseline_safety_stops: Vec<Value> = baseline_attempts
        .iter()
        .filter(|row| is_safety_stop(row))
        .cloned()
        .collect();

    let identity_enabled = parse_bool_str(
        std::env::var("AUTONOMY_SIM_IDENTITY_PROJECTION_ENABLED")
            .ok()
            .as_deref(),
        parse_bool_str(
            std::env::var("SPINE_IDENTITY_ANCHOR_ENABLED")
                .ok()
                .as_deref(),
            false,
        ),
    );
    let block_unknown = parse_bool_str(
        std::env::var("AUTONOMY_SIM_IDENTITY_BLOCK_UNKNOWN_OBJECTIVE")
            .ok()
            .as_deref(),
        parse_bool_str(
            std::env::var("SPINE_IDENTITY_BLOCK_UNKNOWN_OBJECTIVE")
                .ok()
                .as_deref(),
            true,
        ),
    );
    let active_objective_ids: HashSet<String> = std::env::var("AUTONOMY_SIM_ACTIVE_OBJECTIVE_IDS")
        .ok()
        .map(|v| {
            v.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<HashSet<_>>()
        })
        .unwrap_or_else(|| {
            ["T1_build_sovereign_capital_v1".to_string()]
                .iter()
                .cloned()
                .collect()
        });

    let mut identity_blocked = Vec::<Value>::new();
    let mut identity_accepted = Vec::<Value>::new();
    if identity_enabled {
        for row in &baseline_attempts_raw {
            let objective_id = row
                .get("objective_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string();
            let unknown = !objective_id.is_empty() && !active_objective_ids.contains(&objective_id);
            if block_unknown && (objective_id.is_empty() || unknown) {
                identity_blocked.push(json!({
                    "evt": row,
                    "context": {
                        "objective_id": if objective_id.is_empty() { Value::Null } else { json!(objective_id) }
                    },
                    "verdict": {
                        "blocking_codes": ["unknown_active_objective"]
                    }
                }));
                continue;
            }
            identity_accepted.push(row.clone());
        }
    } else {
        identity_accepted = baseline_attempts_raw.clone();
    }

    let effective_policy_holds: Vec<Value> = identity_accepted
        .iter()
        .filter(|row| is_policy_hold_event(row))
        .cloned()
        .collect();
    let effective_budget_holds: Vec<Value> = effective_policy_holds
        .iter()
        .filter(|row| is_budget_hold_event(row))
        .cloned()
        .collect();
    let effective_attempts: Vec<Value> = identity_accepted
        .iter()
        .filter(|row| !is_policy_hold_event(row))
        .cloned()
        .collect();
    let effective_executed: Vec<Value> = effective_attempts
        .iter()
        .filter(|row| row.get("result").and_then(Value::as_str) == Some("executed"))
        .cloned()
        .collect();
    let effective_shipped: Vec<Value> = effective_executed
        .iter()
        .filter(|row| row.get("outcome").and_then(Value::as_str) == Some("shipped"))
        .cloned()
        .collect();
    let effective_no_progress: Vec<Value> = effective_attempts
        .iter()
        .filter(|row| is_no_progress(row))
        .cloned()
        .collect();
    let effective_safety_stops: Vec<Value> = effective_attempts
        .iter()
        .filter(|row| is_safety_stop(row))
        .cloned()
        .collect();

    let baseline_counters = json!({
        "attempts": baseline_attempts_raw.len(),
        "executed": baseline_executed.len(),
        "shipped": baseline_shipped.len(),
        "no_progress": baseline_no_progress.len(),
        "safety_stops": baseline_safety_stops.len(),
        "policy_holds": baseline_policy_holds.len(),
        "budget_holds": baseline_budget_holds.len()
    });
    let effective_counters = json!({
        "attempts": effective_attempts.len(),
        "executed": effective_executed.len(),
        "shipped": effective_shipped.len(),
        "no_progress": effective_no_progress.len(),
        "safety_stops": effective_safety_stops.len(),
        "policy_holds": effective_policy_holds.len(),
        "budget_holds": effective_budget_holds.len()
    });

    let budget_autopause = read_budget_snapshot(&budget_path, &end_date, &run_rows);
    let checks = build_checks(
        baseline_counters
            .as_object()
            .expect("baseline counters object"),
        budget_autopause
            .get("active_relevant")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    );
    let checks_effective = build_checks(
        effective_counters
            .as_object()
            .expect("effective counters object"),
        budget_autopause
            .get("active_relevant")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    );

    let verdict_raw = verdict_from_checks(&checks);
    let verdict_effective = verdict_from_checks(&checks_effective);
    let verdict = if verdict_raw == "fail" || verdict_effective == "fail" {
        "fail"
    } else if verdict_raw == "warn" || verdict_effective == "warn" {
        "warn"
    } else {
        "pass"
    };

    let hold_reasons = json!({
        "baseline": {
            "policy": reason_counts(&baseline_policy_holds, hold_reason),
            "budget": reason_counts(&baseline_budget_holds, hold_reason)
        },
        "effective": {
            "policy": reason_counts(&effective_policy_holds, hold_reason),
            "budget": reason_counts(&effective_budget_holds, hold_reason)
        }
    });

    let mut objective_mix_counts = BTreeMap::<String, i64>::new();
    for row in &effective_executed {
        let objective_id = row
            .get("objective_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        if objective_id.is_empty() {
            continue;
        }
        *objective_mix_counts.entry(objective_id).or_insert(0) += 1;
    }
    let objective_mix_map: Map<String, Value> = objective_mix_counts
        .iter()
        .map(|(k, v)| (k.clone(), json!(*v)))
        .collect();

    let queue = queue_snapshot(&dates, &proposals_dir);

    let compiler_projection_enabled = parse_bool_str(
        std::env::var("AUTONOMY_SIM_LINEAGE_REQUIRED")
            .ok()
            .as_deref(),
        true,
    );

    let identity_summary = json!({
        "checked": baseline_attempts_raw.len(),
        "blocked": identity_blocked.len(),
        "allowed": identity_accepted.len(),
        "identity_drift_score": 0,
        "max_identity_drift_score": 0.58,
        "max_candidate_drift_score": 0,
        "blocking_code_counts": {
            "unknown_active_objective": identity_blocked.len()
        }
    });

    let mut insufficient_reasons = Vec::<Value>::new();
    if run_rows.is_empty() {
        insufficient_reasons.push(json!("no_run_rows_in_window"));
    }
    if baseline_attempts_raw.len()
        < to_int(
            std::env::var("AUTONOMY_SIM_MIN_ATTEMPTS").ok().as_deref(),
            5,
            1,
            100000,
        ) as usize
    {
        insufficient_reasons.push(json!("attempt_volume_below_min"));
    }
    if baseline_executed.is_empty() {
        insufficient_reasons.push(json!("no_executed_attempts"));
    }
    if baseline_shipped.is_empty() {
        insufficient_reasons.push(json!("no_shipped_outcomes"));
    }

    let mut recommendations = Vec::<Value>::new();
    if checks
        .get("policy_hold_rate")
        .and_then(|v| v.get("status"))
        .and_then(Value::as_str)
        != Some("pass")
    {
        recommendations.push(json!(
            "Reduce policy-hold churn by tightening admission and queue hygiene."
        ));
    }
    if checks
        .get("budget_hold_rate")
        .and_then(|v| v.get("status"))
        .and_then(Value::as_str)
        != Some("pass")
    {
        recommendations.push(json!(
            "Budget holds are elevated; apply pacing/defer strategy before full cadence."
        ));
    }
    if recommendations.is_empty() {
        recommendations.push(json!(
            "Simulation is stable; continue collecting telemetry and tighten targeted bottlenecks."
        ));
    }

    let mut payload = json!({
        "ok": true,
        "type": "autonomy_simulation_harness",
        "ts": now_iso(),
        "end_date": end_date,
        "days": days,
        "verdict": verdict,
        "verdict_raw": verdict_raw,
        "verdict_effective": verdict_effective,
        "checks": checks,
        "checks_effective": checks_effective,
        "metric_integrity": {
            "mode": "dual_track",
            "baseline_preserved": true,
            "effective_projection_present": true,
            "denominator_reduction_only": effective_attempts.len() < baseline_attempts_raw.len(),
            "denominator_delta": baseline_attempts_raw.len() as i64 - effective_attempts.len() as i64,
            "identity_projection_enabled": identity_enabled,
            "identity_projection_blocked_attempts": identity_blocked.len()
        },
        "counters": baseline_counters,
        "baseline_counters": baseline_counters,
        "effective_counters": effective_counters,
        "hold_reasons": hold_reasons,
        "budget_autopause": budget_autopause,
        "compiler_projection": {
            "enabled": compiler_projection_enabled,
            "lineage_require_t1_root": parse_bool_str(std::env::var("AUTONOMY_SIM_LINEAGE_REQUIRE_T1_ROOT").ok().as_deref(), true),
            "lineage_block_missing_objective": parse_bool_str(std::env::var("AUTONOMY_SIM_LINEAGE_BLOCK_MISSING_OBJECTIVE").ok().as_deref(), true),
            "filter_contextless_attempts": parse_bool_str(std::env::var("AUTONOMY_SIM_LINEAGE_FILTER_CONTEXTLESS").ok().as_deref(), true),
            "rolling_context_enabled": parse_bool_str(std::env::var("AUTONOMY_SIM_LINEAGE_ROLLING_CONTEXT").ok().as_deref(), false),
            "compiler_hash": Value::Null,
            "compiler_active_count": 0,
            "accepted_attempts": identity_accepted.len(),
            "rejected_attempts": 0,
            "skipped_attempts": 0,
            "rejected_by_reason": {},
            "skipped_by_reason": {},
            "sample_rejected": [],
            "sample_skipped": []
        },
        "identity_projection": {
            "enabled": identity_enabled,
            "unavailable": false,
            "unavailable_reason": Value::Null,
            "policy_path": Value::Null,
            "active_objective_ids": active_objective_ids.into_iter().collect::<Vec<_>>(),
            "attempted": baseline_attempts_raw.len(),
            "blocked_attempts": identity_blocked.len(),
            "blocked_by_reason": {
                "unknown_active_objective": identity_blocked.len()
            },
            "summary": identity_summary,
            "sample_blocked": identity_blocked.into_iter().take(8).collect::<Vec<_>>()
        },
        "queue": queue,
        "objective_mix": {
            "executed_total": effective_executed.len(),
            "objective_count": objective_mix_counts.len(),
            "counts": objective_mix_map
        },
        "insufficient_data": {
            "active": !insufficient_reasons.is_empty(),
            "reasons": insufficient_reasons
        },
        "recommendations": recommendations.into_iter().take(5).collect::<Vec<_>>()
    });

    if write_output {
        let report_path = output_dir.join(format!("{}.json", parse_date_or_today(Some(&end_date))));
        let _ = write_json_atomic(&report_path, &payload);
        payload["report_path"] = json!(report_path);
    }

    let _ = append_jsonl(
        &resolve_runtime_path(
            root,
            Some("local/state/autonomy/simulations/history.jsonl"),
            "local/state/autonomy/simulations/history.jsonl",
        ),
        &json!({
            "ts": payload.get("ts").cloned().unwrap_or(Value::Null),
            "type": "autonomy_simulation_harness",
            "date": payload.get("end_date").cloned().unwrap_or(Value::Null),
            "verdict": payload.get("verdict").cloned().unwrap_or(Value::Null)
        }),
    );

    payload
}
