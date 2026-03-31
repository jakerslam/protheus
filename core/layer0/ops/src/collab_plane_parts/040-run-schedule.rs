fn run_schedule(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        SCHEDULER_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "collab_team_scheduler_contract",
            "allowed_ops": ["upsert", "kickoff", "list"]
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("collab_scheduler_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "collab_team_scheduler_contract"
    {
        errors.push("collab_scheduler_contract_kind_invalid".to_string());
    }
    let team = team_slug(
        parsed
            .flags
            .get("team")
            .map(String::as_str)
            .unwrap_or("default-team"),
    );
    let op = clean(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "list".to_string()),
        40,
    )
    .to_ascii_lowercase();
    let allowed_ops = contract
        .get("allowed_ops")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!("upsert"), json!("kickoff"), json!("list")])
        .iter()
        .filter_map(Value::as_str)
        .map(|v| clean(v, 30).to_ascii_lowercase())
        .collect::<Vec<_>>();
    if strict && !allowed_ops.iter().any(|v| v == &op) {
        errors.push("collab_scheduler_op_invalid".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "collab_plane_schedule",
            "errors": errors
        });
    }

    let path = schedule_state_path(root, &team);
    let mut schedule_state = read_json(&path).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "team": team,
            "jobs": []
        })
    });
    if !schedule_state
        .get("jobs")
        .map(Value::is_array)
        .unwrap_or(false)
    {
        schedule_state["jobs"] = Value::Array(Vec::new());
    }

    let job_id = clean(
        parsed
            .flags
            .get("job")
            .cloned()
            .unwrap_or_else(|| "default-job".to_string()),
        120,
    );
    let cron = clean(
        parsed
            .flags
            .get("cron")
            .cloned()
            .unwrap_or_else(|| "*/30 * * * *".to_string()),
        120,
    );
    let shadows = parsed
        .flags
        .get("shadows")
        .map(|raw| split_csv_clean(raw, 80))
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| vec!["default-shadow".to_string()]);

    let mut kickoff_receipts = Vec::<Value>::new();
    match op.as_str() {
        "upsert" => {
            let mut jobs = schedule_state
                .get("jobs")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let mut replaced = false;
            for row in &mut jobs {
                if row.get("job_id").and_then(Value::as_str) == Some(job_id.as_str()) {
                    *row = json!({
                        "job_id": job_id,
                        "cron": cron,
                        "shadows": shadows,
                        "updated_at": crate::now_iso()
                    });
                    replaced = true;
                }
            }
            if !replaced {
                jobs.push(json!({
                    "job_id": job_id,
                    "cron": cron,
                    "shadows": shadows,
                    "updated_at": crate::now_iso()
                }));
            }
            schedule_state["jobs"] = Value::Array(jobs);
        }
        "kickoff" => {
            kickoff_receipts = shadows
                .iter()
                .enumerate()
                .map(|(idx, shadow)| {
                    json!({
                        "index": idx + 1,
                        "job_id": job_id,
                        "shadow": shadow,
                        "kickoff_ts": crate::now_iso(),
                        "handoff_hash": sha256_hex_str(&format!("{}:{}:{}:{}", team, job_id, shadow, idx + 1))
                    })
                })
                .collect::<Vec<_>>();
            let mut team_state = read_json(&team_state_path(root, &team))
                .unwrap_or_else(|| default_team_state(&team));
            let mut handoffs = team_state
                .get("handoffs")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            handoffs.extend(kickoff_receipts.clone());
            team_state["handoffs"] = Value::Array(handoffs);
            let limits = parse_launcher_limits(&load_json_or(
                root,
                LAUNCHER_CONTRACT_PATH,
                json!({
                    "version": "v1",
                    "kind": "collab_role_launcher_contract",
                    "limits": {
                        "base_max_active_agents": BASE_STABLE_MAX_ACTIVE_AGENTS,
                        "max_active_agents": DEFAULT_STABLE_MAX_ACTIVE_AGENTS,
                        "max_agents_per_cell": DEFAULT_MAX_AGENTS_PER_CELL,
                        "director_fanout_cells": DEFAULT_DIRECTOR_FANOUT_CELLS,
                        "max_directors": DEFAULT_MAX_DIRECTORS,
                        "decentralized_min_agents": DEFAULT_DECENTRALIZED_MIN_AGENTS,
                        "auto_director_spawn": true
                    }
                }),
            ));
            refresh_team_topology(&mut team_state, limits);
            let _ = write_json(&team_state_path(root, &team), &team_state);
        }
        _ => {}
    }
    schedule_state["updated_at"] = Value::String(crate::now_iso());
    let _ = write_json(&path, &schedule_state);
    let _ = append_jsonl(
        &state_root(root).join("schedules").join("history.jsonl"),
        &json!({
            "type": "collab_schedule",
            "team": team,
            "op": op,
            "job_id": job_id,
            "cron": cron,
            "shadows": shadows,
            "kickoff_count": kickoff_receipts.len(),
            "ts": crate::now_iso()
        }),
    );

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "collab_plane_schedule",
        "lane": "core/layer0/ops",
        "op": op,
        "team": team,
        "job_id": job_id,
        "schedule": schedule_state,
        "kickoff_receipts": kickoff_receipts,
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&schedule_state.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-COLLAB-001.3",
                "claim": "team_scheduler_supports_deterministic_kickoff_and_handoff_receipts",
                "evidence": {
                    "team": team,
                    "op": op,
                    "kickoff_count": kickoff_receipts.len()
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_throttle(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        THROTTLE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "collab_team_throttle_contract",
            "min_depth": 1,
            "max_depth": 1000000,
            "allowed_strategies": ["priority-shed", "pause-noncritical", "batch-sync"]
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("collab_throttle_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "collab_team_throttle_contract"
    {
        errors.push("collab_throttle_contract_kind_invalid".to_string());
    }

    let team = team_slug(
        parsed
            .flags
            .get("team")
            .map(String::as_str)
            .unwrap_or("default-team"),
    );
    let plane = clean(
        parsed
            .flags
            .get("plane")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_default(),
        120,
    );
    if plane.is_empty() {
        errors.push("collab_throttle_plane_required".to_string());
    }
    let min_depth = contract
        .get("min_depth")
        .and_then(Value::as_u64)
        .unwrap_or(1);
    let max_depth_allowed = contract
        .get("max_depth")
        .and_then(Value::as_u64)
        .unwrap_or(1_000_000);
    let max_depth = parse_u64(parsed.flags.get("max-depth"), 75);
    if strict && (max_depth < min_depth || max_depth > max_depth_allowed) {
        errors.push("collab_throttle_max_depth_out_of_range".to_string());
    }
    let strategy = clean(
        parsed
            .flags
            .get("strategy")
            .cloned()
            .unwrap_or_else(|| "priority-shed".to_string()),
        80,
    )
    .to_ascii_lowercase();
    let allowed_strategies = contract
        .get("allowed_strategies")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!("priority-shed")])
        .iter()
        .filter_map(Value::as_str)
        .map(|v| clean(v, 80).to_ascii_lowercase())
        .collect::<Vec<_>>();
    if strict && !allowed_strategies.iter().any(|v| v == &strategy) {
        errors.push("collab_throttle_strategy_invalid".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "collab_plane_throttle",
            "errors": errors
        });
    }

    let state_path = throttle_state_path(root, &team);
    let mut throttle_state = read_json(&state_path).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "team": team,
            "policies": []
        })
    });
    let mut policies = throttle_state
        .get("policies")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let policy = json!({
        "plane": plane,
        "max_depth": max_depth,
        "strategy": strategy,
        "active": true,
        "updated_at": crate::now_iso()
    });
    let mut replaced = false;
    for row in &mut policies {
        if row.get("plane").and_then(Value::as_str) == Some(plane.as_str()) {
            *row = policy.clone();
            replaced = true;
        }
    }
    if !replaced {
        policies.push(policy.clone());
    }
    throttle_state["policies"] = Value::Array(policies.clone());
    throttle_state["updated_at"] = Value::String(crate::now_iso());
    let _ = write_json(&state_path, &throttle_state);
    let _ = append_jsonl(
        &state_root(root).join("throttle").join("history.jsonl"),
        &json!({
            "type": "collab_throttle",
            "team": team,
            "plane": plane,
            "max_depth": max_depth,
            "strategy": strategy,
            "ts": crate::now_iso()
        }),
    );

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "collab_plane_throttle",
        "lane": "core/layer0/ops",
        "team": team,
        "plane": plane,
        "max_depth": max_depth,
        "strategy": strategy,
        "policy": policy,
        "state": throttle_state,
        "artifact": {
            "path": state_path.display().to_string(),
            "sha256": sha256_hex_str(&Value::Array(policies.clone()).to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-COLLAB-001.3",
                "claim": "team_scheduler_and_throttle_policies_support_deterministic_backpressure_controls",
                "evidence": {
                    "team": team,
                    "plane": plane,
                    "strategy": strategy,
                    "max_depth": max_depth
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

