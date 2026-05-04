// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};

use super::system_understanding_dossier::SystemUnderstandingDossier;

const WORKSHEET_SCHEMA_VERSION: u32 = 1;

fn confidence_gate(confidence: f64, minimum: f64) -> &'static str {
    if confidence >= minimum {
        "complete"
    } else {
        "needs_probe"
    }
}

fn nonempty_or_unknown(rows: &[String]) -> Value {
    if rows.is_empty() {
        json!(["unknown"])
    } else {
        json!(rows)
    }
}

fn worksheet_phase(
    id: &str,
    question: &str,
    confidence: f64,
    minimum_confidence: f64,
    evidence_refs: &[String],
    unknowns: &[String],
    next_probe: &str,
) -> Value {
    let status = confidence_gate(confidence, minimum_confidence);
    json!({
        "id": id,
        "question": question,
        "priority_order": [
            "soul_before_behavior",
            "behavior_before_structure",
            "structure_before_syntax"
        ],
        "confidence": confidence,
        "minimum_confidence": minimum_confidence,
        "status": status,
        "must_complete_before_next_zoom": status != "complete",
        "evidence_refs": nonempty_or_unknown(evidence_refs),
        "unknowns": nonempty_or_unknown(unknowns),
        "required_next_probe": if status == "complete" { Value::Null } else { json!(next_probe) }
    })
}

fn top_blocker(phases: &[Value], dossier: &SystemUnderstandingDossier) -> String {
    phases
        .iter()
        .find(|phase| phase["status"].as_str() != Some("complete"))
        .and_then(|phase| phase["id"].as_str())
        .map(|id| format!("phase_requires_more_evidence:{id}"))
        .or_else(|| dossier.blocking_unknowns.first().cloned())
        .unwrap_or_else(|| "none".to_string())
}

pub fn build_system_understanding_worksheet(
    dossier: &SystemUnderstandingDossier,
    report: &Value,
    self_study_outputs: &Value,
    diagnostic_run: &Value,
) -> Value {
    let authority_unknowns = [
        dossier.authority_unknowns.clone(),
        dossier.authority_risks.clone(),
    ]
    .concat();
    let boundary_evidence = [
        dossier.architecture_evidence.clone(),
        dossier.authority_evidence.clone(),
    ]
    .concat();
    let boundary_unknowns = [
        dossier.architecture_unknowns.clone(),
        dossier.runtime_architecture_mismatches.clone(),
    ]
    .concat();
    let mut drift_unknowns = Vec::new();
    if self_study_outputs["regression_count"].as_u64().unwrap_or(0) > 0 {
        drift_unknowns.push("active_regression_delta".to_string());
    }
    if report["operator_summary"]["scheduler_stale"]
        .as_bool()
        .unwrap_or(false)
    {
        drift_unknowns.push("scheduler_runtime_stale".to_string());
    }
    let gap_unknowns = [
        dossier.blocking_unknowns.clone(),
        dossier.runtime_unknowns.clone(),
        dossier.capability_unknowns.clone(),
        dossier.implementation_unknowns.clone(),
    ]
    .concat();
    let phases = vec![
        worksheet_phase(
            "soul",
            "What is this system trying to be, and what must it refuse to become?",
            dossier.soul_confidence,
            0.60,
            &dossier.soul_evidence,
            &dossier.soul_unknowns,
            "refresh_system_philosophy_evidence",
        ),
        worksheet_phase(
            "runtime_behavior",
            "What does the system actually do while running, under current evidence?",
            dossier.runtime_confidence,
            0.70,
            &dossier.runtime_evidence,
            &dossier.runtime_unknowns,
            "collect_fresh_runtime_behavior_trace",
        ),
        worksheet_phase(
            "authority_map",
            "Which components are allowed to decide truth, permission, and mutation?",
            dossier.authority_confidence,
            0.80,
            &dossier.authority_evidence,
            &authority_unknowns,
            "collect_authority_boundary_receipts",
        ),
        worksheet_phase(
            "boundary_map",
            "Where do authority, projection, gateway, validation, and observability boundaries meet?",
            dossier.architecture_confidence,
            0.70,
            &boundary_evidence,
            &boundary_unknowns,
            "collect_boundary_nexus_trace",
        ),
        worksheet_phase(
            "drift",
            "Where does current behavior diverge from doctrine, prior runs, or intended boundaries?",
            dossier.failure_model_confidence,
            0.65,
            &dossier.known_failure_modes,
            &drift_unknowns,
            "accumulate_trend_and_regression_evidence",
        ),
        worksheet_phase(
            "gaps",
            "What is still unknown enough to block structural changes or assimilation transfer?",
            dossier.transfer_confidence,
            0.80,
            &dossier.evidence_index,
            &gap_unknowns,
            "close_blocking_unknowns_before_transfer",
        ),
        worksheet_phase(
            "confidence",
            "Is the self-model strong enough to guide implementation without symptom patching?",
            dossier.confidence_overall,
            0.75,
            &dossier.proof_requirements,
            &dossier.required_next_probes,
            "raise_dossier_confidence_before_zooming_down",
        ),
        worksheet_phase(
            "syntax_detail",
            "Which low-level files or syntax details matter after higher-level understanding is stable?",
            dossier.syntax_confidence,
            0.55,
            &dossier.syntax_evidence,
            &dossier.syntax_unknowns,
            "inspect_low_level_files_after_higher_level_gates",
        ),
    ];
    let complete_phase_count = phases
        .iter()
        .filter(|phase| phase["status"].as_str() == Some("complete"))
        .count();
    let blocker = top_blocker(&phases, dossier);
    let ready_to_zoom_down = blocker == "none";
    let mut worksheet = json!({
        "type": "kernel_sentinel_system_understanding_worksheet",
        "schema_version": WORKSHEET_SCHEMA_VERSION,
        "generated_at": crate::now_iso(),
        "cadence": "every_kernel_sentinel_auto_run",
        "source_dossier_id": dossier.dossier_id,
        "target_system": dossier.target_system,
        "target_mode": dossier.target_mode,
        "recurring": true,
        "method": "soul_then_runtime_then_structure_then_syntax",
        "ready_to_zoom_down": ready_to_zoom_down,
        "top_blocker": blocker,
        "complete_phase_count": complete_phase_count,
        "phase_count": phases.len(),
        "phases": phases,
        "operator_summary": {
            "status": if ready_to_zoom_down { "usable" } else { "needs_more_understanding" },
            "complete_phase_count": complete_phase_count,
            "phase_count": phases.len(),
            "top_blocker": blocker,
            "diagnostic_probe_count": diagnostic_run["authorized_probe_count"].as_u64().unwrap_or(0),
            "trend_history_runs": self_study_outputs["trend_history_runs"].as_u64().unwrap_or(0)
        }
    });
    worksheet["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&worksheet));
    worksheet
}

pub fn render_system_understanding_worksheet_markdown(worksheet: &Value) -> String {
    let mut body = String::new();
    body.push_str("# Kernel Sentinel System Understanding Worksheet\n\n");
    body.push_str(&format!(
        "- generated_at: {}\n",
        worksheet["generated_at"].as_str().unwrap_or("unknown")
    ));
    body.push_str(&format!(
        "- cadence: {}\n",
        worksheet["cadence"].as_str().unwrap_or("unknown")
    ));
    body.push_str(&format!(
        "- method: {}\n",
        worksheet["method"].as_str().unwrap_or("unknown")
    ));
    body.push_str(&format!(
        "- ready_to_zoom_down: {}\n",
        worksheet["ready_to_zoom_down"].as_bool().unwrap_or(false)
    ));
    body.push_str(&format!(
        "- top_blocker: {}\n\n",
        worksheet["top_blocker"].as_str().unwrap_or("unknown")
    ));
    body.push_str("## Phases\n\n");
    for phase in worksheet["phases"].as_array().into_iter().flatten() {
        body.push_str(&format!(
            "- `{}`: {} ({:.2}/{:.2})\n",
            phase["id"].as_str().unwrap_or("unknown"),
            phase["status"].as_str().unwrap_or("unknown"),
            phase["confidence"].as_f64().unwrap_or(0.0),
            phase["minimum_confidence"].as_f64().unwrap_or(0.0)
        ));
        body.push_str(&format!(
            "  - Question: {}\n",
            phase["question"].as_str().unwrap_or("unknown")
        ));
        if let Some(probe) = phase["required_next_probe"].as_str() {
            body.push_str(&format!("  - Required next probe: {probe}\n"));
        }
    }
    body
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel_sentinel::{
        SystemUnderstandingDossierStatus, SystemUnderstandingDossierTargetMode,
        SYSTEM_UNDERSTANDING_DOSSIER_SCHEMA_VERSION,
    };

    fn dossier() -> SystemUnderstandingDossier {
        serde_json::from_value(json!({
            "dossier_id": "infring",
            "target_mode": "internal_rsi",
            "target_system": "InfRing",
            "target_version_or_revision": "main",
            "dossier_version": SYSTEM_UNDERSTANDING_DOSSIER_SCHEMA_VERSION,
            "created_at": "2026-05-01T00:00:00Z",
            "updated_at": "2026-05-01T00:00:00Z",
            "owners": ["kernel-sentinel"],
            "status": SystemUnderstandingDossierStatus::Draft,
            "confidence_overall": 0.72,
            "blocking_unknowns": ["trend_history_insufficient"],
            "evidence_index": ["kernel://report"],
            "soul_confidence": 0.90,
            "soul_evidence": ["docs://philosophy"],
            "soul_unknowns": [],
            "runtime_confidence": 0.91,
            "runtime_evidence": ["kernel://runtime"],
            "runtime_unknowns": [],
            "required_next_probes": ["accumulate_three_kernel_sentinel_trend_runs"],
            "ecology_confidence": 0.80,
            "ecology_evidence": ["kernel://ecology"],
            "ecology_unknowns": [],
            "authority_confidence": 0.88,
            "authority_evidence": ["kernel://authority"],
            "authority_unknowns": [],
            "authority_risks": [],
            "architecture_confidence": 0.86,
            "architecture_evidence": ["kernel://architecture"],
            "architecture_unknowns": [],
            "runtime_architecture_mismatches": [],
            "capability_confidence": 0.80,
            "capabilities": [],
            "rejected_capabilities": [],
            "capability_unknowns": [],
            "failure_model_confidence": 0.82,
            "known_failure_modes": ["authority_shape_residue"],
            "violated_invariants": [],
            "stop_patching_triggers": [],
            "transfer_confidence": 0.71,
            "implementation_items": [],
            "proof_requirements": ["proof://worksheet"],
            "rollback_plan": ["rollback://worksheet"],
            "implementation_confidence": 0.66,
            "files_inspected": ["core/layer0/ops/src/kernel_sentinel.rs"],
            "implementation_unknowns": [],
            "syntax_confidence": 0.60,
            "syntax_evidence": ["code://kernel_sentinel"],
            "syntax_unknowns": []
        }))
        .unwrap()
    }

    #[test]
    fn worksheet_preserves_high_to_low_understanding_order() {
        let worksheet = build_system_understanding_worksheet(
            &dossier(),
            &json!({"operator_summary": {"scheduler_stale": false}}),
            &json!({"trend_history_runs": 1, "regression_count": 0}),
            &json!({"authorized_probe_count": 2}),
        );
        assert_eq!(
            worksheet["type"],
            "kernel_sentinel_system_understanding_worksheet"
        );
        assert_eq!(worksheet["cadence"], "every_kernel_sentinel_auto_run");
        assert_eq!(worksheet["phases"][0]["id"], "soul");
        assert_eq!(worksheet["phases"][1]["id"], "runtime_behavior");
        assert_eq!(worksheet["phases"][2]["id"], "authority_map");
        assert_eq!(worksheet["phases"][7]["id"], "syntax_detail");
        assert_eq!(worksheet["ready_to_zoom_down"], false);
        assert_eq!(
            worksheet["top_blocker"],
            "phase_requires_more_evidence:gaps"
        );
        assert_eq!(
            worksheet["phases"][5]["required_next_probe"],
            "close_blocking_unknowns_before_transfer"
        );
        assert!(render_system_understanding_worksheet_markdown(&worksheet)
            .contains("Kernel Sentinel System Understanding Worksheet"));
        let mode: SystemUnderstandingDossierTargetMode =
            serde_json::from_value(worksheet["target_mode"].clone()).unwrap();
        assert_eq!(mode, SystemUnderstandingDossierTargetMode::InternalRsi);
    }
}
