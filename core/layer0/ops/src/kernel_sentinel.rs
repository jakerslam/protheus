// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

mod authority; mod auto_run; mod boot_watch; mod cli_args; mod collector; mod evidence; mod failure_level; mod finding_lifecycle; mod findings_io; mod governance; mod graders; mod incident_clustering; mod incident_event; mod incident_report; mod incident_synthesis; mod invariant_registry; mod issue_cluster_semantics; mod issue_synthesis; mod maintenance_synthesis; mod release_gate_synthesis; mod report_summary; mod rsi_handoff; mod scheduler; mod self_dossier; mod self_dossier_markdown; mod self_study; mod system_understanding_dossier; mod waivers;
pub use authority::{authority_rule, kernel_sentinel_contract};
use cli_args::{bool_flag, option_path, option_usize, state_dir_from_args};
pub use evidence::{ingest_evidence_sources, KernelSentinelEvidenceIngestion};
pub use failure_level::{
    kernel_sentinel_failure_level_for_finding, kernel_sentinel_failure_level_for_parts,
    kernel_sentinel_failure_level_taxonomy, kernel_sentinel_root_frame_for_finding,
    kernel_sentinel_semantic_frame_for_finding, kernel_sentinel_semantic_frame_for_parts,
    KernelSentinelFailureLevel, KERNEL_SENTINEL_FAILURE_LEVELS,
};
pub use invariant_registry::{
    kernel_sentinel_invariant_by_id, kernel_sentinel_invariant_registry,
    kernel_sentinel_invariant_registry_report, KernelSentinelInvariant,
    KERNEL_SENTINEL_INVARIANTS,
};
pub use incident_event::{
    kernel_sentinel_incident_event_model, validate_kernel_sentinel_incident_event,
    KernelSentinelIncidentEvent, KernelSentinelIncidentEvidenceLevel,
    KERNEL_SENTINEL_INCIDENT_EVENT_SCHEMA_VERSION,
};
pub use incident_clustering::{
    cluster_kernel_sentinel_incident_events, KernelSentinelIncidentCluster,
    KernelSentinelIncidentClusterKey,
};
pub use incident_synthesis::{
    kernel_sentinel_architectural_issue_template,
    synthesize_kernel_sentinel_architectural_incidents,
    KernelSentinelArchitecturalIncident,
};
pub use system_understanding_dossier::{
    kernel_system_understanding_dossier_model, validate_system_understanding_dossier,
    SystemUnderstandingCapabilityKind, SystemUnderstandingCapabilityRow,
    SystemUnderstandingCapabilityValue, SystemUnderstandingDossier,
    SystemUnderstandingDossierStatus, SystemUnderstandingDossierTargetMode,
    SystemUnderstandingTransferTarget, SYSTEM_UNDERSTANDING_DOSSIER_SCHEMA_VERSION,
};
pub use incident_report::kernel_sentinel_architectural_incident_report_section;
use finding_lifecycle::{dedupe_findings, sanitize_finding};
use findings_io::read_jsonl_findings;
use report_summary::{
    build_health_report, count_by_category, count_by_severity, count_by_status,
    count_malformed_by_source, count_malformed_by_source_kind, critical_open_count,
    release_blockers,
};

pub const KERNEL_SENTINEL_NAME: &str = "Kernel Sentinel";
pub const KERNEL_SENTINEL_MODULE_ID: &str = "kernel_sentinel";
pub const KERNEL_SENTINEL_CLI_DOMAIN: &str = "kernel-sentinel";
pub const KERNEL_SENTINEL_CONTRACT_VERSION: u32 = 1;
pub const KERNEL_SENTINEL_FINDING_SCHEMA_VERSION: u32 = 1;
const DEFAULT_REPORT_FINDING_LIMIT: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelSentinelEvidenceSource {
    KernelReceipt,
    RuntimeObservation,
    ReleaseProofPack,
    GatewayHealth,
    QueueBackpressure,
    ControlPlaneEval,
    ShellTelemetry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelSentinelAuthorityClass {
    DeterministicKernelAuthority,
    AdvisoryWorkflowQuality,
    PresentationTelemetryOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelSentinelAuthorityRule {
    pub source: KernelSentinelEvidenceSource,
    pub authority_class: KernelSentinelAuthorityClass,
    pub may_open_finding: bool,
    pub may_write_verdict: bool,
    pub may_block_release: bool,
    pub may_waive_finding: bool,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelSentinelSeverity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelSentinelFindingCategory {
    ReceiptIntegrity,
    CapabilityEnforcement,
    StateTransition,
    NexusBoundary,
    Boundedness,
    GatewayIsolation,
    QueueBackpressure,
    RetryStorm,
    ReleaseEvidence,
    SelfMaintenanceLoop,
    SecurityBoundary,
    RuntimeCorrectness,
    PerformanceRegression,
    AutomationCandidate,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct KernelSentinelFinding {
    pub schema_version: u32,
    pub id: String,
    pub severity: KernelSentinelSeverity,
    pub category: KernelSentinelFindingCategory,
    pub fingerprint: String,
    pub evidence: Vec<String>,
    pub summary: String,
    pub recommended_action: String,
    pub status: String,
}

impl Serialize for KernelSentinelFinding {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let failure_level = kernel_sentinel_failure_level_for_finding(self);
        let root_frame = kernel_sentinel_root_frame_for_finding(self);
        let mut state = serializer.serialize_struct("KernelSentinelFinding", 12)?;
        state.serialize_field("schema_version", &self.schema_version)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("severity", &self.severity)?;
        state.serialize_field("category", &self.category)?;
        state.serialize_field("fingerprint", &self.fingerprint)?;
        state.serialize_field("evidence", &self.evidence)?;
        state.serialize_field("summary", &self.summary)?;
        state.serialize_field("recommended_action", &self.recommended_action)?;
        state.serialize_field("status", &self.status)?;
        state.serialize_field("failure_level", failure_level.code())?;
        state.serialize_field("root_frame", root_frame)?;
        state.serialize_field("remediation_level", failure_level.remediation_level())?;
        state.end()
    }
}

pub fn validate_finding(finding: &KernelSentinelFinding) -> Result<(), String> {
    if finding.schema_version != KERNEL_SENTINEL_FINDING_SCHEMA_VERSION {
        return Err("invalid_schema_version".to_string());
    }
    for (field, value) in [
        ("id", finding.id.as_str()),
        ("fingerprint", finding.fingerprint.as_str()),
        ("summary", finding.summary.as_str()),
        ("recommended_action", finding.recommended_action.as_str()),
        ("status", finding.status.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("missing_{field}"));
        }
    }
    if finding.evidence.is_empty() || finding.evidence.iter().any(|row| row.trim().is_empty()) {
        return Err("missing_evidence".to_string());
    }
    Ok(())
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let body = serde_json::to_string_pretty(value).map_err(|err| err.to_string())?;
    fs::write(path, format!("{body}\n")).map_err(|err| err.to_string())
}

pub fn build_report(root: &Path, args: &[String]) -> (Value, Value, i32) {
    let dir = state_dir_from_args(root, args);
    let findings_path = option_path(args, "--findings-path", dir.join("findings.jsonl"));
    let (mut findings, mut malformed) = read_jsonl_findings(&findings_path);
    let KernelSentinelEvidenceIngestion {
        findings: evidence_findings,
        malformed_records: evidence_malformed,
        report: evidence_report,
    } = ingest_evidence_sources(&dir, args);
    findings.extend(evidence_findings);
    malformed.extend(evidence_malformed);
    let (boot_watch_report, boot_watch_findings) = boot_watch::build_boot_watch_report(&dir, args);
    findings.extend(boot_watch_findings);
    let (waiver_report, waiver_findings) = waivers::apply_waivers(&mut findings, &dir, args);
    findings.extend(waiver_findings);
    let (governance_preflight, governance_findings) =
        governance::build_governance_preflight(&findings, &evidence_report, args);
    findings.extend(governance_findings);
    let architectural_incident_report =
        incident_report::kernel_sentinel_architectural_incident_report_section(&findings);
    let issue_synthesis = issue_synthesis::build_issue_synthesis(&findings, args);
    let maintenance_synthesis = maintenance_synthesis::build_maintenance_synthesis(&findings, args);
    let deduped = dedupe_findings(findings);
    let report_finding_limit = option_usize(args, "--report-finding-limit", DEFAULT_REPORT_FINDING_LIMIT);
    let report_findings = deduped
        .iter()
        .cloned()
        .take(report_finding_limit)
        .map(sanitize_finding)
        .collect::<Vec<_>>();
    let truncated_finding_count = deduped.len().saturating_sub(report_findings.len());
    let critical_open_count = critical_open_count(&deduped);
    let release_gate = governance::build_release_gate(
        &deduped,
        &malformed,
        &architectural_incident_report,
        &issue_synthesis,
        &maintenance_synthesis,
        &governance_preflight,
        &evidence_report,
    );
    let scheduler_health = scheduler::build_scheduler_health_summary(root, args);
    let scheduler_stale = scheduler_health["stale"].as_bool().unwrap_or(true);
    let scheduler_status = scheduler_health["status"]
        .as_str()
        .unwrap_or("unconfigured")
        .to_string();
    let scheduler_running = scheduler_health["running"].as_bool().unwrap_or(false);
    let strict = bool_flag(args, "--strict");
    let release_gate_pass = release_gate["pass"].as_bool().unwrap_or(false);
    let release_blockers =
        release_blockers(critical_open_count, malformed.len(), release_gate_pass, scheduler_stale);
    let verdict_state = if !malformed.is_empty() {
        "invalid"
    } else if critical_open_count > 0 || !release_gate_pass || scheduler_stale {
        "release_fail"
    } else {
        "allow"
    };
    let verdict = json!({
        "ok": malformed.is_empty()
            && critical_open_count == 0
            && release_gate_pass
            && !scheduler_stale,
        "type": "kernel_sentinel_verdict",
        "contract_version": KERNEL_SENTINEL_CONTRACT_VERSION,
        "verdict": verdict_state,
        "strict": strict,
        "critical_open_count": critical_open_count,
        "malformed_finding_count": malformed.len(),
        "finding_count": deduped.len(),
        "scheduler_stale": scheduler_stale,
        "scheduler_running": scheduler_running,
        "scheduler_status": scheduler_status,
        "release_blockers": release_blockers.clone(),
        "receipt_hash": null
    });
    let mut verdict = verdict;
    verdict["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&verdict));
    let report = json!({
        "ok": verdict["ok"],
        "type": "kernel_sentinel_report",
        "canonical_name": KERNEL_SENTINEL_NAME,
        "state_dir": dir,
        "operator_summary": {
            "status_counts": count_by_status(&deduped),
            "severity_counts": count_by_severity(&deduped),
            "category_counts": count_by_category(&deduped),
            "critical_open_count": critical_open_count,
            "malformed_finding_count": malformed.len(),
            "malformed_by_source_kind": count_malformed_by_source_kind(&malformed),
            "malformed_by_source": count_malformed_by_source(&malformed),
            "report_finding_limit": report_finding_limit,
            "reported_finding_count": report_findings.len(),
            "truncated_finding_count": truncated_finding_count,
            "release_gate_pass": release_gate_pass,
            "observation_state": evidence_report["observation_state"],
            "data_starved": evidence_report["data_starved"],
            "partial_evidence": evidence_report["partial_evidence"],
            "malformed_evidence": evidence_report["malformed_evidence"],
            "evidence_record_count": evidence_report["normalized_record_count"],
            "malformed_evidence_count": evidence_report["malformed_record_count"],
            "present_source_count": evidence_report["present_source_count"],
            "missing_source_count": evidence_report["missing_source_count"],
            "present_required_source_count": evidence_report["present_required_source_count"],
            "missing_required_source_count": evidence_report["missing_required_source_count"],
            "present_optional_source_count": evidence_report["present_optional_source_count"],
            "missing_optional_source_count": evidence_report["missing_optional_source_count"],
            "source_coverage": {
                "required": {
                    "present_count": evidence_report["present_required_source_count"],
                    "missing_count": evidence_report["missing_required_source_count"],
                    "ready": evidence_report["missing_required_source_count"].as_u64().unwrap_or(u64::MAX) == 0
                },
                "optional": {
                    "present_count": evidence_report["present_optional_source_count"],
                    "missing_count": evidence_report["missing_optional_source_count"],
                    "fully_present": evidence_report["missing_optional_source_count"].as_u64().unwrap_or(u64::MAX) == 0
                }
            },
            "stale_evidence": evidence_report["stale_evidence"],
            "stale_record_count": evidence_report["stale_record_count"],
            "freshness_observed_record_count": evidence_report["freshness_observed_record_count"],
            "stale_evidence_seconds": evidence_report["stale_evidence_seconds"],
            "max_evidence_age_seconds": evidence_report["max_evidence_age_seconds"],
            "scheduler_stale": scheduler_stale,
            "scheduler_running": scheduler_running,
            "scheduler_status": scheduler_status,
            "scheduler_health": scheduler_health,
            "release_blockers": release_blockers
        },
        "contract": kernel_sentinel_contract(),
        "finding_schema_version": KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        "findings_path": findings_path,
        "evidence_ingestion": evidence_report,
        "boot_watch": boot_watch_report,
        "governance_preflight": governance_preflight,
        "architectural_incident_report": architectural_incident_report,
        "waivers": waiver_report,
        "release_gate": release_gate,
        "issue_synthesis": issue_synthesis,
        "maintenance_synthesis": maintenance_synthesis,
        "findings": report_findings,
        "malformed_findings": malformed,
        "verdict": verdict
    });
    let exit = if strict
        && (critical_open_count > 0
            || !release_gate_pass
            || scheduler_stale
            || !report["malformed_findings"].as_array().unwrap().is_empty())
    {
        2
    } else {
        0
    };
    let verdict = report["verdict"].clone();
    (report, verdict, exit)
}

pub fn run(root: &Path, args: &[String]) -> i32 {
    let command = args.first().map(String::as_str).unwrap_or("help");
    if command == "help" || command == "--help" || command == "-h" {
        println!("infring-ops kernel-sentinel <run|status|report|auto|collect|schedule|heartbeat|help> [--strict=1|0] [--state-dir=<path>|--state-root=<path>] [--findings-path=<path>] [--evidence-dir=<path>] [--collector-artifact=<path>] [--require-evidence=1] [--issue-threshold=<n>] [--suggestion-threshold=<n>] [--automation-threshold=<n>] [--boot-self-check=1] [--watch-refresh=1] [--waivers-path=<path>] [--cadence=maintenance|release|heartbeat] [--auto-artifact=<path>] [--schedule-artifact=<path>] [--interval-seconds=<n>] [--stale-window-seconds=<n>] [--max-stale-minutes=<n>]");
        println!("{}", serde_json::to_string_pretty(&kernel_sentinel_contract()).unwrap());
        return 0;
    }
    let rest = args.iter().skip(1).cloned().collect::<Vec<_>>();
    if command == "auto" {
        return auto_run::run_auto(root, &rest);
    }
    if command == "collect" {
        return collector::run_collect(root, &rest);
    }
    if command == "schedule" {
        return scheduler::run_schedule(root, &rest);
    }
    if command == "heartbeat" {
        return scheduler::run_heartbeat(root, &rest);
    }
    let (report, verdict, exit) = build_report(root, &rest);
    let dir = state_dir_from_args(root, &rest);
    let report_path = dir.join("kernel_sentinel_report_current.json");
    let verdict_path = dir.join("kernel_sentinel_verdict.json");
    let health_path = dir.join("kernel_sentinel_health_current.json");
    if matches!(command, "run" | "report") {
        if let Err(err) = write_json(&report_path, &report) {
            eprintln!("kernel_sentinel_write_report_failed: {err}");
            return 1;
        }
        if let Err(err) = write_json(&verdict_path, &verdict) {
            eprintln!("kernel_sentinel_write_verdict_failed: {err}");
            return 1;
        }
        if let Err(err) = write_json(&health_path, &build_health_report(&report, &verdict, None)) {
            eprintln!("kernel_sentinel_write_health_failed: {err}");
            return 1;
        }
        if let Err(err) = issue_synthesis::write_issue_drafts_jsonl(&dir.join("issues.jsonl"), &report) {
            eprintln!("kernel_sentinel_write_issues_failed: {err}");
            return 1;
        }
        if let Err(err) = maintenance_synthesis::write_maintenance_jsonl(&dir, &report) {
            eprintln!("kernel_sentinel_write_maintenance_failed: {err}");
            return 1;
        }
        if let Err(err) = boot_watch::write_watch_metadata(&dir, &report, &rest) {
            eprintln!("kernel_sentinel_write_watch_metadata_failed: {err}");
            return 1;
        }
        if let Err(err) = waivers::write_waiver_audit(&dir, &report) {
            eprintln!("kernel_sentinel_write_waiver_audit_failed: {err}");
            return 1;
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(if command == "status" { &verdict } else { &report })
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
    if matches!(command, "run" | "status" | "report") {
        exit
    } else {
        eprintln!("kernel_sentinel_unknown_command: {command}");
        1
    }
}

#[cfg(test)]
mod root_tests;
