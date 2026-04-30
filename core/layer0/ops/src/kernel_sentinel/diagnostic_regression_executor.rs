// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    authorize_kernel_sentinel_diagnostic_request, validate_kernel_sentinel_diagnostic_result,
    KernelSentinelDiagnosticAuthorizationStatus, KernelSentinelDiagnosticOutcome,
    KernelSentinelDiagnosticProbeClass, KernelSentinelDiagnosticRequest,
    KernelSentinelDiagnosticResult, KernelSentinelDiagnosticStopReason,
    KERNEL_SENTINEL_DIAGNOSTIC_RESULT_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const DEFAULT_REGRESSION_ARTIFACT: &str =
    "local/state/kernel_sentinel/diagnostic_targeted_regression_current.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelSentinelTargetedRegressionSnapshot {
    pub regression_passed: bool,
    pub regression_completed: bool,
    pub assertion_family_matched: bool,
    pub artifact_path: Option<String>,
}

pub fn kernel_sentinel_targeted_regression_executor_model() -> Value {
    json!({
        "ok": true,
        "type": "kernel_sentinel_targeted_regression_executor_model",
        "executor_family": {
            "name": "targeted_regression_probe",
            "supported_probe_class": "diagnostic_test",
            "read_only": true,
            "supported_prefixes": ["test://", "regression://", "exact://"],
            "snapshot_fields": [
                "regression_passed",
                "regression_completed",
                "assertion_family_matched"
            ]
        }
    })
}

pub fn execute_kernel_sentinel_targeted_regression_probe(
    request: &KernelSentinelDiagnosticRequest,
    snapshot: &KernelSentinelTargetedRegressionSnapshot,
) -> Result<KernelSentinelDiagnosticResult, String> {
    let authorization = authorize_kernel_sentinel_diagnostic_request(request);
    if authorization.status != KernelSentinelDiagnosticAuthorizationStatus::Authorized {
        let result = refused_result(
            request,
            authorization.authorization_reason,
            snapshot.artifact_path.as_deref(),
        );
        validate_kernel_sentinel_diagnostic_result(&result)?;
        return Ok(result);
    }

    if request.probe_class != KernelSentinelDiagnosticProbeClass::DiagnosticTest {
        let result = refused_result(
            request,
            "targeted_regression_executor_requires_diagnostic_test".to_string(),
            snapshot.artifact_path.as_deref(),
        );
        validate_kernel_sentinel_diagnostic_result(&result)?;
        return Ok(result);
    }

    let confidence_before = 0.4;
    let (outcome, confidence_after, confidence_delta, stop_reason_code, stop_reason, next_probe) =
        if snapshot.regression_completed
            && snapshot.regression_passed
            && snapshot.assertion_family_matched
        {
            (
                KernelSentinelDiagnosticOutcome::Pass,
                0.9,
                0.5,
                KernelSentinelDiagnosticStopReason::SufficientConfidenceReached,
                "targeted_regression_confirmed_expected_failure_family_contract".to_string(),
                None,
            )
        } else if snapshot.regression_completed
            && (!snapshot.regression_passed || !snapshot.assertion_family_matched)
        {
            (
                KernelSentinelDiagnosticOutcome::Fail,
                0.86,
                0.46,
                KernelSentinelDiagnosticStopReason::SufficientConfidenceReached,
                "targeted_regression_exposed_contract_or_assertion_family_mismatch".to_string(),
                Some("contract://kernel_sentinel/diagnostic_authorization".to_string()),
            )
        } else {
            (
                KernelSentinelDiagnosticOutcome::Inconclusive,
                0.48,
                0.08,
                KernelSentinelDiagnosticStopReason::ConfidenceGainExhausted,
                "targeted_regression_did_not_complete_and_could_not_raise_confidence".to_string(),
                Some("regression://retry/targeted".to_string()),
            )
        };

    let result = KernelSentinelDiagnosticResult {
        schema_version: KERNEL_SENTINEL_DIAGNOSTIC_RESULT_SCHEMA_VERSION,
        result_id: format!("{}:targeted-regression", request.request_id),
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
            .unwrap_or_else(|| DEFAULT_REGRESSION_ARTIFACT.to_string())],
        next_recommended_probe: next_probe,
        stop_reason_code,
        stop_reason,
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
        result_id: format!("{}:targeted-regression-refused", request.request_id),
        request_id: request.request_id.clone(),
        incident_id: request.incident_id.clone(),
        selected_probe: request.selected_probe.clone(),
        outcome: KernelSentinelDiagnosticOutcome::Refused,
        confidence_before: 0.4,
        confidence_after: 0.4,
        confidence_delta: 0.0,
        artifacts: vec![artifact_path
            .unwrap_or(DEFAULT_REGRESSION_ARTIFACT)
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
        KernelSentinelDiagnosticBudgetImpact, KernelSentinelDiagnosticSafetyClass,
        KERNEL_SENTINEL_DIAGNOSTIC_REQUEST_SCHEMA_VERSION,
    };

    fn sample_request() -> KernelSentinelDiagnosticRequest {
        KernelSentinelDiagnosticRequest {
            schema_version: KERNEL_SENTINEL_DIAGNOSTIC_REQUEST_SCHEMA_VERSION,
            request_id: "diag-targeted-001".to_string(),
            incident_id: "incident-targeted-001".to_string(),
            failure_signature: "typed_probe_contract_gap".to_string(),
            hypothesis: "a targeted regression should confirm the probe-family contract"
                .to_string(),
            competing_explanation:
                "authorization logic may still pass the wrong assertion family".to_string(),
            probe_class: KernelSentinelDiagnosticProbeClass::DiagnosticTest,
            selected_probe: "regression://kernel_sentinel/diagnostic_authorization".to_string(),
            expected_confidence_gain: 0.27,
            safety_class: KernelSentinelDiagnosticSafetyClass::TargetedTestSafe,
            budget_impact: KernelSentinelDiagnosticBudgetImpact {
                projected_probe_count: 1,
                projected_runtime_seconds: 30,
                projected_total_runtime_seconds: 30,
                projected_scope_escalation_depth: 0,
            },
        }
    }

    #[test]
    fn targeted_regression_probe_passes_when_regression_confirms_expected_contract() {
        let result = execute_kernel_sentinel_targeted_regression_probe(
            &sample_request(),
            &KernelSentinelTargetedRegressionSnapshot {
                regression_passed: true,
                regression_completed: true,
                assertion_family_matched: true,
                artifact_path: Some(
                    "local/state/kernel_sentinel/targeted_regression_ok.json".to_string(),
                ),
            },
        )
        .expect("targeted regression should execute");
        assert_eq!(result.outcome, KernelSentinelDiagnosticOutcome::Pass);
        assert_eq!(
            result.stop_reason_code,
            KernelSentinelDiagnosticStopReason::SufficientConfidenceReached
        );
    }

    #[test]
    fn targeted_regression_probe_fails_when_assertion_family_or_contract_mismatches() {
        let result = execute_kernel_sentinel_targeted_regression_probe(
            &sample_request(),
            &KernelSentinelTargetedRegressionSnapshot {
                regression_passed: false,
                regression_completed: true,
                assertion_family_matched: true,
                artifact_path: Some(
                    "local/state/kernel_sentinel/targeted_regression_fail.json".to_string(),
                ),
            },
        )
        .expect("targeted regression should execute");
        assert_eq!(result.outcome, KernelSentinelDiagnosticOutcome::Fail);
        assert_eq!(
            result.next_recommended_probe,
            Some("contract://kernel_sentinel/diagnostic_authorization".to_string())
        );
    }

    #[test]
    fn targeted_regression_probe_refuses_non_test_probe_class() {
        let mut request = sample_request();
        request.probe_class = KernelSentinelDiagnosticProbeClass::DiagnosticReplay;
        request.safety_class = KernelSentinelDiagnosticSafetyClass::DeterministicReplaySafe;
        request.selected_probe = "replay://kernel_sentinel/diagnostic_authorization".to_string();
        let result = execute_kernel_sentinel_targeted_regression_probe(
            &request,
            &KernelSentinelTargetedRegressionSnapshot {
                regression_passed: true,
                regression_completed: true,
                assertion_family_matched: true,
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

    #[test]
    fn targeted_regression_probe_refuses_unmapped_failure_signature_fail_closed() {
        let mut request = sample_request();
        request.failure_signature = "unmapped_failure_signature".to_string();
        let result = execute_kernel_sentinel_targeted_regression_probe(
            &request,
            &KernelSentinelTargetedRegressionSnapshot {
                regression_passed: true,
                regression_completed: true,
                assertion_family_matched: true,
                artifact_path: None,
            },
        )
        .expect("executor should return a refused result");
        assert_eq!(result.outcome, KernelSentinelDiagnosticOutcome::Refused);
        assert_eq!(
            result.stop_reason_code,
            KernelSentinelDiagnosticStopReason::UnauthorizedProbeClass
        );
        assert_eq!(result.stop_reason, "failure_signature_unmapped");
    }

    #[test]
    fn targeted_regression_probe_stops_when_confidence_gain_is_exhausted() {
        let result = execute_kernel_sentinel_targeted_regression_probe(
            &sample_request(),
            &KernelSentinelTargetedRegressionSnapshot {
                regression_passed: false,
                regression_completed: false,
                assertion_family_matched: false,
                artifact_path: Some(
                    "local/state/kernel_sentinel/targeted_regression_inconclusive.json"
                        .to_string(),
                ),
            },
        )
        .expect("targeted regression should execute");
        assert_eq!(result.outcome, KernelSentinelDiagnosticOutcome::Inconclusive);
        assert_eq!(
            result.stop_reason_code,
            KernelSentinelDiagnosticStopReason::ConfidenceGainExhausted
        );
        assert_eq!(
            result.next_recommended_probe,
            Some("regression://retry/targeted".to_string())
        );
    }
}
