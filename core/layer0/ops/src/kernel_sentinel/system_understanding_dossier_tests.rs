use super::*;

fn sample_dossier() -> SystemUnderstandingDossier {
    SystemUnderstandingDossier {
        dossier_id: "infring-self".to_string(),
        target_mode: SystemUnderstandingDossierTargetMode::InternalRsi,
        target_system: "InfRing".to_string(),
        target_version_or_revision: "main".to_string(),
        dossier_version: SYSTEM_UNDERSTANDING_DOSSIER_SCHEMA_VERSION,
        created_at: "2026-04-29T00:00:00Z".to_string(),
        updated_at: "2026-04-29T00:00:00Z".to_string(),
        owners: vec![
            "kernel-sentinel".to_string(),
            "assimilation".to_string(),
            "rsi".to_string(),
        ],
        status: SystemUnderstandingDossierStatus::Draft,
        confidence_overall: 0.72,
        blocking_unknowns: vec!["needs_more_runtime_traces".to_string()],
        evidence_index: vec!["kernel://report".to_string()],
        soul_confidence: 0.62,
        soul_evidence: vec!["docs://readme".to_string()],
        soul_unknowns: vec![],
        runtime_confidence: 0.74,
        runtime_evidence: vec!["kernel://runtime".to_string()],
        runtime_unknowns: vec![],
        required_next_probes: vec!["probe://gateway_restart".to_string()],
        ecology_confidence: 0.51,
        ecology_evidence: vec!["kernel://deps".to_string()],
        ecology_unknowns: vec![],
        authority_confidence: 0.82,
        authority_evidence: vec!["kernel://authority".to_string()],
        authority_unknowns: vec![],
        authority_risks: vec!["shell_truth_residue".to_string()],
        architecture_confidence: 0.73,
        architecture_evidence: vec!["kernel://architecture".to_string()],
        architecture_unknowns: vec![],
        runtime_architecture_mismatches: vec!["gateway_state_vs_shell_state".to_string()],
        capability_confidence: 0.76,
        capabilities: vec![SystemUnderstandingCapabilityRow {
            id: "kernel_sentinel_runtime_truth".to_string(),
            kind: SystemUnderstandingCapabilityKind::Evidence,
            value: SystemUnderstandingCapabilityValue::Critical,
            evidence: vec!["kernel://sentinel".to_string()],
            runtime_proof: vec!["proof://sentinel".to_string()],
            transfer_target: SystemUnderstandingTransferTarget::Kernel,
            fit_rationale: "Kernel-owned truth synthesis belongs in the kernel.".to_string(),
        }],
        rejected_capabilities: vec![SystemUnderstandingCapabilityRow {
            id: "shell_authority_duplication".to_string(),
            kind: SystemUnderstandingCapabilityKind::Ux,
            value: SystemUnderstandingCapabilityValue::High,
            evidence: vec!["shell://legacy".to_string()],
            runtime_proof: vec![],
            transfer_target: SystemUnderstandingTransferTarget::Reject,
            fit_rationale: "Authority duplication conflicts with Kernel truth ownership."
                .to_string(),
        }],
        capability_unknowns: vec![],
        failure_model_confidence: 0.77,
        known_failure_modes: vec!["source_of_truth_ambiguity".to_string()],
        violated_invariants: vec!["kernel_owns_truth".to_string()],
        stop_patching_triggers: vec!["multi_layer_contradiction".to_string()],
        transfer_confidence: 0.84,
        implementation_items: vec![SystemUnderstandingImplementationItem {
            id: "strengthen-sentinel-dossier".to_string(),
            summary: "Emit dossier before proposing structural changes.".to_string(),
            owner_layer: "kernel".to_string(),
            invariant: "kernel_owns_truth".to_string(),
            proof_requirement: "proof://dossier".to_string(),
            rollback_plan: "revert to advisory-only output".to_string(),
        }],
        proof_requirements: vec!["proof://dossier".to_string()],
        rollback_plan: vec!["rollback://dossier".to_string()],
        implementation_confidence: 0.63,
        files_inspected: vec!["core/layer0/ops/src/kernel_sentinel.rs".to_string()],
        implementation_unknowns: vec![],
        syntax_confidence: 0.4,
        syntax_evidence: vec!["code://kernel_sentinel".to_string()],
        syntax_unknowns: vec![],
    }
}

#[test]
fn dossier_model_exposes_shared_schema_for_sentinel_rsi_and_assimilation() {
    let model = kernel_system_understanding_dossier_model();
    assert_eq!(model["type"], "kernel_system_understanding_dossier_model");
    assert_eq!(model["schema_version"], 1);
    assert_eq!(
        model["shared_consumers"],
        json!(["kernel_sentinel", "internal_rsi_planning", "external_assimilation"])
    );
    assert_eq!(
        model["sections"]["authority"]["minimum_confidence"],
        json!(0.80)
    );
    assert_eq!(
        model["capability_row_contract"]["required_fields"],
        json!([
            "id",
            "kind",
            "value",
            "evidence",
            "runtime_proof",
            "transfer_target",
            "fit_rationale"
        ])
    );
}

#[test]
fn dossier_validation_fails_closed_on_missing_authority_or_capability_shape() {
    let mut dossier = sample_dossier();
    dossier.authority_confidence = 1.2;
    assert_eq!(
        validate_system_understanding_dossier(&dossier),
        Err("invalid_authority_confidence".to_string())
    );

    let mut dossier = sample_dossier();
    dossier.capabilities[0].fit_rationale.clear();
    assert_eq!(
        validate_system_understanding_dossier(&dossier),
        Err("missing_capability_fit_rationale".to_string())
    );
}

#[test]
fn dossier_validation_accepts_complete_kernel_owned_sample() {
    let dossier = sample_dossier();
    assert!(validate_system_understanding_dossier(&dossier).is_ok());
}
