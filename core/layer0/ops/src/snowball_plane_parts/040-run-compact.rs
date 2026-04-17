fn run_compact(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
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
            "type": "snowball_plane_compact",
            "action": "compact",
            "errors": ["snowball_cycle_not_found"],
            "cycle_id": cycle_id
        });
    }
    let stage = cycle
        .as_ref()
        .and_then(|v| v.get("stage"))
        .and_then(Value::as_str)
        .unwrap_or("running");
    let ts = crate::now_iso();
    let snapshot = json!({
        "version": "v1",
        "cycle_id": cycle_id,
        "stage": stage,
        "sphere_of_ice": true,
        "captured_at": ts,
        "restore_pointer": {
            "cycles_path": cycles_path(root).display().to_string(),
            "cycle_id": cycle_id
        }
    });
    let snapshot_path =
        snapshot_dir(root, &cycle_id).join(format!("sphere_of_ice_{}.json", ts.replace(':', "-")));
    if let Err(err) = write_json(&snapshot_path, &snapshot) {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "snowball_plane_compact",
            "action": "compact",
            "errors": [format!("snapshot_write_failed:{err}")],
            "cycle_id": cycle_id,
            "snapshot_path": snapshot_path.display().to_string()
        });
    }
    let snapshot_hash =
        sha256_hex_str(&read_json(&snapshot_path).unwrap_or(Value::Null).to_string());

    let benchmark_path = benchmark_report_path(root, parsed);
    let benchmark_after = load_benchmark_modes(&benchmark_path);
    let benchmark_before = cycle
        .as_ref()
        .and_then(|v| v.get("benchmark_before"))
        .cloned()
        .unwrap_or_else(|| benchmark_after.clone());
    let bench_delta = benchmark_delta(&benchmark_before, &benchmark_after);
    let reliability_before = parse_f64(parsed.flags.get("reliability-before"), 1.0);
    let reliability_after = parse_f64(parsed.flags.get("reliability-after"), reliability_before);
    let reliability_gate_pass = reliability_after >= reliability_before;

    let assimilation_plan = load_assimilation_plan(root, &cycle_id, parsed);
    let review = build_fitness_review(
        root,
        &cycle_id,
        &bench_delta,
        reliability_before,
        reliability_after,
        assimilation_plan.as_slice(),
    );
    let kept = review
        .get("survivors")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut discarded = review
        .get("demoted")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    discarded.extend(
        review
            .get("rejected")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
    );
    if let Err(err) = write_json(&fitness_review_path(root, &cycle_id), &review) {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "snowball_plane_compact",
            "action": "compact",
            "errors": [format!("fitness_review_write_failed:{err}")],
            "cycle_id": cycle_id
        });
    }
    if let Err(err) = write_json(
        &kept_path(root, &cycle_id),
        &json!({
            "version": "v1",
            "cycle_id": cycle_id,
            "generated_at": crate::now_iso(),
            "items": kept
        }),
    ) {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "snowball_plane_compact",
            "action": "compact",
            "errors": [format!("kept_manifest_write_failed:{err}")],
            "cycle_id": cycle_id
        });
    }
    if let Err(err) = write_json(
        &discarded_path(root, &cycle_id),
        &json!({
            "version": "v1",
            "cycle_id": cycle_id,
            "generated_at": crate::now_iso(),
            "items": discarded
        }),
    ) {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "snowball_plane_compact",
            "action": "compact",
            "errors": [format!("discarded_manifest_write_failed:{err}")],
            "cycle_id": cycle_id
        });
    }
    let (discarded_blob_rows, discarded_blob_index) =
        archive_discarded_blobs(root, &cycle_id, &discarded);
    let prime_state = json!({
        "version": "v1",
        "cycle_id": cycle_id,
        "updated_at": crate::now_iso(),
        "benchmarks": {
            "before": benchmark_before,
            "after": benchmark_after,
            "delta": bench_delta
        },
        "assimilation": {
            "kept_count": kept.len(),
            "discarded_count": discarded.len(),
            "discarded_blob_index_path": discarded_blob_index_path(root, &cycle_id).display().to_string(),
            "fitness_review_path": fitness_review_path(root, &cycle_id).display().to_string()
        },
        "reliability": {
            "before": reliability_before,
            "after": reliability_after,
            "pass": reliability_gate_pass
        }
    });
    if let Err(err) = write_json(&prime_directive_compacted_state_path(root), &prime_state) {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "snowball_plane_compact",
            "action": "compact",
            "errors": [format!("prime_directive_compacted_state_write_failed:{err}")],
            "cycle_id": cycle_id
        });
    }

    let mut next_cycle = cycle.unwrap_or_else(|| json!({"cycle_id": cycle_id, "stage":"running"}));
    next_cycle["snapshot"] = json!({
        "path": snapshot_path.display().to_string(),
        "sha256": snapshot_hash,
        "captured_at": ts
    });
    next_cycle["stage"] = Value::String("compacted".to_string());
    next_cycle["benchmark_after"] = benchmark_after.clone();
    next_cycle["benchmark_delta"] = bench_delta.clone();
    next_cycle["fitness_review"] = json!({
        "path": fitness_review_path(root, &cycle_id).display().to_string(),
        "survivor_count": review.pointer("/summary/survivor_count").and_then(Value::as_u64).unwrap_or(0),
        "demoted_count": review.pointer("/summary/demoted_count").and_then(Value::as_u64).unwrap_or(0),
        "rejected_count": review.pointer("/summary/rejected_count").and_then(Value::as_u64).unwrap_or(0)
    });
    next_cycle["assimilation"] = json!({
        "kept_path": kept_path(root, &cycle_id).display().to_string(),
        "discarded_path": discarded_path(root, &cycle_id).display().to_string(),
        "kept_count": kept.len(),
        "discarded_count": discarded.len(),
        "discarded_blob_index_path": discarded_blob_index_path(root, &cycle_id).display().to_string()
    });
    next_cycle["prime_directive_compacted_state_path"] = Value::String(
        prime_directive_compacted_state_path(root)
            .display()
            .to_string(),
    );
    next_cycle["updated_at"] = Value::String(crate::now_iso());
    cycles_map.insert(cycle_id.clone(), next_cycle.clone());
    cycles["cycles"] = Value::Object(cycles_map);
    cycles["active_cycle_id"] = Value::String(cycle_id.clone());
    cycles["updated_at"] = Value::String(crate::now_iso());
    store_cycles(root, &cycles);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "snowball_plane_compact",
        "lane": "core/layer0/ops",
        "action": "compact",
        "cycle_id": cycle_id,
        "snapshot": next_cycle.get("snapshot").cloned().unwrap_or(Value::Null),
        "benchmark_delta": bench_delta,
        "assimilation": {
            "kept_count": kept.len(),
            "discarded_count": discarded.len(),
            "discarded_blob_count": discarded_blob_rows.len(),
            "kept_path": kept_path(root, &cycle_id).display().to_string(),
            "discarded_path": discarded_path(root, &cycle_id).display().to_string(),
            "fitness_review_path": fitness_review_path(root, &cycle_id).display().to_string(),
            "discarded_blob_index": discarded_blob_index
        },
        "prime_directive_compacted_state": {
            "path": prime_directive_compacted_state_path(root).display().to_string(),
            "sha256": sha256_hex_str(&read_json(&prime_directive_compacted_state_path(root)).unwrap_or(Value::Null).to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-APP-023.3",
                "claim": "snowball_compaction_writes_versioned_sphere_of_ice_snapshots_with_restore_pointers",
                "evidence": {
                    "cycle_id": cycle_id,
                    "snapshot_path": snapshot_path.display().to_string()
                }
            },
            {
                "id": "V6-APP-023.7",
                "claim": "snowball_compaction_scores_assimilations_against_runtime_metrics_reliability_tiny_modes_and_rsi_utility",
                "evidence": {
                    "cycle_id": cycle_id,
                    "kept_count": kept.len(),
                    "discarded_count": discarded.len(),
                    "fitness_review_path": fitness_review_path(root, &cycle_id).display().to_string(),
                    "benchmark_report_path": benchmark_path.display().to_string()
                }
            },
            {
                "id": "V6-APP-023.9",
                "claim": "snowball_compaction_archives_discarded_or_demoted_ideas_as_versioned_blob_artifacts_with_provenance",
                "evidence": {
                    "cycle_id": cycle_id,
                    "discarded_blob_count": discarded_blob_rows.len(),
                    "discarded_blob_index_path": discarded_blob_index_path(root, &cycle_id).display().to_string()
                }
            },
            {
                "id": "V6-APP-023.11",
                "claim": "snowball_compaction_records_compacted_state_and_prime_directive_lineage_for_successful_cycles",
                "evidence": {
                    "cycle_id": cycle_id,
                    "prime_state_path": prime_directive_compacted_state_path(root).display().to_string()
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
                    "snapshot_path": snapshot_path.display().to_string()
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_fitness_review(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
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
            "type": "snowball_plane_fitness_review",
            "action": "fitness-review",
            "errors": ["snowball_cycle_not_found"],
            "cycle_id": cycle_id
        });
    }
    let benchmark_path = benchmark_report_path(root, parsed);
    let benchmark_after = load_benchmark_modes(&benchmark_path);
    let benchmark_before = cycle
        .as_ref()
        .and_then(|v| v.get("benchmark_before"))
        .cloned()
        .unwrap_or_else(|| benchmark_after.clone());
    let bench_delta = benchmark_delta(&benchmark_before, &benchmark_after);
    let reliability_before = parse_f64(parsed.flags.get("reliability-before"), 1.0);
    let reliability_after = parse_f64(parsed.flags.get("reliability-after"), reliability_before);
    let review = build_fitness_review(
        root,
        &cycle_id,
        &bench_delta,
        reliability_before,
        reliability_after,
        load_assimilation_plan(root, &cycle_id, parsed).as_slice(),
    );
    let review_path = fitness_review_path(root, &cycle_id);
    let _ = write_json(&review_path, &review);

    let mut next_cycle = cycle.unwrap_or_else(|| json!({"cycle_id": cycle_id, "stage":"reviewed"}));
    next_cycle["fitness_review"] = json!({
        "path": review_path.display().to_string(),
        "survivor_count": review.pointer("/summary/survivor_count").and_then(Value::as_u64).unwrap_or(0),
        "demoted_count": review.pointer("/summary/demoted_count").and_then(Value::as_u64).unwrap_or(0),
        "rejected_count": review.pointer("/summary/rejected_count").and_then(Value::as_u64).unwrap_or(0)
    });
    next_cycle["benchmark_after"] = benchmark_after;
    next_cycle["benchmark_delta"] = bench_delta.clone();
    next_cycle["updated_at"] = Value::String(crate::now_iso());
    cycles_map.insert(cycle_id.clone(), next_cycle.clone());
    cycles["cycles"] = Value::Object(cycles_map);
    cycles["active_cycle_id"] = Value::String(cycle_id.clone());
    cycles["updated_at"] = Value::String(crate::now_iso());
    store_cycles(root, &cycles);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "snowball_plane_fitness_review",
        "lane": "core/layer0/ops",
        "action": "fitness-review",
        "cycle_id": cycle_id,
        "review": review,
        "artifact": {
            "path": review_path.display().to_string(),
            "sha256": sha256_hex_str(&read_json(&review_path).unwrap_or(Value::Null).to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-APP-023.7",
                "claim": "snowball_fitness_review_scores_assimilations_against_metrics_reliability_tiny_modes_and_rsi_utility",
                "evidence": {
                    "cycle_id": cycle_id,
                    "survivor_count": next_cycle.pointer("/fitness_review/survivor_count").and_then(Value::as_u64).unwrap_or(0),
                    "demoted_count": next_cycle.pointer("/fitness_review/demoted_count").and_then(Value::as_u64).unwrap_or(0),
                    "rejected_count": next_cycle.pointer("/fitness_review/rejected_count").and_then(Value::as_u64).unwrap_or(0)
                }
            },
            {
                "id": "V6-APP-023.5",
                "claim": "snowball_runtime_publishes_live_cycle_state_for_operator_controls",
                "evidence": {"cycle_id": cycle_id}
            },
            {
                "id": "V6-APP-023.6",
                "claim": "snowball_status_and_compact_controls_surface_cycle_stage_batch_outcomes_and_regression_state",
                "evidence": {"cycle_id": cycle_id, "has_review": true}
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_archive_discarded(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let mut cycles = load_cycles(root);
    let cycle_id = active_or_requested_cycle(parsed, &cycles, "snowball-default");
    let mut cycles_map = cycles
        .get("cycles")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let cycle = load_cycle_value(&cycles, &cycle_id);
    let review = load_review(root, &cycle_id);
    if strict && review.is_none() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "snowball_plane_archive_discarded",
            "action": "archive-discarded",
            "errors": ["snowball_fitness_review_missing"],
            "cycle_id": cycle_id
        });
    }
    let review = review.unwrap_or_else(|| json!({}));
    let mut discarded = review
        .get("demoted")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    discarded.extend(
        review
            .get("rejected")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
    );
    let (items, index) = archive_discarded_blobs(root, &cycle_id, discarded.as_slice());
    let index_path = discarded_blob_index_path(root, &cycle_id);

    let mut next_cycle = cycle.unwrap_or_else(|| json!({"cycle_id": cycle_id, "stage":"archived"}));
    next_cycle["discarded_archive"] = json!({
        "path": index_path.display().to_string(),
        "count": items.len()
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
        "type": "snowball_plane_archive_discarded",
        "lane": "core/layer0/ops",
        "action": "archive-discarded",
        "cycle_id": cycle_id,
        "archive": index,
        "claim_evidence": [
            {
                "id": "V6-APP-023.9",
                "claim": "snowball_archives_discarded_and_demoted_ideas_as_versioned_blob_artifacts_with_resurrection_metadata",
                "evidence": {
                    "cycle_id": cycle_id,
                    "discarded_blob_count": items.len(),
                    "discarded_blob_index_path": index_path.display().to_string()
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
