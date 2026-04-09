use crate::self_maintenance::contracts::{
    ConfidenceVector, EvidenceCard, EvidenceSourceKind, ObservationInputs,
};
use serde_json::json;
use sha2::{Digest, Sha256};

pub fn collect_evidence_cards(inputs: &ObservationInputs, now_ms: u64) -> Vec<EvidenceCard> {
    let mut out = Vec::<EvidenceCard>::new();

    for audit in &inputs.architecture_audits {
        out.push(EvidenceCard {
            evidence_id: stable_id("architecture_audit", audit.audit_id.as_str(), now_ms),
            source_kind: EvidenceSourceKind::ArchitectureAudit,
            source_ref: audit.source_ref.clone(),
            summary: format!("architecture audit: {}", audit.summary),
            details: json!({
                "severity": audit.severity,
                "audit_id": audit.audit_id,
            }),
            tags: vec!["architecture".to_string(), "audit".to_string()],
            confidence_vector: ConfidenceVector {
                relevance: 0.90,
                reliability: 0.85,
                freshness: 0.70,
            },
            timestamp_ms: now_ms,
        });
    }

    for violation in &inputs.dependency_violations {
        out.push(EvidenceCard {
            evidence_id: stable_id(
                "dependency_violation",
                violation.violation_id.as_str(),
                now_ms,
            ),
            source_kind: EvidenceSourceKind::DependencyViolation,
            source_ref: violation.source_ref.clone(),
            summary: format!("dependency violation: {}", violation.summary),
            details: json!({
                "violation_id": violation.violation_id,
            }),
            tags: vec!["dependency".to_string(), "violation".to_string()],
            confidence_vector: ConfidenceVector {
                relevance: 0.92,
                reliability: 0.88,
                freshness: 0.75,
            },
            timestamp_ms: now_ms,
        });
    }

    if !inputs.task_fabric_signals.stale_tasks.is_empty() {
        out.push(EvidenceCard {
            evidence_id: stable_id(
                "task_fabric_stale",
                &inputs.task_fabric_signals.stale_tasks.join(","),
                now_ms,
            ),
            source_kind: EvidenceSourceKind::TaskFabricSignal,
            source_ref: "task_fabric:stale_tasks".to_string(),
            summary: format!(
                "stale task count={}",
                inputs.task_fabric_signals.stale_tasks.len()
            ),
            details: json!({
                "stale_tasks": inputs.task_fabric_signals.stale_tasks,
            }),
            tags: vec!["task_fabric".to_string(), "stale".to_string()],
            confidence_vector: ConfidenceVector {
                relevance: 0.85,
                reliability: 0.90,
                freshness: 0.80,
            },
            timestamp_ms: now_ms,
        });
    }

    if !inputs.task_fabric_signals.blocked_tasks.is_empty() {
        out.push(EvidenceCard {
            evidence_id: stable_id(
                "task_fabric_blocked",
                &inputs.task_fabric_signals.blocked_tasks.join(","),
                now_ms,
            ),
            source_kind: EvidenceSourceKind::TaskFabricSignal,
            source_ref: "task_fabric:blocked_tasks".to_string(),
            summary: format!(
                "blocked task count={}",
                inputs.task_fabric_signals.blocked_tasks.len()
            ),
            details: json!({
                "blocked_tasks": inputs.task_fabric_signals.blocked_tasks,
            }),
            tags: vec!["task_fabric".to_string(), "blocked".to_string()],
            confidence_vector: ConfidenceVector {
                relevance: 0.88,
                reliability: 0.90,
                freshness: 0.80,
            },
            timestamp_ms: now_ms,
        });
    }

    for report in &inputs.ci_reports {
        out.push(EvidenceCard {
            evidence_id: stable_id("ci_report", report.report_id.as_str(), now_ms),
            source_kind: EvidenceSourceKind::CiReport,
            source_ref: report.source_ref.clone(),
            summary: format!("ci report {}: {}", report.status, report.summary),
            details: json!({
                "report_id": report.report_id,
                "status": report.status,
            }),
            tags: vec!["ci".to_string(), "report".to_string()],
            confidence_vector: ConfidenceVector {
                relevance: 0.82,
                reliability: 0.84,
                freshness: 0.88,
            },
            timestamp_ms: now_ms,
        });
    }

    for metric in &inputs.health_metrics {
        let over_threshold = metric.observed > metric.threshold;
        out.push(EvidenceCard {
            evidence_id: stable_id("health_metric", metric.metric_name.as_str(), now_ms),
            source_kind: EvidenceSourceKind::HealthMetric,
            source_ref: metric.source_ref.clone(),
            summary: format!(
                "health metric {} observed={} threshold={}",
                metric.metric_name, metric.observed, metric.threshold
            ),
            details: json!({
                "observed": metric.observed,
                "threshold": metric.threshold,
                "over_threshold": over_threshold,
            }),
            tags: vec![
                "health".to_string(),
                if over_threshold {
                    "degraded".to_string()
                } else {
                    "stable".to_string()
                },
            ],
            confidence_vector: ConfidenceVector {
                relevance: if over_threshold { 0.90 } else { 0.55 },
                reliability: 0.80,
                freshness: 0.85,
            },
            timestamp_ms: now_ms,
        });
    }

    for pressure in &inputs.memory_pressure {
        let ratio = if pressure.limit_bytes == 0 {
            1.0
        } else {
            pressure.used_bytes as f64 / pressure.limit_bytes as f64
        };
        out.push(EvidenceCard {
            evidence_id: stable_id("memory_pressure", pressure.scope.as_str(), now_ms),
            source_kind: EvidenceSourceKind::MemoryPressure,
            source_ref: format!("memory:{}", pressure.scope),
            summary: format!(
                "memory pressure scope={} usage_ratio={ratio:.3}",
                pressure.scope
            ),
            details: json!({
                "used_bytes": pressure.used_bytes,
                "limit_bytes": pressure.limit_bytes,
                "ratio": ratio,
            }),
            tags: vec!["memory".to_string(), "pressure".to_string()],
            confidence_vector: ConfidenceVector {
                relevance: if ratio >= 0.90 { 0.90 } else { 0.60 },
                reliability: 0.86,
                freshness: 0.90,
            },
            timestamp_ms: now_ms,
        });
    }

    for orphan in &inputs.orphaned_objects {
        out.push(EvidenceCard {
            evidence_id: stable_id("orphaned_object", orphan.object_id.as_str(), now_ms),
            source_kind: EvidenceSourceKind::OrphanedObject,
            source_ref: orphan.source_ref.clone(),
            summary: format!("orphaned object {}: {}", orphan.object_id, orphan.summary),
            details: json!({
                "object_id": orphan.object_id,
            }),
            tags: vec!["orphaned".to_string(), "cleanup".to_string()],
            confidence_vector: ConfidenceVector {
                relevance: 0.83,
                reliability: 0.86,
                freshness: 0.72,
            },
            timestamp_ms: now_ms,
        });
    }

    out
}

fn stable_id(prefix: &str, value: &str, now_ms: u64) -> String {
    let payload = format!("{prefix}::{value}::{now_ms}");
    let mut hasher = Sha256::new();
    hasher.update(payload.as_bytes());
    format!("{prefix}-{:x}", hasher.finalize())
}
