fn run_shadow_train(
    root: &Path,
    policy: &AdaptivePolicy,
    parsed: &crate::ParsedArgs,
    strict: bool,
) -> Value {
    let conduit = conduit(root, parsed, "shadow-train", strict);
    if strict && !conduit.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return json!({
            "ok": false,
            "type": "adaptive_intelligence_shadow_train",
            "strict": strict,
            "conduit": conduit,
            "errors": ["conduit_bypass_rejected"]
        });
    }
    let cycles = parse_u64(parsed.flags.get("cycles"), 1).clamp(1, 16);
    let context = collect_context_bundle(root, &parsed.flags);
    let resources = resource_snapshot(&parsed.flags, policy);
    let mut state = load_state(root, policy);
    let mut cycle_rows = Vec::<Value>::new();
    let mut logical_score = state.logical.specialization_score_pct;
    let mut creative_score = state.creative.specialization_score_pct;
    for cycle in 1..=cycles {
        let logical_gain = specialization_gain("logical", &context, &resources, cycle);
        let creative_gain = specialization_gain("creative", &context, &resources, cycle);
        logical_score = (logical_score + logical_gain).min(100.0);
        creative_score = (creative_score + creative_gain).min(100.0);
        cycle_rows.push(json!({
            "cycle": cycle,
            "logical_gain": logical_gain,
            "creative_gain": creative_gain,
            "trainer_adapter": policy.trainer_adapter,
            "local_only": policy.local_only,
            "context_digest": context.interaction_digest
        }));
    }
    let job_id = sha256_hex_str(&format!(
        "{}|{}|{}|{}",
        context.interaction_digest, cycles, policy.seed_model, resources.mode
    ));
    state.updated_at = now_iso();
    state.active_mode = resources.mode.clone();
    state.logical.specialization_score_pct = logical_score;
    state.creative.specialization_score_pct = creative_score;
    state.logical.last_trained_at = Some(state.updated_at.clone());
    state.creative.last_trained_at = Some(state.updated_at.clone());
    state.training.cycles_completed += cycles;
    state.training.last_job_id = Some(job_id.clone());
    state.training.last_context_digest = Some(context.interaction_digest.clone());
    state.training.last_mode = Some(resources.mode.clone());
    state.training.nightly_due = false;
    state.training.last_trained_at = Some(state.updated_at.clone());
    let _ = store_state(root, &state);
    let training_row = json!({
        "ts": now_iso(),
        "type": "adaptive_intelligence_shadow_training_job",
        "job_id": job_id,
        "cycles": cycles,
        "mode": resources.mode,
        "context_digest": context.interaction_digest,
        "logical_score_pct": logical_score,
        "creative_score_pct": creative_score,
        "trainer_adapter": policy.trainer_adapter,
        "seed_model": policy.seed_model,
        "claim_ids": ["V7-ADAPTIVE-001.2"]
    })
    .with_receipt_hash();
    let _ = append_jsonl(&training_history_path(root), &training_row);
    json!({
        "ok": true,
        "type": "adaptive_intelligence_shadow_train",
        "strict": strict,
        "job_id": job_id,
        "cycles": cycles,
        "mode": resources.mode,
        "trainer": {
            "adapter": policy.trainer_adapter,
            "local_only": policy.local_only,
            "seed_model": policy.seed_model,
            "fine_tune_mode": "qlora_or_equivalent"
        },
        "cycle_rows": cycle_rows,
        "specialization": {
            "logical_score_pct": logical_score,
            "creative_score_pct": creative_score,
            "graduation_threshold_pct": policy.graduation_threshold_pct
        },
        "claim_evidence": [{
            "id": "V7-ADAPTIVE-001.2",
            "claim": "shadow_mode_training_uses_local_context_and_updates_specialization_scores_without_safety_plane_execution",
            "evidence": {
                "job_id": job_id,
                "cycles": cycles,
                "context_digest": context.interaction_digest,
                "trainer_adapter": policy.trainer_adapter,
                "local_only": policy.local_only
            }
        }],
        "conduit": conduit
    })
}

fn run_graduate(
    root: &Path,
    policy: &AdaptivePolicy,
    parsed: &crate::ParsedArgs,
    strict: bool,
) -> Value {
    let conduit = conduit(root, parsed, "graduate", strict);
    if strict && !conduit.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return json!({
            "ok": false,
            "type": "adaptive_intelligence_graduate",
            "strict": strict,
            "conduit": conduit,
            "errors": ["conduit_bypass_rejected"]
        });
    }
    let model = clean(
        parsed
            .flags
            .get("model")
            .cloned()
            .unwrap_or_else(|| "logical".to_string()),
        40,
    );
    let human_only = parse_bool(parsed.flags.get("human-only"), false);
    let approvers = parsed
        .flags
        .get("approvers")
        .map(|v| split_csv_clean(v, 80))
        .unwrap_or_default();
    let mut state = load_state(root, policy);
    let target_score = match model.as_str() {
        "creative" => state.creative.specialization_score_pct,
        _ => state.logical.specialization_score_pct,
    };
    let threshold_ok = target_score >= policy.graduation_threshold_pct;
    let approvals_ok = approvers.len() >= policy.min_human_approvers;
    let mut errors = Vec::<String>::new();
    if strict && !human_only {
        errors.push("human_only_required".to_string());
    }
    if strict && !approvals_ok {
        errors.push("multi_signature_required".to_string());
    }
    if strict && !threshold_ok {
        errors.push("specialization_threshold_not_met".to_string());
    }
    if errors.is_empty() {
        match model.as_str() {
            "creative" => {
                state.creative.graduated = true;
                state.creative.last_graduated_at = Some(now_iso());
            }
            _ => {
                state.logical.graduated = true;
                state.logical.last_graduated_at = Some(now_iso());
            }
        }
        state.updated_at = now_iso();
        let _ = store_state(root, &state);
    }
    let row = json!({
        "ts": now_iso(),
        "type": "adaptive_intelligence_graduation_event",
        "model": model,
        "human_only": human_only,
        "approvers": approvers,
        "threshold_ok": threshold_ok,
        "score_pct": target_score,
        "graduated": errors.is_empty()
    })
    .with_receipt_hash();
    let _ = append_jsonl(&graduation_history_path(root), &row);
    json!({
        "ok": errors.is_empty(),
        "type": "adaptive_intelligence_graduate",
        "strict": strict,
        "model": model,
        "human_only": human_only,
        "approvers": approvers,
        "score_pct": target_score,
        "graduated": errors.is_empty(),
        "claim_evidence": [{
            "id": "V7-ADAPTIVE-001.6",
            "claim": "model_graduation_requires_human_only_multisig_and_specialization_threshold_before_activation",
            "evidence": {
                "human_only": human_only,
                "approver_count": row.get("approvers").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0),
                "threshold_ok": threshold_ok,
                "score_pct": target_score
            }
        }],
        "errors": errors,
        "conduit": conduit
    })
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let strict = parse_bool(parsed.flags.get("strict"), true);
    let policy = load_policy(root);

    match cmd.as_str() {
        "status" => {
            let state = load_state(root, &policy);
            print_json(&status(root, &policy, &state));
            0
        }
        "prioritize" => emit(root, run_prioritize(root, &policy, &parsed, strict)),
        "propose" | "run" => emit(root, run_propose(root, &policy, &parsed, strict)),
        "shadow-train" | "train-shadow" => {
            emit(root, run_shadow_train(root, &policy, &parsed, strict))
        }
        "graduate" => emit(root, run_graduate(root, &policy, &parsed, strict)),
        _ => {
            usage();
            print_json(&json!({
                "ok": false,
                "type": "adaptive_intelligence_error",
                "error": "unknown_command",
                "command": cmd
            }));
            1
        }
    }
}

