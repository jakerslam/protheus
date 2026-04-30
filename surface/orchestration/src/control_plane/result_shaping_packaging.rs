// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use super::{SubdomainBoundary, SubdomainContract};

pub struct ResultShapingPackagingContract;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticVisibility {
    TelemetryOnly,
    ChatVisibleFinalLlmOutput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticProjection {
    pub origin: String,
    pub detail: String,
    pub visibility: DiagnosticVisibility,
}

impl SubdomainContract for ResultShapingPackagingContract {
    fn boundary() -> SubdomainBoundary {
        boundary()
    }
}

pub fn package_runtime_diagnostic(origin: &str, detail: &str) -> Option<DiagnosticProjection> {
    let origin = origin.trim();
    let detail = detail.trim();
    if detail.is_empty() {
        return None;
    }
    Some(DiagnosticProjection {
        origin: if origin.is_empty() {
            "runtime:diagnostic".to_string()
        } else {
            origin.to_string()
        },
        detail: detail.to_string(),
        visibility: DiagnosticVisibility::TelemetryOnly,
    })
}

pub fn package_final_llm_output(detail: &str) -> Option<DiagnosticProjection> {
    let detail = detail.trim();
    if detail.is_empty() {
        return None;
    }
    Some(DiagnosticProjection {
        origin: "workflow:final_llm_output".to_string(),
        detail: detail.to_string(),
        visibility: DiagnosticVisibility::ChatVisibleFinalLlmOutput,
    })
}

pub fn boundary() -> SubdomainBoundary {
    SubdomainBoundary {
        id: "result_shaping_packaging",
        legacy_module_bindings: &["result_packaging", "progress", "contracts"],
        allowed_kernel_inputs: &[
            "execution_observation_snapshot",
            "core_probe_envelope",
            "typed_request_snapshot",
            "workspace_tooling_probe_snapshot",
        ],
        allowed_kernel_outputs: &[
            "result_package_projection",
            "fallback_action_projection",
            "human_readable_progress_projection",
            "diagnostic_telemetry_projection",
        ],
        message_boundaries: &[
            "packaging_to_shell_boundary",
            "packaging_to_synthesis_summary_boundary",
            "packaging_to_kernel_recommendation_boundary",
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_diagnostics_are_telemetry_only_by_default() {
        let projection = package_runtime_diagnostic("runtime:error", "gateway unavailable")
            .expect("non-empty diagnostic should package");

        assert_eq!(projection.origin, "runtime:error");
        assert_eq!(projection.detail, "gateway unavailable");
        assert_eq!(projection.visibility, DiagnosticVisibility::TelemetryOnly);
    }

    #[test]
    fn only_final_llm_packaging_is_chat_visible() {
        let projection = package_final_llm_output("Here is the answer.")
            .expect("non-empty final output should package");

        assert_eq!(projection.origin, "workflow:final_llm_output");
        assert_eq!(
            projection.visibility,
            DiagnosticVisibility::ChatVisibleFinalLlmOutput
        );
    }
}
