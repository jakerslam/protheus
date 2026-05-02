// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    causal_hypothesis::root_cause_hypothesis_text,
    diagnostic_run_artifact::attach_diagnostic_context_to_issue_draft,
    incident_report::violated_invariants,
    kernel_sentinel_semantic_frame_for_finding,
    issue_cluster_semantics::{
        cluster_fields, issue_cluster_key, issue_family_fingerprint, issue_family_kind,
        issue_summary, issue_title, severity_rank, synthetic_issue_scenario_id, FindingCluster,
    },
    KernelSentinelFinding,
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
    let component = issue_component(finding, cluster);
    let root_cause_hypothesis = issue_root_cause_hypothesis(finding, cluster, &semantic_frame);
    let repair_type = semantic_frame["remediation_level"]
        .as_str()
        .unwrap_or("targeted_repair")
        .to_string();
    let validation_route = issue_validation_route(cluster, &semantic_frame);
    let acceptance_criteria = issue_acceptance_criteria(&validation_route);
    let recommended_fix =
        issue_recommended_fix(finding, cluster, &semantic_frame, &component, &validation_route);
    let anti_patching = anti_patching_assessment(cluster);
    json!({
        "type": "kernel_sentinel_issue_draft",
        "status": "draft",
        "title": issue_title(finding),
        "summary": summary,
        "component": component,
        "observed_failure": finding.summary,
        "root_cause_hypothesis": root_cause_hypothesis,
        "repair_type": repair_type,
        "validation_route": validation_route,
        "severity": finding.severity,
        "category": finding.category,
        "failure_level": semantic_frame["failure_level"].clone(),
        "root_frame": semantic_frame["root_frame"].clone(),
        "remediation_level": semantic_frame["remediation_level"].clone(),
        "anti_patching": anti_patching,
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
        "recommended_fix": recommended_fix,
        "acceptance_criteria": acceptance_criteria,
        "todo_actionability": {
            "todo_ready": true,
            "component_present": true,
            "observed_failure_present": true,
            "root_cause_hypothesis_present": true,
            "repair_type_present": true,
            "validation_route_present": true,
            "evidence_present": !evidence.is_empty(),
            "human_review_required": true,
            "safe_to_mutate_todo": false
        }
    })
}

fn anti_patching_assessment(cluster: &FindingCluster) -> Value {
    let distinct_symptom_fingerprint_count = cluster.issue_family_fingerprints.len();
    let loop_detected =
        distinct_symptom_fingerprint_count > 1 || cluster.symptom_patch_signal_count > 0;
    json!({
        "symptom_patching_loop_detected": loop_detected,
        "structural_root_required": loop_detected,
        "distinct_symptom_fingerprint_count": distinct_symptom_fingerprint_count,
        "symptom_patch_signal_count": cluster.symptom_patch_signal_count,
        "policy": "multiple visible symptoms under one structural root must collapse into one root-cause repair before opening separate local tickets",
        "next_action": if loop_detected {
            "stop_local_symptom_patching_and_repair_structural_root"
        } else {
            "continue_standard_issue_triage"
        }
    })
}

fn symptom_patch_signal(finding: &KernelSentinelFinding) -> bool {
    let text = format!(
        "{} {} {}",
        finding.fingerprint, finding.summary, finding.recommended_action
    )
    .to_ascii_lowercase();
    text.contains("stop_patching")
        || text.contains("symptom patch")
        || text.contains("patching symptoms")
        || text.contains("cosmetic fix")
        || text.contains("visible symptom")
        || text.contains("local patch")
}

fn issue_component(finding: &KernelSentinelFinding, cluster: &FindingCluster) -> String {
    for evidence in &cluster.evidence {
        if let Some(component) = evidence_component(evidence) {
            return component;
        }
    }
    for candidate in [&cluster.surface, &cluster.receipt_type, &cluster.session] {
        if !candidate.trim().is_empty() && candidate != "unknown" && candidate != "none" {
            return candidate.to_string();
        }
    }
    format!("{:?}", finding.category)
}

fn evidence_component(reference: &str) -> Option<String> {
    let after_scheme = reference.split_once("://")?.1;
    let component = after_scheme
        .split(['/', ';', '?', '#'])
        .next()
        .unwrap_or("")
        .trim();
    (!component.is_empty() && component != "unknown").then(|| component.to_string())
}

fn issue_root_cause_hypothesis(
    finding: &KernelSentinelFinding,
    cluster: &FindingCluster,
    semantic_frame: &Value,
) -> String {
    let root_frame = semantic_frame["root_frame"].as_str().unwrap_or("unknown_root_frame");
    let invariant = cluster
        .violated_invariants
        .iter()
        .next()
        .map(String::as_str)
        .unwrap_or("unknown_invariant");
    let evidence_refs = cluster.evidence.iter().cloned().collect::<Vec<_>>();
    let causal = root_cause_hypothesis_text(finding, &evidence_refs, invariant, semantic_frame);
    format!("{root_frame} breach; {causal}")
}

fn issue_recommended_fix(
    finding: &KernelSentinelFinding,
    cluster: &FindingCluster,
    semantic_frame: &Value,
    component: &str,
    validation_route: &[Value],
) -> String {
    let root_frame = semantic_frame["root_frame"].as_str().unwrap_or("unknown_root_frame");
    let invariant = cluster
        .violated_invariants
        .iter()
        .next()
        .map(String::as_str)
        .unwrap_or("unknown_invariant");
    let validation_command = validation_route
        .first()
        .and_then(|route| route["command"].as_str())
        .unwrap_or("cargo test --manifest-path core/layer0/ops/Cargo.toml kernel_sentinel -- --nocapture");
    let upstream = finding.recommended_action.trim();
    let prefix = if upstream.is_empty()
        || upstream == "inspect deterministic kernel evidence and restore fail-closed behavior"
    {
        format!("Repair `{component}` at `{root_frame}`")
    } else {
        format!("{upstream}; repair `{component}` at `{root_frame}`")
    };
    format!(
        "{prefix} by resolving `{}` against invariant `{invariant}` for `{}`; then rerun `{validation_command}` and keep this draft open until the evidence stream stops emitting `{}`.",
        cluster.recovery_reason, finding.summary, finding.fingerprint
    )
}

fn issue_validation_route(cluster: &FindingCluster, semantic_frame: &Value) -> Vec<Value> {
    let root_frame = semantic_frame["root_frame"].as_str().unwrap_or("unknown_root_frame");
    vec![json!({
        "route": "kernel_sentinel_regression",
        "command": "cargo test --manifest-path core/layer0/ops/Cargo.toml kernel_sentinel -- --nocapture",
        "expected_result": "finding family absent or explicitly waived by reviewed evidence",
        "cluster_key": cluster.cluster_key,
        "root_frame": root_frame
    })]
}

fn issue_acceptance_criteria(validation_route: &[Value]) -> Vec<String> {
    let validation_command = validation_route
        .first()
        .and_then(|route| route["command"].as_str())
        .unwrap_or("cargo test --manifest-path core/layer0/ops/Cargo.toml kernel_sentinel -- --nocapture");
    vec![
        "deterministic evidence no longer emits this fingerprint".to_string(),
        "strict Kernel Sentinel report returns allow for this scenario".to_string(),
        "regression fixture covers the failure signature".to_string(),
        format!("validation route passes: {validation_command}"),
    ]
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
        let validation_route_count = draft
            .get("validation_route")
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
        for (key, reason, min_len) in [
            ("component", "missing_component", 3usize),
            ("observed_failure", "missing_observed_failure", 16usize),
            ("root_cause_hypothesis", "missing_root_cause_hypothesis", 24usize),
            ("repair_type", "missing_repair_type", 3usize),
        ] {
            if draft.get(key).and_then(Value::as_str).unwrap_or("").trim().len() < min_len {
                reasons.push(reason);
            }
        }
        if validation_route_count == 0 {
            reasons.push("missing_validation_route");
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
        let current_issue_family_fingerprint = issue_family_fingerprint.clone();
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
                issue_family_fingerprints: BTreeSet::new(),
                symptom_patch_signal_count: 0,
            });
        entry.occurrence_count += 1;
        entry.last_seen_index = index;
        if severity_rank(finding.severity) < severity_rank(entry.exemplar.severity) {
            entry.exemplar = finding.clone();
        }
        entry.evidence.extend(finding.evidence.iter().cloned());
        entry
            .issue_family_fingerprints
            .insert(current_issue_family_fingerprint);
        if symptom_patch_signal(finding) {
            entry.symptom_patch_signal_count += 1;
        }
    }
    let issue_drafts = clusters
        .values()
        .filter(|cluster| cluster.occurrence_count >= threshold)
        .map(issue_draft)
        .collect::<Vec<_>>();
    let issue_quality_failures = issue_quality_failures(&issue_drafts);
    let active_issue_window_count = issue_drafts.len();
    let anti_patching_loop_count = issue_drafts
        .iter()
        .filter(|draft| {
            draft["anti_patching"]["symptom_patching_loop_detected"]
                .as_bool()
                .unwrap_or(false)
        })
        .count();
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
        "anti_patching_loop_count": anti_patching_loop_count,
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
