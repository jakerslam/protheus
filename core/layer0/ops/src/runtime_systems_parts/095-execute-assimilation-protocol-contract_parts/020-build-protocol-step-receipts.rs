
fn build_protocol_step_receipts(
    system_id: &str,
    plan_steps: &[Value],
    strict: bool,
    apply: bool,
    previous_hash: &str,
) -> (Vec<Value>, String) {
    let mut rows = Vec::<Value>::new();
    let mut prior = previous_hash.to_string();
    for (idx, step) in plan_steps.iter().enumerate() {
        let step_name = step
            .get("step_name")
            .and_then(Value::as_str)
            .unwrap_or("unknown_step");
        let status = step
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("planned");
        let mut row = json!({
            "type": "assimilation_protocol_step_receipt",
            "protocol_version": ASSIMILATION_PROTOCOL_VERSION,
            "system_id": system_id,
            "step_number": idx + 1,
            "step_name": step_name,
            "status": status,
            "strict": strict,
            "apply": apply,
            "ts": now_iso(),
            "previous_hash": prior,
            "step_context": step
        });
        let step_hash = receipt_hash(&row);
        row["step_hash"] = Value::String(step_hash.clone());
        prior = step_hash;
        rows.push(row);
    }
    (rows, prior)
}

fn validate_protocol_step_receipt_chain(
    system_id: &str,
    rows: &[Value],
    previous_hash: &str,
) -> (bool, Vec<String>) {
    let mut errors = Vec::<String>::new();
    let mut prior = previous_hash.to_string();
    for (idx, row) in rows.iter().enumerate() {
        if row.get("protocol_version").and_then(Value::as_str)
            != Some(ASSIMILATION_PROTOCOL_VERSION)
        {
            errors.push(format!("step_{}_protocol_version_mismatch", idx + 1));
        }
        if row.get("system_id").and_then(Value::as_str) != Some(system_id) {
            errors.push(format!("step_{}_system_id_mismatch", idx + 1));
        }
        if row.get("step_number").and_then(Value::as_u64) != Some((idx + 1) as u64) {
            errors.push(format!("step_{}_number_mismatch", idx + 1));
        }
        if row.get("previous_hash").and_then(Value::as_str) != Some(prior.as_str()) {
            errors.push(format!("step_{}_previous_hash_mismatch", idx + 1));
        }
        let recorded_hash = row.get("step_hash").and_then(Value::as_str).unwrap_or("");
        let mut recomputable = row.clone();
        if let Some(obj) = recomputable.as_object_mut() {
            obj.remove("step_hash");
        }
        let expected_hash = receipt_hash(&recomputable);
        if recorded_hash != expected_hash {
            errors.push(format!("step_{}_hash_mismatch", idx + 1));
        }
        prior = recorded_hash.to_string();
    }
    (errors.is_empty(), errors)
}

fn execute_assimilation_protocol_for_system(
    root: &Path,
    system_id: &str,
    command: &str,
    payload: &Value,
    args: &[String],
    apply: bool,
    strict: bool,
) -> Result<Option<ContractExecution>, String> {
    if !is_assimilation_system_id(system_id) {
        return Ok(None);
    }

    let component = assimilation_component(system_id);
    let operation = normalize_assimilation_operation(command, args);
    let phase = resolve_assimilation_phase(component, payload, args);
    let default_phase = assimilation_default_phase(component);
    let allowed_ops = assimilation_allowed_ops(component);
    let allowed_phases = assimilation_allowed_phases(component);
    let hard_selector = resolve_hard_selector(payload, args);
    let hard_selector_present = !hard_selector.is_empty();
    let bypass_requested = selector_bypass_requested(payload, args);
    let payload_sha = payload_sha(payload);
    let trace_id = format!(
        "trace_{}",
        &receipt_hash(&json!({
            "system_id": system_id,
            "component": component,
            "operation": operation,
            "phase": phase,
            "payload_sha256": payload_sha,
            "ts": now_iso()
        }))[..16]
    );

    let (state_path, history_path) = assimilation_protocol_paths(root, system_id);
    let step_receipts_path = assimilation_protocol_step_receipts_path(root, system_id);
    let state_rel = lane_utils::rel_path(root, &state_path);
    let history_rel = lane_utils::rel_path(root, &history_path);
    let step_receipts_rel = lane_utils::rel_path(root, &step_receipts_path);
    let previous = lane_utils::read_json(&state_path).unwrap_or_else(|| json!({}));
    let prior_count = previous
        .get("run_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let previous_step_hash = previous
        .get("last_step_hash")
        .and_then(Value::as_str)
        .unwrap_or("GENESIS");
    let run_count = if apply {
        prior_count.saturating_add(1)
    } else {
        prior_count
    };
    let ts = now_iso();
    let recon_surfaces = assimilation_recon_surfaces(component, system_id);
    let intent_spec = json!({
        "trace_id": trace_id,
        "system_id": system_id,
        "component": component,
        "operation": operation,
        "phase": phase,
        "hard_selector": hard_selector,
        "hard_selector_present": hard_selector_present,
        "strict": strict,
        "apply": apply,
        "payload_sha256": payload_sha
    });
    let recon_index = json!({
        "trace_id": trace_id,
        "provider_plane": "substrate_runtime_systems",
        "surface_count": recon_surfaces.len(),
        "surfaces": recon_surfaces
    });

    let op_allowed = allowed_ops.iter().any(|candidate| *candidate == operation);
    let phase_allowed = allowed_phases.iter().any(|candidate| *candidate == phase);

    let candidate_set_rows = recon_index
        .get("surfaces")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|surface| {
            let supported_ops = surface
                .get("supported_operations")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .filter_map(Value::as_str)
                .map(|row| row.trim().to_ascii_lowercase())
                .collect::<Vec<_>>();
            let supported_phases = surface
                .get("supported_phases")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .filter_map(Value::as_str)
                .map(|row| row.trim().to_ascii_lowercase())
                .collect::<Vec<_>>();
            let selector_match = surface_matches_hard_selector(&surface, &hard_selector);
            let operation_match = contains_string(&supported_ops, operation.as_str());
            let phase_match = contains_string(&supported_phases, phase.as_str());
            let admissible = selector_match && operation_match && phase_match;
            let mut denial_reasons = Vec::<String>::new();
            if !selector_match {
                denial_reasons.push("selector_mismatch".to_string());
            }
            if !operation_match {
                denial_reasons.push("operation_not_supported".to_string());
            }
            if !phase_match {
                denial_reasons.push("phase_not_supported".to_string());
            }
            json!({
                "surface_id": surface.get("surface_id").cloned().unwrap_or(Value::Null),
                "provider": surface.get("provider").cloned().unwrap_or(Value::Null),
                "domain": surface.get("domain").cloned().unwrap_or(Value::Null),
                "selector_match": selector_match,
                "operation_match": operation_match,
                "phase_match": phase_match,
                "admissible": admissible,
                "denial_reasons": denial_reasons
            })
        })
        .collect::<Vec<_>>();
    let admissible_rows = candidate_set_rows
        .iter()
        .filter(|row| {
            row.get("admissible")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();
    let candidate_set = json!({
        "trace_id": trace_id,
        "hard_selector_present": hard_selector_present,
        "hard_selector": hard_selector.clone(),
        "candidates": candidate_set_rows,
        "candidate_count": candidate_set_rows.len(),
        "admissible_count": admissible_rows.len()
    });
    let candidate_closure = json!({
        "trace_id": trace_id,
        "hard_selector_present": hard_selector_present,
        "hard_selector": hard_selector,
        "closure_complete": !admissible_rows.is_empty(),
        "selected_candidate": admissible_rows.first().cloned().unwrap_or(Value::Null),
        "admissible_candidates": admissible_rows
    });

    let mut gaps = Vec::<Value>::new();
    if !op_allowed {
        gaps.push(json!({
            "gap_id": "assimilation_protocol_op_not_allowed",
            "severity": "blocker",
            "detail": format!("operation `{}` is not allowed for component `{}`", operation, component)
        }));
    }
    if !phase_allowed {
        gaps.push(json!({
            "gap_id": "assimilation_protocol_phase_mismatch",
            "severity": "blocker",
            "detail": format!("phase `{}` is not allowed for component `{}` (default `{}`)", phase, component, default_phase)
        }));
    }
    if candidate_closure
        .get("closure_complete")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        == false
    {
        gaps.push(json!({
            "gap_id": "assimilation_candidate_closure_incomplete",
            "severity": "blocker",
            "detail": "candidate closure has no admissible substrate surface"
        }));
    }
    if hard_selector_present
        && candidate_set
            .get("admissible_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            == 0
    {
        gaps.push(json!({
            "gap_id": "assimilation_hard_selector_closure_reject",
            "severity": "blocker",
            "detail": format!("hard selector `{}` did not resolve to an admissible closure candidate", hard_selector)
        }));
    }
    if bypass_requested {
        gaps.push(json!({
            "gap_id": "assimilation_selector_bypass_rejected",
            "severity": "blocker",
            "detail": "selector bypass is not permitted in canonical assimilation protocol chain"
        }));
    }
    if payload
        .as_object()
        .map(|row| row.is_empty())
        .unwrap_or(true)
    {
        gaps.push(json!({
            "gap_id": "assimilation_payload_sparse",
            "severity": "warning",
            "detail": "payload is empty or sparse; execution remains deterministic but may be low-context"
        }));
    }
    let blocker_count = gaps
        .iter()
        .filter(|row| row.get("severity").and_then(Value::as_str) == Some("blocker"))
        .count();
    let warning_count = gaps
        .iter()
        .filter(|row| row.get("severity").and_then(Value::as_str) == Some("warning"))
        .count();
    let mut denial_codes = gaps
        .iter()
        .filter(|row| row.get("severity").and_then(Value::as_str) == Some("blocker"))
        .filter_map(|row| row.get("gap_id").and_then(Value::as_str))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let mut seen_codes = BTreeSet::<String>::new();
    denial_codes.retain(|code| seen_codes.insert(code.clone()));
    denial_codes.sort_by(|a, b| {
        assimilation_denial_priority(a)
            .cmp(&assimilation_denial_priority(b))
            .then_with(|| a.cmp(b))
    });
    let provisional_gap_report = json!({
        "trace_id": trace_id,
        "blocker_count": blocker_count,
        "warning_count": warning_count,
        "gaps": gaps,
        "denial_codes": denial_codes.clone()
    });
    let admitted = blocker_count == 0
        && candidate_closure
            .get("closure_complete")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let admission_verdict = json!({
        "trace_id": trace_id,
        "admitted": admitted,
        "decision": if admitted { "allow" } else { "deny" },
        "strict": strict,
        "required_controls": [
            "IntentSpec",
            "ReconIndex",
            "CandidateClosure",
            "ProvisionalGapReport",
            "AdmissionVerdict",
            "ProtocolStepReceipt"
        ],
        "denial_codes": denial_codes.clone()
    });

    let mut plan_steps = vec![
        json!({"step_name": "IntentSpec", "status": "complete", "control_class": "governance"}),
        json!({"step_name": "ReconIndex", "status": "complete", "control_class": "governance"}),
        json!({"step_name": "CandidateClosure", "status": "complete", "control_class": "safety"}),
        json!({"step_name": "ProvisionalGapReport", "status": "complete", "control_class": "safety"}),
        json!({"step_name": "AdmissionVerdict", "status": if admitted { "allow" } else { "deny" }, "control_class": "policy"}),
    ];
    if admitted {
        plan_steps.push(
            json!({"step_name": "SubstrateExecution", "status": if apply { "executed" } else { "dry_run" }, "control_class": "substrate"}),
        );
        plan_steps.push(
            json!({"step_name": "ProtocolStepReceiptPersist", "status": if apply { "persisted" } else { "dry_run" }, "control_class": "receipt"}),
        );
    }
    let admitted_assimilation_plan = json!({
        "trace_id": trace_id,
        "plan_id": format!("plan_{}", &receipt_hash(&json!({"trace_id": trace_id, "system_id": system_id}))[..16]),
        "admitted": admitted,
        "steps": plan_steps
    });

    let plan_steps_rows = admitted_assimilation_plan
        .get("steps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let (step_receipts, last_step_hash) = build_protocol_step_receipts(
        system_id,
        &plan_steps_rows,
        strict,
        apply,
        previous_step_hash,
    );
    let (step_hash_chain_valid, step_hash_chain_errors) =
        validate_protocol_step_receipt_chain(system_id, &step_receipts, previous_step_hash);
    if strict && !step_hash_chain_valid {
        return Err(format!(
            "assimilation_protocol_step_hash_chain_invalid:{}:{}",
            component,
            step_hash_chain_errors.join(",")
        ));
    }
    let protocol_step_receipt = json!({
        "trace_id": trace_id,
        "count": step_receipts.len(),
        "last_step_hash": last_step_hash,
        "last_step": step_receipts.last().cloned().unwrap_or(Value::Null),
        "step_receipts_path": step_receipts_rel,
        "chain": {
            "previous_hash": previous_step_hash,
            "valid": step_hash_chain_valid,
            "error_count": step_hash_chain_errors.len(),
            "errors": step_hash_chain_errors
        }
    });

    let history_row = json!({
        "type": "assimilation_protocol_event",
        "protocol_version": ASSIMILATION_PROTOCOL_VERSION,
        "system_id": system_id,
        "trace_id": trace_id,
        "component": component,
        "IntentSpec": intent_spec,
        "ReconIndex": recon_index,
        "CandidateSet": candidate_set,
        "CandidateClosure": candidate_closure,
        "ProvisionalGapReport": provisional_gap_report,
        "AdmissionVerdict": admission_verdict,
        "AdmittedAssimilationPlan": admitted_assimilation_plan,
        "ProtocolStepReceipt": protocol_step_receipt,
        "strict": strict,
        "apply": apply,
        "payload_sha256": payload_sha,
        "ts": ts
    });

    if apply {
        lane_utils::write_json(
            &state_path,
            &json!({
                "type": "assimilation_protocol_state",
                "protocol_version": ASSIMILATION_PROTOCOL_VERSION,
                "system_id": system_id,
                "component": component,
                "trace_id": trace_id,
                "IntentSpec": intent_spec,
                "ReconIndex": recon_index,
                "CandidateSet": candidate_set,
                "CandidateClosure": candidate_closure,
                "ProvisionalGapReport": provisional_gap_report,
                "AdmissionVerdict": admission_verdict,
                "AdmittedAssimilationPlan": admitted_assimilation_plan,
                "ProtocolStepReceipt": protocol_step_receipt,
                "run_count": run_count,
                "last_payload_sha256": payload_sha,
                "last_step_hash": last_step_hash,
                "updated_at": ts
            }),
        )?;
        lane_utils::append_jsonl(&history_path, &history_row)?;
        for row in &step_receipts {
            lane_utils::append_jsonl(&step_receipts_path, row)?;
        }
    }

    let summary = json!({
        "protocol_version": ASSIMILATION_PROTOCOL_VERSION,
        "system_id": system_id,
        "trace_id": trace_id,
        "component": component,
        "IntentSpec": intent_spec,
        "ReconIndex": recon_index,
        "CandidateSet": candidate_set,
        "CandidateClosure": candidate_closure,
        "ProvisionalGapReport": provisional_gap_report,
        "AdmissionVerdict": admission_verdict,
        "AdmittedAssimilationPlan": admitted_assimilation_plan,
        "ProtocolStepReceipt": protocol_step_receipt,
        "default_phase": default_phase,
        "allowed_operations": allowed_ops,
        "allowed_phases": allowed_phases,
        "strict": strict,
        "apply": apply,
        "run_count": run_count,
        "state_path": state_rel,
        "history_path": history_rel,
        "step_receipts_path": step_receipts_rel,
        "governance": {
            "conduit_only_enforced": true,
            "receipt_first": true,
            "append_only_history": true,
            "hard_selector_gate": true,
            "closure_required": true,
            "gap_analysis_required": true,
            "admission_required": true,
            "rust_authority_plane": "core/layer0/ops::runtime_systems"
        }
    });

    let artifacts = if apply {
        vec![
            state_rel.clone(),
            history_rel.clone(),
            step_receipts_rel.clone(),
        ]
    } else {
        Vec::new()
    };
    let claims = vec![json!({
        "id": "assimilation_protocol_receipted_lane",
        "claim": "assimilation_runtime_operations_route_through_canonical_protocol_chain_with_mandatory_closure_gap_admission_and_step_receipts",
        "evidence": {
            "protocol_version": ASSIMILATION_PROTOCOL_VERSION,
            "system_id": system_id,
            "component": component,
            "trace_id": trace_id,
            "intent_spec_present": summary.get("IntentSpec").is_some(),
            "candidate_closure_present": summary.get("CandidateClosure").is_some(),
            "gap_report_present": summary.get("ProvisionalGapReport").is_some(),
            "admission_verdict_present": summary.get("AdmissionVerdict").is_some(),
            "step_receipt_count": step_receipts.len(),
            "state_path": state_rel,
            "history_path": history_rel,
            "step_receipts_path": step_receipts_rel
        }
    })];

    if strict && !admitted {
        let denial_code = denial_codes
            .first()
            .cloned()
            .unwrap_or_else(|| "assimilation_admission_denied".to_string());
        return Err(format!("{denial_code}:{component}:{operation}"));
    }

    Ok(Some(ContractExecution {
        summary,
        claims,
        artifacts,
    }))
}
