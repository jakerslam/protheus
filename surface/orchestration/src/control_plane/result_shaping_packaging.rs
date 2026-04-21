// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use super::SubdomainBoundary;

pub fn boundary() -> SubdomainBoundary {
    SubdomainBoundary {
        id: "result_shaping_packaging",
        legacy_module_bindings: &["result_packaging", "progress", "contracts"],
        allowed_kernel_inputs: &[
            "execution_observation_snapshot",
            "core_probe_envelope",
            "typed_request_snapshot",
        ],
        allowed_kernel_outputs: &[
            "result_package_projection",
            "fallback_action_projection",
            "human_readable_progress_projection",
        ],
        message_boundaries: &[
            "packaging_to_shell_boundary",
            "packaging_to_kernel_recommendation_boundary",
        ],
    }
}

