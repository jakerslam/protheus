// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    KernelSentinelFinding, KernelSentinelFindingCategory, KernelSentinelSeverity,
    KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy)]
struct ReceiptRequirement {
    action_kind: &'static str,
    expected_receipt_type: &'static str,
}

fn receipt_requirement(kind: &str) -> Option<ReceiptRequirement> {
    match kind {
        "state_mutation" | "state_mutation_committed" | "state_write" => Some(ReceiptRequirement {
            action_kind: "state_mutation",
            expected_receipt_type: "state_mutation_receipt",
        }),
        "tool_execution" | "tool_call" | "tool_result" => Some(ReceiptRequirement {
            action_kind: "tool_execution",
            expected_receipt_type: "tool_execution_receipt",
        }),
        "rollback" | "rollback_applied" => Some(ReceiptRequirement {
            action_kind: "rollback",
            expected_receipt_type: "rollback_receipt",
        }),
        "release_decision" | "release_verdict" => Some(ReceiptRequirement {
            action_kind: "release_decision",
            expected_receipt_type: "release_decision_receipt",
        }),
        "gateway_quarantine" | "gateway_quarantine_action" => Some(ReceiptRequirement {
            action_kind: "gateway_quarantine",
            expected_receipt_type: "gateway_quarantine_receipt",
        }),
        _ => None,
    }
}

fn value_str<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

fn row_contains_token(value: &Value, token: &str) -> bool {
    !token.trim().is_empty()
        && (["id", "subject", "kind", "fingerprint"]
            .iter()
            .any(|key| value_str(value, key).contains(token))
            || value
                .get("evidence")
                .and_then(Value::as_array)
                .map(|rows| rows.iter().any(|row| row.as_str().unwrap_or("").contains(token)))
                .unwrap_or(false))
}

fn receipt_matches_action(receipt: &Value, action: &Value, requirement: ReceiptRequirement) -> bool {
    if value_str(receipt, "source") != "kernel_receipt" {
        return false;
    }
    let receipt_kind = value_str(receipt, "kind");
    let type_matches = receipt_kind == requirement.expected_receipt_type
        || receipt_kind == "receipt"
        || receipt_kind.ends_with("_receipt");
    if !type_matches {
        return false;
    }
    row_contains_token(receipt, value_str(action, "id"))
        || row_contains_token(receipt, value_str(action, "subject"))
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

pub(super) fn receipt_completeness_findings(records: &[Value]) -> Vec<KernelSentinelFinding> {
    let receipts = records
        .iter()
        .filter(|record| value_str(record, "source") == "kernel_receipt")
        .collect::<Vec<_>>();
    records
        .iter()
        .filter(|record| value_str(record, "source") != "kernel_receipt")
        .filter_map(|record| receipt_requirement(value_str(record, "kind")).map(|req| (record, req)))
        .filter(|(record, req)| {
            !receipts
                .iter()
                .any(|receipt| receipt_matches_action(receipt, record, *req))
        })
        .map(|(record, req)| {
            let action_id = value_str(record, "id");
            let subject = value_str(record, "subject");
            KernelSentinelFinding {
                schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
                id: format!("missing_receipt:{action_id}"),
                severity: KernelSentinelSeverity::Critical,
                category: KernelSentinelFindingCategory::ReceiptIntegrity,
                fingerprint: format!(
                    "receipt_completeness:{}:{}:{}",
                    req.action_kind, subject, req.expected_receipt_type
                ),
                evidence: evidence_refs(record),
                summary: format!(
                    "{subject} performed {} without matching {}",
                    req.action_kind, req.expected_receipt_type
                ),
                recommended_action: format!(
                    "restore {} emission before accepting {} as runtime truth",
                    req.expected_receipt_type, req.action_kind
                ),
                status: "open".to_string(),
            }
        })
        .collect()
}

pub(super) fn build_receipt_completeness_report(
    records: &[Value],
) -> (Value, Vec<KernelSentinelFinding>) {
    let findings = receipt_completeness_findings(records);
    let report = json!({
        "ok": findings.is_empty(),
        "checked_action_kinds": [
            "state_mutation",
            "tool_execution",
            "rollback",
            "release_decision",
            "gateway_quarantine"
        ],
        "missing_receipt_count": findings.len(),
        "findings": findings
    });
    (report, findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_required_action_receipt_opens_critical_finding() {
        let records = vec![json!({
            "source": "runtime_observation",
            "id": "action-1",
            "subject": "session-1",
            "kind": "state_mutation",
            "evidence": ["trace://session-1/action-1"]
        })];
        let (report, findings) = build_receipt_completeness_report(&records);
        assert_eq!(report["missing_receipt_count"], Value::from(1));
        assert_eq!(findings[0].severity, KernelSentinelSeverity::Critical);
        assert_eq!(
            findings[0].fingerprint,
            "receipt_completeness:state_mutation:session-1:state_mutation_receipt"
        );
    }

    #[test]
    fn matching_kernel_receipt_satisfies_action_receipt_requirement() {
        let records = vec![
            json!({
                "source": "runtime_observation",
                "id": "action-2",
                "subject": "tool-run-2",
                "kind": "tool_execution",
                "evidence": ["trace://tool-run-2/action-2"]
            }),
            json!({
                "source": "kernel_receipt",
                "id": "receipt-2",
                "subject": "tool-run-2",
                "kind": "tool_execution_receipt",
                "evidence": ["receipt://tool-run-2/action-2"]
            }),
        ];
        let (report, findings) = build_receipt_completeness_report(&records);
        assert!(findings.is_empty());
        assert_eq!(report["missing_receipt_count"], Value::from(0));
    }
}
