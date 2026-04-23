fn context_stacks_usage() {
    println!("context-stacks commands:");
    println!("  infring-ops context-stacks create --stack-id=<id> [--system-prompt=<text>] [--tools=a,b] [--stable-nodes-json='[\"...\"]']");
    println!("  infring-ops context-stacks list [--all=1]");
    println!("  infring-ops context-stacks archive --stack-id=<id>");
    println!("  infring-ops context-stacks tail-merge --stack-id=<id> [--tail-id=<id>] --merge-type=<append_working_note|append_turn|replace_objective> --value=<text>");
    println!("  infring-ops context-stacks tail-promote --stack-id=<id> [--tail-id=<id>]");
    println!("  infring-ops context-stacks render --stack-id=<id>");
    println!("  infring-ops context-stacks batch-class --stack-id=<id> [--lane=live_microbatch|provider_batch]");
    println!("  infring-ops context-stacks scheduler-check --stack-id=<id> [--prompt-tokens=<n>] [--stable-prefix-tokens=<n>] [--fresh-cohort-size=<n>]");
    println!("  infring-ops context-stacks node-spike --stack-id=<id> --node-id=<id> --delta=<0..1> [--staleness-seconds=<n>] [--external-trigger=1]");
    println!("  infring-ops context-stacks contract-verify --stack-id=<id>");
    println!("  infring-ops context-stacks taste-tune --family=<name> --merge-lift=<-1..1>");
    println!("  infring-ops context-stacks partial-merge --stack-id=<id> --patch-json=<json>");
    println!("  infring-ops context-stacks hybrid-retrieve --stack-id=<id> --query=<text> --vector-json=<json> --edges-json=<json> [--top-k=<n>]");
    println!("  infring-ops context-stacks speculative-start --stack-id=<id> --patch-json=<json> [--overlay-id=<id>] [--approval-note=<text>] [--verify-merge=1|0]");
    println!("  infring-ops context-stacks speculative-merge --overlay-id=<id> [--approval-note=<text>] [--verify-merge=1|0]");
    println!("  infring-ops context-stacks speculative-rollback --overlay-id=<id> [--reason=<text>]");
    println!("  infring-ops context-stacks speculative-status [--overlay-id=<id>] [--stack-id=<id>]");
    println!("  infring-ops context-stacks status");
}

fn stack_id_from(parsed: &crate::ParsedArgs) -> String {
    ensure_not_empty(
        clean(
            parsed
                .flags
                .get("stack-id")
                .map(String::as_str)
                .unwrap_or(""),
            120,
        ),
        "",
        120,
    )
}

fn create_context_stack(root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let _ = ensure_workspace_root(root);
    let mut state = load_context_stacks_state(root);
    let stack_id = {
        let requested = stack_id_from(parsed);
        if requested.is_empty() {
            generate_id("stack")
        } else {
            requested
        }
    };
    if state
        .manifests
        .iter()
        .any(|row| row.stack_id == stack_id && !row.archived)
    {
        return json!({
            "ok": false,
            "status": "blocked",
            "error": "context_stack_exists",
            "stack_id": stack_id
        });
    }

    let stable_head = build_stable_head(parsed);
    let semantic_snapshot_id = semantic_snapshot_id_for(&stable_head);
    let volatile_metadata = parse_json_value(parsed.flags.get("volatile-meta-json"));
    let ts = now_iso();
    let semantic_snapshot = SemanticSnapshot {
        semantic_snapshot_id: semantic_snapshot_id.clone(),
        stable_head,
        volatile_metadata,
        created_at: ts.clone(),
        updated_at: ts.clone(),
    };
    if let Some(index) = state
        .semantic_snapshots
        .iter()
        .position(|row| row.semantic_snapshot_id == semantic_snapshot_id)
    {
        state.semantic_snapshots[index].volatile_metadata = semantic_snapshot.volatile_metadata.clone();
        state.semantic_snapshots[index].updated_at = ts.clone();
    } else {
        state.semantic_snapshots.push(semantic_snapshot.clone());
    }

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
    let provider_snapshot = derive_provider_snapshot(&semantic_snapshot, &plan);
    if let Some(index) = state
        .provider_snapshots
        .iter()
        .position(|row| row.render_fingerprint == provider_snapshot.render_fingerprint)
    {
        state.provider_snapshots[index] = provider_snapshot.clone();
    } else {
        state.provider_snapshots.push(provider_snapshot.clone());
    }

    let objective = clean(
        parsed
            .flags
            .get("objective")
            .map(String::as_str)
            .unwrap_or(""),
        500,
    );
    let mut active_tail_ids = Vec::<String>::new();
    if !objective.is_empty() {
        let tail_id = generate_id("tail");
        let tail = DeltaTail {
            tail_id: tail_id.clone(),
            stack_id: stack_id.clone(),
            session_id: clean(
                parsed
                    .flags
                    .get("session-id")
                    .map(String::as_str)
                    .unwrap_or("session"),
                120,
            ),
            current_objective: objective,
            entries: Vec::new(),
            created_at: ts.clone(),
            updated_at: ts.clone(),
            last_promoted_at: None,
        };
        state.delta_tails.push(tail);
        active_tail_ids.push(tail_id);
    }

    let manifest = ContextStackManifest {
        stack_id: stack_id.clone(),
        semantic_snapshot_id: semantic_snapshot_id.clone(),
        active_delta_tail_ids: active_tail_ids.clone(),
        archived: false,
        created_at: ts.clone(),
        updated_at: ts.clone(),
    };
    if let Some(index) = find_manifest_index(&state, &stack_id) {
        state.manifests[index] = manifest.clone();
    } else {
        state.manifests.push(manifest.clone());
    }

    state
        .manifests
        .sort_by(|a, b| a.stack_id.to_ascii_lowercase().cmp(&b.stack_id.to_ascii_lowercase()));
    let _ = persist_context_stacks_state(root, &state);

    let batch_class =
        normalize_batch_class(&plan, BatchLane::LiveMicrobatch, &provider_snapshot.render_fingerprint);
    let batch_id = batch_class_id_for(&batch_class);
    let decision = SchedulerEdgeCaseDecision {
        scheduler_mode: "single_shot".to_string(),
        cache_hit: false,
        cache_creation_input_tokens: provider_snapshot_token_estimate(&provider_snapshot.serialized_prefix),
        cache_read_input_tokens: 0,
        seed_then_fanout: false,
        breakpoint_mode: None,
    };
    let receipt = receipt_with_common_fields(
        "context_stack_create",
        &stack_id,
        "created",
        Some(batch_id),
        Some(&decision),
    );
    let _ = append_context_stacks_receipt(root, &receipt);
    let _ = append_context_stacks_digestion_log(
        root,
        &stack_id,
        &[
            "context_stack_manifest_created".to_string(),
            format!("semantic_snapshot_id={semantic_snapshot_id}"),
            format!("render_fingerprint={}", provider_snapshot.render_fingerprint),
        ],
    );

    json!({
        "ok": true,
        "status": "ok",
        "type": "context_stacks_create",
        "stack": manifest,
        "semantic_snapshot": semantic_snapshot,
        "render_plan": plan,
        "provider_snapshot": provider_snapshot,
        "receipt_id": receipt.get("receipt_id").cloned().unwrap_or(Value::Null)
    })
}

fn list_context_stacks(root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let state = load_context_stacks_state(root);
    let include_archived = truthy(parsed.flags.get("all")) || truthy(parsed.flags.get("include-archived"));
    let stacks = state
        .manifests
        .iter()
        .filter(|row| include_archived || !row.archived)
        .cloned()
        .collect::<Vec<_>>();
    json!({
        "ok": true,
        "type": "context_stacks_list",
        "count": stacks.len(),
        "include_archived": include_archived,
        "stacks": stacks
    })
}

fn archive_context_stack(root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let mut state = load_context_stacks_state(root);
    let stack_id = stack_id_from(parsed);
    if stack_id.is_empty() {
        return json!({"ok": false, "status": "blocked", "error": "stack_id_required"});
    }
    let Some(index) = find_manifest_index(&state, &stack_id) else {
        return json!({"ok": false, "status": "blocked", "error": "stack_not_found", "stack_id": stack_id});
    };
    state.manifests[index].archived = true;
    state.manifests[index].updated_at = now_iso();
    let _ = persist_context_stacks_state(root, &state);

    let receipt = receipt_with_common_fields(
        "context_stack_archive",
        &stack_id,
        "archived",
        None,
        None,
    );
    let _ = append_context_stacks_receipt(root, &receipt);
    let _ = append_context_stacks_digestion_log(
        root,
        &stack_id,
        &[String::from("context_stack_archived")],
    );
    json!({
        "ok": true,
        "type": "context_stacks_archive",
        "stack_id": stack_id,
        "receipt_id": receipt.get("receipt_id").cloned().unwrap_or(Value::Null)
    })
}

fn ensure_tail_index(
    state: &mut ContextStacksState,
    stack_id: &str,
    tail_id_flag: Option<&String>,
) -> usize {
    let requested = clean(tail_id_flag.map(String::as_str).unwrap_or(""), 120);
    if !requested.is_empty() {
        if let Some(index) = find_tail_index(state, &requested) {
            return index;
        }
        state.delta_tails.push(DeltaTail {
            tail_id: requested.clone(),
            stack_id: stack_id.to_string(),
            session_id: "session".to_string(),
            current_objective: String::new(),
            entries: Vec::new(),
            created_at: now_iso(),
            updated_at: now_iso(),
            last_promoted_at: None,
        });
        return state.delta_tails.len() - 1;
    }
    if let Some(manifest) = state.manifests.iter().find(|row| row.stack_id == stack_id) {
        for tail_id in &manifest.active_delta_tail_ids {
            if let Some(index) = find_tail_index(state, tail_id) {
                return index;
            }
        }
    }
    let generated = generate_id("tail");
    state.delta_tails.push(DeltaTail {
        tail_id: generated.clone(),
        stack_id: stack_id.to_string(),
        session_id: "session".to_string(),
        current_objective: String::new(),
        entries: Vec::new(),
        created_at: now_iso(),
        updated_at: now_iso(),
        last_promoted_at: None,
    });
    if let Some(manifest_index) = find_manifest_index(state, stack_id) {
        if !state.manifests[manifest_index]
            .active_delta_tail_ids
            .contains(&generated)
        {
            state.manifests[manifest_index].active_delta_tail_ids.push(generated);
        }
    }
    state.delta_tails.len() - 1
}

fn apply_typed_tail_merge(tail: &mut DeltaTail, merge_type: &str, value: &str) -> String {
    let merge_key = clean(merge_type, 80).to_ascii_lowercase();
    let value_clean = clean(value, 4000);
    match merge_key.as_str() {
        "replace_objective" => {
            tail.current_objective = value_clean;
            "objective_replaced".to_string()
        }
        "append_turn" => {
            tail.entries.push(DeltaTailEntry {
                kind: "turn".to_string(),
                text: value_clean,
                ts: now_iso(),
            });
            "turn_appended".to_string()
        }
        _ => {
            tail.entries.push(DeltaTailEntry {
                kind: "working_note".to_string(),
                text: value_clean,
                ts: now_iso(),
            });
            "working_note_appended".to_string()
        }
    }
}

fn merge_context_stack_tail(root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let mut state = load_context_stacks_state(root);
    let stack_id = stack_id_from(parsed);
    if stack_id.is_empty() {
        return json!({"ok": false, "status": "blocked", "error": "stack_id_required"});
    }
    let Some(manifest_index) = find_manifest_index(&state, &stack_id) else {
        return json!({"ok": false, "status": "blocked", "error": "stack_not_found", "stack_id": stack_id});
    };
    if state.manifests[manifest_index].archived {
        return json!({"ok": false, "status": "blocked", "error": "stack_archived", "stack_id": stack_id});
    }
    let tail_index = ensure_tail_index(&mut state, &stack_id, parsed.flags.get("tail-id"));
    let merge_type = parsed
        .flags
        .get("merge-type")
        .map(String::as_str)
        .unwrap_or("append_working_note");
    let value = parsed
        .flags
        .get("value")
        .or_else(|| parsed.flags.get("text"))
        .map(String::as_str)
        .unwrap_or("");
    if clean(value, 20).is_empty() {
        return json!({"ok": false, "status": "blocked", "error": "merge_value_required"});
    }
    let merge_outcome = apply_typed_tail_merge(&mut state.delta_tails[tail_index], merge_type, value);
    state.delta_tails[tail_index].updated_at = now_iso();
    state.manifests[manifest_index].updated_at = now_iso();
    let _ = persist_context_stacks_state(root, &state);

    let receipt = receipt_with_common_fields(
        "context_stack_tail_merge",
        &stack_id,
        &merge_outcome,
        None,
        None,
    );
    let _ = append_context_stacks_receipt(root, &receipt);
    let _ = append_context_stacks_digestion_log(
        root,
        &stack_id,
        &[format!("tail_merge:{merge_outcome}")],
    );
    json!({
        "ok": true,
        "type": "context_stacks_tail_merge",
        "stack_id": stack_id,
        "tail": state.delta_tails[tail_index],
        "merge_outcome": merge_outcome,
        "receipt_id": receipt.get("receipt_id").cloned().unwrap_or(Value::Null)
    })
}

fn promote_context_stack_tail(root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let mut state = load_context_stacks_state(root);
    let stack_id = stack_id_from(parsed);
    if stack_id.is_empty() {
        return json!({"ok": false, "status": "blocked", "error": "stack_id_required"});
    }
    let Some(manifest_index) = find_manifest_index(&state, &stack_id) else {
        return json!({"ok": false, "status": "blocked", "error": "stack_not_found", "stack_id": stack_id});
    };
    let tail_index = ensure_tail_index(&mut state, &stack_id, parsed.flags.get("tail-id"));
    let previous_semantic_snapshot_id = state.manifests[manifest_index].semantic_snapshot_id.clone();
    let Some(current_snapshot) = find_semantic_snapshot(&state, &previous_semantic_snapshot_id).cloned() else {
        return json!({"ok": false, "status": "blocked", "error": "semantic_snapshot_missing", "semantic_snapshot_id": previous_semantic_snapshot_id});
    };
    let mut promoted_nodes = current_snapshot.stable_head.ordered_stable_nodes.clone();
    for row in &state.delta_tails[tail_index].entries {
        let line = clean(format!("[{}] {}", clean(&row.kind, 60), row.text), 1000);
        if !line.is_empty() {
            promoted_nodes.push(line);
        }
    }
    promoted_nodes = dedupe_preserving_order(promoted_nodes);
    let promoted_head = StableHead {
        system_prompt: current_snapshot.stable_head.system_prompt.clone(),
        tools: current_snapshot.stable_head.tools.clone(),
        ordered_stable_nodes: promoted_nodes,
    };
    let semantic_snapshot_id = semantic_snapshot_id_for(&promoted_head);
    let promoted_snapshot = SemanticSnapshot {
        semantic_snapshot_id: semantic_snapshot_id.clone(),
        stable_head: promoted_head,
        volatile_metadata: current_snapshot.volatile_metadata.clone(),
        created_at: now_iso(),
        updated_at: now_iso(),
    };
    if let Some(index) = state
        .semantic_snapshots
        .iter()
        .position(|row| row.semantic_snapshot_id == semantic_snapshot_id)
    {
        state.semantic_snapshots[index].updated_at = now_iso();
    } else {
        state.semantic_snapshots.push(promoted_snapshot.clone());
    }
    state.manifests[manifest_index].semantic_snapshot_id = semantic_snapshot_id.clone();
    state.manifests[manifest_index].updated_at = now_iso();
    state.delta_tails[tail_index].entries.clear();
    state.delta_tails[tail_index].last_promoted_at = Some(now_iso());
    state.delta_tails[tail_index].updated_at = now_iso();
    let _ = persist_context_stacks_state(root, &state);

    let receipt = receipt_with_common_fields(
        "context_stack_tail_promote",
        &stack_id,
        "tail_promoted",
        None,
        None,
    );
    let _ = append_context_stacks_receipt(root, &receipt);
    let _ = append_context_stacks_digestion_log(
        root,
        &stack_id,
        &[format!("tail_promoted:{}", semantic_snapshot_id)],
    );
    json!({
        "ok": true,
        "type": "context_stacks_tail_promote",
        "stack_id": stack_id,
        "semantic_snapshot_id": semantic_snapshot_id,
        "receipt_id": receipt.get("receipt_id").cloned().unwrap_or(Value::Null)
    })
}
