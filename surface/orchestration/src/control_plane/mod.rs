// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
pub mod decomposition_planning;
pub mod intake_normalization;
pub mod lifecycle;
pub mod recovery_escalation;
pub mod result_shaping_packaging;
pub mod workflow_graph_dependency;

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
