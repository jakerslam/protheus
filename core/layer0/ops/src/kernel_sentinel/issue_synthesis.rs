// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{KernelSentinelFinding, KernelSentinelSeverity};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
struct FindingCluster {
    cluster_key: String,
    issue_family_fingerprint: String,
    issue_family_kind: String,
    scenario_id: String,
    exemplar: KernelSentinelFinding,
    occurrence_count: usize,
    first_seen_index: usize,
    last_seen_index: usize,
    session: String,
    surface: String,
    receipt_type: String,
    recovery_reason: String,
    evidence: BTreeSet<String>,
}

fn option_usize(args: &[String], name: &str, fallback: usize) -> usize {
    let prefix = format!("{name}=");
    args.iter()
        .find_map(|arg| arg.strip_prefix(&prefix).and_then(|raw| raw.parse::<usize>().ok()))
        .unwrap_or(fallback)
}

fn bool_flag(args: &[String], name: &str) -> bool {
    args.iter()
        .any(|arg| arg == name || arg == &format!("{name}=1") || arg == &format!("{name}=true"))
}

fn raw_source_fingerprint(fingerprint: &str) -> bool {
    [
        "kernel_receipt:",
        "runtime_observation:",
        "release_proof_pack:",
        "gateway_health:",
        "queue_backpressure:",
        "control_plane_eval:",
    ]
    .iter()
    .any(|prefix| fingerprint.starts_with(prefix))
}

fn redacted_evidence(rows: &BTreeSet<String>) -> Vec<String> {
    rows.iter()
        .map(|row| {
            if row.contains("github_pat_") || row.contains("ghp_") || row.contains("api_key=") {
                "redacted://secret-bearing-evidence-ref".to_string()
            } else {
                row.to_string()
            }
        })
        .collect()
}

fn severity_rank(severity: KernelSentinelSeverity) -> u8 {
    match severity {
        KernelSentinelSeverity::Critical => 0,
        KernelSentinelSeverity::High => 1,
        KernelSentinelSeverity::Medium => 2,
        KernelSentinelSeverity::Low => 3,
    }
}

fn evidence_token(rows: &[String], key: &str) -> Option<String> {
    let needle = format!("{key}=");
    rows.iter().find_map(|row| {
        let start = row.find(&needle)? + needle.len();
        let value = row[start..]
            .split(|ch: char| matches!(ch, ';' | ',' | '|' | '&' | ' ' | '#'))
            .next()
            .unwrap_or("")
            .trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

fn evidence_scheme(rows: &[String], scheme: &str) -> Option<String> {
    let prefix = format!("{scheme}://");
    rows.iter().find_map(|row| {
        let value = row.strip_prefix(&prefix)?;
        let value = value
            .split(|ch: char| matches!(ch, '/' | ';' | ',' | '|' | '&' | ' ' | '#'))
            .next()
            .unwrap_or("")
            .trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

fn recovery_reason(finding: &KernelSentinelFinding) -> String {
    let text = format!("{} {}", finding.summary, finding.recommended_action).to_lowercase();
    if text.contains("quarantine") {
        "quarantine".to_string()
    } else if text.contains("rollback") {
        "rollback".to_string()
    } else if text.contains("shed") || text.contains("backpressure") {
        "shed_or_defer".to_string()
    } else if text.contains("receipt") {
        "restore_receipt".to_string()
    } else if text.contains("grant") || text.contains("capability") {
        "restore_capability_grant".to_string()
    } else {
        "inspect_kernel_evidence".to_string()
    }
}

fn cluster_fields(finding: &KernelSentinelFinding) -> (String, String, String, String) {
    let session = evidence_token(&finding.evidence, "session")
        .or_else(|| evidence_scheme(&finding.evidence, "session"))
        .unwrap_or_else(|| "unknown_session".to_string());
    let surface = evidence_token(&finding.evidence, "surface")
        .or_else(|| evidence_scheme(&finding.evidence, "surface"))
        .unwrap_or_else(|| format!("{:?}", finding.category).to_lowercase());
    let receipt_type = evidence_token(&finding.evidence, "receipt_type")
        .or_else(|| evidence_scheme(&finding.evidence, "receipt"))
        .unwrap_or_else(|| "unspecified_receipt".to_string());
    let recovery_reason = recovery_reason(finding);
    (session, surface, receipt_type, recovery_reason)
}

fn issue_family_fingerprint(fingerprint: &str) -> String {
    const MISTY_ROUND_PREFIX: &str = "misty_simulated_round";
    let normalized = fingerprint.to_ascii_lowercase();
    let Some(prefix_index) = normalized.find(MISTY_ROUND_PREFIX) else {
        return fingerprint.to_string();
    };
    let suffix_index = prefix_index + MISTY_ROUND_PREFIX.len();
    let rest = &normalized[suffix_index..];
    let digit_count = rest.chars().take_while(|ch| ch.is_ascii_digit()).count();
    if digit_count == 0 {
        return fingerprint.to_string();
    }
    let after_digits = &rest[digit_count..];
    if matches!(
        after_digits,
        "_failure" | "_failures" | "-failure" | "-failures" | ":failure" | ":failures"
    ) {
        "synthetic_user_chat_harness:misty_simulated_failures".to_string()
    } else {
        fingerprint.to_string()
    }
}

fn synthetic_issue_scenario_id(issue_family_fingerprint: &str) -> String {
    if issue_family_fingerprint == "synthetic_user_chat_harness:misty_simulated_failures" {
        "misty_simulated_failures".to_string()
    } else {
        "none".to_string()
    }
}

fn issue_family_kind(scenario_id: &str) -> String {
    if scenario_id == "none" {
        "fingerprint_cluster".to_string()
    } else {
        "synthetic_scenario".to_string()
    }
}

fn issue_cluster_key(
    issue_family_fingerprint: &str,
    scenario_id: &str,
    session: &str,
    surface: &str,
    receipt_type: &str,
    recovery_reason: &str,
) -> String {
    if scenario_id != "none" {
        return format!("scenario={scenario_id}|fingerprint={issue_family_fingerprint}");
    }
    format!(
        "{issue_family_fingerprint}|session={session}|surface={surface}|receipt_type={receipt_type}|recovery={recovery_reason}"
    )
}

fn issue_title(finding: &KernelSentinelFinding) -> String {
    format!(
        "[{:?}] Kernel Sentinel {:?}: {}",
        finding.severity, finding.category, finding.summary
    )
}

fn issue_draft(cluster: &FindingCluster) -> Value {
    let finding = &cluster.exemplar;
    let evidence = redacted_evidence(&cluster.evidence);
    json!({
        "type": "kernel_sentinel_issue_draft",
        "status": "draft",
        "title": issue_title(finding),
        "severity": finding.severity,
        "category": finding.category,
        "fingerprint": cluster.issue_family_fingerprint,
        "issue_family_kind": cluster.issue_family_kind,
        "scenario_level": cluster.scenario_id != "none",
        "scenario_id": cluster.scenario_id,
        "exemplar_fingerprint": finding.fingerprint,
        "cluster_key": cluster.cluster_key,
        "occurrence_count": cluster.occurrence_count,
        "first_seen_index": cluster.first_seen_index,
        "last_seen_index": cluster.last_seen_index,
        "session": cluster.session,
        "surface": cluster.surface,
        "receipt_type": cluster.receipt_type,
        "recovery_reason": cluster.recovery_reason,
        "evidence": evidence,
        "expected_behavior": "Kernel-owned runtime law remains fail-closed, receipted, bounded, and recoverable.",
        "actual_behavior": finding.summary,
        "impact": format!(
            "Repeated {:?} Kernel Sentinel finding can degrade runtime correctness, security, or release confidence.",
            finding.category
        ),
        "recommended_fix": finding.recommended_action,
        "acceptance_criteria": [
            "deterministic evidence no longer emits this fingerprint",
            "strict Kernel Sentinel report returns allow for this scenario",
            "regression fixture covers the failure signature"
        ]
    })
}

fn deterministic_evidence_present(row: &Value) -> bool {
    row.get("evidence")
        .and_then(Value::as_array)
        .map(|evidence| {
            evidence.iter().filter_map(Value::as_str).any(|ref_id| {
                !ref_id.starts_with("semantic://")
                    && !ref_id.starts_with("control_plane_eval://")
                    && ref_id != "redacted://secret-bearing-evidence-ref"
            })
        })
        .unwrap_or(false)
}

fn issue_quality_failures(drafts: &[Value]) -> Vec<Value> {
    let mut failures = Vec::new();
    for draft in drafts {
        let fingerprint = draft
            .get("fingerprint")
            .and_then(Value::as_str)
            .unwrap_or("unknown_fingerprint");
        let title = draft.get("title").and_then(Value::as_str).unwrap_or("").trim();
        let impact = draft.get("impact").and_then(Value::as_str).unwrap_or("").trim();
        let recommended_fix = draft
            .get("recommended_fix")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        let acceptance_count = draft
            .get("acceptance_criteria")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        let mut reasons = Vec::new();
        if title.len() < 16 || title.eq_ignore_ascii_case("issue") {
            reasons.push("vague_title");
        }
        if !deterministic_evidence_present(draft) {
            reasons.push("missing_deterministic_evidence");
        }
        if impact.len() < 24 {
            reasons.push("missing_impact");
        }
        if recommended_fix.len() < 16 {
            reasons.push("missing_recommended_fix");
        }
        if acceptance_count < 3 {
            reasons.push("insufficient_acceptance_criteria");
        }
        if !reasons.is_empty() {
            failures.push(json!({
                "fingerprint": fingerprint,
                "reasons": reasons,
                "blocks_release": true
            }));
        }
    }
    failures
}

pub fn build_issue_synthesis(findings: &[KernelSentinelFinding], args: &[String]) -> Value {
    let threshold = option_usize(args, "--issue-threshold", 2).max(1);
    let include_raw_source_drafts = bool_flag(args, "--issue-include-raw-source");
    let mut clusters: BTreeMap<String, FindingCluster> = BTreeMap::new();
    for (index, finding) in findings.iter().enumerate() {
        if finding.status != "open" {
            continue;
        }
        if !include_raw_source_drafts && raw_source_fingerprint(&finding.fingerprint) {
            continue;
        }
        let (session, surface, receipt_type, recovery_reason) = cluster_fields(finding);
        let issue_family_fingerprint = issue_family_fingerprint(&finding.fingerprint);
        let scenario_id = synthetic_issue_scenario_id(&issue_family_fingerprint);
        let issue_family_kind = issue_family_kind(&scenario_id);
        let cluster_key = issue_cluster_key(
            &issue_family_fingerprint,
            &scenario_id,
            &session,
            &surface,
            &receipt_type,
            &recovery_reason,
        );
        let entry = clusters
            .entry(cluster_key.clone())
            .or_insert_with(|| FindingCluster {
                cluster_key,
                issue_family_fingerprint,
                issue_family_kind,
                scenario_id,
                exemplar: finding.clone(),
                occurrence_count: 0,
                first_seen_index: index,
                last_seen_index: index,
                session,
                surface,
                receipt_type,
                recovery_reason,
                evidence: BTreeSet::new(),
            });
        entry.occurrence_count += 1;
        entry.last_seen_index = index;
        if severity_rank(finding.severity) < severity_rank(entry.exemplar.severity) {
            entry.exemplar = finding.clone();
        }
        entry.evidence.extend(finding.evidence.iter().cloned());
    }
    let issue_drafts = clusters
        .values()
        .filter(|cluster| cluster.occurrence_count >= threshold)
        .map(issue_draft)
        .collect::<Vec<_>>();
    let issue_quality_failures = issue_quality_failures(&issue_drafts);
    let active_issue_window_count = issue_drafts.len();
    let rate_limited_cluster_count = clusters
        .values()
        .filter(|cluster| cluster.occurrence_count < threshold)
        .count();
    json!({
        "ok": true,
        "type": "kernel_sentinel_issue_synthesis",
        "issue_threshold": threshold,
        "raw_source_drafts_included": include_raw_source_drafts,
        "cluster_count": clusters.len(),
        "active_issue_window_count": active_issue_window_count,
        "rate_limited_cluster_count": rate_limited_cluster_count,
        "cluster_dimensions": ["fingerprint", "session", "surface", "receipt_type", "recovery_reason"],
        "issue_draft_count": issue_drafts.len(),
        "issue_quality": {
            "ok": issue_quality_failures.is_empty(),
            "type": "kernel_sentinel_issue_quality_guard",
            "low_quality_issue_count": issue_quality_failures.len(),
            "failures": issue_quality_failures
        },
        "issue_drafts": issue_drafts
    })
}

pub fn write_issue_drafts_jsonl(path: &Path, report: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let mut body = String::new();
    if let Some(drafts) = report["issue_synthesis"]["issue_drafts"].as_array() {
        for draft in drafts {
            body.push_str(&serde_json::to_string(draft).map_err(|err| err.to_string())?);
            body.push('\n');
        }
    }
    fs::write(path, body).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel_sentinel::{
        KernelSentinelFindingCategory, KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
    };

    fn repeated_finding() -> KernelSentinelFinding {
        KernelSentinelFinding {
            schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
            id: "finding-1".to_string(),
            severity: KernelSentinelSeverity::High,
            category: KernelSentinelFindingCategory::GatewayIsolation,
            fingerprint: "gateway_isolation:gateway_missing_quarantine:ollama".to_string(),
            evidence: vec!["gateway://ollama/flap".to_string()],
            summary: "ollama gateway flapped without quarantine".to_string(),
            recommended_action: "quarantine the gateway".to_string(),
            status: "open".to_string(),
        }
    }

    #[test]
    fn repeated_fingerprint_produces_one_issue_draft() {
        let finding = repeated_finding();
        let report = build_issue_synthesis(&[finding.clone(), finding], &[]);
        assert_eq!(report["active_issue_window_count"], Value::from(1));
        assert_eq!(report["issue_drafts"][0]["occurrence_count"], Value::from(2));
    }

    #[test]
    fn singleton_fingerprint_is_rate_limited_by_default() {
        let report = build_issue_synthesis(&[repeated_finding()], &[]);
        assert_eq!(report["active_issue_window_count"], Value::from(0));
        assert_eq!(report["rate_limited_cluster_count"], Value::from(1));
    }

    #[test]
    fn raw_source_fingerprints_are_not_issue_drafts_by_default() {
        let mut finding = repeated_finding();
        finding.fingerprint = "gateway_health:ollama:gateway_health".to_string();
        let report = build_issue_synthesis(&[finding.clone(), finding], &[]);
        assert_eq!(report["cluster_count"], Value::from(0));
        assert_eq!(report["issue_draft_count"], Value::from(0));
    }

    #[test]
    fn synthetic_round_failures_collapse_into_one_issue_family() {
        let mut round_1 = repeated_finding();
        round_1.category = KernelSentinelFindingCategory::RuntimeCorrectness;
        round_1.fingerprint = "runtime_correctness:misty_simulated_round01_failures".to_string();
        round_1.evidence = vec![
            "runtime://misty;session=synthetic-user-chat;surface=chat;receipt_type=tool_route"
                .to_string(),
        ];
        round_1.summary = "misty synthetic chat harness failed during simulated round 01".to_string();
        round_1.recommended_action =
            "collapse repeated synthetic round failures into one issue family".to_string();

        let mut round_2 = round_1.clone();
        round_2.id = "finding-2".to_string();
        round_2.fingerprint = "runtime_correctness:misty_simulated_round02_failures".to_string();
        round_2.summary = "misty synthetic chat harness failed during simulated round 02".to_string();

        let report = build_issue_synthesis(&[round_1, round_2], &[]);
        assert_eq!(report["cluster_count"], Value::from(1));
        assert_eq!(report["active_issue_window_count"], Value::from(1));
        assert_eq!(report["issue_draft_count"], Value::from(1));
        assert_eq!(
            report["issue_drafts"][0]["fingerprint"],
            "synthetic_user_chat_harness:misty_simulated_failures"
        );
        assert_eq!(report["issue_drafts"][0]["occurrence_count"], Value::from(2));
        assert_eq!(
            report["issue_drafts"][0]["exemplar_fingerprint"],
            "runtime_correctness:misty_simulated_round01_failures"
        );
    }

    #[test]
    fn synthetic_round_failures_collapse_across_sessions_into_scenario_issue_candidate() {
        let mut round_1 = repeated_finding();
        round_1.category = KernelSentinelFindingCategory::RuntimeCorrectness;
        round_1.fingerprint = "runtime_correctness:misty_simulated_round01_failures".to_string();
        round_1.evidence = vec![
            "runtime://misty;session=synthetic-user-chat-a;surface=chat;receipt_type=tool_route"
                .to_string(),
        ];
        round_1.summary = "misty synthetic chat harness failed during simulated round 01".to_string();
        round_1.recommended_action =
            "collapse repeated synthetic round failures into one scenario issue".to_string();

        let mut round_2 = round_1.clone();
        round_2.id = "finding-2".to_string();
        round_2.fingerprint = "runtime_correctness:misty_simulated_round02_failures".to_string();
        round_2.evidence = vec![
            "runtime://misty;session=synthetic-user-chat-b;surface=chat;receipt_type=final_response"
                .to_string(),
        ];

        let report = build_issue_synthesis(&[round_1, round_2], &[]);
        let draft = &report["issue_drafts"][0];

        assert_eq!(report["cluster_count"], Value::from(1));
        assert_eq!(draft["scenario_level"], true);
        assert_eq!(draft["issue_family_kind"], "synthetic_scenario");
        assert_eq!(draft["scenario_id"], "misty_simulated_failures");
        assert_eq!(
            draft["cluster_key"],
            "scenario=misty_simulated_failures|fingerprint=synthetic_user_chat_harness:misty_simulated_failures"
        );
        assert_eq!(draft["occurrence_count"], Value::from(2));
    }

    #[test]
    fn cluster_key_separates_sessions_and_preserves_rate_limit() {
        let mut first = repeated_finding();
        first.evidence = vec!["gateway://ollama/flap;session=a;surface=gateway;receipt_type=quarantine".to_string()];
        let mut second = first.clone();
        second.evidence = vec!["gateway://ollama/flap;session=b;surface=gateway;receipt_type=quarantine".to_string()];
        let report = build_issue_synthesis(&[first, second], &[]);
        assert_eq!(report["cluster_count"], Value::from(2));
        assert_eq!(report["active_issue_window_count"], Value::from(0));
        assert_eq!(report["rate_limited_cluster_count"], Value::from(2));
    }

    #[test]
    fn issue_quality_guard_rejects_advisory_only_vague_drafts() {
        let failures = issue_quality_failures(&[json!({
            "title": "issue",
            "fingerprint": "semantic_monitor:maybe",
            "evidence": ["semantic://summary-only"],
            "impact": "",
            "recommended_fix": "",
            "acceptance_criteria": ["look again"]
        })]);
        assert_eq!(failures.len(), 1);
        assert!(failures[0]["reasons"].as_array().unwrap().contains(&Value::from("missing_deterministic_evidence")));
        assert_eq!(failures[0]["blocks_release"], true);
    }
}
