const ASSIMILATION_PROTOCOL_VERSION: &str = "infring_assimilation_protocol_v1";

const ASSIMILATION_ATTESTATION_OPS: &[&str] = &["run", "attest", "verify"];
const ASSIMILATION_DISTILLER_OPS: &[&str] = &["run", "distill", "verify"];
const ASSIMILATION_FRESHNESS_OPS: &[&str] = &["run", "freshness", "verify"];
const ASSIMILATION_GENERIC_OPS: &[&str] = &["run", "verify"];
const ASSIMILATION_ATTESTATION_PHASES: &[&str] = &["attestation", "generic"];
const ASSIMILATION_DISTILLER_PHASES: &[&str] = &["distillation", "generic"];
const ASSIMILATION_FRESHNESS_PHASES: &[&str] = &["freshness", "generic"];
const ASSIMILATION_GENERIC_PHASES: &[&str] = &["generic"];

#[derive(Clone, Copy)]
struct AssimilationComponentProfile {
    default_phase: &'static str,
    allowed_ops: &'static [&'static str],
    allowed_phases: &'static [&'static str],
    surface_id: &'static str,
    substrate_surface: &'static str,
}

fn is_assimilation_system_id(system_id: &str) -> bool {
    system_id
        .trim()
        .to_ascii_uppercase()
        .starts_with("SYSTEMS-ASSIMILATION-")
}

fn assimilation_component(system_id: &str) -> &'static str {
    let id = system_id.trim().to_ascii_uppercase();
    if id.contains("SOURCE_ATTESTATION_EXTENSION") {
        "source_attestation_extension"
    } else if id.contains("TRAJECTORY_SKILL_DISTILLER") {
        "trajectory_skill_distiller"
    } else if id.contains("WORLD_MODEL_FRESHNESS") {
        "world_model_freshness"
    } else {
        "assimilation_generic"
    }
}

fn assimilation_component_profile(component: &str) -> AssimilationComponentProfile {
    match component {
        "source_attestation_extension" => AssimilationComponentProfile {
            default_phase: "attestation",
            allowed_ops: ASSIMILATION_ATTESTATION_OPS,
            allowed_phases: ASSIMILATION_ATTESTATION_PHASES,
            surface_id: "substrate://runtime-systems/source_attestation_extension",
            substrate_surface: "source_attestation_surface",
        },
        "trajectory_skill_distiller" => AssimilationComponentProfile {
            default_phase: "distillation",
            allowed_ops: ASSIMILATION_DISTILLER_OPS,
            allowed_phases: ASSIMILATION_DISTILLER_PHASES,
            surface_id: "substrate://runtime-systems/trajectory_skill_distiller",
            substrate_surface: "trajectory_distillation_surface",
        },
        "world_model_freshness" => AssimilationComponentProfile {
            default_phase: "freshness",
            allowed_ops: ASSIMILATION_FRESHNESS_OPS,
            allowed_phases: ASSIMILATION_FRESHNESS_PHASES,
            surface_id: "substrate://runtime-systems/world_model_freshness",
            substrate_surface: "world_model_freshness_surface",
        },
        _ => AssimilationComponentProfile {
            default_phase: "generic",
            allowed_ops: ASSIMILATION_GENERIC_OPS,
            allowed_phases: ASSIMILATION_GENERIC_PHASES,
            surface_id: "substrate://runtime-systems/assimilation_generic",
            substrate_surface: "assimilation_generic_surface",
        },
    }
}

fn assimilation_default_phase(component: &str) -> &'static str {
    assimilation_component_profile(component).default_phase
}

fn assimilation_allowed_ops(component: &str) -> &'static [&'static str] {
    assimilation_component_profile(component).allowed_ops
}

fn assimilation_allowed_phases(component: &str) -> &'static [&'static str] {
    assimilation_component_profile(component).allowed_phases
}

fn normalize_assimilation_operation(command: &str, args: &[String]) -> String {
    let op = lane_utils::parse_flag(args, "op", true).unwrap_or_else(|| command.to_string());
    lane_utils::clean_text(Some(op.as_str()), 64)
        .to_ascii_lowercase()
        .replace('_', "-")
}

fn resolve_assimilation_phase(component: &str, payload: &Value, args: &[String]) -> String {
    let phase_flag = lane_utils::parse_flag(args, "phase", true);
    let phase_payload = payload.get("phase").and_then(Value::as_str);
    let cleaned = lane_utils::clean_text(phase_flag.as_deref().or(phase_payload), 64)
        .to_ascii_lowercase()
        .replace('_', "-");
    if cleaned.is_empty() {
        assimilation_default_phase(component).to_string()
    } else {
        cleaned
    }
}

fn assimilation_protocol_paths(root: &Path, system_id: &str) -> (PathBuf, PathBuf) {
    let canonical_id = lane_utils::clean_token(Some(system_id), "runtime-assimilation");
    let dir = systems_dir(root).join("_assimilation").join(canonical_id);
    (
        dir.join("protocol_state.json"),
        dir.join("protocol_history.jsonl"),
    )
}

fn assimilation_protocol_step_receipts_path(root: &Path, system_id: &str) -> PathBuf {
    let canonical_id = lane_utils::clean_token(Some(system_id), "runtime-assimilation");
    systems_dir(root)
        .join("_assimilation")
        .join(canonical_id)
        .join("protocol_step_receipts.jsonl")
}

fn string_from_payload(payload: &Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(|raw| lane_utils::clean_text(Some(raw), 96))
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
}

fn resolve_hard_selector(payload: &Value, args: &[String]) -> String {
    let selector = lane_utils::parse_flag(args, "hard-selector", true)
        .or_else(|| lane_utils::parse_flag(args, "selector", true))
        .or_else(|| lane_utils::parse_flag(args, "surface-id", true))
        .or_else(|| lane_utils::parse_flag(args, "core-domain", true))
        .or_else(|| string_from_payload(payload, "hard_selector"))
        .or_else(|| string_from_payload(payload, "selector"))
        .or_else(|| string_from_payload(payload, "surface_id"))
        .unwrap_or_default();
    lane_utils::clean_text(Some(selector.as_str()), 96)
        .trim()
        .to_ascii_lowercase()
}

fn selector_bypass_requested(payload: &Value, args: &[String]) -> bool {
    let from_flags = lane_utils::parse_bool(
        lane_utils::parse_flag(args, "selector-bypass", true).as_deref(),
        false,
    ) || lane_utils::parse_bool(
        lane_utils::parse_flag(args, "bypass-selector", true).as_deref(),
        false,
    );
    let from_payload = payload
        .get("selector_bypass")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || payload
            .get("bypass_selector")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    from_flags || from_payload
}

fn assimilation_denial_priority(code: &str) -> usize {
    match code {
        "assimilation_selector_bypass_rejected" => 0,
        "assimilation_hard_selector_closure_reject" => 1,
        "assimilation_protocol_op_not_allowed" => 2,
        "assimilation_protocol_phase_mismatch" => 3,
        "assimilation_candidate_closure_incomplete" => 4,
        _ => 100,
    }
}

fn surface_matches_hard_selector(surface: &Value, hard_selector: &str) -> bool {
    if hard_selector.is_empty() {
        return true;
    }
    let selector = hard_selector.trim().to_ascii_lowercase();
    if selector.is_empty() {
        return true;
    }
    let candidates = [
        surface
            .get("surface_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_ascii_lowercase(),
        surface
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_ascii_lowercase(),
        surface
            .get("domain")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_ascii_lowercase(),
        surface
            .get("component")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_ascii_lowercase(),
        surface
            .get("binding_system_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_ascii_lowercase(),
    ];
    candidates
        .iter()
        .any(|candidate| !candidate.is_empty() && candidate.contains(&selector))
}

fn contains_string(values: &[String], candidate: &str) -> bool {
    let normalized = candidate.trim().to_ascii_lowercase();
    values.iter().any(|row| row == &normalized)
}

fn assimilation_recon_surfaces(component: &str, system_id: &str) -> Vec<Value> {
    let profile = assimilation_component_profile(component);
    vec![json!({
        "surface_id": profile.surface_id,
        "provider": "substrate_runtime_systems",
        "domain": "runtime-systems",
        "component": component,
        "binding_system_id": system_id,
        "substrate_surface": profile.substrate_surface,
        "supported_operations": profile.allowed_ops,
        "supported_phases": profile.allowed_phases
    })]
}

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
            json!({
                "surface_id": surface.get("surface_id").cloned().unwrap_or(Value::Null),
                "provider": surface.get("provider").cloned().unwrap_or(Value::Null),
                "domain": surface.get("domain").cloned().unwrap_or(Value::Null),
                "selector_match": selector_match,
                "operation_match": operation_match,
                "phase_match": phase_match,
                "admissible": admissible
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
    let provisional_gap_report = json!({
        "trace_id": trace_id,
        "blocker_count": blocker_count,
        "warning_count": warning_count,
        "gaps": gaps
    });
    let admitted = blocker_count == 0
        && candidate_closure
            .get("closure_complete")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let mut denial_codes = provisional_gap_report
        .get("gaps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
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
        "denial_codes": denial_codes
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
    let protocol_step_receipt = json!({
        "trace_id": trace_id,
        "count": step_receipts.len(),
        "last_step_hash": last_step_hash,
        "last_step": step_receipts.last().cloned().unwrap_or(Value::Null),
        "step_receipts_path": step_receipts_rel
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
