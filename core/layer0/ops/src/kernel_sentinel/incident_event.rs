// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::KernelSentinelFailureLevel;

pub const KERNEL_SENTINEL_INCIDENT_EVENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelSentinelIncidentEvidenceLevel {
    Event,
    Component,
    Boundary,
    Policy,
    Architecture,
    SelfModel,
}

pub const KERNEL_SENTINEL_INCIDENT_EVIDENCE_LEVELS: [KernelSentinelIncidentEvidenceLevel; 6] = [
    KernelSentinelIncidentEvidenceLevel::Event,
    KernelSentinelIncidentEvidenceLevel::Component,
    KernelSentinelIncidentEvidenceLevel::Boundary,
    KernelSentinelIncidentEvidenceLevel::Policy,
    KernelSentinelIncidentEvidenceLevel::Architecture,
    KernelSentinelIncidentEvidenceLevel::SelfModel,
];

impl KernelSentinelIncidentEvidenceLevel {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Event => "event_level",
            Self::Component => "component_level",
            Self::Boundary => "boundary_level",
            Self::Policy => "policy_level",
            Self::Architecture => "architecture_level",
            Self::SelfModel => "self_model_level",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Event => "Event-level evidence",
            Self::Component => "Component-level evidence",
            Self::Boundary => "Boundary-level evidence",
            Self::Policy => "Policy-level evidence",
            Self::Architecture => "Architecture-level evidence",
            Self::SelfModel => "Self-model-level evidence",
        }
    }

    pub const fn captures(self) -> &'static str {
        match self {
            Self::Event => "single observation, command output, receipt, probe, or log line",
            Self::Component => "repeated evidence localized to one component or runtime unit",
            Self::Boundary => "evidence that two surfaces disagree across a declared boundary",
            Self::Policy => "evidence that runtime behavior contradicts declared authority or policy",
            Self::Architecture => "evidence that system shape makes local fixes recur or fail",
            Self::SelfModel => "evidence that the system misunderstood itself or its remediation level",
        }
    }

    pub const fn failure_floor(self) -> KernelSentinelFailureLevel {
        match self {
            Self::Event => KernelSentinelFailureLevel::L0LocalDefect,
            Self::Component => KernelSentinelFailureLevel::L1ComponentRegression,
            Self::Boundary => KernelSentinelFailureLevel::L2BoundaryContractBreach,
            Self::Policy => KernelSentinelFailureLevel::L3PolicyTruthFailure,
            Self::Architecture => KernelSentinelFailureLevel::L4ArchitecturalMisalignment,
            Self::SelfModel => KernelSentinelFailureLevel::L5SelfModelFailure,
        }
    }

    pub const fn required_focus_field(self) -> &'static str {
        match self {
            Self::Event => "lifecycle_state",
            Self::Component => "component",
            Self::Boundary => "boundary",
            Self::Policy => "policy",
            Self::Architecture => "architecture_scope",
            Self::SelfModel => "self_model_scope",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelSentinelIncidentEvent {
    pub schema_version: u32,
    pub id: String,
    pub evidence_level: KernelSentinelIncidentEvidenceLevel,
    pub observed_at: String,
    pub source: String,
    pub affected_layer: String,
    pub component: String,
    pub boundary: String,
    pub policy: String,
    pub architecture_scope: String,
    pub self_model_scope: String,
    pub invariant_id: String,
    pub failure_level: KernelSentinelFailureLevel,
    pub route_family: String,
    pub process_identity: String,
    pub lifecycle_state: String,
    pub evidence_refs: Vec<String>,
    pub summary: String,
}

fn missing(value: &str) -> bool {
    value.trim().is_empty()
}

pub fn validate_kernel_sentinel_incident_event(
    event: &KernelSentinelIncidentEvent,
) -> Result<(), String> {
    if event.schema_version != KERNEL_SENTINEL_INCIDENT_EVENT_SCHEMA_VERSION {
        return Err("invalid_incident_event_schema_version".to_string());
    }
    for (field, value) in [
        ("id", event.id.as_str()),
        ("observed_at", event.observed_at.as_str()),
        ("source", event.source.as_str()),
        ("affected_layer", event.affected_layer.as_str()),
        ("invariant_id", event.invariant_id.as_str()),
        ("summary", event.summary.as_str()),
    ] {
        if missing(value) {
            return Err(format!("missing_{field}"));
        }
    }
    if event.evidence_refs.is_empty() || event.evidence_refs.iter().any(|row| missing(row)) {
        return Err("missing_evidence_refs".to_string());
    }
    let focus = event.evidence_level.required_focus_field();
    let focus_value = match event.evidence_level {
        KernelSentinelIncidentEvidenceLevel::Event => event.lifecycle_state.as_str(),
        KernelSentinelIncidentEvidenceLevel::Component => event.component.as_str(),
        KernelSentinelIncidentEvidenceLevel::Boundary => event.boundary.as_str(),
        KernelSentinelIncidentEvidenceLevel::Policy => event.policy.as_str(),
        KernelSentinelIncidentEvidenceLevel::Architecture => event.architecture_scope.as_str(),
        KernelSentinelIncidentEvidenceLevel::SelfModel => event.self_model_scope.as_str(),
    };
    if missing(focus_value) {
        return Err(format!("missing_{focus}"));
    }
    if event.failure_level < event.evidence_level.failure_floor() {
        return Err(format!(
            "failure_level_below_{}_floor",
            event.evidence_level.code()
        ));
    }
    Ok(())
}

pub fn kernel_sentinel_incident_event_model() -> Value {
    json!({
        "type": "kernel_sentinel_incident_event_model",
        "schema_version": KERNEL_SENTINEL_INCIDENT_EVENT_SCHEMA_VERSION,
        "owner": "kernel",
        "purpose": "represent Sentinel evidence from local symptoms through self-model failures before clustering or remediation",
        "common_required_fields": [
            "id",
            "evidence_level",
            "observed_at",
            "source",
            "affected_layer",
            "invariant_id",
            "failure_level",
            "evidence_refs",
            "summary"
        ],
        "cluster_keys": [
            "observed_at_time_window",
            "affected_layer",
            "invariant_id",
            "failure_level",
            "route_family",
            "process_identity",
            "lifecycle_state"
        ],
        "levels": KERNEL_SENTINEL_INCIDENT_EVIDENCE_LEVELS
            .iter()
            .map(|level| {
                json!({
                    "code": level.code(),
                    "label": level.label(),
                    "captures": level.captures(),
                    "required_focus_field": level.required_focus_field(),
                    "minimum_failure_level": level.failure_floor().code(),
                    "minimum_remediation_level": level.failure_floor().remediation_level()
                })
            })
            .collect::<Vec<_>>()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(level: KernelSentinelIncidentEvidenceLevel) -> KernelSentinelIncidentEvent {
        KernelSentinelIncidentEvent {
            schema_version: KERNEL_SENTINEL_INCIDENT_EVENT_SCHEMA_VERSION,
            id: format!("incident-{}", level.code()),
            evidence_level: level,
            observed_at: "2026-04-29T06:20:00Z".to_string(),
            source: "kernel_sentinel_fixture".to_string(),
            affected_layer: "gateway".to_string(),
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
            evidence_refs: vec!["process_inventory://dashboard:4173".to_string()],
            summary: "duplicate dashboard host stayed alive after restart".to_string(),
        }
    }

    #[test]
    fn incident_event_model_lists_all_evidence_levels_in_order() {
        let model = kernel_sentinel_incident_event_model();
        let codes = model["levels"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["code"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            codes,
            vec![
                "event_level",
                "component_level",
                "boundary_level",
                "policy_level",
                "architecture_level",
                "self_model_level"
            ]
        );
        assert_eq!(
            model["cluster_keys"].as_array().unwrap().len(),
            7,
            "incident events must carry enough keys for cross-layer symptom clustering"
        );
    }

    #[test]
    fn incident_events_require_level_specific_focus_and_failure_floor() {
        for level in KERNEL_SENTINEL_INCIDENT_EVIDENCE_LEVELS {
            validate_kernel_sentinel_incident_event(&event(level)).unwrap();
        }

        let mut boundary = event(KernelSentinelIncidentEvidenceLevel::Boundary);
        boundary.boundary.clear();
        assert_eq!(
            validate_kernel_sentinel_incident_event(&boundary).unwrap_err(),
            "missing_boundary"
        );

        let mut architectural = event(KernelSentinelIncidentEvidenceLevel::Architecture);
        architectural.failure_level = KernelSentinelFailureLevel::L1ComponentRegression;
        assert_eq!(
            validate_kernel_sentinel_incident_event(&architectural).unwrap_err(),
            "failure_level_below_architecture_level_floor"
        );
    }
}
