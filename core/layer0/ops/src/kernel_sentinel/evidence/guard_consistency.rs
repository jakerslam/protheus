// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use crate::kernel_sentinel::{KernelSentinelFinding, KernelSentinelSeverity};
use serde_json::{json, Value};
use std::collections::BTreeSet;

fn record_bool(record: &Value, key: &str) -> Option<bool> {
    record
        .get(key)
        .and_then(Value::as_bool)
        .or_else(|| record.get("details").and_then(|details| details.get(key)).and_then(Value::as_bool))
}

fn record_status(record: &Value) -> String {
    record
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
}

fn record_fingerprint(record: &Value) -> String {
    record
        .get("fingerprint")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string()
}

fn record_evidence(record: &Value) -> Vec<String> {
    record
        .get("evidence")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn record_claims_pass(record: &Value) -> bool {
    record_bool(record, "pass") == Some(true)
        || record.get("ok").and_then(Value::as_bool) == Some(true)
        || matches!(record_status(record).as_str(), "pass" | "passed" | "ok" | "healthy")
}

fn record_claims_failure(record: &Value) -> bool {
    record_bool(record, "pass") == Some(false)
        || record.get("ok").and_then(Value::as_bool) == Some(false)
        || matches!(
            record_status(record).as_str(),
            "fail" | "failed" | "blocked" | "invalid" | "critical" | "error"
        )
}

fn record_is_authoritative(record: &Value) -> bool {
    !record
        .get("advisory")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn records_overlap(left: &Value, right: &Value) -> bool {
    let left_fingerprint = record_fingerprint(left);
    let right_fingerprint = record_fingerprint(right);
    let left_evidence: BTreeSet<String> = record_evidence(left).into_iter().collect();
    !left_fingerprint.is_empty()
        && left_fingerprint == right_fingerprint
        || record_evidence(right)
            .iter()
            .any(|ref_id| left_evidence.contains(ref_id))
}

fn record_label(record: &Value) -> Value {
    json!({
        "source": record.get("source").cloned().unwrap_or(Value::Null),
        "collector_family": record.get("collector_family").cloned().unwrap_or(Value::Null),
        "record_id": record.get("id").cloned().unwrap_or(Value::Null),
        "fingerprint": record.get("fingerprint").cloned().unwrap_or(Value::Null),
        "ok": record.get("ok").cloned().unwrap_or(Value::Null),
        "pass": record_bool(record, "pass"),
        "status": record.get("status").cloned().unwrap_or(Value::Null),
        "evidence": record.get("evidence").cloned().unwrap_or(Value::Array(vec![]))
    })
}

fn build_cross_artifact_contradictions(normalized_records: &[Value]) -> Vec<Value> {
    let authoritative = normalized_records
        .iter()
        .filter(|record| record_is_authoritative(record))
        .collect::<Vec<_>>();
    let mut contradictions = Vec::new();
    for (left_index, left) in authoritative.iter().enumerate() {
        for right in authoritative.iter().skip(left_index + 1) {
            if !records_overlap(left, right) {
                continue;
            }
            let left_pass = record_claims_pass(left);
            let right_pass = record_claims_pass(right);
            let left_fail = record_claims_failure(left);
            let right_fail = record_claims_failure(right);
            if left_pass && right_fail {
                contradictions.push(json!({"pass_record": record_label(left), "failed_record": record_label(right)}));
            } else if right_pass && left_fail {
                contradictions.push(json!({"pass_record": record_label(right), "failed_record": record_label(left)}));
            }
        }
    }
    contradictions
}

fn overlaps(record: &Value, finding: &KernelSentinelFinding) -> bool {
    let finding_evidence: BTreeSet<&str> = finding.evidence.iter().map(String::as_str).collect();
    let record_evidence = record_evidence(record);
    (!record_fingerprint(record).is_empty() && record_fingerprint(record) == finding.fingerprint)
        || record_evidence
            .iter()
            .any(|ref_id| finding_evidence.contains(ref_id.as_str()))
}

fn corroborating_evidence_exists(record: &Value, finding: &KernelSentinelFinding) -> bool {
    let record_evidence: BTreeSet<String> = record_evidence(record).into_iter().collect();
    finding
        .evidence
        .iter()
        .any(|ref_id| !record_evidence.contains(ref_id))
}

pub(super) fn cap_uncorroborated_critical_findings_against_authoritative_pass(
    normalized_records: &[Value],
    findings: &mut [KernelSentinelFinding],
) {
    for finding in findings.iter_mut() {
        if finding.severity != KernelSentinelSeverity::Critical {
            continue;
        }
        let contradicted_by_authoritative_pass = normalized_records.iter().any(|record| {
            record_is_authoritative(record)
                && record_claims_pass(record)
                && overlaps(record, finding)
                && !corroborating_evidence_exists(record, finding)
        });
        if contradicted_by_authoritative_pass {
            finding.severity = KernelSentinelSeverity::High;
        }
    }
}

pub(super) fn build_guard_consistency_report(
    normalized_records: &[Value],
    findings: &[KernelSentinelFinding],
) -> Value {
    let cross_artifact_contradictions = build_cross_artifact_contradictions(normalized_records);
    let contradictions = normalized_records
        .iter()
        .filter(|record| record_is_authoritative(record) && record_claims_pass(record))
        .filter_map(|record| {
            let matching_findings = findings
                .iter()
                .filter(|finding| overlaps(record, finding))
                .map(|finding| json!({
                    "finding_id": finding.id,
                    "fingerprint": finding.fingerprint,
                    "severity": finding.severity,
                    "category": finding.category,
                    "evidence": finding.evidence
                }))
                .collect::<Vec<_>>();
            if matching_findings.is_empty() {
                None
            } else {
                Some(json!({
                    "source": record.get("source").cloned().unwrap_or(Value::Null),
                    "collector_family": record.get("collector_family").cloned().unwrap_or(Value::Null),
                    "record_id": record.get("id").cloned().unwrap_or(Value::Null),
                    "fingerprint": record.get("fingerprint").cloned().unwrap_or(Value::Null),
                    "pass": record_bool(record, "pass"),
                    "ok": record.get("ok").cloned().unwrap_or(Value::Null),
                    "status": record.get("status").cloned().unwrap_or(Value::Null),
                    "evidence": record.get("evidence").cloned().unwrap_or(Value::Array(vec![])),
                    "corroborated": matching_findings.iter().any(|finding| {
                        finding
                            .get("evidence")
                            .and_then(Value::as_array)
                            .map(|rows| rows.iter().filter_map(Value::as_str).any(|ref_id| {
                                !record_evidence(record).iter().any(|record_ref| record_ref == ref_id)
                            }))
                            .unwrap_or(false)
                    }),
                    "matching_findings": matching_findings
                }))
            }
        })
        .collect::<Vec<_>>();
    json!({
        "ok": contradictions.is_empty() && cross_artifact_contradictions.is_empty(),
        "checked_count": normalized_records
            .iter()
            .filter(|record| record_is_authoritative(record) && record_claims_pass(record))
            .count(),
        "contradiction_count": contradictions.len(),
        "cross_artifact_contradiction_count": cross_artifact_contradictions.len(),
        "constraints": {
            "authoritative_upstream_pass_requires_no_matching_sentinel_finding": true,
            "authoritative_artifacts_must_not_disagree_on_shared_truth": true
        },
        "contradictions": contradictions,
        "cross_artifact_contradictions": cross_artifact_contradictions
    })
}
