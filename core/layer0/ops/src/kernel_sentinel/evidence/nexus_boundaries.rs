// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    KernelSentinelFinding, KernelSentinelFindingCategory, KernelSentinelSeverity,
    KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy)]
struct BoundaryRule {
    rule: &'static str,
    category: KernelSentinelFindingCategory,
    severity: KernelSentinelSeverity,
    recommended_action: &'static str,
}

const CHECKED_BOUNDARY_RULES: [&str; 7] = [
    "non_nexus_direct_authority_path",
    "direct_authority_path",
    "nexus_bypass",
    "shell_truth_leak",
    "gateway_scheduler_admission_touch",
    "orchestration_policy_ownership",
    "orchestration_receipt_authority",
];

fn value_str<'a>(value: &'a Value, key: &str) -> &'a str {
    value
        .get(key)
        .or_else(|| value.get("details").and_then(|details| details.get(key)))
        .and_then(Value::as_str)
        .unwrap_or("")
}

fn row_contains_token(value: &Value, token: &str) -> bool {
    !token.trim().is_empty()
        && ([
            "id",
            "subject",
            "kind",
            "fingerprint",
            "boundary_rule",
            "violation",
            "violation_kind",
            "owner_layer",
            "target_layer",
        ]
        .iter()
        .any(|key| value_str(value, key).contains(token))
            || value
                .get("evidence")
                .and_then(Value::as_array)
                .map(|rows| rows.iter().any(|row| row.as_str().unwrap_or("").contains(token)))
                .unwrap_or(false))
}

fn evidence_refs(record: &Value) -> Vec<String> {
    record
        .get("evidence")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| vec![format!("evidence://{}", value_str(record, "id"))])
}

fn clean_token(raw: &str, fallback: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn boundary_rule(record: &Value) -> Option<BoundaryRule> {
    let explicit_rule = value_str(record, "boundary_rule");
    let violation = value_str(record, "violation");
    let violation_kind = value_str(record, "violation_kind");
    let kind = value_str(record, "kind");
    let category = value_str(record, "category");

    for raw_rule in [explicit_rule, violation, violation_kind, kind, category] {
        match raw_rule {
            "non_nexus_direct_authority_path" => {
                return Some(BoundaryRule {
                    rule: "non_nexus_direct_authority_path",
                    category: KernelSentinelFindingCategory::NexusBoundary,
                    severity: KernelSentinelSeverity::Critical,
                    recommended_action:
                        "route authority-bearing calls through the required nexus boundary",
                });
            }
            "direct_authority_path" | "direct_authority_bypass" => {
                return Some(BoundaryRule {
                    rule: "direct_authority_path",
                    category: KernelSentinelFindingCategory::NexusBoundary,
                    severity: KernelSentinelSeverity::Critical,
                    recommended_action:
                        "replace direct authority coupling with the canonical nexus path",
                });
            }
            "nexus_bypass" | "non_nexus_bypass" => {
                return Some(BoundaryRule {
                    rule: "nexus_bypass",
                    category: KernelSentinelFindingCategory::NexusBoundary,
                    severity: KernelSentinelSeverity::Critical,
                    recommended_action:
                        "remove the bypass and restore nexus-mediated execution",
                });
            }
            "shell_truth_leak" | "client_truth_leak" => {
                return Some(BoundaryRule {
                    rule: "shell_truth_leak",
                    category: KernelSentinelFindingCategory::SecurityBoundary,
                    severity: KernelSentinelSeverity::Critical,
                    recommended_action:
                        "move truth inference back to Kernel authority and expose only backend state",
                });
            }
            "gateway_scheduler_admission_touch" | "gateway_admission_touch"
            | "gateway_scheduler_touch" => {
                return Some(BoundaryRule {
                    rule: "gateway_scheduler_admission_touch",
                    category: KernelSentinelFindingCategory::SecurityBoundary,
                    severity: KernelSentinelSeverity::Critical,
                    recommended_action:
                        "remove scheduler/admission authority from the gateway boundary",
                });
            }
            "orchestration_policy_ownership" | "orchestration_policy_authority" => {
                return Some(BoundaryRule {
                    rule: "orchestration_policy_ownership",
                    category: KernelSentinelFindingCategory::SecurityBoundary,
                    severity: KernelSentinelSeverity::Critical,
                    recommended_action:
                        "move hard policy ownership back to Kernel authority",
                });
            }
            "orchestration_receipt_authority" | "orchestration_receipt_ownership" => {
                return Some(BoundaryRule {
                    rule: "orchestration_receipt_authority",
                    category: KernelSentinelFindingCategory::SecurityBoundary,
                    severity: KernelSentinelSeverity::Critical,
                    recommended_action:
                        "move receipt authority back to Kernel authority",
                });
            }
            "boundary_violation" | "architecture_boundary_violation" | "nexus_boundary" => {
                return Some(BoundaryRule {
                    rule: "boundary_violation",
                    category: KernelSentinelFindingCategory::NexusBoundary,
                    severity: KernelSentinelSeverity::High,
                    recommended_action:
                        "classify the boundary violation and route it through the owning nexus",
                });
            }
            _ => {}
        }
    }

    for rule in CHECKED_BOUNDARY_RULES {
        if row_contains_token(record, rule) {
            let mut probe = record.clone();
            probe["boundary_rule"] = Value::String(rule.to_string());
            return boundary_rule(&probe);
        }
    }
    None
}

fn boundary_finding(record: &Value, rule: BoundaryRule) -> KernelSentinelFinding {
    let action_id = value_str(record, "id");
    let subject = clean_token(value_str(record, "subject"), "unknown_boundary_subject");
    let owner_layer = clean_token(value_str(record, "owner_layer"), "unknown_layer");
    KernelSentinelFinding {
        schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        id: format!("nexus_boundary:{action_id}"),
        severity: rule.severity,
        category: rule.category,
        fingerprint: format!("nexus_boundary:{}:{}:{}", rule.rule, owner_layer, subject),
        evidence: evidence_refs(record),
        summary: format!("{subject} violates {} across {owner_layer}", rule.rule),
        recommended_action: rule.recommended_action.to_string(),
        status: "open".to_string(),
    }
}

pub(super) fn build_nexus_boundary_report(
    records: &[Value],
) -> (Value, Vec<KernelSentinelFinding>) {
    let mut findings = Vec::new();
    let mut checked_violation_count = 0usize;
    for record in records {
        let Some(rule) = boundary_rule(record) else {
            continue;
        };
        checked_violation_count += 1;
        findings.push(boundary_finding(record, rule));
    }
    let report = json!({
        "ok": findings.is_empty(),
        "checked_boundary_rules": CHECKED_BOUNDARY_RULES,
        "checked_violation_count": checked_violation_count,
        "finding_count": findings.len(),
        "findings": findings
    });
    (report, findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_truth_leak_opens_critical_nexus_finding() {
        let records = vec![json!({
            "source": "runtime_observation",
            "id": "boundary-1",
            "subject": "client/runtime/ui/health.ts",
            "kind": "shell_truth_leak",
            "owner_layer": "shell",
            "evidence": ["audit://shell-truth-leak/client-runtime-ui-health"]
        })];
        let (report, findings) = build_nexus_boundary_report(&records);
        assert_eq!(report["finding_count"], Value::from(1));
        assert_eq!(findings[0].severity, KernelSentinelSeverity::Critical);
        assert_eq!(
            findings[0].fingerprint,
            "nexus_boundary:shell_truth_leak:shell:client/runtime/ui/health.ts"
        );
    }

    #[test]
    fn gateway_scheduler_admission_touch_opens_critical_security_finding() {
        let records = vec![json!({
            "source": "gateway_health",
            "id": "boundary-2",
            "subject": "adapters/forgecode/gateway.rs",
            "boundary_rule": "gateway_scheduler_admission_touch",
            "owner_layer": "gateway",
            "evidence": ["audit://gateway-scheduler-touch/forgecode"]
        })];
        let (report, findings) = build_nexus_boundary_report(&records);
        assert_eq!(report["finding_count"], Value::from(1));
        assert_eq!(
            findings[0].category,
            KernelSentinelFindingCategory::SecurityBoundary
        );
        assert_eq!(findings[0].severity, KernelSentinelSeverity::Critical);
    }

    #[test]
    fn unrelated_records_are_ignored() {
        let records = vec![json!({
            "source": "runtime_observation",
            "id": "safe-1",
            "subject": "kernel/safe-path",
            "kind": "state_transition",
            "owner_layer": "kernel"
        })];
        let (report, findings) = build_nexus_boundary_report(&records);
        assert_eq!(report["ok"], Value::Bool(true));
        assert!(findings.is_empty());
    }
}
