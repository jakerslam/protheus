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

pub fn control_plane_api_contract() -> ControlPlaneApiContract {
    ControlPlaneApiContract {
        allowed_kernel_inputs: &[
            "core_probe_envelope",
            "typed_request_snapshot",
            "execution_observation_snapshot",
            "capability_probe_snapshot",
            "policy_scope_snapshot",
        ],
        allowed_kernel_outputs: &[
            "core_contract_call_envelope",
            "task_fabric_proposal_envelope",
            "tool_broker_request_envelope",
            "recovery_recommendation_envelope",
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
