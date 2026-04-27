// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/assimilation (authoritative).

pub mod abstraction;
pub mod admission;
pub mod capability_gap;
pub mod domain_wrapper;
pub mod frontier;
pub mod identity;
pub mod ir0;
pub mod ir1;
pub mod ir2;
pub mod receipts;

use abstraction::{AbstractionStep, HierarchicalAbstraction, RefinementHook, UncertaintyMetadata};
use admission::{
    AdmissionCheck, AdmissionProtocol, MutationClass, PlaneRole, Proposal, StateTransition,
};
use capability_gap::{
    evaluate_capability_mapping, CapabilityClass, CapabilityMappingOutput, FallbackPolicy,
};
use frontier::{EnvironmentAssumptionClass, FrontierManager, FrontierPath};
use identity::IdentityResolver;
use ir0::Ir0ArtifactGraph;
use ir1::Ir1ExecutionStructure;
use ir2::Ir2SemanticLift;
use receipts::{
    receipt_id, DependencyGraph, ResumeIndex, TransformationReceipt, UncertaintyEngine,
};

#[derive(Debug, Clone)]
pub struct AssimilationRunInput {
    pub target_id: String,
    pub ir0: Ir0ArtifactGraph,
    pub policy_version: String,
    pub toolchain_fingerprint: String,
    pub assumption_set_hash: String,
    pub equivalence_scope: String,
    pub proposed_by: PlaneRole,
    pub capability_class: CapabilityClass,
    pub target_surface: String,
    pub equivalent_supported: bool,
    pub fallback_policy: FallbackPolicy,
}

#[derive(Debug, Clone)]
pub struct AssimilationRunOutput {
    pub ir1: Ir1ExecutionStructure,
    pub ir2: Ir2SemanticLift,
    pub frontier_kept: Vec<FrontierPath>,
    pub frontier_pruned: Vec<FrontierPath>,
    pub abstraction: HierarchicalAbstraction,
    pub admission: AdmissionCheck,
    pub transition: Option<StateTransition>,
    pub capability_mapping: CapabilityMappingOutput,
    pub receipts: Vec<TransformationReceipt>,
}

#[derive(Debug, Default, Clone)]
pub struct AssimilationKernel {
    pub admission_protocol: AdmissionProtocol,
    pub frontier_manager: FrontierManager,
    pub identity_resolver: IdentityResolver,
    pub dependency_graph: DependencyGraph,
    pub resume_index: ResumeIndex,
}

impl AssimilationKernel {
    pub fn run(&mut self, input: AssimilationRunInput) -> Result<AssimilationRunOutput, String> {
        input.ir0.validate()?;

        let ir1 = Ir1ExecutionStructure::commit_from_ir0(&input.ir0);
        ir1.validate_against_ir0(&input.ir0)?;

        let mut candidate_paths = build_frontier_candidates(&ir1);
        let (frontier_kept, frontier_pruned) = self
            .frontier_manager
            .prune(std::mem::take(&mut candidate_paths));

        let mut ir2 = Ir2SemanticLift::from_ir1(&ir1);
        ir2.admit_hypotheses(0.55)?;
        ir2.validate()?;
        let concept_ids = ir2
            .ontology
            .concepts
            .iter()
            .map(|concept| concept.concept_id.clone())
            .collect::<Vec<_>>();
        let _canonical_concepts = self.identity_resolver.collapse_duplicates(&concept_ids);

        let abstraction = build_default_abstraction()?;

        let proposal = Proposal {
            proposal_id: format!("proposal:{}", input.target_id),
            target_id: input.target_id.clone(),
            requested_by: input.proposed_by.clone(),
            mutation_class: MutationClass::CanonicalStateMutation,
            summary: "Canonical semantic assimilation transition".to_string(),
        };
        let admission = self.admission_protocol.evaluate(&proposal);
        let transition = if admission.admitted {
            Some(self.admission_protocol.execute_transition(
                &proposal,
                &admission,
                PlaneRole::Assimilation,
                "canonical_semantic_lift_committed",
            )?)
        } else {
            None
        };

        let capability_mapping = evaluate_capability_mapping(
            input.capability_class.clone(),
            "binary_artifact",
            &input.target_surface,
            input.equivalent_supported,
            input.fallback_policy.clone(),
        );

        let mut receipts = Vec::new();
        let receipt_ir0 = build_receipt(
            &input,
            &[],
            ReceiptBuildSpec {
                plane: "safety",
                stage: "ir0",
                action: "ingest_artifact_graph",
                proof_type: "byte_exact_observation",
                confidence: 0.98,
                coverage: 0.95,
                local_uncertainty: vec![0.01, 0.02, 0.03],
                capability_gaps: vec![],
                degraded: false,
            },
        )?;
        receipts.push(receipt_ir0);
        let receipt_ir1 = build_receipt(
            &input,
            &[&receipts[0]],
            ReceiptBuildSpec {
                plane: "assimilation",
                stage: "ir1",
                action: "commit_execution_structure",
                proof_type: "control_flow_commit",
                confidence: 0.92,
                coverage: 0.88,
                local_uncertainty: vec![0.08, 0.09, 0.10],
                capability_gaps: vec![],
                degraded: false,
            },
        )?;
        self.dependency_graph
            .add_dependency(&receipts[0].receipt_id, &receipt_ir1.receipt_id);
        receipts.push(receipt_ir1);
        let receipt_ir2 = build_receipt(
            &input,
            &[&receipts[1]],
            ReceiptBuildSpec {
                plane: "assimilation",
                stage: "ir2",
                action: "semantic_lift",
                proof_type: "proof_linked_ontology",
                confidence: 0.86,
                coverage: 0.80,
                local_uncertainty: vec![0.15, 0.16, 0.12],
                capability_gaps: capability_mapping.gap_report.gaps.clone(),
                degraded: capability_mapping.degradation.degraded,
            },
        )?;
        self.dependency_graph
            .add_dependency(&receipts[1].receipt_id, &receipt_ir2.receipt_id);
        receipts.push(receipt_ir2);

        if let Some(last) = receipts.last() {
            self.resume_index
                .update(&input.ir0.artifact_hash, &last.receipt_id);
        }

        Ok(AssimilationRunOutput {
            ir1,
            ir2,
            frontier_kept,
            frontier_pruned,
            abstraction,
            admission,
            transition,
            capability_mapping,
            receipts,
        })
    }
}

fn build_frontier_candidates(ir1: &Ir1ExecutionStructure) -> Vec<FrontierPath> {
    ir1.blocks
        .iter()
        .enumerate()
        .map(|(idx, block)| FrontierPath {
            path_id: format!("frontier:{idx:04}"),
            symbolic_constraints: vec![
                format!("start_offset={}", block.start_offset),
                format!("end_offset={}", block.end_offset),
            ],
            information_gain_score: 0.35 + (idx as f64 * 0.03),
            compute_cost_score: 1.0 + (idx as f64 * 0.05),
            assumption_class: if idx % 5 == 0 {
                EnvironmentAssumptionClass::TimingSensitive
            } else {
                EnvironmentAssumptionClass::NativeOs
            },
            pruned: false,
            prune_reason: None,
        })
        .collect()
}

fn build_default_abstraction() -> Result<HierarchicalAbstraction, String> {
    let mut abstraction = HierarchicalAbstraction::default();
    abstraction.add_step(AbstractionStep {
        step_id: "abst-0001".to_string(),
        operator: "region_to_behavioral_cluster".to_string(),
        source_ids: vec!["ir1:block".to_string()],
        target_id: "ir2:cluster".to_string(),
        uncertainty: UncertaintyMetadata {
            confidence: 0.84,
            uncertainty_vector: vec![0.11, 0.15, 0.09],
            unresolved_assumptions: vec!["unknown_mmio_layout".to_string()],
            loss_class: "behavioral_generalization".to_string(),
        },
        back_references: vec!["ir1:block".to_string()],
        reversible: true,
        requires_reversible: true,
    })?;
    abstraction.add_refinement_hook(RefinementHook {
        hook_id: "hook-mmio-refine".to_string(),
        stage: "ir2".to_string(),
        validator: "mmio_layout_probe".to_string(),
    });
    Ok(abstraction)
}

struct ReceiptBuildSpec {
    plane: &'static str,
    stage: &'static str,
    action: &'static str,
    proof_type: &'static str,
    confidence: f64,
    coverage: f64,
    local_uncertainty: Vec<f64>,
    capability_gaps: Vec<String>,
    degraded: bool,
}

fn build_receipt(
    input: &AssimilationRunInput,
    parents: &[&TransformationReceipt],
    spec: ReceiptBuildSpec,
) -> Result<TransformationReceipt, String> {
    let parent_receipt_ids = parents
        .iter()
        .map(|receipt| receipt.receipt_id.clone())
        .collect::<Vec<_>>();
    let propagated = UncertaintyEngine::propagate(
        &parents
            .iter()
            .map(|receipt| (*receipt).clone())
            .collect::<Vec<_>>(),
        &spec.local_uncertainty,
    );
    let seed = format!(
        "{}:{}:{}:{}:{}:{}",
        input.ir0.artifact_hash,
        spec.stage,
        spec.action,
        input.policy_version,
        input.assumption_set_hash,
        spec.degraded
    );
    let receipt = TransformationReceipt {
        receipt_id: receipt_id(&seed),
        parent_receipt_ids,
        artifact_hash: input.ir0.artifact_hash.clone(),
        plane: spec.plane.to_string(),
        stage: spec.stage.to_string(),
        action: spec.action.to_string(),
        policy_version: input.policy_version.clone(),
        toolchain_fingerprint: input.toolchain_fingerprint.clone(),
        assumption_set_hash: input.assumption_set_hash.clone(),
        equivalence_scope: input.equivalence_scope.clone(),
        proof_type: spec.proof_type.to_string(),
        confidence: spec.confidence,
        coverage: spec.coverage,
        uncertainty_vector: propagated,
        capability_gaps: spec.capability_gaps,
        degraded: spec.degraded,
        event_id: format!("evt:{}:{}", spec.stage, spec.action),
    };
    receipt.validate()?;
    Ok(receipt)
}
