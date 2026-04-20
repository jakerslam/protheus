
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
