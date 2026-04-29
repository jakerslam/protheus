// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::collections::BTreeSet;

use super::{
    kernel_sentinel_failure_level_for_finding, kernel_sentinel_invariant_registry,
    kernel_sentinel_root_frame_for_finding, KernelSentinelFinding,
    KernelSentinelFindingCategory,
};

pub(super) fn violated_invariants(finding: &KernelSentinelFinding) -> Vec<String> {
    let haystacks = [
        finding.fingerprint.to_ascii_lowercase(),
        finding.summary.to_ascii_lowercase(),
        finding.recommended_action.to_ascii_lowercase(),
        finding.evidence.join(" ").to_ascii_lowercase(),
    ];
    let mut matched = BTreeSet::new();
    for invariant in kernel_sentinel_invariant_registry() {
        let needle = invariant.id.to_ascii_lowercase();
        if haystacks.iter().any(|row| row.contains(&needle)) {
            matched.insert(invariant.id.to_string());
        }
    }
    if matched.is_empty() {
        matched.insert("unknown_invariant".to_string());
    }
    matched.into_iter().collect()
}

fn confidence_percent(finding: &KernelSentinelFinding) -> u8 {
    let evidence_bonus = finding.evidence.len().min(5) as u8 * 6;
    let severity_bonus = match finding.severity {
        super::KernelSentinelSeverity::Critical => 30,
        super::KernelSentinelSeverity::High => 22,
        super::KernelSentinelSeverity::Medium => 14,
        super::KernelSentinelSeverity::Low => 8,
    };
    (45 + evidence_bonus + severity_bonus).min(100)
}

fn stop_patching(finding: &KernelSentinelFinding, root_frame: &str) -> bool {
    root_frame != "local_defect"
        || finding.summary.to_ascii_lowercase().contains("stop_patching")
        || finding.recommended_action.to_ascii_lowercase().contains("structural")
}

fn recommended_refactor_boundary(finding: &KernelSentinelFinding, root_frame: &str) -> &'static str {
    match (root_frame, finding.category) {
        ("authority_policy_contradiction", _) => "shell_gateway_kernel_authority_boundary",
        ("architectural_shape_mismatch", _) => "shell_gateway_runtime_topology_boundary",
        ("cross_boundary_contract", KernelSentinelFindingCategory::GatewayIsolation) => "kernel_gateway_isolation_boundary",
        ("cross_boundary_contract", _) => "kernel_runtime_contract_boundary",
        ("system_self_model", _) => "kernel_sentinel_self_model_boundary",
        _ => "local_owner_boundary",
    }
}

fn affected_layers(finding: &KernelSentinelFinding) -> Vec<String> {
    let mut layers = BTreeSet::new();
    for evidence in &finding.evidence {
        if let Some((layer, _)) = evidence
            .strip_prefix("layer://")
            .and_then(|row| row.split_once('/'))
        {
            if !layer.trim().is_empty() {
                layers.insert(layer.trim().to_string());
            }
        }
    }
    layers.into_iter().collect()
}

fn multi_layer_synthesis_guard(incidents: &[Value]) -> Value {
    let multi_layer = incidents
        .iter()
        .filter(|incident| incident["multi_layer"].as_bool().unwrap_or(false))
        .collect::<Vec<_>>();
    let missing_architectural_synthesis_count = multi_layer
        .iter()
        .filter(|incident| {
            incident["root_frame"].as_str().unwrap_or("").trim().is_empty()
                || incident["recommended_refactor_boundary"]
                    .as_str()
                    .unwrap_or("")
                    .trim()
                    .is_empty()
                || incident["violated_invariants"]
                    .as_array()
                    .map_or(true, |rows| rows.is_empty())
        })
        .count();
    let missing_remediation_classification_count = multi_layer
        .iter()
        .filter(|incident| {
            incident["remediation_level"]
                .as_str()
                .unwrap_or("")
                .trim()
                .is_empty()
        })
        .count();
    let missing_total =
        missing_architectural_synthesis_count + missing_remediation_classification_count;
    json!({
        "pass": missing_total == 0,
        "multi_layer_incident_count": multi_layer.len(),
        "missing_architectural_synthesis_count": missing_architectural_synthesis_count,
        "missing_remediation_classification_count": missing_remediation_classification_count,
        "missing_total": missing_total,
    })
}

pub fn kernel_sentinel_architectural_incident_report_section(
    findings: &[KernelSentinelFinding],
) -> Value {
    let incidents = findings
        .iter()
        .map(|finding| {
            let failure_level = kernel_sentinel_failure_level_for_finding(finding);
            let root_frame = kernel_sentinel_root_frame_for_finding(finding);
            let affected_layers = affected_layers(finding);
            json!({
                "finding_id": finding.id,
                "failure_level": failure_level.code(),
                "root_frame": root_frame,
                "remediation_level": failure_level.remediation_level(),
                "violated_invariants": violated_invariants(finding),
                "affected_layers": affected_layers,
                "multi_layer": affected_layers.len() >= 2,
                "confidence_percent": confidence_percent(finding),
                "stop_patching": stop_patching(finding, root_frame),
                "recommended_refactor_boundary": recommended_refactor_boundary(finding, root_frame),
            })
        })
        .collect::<Vec<_>>();
    let synthesis_guard = multi_layer_synthesis_guard(&incidents);
    json!({
        "type": "kernel_sentinel_architectural_incident_report_section",
        "count": incidents.len(),
        "multi_layer_incident_count": synthesis_guard["multi_layer_incident_count"].clone(),
        "synthesis_guard": synthesis_guard,
        "incidents": incidents,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel_sentinel::{
        KernelSentinelFindingCategory, KernelSentinelSeverity,
        KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
    };

    #[test]
    fn architectural_incident_report_section_includes_required_fields() {
        let finding = KernelSentinelFinding {
            schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
            id: "ksent-architectural-report".to_string(),
            severity: KernelSentinelSeverity::Critical,
            category: KernelSentinelFindingCategory::GatewayIsolation,
            fingerprint: "watchdog_owns_process_uniqueness_and_stale_host_cleanup".to_string(),
            evidence: vec![
                "layer://shell/taskbar_offline".to_string(),
                "layer://gateway/healthz_not_ready".to_string(),
                "process://dashboard_host/lifecycle=stale_duplicate".to_string(),
            ],
            summary: "source-of-truth contradiction with STOP_PATCHING active".to_string(),
            recommended_action: "perform structural refactor across the boundary".to_string(),
            status: "open".to_string(),
        };

        let report = kernel_sentinel_architectural_incident_report_section(&[finding]);
        let incident = &report["incidents"][0];
        assert_eq!(report["type"], "kernel_sentinel_architectural_incident_report_section");
        assert_eq!(report["count"], 1);
        assert_eq!(report["multi_layer_incident_count"], 1);
        assert_eq!(report["synthesis_guard"]["pass"], true);
        assert!(incident["failure_level"].is_string());
        assert!(incident["root_frame"].is_string());
        assert!(incident["remediation_level"].is_string());
        assert!(incident["violated_invariants"].is_array());
        assert!(incident["affected_layers"].is_array());
        assert_eq!(incident["multi_layer"], true);
        assert!(incident["confidence_percent"].as_u64().unwrap() >= 75);
        assert_eq!(incident["stop_patching"], true);
        assert!(incident["recommended_refactor_boundary"].is_string());
    }
}
