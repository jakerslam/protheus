// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    KernelSentinelDiagnosticProbeClass, KernelSentinelDiagnosticRequest,
    KernelSentinelDiagnosticSafetyClass,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const KERNEL_SENTINEL_DIAGNOSTIC_POLICY_REF: &str =
    "docs/workspace/kernel_sentinel_diagnostic_execution_policy.md";
const MAX_PROBES_PER_INCIDENT: u32 = 3;
const MAX_PROBE_RUNTIME_SECONDS: u32 = 90;
const MAX_TOTAL_RUNTIME_SECONDS: u32 = 240;
const MAX_SCOPE_ESCALATION_DEPTH: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelSentinelDiagnosticAuthorizationStatus {
    Authorized,
    Refused,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelSentinelDiagnosticAuthorizationDecision {
    pub request_id: String,
    pub status: KernelSentinelDiagnosticAuthorizationStatus,
    pub policy_ref: &'static str,
    pub authorization_reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KernelSentinelDiagnosticFailureProbePolicy {
    pub failure_signature: &'static str,
    pub allowed_probe_classes: &'static [&'static str],
    pub allowed_probe_prefixes: &'static [&'static str],
}

const FAILURE_SIGNATURE_PROBE_POLICIES: &[KernelSentinelDiagnosticFailureProbePolicy] = &[
    KernelSentinelDiagnosticFailureProbePolicy {
        failure_signature: "gateway_restart_success_without_listener",
        allowed_probe_classes: &[
            "diagnostic_topology_probe",
            "diagnostic_evidence_refresh",
            "diagnostic_replay",
        ],
        allowed_probe_prefixes: &[
            "topology://",
            "health://",
            "listener://",
            "process://",
            "watchdog://",
            "evidence://",
            "receipt://",
            "replay://",
            "golden://",
        ],
    },
    KernelSentinelDiagnosticFailureProbePolicy {
        failure_signature: "dashboard_healthz_not_ready",
        allowed_probe_classes: &["diagnostic_topology_probe", "diagnostic_evidence_refresh"],
        allowed_probe_prefixes: &[
            "health://",
            "listener://",
            "process://",
            "watchdog://",
            "evidence://",
            "artifact://",
            "report://",
        ],
    },
    KernelSentinelDiagnosticFailureProbePolicy {
        failure_signature: "authority_shape_residue_reemergence",
        allowed_probe_classes: &[
            "diagnostic_contract_probe",
            "diagnostic_replay",
            "diagnostic_evidence_refresh",
        ],
        allowed_probe_prefixes: &[
            "contract://",
            "invariant://",
            "policy://",
            "golden://",
            "replay://",
            "evidence://",
            "report://",
        ],
    },
    KernelSentinelDiagnosticFailureProbePolicy {
        failure_signature: "typed_probe_contract_gap",
        allowed_probe_classes: &[
            "diagnostic_contract_probe",
            "diagnostic_test",
            "diagnostic_replay",
        ],
        allowed_probe_prefixes: &[
            "contract://",
            "invariant://",
            "policy://",
            "test://",
            "regression://",
            "exact://",
            "replay://",
            "golden://",
        ],
    },
];

fn class_prefix_allowed(class: &KernelSentinelDiagnosticProbeClass, probe: &str) -> bool {
    let prefixes: &[&str] = match class {
        KernelSentinelDiagnosticProbeClass::DiagnosticTopologyProbe => &[
            "topology://",
            "health://",
            "listener://",
            "process://",
            "watchdog://",
        ],
        KernelSentinelDiagnosticProbeClass::DiagnosticEvidenceRefresh => {
            &["evidence://", "artifact://", "receipt://", "report://"]
        }
        KernelSentinelDiagnosticProbeClass::DiagnosticReplay => {
            &["replay://", "golden://", "scenario://"]
        }
        KernelSentinelDiagnosticProbeClass::DiagnosticContractProbe => {
            &["contract://", "invariant://", "policy://"]
        }
        KernelSentinelDiagnosticProbeClass::DiagnosticTest => {
            &["test://", "regression://", "exact://"]
        }
    };
    prefixes.iter().any(|prefix| probe.starts_with(prefix))
}

fn safety_matches_class(
    class: &KernelSentinelDiagnosticProbeClass,
    safety: &KernelSentinelDiagnosticSafetyClass,
) -> bool {
    matches!(
        (class, safety),
        (
            KernelSentinelDiagnosticProbeClass::DiagnosticTopologyProbe
                | KernelSentinelDiagnosticProbeClass::DiagnosticEvidenceRefresh,
            KernelSentinelDiagnosticSafetyClass::ReadOnlySafe
        ) | (
            KernelSentinelDiagnosticProbeClass::DiagnosticReplay,
            KernelSentinelDiagnosticSafetyClass::DeterministicReplaySafe
        ) | (
            KernelSentinelDiagnosticProbeClass::DiagnosticContractProbe,
            KernelSentinelDiagnosticSafetyClass::ContractBoundedSafe
        ) | (
            KernelSentinelDiagnosticProbeClass::DiagnosticTest,
            KernelSentinelDiagnosticSafetyClass::TargetedTestSafe
        )
    )
}

pub fn kernel_sentinel_diagnostic_failure_probe_policies() -> Value {
    Value::Array(
        FAILURE_SIGNATURE_PROBE_POLICIES
            .iter()
            .map(|policy| {
                json!({
                    "failure_signature": policy.failure_signature,
                    "allowed_probe_classes": policy.allowed_probe_classes,
                    "allowed_probe_prefixes": policy.allowed_probe_prefixes,
                })
            })
            .collect(),
    )
}

fn probe_class_code(class: &KernelSentinelDiagnosticProbeClass) -> &'static str {
    match class {
        KernelSentinelDiagnosticProbeClass::DiagnosticTopologyProbe => "diagnostic_topology_probe",
        KernelSentinelDiagnosticProbeClass::DiagnosticEvidenceRefresh => {
            "diagnostic_evidence_refresh"
        }
        KernelSentinelDiagnosticProbeClass::DiagnosticReplay => "diagnostic_replay",
        KernelSentinelDiagnosticProbeClass::DiagnosticContractProbe => "diagnostic_contract_probe",
        KernelSentinelDiagnosticProbeClass::DiagnosticTest => "diagnostic_test",
    }
}

fn failure_signature_policy(
    signature: &str,
) -> Option<&'static KernelSentinelDiagnosticFailureProbePolicy> {
    FAILURE_SIGNATURE_PROBE_POLICIES
        .iter()
        .find(|policy| policy.failure_signature == signature)
}

pub fn authorize_kernel_sentinel_diagnostic_request(
    request: &KernelSentinelDiagnosticRequest,
) -> KernelSentinelDiagnosticAuthorizationDecision {
    let refusal = |reason: &str| KernelSentinelDiagnosticAuthorizationDecision {
        request_id: request.request_id.clone(),
        status: KernelSentinelDiagnosticAuthorizationStatus::Refused,
        policy_ref: KERNEL_SENTINEL_DIAGNOSTIC_POLICY_REF,
        authorization_reason: reason.to_string(),
    };

    let Some(policy) = failure_signature_policy(&request.failure_signature) else {
        return refusal("failure_signature_unmapped");
    };

    if request.budget_impact.projected_probe_count > MAX_PROBES_PER_INCIDENT
        || request.budget_impact.projected_runtime_seconds > MAX_PROBE_RUNTIME_SECONDS
        || request.budget_impact.projected_total_runtime_seconds > MAX_TOTAL_RUNTIME_SECONDS
        || request.budget_impact.projected_scope_escalation_depth > MAX_SCOPE_ESCALATION_DEPTH
    {
        return refusal("diagnostic_budget_exceeded");
    }

    if !safety_matches_class(&request.probe_class, &request.safety_class) {
        return refusal("probe_class_safety_class_mismatch");
    }

    if !policy
        .allowed_probe_classes
        .iter()
        .any(|class| *class == probe_class_code(&request.probe_class))
    {
        return refusal("probe_class_exceeds_failure_signature_authority");
    }

    if !class_prefix_allowed(&request.probe_class, &request.selected_probe) {
        return refusal("probe_target_not_allowed_for_class");
    }

    if !policy
        .allowed_probe_prefixes
        .iter()
        .any(|prefix| request.selected_probe.starts_with(prefix))
    {
        return refusal("probe_target_unmapped_for_failure_signature");
    }

    KernelSentinelDiagnosticAuthorizationDecision {
        request_id: request.request_id.clone(),
        status: KernelSentinelDiagnosticAuthorizationStatus::Authorized,
        policy_ref: KERNEL_SENTINEL_DIAGNOSTIC_POLICY_REF,
        authorization_reason: "request_matches_policy_authorized_probe_class_contract".to_string(),
    }
}

pub fn kernel_sentinel_diagnostic_authorization_model() -> Value {
    json!({
        "ok": true,
        "type": "kernel_sentinel_diagnostic_authorization_model",
        "policy_ref": KERNEL_SENTINEL_DIAGNOSTIC_POLICY_REF,
        "statuses": ["authorized", "refused"],
        "class_safety_requirements": {
            "diagnostic_topology_probe": "read_only_safe",
            "diagnostic_evidence_refresh": "read_only_safe",
            "diagnostic_replay": "deterministic_replay_safe",
            "diagnostic_contract_probe": "contract_bounded_safe",
            "diagnostic_test": "targeted_test_safe"
        },
        "class_probe_prefixes": {
            "diagnostic_topology_probe": ["topology://", "health://", "listener://", "process://", "watchdog://"],
            "diagnostic_evidence_refresh": ["evidence://", "artifact://", "receipt://", "report://"],
            "diagnostic_replay": ["replay://", "golden://", "scenario://"],
            "diagnostic_contract_probe": ["contract://", "invariant://", "policy://"],
            "diagnostic_test": ["test://", "regression://", "exact://"]
        },
        "diagnostic_budget_limits": {
            "max_probes_per_incident": MAX_PROBES_PER_INCIDENT,
            "max_probe_runtime_seconds": MAX_PROBE_RUNTIME_SECONDS,
            "max_total_runtime_seconds": MAX_TOTAL_RUNTIME_SECONDS,
            "max_scope_escalation_depth": MAX_SCOPE_ESCALATION_DEPTH
        },
        "failure_signature_probe_policies": kernel_sentinel_diagnostic_failure_probe_policies(),
        "refusal_reasons": [
            "failure_signature_unmapped",
            "diagnostic_budget_exceeded",
            "probe_class_safety_class_mismatch",
            "probe_class_exceeds_failure_signature_authority",
            "probe_target_not_allowed_for_class",
            "probe_target_unmapped_for_failure_signature"
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel_sentinel::{
        KernelSentinelDiagnosticBudgetImpact, KERNEL_SENTINEL_DIAGNOSTIC_REQUEST_SCHEMA_VERSION,
    };

    fn sample_request() -> KernelSentinelDiagnosticRequest {
        KernelSentinelDiagnosticRequest {
            schema_version: KERNEL_SENTINEL_DIAGNOSTIC_REQUEST_SCHEMA_VERSION,
            request_id: "diag-auth-001".to_string(),
            incident_id: "incident-001".to_string(),
            failure_signature: "gateway_restart_success_without_listener".to_string(),
            hypothesis: "listener truth is stale relative to reported restart success".to_string(),
            competing_explanation: "shell telemetry alone is stale".to_string(),
            probe_class: KernelSentinelDiagnosticProbeClass::DiagnosticTopologyProbe,
            selected_probe: "topology://gateway/listener_healthz_readiness".to_string(),
            expected_confidence_gain: 0.30,
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
    fn authorize_diagnostic_request_allows_matching_probe_contract() {
        let decision = authorize_kernel_sentinel_diagnostic_request(&sample_request());
        assert_eq!(
            decision.status,
            KernelSentinelDiagnosticAuthorizationStatus::Authorized
        );
        assert_eq!(
            decision.authorization_reason,
            "request_matches_policy_authorized_probe_class_contract"
        );
    }

    #[test]
    fn authorize_diagnostic_request_refuses_safety_class_mismatch() {
        let mut request = sample_request();
        request.safety_class = KernelSentinelDiagnosticSafetyClass::TargetedTestSafe;
        let decision = authorize_kernel_sentinel_diagnostic_request(&request);
        assert_eq!(
            decision.status,
            KernelSentinelDiagnosticAuthorizationStatus::Refused
        );
        assert_eq!(
            decision.authorization_reason,
            "probe_class_safety_class_mismatch"
        );
    }

    #[test]
    fn authorize_diagnostic_request_refuses_invalid_probe_prefix_for_class() {
        let mut request = sample_request();
        request.selected_probe = "replay://gateway/listener_healthz_readiness".to_string();
        let decision = authorize_kernel_sentinel_diagnostic_request(&request);
        assert_eq!(
            decision.status,
            KernelSentinelDiagnosticAuthorizationStatus::Refused
        );
        assert_eq!(
            decision.authorization_reason,
            "probe_target_not_allowed_for_class"
        );
    }

    #[test]
    fn authorize_diagnostic_request_refuses_unmapped_failure_signature() {
        let mut request = sample_request();
        request.failure_signature = "totally_unknown_failure_family".to_string();
        let decision = authorize_kernel_sentinel_diagnostic_request(&request);
        assert_eq!(
            decision.status,
            KernelSentinelDiagnosticAuthorizationStatus::Refused
        );
        assert_eq!(decision.authorization_reason, "failure_signature_unmapped");
    }

    #[test]
    fn authorize_diagnostic_request_refuses_over_budget_request() {
        let mut request = sample_request();
        request.budget_impact.projected_total_runtime_seconds = 241;
        let decision = authorize_kernel_sentinel_diagnostic_request(&request);
        assert_eq!(
            decision.status,
            KernelSentinelDiagnosticAuthorizationStatus::Refused
        );
        assert_eq!(decision.authorization_reason, "diagnostic_budget_exceeded");
    }

    #[test]
    fn authorize_diagnostic_request_refuses_probe_class_above_signature_authority() {
        let mut request = sample_request();
        request.failure_signature = "dashboard_healthz_not_ready".to_string();
        request.probe_class = KernelSentinelDiagnosticProbeClass::DiagnosticContractProbe;
        request.safety_class = KernelSentinelDiagnosticSafetyClass::ContractBoundedSafe;
        request.selected_probe = "contract://dashboard/healthz".to_string();
        let decision = authorize_kernel_sentinel_diagnostic_request(&request);
        assert_eq!(
            decision.status,
            KernelSentinelDiagnosticAuthorizationStatus::Refused
        );
        assert_eq!(
            decision.authorization_reason,
            "probe_class_exceeds_failure_signature_authority"
        );
    }

    #[test]
    fn diagnostic_failure_signature_policy_map_exposes_recognized_family() {
        let policies = kernel_sentinel_diagnostic_failure_probe_policies();
        let rows = policies
            .as_array()
            .expect("policy export should be an array");
        let gateway_row = rows
            .iter()
            .find(|row| row["failure_signature"] == "gateway_restart_success_without_listener")
            .expect("gateway family should be present");
        assert!(gateway_row["allowed_probe_classes"]
            .as_array()
            .expect("classes should be an array")
            .iter()
            .any(|value| value == "diagnostic_topology_probe"));
        assert!(gateway_row["allowed_probe_prefixes"]
            .as_array()
            .expect("prefixes should be an array")
            .iter()
            .any(|value| value == "listener://"));
    }
}
