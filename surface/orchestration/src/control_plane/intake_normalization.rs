// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use super::{SubdomainBoundary, SubdomainContract};

pub struct IntakeNormalizationContract;

impl SubdomainContract for IntakeNormalizationContract {
    fn boundary() -> SubdomainBoundary {
        boundary()
    }
}

pub fn boundary() -> SubdomainBoundary {
    SubdomainBoundary {
        id: "intake_normalization",
        legacy_module_bindings: &["ingress", "ingress/parser", "request_classifier"],
        allowed_kernel_inputs: &[
            "typed_request_snapshot",
            "policy_scope_snapshot",
            "core_probe_envelope",
            "workspace_tooling_probe_snapshot",
        ],
        allowed_kernel_outputs: &[
            "normalized_request_projection",
            "classification_seed",
            "clarification_signal",
        ],
        message_boundaries: &[
            "ingress_to_planning_boundary",
            "ingress_to_tool_route_boundary",
            "surface_to_kernel_snapshot_boundary",
        ],
    }
}
