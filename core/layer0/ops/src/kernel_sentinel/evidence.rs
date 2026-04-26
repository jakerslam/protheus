// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    authority_rule, KernelSentinelAuthorityClass, KernelSentinelEvidenceSource,
    KernelSentinelFinding, KernelSentinelFindingCategory, KernelSentinelSeverity,
    KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
};
use serde::{Deserialize, Deserializer};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

mod capability_grants;
mod boundedness;
mod gateway_isolation;
mod nexus_boundaries;
mod receipt_completeness;
mod state_transitions;
mod trajectories;
use boundedness::build_boundedness_report;
use capability_grants::build_capability_grant_report;
use gateway_isolation::build_gateway_isolation_report;
use nexus_boundaries::build_nexus_boundary_report;
use receipt_completeness::build_receipt_completeness_report;
use state_transitions::build_state_transition_report;
use trajectories::build_trajectory_report;

#[derive(Debug, Clone)]
pub struct KernelSentinelEvidenceIngestion {
    pub findings: Vec<KernelSentinelFinding>,
    pub malformed_records: Vec<Value>,
    pub report: Value,
}

#[derive(Debug, Clone, Copy)]
struct EvidenceSourceConfig {
    source: KernelSentinelEvidenceSource,
    file_name: &'static str,
    default_category: KernelSentinelFindingCategory,
    default_severity: KernelSentinelSeverity,
    missing_required_severity: KernelSentinelSeverity,
}

#[derive(Debug, Clone, Deserialize)]
struct RawEvidenceRecord {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    ok: Option<bool>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_severity")]
    severity: Option<KernelSentinelSeverity>,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_category")]
    category: Option<KernelSentinelFindingCategory>,
    #[serde(default)]
    fingerprint: Option<String>,
    #[serde(default)]
    subject: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    recommended_action: Option<String>,
    #[serde(default)]
    evidence: Vec<String>,
    #[serde(flatten)]
    details: BTreeMap<String, Value>,
}

fn normalize_key(raw: &str) -> String {
    let mut out = String::new();
    let mut previous_lower_or_digit = false;
    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_uppercase() && previous_lower_or_digit && !out.ends_with('_') {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            previous_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        } else if !out.ends_with('_') {
            out.push('_');
            previous_lower_or_digit = false;
        }
    }
    out.trim_matches('_').to_string()
}

fn category_from_str(raw: &str) -> Option<KernelSentinelFindingCategory> {
    match normalize_key(raw).as_str() {
        "receipt_integrity" => Some(KernelSentinelFindingCategory::ReceiptIntegrity),
        "capability_enforcement" => Some(KernelSentinelFindingCategory::CapabilityEnforcement),
        "state_transition" => Some(KernelSentinelFindingCategory::StateTransition),
        "nexus_boundary" => Some(KernelSentinelFindingCategory::NexusBoundary),
        "boundedness" => Some(KernelSentinelFindingCategory::Boundedness),
        "gateway_isolation" => Some(KernelSentinelFindingCategory::GatewayIsolation),
        "queue_backpressure" => Some(KernelSentinelFindingCategory::QueueBackpressure),
        "retry_storm" => Some(KernelSentinelFindingCategory::RetryStorm),
        "release_evidence" => Some(KernelSentinelFindingCategory::ReleaseEvidence),
        "self_maintenance_loop" => Some(KernelSentinelFindingCategory::SelfMaintenanceLoop),
        "security_boundary" => Some(KernelSentinelFindingCategory::SecurityBoundary),
        "runtime_correctness" => Some(KernelSentinelFindingCategory::RuntimeCorrectness),
        "performance_regression" => Some(KernelSentinelFindingCategory::PerformanceRegression),
        "automation_candidate" => Some(KernelSentinelFindingCategory::AutomationCandidate),
        _ => None,
    }
}

fn severity_from_str(raw: &str) -> Option<KernelSentinelSeverity> {
    match normalize_key(raw).as_str() {
        "critical" => Some(KernelSentinelSeverity::Critical),
        "high" => Some(KernelSentinelSeverity::High),
        "medium" => Some(KernelSentinelSeverity::Medium),
        "low" => Some(KernelSentinelSeverity::Low),
        _ => None,
    }
}

fn deserialize_optional_category<'de, D>(
    deserializer: D,
) -> Result<Option<KernelSentinelFindingCategory>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(match value {
        Some(Value::String(raw)) => category_from_str(&raw),
        Some(other) => serde_json::from_value(other).ok(),
        None => None,
    })
}

fn deserialize_optional_severity<'de, D>(
    deserializer: D,
) -> Result<Option<KernelSentinelSeverity>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(match value {
        Some(Value::String(raw)) => severity_from_str(&raw),
        Some(other) => serde_json::from_value(other).ok(),
        None => None,
    })
}

fn merged_details(mut details: BTreeMap<String, Value>) -> Value {
    if let Some(Value::Object(nested)) = details.remove("details") {
        for (key, value) in nested {
            details.entry(key).or_insert(value);
        }
    }
    Value::Object(details.into_iter().collect())
}

fn source_configs() -> [EvidenceSourceConfig; 6] {
    [
        EvidenceSourceConfig {
            source: KernelSentinelEvidenceSource::KernelReceipt,
            file_name: "kernel_receipts.jsonl",
            default_category: KernelSentinelFindingCategory::ReceiptIntegrity,
            default_severity: KernelSentinelSeverity::Critical,
            missing_required_severity: KernelSentinelSeverity::Critical,
        },
        EvidenceSourceConfig {
            source: KernelSentinelEvidenceSource::RuntimeObservation,
            file_name: "runtime_observations.jsonl",
            default_category: KernelSentinelFindingCategory::RuntimeCorrectness,
            default_severity: KernelSentinelSeverity::High,
            missing_required_severity: KernelSentinelSeverity::Critical,
        },
        EvidenceSourceConfig {
            source: KernelSentinelEvidenceSource::ReleaseProofPack,
            file_name: "release_proof_packs.jsonl",
            default_category: KernelSentinelFindingCategory::ReleaseEvidence,
            default_severity: KernelSentinelSeverity::Critical,
            missing_required_severity: KernelSentinelSeverity::Critical,
        },
        EvidenceSourceConfig {
            source: KernelSentinelEvidenceSource::GatewayHealth,
            file_name: "gateway_health.jsonl",
            default_category: KernelSentinelFindingCategory::GatewayIsolation,
            default_severity: KernelSentinelSeverity::High,
            missing_required_severity: KernelSentinelSeverity::High,
        },
        EvidenceSourceConfig {
            source: KernelSentinelEvidenceSource::QueueBackpressure,
            file_name: "queue_backpressure.jsonl",
            default_category: KernelSentinelFindingCategory::QueueBackpressure,
            default_severity: KernelSentinelSeverity::High,
            missing_required_severity: KernelSentinelSeverity::High,
        },
        EvidenceSourceConfig {
            source: KernelSentinelEvidenceSource::ControlPlaneEval,
            file_name: "control_plane_eval.jsonl",
            default_category: KernelSentinelFindingCategory::RuntimeCorrectness,
            default_severity: KernelSentinelSeverity::Medium,
            missing_required_severity: KernelSentinelSeverity::Low,
        },
    ]
}

fn source_key(source: KernelSentinelEvidenceSource) -> &'static str {
    match source {
        KernelSentinelEvidenceSource::KernelReceipt => "kernel_receipt",
        KernelSentinelEvidenceSource::RuntimeObservation => "runtime_observation",
        KernelSentinelEvidenceSource::ReleaseProofPack => "release_proof_pack",
        KernelSentinelEvidenceSource::GatewayHealth => "gateway_health",
        KernelSentinelEvidenceSource::QueueBackpressure => "queue_backpressure",
        KernelSentinelEvidenceSource::ControlPlaneEval => "control_plane_eval",
        KernelSentinelEvidenceSource::ShellTelemetry => "shell_telemetry",
    }
}

fn option_path(args: &[String], name: &str, fallback: PathBuf) -> PathBuf {
    let prefix = format!("{name}=");
    args.iter()
        .find_map(|arg| arg.strip_prefix(&prefix).map(PathBuf::from))
        .unwrap_or(fallback)
}

fn bool_flag(args: &[String], name: &str) -> bool {
    args.iter()
        .any(|arg| arg == name || arg == &format!("{name}=1") || arg == &format!("{name}=true"))
}

fn clean_token(value: Option<String>, fallback: &str) -> String {
    value
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn cap_advisory_severity(
    source: KernelSentinelEvidenceSource,
    severity: KernelSentinelSeverity,
) -> KernelSentinelSeverity {
    if source == KernelSentinelEvidenceSource::ControlPlaneEval
        && severity == KernelSentinelSeverity::Critical
    {
        KernelSentinelSeverity::High
    } else {
        severity
    }
}

fn should_open_finding(record: &RawEvidenceRecord) -> bool {
    record.ok == Some(false)
        || record
            .status
            .as_deref()
            .map(|status| matches!(status, "failed" | "blocked" | "invalid" | "critical"))
            .unwrap_or(false)
        || record.severity.is_some()
}

fn normalize_record(
    config: EvidenceSourceConfig,
    path: &Path,
    line: usize,
    record: RawEvidenceRecord,
) -> (Value, Option<KernelSentinelFinding>) {
    let source = source_key(config.source);
    let authority = authority_rule(config.source);
    let open_finding = should_open_finding(&record);
    let subject = clean_token(record.subject, "unknown_subject");
    let kind = clean_token(record.kind, "evidence_observation");
    let id = clean_token(record.id.clone(), &format!("{source}:{line}"));
    let category = record.category.unwrap_or(config.default_category);
    let severity = cap_advisory_severity(
        config.source,
        record.severity.unwrap_or(config.default_severity),
    );
    let fingerprint = clean_token(record.fingerprint, &format!("{source}:{subject}:{kind}"));
    let evidence = if record.evidence.is_empty() {
        vec![format!("{}:{line}", path.display())]
    } else {
        record.evidence
    };
    let details = merged_details(record.details);
    let normalized = json!({
        "source": source,
        "authority_class": authority.authority_class,
        "may_block_release": authority.may_block_release,
        "advisory": authority.authority_class != KernelSentinelAuthorityClass::DeterministicKernelAuthority,
        "id": id,
        "ok": record.ok,
        "status": record.status,
        "subject": subject,
        "kind": kind,
        "severity": severity,
        "category": category,
        "fingerprint": fingerprint,
        "evidence": evidence,
        "details": details,
        "path": path,
        "line": line
    });
    if !open_finding {
        return (normalized, None);
    }
    let summary = clean_token(
        record.summary,
        &format!("{source} evidence reported {kind} for {subject}"),
    );
    let recommended_action = clean_token(
        record.recommended_action,
        "inspect deterministic kernel evidence and restore fail-closed behavior",
    );
    let finding = KernelSentinelFinding {
        schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        id,
        severity,
        category,
        fingerprint,
        evidence,
        summary,
        recommended_action,
        status: "open".to_string(),
    };
    (normalized, Some(finding))
}

fn missing_required_finding(config: EvidenceSourceConfig, path: &Path) -> KernelSentinelFinding {
    let source = source_key(config.source);
    KernelSentinelFinding {
        schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        id: format!("{source}:missing_source"),
        severity: config.missing_required_severity,
        category: config.default_category,
        fingerprint: format!("{source}:missing_required_source"),
        evidence: vec![format!("missing://{}", path.display())],
        summary: format!("Kernel Sentinel evidence source `{source}` is missing"),
        recommended_action: format!(
            "restore `{}` or disable require-evidence for non-release diagnostics",
            path.display()
        ),
        status: "open".to_string(),
    }
}

pub fn ingest_evidence_sources(state_dir: &Path, args: &[String]) -> KernelSentinelEvidenceIngestion {
    let evidence_dir = option_path(args, "--evidence-dir", state_dir.join("evidence"));
    let require_evidence = bool_flag(args, "--require-evidence");
    let mut findings = Vec::new();
    let mut malformed_records = Vec::new();
    let mut normalized_records = Vec::new();
    let mut source_reports = Vec::new();

    for config in source_configs() {
        let path = evidence_dir.join(config.file_name);
        let source = source_key(config.source);
        if !path.exists() {
            if require_evidence {
                findings.push(missing_required_finding(config, &path));
            }
            source_reports.push(json!({
                "source": source,
                "path": path,
                "present": false,
                "required": require_evidence,
                "authority_class": authority_rule(config.source).authority_class
            }));
            continue;
        }
        let raw = fs::read_to_string(&path).unwrap_or_default();
        let mut record_count = 0usize;
        for (idx, line) in raw.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<RawEvidenceRecord>(trimmed) {
                Ok(record) => {
                    record_count += 1;
                    let (normalized, finding) = normalize_record(config, &path, idx + 1, record);
                    normalized_records.push(normalized);
                    if let Some(finding) = finding {
                        findings.push(finding);
                    }
                }
                Err(err) => malformed_records.push(json!({
                    "source": source,
                    "path": path,
                    "source_kind": "kernel_sentinel_evidence",
                    "line": idx + 1,
                    "error": err.to_string()
                })),
            }
        }
        source_reports.push(json!({
            "source": source,
            "path": path,
            "present": true,
            "required": require_evidence,
            "record_count": record_count,
            "authority_class": authority_rule(config.source).authority_class
        }));
    }

    let (receipt_completeness_report, receipt_findings) =
        build_receipt_completeness_report(&normalized_records);
    findings.extend(receipt_findings);
    let (capability_grants_report, capability_findings) =
        build_capability_grant_report(&normalized_records);
    findings.extend(capability_findings);
    let (state_transitions_report, state_transition_findings) =
        build_state_transition_report(&normalized_records);
    findings.extend(state_transition_findings);
    let (nexus_boundaries_report, nexus_boundary_findings) =
        build_nexus_boundary_report(&normalized_records);
    findings.extend(nexus_boundary_findings);
    let (boundedness_report, boundedness_findings) =
        build_boundedness_report(&normalized_records);
    findings.extend(boundedness_findings);
    let (gateway_isolation_report, gateway_isolation_findings) =
        build_gateway_isolation_report(&normalized_records);
    findings.extend(gateway_isolation_findings);
    let (trajectory_report, trajectory_findings) = build_trajectory_report(&normalized_records);
    findings.extend(trajectory_findings);
    let report = json!({
        "ok": malformed_records.is_empty(),
        "type": "kernel_sentinel_evidence_ingestion",
        "evidence_dir": evidence_dir,
        "require_evidence": require_evidence,
        "sources": source_reports,
        "receipt_completeness": receipt_completeness_report,
        "capability_grants": capability_grants_report,
        "state_transitions": state_transitions_report,
        "nexus_boundaries": nexus_boundaries_report,
        "boundedness": boundedness_report,
        "gateway_isolation": gateway_isolation_report,
        "trajectories": trajectory_report,
        "normalized_record_count": normalized_records.len(),
        "finding_count": findings.len(),
        "malformed_record_count": malformed_records.len(),
        "normalized_records": normalized_records
    });
    KernelSentinelEvidenceIngestion {
        findings,
        malformed_records,
        report,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_missing_kernel_sources_become_blocking_findings() {
        let dir = std::env::temp_dir().join("kernel-sentinel-evidence-missing");
        let args = vec!["--require-evidence=1".to_string(), format!("--evidence-dir={}", dir.display())];
        let ingestion = ingest_evidence_sources(&dir, &args);
        assert!(ingestion.findings.iter().any(|finding| {
            finding.fingerprint == "kernel_receipt:missing_required_source"
                && finding.severity == KernelSentinelSeverity::Critical
        }));
    }

    #[test]
    fn control_plane_eval_is_advisory_even_when_reported_critical() {
        let dir = std::env::temp_dir().join("kernel-sentinel-evidence-advisory");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("control_plane_eval.jsonl"),
            r#"{"id":"eval-1","ok":false,"severity":"critical","subject":"chat","kind":"wrong_tool","summary":"web search routed for file request","evidence":["eval://round/1"]}"#,
        )
        .unwrap();
        let args = vec![format!("--evidence-dir={}", dir.display())];
        let ingestion = ingest_evidence_sources(&dir, &args);
        assert_eq!(ingestion.findings.len(), 1);
        assert_eq!(ingestion.findings[0].severity, KernelSentinelSeverity::High);
        assert_eq!(
            ingestion.report["normalized_records"][0]["advisory"],
            Value::Bool(true)
        );
    }

}
