
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
