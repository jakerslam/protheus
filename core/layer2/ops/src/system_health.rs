use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub const DEFAULT_SYSTEM_HEALTH_POLICY_PATH: &str =
    "core/layer2/ops/config/system_health_policy.json";
pub const DEFAULT_SYSTEM_HEALTH_REPORT_PATH: &str =
    "core/local/artifacts/system_health_status_current.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemHealthPolicy {
    pub version: String,
    pub owner: String,
    pub eval_failure_critical_threshold: u32,
    pub boundedness_regression_critical_score: u32,
    pub routing_anomaly_threshold: u32,
    pub retry_spike_threshold: u32,
    pub latency_anomaly_p95_ms: u64,
    pub adaptation: BoundedAdaptationPolicy,
    pub gateway_capabilities: Vec<GatewayCapabilityAnnouncement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BoundedAdaptationPolicy {
    pub require_eval_validation: bool,
    pub require_rollback_safety: bool,
    pub block_when_critical: bool,
    pub max_threshold_delta_percent: u32,
    pub max_retry_delta: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GatewayCapabilityAnnouncement {
    pub gateway_id: String,
    pub capabilities: Vec<String>,
    pub max_concurrent_requests: u32,
    pub max_payload_kb: u32,
    pub timeout_ms: u64,
    pub planning_graph_contributions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemSignals {
    pub snapshot_id: String,
    pub eval_failures: u32,
    pub boundedness_regression_score: u32,
    pub routing_anomaly_count: u32,
    pub retry_count_p95: u32,
    pub latency_p95_ms: u64,
    pub gateway_quarantined_count: u32,
    pub recovery_active: bool,
    pub recovery_successes: u32,
    pub unknown_failure_clusters: u32,
    pub eval_validation_passed: bool,
    pub rollback_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthAnomaly {
    pub anomaly_type: String,
    pub severity: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BoundedAdaptationDecision {
    pub allowed: bool,
    pub reason: String,
    pub threshold_delta_percent: u32,
    pub retry_delta: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemHealthStatus {
    pub snapshot_id: String,
    pub status: String,
    pub reasons: Vec<String>,
    pub anomalies: Vec<HealthAnomaly>,
    pub adaptation: BoundedAdaptationDecision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealthGuardReport {
    pub ok: bool,
    #[serde(rename = "type")]
    pub report_type: String,
    pub policy_path: String,
    pub summary: Value,
    pub checks: Vec<Value>,
    pub statuses: Vec<SystemHealthStatus>,
    pub gateway_capabilities: Vec<GatewayCapabilityAnnouncement>,
}

pub fn load_system_health_policy(path: &str) -> Result<SystemHealthPolicy, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("read_policy_failed:{err}"))?;
    serde_json::from_str(&raw).map_err(|err| format!("parse_policy_failed:{err}"))
}

pub fn default_system_signals() -> Vec<SystemSignals> {
    vec![
        signals("healthy", 0, 0, 0, 1, 100, 0, false, 0, 0, true, true),
        signals("degraded", 0, 12, 3, 9, 2_500, 1, false, 0, 0, true, true),
        signals("recovering", 0, 3, 0, 2, 250, 0, true, 2, 0, true, true),
        signals("critical", 2, 92, 8, 20, 9_000, 2, false, 0, 1, false, true),
        signals("rollback_blocked", 0, 8, 2, 7, 1_500, 0, false, 0, 0, true, false),
    ]
}

pub fn derive_system_health_status(
    policy: &SystemHealthPolicy,
    signals: &SystemSignals,
) -> SystemHealthStatus {
    let anomalies = detect_anomalies(policy, signals);
    let mut reasons = Vec::new();
    if signals.eval_failures >= policy.eval_failure_critical_threshold {
        reasons.push("eval_regression_signal".to_string());
    }
    if signals.boundedness_regression_score >= policy.boundedness_regression_critical_score {
        reasons.push("boundedness_regression_signal".to_string());
    }
    if signals.routing_anomaly_count >= policy.routing_anomaly_threshold {
        reasons.push("routing_anomaly_signal".to_string());
    }
    if signals.retry_count_p95 >= policy.retry_spike_threshold {
        reasons.push("retry_spike_signal".to_string());
    }
    if signals.latency_p95_ms >= policy.latency_anomaly_p95_ms {
        reasons.push("latency_anomaly_signal".to_string());
    }
    if signals.gateway_quarantined_count > 0 {
        reasons.push("gateway_quarantine_signal".to_string());
    }
    if signals.recovery_active || signals.recovery_successes > 0 {
        reasons.push("recovery_signal".to_string());
    }
    if signals.unknown_failure_clusters > 0 {
        reasons.push("unknown_failure_cluster_signal".to_string());
    }
    let critical = signals.eval_failures >= policy.eval_failure_critical_threshold
        || signals.boundedness_regression_score >= policy.boundedness_regression_critical_score
        || signals.unknown_failure_clusters > 0;
    let recovering = !critical && (signals.recovery_active || signals.recovery_successes > 0);
    let degraded = !critical
        && !recovering
        && (!anomalies.is_empty()
            || signals.gateway_quarantined_count > 0
            || signals.boundedness_regression_score > 0);
    let status = if critical {
        "critical"
    } else if recovering {
        "recovering"
    } else if degraded {
        "degraded"
    } else {
        "healthy"
    };
    SystemHealthStatus {
        snapshot_id: signals.snapshot_id.clone(),
        status: status.to_string(),
        reasons,
        anomalies,
        adaptation: decide_bounded_adaptation(policy, status, signals),
    }
}

pub fn detect_anomalies(
    policy: &SystemHealthPolicy,
    signals: &SystemSignals,
) -> Vec<HealthAnomaly> {
    let mut anomalies = Vec::new();
    if signals.routing_anomaly_count >= policy.routing_anomaly_threshold {
        anomalies.push(anomaly(
            "unusual_routing_patterns",
            "high",
            "routing_anomaly_threshold_exceeded",
        ));
    }
    if signals.retry_count_p95 >= policy.retry_spike_threshold {
        anomalies.push(anomaly("retry_spike", "medium", "retry_p95_threshold_exceeded"));
    }
    if signals.latency_p95_ms >= policy.latency_anomaly_p95_ms {
        anomalies.push(anomaly("latency_anomaly", "medium", "latency_p95_threshold_exceeded"));
    }
    if signals.unknown_failure_clusters > 0 {
        anomalies.push(anomaly(
            "unknown_failure_mode_cluster",
            "critical",
            "unknown_failure_cluster_detected",
        ));
    }
    anomalies
}

pub fn decide_bounded_adaptation(
    policy: &SystemHealthPolicy,
    status: &str,
    signals: &SystemSignals,
) -> BoundedAdaptationDecision {
    if policy.adaptation.require_eval_validation && !signals.eval_validation_passed {
        return adaptation(false, "eval_validation_required", 0, 0);
    }
    if policy.adaptation.require_rollback_safety && !signals.rollback_available {
        return adaptation(false, "rollback_safety_required", 0, 0);
    }
    if policy.adaptation.block_when_critical && status == "critical" {
        return adaptation(false, "critical_health_blocks_adaptation", 0, 0);
    }
    if status == "healthy" {
        return adaptation(true, "no_change_required", 0, 0);
    }
    adaptation(
        true,
        "bounded_adaptation_allowed",
        policy.adaptation.max_threshold_delta_percent.min(5),
        policy.adaptation.max_retry_delta.min(1),
    )
}

pub fn gateway_capability_graph(policy: &SystemHealthPolicy) -> Vec<GatewayCapabilityAnnouncement> {
    policy.gateway_capabilities.clone()
}

pub fn build_system_health_guard_report(
    policy_path: &str,
    policy: &SystemHealthPolicy,
    snapshots: &[SystemSignals],
) -> SystemHealthGuardReport {
    let statuses = snapshots
        .iter()
        .map(|signals| derive_system_health_status(policy, signals))
        .collect::<Vec<_>>();
    let gateway_capabilities = gateway_capability_graph(policy);
    let states = statuses.iter().map(|row| row.status.as_str()).collect::<Vec<_>>();
    let anomalies = statuses
        .iter()
        .flat_map(|row| row.anomalies.iter().map(|anomaly| anomaly.anomaly_type.as_str()))
        .collect::<Vec<_>>();
    let status_model_ok = ["healthy", "degraded", "recovering", "critical"]
        .iter()
        .all(|state| states.contains(state));
    let derivation_ok = statuses.iter().any(|row| row.reasons.iter().any(|r| r == "eval_regression_signal"))
        && statuses.iter().any(|row| row.reasons.iter().any(|r| r == "boundedness_regression_signal"))
        && statuses.iter().any(|row| row.reasons.iter().any(|r| r == "routing_anomaly_signal"))
        && statuses.iter().any(|row| row.reasons.iter().any(|r| r == "gateway_quarantine_signal"))
        && statuses.iter().any(|row| row.reasons.iter().any(|r| r == "recovery_signal"));
    let anomaly_ok = [
        "unusual_routing_patterns",
        "retry_spike",
        "latency_anomaly",
        "unknown_failure_mode_cluster",
    ]
    .iter()
    .all(|kind| anomalies.contains(kind));
    let adaptation_ok = statuses.iter().any(|row| {
        row.status == "degraded" && row.adaptation.allowed && row.adaptation.reason == "bounded_adaptation_allowed"
    }) && statuses.iter().any(|row| {
        row.snapshot_id == "critical" && !row.adaptation.allowed
    }) && statuses.iter().any(|row| {
        row.snapshot_id == "rollback_blocked" && !row.adaptation.allowed
    });
    let gateway_capability_ok = !gateway_capabilities.is_empty()
        && gateway_capabilities.iter().all(|gateway| {
            !gateway.gateway_id.is_empty()
                && !gateway.capabilities.is_empty()
                && gateway.max_concurrent_requests > 0
                && gateway.max_payload_kb > 0
                && gateway.timeout_ms > 0
                && !gateway.planning_graph_contributions.is_empty()
        });
    let checks = vec![
        check_row("system_health_status_model_contract", status_model_ok),
        check_row("system_health_signal_derivation_contract", derivation_ok),
        check_row("system_health_anomaly_detection_contract", anomaly_ok),
        check_row("bounded_adaptation_eval_rollback_contract", adaptation_ok),
        check_row("gateway_capability_discovery_contract", gateway_capability_ok),
    ];
    let ok = checks
        .iter()
        .all(|check| check.get("ok").and_then(Value::as_bool) == Some(true));
    SystemHealthGuardReport {
        ok,
        report_type: "system_health_guard".to_string(),
        policy_path: policy_path.to_string(),
        summary: json!({
            "status_count": statuses.len(),
            "gateway_capability_count": gateway_capabilities.len(),
            "anomaly_count": statuses.iter().map(|row| row.anomalies.len()).sum::<usize>(),
            "adaptation_allowed_count": statuses.iter().filter(|row| row.adaptation.allowed).count(),
            "pass": ok
        }),
        checks,
        statuses,
        gateway_capabilities,
    }
}

pub fn run_system_health_guard(
    policy_path: &str,
    out_json: &str,
    strict: bool,
) -> Result<SystemHealthGuardReport, String> {
    let policy = load_system_health_policy(policy_path)?;
    let report = build_system_health_guard_report(
        policy_path,
        &policy,
        default_system_signals().as_slice(),
    );
    write_report(out_json, &report)?;
    if strict && !report.ok {
        return Err("system_health_guard_failed".to_string());
    }
    Ok(report)
}

fn signals(
    snapshot_id: &str,
    eval_failures: u32,
    boundedness_regression_score: u32,
    routing_anomaly_count: u32,
    retry_count_p95: u32,
    latency_p95_ms: u64,
    gateway_quarantined_count: u32,
    recovery_active: bool,
    recovery_successes: u32,
    unknown_failure_clusters: u32,
    eval_validation_passed: bool,
    rollback_available: bool,
) -> SystemSignals {
    SystemSignals {
        snapshot_id: snapshot_id.to_string(),
        eval_failures,
        boundedness_regression_score,
        routing_anomaly_count,
        retry_count_p95,
        latency_p95_ms,
        gateway_quarantined_count,
        recovery_active,
        recovery_successes,
        unknown_failure_clusters,
        eval_validation_passed,
        rollback_available,
    }
}

fn anomaly(anomaly_type: &str, severity: &str, reason: &str) -> HealthAnomaly {
    HealthAnomaly {
        anomaly_type: anomaly_type.to_string(),
        severity: severity.to_string(),
        reason: reason.to_string(),
    }
}

fn adaptation(
    allowed: bool,
    reason: &str,
    threshold_delta_percent: u32,
    retry_delta: u32,
) -> BoundedAdaptationDecision {
    BoundedAdaptationDecision {
        allowed,
        reason: reason.to_string(),
        threshold_delta_percent,
        retry_delta,
    }
}

fn check_row(id: &str, ok: bool) -> Value {
    json!({ "id": id, "ok": ok })
}

fn write_report(out_json: &str, report: &SystemHealthGuardReport) -> Result<(), String> {
    let path = Path::new(out_json);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create_report_dir_failed:{err}"))?;
    }
    let raw = serde_json::to_string_pretty(report)
        .map_err(|err| format!("serialize_report_failed:{err}"))?;
    fs::write(path, format!("{raw}\n")).map_err(|err| format!("write_report_failed:{err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> SystemHealthPolicy {
        serde_json::from_str(include_str!("../config/system_health_policy.json")).expect("policy")
    }

    #[test]
    fn status_model_covers_all_operational_states() {
        let policy = policy();
        let report = build_system_health_guard_report(
            DEFAULT_SYSTEM_HEALTH_POLICY_PATH,
            &policy,
            default_system_signals().as_slice(),
        );
        assert!(report.ok);
        let states = report.statuses.iter().map(|row| row.status.as_str()).collect::<Vec<_>>();
        for expected in ["healthy", "degraded", "recovering", "critical"] {
            assert!(states.contains(&expected), "missing {expected}");
        }
    }

    #[test]
    fn anomalies_are_detected_from_routing_retry_latency_and_unknown_clusters() {
        let policy = policy();
        let critical = default_system_signals()
            .into_iter()
            .find(|signals| signals.snapshot_id == "critical")
            .expect("critical signals");
        let anomalies = detect_anomalies(&policy, &critical);
        assert!(anomalies.iter().any(|row| row.anomaly_type == "unusual_routing_patterns"));
        assert!(anomalies.iter().any(|row| row.anomaly_type == "retry_spike"));
        assert!(anomalies.iter().any(|row| row.anomaly_type == "latency_anomaly"));
        assert!(anomalies.iter().any(|row| row.anomaly_type == "unknown_failure_mode_cluster"));
    }

    #[test]
    fn adaptation_requires_eval_validation_and_rollback_safety() {
        let policy = policy();
        let report = build_system_health_guard_report(
            DEFAULT_SYSTEM_HEALTH_POLICY_PATH,
            &policy,
            default_system_signals().as_slice(),
        );
        let degraded = report
            .statuses
            .iter()
            .find(|row| row.snapshot_id == "degraded")
            .expect("degraded");
        assert!(degraded.adaptation.allowed);
        let rollback_blocked = report
            .statuses
            .iter()
            .find(|row| row.snapshot_id == "rollback_blocked")
            .expect("rollback blocked");
        assert!(!rollback_blocked.adaptation.allowed);
        assert_eq!(rollback_blocked.adaptation.reason, "rollback_safety_required");
    }

    #[test]
    fn gateway_capability_discovery_publishes_limits_and_planning_graph() {
        let policy = policy();
        let capabilities = gateway_capability_graph(&policy);
        assert!(capabilities.iter().any(|gateway| gateway.gateway_id == "ollama"));
        assert!(capabilities.iter().all(|gateway| {
            !gateway.capabilities.is_empty()
                && gateway.max_concurrent_requests > 0
                && gateway.max_payload_kb > 0
                && gateway.timeout_ms > 0
                && !gateway.planning_graph_contributions.is_empty()
        }));
    }
}
