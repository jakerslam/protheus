// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    authorize_kernel_sentinel_diagnostic_request, validate_kernel_sentinel_diagnostic_result,
    KernelSentinelDiagnosticOutcome, KernelSentinelDiagnosticRequest,
    KernelSentinelDiagnosticResult, KernelSentinelDiagnosticStopReason,
    KERNEL_SENTINEL_DIAGNOSTIC_RESULT_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const DEFAULT_TOPOLOGY_ARTIFACT: &str =
    "local/state/kernel_sentinel/diagnostic_topology_probe_current.json";
const DEFAULT_REPLAY_ARTIFACT: &str =
    "local/state/kernel_sentinel/diagnostic_replay_probe_current.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelSentinelTopologyHealthSnapshot {
    pub healthz_ready: Option<bool>,
    pub listener_ready: Option<bool>,
    pub process_present: Option<bool>,
    pub watchdog_healthy: Option<bool>,
    pub lifecycle_running: Option<bool>,
    pub artifact_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelSentinelGoldenReplaySnapshot {
    pub fixture_detected_expected_incident: bool,
    pub fixture_preserved_invariant_labels: bool,
    pub replay_completed: bool,
    pub artifact_path: Option<String>,
}

pub fn kernel_sentinel_diagnostic_executor_model() -> Value {
    json!({
        "ok": true,
        "type": "kernel_sentinel_diagnostic_executor_model",
        "executor_families": [
            {
                "name": "read_only_topology_health_probe",
                "supported_probe_class": "diagnostic_topology_probe",
                "read_only": true,
                "supported_prefixes": [
                    "topology://",
                    "health://",
                    "listener://",
                    "process://",
                    "watchdog://"
                ],
                "snapshot_fields": [
                    "healthz_ready",
                    "listener_ready",
                    "process_present",
                    "watchdog_healthy",
                    "lifecycle_running"
                ]
            },
            {
                "name": "golden_fixture_replay_probe",
                "supported_probe_class": "diagnostic_replay",
                "read_only": true,
                "supported_prefixes": [
                    "replay://",
                    "golden://",
                    "scenario://"
                ],
                "snapshot_fields": [
                    "fixture_detected_expected_incident",
                    "fixture_preserved_invariant_labels",
                    "replay_completed"
                ]
            }
        ]
    })
}

pub fn execute_kernel_sentinel_read_only_topology_probe(
    request: &KernelSentinelDiagnosticRequest,
    snapshot: &KernelSentinelTopologyHealthSnapshot,
) -> Result<KernelSentinelDiagnosticResult, String> {
    let authorization = authorize_kernel_sentinel_diagnostic_request(request);
    if authorization.status != super::KernelSentinelDiagnosticAuthorizationStatus::Authorized {
        let result = refused_result(
            request,
            authorization.authorization_reason,
            snapshot.artifact_path.as_deref(),
        );
        validate_kernel_sentinel_diagnostic_result(&result)?;
        return Ok(result);
    }

    if request.probe_class != super::KernelSentinelDiagnosticProbeClass::DiagnosticTopologyProbe {
        let result = refused_result(
            request,
            "read_only_topology_executor_requires_diagnostic_topology_probe".to_string(),
            snapshot.artifact_path.as_deref(),
        );
        validate_kernel_sentinel_diagnostic_result(&result)?;
        return Ok(result);
    }

    let confidence_before = 0.35;
    let evidence = [
        snapshot.healthz_ready,
        snapshot.listener_ready,
        snapshot.process_present,
        snapshot.watchdog_healthy,
        snapshot.lifecycle_running,
    ];
    let known_count = evidence.iter().filter(|value| value.is_some()).count();
    let all_ready = evidence.iter().all(|value| matches!(value, Some(true)));
    let any_not_ready = evidence.iter().any(|value| matches!(value, Some(false)));

    let (outcome, confidence_after, confidence_delta, stop_reason, next_probe) = if all_ready {
        (
            KernelSentinelDiagnosticOutcome::Pass,
            0.88,
            0.53,
            (
                KernelSentinelDiagnosticStopReason::SufficientConfidenceReached,
                "topology_health_probe_confirmed_listener_health_and_lifecycle_readiness"
                    .to_string(),
            ),
            None,
        )
    } else if any_not_ready {
        (
            KernelSentinelDiagnosticOutcome::Fail,
            0.84,
            0.49,
            (
                KernelSentinelDiagnosticStopReason::SufficientConfidenceReached,
                "topology_health_probe_found_runtime_readiness_contradiction".to_string(),
            ),
            Some("evidence://kernel/runtime_observation".to_string()),
        )
    } else if known_count >= 2 {
        (
            KernelSentinelDiagnosticOutcome::Inconclusive,
            0.57,
            0.22,
            (
                KernelSentinelDiagnosticStopReason::UnresolvedEvidenceContradiction,
                "topology_health_probe_increased_confidence_but_left_missing_runtime_truth"
                    .to_string(),
            ),
            Some("evidence://kernel/runtime_observation".to_string()),
        )
    } else {
        (
            KernelSentinelDiagnosticOutcome::Inconclusive,
            0.41,
            0.06,
            (
                KernelSentinelDiagnosticStopReason::ConfidenceGainExhausted,
                "topology_health_probe_lacked_enough_observation_fields_to_raise_confidence"
                    .to_string(),
            ),
            Some("health://dashboard/healthz".to_string()),
        )
    };

    let result = KernelSentinelDiagnosticResult {
        schema_version: KERNEL_SENTINEL_DIAGNOSTIC_RESULT_SCHEMA_VERSION,
        result_id: format!("{}:topology", request.request_id),
        request_id: request.request_id.clone(),
        incident_id: request.incident_id.clone(),
        selected_probe: request.selected_probe.clone(),
        outcome,
        confidence_before,
        confidence_after,
        confidence_delta,
        artifacts: vec![snapshot
            .artifact_path
            .clone()
            .unwrap_or_else(|| DEFAULT_TOPOLOGY_ARTIFACT.to_string())],
        next_recommended_probe: next_probe,
        stop_reason_code: stop_reason.0,
        stop_reason: stop_reason.1,
    };
    validate_kernel_sentinel_diagnostic_result(&result)?;
    Ok(result)
}

pub fn execute_kernel_sentinel_golden_replay_probe(
    request: &KernelSentinelDiagnosticRequest,
    snapshot: &KernelSentinelGoldenReplaySnapshot,
) -> Result<KernelSentinelDiagnosticResult, String> {
    let authorization = authorize_kernel_sentinel_diagnostic_request(request);
    if authorization.status != super::KernelSentinelDiagnosticAuthorizationStatus::Authorized {
        let result = refused_result(
            request,
            authorization.authorization_reason,
            snapshot
                .artifact_path
                .as_deref()
                .or(Some(DEFAULT_REPLAY_ARTIFACT)),
        );
        validate_kernel_sentinel_diagnostic_result(&result)?;
        return Ok(result);
    }

    if request.probe_class != super::KernelSentinelDiagnosticProbeClass::DiagnosticReplay {
        let result = refused_result(
            request,
            "golden_replay_executor_requires_diagnostic_replay".to_string(),
            snapshot
                .artifact_path
                .as_deref()
                .or(Some(DEFAULT_REPLAY_ARTIFACT)),
        );
        validate_kernel_sentinel_diagnostic_result(&result)?;
        return Ok(result);
    }

    let confidence_before = 0.42;
    let (outcome, confidence_after, confidence_delta, stop_reason, next_probe) = if snapshot
        .replay_completed
        && snapshot.fixture_detected_expected_incident
        && snapshot.fixture_preserved_invariant_labels
    {
        (
            KernelSentinelDiagnosticOutcome::Pass,
            0.9,
            0.48,
            (
                KernelSentinelDiagnosticStopReason::SufficientConfidenceReached,
                "golden_replay_confirmed_expected_incident_and_invariant_projection".to_string(),
            ),
            None,
        )
    } else if snapshot.replay_completed
        && (!snapshot.fixture_detected_expected_incident
            || !snapshot.fixture_preserved_invariant_labels)
    {
        (
            KernelSentinelDiagnosticOutcome::Fail,
            0.86,
            0.44,
            (
                KernelSentinelDiagnosticStopReason::SufficientConfidenceReached,
                "golden_replay_exposed_fixture_to_runtime_projection_mismatch".to_string(),
            ),
            Some("contract://kernel_sentinel/invariant_registry".to_string()),
        )
    } else {
        (
            KernelSentinelDiagnosticOutcome::Inconclusive,
            0.5,
            0.08,
            (
                KernelSentinelDiagnosticStopReason::ConfidenceGainExhausted,
                "golden_replay_did_not_complete_and_cannot_raise_confidence".to_string(),
            ),
            Some("replay://retry/golden_fixture".to_string()),
        )
    };

    let result = KernelSentinelDiagnosticResult {
        schema_version: KERNEL_SENTINEL_DIAGNOSTIC_RESULT_SCHEMA_VERSION,
        result_id: format!("{}:replay", request.request_id),
        request_id: request.request_id.clone(),
        incident_id: request.incident_id.clone(),
        selected_probe: request.selected_probe.clone(),
        outcome,
        confidence_before,
        confidence_after,
        confidence_delta,
        artifacts: vec![snapshot
            .artifact_path
            .clone()
            .unwrap_or_else(|| DEFAULT_REPLAY_ARTIFACT.to_string())],
        next_recommended_probe: next_probe,
        stop_reason_code: stop_reason.0,
        stop_reason: stop_reason.1,
    };
    validate_kernel_sentinel_diagnostic_result(&result)?;
    Ok(result)
}

fn refused_result(
    request: &KernelSentinelDiagnosticRequest,
    reason: String,
    artifact_path: Option<&str>,
) -> KernelSentinelDiagnosticResult {
    KernelSentinelDiagnosticResult {
        schema_version: KERNEL_SENTINEL_DIAGNOSTIC_RESULT_SCHEMA_VERSION,
        result_id: format!("{}:topology-refused", request.request_id),
        request_id: request.request_id.clone(),
        incident_id: request.incident_id.clone(),
        selected_probe: request.selected_probe.clone(),
        outcome: KernelSentinelDiagnosticOutcome::Refused,
        confidence_before: 0.35,
        confidence_after: 0.35,
        confidence_delta: 0.0,
        artifacts: vec![artifact_path
            .unwrap_or(DEFAULT_TOPOLOGY_ARTIFACT)
            .to_string()],
        next_recommended_probe: None,
        stop_reason_code: KernelSentinelDiagnosticStopReason::UnauthorizedProbeClass,
        stop_reason: reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel_sentinel::{
        KernelSentinelDiagnosticBudgetImpact, KernelSentinelDiagnosticProbeClass,
        KernelSentinelDiagnosticRequest, KernelSentinelDiagnosticSafetyClass,
        KERNEL_SENTINEL_DIAGNOSTIC_REQUEST_SCHEMA_VERSION,
    };

    fn sample_request() -> KernelSentinelDiagnosticRequest {
        KernelSentinelDiagnosticRequest {
            schema_version: KERNEL_SENTINEL_DIAGNOSTIC_REQUEST_SCHEMA_VERSION,
            request_id: "diag-exec-001".to_string(),
            incident_id: "incident-001".to_string(),
            failure_signature: "gateway_restart_success_without_listener".to_string(),
            hypothesis: "listener readiness should match reported gateway success".to_string(),
            competing_explanation: "dashboard shell projection alone is stale".to_string(),
            probe_class: KernelSentinelDiagnosticProbeClass::DiagnosticTopologyProbe,
            selected_probe: "health://gateway/listener_healthz".to_string(),
            expected_confidence_gain: 0.30,
            safety_class: KernelSentinelDiagnosticSafetyClass::ReadOnlySafe,
            budget_impact: KernelSentinelDiagnosticBudgetImpact {
                projected_probe_count: 1,
                projected_runtime_seconds: 15,
                projected_total_runtime_seconds: 15,
                projected_scope_escalation_depth: 0,
            },
        }
    }

    fn sample_replay_request() -> KernelSentinelDiagnosticRequest {
        KernelSentinelDiagnosticRequest {
            schema_version: KERNEL_SENTINEL_DIAGNOSTIC_REQUEST_SCHEMA_VERSION,
            request_id: "diag-replay-001".to_string(),
            incident_id: "incident-replay-001".to_string(),
            failure_signature: "authority_shape_residue_reemergence".to_string(),
            hypothesis: "golden replay should reproduce the architectural incident cleanly"
                .to_string(),
            competing_explanation: "fixture may no longer align with runtime synthesis".to_string(),
            probe_class: KernelSentinelDiagnosticProbeClass::DiagnosticReplay,
            selected_probe: "golden://kernel_sentinel/authority_shape_residue".to_string(),
            expected_confidence_gain: 0.28,
            safety_class: KernelSentinelDiagnosticSafetyClass::DeterministicReplaySafe,
            budget_impact: KernelSentinelDiagnosticBudgetImpact {
                projected_probe_count: 1,
                projected_runtime_seconds: 20,
                projected_total_runtime_seconds: 20,
                projected_scope_escalation_depth: 0,
            },
        }
    }

    #[test]
    fn read_only_topology_probe_passes_when_runtime_surface_is_healthy() {
        let result = execute_kernel_sentinel_read_only_topology_probe(
            &sample_request(),
            &KernelSentinelTopologyHealthSnapshot {
                healthz_ready: Some(true),
                listener_ready: Some(true),
                process_present: Some(true),
                watchdog_healthy: Some(true),
                lifecycle_running: Some(true),
                artifact_path: Some("local/state/kernel_sentinel/topology_ok.json".to_string()),
            },
        )
        .expect("executor should succeed");
        assert_eq!(result.outcome, KernelSentinelDiagnosticOutcome::Pass);
        assert_eq!(
            result.stop_reason_code,
            KernelSentinelDiagnosticStopReason::SufficientConfidenceReached
        );
    }

    #[test]
    fn read_only_topology_probe_fails_when_listener_truth_contradicts_success() {
        let result = execute_kernel_sentinel_read_only_topology_probe(
            &sample_request(),
            &KernelSentinelTopologyHealthSnapshot {
                healthz_ready: Some(true),
                listener_ready: Some(false),
                process_present: Some(true),
                watchdog_healthy: Some(true),
                lifecycle_running: Some(true),
                artifact_path: Some("local/state/kernel_sentinel/topology_fail.json".to_string()),
            },
        )
        .expect("executor should succeed");
        assert_eq!(result.outcome, KernelSentinelDiagnosticOutcome::Fail);
        assert_eq!(
            result.next_recommended_probe,
            Some("evidence://kernel/runtime_observation".to_string())
        );
    }

    #[test]
    fn read_only_topology_probe_refuses_non_topology_probe_class() {
        let mut request = sample_request();
        request.probe_class = KernelSentinelDiagnosticProbeClass::DiagnosticContractProbe;
        request.safety_class = KernelSentinelDiagnosticSafetyClass::ContractBoundedSafe;
        request.selected_probe = "contract://gateway/listener".to_string();
        let result = execute_kernel_sentinel_read_only_topology_probe(
            &request,
            &KernelSentinelTopologyHealthSnapshot {
                healthz_ready: Some(true),
                listener_ready: Some(true),
                process_present: Some(true),
                watchdog_healthy: Some(true),
                lifecycle_running: Some(true),
                artifact_path: None,
            },
        )
        .expect("executor should still return a refused result");
        assert_eq!(result.outcome, KernelSentinelDiagnosticOutcome::Refused);
        assert_eq!(
            result.stop_reason_code,
            KernelSentinelDiagnosticStopReason::UnauthorizedProbeClass
        );
    }

    #[test]
    fn golden_replay_probe_passes_when_fixture_matches_expected_incident() {
        let result = execute_kernel_sentinel_golden_replay_probe(
            &sample_replay_request(),
            &KernelSentinelGoldenReplaySnapshot {
                fixture_detected_expected_incident: true,
                fixture_preserved_invariant_labels: true,
                replay_completed: true,
                artifact_path: Some("local/state/kernel_sentinel/replay_ok.json".to_string()),
            },
        )
        .expect("golden replay should execute");
        assert_eq!(result.outcome, KernelSentinelDiagnosticOutcome::Pass);
        assert_eq!(
            result.stop_reason_code,
            KernelSentinelDiagnosticStopReason::SufficientConfidenceReached
        );
    }

    #[test]
    fn golden_replay_probe_fails_when_fixture_projection_mismatches_runtime_expectation() {
        let result = execute_kernel_sentinel_golden_replay_probe(
            &sample_replay_request(),
            &KernelSentinelGoldenReplaySnapshot {
                fixture_detected_expected_incident: false,
                fixture_preserved_invariant_labels: true,
                replay_completed: true,
                artifact_path: Some("local/state/kernel_sentinel/replay_fail.json".to_string()),
            },
        )
        .expect("golden replay should execute");
        assert_eq!(result.outcome, KernelSentinelDiagnosticOutcome::Fail);
        assert_eq!(
            result.next_recommended_probe,
            Some("contract://kernel_sentinel/invariant_registry".to_string())
        );
    }

    #[test]
    fn golden_replay_probe_refuses_non_replay_probe_class() {
        let mut request = sample_replay_request();
        request.probe_class = KernelSentinelDiagnosticProbeClass::DiagnosticTopologyProbe;
        request.safety_class = KernelSentinelDiagnosticSafetyClass::ReadOnlySafe;
        request.selected_probe = "health://gateway/listener_healthz".to_string();
        let result = execute_kernel_sentinel_golden_replay_probe(
            &request,
            &KernelSentinelGoldenReplaySnapshot {
                fixture_detected_expected_incident: true,
                fixture_preserved_invariant_labels: true,
                replay_completed: true,
                artifact_path: None,
            },
        )
        .expect("executor should return a refused result");
        assert_eq!(result.outcome, KernelSentinelDiagnosticOutcome::Refused);
        assert_eq!(
            result.stop_reason_code,
            KernelSentinelDiagnosticStopReason::UnauthorizedProbeClass
        );
    }
}
