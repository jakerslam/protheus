fn bool_like(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "y" | "on"
    )
}

fn context_stacks_nexus_enabled(parsed: &crate::ParsedArgs) -> bool {
    parsed
        .flags
        .get("nexus")
        .map(|raw| bool_like(raw.as_str()))
        .or_else(|| {
            std::env::var("PROTHEUS_HIERARCHICAL_NEXUS_V1")
                .ok()
                .map(|raw| bool_like(raw.as_str()))
        })
        .unwrap_or(true)
}

fn context_stacks_force_block_pair_enabled() -> bool {
    std::env::var("PROTHEUS_HIERARCHICAL_NEXUS_BLOCK_CONTEXT_STACKS_ROUTE")
        .ok()
        .map(|raw| bool_like(raw.as_str()))
        .unwrap_or(false)
}

fn authorize_context_stacks_command_with_nexus_inner(
    command: &str,
    force_block_pair: bool,
) -> Result<Value, String> {
    let mut policy = DefaultNexusPolicy::default();
    if force_block_pair {
        policy.block_pair("client_ingress", "context_stacks");
    }
    let mut nexus = MainNexusControlPlane::new(
        NexusFeatureFlags {
            hierarchical_nexus_enabled: true,
            coexist_with_flat_routing: true,
        },
        policy,
    );
    let _ = nexus.register_v1_adapters("context_stacks_kernel")?;
    let schema = format!("context_stacks.command.{}", clean(command, 64));
    let lease = nexus.issue_route_lease(
        "context_stacks_kernel",
        LeaseIssueRequest {
            source: "client_ingress".to_string(),
            target: "context_stacks".to_string(),
            schema_ids: vec![schema.clone()],
            verbs: vec!["invoke".to_string()],
            required_verity: VerityClass::Standard,
            trust_class: TrustClass::ClientIngressBoundary,
            requested_ttl_ms: 30_000,
            template_id: None,
            template_version: None,
        },
    )?;
    let delivery = nexus.authorize_direct_delivery(
        "context_stacks_kernel",
        DeliveryAuthorizationInput {
            lease_id: Some(lease.lease_id.clone()),
            source: "client_ingress".to_string(),
            target: "context_stacks".to_string(),
            schema_id: schema,
            verb: "invoke".to_string(),
            offered_verity: VerityClass::Standard,
            now_ms: None,
        },
    );
    if !delivery.allowed {
        return Err(format!(
            "context_stacks_nexus_delivery_denied:{}",
            clean(delivery.reason.as_str(), 200)
        ));
    }
    let receipt_ids = nexus
        .receipts()
        .iter()
        .map(|row| row.receipt_id.clone())
        .collect::<Vec<_>>();
    Ok(json!({
      "enabled": true,
      "route": {"source":"client_ingress","target":"context_stacks","verb":"invoke"},
      "lease_id": lease.lease_id,
      "delivery": delivery,
      "metrics": nexus.metrics(),
      "receipt_ids": receipt_ids
    }))
}

fn authorize_context_stacks_command_with_nexus(command: &str) -> Result<Value, String> {
    authorize_context_stacks_command_with_nexus_inner(
        command,
        context_stacks_force_block_pair_enabled(),
    )
}

fn render_context_stack(root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let mut state = load_context_stacks_state(root);
    let stack_id = stack_id_from(parsed);
    if stack_id.is_empty() {
        return json!({"ok": false, "status": "blocked", "error": "stack_id_required"});
    }
    let Some(manifest_index) = find_manifest_index(&state, &stack_id) else {
        return json!({"ok": false, "status": "blocked", "error": "stack_not_found", "stack_id": stack_id});
    };
    let semantic_snapshot_id = state.manifests[manifest_index].semantic_snapshot_id.clone();
    let Some(snapshot) = find_semantic_snapshot(&state, &semantic_snapshot_id).cloned() else {
        return json!({"ok": false, "status": "blocked", "error": "semantic_snapshot_missing", "semantic_snapshot_id": semantic_snapshot_id});
    };
    let plan = build_render_plan(parsed, &semantic_snapshot_id, None);
    if let Some(index) = state
        .render_plans
        .iter()
        .position(|row| row.render_plan_id == plan.render_plan_id)
    {
        state.render_plans[index] = plan.clone();
    } else {
        state.render_plans.push(plan.clone());
    }
    let provider_snapshot = derive_provider_snapshot(&snapshot, &plan);
    let cache_hit = state
        .provider_snapshots
        .iter()
        .any(|row| row.render_fingerprint == provider_snapshot.render_fingerprint);
    if !cache_hit {
        state.provider_snapshots.push(provider_snapshot.clone());
    }
    let decision = SchedulerEdgeCaseDecision {
        scheduler_mode: "single_shot".to_string(),
        cache_hit,
        cache_creation_input_tokens: if cache_hit {
            0
        } else {
            provider_snapshot_token_estimate(&provider_snapshot.serialized_prefix)
        },
        cache_read_input_tokens: if cache_hit {
            provider_snapshot_token_estimate(&provider_snapshot.serialized_prefix)
        } else {
            0
        },
        seed_then_fanout: false,
        breakpoint_mode: None,
    };
    let batch_class =
        normalize_batch_class(&plan, BatchLane::LiveMicrobatch, &provider_snapshot.render_fingerprint);
    let batch_id = batch_class_id_for(&batch_class);
    let receipt = receipt_with_common_fields(
        "context_stack_render",
        &stack_id,
        if cache_hit { "render_cache_hit" } else { "render_created" },
        Some(batch_id),
        Some(&decision),
    );
    let _ = append_context_stacks_receipt(root, &receipt);
    let _ = persist_context_stacks_state(root, &state);
    json!({
        "ok": true,
        "type": "context_stacks_render",
        "stack_id": stack_id,
        "provider_snapshot": provider_snapshot,
        "cache_hit": cache_hit,
        "receipt_id": receipt.get("receipt_id").cloned().unwrap_or(Value::Null)
    })
}

fn batch_class_context_stack(root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let mut state = load_context_stacks_state(root);
    let stack_id = stack_id_from(parsed);
    if stack_id.is_empty() {
        return json!({"ok": false, "status": "blocked", "error": "stack_id_required"});
    }
    let Some(manifest_index) = find_manifest_index(&state, &stack_id) else {
        return json!({"ok": false, "status": "blocked", "error": "stack_not_found"});
    };
    let semantic_snapshot_id = state.manifests[manifest_index].semantic_snapshot_id.clone();
    let Some(snapshot) = find_semantic_snapshot(&state, &semantic_snapshot_id).cloned() else {
        return json!({"ok": false, "status": "blocked", "error": "semantic_snapshot_missing"});
    };
    let plan = build_render_plan(parsed, &semantic_snapshot_id, None);
    let provider_snapshot = derive_provider_snapshot(&snapshot, &plan);
    let lane = BatchLane::from_raw(
        parsed
            .flags
            .get("lane")
            .map(String::as_str)
            .unwrap_or("live_microbatch"),
    );
    let policy = load_context_stacks_policy(root);
    if lane == BatchLane::ProviderBatch && !policy.allow_provider_batch_lane {
        return json!({"ok": false, "status": "blocked", "error": "provider_batch_lane_blocked"});
    }
    let batch_class = normalize_batch_class(&plan, lane, &provider_snapshot.render_fingerprint);
    if !state.batch_classes.contains(&batch_class) {
        state.batch_classes.push(batch_class.clone());
    }
    let _ = persist_context_stacks_state(root, &state);
    json!({
        "ok": true,
        "type": "context_stacks_batch_class",
        "batch_id": batch_class_id_for(&batch_class),
        "batch_class": batch_class
    })
}

fn scheduler_check_context_stack(root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let state = load_context_stacks_state(root);
    let policy = load_context_stacks_policy(root);
    let stack_id = stack_id_from(parsed);
    if stack_id.is_empty() {
        return json!({"ok": false, "status": "blocked", "error": "stack_id_required"});
    }
    let Some(manifest_index) = find_manifest_index(&state, &stack_id) else {
        return json!({"ok": false, "status": "blocked", "error": "stack_not_found"});
    };
    let semantic_snapshot_id = state.manifests[manifest_index].semantic_snapshot_id.clone();
    let Some(snapshot) = find_semantic_snapshot(&state, &semantic_snapshot_id).cloned() else {
        return json!({"ok": false, "status": "blocked", "error": "semantic_snapshot_missing"});
    };
    let plan = build_render_plan(parsed, &semantic_snapshot_id, None);
    let provider_snapshot = derive_provider_snapshot(&snapshot, &plan);
    let has_cached = state
        .provider_snapshots
        .iter()
        .any(|row| row.render_fingerprint == provider_snapshot.render_fingerprint);
    let prompt_tokens = parse_u64_flag(parsed, "prompt-tokens", 800);
    let stable_prefix_tokens = parse_u64_flag(
        parsed,
        "stable-prefix-tokens",
        provider_snapshot_token_estimate(&provider_snapshot.serialized_prefix),
    );
    let lookback_window_tokens =
        parse_u64_flag(parsed, "lookback-window-tokens", policy.lookback_window_tokens);
    let fresh_cohort_size =
        parse_usize_flag(parsed, "fresh-cohort-size", policy.seed_then_fanout_min_cohort);
    let decision = evaluate_scheduler_edge_cases(
        &policy,
        plan.cache_policy,
        prompt_tokens,
        stable_prefix_tokens,
        lookback_window_tokens,
        fresh_cohort_size,
        has_cached,
    );
    let batch_class = normalize_batch_class(
        &plan,
        BatchLane::from_raw(
            parsed
                .flags
                .get("lane")
                .map(String::as_str)
                .unwrap_or("live_microbatch"),
        ),
        &provider_snapshot.render_fingerprint,
    );
    let batch_id = batch_class_id_for(&batch_class);
    let receipt = receipt_with_common_fields(
        "context_stack_scheduler_check",
        &stack_id,
        &decision.scheduler_mode,
        Some(batch_id.clone()),
        Some(&decision),
    );
    let _ = append_context_stacks_receipt(root, &receipt);
    let _ = append_context_stacks_digestion_log(
        root,
        &stack_id,
        &[
            format!("scheduler_mode={}", decision.scheduler_mode),
            format!("seed_then_fanout={}", decision.seed_then_fanout),
        ],
    );
    json!({
        "ok": true,
        "type": "context_stacks_scheduler_check",
        "stack_id": stack_id,
        "batch_id": batch_id,
        "decision": decision,
        "receipt_id": receipt.get("receipt_id").cloned().unwrap_or(Value::Null)
    })
}

fn context_stacks_status(root: &Path) -> Value {
    let state = load_context_stacks_state(root);
    let policy = load_context_stacks_policy(root);
    json!({
        "ok": true,
        "type": "context_stacks_status",
        "policy_path": context_stacks_policy_path(root).to_string_lossy().to_string(),
        "state_path": context_stacks_state_path(root).to_string_lossy().to_string(),
        "receipts_path": context_stacks_receipts_path(root).to_string_lossy().to_string(),
        "digestion_log_path": context_stacks_digestion_log_path(root).to_string_lossy().to_string(),
        "counts": {
            "stacks": state.manifests.len(),
            "active_stacks": state.manifests.iter().filter(|row| !row.archived).count(),
            "semantic_snapshots": state.semantic_snapshots.len(),
            "provider_snapshots": state.provider_snapshots.len(),
            "delta_tails": state.delta_tails.len(),
            "batch_classes": state.batch_classes.len(),
            "taste_vectors": state.taste_vectors.len(),
            "partial_merge_events": state.partial_merge_events.len(),
            "hybrid_retrieval_events": state.hybrid_retrieval_events.len(),
            "node_spike_thresholds": state.node_spike_thresholds.len(),
            "node_spike_events": state.node_spike_events.len(),
            "node_spike_queue_depth": state.node_spike_queue.len(),
            "merge_feedback_events": state.merge_feedback_events.len(),
            "skill_performance_ledger": state.skill_performance_ledger.len(),
            "speculative_overlays": state.speculative_overlays.len(),
            "speculative_overlay_receipts": state.speculative_overlay_receipts.len()
        },
        "policy": policy,
        "digestion_log_examples": [
            "- ts: \"2026-04-05T00:00:00Z\"\n  stack_id: \"demo\"\n  events:\n    - \"context_stack_manifest_created\"\n    - \"semantic_snapshot_id=semantic_...\"",
            "- ts: \"2026-04-05T00:01:00Z\"\n  stack_id: \"demo\"\n  events:\n    - \"scheduler_mode=seed_then_fanout\"\n    - \"seed_then_fanout=true\""
        ]
    })
}

fn context_stacks_policy_json(root: &Path) -> Value {
    let policy = load_context_stacks_policy(root);
    json!({
        "ok": true,
        "type": "context_stacks_policy",
        "policy": policy
    })
}

fn context_stacks_taste_tune(root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let mut state = load_context_stacks_state(root);
    let family = clean(
        parsed
            .flags
            .get("family")
            .or_else(|| parsed.flags.get("skill-family"))
            .map(String::as_str)
            .unwrap_or("general"),
        120,
    );
    let merge_lift = parsed
        .flags
        .get("merge-lift")
        .and_then(|row| clean(row, 32).parse::<f64>().ok())
        .unwrap_or(0.0)
        .clamp(-1.0, 1.0);
    let current = state.taste_vectors.get(&family).copied().unwrap_or(1.0_f64);
    let target = (current + merge_lift * 0.25).clamp(0.2, 2.0);
    let smoothed = (current * 0.7 + target * 0.3).clamp(0.2, 2.0);
    let bounded_delta = (smoothed - current).clamp(-0.15, 0.15);
    let next = (current + bounded_delta).clamp(0.2, 2.0);
    state.taste_vectors.insert(family.clone(), next);
    let _ = persist_context_stacks_state(root, &state);

    let receipt = receipt_with_common_fields(
        "context_stack_taste_tune",
        "global",
        "taste_tuned",
        None,
        None,
    );
    let _ = append_context_stacks_receipt(root, &receipt);
    let _ = append_context_stacks_digestion_log(
        root,
        "global",
        &[format!(
            "taste_tuned:{} current={:.4} next={:.4} merge_lift={:.4}",
            family, current, next, merge_lift
        )],
    );
    json!({
        "ok": true,
        "type": "context_stacks_taste_tune",
        "family": family,
        "current": current,
        "next": next,
        "merge_lift": merge_lift,
        "delta": next - current,
        "receipt_id": receipt.get("receipt_id").cloned().unwrap_or(Value::Null)
    })
}

fn context_stacks_partial_merge(root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let mut state = load_context_stacks_state(root);
    let stack_id = stack_id_from(parsed);
    if stack_id.is_empty() {
        return json!({"ok": false, "status": "blocked", "error": "stack_id_required"});
    }
    let Some(manifest_index) = find_manifest_index(&state, &stack_id) else {
        return json!({"ok": false, "status": "blocked", "error": "stack_not_found", "stack_id": stack_id});
    };
    let semantic_snapshot_id = state.manifests[manifest_index].semantic_snapshot_id.clone();
    let Some(snapshot_index) = state
        .semantic_snapshots
        .iter()
        .position(|row| row.semantic_snapshot_id == semantic_snapshot_id)
    else {
        return json!({"ok": false, "status": "blocked", "error": "semantic_snapshot_missing", "stack_id": stack_id});
    };
    let patch = parse_json_value(parsed.flags.get("patch-json"));
    let mut changed_slices = Vec::<String>::new();
    let mut next_snapshot = state.semantic_snapshots[snapshot_index].clone();

    if let Some(add) = patch
        .get("stable_nodes_add")
        .and_then(Value::as_array)
        .cloned()
    {
        let mut next_nodes = next_snapshot.stable_head.ordered_stable_nodes.clone();
        for row in add {
            if let Some(text) = row.as_str() {
                let clean_text = clean(text, 600);
                if !clean_text.is_empty() {
                    next_nodes.push(clean_text);
                }
            }
        }
        let deduped = dedupe_preserving_order(next_nodes);
        if deduped != next_snapshot.stable_head.ordered_stable_nodes {
            next_snapshot.stable_head.ordered_stable_nodes = deduped;
            changed_slices.push("stable_head".to_string());
        }
    }
    if let Some(meta_patch) = patch.get("volatile_metadata_patch").cloned() {
        if meta_patch.is_object() {
            let mut merged = next_snapshot.volatile_metadata.clone();
            if !merged.is_object() {
                merged = json!({});
            }
            if let Some(map) = meta_patch.as_object() {
                for (k, v) in map {
                    merged[k] = v.clone();
                }
            }
            if merged != next_snapshot.volatile_metadata {
                next_snapshot.volatile_metadata = merged;
                changed_slices.push("volatile_metadata".to_string());
            }
        }
    }
    if changed_slices.is_empty() {
        return json!({"ok": false, "status": "blocked", "error": "partial_merge_no_changes", "stack_id": stack_id});
    }

    next_snapshot.semantic_snapshot_id = semantic_snapshot_id_for(&next_snapshot.stable_head);
    next_snapshot.updated_at = now_iso();
    let parity_hash = sha256_hex(
        &serde_json::to_string(&next_snapshot.stable_head).unwrap_or_else(|_| "{}".to_string()),
    );
    let token_budget_estimate = json!({
        "partial": changed_slices.len() as u64 * 64,
        "full": (next_snapshot.stable_head.ordered_stable_nodes.len() as u64).saturating_mul(8)
    });
    state.semantic_snapshots.push(next_snapshot.clone());
    state.manifests[manifest_index].semantic_snapshot_id = next_snapshot.semantic_snapshot_id.clone();
    state.manifests[manifest_index].updated_at = now_iso();
    state.partial_merge_events.push(json!({
        "stack_id": stack_id,
        "ts": now_iso(),
        "changed_slices": changed_slices,
        "mode": "diff_scoped_partial",
        "semantic_snapshot_id": next_snapshot.semantic_snapshot_id,
        "parity_hash": parity_hash,
        "token_budget_estimate": token_budget_estimate
    }));
    let skill_family = clean(
        parsed
            .flags
            .get("skill-family")
            .or_else(|| parsed.flags.get("family"))
            .map(String::as_str)
            .unwrap_or("general"),
        120,
    );
    let throughput_delta = parsed
        .flags
        .get("throughput-delta")
        .and_then(|row| clean(row, 32).parse::<f64>().ok())
        .unwrap_or((changed_slices.len() as f64 * 0.02).clamp(0.0, 0.25));
    let memory_delta = parsed
        .flags
        .get("memory-delta")
        .and_then(|row| clean(row, 32).parse::<f64>().ok())
        .unwrap_or((-0.01 * changed_slices.len() as f64).clamp(-0.2, 0.0));
    let stability_delta = parsed
        .flags
        .get("stability-delta")
        .and_then(|row| clean(row, 32).parse::<f64>().ok())
        .unwrap_or(0.02_f64.clamp(-0.2, 0.2));
    let error_rate_delta = parsed
        .flags
        .get("error-rate-delta")
        .and_then(|row| clean(row, 32).parse::<f64>().ok())
        .unwrap_or((-0.005 * changed_slices.len() as f64).clamp(-0.2, 0.0));
    let merge_feedback_event = json!({
        "event_id": receipt_hash(&json!({
            "stack_id": stack_id,
            "semantic_snapshot_id": next_snapshot.semantic_snapshot_id,
            "skill_family": skill_family,
            "parity_hash": parity_hash
        })),
        "ts": now_iso(),
        "stack_id": stack_id,
        "skill_family": skill_family,
        "metrics": {
            "throughput_delta": throughput_delta,
            "memory_delta": memory_delta,
            "stability_delta": stability_delta,
            "error_rate_delta": error_rate_delta
        },
        "mode": "post_merge_feedback_loop",
        "semantic_snapshot_id": next_snapshot.semantic_snapshot_id
    });
    state.merge_feedback_events.push(merge_feedback_event.clone());
    let prev_ledger = state
        .skill_performance_ledger
        .get(skill_family.as_str())
        .cloned()
        .unwrap_or_else(|| json!({}));
    let prev_samples = prev_ledger
        .get("samples")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let next_samples = prev_samples.saturating_add(1);
    let rolling = |key: &str, next: f64| -> f64 {
        let prev = prev_ledger
            .pointer(&format!("/rolling/{key}"))
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        ((prev * prev_samples as f64) + next) / next_samples as f64
    };
    state.skill_performance_ledger.insert(
        skill_family.clone(),
        json!({
            "samples": next_samples,
            "last_event_id": merge_feedback_event.get("event_id").cloned().unwrap_or(Value::Null),
            "last_stack_id": stack_id,
            "last_semantic_snapshot_id": next_snapshot.semantic_snapshot_id,
            "last_updated_at": now_iso(),
            "rolling": {
                "throughput_delta": rolling("throughput_delta", throughput_delta),
                "memory_delta": rolling("memory_delta", memory_delta),
                "stability_delta": rolling("stability_delta", stability_delta),
                "error_rate_delta": rolling("error_rate_delta", error_rate_delta)
            }
        }),
    );
    let _ = persist_context_stacks_state(root, &state);

    let receipt = receipt_with_common_fields(
        "context_stack_partial_merge",
        &stack_id,
        "partial_merge_applied",
        None,
        None,
    );
    let _ = append_context_stacks_receipt(root, &receipt);
    let feedback_receipt = json!({
        "type": "context_stack_merge_feedback",
        "stack_id": stack_id,
        "skill_family": skill_family,
        "event": merge_feedback_event,
        "ts": now_iso()
    });
    let _ = append_context_stacks_receipt(root, &feedback_receipt);
    let _ = append_context_stacks_digestion_log(
        root,
        &stack_id,
        &[format!(
            "partial_merge changed_slices={} parity_hash={}",
            state
                .partial_merge_events
                .last()
                .and_then(|row| row.get("changed_slices"))
                .cloned()
                .unwrap_or_else(|| json!([])),
            parity_hash
        )],
    );
    json!({
        "ok": true,
        "type": "context_stacks_partial_merge",
        "stack_id": stack_id,
        "mode": "diff_scoped_partial",
        "changed_slices": state
            .partial_merge_events
            .last()
            .and_then(|row| row.get("changed_slices"))
            .cloned()
            .unwrap_or_else(|| json!([])),
        "semantic_snapshot_id": next_snapshot.semantic_snapshot_id,
        "parity_hash": parity_hash,
        "token_budget_estimate": token_budget_estimate,
        "merge_feedback_event_id": merge_feedback_event
            .get("event_id")
            .cloned()
            .unwrap_or(Value::Null),
        "skill_family": skill_family,
        "skill_performance_ledger": state
            .skill_performance_ledger
            .get(skill_family.as_str())
            .cloned()
            .unwrap_or(Value::Null),
        "receipt_id": receipt.get("receipt_id").cloned().unwrap_or(Value::Null)
    })
}

fn context_stacks_hybrid_retrieve(root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let mut state = load_context_stacks_state(root);
    let stack_id = stack_id_from(parsed);
    if stack_id.is_empty() {
        return json!({"ok": false, "status": "blocked", "error": "stack_id_required"});
    }
    if find_manifest_index(&state, &stack_id).is_none() {
        return json!({"ok": false, "status": "blocked", "error": "stack_not_found", "stack_id": stack_id});
    }
    let top_k = parse_usize_flag(parsed, "top-k", 5).max(1);
    let query = clean(
        parsed
            .flags
            .get("query")
            .map(String::as_str)
            .unwrap_or(""),
        500,
    );
    let vector_rows = parse_json_value(parsed.flags.get("vector-json"))
        .as_array()
        .cloned()
        .unwrap_or_default();
    let edge_rows = parse_json_value(parsed.flags.get("edges-json"))
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut vector_scores = std::collections::BTreeMap::<String, f64>::new();
    for row in vector_rows {
        if let Some(id) = row.get("id").and_then(Value::as_str) {
            let score = row
                .get("score")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                .clamp(0.0, 1.0);
            vector_scores.insert(clean(id, 160), score);
        }
    }
    let mut edge_scores = std::collections::BTreeMap::<String, f64>::new();
    for row in edge_rows {
        if let Some(id) = row.get("id").and_then(Value::as_str) {
            let score = row
                .get("edge_confidence")
                .or_else(|| row.get("score"))
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                .clamp(0.0, 1.0);
            edge_scores.insert(clean(id, 160), score);
        }
    }
    let mut candidate_ids = vector_scores
        .keys()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    for id in edge_scores.keys() {
        candidate_ids.insert(id.clone());
    }
    let mut ranked = candidate_ids
        .into_iter()
        .map(|id| {
            let vector = vector_scores.get(&id).copied().unwrap_or(0.0);
            let edge = edge_scores.get(&id).copied().unwrap_or(0.0);
            let combined = ((vector * 0.65) + (edge * 0.35)).clamp(0.0, 1.0);
            json!({
                "id": id,
                "vector_score": vector,
                "edge_confidence": edge,
                "combined_score": combined
            })
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| {
        let av = a
            .get("combined_score")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let bv = b
            .get("combined_score")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
    });
    ranked.truncate(top_k);

    let event = json!({
        "ts": now_iso(),
        "stack_id": stack_id,
        "query": query,
        "top_k": top_k,
        "results": ranked
    });
    state.hybrid_retrieval_events.push(event.clone());
    let _ = persist_context_stacks_state(root, &state);

    let receipt = receipt_with_common_fields(
        "context_stack_hybrid_retrieve",
        &stack_id,
        "hybrid_retrieval_ok",
        None,
        None,
    );
    let _ = append_context_stacks_receipt(root, &receipt);
    let _ = append_context_stacks_digestion_log(
        root,
        &stack_id,
        &[format!("hybrid_retrieve top_k={top_k} query={query}")],
    );
    json!({
        "ok": true,
        "type": "context_stacks_hybrid_retrieve",
        "stack_id": stack_id,
        "query": query,
        "results": event.get("results").cloned().unwrap_or_else(|| json!([])),
        "receipt_id": receipt.get("receipt_id").cloned().unwrap_or(Value::Null)
    })
}

fn context_stacks_node_spike(root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let mut state = load_context_stacks_state(root);
    let stack_id = stack_id_from(parsed);
    if stack_id.is_empty() {
        return json!({"ok": false, "status": "blocked", "error": "stack_id_required"});
    }
    if find_manifest_index(&state, &stack_id).is_none() {
        return json!({"ok": false, "status": "blocked", "error": "stack_not_found", "stack_id": stack_id});
    }
    if !state.node_spike_metrics.is_object() {
        state.node_spike_metrics = json!({
            "queue_limit": 128u64,
            "dropped_non_critical": 0u64,
            "critical_retained": 0u64,
            "critical_journaled": 0u64,
            "critical_dropped": 0u64,
            "last_overload_at": Value::Null
        });
    }
    let node_id = clean(
        parsed
            .flags
            .get("node-id")
            .or_else(|| parsed.flags.get("node"))
            .map(String::as_str)
            .unwrap_or("root"),
        120,
    );
    let delta = parsed
        .flags
        .get("delta")
        .and_then(|row| clean(row, 32).parse::<f64>().ok())
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let staleness_seconds = parse_u64_flag(parsed, "staleness-seconds", 0).min(172_800);
    let staleness_norm = (staleness_seconds as f64 / 3600.0).clamp(0.0, 1.0);
    let external_trigger = parsed
        .flags
        .get("external-trigger")
        .map(|row| bool_like(row))
        .unwrap_or(false);
    let queue_limit_default = state
        .node_spike_metrics
        .get("queue_limit")
        .and_then(Value::as_u64)
        .unwrap_or(128);
    let queue_limit = parse_u64_flag(parsed, "queue-limit", queue_limit_default)
        .clamp(8, 4096) as usize;
    let queue_depth_before = state.node_spike_queue.len();
    let inferred_load = if queue_limit == 0 {
        0.0
    } else {
        queue_depth_before as f64 / queue_limit as f64
    };
    let load_signal = parsed
        .flags
        .get("load-signal")
        .and_then(|row| clean(row, 32).parse::<f64>().ok())
        .unwrap_or(inferred_load)
        .clamp(0.0, 1.0);
    let success_signal = parsed
        .flags
        .get("success-signal")
        .and_then(|row| clean(row, 32).parse::<f64>().ok())
        .unwrap_or(0.5)
        .clamp(0.0, 1.0);
    let utility = (delta * 0.6 + staleness_norm * 0.3 + if external_trigger { 0.35 } else { 0.0 })
        .clamp(0.0, 1.0);
    let threshold_before = state
        .node_spike_thresholds
        .get(&node_id)
        .copied()
        .unwrap_or(0.35);
    let mut threshold_after = threshold_before + (load_signal - 0.5) * 0.2 - (success_signal - 0.5) * 0.15;
    threshold_after = threshold_after.clamp(0.05, 0.95);
    let should_fire = utility >= threshold_after;
    let critical = external_trigger || utility >= 0.9;
    let mut backpressure_action = "none".to_string();

    let mut dropped_non_critical = state
        .node_spike_metrics
        .get("dropped_non_critical")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let mut critical_retained = state
        .node_spike_metrics
        .get("critical_retained")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let mut critical_journaled = state
        .node_spike_metrics
        .get("critical_journaled")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let critical_dropped = state
        .node_spike_metrics
        .get("critical_dropped")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let mut event = Value::Null;
    let mut enqueued = false;
    if should_fire {
        let spike_event = json!({
            "event_id": receipt_hash(&json!({
                "stack_id": stack_id,
                "node_id": node_id,
                "delta": delta,
                "staleness_seconds": staleness_seconds,
                "external_trigger": external_trigger,
                "utility": utility,
                "threshold_after": threshold_after,
                "ts": now_iso()
            })),
            "stack_id": stack_id,
            "node_id": node_id,
            "critical": critical,
            "delta": delta,
            "staleness_seconds": staleness_seconds,
            "external_trigger": external_trigger,
            "utility": utility,
            "threshold_before": threshold_before,
            "threshold_after": threshold_after,
            "ts": now_iso()
        });
        event = spike_event.clone();
        state.node_spike_events.push(spike_event.clone());
        if state.node_spike_events.len() > 256 {
            let trim = state.node_spike_events.len().saturating_sub(256);
            state.node_spike_events.drain(0..trim);
        }
        state.node_spike_queue.push(spike_event);
        enqueued = true;
        if state.node_spike_queue.len() > queue_limit {
            if let Some(non_critical_idx) = state
                .node_spike_queue
                .iter()
                .position(|row| !row.get("critical").and_then(Value::as_bool).unwrap_or(false))
            {
                state.node_spike_queue.remove(non_critical_idx);
                dropped_non_critical = dropped_non_critical.saturating_add(1);
                backpressure_action = "drop_non_critical".to_string();
            } else {
                let _ = state.node_spike_queue.pop();
                enqueued = false;
                critical_journaled = critical_journaled.saturating_add(1);
                backpressure_action = "critical_journaled".to_string();
            }
            state.node_spike_metrics["last_overload_at"] = json!(now_iso());
        }
        if critical {
            critical_retained = critical_retained.saturating_add(1);
        }
    }
    let queue_depth_after = state.node_spike_queue.len();
    let queue_pressure_after = if queue_limit == 0 {
        0.0
    } else {
        queue_depth_after as f64 / queue_limit as f64
    };
    threshold_after = (threshold_after + (queue_pressure_after - 0.5) * 0.1).clamp(0.05, 0.95);
    state
        .node_spike_thresholds
        .insert(node_id.clone(), threshold_after);
    state.node_spike_metrics["queue_limit"] = json!(queue_limit as u64);
    state.node_spike_metrics["queue_depth"] = json!(queue_depth_after as u64);
    state.node_spike_metrics["queue_pressure"] = json!(queue_pressure_after);
    state.node_spike_metrics["dropped_non_critical"] = json!(dropped_non_critical);
    state.node_spike_metrics["critical_retained"] = json!(critical_retained);
    state.node_spike_metrics["critical_journaled"] = json!(critical_journaled);
    state.node_spike_metrics["critical_dropped"] = json!(critical_dropped);
    state.node_spike_metrics["last_backpressure_action"] = json!(backpressure_action.clone());
    state.node_spike_metrics["last_threshold_after"] = json!(threshold_after);
    state.node_spike_metrics["last_utility"] = json!(utility);

    let _ = persist_context_stacks_state(root, &state);

    let receipt = json!({
        "type": "context_stack_node_spike",
        "stack_id": stack_id,
        "node_id": node_id,
        "should_fire": should_fire,
        "critical": critical,
        "utility": utility,
        "threshold_before": threshold_before,
        "threshold_after": threshold_after,
        "enqueued": enqueued,
        "queue_depth_before": queue_depth_before,
        "queue_depth_after": queue_depth_after,
        "queue_limit": queue_limit,
        "backpressure_action": backpressure_action,
        "event": event,
        "ts": now_iso()
    });
    let mut receipt_with_id = receipt.clone();
    receipt_with_id["receipt_id"] = json!(receipt_hash(&receipt));
    let _ = append_context_stacks_receipt(root, &receipt_with_id);
    let _ = append_context_stacks_digestion_log(
        root,
        &stack_id,
        &[format!(
            "node_spike node={} fire={} utility={:.4} threshold={:.4} action={} queue={}/{}",
            node_id, should_fire, utility, threshold_after, backpressure_action, queue_depth_after, queue_limit
        )],
    );
    json!({
        "ok": true,
        "type": "context_stacks_node_spike",
        "stack_id": stack_id,
        "node_id": node_id,
        "should_fire": should_fire,
        "critical": critical,
        "utility": utility,
        "threshold_before": threshold_before,
        "threshold_after": threshold_after,
        "enqueued": enqueued,
        "queue": {
            "depth_before": queue_depth_before,
            "depth_after": queue_depth_after,
            "limit": queue_limit
        },
        "metrics": state.node_spike_metrics,
        "backpressure_action": backpressure_action,
        "event": event,
        "receipt_id": receipt_with_id.get("receipt_id").cloned().unwrap_or(Value::Null)
    })
}

fn apply_patch_to_snapshot(snapshot: &SemanticSnapshot, patch: &Value) -> (SemanticSnapshot, Vec<String>) {
    let mut next_snapshot = snapshot.clone();
    let mut changed_slices = Vec::<String>::new();
    if let Some(add) = patch.get("stable_nodes_add").and_then(Value::as_array).cloned() {
        let mut next_nodes = next_snapshot.stable_head.ordered_stable_nodes.clone();
        for row in add {
            if let Some(text) = row.as_str() {
                let clean_text = clean(text, 600);
                if !clean_text.is_empty() {
                    next_nodes.push(clean_text);
                }
            }
        }
        let deduped = dedupe_preserving_order(next_nodes);
        if deduped != next_snapshot.stable_head.ordered_stable_nodes {
            next_snapshot.stable_head.ordered_stable_nodes = deduped;
            changed_slices.push("stable_head".to_string());
        }
    }
    if let Some(meta_patch) = patch.get("volatile_metadata_patch").cloned() {
        if meta_patch.is_object() {
            let mut merged = next_snapshot.volatile_metadata.clone();
            if !merged.is_object() {
                merged = json!({});
            }
            if let Some(map) = meta_patch.as_object() {
                for (k, v) in map {
                    merged[k] = v.clone();
                }
            }
            if merged != next_snapshot.volatile_metadata {
                next_snapshot.volatile_metadata = merged;
                changed_slices.push("volatile_metadata".to_string());
            }
        }
    }
    next_snapshot.semantic_snapshot_id = semantic_snapshot_id_for(&next_snapshot.stable_head);
    next_snapshot.updated_at = now_iso();
    (next_snapshot, changed_slices)
}

fn context_stacks_speculative_start(root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let mut state = load_context_stacks_state(root);
    let stack_id = stack_id_from(parsed);
    if stack_id.is_empty() {
        return json!({"ok": false, "status": "blocked", "error": "stack_id_required"});
    }
    let Some(manifest_index) = find_manifest_index(&state, &stack_id) else {
        return json!({"ok": false, "status": "blocked", "error": "stack_not_found", "stack_id": stack_id});
    };
    let base_semantic_snapshot_id = state.manifests[manifest_index].semantic_snapshot_id.clone();
    let Some(snapshot) = find_semantic_snapshot(&state, &base_semantic_snapshot_id).cloned() else {
        return json!({"ok": false, "status": "blocked", "error": "semantic_snapshot_missing", "stack_id": stack_id});
    };
    let patch = parse_json_value(parsed.flags.get("patch-json"));
    let (proposed_snapshot, changed_slices) = apply_patch_to_snapshot(&snapshot, &patch);
    if changed_slices.is_empty() {
        return json!({"ok": false, "status": "blocked", "error": "speculative_overlay_no_changes", "stack_id": stack_id});
    }
    let overlay_id = clean(
        parsed
            .flags
            .get("overlay-id")
            .map(String::as_str)
            .unwrap_or_else(|| generate_id("overlay").as_str()),
        120,
    );
    if find_overlay_index(&state, &overlay_id).is_some() {
        return json!({"ok": false, "status": "blocked", "error": "overlay_exists", "overlay_id": overlay_id});
    }
    let verify_merge = truthy(parsed.flags.get("verify-merge"));
    let approval_note = parsed
        .flags
        .get("approval-note")
        .map(|raw| clean(raw, 240))
        .filter(|raw| !raw.is_empty());
    let overlay = SpeculativeOverlayExecution {
        overlay_id: overlay_id.clone(),
        stack_id: stack_id.clone(),
        base_semantic_snapshot_id: base_semantic_snapshot_id.clone(),
        proposed_semantic_snapshot: proposed_snapshot.clone(),
        patch: patch.clone(),
        status: "active".to_string(),
        verity_required: true,
        verity_approved: verify_merge,
        approval_note,
        created_at: now_iso(),
        updated_at: now_iso(),
        merged_at: None,
        rolled_back_at: None,
    };
    state.speculative_overlays.push(overlay.clone());
    let receipt = json!({
        "type": "context_stack_speculative_start",
        "stack_id": stack_id,
        "overlay_id": overlay_id,
        "base_semantic_snapshot_id": base_semantic_snapshot_id,
        "proposed_semantic_snapshot_id": proposed_snapshot.semantic_snapshot_id,
        "changed_slices": changed_slices,
        "sandbox_mutation": "none",
        "ts": now_iso(),
    });
    let mut receipt_with_id = receipt.clone();
    receipt_with_id["receipt_id"] = json!(receipt_hash(&receipt));
    state.speculative_overlay_receipts.push(receipt_with_id.clone());
    let _ = persist_context_stacks_state(root, &state);
    let _ = append_context_stacks_receipt(root, &receipt_with_id);
    let _ = append_context_stacks_digestion_log(
        root,
        &stack_id,
        &[format!(
            "speculative_overlay_started overlay={} base={} proposed={}",
            overlay_id,
            overlay.base_semantic_snapshot_id,
            overlay.proposed_semantic_snapshot.semantic_snapshot_id
        )],
    );
    json!({
        "ok": true,
        "type": "context_stacks_speculative_start",
        "overlay": overlay,
        "changed_slices": changed_slices,
        "receipt_id": receipt_with_id.get("receipt_id").cloned().unwrap_or(Value::Null)
    })
}

fn context_stacks_speculative_merge(root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let mut state = load_context_stacks_state(root);
    let overlay_id = clean(
        parsed
            .flags
            .get("overlay-id")
            .map(String::as_str)
            .unwrap_or(""),
        120,
    );
    if overlay_id.is_empty() {
        return json!({"ok": false, "status": "blocked", "error": "overlay_id_required"});
    }
    let Some(overlay_index) = find_overlay_index(&state, &overlay_id) else {
        return json!({"ok": false, "status": "blocked", "error": "overlay_not_found", "overlay_id": overlay_id});
    };
    let overlay_snapshot = state.speculative_overlays[overlay_index].clone();
    if overlay_snapshot.status != "active" {
        return json!({"ok": false, "status": "blocked", "error": "overlay_not_active", "overlay_id": overlay_id, "status_current": overlay_snapshot.status});
    }
    let verify_merge = truthy(parsed.flags.get("verify-merge")) || overlay_snapshot.verity_approved;
    let approval_note = parsed
        .flags
        .get("approval-note")
        .map(|raw| clean(raw, 240))
        .or(overlay_snapshot.approval_note.clone());
    let approval_note_valid = approval_note
        .as_ref()
        .map(|note| note.len() >= 12)
        .unwrap_or(false);
    if overlay_snapshot.verity_required && !(verify_merge && approval_note_valid) {
        return json!({
            "ok": false,
            "status": "blocked",
            "error": "speculative_merge_approval_required",
            "overlay_id": overlay_id,
            "verity_required": overlay_snapshot.verity_required,
            "verify_merge": verify_merge,
            "approval_note_valid": approval_note_valid
        });
    }
    let Some(manifest_index) = find_manifest_index(&state, &overlay_snapshot.stack_id) else {
        return json!({"ok": false, "status": "blocked", "error": "stack_not_found", "stack_id": overlay_snapshot.stack_id});
    };
    if !state
        .semantic_snapshots
        .iter()
        .any(|row| row.semantic_snapshot_id == overlay_snapshot.proposed_semantic_snapshot.semantic_snapshot_id)
    {
        state
            .semantic_snapshots
            .push(overlay_snapshot.proposed_semantic_snapshot.clone());
    }
    state.manifests[manifest_index].semantic_snapshot_id =
        overlay_snapshot.proposed_semantic_snapshot.semantic_snapshot_id.clone();
    state.manifests[manifest_index].updated_at = now_iso();

    state.speculative_overlays[overlay_index].status = "merged".to_string();
    state.speculative_overlays[overlay_index].verity_approved = true;
    state.speculative_overlays[overlay_index].approval_note = approval_note.clone();
    state.speculative_overlays[overlay_index].merged_at = Some(now_iso());
    state.speculative_overlays[overlay_index].updated_at = now_iso();

    let receipt = json!({
        "type": "context_stack_speculative_merge",
        "overlay_id": overlay_id,
        "stack_id": overlay_snapshot.stack_id,
        "base_semantic_snapshot_id": overlay_snapshot.base_semantic_snapshot_id,
        "merged_semantic_snapshot_id": overlay_snapshot.proposed_semantic_snapshot.semantic_snapshot_id,
        "verified_merge_gate": true,
        "approval_note_hash": approval_note.as_ref().map(|raw| sha256_hex(raw)),
        "single_step_rollback_ready": true,
        "ts": now_iso(),
    });
    let mut receipt_with_id = receipt.clone();
    receipt_with_id["receipt_id"] = json!(receipt_hash(&receipt));
    state.speculative_overlay_receipts.push(receipt_with_id.clone());
    let _ = persist_context_stacks_state(root, &state);
    let _ = append_context_stacks_receipt(root, &receipt_with_id);
    let _ = append_context_stacks_digestion_log(
        root,
        &overlay_snapshot.stack_id,
        &[format!(
            "speculative_overlay_merged overlay={} merged_snapshot={}",
            overlay_id, overlay_snapshot.proposed_semantic_snapshot.semantic_snapshot_id
        )],
    );
    json!({
        "ok": true,
        "type": "context_stacks_speculative_merge",
        "overlay_id": overlay_id,
        "stack_id": overlay_snapshot.stack_id,
        "merged_semantic_snapshot_id": overlay_snapshot.proposed_semantic_snapshot.semantic_snapshot_id,
        "receipt_id": receipt_with_id.get("receipt_id").cloned().unwrap_or(Value::Null)
    })
}

fn context_stacks_speculative_rollback(root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let mut state = load_context_stacks_state(root);
    let overlay_id = clean(
        parsed
            .flags
            .get("overlay-id")
            .map(String::as_str)
            .unwrap_or(""),
        120,
    );
    if overlay_id.is_empty() {
        return json!({"ok": false, "status": "blocked", "error": "overlay_id_required"});
    }
    let Some(overlay_index) = find_overlay_index(&state, &overlay_id) else {
        return json!({"ok": false, "status": "blocked", "error": "overlay_not_found", "overlay_id": overlay_id});
    };
    let overlay = state.speculative_overlays[overlay_index].clone();
    let reason = clean(
        parsed
            .flags
            .get("reason")
            .map(String::as_str)
            .unwrap_or("manual_rollback"),
        160,
    );
    if let Some(manifest_index) = find_manifest_index(&state, &overlay.stack_id) {
        state.manifests[manifest_index].semantic_snapshot_id = overlay.base_semantic_snapshot_id.clone();
        state.manifests[manifest_index].updated_at = now_iso();
    }
    state.speculative_overlays[overlay_index].status = "rolled_back".to_string();
    state.speculative_overlays[overlay_index].rolled_back_at = Some(now_iso());
    state.speculative_overlays[overlay_index].updated_at = now_iso();
    let receipt = json!({
        "type": "context_stack_speculative_rollback",
        "overlay_id": overlay_id,
        "stack_id": overlay.stack_id,
        "rollback_semantic_snapshot_id": overlay.base_semantic_snapshot_id,
        "reason": reason,
        "single_step": true,
        "ts": now_iso(),
    });
    let mut receipt_with_id = receipt.clone();
    receipt_with_id["receipt_id"] = json!(receipt_hash(&receipt));
    state.speculative_overlay_receipts.push(receipt_with_id.clone());
    let _ = persist_context_stacks_state(root, &state);
    let _ = append_context_stacks_receipt(root, &receipt_with_id);
    let _ = append_context_stacks_digestion_log(
        root,
        &overlay.stack_id,
        &[format!(
            "speculative_overlay_rolled_back overlay={} reason={}",
            overlay_id, reason
        )],
    );
    json!({
        "ok": true,
        "type": "context_stacks_speculative_rollback",
        "overlay_id": overlay_id,
        "stack_id": overlay.stack_id,
        "rollback_semantic_snapshot_id": overlay.base_semantic_snapshot_id,
        "receipt_id": receipt_with_id.get("receipt_id").cloned().unwrap_or(Value::Null)
    })
}

fn context_stacks_speculative_status(root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let state = load_context_stacks_state(root);
    let overlay_filter = clean(
        parsed
            .flags
            .get("overlay-id")
            .map(String::as_str)
            .unwrap_or(""),
        120,
    );
    let stack_filter = clean(
        parsed
            .flags
            .get("stack-id")
            .map(String::as_str)
            .unwrap_or(""),
        120,
    );
    let overlays = state
        .speculative_overlays
        .iter()
        .filter(|row| overlay_filter.is_empty() || row.overlay_id == overlay_filter)
        .filter(|row| stack_filter.is_empty() || row.stack_id == stack_filter)
        .cloned()
        .collect::<Vec<_>>();
    json!({
        "ok": true,
        "type": "context_stacks_speculative_status",
        "overlay_count": overlays.len(),
        "overlays": overlays,
        "receipt_count": state.speculative_overlay_receipts.len()
    })
}

fn context_stacks_contract_verify(root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let state = load_context_stacks_state(root);
    let policy = load_context_stacks_policy(root);
    let stack_id = stack_id_from(parsed);
    if stack_id.is_empty() {
        return json!({"ok": false, "status": "blocked", "error": "stack_id_required"});
    }
    let Some(manifest_index) = find_manifest_index(&state, &stack_id) else {
        return json!({"ok": false, "status": "blocked", "error": "stack_not_found", "stack_id": stack_id});
    };
    let semantic_snapshot_id = state.manifests[manifest_index].semantic_snapshot_id.clone();
    let Some(snapshot) = find_semantic_snapshot(&state, &semantic_snapshot_id).cloned() else {
        return json!({"ok": false, "status": "blocked", "error": "semantic_snapshot_missing", "stack_id": stack_id});
    };

    let mut volatile_mutated = snapshot.clone();
    volatile_mutated.volatile_metadata = json!({
        "verification_probe": "volatile_only_mutation",
        "ts": now_iso()
    });
    let stable_head_id_before = semantic_snapshot_id_for(&snapshot.stable_head);
    let stable_head_id_after = semantic_snapshot_id_for(&volatile_mutated.stable_head);
    let semantic_snapshot_stable_head_contract_ok = stable_head_id_before == stable_head_id_after;

    let base_plan = build_render_plan(parsed, &semantic_snapshot_id, None);
    let mut changed_plan = base_plan.clone();
    changed_plan.response_mode = if base_plan.response_mode == "chat" {
        "json".to_string()
    } else {
        "chat".to_string()
    };
    let base_provider_snapshot = derive_provider_snapshot(&snapshot, &base_plan);
    let changed_provider_snapshot = derive_provider_snapshot(&snapshot, &changed_plan);
    let provider_snapshot_disposable_contract_ok = base_provider_snapshot.derived_disposable
        && changed_provider_snapshot.derived_disposable
        && base_provider_snapshot.semantic_snapshot_id == semantic_snapshot_id
        && base_provider_snapshot.render_plan_id == base_plan.render_plan_id;
    let render_fingerprint_mode_contract_ok =
        base_provider_snapshot.render_fingerprint != changed_provider_snapshot.render_fingerprint;

    let live_batch = normalize_batch_class(
        &base_plan,
        BatchLane::LiveMicrobatch,
        &base_provider_snapshot.render_fingerprint,
    );
    let provider_batch = normalize_batch_class(
        &base_plan,
        BatchLane::ProviderBatch,
        &base_provider_snapshot.render_fingerprint,
    );
    let strict_two_lane_batch_contract_ok =
        batch_class_id_for(&live_batch) != batch_class_id_for(&provider_batch);

    let no_cache = evaluate_scheduler_edge_cases(
        &policy,
        CachePolicy::NoCache,
        policy.cache_threshold_tokens.saturating_sub(1),
        300,
        policy.lookback_window_tokens,
        1,
        false,
    );
    let seed_then_fanout = evaluate_scheduler_edge_cases(
        &policy,
        CachePolicy::Auto,
        policy.cache_threshold_tokens.saturating_add(20),
        640,
        policy.lookback_window_tokens,
        policy.seed_then_fanout_min_cohort.saturating_add(1),
        false,
    );
    let explicit_breakpoint = evaluate_scheduler_edge_cases(
        &policy,
        CachePolicy::ExplicitBreakpoint,
        policy.cache_threshold_tokens.saturating_add(100),
        policy.lookback_window_tokens.saturating_add(1200),
        policy.lookback_window_tokens,
        1,
        true,
    );
    let scheduler_cache_edge_contract_ok = no_cache.scheduler_mode == "no_cache"
        && seed_then_fanout.scheduler_mode == "seed_then_fanout"
        && explicit_breakpoint.cache_hit
        && explicit_breakpoint.breakpoint_mode.as_deref() == Some("explicit_breakpoint");

    let manifest = state.manifests[manifest_index].clone();
    let manifest_semantic_snapshot_ref_contract_ok = state
        .semantic_snapshots
        .iter()
        .any(|row| row.semantic_snapshot_id == manifest.semantic_snapshot_id);
    let active_tail_id_set = manifest
        .active_delta_tail_ids
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let active_tail_ids_unique = active_tail_id_set.len() == manifest.active_delta_tail_ids.len();
    let active_tails = manifest
        .active_delta_tail_ids
        .iter()
        .filter_map(|tail_id| state.delta_tails.iter().find(|tail| tail.tail_id == *tail_id))
        .cloned()
        .collect::<Vec<_>>();
    let active_tail_binding_ok = active_tails.len() == manifest.active_delta_tail_ids.len()
        && active_tails.iter().all(|tail| tail.stack_id == stack_id);
    let manifest_active_delta_tail_contract_ok = manifest_semantic_snapshot_ref_contract_ok
        && active_tail_ids_unique
        && active_tail_binding_ok;

    let mut typed_probe_tail = DeltaTail {
        tail_id: "probe".to_string(),
        stack_id: stack_id.clone(),
        session_id: "verify".to_string(),
        current_objective: "start".to_string(),
        entries: Vec::new(),
        created_at: now_iso(),
        updated_at: now_iso(),
        last_promoted_at: None,
    };
    let merge_note = apply_typed_tail_merge(
        &mut typed_probe_tail,
        "append_working_note",
        "verify typed merge note",
    );
    let merge_turn = apply_typed_tail_merge(
        &mut typed_probe_tail,
        "append_turn",
        "verify typed merge turn",
    );
    let merge_objective = apply_typed_tail_merge(
        &mut typed_probe_tail,
        "replace_objective",
        "updated objective",
    );
    let delta_tail_typed_merge_contract_ok = merge_note == "working_note_appended"
        && merge_turn == "turn_appended"
        && merge_objective == "objective_replaced"
        && typed_probe_tail.current_objective == "updated objective"
        && typed_probe_tail
            .entries
            .iter()
            .all(|entry| matches!(entry.kind.as_str(), "working_note" | "turn"));
    let mut promoted_nodes = snapshot.stable_head.ordered_stable_nodes.clone();
    for row in &typed_probe_tail.entries {
        promoted_nodes.push(clean(format!("[{}] {}", row.kind, row.text), 1000));
    }
    let promoted_head = StableHead {
        system_prompt: snapshot.stable_head.system_prompt.clone(),
        tools: snapshot.stable_head.tools.clone(),
        ordered_stable_nodes: dedupe_preserving_order(promoted_nodes),
    };
    let promoted_snapshot_id = semantic_snapshot_id_for(&promoted_head);
    let delta_tail_promotion_contract_ok = !typed_probe_tail.entries.is_empty()
        && promoted_snapshot_id != semantic_snapshot_id
        && !promoted_head.ordered_stable_nodes.is_empty();

    let all_ok = semantic_snapshot_stable_head_contract_ok
        && provider_snapshot_disposable_contract_ok
        && render_fingerprint_mode_contract_ok
        && strict_two_lane_batch_contract_ok
        && scheduler_cache_edge_contract_ok
        && manifest_active_delta_tail_contract_ok
        && delta_tail_typed_merge_contract_ok
        && delta_tail_promotion_contract_ok;
    let payload = json!({
        "ok": all_ok,
        "type": "context_stacks_contract_verify",
        "stack_id": stack_id,
        "contracts": {
            "semantic_snapshot_stable_head_contract_ok": semantic_snapshot_stable_head_contract_ok,
            "provider_snapshot_disposable_contract_ok": provider_snapshot_disposable_contract_ok,
            "render_fingerprint_mode_contract_ok": render_fingerprint_mode_contract_ok,
            "strict_two_lane_batch_contract_ok": strict_two_lane_batch_contract_ok,
            "scheduler_cache_edge_contract_ok": scheduler_cache_edge_contract_ok,
            "manifest_active_delta_tail_contract_ok": manifest_active_delta_tail_contract_ok,
            "delta_tail_typed_merge_contract_ok": delta_tail_typed_merge_contract_ok,
            "delta_tail_promotion_contract_ok": delta_tail_promotion_contract_ok
        },
        "proof": {
            "stable_head_id_before": stable_head_id_before,
            "stable_head_id_after": stable_head_id_after,
            "base_render_fingerprint": base_provider_snapshot.render_fingerprint,
            "changed_render_fingerprint": changed_provider_snapshot.render_fingerprint,
            "no_cache_mode": no_cache.scheduler_mode,
            "seed_then_fanout_mode": seed_then_fanout.scheduler_mode,
            "explicit_breakpoint_mode": explicit_breakpoint.breakpoint_mode,
            "explicit_breakpoint_cache_hit": explicit_breakpoint.cache_hit,
            "manifest_semantic_snapshot_ref_contract_ok": manifest_semantic_snapshot_ref_contract_ok,
            "active_tail_ids_unique": active_tail_ids_unique,
            "active_tail_binding_ok": active_tail_binding_ok,
            "promoted_snapshot_id_probe": promoted_snapshot_id
        },
        "claim_evidence": [
            {"id":"V6-MEMORY-041.1","claim":"manifest_semantic_snapshot_reference_and_active_delta_tails_contract"},
            {"id":"V6-MEMORY-041.4","claim":"delta_tail_typed_merge_and_promotion_contract"},
            {"id":"V6-MEMORY-041.2","claim":"semantic_snapshot_stable_head_contract"},
            {"id":"V6-MEMORY-041.3","claim":"provider_snapshot_disposable_render_fingerprint_contract"},
            {"id":"V6-MEMORY-041.5","claim":"strict_two_lane_batch_class_contract"},
            {"id":"V6-MEMORY-041.6","claim":"scheduler_cache_edge_contract"}
        ]
    });
    let mut receipt = payload.clone();
    receipt["receipt_id"] = json!(receipt_hash(&payload));
    let _ = append_context_stacks_receipt(root, &receipt);
    let _ = append_context_stacks_digestion_log(
        root,
        &stack_id,
        &[format!(
            "contract_verify ok={} stable_head={} provider_disposable={} batch={} scheduler={}",
            all_ok,
            semantic_snapshot_stable_head_contract_ok,
            provider_snapshot_disposable_contract_ok,
            strict_two_lane_batch_contract_ok,
            scheduler_cache_edge_contract_ok
        )],
    );
    receipt
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = crate::parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|row| row.to_ascii_lowercase())
        .unwrap_or_else(|| "list".to_string());
    let nexus_connection = if context_stacks_nexus_enabled(&parsed) {
        match authorize_context_stacks_command_with_nexus(command.as_str()) {
            Ok(meta) => Some(meta),
            Err(err) => {
                let fail_payload = json!({
                    "ok": false,
                    "status": "blocked",
                    "error": "context_stacks_nexus_error",
                    "reason": clean(err.as_str(), 220),
                    "fail_closed": true
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&fail_payload).unwrap_or_else(|_| {
                        "{\"ok\":false,\"status\":\"blocked\",\"error\":\"encode_failed\"}"
                            .to_string()
                    })
                );
                return 1;
            }
        }
    } else {
        Some(json!({
            "enabled": false,
            "reason": "nexus_disabled_by_flag_or_env"
        }))
    };
    let payload = match command.as_str() {
        "help" | "--help" | "-h" => {
            context_stacks_usage();
            json!({"ok": true, "type": "context_stacks_help"})
        }
        "status" => context_stacks_status(root),
        "policy" => context_stacks_policy_json(root),
        "create" => create_context_stack(root, &parsed),
        "list" => list_context_stacks(root, &parsed),
        "archive" => archive_context_stack(root, &parsed),
        "tail-merge" | "tail_merge" | "tail-append" | "tail_append" => {
            merge_context_stack_tail(root, &parsed)
        }
        "tail-promote" | "tail_promote" => promote_context_stack_tail(root, &parsed),
        "render" => render_context_stack(root, &parsed),
        "batch-class" | "batch_class" => batch_class_context_stack(root, &parsed),
        "scheduler-check" | "scheduler_check" => scheduler_check_context_stack(root, &parsed),
        "node-spike" | "node_spike" | "spike" => context_stacks_node_spike(root, &parsed),
        "contract-verify" | "contract_verify" => context_stacks_contract_verify(root, &parsed),
        "taste-tune" | "taste_tune" => context_stacks_taste_tune(root, &parsed),
        "partial-merge" | "partial_merge" => context_stacks_partial_merge(root, &parsed),
        "hybrid-retrieve" | "hybrid_retrieve" => context_stacks_hybrid_retrieve(root, &parsed),
        "speculative-start" | "speculative_start" => context_stacks_speculative_start(root, &parsed),
        "speculative-merge" | "speculative_merge" => context_stacks_speculative_merge(root, &parsed),
        "speculative-rollback" | "speculative_rollback" => {
            context_stacks_speculative_rollback(root, &parsed)
        }
        "speculative-status" | "speculative_status" => context_stacks_speculative_status(root, &parsed),
        _ => json!({
            "ok": false,
            "status": "blocked",
            "error": "context_stacks_unknown_command",
            "command": command
        }),
    };
    let mut payload = payload;
    if let Some(meta) = nexus_connection {
        payload["nexus_connection"] = meta;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&payload)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
    if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        0
    } else {
        1
    }
}
