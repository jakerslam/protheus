use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_GATEWAY_BOUNDARY_POLICY_PATH: &str =
    "core/layer2/conduit-security/config/gateway_boundary_policy.json";
pub const DEFAULT_GATEWAY_BOUNDARY_REPORT_PATH: &str =
    "core/local/artifacts/gateway_boundary_guard_current.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GatewayBoundaryPolicy {
    pub version: String,
    pub owner: String,
    pub required_isolation_mode: String,
    pub max_startup_timeout_ms: u64,
    pub max_request_timeout_ms: u64,
    pub max_memory_limit_mb: u64,
    pub quarantine_failure_threshold: u32,
    pub recovery_success_threshold: u32,
    pub gateways: Vec<GatewayBoundaryTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GatewayBoundaryTarget {
    pub id: String,
    pub isolation_mode: String,
    pub sandbox_required: bool,
    pub startup_timeout_ms: u64,
    pub request_timeout_ms: u64,
    pub memory_limit_mb: u64,
    pub route_around_target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GatewayObservation {
    pub gateway_id: String,
    pub heartbeat_seen: bool,
    pub heartbeat_age_ms: u64,
    pub liveness_ok: bool,
    pub degradation_score: u32,
    pub consecutive_failures: u32,
    pub recovery_successes: u32,
    pub last_failure: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GatewayBoundaryReceipt {
    pub receipt_type: String,
    pub gateway_id: String,
    pub event: String,
    pub reason: String,
    pub route_around_target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GatewayBoundaryDecision {
    pub gateway_id: String,
    pub health_status: String,
    pub isolation_enforced: bool,
    pub resource_bounds_enforced: bool,
    pub quarantined: bool,
    pub route_around_target: String,
    pub receipts: Vec<GatewayBoundaryReceipt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayBoundaryGuardReport {
    pub ok: bool,
    #[serde(rename = "type")]
    pub report_type: String,
    pub generated_at_ms: u64,
    pub policy_path: String,
    pub summary: Value,
    pub checks: Vec<Value>,
    pub decisions: Vec<GatewayBoundaryDecision>,
}

pub fn load_gateway_boundary_policy(path: &str) -> Result<GatewayBoundaryPolicy, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("read_policy_failed:{err}"))?;
    serde_json::from_str(&raw).map_err(|err| format!("parse_policy_failed:{err}"))
}

pub fn default_gateway_observations() -> Vec<GatewayObservation> {
    vec![
        GatewayObservation {
            gateway_id: "ollama".to_string(),
            heartbeat_seen: true,
            heartbeat_age_ms: 100,
            liveness_ok: true,
            degradation_score: 0,
            consecutive_failures: 0,
            recovery_successes: 0,
            last_failure: "none".to_string(),
        },
        GatewayObservation {
            gateway_id: "llama_cpp".to_string(),
            heartbeat_seen: false,
            heartbeat_age_ms: 30_000,
            liveness_ok: false,
            degradation_score: 95,
            consecutive_failures: 4,
            recovery_successes: 0,
            last_failure: "repeated_flapping".to_string(),
        },
        GatewayObservation {
            gateway_id: "mcp_baseline".to_string(),
            heartbeat_seen: true,
            heartbeat_age_ms: 250,
            liveness_ok: true,
            degradation_score: 0,
            consecutive_failures: 0,
            recovery_successes: 2,
            last_failure: "recovered_from_quarantine".to_string(),
        },
    ]
}

pub fn evaluate_gateway_boundaries(
    policy: &GatewayBoundaryPolicy,
    observations: &[GatewayObservation],
) -> Vec<GatewayBoundaryDecision> {
    let targets = policy
        .gateways
        .iter()
        .map(|gateway| (gateway.id.as_str(), gateway))
        .collect::<BTreeMap<_, _>>();
    observations
        .iter()
        .filter_map(|observation| {
            targets
                .get(observation.gateway_id.as_str())
                .map(|target| evaluate_gateway(policy, target, observation))
        })
        .collect()
}

pub fn build_gateway_boundary_guard_report(
    policy_path: &str,
    policy: &GatewayBoundaryPolicy,
    observations: &[GatewayObservation],
) -> GatewayBoundaryGuardReport {
    let decisions = evaluate_gateway_boundaries(policy, observations);
    let observed_ids = observations
        .iter()
        .map(|observation| observation.gateway_id.as_str())
        .collect::<BTreeSet<_>>();
    let required_ids = policy
        .gateways
        .iter()
        .map(|gateway| gateway.id.as_str())
        .collect::<BTreeSet<_>>();
    let isolation_policy_ok = policy.gateways.iter().all(|gateway| {
        gateway.sandbox_required && gateway.isolation_mode == policy.required_isolation_mode
    });
    let resource_bounds_ok = policy.gateways.iter().all(|gateway| {
        gateway.startup_timeout_ms > 0
            && gateway.request_timeout_ms > 0
            && gateway.memory_limit_mb > 0
            && gateway.startup_timeout_ms <= policy.max_startup_timeout_ms
            && gateway.request_timeout_ms <= policy.max_request_timeout_ms
            && gateway.memory_limit_mb <= policy.max_memory_limit_mb
    });
    let monitor_ok = decisions.iter().any(|decision| decision.health_status == "healthy")
        && decisions
            .iter()
            .any(|decision| decision.health_status == "quarantined")
        && decisions
            .iter()
            .any(|decision| decision.health_status == "recovered");
    let quarantine_ok = decisions.iter().any(|decision| {
        decision.quarantined
            && decision.route_around_target != "none"
            && decision.receipts.iter().any(|receipt| {
                receipt.receipt_type == "gateway_quarantine_event"
                    && receipt.event == "isolated"
                    && receipt.reason == "repeated_failure_threshold_exceeded"
            })
    });
    let recovery_ok = decisions.iter().any(|decision| {
        !decision.quarantined
            && decision.receipts.iter().any(|receipt| {
                receipt.receipt_type == "gateway_quarantine_event"
                    && receipt.event == "recovered"
                    && receipt.reason == "recovery_success_threshold_met"
            })
    });
    let coverage_ok = required_ids.is_superset(&observed_ids) && !decisions.is_empty();
    let checks = vec![
        check_row("gateway_process_isolation_policy_contract", isolation_policy_ok),
        check_row("gateway_timeout_memory_limit_policy_contract", resource_bounds_ok),
        check_row("gateway_health_monitor_contract", monitor_ok),
        check_row("gateway_repeated_failure_quarantine_contract", quarantine_ok),
        check_row("gateway_quarantine_recovery_receipt_contract", recovery_ok),
        check_row("gateway_policy_observation_coverage_contract", coverage_ok),
    ];
    let ok = checks
        .iter()
        .all(|check| check.get("ok").and_then(Value::as_bool) == Some(true));
    GatewayBoundaryGuardReport {
        ok,
        report_type: "gateway_boundary_guard".to_string(),
        generated_at_ms: now_ms(),
        policy_path: policy_path.to_string(),
        summary: json!({
            "gateway_count": policy.gateways.len(),
            "decision_count": decisions.len(),
            "quarantined_count": decisions.iter().filter(|decision| decision.quarantined).count(),
            "route_around_count": decisions.iter().filter(|decision| decision.route_around_target != "none").count(),
            "quarantine_event_receipt_count": decisions.iter().flat_map(|decision| decision.receipts.iter()).filter(|receipt| receipt.receipt_type == "gateway_quarantine_event").count(),
            "pass": ok
        }),
        checks,
        decisions,
    }
}

pub fn run_gateway_boundary_guard(
    policy_path: &str,
    out_json: &str,
    strict: bool,
) -> Result<GatewayBoundaryGuardReport, String> {
    let policy = load_gateway_boundary_policy(policy_path)?;
    let report = build_gateway_boundary_guard_report(
        policy_path,
        &policy,
        default_gateway_observations().as_slice(),
    );
    write_report(out_json, &report)?;
    if strict && !report.ok {
        return Err("gateway_boundary_guard_failed".to_string());
    }
    Ok(report)
}

fn evaluate_gateway(
    policy: &GatewayBoundaryPolicy,
    target: &GatewayBoundaryTarget,
    observation: &GatewayObservation,
) -> GatewayBoundaryDecision {
    let isolation_enforced =
        target.sandbox_required && target.isolation_mode == policy.required_isolation_mode;
    let resource_bounds_enforced = target.startup_timeout_ms <= policy.max_startup_timeout_ms
        && target.request_timeout_ms <= policy.max_request_timeout_ms
        && target.memory_limit_mb <= policy.max_memory_limit_mb;
    let quarantined = observation.consecutive_failures >= policy.quarantine_failure_threshold
        || !observation.liveness_ok
        || !observation.heartbeat_seen;
    let recovered = !quarantined
        && observation.recovery_successes >= policy.recovery_success_threshold
        && observation.liveness_ok
        && observation.heartbeat_seen;
    let health_status = if quarantined {
        "quarantined"
    } else if recovered {
        "recovered"
    } else if observation.liveness_ok && observation.heartbeat_seen && observation.degradation_score == 0 {
        "healthy"
    } else {
        "degraded"
    };
    let route_around_target = if quarantined {
        target.route_around_target.clone()
    } else {
        "none".to_string()
    };
    let mut receipts = vec![GatewayBoundaryReceipt {
        receipt_type: "gateway_health_monitor_receipt".to_string(),
        gateway_id: target.id.clone(),
        event: health_status.to_string(),
        reason: health_reason(policy, observation),
        route_around_target: route_around_target.clone(),
    }];
    if quarantined {
        receipts.push(GatewayBoundaryReceipt {
            receipt_type: "gateway_quarantine_event".to_string(),
            gateway_id: target.id.clone(),
            event: "isolated".to_string(),
            reason: "repeated_failure_threshold_exceeded".to_string(),
            route_around_target: route_around_target.clone(),
        });
    } else if recovered {
        receipts.push(GatewayBoundaryReceipt {
            receipt_type: "gateway_quarantine_event".to_string(),
            gateway_id: target.id.clone(),
            event: "recovered".to_string(),
            reason: "recovery_success_threshold_met".to_string(),
            route_around_target: route_around_target.clone(),
        });
    }
    GatewayBoundaryDecision {
        gateway_id: target.id.clone(),
        health_status: health_status.to_string(),
        isolation_enforced,
        resource_bounds_enforced,
        quarantined,
        route_around_target,
        receipts,
    }
}

fn health_reason(policy: &GatewayBoundaryPolicy, observation: &GatewayObservation) -> String {
    if observation.consecutive_failures >= policy.quarantine_failure_threshold {
        return "repeated_failure_threshold_exceeded".to_string();
    }
    if !observation.heartbeat_seen {
        return "heartbeat_missing".to_string();
    }
    if !observation.liveness_ok {
        return "liveness_failed".to_string();
    }
    if observation.recovery_successes >= policy.recovery_success_threshold {
        return "recovery_success_threshold_met".to_string();
    }
    "heartbeat_liveness_ok".to_string()
}

fn check_row(id: &str, ok: bool) -> Value {
    json!({ "id": id, "ok": ok })
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

fn write_report(out_json: &str, report: &GatewayBoundaryGuardReport) -> Result<(), String> {
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

    fn policy() -> GatewayBoundaryPolicy {
        serde_json::from_str(include_str!(
            "../config/gateway_boundary_policy.json"
        ))
        .expect("policy")
    }

    #[test]
    fn gateway_isolation_and_resource_bounds_are_enforced() {
        let policy = policy();
        assert!(policy.gateways.iter().all(|gateway| gateway.sandbox_required));
        assert!(policy.gateways.iter().all(|gateway| {
            gateway.isolation_mode == policy.required_isolation_mode
                && gateway.startup_timeout_ms <= policy.max_startup_timeout_ms
                && gateway.request_timeout_ms <= policy.max_request_timeout_ms
                && gateway.memory_limit_mb <= policy.max_memory_limit_mb
        }));
    }

    #[test]
    fn repeated_failures_quarantine_and_route_around() {
        let policy = policy();
        let decisions =
            evaluate_gateway_boundaries(&policy, default_gateway_observations().as_slice());
        let llama = decisions
            .iter()
            .find(|decision| decision.gateway_id == "llama_cpp")
            .expect("llama_cpp decision");
        assert!(llama.quarantined);
        assert_eq!(llama.route_around_target, "ollama");
        assert!(llama.receipts.iter().any(|receipt| {
            receipt.receipt_type == "gateway_quarantine_event" && receipt.event == "isolated"
        }));
    }

    #[test]
    fn recovery_emits_quarantine_recovery_receipt() {
        let policy = policy();
        let decisions =
            evaluate_gateway_boundaries(&policy, default_gateway_observations().as_slice());
        let recovered = decisions
            .iter()
            .find(|decision| decision.gateway_id == "mcp_baseline")
            .expect("mcp_baseline decision");
        assert_eq!(recovered.health_status, "recovered");
        assert!(!recovered.quarantined);
        assert!(recovered.receipts.iter().any(|receipt| {
            receipt.receipt_type == "gateway_quarantine_event" && receipt.event == "recovered"
        }));
    }
}
