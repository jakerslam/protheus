// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    SystemUnderstandingCapabilityRow, SystemUnderstandingDossier,
    SystemUnderstandingDossierTargetMode, SystemUnderstandingTransferTarget,
};
use serde_json::{json, Value};

fn transfer_target_label(target: &SystemUnderstandingTransferTarget) -> &'static str {
    match target {
        SystemUnderstandingTransferTarget::Kernel => "kernel",
        SystemUnderstandingTransferTarget::Orchestration => "orchestration",
        SystemUnderstandingTransferTarget::Shell => "shell",
        SystemUnderstandingTransferTarget::Gateway => "gateway",
        SystemUnderstandingTransferTarget::WorkflowJson => "workflow_json",
        SystemUnderstandingTransferTarget::Docs => "docs",
        SystemUnderstandingTransferTarget::Tests => "tests",
        SystemUnderstandingTransferTarget::Reject => "reject",
    }
}

fn capability_transfer_row(
    source_dossier: &SystemUnderstandingDossier,
    row: &SystemUnderstandingCapabilityRow,
) -> Value {
    json!({
        "transfer_id": format!("external_assimilation:{}", row.id),
        "capability_id": row.id,
        "kind": row.kind,
        "value": row.value,
        "transfer_target": transfer_target_label(&row.transfer_target),
        "fit_rationale": row.fit_rationale,
        "runtime_proof": row.runtime_proof,
        "evidence": row.evidence,
        "source_system": source_dossier.target_system,
        "source_revision": source_dossier.target_version_or_revision,
        "proof_burden": if row.runtime_proof.is_empty() {
            source_dossier.proof_requirements.clone()
        } else {
            row.runtime_proof.clone()
        },
        "assimilation_priority": "capability_before_file_burn_down",
        "requires_dossier_capability_tracking": true
    })
}

pub fn build_external_assimilation_transfer_plan(
    source_dossier: &SystemUnderstandingDossier,
) -> Value {
    let dossier_mode_ok = matches!(
        source_dossier.target_mode,
        SystemUnderstandingDossierTargetMode::ExternalAssimilation
    );
    let ready_for_capability_transfer =
        dossier_mode_ok && source_dossier.required_next_probes.is_empty();
    let transfer_rows = if ready_for_capability_transfer {
        source_dossier
            .capabilities
            .iter()
            .filter(|row| {
                !matches!(
                    row.transfer_target,
                    SystemUnderstandingTransferTarget::Reject
                )
            })
            .map(|row| capability_transfer_row(source_dossier, row))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let rejected = source_dossier
        .rejected_capabilities
        .iter()
        .map(|row| {
            json!({
                "capability_id": row.id,
                "reason": row.fit_rationale,
                "transfer_target": transfer_target_label(&row.transfer_target)
            })
        })
        .collect::<Vec<_>>();
    json!({
        "ok": ready_for_capability_transfer,
        "type": "kernel_sentinel_external_assimilation_transfer_plan",
        "mode": if ready_for_capability_transfer { "capability_plan_ready" } else { "probe_first" },
        "source_dossier_id": source_dossier.dossier_id,
        "source_system": source_dossier.target_system,
        "source_revision": source_dossier.target_version_or_revision,
        "source_dossier_mode": source_dossier.target_mode,
        "capability_plan_count": transfer_rows.len(),
        "capability_transfer_plan": transfer_rows,
        "required_next_probes": source_dossier.required_next_probes,
        "blocking_unknowns": source_dossier.blocking_unknowns,
        "rejected_capabilities": rejected,
        "contract": {
            "strategy": "capability_first_not_file_burn_down",
            "requires_shared_dossier_schema": true,
            "requires_capability_id_before_file_rows": true,
            "probe_first_when_required_next_probes_present": true
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel_sentinel::system_understanding_dossier::SystemUnderstandingImplementationItem;
    use crate::kernel_sentinel::{
        SystemUnderstandingCapabilityKind, SystemUnderstandingCapabilityValue,
        SystemUnderstandingDossierStatus,
    };

    fn source_dossier(required_next_probes: Vec<&str>) -> SystemUnderstandingDossier {
        let ready = required_next_probes.is_empty();
        SystemUnderstandingDossier {
            dossier_id: "forgecode".to_string(),
            target_mode: SystemUnderstandingDossierTargetMode::ExternalAssimilation,
            target_system: "ForgeCode".to_string(),
            target_version_or_revision: "main".to_string(),
            dossier_version: 1,
            created_at: "2026-04-29T00:00:00Z".to_string(),
            updated_at: "2026-04-29T00:00:00Z".to_string(),
            owners: vec!["kernel-sentinel".to_string(), "assimilation".to_string()],
            status: if ready {
                SystemUnderstandingDossierStatus::Usable
            } else {
                SystemUnderstandingDossierStatus::Draft
            },
            confidence_overall: if ready { 0.84 } else { 0.61 },
            blocking_unknowns: if ready {
                Vec::new()
            } else {
                vec!["needs_runtime_and_authority_probes".to_string()]
            },
            evidence_index: vec!["external/forgecode/runtime.json".to_string()],
            soul_confidence: 0.78,
            soul_evidence: vec!["agentic coding workflow".to_string()],
            soul_unknowns: Vec::new(),
            runtime_confidence: if ready { 0.82 } else { 0.58 },
            runtime_evidence: vec!["external/forgecode/runtime.json".to_string()],
            runtime_unknowns: Vec::new(),
            required_next_probes: required_next_probes.into_iter().map(str::to_string).collect(),
            ecology_confidence: 0.72,
            ecology_evidence: vec!["external/forgecode/ecology.json".to_string()],
            ecology_unknowns: Vec::new(),
            authority_confidence: if ready { 0.81 } else { 0.55 },
            authority_evidence: vec!["external/forgecode/authority.json".to_string()],
            authority_unknowns: Vec::new(),
            authority_risks: Vec::new(),
            architecture_confidence: 0.77,
            architecture_evidence: vec!["external/forgecode/architecture.json".to_string()],
            architecture_unknowns: Vec::new(),
            runtime_architecture_mismatches: Vec::new(),
            capability_confidence: 0.80,
            capabilities: vec![
                SystemUnderstandingCapabilityRow {
                    id: "forgecode_role_workflow".to_string(),
                    kind: SystemUnderstandingCapabilityKind::Workflow,
                    value: SystemUnderstandingCapabilityValue::High,
                    evidence: vec!["external/forgecode/workflows/role.json".to_string()],
                    runtime_proof: vec!["external/forgecode/replays/role.json".to_string()],
                    transfer_target: SystemUnderstandingTransferTarget::WorkflowJson,
                    fit_rationale: "Role workflow should land as JSON workflow orchestration, not raw file burn-down.".to_string(),
                },
                SystemUnderstandingCapabilityRow {
                    id: "forgecode_tooling_surface".to_string(),
                    kind: SystemUnderstandingCapabilityKind::Tooling,
                    value: SystemUnderstandingCapabilityValue::Critical,
                    evidence: vec!["external/forgecode/tooling.json".to_string()],
                    runtime_proof: vec!["external/forgecode/replays/tooling.json".to_string()],
                    transfer_target: SystemUnderstandingTransferTarget::Kernel,
                    fit_rationale: "Raw capability routing belongs in kernel-authoritative tooling surfaces.".to_string(),
                },
            ],
            rejected_capabilities: vec![SystemUnderstandingCapabilityRow {
                id: "forgecode_shell_authority".to_string(),
                kind: SystemUnderstandingCapabilityKind::Ux,
                value: SystemUnderstandingCapabilityValue::High,
                evidence: vec!["external/forgecode/ui-authority.json".to_string()],
                runtime_proof: Vec::new(),
                transfer_target: SystemUnderstandingTransferTarget::Reject,
                fit_rationale: "Shell authority duplication should not be assimilated.".to_string(),
            }],
            capability_unknowns: Vec::new(),
            failure_model_confidence: 0.74,
            known_failure_modes: Vec::new(),
            violated_invariants: Vec::new(),
            stop_patching_triggers: Vec::new(),
            transfer_confidence: if ready { 0.83 } else { 0.49 },
            implementation_items: vec![SystemUnderstandingImplementationItem {
                id: "forgecode-capability-transfer".to_string(),
                summary: "Transfer ForgeCode capabilities before touching file ledgers.".to_string(),
                owner_layer: "assimilation".to_string(),
                invariant: "capability_before_file_burn_down".to_string(),
                proof_requirement: "capability transfer rows must exist".to_string(),
                rollback_plan: "revert to dossier-only planning".to_string(),
            }],
            proof_requirements: vec!["external proof burden".to_string()],
            rollback_plan: vec!["rollback external transfer plan".to_string()],
            implementation_confidence: 0.68,
            files_inspected: vec!["ForgeCode-Assimilation/assimilation-map.md".to_string()],
            implementation_unknowns: Vec::new(),
            syntax_confidence: 0.66,
            syntax_evidence: vec!["external/forgecode/syntax.json".to_string()],
            syntax_unknowns: Vec::new(),
        }
    }

    #[test]
    fn external_assimilation_plan_emits_capability_transfer_rows() {
        let plan = build_external_assimilation_transfer_plan(&source_dossier(Vec::new()));
        assert_eq!(
            plan["type"],
            "kernel_sentinel_external_assimilation_transfer_plan"
        );
        assert_eq!(plan["mode"], "capability_plan_ready");
        assert_eq!(plan["capability_plan_count"], 2);
        assert_eq!(
            plan["capability_transfer_plan"][0]["assimilation_priority"],
            "capability_before_file_burn_down"
        );
        assert_eq!(
            plan["capability_transfer_plan"][0]["transfer_target"],
            "workflow_json"
        );
    }

    #[test]
    fn external_assimilation_plan_requires_probes_before_transfer_when_source_is_not_ready() {
        let plan = build_external_assimilation_transfer_plan(&source_dossier(vec![
            "probe_runtime_surface",
            "probe_authority_boundary",
        ]));
        assert_eq!(plan["mode"], "probe_first");
        assert_eq!(plan["capability_plan_count"], 0);
        assert!(plan["required_next_probes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row.as_str() == Some("probe_runtime_surface")));
    }
}
