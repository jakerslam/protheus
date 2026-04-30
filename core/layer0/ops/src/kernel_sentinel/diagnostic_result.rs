// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const KERNEL_SENTINEL_DIAGNOSTIC_RESULT_SCHEMA_VERSION: u32 = 1;
const KERNEL_SENTINEL_DIAGNOSTIC_POLICY_REF: &str =
    "docs/workspace/kernel_sentinel_diagnostic_execution_policy.md";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelSentinelDiagnosticOutcome {
    Pass,
    Fail,
    Inconclusive,
    Refused,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelSentinelDiagnosticStopReason {
    ConfidenceGainExhausted,
    BudgetExhausted,
    UnauthorizedProbeClass,
    UnresolvedEvidenceContradiction,
    SufficientConfidenceReached,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KernelSentinelDiagnosticResult {
    pub schema_version: u32,
    pub result_id: String,
    pub request_id: String,
    pub incident_id: String,
    pub selected_probe: String,
    pub outcome: KernelSentinelDiagnosticOutcome,
    pub confidence_before: f64,
    pub confidence_after: f64,
    pub confidence_delta: f64,
    pub artifacts: Vec<String>,
    pub next_recommended_probe: Option<String>,
    pub stop_reason_code: KernelSentinelDiagnosticStopReason,
    pub stop_reason: String,
}

fn require_nonempty(name: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("missing_{name}"));
    }
    Ok(())
}

fn require_confidence(name: &str, value: f64) -> Result<(), String> {
    if !(0.0..=1.0).contains(&value) {
        return Err(format!("invalid_{name}"));
    }
    Ok(())
}

pub fn validate_kernel_sentinel_diagnostic_result(
    result: &KernelSentinelDiagnosticResult,
) -> Result<(), String> {
    if result.schema_version != KERNEL_SENTINEL_DIAGNOSTIC_RESULT_SCHEMA_VERSION {
        return Err("invalid_schema_version".to_string());
    }
    require_nonempty("result_id", &result.result_id)?;
    require_nonempty("request_id", &result.request_id)?;
    require_nonempty("incident_id", &result.incident_id)?;
    require_nonempty("selected_probe", &result.selected_probe)?;
    require_nonempty("stop_reason", &result.stop_reason)?;
    require_confidence("confidence_before", result.confidence_before)?;
    require_confidence("confidence_after", result.confidence_after)?;
    if !(-1.0..=1.0).contains(&result.confidence_delta) {
        return Err("invalid_confidence_delta".to_string());
    }
    if result.artifacts.iter().any(|item| item.trim().is_empty()) {
        return Err("invalid_artifacts".to_string());
    }
    if let Some(next_probe) = &result.next_recommended_probe {
        require_nonempty("next_recommended_probe", next_probe)?;
    }
    Ok(())
}

pub fn kernel_sentinel_diagnostic_result_model() -> Value {
    json!({
        "ok": true,
        "type": "kernel_sentinel_diagnostic_result_model",
        "schema_version": KERNEL_SENTINEL_DIAGNOSTIC_RESULT_SCHEMA_VERSION,
        "policy_ref": KERNEL_SENTINEL_DIAGNOSTIC_POLICY_REF,
        "required_fields": [
            "result_id",
            "request_id",
            "incident_id",
            "selected_probe",
            "outcome",
            "confidence_before",
            "confidence_after",
            "confidence_delta",
            "artifacts",
            "stop_reason_code",
            "stop_reason"
        ],
        "optional_fields": [
            "next_recommended_probe"
        ],
        "outcomes": [
            "pass",
            "fail",
            "inconclusive",
            "refused"
        ],
        "stop_reasons": [
            "confidence_gain_exhausted",
            "budget_exhausted",
            "unauthorized_probe_class",
            "unresolved_evidence_contradiction",
            "sufficient_confidence_reached"
        ],
        "confidence_contract": {
            "before_after_range": [0.0, 1.0],
            "delta_range": [-1.0, 1.0]
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_result() -> KernelSentinelDiagnosticResult {
        KernelSentinelDiagnosticResult {
            schema_version: KERNEL_SENTINEL_DIAGNOSTIC_RESULT_SCHEMA_VERSION,
            result_id: "diag-result-001".to_string(),
            request_id: "diag-request-001".to_string(),
            incident_id: "incident-001".to_string(),
            selected_probe: "topology://gateway/listener_healthz_readiness".to_string(),
            outcome: KernelSentinelDiagnosticOutcome::Inconclusive,
            confidence_before: 0.45,
            confidence_after: 0.67,
            confidence_delta: 0.22,
            artifacts: vec![
                "local/state/kernel_sentinel/diagnostic_run_current.json".to_string(),
                "core/local/artifacts/churn_guard_current.json".to_string(),
            ],
            next_recommended_probe: Some("replay://shell_gateway_lifecycle".to_string()),
            stop_reason_code: KernelSentinelDiagnosticStopReason::SufficientConfidenceReached,
            stop_reason: "confidence_increased_but_competing_hypothesis_remains".to_string(),
        }
    }

    #[test]
    fn diagnostic_result_model_exposes_expected_contract() {
        let model = kernel_sentinel_diagnostic_result_model();
        assert_eq!(model["type"], "kernel_sentinel_diagnostic_result_model");
        assert_eq!(model["schema_version"], 1);
        assert!(
            model["outcomes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value == "refused")
        );
        assert!(
            model["stop_reasons"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value == "budget_exhausted")
        );
    }

    #[test]
    fn validate_diagnostic_result_accepts_valid_payload() {
        let result = sample_result();
        assert!(validate_kernel_sentinel_diagnostic_result(&result).is_ok());
    }

    #[test]
    fn validate_diagnostic_result_rejects_invalid_confidence_and_missing_stop_reason() {
        let mut result = sample_result();
        result.confidence_delta = 1.5;
        assert_eq!(
            validate_kernel_sentinel_diagnostic_result(&result).unwrap_err(),
            "invalid_confidence_delta"
        );

        let mut result = sample_result();
        result.stop_reason.clear();
        assert_eq!(
            validate_kernel_sentinel_diagnostic_result(&result).unwrap_err(),
            "missing_stop_reason"
        );
    }

    #[test]
    fn diagnostic_result_model_carries_explicit_stop_conditions() {
        let model = kernel_sentinel_diagnostic_result_model();
        let stop_reasons = model["stop_reasons"].as_array().unwrap();
        for expected in [
            "confidence_gain_exhausted",
            "budget_exhausted",
            "unauthorized_probe_class",
            "unresolved_evidence_contradiction",
        ] {
            assert!(stop_reasons.iter().any(|value| value == expected));
        }
    }
}
