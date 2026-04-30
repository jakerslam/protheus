// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    KernelSentinelFinding, KernelSentinelFindingCategory, KernelSentinelSeverity,
    SystemUnderstandingDossier,
    KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
};
use serde_json::{json, Value};

fn text(value: &Value, key: &str, fallback: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn detail<'a>(record: &'a Value, key: &str) -> Option<&'a Value> {
    record
        .get("details")
        .and_then(|details| details.get(key))
        .or_else(|| record.get(key))
        .or_else(|| {
            record
                .get("details")
                .and_then(|details| details.get("details"))
                .and_then(|details| details.get(key))
        })
}

fn detail_bool(record: &Value, key: &str) -> bool {
    detail(record, key).and_then(Value::as_bool).unwrap_or(false)
}

fn detail_text(record: &Value, key: &str) -> String {
    detail(record, key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .unwrap_or("")
        .to_string()
}

fn detail_array_nonempty(record: &Value, key: &str) -> bool {
    detail(record, key)
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false)
}

fn is_self_modification(record: &Value) -> bool {
    matches!(
        text(record, "kind", "").as_str(),
        "self_modification_proposal" | "rsi_safety_handoff" | "self_modification"
    )
        || detail_bool(record, "rsi_safety_handoff")
        || detail_bool(record, "self_modification_proposal")
}

fn wants_advance(record: &Value) -> bool {
    detail_bool(record, "advance_requested")
        || detail_bool(record, "apply_requested")
        || detail_bool(record, "monitor_requested")
        || matches!(
            detail_text(record, "requested_stage").as_str(),
            "apply" | "advance" | "monitor" | "rollback"
        )
        || matches!(
            detail_text(record, "stage").as_str(),
            "apply" | "advance" | "monitor" | "rollback"
        )
}

fn evidence(record: &Value, fallback: &str) -> Vec<String> {
    record
        .get("evidence")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| vec![fallback.to_string()])
}

fn missing_fields(record: &Value) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if detail_text(record, "sentinel_verdict").is_empty() {
        missing.push("sentinel_verdict");
    }
    if !detail_array_nonempty(record, "sentinel_evidence_refs") {
        missing.push("sentinel_evidence_refs");
    }
    if detail_text(record, "rollback_plan").is_empty() {
        missing.push("rollback_plan");
    }
    if detail_text(record, "post_apply_monitoring_criteria").is_empty()
        && !detail_array_nonempty(record, "post_apply_monitoring_criteria")
    {
        missing.push("post_apply_monitoring_criteria");
    }
    missing
}

fn handoff_finding(record: &Value, missing: &[&str]) -> KernelSentinelFinding {
    let subject = text(record, "subject", "self_modification");
    KernelSentinelFinding {
        schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        id: format!("rsi_handoff:missing_contract:{subject}"),
        severity: KernelSentinelSeverity::Critical,
        category: KernelSentinelFindingCategory::SecurityBoundary,
        fingerprint: format!("rsi_handoff:missing_contract:{subject}"),
        evidence: evidence(record, &format!("rsi://handoff/{subject}")),
        summary: format!(
            "{subject} attempted RSI/self-modification advance without required Sentinel handoff fields: {}",
            missing.join(",")
        ),
        recommended_action: "require Sentinel verdict, deterministic evidence refs, rollback plan, and post-apply monitoring before advance".to_string(),
        status: "open".to_string(),
    }
}

pub fn build_rsi_handoff_report(records: &[Value]) -> (Value, Vec<KernelSentinelFinding>) {
    let mut checked = Vec::new();
    let mut findings = Vec::new();
    for record in records {
        if !is_self_modification(record) {
            continue;
        }
        let subject = text(record, "subject", "self_modification");
        let advance_requested = wants_advance(record);
        let missing = if advance_requested {
            missing_fields(record)
        } else {
            Vec::new()
        };
        if !missing.is_empty() {
            findings.push(handoff_finding(record, &missing));
        }
        checked.push(json!({
            "subject": subject,
            "advance_requested": advance_requested,
            "missing_fields": missing,
            "ok": missing.is_empty()
        }));
    }
    (
        json!({
            "ok": findings.is_empty(),
            "type": "kernel_sentinel_rsi_safety_handoff",
            "checked_count": checked.len(),
            "blocking_failure_count": findings.len(),
            "required_fields": [
                "sentinel_verdict",
                "sentinel_evidence_refs",
                "rollback_plan",
                "post_apply_monitoring_criteria"
            ],
            "checked": checked
        }),
        findings,
    )
}

pub fn build_internal_rsi_proposals(dossier: &SystemUnderstandingDossier) -> Value {
    let diagnostic_evidence_refs = dossier
        .evidence_index
        .iter()
        .filter(|row| row.contains("kernel_sentinel_diagnostic_run_current.json"))
        .cloned()
        .collect::<Vec<_>>();
    let ready_for_structural_proposals =
        dossier.required_next_probes.is_empty() && !dossier.implementation_items.is_empty();
    let proposals = if ready_for_structural_proposals {
        dossier
            .implementation_items
            .iter()
            .map(|item| {
                json!({
                    "proposal_id": format!("internal_rsi:{}", item.id),
                    "type": "kernel_sentinel_internal_rsi_proposal",
                    "status": "proposal_ready",
                    "source_dossier_id": dossier.dossier_id,
                    "target_system": dossier.target_system,
                    "owner_layer": item.owner_layer,
                    "summary": item.summary,
                    "invariant": item.invariant,
                    "proof_requirement": item.proof_requirement,
                    "rollback_plan": item.rollback_plan,
                    "blocking_unknowns": dossier.blocking_unknowns,
                    "diagnostic_evidence_refs": diagnostic_evidence_refs,
                    "evidence_refs": dossier.evidence_index,
                })
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    json!({
        "ok": ready_for_structural_proposals,
        "type": "kernel_sentinel_internal_rsi_proposal_bundle",
        "mode": if ready_for_structural_proposals { "proposal_ready" } else { "probe_first" },
        "source_dossier_id": dossier.dossier_id,
        "target_system": dossier.target_system,
        "confidence_overall": dossier.confidence_overall,
        "transfer_confidence": dossier.transfer_confidence,
        "authority_confidence": dossier.authority_confidence,
        "runtime_confidence": dossier.runtime_confidence,
        "diagnostic_evidence_refs": diagnostic_evidence_refs,
        "required_next_probes": dossier.required_next_probes,
        "blocking_unknowns": dossier.blocking_unknowns,
        "proposal_count": proposals.len(),
        "proposals": proposals,
        "contract": {
            "required_fields_per_proposal": [
                "proposal_id",
                "owner_layer",
                "summary",
                "invariant",
                "proof_requirement",
                "rollback_plan"
            ],
            "probe_first_when_required_next_probes_present": true,
            "self_modification_is_proposal_only": true
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel_sentinel::{
        SystemUnderstandingCapabilityKind, SystemUnderstandingCapabilityRow,
        SystemUnderstandingCapabilityValue, SystemUnderstandingDossier,
        SystemUnderstandingDossierStatus, SystemUnderstandingDossierTargetMode,
        SystemUnderstandingTransferTarget,
    };
    use crate::kernel_sentinel::system_understanding_dossier::SystemUnderstandingImplementationItem;

    fn sample_dossier(required_next_probes: Vec<&str>) -> SystemUnderstandingDossier {
        let has_required_next_probes = !required_next_probes.is_empty();
        SystemUnderstandingDossier {
            dossier_id: "infring".to_string(),
            target_mode: SystemUnderstandingDossierTargetMode::InternalRsi,
            target_system: "InfRing".to_string(),
            target_version_or_revision: "main".to_string(),
            dossier_version: 1,
            created_at: "2026-04-29T00:00:00Z".to_string(),
            updated_at: "2026-04-29T00:00:00Z".to_string(),
            owners: vec!["kernel-sentinel".to_string()],
            status: SystemUnderstandingDossierStatus::Usable,
            confidence_overall: 0.86,
            blocking_unknowns: if !has_required_next_probes {
                Vec::new()
            } else {
                vec!["structural_recommendations_blocked_until_dossier_confidence_recovers".to_string()]
            },
            evidence_index: vec!["local/state/kernel_sentinel/kernel_sentinel_report_current.json".to_string()],
            soul_confidence: 0.80,
            soul_evidence: vec!["receipt-first deterministic runtime".to_string()],
            soul_unknowns: Vec::new(),
            runtime_confidence: 0.84,
            runtime_evidence: vec!["local/state/kernel_sentinel/kernel_sentinel_health_current.json".to_string()],
            runtime_unknowns: Vec::new(),
            required_next_probes: required_next_probes.into_iter().map(str::to_string).collect(),
            ecology_confidence: 0.75,
            ecology_evidence: vec!["feedback_inbox.jsonl".to_string()],
            ecology_unknowns: Vec::new(),
            authority_confidence: 0.88,
            authority_evidence: vec!["kernel_sentinel_verdict.json".to_string()],
            authority_unknowns: Vec::new(),
            authority_risks: Vec::new(),
            architecture_confidence: 0.82,
            architecture_evidence: vec!["architectural_incident_report_current.json".to_string()],
            architecture_unknowns: Vec::new(),
            runtime_architecture_mismatches: Vec::new(),
            capability_confidence: 0.80,
            capabilities: vec![SystemUnderstandingCapabilityRow {
                id: "kernel_runtime_truth_loop".to_string(),
                kind: SystemUnderstandingCapabilityKind::Evidence,
                value: SystemUnderstandingCapabilityValue::Critical,
                evidence: vec!["kernel_sentinel_report_current.json".to_string()],
                runtime_proof: vec!["kernel_sentinel_report_current.json".to_string()],
                transfer_target: SystemUnderstandingTransferTarget::Kernel,
                fit_rationale: "Kernel owns runtime truth.".to_string(),
            }],
            rejected_capabilities: Vec::new(),
            capability_unknowns: Vec::new(),
            failure_model_confidence: 0.79,
            known_failure_modes: Vec::new(),
            violated_invariants: Vec::new(),
            stop_patching_triggers: Vec::new(),
            transfer_confidence: if !has_required_next_probes { 0.84 } else { 0.62 },
            implementation_items: if !has_required_next_probes {
                vec![SystemUnderstandingImplementationItem {
                    id: "strengthen-sentinel-dossier".to_string(),
                    summary: "Emit internal RSI proposals from the self-dossier.".to_string(),
                    owner_layer: "core/layer0/ops".to_string(),
                    invariant: "kernel_owns_truth".to_string(),
                    proof_requirement: "internal_rsi_proposals_current.json must stay schema-complete".to_string(),
                    rollback_plan: "revert to dossier-only output".to_string(),
                }]
            } else {
                Vec::new()
            },
            proof_requirements: vec!["proof://internal-rsi".to_string()],
            rollback_plan: vec!["rollback://internal-rsi".to_string()],
            implementation_confidence: 0.71,
            files_inspected: vec!["core/layer0/ops/src/kernel_sentinel/rsi_handoff.rs".to_string()],
            implementation_unknowns: Vec::new(),
            syntax_confidence: 0.68,
            syntax_evidence: vec!["rsi_handoff.rs".to_string()],
            syntax_unknowns: Vec::new(),
        }
    }

    #[test]
    fn self_modification_cannot_advance_without_sentinel_contract() {
        let records = vec![json!({
            "subject": "patch-loop-1",
            "kind": "self_modification_proposal",
            "evidence": ["proposal://patch-loop-1"],
            "details": {"advance_requested": true, "sentinel_verdict": "allow"}
        })];
        let (report, findings) = build_rsi_handoff_report(&records);
        assert_eq!(report["blocking_failure_count"], Value::from(1));
        assert_eq!(findings[0].fingerprint, "rsi_handoff:missing_contract:patch-loop-1");
        assert!(findings[0].summary.contains("rollback_plan"));
    }

    #[test]
    fn complete_handoff_passes() {
        let records = vec![json!({
            "subject": "patch-loop-2",
            "kind": "self_modification_proposal",
            "details": {
                "advance_requested": true,
                "sentinel_verdict": "allow",
                "sentinel_evidence_refs": ["receipt://ok"],
                "rollback_plan": "restore previous artifact",
                "post_apply_monitoring_criteria": "no new critical Sentinel findings"
            }
        })];
        let (report, findings) = build_rsi_handoff_report(&records);
        assert!(findings.is_empty());
        assert_eq!(report["checked"][0]["ok"], true);
    }

    #[test]
    fn internal_rsi_bundle_emits_structured_proposals_when_dossier_is_ready() {
        let bundle = build_internal_rsi_proposals(&sample_dossier(Vec::new()));
        assert_eq!(bundle["type"], "kernel_sentinel_internal_rsi_proposal_bundle");
        assert_eq!(bundle["mode"], "proposal_ready");
        assert_eq!(bundle["proposal_count"], 1);
        assert_eq!(bundle["proposals"][0]["owner_layer"], "core/layer0/ops");
        assert_eq!(bundle["proposals"][0]["invariant"], "kernel_owns_truth");
        assert!(bundle["proposals"][0]["proof_requirement"]
            .as_str()
            .unwrap_or("")
            .contains("internal_rsi_proposals_current.json"));
    }

    #[test]
    fn internal_rsi_bundle_requires_probes_when_dossier_confidence_is_low() {
        let bundle = build_internal_rsi_proposals(&sample_dossier(vec![
            "raise_runtime_dossier_confidence",
            "raise_transfer_dossier_confidence",
        ]));
        assert_eq!(bundle["mode"], "probe_first");
        assert_eq!(bundle["proposal_count"], 0);
        assert!(bundle["required_next_probes"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .any(|row| row.as_str() == Some("raise_runtime_dossier_confidence")));
    }
}
