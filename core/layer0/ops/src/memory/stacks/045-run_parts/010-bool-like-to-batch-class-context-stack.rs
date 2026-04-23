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
            std::env::var("INFRING_HIERARCHICAL_NEXUS_V1")
                .ok()
                .map(|raw| bool_like(raw.as_str()))
        })
        .unwrap_or(true)
}

fn context_stacks_force_block_pair_enabled() -> bool {
    std::env::var("INFRING_HIERARCHICAL_NEXUS_BLOCK_CONTEXT_STACKS_ROUTE")
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
