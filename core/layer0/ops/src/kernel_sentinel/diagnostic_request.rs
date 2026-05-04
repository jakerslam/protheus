// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const KERNEL_SENTINEL_DIAGNOSTIC_REQUEST_SCHEMA_VERSION: u32 = 1;
const KERNEL_SENTINEL_DIAGNOSTIC_POLICY_REF: &str =
    "docs/workspace/kernel_sentinel_diagnostic_execution_policy.md";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelSentinelDiagnosticProbeClass {
    DiagnosticTopologyProbe,
    DiagnosticEvidenceRefresh,
    DiagnosticReplay,
    DiagnosticContractProbe,
    DiagnosticTest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelSentinelDiagnosticSafetyClass {
    ReadOnlySafe,
    DeterministicReplaySafe,
    ContractBoundedSafe,
    TargetedTestSafe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelSentinelDiagnosticBudgetImpact {
    pub projected_probe_count: u32,
    pub projected_runtime_seconds: u32,
    pub projected_total_runtime_seconds: u32,
    pub projected_scope_escalation_depth: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KernelSentinelDiagnosticRequest {
    pub schema_version: u32,
    pub request_id: String,
    pub incident_id: String,
    pub failure_signature: String,
    pub hypothesis: String,
    pub competing_explanation: String,
    pub probe_class: KernelSentinelDiagnosticProbeClass,
    pub selected_probe: String,
    pub expected_confidence_gain: f64,
    pub safety_class: KernelSentinelDiagnosticSafetyClass,
    pub budget_impact: KernelSentinelDiagnosticBudgetImpact,
}

fn require_nonempty(name: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("missing_{name}"));
    }
    Ok(())
}

pub fn validate_kernel_sentinel_diagnostic_request(
    request: &KernelSentinelDiagnosticRequest,
) -> Result<(), String> {
    if request.schema_version != KERNEL_SENTINEL_DIAGNOSTIC_REQUEST_SCHEMA_VERSION {
        return Err("invalid_schema_version".to_string());
    }
    require_nonempty("request_id", &request.request_id)?;
    require_nonempty("incident_id", &request.incident_id)?;
    require_nonempty("failure_signature", &request.failure_signature)?;
    require_nonempty("hypothesis", &request.hypothesis)?;
    require_nonempty("competing_explanation", &request.competing_explanation)?;
    require_nonempty("selected_probe", &request.selected_probe)?;
    if !(0.0..=1.0).contains(&request.expected_confidence_gain)
        || request.expected_confidence_gain <= 0.0
    {
        return Err("invalid_expected_confidence_gain".to_string());
    }
    if request.budget_impact.projected_probe_count == 0 {
        return Err("invalid_projected_probe_count".to_string());
    }
    if request.budget_impact.projected_runtime_seconds == 0 {
        return Err("invalid_projected_runtime_seconds".to_string());
    }
    if request.budget_impact.projected_total_runtime_seconds == 0 {
        return Err("invalid_projected_total_runtime_seconds".to_string());
    }
    if request.budget_impact.projected_scope_escalation_depth > 2 {
        return Err("invalid_projected_scope_escalation_depth".to_string());
    }
    Ok(())
}

pub fn kernel_sentinel_diagnostic_request_model() -> Value {
    json!({
        "ok": true,
        "type": "kernel_sentinel_diagnostic_request_model",
        "schema_version": KERNEL_SENTINEL_DIAGNOSTIC_REQUEST_SCHEMA_VERSION,
        "policy_ref": KERNEL_SENTINEL_DIAGNOSTIC_POLICY_REF,
        "required_fields": [
            "request_id",
            "incident_id",
            "failure_signature",
            "hypothesis",
            "competing_explanation",
            "probe_class",
            "selected_probe",
            "expected_confidence_gain",
            "safety_class",
            "budget_impact"
        ],
        "probe_classes": [
            "diagnostic_topology_probe",
            "diagnostic_evidence_refresh",
            "diagnostic_replay",
            "diagnostic_contract_probe",
            "diagnostic_test"
        ],
        "safety_classes": [
            "read_only_safe",
            "deterministic_replay_safe",
            "contract_bounded_safe",
            "targeted_test_safe"
        ],
        "budget_contract": {
            "required_fields": [
                "projected_probe_count",
                "projected_runtime_seconds",
                "projected_total_runtime_seconds",
                "projected_scope_escalation_depth"
            ],
            "max_scope_escalation_depth": 2,
            "expected_confidence_gain_range": [0.0, 1.0]
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request() -> KernelSentinelDiagnosticRequest {
        KernelSentinelDiagnosticRequest {
            schema_version: KERNEL_SENTINEL_DIAGNOSTIC_REQUEST_SCHEMA_VERSION,
            request_id: "diag-001".to_string(),
            incident_id: "incident-001".to_string(),
            failure_signature: "gateway_restart_success_without_listener".to_string(),
            hypothesis: "listener truth is stale relative to reported restart success".to_string(),
            competing_explanation: "shell projection alone is stale while gateway truth is healthy"
                .to_string(),
            probe_class: KernelSentinelDiagnosticProbeClass::DiagnosticTopologyProbe,
            selected_probe: "topology://gateway/listener_healthz_readiness".to_string(),
            expected_confidence_gain: 0.35,
            safety_class: KernelSentinelDiagnosticSafetyClass::ReadOnlySafe,
            budget_impact: KernelSentinelDiagnosticBudgetImpact {
                projected_probe_count: 1,
                projected_runtime_seconds: 20,
                projected_total_runtime_seconds: 20,
                projected_scope_escalation_depth: 0,
            },
        }
    }

    #[test]
    fn diagnostic_request_model_exposes_expected_contract() {
        let model = kernel_sentinel_diagnostic_request_model();
        assert_eq!(model["type"], "kernel_sentinel_diagnostic_request_model");
        assert_eq!(model["schema_version"], 1);
        assert_eq!(
            model["policy_ref"],
            "docs/workspace/kernel_sentinel_diagnostic_execution_policy.md"
        );
        assert!(model["probe_classes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "diagnostic_test"));
    }

    #[test]
    fn validate_diagnostic_request_accepts_valid_payload() {
        let request = sample_request();
        assert!(validate_kernel_sentinel_diagnostic_request(&request).is_ok());
    }

    #[test]
    fn validate_diagnostic_request_rejects_invalid_budget_and_confidence() {
        let mut request = sample_request();
        request.expected_confidence_gain = 0.0;
        assert_eq!(
            validate_kernel_sentinel_diagnostic_request(&request).unwrap_err(),
            "invalid_expected_confidence_gain"
        );

        let mut request = sample_request();
        request.budget_impact.projected_scope_escalation_depth = 3;
        assert_eq!(
            validate_kernel_sentinel_diagnostic_request(&request).unwrap_err(),
            "invalid_projected_scope_escalation_depth"
        );
    }
}
