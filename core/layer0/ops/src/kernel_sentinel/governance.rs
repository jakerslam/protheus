// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::graders::build_grader_stack;
use super::rsi_handoff::build_rsi_handoff_report;
use super::{
    KernelSentinelFinding, KernelSentinelFindingCategory, KernelSentinelSeverity,
    KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
};
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_FRESHNESS_WINDOW_SECONDS: u64 = 3600;

fn option_u64(args: &[String], name: &str, fallback: u64) -> u64 {
    let prefix = format!("{name}=");
    args.iter()
        .find_map(|arg| arg.strip_prefix(&prefix).and_then(|raw| raw.parse::<u64>().ok()))
        .unwrap_or(fallback)
}

fn value_u64(value: &Value, key: &str) -> Option<u64> {
    field(value, key).and_then(|raw| {
        raw.as_u64()
            .or_else(|| raw.as_i64().and_then(|number| u64::try_from(number).ok()))
            .or_else(|| raw.as_str().and_then(|text| text.trim().parse::<u64>().ok()))
    })
}

fn value_bool(value: &Value, key: &str) -> bool {
    field(value, key)
        .map(|raw| {
            raw.as_bool()
                .unwrap_or_else(|| matches!(raw.as_str().unwrap_or("").trim().to_lowercase().as_str(), "1" | "true" | "yes" | "fail" | "failed"))
        })
        .unwrap_or(false)
}

fn field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value
        .get(key)
        .or_else(|| value.get("details").and_then(|details| details.get(key)))
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn text(value: &Value, key: &str, fallback: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .unwrap_or(fallback)
        .to_string()
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

fn finding(
    id: String,
    category: KernelSentinelFindingCategory,
    fingerprint: String,
    evidence: Vec<String>,
    summary: String,
    recommended_action: &str,
) -> KernelSentinelFinding {
    KernelSentinelFinding {
        schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        id,
        severity: KernelSentinelSeverity::Critical,
        category,
        fingerprint,
        evidence,
        summary,
        recommended_action: recommended_action.to_string(),
        status: "open".to_string(),
    }
}

fn normalized_records(evidence_report: &Value) -> Vec<Value> {
    evidence_report
        .get("normalized_records")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn record_subject(record: &Value) -> String {
    text(record, "subject", "unknown_subject")
}

fn record_kind(record: &Value) -> String {
    text(record, "kind", "evidence_observation")
}

fn invariant_from_record(record: &Value) -> Option<(String, KernelSentinelFinding)> {
    let details = record.get("details").unwrap_or(&Value::Null);
    let subject = record_subject(record);
    let kind = record_kind(record);
    let source = text(record, "source", "unknown_source");
    let base_evidence = evidence(record, &format!("{source}://{subject}/{kind}"));
    if value_bool(details, "forged_receipt") || value_bool(details, "receipt_forgery") {
        return Some((
            "receipt_forgery".to_string(),
            finding(
                format!("hard_fail:receipt_forgery:{subject}"),
                KernelSentinelFindingCategory::ReceiptIntegrity,
                format!("hard_fail:receipt_forgery:{subject}"),
                base_evidence,
                format!("{subject} reported forged receipt evidence"),
                "reject forged receipts and restore receipt verification before release",
            ),
        ));
    }
    if value_bool(details, "capability_bypass")
        || value_bool(details, "payload_shortcut")
        || value_bool(details, "transport_available")
    {
        return Some((
            "capability_bypass".to_string(),
            finding(
                format!("hard_fail:capability_bypass:{subject}"),
                KernelSentinelFindingCategory::CapabilityEnforcement,
                format!("hard_fail:capability_bypass:{subject}:{kind}"),
                base_evidence,
                format!("{subject} used a non-authoritative capability shortcut"),
                "remove raw-payload shortcuts and require Kernel capability grants",
            ),
        ));
    }
    if value_bool(details, "expired_waiver") || value_bool(details, "expired_exemption") {
        return Some((
            "expired_critical_exemption".to_string(),
            finding(
                format!("hard_fail:expired_exemption:{subject}"),
                KernelSentinelFindingCategory::SecurityBoundary,
                format!("hard_fail:expired_critical_exemption:{subject}"),
                base_evidence,
                format!("{subject} relied on an expired critical exemption"),
                "block the operation until a fresh human-reviewed waiver exists",
            ),
        ));
    }
    if value_u64(details, "required_missing").unwrap_or(0) > 0
        || value_u64(details, "missing_required_artifact_count").unwrap_or(0) > 0
    {
        return Some((
            "missing_proof_pack_required_artifact".to_string(),
            finding(
                format!("hard_fail:proof_pack_missing:{subject}"),
                KernelSentinelFindingCategory::ReleaseEvidence,
                format!("hard_fail:missing_proof_pack_required_artifact:{subject}"),
                base_evidence,
                format!("{subject} is missing required release proof artifacts"),
                "regenerate the proof pack and require required_missing=0",
            ),
        ));
    }
    if value_bool(details, "unsafe_gateway_mutation") {
        return Some((
            "unsafe_gateway_mutation".to_string(),
            finding(
                format!("hard_fail:unsafe_gateway_mutation:{subject}"),
                KernelSentinelFindingCategory::GatewayIsolation,
                format!("hard_fail:unsafe_gateway_mutation:{subject}"),
                base_evidence,
                format!("{subject} attempted unsafe gateway-owned mutation"),
                "move mutation authority back to Kernel and quarantine the Gateway",
            ),
        ));
    }
    None
}

fn invariant_from_finding(finding: &KernelSentinelFinding) -> Option<String> {
    if finding.status != "open" {
        return None;
    }
    let haystack = format!("{} {} {}", finding.fingerprint, finding.summary, finding.recommended_action).to_lowercase();
    if haystack.contains("forged") || haystack.contains("forgery") {
        Some("receipt_forgery".to_string())
    } else if haystack.contains("payload_shortcut") || haystack.contains("transport_available") {
        Some("capability_bypass".to_string())
    } else if finding.category == KernelSentinelFindingCategory::StateTransition {
        Some("illegal_state_transition".to_string())
    } else if haystack.contains("expired") && (haystack.contains("waiver") || haystack.contains("exemption")) {
        Some("expired_critical_exemption".to_string())
    } else if finding.category == KernelSentinelFindingCategory::ReleaseEvidence
        && (haystack.contains("required_missing") || haystack.contains("missing_required"))
    {
        Some("missing_proof_pack_required_artifact".to_string())
    } else if finding.category == KernelSentinelFindingCategory::GatewayIsolation
        && haystack.contains("unsafe_gateway_mutation")
    {
        Some("unsafe_gateway_mutation".to_string())
    } else {
        None
    }
}

fn freshness_finding(record: &Value, window_seconds: u64) -> Option<KernelSentinelFinding> {
    let details = record.get("details").unwrap_or(&Value::Null);
    let age_seconds = value_u64(details, "freshness_age_seconds")
        .or_else(|| value_u64(details, "age_seconds"))
        .or_else(|| {
            value_u64(details, "generated_at_epoch_seconds")
                .map(|generated_at| unix_now().saturating_sub(generated_at))
        })?;
    if age_seconds <= window_seconds {
        return None;
    }
    let subject = record_subject(record);
    let kind = record_kind(record);
    Some(finding(
        format!("sentinel_freshness_stale:{kind}:{subject}"),
        KernelSentinelFindingCategory::RuntimeCorrectness,
        format!("sentinel_freshness_stale:{kind}:{subject}"),
        evidence(record, &format!("freshness://{kind}/{subject}")),
        format!("{kind} for {subject} is stale: {age_seconds}s exceeds {window_seconds}s"),
        "refresh Sentinel artifacts before trusting release or live safety state",
    ))
}

pub fn build_governance_preflight(
    findings: &[KernelSentinelFinding],
    evidence_report: &Value,
    args: &[String],
) -> (Value, Vec<KernelSentinelFinding>) {
    let freshness_window_seconds =
        option_u64(args, "--freshness-window-seconds", DEFAULT_FRESHNESS_WINDOW_SECONDS);
    let records = normalized_records(evidence_report);
    let mut generated = Vec::new();
    let mut invariant_rows = Vec::new();
    let mut freshness_rows = Vec::new();
    for finding in findings {
        if let Some(invariant) = invariant_from_finding(finding) {
            invariant_rows.push(json!({
                "invariant": invariant,
                "status": "fail",
                "fingerprint": finding.fingerprint,
                "source": "finding"
            }));
        }
    }
    for record in &records {
        if let Some((invariant, record_finding)) = invariant_from_record(record) {
            invariant_rows.push(json!({
                "invariant": invariant,
                "status": "fail",
                "fingerprint": record_finding.fingerprint,
                "source": "normalized_record"
            }));
            generated.push(record_finding);
        }
        if let Some(record_finding) = freshness_finding(record, freshness_window_seconds) {
            freshness_rows.push(json!({
                "fingerprint": record_finding.fingerprint,
                "status": "fail",
                "window_seconds": freshness_window_seconds
            }));
            generated.push(record_finding);
        }
    }
    let (grader_stack, grader_findings) = build_grader_stack(findings, &records);
    generated.extend(grader_findings);
    let (rsi_handoff_report, rsi_findings) = build_rsi_handoff_report(&records);
    generated.extend(rsi_findings);
    let hard_fail_count = invariant_rows.len();
    let stale_count = freshness_rows.len();
    let grader_blocking_count = grader_stack
        .get("blocking_failure_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let rsi_blocking_count = rsi_handoff_report
        .get("blocking_failure_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    (json!({
        "ok": hard_fail_count == 0 && stale_count == 0 && grader_blocking_count == 0 && rsi_blocking_count == 0,
        "type": "kernel_sentinel_governance_preflight",
        "hard_fail_invariant_count": hard_fail_count,
        "freshness_stale_count": stale_count,
        "grader_blocking_count": grader_blocking_count,
        "rsi_handoff_blocking_count": rsi_blocking_count,
        "freshness_window_seconds": freshness_window_seconds,
        "hard_fail_invariants": invariant_rows,
        "freshness_slos": freshness_rows,
        "grader_stack": grader_stack,
        "rsi_safety_handoff": rsi_handoff_report
    }), generated)
}

fn malformed_issue_count(issue_synthesis: &Value) -> usize {
    let malformed_drafts = issue_synthesis
        .get("issue_drafts")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter(|row| {
                    text(row, "title", "").is_empty()
                        || text(row, "fingerprint", "").is_empty()
                        || text(row, "recommended_fix", "").is_empty()
                        || row.get("evidence").and_then(Value::as_array).map_or(true, |e| e.is_empty())
                        || row
                            .get("acceptance_criteria")
                            .and_then(Value::as_array)
                            .map_or(true, |criteria| criteria.is_empty())
                })
                .count()
        })
        .unwrap_or(0);
    let low_quality_drafts = issue_synthesis
        .get("issue_quality")
        .and_then(|quality| quality.get("low_quality_issue_count"))
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    malformed_drafts + low_quality_drafts
}

fn malformed_maintenance_count(maintenance_synthesis: &Value) -> usize {
    ["suggestions", "automation_candidates"]
        .iter()
        .map(|key| {
            maintenance_synthesis
                .get(key)
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .filter(|row| text(row, "fingerprint", "").is_empty() || text(row, "type", "").is_empty())
                        .count()
                })
                .unwrap_or(0)
        })
        .sum()
}

pub fn build_release_gate(
    findings: &[KernelSentinelFinding],
    malformed_findings: &[Value],
    issue_synthesis: &Value,
    maintenance_synthesis: &Value,
    governance_preflight: &Value,
) -> Value {
    let critical_open_count = findings
        .iter()
        .filter(|finding| finding.severity == KernelSentinelSeverity::Critical && finding.status == "open")
        .count();
    let malformed_issue_count = malformed_issue_count(issue_synthesis);
    let malformed_maintenance_count = malformed_maintenance_count(maintenance_synthesis);
    let hard_fail_count = governance_preflight["hard_fail_invariant_count"].as_u64().unwrap_or(0);
    let freshness_stale_count = governance_preflight["freshness_stale_count"].as_u64().unwrap_or(0);
    let grader_blocking_count = governance_preflight["grader_blocking_count"].as_u64().unwrap_or(0);
    let rsi_handoff_blocking_count = governance_preflight["rsi_handoff_blocking_count"].as_u64().unwrap_or(0);
    let pass = critical_open_count == 0
        && malformed_findings.is_empty()
        && malformed_issue_count == 0
        && malformed_maintenance_count == 0
        && hard_fail_count == 0
        && freshness_stale_count == 0
        && grader_blocking_count == 0
        && rsi_handoff_blocking_count == 0;
    json!({
        "type": "kernel_sentinel_release_gate",
        "pass": pass,
        "required_artifacts": [
            "kernel_sentinel_report_current.json",
            "kernel_sentinel_verdict.json",
            "issues.jsonl",
            "suggestions.jsonl",
            "automation_candidates.jsonl",
            "watch_freshness.json",
            "waiver_audit.jsonl"
        ],
        "proof_pack_manifest_required_artifacts": [
            "kernel_sentinel_report_current.json",
            "kernel_sentinel_verdict.json"
        ],
        "critical_open_count": critical_open_count,
        "malformed_finding_count": malformed_findings.len(),
        "malformed_issue_count": malformed_issue_count,
        "malformed_maintenance_count": malformed_maintenance_count,
        "hard_fail_invariant_count": hard_fail_count,
        "freshness_stale_count": freshness_stale_count,
        "grader_blocking_count": grader_blocking_count,
        "rsi_handoff_blocking_count": rsi_handoff_blocking_count
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding_with(category: KernelSentinelFindingCategory, fingerprint: &str) -> KernelSentinelFinding {
        KernelSentinelFinding {
            schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
            id: "finding-1".to_string(),
            severity: KernelSentinelSeverity::Critical,
            category,
            fingerprint: fingerprint.to_string(),
            evidence: vec!["receipt://one".to_string()],
            summary: fingerprint.to_string(),
            recommended_action: "restore Kernel invariant".to_string(),
            status: "open".to_string(),
        }
    }

    #[test]
    fn hard_fail_preflight_detects_required_proof_pack_gaps() {
        let evidence_report = json!({"normalized_records": [{
            "source": "release_proof_pack",
            "subject": "rc-pack",
            "kind": "proof_pack",
            "evidence": ["proof://rc-pack"],
            "details": {"required_missing": 1}
        }]});
        let (report, findings) = build_governance_preflight(&[], &evidence_report, &[]);
        assert_eq!(report["hard_fail_invariant_count"], Value::from(1));
        assert!(findings.iter().any(|f| f.fingerprint == "hard_fail:missing_proof_pack_required_artifact:rc-pack"));
    }

    #[test]
    fn stale_freshness_record_creates_release_blocking_finding() {
        let evidence_report = json!({"normalized_records": [{
            "source": "runtime_observation",
            "subject": "watch",
            "kind": "background_watch",
            "evidence": ["freshness://watch"],
            "details": {"freshness_age_seconds": 7200}
        }]});
        let args = vec!["--freshness-window-seconds=60".to_string()];
        let (report, findings) = build_governance_preflight(&[], &evidence_report, &args);
        assert_eq!(report["freshness_stale_count"], Value::from(1));
        assert!(findings.iter().any(|f| f.fingerprint == "sentinel_freshness_stale:background_watch:watch"));
    }

    #[test]
    fn release_gate_fails_on_critical_and_passes_on_clean_inputs() {
        let critical = finding_with(KernelSentinelFindingCategory::ReceiptIntegrity, "receipt_forgery:demo");
        let issue = json!({"issue_drafts": []});
        let maintenance = json!({"suggestions": [], "automation_candidates": []});
        let governance = json!({"hard_fail_invariant_count": 0, "freshness_stale_count": 0});
        let failed = build_release_gate(&[critical], &[], &issue, &maintenance, &governance);
        assert_eq!(failed["pass"], false);
        let passed = build_release_gate(&[], &[], &issue, &maintenance, &governance);
        assert_eq!(passed["pass"], true);
    }
}
