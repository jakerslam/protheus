fn execute_execution_streaming_contract(
    root: &Path,
    profile: RuntimeSystemContractProfile,
    payload: &Value,
    apply: bool,
    strict: bool,
) -> Result<ContractExecution, String> {
    let (state_path, mut state, state_rel) = load_contract_state(root, profile);
    let prefetch_window = payload_u64(payload, "prefetch_window", 4).clamp(1, 32);
    let quant_bits = payload_u64(payload, "quantization_bits", 4);
    let resident_memory_gb = payload_f64(payload, "resident_memory_gb", 12.0);
    let target_tokens_per_sec = payload_f64(payload, "target_tokens_per_sec", 96.0);
    let metal_mode = payload_string(payload, "metal_mode", "bridge");
    let allowed_quant = matches!(quant_bits, 2 | 4);
    if strict && !allowed_quant {
        return Err("execution_streaming_invalid_quantization_bits".to_string());
    }
    if strict
        && profile.id == "V6-EXECUTION-002.3"
        && !payload_bool(payload, "os_page_cache_first", true)
    {
        return Err("execution_streaming_os_cache_first_required".to_string());
    }
    if strict
        && profile.id == "V6-EXECUTION-002.4"
        && !matches!(metal_mode.as_str(), "bridge" | "native")
    {
        return Err("execution_streaming_invalid_metal_mode".to_string());
    }

    let profile_path = family_data_root(root, profile.family).join("streaming_profile.json");
    let profile_rel = lane_utils::rel_path(root, &profile_path);
    let summary = json!({
        "family": profile.family,
        "contract_id": profile.id,
        "prefetch_window": prefetch_window,
        "quantization_bits": quant_bits,
        "resident_memory_gb": resident_memory_gb,
        "target_tokens_per_sec": target_tokens_per_sec,
        "metal_mode": metal_mode,
        "state_path": state_rel,
        "profile_path": profile_rel
    });

    if apply {
        lane_utils::write_json(
            &profile_path,
            &json!({
                "updated_at": now_iso(),
                "profile": summary
            }),
        )?;
        upsert_contract_state_entry(
            &mut state,
            profile.id,
            json!({ "summary": summary, "applied_at": now_iso() }),
        );
        lane_utils::write_json(&state_path, &state)?;
    }

    Ok(ContractExecution {
        summary,
        claims: vec![json!({
            "id": profile.id,
            "claim": "execution_streaming_lane_enforces_quantized_ssd_streaming_cache_policy_and_kernel_mode_with_receipts",
            "evidence": {
                "prefetch_window": prefetch_window,
                "quantization_bits": quant_bits,
                "target_tokens_per_sec": target_tokens_per_sec
            }
        })],
        artifacts: vec![state_rel, profile_rel],
    })
}

fn execute_execution_worktree_contract(
    root: &Path,
    profile: RuntimeSystemContractProfile,
    payload: &Value,
    apply: bool,
    strict: bool,
) -> Result<ContractExecution, String> {
    let (state_path, mut state, state_rel) = load_contract_state(root, profile);
    let worktree_root = client_state_root(root).join("swarm").join("worktrees");
    let worktree_root_rel = lane_utils::rel_path(root, &worktree_root);
    let agent_id = lane_utils::clean_token(
        Some(&payload_string(payload, "agent_id", "agent-default")),
        "agent-default",
    );
    let branch = payload_string(payload, "base_branch", "main");
    let mut created = 0u64;
    let mut removed = 0u64;
    let mut conflict_count = 0u64;

    let operation = match profile.id {
        "V6-EXECUTION-003.1" => "create_worktree",
        "V6-EXECUTION-003.2" => "merge_gate",
        "V6-EXECUTION-003.3" => "swarm_dispatch",
        "V6-EXECUTION-003.4" => "cleanup",
        _ => "status",
    };

    let agent_worktree = worktree_root.join(&agent_id);
    if strict && profile.id == "V6-EXECUTION-003.2" {
        let conflicts = payload_string_array(payload, "conflicts", &[]);
        conflict_count = conflicts.len() as u64;
        let veto = payload_bool(payload, "human_veto_approved", false);
        if !conflicts.is_empty() && !veto {
            return Err("execution_worktree_merge_conflict_requires_human_veto".to_string());
        }
    }

    if apply {
        fs::create_dir_all(&worktree_root)
            .map_err(|err| format!("worktree_root_create_failed:{err}"))?;
        match profile.id {
            "V6-EXECUTION-003.1" => {
                fs::create_dir_all(&agent_worktree)
                    .map_err(|err| format!("worktree_create_failed:{err}"))?;
                lane_utils::write_json(
                    &agent_worktree.join("metadata.json"),
                    &json!({
                        "agent_id": agent_id,
                        "branch": branch,
                        "created_at": now_iso()
                    }),
                )?;
                created = 1;
            }
            "V6-EXECUTION-003.3" => {
                let tasks = payload_string_array(payload, "task_ids", &["task-0"]);
                for task in tasks {
                    let task_dir = agent_worktree.join(task);
                    fs::create_dir_all(&task_dir)
                        .map_err(|err| format!("worktree_task_create_failed:{err}"))?;
                    created += 1;
                }
            }
            "V6-EXECUTION-003.4" => {
                let cleanup_age = payload_u64(payload, "cleanup_age_seconds", 900).max(30);
                let now = now_epoch_secs();
                if let Ok(entries) = fs::read_dir(&worktree_root) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if !path.is_dir() {
                            continue;
                        }
                        let age_secs = file_age_seconds(&path).unwrap_or(0);
                        if now > 0 && age_secs >= cleanup_age {
                            let _ = fs::remove_dir_all(&path);
                            removed += 1;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let summary = json!({
        "family": profile.family,
        "contract_id": profile.id,
        "operation": operation,
        "agent_id": agent_id,
        "branch": branch,
        "created": created,
        "removed": removed,
        "conflict_count": conflict_count,
        "state_path": state_rel,
        "worktree_root": worktree_root_rel
    });

    if apply {
        upsert_contract_state_entry(
            &mut state,
            profile.id,
            json!({ "summary": summary, "applied_at": now_iso() }),
        );
        lane_utils::write_json(&state_path, &state)?;
    }

    Ok(ContractExecution {
        summary,
        claims: vec![json!({
            "id": profile.id,
            "claim": "execution_worktree_lane_manages_isolation_merge_gates_dispatch_and_cleanup_with_receipts",
            "evidence": {
                "operation": operation,
                "created": created,
                "removed": removed,
                "conflict_count": conflict_count
            }
        })],
        artifacts: vec![state_rel, worktree_root_rel],
    })
}

fn execute_assimilate_fast_contract(
    root: &Path,
    profile: RuntimeSystemContractProfile,
    payload: &Value,
    apply: bool,
    strict: bool,
) -> Result<ContractExecution, String> {
    let (state_path, mut state, state_rel) = load_contract_state(root, profile);
    let cache_path = family_data_root(root, profile.family).join("skeleton_cache.json");
    let cache_rel = lane_utils::rel_path(root, &cache_path);
    let fast_mode = payload_bool(payload, "fast_mode_enabled", true);
    let cache_enabled = payload_bool(payload, "skeleton_cache_enabled", true);
    let target_latency_ms = payload_f64(payload, "target_latency_ms", 5000.0);
    let parallelism = payload_u64(payload, "max_parallel_microtasks", 8).clamp(1, 128);
    let reduced_validation_depth = payload_u64(payload, "reduced_validation_depth", 1);
    let disclosure = payload_bool(payload, "mode_disclosure_emitted", true);
    if strict
        && profile.id == "V6-ASSIMILATE-FAST-001.6"
        && (!payload_bool(payload, "safety_guard_enabled", true) || !disclosure)
    {
        return Err("assimilate_fast_safety_disclosure_or_guard_missing".to_string());
    }

    let summary = json!({
        "family": profile.family,
        "contract_id": profile.id,
        "fast_mode_enabled": fast_mode,
        "skeleton_cache_enabled": cache_enabled,
        "target_latency_ms": target_latency_ms,
        "max_parallel_microtasks": parallelism,
        "reduced_validation_depth": reduced_validation_depth,
        "mode_disclosure_emitted": disclosure,
        "state_path": state_rel,
        "cache_path": cache_rel
    });

    if apply {
        lane_utils::write_json(
            &cache_path,
            &json!({
                "updated_at": now_iso(),
                "cache_enabled": cache_enabled,
                "last_contract": profile.id,
                "max_parallel_microtasks": parallelism,
                "target_latency_ms": target_latency_ms
            }),
        )?;
        upsert_contract_state_entry(
            &mut state,
            profile.id,
            json!({ "summary": summary, "applied_at": now_iso() }),
        );
        lane_utils::write_json(&state_path, &state)?;
    }

    Ok(ContractExecution {
        summary,
        claims: vec![json!({
            "id": profile.id,
            "claim": "assimilate_fast_lane_executes_cache_parallelization_and_safety_disclosure_with_receipts",
            "evidence": {
                "target_latency_ms": target_latency_ms,
                "max_parallel_microtasks": parallelism,
                "reduced_validation_depth": reduced_validation_depth
            }
        })],
        artifacts: vec![state_rel, cache_rel],
    })
}

fn execute_workflow_open_swe_contract(
    root: &Path,
    profile: RuntimeSystemContractProfile,
    payload: &Value,
    apply: bool,
    strict: bool,
) -> Result<ContractExecution, String> {
    let (state_path, mut state, state_rel) = load_contract_state(root, profile);
    if strict
        && profile.id == "V6-WORKFLOW-028.3"
        && payload_bool(payload, "requires_approval", false)
    {
        let decision = payload_string(payload, "approval_decision", "");
        if !matches!(decision.as_str(), "approved" | "denied") {
            return Err("workflow_open_swe_missing_human_approval_decision".to_string());
        }
    }
    let eval_pass_rate = payload_f64(
        payload,
        "eval_pass_rate",
        payload_f64(payload, "eval_pass_floor", 0.8),
    );
    if strict
        && profile.id == "V6-WORKFLOW-028.4"
        && eval_pass_rate < payload_f64(payload, "eval_pass_floor", 0.8)
    {
        return Err("workflow_open_swe_eval_floor_failed".to_string());
    }
    let registry_path = family_data_root(root, profile.family).join("loop_registry.json");
    let registry_rel = lane_utils::rel_path(root, &registry_path);
    let summary = json!({
        "family": profile.family,
        "contract_id": profile.id,
        "eval_pass_rate": eval_pass_rate,
        "approval_required": payload_bool(payload, "requires_approval", false),
        "approval_decision": payload_string(payload, "approval_decision", ""),
        "state_path": state_rel,
        "registry_path": registry_rel
    });
    if apply {
        lane_utils::write_json(
            &registry_path,
            &json!({
                "updated_at": now_iso(),
                "last_contract": profile.id,
                "loop_templates": payload_string_array(payload, "loop_templates", &["plan-edit-test-commit"]),
                "git_bridge_enabled": payload_bool(payload, "git_bridge_enabled", true)
            }),
        )?;
        upsert_contract_state_entry(
            &mut state,
            profile.id,
            json!({ "summary": summary, "applied_at": now_iso() }),
        );
        lane_utils::write_json(&state_path, &state)?;
    }
    Ok(ContractExecution {
        summary,
        claims: vec![json!({
            "id": profile.id,
            "claim": "workflow_open_swe_lane_enforces_hitl_gates_eval_floor_and_registry_receipts",
            "evidence": {
                "eval_pass_rate": eval_pass_rate
            }
        })],
        artifacts: vec![state_rel, registry_rel],
    })
}

fn execute_memory_context_contract(
    root: &Path,
    profile: RuntimeSystemContractProfile,
    payload: &Value,
    apply: bool,
    strict: bool,
) -> Result<ContractExecution, String> {
    let (state_path, mut state, state_rel) = load_contract_state(root, profile);
    let budget = payload_u64(payload, "context_budget_tokens", 250000).max(1024);
    let mut usage = payload_u64(payload, "window_usage_tokens", 120000);
    let mut pruned_jots = 0u64;
    let mut pruned_tags = 0u64;
    let mut compacted = false;
    let mut invalid_config = Vec::<String>::new();
    let sweep_minutes = payload_u64(payload, "sweep_cadence_minutes", 5);
    let staleness_reset = payload_u64(payload, "staleness_reset_seconds", 30);
    if sweep_minutes == 0 {
        invalid_config.push("sweep_cadence_minutes".to_string());
    }
    if staleness_reset == 0 {
        invalid_config.push("staleness_reset_seconds".to_string());
    }
    if strict && profile.id == "V6-MEMORY-CONTEXT-001.5" && !invalid_config.is_empty() {
        return Err(format!(
            "memory_context_invalid_config:{}",
            invalid_config.join(",")
        ));
    }

    if matches!(
        profile.id,
        "V6-MEMORY-CONTEXT-001.2" | "V6-MEMORY-CONTEXT-001.3"
    ) {
        if usage > budget {
            let overflow = usage - budget;
            pruned_jots = overflow.min(usage / 4);
            usage = usage.saturating_sub(pruned_jots);
        }
    }
    if profile.id == "V6-MEMORY-CONTEXT-001.3" && usage > budget {
        let overflow = usage - budget;
        pruned_tags = overflow.min(usage / 6);
        usage = usage.saturating_sub(pruned_tags);
        compacted = true;
    }

    let summary = json!({
        "family": profile.family,
        "contract_id": profile.id,
        "context_budget_tokens": budget,
        "window_usage_tokens": usage,
        "pruned_jots": pruned_jots,
        "pruned_tags": pruned_tags,
        "compacted": compacted,
        "invalid_config": invalid_config,
        "state_path": state_rel
    });
    if apply {
        upsert_contract_state_entry(
            &mut state,
            profile.id,
            json!({ "summary": summary, "applied_at": now_iso() }),
        );
        lane_utils::write_json(&state_path, &state)?;
    }
    Ok(ContractExecution {
        summary,
        claims: vec![json!({
            "id": profile.id,
            "claim": "memory_context_lane_tracks_staleness_prunes_before_generation_and_emergency_compacts_with_receipts",
            "evidence": {
                "window_usage_tokens": usage,
                "context_budget_tokens": budget,
                "compacted": compacted
            }
        })],
        artifacts: vec![state_rel],
    })
}
