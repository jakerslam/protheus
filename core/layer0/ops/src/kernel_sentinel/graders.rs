// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    KernelSentinelFinding, KernelSentinelFindingCategory, KernelSentinelSeverity,
    KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

fn text(value: &Value, key: &str, fallback: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn bool_detail(record: &Value, key: &str) -> bool {
    value(record, key)
        .map(|raw| {
            raw.as_bool()
                .unwrap_or_else(|| matches!(raw.as_str().unwrap_or("").trim().to_lowercase().as_str(), "1" | "true" | "yes" | "fail" | "failed"))
        })
        .unwrap_or(false)
}

fn value<'a>(record: &'a Value, key: &str) -> Option<&'a Value> {
    record
        .get(key)
        .or_else(|| {
            record
                .get("details")
                .and_then(|details| details.get(key))
        })
        .or_else(|| {
            record
                .get("details")
                .and_then(|details| details.get("details"))
                .and_then(|details| details.get(key))
        })
}

fn str_detail(record: &Value, key: &str, fallback: &str) -> String {
    value(record, key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn numeric_detail(record: &Value, key: &str) -> Option<f64> {
    value(record, key).and_then(|raw| {
        raw.as_f64()
            .or_else(|| raw.as_u64().map(|value| value as f64))
            .or_else(|| raw.as_i64().map(|value| value as f64))
            .or_else(|| raw.as_str().and_then(|text| text.trim().parse::<f64>().ok()))
    })
}

fn regression_detail(record: &Value, key: &str) -> bool {
    bool_detail(record, key)
        || numeric_detail(record, key)
            .map(|value| value > 0.0)
            .unwrap_or(false)
}

fn explicit_failure_status(status: &str) -> bool {
    matches!(
        status.trim().to_lowercase().as_str(),
        "fail" | "failed" | "failure" | "error" | "critical"
    )
}

fn evidence(record: &Value, fallback: &str) -> Vec<String> {
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
        .unwrap_or_else(|| vec![fallback.to_string()])
}

fn subject(record: &Value) -> String {
    text(record, "subject", "unknown_subject")
}

fn replay_finding(record: &Value, reason: &str) -> KernelSentinelFinding {
    let subject = subject(record);
    let mut evidence = evidence(record, &format!("replay://{subject}"));
    evidence.push(format!("replay://{subject};reason={reason}"));
    KernelSentinelFinding {
        schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        id: format!("outcome_replay:{reason}:{subject}"),
        severity: KernelSentinelSeverity::Critical,
        category: KernelSentinelFindingCategory::RuntimeCorrectness,
        fingerprint: format!("outcome_replay:{reason}:{subject}"),
        evidence,
        summary: format!("{subject} failed outcome replay: {reason}"),
        recommended_action: "restore state/receipt replay agreement before accepting the claimed outcome".to_string(),
        status: "open".to_string(),
    }
}

fn replay_findings(records: &[Value]) -> Vec<KernelSentinelFinding> {
    let mut findings = Vec::new();
    for record in records {
        if bool_detail(record, "claimed_done")
            && (!bool_detail(record, "state_matches_claim") || !bool_detail(record, "receipt_present"))
        {
            findings.push(replay_finding(record, "claim_without_state_or_receipt"));
        }
        if bool_detail(record, "expected_blocked") && bool_detail(record, "mutation_applied") {
            findings.push(replay_finding(record, "blocked_mutation_applied"));
        }
        if bool_detail(record, "expected_gateway_quarantined")
            && !bool_detail(record, "gateway_quarantined")
        {
            findings.push(replay_finding(record, "gateway_quarantine_missing"));
        }
        if bool_detail(record, "rollback_expected") && !bool_detail(record, "rollback_receipt_present") {
            findings.push(replay_finding(record, "rollback_receipt_missing"));
        }
    }
    findings
}

fn trend_failure_count(records: &[Value]) -> usize {
    records
        .iter()
        .filter(|record| regression_detail(record, "trend_regression") || regression_detail(record, "boundedness_regression"))
        .count()
}

fn semantic_monitor_count(records: &[Value]) -> usize {
    records
        .iter()
        .filter(|record| {
            str_detail(record, "semantic_monitor_status", "") == "warn"
                || str_detail(record, "semantic_monitor_status", "") == "fail"
        })
        .count()
}

fn semantic_monitor_advisories(records: &[Value]) -> Vec<Value> {
    let mut clusters: BTreeMap<String, (usize, BTreeSet<String>, String)> = BTreeMap::new();
    for record in records {
        let status = str_detail(record, "semantic_monitor_status", "");
        if status != "warn" && status != "fail" {
            continue;
        }
        let subject = subject(record);
        let cluster_key = str_detail(record, "semantic_cluster_key", &subject);
        let summary = str_detail(record, "semantic_summary", &text(record, "summary", "semantic monitor advisory"));
        let entry = clusters
            .entry(cluster_key)
            .or_insert_with(|| (0, BTreeSet::new(), summary.clone()));
        entry.0 += 1;
        entry.2 = summary;
        for ref_id in evidence(record, &format!("semantic://{subject}")) {
            if !ref_id.starts_with("semantic://") && !ref_id.starts_with("control_plane_eval://") {
                entry.1.insert(ref_id);
            }
        }
    }
    clusters
        .into_iter()
        .map(|(cluster_key, (occurrence_count, deterministic_refs, summary))| {
            json!({
                "cluster_key": cluster_key,
                "occurrence_count": occurrence_count,
                "summary": summary,
                "suggested_issue_text": summary,
                "authority": "advisory_non_authoritative",
                "may_block": false,
                "may_waive": false,
                "deterministic_evidence_refs": deterministic_refs.into_iter().collect::<Vec<_>>()
            })
        })
        .collect()
}

fn canary_failure_count(records: &[Value]) -> usize {
    records
        .iter()
        .filter(|record| {
            bool_detail(record, "canary_failed")
                || (text(record, "kind", "") == "sentinel_canary"
                    && explicit_failure_status(&str_detail(record, "canary_status", "pass")))
        })
        .count()
}

fn canary_case_rows(records: &[Value]) -> Vec<Value> {
    records
        .iter()
        .filter(|record| bool_detail(record, "canary_failed") || text(record, "kind", "") == "sentinel_canary")
        .map(|record| {
            let subject = subject(record);
            let case = str_detail(record, "canary_case", &subject);
            let raw_status = str_detail(record, "canary_status", "pass");
            let status = if bool_detail(record, "canary_failed") {
                "fail".to_string()
            } else if explicit_failure_status(&raw_status) {
                raw_status
            } else {
                "pass".to_string()
            };
            json!({
                "case": case,
                "subject": subject,
                "status": status,
                "evidence": evidence(record, "canary://sentinel")
            })
        })
        .collect()
}

fn invariant_failure_count(findings: &[KernelSentinelFinding]) -> usize {
    findings
        .iter()
        .filter(|finding| {
            finding.status == "open"
                && finding.severity == KernelSentinelSeverity::Critical
                && matches!(
                    finding.category,
                    KernelSentinelFindingCategory::ReceiptIntegrity
                        | KernelSentinelFindingCategory::CapabilityEnforcement
                        | KernelSentinelFindingCategory::StateTransition
                        | KernelSentinelFindingCategory::SecurityBoundary
                        | KernelSentinelFindingCategory::GatewayIsolation
                        | KernelSentinelFindingCategory::ReleaseEvidence
                )
        })
        .count()
}

fn grader(id: &str, authority: &str, failures: usize, blocks: bool, notes: Value) -> Value {
    json!({
        "id": id,
        "authority": authority,
        "status": if failures == 0 { "pass" } else { "fail" },
        "failure_count": failures,
        "blocks_release": blocks && failures > 0,
        "notes": notes
    })
}

pub fn build_grader_stack(
    findings: &[KernelSentinelFinding],
    records: &[Value],
) -> (Value, Vec<KernelSentinelFinding>) {
    let replay_findings = replay_findings(records);
    let invariant_failures = invariant_failure_count(findings);
    let replay_failures = replay_findings.len();
    let trend_failures = trend_failure_count(records);
    let semantic_warnings = semantic_monitor_count(records);
    let semantic_advisories = semantic_monitor_advisories(records);
    let canary_failures = canary_failure_count(records);
    let canary_cases = canary_case_rows(records);
    let graders = vec![
        grader("invariant", "deterministic_blocking", invariant_failures, true, json!({"source": "kernel_findings"})),
        grader("replay", "policy_blocking", replay_failures, true, json!({"source": "state_receipt_replay"})),
        grader("trend", "policy_blocking", trend_failures, true, json!({"source": "boundedness_trend"})),
        grader(
            "semantic_monitor",
            "advisory_non_authoritative",
            semantic_warnings,
            false,
            json!({
                "source": "semantic_monitor",
                "advisory_cluster_count": semantic_advisories.len(),
                "advisory_clusters": semantic_advisories
            }),
        ),
        grader(
            "canary",
            "sentinel_robustness_blocking",
            canary_failures,
            true,
            json!({
                "source": "sentinel_canary",
                "bridge_presence_policy": "observed_without_explicit_failure_signal_is_non_failure",
                "case_count": canary_cases.len(),
                "cases": canary_cases
            }),
        ),
    ];
    let blocking_failure_count = graders
        .iter()
        .filter(|row| row["blocks_release"].as_bool().unwrap_or(false))
        .count();
    (
        json!({
            "ok": blocking_failure_count == 0,
            "type": "kernel_sentinel_grader_stack",
            "combined_rule": "fail_if_any_blocking_grader_fails",
            "blocking_failure_count": blocking_failure_count,
            "grader_count": graders.len(),
            "graders": graders
        }),
        replay_findings,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_grader_fails_done_claim_without_receipt() {
        let records = vec![json!({
            "subject": "mutation-1",
            "evidence": ["state://mutation-1"],
            "details": {
                "claimed_done": true,
                "state_matches_claim": true,
                "receipt_present": false
            }
        })];
        let (report, findings) = build_grader_stack(&[], &records);
        assert_eq!(findings[0].fingerprint, "outcome_replay:claim_without_state_or_receipt:mutation-1");
        assert_eq!(report["graders"][1]["id"], "replay");
        assert_eq!(report["graders"][1]["blocks_release"], true);
    }

    #[test]
    fn semantic_monitor_is_advisory_only() {
        let records = vec![json!({
            "subject": "summary-1",
            "evidence": ["receipt://summary-1"],
            "details": {
                "semantic_monitor_status": "fail",
                "semantic_cluster_key": "cluster-a",
                "semantic_summary": "semantic monitor sees a likely duplicate failure"
            }
        })];
        let (report, findings) = build_grader_stack(&[], &records);
        assert!(findings.is_empty());
        assert_eq!(report["graders"][3]["id"], "semantic_monitor");
        assert_eq!(report["graders"][3]["blocks_release"], false);
        assert_eq!(report["graders"][3]["notes"]["advisory_clusters"][0]["may_waive"], false);
        assert_eq!(
            report["graders"][3]["notes"]["advisory_clusters"][0]["deterministic_evidence_refs"][0],
            Value::from("receipt://summary-1")
        );
    }

    #[test]
    fn canary_grader_blocks_on_sentinel_robustness_failure() {
        let records = vec![json!({
            "subject": "forged-receipt-canary",
            "kind": "sentinel_canary",
            "details": {
                "canary_case": "forged_receipt",
                "canary_failed": true
            }
        })];
        let (report, _findings) = build_grader_stack(&[], &records);
        assert_eq!(report["graders"][4]["id"], "canary");
        assert_eq!(report["graders"][4]["blocks_release"], true);
        assert_eq!(report["graders"][4]["notes"]["cases"][0]["case"], Value::from("forged_receipt"));
    }

    #[test]
    fn passing_canary_case_is_reported_without_blocking() {
        let records = vec![json!({
            "subject": "reordered-trace-canary",
            "kind": "sentinel_canary",
            "details": {
                "canary_case": "reordered_trace",
                "canary_status": "pass"
            }
        })];
        let (report, _findings) = build_grader_stack(&[], &records);
        assert_eq!(report["graders"][4]["failure_count"], Value::from(0));
        assert_eq!(report["graders"][4]["blocks_release"], false);
        assert_eq!(report["graders"][4]["notes"]["cases"][0]["case"], Value::from("reordered_trace"));
    }

    #[test]
    fn observed_canary_bridge_presence_does_not_fail_grader() {
        let records = vec![json!({
            "subject": "bridge-present-canary",
            "kind": "sentinel_canary",
            "details": {
                "canary_case": "bridge_presence",
                "canary_status": "observed"
            }
        })];
        let (report, findings) = build_grader_stack(&[], &records);
        assert!(findings.is_empty());
        assert_eq!(report["graders"][4]["failure_count"], Value::from(0));
        assert_eq!(report["graders"][4]["blocks_release"], false);
        assert_eq!(
            report["graders"][4]["notes"]["bridge_presence_policy"],
            "observed_without_explicit_failure_signal_is_non_failure"
        );
        assert_eq!(report["graders"][4]["notes"]["cases"][0]["status"], "pass");
    }
}
