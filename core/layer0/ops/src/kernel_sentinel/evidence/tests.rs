// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

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
        r#"{"id":"eval-1","ok":false,"severity":"critical","subject":"chat","kind":"wrong_tool","summary":"web search routed for file request","source_reference":"eval://round/1","evidence":["eval://round/1"]}"#,
    )
    .unwrap();
    let args = vec![format!("--evidence-dir={}", dir.display())];
    let ingestion = ingest_evidence_sources(&dir, &args);
    assert_eq!(ingestion.findings.len(), 1);
    assert_eq!(ingestion.findings[0].severity, KernelSentinelSeverity::High);
    assert_eq!(ingestion.report["normalized_records"][0]["advisory"], Value::Bool(true));
}

#[test]
fn severity_without_explicit_failure_signal_does_not_open_finding() {
    let dir = std::env::temp_dir().join("kernel-sentinel-evidence-severity-observed");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("runtime_observations.jsonl"),
        r#"{"id":"obs-1","ok":true,"severity":"critical","subject":"runtime","kind":"bridge_presence","summary":"runtime bridge observed","evidence":["runtime://obs/1"]}"#,
    )
    .unwrap();
    let args = vec![format!("--evidence-dir={}", dir.display())];
    let ingestion = ingest_evidence_sources(&dir, &args);
    assert!(ingestion.findings.is_empty());
    assert_eq!(ingestion.report["normalized_records"][0]["severity"], "critical");
}

#[test]
fn runtime_collector_catalog_ingests_scheduler_and_boundedness_files() {
    let dir = std::env::temp_dir().join("kernel-sentinel-runtime-collector-catalog");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("scheduler_admission.jsonl"),
        r#"{"id":"sched-1","ok":false,"subject":"kernel-admission","kind":"scheduler_admission_denied","summary":"admission denied without receipt","evidence":["scheduler://admission/1"]}"#,
    )
    .unwrap();
    fs::write(
        dir.join("boundedness_observations.jsonl"),
        r#"{"id":"bounded-1","subject":"runtime","kind":"boundedness_report","max_rss_mb":900,"rss_ceiling_mb":512,"evidence":["boundedness://runtime/rss"]}"#,
    )
    .unwrap();
    let args = vec![format!("--evidence-dir={}", dir.display())];
    let ingestion = ingest_evidence_sources(&dir, &args);
    assert!(ingestion.report["sources"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["file_name"] == "scheduler_admission.jsonl"
            && row["collector_family"] == "scheduler_admission"));
    assert!(ingestion.findings.iter().any(|finding| {
        finding.category == KernelSentinelFindingCategory::Boundedness
            && finding.fingerprint == "boundedness:rss_ceiling:runtime"
    }));
}

#[test]
fn control_plane_eval_authority_claim_opens_advisory_bridge_finding() {
    let dir = std::env::temp_dir().join("kernel-sentinel-evidence-advisory-claim");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("control_plane_eval.jsonl"),
        r#"{"id":"eval-claim","ok":true,"subject":"workflow","kind":"eval_summary","may_block_release":true,"may_write_verdict":true,"evidence":["eval://claim"]}"#,
    )
    .unwrap();
    let args = vec![format!("--evidence-dir={}", dir.display())];
    let ingestion = ingest_evidence_sources(&dir, &args);
    assert_eq!(ingestion.report["advisory_bridge"]["authority_claim_count"], Value::from(1));
    assert!(ingestion.findings.iter().any(|finding| {
        finding.category == KernelSentinelFindingCategory::SecurityBoundary
            && finding.fingerprint == "advisory_bridge:authority_claim:workflow"
    }));
}
