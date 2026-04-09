use infring_assimilation_core_v1::abstraction::{
    AbstractionStep, HierarchicalAbstraction, UncertaintyMetadata,
};
use infring_assimilation_core_v1::admission::{
    AdmissionProtocol, MutationClass, PlaneRole, Proposal,
};
use infring_assimilation_core_v1::capability_gap::{
    evaluate_capability_mapping, CapabilityClass, FallbackPolicy,
};
use infring_assimilation_core_v1::domain_wrapper::{
    AssimilationCoreView, DomainWrapper, GameRemasterWrapper,
};
use infring_assimilation_core_v1::ir0::{
    DecodeCandidate, Ir0ArtifactGraph, Ir0Region, ProvenanceAnchor,
};
use infring_assimilation_core_v1::{AssimilationKernel, AssimilationRunInput};

fn sample_ir0() -> Ir0ArtifactGraph {
    Ir0ArtifactGraph {
        artifact_id: "artifact-demo".to_string(),
        artifact_hash: "sha256:demo".to_string(),
        regions: vec![
            Ir0Region {
                region_id: "r0".to_string(),
                offset: 0,
                length: 64,
                bytes_sha256: "sha256:r0".to_string(),
                interleaved_code_data: true,
                packed_region: false,
                embedded_blob: false,
                partial_self_modifying: false,
                decode_candidates: vec![
                    DecodeCandidate {
                        candidate_id: "cand-a".to_string(),
                        decoder: "x86_64".to_string(),
                        hypothesis: "code".to_string(),
                        window_start: 0,
                        window_end: 40,
                        confidence: 0.7,
                    },
                    DecodeCandidate {
                        candidate_id: "cand-b".to_string(),
                        decoder: "blob".to_string(),
                        hypothesis: "data".to_string(),
                        window_start: 24,
                        window_end: 64,
                        confidence: 0.62,
                    },
                ],
                provenance_anchor: ProvenanceAnchor {
                    artifact_hash: "sha256:demo".to_string(),
                    offset: 0,
                    length: 64,
                    source_hint: "ingest".to_string(),
                },
            },
            Ir0Region {
                region_id: "r1".to_string(),
                offset: 64,
                length: 48,
                bytes_sha256: "sha256:r1".to_string(),
                interleaved_code_data: true,
                packed_region: true,
                embedded_blob: true,
                partial_self_modifying: true,
                decode_candidates: vec![DecodeCandidate {
                    candidate_id: "cand-c".to_string(),
                    decoder: "fallback".to_string(),
                    hypothesis: "mixed".to_string(),
                    window_start: 64,
                    window_end: 112,
                    confidence: 0.55,
                }],
                provenance_anchor: ProvenanceAnchor {
                    artifact_hash: "sha256:demo".to_string(),
                    offset: 64,
                    length: 48,
                    source_hint: "ingest".to_string(),
                },
            },
        ],
        edges: vec![],
    }
}

#[test]
fn ir0_accepts_overlapping_decode_windows() {
    let ir0 = sample_ir0();
    assert!(ir0.validate().is_ok());
}

#[test]
fn admission_blocks_cognition_canonical_mutations() {
    let protocol = AdmissionProtocol::default();
    let proposal = Proposal {
        proposal_id: "p-1".to_string(),
        target_id: "target".to_string(),
        requested_by: PlaneRole::Cognition,
        mutation_class: MutationClass::CanonicalStateMutation,
        summary: "should be denied".to_string(),
    };
    let check = protocol.evaluate(&proposal);
    assert!(!check.admitted);
    assert!(check.reason.contains("cognition_cannot_mutate"));
}

#[test]
fn capability_gap_requires_explicit_degradation_for_high_risk_classes() {
    let mapping = evaluate_capability_mapping(
        CapabilityClass::TimingSensitive,
        "legacy_binary",
        "modern_target",
        false,
        FallbackPolicy::ApproximateWithReceipt,
    );
    assert!(mapping.degradation.degraded);
    assert!(mapping
        .gap_report
        .gaps
        .contains(&"high_risk_capability_class".to_string()));
}

#[test]
fn assimilation_kernel_emits_lineage_receipts() {
    let mut kernel = AssimilationKernel::default();
    let output = kernel
        .run(AssimilationRunInput {
            target_id: "legacy-demo".to_string(),
            ir0: sample_ir0(),
            policy_version: "assimilation_policy_v1".to_string(),
            toolchain_fingerprint: "rustc-1.84".to_string(),
            assumption_set_hash: "assume:default".to_string(),
            equivalence_scope: "behavioral".to_string(),
            proposed_by: PlaneRole::Assimilation,
            capability_class: CapabilityClass::GeneralCompute,
            target_surface: "native_runtime".to_string(),
            equivalent_supported: true,
            fallback_policy: FallbackPolicy::Emulate,
        })
        .expect("assimilation run should succeed");
    assert_eq!(output.receipts.len(), 3);
    assert!(output.receipts[1]
        .parent_receipt_ids
        .contains(&output.receipts[0].receipt_id));
    assert!(output.admission.admitted);
    assert!(output.transition.is_some());
}

#[test]
fn abstraction_requires_backrefs_when_reversible() {
    let mut abstraction = HierarchicalAbstraction::default();
    let result = abstraction.add_step(AbstractionStep {
        step_id: "s-1".to_string(),
        operator: "test".to_string(),
        source_ids: vec!["source".to_string()],
        target_id: "target".to_string(),
        uncertainty: UncertaintyMetadata {
            confidence: 0.7,
            uncertainty_vector: vec![0.2, 0.3],
            unresolved_assumptions: vec![],
            loss_class: "lossy".to_string(),
        },
        back_references: vec![],
        reversible: true,
        requires_reversible: true,
    });
    assert!(result.is_err());
}

#[derive(Default)]
struct FakeCoreView;

impl AssimilationCoreView for FakeCoreView {
    fn concepts_for_artifact(
        &self,
        _artifact_hash: &str,
    ) -> Vec<infring_assimilation_core_v1::ir2::CanonicalConcept> {
        vec![]
    }

    fn receipt_lineage_for_artifact(&self, _artifact_hash: &str) -> Vec<String> {
        vec!["rcpt:a".to_string(), "rcpt:b".to_string()]
    }
}

#[test]
fn domain_wrapper_stays_thin_and_query_only() {
    let wrapper = GameRemasterWrapper;
    let projection = wrapper
        .project(&FakeCoreView, "sha256:demo")
        .expect("projection should succeed");
    assert_eq!(projection.domain, "game_remaster");
    assert_eq!(projection.receipt_count, 2);
}
