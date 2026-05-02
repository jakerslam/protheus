// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;

mod freshness;

use freshness::{classify_finding_freshness, RELEASE_FINDING_STALE_AFTER_SECONDS};
pub(super) const DEFAULT_FINAL_REPORT_FINDING_LIMIT: usize = 10;
pub(super) const DEFAULT_FINAL_REPORT_BYTE_BUDGET: usize = 32_768;
const MAX_TEXT_CHARS: usize = 360;
const MAX_EVIDENCE_REFS_PER_FINDING: usize = 3;
const MAX_TRIAGE_SUMMARIES: usize = 8;

pub(super) fn build_final_report(
    report: &Value,
    state_dir: &Path,
    finding_limit: usize,
    byte_budget: usize,
) -> Value {
    let (requested_findings, triage_findings, root_cause_clusters, quality_filter) =
        quality_filtered_findings(report, finding_limit);
    let requested_count = requested_findings.len();
    let mut retained_count = requested_count;

    loop {
        let mut final_report = assemble_final_report(
            report,
            state_dir,
            &requested_findings[..retained_count],
            finding_limit,
            requested_count,
            byte_budget,
            &triage_findings,
            &root_cause_clusters,
            &quality_filter,
        );
        let serialized_bytes = serialized_len(&final_report);
        final_report["report_budget"]["serialized_bytes"] = json!(serialized_bytes);
        final_report["report_budget"]["within_budget"] = json!(serialized_bytes <= byte_budget);
        final_report["report_budget"]["retained_top_finding_count"] = json!(retained_count);
        final_report["report_budget"]["dropped_top_finding_count"] =
            json!(requested_count.saturating_sub(retained_count));
        final_report["quality_filter"]["budget_retained_released_finding_count"] =
            json!(retained_count);
        let final_serialized_bytes = serialized_len(&final_report);
        final_report["report_budget"]["serialized_bytes"] = json!(final_serialized_bytes);
        final_report["report_budget"]["within_budget"] = json!(final_serialized_bytes <= byte_budget);
        if final_serialized_bytes <= byte_budget || retained_count == 0 {
            return final_report;
        }
        retained_count -= 1;
    }
}

fn assemble_final_report(
    report: &Value,
    state_dir: &Path,
    findings: &[Value],
    finding_limit: usize,
    requested_count: usize,
    byte_budget: usize,
    triage_findings: &[Value],
    root_cause_clusters: &[Value],
    quality_filter: &Value,
) -> Value {
    json!({
        "ok": report["ok"].clone(),
        "type": "kernel_sentinel_final_report",
        "artifact_kind": "operator_summary",
        "generated_at": crate::now_iso(),
        "canonical_name": super::KERNEL_SENTINEL_NAME,
        "verdict": {
            "verdict": report["verdict"]["verdict"].clone(),
            "strict": report["verdict"]["strict"].clone(),
            "critical_open_count": report["verdict"]["critical_open_count"].clone(),
            "finding_count": report["verdict"]["finding_count"].clone(),
            "malformed_finding_count": report["verdict"]["malformed_finding_count"].clone(),
            "release_blockers": report["verdict"]["release_blockers"].clone(),
        },
        "summary": {
            "status_counts": report["operator_summary"]["status_counts"].clone(),
            "severity_counts": report["operator_summary"]["severity_counts"].clone(),
            "category_counts": report["operator_summary"]["category_counts"].clone(),
            "release_gate_pass": report["operator_summary"]["release_gate_pass"].clone(),
            "data_starved": report["operator_summary"]["data_starved"].clone(),
            "partial_evidence": report["operator_summary"]["partial_evidence"].clone(),
            "malformed_evidence": report["operator_summary"]["malformed_evidence"].clone(),
            "stale_evidence": report["operator_summary"]["stale_evidence"].clone(),
            "evidence_record_count": report["operator_summary"]["evidence_record_count"].clone(),
            "reported_finding_count": report["operator_summary"]["reported_finding_count"].clone(),
            "truncated_finding_count": report["operator_summary"]["truncated_finding_count"].clone(),
        },
        "quality_filter": quality_filter.clone(),
        "root_cause_clustering": {
            "type": "kernel_sentinel_root_cause_cluster_summary",
            "cluster_count": root_cause_clusters.len(),
            "clustered_finding_count": quality_filter["released_finding_count"].clone(),
            "duplicate_symptom_count": quality_filter["clustered_duplicate_finding_count"].clone(),
            "policy": "release_ready_symptoms_share_a_single_structural_cluster_before_operator_promotion",
        },
        "causal_hypothesis_synthesis": {"hypothesis_count": report["causal_hypothesis_synthesis"]["hypothesis_count"].clone(), "quality_gate": report["causal_hypothesis_synthesis"]["quality_gate"].clone(), "top_hypotheses": report["causal_hypothesis_synthesis"]["top_hypotheses"].clone()},
        "causal_calibration": report["causal_calibration"]["final_report_summary"].clone(),
        "root_cause_clusters": root_cause_clusters, "failure_level_summary": super::report_failure_levels::build_failure_level_summary(findings, root_cause_clusters, triage_findings),
        "top_findings": findings,
        "triage_findings": triage_findings,
        "promotion_lane": super::report_promotion::build_promotion_lane(findings, root_cause_clusters, triage_findings),
        "issue_synthesis": {
            "issue_draft_count": report["issue_synthesis"]["issue_draft_count"].clone(),
            "active_issue_window_count": report["issue_synthesis"]["active_issue_window_count"].clone(),
            "rate_limited_cluster_count": report["issue_synthesis"]["rate_limited_cluster_count"].clone(),
            "issue_quality": report["issue_synthesis"]["issue_quality"].clone(),
        },
        "maintenance_synthesis": {
            "suggestion_count": report["maintenance_synthesis"]["suggestion_count"].clone(),
            "automation_candidate_count": report["maintenance_synthesis"]["automation_candidate_count"].clone(),
        },
        "raw_evidence": {
            "embedded": false,
            "reason": "raw evidence remains in append-only evidence streams and detail artifacts",
            "stream_refs": evidence_stream_refs(state_dir),
        },
        "artifact_refs": {
            "report_index": state_dir.join("kernel_sentinel_report_current.json").display().to_string(),
            "full_internal_report_opt_in": state_dir.join("kernel_sentinel_internal_report_current.json").display().to_string(),
            "verdict": state_dir.join("kernel_sentinel_verdict.json").display().to_string(),
            "health": state_dir.join("kernel_sentinel_health_current.json").display().to_string(),
            "findings": report["findings_path"].clone(),
            "issues": state_dir.join("issues.jsonl").display().to_string(),
            "suggestions": state_dir.join("suggestions.jsonl").display().to_string(),
            "automation_candidates": state_dir.join("automation_candidates.jsonl").display().to_string(),
        },
        "report_budget": {
            "max_bytes": byte_budget,
            "serialized_bytes": 0,
            "within_budget": false,
            "finding_limit": finding_limit,
            "requested_top_finding_count": requested_count,
            "retained_top_finding_count": findings.len(),
            "dropped_top_finding_count": 0,
            "quality_filtered_finding_count": quality_filter["triage_finding_count"].clone(),
            "root_cause_cluster_count": root_cause_clusters.len(),
            "raw_evidence_embedded": false,
            "full_report_embedded": false,
        },
    })
}

fn quality_filtered_findings(report: &Value, limit: usize) -> (Vec<Value>, Vec<Value>, Vec<Value>, Value) {
    let mut cluster_order: Vec<String> = Vec::new();
    let mut clusters: BTreeMap<String, RootCauseCluster> = BTreeMap::new();
    let mut triage = Vec::new();
    let mut candidate_count = 0usize;
    let mut released_finding_count = 0usize;
    if let Some(findings) = report["findings"].as_array() {
        for finding in findings.iter().take(limit) {
            candidate_count += 1;
            let quality = finding_quality(finding);
            if quality.release_ready {
                let mut compact = compact_finding(finding);
                compact["quality"] = quality.to_json("released");
                released_finding_count += 1;
                let key = root_cause_cluster_key(&compact);
                if !clusters.contains_key(&key) {
                    cluster_order.push(key.clone());
                }
                clusters.entry(key).or_insert_with(|| RootCauseCluster::new(&compact)).push(&compact);
            } else if triage.len() < MAX_TRIAGE_SUMMARIES {
                triage.push(triage_finding(finding, &quality));
            }
        }
    }
    let mut released = Vec::new();
    let mut root_cause_clusters = Vec::new();
    for key in &cluster_order {
        if let Some(cluster) = clusters.get(key) {
            let mut exemplar = cluster.exemplar.clone();
            exemplar["cluster"] = cluster.summary_json();
            released.push(exemplar);
            root_cause_clusters.push(cluster.cluster_json());
        }
    }
    let triage_count = candidate_count.saturating_sub(released_finding_count);
    let duplicate_count = released_finding_count.saturating_sub(root_cause_clusters.len());
    let filter = json!({
        "type": "kernel_sentinel_finding_release_quality_filter",
        "required_fields": ["evidence", "recurrence_or_freshness_support", "owner_guess", "root_cause_hypothesis", "concrete_next_action", "current_truth_freshness_window"],
        "release_stale_after_seconds": RELEASE_FINDING_STALE_AFTER_SECONDS,
        "candidate_finding_count": candidate_count,
        "released_finding_count": released_finding_count,
        "clustered_top_finding_count": released.len(),
        "root_cause_cluster_count": root_cause_clusters.len(),
        "clustered_duplicate_finding_count": duplicate_count,
        "triage_finding_count": triage_count,
        "triage_summary_limit": MAX_TRIAGE_SUMMARIES,
        "triage_summary_count": triage.len(),
        "release_policy": "final_report_top_findings_only_contains_release_quality_findings",
        "draft_policy": "missing_quality_findings_remain_in_triage_findings_or_raw_internal_report",
        "clustering_policy": "release_ready_findings_are_clustered_by_owner_root_cause_category_and_fingerprint_family"
    });
    (released, triage, root_cause_clusters, filter)
}

struct RootCauseCluster {
    key: String,
    owner: String,
    category: Value,
    root_cause_hypothesis: String,
    fingerprint_family: String,
    exemplar: Value,
    finding_count: usize,
    finding_ids: Vec<String>,
    evidence_refs: Vec<String>,
}

impl RootCauseCluster {
    fn new(finding: &Value) -> Self {
        Self {
            key: root_cause_cluster_key(finding),
            owner: finding["owner_guess"].as_str().unwrap_or("unknown").to_string(),
            category: finding["category"].clone(),
            root_cause_hypothesis: finding["root_cause_hypothesis"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            fingerprint_family: fingerprint_family(finding["fingerprint"].as_str().unwrap_or("unknown")),
            exemplar: finding.clone(),
            finding_count: 0,
            finding_ids: Vec::new(),
            evidence_refs: Vec::new(),
        }
    }

    fn push(&mut self, finding: &Value) {
        self.finding_count += 1;
        push_unique_compact(
            &mut self.finding_ids,
            finding["id"].as_str().unwrap_or("unknown"),
            6,
        );
        if let Some(refs) = finding["evidence_refs"].as_array() {
            for reference in refs.iter().filter_map(Value::as_str) {
                push_unique_compact(&mut self.evidence_refs, reference, MAX_EVIDENCE_REFS_PER_FINDING);
            }
        }
    }

    fn summary_json(&self) -> Value {
        json!({
            "cluster_key": self.key,
            "occurrence_count": self.finding_count,
            "fingerprint_family": self.fingerprint_family,
            "dedupe_policy": "symptom_family_collapsed_to_cluster_exemplar",
        })
    }

    fn cluster_json(&self) -> Value {
        json!({
            "cluster_key": self.key,
            "owner_guess": self.owner,
            "category": self.category, "failure_level": self.exemplar["failure_level"].clone(), "failure_class": self.exemplar["failure_class"].clone(), "remediation_level": self.exemplar["remediation_level"].clone(), "review_depth": self.exemplar["review_depth"].clone(),
            "root_cause_hypothesis": compact_text(&self.root_cause_hypothesis),
            "fingerprint_family": self.fingerprint_family,
            "occurrence_count": self.finding_count,
            "exemplar_id": self.exemplar["id"].clone(),
            "sample_finding_ids": self.finding_ids,
            "evidence_refs": self.evidence_refs,
            "recommended_next_action": self.exemplar["recommended_action"].clone(),
            "promotion_state": "cluster_ready_for_human_review",
        })
    }
}

fn root_cause_cluster_key(finding: &Value) -> String {
    let root_family = finding["root_frame"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| finding["root_cause_hypothesis"].as_str().unwrap_or("unknown"));
    format!(
        "{}|{}|{}|{}",
        finding["owner_guess"].as_str().unwrap_or("unknown"),
        finding["category"].as_str().unwrap_or("unknown"),
        normalized_cluster_text(root_family),
        fingerprint_family(finding["fingerprint"].as_str().unwrap_or("unknown"))
    )
}

fn fingerprint_family(fingerprint: &str) -> String {
    let parts = fingerprint.split(':').take(2).collect::<Vec<_>>();
    if parts.len() >= 2 {
        compact_text(&parts.join(":"))
    } else {
        compact_text(fingerprint)
    }
}

fn normalized_cluster_text(raw: &str) -> String {
    raw.chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch.to_ascii_lowercase() } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .take(8)
        .collect::<Vec<_>>()
        .join("_")
}

fn push_unique_compact(values: &mut Vec<String>, raw: &str, limit: usize) {
    if values.len() >= limit {
        return;
    }
    let value = compact_text(raw);
    if !value.is_empty() && !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn compact_finding(finding: &Value) -> Value {
    let evidence_refs = finding["evidence"]
        .as_array()
        .map(|refs| {
            refs.iter()
                .filter_map(Value::as_str)
                .take(MAX_EVIDENCE_REFS_PER_FINDING)
                .map(compact_text)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    json!({
        "id": compact_text(finding["id"].as_str().unwrap_or("unknown")),
        "severity": finding["severity"].clone(),
        "category": finding["category"].clone(),
        "status": finding["status"].clone(),
        "failure_level": finding["failure_level"].clone(), "failure_class": finding["failure_class"].clone(), "root_frame": finding["root_frame"].clone(), "remediation_level": finding["remediation_level"].clone(), "review_depth": finding["review_depth"].clone(),
        "fingerprint": compact_text(finding["fingerprint"].as_str().unwrap_or("unknown")),
        "summary": compact_text(finding["summary"].as_str().unwrap_or("")),
        "recommended_action": compact_text(finding["recommended_action"].as_str().unwrap_or("")),
        "evidence_refs": evidence_refs,
        "evidence_ref_count": finding["evidence"].as_array().map(Vec::len).unwrap_or(0),
        "freshness": classify_finding_freshness(finding).to_json(),
        "owner_guess": owner_guess(finding),
        "root_cause_hypothesis": root_cause_hypothesis(finding),
    })
}

struct FindingQuality {
    release_ready: bool,
    missing_requirements: Vec<&'static str>,
    owner_guess: String,
    root_cause_hypothesis: String,
    recurrence_or_freshness_support: bool, current_truth_freshness_window: bool,
    stale_do_not_use: bool, freshness: Value,
    concrete_next_action: bool,
}

impl FindingQuality {
    fn to_json(&self, state: &str) -> Value {
        json!({
            "actionability_state": state,
            "release_ready": self.release_ready,
            "missing_requirements": self.missing_requirements,
            "owner_guess": self.owner_guess,
            "root_cause_hypothesis": self.root_cause_hypothesis,
            "recurrence_or_freshness_support": self.recurrence_or_freshness_support,
            "current_truth_freshness_window": self.current_truth_freshness_window,
            "stale_do_not_use": self.stale_do_not_use,
            "freshness": self.freshness,
            "concrete_next_action": self.concrete_next_action,
        })
    }
}

fn triage_finding(finding: &Value, quality: &FindingQuality) -> Value {
    json!({
        "id": compact_text(finding["id"].as_str().unwrap_or("unknown")),
        "severity": finding["severity"].clone(),
        "category": finding["category"].clone(),
        "status": finding["status"].clone(), "failure_level": finding["failure_level"].clone(), "failure_class": finding["failure_class"].clone(), "remediation_level": finding["remediation_level"].clone(), "review_depth": finding["review_depth"].clone(),
        "fingerprint": compact_text(finding["fingerprint"].as_str().unwrap_or("unknown")),
        "summary": compact_text(finding["summary"].as_str().unwrap_or("")),
        "actionability_state": if quality.stale_do_not_use { "stale_do_not_use" } else { "needs_triage" },
        "quality": quality.to_json("needs_triage"),
    })
}

fn finding_quality(finding: &Value) -> FindingQuality {
    let evidence_refs = finding["evidence"]
        .as_array()
        .map(|refs| refs.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();
    let has_evidence = !evidence_refs.is_empty();
    let freshness = classify_finding_freshness(finding);
    let recurrence_or_freshness_support = freshness.current_truth
        || evidence_refs.iter().any(|reference| trusted_evidence_ref(reference));
    let owner_guess = owner_guess(finding);
    let root_cause_hypothesis = root_cause_hypothesis(finding);
    let concrete_next_action =
        concrete_next_action(finding["recommended_action"].as_str().unwrap_or(""));
    let status = finding["status"].as_str().unwrap_or("").to_ascii_lowercase();
    let mut missing_requirements = Vec::new();
    if !has_evidence { missing_requirements.push("evidence"); }
    if !recurrence_or_freshness_support { missing_requirements.push("recurrence_or_freshness_support"); }
    if freshness.stale_do_not_use {
        missing_requirements.push("stale_do_not_use");
    } else if !freshness.current_truth {
        missing_requirements.push("current_truth_freshness_window");
    }
    if owner_guess.is_empty() { missing_requirements.push("owner_guess"); }
    if root_cause_hypothesis.is_empty() { missing_requirements.push("root_cause_hypothesis"); }
    if !concrete_next_action { missing_requirements.push("concrete_next_action"); }
    if matches!(
        status.as_str(),
        "draft" | "triage" | "needs_root_cause_synthesis" | "stale_do_not_use"
    ) {
        missing_requirements.push("release_status");
    }
    FindingQuality {
        release_ready: missing_requirements.is_empty(),
        missing_requirements,
        owner_guess,
        root_cause_hypothesis,
        recurrence_or_freshness_support,
        current_truth_freshness_window: freshness.current_truth,
        stale_do_not_use: freshness.stale_do_not_use,
        freshness: freshness.to_json(),
        concrete_next_action,
    }
}

fn owner_guess(finding: &Value) -> String {
    match finding["category"].as_str().unwrap_or("") {
        "receipt_integrity" | "capability_enforcement" | "state_transition" | "security_boundary"
        | "runtime_correctness" | "self_maintenance_loop" => "kernel".to_string(),
        "gateway_isolation" => "gateways".to_string(),
        "queue_backpressure" | "retry_storm" | "boundedness" | "performance_regression" => {
            "observability".to_string()
        }
        "release_evidence" => "validation".to_string(),
        "automation_candidate" => "governance".to_string(),
        _ => String::new(),
    }
}

fn root_cause_hypothesis(finding: &Value) -> String {
    let root_frame = finding["root_frame"].as_str().unwrap_or("");
    let fingerprint = finding["fingerprint"].as_str().unwrap_or("");
    if root_frame.is_empty() || fingerprint.is_empty() {
        return String::new();
    }
    compact_text(&format!("{root_frame} via {fingerprint}"))
}

fn trusted_evidence_ref(reference: &str) -> bool {
    [
        "artifact://",
        "evidence://",
        "fixture://",
        "guard://",
        "health://",
        "local://",
        "path://",
        "receipt://",
        "release://",
        "trace://",
    ]
    .iter()
    .any(|prefix| reference.starts_with(prefix))
}

fn concrete_next_action(action: &str) -> bool {
    let words = action.split_whitespace().count();
    action.trim().chars().count() >= 16 && words >= 4
}

fn compact_text(input: &str) -> String {
    let mut output = input.chars().take(MAX_TEXT_CHARS).collect::<String>();
    if input.chars().count() > MAX_TEXT_CHARS {
        output.push_str("...");
    }
    output
}

fn evidence_stream_refs(state_dir: &Path) -> Vec<String> {
    vec![
        state_dir.join("evidence/kernel_receipts.jsonl").display().to_string(),
        state_dir.join("evidence/runtime_observations.jsonl").display().to_string(),
        state_dir.join("evidence/control_plane_eval.jsonl").display().to_string(),
        state_dir.join("evidence/release_proof_packs.jsonl").display().to_string(),
        state_dir.join("evidence/gateway_health.jsonl").display().to_string(),
        state_dir.join("evidence/queue_backpressure.jsonl").display().to_string(),
    ]
}

fn serialized_len(value: &Value) -> usize {
    serde_json::to_vec(value).map(|body| body.len()).unwrap_or(usize::MAX)
}
