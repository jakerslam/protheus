// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    incident_report::violated_invariants,
    kernel_sentinel_semantic_frame_for_finding,
    issue_cluster_semantics::{
        cluster_fields, issue_cluster_key, issue_family_fingerprint, issue_family_kind,
        issue_title, severity_rank, synthetic_issue_scenario_id, FindingCluster,
    },
    KernelSentinelFinding, KernelSentinelSeverity,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

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

fn issue_draft(cluster: &FindingCluster) -> Value {
    let finding = &cluster.exemplar;
    let evidence = redacted_evidence(&cluster.evidence);
    let semantic_frame = kernel_sentinel_semantic_frame_for_finding(finding);
    json!({
        "type": "kernel_sentinel_issue_draft",
        "status": "draft",
        "title": issue_title(finding),
        "severity": finding.severity,
        "category": finding.category,
        "failure_level": semantic_frame["failure_level"].clone(),
        "root_frame": semantic_frame["root_frame"].clone(),
        "remediation_level": semantic_frame["remediation_level"].clone(),
        "violated_invariants": cluster.violated_invariants,
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
        for required in ["failure_level", "root_frame", "remediation_level"] {
            if draft.get(required).and_then(Value::as_str).unwrap_or("").trim().is_empty() {
                reasons.push("missing_semantic_frame");
                break;
            }
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
        let violated_invariants = violated_invariants(finding);
        let cluster_key = issue_cluster_key(
            &issue_family_fingerprint,
            &scenario_id,
            finding,
            &violated_invariants,
        );
        let entry = clusters
            .entry(cluster_key.clone())
            .or_insert_with(|| FindingCluster {
                cluster_key,
                issue_family_fingerprint,
                issue_family_kind,
                scenario_id,
                violated_invariants,
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
        "cluster_dimensions": ["root_frame", "violated_invariants"],
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
        assert_eq!(
            report["issue_drafts"][0]["failure_level"],
            "L2_boundary_contract_breach"
        );
        assert_eq!(
            report["issue_drafts"][0]["root_frame"],
            "cross_boundary_contract"
        );
        assert_eq!(
            report["issue_drafts"][0]["remediation_level"],
            "boundary_repair"
        );
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
    fn cluster_key_collapses_sessions_when_root_frame_and_invariant_match() {
        let mut first = repeated_finding();
        first.evidence = vec!["gateway://ollama/flap;session=a;surface=gateway;receipt_type=quarantine".to_string()];
        let mut second = first.clone();
        second.evidence = vec!["gateway://ollama/flap;session=b;surface=gateway;receipt_type=quarantine".to_string()];
        let report = build_issue_synthesis(&[first, second], &[]);
        assert_eq!(report["cluster_count"], Value::from(1));
        assert_eq!(report["active_issue_window_count"], Value::from(1));
        assert_eq!(report["rate_limited_cluster_count"], Value::from(0));
        assert_eq!(
            report["issue_drafts"][0]["cluster_key"],
            "root_frame=cross_boundary_contract|violated_invariants=unknown_invariant"
        );
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
