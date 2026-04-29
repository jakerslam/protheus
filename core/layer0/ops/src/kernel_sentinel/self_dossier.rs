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

fn strings(value: &Value) -> Vec<String> {
    value.as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| row.as_str().map(str::to_string))
        .filter(|row| !row.trim().is_empty())
        .collect()
}

pub fn build_infring_self_dossier(
    root: &Path,
    report: &Value,
    verdict: &Value,
    self_study_outputs: &Value,
) -> Result<Value, String> {
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
    let trend_history_runs = self_study_outputs["trend_history_runs"].as_u64().unwrap_or(0);
    let runtime_confidence = if missing_required_source_count == 0 { 0.86 } else { 0.64 };
    let architecture_confidence = if scheduler_stale { 0.69 } else { 0.82 };
    let authority_confidence = if release_gate_pass { 0.91 } else { 0.76 };
    let confidence_overall =
        (runtime_confidence + architecture_confidence + authority_confidence) / 3.0;
    let incidents = report["architectural_incident_report"]["incidents"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut known_failure_modes = Vec::new();
    let mut violated_invariants = Vec::new();
    let mut stop_patching_triggers = Vec::new();
    let mut runtime_architecture_mismatches = Vec::new();
    for incident in &incidents {
        if let Some(summary) = incident["summary"].as_str().filter(|row| !row.trim().is_empty()) {
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
            "local/state/kernel_sentinel/kernel_sentinel_report_current.json".to_string(),
            "local/state/kernel_sentinel/kernel_sentinel_verdict.json".to_string(),
            "local/state/kernel_sentinel/architectural_incident_report_current.json".to_string(),
            "local/state/kernel_sentinel/rsi_readiness_summary_current.json".to_string(),
            "local/state/kernel_sentinel/top_system_holes_current.json".to_string(),
            root.join("local/state/system_understanding/infring_dossier.json")
                .display()
                .to_string(),
        ],
        soul_confidence: 0.78,
        soul_evidence: vec![
            "receipt-first deterministic runtime".to_string(),
            "resident-ipc-only production topology".to_string(),
            "kernel authority with orchestration as non-canonical coordination".to_string(),
        ],
        soul_unknowns: Vec::new(),
        runtime_confidence,
        runtime_evidence: vec![
            "local/state/kernel_sentinel/kernel_sentinel_report_current.json".to_string(),
            "local/state/kernel_sentinel/kernel_sentinel_health_current.json".to_string(),
            "local/state/kernel_sentinel/rsi_readiness_summary_current.json".to_string(),
        ],
        runtime_unknowns: if scheduler_stale {
            vec!["fresh scheduler lifecycle evidence unavailable".to_string()]
        } else {
            Vec::new()
        },
        required_next_probes,
        ecology_confidence: 0.74,
        ecology_evidence: vec![
            "gateway health and quarantine evidence streams".to_string(),
            "release proof-pack and boundedness artifact inputs".to_string(),
            "control-plane eval and queue backpressure collectors".to_string(),
        ],
        ecology_unknowns: Vec::new(),
        authority_confidence,
        authority_evidence: vec![
            "local/state/kernel_sentinel/kernel_sentinel_verdict.json".to_string(),
            "local/state/kernel_sentinel/architectural_incident_report_current.json".to_string(),
            "local/state/kernel_sentinel/issues.jsonl".to_string(),
        ],
        authority_unknowns: Vec::new(),
        authority_risks: release_blockers,
        architecture_confidence,
        architecture_evidence: vec![
            "core/layer0/ops/src/kernel_sentinel.rs".to_string(),
            "core/layer0/ops/src/kernel_sentinel/self_study.rs".to_string(),
            "core/layer0/ops/src/kernel_sentinel/governance.rs".to_string(),
        ],
        architecture_unknowns: Vec::new(),
        runtime_architecture_mismatches,
        capability_confidence: 0.84,
        capabilities: vec![
            SystemUnderstandingCapabilityRow {
                id: "kernel_runtime_truth_loop".to_string(),
                kind: SystemUnderstandingCapabilityKind::Evidence,
                value: SystemUnderstandingCapabilityValue::Critical,
                evidence: vec![
                    "local/state/kernel_sentinel/kernel_sentinel_report_current.json"
                        .to_string(),
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
                    "local/state/kernel_sentinel/rsi_readiness_summary_current.json"
                        .to_string(),
                ],
                runtime_proof: vec!["rsi_readiness_summary_current.json".to_string()],
                transfer_target: SystemUnderstandingTransferTarget::Kernel,
                fit_rationale: "Issue generation is proposal-only and evidence-gated, which matches InfRing's fail-closed self-improvement posture.".to_string(),
            },
        ],
        rejected_capabilities: vec![SystemUnderstandingCapabilityRow {
            id: "shell_truth_authority".to_string(),
            kind: SystemUnderstandingCapabilityKind::Ux,
            value: SystemUnderstandingCapabilityValue::Critical,
            evidence: vec!["docs/workspace/shell_ui_projection_policy.md".to_string()],
            runtime_proof: Vec::new(),
            transfer_target: SystemUnderstandingTransferTarget::Reject,
            fit_rationale: "Shell-owned truth or retry authority violates InfRing's authority boundary and must remain rejected.".to_string(),
        }],
        capability_unknowns: Vec::new(),
        failure_model_confidence: 0.87,
        known_failure_modes,
        violated_invariants,
        stop_patching_triggers,
        transfer_confidence: 0.82,
        implementation_items: vec![
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
        ],
        proof_requirements: vec![
            "kernel_sentinel_report_current.json must exist".to_string(),
            "architectural_incident_report_current.json must exist".to_string(),
            "rsi_readiness_summary_current.json must exist".to_string(),
            format!(
                "kernel_sentinel_verdict must remain {}",
                verdict["verdict"].as_str().unwrap_or("unknown")
            ),
        ],
        rollback_plan: vec![
            "keep dossier proposal-only until downstream consumers are proven stable".to_string(),
            "treat missing dossier artifacts as emitter regressions rather than shell-facing runtime truth".to_string(),
        ],
        implementation_confidence: 0.79,
        files_inspected: vec![
            "core/layer0/ops/src/kernel_sentinel.rs".to_string(),
            "core/layer0/ops/src/kernel_sentinel/auto_run.rs".to_string(),
            "core/layer0/ops/src/kernel_sentinel/self_study.rs".to_string(),
            "core/layer0/ops/src/kernel_sentinel/governance.rs".to_string(),
            "core/layer0/ops/src/kernel_sentinel/system_understanding_dossier.rs".to_string(),
        ],
        implementation_unknowns: Vec::new(),
        syntax_confidence: 0.66,
        syntax_evidence: vec![
            "core/layer0/ops/src/kernel_sentinel/auto_run.rs".to_string(),
            "core/layer0/ops/src/kernel_sentinel/system_understanding_dossier.rs".to_string(),
        ],
        syntax_unknowns: Vec::new(),
    };
    validate_system_understanding_dossier(&dossier)?;
    serde_json::to_value(dossier).map_err(|err| err.to_string())
}
