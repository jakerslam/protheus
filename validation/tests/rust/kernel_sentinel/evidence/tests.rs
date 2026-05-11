// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::*;

#[test]
fn required_missing_kernel_sources_become_blocking_findings() {
    let dir = std::env::temp_dir().join("kernel-sentinel-evidence-missing");
    let args = vec![
        "--require-evidence=1".to_string(),
        format!("--evidence-dir={}", dir.display()),
    ];
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
    assert_eq!(
        ingestion.report["normalized_records"][0]["advisory"],
        Value::Bool(true)
    );
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
    assert_eq!(
        ingestion.report["normalized_records"][0]["severity"],
        "critical"
    );
}

#[test]
fn stale_verity_drift_events_are_historical_not_current_receipt_blockers() {
    let dir = std::env::temp_dir().join("kernel-sentinel-evidence-stale-verity-drift");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("kernel_receipts.jsonl"),
        r#"{"id":"old-drift","ok":false,"severity":"critical","category":"receipt_integrity","subject":"drift_events","kind":"kernel_receipt_bridge","type":"verity_drift_violation","ts_ms":1,"summary":"old drift","evidence":["verity://old"]}"#,
    )
    .unwrap();

    let args = vec![format!("--evidence-dir={}", dir.display())];
    let ingestion = ingest_evidence_sources(&dir, &args);

    assert!(ingestion.findings.is_empty());
    assert_eq!(
        ingestion.report["normalized_records"][0]["stale_historical_failure"],
        Value::Bool(true)
    );
}

#[test]
// Regression token: stale_historical_evidence_failure must stay covered here.
    fn stale_generated_at_failures_are_historical_not_current_receipt_blockers() {
    let dir = std::env::temp_dir().join("kernel-sentinel-evidence-stale-generated-at");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("runtime_observations.jsonl"),
        r#"{"id":"old-health","ok":false,"severity":"high","category":"runtime_correctness","subject":"receipts","kind":"runtime_observation_bridge","generated_at":"2026-04-17T22:44:50.318Z","summary":"old health audit failure","evidence":["health://old/receipts"]}"#,
    )
    .unwrap();

    let args = vec![format!("--evidence-dir={}", dir.display())];
    let ingestion = ingest_evidence_sources(&dir, &args);

    assert!(ingestion.findings.is_empty());
    assert_eq!(
        ingestion.report["normalized_records"][0]["stale_historical_failure"],
        Value::Bool(true)
    );
    assert!(ingestion.report["normalized_records"][0]["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row.as_str().unwrap_or("").starts_with("freshness://age_seconds/")));
}

#[test]
fn fresh_verity_drift_events_still_open_receipt_findings() {
    let dir = std::env::temp_dir().join("kernel-sentinel-evidence-fresh-verity-drift");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("kernel_receipts.jsonl"),
        format!(
            r#"{{"id":"fresh-drift","ok":false,"severity":"critical","category":"receipt_integrity","subject":"drift_events","kind":"kernel_receipt_bridge","type":"verity_drift_violation","ts_ms":{},"summary":"fresh drift","evidence":["verity://fresh"]}}"#,
            now_epoch_ms()
        ),
    )
    .unwrap();

    let args = vec![format!("--evidence-dir={}", dir.display())];
    let ingestion = ingest_evidence_sources(&dir, &args);

    assert!(ingestion.findings.iter().any(|finding| {
        finding.fingerprint == "kernel_receipt:drift_events:kernel_receipt_bridge"
            && finding.severity == KernelSentinelSeverity::Critical
    }));
    assert_eq!(
        ingestion.report["normalized_records"][0]["stale_historical_failure"],
        Value::Bool(false)
    );
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
        r#"{"id":"eval-claim","ok":true,"subject":"workflow","kind":"eval_summary","may_block_release":true,"may_write_verdict":true,"evidence":["eval://claim","receipt://kernel/verdict/1"]}"#,
    )
    .unwrap();
    let args = vec![format!("--evidence-dir={}", dir.display())];
    let ingestion = ingest_evidence_sources(&dir, &args);
    assert_eq!(
        ingestion.report["advisory_bridge"]["authority_claim_count"],
        Value::from(1)
    );
    assert!(ingestion.findings.iter().any(|finding| {
        finding.category == KernelSentinelFindingCategory::SecurityBoundary
            && finding.fingerprint == "advisory_bridge:authority_claim:workflow"
    }));
}

#[test]
fn bridge_only_authority_claim_stays_advisory_without_corroboration() {
    let dir = std::env::temp_dir().join("kernel-sentinel-evidence-bridge-only-advisory");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("control_plane_eval.jsonl"),
        r#"{"id":"eval-claim","ok":true,"subject":"workflow","kind":"eval_summary","may_block_release":true,"may_write_verdict":true,"evidence":["eval://claim"]}"#,
    )
    .unwrap();
    let args = vec![format!("--evidence-dir={}", dir.display())];
    let ingestion = ingest_evidence_sources(&dir, &args);
    assert_eq!(
        ingestion.report["advisory_bridge"]["authority_claim_count"],
        Value::from(1)
    );
    assert_eq!(
        ingestion.report["advisory_bridge"]["advisory_only_count"],
        Value::from(1)
    );
    assert!(!ingestion
        .findings
        .iter()
        .any(|finding| { finding.fingerprint == "advisory_bridge:authority_claim:workflow" }));
}

#[test]
fn bridge_only_missing_source_reference_stays_advisory_without_corroboration() {
    let records = vec![json!({
        "id": "eval-source-ref",
        "ok": true,
        "source": "control_plane_eval",
        "subject": "workflow",
        "kind": "eval_summary",
        "may_write_verdict": false
    })];
    let (report, findings) = build_advisory_bridge_report(&records);
    assert_eq!(report["missing_source_reference_count"], Value::from(1));
    assert_eq!(report["advisory_only_count"], Value::from(1));
    assert!(!findings.iter().any(|finding| {
        finding.fingerprint == "advisory_bridge:missing_source_reference:workflow"
    }));
}

#[test]
fn source_reports_distinguish_observed_malformed_and_failed_artifact_states() {
    let dir = std::env::temp_dir().join("kernel-sentinel-evidence-artifact-states");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("runtime_observations.jsonl"),
        r#"{"id":"obs-1","ok":true,"subject":"runtime","kind":"bridge_presence","summary":"runtime bridge observed","evidence":["runtime://obs/1"]}"#,
    )
    .unwrap();
    fs::write(
        dir.join("gateway_health.jsonl"),
        r#"{"id":"gw-1","ok":false,"subject":"gateway","kind":"healthz","summary":"gateway failed","evidence":["gateway://health/1"]}"#,
    )
    .unwrap();
    fs::write(dir.join("control_plane_eval.jsonl"), r#"{"id":"bad-json""#).unwrap();
    let args = vec![format!("--evidence-dir={}", dir.display())];
    let ingestion = ingest_evidence_sources(&dir, &args);
    let sources = ingestion.report["sources"].as_array().unwrap();
    let runtime = sources
        .iter()
        .find(|row| row["file_name"] == "runtime_observations.jsonl")
        .unwrap();
    let gateway = sources
        .iter()
        .find(|row| row["file_name"] == "gateway_health.jsonl")
        .unwrap();
    let eval = sources
        .iter()
        .find(|row| row["file_name"] == "control_plane_eval.jsonl")
        .unwrap();
    assert_eq!(runtime["artifact_state"], "artifact_observed");
    assert_eq!(gateway["artifact_state"], "artifact_failed");
    assert_eq!(eval["artifact_state"], "artifact_malformed");
    assert_eq!(
        ingestion.report["artifact_state_counts"]["artifact_observed"],
        Value::from(1)
    );
    assert_eq!(
        ingestion.report["artifact_state_counts"]["artifact_failed"],
        Value::from(1)
    );
    assert_eq!(
        ingestion.report["artifact_state_counts"]["artifact_malformed"],
        Value::from(1)
    );
}

#[test]
fn authoritative_guard_pass_with_matching_failure_is_reported_as_contradiction() {
    let dir = std::env::temp_dir().join("kernel-sentinel-evidence-guard-consistency");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("gateway_health.jsonl"),
        r#"{"id":"gateway-pass","ok":true,"status":"passed","subject":"gateway","kind":"healthz","pass":true,"evidence":["gateway://health/1"]}"#,
    )
    .unwrap();
    fs::write(
        dir.join("runtime_observations.jsonl"),
        r#"{"id":"runtime-fail","ok":false,"subject":"gateway","kind":"listener_missing","summary":"listener missing while upstream health guard passed","evidence":["gateway://health/1"]}"#,
    )
    .unwrap();
    let args = vec![format!("--evidence-dir={}", dir.display())];
    let ingestion = ingest_evidence_sources(&dir, &args);
    assert_eq!(
        ingestion.report["guard_consistency"]["checked_count"],
        Value::from(1)
    );
    assert_eq!(
        ingestion.report["guard_consistency"]["contradiction_count"],
        Value::from(1)
    );
    assert_eq!(
        ingestion.report["guard_consistency"]["ok"],
        Value::Bool(false)
    );
    assert_eq!(
        ingestion.report["guard_consistency"]["contradictions"][0]["record_id"],
        "gateway-pass"
    );
}

#[test]
fn authoritative_guard_pass_caps_uncorroborated_critical_finding_to_high() {
    let dir = std::env::temp_dir().join("kernel-sentinel-evidence-guard-cap");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("gateway_health.jsonl"),
        r#"{"id":"gateway-pass","ok":true,"status":"passed","subject":"gateway","kind":"healthz","pass":true,"evidence":["gateway://health/1"]}"#,
    )
    .unwrap();
    fs::write(
        dir.join("release_proof_packs.jsonl"),
        r#"{"id":"release-fail","ok":false,"severity":"critical","subject":"gateway","kind":"release_guard_failed","summary":"release guard failed despite upstream pass","evidence":["gateway://health/1"]}"#,
    )
    .unwrap();
    let args = vec![format!("--evidence-dir={}", dir.display())];
    let ingestion = ingest_evidence_sources(&dir, &args);
    let finding = ingestion
        .findings
        .iter()
        .find(|finding| finding.id == "release-fail")
        .unwrap();
    assert_eq!(finding.severity, KernelSentinelSeverity::High);
    assert_eq!(
        ingestion.report["guard_consistency"]["contradictions"][0]["matching_findings"][0]
            ["severity"],
        "high"
    );
}

#[test]
fn sentinel_finding_cites_exact_failing_fields_from_upstream_artifact() {
    let dir = std::env::temp_dir().join("kernel-sentinel-evidence-failure-citations");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("release_proof_packs.jsonl"),
        r#"{"id":"release-fail","ok":false,"status":"error","pass":false,"subject":"release","kind":"release_guard","summary":"release guard failed","evidence":["release://proof/1"],"failing_check":"boundedness_budget"}"#,
    )
    .unwrap();
    let args = vec![format!("--evidence-dir={}", dir.display())];
    let ingestion = ingest_evidence_sources(&dir, &args);
    let finding = ingestion
        .findings
        .iter()
        .find(|finding| finding.id == "release-fail")
        .unwrap();
    assert!(finding
        .evidence
        .iter()
        .any(|row| row == "field://release_proof_pack/release-fail/ok=false"));
    assert!(finding
        .evidence
        .iter()
        .any(|row| row == "field://release_proof_pack/release-fail/status=error"));
    assert!(finding
        .evidence
        .iter()
        .any(|row| row == "field://release_proof_pack/release-fail/pass=false"));
    assert!(finding.evidence.iter().any(
        |row| row == "check://release_proof_pack/release-fail/failing_check=boundedness_budget"
    ));
}

#[test]
fn cross_artifact_truth_consistency_reports_authoritative_artifact_disagreement() {
    let dir = std::env::temp_dir().join("kernel-sentinel-evidence-cross-artifact-truth");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("gateway_health.jsonl"),
        r#"{"id":"gateway-pass","ok":true,"status":"passed","subject":"gateway","kind":"healthz","pass":true,"evidence":["gateway://health/1"]}"#,
    )
    .unwrap();
    fs::write(
        dir.join("runtime_observations.jsonl"),
        r#"{"id":"runtime-fail","ok":false,"status":"failed","subject":"gateway","kind":"listener_missing","summary":"runtime listener missing","evidence":["gateway://health/1"]}"#,
    )
    .unwrap();
    let args = vec![format!("--evidence-dir={}", dir.display())];
    let ingestion = ingest_evidence_sources(&dir, &args);
    assert_eq!(
        ingestion.report["guard_consistency"]["cross_artifact_contradiction_count"],
        Value::from(1)
    );
    assert_eq!(
        ingestion.report["guard_consistency"]["ok"],
        Value::Bool(false)
    );
    assert_eq!(
        ingestion.report["guard_consistency"]["cross_artifact_contradictions"][0]["pass_record"]
            ["record_id"],
        "gateway-pass"
    );
    assert_eq!(
        ingestion.report["guard_consistency"]["cross_artifact_contradictions"][0]["failed_record"]
            ["record_id"],
        "runtime-fail"
    );
}
