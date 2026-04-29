// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;

use super::{
    KernelSentinelFailureLevel, KernelSentinelIncidentCluster,
    KernelSentinelIncidentClusterKey, KernelSentinelIncidentEvidenceLevel,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelSentinelArchitecturalIncident {
    pub cluster_key: KernelSentinelIncidentClusterKey,
    pub occurrence_count: usize,
    pub violated_invariants: Vec<String>,
    pub likely_root_frame: String,
    pub confidence_percent: u8,
    pub affected_layers: Vec<String>,
    pub remediation_class: String,
    pub stop_patching: bool,
    pub stop_patching_reasons: Vec<String>,
    pub failure_level: KernelSentinelFailureLevel,
    pub evidence_levels: Vec<KernelSentinelIncidentEvidenceLevel>,
    pub evidence_refs: Vec<String>,
    pub summaries: Vec<String>,
}

fn root_frame_for_failure_level(level: KernelSentinelFailureLevel) -> &'static str {
    match level {
        KernelSentinelFailureLevel::L0LocalDefect => "local_defect",
        KernelSentinelFailureLevel::L1ComponentRegression => "component_regression",
        KernelSentinelFailureLevel::L2BoundaryContractBreach => "cross_boundary_contract",
        KernelSentinelFailureLevel::L3PolicyTruthFailure => "authority_policy_contradiction",
        KernelSentinelFailureLevel::L4ArchitecturalMisalignment => "architectural_shape_mismatch",
        KernelSentinelFailureLevel::L5SelfModelFailure => "system_self_model",
    }
}

fn confidence_percent(cluster: &KernelSentinelIncidentCluster) -> u8 {
    let occurrence_bonus = match cluster.occurrence_count {
        0 | 1 => 0,
        2 => 15,
        3 => 25,
        _ => 35,
    };
    let level_bonus = cluster.evidence_levels.len().saturating_sub(1).min(3) as u8 * 10;
    let invariant_bonus = if cluster.invariant_ids.is_empty() { 0 } else { 10 };
    let failure_bonus = cluster.highest_failure_level.priority().saturating_mul(4);
    40u8
        .saturating_add(occurrence_bonus)
        .saturating_add(level_bonus)
        .saturating_add(invariant_bonus)
        .saturating_add(failure_bonus)
        .min(100)
}

fn affected_layers(cluster: &KernelSentinelIncidentCluster) -> Vec<String> {
    let mut layers = BTreeSet::new();
    if !cluster.key.affected_layer.trim().is_empty() {
        layers.insert(cluster.key.affected_layer.clone());
    }
    for evidence in &cluster.evidence_refs {
        if let Some((layer, _)) = evidence.strip_prefix("layer://").and_then(|row| row.split_once('/')) {
            if !layer.trim().is_empty() {
                layers.insert(layer.trim().to_string());
            }
        }
    }
    layers.into_iter().collect()
}

fn violated_invariants(cluster: &KernelSentinelIncidentCluster) -> Vec<String> {
    if !cluster.invariant_ids.is_empty() {
        return cluster.invariant_ids.clone();
    }
    if cluster.key.invariant_family.trim().is_empty() {
        vec!["unknown_invariant".to_string()]
    } else {
        vec![format!("invariant_family:{}", cluster.key.invariant_family)]
    }
}

fn mentions_any(cluster: &KernelSentinelIncidentCluster, needles: &[&str]) -> bool {
    cluster
        .evidence_refs
        .iter()
        .chain(cluster.summaries.iter())
        .any(|row| {
            let lowered = row.to_ascii_lowercase();
            needles.iter().any(|needle| lowered.contains(needle))
        })
}

fn command_success_runtime_contradiction(cluster: &KernelSentinelIncidentCluster) -> bool {
    if mentions_any(cluster, &["command_output_contradicts_runtime", "command_success_but_runtime_failed", "success output contradicts"]) {
        return true;
    }
    let rows: Vec<String> = cluster.evidence_refs.iter().chain(cluster.summaries.iter()).map(|row| row.to_ascii_lowercase()).collect();
    let command_success = rows.iter().any(|row| (row.contains("command") || row.contains("restart")) && (row.contains("success") || row.contains("active")));
    let runtime_unready = rows.iter().any(|row| ["listener_absent", "listener=absent", "without_listener", "healthz_not_ready", "not ready", "offline", "unavailable", "stale_duplicate", "lifecycle=failed", "runtime_failed"].iter().any(|needle| row.contains(needle)));
    command_success && runtime_unready
}

fn process_ownership_invariant_breach(cluster: &KernelSentinelIncidentCluster) -> bool {
    mentions_any(cluster, &["duplicate_dashboard_hosts", "duplicate_host", "stale_duplicate", "stale host", "watchdog_owns_process_uniqueness_and_stale_host_cleanup", "process://dashboard_host"])
}

fn boot_route_dependency_risk(cluster: &KernelSentinelIncidentCluster) -> bool {
    cluster.key.route_family.contains("startup") && mentions_any(cluster, &["unbounded_roster_scan", "rich_preview", "mutable_state", "registry_scan", "oversized_api_agents", "api/agents"])
}

fn source_of_truth_ambiguity(cluster: &KernelSentinelIncidentCluster) -> bool {
    let rows: Vec<String> = cluster.evidence_refs.iter().chain(cluster.summaries.iter()).map(|row| row.to_ascii_lowercase()).collect();
    let domains = [
        rows.iter().any(|row| row.contains("layer://shell") || row.contains("taskbar") || row.contains("shell_state")),
        rows.iter().any(|row| row.contains("gateway://") || row.contains("api/agents") || row.contains("healthz")),
        rows.iter().any(|row| row.contains("layer://kernel") || row.contains("lifecycle=") || row.contains("runtime_failed")),
        rows.iter().any(|row| row.contains("watchdog")),
        rows.iter().any(|row| row.contains("listener://") || row.contains("healthz") || row.contains("api health")),
    ];
    let available = rows.iter().any(|row| ["ready", "healthy", "success", "active", "available"].iter().any(|needle| row.contains(needle)));
    let unavailable = rows.iter().any(|row| ["offline", "not ready", "unavailable", "failed", "absent", "stale_duplicate"].iter().any(|needle| row.contains(needle)));
    domains.into_iter().filter(|present| *present).count() >= 3 && available && unavailable
}

fn stop_patching_reasons(
    cluster: &KernelSentinelIncidentCluster,
    affected_layers: &[String],
) -> Vec<String> {
    let mut reasons = Vec::new();
    if affected_layers.len() >= 3 {
        reasons.push("failures_span_three_or_more_layers".to_string());
    }
    if command_success_runtime_contradiction(cluster) {
        reasons.push("command_output_contradicts_runtime_observation".to_string());
    }
    if process_ownership_invariant_breach(cluster) {
        reasons.push("process_ownership_invariant_breach".to_string());
    }
    if boot_route_dependency_risk(cluster) {
        reasons.push("boot_route_dependency_risk".to_string());
    }
    if source_of_truth_ambiguity(cluster) {
        reasons.push("source_of_truth_ambiguity".to_string());
    }
    if mentions_any(
        cluster,
        &[
            "local_fix_failed",
            "local patch failed",
            "symptom patch failed",
            "patching did not resolve",
            "fix did not resolve",
            "architectural contradiction remains",
        ],
    ) || matches!(
        cluster.highest_failure_level,
        KernelSentinelFailureLevel::L4ArchitecturalMisalignment
            | KernelSentinelFailureLevel::L5SelfModelFailure
    ) {
        reasons.push("local_fixes_do_not_resolve_architectural_contradiction".to_string());
    }
    reasons
}

fn remediation_class(
    cluster: &KernelSentinelIncidentCluster,
    affected_layers: &[String],
    stop_patching_reasons: &[String],
) -> &'static str {
    match cluster.highest_failure_level {
        KernelSentinelFailureLevel::L5SelfModelFailure => "self_model_repair",
        KernelSentinelFailureLevel::L4ArchitecturalMisalignment
        | KernelSentinelFailureLevel::L2BoundaryContractBreach => "structural_fix",
        KernelSentinelFailureLevel::L3PolicyTruthFailure => "policy_realignment",
        KernelSentinelFailureLevel::L0LocalDefect
        | KernelSentinelFailureLevel::L1ComponentRegression => {
            if affected_layers.len() >= 2 || !stop_patching_reasons.is_empty() {
                "structural_fix"
            } else {
                "symptom_patch"
            }
        }
    }
}

pub fn synthesize_kernel_sentinel_architectural_incidents(
    clusters: &[KernelSentinelIncidentCluster],
) -> Vec<KernelSentinelArchitecturalIncident> {
    clusters
        .iter()
        .map(|cluster| {
            let affected_layers = affected_layers(cluster);
            let stop_patching_reasons = stop_patching_reasons(cluster, &affected_layers);
            let failure_level = if source_of_truth_ambiguity(cluster) && cluster.highest_failure_level < KernelSentinelFailureLevel::L3PolicyTruthFailure { KernelSentinelFailureLevel::L3PolicyTruthFailure } else if process_ownership_invariant_breach(cluster) && cluster.highest_failure_level < KernelSentinelFailureLevel::L2BoundaryContractBreach { KernelSentinelFailureLevel::L2BoundaryContractBreach } else { cluster.highest_failure_level };
            let remediation_class = remediation_class(&KernelSentinelIncidentCluster { highest_failure_level: failure_level, ..cluster.clone() }, &affected_layers, &stop_patching_reasons);
            KernelSentinelArchitecturalIncident {
                cluster_key: cluster.key.clone(),
                occurrence_count: cluster.occurrence_count,
                violated_invariants: violated_invariants(cluster),
                likely_root_frame: root_frame_for_failure_level(failure_level).to_string(),
                confidence_percent: confidence_percent(cluster),
                affected_layers,
                remediation_class: remediation_class.to_string(),
                stop_patching: !stop_patching_reasons.is_empty(),
                stop_patching_reasons,
                failure_level,
                evidence_levels: cluster.evidence_levels.clone(),
                evidence_refs: cluster.evidence_refs.clone(),
                summaries: cluster.summaries.clone(),
            }
        })
        .collect()
}

fn structural_remediation(incident: &KernelSentinelArchitecturalIncident) -> &'static str {
    match incident.remediation_class.as_str() {
        "self_model_repair" => "repair the system self-model before further local patching; update the invariant, dossier, and evidence model that caused Sentinel to misunderstand the failure",
        "policy_realignment" => "realign the authority or policy source of truth, then prove Shell, Gateway, Orchestration, and Kernel projections agree on the same runtime law",
        "structural_fix" => "fix the boundary, lifecycle, ownership, or routing structure that allows the symptom to recur across layers",
        _ => "apply a local symptom patch only after proving the incident is not a boundary, policy, architecture, or self-model failure",
    }
}

fn local_patching_insufficiency(incident: &KernelSentinelArchitecturalIncident) -> String {
    if incident.stop_patching {
        format!("STOP_PATCHING is active because {}. Local edits are insufficient until the architectural contradiction is resolved at the owning boundary.", incident.stop_patching_reasons.join(", "))
    } else if incident.affected_layers.len() > 1 {
        format!("The incident spans {} layers, so a single local edit may mask drift between authorities rather than remove the cause.", incident.affected_layers.len())
    } else {
        "Local patching is allowed only with regression evidence proving the failure remains local to the cited layer and invariant.".to_string()
    }
}

fn architectural_acceptance_criteria(incident: &KernelSentinelArchitecturalIncident) -> Vec<String> {
    vec![
        format!("Kernel Sentinel no longer emits root_frame={} for this incident family", incident.likely_root_frame),
        format!("violated invariants are either satisfied or explicitly waived: {}", incident.violated_invariants.join(", ")),
        format!("affected layers publish consistent authoritative receipts: {}", incident.affected_layers.join(", ")),
        "regression fixture covers the original evidence refs and failure level".to_string(),
        "remediation proof includes rollback or recovery behavior for the owning boundary".to_string(),
    ]
}

pub fn kernel_sentinel_architectural_issue_template(
    incident: &KernelSentinelArchitecturalIncident,
) -> Value {
    json!({
        "type": "kernel_sentinel_architectural_issue_template",
        "status": "draft",
        "title": format!(
            "[Kernel Sentinel] {}: {}",
            incident.likely_root_frame,
            incident.violated_invariants.first().map(String::as_str).unwrap_or("unknown_invariant")
        ),
        "failure_level": incident.failure_level.code(),
        "root_frame": incident.likely_root_frame,
        "remediation_class": incident.remediation_class,
        "violated_invariants": incident.violated_invariants,
        "affected_layers": incident.affected_layers,
        "confidence_percent": incident.confidence_percent,
        "stop_patching": incident.stop_patching,
        "stop_patching_reasons": incident.stop_patching_reasons,
        "why_local_patching_is_insufficient": local_patching_insufficiency(incident),
        "structural_remediation": structural_remediation(incident),
        "acceptance_criteria": architectural_acceptance_criteria(incident),
        "evidence": incident.evidence_refs,
        "summaries": incident.summaries
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel_sentinel::{
        cluster_kernel_sentinel_incident_events, KernelSentinelIncidentEvent,
        KernelSentinelIncidentEvidenceLevel, KERNEL_SENTINEL_INCIDENT_EVENT_SCHEMA_VERSION,
        validate_kernel_sentinel_incident_event,
    };
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct ShellGatewayLifecycleFixture {
        fixture_id: String,
        required_signals: Vec<String>,
        events: Vec<KernelSentinelIncidentEvent>,
    }

    fn event(
        id: &str,
        layer: &str,
        level: KernelSentinelIncidentEvidenceLevel,
    ) -> KernelSentinelIncidentEvent {
        KernelSentinelIncidentEvent {
            schema_version: KERNEL_SENTINEL_INCIDENT_EVENT_SCHEMA_VERSION,
            id: id.to_string(),
            evidence_level: level,
            observed_at: "2026-04-29T06:31:00Z".to_string(),
            source: "synthesis_fixture".to_string(),
            affected_layer: layer.to_string(),
            component: "dashboard_host".to_string(),
            boundary: "shell_gateway_lifecycle".to_string(),
            policy: "watchdog_process_lifecycle".to_string(),
            architecture_scope: "runtime_topology".to_string(),
            self_model_scope: "sentinel_understanding".to_string(),
            invariant_id: "watchdog_owns_process_uniqueness_and_stale_host_cleanup".to_string(),
            failure_level: level.failure_floor(),
            route_family: "gateway_startup".to_string(),
            process_identity: "dashboard:4173".to_string(),
            lifecycle_state: "stale_duplicate".to_string(),
            evidence_refs: vec![format!("layer://{layer}/evidence/{id}")],
            summary: format!("summary for {id}"),
        }
    }

    #[test]
    fn architectural_synthesis_maps_cluster_to_root_frame_and_remediation() {
        let clusters = cluster_kernel_sentinel_incident_events(
            &[
                event("a", "gateway", KernelSentinelIncidentEvidenceLevel::Policy),
                event("b", "gateway", KernelSentinelIncidentEvidenceLevel::Architecture),
            ],
            60,
        );
        let incidents = synthesize_kernel_sentinel_architectural_incidents(&clusters);
        assert_eq!(incidents.len(), 1);
        let incident = &incidents[0];
        assert_eq!(incident.violated_invariants, vec!["watchdog_owns_process_uniqueness_and_stale_host_cleanup"]);
        assert_eq!(incident.likely_root_frame, "architectural_shape_mismatch");
        assert_eq!(incident.remediation_class, "structural_fix");
        assert!(incident.stop_patching);
        assert_eq!(incident.stop_patching_reasons, vec!["local_fixes_do_not_resolve_architectural_contradiction"]);
        assert_eq!(incident.failure_level, KernelSentinelFailureLevel::L4ArchitecturalMisalignment);
        assert!(incident.confidence_percent >= 80);
        assert_eq!(incident.affected_layers, vec!["gateway"]);
    }

    #[test]
    fn architectural_synthesis_keeps_distinct_layers_as_distinct_incidents() {
        let clusters = cluster_kernel_sentinel_incident_events(
            &[
                event("a", "gateway", KernelSentinelIncidentEvidenceLevel::Boundary),
                event("b", "shell", KernelSentinelIncidentEvidenceLevel::Boundary),
            ],
            60,
        );
        let incidents = synthesize_kernel_sentinel_architectural_incidents(&clusters);
        assert_eq!(incidents.len(), 2);
        assert!(incidents.iter().any(|incident| incident.affected_layers == vec!["gateway"]));
        assert!(incidents.iter().any(|incident| incident.affected_layers == vec!["shell"]));
    }

    #[test]
    fn detects_command_success_contradicting_listener_process_lifecycle_state() {
        let mut cluster = cluster_kernel_sentinel_incident_events(
            &[event("a", "gateway", KernelSentinelIncidentEvidenceLevel::Event)],
            60,
        )
        .remove(0);
        cluster.evidence_refs.extend([
            "layer://shell/taskbar_offline".to_string(),
            "layer://kernel/watchdog_state".to_string(),
            "command://infring_gateway_restart/status=success".to_string(),
            "listener://dashboard:4173/state=listener_absent".to_string(),
            "process://dashboard_host/lifecycle=stale_duplicate".to_string(),
        ]);
        cluster.summaries.push(
            "local patch failed; restart command reported success while runtime was not ready".to_string(),
        );

        let incident = synthesize_kernel_sentinel_architectural_incidents(&[cluster])
            .remove(0);
        assert!(incident.stop_patching);
        assert_eq!(incident.stop_patching_reasons, vec!["failures_span_three_or_more_layers", "command_output_contradicts_runtime_observation", "process_ownership_invariant_breach", "source_of_truth_ambiguity", "local_fixes_do_not_resolve_architectural_contradiction"]);
        assert_eq!(incident.failure_level, KernelSentinelFailureLevel::L3PolicyTruthFailure);
        assert_eq!(incident.likely_root_frame, "authority_policy_contradiction");
        assert_eq!(incident.affected_layers, vec!["gateway", "kernel", "shell"]);
    }

    #[test]
    fn remediation_classifier_distinguishes_all_required_classes() {
        let cases = [
            (KernelSentinelIncidentEvidenceLevel::Event, "symptom_patch"),
            (KernelSentinelIncidentEvidenceLevel::Boundary, "structural_fix"),
            (KernelSentinelIncidentEvidenceLevel::Policy, "policy_realignment"),
            (KernelSentinelIncidentEvidenceLevel::SelfModel, "self_model_repair"),
        ];

        for (idx, (level, expected)) in cases.into_iter().enumerate() {
            let mut incident_event = event(&format!("case-{idx}"), "gateway", level);
            incident_event.invariant_id = format!("remediation_classifier_case_{idx}");
            let cluster = cluster_kernel_sentinel_incident_events(&[incident_event], 60)
                .remove(0);
            let incident = synthesize_kernel_sentinel_architectural_incidents(&[cluster])
                .remove(0);
            assert_eq!(incident.remediation_class, expected);
        }
    }

    #[test]
    fn architectural_issue_template_carries_multilevel_diagnosis() {
        let mut cluster = cluster_kernel_sentinel_incident_events(
            &[
                event("a", "gateway", KernelSentinelIncidentEvidenceLevel::Policy),
                event("b", "gateway", KernelSentinelIncidentEvidenceLevel::Architecture),
            ],
            60,
        )
        .remove(0);
        cluster.evidence_refs.extend([
            "layer://shell/taskbar_offline".to_string(),
            "layer://kernel/watchdog_state".to_string(),
            "command_output_contradicts_runtime://gateway_restart_success_without_listener"
                .to_string(),
        ]);
        let incident = synthesize_kernel_sentinel_architectural_incidents(&[cluster])
            .remove(0);
        let template = kernel_sentinel_architectural_issue_template(&incident);

        assert_eq!(template["type"], "kernel_sentinel_architectural_issue_template");
        assert_eq!(template["failure_level"], "L4_architectural_misalignment");
        assert_eq!(template["root_frame"], "architectural_shape_mismatch");
        assert_eq!(
            template["violated_invariants"][0],
            "watchdog_owns_process_uniqueness_and_stale_host_cleanup"
        );
        assert_eq!(template["affected_layers"].as_array().unwrap().len(), 3);
        assert_eq!(template["stop_patching"], true);
        assert!(
            template["why_local_patching_is_insufficient"]
                .as_str()
                .unwrap()
                .contains("STOP_PATCHING is active")
        );
        assert!(
            template["structural_remediation"]
                .as_str()
                .unwrap()
                .contains("fix the boundary")
        );
        assert!(template["acceptance_criteria"].as_array().unwrap().len() >= 5);
    }

    #[test]
    fn shell_gateway_lifecycle_golden_fixture_synthesizes_one_architectural_incident() {
        let fixture: ShellGatewayLifecycleFixture = serde_json::from_str(include_str!(
            "golden/shell_gateway_lifecycle_incident.json"
        ))
        .expect("parse shell/gateway lifecycle golden fixture");
        assert_eq!(
            fixture.fixture_id,
            "kernel_sentinel_shell_gateway_lifecycle_regression_v1"
        );
        for required_signal in [
            "escaped_chat_metadata_outside_bubble",
            "offline_taskbar_connectivity_indicator",
            "hanging_oversized_api_agents",
            "dropped_query_semantics",
            "false_restart_success",
            "stale_duplicate_dashboard_hosts",
        ] {
            assert!(fixture.required_signals.iter().any(|row| row == required_signal));
        }
        for event in &fixture.events {
            validate_kernel_sentinel_incident_event(event).expect("valid fixture event");
        }

        let clusters = cluster_kernel_sentinel_incident_events(&fixture.events, 60);
        assert_eq!(clusters.len(), 1);
        let incident = synthesize_kernel_sentinel_architectural_incidents(&clusters).remove(0);
        assert!(
            matches!(incident.failure_level, KernelSentinelFailureLevel::L3PolicyTruthFailure | KernelSentinelFailureLevel::L4ArchitecturalMisalignment)
                && !matches!(incident.failure_level, KernelSentinelFailureLevel::L0LocalDefect | KernelSentinelFailureLevel::L1ComponentRegression),
            "Shell/Gateway lifecycle fixture must classify as L3/L4 and never L0/L1"
        );
        assert_eq!(incident.failure_level, KernelSentinelFailureLevel::L4ArchitecturalMisalignment);
        assert_eq!(incident.likely_root_frame, "architectural_shape_mismatch");
        assert_eq!(incident.remediation_class, "structural_fix");
        assert!(incident.stop_patching);
        assert!(incident.stop_patching_reasons.iter().any(|reason| reason == "boot_route_dependency_risk"));
        assert!(incident.stop_patching_reasons.iter().any(|reason| reason == "source_of_truth_ambiguity"));
        assert_eq!(incident.affected_layers, vec!["gateway", "kernel", "shell"]);
        assert_eq!(incident.occurrence_count, 6);
        assert!(
            incident
                .summaries
                .iter()
                .any(|summary| summary.contains("hanging_oversized_api_agents"))
        );
    }
}
