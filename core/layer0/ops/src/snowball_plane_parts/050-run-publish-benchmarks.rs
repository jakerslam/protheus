fn run_publish_benchmarks(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let mut cycles = load_cycles(root);
    let cycle_id = active_or_requested_cycle(parsed, &cycles, "snowball-default");
    let mut cycles_map = cycles
        .get("cycles")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let cycle = load_cycle_value(&cycles, &cycle_id);
    if strict && cycle.is_none() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "snowball_plane_publish_benchmarks",
            "action": "publish-benchmarks",
            "errors": ["snowball_cycle_not_found"],
            "cycle_id": cycle_id
        });
    }
    let benchmark_path = benchmark_report_path(root, parsed);
    let report = read_json(&benchmark_path).unwrap_or(Value::Null);
    let benchmark_after = benchmark_modes_from_report(&report);
    let benchmark_before = cycle
        .as_ref()
        .and_then(|v| v.get("benchmark_before"))
        .cloned()
        .unwrap_or_else(|| benchmark_after.clone());
    let delta = benchmark_delta(&benchmark_before, &benchmark_after);
    let readme_path = readme_path(root, parsed);
    let readme_text = fs::read_to_string(&readme_path).unwrap_or_default();
    let sync = readme_sync_summary(&report, &readme_text);
    let synced = sync.get("synced").and_then(Value::as_bool).unwrap_or(false);
    let publication_path = benchmark_publication_path(root, &cycle_id);
    let summary = json!({
        "version": "v1",
        "cycle_id": cycle_id,
        "generated_at": crate::now_iso(),
        "benchmark_report_path": benchmark_path.display().to_string(),
        "readme_path": readme_path.display().to_string(),
        "delta": delta,
        "readme_sync": sync
    });
    let _ = write_json(&publication_path, &summary);

    let mut next_cycle =
        cycle.unwrap_or_else(|| json!({"cycle_id": cycle_id, "stage":"published"}));
    next_cycle["benchmark_publication"] = json!({
        "path": publication_path.display().to_string(),
        "readme_synced": synced
    });
    next_cycle["updated_at"] = Value::String(crate::now_iso());
    cycles_map.insert(cycle_id.clone(), next_cycle);
    cycles["cycles"] = Value::Object(cycles_map);
    cycles["active_cycle_id"] = Value::String(cycle_id.clone());
    cycles["updated_at"] = Value::String(crate::now_iso());
    store_cycles(root, &cycles);

    let ok = if strict { synced } else { true };
    let mut out = json!({
        "ok": ok,
        "strict": strict,
        "type": "snowball_plane_publish_benchmarks",
        "lane": "core/layer0/ops",
        "action": "publish-benchmarks",
        "cycle_id": cycle_id,
        "publication": summary,
        "claim_evidence": [
            {
                "id": "V6-APP-023.10",
                "claim": "snowball_benchmark_publication_emits_receipted_deltas_and_fails_closed_when_readme_evidence_is_stale",
                "evidence": {
                    "cycle_id": cycle_id,
                    "publication_path": publication_path.display().to_string(),
                    "readme_synced": synced
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_promote(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let mut cycles = load_cycles(root);
    let cycle_id = active_or_requested_cycle(parsed, &cycles, "snowball-default");
    let mut cycles_map = cycles
        .get("cycles")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let cycle = load_cycle_value(&cycles, &cycle_id);
    if strict && cycle.is_none() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "snowball_plane_promote",
            "action": "promote",
            "errors": ["snowball_cycle_not_found"],
            "cycle_id": cycle_id
        });
    }
    let review = load_review(root, &cycle_id).unwrap_or_else(|| json!({}));
    let publication =
        read_json(&benchmark_publication_path(root, &cycle_id)).unwrap_or_else(|| json!({}));
    let regression_pass = cycle
        .as_ref()
        .and_then(|v| v.pointer("/melt_refine/pass"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let improved = review
        .pointer("/summary/improved_metric_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0;
    let regressed = review
        .pointer("/summary/regressed_metric_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0;
    let allow_neutral = parse_bool(parsed.flags.get("allow-neutral"), false);
    let neutral_justification = clean(
        parsed
            .flags
            .get("neutral-justification")
            .cloned()
            .unwrap_or_default(),
        240,
    );
    let neutral_ok = allow_neutral && !neutral_justification.is_empty() && !regressed;
    let publication_ok = publication
        .pointer("/readme_sync/synced")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let promoted = regression_pass && publication_ok && (improved || neutral_ok);
    let rollback_pointer = json!({
        "cycles_path": cycles_path(root).display().to_string(),
        "cycle_id": cycle_id,
        "snapshot_path": cycle
            .as_ref()
            .and_then(|v| v.pointer("/snapshot/path"))
            .and_then(Value::as_str)
            .unwrap_or("")
    });
    let survivors = review
        .get("survivors")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let promotion = json!({
        "version": "v1",
        "cycle_id": cycle_id,
        "generated_at": crate::now_iso(),
        "promoted": promoted,
        "regression_pass": regression_pass,
        "publication_ok": publication_ok,
        "improved": improved,
        "neutral_ok": neutral_ok,
        "neutral_justification": neutral_justification,
        "survivors": survivors,
        "rollback_pointer": rollback_pointer
    });
    let promotion_out = promotion_path(root, &cycle_id);
    let _ = write_json(&promotion_out, &promotion);

    let mut next_cycle = cycle.unwrap_or_else(|| json!({"cycle_id": cycle_id, "stage":"running"}));
    next_cycle["promotion"] = json!({
        "path": promotion_out.display().to_string(),
        "promoted": promoted
    });
    next_cycle["stage"] = Value::String(if promoted {
        "promoted".to_string()
    } else {
        "rollback".to_string()
    });
    next_cycle["updated_at"] = Value::String(crate::now_iso());
    cycles_map.insert(cycle_id.clone(), next_cycle.clone());
    cycles["cycles"] = Value::Object(cycles_map);
    cycles["active_cycle_id"] = Value::String(cycle_id.clone());
    cycles["updated_at"] = Value::String(crate::now_iso());
    store_cycles(root, &cycles);

    let ok = if strict { promoted } else { true };
    let mut out = json!({
        "ok": ok,
        "strict": strict,
        "type": "snowball_plane_promote",
        "lane": "core/layer0/ops",
        "action": "promote",
        "cycle_id": cycle_id,
        "promotion": promotion,
        "claim_evidence": [
            {
                "id": "V6-APP-023.8",
                "claim": "snowball_promotion_requires_regression_and_benchmark_publication_evidence_before_advancing_active_state",
                "evidence": {
                    "cycle_id": cycle_id,
                    "promoted": promoted,
                    "publication_ok": publication_ok,
                    "regression_pass": regression_pass,
                    "rollback_pointer": rollback_pointer
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_prime_update(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let mut cycles = load_cycles(root);
    let cycle_id = active_or_requested_cycle(parsed, &cycles, "snowball-default");
    let mut cycles_map = cycles
        .get("cycles")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let cycle = load_cycle_value(&cycles, &cycle_id);
    let promotion = read_json(&promotion_path(root, &cycle_id)).unwrap_or_else(|| json!({}));
    if strict && promotion.get("promoted").and_then(Value::as_bool) != Some(true) {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "snowball_plane_prime_update",
            "action": "prime-update",
            "errors": ["snowball_promotion_not_ready"],
            "cycle_id": cycle_id
        });
    }
    let archive_path = discarded_blob_index_path(root, &cycle_id);
    let publication_path = benchmark_publication_path(root, &cycle_id);
    if strict && !archive_path.exists() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "snowball_plane_prime_update",
            "action": "prime-update",
            "errors": ["snowball_discarded_archive_missing"],
            "cycle_id": cycle_id
        });
    }
    if strict && !publication_path.exists() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "snowball_plane_prime_update",
            "action": "prime-update",
            "errors": ["snowball_benchmark_publication_missing"],
            "cycle_id": cycle_id
        });
    }
    let archive = read_json(&archive_path).unwrap_or_else(|| json!({}));
    let directive_text = parsed.flags.get("directive").cloned().unwrap_or_default();
    let signer = clean(
        parsed
            .flags
            .get("signer")
            .cloned()
            .unwrap_or_else(|| "snowball-plane".to_string()),
        64,
    );
    let directive_result = if directive_text.trim().is_empty() {
        Ok(None)
    } else {
        directive_kernel::append_compaction_directive_entry(
            root,
            directive_text.as_str(),
            signer.as_str(),
            None,
            "snowball_compaction",
        )
        .map(Some)
    };
    if strict && directive_result.is_err() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "snowball_plane_prime_update",
            "action": "prime-update",
            "errors": [directive_result.err().unwrap_or_else(|| "directive_append_failed".to_string())],
            "cycle_id": cycle_id
        });
    }
    let directive_entry = directive_result.ok().flatten();
    let prime_state = json!({
        "version": "v1",
        "cycle_id": cycle_id,
        "updated_at": crate::now_iso(),
        "promotion_path": promotion_path(root, &cycle_id).display().to_string(),
        "benchmark_publication_path": publication_path.display().to_string(),
        "discarded_blob_index_path": archive_path.display().to_string(),
        "promoted_survivors": promotion.get("survivors").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "discarded_artifacts": archive.get("items").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "directive_delta": directive_entry.clone().unwrap_or_else(|| json!({"applied": false})),
        "active_state_delta": {
            "previous_stage": cycle
                .as_ref()
                .and_then(|v| v.get("stage"))
                .cloned()
                .unwrap_or(Value::String("unknown".to_string())),
            "next_stage": "prime-updated"
        }
    });
    let prime_path = prime_directive_compacted_state_path(root);
    let _ = write_json(&prime_path, &prime_state);

    let mut next_cycle = cycle.unwrap_or_else(|| json!({"cycle_id": cycle_id, "stage":"promoted"}));
    next_cycle["prime_directive_update"] = json!({
        "path": prime_path.display().to_string(),
        "directive_delta_applied": directive_entry.is_some()
    });
    next_cycle["stage"] = Value::String("prime-updated".to_string());
    next_cycle["updated_at"] = Value::String(crate::now_iso());
    cycles_map.insert(cycle_id.clone(), next_cycle.clone());
    cycles["cycles"] = Value::Object(cycles_map);
    cycles["active_cycle_id"] = Value::String(cycle_id.clone());
    cycles["updated_at"] = Value::String(crate::now_iso());
    store_cycles(root, &cycles);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "snowball_plane_prime_update",
        "lane": "core/layer0/ops",
        "action": "prime-update",
        "cycle_id": cycle_id,
        "prime_directive_state": prime_state,
        "claim_evidence": [
            {
                "id": "V6-APP-023.11",
                "claim": "snowball_prime_update_records_promoted_survivors_discarded_artifacts_and_directive_deltas_through_prime_governance",
                "evidence": {
                    "cycle_id": cycle_id,
                    "prime_state_path": prime_path.display().to_string(),
                    "directive_delta_applied": directive_entry.is_some()
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_backlog_pack(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
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
            "type": "snowball_plane_backlog_pack",
            "action": "backlog-pack",
            "errors": ["snowball_cycle_not_found"],
            "cycle_id": cycle_id
        });
    }
    let unresolved = parse_json_flag(parsed.flags.get("unresolved-json"))
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_else(|| {
            let mut defaults = vec![json!({
                "id":"verify-regression",
                "depends_on": [],
                "priority": 1
            })];
            if cycle
                .as_ref()
                .and_then(|v| v.get("melt_refine"))
                .and_then(|v| v.get("pass"))
                .and_then(Value::as_bool)
                == Some(false)
            {
                defaults.push(json!({
                    "id":"rollback-analysis",
                    "depends_on": ["verify-regression"],
                    "priority": 0
                }));
            }
            defaults
        });
    let ordered = dependency_ordered_backlog(unresolved);
    let backlog = json!({
        "version":"v1",
        "cycle_id": cycle_id,
        "generated_at": crate::now_iso(),
        "items": ordered
    });
    let out_path = backlog_path(root, &cycle_id);
    let _ = write_json(&out_path, &backlog);

    let mut next_cycle = cycle.unwrap_or_else(|| json!({"cycle_id": cycle_id, "stage":"running"}));
    next_cycle["next_backlog"] = json!({
        "path": out_path.display().to_string(),
        "count": backlog.get("items").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0)
    });
    next_cycle["updated_at"] = Value::String(crate::now_iso());
    cycles_map.insert(cycle_id.clone(), next_cycle.clone());
    cycles["cycles"] = Value::Object(cycles_map);
    cycles["active_cycle_id"] = Value::String(cycle_id.clone());
    cycles["updated_at"] = Value::String(crate::now_iso());
    store_cycles(root, &cycles);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "snowball_plane_backlog_pack",
        "lane": "core/layer0/ops",
        "action": "backlog-pack",
        "cycle_id": cycle_id,
        "backlog": backlog,
        "artifact": {
            "path": out_path.display().to_string(),
            "sha256": sha256_hex_str(&read_json(&out_path).unwrap_or(Value::Null).to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-APP-023.4",
                "claim": "snowball_backlog_pack_generates_dependency_ordered_next_cycle_items_from_unresolved_findings",
                "evidence": {
                    "cycle_id": cycle_id
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
                    "queued_items": backlog.get("items").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0)
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

