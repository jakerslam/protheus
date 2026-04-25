// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

mod evidence;
pub use evidence::{ingest_evidence_sources, KernelSentinelEvidenceIngestion};

pub const KERNEL_SENTINEL_NAME: &str = "Kernel Sentinel";
pub const KERNEL_SENTINEL_MODULE_ID: &str = "kernel_sentinel";
pub const KERNEL_SENTINEL_CLI_DOMAIN: &str = "kernel-sentinel";
pub const KERNEL_SENTINEL_CONTRACT_VERSION: u32 = 1;
pub const KERNEL_SENTINEL_FINDING_SCHEMA_VERSION: u32 = 1;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

pub fn dedupe_findings(findings: Vec<KernelSentinelFinding>) -> Vec<KernelSentinelFinding> {
    let mut by_fingerprint: BTreeMap<String, KernelSentinelFinding> = BTreeMap::new();
    for finding in findings {
        if validate_finding(&finding).is_err() {
            continue;
        }
        by_fingerprint
            .entry(finding.fingerprint.clone())
            .and_modify(|existing| {
                if finding.severity < existing.severity {
                    *existing = finding.clone();
                }
            })
            .or_insert(finding);
    }
    by_fingerprint.into_values().collect()
}

pub fn authority_rule(source: KernelSentinelEvidenceSource) -> KernelSentinelAuthorityRule {
    match source {
        KernelSentinelEvidenceSource::KernelReceipt
        | KernelSentinelEvidenceSource::RuntimeObservation
        | KernelSentinelEvidenceSource::ReleaseProofPack
        | KernelSentinelEvidenceSource::GatewayHealth
        | KernelSentinelEvidenceSource::QueueBackpressure => KernelSentinelAuthorityRule {
            source,
            authority_class: KernelSentinelAuthorityClass::DeterministicKernelAuthority,
            may_open_finding: true,
            may_write_verdict: true,
            may_block_release: true,
            may_waive_finding: false,
            reason: "kernel_owned_evidence_can_drive_sentinel_verdicts_but_cannot_self_waive",
        },
        KernelSentinelEvidenceSource::ControlPlaneEval => KernelSentinelAuthorityRule {
            source,
            authority_class: KernelSentinelAuthorityClass::AdvisoryWorkflowQuality,
            may_open_finding: true,
            may_write_verdict: false,
            may_block_release: false,
            may_waive_finding: false,
            reason: "control_plane_eval_is_advisory_input_only_for_kernel_sentinel",
        },
        KernelSentinelEvidenceSource::ShellTelemetry => KernelSentinelAuthorityRule {
            source,
            authority_class: KernelSentinelAuthorityClass::PresentationTelemetryOnly,
            may_open_finding: false,
            may_write_verdict: false,
            may_block_release: false,
            may_waive_finding: false,
            reason: "shell_telemetry_cannot_authorize_kernel_runtime_truth",
        },
    }
}

pub fn kernel_sentinel_contract() -> Value {
    let authority_rules = [
        KernelSentinelEvidenceSource::KernelReceipt,
        KernelSentinelEvidenceSource::RuntimeObservation,
        KernelSentinelEvidenceSource::ReleaseProofPack,
        KernelSentinelEvidenceSource::GatewayHealth,
        KernelSentinelEvidenceSource::QueueBackpressure,
        KernelSentinelEvidenceSource::ControlPlaneEval,
        KernelSentinelEvidenceSource::ShellTelemetry,
    ]
    .into_iter()
    .map(|source| serde_json::to_value(authority_rule(source)).unwrap_or(Value::Null))
    .collect::<Vec<_>>();

    let mut payload = json!({
        "ok": true,
        "type": KERNEL_SENTINEL_MODULE_ID,
        "contract_version": KERNEL_SENTINEL_CONTRACT_VERSION,
        "canonical_name": KERNEL_SENTINEL_NAME,
        "module_id": KERNEL_SENTINEL_MODULE_ID,
        "cli_domain": KERNEL_SENTINEL_CLI_DOMAIN,
        "not_names": ["kernel_eval_agent", "eval_agent", "control_plane_eval"],
        "mission": "kernel_resident_failure_intelligence_and_runtime_law_verification",
        "priority_order": [
            "failures",
            "security_correctness",
            "reliability_hardening",
            "optimization",
            "automation"
        ],
        "control_plane_eval_relationship": {
            "role": "advisory_workflow_quality_input",
            "may_write_sentinel_verdict": false,
            "may_waive_sentinel_finding": false,
            "promotion_requires_deterministic_kernel_rule": true
        },
        "authority_rules": authority_rules
    });
    let receipt_hash = crate::deterministic_receipt_hash(&payload);
    payload["receipt_hash"] = Value::String(receipt_hash);
    payload
}

fn workspace_root(root: &Path) -> PathBuf {
    std::env::var_os("INFRING_WORKSPACE")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| root.to_path_buf())
}

fn state_dir(root: &Path) -> PathBuf {
    workspace_root(root).join("local/state/kernel_sentinel")
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let body = serde_json::to_string_pretty(value).map_err(|err| err.to_string())?;
    fs::write(path, format!("{body}\n")).map_err(|err| err.to_string())
}

fn read_jsonl_findings(path: &Path) -> (Vec<KernelSentinelFinding>, Vec<Value>) {
    let raw = fs::read_to_string(path).unwrap_or_default();
    let mut findings = Vec::new();
    let mut malformed = Vec::new();
    for (idx, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<KernelSentinelFinding>(trimmed) {
            Ok(finding) if validate_finding(&finding).is_ok() => findings.push(finding),
            Ok(finding) => malformed.push(json!({"line": idx + 1, "id": finding.id, "error": "invalid_finding"})),
            Err(err) => malformed.push(json!({"line": idx + 1, "error": err.to_string()})),
        }
    }
    (findings, malformed)
}

fn bool_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|arg| arg == name || arg == &format!("{name}=1") || arg == &format!("{name}=true"))
}

fn option_path(args: &[String], name: &str, fallback: PathBuf) -> PathBuf {
    let prefix = format!("{name}=");
    args.iter()
        .find_map(|arg| arg.strip_prefix(&prefix).map(PathBuf::from))
        .unwrap_or(fallback)
}

pub fn build_report(root: &Path, args: &[String]) -> (Value, Value, i32) {
    let dir = state_dir(root);
    let findings_path = option_path(args, "--findings-path", dir.join("findings.jsonl"));
    let (mut findings, mut malformed) = read_jsonl_findings(&findings_path);
    let KernelSentinelEvidenceIngestion {
        findings: evidence_findings,
        malformed_records: evidence_malformed,
        report: evidence_report,
    } = ingest_evidence_sources(&dir, args);
    findings.extend(evidence_findings);
    malformed.extend(evidence_malformed);
    let deduped = dedupe_findings(findings);
    let critical_open_count = deduped
        .iter()
        .filter(|f| f.severity == KernelSentinelSeverity::Critical && f.status == "open")
        .count();
    let strict = bool_flag(args, "--strict");
    let verdict_state = if !malformed.is_empty() {
        "invalid"
    } else if critical_open_count > 0 {
        "release_fail"
    } else {
        "allow"
    };
    let verdict = json!({
        "ok": malformed.is_empty() && critical_open_count == 0,
        "type": "kernel_sentinel_verdict",
        "contract_version": KERNEL_SENTINEL_CONTRACT_VERSION,
        "verdict": verdict_state,
        "strict": strict,
        "critical_open_count": critical_open_count,
        "malformed_finding_count": malformed.len(),
        "finding_count": deduped.len(),
        "receipt_hash": null
    });
    let mut verdict = verdict;
    verdict["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&verdict));
    let report = json!({
        "ok": verdict["ok"],
        "type": "kernel_sentinel_report",
        "canonical_name": KERNEL_SENTINEL_NAME,
        "contract": kernel_sentinel_contract(),
        "finding_schema_version": KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        "findings_path": findings_path,
        "evidence_ingestion": evidence_report,
        "findings": deduped,
        "malformed_findings": malformed,
        "verdict": verdict
    });
    let exit = if strict && (critical_open_count > 0 || !report["malformed_findings"].as_array().unwrap().is_empty()) {
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
        println!("infring-ops kernel-sentinel <run|status|report|help> [--strict=1|0] [--findings-path=<path>] [--evidence-dir=<path>] [--require-evidence=1]");
        println!("{}", serde_json::to_string_pretty(&kernel_sentinel_contract()).unwrap());
        return 0;
    }
    let rest = args.iter().skip(1).cloned().collect::<Vec<_>>();
    let (report, verdict, exit) = build_report(root, &rest);
    let dir = state_dir(root);
    let report_path = dir.join("kernel_sentinel_report_current.json");
    let verdict_path = dir.join("kernel_sentinel_verdict.json");
    if matches!(command, "run" | "report") {
        if let Err(err) = write_json(&report_path, &report) {
            eprintln!("kernel_sentinel_write_report_failed: {err}");
            return 1;
        }
        if let Err(err) = write_json(&verdict_path, &verdict) {
            eprintln!("kernel_sentinel_write_verdict_failed: {err}");
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
mod tests {
    use super::*;

    #[test]
    fn kernel_sentinel_contract_uses_distinct_canonical_name() {
        let contract = kernel_sentinel_contract();
        assert_eq!(contract["canonical_name"], KERNEL_SENTINEL_NAME);
        assert_eq!(contract["module_id"], KERNEL_SENTINEL_MODULE_ID);
        assert_eq!(contract["cli_domain"], KERNEL_SENTINEL_CLI_DOMAIN);
        let not_names = contract["not_names"].as_array().unwrap();
        assert!(not_names
            .iter()
            .any(|value| value.as_str() == Some("control_plane_eval")));
        assert!(not_names
            .iter()
            .any(|value| value.as_str() == Some("eval_agent")));
        assert_eq!(
            contract["control_plane_eval_relationship"]["may_write_sentinel_verdict"],
            false
        );
    }

    #[test]
    fn control_plane_eval_cannot_write_or_waive_sentinel_verdicts() {
        let rule = authority_rule(KernelSentinelEvidenceSource::ControlPlaneEval);
        assert_eq!(
            rule.authority_class,
            KernelSentinelAuthorityClass::AdvisoryWorkflowQuality
        );
        assert!(rule.may_open_finding);
        assert!(!rule.may_write_verdict);
        assert!(!rule.may_block_release);
        assert!(!rule.may_waive_finding);
        assert_eq!(
            rule.reason,
            "control_plane_eval_is_advisory_input_only_for_kernel_sentinel"
        );
    }

    #[test]
    fn kernel_owned_evidence_can_drive_fail_closed_verdicts_without_self_waiver() {
        let rule = authority_rule(KernelSentinelEvidenceSource::KernelReceipt);
        assert_eq!(
            rule.authority_class,
            KernelSentinelAuthorityClass::DeterministicKernelAuthority
        );
        assert!(rule.may_open_finding);
        assert!(rule.may_write_verdict);
        assert!(rule.may_block_release);
        assert!(!rule.may_waive_finding);
    }

    #[test]
    fn finding_schema_rejects_missing_evidence_and_dedupes_by_fingerprint() {
        let mut valid = KernelSentinelFinding {
            schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
            id: "ks-1".to_string(),
            severity: KernelSentinelSeverity::High,
            category: KernelSentinelFindingCategory::CapabilityEnforcement,
            fingerprint: "capability:workspace_search:missing_grant".to_string(),
            evidence: vec!["receipt://tool-attempt/1".to_string()],
            summary: "workspace search lacked grant".to_string(),
            recommended_action: "fail closed before execution".to_string(),
            status: "open".to_string(),
        };
        assert!(validate_finding(&valid).is_ok());
        valid.evidence.clear();
        assert_eq!(validate_finding(&valid), Err("missing_evidence".to_string()));
        valid.evidence.push("receipt://tool-attempt/1".to_string());
        let mut critical = valid.clone();
        critical.id = "ks-2".to_string();
        critical.severity = KernelSentinelSeverity::Critical;
        let deduped = dedupe_findings(vec![valid, critical]);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].severity, KernelSentinelSeverity::Critical);
    }

    #[test]
    fn strict_report_fails_on_open_critical_findings() {
        let root = std::env::temp_dir().join(format!(
            "kernel-sentinel-test-{}",
            crate::deterministic_receipt_hash(&json!({"test": "strict"}))
        ));
        let findings_path = root.join("findings.jsonl");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            &findings_path,
            serde_json::to_string(&KernelSentinelFinding {
                schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
                id: "ks-critical".to_string(),
                severity: KernelSentinelSeverity::Critical,
                category: KernelSentinelFindingCategory::ReceiptIntegrity,
                fingerprint: "receipt:missing:mutation".to_string(),
                evidence: vec!["local://state/mutation".to_string()],
                summary: "mutation missing receipt".to_string(),
                recommended_action: "block release until receipt emission is restored".to_string(),
                status: "open".to_string(),
            })
            .unwrap(),
        )
        .unwrap();
        let args = vec![
            "--strict=1".to_string(),
            format!("--findings-path={}", findings_path.display()),
        ];
        let (_report, verdict, exit) = build_report(&root, &args);
        assert_eq!(exit, 2);
        assert_eq!(verdict["verdict"], "release_fail");
        assert_eq!(verdict["critical_open_count"], 1);
    }

    #[test]
    fn evidence_ingestion_adds_runtime_failures_to_report() {
        let root = std::env::temp_dir().join(format!(
            "kernel-sentinel-test-{}",
            crate::deterministic_receipt_hash(&json!({"test": "evidence-ingestion"}))
        ));
        let evidence_dir = root.join("evidence");
        fs::create_dir_all(&evidence_dir).unwrap();
        fs::write(
            evidence_dir.join("runtime_observations.jsonl"),
            r#"{"id":"obs-1","ok":false,"subject":"task-7","kind":"illegal_reopen","summary":"failed task reopened","evidence":["trace://task-7"],"recommended_action":"require rollback receipt before reopen"}"#,
        )
        .unwrap();
        let args = vec![format!("--evidence-dir={}", evidence_dir.display())];
        let (report, verdict, exit) = build_report(&root, &args);
        assert_eq!(exit, 0);
        assert_eq!(verdict["finding_count"], 1);
        assert_eq!(report["evidence_ingestion"]["normalized_record_count"], 1);
        assert_eq!(report["findings"][0]["category"], "runtime_correctness");
    }
}
