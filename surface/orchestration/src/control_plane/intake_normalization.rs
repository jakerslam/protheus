// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use super::SubdomainBoundary;

pub fn boundary() -> SubdomainBoundary {
    SubdomainBoundary {
        id: "intake_normalization",
        legacy_module_bindings: &["ingress", "ingress/parser", "request_classifier"],
        allowed_kernel_inputs: &[
            "typed_request_snapshot",
            "policy_scope_snapshot",
            "core_probe_envelope",
        ],
        allowed_kernel_outputs: &[
            "normalized_request_projection",
            "classification_seed",
            "clarification_signal",
        ],
        message_boundaries: &[
            "ingress_to_planning_boundary",
            "surface_to_kernel_snapshot_boundary",
        ],
    }
}
