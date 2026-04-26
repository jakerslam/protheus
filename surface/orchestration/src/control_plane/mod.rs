// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
pub mod decomposition_planning;
pub mod intake_normalization;
pub mod lifecycle;
pub mod recovery_escalation;
pub mod result_shaping_packaging;
pub mod templates;
pub mod workflow_contract_guard;
pub mod workflow_contracts;
pub mod workflow_graph_dependency;
pub mod workflow_runtime;
pub mod workflow_runtime_fixtures;
pub mod workflow_runtime_types;

#[cfg(test)]
mod workflow_contracts_tests;
#[cfg(test)]
mod workflow_runtime_tests;

use crate::contracts::{ControlPlaneDecisionTrace, WorkflowStage};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubdomainBoundary {
    pub id: &'static str,
    pub legacy_module_bindings: &'static [&'static str],
    pub allowed_kernel_inputs: &'static [&'static str],
    pub allowed_kernel_outputs: &'static [&'static str],
    pub message_boundaries: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyModuleBinding {
    pub module: &'static str,
    pub subdomain_id: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlPlaneApiContract {
    pub allowed_kernel_inputs: &'static [&'static str],
    pub allowed_kernel_outputs: &'static [&'static str],
    pub forbidden_authority_domains: &'static [&'static str],
    pub message_boundary_invariants: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubdomainTraceContract {
    pub trace_id: &'static str,
    pub subdomain_id: &'static str,
    pub stage: WorkflowStage,
    pub required_decision_fields: &'static [&'static str],
    pub receipt_metadata_sources: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractViolationKind {
    UnknownSubdomain,
    KernelInputDenied,
    KernelOutputDenied,
    MessageBoundaryDenied,
    ForbiddenAuthorityDomain,
    GlobalContractMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlPlaneContractViolation {
    pub kind: ContractViolationKind,
    pub subdomain_id: Option<String>,
    pub token: String,
    pub allowed: Vec<String>,
}

pub trait SubdomainContract {
    fn boundary() -> SubdomainBoundary;

    fn assert_kernel_input_allowed(input: &str) -> Result<(), ControlPlaneContractViolation> {
        enforce_subdomain_kernel_input(Self::boundary().id, input)
    }

    fn assert_kernel_output_allowed(output: &str) -> Result<(), ControlPlaneContractViolation> {
        enforce_subdomain_kernel_output(Self::boundary().id, output)
    }

    fn assert_message_boundary_allowed(
        boundary: &str,
    ) -> Result<(), ControlPlaneContractViolation> {
        enforce_subdomain_message_boundary(Self::boundary().id, boundary)
    }
}

pub fn control_plane_api_contract() -> ControlPlaneApiContract {
    ControlPlaneApiContract {
        allowed_kernel_inputs: &[
            "core_probe_envelope",
            "typed_request_snapshot",
            "execution_observation_snapshot",
            "capability_probe_snapshot",
            "policy_scope_snapshot",
            "workspace_tooling_probe_snapshot",
        ],
        allowed_kernel_outputs: &[
            "core_contract_call_envelope",
            "task_fabric_proposal_envelope",
            "tool_broker_request_envelope",
            "tool_route_recommendation_envelope",
            "recovery_recommendation_envelope",
            "clarification_request_envelope",
            "result_package_projection",
        ],
        forbidden_authority_domains: &[
            "canonical_policy_truth",
            "execution_admission_truth",
            "deterministic_receipt_authority",
            "scheduler_truth",
            "queue_truth",
        ],
        message_boundary_invariants: &[
            "control_plane_reads_kernel_snapshots_only",
            "control_plane_writes_recommendations_only",
            "control_plane_receipt_binding_forbidden",
            "kernel_is_final_authority",
        ],
    }
}

pub fn enforce_subdomain_kernel_input(
    subdomain_id: &str,
    input: &str,
) -> Result<(), ControlPlaneContractViolation> {
    let Some(subdomain) = subdomain_boundary_by_id(subdomain_id) else {
        return Err(unknown_subdomain_violation(subdomain_id, input));
    };
    if !subdomain.allowed_kernel_inputs.contains(&input) {
        return Err(ControlPlaneContractViolation {
            kind: ContractViolationKind::KernelInputDenied,
            subdomain_id: Some(subdomain_id.to_string()),
            token: input.to_string(),
            allowed: to_owned_tokens(subdomain.allowed_kernel_inputs),
        });
    }
    let contract = control_plane_api_contract();
    if !contract.allowed_kernel_inputs.contains(&input) {
        return Err(ControlPlaneContractViolation {
            kind: ContractViolationKind::GlobalContractMismatch,
            subdomain_id: Some(subdomain_id.to_string()),
            token: input.to_string(),
            allowed: to_owned_tokens(contract.allowed_kernel_inputs),
        });
    }
    Ok(())
}

pub fn enforce_subdomain_kernel_output(
    subdomain_id: &str,
    output: &str,
) -> Result<(), ControlPlaneContractViolation> {
    let Some(subdomain) = subdomain_boundary_by_id(subdomain_id) else {
        return Err(unknown_subdomain_violation(subdomain_id, output));
    };
    if !subdomain.allowed_kernel_outputs.contains(&output) {
        return Err(ControlPlaneContractViolation {
            kind: ContractViolationKind::KernelOutputDenied,
            subdomain_id: Some(subdomain_id.to_string()),
            token: output.to_string(),
            allowed: to_owned_tokens(subdomain.allowed_kernel_outputs),
        });
    }
    let contract = control_plane_api_contract();
    if !contract.allowed_kernel_outputs.contains(&output) {
        return Err(ControlPlaneContractViolation {
            kind: ContractViolationKind::GlobalContractMismatch,
            subdomain_id: Some(subdomain_id.to_string()),
            token: output.to_string(),
            allowed: to_owned_tokens(contract.allowed_kernel_outputs),
        });
    }
    Ok(())
}

pub fn enforce_subdomain_message_boundary(
    subdomain_id: &str,
    message_boundary: &str,
) -> Result<(), ControlPlaneContractViolation> {
    let Some(subdomain) = subdomain_boundary_by_id(subdomain_id) else {
        return Err(unknown_subdomain_violation(subdomain_id, message_boundary));
    };
    if !subdomain.message_boundaries.contains(&message_boundary) {
        return Err(ControlPlaneContractViolation {
            kind: ContractViolationKind::MessageBoundaryDenied,
            subdomain_id: Some(subdomain_id.to_string()),
            token: message_boundary.to_string(),
            allowed: to_owned_tokens(subdomain.message_boundaries),
        });
    }
    Ok(())
}

pub fn enforce_authority_domain(
    authority_domain: &str,
) -> Result<(), ControlPlaneContractViolation> {
    let contract = control_plane_api_contract();
    if contract
        .forbidden_authority_domains
        .contains(&authority_domain)
    {
        return Err(ControlPlaneContractViolation {
            kind: ContractViolationKind::ForbiddenAuthorityDomain,
            subdomain_id: None,
            token: authority_domain.to_string(),
            allowed: to_owned_tokens(contract.forbidden_authority_domains),
        });
    }
    Ok(())
}

pub fn contract_consistency_violations() -> Vec<ControlPlaneContractViolation> {
    let mut violations = Vec::new();
    let contract = control_plane_api_contract();
    for subdomain in subdomain_boundaries() {
        for input in subdomain.allowed_kernel_inputs {
            if !contract.allowed_kernel_inputs.contains(input) {
                violations.push(ControlPlaneContractViolation {
                    kind: ContractViolationKind::GlobalContractMismatch,
                    subdomain_id: Some(subdomain.id.to_string()),
                    token: (*input).to_string(),
                    allowed: to_owned_tokens(contract.allowed_kernel_inputs),
                });
            }
        }
        for output in subdomain.allowed_kernel_outputs {
            if is_kernel_output_token(output) && !contract.allowed_kernel_outputs.contains(output) {
                violations.push(ControlPlaneContractViolation {
                    kind: ContractViolationKind::GlobalContractMismatch,
                    subdomain_id: Some(subdomain.id.to_string()),
                    token: (*output).to_string(),
                    allowed: to_owned_tokens(contract.allowed_kernel_outputs),
                });
            }
        }
    }
    violations
}

pub fn assert_contract_consistency() -> Result<(), Vec<ControlPlaneContractViolation>> {
    let violations = contract_consistency_violations();
    if violations.is_empty() {
        return Ok(());
    }
    Err(violations)
}

pub fn subdomain_trace_contracts() -> Vec<SubdomainTraceContract> {
    const REQUIRED_DECISION_FIELDS: &[&str] = &[
        "chosen_path",
        "alternatives_rejected",
        "confidence",
        "rationale",
        "receipt_metadata",
    ];
    const RECEIPT_METADATA_SOURCES: &[&str] = &[
        "orchestration_trace_id",
        "expected_core_contract_ids",
        "observed_core_receipt_ids",
        "observed_core_outcome_refs",
    ];
    vec![
        SubdomainTraceContract {
            trace_id: "intake_normalization.trace",
            subdomain_id: "intake_normalization",
            stage: WorkflowStage::IntakeNormalization,
            required_decision_fields: REQUIRED_DECISION_FIELDS,
            receipt_metadata_sources: RECEIPT_METADATA_SOURCES,
        },
        SubdomainTraceContract {
            trace_id: "decomposition_planning.trace",
            subdomain_id: "decomposition_planning",
            stage: WorkflowStage::DecompositionPlanning,
            required_decision_fields: REQUIRED_DECISION_FIELDS,
            receipt_metadata_sources: RECEIPT_METADATA_SOURCES,
        },
        SubdomainTraceContract {
            trace_id: "workflow_graph_dependency_tracking.trace",
            subdomain_id: "workflow_graph_dependency_tracking",
            stage: WorkflowStage::CoordinationSequencing,
            required_decision_fields: REQUIRED_DECISION_FIELDS,
            receipt_metadata_sources: RECEIPT_METADATA_SOURCES,
        },
        SubdomainTraceContract {
            trace_id: "recovery_escalation.trace",
            subdomain_id: "recovery_escalation",
            stage: WorkflowStage::RecoveryEscalation,
            required_decision_fields: REQUIRED_DECISION_FIELDS,
            receipt_metadata_sources: RECEIPT_METADATA_SOURCES,
        },
        SubdomainTraceContract {
            trace_id: "result_shaping_packaging.trace",
            subdomain_id: "result_shaping_packaging",
            stage: WorkflowStage::ResultPackaging,
            required_decision_fields: REQUIRED_DECISION_FIELDS,
            receipt_metadata_sources: RECEIPT_METADATA_SOURCES,
        },
        SubdomainTraceContract {
            trace_id: "verification_closure.trace",
            subdomain_id: "result_shaping_packaging",
            stage: WorkflowStage::VerificationClosure,
            required_decision_fields: REQUIRED_DECISION_FIELDS,
            receipt_metadata_sources: RECEIPT_METADATA_SOURCES,
        },
    ]
}

pub fn subdomain_trace_contract_by_stage(stage: WorkflowStage) -> Option<SubdomainTraceContract> {
    subdomain_trace_contracts()
        .into_iter()
        .find(|row| row.stage == stage)
}

pub fn decision_trace_contract_failures(trace: &ControlPlaneDecisionTrace) -> Vec<&'static str> {
    let mut failures = Vec::new();
    if trace.chosen.trim().is_empty() {
        failures.push("chosen_path");
    }
    if !trace.confidence.is_finite() || !(0.0..=1.0).contains(&trace.confidence) {
        failures.push("confidence");
    }
    if trace.rationale.iter().all(|row| row.trim().is_empty()) {
        failures.push("rationale");
    }
    if trace
        .receipt_metadata
        .iter()
        .all(|row| row.trim().is_empty())
    {
        failures.push("receipt_metadata");
    }
    if trace.step_records.is_empty() {
        failures.push("step_records");
    }
    if trace.step_records.iter().any(|step| {
        step.step_id.trim().is_empty()
            || step.inputs.iter().all(|row| row.trim().is_empty())
            || step.chosen_path.trim().is_empty()
            || !step.confidence.is_finite()
            || !(0.0..=1.0).contains(&step.confidence)
            || step
                .receipt_metadata
                .iter()
                .all(|row| row.trim().is_empty())
    }) {
        failures.push("step_record_malformed");
    }
    failures
}

pub fn assert_decision_trace_contract(
    trace: &ControlPlaneDecisionTrace,
) -> Result<(), Vec<&'static str>> {
    let failures = decision_trace_contract_failures(trace);
    if failures.is_empty() {
        return Ok(());
    }
    Err(failures)
}

pub fn subdomain_boundaries() -> Vec<SubdomainBoundary> {
    vec![
        intake_normalization::boundary(),
        decomposition_planning::boundary(),
        workflow_graph_dependency::boundary(),
        recovery_escalation::boundary(),
        result_shaping_packaging::boundary(),
    ]
}

pub fn subdomain_boundary_by_id(id: &str) -> Option<SubdomainBoundary> {
    subdomain_boundaries().into_iter().find(|row| row.id == id)
}

pub fn legacy_module_bindings() -> Vec<LegacyModuleBinding> {
    let mut bindings = Vec::new();
    for subdomain in subdomain_boundaries() {
        for module in subdomain.legacy_module_bindings {
            bindings.push(LegacyModuleBinding {
                module,
                subdomain_id: subdomain.id,
            });
        }
    }
    bindings
}

fn unknown_subdomain_violation(subdomain_id: &str, token: &str) -> ControlPlaneContractViolation {
    let allowed = subdomain_boundaries()
        .into_iter()
        .map(|row| row.id.to_string())
        .collect::<Vec<_>>();
    ControlPlaneContractViolation {
        kind: ContractViolationKind::UnknownSubdomain,
        subdomain_id: Some(subdomain_id.to_string()),
        token: token.to_string(),
        allowed,
    }
}

fn to_owned_tokens(values: &[&str]) -> Vec<String> {
    values.iter().map(|row| (*row).to_string()).collect()
}

fn is_kernel_output_token(output: &str) -> bool {
    output.ends_with("_envelope") || output == "result_package_projection"
}
