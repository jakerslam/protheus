// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use super::SubdomainBoundary;

pub fn boundary() -> SubdomainBoundary {
    SubdomainBoundary {
        id: "recovery_escalation",
        legacy_module_bindings: &["recovery", "clarification", "posture"],
        allowed_kernel_inputs: &[
            "execution_observation_snapshot",
            "core_probe_envelope",
            "policy_scope_snapshot",
        ],
        allowed_kernel_outputs: &[
            "recovery_recommendation_envelope",
            "clarification_request_envelope",
            "degradation_projection",
        ],
        message_boundaries: &[
            "recovery_to_packaging_boundary",
            "recovery_to_kernel_recommendation_boundary",
        ],
    }
}
