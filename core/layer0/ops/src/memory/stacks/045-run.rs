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
            "batch_classes": state.batch_classes.len()
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
