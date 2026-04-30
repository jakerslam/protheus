// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    SystemUnderstandingDossier, SystemUnderstandingDossierTargetMode,
    SystemUnderstandingTransferTarget,
};
use serde_json::{json, Value};
use std::collections::BTreeSet;

fn normalized_set(rows: &[String]) -> BTreeSet<String> {
    rows.iter()
        .map(|row| row.trim().to_ascii_lowercase())
        .filter(|row| !row.is_empty())
        .collect()
}

fn capability_id_set(dossier: &SystemUnderstandingDossier) -> BTreeSet<String> {
    dossier
        .capabilities
        .iter()
        .map(|row| row.id.clone())
        .collect::<BTreeSet<_>>()
}

fn rejected_id_set(dossier: &SystemUnderstandingDossier) -> BTreeSet<String> {
    dossier
        .rejected_capabilities
        .iter()
        .map(|row| row.id.clone())
        .collect::<BTreeSet<_>>()
}

fn ready_for_comparison(
    source_dossier: &SystemUnderstandingDossier,
    self_dossier: &SystemUnderstandingDossier,
) -> bool {
    matches!(
        source_dossier.target_mode,
        SystemUnderstandingDossierTargetMode::ExternalAssimilation
    ) && matches!(
        self_dossier.target_mode,
        SystemUnderstandingDossierTargetMode::InternalRsi
    ) && source_dossier.required_next_probes.is_empty()
        && self_dossier.required_next_probes.is_empty()
}

fn ordered_strings(rows: BTreeSet<String>) -> Vec<String> {
    rows.into_iter().collect()
}

fn transfer_ready_capability_ids(source_dossier: &SystemUnderstandingDossier) -> Vec<String> {
    source_dossier
        .capabilities
        .iter()
        .filter(|row| !matches!(row.transfer_target, SystemUnderstandingTransferTarget::Reject))
        .map(|row| row.id.clone())
        .collect()
}

fn soul_fit(source_dossier: &SystemUnderstandingDossier, self_dossier: &SystemUnderstandingDossier) -> Value {
    let source = normalized_set(&source_dossier.soul_evidence);
    let target = normalized_set(&self_dossier.soul_evidence);
    let shared = ordered_strings(source.intersection(&target).cloned().collect());
    let source_only = ordered_strings(source.difference(&target).cloned().collect());
    let target_only = ordered_strings(target.difference(&source).cloned().collect());
    let denom = source.len().max(target.len()).max(1) as f64;
    let score = (shared.len() as f64 / denom).clamp(0.0, 1.0);
    json!({
        "score": score,
        "posture": if score >= 0.66 { "aligned" } else if score >= 0.33 { "partial" } else { "misaligned" },
        "shared_evidence": shared,
        "source_only_evidence": source_only,
        "self_only_evidence": target_only
    })
}

fn authority_fit(
    source_dossier: &SystemUnderstandingDossier,
    self_dossier: &SystemUnderstandingDossier,
) -> Value {
    let shell_pressure = source_dossier
        .capabilities
        .iter()
        .filter(|row| matches!(row.transfer_target, SystemUnderstandingTransferTarget::Shell))
        .map(|row| row.id.clone())
        .collect::<Vec<_>>();
    let aligned = source_dossier
        .capabilities
        .iter()
        .filter(|row| {
            matches!(
                row.transfer_target,
                SystemUnderstandingTransferTarget::Kernel
                    | SystemUnderstandingTransferTarget::Orchestration
                    | SystemUnderstandingTransferTarget::Gateway
                    | SystemUnderstandingTransferTarget::WorkflowJson
                    | SystemUnderstandingTransferTarget::Docs
                    | SystemUnderstandingTransferTarget::Tests
            )
        })
        .count();
    let shared_rejections = ordered_strings(
        rejected_id_set(source_dossier)
            .intersection(&rejected_id_set(self_dossier))
            .cloned()
            .collect(),
    );
    let denominator = source_dossier.capabilities.len().max(1) as f64;
    let score = ((aligned as f64 - shell_pressure.len() as f64) / denominator).clamp(0.0, 1.0);
    let mut risks = source_dossier.authority_risks.clone();
    risks.extend(self_dossier.authority_risks.iter().cloned());
    json!({
        "score": score,
        "posture": if score >= 0.75 { "aligned" } else if score >= 0.40 { "mixed" } else { "misaligned" },
        "aligned_transfer_targets_count": aligned,
        "shell_authority_pressure_capability_ids": shell_pressure,
        "shared_rejected_capability_ids": shared_rejections,
        "risks": risks
    })
}

pub fn build_external_assimilation_dossier_comparison(
    source_dossier: &SystemUnderstandingDossier,
    self_dossier: &SystemUnderstandingDossier,
) -> Value {
    let comparison_ready = ready_for_comparison(source_dossier, self_dossier);
    let source_ids = capability_id_set(source_dossier);
    let self_ids = capability_id_set(self_dossier);
    let shared_ids = ordered_strings(source_ids.intersection(&self_ids).cloned().collect());
    let source_only_ids = ordered_strings(source_ids.difference(&self_ids).cloned().collect());
    let self_only_ids = ordered_strings(self_ids.difference(&source_ids).cloned().collect());
    let mut required_next_probes = source_dossier.required_next_probes.clone();
    for probe in &self_dossier.required_next_probes {
        if !required_next_probes.iter().any(|row| row == probe) {
            required_next_probes.push(probe.clone());
        }
    }
    let capabilities_missing_runtime_proof = source_dossier
        .capabilities
        .iter()
        .filter(|row| row.runtime_proof.is_empty())
        .map(|row| row.id.clone())
        .collect::<Vec<_>>();
    let soul_fit = soul_fit(source_dossier, self_dossier);
    let authority_fit = authority_fit(source_dossier, self_dossier);
    json!({
        "ok": comparison_ready,
        "type": "kernel_sentinel_external_assimilation_dossier_comparison",
        "mode": if comparison_ready { "comparison_ready" } else { "probe_first" },
        "source_dossier_id": source_dossier.dossier_id,
        "self_dossier_id": self_dossier.dossier_id,
        "source_system": source_dossier.target_system,
        "self_system": self_dossier.target_system,
        "capability_gap_analysis": {
            "shared_capability_ids": shared_ids,
            "source_only_capability_ids": source_only_ids,
            "self_only_capability_ids": self_only_ids,
            "transfer_ready_capability_ids": transfer_ready_capability_ids(source_dossier),
            "rejected_source_capability_ids": rejected_id_set(source_dossier).into_iter().collect::<Vec<_>>()
        },
        "soul_fit": soul_fit,
        "authority_fit": authority_fit,
        "proof_burden": {
            "required_next_probes": required_next_probes,
            "source_blocking_unknowns": source_dossier.blocking_unknowns,
            "self_blocking_unknowns": self_dossier.blocking_unknowns,
            "capabilities_missing_runtime_proof": capabilities_missing_runtime_proof,
            "source_proof_requirements": source_dossier.proof_requirements,
            "self_proof_requirements": self_dossier.proof_requirements
        },
        "comparison_contract": {
            "strategy": "understand_target_then_compare_against_infring_self_model",
            "requires_shared_dossier_schema": true,
            "requires_soul_fit_authority_fit_and_capability_gap": true,
            "requires_probe_first_when_any_required_probe_is_missing": true
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel_sentinel::system_understanding_dossier::SystemUnderstandingImplementationItem;
    use crate::kernel_sentinel::{
        SystemUnderstandingCapabilityKind, SystemUnderstandingCapabilityRow,
        SystemUnderstandingCapabilityValue, SystemUnderstandingDossier,
        SystemUnderstandingDossierStatus, SystemUnderstandingTransferTarget,
    };

    fn dossier(
        mode: SystemUnderstandingDossierTargetMode,
        dossier_id: &str,
        target_system: &str,
        soul_evidence: Vec<&str>,
        required_next_probes: Vec<&str>,
        capabilities: Vec<SystemUnderstandingCapabilityRow>,
        rejected_capabilities: Vec<SystemUnderstandingCapabilityRow>,
    ) -> SystemUnderstandingDossier {
        SystemUnderstandingDossier {
            dossier_id: dossier_id.to_string(),
            target_mode: mode,
            target_system: target_system.to_string(),
            target_version_or_revision: "main".to_string(),
            dossier_version: 1,
            created_at: "2026-04-29T00:00:00Z".to_string(),
            updated_at: "2026-04-29T00:00:00Z".to_string(),
            owners: vec!["kernel-sentinel".to_string()],
            status: SystemUnderstandingDossierStatus::Usable,
            confidence_overall: 0.85,
            blocking_unknowns: if required_next_probes.is_empty() { Vec::new() } else { vec!["needs_more_truth".to_string()] },
            evidence_index: vec!["evidence.json".to_string()],
            soul_confidence: 0.8,
            soul_evidence: soul_evidence.into_iter().map(str::to_string).collect(),
            soul_unknowns: Vec::new(),
            runtime_confidence: 0.82,
            runtime_evidence: vec!["runtime.json".to_string()],
            runtime_unknowns: Vec::new(),
            required_next_probes: required_next_probes.into_iter().map(str::to_string).collect(),
            ecology_confidence: 0.72,
            ecology_evidence: vec!["ecology.json".to_string()],
            ecology_unknowns: Vec::new(),
            authority_confidence: 0.81,
            authority_evidence: vec!["authority.json".to_string()],
            authority_unknowns: Vec::new(),
            authority_risks: Vec::new(),
            architecture_confidence: 0.76,
            architecture_evidence: vec!["arch.json".to_string()],
            architecture_unknowns: Vec::new(),
            runtime_architecture_mismatches: Vec::new(),
            capability_confidence: 0.79,
            capabilities,
            rejected_capabilities,
            capability_unknowns: Vec::new(),
            failure_model_confidence: 0.72,
            known_failure_modes: Vec::new(),
            violated_invariants: Vec::new(),
            stop_patching_triggers: Vec::new(),
            transfer_confidence: 0.8,
            implementation_items: vec![SystemUnderstandingImplementationItem {
                id: "impl".to_string(),
                summary: "summary".to_string(),
                owner_layer: "core".to_string(),
                invariant: "inv".to_string(),
                proof_requirement: "proof".to_string(),
                rollback_plan: "rollback".to_string(),
            }],
            proof_requirements: vec!["proof.json".to_string()],
            rollback_plan: vec!["rollback".to_string()],
            implementation_confidence: 0.75,
            files_inspected: vec!["file.rs".to_string()],
            implementation_unknowns: Vec::new(),
            syntax_confidence: 0.7,
            syntax_evidence: vec!["syntax.json".to_string()],
            syntax_unknowns: Vec::new(),
        }
    }

    fn capability(
        id: &str,
        target: SystemUnderstandingTransferTarget,
        runtime_proof: Vec<&str>,
    ) -> SystemUnderstandingCapabilityRow {
        SystemUnderstandingCapabilityRow {
            id: id.to_string(),
            kind: SystemUnderstandingCapabilityKind::Tooling,
            value: SystemUnderstandingCapabilityValue::High,
            evidence: vec!["capability.json".to_string()],
            runtime_proof: runtime_proof.into_iter().map(str::to_string).collect(),
            transfer_target: target,
            fit_rationale: "fit".to_string(),
        }
    }

    #[test]
    fn dossier_comparison_emits_gap_fit_and_proof_burden() {
        let source = dossier(
            SystemUnderstandingDossierTargetMode::ExternalAssimilation,
            "forgecode",
            "ForgeCode",
            vec!["agentic coding workflow", "receipt-first deterministic runtime"],
            Vec::new(),
            vec![
                capability("tooling_surface", SystemUnderstandingTransferTarget::Kernel, vec!["tooling-proof.json"]),
                capability("role_workflow", SystemUnderstandingTransferTarget::WorkflowJson, vec!["workflow-proof.json"]),
                capability("shell_agent_authority", SystemUnderstandingTransferTarget::Shell, vec!["shell-proof.json"]),
            ],
            vec![capability("shell_truth_authority", SystemUnderstandingTransferTarget::Reject, Vec::new())],
        );
        let self_dossier = dossier(
            SystemUnderstandingDossierTargetMode::InternalRsi,
            "infring",
            "InfRing",
            vec!["receipt-first deterministic runtime", "kernel authority with orchestration as non-canonical coordination"],
            Vec::new(),
            vec![
                capability("tooling_surface", SystemUnderstandingTransferTarget::Kernel, vec!["tooling-proof.json"]),
                capability("architectural_incident_synthesis", SystemUnderstandingTransferTarget::Kernel, vec!["incident-proof.json"]),
            ],
            vec![capability("shell_truth_authority", SystemUnderstandingTransferTarget::Reject, Vec::new())],
        );
        let comparison = build_external_assimilation_dossier_comparison(&source, &self_dossier);
        assert_eq!(comparison["mode"], "comparison_ready");
        assert!(comparison["capability_gap_analysis"]["shared_capability_ids"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row.as_str() == Some("tooling_surface")));
        assert!(comparison["capability_gap_analysis"]["source_only_capability_ids"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row.as_str() == Some("role_workflow")));
        assert!(comparison["authority_fit"]["shell_authority_pressure_capability_ids"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row.as_str() == Some("shell_agent_authority")));
        assert!(comparison["soul_fit"]["score"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn dossier_comparison_requires_probes_before_fit_claims() {
        let source = dossier(
            SystemUnderstandingDossierTargetMode::ExternalAssimilation,
            "forgecode",
            "ForgeCode",
            vec!["agentic coding workflow"],
            vec!["probe_runtime_surface"],
            vec![capability("tooling_surface", SystemUnderstandingTransferTarget::Kernel, Vec::new())],
            Vec::new(),
        );
        let self_dossier = dossier(
            SystemUnderstandingDossierTargetMode::InternalRsi,
            "infring",
            "InfRing",
            vec!["receipt-first deterministic runtime"],
            vec!["raise_runtime_dossier_confidence"],
            vec![capability("tooling_surface", SystemUnderstandingTransferTarget::Kernel, vec!["proof.json"])],
            Vec::new(),
        );
        let comparison = build_external_assimilation_dossier_comparison(&source, &self_dossier);
        assert_eq!(comparison["mode"], "probe_first");
        assert!(comparison["proof_burden"]["required_next_probes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row.as_str() == Some("probe_runtime_surface")));
        assert!(comparison["proof_burden"]["required_next_probes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row.as_str() == Some("raise_runtime_dossier_confidence")));
        assert!(comparison["proof_burden"]["capabilities_missing_runtime_proof"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row.as_str() == Some("tooling_surface")));
    }
}
