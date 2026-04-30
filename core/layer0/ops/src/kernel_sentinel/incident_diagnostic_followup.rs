// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    authorize_kernel_sentinel_diagnostic_request, KernelSentinelDiagnosticAuthorizationStatus,
    KernelSentinelDiagnosticBudgetImpact, KernelSentinelDiagnosticProbeClass,
    KernelSentinelDiagnosticRequest, KernelSentinelDiagnosticSafetyClass,
    KernelSentinelFailureLevel,
};

pub fn build_incident_diagnostic_follow_up_request(
    occurrence_count: usize,
    failure_level: KernelSentinelFailureLevel,
    likely_root_frame: &str,
    stop_patching_reasons: &[String],
    evidence_refs: &[String],
    summaries: &[String],
) -> Option<KernelSentinelDiagnosticRequest> {
    let follow_up = select_safe_follow_up(
        failure_level,
        likely_root_frame,
        stop_patching_reasons,
        evidence_refs,
        summaries,
    )?;
    let request = KernelSentinelDiagnosticRequest {
        schema_version: super::KERNEL_SENTINEL_DIAGNOSTIC_REQUEST_SCHEMA_VERSION,
        request_id: format!("incident-follow-up-{}", follow_up.request_slug),
        incident_id: format!("incident-occurrence-{occurrence_count}"),
        failure_signature: follow_up.failure_signature.to_string(),
        hypothesis: follow_up.hypothesis.to_string(),
        competing_explanation: follow_up.competing_explanation.to_string(),
        probe_class: follow_up.probe_class,
        selected_probe: follow_up.selected_probe.to_string(),
        expected_confidence_gain: follow_up.expected_confidence_gain,
        safety_class: follow_up.safety_class,
        budget_impact: KernelSentinelDiagnosticBudgetImpact {
            projected_probe_count: 1,
            projected_runtime_seconds: follow_up.projected_runtime_seconds,
            projected_total_runtime_seconds: follow_up.projected_runtime_seconds,
            projected_scope_escalation_depth: 0,
        },
    };
    let authorization = authorize_kernel_sentinel_diagnostic_request(&request);
    if authorization.status == KernelSentinelDiagnosticAuthorizationStatus::Authorized {
        Some(request)
    } else {
        None
    }
}

struct FollowUpBundle {
    request_slug: &'static str,
    failure_signature: &'static str,
    hypothesis: &'static str,
    competing_explanation: &'static str,
    probe_class: KernelSentinelDiagnosticProbeClass,
    selected_probe: &'static str,
    expected_confidence_gain: f64,
    safety_class: KernelSentinelDiagnosticSafetyClass,
    projected_runtime_seconds: u32,
}

fn select_safe_follow_up(
    failure_level: KernelSentinelFailureLevel,
    likely_root_frame: &str,
    stop_patching_reasons: &[String],
    evidence_refs: &[String],
    summaries: &[String],
) -> Option<FollowUpBundle> {
    let haystack = evidence_refs
        .iter()
        .chain(summaries.iter())
        .map(|row| row.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let mentions = |needle: &str| haystack.iter().any(|row| row.contains(needle));

    if stop_patching_reasons
        .iter()
        .any(|reason| reason == "authority_shaped_residue")
    {
        return Some(FollowUpBundle {
            request_slug: "authority-shape-replay",
            failure_signature: "authority_shape_residue_reemergence",
            hypothesis:
                "golden replay can disambiguate whether authority residue remains structurally reproducible",
            competing_explanation:
                "existing residue evidence may be stale or misprojected instead of proving a live structural leak",
            probe_class: KernelSentinelDiagnosticProbeClass::DiagnosticReplay,
            selected_probe: "golden://kernel_sentinel/authority_shape_residue",
            expected_confidence_gain: 0.28,
            safety_class: KernelSentinelDiagnosticSafetyClass::DeterministicReplaySafe,
            projected_runtime_seconds: 20,
        });
    }

    if stop_patching_reasons
        .iter()
        .any(|reason| reason == "source_of_truth_ambiguity")
        || stop_patching_reasons
            .iter()
            .any(|reason| reason == "command_output_contradicts_runtime_observation")
        || stop_patching_reasons
            .iter()
            .any(|reason| reason == "process_ownership_invariant_breach")
    {
        let healthz_variant = mentions("healthz_not_ready") || mentions("healthz");
        return Some(FollowUpBundle {
            request_slug: if healthz_variant {
                "dashboard-health-topology"
            } else {
                "gateway-listener-topology"
            },
            failure_signature: if healthz_variant {
                "dashboard_healthz_not_ready"
            } else {
                "gateway_restart_success_without_listener"
            },
            hypothesis:
                "a read-only topology probe can disambiguate runtime truth from shell or command-surface success claims",
            competing_explanation:
                "the contradiction may come from stale shell presentation or a mismatched process/lifecycle projection instead of runtime unavailability",
            probe_class: KernelSentinelDiagnosticProbeClass::DiagnosticTopologyProbe,
            selected_probe: if healthz_variant {
                "health://dashboard/healthz"
            } else {
                "listener://gateway/dashboard_host"
            },
            expected_confidence_gain: 0.26,
            safety_class: KernelSentinelDiagnosticSafetyClass::ReadOnlySafe,
            projected_runtime_seconds: 15,
        });
    }

    if failure_level == KernelSentinelFailureLevel::L3PolicyTruthFailure
        && likely_root_frame == "authority_policy_contradiction"
        && mentions("probe")
    {
        return Some(FollowUpBundle {
            request_slug: "typed-probe-contract-regression",
            failure_signature: "typed_probe_contract_gap",
            hypothesis:
                "a targeted regression can disambiguate whether the typed probe contract still fails in the control path",
            competing_explanation:
                "the observed failure may come from downstream evidence projection rather than the contract family itself",
            probe_class: KernelSentinelDiagnosticProbeClass::DiagnosticTest,
            selected_probe: "regression://kernel_sentinel/diagnostic_authorization",
            expected_confidence_gain: 0.24,
            safety_class: KernelSentinelDiagnosticSafetyClass::TargetedTestSafe,
            projected_runtime_seconds: 30,
        });
    }

    None
}
