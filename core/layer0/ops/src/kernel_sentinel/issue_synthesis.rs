// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    diagnostic_run_artifact::attach_diagnostic_context_to_issue_draft,
    incident_report::violated_invariants,
    kernel_sentinel_semantic_frame_for_finding,
    issue_cluster_semantics::{
        cluster_fields, issue_cluster_key, issue_family_fingerprint, issue_family_kind,
        issue_summary, issue_title, severity_rank, synthetic_issue_scenario_id, FindingCluster,
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
    let summary = issue_summary(finding, cluster.occurrence_count, &cluster.recovery_reason);
    json!({
        "type": "kernel_sentinel_issue_draft",
        "status": "draft",
        "title": issue_title(finding),
        "summary": summary,
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

pub fn write_issue_drafts_jsonl(
    path: &Path,
    report: &Value,
    diagnostic_run: Option<&Value>,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let mut body = String::new();
    if let Some(drafts) = report["issue_synthesis"]["issue_drafts"].as_array() {
        for draft in drafts {
            let row = diagnostic_run
                .map(|run| attach_diagnostic_context_to_issue_draft(draft, run))
                .unwrap_or_else(|| draft.clone());
            body.push_str(&serde_json::to_string(&row).map_err(|err| err.to_string())?);
            body.push('\n');
        }
    }
    fs::write(path, body).map_err(|err| err.to_string())
}

#[cfg(test)]
#[path = "issue_synthesis_tests.rs"]
mod issue_synthesis_tests;
