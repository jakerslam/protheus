
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
