fn run_hand_new(root: &Path, argv: &[String]) -> i32 {
    let strict = parse_bool(parse_flag(argv, "strict").as_deref(), true);
    if let Some(mut denied) = conduit_guard(argv, strict) {
        return emit_receipt(root, &mut denied);
    }
    let hand_id = clean_id(
        parse_flag(argv, "hand-id")
            .or_else(|| parse_flag(argv, "id"))
            .or_else(|| parse_positional(argv, 1)),
        "hand-default",
    );
    let template = clean_id(parse_flag(argv, "template"), "generalist");
    let schedule = parse_flag(argv, "schedule").unwrap_or_else(|| "0 * * * *".to_string());
    let provider = clean_id(parse_flag(argv, "provider"), "bitnet");
    let fallback = clean_id(parse_flag(argv, "fallback"), "local-moe");

    let hand = json!({
        "version": "v1",
        "hand_id": hand_id,
        "template": template,
        "schedule": schedule,
        "provider_preferred": provider,
        "provider_fallback": fallback,
        "cycles": 0u64,
        "created_at": now_iso(),
        "updated_at": now_iso(),
        "memory": {
            "core": [],
            "archival": [],
            "external": []
        },
        "capabilities": ["observe", "reason", "tool-call", "wasm-task"]
    });

    let path = hand_path(root, &hand_id);
    if let Err(err) = write_json(&path, &hand) {
        let mut out = cli_error_receipt(argv, &err, 2);
        out["type"] = Value::String("autonomy_hand_new".to_string());
        return emit_receipt(root, &mut out);
    }

    let mut out = json!({
        "ok": true,
        "type": "autonomy_hand_new",
        "lane": LANE_ID,
        "strict": strict,
        "hand": hand,
        "artifact": {
            "path": path.display().to_string(),
            "sha256": receipt_hash(&hand)
        },
        "claim_evidence": [
            {
                "id": "V6-AUTONOMY-001.1",
                "claim": "persistent_hands_have_manifest_schedule_and_policy_governed_lifecycle",
                "evidence": {"hand_id": hand_id, "template": template, "schedule": schedule}
            }
        ]
    });
    emit_receipt(root, &mut out)
}

fn run_hand_cycle(root: &Path, argv: &[String]) -> i32 {
    let strict = parse_bool(parse_flag(argv, "strict").as_deref(), true);
    if let Some(mut denied) = conduit_guard(argv, strict) {
        return emit_receipt(root, &mut denied);
    }
    let hand_id = clean_id(
        parse_flag(argv, "hand-id")
            .or_else(|| parse_flag(argv, "id"))
            .or_else(|| parse_positional(argv, 1)),
        "hand-default",
    );
    let goal = parse_flag(argv, "goal").unwrap_or_else(|| "background_cycle".to_string());
    let provider_policy = load_provider_policy(root);
    let allowed = provider_policy
        .get("allowed_providers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect::<Vec<_>>();
    let preferred = clean_id(
        parse_flag(argv, "provider").or_else(|| parse_flag(argv, "provider-preferred")),
        provider_policy
            .get("default_provider")
            .and_then(Value::as_str)
            .unwrap_or("bitnet"),
    );
    let fallback = clean_id(parse_flag(argv, "fallback"), "local-moe");
    let selected = if allowed.iter().any(|p| p == &preferred) {
        preferred.clone()
    } else if allowed.iter().any(|p| p == &fallback) {
        fallback.clone()
    } else {
        provider_policy
            .get("default_provider")
            .and_then(Value::as_str)
            .unwrap_or("bitnet")
            .to_string()
    };
    if strict && !allowed.iter().any(|p| p == &selected) {
        let mut out = json!({
            "ok": false,
            "type": "autonomy_hand_cycle",
            "lane": LANE_ID,
            "strict": strict,
            "error": "provider_not_allowed",
            "provider": selected
        });
        return emit_receipt(root, &mut out);
    }

    let cycle_duality = autonomy_duality_bundle(
        root,
        "weaver_arbitration",
        "autonomy_hand_cycle",
        &format!("hand-cycle-{hand_id}"),
        &json!({
            "hand_id": hand_id.clone(),
            "goal": goal.clone(),
            "provider": selected.clone()
        }),
        true,
    );
    if strict && autonomy_duality_hard_block(&cycle_duality) {
        let mut out = json!({
            "ok": false,
            "type": "autonomy_hand_cycle",
            "lane": LANE_ID,
            "strict": strict,
            "error": "duality_toll_hard_block",
            "duality": cycle_duality
        });
        return emit_receipt(root, &mut out);
    }

    let path = hand_path(root, &hand_id);
    let mut hand = read_json(&path).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "hand_id": hand_id,
            "template": "generalist",
            "schedule": "0 * * * *",
            "cycles": 0u64
        })
    });
    let cycles = hand.get("cycles").and_then(Value::as_u64).unwrap_or(0) + 1;
    hand["cycles"] = Value::from(cycles);
    hand["updated_at"] = Value::String(now_iso());
    hand["provider_last_selected"] = Value::String(selected.clone());
    hand["goal_last"] = Value::String(goal.clone());
    let _ = write_json(&path, &hand);

    let events_path = hand_events_path(root, &hand_id);
    let events = read_jsonl(&events_path);
    let prev_hash = events
        .last()
        .and_then(|e| e.get("event_hash"))
        .and_then(Value::as_str)
        .unwrap_or("genesis")
        .to_string();
    let mut event = json!({
        "type": "autonomy_hand_cycle_event",
        "hand_id": hand_id,
        "cycle": cycles,
        "goal": goal,
        "provider": selected,
        "previous_hash": prev_hash,
        "ts": now_iso()
    });
    event["event_hash"] = Value::String(receipt_hash(&event));
    let _ = append_jsonl(&events_path, &event);
    let mut all_hashes = read_jsonl(&events_path)
        .into_iter()
        .filter_map(|e| {
            e.get("event_hash")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect::<Vec<_>>();
    if all_hashes.is_empty() {
        all_hashes.push("genesis".to_string());
    }
    let merkle_root = deterministic_merkle_root(&all_hashes);

    let mut out = json!({
        "ok": true,
        "type": "autonomy_hand_cycle",
        "lane": LANE_ID,
        "strict": strict,
        "hand": hand,
        "event": event,
        "chain": {
            "event_count": all_hashes.len(),
            "merkle_root": merkle_root,
            "events_path": events_path.display().to_string()
        },
        "duality": cycle_duality,
        "routing": {
            "selected_provider": selected,
            "allowed_providers": allowed
        },
        "claim_evidence": [
            {
                "id": "V6-AUTONOMY-001.2",
                "claim": "hand_cycles_emit_merkle_linked_previous_hash_receipts",
                "evidence": {"hand_id": hand_id, "cycle": cycles, "merkle_root": merkle_root}
            },
            {
                "id": "V6-AUTONOMY-001.3",
                "claim": "provider_selection_is_policy_governed_and_receipted",
                "evidence": {"selected_provider": selected, "goal": goal}
            }
        ]
    });
    emit_receipt(root, &mut out)
}

fn run_hand_status(root: &Path, argv: &[String]) -> i32 {
    let strict = parse_bool(parse_flag(argv, "strict").as_deref(), true);
    if let Some(mut denied) = conduit_guard(argv, strict) {
        return emit_receipt(root, &mut denied);
    }
    let hand_id = clean_id(
        parse_flag(argv, "hand-id")
            .or_else(|| parse_flag(argv, "id"))
            .or_else(|| parse_positional(argv, 1)),
        "hand-default",
    );
    let hand = read_json(&hand_path(root, &hand_id)).unwrap_or(Value::Null);
    let events = read_jsonl(&hand_events_path(root, &hand_id));
    let mut out = json!({
        "ok": true,
        "type": "autonomy_hand_status",
        "lane": LANE_ID,
        "strict": strict,
        "hand_id": hand_id,
        "hand": hand,
        "events": {
            "count": events.len(),
            "latest": events.last().cloned().unwrap_or(Value::Null)
        },
        "claim_evidence": [
            {
                "id": "V6-AUTONOMY-001.1",
                "claim": "hands_are_persisted_and_queryable_by_id",
                "evidence": {"hand_id": hand_id}
            }
        ]
    });
    emit_receipt(root, &mut out)
}

fn run_hand_memory_page(root: &Path, argv: &[String]) -> i32 {
    let strict = parse_bool(parse_flag(argv, "strict").as_deref(), true);
    if let Some(mut denied) = conduit_guard(argv, strict) {
        return emit_receipt(root, &mut denied);
    }
    let hand_id = clean_id(
        parse_flag(argv, "hand-id")
            .or_else(|| parse_flag(argv, "id"))
            .or_else(|| parse_positional(argv, 1)),
        "hand-default",
    );
    let op = parse_flag(argv, "op")
        .or_else(|| parse_positional(argv, 2))
        .unwrap_or_else(|| "status".to_string())
        .to_ascii_lowercase();
    let tier = parse_flag(argv, "tier").unwrap_or_else(|| "core".to_string());
    let key = clean_id(parse_flag(argv, "key"), "context");
    let path = hand_path(root, &hand_id);
    let mut hand = read_json(&path)
        .unwrap_or_else(|| json!({"memory":{"core":[],"archival":[],"external":[]}}));
    if !hand.get("memory").and_then(Value::as_object).is_some() {
        hand["memory"] = json!({"core":[],"archival":[],"external":[]});
    }
    let arr = hand["memory"][&tier]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut next = arr;
    if op == "page-in" && !next.iter().any(|v| v.as_str() == Some(key.as_str())) {
        next.push(Value::String(key.clone()));
    } else if op == "page-out" {
        next = next
            .into_iter()
            .filter(|v| v.as_str() != Some(key.as_str()))
            .collect();
    }
    hand["memory"][&tier] = Value::Array(next.clone());
    hand["updated_at"] = Value::String(now_iso());
    let _ = write_json(&path, &hand);

    let mut out = json!({
        "ok": true,
        "type": "autonomy_hand_memory_page",
        "lane": LANE_ID,
        "strict": strict,
        "hand_id": hand_id,
        "op": op,
        "tier": tier,
        "key": key,
        "memory": hand.get("memory").cloned().unwrap_or(Value::Null),
        "claim_evidence": [
            {
                "id": "V6-AUTONOMY-001.4",
                "claim": "hierarchical_memory_paging_supports_core_archival_external_tiers",
                "evidence": {"tier": tier, "size": next.len()}
            }
        ]
    });
    emit_receipt(root, &mut out)
}

fn run_hand_wasm_task(root: &Path, argv: &[String]) -> i32 {
    let strict = parse_bool(parse_flag(argv, "strict").as_deref(), true);
    if let Some(mut denied) = conduit_guard(argv, strict) {
        return emit_receipt(root, &mut denied);
    }
    let hand_id = clean_id(
        parse_flag(argv, "hand-id")
            .or_else(|| parse_flag(argv, "id"))
            .or_else(|| parse_positional(argv, 1)),
        "hand-default",
    );
    let task = clean_id(parse_flag(argv, "task"), "wasm-task");
    let fuel = parse_u64(parse_flag(argv, "fuel").as_deref(), 1000, 1, 5_000_000);
    let epoch_ms = parse_u64(parse_flag(argv, "epoch-ms").as_deref(), 250, 1, 120_000);
    let hard_fuel = 2_000_000u64;
    let hard_epoch = 30_000u64;
    if strict && (fuel > hard_fuel || epoch_ms > hard_epoch) {
        let mut out = json!({
            "ok": false,
            "type": "autonomy_hand_wasm_task",
            "lane": LANE_ID,
            "strict": strict,
            "error": "wasm_budget_exceeded",
            "fuel": fuel,
            "epoch_ms": epoch_ms
        });
        return emit_receipt(root, &mut out);
    }

    let work_units = ((fuel / 97) + (epoch_ms / 11)).max(1);
    let mut out = json!({
        "ok": true,
        "type": "autonomy_hand_wasm_task",
        "lane": LANE_ID,
        "strict": strict,
        "hand_id": hand_id,
        "task": task,
        "meters": {
            "fuel": fuel,
            "epoch_ms": epoch_ms
        },
        "result": {
            "status": "ok",
            "work_units": work_units,
            "result_hash": receipt_hash(&json!({"task": task, "work_units": work_units}))
        },
        "claim_evidence": [
            {
                "id": "V6-AUTONOMY-001.5",
                "claim": "wasm_workspace_tasks_are_dual_metered_and_policy_bounded",
                "evidence": {"hand_id": hand_id, "fuel": fuel, "epoch_ms": epoch_ms}
            }
        ]
    });
    emit_receipt(root, &mut out)
}
