
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
