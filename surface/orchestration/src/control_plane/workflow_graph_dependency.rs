// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use super::{SubdomainBoundary, SubdomainContract};

pub struct WorkflowGraphDependencyContract;

impl SubdomainContract for WorkflowGraphDependencyContract {
    fn boundary() -> SubdomainBoundary {
        boundary()
    }
}

pub fn boundary() -> SubdomainBoundary {
    SubdomainBoundary {
        id: "workflow_graph_dependency_tracking",
        legacy_module_bindings: &["sequencing", "progress", "transient_context"],
        allowed_kernel_inputs: &[
            "execution_observation_snapshot",
            "policy_scope_snapshot",
            "typed_request_snapshot",
        ],
        allowed_kernel_outputs: &[
            "dependency_graph_projection",
            "step_sequence_projection",
            "progress_projection",
        ],
        message_boundaries: &[
            "graph_to_recovery_boundary",
            "graph_to_packaging_boundary",
            "graph_to_synthesis_boundary",
            "graph_state_transient_only_boundary",
        ],
    }
}
