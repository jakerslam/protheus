// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use super::{
    KernelSentinelFailureLevel, KernelSentinelIncidentEvent,
    KernelSentinelIncidentEvidenceLevel,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct KernelSentinelIncidentClusterKey {
    pub time_window: String,
    pub affected_layer: String,
    pub invariant_family: String,
    pub process_identity: String,
    pub route_family: String,
    pub lifecycle_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelSentinelIncidentCluster {
    pub key: KernelSentinelIncidentClusterKey,
    pub occurrence_count: usize,
    pub incident_ids: Vec<String>,
    pub invariant_ids: Vec<String>,
    pub evidence_levels: Vec<KernelSentinelIncidentEvidenceLevel>,
    pub highest_failure_level: KernelSentinelFailureLevel,
    pub first_observed_at: String,
    pub last_observed_at: String,
    pub evidence_refs: Vec<String>,
    pub summaries: Vec<String>,
}

fn parse_digits(raw: &str, start: usize, end: usize) -> Option<i64> {
    raw.get(start..end)?.parse::<i64>().ok()
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * month + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn parse_observed_at_epoch_seconds(raw: &str) -> Option<i64> {
    let year = parse_digits(raw, 0, 4)?;
    let month = parse_digits(raw, 5, 7)?;
    let day = parse_digits(raw, 8, 10)?;
    let hour = parse_digits(raw, 11, 13)?;
    let minute = parse_digits(raw, 14, 16)?;
    let second = parse_digits(raw, 17, 19)?;
    Some(days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 + second)
}

fn time_window_label(observed_at: &str, window_seconds: i64) -> String {
    let window_seconds = window_seconds.max(1);
    match parse_observed_at_epoch_seconds(observed_at) {
        Some(epoch) => {
            let start = epoch - epoch.rem_euclid(window_seconds);
            format!("epoch_seconds:{start}:window_seconds:{window_seconds}")
        }
        None => format!("unparsed:{}:window_seconds:{window_seconds}", observed_at.trim()),
    }
}

fn invariant_family(invariant_id: &str) -> String {
    let trimmed = invariant_id.trim();
    for delimiter in ["::", ":", "/"] {
        if let Some((family, _)) = trimmed.split_once(delimiter) {
            let family = family.trim();
            if !family.is_empty() {
                return family.to_string();
            }
        }
    }
    let family = trimmed.split('_').next().unwrap_or("").trim();
    if family.is_empty() {
        "unknown_invariant_family".to_string()
    } else {
        family.to_string()
    }
}

fn cluster_key(
    event: &KernelSentinelIncidentEvent,
    window_seconds: i64,
) -> KernelSentinelIncidentClusterKey {
    KernelSentinelIncidentClusterKey {
        time_window: time_window_label(&event.observed_at, window_seconds),
        affected_layer: event.affected_layer.trim().to_string(),
        invariant_family: invariant_family(&event.invariant_id),
        process_identity: event.process_identity.trim().to_string(),
        route_family: event.route_family.trim().to_string(),
        lifecycle_state: event.lifecycle_state.trim().to_string(),
    }
}

fn empty_cluster(
    key: KernelSentinelIncidentClusterKey,
    event: &KernelSentinelIncidentEvent,
) -> KernelSentinelIncidentCluster {
    KernelSentinelIncidentCluster {
        key,
        occurrence_count: 0,
        incident_ids: Vec::new(),
        invariant_ids: Vec::new(),
        evidence_levels: Vec::new(),
        highest_failure_level: event.failure_level,
        first_observed_at: event.observed_at.clone(),
        last_observed_at: event.observed_at.clone(),
        evidence_refs: Vec::new(),
        summaries: Vec::new(),
    }
}

fn insert_unique<T>(rows: &mut Vec<T>, seen: &mut BTreeSet<T>, value: T)
where
    T: Ord + Clone,
{
    if seen.insert(value.clone()) {
        rows.push(value);
    }
}

pub fn cluster_kernel_sentinel_incident_events(
    events: &[KernelSentinelIncidentEvent],
    window_seconds: i64,
) -> Vec<KernelSentinelIncidentCluster> {
    let mut clusters = BTreeMap::<KernelSentinelIncidentClusterKey, KernelSentinelIncidentCluster>::new();
    let mut cluster_invariant_seen = BTreeMap::<KernelSentinelIncidentClusterKey, BTreeSet<String>>::new();
    let mut cluster_level_seen = BTreeMap::<KernelSentinelIncidentClusterKey, BTreeSet<KernelSentinelIncidentEvidenceLevel>>::new();
    let mut cluster_evidence_seen = BTreeMap::<KernelSentinelIncidentClusterKey, BTreeSet<String>>::new();
    let mut cluster_summary_seen = BTreeMap::<KernelSentinelIncidentClusterKey, BTreeSet<String>>::new();

    for event in events {
        let key = cluster_key(event, window_seconds);
        let cluster = clusters
            .entry(key.clone())
            .or_insert_with(|| empty_cluster(key.clone(), event));
        cluster.occurrence_count += 1;
        cluster.incident_ids.push(event.id.clone());
        let invariant_ids = cluster_invariant_seen.entry(key.clone()).or_default();
        insert_unique(&mut cluster.invariant_ids, invariant_ids, event.invariant_id.clone());
        if event.failure_level > cluster.highest_failure_level {
            cluster.highest_failure_level = event.failure_level;
        }
        if event.observed_at < cluster.first_observed_at {
            cluster.first_observed_at = event.observed_at.clone();
        }
        if event.observed_at > cluster.last_observed_at {
            cluster.last_observed_at = event.observed_at.clone();
        }

        let levels = cluster_level_seen.entry(key.clone()).or_default();
        insert_unique(&mut cluster.evidence_levels, levels, event.evidence_level);

        let evidence = cluster_evidence_seen.entry(key.clone()).or_default();
        for evidence_ref in &event.evidence_refs {
            insert_unique(&mut cluster.evidence_refs, evidence, evidence_ref.clone());
        }

        let summaries = cluster_summary_seen.entry(key).or_default();
        insert_unique(&mut cluster.summaries, summaries, event.summary.clone());
    }

    clusters.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel_sentinel::{
        KERNEL_SENTINEL_INCIDENT_EVENT_SCHEMA_VERSION, KernelSentinelIncidentEvidenceLevel,
    };

    fn event(
        id: &str,
        observed_at: &str,
        layer: &str,
        invariant_id: &str,
        process: &str,
        route: &str,
        lifecycle: &str,
        level: KernelSentinelIncidentEvidenceLevel,
    ) -> KernelSentinelIncidentEvent {
        KernelSentinelIncidentEvent {
            schema_version: KERNEL_SENTINEL_INCIDENT_EVENT_SCHEMA_VERSION,
            id: id.to_string(),
            evidence_level: level,
            observed_at: observed_at.to_string(),
            source: "cluster_fixture".to_string(),
            affected_layer: layer.to_string(),
            component: "dashboard_host".to_string(),
            boundary: "shell_gateway_lifecycle".to_string(),
            policy: "watchdog_process_lifecycle".to_string(),
            architecture_scope: "runtime_topology".to_string(),
            self_model_scope: "sentinel_understanding".to_string(),
            invariant_id: invariant_id.to_string(),
            failure_level: level.failure_floor(),
            route_family: route.to_string(),
            process_identity: process.to_string(),
            lifecycle_state: lifecycle.to_string(),
            evidence_refs: vec![format!("evidence://{id}")],
            summary: format!("summary for {id}"),
        }
    }

    #[test]
    fn clusters_symptoms_by_window_layer_invariant_process_route_and_lifecycle() {
        let events = vec![
            event(
                "a",
                "2026-04-29T06:20:10Z",
                "gateway",
                "watchdog_owns_process_uniqueness_and_stale_host_cleanup",
                "dashboard:4173",
                "gateway_startup",
                "stale_duplicate",
                KernelSentinelIncidentEvidenceLevel::Boundary,
            ),
            event(
                "b",
                "2026-04-29T06:20:45Z",
                "gateway",
                "watchdog_owns_process_uniqueness_and_stale_host_cleanup",
                "dashboard:4173",
                "gateway_startup",
                "stale_duplicate",
                KernelSentinelIncidentEvidenceLevel::Policy,
            ),
            event(
                "c",
                "2026-04-29T06:21:02Z",
                "shell",
                "shell_connectivity_uses_authoritative_runtime_state",
                "taskbar",
                "shell_status",
                "offline_projection",
                KernelSentinelIncidentEvidenceLevel::Component,
            ),
        ];

        let clusters = cluster_kernel_sentinel_incident_events(&events, 60);
        assert_eq!(clusters.len(), 2);
        let gateway = clusters
            .iter()
            .find(|cluster| cluster.key.affected_layer == "gateway")
            .unwrap();
        assert_eq!(gateway.occurrence_count, 2);
        assert_eq!(
            gateway.invariant_ids,
            vec!["watchdog_owns_process_uniqueness_and_stale_host_cleanup"]
        );
        assert_eq!(gateway.key.invariant_family, "watchdog");
        assert_eq!(gateway.key.process_identity, "dashboard:4173");
        assert_eq!(gateway.key.route_family, "gateway_startup");
        assert_eq!(gateway.key.lifecycle_state, "stale_duplicate");
        assert_eq!(
            gateway.highest_failure_level,
            KernelSentinelFailureLevel::L3PolicyTruthFailure
        );
        assert_eq!(
            gateway.evidence_levels,
            vec![
                KernelSentinelIncidentEvidenceLevel::Boundary,
                KernelSentinelIncidentEvidenceLevel::Policy
            ]
        );
    }

    #[test]
    fn time_window_boundary_splits_otherwise_matching_symptoms() {
        let events = vec![
            event(
                "a",
                "2026-04-29T06:20:59Z",
                "gateway",
                "watchdog_owns_process_uniqueness_and_stale_host_cleanup",
                "dashboard:4173",
                "gateway_startup",
                "stale_duplicate",
                KernelSentinelIncidentEvidenceLevel::Boundary,
            ),
            event(
                "b",
                "2026-04-29T06:21:00Z",
                "gateway",
                "watchdog_owns_process_uniqueness_and_stale_host_cleanup",
                "dashboard:4173",
                "gateway_startup",
                "stale_duplicate",
                KernelSentinelIncidentEvidenceLevel::Boundary,
            ),
        ];
        assert_eq!(cluster_kernel_sentinel_incident_events(&events, 60).len(), 2);
        assert_eq!(cluster_kernel_sentinel_incident_events(&events, 120).len(), 1);
    }
}
