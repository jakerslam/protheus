
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
