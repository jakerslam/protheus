// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::{KernelSentinelFinding, KernelSentinelFindingCategory, KernelSentinelSeverity};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum KernelSentinelFailureLevel {
    #[serde(rename = "L0_local_defect")]
    L0LocalDefect,
    #[serde(rename = "L1_component_regression")]
    L1ComponentRegression,
    #[serde(rename = "L2_boundary_contract_breach")]
    L2BoundaryContractBreach,
    #[serde(rename = "L3_policy_truth_failure")]
    L3PolicyTruthFailure,
    #[serde(rename = "L4_architectural_misalignment")]
    L4ArchitecturalMisalignment,
    #[serde(rename = "L5_self_model_failure")]
    L5SelfModelFailure,
}

pub const KERNEL_SENTINEL_FAILURE_LEVELS: [KernelSentinelFailureLevel; 6] = [
    KernelSentinelFailureLevel::L0LocalDefect,
    KernelSentinelFailureLevel::L1ComponentRegression,
    KernelSentinelFailureLevel::L2BoundaryContractBreach,
    KernelSentinelFailureLevel::L3PolicyTruthFailure,
    KernelSentinelFailureLevel::L4ArchitecturalMisalignment,
    KernelSentinelFailureLevel::L5SelfModelFailure,
];

impl KernelSentinelFailureLevel {
    pub const fn code(self) -> &'static str {
        match self {
            Self::L0LocalDefect => "L0_local_defect",
            Self::L1ComponentRegression => "L1_component_regression",
            Self::L2BoundaryContractBreach => "L2_boundary_contract_breach",
            Self::L3PolicyTruthFailure => "L3_policy_truth_failure",
            Self::L4ArchitecturalMisalignment => "L4_architectural_misalignment",
            Self::L5SelfModelFailure => "L5_self_model_failure",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::L0LocalDefect => "Local defect",
            Self::L1ComponentRegression => "Component regression",
            Self::L2BoundaryContractBreach => "Boundary contract breach",
            Self::L3PolicyTruthFailure => "Policy truth failure",
            Self::L4ArchitecturalMisalignment => "Architectural misalignment",
            Self::L5SelfModelFailure => "Self-model failure",
        }
    }

    pub const fn captures(self) -> &'static str {
        match self {
            Self::L0LocalDefect => {
                "isolated syntax, data, or implementation defect with no evidence of wider system drift"
            }
            Self::L1ComponentRegression => {
                "single component behavior regressed while surrounding contracts still hold"
            }
            Self::L2BoundaryContractBreach => {
                "two or more components disagree across an explicit API, nexus, receipt, or ownership boundary"
            }
            Self::L3PolicyTruthFailure => {
                "declared authority, lifecycle, or source-of-truth policy is contradicted by runtime behavior"
            }
            Self::L4ArchitecturalMisalignment => {
                "system shape makes the failure recur because responsibilities, layers, or runtime topology are wrong"
            }
            Self::L5SelfModelFailure => {
                "the system misunderstood its own purpose, capabilities, constraints, or remediation level"
            }
        }
    }

    pub const fn remediation_level(self) -> &'static str {
        match self {
            Self::L0LocalDefect => "symptom_patch",
            Self::L1ComponentRegression => "component_fix",
            Self::L2BoundaryContractBreach => "boundary_repair",
            Self::L3PolicyTruthFailure => "policy_realignment",
            Self::L4ArchitecturalMisalignment => "architectural_refactor",
            Self::L5SelfModelFailure => "self_model_repair",
        }
    }

    pub const fn priority(self) -> u8 {
        match self {
            Self::L0LocalDefect => 0,
            Self::L1ComponentRegression => 1,
            Self::L2BoundaryContractBreach => 2,
            Self::L3PolicyTruthFailure => 3,
            Self::L4ArchitecturalMisalignment => 4,
            Self::L5SelfModelFailure => 5,
        }
    }
}

pub fn kernel_sentinel_failure_level_taxonomy() -> Value {
    Value::Array(
        KERNEL_SENTINEL_FAILURE_LEVELS
            .iter()
            .map(|level| {
                json!({
                    "code": level.code(),
                    "label": level.label(),
                    "priority": level.priority(),
                    "captures": level.captures(),
                    "remediation_level": level.remediation_level(),
                    "sentinel_use": "classify the highest-level failure frame before selecting remediation"
                })
            })
            .collect(),
    )
}

pub fn kernel_sentinel_failure_level_for_parts(
    category: &str,
    severity: &str,
    fingerprint: &str,
    summary: &str,
    recommended_action: &str,
) -> KernelSentinelFailureLevel {
    let text = format!("{category} {severity} {fingerprint} {summary} {recommended_action}")
        .to_ascii_lowercase();
    if text.contains("self_model")
        || text.contains("self-model")
        || text.contains("rsi")
        || text.contains("system_understanding")
        || text.contains("understand itself")
    {
        return KernelSentinelFailureLevel::L5SelfModelFailure;
    }
    if text.contains("architect")
        || text.contains("wrong layer")
        || text.contains("source-of-truth")
        || text.contains("source of truth")
        || text.contains("mini os")
        || text.contains("subsystem in the shell")
    {
        return KernelSentinelFailureLevel::L4ArchitecturalMisalignment;
    }
    if text.contains("policy")
        || text.contains("authority")
        || text.contains("truth")
        || text.contains("receipt")
        || text.contains("capability")
    {
        return KernelSentinelFailureLevel::L3PolicyTruthFailure;
    }
    match category {
        "receipt_integrity" | "capability_enforcement" | "release_evidence"
        | "security_boundary" | "state_transition" => {
            KernelSentinelFailureLevel::L3PolicyTruthFailure
        }
        "nexus_boundary" | "gateway_isolation" => {
            KernelSentinelFailureLevel::L2BoundaryContractBreach
        }
        "self_maintenance_loop" | "automation_candidate" => {
            KernelSentinelFailureLevel::L5SelfModelFailure
        }
        "runtime_correctness" | "correctness" if severity == "critical" || severity == "high" => {
            KernelSentinelFailureLevel::L2BoundaryContractBreach
        }
        "boundedness" | "queue_backpressure" | "retry_storm" | "performance_regression"
        | "runtime_correctness" | "correctness" => KernelSentinelFailureLevel::L1ComponentRegression,
        _ => KernelSentinelFailureLevel::L0LocalDefect,
    }
}

pub fn kernel_sentinel_failure_level_for_finding(
    finding: &KernelSentinelFinding,
) -> KernelSentinelFailureLevel {
    let text = format!("{} {} {}", finding.fingerprint, finding.summary, finding.recommended_action)
        .to_ascii_lowercase();
    if text.contains("self_model")
        || text.contains("self-model")
        || text.contains("rsi")
        || text.contains("system_understanding")
        || text.contains("understand itself")
    {
        return KernelSentinelFailureLevel::L5SelfModelFailure;
    }
    if text.contains("architect")
        || text.contains("wrong layer")
        || text.contains("source-of-truth")
        || text.contains("source of truth")
        || text.contains("mini os")
        || text.contains("subsystem in the shell")
    {
        return KernelSentinelFailureLevel::L4ArchitecturalMisalignment;
    }
    match finding.category {
        KernelSentinelFindingCategory::ReceiptIntegrity
        | KernelSentinelFindingCategory::CapabilityEnforcement
        | KernelSentinelFindingCategory::ReleaseEvidence
        | KernelSentinelFindingCategory::SecurityBoundary
        | KernelSentinelFindingCategory::StateTransition => {
            KernelSentinelFailureLevel::L3PolicyTruthFailure
        }
        KernelSentinelFindingCategory::NexusBoundary
        | KernelSentinelFindingCategory::GatewayIsolation => {
            KernelSentinelFailureLevel::L2BoundaryContractBreach
        }
        KernelSentinelFindingCategory::SelfMaintenanceLoop
        | KernelSentinelFindingCategory::AutomationCandidate => {
            KernelSentinelFailureLevel::L5SelfModelFailure
        }
        KernelSentinelFindingCategory::RuntimeCorrectness
            if matches!(
                finding.severity,
                KernelSentinelSeverity::Critical | KernelSentinelSeverity::High
            ) =>
        {
            KernelSentinelFailureLevel::L2BoundaryContractBreach
        }
        KernelSentinelFindingCategory::Boundedness
        | KernelSentinelFindingCategory::QueueBackpressure
        | KernelSentinelFindingCategory::RetryStorm
        | KernelSentinelFindingCategory::PerformanceRegression
        | KernelSentinelFindingCategory::RuntimeCorrectness => {
            KernelSentinelFailureLevel::L1ComponentRegression
        }
    }
}

pub const fn kernel_sentinel_root_frame_for_level(
    level: KernelSentinelFailureLevel,
) -> &'static str {
    match level {
        KernelSentinelFailureLevel::L0LocalDefect => "local_implementation_defect",
        KernelSentinelFailureLevel::L1ComponentRegression => "component_runtime_regression",
        KernelSentinelFailureLevel::L2BoundaryContractBreach => "cross_boundary_contract",
        KernelSentinelFailureLevel::L3PolicyTruthFailure => "policy_truth_contradiction",
        KernelSentinelFailureLevel::L4ArchitecturalMisalignment => "architectural_source_of_truth",
        KernelSentinelFailureLevel::L5SelfModelFailure => "system_self_model",
    }
}

pub fn kernel_sentinel_root_frame_for_finding(finding: &KernelSentinelFinding) -> &'static str {
    kernel_sentinel_root_frame_for_level(kernel_sentinel_failure_level_for_finding(finding))
}

pub fn kernel_sentinel_semantic_frame_for_parts(
    category: &str,
    severity: &str,
    fingerprint: &str,
    summary: &str,
    recommended_action: &str,
) -> Value {
    let failure_level = kernel_sentinel_failure_level_for_parts(
        category,
        severity,
        fingerprint,
        summary,
        recommended_action,
    );
    json!({
        "failure_level": failure_level.code(),
        "root_frame": kernel_sentinel_root_frame_for_level(failure_level),
        "remediation_level": failure_level.remediation_level()
    })
}

pub fn kernel_sentinel_semantic_frame_for_finding(finding: &KernelSentinelFinding) -> Value {
    let failure_level = kernel_sentinel_failure_level_for_finding(finding);
    json!({
        "failure_level": failure_level.code(),
        "root_frame": kernel_sentinel_root_frame_for_level(failure_level),
        "remediation_level": failure_level.remediation_level()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_level_taxonomy_is_stable_and_ordered() {
        let codes = KERNEL_SENTINEL_FAILURE_LEVELS
            .iter()
            .map(|level| level.code())
            .collect::<Vec<_>>();
        assert_eq!(
            codes,
            vec![
                "L0_local_defect",
                "L1_component_regression",
                "L2_boundary_contract_breach",
                "L3_policy_truth_failure",
                "L4_architectural_misalignment",
                "L5_self_model_failure",
            ]
        );
        assert_eq!(
            serde_json::to_string(&KernelSentinelFailureLevel::L4ArchitecturalMisalignment)
                .unwrap(),
            "\"L4_architectural_misalignment\""
        );
        assert_eq!(
            KernelSentinelFailureLevel::L5SelfModelFailure.remediation_level(),
            "self_model_repair"
        );
    }
}
