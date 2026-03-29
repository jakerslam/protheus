fn execute_openclaw_detachment_contract(
    root: &Path,
    profile: RuntimeSystemContractProfile,
    payload: &Value,
    apply: bool,
    strict: bool,
) -> Result<ContractExecution, String> {
    let (state_path, mut state, state_rel) = load_contract_state(root, profile);
    let source_root = source_path_from_payload(root, payload, "source_root", "..");
    let source_nursery = source_root.join("nursery");
    let assimilation_root = root.join("local/state/assimilations/openclaw");
    let nursery_root = root.join("local/state/nursery");
    let source_control_root = root.join("config/openclaw_assimilation");

    if strict && !source_root.exists() {
        return Err(format!(
            "openclaw_source_root_missing:{}",
            lane_utils::rel_path(root, &source_root)
        ));
    }

    let mut copied_rows = Vec::<Value>::new();
    let mut source_control_copied_rows = Vec::<Value>::new();
    let copy_plan = vec![
        (
            source_root.join("openclaw.json"),
            assimilation_root.join("openclaw.json"),
        ),
        (
            source_root.join("MEMORY_INDEX.md"),
            assimilation_root.join("memory/MEMORY_INDEX.md"),
        ),
        (
            source_root.join("TAGS_INDEX.md"),
            assimilation_root.join("memory/TAGS_INDEX.md"),
        ),
        (
            source_root.join("cron/jobs.json"),
            assimilation_root.join("cron/jobs.json"),
        ),
        (
            source_root.join("memory/main.sqlite"),
            assimilation_root.join("memory/main.sqlite"),
        ),
        (
            source_root.join("subagents/runs.json"),
            assimilation_root.join("subagents/runs.json"),
        ),
        (
            source_root.join("client/local/memory/.rebuild_delta_cache.json"),
            assimilation_root.join("memory/rebuild_delta_cache.json"),
        ),
        (
            source_root.join("local/state/sensory/eyes/collector_rate_state.json"),
            assimilation_root.join("sensory/eyes/collector_rate_state.json"),
        ),
        (
            source_root.join("devices/paired.json"),
            assimilation_root.join("devices/paired.json"),
        ),
        (
            source_root.join("devices/pending.json"),
            assimilation_root.join("devices/pending.json"),
        ),
        (
            source_root.join("identity/device.json"),
            assimilation_root.join("identity/device.json"),
        ),
        (
            source_root.join("identity/device-auth.json"),
            assimilation_root.join("identity/device-auth.json"),
        ),
        (
            source_root.join("agents/main/agent/state.json"),
            assimilation_root.join("agents/main/agent/state.json"),
        ),
        (
            source_root.join("agents/main/agent/models.json"),
            assimilation_root.join("agents/main/agent/models.json"),
        ),
        (
            source_root.join("agents/main/agent/routing-policy.json"),
            assimilation_root.join("agents/main/agent/routing-policy.json"),
        ),
        (
            source_root.join("agents/main/sessions/sessions.json"),
            assimilation_root.join("agents/main/sessions/sessions.json"),
        ),
        (
            source_nursery.join("containment/permissions.json"),
            nursery_root.join("containment/permissions.json"),
        ),
        (
            source_nursery.join("containment/policy-gates.json"),
            nursery_root.join("containment/policy-gates.json"),
        ),
        (
            source_nursery.join("manifests/seed_manifest.json"),
            nursery_root.join("manifests/seed_manifest.json"),
        ),
    ];

    for (source, destination) in copy_plan {
        if let Some(row) = copy_file_if_present(root, &source, &destination, apply)? {
            copied_rows.push(row);
        }
    }

    let source_control_copy_plan = vec![
        (
            source_root.join("cron/jobs.json"),
            source_control_root.join("cron/jobs.json"),
        ),
        (
            source_nursery.join("containment/permissions.json"),
            source_control_root.join("nursery/containment/permissions.json"),
        ),
        (
            source_nursery.join("containment/policy-gates.json"),
            source_control_root.join("nursery/containment/policy-gates.json"),
        ),
        (
            source_nursery.join("manifests/seed_manifest.json"),
            source_control_root.join("nursery/manifests/seed_manifest.json"),
        ),
        (
            source_root.join("agents/main/sessions/sessions.json"),
            source_control_root.join("agents/main/sessions/sessions.json"),
        ),
    ];
    for (source, destination) in source_control_copy_plan {
        if let Some(row) = copy_file_if_present(root, &source, &destination, apply)? {
            source_control_copied_rows.push(row);
        }
    }

    let tree_copy_plan = vec![
        (
            source_root.join("cron/runs"),
            assimilation_root.join("cron/runs"),
        ),
        (source_nursery.join("logs"), nursery_root.join("logs")),
        (
            source_nursery.join("promotion"),
            nursery_root.join("promotion"),
        ),
        (
            source_nursery.join("quarantine"),
            nursery_root.join("quarantine"),
        ),
        (source_nursery.join("seeds"), nursery_root.join("seeds")),
        (
            source_root.join("agents/main/sessions"),
            assimilation_root.join("agents/main/sessions"),
        ),
    ];
    for (source_tree, destination_tree) in tree_copy_plan {
        let rows = copy_tree_files_if_present(root, &source_tree, &destination_tree, apply)?;
        if !rows.is_empty() {
            copied_rows.extend(rows);
        }
    }

    let permissions_path = nursery_root.join("containment/permissions.json");
    let policy_gates_path = nursery_root.join("containment/policy-gates.json");
    let seed_manifest_path = nursery_root.join("manifests/seed_manifest.json");
    let mut policy_synced = false;
    let policy_path = root.join("client/runtime/config/nursery_policy.json");
    let mut specialist_count = 0usize;
    let mut training_plan_rel = String::new();
    let mut llm_registry_rel = String::new();
    let mut llm_model_count = 0usize;
    let mut recommended_local_model = String::new();

    let permissions = read_json_if_exists(&permissions_path)
        .or_else(|| read_json_if_exists(&source_nursery.join("containment/permissions.json")))
        .unwrap_or_else(|| json!({}));
    let policy_gates = read_json_if_exists(&policy_gates_path)
        .or_else(|| read_json_if_exists(&source_nursery.join("containment/policy-gates.json")))
        .unwrap_or_else(|| json!({}));
    let seed_manifest = read_json_if_exists(&seed_manifest_path)
        .or_else(|| read_json_if_exists(&source_nursery.join("manifests/seed_manifest.json")))
        .unwrap_or_else(|| json!({}));

    if apply {
        let mut policy = read_json_if_exists(&policy_path).unwrap_or_else(|| json!({}));
        if policy
            .get("containment")
            .and_then(Value::as_object)
            .is_none()
        {
            policy["containment"] = json!({});
        }
        policy["root_dir"] = Value::String("local/state/nursery".to_string());
        policy["fallback_repo_root_dir"] =
            Value::String("local/state/nursery/containment".to_string());
        if !permissions.is_null() {
            policy["containment"]["permissions"] = permissions.clone();
        }
        if !policy_gates.is_null() {
            policy["containment"]["policy_gates"] = policy_gates.clone();
        }
        let assimilated_artifacts = openclaw_seed_to_model_artifacts(&seed_manifest);
        if !assimilated_artifacts.is_empty() {
            policy["model_artifacts"] = Value::Array(assimilated_artifacts);
        }
        lane_utils::write_json(&policy_path, &policy)?;
        policy_synced = true;
    }

    if profile.id == "V6-OPENCLAW-DETACH-001.2" {
        let max_train_minutes = permissions
            .get("max_train_minutes")
            .and_then(Value::as_u64)
            .unwrap_or(30);
        let specialists = seed_manifest
            .get("artifacts")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(|row| {
                        let id = row.get("id").and_then(Value::as_str)?.trim();
                        let model = row.get("model").and_then(Value::as_str)?.trim();
                        let provider = row.get("provider").and_then(Value::as_str)?.trim();
                        if id.is_empty() || model.is_empty() || provider.is_empty() {
                            return None;
                        }
                        let required = row
                            .get("required")
                            .and_then(Value::as_bool)
                            .unwrap_or(false);
                        Some(json!({
                            "specialist_id": format!("nursery-{id}"),
                            "seed_id": id,
                            "provider": provider,
                            "model": model,
                            "tier": if required { "primary" } else { "shadow" },
                            "max_train_minutes": max_train_minutes,
                        }))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        specialist_count = specialists.len();
        if strict && specialist_count == 0 {
            return Err("openclaw_nursery_seed_manifest_empty".to_string());
        }
        let training_plan_path = nursery_root.join("promotion/specialist_training_plan.json");
        training_plan_rel = lane_utils::rel_path(root, &training_plan_path);
        if apply {
            lane_utils::write_json(
                &training_plan_path,
                &json!({
                    "ts": now_iso(),
                    "source": lane_utils::rel_path(root, &source_root),
                    "specialists": specialists,
                    "max_train_minutes": max_train_minutes,
                    "claim_evidence": [{
                        "id": profile.id,
                        "claim": "nursery_specialists_are_assimilated_from_openclaw_seed_artifacts_with_local_policy_bounds"
                    }]
                }),
            )?;
        }
    }

    if profile.id == "V6-OPENCLAW-DETACH-001.4" {
        let mut llm_models = openclaw_seed_to_llm_models(&seed_manifest);
        if strict && llm_models.is_empty() {
            return Err("openclaw_detach_missing_llm_seed_models".to_string());
        }
        normalize_model_scores(&mut llm_models);
        llm_model_count = llm_models.len();
        let recommended_local = choose_best_model(
            &llm_models,
            &RoutingRequest {
                workload: WorkloadClass::Coding,
                min_context_tokens: 8_192,
                max_cost_score_1_to_5: 5,
                local_only: true,
            },
        );
        if let Some(best_local) = recommended_local {
            recommended_local_model = best_local.name;
        }
        let registry = json!({
            "version": "1.0",
            "ts": now_iso(),
            "source": lane_utils::rel_path(root, &source_root),
            "models": llm_models.iter().map(llm_model_to_json).collect::<Vec<_>>(),
            "recommended_local_model": if recommended_local_model.is_empty() { Value::Null } else { Value::String(recommended_local_model.clone()) }
        });
        let llm_registry_path = root.join("local/state/llm_runtime/model_registry.json");
        let source_registry_path = source_control_root.join("llm/model_registry.json");
        llm_registry_rel = lane_utils::rel_path(root, &llm_registry_path);
        if apply {
            lane_utils::write_json(&llm_registry_path, &registry)?;
            lane_utils::write_json(&source_registry_path, &registry)?;
        }
    }

    if strict && copied_rows.is_empty() {
        return Err("openclaw_assimilation_no_artifacts_copied".to_string());
    }
    if strict
        && profile.id == "V6-OPENCLAW-DETACH-001.3"
        && source_control_copied_rows.is_empty()
        && !source_control_root.join("cron/jobs.json").exists()
    {
        return Err("openclaw_detach_source_control_mirror_empty".to_string());
    }

    let copied_bytes = copied_rows
        .iter()
        .map(|row| row.get("bytes").and_then(Value::as_u64).unwrap_or(0))
        .sum::<u64>();
    let copied_mb = (copied_bytes as f64) / (1024.0 * 1024.0);
    let summary = json!({
        "family": profile.family,
        "contract_id": profile.id,
        "source_root": lane_utils::rel_path(root, &source_root),
        "copied_count": copied_rows.len(),
        "copied_bytes": copied_bytes,
        "copied_mb": copied_mb,
        "copied": copied_rows,
        "source_control_copied_count": source_control_copied_rows.len(),
        "source_control_copied": source_control_copied_rows,
        "source_control_root": lane_utils::rel_path(root, &source_control_root),
        "policy_synced": policy_synced,
        "specialist_count": specialist_count,
        "training_plan_path": training_plan_rel,
        "llm_registry_path": llm_registry_rel,
        "llm_model_count": llm_model_count,
        "recommended_local_model": if recommended_local_model.is_empty() { Value::Null } else { Value::String(recommended_local_model.clone()) },
        "state_path": state_rel,
        "assimilation_root": lane_utils::rel_path(root, &assimilation_root),
        "nursery_root": lane_utils::rel_path(root, &nursery_root),
    });

    if apply {
        upsert_contract_state_entry(
            &mut state,
            profile.id,
            json!({ "summary": summary, "applied_at": now_iso() }),
        );
        lane_utils::write_json(&state_path, &state)?;
    }

    let claim = if profile.id == "V6-OPENCLAW-DETACH-001.2" {
        "openclaw_nursery_seed_training_is_materialized_locally_with_specialist_plan_and_receipts"
    } else if profile.id == "V6-OPENCLAW-DETACH-001.3" {
        "openclaw_cron_and_nursery_contracts_are_mirrored_into_source_controlled_infring_paths"
    } else if profile.id == "V6-OPENCLAW-DETACH-001.4" {
        "llm_runtime_registry_is_bootstrapped_from_assimilated_seed_models_with_deterministic_power_cost_ranking"
    } else {
        "openclaw_operator_state_and_nursery_artifacts_are_assimilated_into_infring_owned_paths_with_detachment_controls"
    };
    Ok(ContractExecution {
        summary,
        claims: vec![json!({
            "id": profile.id,
            "claim": claim,
            "evidence": {
                "copied_count": copied_rows.len(),
                "copied_mb": copied_mb,
                "policy_synced": policy_synced,
                "specialist_count": specialist_count
            }
        })],
        artifacts: vec![
            state_rel,
            lane_utils::rel_path(root, &assimilation_root),
            lane_utils::rel_path(root, &nursery_root),
            lane_utils::rel_path(root, &source_control_root),
        ],
    })
}

fn execute_contract_profile(
    root: &Path,
    profile: RuntimeSystemContractProfile,
    payload: &Value,
    apply: bool,
    strict: bool,
) -> Result<ContractExecution, String> {
    match profile.family {
        "v5_hold_remediation" => execute_v5_hold_contract(root, profile, payload, apply),
        "v5_rust_hybrid" => execute_v5_rust_hybrid_contract(root, profile, payload, apply, strict),
        "v5_rust_productivity" => {
            execute_v5_rust_productivity_contract(root, profile, payload, apply, strict)
        }
        "execution_streaming_stack" => {
            execute_execution_streaming_contract(root, profile, payload, apply, strict)
        }
        "execution_worktree_stack" => {
            execute_execution_worktree_contract(root, profile, payload, apply, strict)
        }
        "assimilate_fast_stack" => {
            execute_assimilate_fast_contract(root, profile, payload, apply, strict)
        }
        "workflow_open_swe_stack" => {
            execute_workflow_open_swe_contract(root, profile, payload, apply, strict)
        }
        "memory_context_maintenance" => {
            execute_memory_context_contract(root, profile, payload, apply, strict)
        }
        "integration_lakehouse_stack" => {
            execute_integration_lakehouse_contract(root, profile, payload, apply, strict)
        }
        "inference_adaptive_routing" => {
            execute_inference_adaptive_contract(root, profile, payload, apply, strict)
        }
        "runtime_cleanup_autonomous" => {
            execute_runtime_cleanup_contract(root, profile, payload, apply, strict)
        }
        "erp_agentic_stack" => execute_erp_agentic_contract(root, profile, payload, apply, strict),
        "tooling_uv_ruff_stack" => {
            execute_tooling_uv_ruff_contract(root, profile, payload, apply, strict)
        }
        "workflow_visual_bridge_stack" => {
            execute_workflow_visual_bridge_contract(root, profile, payload, apply, strict)
        }
        "openclaw_detachment_stack" => {
            execute_openclaw_detachment_contract(root, profile, payload, apply, strict)
        }
        _ => execute_generic_family_contract(root, profile, payload, apply, strict),
    }
}

fn read_only_command(command: &str) -> bool {
    matches!(command, "status" | "verify")
}

fn system_id_from_args(command: &str, args: &[String]) -> String {
    let by_flag = lane_utils::parse_flag(args, "system-id", true)
        .or_else(|| lane_utils::parse_flag(args, "lane-id", true))
        .or_else(|| lane_utils::parse_flag(args, "id", true));
    if by_flag.is_some() {
        return lane_utils::clean_token(by_flag.as_deref(), "runtime-system");
    }
    if command.starts_with('v')
        && command
            .chars()
            .any(|ch| ch.is_ascii_digit() || matches!(ch, '-' | '_' | '.'))
    {
        return lane_utils::clean_token(Some(command), "runtime-system");
    }
    lane_utils::clean_token(None, "runtime-system")
}
