// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use super::{SubdomainBoundary, SubdomainContract};

pub struct DecompositionPlanningContract;

impl SubdomainContract for DecompositionPlanningContract {
    fn boundary() -> SubdomainBoundary {
        boundary()
    }
}

pub fn boundary() -> SubdomainBoundary {
    SubdomainBoundary {
        id: "decomposition_planning",
        legacy_module_bindings: &[
            "planner",
            "planner/plan_candidates",
            "planner/scoring",
            "planner/preconditions",
        ],
        allowed_kernel_inputs: &[
            "typed_request_snapshot",
            "capability_probe_snapshot",
            "execution_observation_snapshot",
        ],
        allowed_kernel_outputs: &[
            "plan_candidate_set",
            "selected_plan_recommendation",
            "core_contract_call_envelope",
        ],
        message_boundaries: &[
            "planning_to_graph_boundary",
            "planning_to_recovery_boundary",
            "planning_to_kernel_recommendation_boundary",
        ],
    }
}
