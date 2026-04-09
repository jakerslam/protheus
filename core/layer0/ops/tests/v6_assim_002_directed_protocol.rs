// SPDX-License-Identifier: Apache-2.0
// SRS coverage: V6-ASSIM-002.1 ... V6-ASSIM-002.8

use protheus_ops_core::directed_assimilation_protocol::{
    run_directed_assimilation, AdmissionVerdict, DirectedAssimilationInput, TransferClass,
};

fn base_input(vector_text: &str) -> DirectedAssimilationInput {
    DirectedAssimilationInput {
        vector_text: vector_text.to_string(),
        source_artifact_hash: "sha256:artifact-demo".to_string(),
        target_descriptor_ref: Some("descriptor:target-runtime".to_string()),
        feature_refs: vec!["feature:cfg".to_string(), "feature:symbols".to_string()],
        region_refs: vec!["region:text".to_string(), "region:data".to_string()],
        substrate_descriptors: vec![
            "surface:code runtime:pure capability:trace".to_string(),
            "surface:code runtime:legacy stale:legacy".to_string(),
        ],
        substrate_adapters: vec!["adapter:elf".to_string(), "adapter:pe".to_string()],
        substrate_execution_surfaces: vec!["exec:emulator".to_string(), "exec:lift".to_string()],
        consumer_demands: vec!["demand:low-latency".to_string()],
        observed_license_state: "MIT".to_string(),
        capability_gaps: Vec::new(),
        adaptation_cost: 0.2,
        environment_coupling: 0.3,
        degradation_likelihood: 0.2,
        required_emulation_shims: vec![],
        contamination_risk: 0.1,
        fallback_options: vec!["analysis_only".to_string()],
        operator_override_requested: false,
        policy_version: "assimilation_policy_v6_assim_002".to_string(),
    }
}

#[test]
fn full_chain_emits_protocol_receipts_and_hard_selectors_only_narrow_candidates() {
    let input = base_input(
        "assimilate artifact surfaces hard:require:runtime=pure hard:exclude:stale=legacy soft:capability=trace@0.9 transfer_class=clean_room_spec output_contract=capability_spec",
    );
    let artifacts = run_directed_assimilation(&input).expect("directed assimilation should run");
    assert_eq!(artifacts.candidate_set.candidate_ids.len(), 1);
    assert_eq!(artifacts.candidate_set.source_refs.len(), 1);
    assert!(artifacts.candidate_set.source_refs[0].contains("runtime:pure"));
    assert!(artifacts
        .candidate_closure
        .dependencies
        .contains(&"adapter:elf".to_string()));
    assert!(artifacts
        .protocol_step_receipts
        .iter()
        .any(|row| row.step_kind == "candidate_closure"));
    assert!(artifacts
        .protocol_step_receipts
        .iter()
        .any(|row| row.step_kind == "provisional_gap_report"));
    assert!(artifacts
        .protocol_step_receipts
        .iter()
        .any(|row| row.step_kind == "admission_verdict"));
    assert!(artifacts
        .protocol_step_receipts
        .iter()
        .any(|row| row.step_kind == "assimilation_outcome_receipt"));
}

#[test]
fn verdict_accepted_path() {
    let input = base_input(
        "assimilate artifact surfaces hard:require:runtime=pure transfer_class=clean_room_spec",
    );
    let artifacts = run_directed_assimilation(&input).expect("accepted path should succeed");
    assert_eq!(
        artifacts.admission_verdict.verdict,
        AdmissionVerdict::Accepted
    );
    assert!(artifacts.admitted_assimilation_plan.is_some());
    assert!(artifacts.degradation_receipt.is_none());
    assert!(artifacts.override_request.is_none());
}

#[test]
fn verdict_accepted_with_degradation_path() {
    let mut input = base_input(
        "assimilate artifact surfaces hard:require:runtime=pure transfer_class=behavioral_clone",
    );
    input.capability_gaps = vec!["timing_equivalence".to_string()];
    input.degradation_likelihood = 0.7;
    let artifacts = run_directed_assimilation(&input).expect("degraded path should succeed");
    assert_eq!(
        artifacts.admission_verdict.verdict,
        AdmissionVerdict::AcceptedWithDegradation
    );
    assert!(artifacts.admitted_assimilation_plan.is_some());
    assert!(artifacts.degradation_receipt.is_some());
    assert!(artifacts.override_request.is_none());
}

#[test]
fn verdict_downgraded_to_analysis_only_path() {
    let input = base_input(
        "assimilate artifact surfaces hard:require:runtime=pure transfer_class=analysis_only output_contract=observation_bundle",
    );
    let artifacts = run_directed_assimilation(&input).expect("analysis-only path should succeed");
    assert_eq!(
        artifacts.admission_verdict.verdict,
        AdmissionVerdict::DowngradedToAnalysisOnly
    );
    assert_eq!(
        artifacts.assimilation_outcome_receipt.transfer,
        Some(TransferClass::AnalysisOnly)
    );
    let plan = artifacts
        .admitted_assimilation_plan
        .expect("analysis-only should still emit admitted plan");
    assert_eq!(
        plan.approved_steps,
        vec!["analysis_bundle_only".to_string()]
    );
}

#[test]
fn verdict_rejected_path() {
    let mut input = base_input(
        "assimilate artifact surfaces hard:require:runtime=pure forbidden_license_classes=GPL-3.0",
    );
    input.observed_license_state = "GPL-3.0".to_string();
    let artifacts =
        run_directed_assimilation(&input).expect("rejected path should still emit artifacts");
    assert_eq!(
        artifacts.admission_verdict.verdict,
        AdmissionVerdict::Rejected
    );
    assert!(artifacts.admitted_assimilation_plan.is_none());
    assert!(artifacts.degradation_receipt.is_none());
    assert!(artifacts.override_request.is_none());
    assert_eq!(artifacts.assimilation_outcome_receipt.transfer, None);
}

#[test]
fn verdict_needs_operator_override_path() {
    let mut input = base_input(
        "assimilate artifact surfaces hard:require:runtime=pure transfer_class=direct_lift",
    );
    input.capability_gaps = vec!["hardware_timing".to_string()];
    input.operator_override_requested = true;
    input.contamination_risk = 0.65;
    let artifacts = run_directed_assimilation(&input).expect("override path should emit artifacts");
    assert_eq!(
        artifacts.admission_verdict.verdict,
        AdmissionVerdict::NeedsOperatorOverride
    );
    assert!(artifacts.admitted_assimilation_plan.is_none());
    assert!(artifacts.degradation_receipt.is_none());
    assert!(artifacts.override_request.is_some());
    let override_request = artifacts.override_request.expect("override request");
    assert!(override_request.operator_ack_required);
}
