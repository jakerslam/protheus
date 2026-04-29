// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    authority_rule, KernelSentinelAuthorityClass, KernelSentinelEvidenceSource,
    KernelSentinelFinding, KernelSentinelFindingCategory, KernelSentinelSeverity,
    KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

mod capability_grants;
mod advisory_bridge;
mod boundedness;
mod gateway_isolation;
mod helpers;
mod nexus_boundaries;
mod receipt_completeness;
mod source_catalog;
mod state_transitions;
mod trajectories;
use advisory_bridge::build_advisory_bridge_report;
use boundedness::build_boundedness_report;
use capability_grants::build_capability_grant_report;
use gateway_isolation::build_gateway_isolation_report;
use helpers::{bool_flag, deserialize_optional_category, deserialize_optional_severity, normalize_key, option_path, option_u64};
use nexus_boundaries::build_nexus_boundary_report;
use receipt_completeness::build_receipt_completeness_report;
use source_catalog::source_configs;
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
    collector_family: &'static str,
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

fn merged_details(mut details: BTreeMap<String, Value>) -> Value {
    if let Some(Value::Object(nested)) = details.remove("details") {
        for (key, value) in nested {
            details.entry(key).or_insert(value);
        }
    }
    Value::Object(details.into_iter().collect())
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
    let status_failed = record
        .status
        .as_deref()
        .map(normalize_key)
        .map(|status| matches!(status.as_str(), "fail" | "failed" | "blocked" | "invalid" | "critical" | "error"))
        .unwrap_or(false);
    let pass_failed = record
        .details
        .get("pass")
        .and_then(Value::as_bool)
        == Some(false);
    record.ok == Some(false)
        || status_failed
        || pass_failed
}

fn normalize_record(
    config: EvidenceSourceConfig,
    path: &Path,
    line: usize,
    record: RawEvidenceRecord,
) -> (Value, Option<KernelSentinelFinding>) {
    let source = source_key(config.source);
    let authority = authority_rule(config.source);
    let open_finding = authority.may_open_finding && should_open_finding(&record);
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
        "collector_family": config.collector_family,
        "authority_class": authority.authority_class,
        "may_write_verdict": authority.may_write_verdict,
        "may_waive_finding": authority.may_waive_finding,
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

fn source_required_for_observation(config: EvidenceSourceConfig) -> bool {
    authority_rule(config.source).authority_class != KernelSentinelAuthorityClass::PresentationTelemetryOnly
}

fn malformed_count_by_key(records: &[Value], key: &str) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::<String, usize>::new();
    for record in records {
        let value = record
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|raw| !raw.is_empty())
            .unwrap_or("unknown")
            .to_string();
        *out.entry(value).or_insert(0) += 1;
    }
    out
}

fn record_freshness_age_seconds(record: &Value) -> Option<u64> {
    let details = record.get("details").unwrap_or(&Value::Null);
    ["freshness_age_seconds", "age_seconds", "source_artifact_age_seconds"]
        .iter()
        .find_map(|key| {
            details
                .get(*key)
                .or_else(|| record.get(*key))
                .and_then(|raw| {
                    raw.as_u64()
                        .or_else(|| raw.as_i64().and_then(|value| u64::try_from(value).ok()))
                        .or_else(|| raw.as_str().and_then(|text| text.trim().parse::<u64>().ok()))
                })
        })
}

pub fn ingest_evidence_sources(state_dir: &Path, args: &[String]) -> KernelSentinelEvidenceIngestion {
    let evidence_dir = option_path(args, "--evidence-dir", state_dir.join("evidence"));
    let require_evidence = bool_flag(args, "--require-evidence");
    let stale_evidence_seconds = option_u64(args, "--stale-evidence-seconds", 24 * 60 * 60);
    let mut findings = Vec::new();
    let mut malformed_records = Vec::new();
    let mut normalized_records = Vec::new();
    let mut source_reports = Vec::new();
    let mut present_source_count = 0usize;
    let mut missing_source_count = 0usize;
    let mut present_required_source_count = 0usize;
    let mut missing_required_source_count = 0usize;
    let mut present_optional_source_count = 0usize;
    let mut missing_optional_source_count = 0usize;

    for config in source_configs() {
        let path = evidence_dir.join(config.file_name);
        let source = source_key(config.source);
        let source_required = source_required_for_observation(config);
        if !path.exists() {
            missing_source_count += 1;
            if source_required {
                missing_required_source_count += 1;
            } else {
                missing_optional_source_count += 1;
            }
            if require_evidence && source_required {
                findings.push(missing_required_finding(config, &path));
            }
            source_reports.push(json!({
                "source": source,
                "path": path,
                "file_name": config.file_name,
                "present": false,
                "required": require_evidence && source_required,
                "required_for_observation": source_required,
                "collector_family": config.collector_family,
                "authority_class": authority_rule(config.source).authority_class
            }));
            continue;
        }
        present_source_count += 1;
        if source_required {
            present_required_source_count += 1;
        } else {
            present_optional_source_count += 1;
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
                    "file_name": config.file_name,
                    "collector_family": config.collector_family,
                    "source_kind": "kernel_sentinel_evidence",
                    "line": idx + 1,
                    "error": err.to_string()
                })),
            }
        }
        source_reports.push(json!({
            "source": source,
            "path": path,
            "file_name": config.file_name,
            "present": true,
            "required": require_evidence && source_required,
            "required_for_observation": source_required,
            "record_count": record_count,
            "collector_family": config.collector_family,
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
    let (advisory_bridge_report, advisory_bridge_findings) =
        build_advisory_bridge_report(&normalized_records);
    findings.extend(advisory_bridge_findings);
    let (trajectory_report, trajectory_findings) = build_trajectory_report(&normalized_records);
    findings.extend(trajectory_findings);
    let normalized_record_count = normalized_records.len();
    let malformed_record_count = malformed_records.len();
    let malformed_by_source = malformed_count_by_key(&malformed_records, "source");
    let malformed_by_path = malformed_count_by_key(&malformed_records, "path");
    let malformed_by_file_name = malformed_count_by_key(&malformed_records, "file_name");
    let freshness_ages = normalized_records
        .iter()
        .filter_map(record_freshness_age_seconds)
        .collect::<Vec<_>>();
    let freshness_observed_record_count = freshness_ages.len();
    let max_evidence_age_seconds = freshness_ages.iter().copied().max().unwrap_or(0);
    let stale_record_count = freshness_ages
        .iter()
        .filter(|age| **age > stale_evidence_seconds)
        .count();
    let observation_state = if malformed_record_count > 0 {
        "malformed_evidence"
    } else if normalized_record_count == 0 {
        "data_starved"
    } else if stale_record_count > 0 {
        "stale_evidence"
    } else if missing_required_source_count > 0 {
        "partial_evidence"
    } else {
        "healthy_observation"
    };
    let report = json!({
        "ok": malformed_records.is_empty(),
        "type": "kernel_sentinel_evidence_ingestion",
        "evidence_dir": evidence_dir,
        "require_evidence": require_evidence,
        "observation_state": observation_state,
        "data_starved": normalized_record_count == 0,
        "partial_evidence": normalized_record_count > 0 && missing_required_source_count > 0,
        "malformed_evidence": malformed_record_count > 0,
        "malformed_by_source": malformed_by_source,
        "malformed_by_path": malformed_by_path,
        "malformed_by_file_name": malformed_by_file_name,
        "stale_evidence": stale_record_count > 0,
        "stale_record_count": stale_record_count,
        "freshness_observed_record_count": freshness_observed_record_count,
        "stale_evidence_seconds": stale_evidence_seconds,
        "max_evidence_age_seconds": max_evidence_age_seconds,
        "present_source_count": present_source_count,
        "missing_source_count": missing_source_count,
        "present_required_source_count": present_required_source_count,
        "missing_required_source_count": missing_required_source_count,
        "present_optional_source_count": present_optional_source_count,
        "missing_optional_source_count": missing_optional_source_count,
        "coverage": {
            "state": observation_state,
            "present_source_count": present_source_count,
            "missing_source_count": missing_source_count,
            "present_required_source_count": present_required_source_count,
            "missing_required_source_count": missing_required_source_count,
            "present_optional_source_count": present_optional_source_count,
            "missing_optional_source_count": missing_optional_source_count,
            "expected_source_count": present_source_count + missing_source_count,
            "expected_required_source_count": present_required_source_count + missing_required_source_count,
            "expected_optional_source_count": present_optional_source_count + missing_optional_source_count,
            "normalized_record_count": normalized_record_count,
            "malformed_record_count": malformed_record_count,
            "stale_record_count": stale_record_count,
            "freshness_observed_record_count": freshness_observed_record_count,
            "stale_evidence_seconds": stale_evidence_seconds,
            "max_evidence_age_seconds": max_evidence_age_seconds,
            "malformed_by_source": malformed_by_source,
            "malformed_by_path": malformed_by_path,
            "malformed_by_file_name": malformed_by_file_name
        },
        "sources": source_reports,
        "receipt_completeness": receipt_completeness_report,
        "capability_grants": capability_grants_report,
        "state_transitions": state_transitions_report,
        "nexus_boundaries": nexus_boundaries_report,
        "boundedness": boundedness_report,
        "gateway_isolation": gateway_isolation_report,
        "advisory_bridge": advisory_bridge_report,
        "trajectories": trajectory_report,
        "normalized_record_count": normalized_record_count,
        "finding_count": findings.len(),
        "malformed_record_count": malformed_record_count,
        "operator_summary": {
            "observation_state": observation_state,
            "data_starved": normalized_record_count == 0,
            "partial_evidence": normalized_record_count > 0 && missing_required_source_count > 0,
            "malformed_evidence": malformed_record_count > 0,
            "normalized_record_count": normalized_record_count,
            "present_source_count": present_source_count,
            "missing_source_count": missing_source_count,
            "present_required_source_count": present_required_source_count,
            "missing_required_source_count": missing_required_source_count,
            "present_optional_source_count": present_optional_source_count,
            "missing_optional_source_count": missing_optional_source_count,
            "malformed_record_count": malformed_record_count,
            "malformed_by_source": malformed_by_source,
            "malformed_by_path": malformed_by_path,
            "malformed_by_file_name": malformed_by_file_name,
            "stale_evidence": stale_record_count > 0,
            "stale_record_count": stale_record_count,
            "freshness_observed_record_count": freshness_observed_record_count,
            "stale_evidence_seconds": stale_evidence_seconds,
            "max_evidence_age_seconds": max_evidence_age_seconds,
            "recommended_action": if normalized_record_count == 0 {
                "wire existing runtime/eval/proof telemetry into local/state/kernel_sentinel/evidence/*.jsonl before trusting Sentinel health"
            } else if malformed_record_count > 0 {
                "repair malformed Sentinel evidence producer rows before trusting health or release readiness"
            } else if stale_record_count > 0 {
                "refresh stale Sentinel evidence producers before trusting health or RSI readiness"
            } else if missing_required_source_count > 0 {
                "finish collector coverage for missing required Sentinel evidence streams"
            } else {
                "continue monitoring normalized Sentinel evidence streams"
            }
        },
        "normalized_records": normalized_records
    });
    KernelSentinelEvidenceIngestion {
        findings,
        malformed_records,
        report,
    }
}

#[cfg(test)]
mod tests;
