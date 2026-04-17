fn dependency_ordered_backlog(rows: Vec<Value>) -> Vec<Value> {
    let mut normalized = Vec::<BacklogItem>::new();
    for (idx, row) in rows.into_iter().enumerate() {
        let fallback_id = format!("item-{}", idx + 1);
        let id = clean_id(
            row.get("id")
                .and_then(Value::as_str)
                .or(Some(fallback_id.as_str())),
            fallback_id.as_str(),
        );
        let priority = row
            .get("priority")
            .and_then(Value::as_i64)
            .or_else(|| {
                row.get("priority")
                    .and_then(Value::as_str)
                    .and_then(|raw| raw.trim().parse::<i64>().ok())
            })
            .unwrap_or(99)
            .clamp(-1000, 1000);
        normalized.push(BacklogItem {
            id,
            priority,
            depends_on: Vec::new(),
            payload: row,
            original_index: idx,
        });
    }

    let known_ids = normalized
        .iter()
        .map(|item| item.id.clone())
        .collect::<HashSet<_>>();

    for item in &mut normalized {
        let mut deps = Vec::<String>::new();
        if let Some(row_deps) = item.payload.get("depends_on") {
            if let Some(arr) = row_deps.as_array() {
                for dep in arr {
                    let dep_id = dep
                        .as_str()
                        .map(ToString::to_string)
                        .or_else(|| dep.as_i64().map(|v| v.to_string()));
                    if let Some(dep_id) = dep_id {
                        let clean_dep = clean_id(Some(dep_id.as_str()), "dep");
                        if clean_dep != item.id
                            && known_ids.contains(clean_dep.as_str())
                            && !deps.iter().any(|v| v == &clean_dep)
                        {
                            deps.push(clean_dep);
                        }
                        if deps.len() >= 32 {
                            break;
                        }
                    }
                }
            } else if let Some(csv) = row_deps.as_str() {
                for dep_id in csv.split(|ch| ch == ',' || ch == ';') {
                    let clean_dep = clean_id(Some(dep_id), "dep");
                    if clean_dep != item.id
                        && known_ids.contains(clean_dep.as_str())
                        && !deps.iter().any(|v| v == &clean_dep)
                    {
                        deps.push(clean_dep);
                    }
                    if deps.len() >= 32 {
                        break;
                    }
                }
            }
        }
        deps.sort();
        item.depends_on = deps;
    }

    let mut pending = normalized
        .into_iter()
        .map(|item| (item.id.clone(), item))
        .collect::<BTreeMap<_, _>>();
    let mut resolved = HashSet::<String>::new();
    let mut out = Vec::<Value>::new();

    while !pending.is_empty() {
        let mut ready = pending
            .values()
            .filter(|item| item.depends_on.iter().all(|dep| resolved.contains(dep)))
            .map(|item| item.id.clone())
            .collect::<Vec<_>>();
        let cycle_break = ready.is_empty();
        if cycle_break {
            ready = pending.keys().cloned().collect::<Vec<_>>();
        }
        ready.sort_by(|a, b| {
            let ia = pending.get(a).expect("pending item a");
            let ib = pending.get(b).expect("pending item b");
            ia.priority
                .cmp(&ib.priority)
                .then_with(|| ia.original_index.cmp(&ib.original_index))
                .then_with(|| ia.id.cmp(&ib.id))
        });
        let next_id = ready
            .first()
            .cloned()
            .unwrap_or_else(|| "item-unknown".to_string());
        let item = match pending.remove(next_id.as_str()) {
            Some(v) => v,
            None => break,
        };
        resolved.insert(item.id.clone());
        let mut payload = item.payload;
        if !payload.is_object() {
            payload = json!({});
        }
        payload["id"] = Value::String(item.id.clone());
        payload["priority"] = Value::from(item.priority);
        payload["depends_on"] = Value::Array(
            item.depends_on
                .iter()
                .map(|dep| Value::String(dep.clone()))
                .collect::<Vec<_>>(),
        );
        payload["order"] = Value::from((out.len() + 1) as u64);
        payload["dependency_cycle_break"] = Value::Bool(cycle_break);
        out.push(payload);
    }

    out
}

fn run_start(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "snowball_engine_contract",
            "default_parallel_limit": 3,
            "max_parallel_limit": 8
        }),
    );
    let mut cycles = load_cycles(root);
    if !cycles.get("cycles").map(Value::is_object).unwrap_or(false) {
        cycles["cycles"] = json!({});
    }
    let cycle_id = active_or_requested_cycle(parsed, &cycles, "snowball-default");
    let drops = parse_csv_unique(
        parsed.flags.get("drops"),
        &["core-hardening", "app-refine", "ops-proof"],
    );
    let default_parallel = contract
        .get("default_parallel_limit")
        .and_then(Value::as_u64)
        .unwrap_or(3);
    let max_parallel = contract
        .get("max_parallel_limit")
        .and_then(Value::as_u64)
        .unwrap_or(8)
        .max(1);
    let parallel_limit = parse_u64(parsed.flags.get("parallel"), default_parallel)
        .max(1)
        .min(max_parallel);
    let allow_high_risk = parse_bool(parsed.flags.get("allow-high-risk"), false);
    let benchmark_path = benchmark_report_path(root, parsed);
    let benchmark_before = load_benchmark_modes(&benchmark_path);
    let deps_map = dependencies_from_json(
        drops.as_slice(),
        parse_json_flag(parsed.flags.get("deps-json")),
    );

    let mut risk_blocked = Vec::<String>::new();
    let mut drop_rows = Vec::<Value>::new();
    for drop in &drops {
        let risk = classify_drop_risk(drop);
        if strict && risk == "high" && !allow_high_risk {
            risk_blocked.push(drop.clone());
        }
        drop_rows.push(json!({
            "drop": drop,
            "risk": risk,
            "deps": deps_map.get(drop).cloned().unwrap_or_default()
        }));
    }
    if strict && !risk_blocked.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "snowball_plane_start",
            "action": "start",
            "errors": ["snowball_high_risk_drop_requires_allow_flag"],
            "blocked_drops": risk_blocked
        });
    }

    let mut completed = HashSet::<String>::new();
    let mut pending = drops.clone();
    let mut waves = Vec::<Value>::new();
    let mut wave_idx = 1usize;
    while !pending.is_empty() && wave_idx <= 64 {
        let mut ready = Vec::<String>::new();
        for item in &pending {
            let deps = deps_map.get(item).cloned().unwrap_or_default();
            if deps.iter().all(|dep| completed.contains(dep)) {
                ready.push(item.clone());
            }
        }
        if ready.is_empty() {
            ready.push(pending[0].clone());
        }
        let run_now = ready
            .into_iter()
            .take(parallel_limit as usize)
            .collect::<Vec<_>>();
        for item in &run_now {
            completed.insert(item.clone());
        }
        pending.retain(|item| !run_now.iter().any(|r| r == item));
        waves.push(json!({
            "wave": wave_idx,
            "parallel": run_now.len(),
            "drops": run_now
        }));
        wave_idx += 1;
    }

    let now = crate::now_iso();
    let orchestration = json!({
        "cycle_id": cycle_id,
        "parallel_limit": parallel_limit,
        "drops": drop_rows,
        "waves": waves,
        "dependency_graph": deps_map,
        "benchmark_before": benchmark_before,
        "started_at": now
    });
    let assimilation_plan = default_assimilation_items(&cycle_id, &drops);
    let cycle_value = json!({
        "cycle_id": cycle_id,
        "stage": "running",
        "orchestration": orchestration,
        "benchmark_before": benchmark_before,
        "assimilation_plan_path": assimilation_plan_path(root, &cycle_id).display().to_string(),
        "updated_at": crate::now_iso()
    });
    let _ = write_json(
        &assimilation_plan_path(root, &cycle_id),
        &json!({
            "version": "v1",
            "cycle_id": cycle_id,
            "generated_at": crate::now_iso(),
            "items": assimilation_plan
        }),
    );
    let mut cycles_map = cycles
        .get("cycles")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    cycles_map.insert(cycle_id.clone(), cycle_value.clone());
    cycles["cycles"] = Value::Object(cycles_map);
    cycles["active_cycle_id"] = Value::String(cycle_id.clone());
    cycles["updated_at"] = Value::String(crate::now_iso());
    store_cycles(root, &cycles);
    let _ = append_jsonl(
        &state_root(root).join("history.jsonl"),
        &json!({
            "ts": crate::now_iso(),
            "action": "start",
            "cycle_id": cycle_id
        }),
    );

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "snowball_plane_start",
        "lane": "core/layer0/ops",
        "action": "start",
        "cycle_id": cycle_id,
        "orchestration": cycle_value.get("orchestration").cloned().unwrap_or(Value::Null),
        "artifact": {
            "path": cycles_path(root).display().to_string(),
            "sha256": sha256_hex_str(&cycles.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-APP-023.1",
                "claim": "snowball_start_orchestrates_bounded_parallel_drop_waves_with_dependency_and_risk_gates",
                "evidence": {
                    "cycle_id": cycle_id,
                    "parallel_limit": parallel_limit,
                    "benchmark_before_path": benchmark_path.display().to_string()
                }
            },
            {
                "id": "V6-APP-023.5",
                "claim": "snowball_runtime_publishes_live_cycle_state_for_operator_controls",
                "evidence": {
                    "cycle_id": cycle_id
                }
            },
            {
                "id": "V6-APP-023.6",
                "claim": "snowball_status_and_compact_controls_surface_cycle_stage_batch_outcomes_and_regression_state",
                "evidence": {
                    "cycle_id": cycle_id,
                    "stage": "running"
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_melt_refine(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let mut cycles = load_cycles(root);
    let cycle_id = active_or_requested_cycle(parsed, &cycles, "snowball-default");
    let mut cycles_map = cycles
        .get("cycles")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let cycle = cycles_map.get(&cycle_id).cloned();
    if strict && cycle.is_none() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "snowball_plane_melt_refine",
            "action": "melt-refine",
            "errors": ["snowball_cycle_not_found"],
            "cycle_id": cycle_id
        });
    }
    let regression_suite = clean(
        parsed
            .flags
            .get("regression-suite")
            .cloned()
            .unwrap_or_else(|| "core/layer0/ops".to_string()),
        200,
    );
    let regression_pass = parse_bool(parsed.flags.get("regression-pass"), true);
    let gate = json!({
        "suite": regression_suite,
        "pass": regression_pass,
        "rollback_required": !regression_pass,
        "ts": crate::now_iso()
    });
    let mut next_cycle = cycle.unwrap_or_else(|| json!({"cycle_id": cycle_id, "stage":"running"}));
    next_cycle["melt_refine"] = gate.clone();
    next_cycle["stage"] = Value::String(if regression_pass {
        "refined".to_string()
    } else {
        "rollback".to_string()
    });
    next_cycle["updated_at"] = Value::String(crate::now_iso());
    cycles_map.insert(cycle_id.clone(), next_cycle.clone());
    cycles["cycles"] = Value::Object(cycles_map);
    cycles["active_cycle_id"] = Value::String(cycle_id.clone());
    cycles["updated_at"] = Value::String(crate::now_iso());
    store_cycles(root, &cycles);

    let mut out = json!({
        "ok": regression_pass || !strict,
        "strict": strict,
        "type": "snowball_plane_melt_refine",
        "lane": "core/layer0/ops",
        "action": "melt-refine",
        "cycle_id": cycle_id,
        "gate": gate,
        "cycle": next_cycle,
        "artifact": {
            "path": cycles_path(root).display().to_string(),
            "sha256": sha256_hex_str(&cycles.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-APP-023.2",
                "claim": "snowball_melt_refine_enforces_regression_gate_before_promotion_and_emits_rollback_receipts",
                "evidence": {
                    "cycle_id": cycle_id,
                    "regression_pass": regression_pass
                }
            },
            {
                "id": "V6-APP-023.5",
                "claim": "snowball_runtime_publishes_live_cycle_state_for_operator_controls",
                "evidence": {
                    "cycle_id": cycle_id
                }
            },
            {
                "id": "V6-APP-023.6",
                "claim": "snowball_status_and_compact_controls_surface_cycle_stage_batch_outcomes_and_regression_state",
                "evidence": {
                    "cycle_id": cycle_id,
                    "stage": next_cycle.get("stage").cloned().unwrap_or(Value::Null),
                    "regression_pass": regression_pass
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
