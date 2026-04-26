// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    KernelSentinelFinding, KernelSentinelFindingCategory, KernelSentinelSeverity,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
struct MaintenanceCluster {
    exemplar: KernelSentinelFinding,
    occurrence_count: usize,
    evidence: BTreeSet<String>,
}

fn option_usize(args: &[String], name: &str, fallback: usize) -> usize {
    let prefix = format!("{name}=");
    args.iter()
        .find_map(|arg| arg.strip_prefix(&prefix).and_then(|raw| raw.parse::<usize>().ok()))
        .unwrap_or(fallback)
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

fn suggestion_label(category: KernelSentinelFindingCategory) -> &'static str {
    match category {
        KernelSentinelFindingCategory::PerformanceRegression
        | KernelSentinelFindingCategory::Boundedness
        | KernelSentinelFindingCategory::QueueBackpressure
        | KernelSentinelFindingCategory::RetryStorm => "optimization",
        KernelSentinelFindingCategory::SelfMaintenanceLoop
        | KernelSentinelFindingCategory::AutomationCandidate => "cleanup",
        _ => "hardening",
    }
}

fn suggestion_action(cluster: &MaintenanceCluster) -> String {
    match suggestion_label(cluster.exemplar.category) {
        "optimization" => format!(
            "tune thresholds or shed earlier for repeated `{}` evidence",
            cluster.exemplar.fingerprint
        ),
        "cleanup" => format!(
            "consolidate duplicate checks or stale remediation loops for `{}`",
            cluster.exemplar.fingerprint
        ),
        _ => format!(
            "strengthen Kernel proof burden and regression coverage for `{}`",
            cluster.exemplar.fingerprint
        ),
    }
}

fn automation_state(cluster: &MaintenanceCluster) -> &'static str {
    match cluster.exemplar.severity {
        KernelSentinelSeverity::Critical | KernelSentinelSeverity::High => "issue_draft",
        KernelSentinelSeverity::Medium | KernelSentinelSeverity::Low => "suggest_patch",
    }
}

fn build_clusters(findings: &[KernelSentinelFinding]) -> BTreeMap<String, MaintenanceCluster> {
    let mut clusters = BTreeMap::new();
    for finding in findings {
        if finding.status != "open" {
            continue;
        }
        if raw_source_fingerprint(&finding.fingerprint) {
            continue;
        }
        let entry = clusters
            .entry(finding.fingerprint.clone())
            .or_insert_with(|| MaintenanceCluster {
                exemplar: finding.clone(),
                occurrence_count: 0,
                evidence: BTreeSet::new(),
            });
        entry.occurrence_count += 1;
        entry.evidence.extend(finding.evidence.iter().cloned());
    }
    clusters
}

fn suggestion(cluster: &MaintenanceCluster) -> Value {
    json!({
        "type": "kernel_sentinel_suggestion",
        "status": "nonblocking",
        "label": suggestion_label(cluster.exemplar.category),
        "fingerprint": cluster.exemplar.fingerprint,
        "severity": cluster.exemplar.severity,
        "category": cluster.exemplar.category,
        "occurrence_count": cluster.occurrence_count,
        "evidence": cluster.evidence.iter().cloned().collect::<Vec<_>>(),
        "suggested_change": suggestion_action(cluster),
        "blocks_release": false,
        "promotion_requires_policy": true
    })
}

fn automation_candidate(cluster: &MaintenanceCluster) -> Value {
    json!({
        "type": "kernel_sentinel_automation_candidate",
        "fingerprint": cluster.exemplar.fingerprint,
        "state": automation_state(cluster),
        "occurrence_count": cluster.occurrence_count,
        "v1_max_state": "suggest_patch",
        "allowed_apply": false,
        "may_waive_findings": false,
        "supervised_apply_enabled": false,
        "reason": "automation remains observe-only/issue/suggestion until separate policy promotes it"
    })
}

pub fn build_maintenance_synthesis(findings: &[KernelSentinelFinding], args: &[String]) -> Value {
    let suggestion_threshold = option_usize(args, "--suggestion-threshold", 2).max(1);
    let automation_threshold = option_usize(args, "--automation-threshold", 3).max(1);
    let clusters = build_clusters(findings);
    let suggestions = clusters
        .values()
        .filter(|cluster| cluster.occurrence_count >= suggestion_threshold)
        .map(suggestion)
        .collect::<Vec<_>>();
    let automation_candidates = clusters
        .values()
        .filter(|cluster| cluster.occurrence_count >= automation_threshold)
        .map(automation_candidate)
        .collect::<Vec<_>>();
    json!({
        "ok": true,
        "type": "kernel_sentinel_maintenance_synthesis",
        "suggestion_threshold": suggestion_threshold,
        "automation_threshold": automation_threshold,
        "automation_ladder": [
            "observe_only",
            "issue_draft",
            "suggest_patch",
            "propose_policy_change",
            "supervised_apply",
            "bounded_auto_apply"
        ],
        "v1_allowed_states": ["observe_only", "issue_draft", "suggest_patch"],
        "cluster_count": clusters.len(),
        "suggestion_count": suggestions.len(),
        "automation_candidate_count": automation_candidates.len(),
        "suggestions": suggestions,
        "automation_candidates": automation_candidates
    })
}

pub fn write_maintenance_jsonl(dir: &Path, report: &Value) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|err| err.to_string())?;
    for (file_name, key) in [
        ("suggestions.jsonl", "suggestions"),
        ("automation_candidates.jsonl", "automation_candidates"),
    ] {
        let mut body = String::new();
        if let Some(rows) = report["maintenance_synthesis"][key].as_array() {
            for row in rows {
                body.push_str(&serde_json::to_string(row).map_err(|err| err.to_string())?);
                body.push('\n');
            }
        }
        fs::write(dir.join(file_name), body).map_err(|err| err.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel_sentinel::KERNEL_SENTINEL_FINDING_SCHEMA_VERSION;

    fn finding(severity: KernelSentinelSeverity) -> KernelSentinelFinding {
        KernelSentinelFinding {
            schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
            id: "finding-1".to_string(),
            severity,
            category: KernelSentinelFindingCategory::QueueBackpressure,
            fingerprint: "boundedness:queue_depth:ops".to_string(),
            evidence: vec!["queue://ops/depth".to_string()],
            summary: "queue depth exceeded budget".to_string(),
            recommended_action: "shed earlier under pressure".to_string(),
            status: "open".to_string(),
        }
    }

    #[test]
    fn repeated_evidence_produces_nonblocking_suggestion() {
        let report = build_maintenance_synthesis(&[finding(KernelSentinelSeverity::Medium), finding(KernelSentinelSeverity::Medium)], &[]);
        assert_eq!(report["suggestion_count"], Value::from(1));
        assert_eq!(report["suggestions"][0]["label"], "optimization");
        assert_eq!(report["suggestions"][0]["blocks_release"], false);
    }

    #[test]
    fn automation_candidates_are_capped_below_apply_states() {
        let report = build_maintenance_synthesis(
            &[
                finding(KernelSentinelSeverity::High),
                finding(KernelSentinelSeverity::High),
                finding(KernelSentinelSeverity::High),
            ],
            &[],
        );
        assert_eq!(report["automation_candidate_count"], Value::from(1));
        assert_eq!(report["automation_candidates"][0]["state"], "issue_draft");
        assert_eq!(report["automation_candidates"][0]["allowed_apply"], false);
        assert_eq!(report["automation_candidates"][0]["may_waive_findings"], false);
    }
}
