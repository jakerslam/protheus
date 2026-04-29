use super::*;
use crate::kernel_sentinel::{
    cluster_kernel_sentinel_incident_events, validate_kernel_sentinel_incident_event,
    KernelSentinelIncidentEvent, KernelSentinelIncidentEvidenceLevel,
    KERNEL_SENTINEL_INCIDENT_EVENT_SCHEMA_VERSION,
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
    assert_eq!(
        incident.violated_invariants,
        vec!["watchdog_owns_process_uniqueness_and_stale_host_cleanup"]
    );
    assert_eq!(incident.likely_root_frame, "architectural_shape_mismatch");
    assert_eq!(incident.remediation_class, "structural_fix");
    assert!(incident.stop_patching);
    assert_eq!(
        incident.stop_patching_reasons,
        vec!["local_fixes_do_not_resolve_architectural_contradiction"]
    );
    assert_eq!(
        incident.failure_level,
        KernelSentinelFailureLevel::L4ArchitecturalMisalignment
    );
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
    assert!(
        incidents
            .iter()
            .any(|incident| incident.affected_layers == vec!["gateway"])
    );
    assert!(
        incidents
            .iter()
            .any(|incident| incident.affected_layers == vec!["shell"])
    );
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
        "local patch failed; restart command reported success while runtime was not ready"
            .to_string(),
    );

    let incident = synthesize_kernel_sentinel_architectural_incidents(&[cluster]).remove(0);
    assert!(incident.stop_patching);
    assert_eq!(
        incident.stop_patching_reasons,
        vec![
            "failures_span_three_or_more_layers",
            "command_output_contradicts_runtime_observation",
            "process_ownership_invariant_breach",
            "source_of_truth_ambiguity",
            "local_fixes_do_not_resolve_architectural_contradiction"
        ]
    );
    assert_eq!(
        incident.failure_level,
        KernelSentinelFailureLevel::L3PolicyTruthFailure
    );
    assert_eq!(incident.likely_root_frame, "authority_policy_contradiction");
    assert_eq!(incident.affected_layers, vec!["gateway", "kernel", "shell"]);
}

#[test]
fn detects_authority_shaped_residue_when_removed_authority_can_re_emerge() {
    let mut cluster = cluster_kernel_sentinel_incident_events(
        &[event("a", "gateway", KernelSentinelIncidentEvidenceLevel::Boundary)],
        60,
    )
    .remove(0);
    cluster.evidence_refs.extend([
        "shape://gateway/data_shape=authority_payload".to_string(),
        "shape://shell/lifecycle_affordance=restart_affordance".to_string(),
        "route://gateway/fallback_path=legacy_fallback_used".to_string(),
        "shim://orchestration/compatibility_shim=legacy_intent_compatibility_shim".to_string(),
    ]);
    cluster.summaries.push(
        "removed authority syntax but retained authority shape, allowing truth leak to re-emerge"
            .to_string(),
    );

    let incident = synthesize_kernel_sentinel_architectural_incidents(&[cluster]).remove(0);
    assert!(incident.stop_patching_reasons.iter().any(|r| r == "authority_shaped_residue"));
    assert_eq!(
        incident.failure_level,
        KernelSentinelFailureLevel::L3PolicyTruthFailure
    );
    assert_eq!(incident.likely_root_frame, "authority_policy_contradiction");
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
        let cluster = cluster_kernel_sentinel_incident_events(&[incident_event], 60).remove(0);
        let incident = synthesize_kernel_sentinel_architectural_incidents(&[cluster]).remove(0);
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
        "command_output_contradicts_runtime://gateway_restart_success_without_listener".to_string(),
    ]);
    let incident = synthesize_kernel_sentinel_architectural_incidents(&[cluster]).remove(0);
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
    let fixture: ShellGatewayLifecycleFixture =
        serde_json::from_str(include_str!("golden/shell_gateway_lifecycle_incident.json"))
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
        matches!(
            incident.failure_level,
            KernelSentinelFailureLevel::L3PolicyTruthFailure
                | KernelSentinelFailureLevel::L4ArchitecturalMisalignment
        ) && !matches!(
            incident.failure_level,
            KernelSentinelFailureLevel::L0LocalDefect
                | KernelSentinelFailureLevel::L1ComponentRegression
        ),
        "Shell/Gateway lifecycle fixture must classify as L3/L4 and never L0/L1"
    );
    assert_eq!(
        incident.failure_level,
        KernelSentinelFailureLevel::L4ArchitecturalMisalignment
    );
    assert_eq!(incident.likely_root_frame, "architectural_shape_mismatch");
    assert_eq!(incident.remediation_class, "structural_fix");
    assert!(incident.stop_patching);
    assert!(
        incident
            .stop_patching_reasons
            .iter()
            .any(|reason| reason == "boot_route_dependency_risk")
    );
    assert!(
        incident
            .stop_patching_reasons
            .iter()
            .any(|reason| reason == "source_of_truth_ambiguity")
    );
    assert_eq!(incident.affected_layers, vec!["gateway", "kernel", "shell"]);
    assert_eq!(incident.occurrence_count, 6);
    assert!(
        incident
            .summaries
            .iter()
            .any(|summary| summary.contains("hanging_oversized_api_agents"))
    );
}

#[test]
fn authority_shape_residue_golden_fixture_flags_policy_truth_risk() {
    let fixture: ShellGatewayLifecycleFixture =
        serde_json::from_str(include_str!("golden/authority_shape_residue_incident.json"))
            .expect("parse authority-shape residue golden fixture");
    assert_eq!(
        fixture.fixture_id,
        "kernel_sentinel_authority_shape_residue_regression_v1"
    );
    for required_signal in [
        "removed_authority_syntax_retained_authority_shape",
        "data_shape_authority_payload",
        "lifecycle_affordance_restart_path",
        "legacy_fallback_path",
        "compatibility_shim_authority_residue",
    ] {
        assert!(fixture.required_signals.iter().any(|row| row == required_signal));
    }
    for event in &fixture.events {
        validate_kernel_sentinel_incident_event(event).expect("valid residue fixture event");
    }

    let clusters = cluster_kernel_sentinel_incident_events(&fixture.events, 60);
    assert_eq!(clusters.len(), 1);
    let incident = synthesize_kernel_sentinel_architectural_incidents(&clusters).remove(0);
    assert_eq!(
        incident.failure_level,
        KernelSentinelFailureLevel::L3PolicyTruthFailure
    );
    assert_eq!(incident.likely_root_frame, "authority_policy_contradiction");
    assert!(
        incident
            .stop_patching_reasons
            .iter()
            .any(|reason| reason == "authority_shaped_residue"),
        "fixture must emit authority_shaped_residue"
    );
}
