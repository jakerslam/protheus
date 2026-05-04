// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::system_understanding_dossier::{
    validate_system_understanding_dossier, SystemUnderstandingCapabilityKind,
    SystemUnderstandingCapabilityRow, SystemUnderstandingCapabilityValue,
    SystemUnderstandingDossier, SystemUnderstandingDossierStatus,
    SystemUnderstandingDossierTargetMode, SystemUnderstandingImplementationItem,
    SystemUnderstandingTransferTarget, SYSTEM_UNDERSTANDING_DOSSIER_SCHEMA_VERSION,
};
use serde_json::Value;
use std::path::Path;

const MIN_RUNTIME_CONFIDENCE_FOR_STRUCTURAL_RECOMMENDATIONS: f64 = 0.70;
const MIN_AUTHORITY_CONFIDENCE_FOR_STRUCTURAL_RECOMMENDATIONS: f64 = 0.80;
const MIN_ARCHITECTURE_CONFIDENCE_FOR_STRUCTURAL_RECOMMENDATIONS: f64 = 0.70;
const MIN_CAPABILITY_CONFIDENCE_FOR_STRUCTURAL_RECOMMENDATIONS: f64 = 0.70;
const MIN_TRANSFER_CONFIDENCE_FOR_STRUCTURAL_RECOMMENDATIONS: f64 = 0.80;

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| row.as_str().map(str::to_string))
        .filter(|row| !row.trim().is_empty())
        .collect()
}

fn confidence_with_unknown_penalty(
    base: f64,
    evidence_count: usize,
    unknown_count: usize,
    max_bonus: f64,
    unknown_penalty: f64,
) -> f64 {
    let evidence_bonus = (evidence_count.min(6) as f64) * max_bonus;
    let penalty = (unknown_count as f64) * unknown_penalty;
    (base + evidence_bonus - penalty).clamp(0.0, 1.0)
}

fn push_unique(rows: &mut Vec<String>, value: &str) {
    if !rows.iter().any(|row| row == value) {
        rows.push(value.to_string());
    }
}

fn structural_recommendation_probe_requirements(
    runtime_confidence: f64,
    authority_confidence: f64,
    architecture_confidence: f64,
    capability_confidence: f64,
    transfer_confidence: f64,
) -> Vec<String> {
    let mut probes = Vec::new();
    if runtime_confidence < MIN_RUNTIME_CONFIDENCE_FOR_STRUCTURAL_RECOMMENDATIONS {
        probes.push("raise_runtime_dossier_confidence".to_string());
    }
    if authority_confidence < MIN_AUTHORITY_CONFIDENCE_FOR_STRUCTURAL_RECOMMENDATIONS {
        probes.push("raise_authority_dossier_confidence".to_string());
    }
    if architecture_confidence < MIN_ARCHITECTURE_CONFIDENCE_FOR_STRUCTURAL_RECOMMENDATIONS {
        probes.push("raise_architecture_dossier_confidence".to_string());
    }
    if capability_confidence < MIN_CAPABILITY_CONFIDENCE_FOR_STRUCTURAL_RECOMMENDATIONS {
        probes.push("raise_capability_dossier_confidence".to_string());
    }
    if transfer_confidence < MIN_TRANSFER_CONFIDENCE_FOR_STRUCTURAL_RECOMMENDATIONS {
        probes.push("raise_transfer_dossier_confidence".to_string());
    }
    probes
}

pub fn build_infring_self_dossier(
    root: &Path,
    report: &Value,
    verdict: &Value,
    self_study_outputs: &Value,
    diagnostic_run: &Value,
) -> Result<Value, String> {
    let report_path = "local/state/kernel_sentinel/kernel_sentinel_report_current.json".to_string();
    let verdict_path = "local/state/kernel_sentinel/kernel_sentinel_verdict.json".to_string();
    let architectural_path =
        "local/state/kernel_sentinel/architectural_incident_report_current.json".to_string();
    let health_path = "local/state/kernel_sentinel/kernel_sentinel_health_current.json".to_string();
    let diagnostic_run_path =
        "local/state/kernel_sentinel/kernel_sentinel_diagnostic_run_current.json".to_string();
    let issues_path = "local/state/kernel_sentinel/issues.jsonl".to_string();
    let auto_dossier_path = root
        .join("local/state/system_understanding/infring_dossier.json")
        .display()
        .to_string();
    let top_holes_path = self_study_outputs["top_system_holes_path"]
        .as_str()
        .unwrap_or("local/state/kernel_sentinel/top_system_holes_current.json")
        .to_string();
    let readiness_path = self_study_outputs["rsi_readiness_path"]
        .as_str()
        .unwrap_or("local/state/kernel_sentinel/rsi_readiness_summary_current.json")
        .to_string();
    let trend_path = self_study_outputs["trend_report_path"]
        .as_str()
        .unwrap_or("local/state/kernel_sentinel/sentinel_trend_report_current.json")
        .to_string();
    let feedback_path = self_study_outputs["feedback_inbox_path"]
        .as_str()
        .unwrap_or("local/state/kernel_sentinel/feedback_inbox.jsonl")
        .to_string();
    let daily_report_path = self_study_outputs["daily_report_path"]
        .as_str()
        .unwrap_or("local/state/kernel_sentinel/daily_report.md")
        .to_string();
    let generated_at = report["generated_at"]
        .as_str()
        .filter(|row| !row.trim().is_empty())
        .unwrap_or("unknown")
        .to_string();
    let release_gate_pass = report["operator_summary"]["release_gate_pass"]
        .as_bool()
        .unwrap_or(false);
    let scheduler_stale = report["operator_summary"]["scheduler_stale"]
        .as_bool()
        .unwrap_or(true);
    let missing_required_source_count = report["operator_summary"]["missing_required_source_count"]
        .as_u64()
        .unwrap_or(0);
    let trend_history_runs = self_study_outputs["trend_history_runs"]
        .as_u64()
        .unwrap_or(0);
    let diagnostic_follow_up_request_count = diagnostic_run["diagnostic_follow_up_request_count"]
        .as_u64()
        .unwrap_or(0);
    let authorized_probe_count = diagnostic_run["authorized_probe_count"]
        .as_u64()
        .unwrap_or(0);
    let incidents = report["architectural_incident_report"]["incidents"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut known_failure_modes = Vec::new();
    let mut violated_invariants = Vec::new();
    let mut stop_patching_triggers = Vec::new();
    let mut runtime_architecture_mismatches = Vec::new();
    for incident in &incidents {
        if let Some(summary) = incident["summary"]
            .as_str()
            .filter(|row| !row.trim().is_empty())
        {
            known_failure_modes.push(summary.to_string());
            if incident["stop_patching"].as_bool().unwrap_or(false) {
                stop_patching_triggers.push(summary.to_string());
            }
            if incident["multi_layer"].as_bool().unwrap_or(false) {
                runtime_architecture_mismatches.push(summary.to_string());
            }
        }
        violated_invariants.extend(strings(&incident["violated_invariants"]));
    }
    let release_blockers = strings(&report["operator_summary"]["release_blockers"]);
    let mut blocking_unknowns = Vec::new();
    let mut required_next_probes = Vec::new();
    if missing_required_source_count > 0 {
        blocking_unknowns.push(format!(
            "missing_required_sources:{missing_required_source_count}"
        ));
        required_next_probes.push("fill_missing_required_sentinel_sources".to_string());
    }
    if scheduler_stale {
        blocking_unknowns.push("scheduler_state_stale".to_string());
        required_next_probes.push("refresh_scheduler_runtime_evidence".to_string());
    }
    if trend_history_runs < 3 {
        blocking_unknowns.push("trend_history_insufficient".to_string());
        required_next_probes.push("accumulate_three_kernel_sentinel_trend_runs".to_string());
    }
    let soul_evidence = vec![
        "receipt-first deterministic runtime".to_string(),
        "resident-ipc-only production topology".to_string(),
        "kernel authority with orchestration as non-canonical coordination".to_string(),
    ];
    let runtime_unknowns = if scheduler_stale {
        vec!["fresh scheduler lifecycle evidence unavailable".to_string()]
    } else {
        Vec::new()
    };
    let runtime_evidence = vec![
        report_path.clone(),
        health_path.clone(),
        diagnostic_run_path.clone(),
        readiness_path.clone(),
        trend_path.clone(),
        format!("diagnostic_follow_up_request_count:{diagnostic_follow_up_request_count}"),
        format!(
            "scheduler_status:{}",
            report["operator_summary"]["scheduler_status"]
                .as_str()
                .unwrap_or("unknown")
        ),
        format!(
            "observation_state:{}",
            report["operator_summary"]["observation_state"]
                .as_str()
                .unwrap_or("unknown")
        ),
    ];
    let ecology_evidence = vec![
        "gateway health and quarantine evidence streams".to_string(),
        "release proof-pack and boundedness artifact inputs".to_string(),
        "control-plane eval and queue backpressure collectors".to_string(),
        feedback_path.clone(),
        top_holes_path.clone(),
    ];
    let authority_evidence = vec![
        verdict_path.clone(),
        architectural_path.clone(),
        diagnostic_run_path.clone(),
        issues_path.clone(),
        format!(
            "release_gate_pass:{}",
            report["operator_summary"]["release_gate_pass"]
                .as_bool()
                .unwrap_or(false)
        ),
        format!("authorized_probe_count:{authorized_probe_count}"),
        format!(
            "verdict:{}",
            verdict["verdict"].as_str().unwrap_or("unknown")
        ),
    ];
    let architecture_evidence = vec![
        architectural_path.clone(),
        report_path.clone(),
        "core/layer0/ops/src/kernel_sentinel.rs".to_string(),
        "core/layer0/ops/src/kernel_sentinel/self_study.rs".to_string(),
        "core/layer0/ops/src/kernel_sentinel/governance.rs".to_string(),
    ];
    let capabilities = vec![
        SystemUnderstandingCapabilityRow {
            id: "kernel_runtime_truth_loop".to_string(),
            kind: SystemUnderstandingCapabilityKind::Evidence,
            value: SystemUnderstandingCapabilityValue::Critical,
            evidence: vec![
                "local/state/kernel_sentinel/kernel_sentinel_report_current.json".to_string(),
            ],
            runtime_proof: vec!["kernel_sentinel_report_current.json".to_string()],
            transfer_target: SystemUnderstandingTransferTarget::Kernel,
            fit_rationale: "Kernel Sentinel already owns deterministic runtime evidence, verdicting, and release-blocking truth.".to_string(),
        },
        SystemUnderstandingCapabilityRow {
            id: "architectural_incident_synthesis".to_string(),
            kind: SystemUnderstandingCapabilityKind::Architecture,
            value: SystemUnderstandingCapabilityValue::High,
            evidence: vec![
                "local/state/kernel_sentinel/architectural_incident_report_current.json"
                    .to_string(),
            ],
            runtime_proof: vec!["architectural_incident_report_current.json".to_string()],
            transfer_target: SystemUnderstandingTransferTarget::Kernel,
            fit_rationale: "Architectural synthesis converts raw failure clusters into invariant-level incidents instead of symptom noise.".to_string(),
        },
        SystemUnderstandingCapabilityRow {
            id: "self_study_issue_governance".to_string(),
            kind: SystemUnderstandingCapabilityKind::Policy,
            value: SystemUnderstandingCapabilityValue::High,
            evidence: vec![
                "local/state/kernel_sentinel/rsi_readiness_summary_current.json".to_string(),
            ],
            runtime_proof: vec!["rsi_readiness_summary_current.json".to_string()],
            transfer_target: SystemUnderstandingTransferTarget::Kernel,
            fit_rationale: "Issue generation is proposal-only and evidence-gated, which matches InfRing's fail-closed self-improvement posture.".to_string(),
        },
    ];
    let rejected_capabilities = vec![SystemUnderstandingCapabilityRow {
        id: "shell_truth_authority".to_string(),
        kind: SystemUnderstandingCapabilityKind::Ux,
        value: SystemUnderstandingCapabilityValue::Critical,
        evidence: vec!["docs/workspace/shell_ui_projection_policy.md".to_string()],
        runtime_proof: Vec::new(),
        transfer_target: SystemUnderstandingTransferTarget::Reject,
        fit_rationale: "Shell-owned truth or retry authority violates InfRing's authority boundary and must remain rejected.".to_string(),
    }];
    let proposed_implementation_items = vec![
        SystemUnderstandingImplementationItem {
            id: "ks-maintain-dossier-freshness".to_string(),
            summary: "Refresh the self-dossier whenever Sentinel auto-run emits new runtime truth.".to_string(),
            owner_layer: "core/layer0/ops".to_string(),
            invariant: "system understanding must track current kernel runtime evidence".to_string(),
            proof_requirement: "auto-run regression must validate dossier emission and schema integrity".to_string(),
            rollback_plan: "remove dossier consumers before relaxing the emitter contract".to_string(),
        },
        SystemUnderstandingImplementationItem {
            id: "ks-route-findings-through-architectural-synthesis".to_string(),
            summary: "Keep architectural incident synthesis ahead of issue filing so failures stay invariant-shaped.".to_string(),
            owner_layer: "core/layer0/ops".to_string(),
            invariant: "issue candidates must come from synthesized kernel evidence, not unsynthesized shell symptoms".to_string(),
            proof_requirement: "architectural incident artifact and release gate must stay in sync".to_string(),
            rollback_plan: "block release filing before allowing raw findings to bypass synthesis".to_string(),
        },
    ];
    let syntax_evidence = vec![
        "core/layer0/ops/src/kernel_sentinel/auto_run.rs".to_string(),
        "core/layer0/ops/src/kernel_sentinel/system_understanding_dossier.rs".to_string(),
        "core/layer0/ops/src/kernel_sentinel/self_dossier.rs".to_string(),
    ];
    let soul_confidence = confidence_with_unknown_penalty(0.66, soul_evidence.len(), 0, 0.04, 0.06);
    let runtime_confidence = confidence_with_unknown_penalty(
        if missing_required_source_count == 0 {
            0.62
        } else {
            0.44
        },
        runtime_evidence.len(),
        runtime_unknowns.len() + required_next_probes.len(),
        0.035,
        0.07,
    );
    let ecology_confidence =
        confidence_with_unknown_penalty(0.58, ecology_evidence.len(), 0, 0.03, 0.05);
    let authority_confidence = confidence_with_unknown_penalty(
        if release_gate_pass { 0.70 } else { 0.52 },
        authority_evidence.len(),
        release_blockers.len(),
        0.03,
        0.05,
    );
    let architecture_confidence = confidence_with_unknown_penalty(
        if scheduler_stale { 0.50 } else { 0.64 },
        architecture_evidence.len(),
        runtime_architecture_mismatches.len(),
        0.03,
        0.04,
    );
    let capability_confidence = confidence_with_unknown_penalty(
        0.67,
        capabilities.len() + rejected_capabilities.len(),
        0,
        0.035,
        0.05,
    );
    let failure_model_confidence = confidence_with_unknown_penalty(
        0.56,
        known_failure_modes.len() + violated_invariants.len() + stop_patching_triggers.len(),
        0,
        0.025,
        0.05,
    );
    let transfer_confidence = confidence_with_unknown_penalty(
        0.74,
        proposed_implementation_items.len(),
        usize::from(missing_required_source_count > 0) + usize::from(scheduler_stale),
        0.04,
        0.03,
    );
    let structural_recommendation_probes = structural_recommendation_probe_requirements(
        runtime_confidence,
        authority_confidence,
        architecture_confidence,
        capability_confidence,
        transfer_confidence,
    );
    for probe in &structural_recommendation_probes {
        push_unique(&mut required_next_probes, probe);
    }
    if !structural_recommendation_probes.is_empty() {
        push_unique(
            &mut blocking_unknowns,
            "structural_recommendations_blocked_until_dossier_confidence_recovers",
        );
    }
    let implementation_items = if structural_recommendation_probes.is_empty() {
        proposed_implementation_items
    } else {
        Vec::new()
    };
    let implementation_confidence = confidence_with_unknown_penalty(
        if structural_recommendation_probes.is_empty() {
            0.61
        } else {
            0.38
        },
        implementation_items.len() + usize::from(structural_recommendation_probes.is_empty()),
        structural_recommendation_probes.len(),
        0.03,
        0.05,
    );
    let syntax_confidence =
        confidence_with_unknown_penalty(0.54, syntax_evidence.len(), 0, 0.04, 0.05);
    let confidence_overall = (soul_confidence
        + runtime_confidence
        + ecology_confidence
        + authority_confidence
        + architecture_confidence
        + capability_confidence
        + failure_model_confidence
        + transfer_confidence
        + implementation_confidence
        + syntax_confidence)
        / 10.0;
    let status = if blocking_unknowns.is_empty() {
        SystemUnderstandingDossierStatus::Usable
    } else {
        SystemUnderstandingDossierStatus::Draft
    };
    let dossier = SystemUnderstandingDossier {
        dossier_id: "infring".to_string(),
        target_mode: SystemUnderstandingDossierTargetMode::InternalRsi,
        target_system: "InfRing".to_string(),
        target_version_or_revision: format!(
            "kernel-sentinel-contract-v{}",
            report["contract_version"].as_u64().unwrap_or(1)
        ),
        dossier_version: SYSTEM_UNDERSTANDING_DOSSIER_SCHEMA_VERSION,
        created_at: generated_at,
        updated_at: crate::now_iso(),
        owners: vec![
            "kernel-sentinel".to_string(),
            "assimilation".to_string(),
            "rsi".to_string(),
        ],
        status,
        confidence_overall,
        blocking_unknowns,
        evidence_index: vec![
            report_path.clone(),
            verdict_path.clone(),
            architectural_path.clone(),
            health_path.clone(),
            top_holes_path.clone(),
            readiness_path.clone(),
            trend_path.clone(),
            feedback_path.clone(),
            daily_report_path.clone(),
            issues_path.clone(),
            diagnostic_run_path.clone(),
            auto_dossier_path.clone(),
        ],
        soul_confidence,
        soul_evidence,
        soul_unknowns: Vec::new(),
        runtime_confidence,
        runtime_evidence,
        runtime_unknowns,
        required_next_probes,
        ecology_confidence,
        ecology_evidence,
        ecology_unknowns: Vec::new(),
        authority_confidence,
        authority_evidence,
        authority_unknowns: Vec::new(),
        authority_risks: release_blockers,
        architecture_confidence,
        architecture_evidence,
        architecture_unknowns: Vec::new(),
        runtime_architecture_mismatches,
        capability_confidence,
        capabilities,
        rejected_capabilities,
        capability_unknowns: Vec::new(),
        failure_model_confidence,
        known_failure_modes,
        violated_invariants,
        stop_patching_triggers,
        transfer_confidence,
        implementation_items,
        proof_requirements: vec![
            format!("{report_path} must exist"),
            format!("{architectural_path} must exist"),
            format!("{readiness_path} must exist"),
            format!(
                "kernel_sentinel_verdict must remain {}",
                verdict["verdict"].as_str().unwrap_or("unknown")
            ),
        ],
        rollback_plan: vec![
            "keep dossier proposal-only until downstream consumers are proven stable".to_string(),
            "treat missing dossier artifacts as emitter regressions rather than shell-facing runtime truth".to_string(),
        ],
        implementation_confidence,
        files_inspected: vec![
            "core/layer0/ops/src/kernel_sentinel.rs".to_string(),
            "core/layer0/ops/src/kernel_sentinel/auto_run.rs".to_string(),
            "core/layer0/ops/src/kernel_sentinel/self_study.rs".to_string(),
            "core/layer0/ops/src/kernel_sentinel/governance.rs".to_string(),
            "core/layer0/ops/src/kernel_sentinel/system_understanding_dossier.rs".to_string(),
        ],
        implementation_unknowns: Vec::new(),
        syntax_confidence,
        syntax_evidence,
        syntax_unknowns: Vec::new(),
    };
    validate_system_understanding_dossier(&dossier)?;
    serde_json::to_value(dossier).map_err(|err| err.to_string())
}
