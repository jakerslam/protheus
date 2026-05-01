// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::*;
use serde_json::Value;
use std::fs;

#[test]
fn final_report_is_budgeted_and_excludes_raw_evidence_payloads() {
    let root = std::env::temp_dir().join(format!(
        "kernel-sentinel-final-report-budget-{}",
        crate::deterministic_receipt_hash(&json!({"test": "final-report-budget"}))
    ));
    fs::create_dir_all(&root).unwrap();
    let findings_path = root.join("findings.jsonl");
    let mut rows = Vec::new();
    for index in 0..40 {
        rows.push(
            serde_json::to_string(&KernelSentinelFinding {
                schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
                id: format!("ks-budget-{index}"),
                severity: KernelSentinelSeverity::High,
                category: KernelSentinelFindingCategory::RuntimeCorrectness,
                fingerprint: format!("runtime:budget:{index}"),
                evidence: vec![format!("evidence://large-raw-stream/{index}/{}", "x".repeat(2048))],
                summary: format!("large summary should be compacted {}", "s".repeat(2048)),
                recommended_action: format!("large action should be compacted {}", "a".repeat(2048)),
                status: "open".to_string(),
            })
            .unwrap(),
        );
    }
    fs::write(&findings_path, rows.join("\n")).unwrap();

    let args = vec![
        format!("--findings-path={}", findings_path.display()),
        "--report-finding-limit=40".to_string(),
        "--final-report-finding-limit=20".to_string(),
        "--final-report-byte-budget=12000".to_string(),
    ];
    let (report, _verdict, _exit) = build_report(&root, &args);
    let final_report = &report["final_report"];
    assert_eq!(final_report["type"], "kernel_sentinel_final_report");
    assert_eq!(final_report["raw_evidence"]["embedded"], false);
    assert_eq!(final_report["report_budget"]["raw_evidence_embedded"], false);
    assert_eq!(final_report["report_budget"]["full_report_embedded"], false);
    assert_eq!(final_report["report_budget"]["within_budget"], true);
    assert!(serde_json::to_vec(final_report).unwrap().len() <= 12_000);
    assert!(final_report.get("evidence_ingestion").is_none());
    assert!(final_report.get("malformed_findings").is_none());
    assert!(final_report["issue_synthesis"].get("issue_drafts").is_none());
    let top_findings = final_report["top_findings"].as_array().unwrap();
    assert!(top_findings.len() <= 20);
    assert_eq!(final_report["quality_filter"]["triage_finding_count"], 0);
    assert!(top_findings
        .iter()
        .all(|finding| finding["evidence_refs"].as_array().unwrap().len() <= 3));
    assert!(top_findings.iter().all(|finding| {
        finding["summary"].as_str().unwrap().len() < 420
            && finding["recommended_action"].as_str().unwrap().len() < 420
    }));
}

#[test]
fn final_report_releases_only_quality_filtered_findings() {
    let root = std::env::temp_dir().join(format!(
        "kernel-sentinel-final-report-quality-filter-{}",
        crate::deterministic_receipt_hash(&json!({"test": "final-report-quality-filter"}))
    ));
    fs::create_dir_all(&root).unwrap();
    let findings_path = root.join("findings.jsonl");
    let good = KernelSentinelFinding {
        schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        id: "ks-quality-good".to_string(),
        severity: KernelSentinelSeverity::High,
        category: KernelSentinelFindingCategory::GatewayIsolation,
        fingerprint: "gateway:quarantine:flapping".to_string(),
        evidence: vec!["evidence://gateway/quarantine/flapping".to_string()],
        summary: "gateway quarantine recurred across fresh health snapshots".to_string(),
        recommended_action: "repair gateway quarantine routing and add replay coverage".to_string(),
        status: "open".to_string(),
    };
    let weak = KernelSentinelFinding {
        schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        id: "ks-quality-weak".to_string(),
        severity: KernelSentinelSeverity::High,
        category: KernelSentinelFindingCategory::RuntimeCorrectness,
        fingerprint: "runtime:maybe:odd".to_string(),
        evidence: vec!["note://manual-hunch".to_string()],
        summary: "maybe odd".to_string(),
        recommended_action: "look".to_string(),
        status: "open".to_string(),
    };
    fs::write(
        &findings_path,
        format!(
            "{}\n{}",
            serde_json::to_string(&good).unwrap(),
            serde_json::to_string(&weak).unwrap()
        ),
    )
    .unwrap();

    let args = vec![
        format!("--findings-path={}", findings_path.display()),
        "--final-report-finding-limit=10".to_string(),
        "--final-report-byte-budget=16000".to_string(),
    ];
    let (report, _verdict, _exit) = build_report(&root, &args);
    let final_report = &report["final_report"];
    let top_findings = final_report["top_findings"].as_array().unwrap();
    assert_eq!(top_findings.len(), 1);
    assert_eq!(top_findings[0]["id"], "ks-quality-good");
    assert_eq!(top_findings[0]["quality"]["release_ready"], true);
    assert_eq!(top_findings[0]["owner_guess"], "gateways");
    assert_eq!(
        final_report["quality_filter"]["triage_finding_count"],
        1
    );
    assert_eq!(
        final_report["quality_filter"]["released_finding_count"],
        1
    );
    assert_eq!(final_report["triage_findings"][0]["id"], "ks-quality-weak");
    assert_eq!(
        final_report["triage_findings"][0]["quality"]["missing_requirements"][0],
        "recurrence_or_freshness_support"
    );
    assert!(final_report["triage_findings"][0]["quality"]["missing_requirements"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row.as_str() == Some("concrete_next_action")));
}

#[test]
fn final_report_clusters_release_ready_symptoms_by_root_cause() {
    let root = std::env::temp_dir().join(format!(
        "kernel-sentinel-final-report-root-cause-clusters-{}",
        crate::deterministic_receipt_hash(&json!({"test": "final-report-root-cause-clusters"}))
    ));
    fs::create_dir_all(&root).unwrap();
    let findings_path = root.join("findings.jsonl");
    let bounded_a = KernelSentinelFinding {
        schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        id: "ks-cluster-bounded-a".to_string(),
        severity: KernelSentinelSeverity::High,
        category: KernelSentinelFindingCategory::Boundedness,
        fingerprint: "boundedness:workspace_queue:a".to_string(),
        evidence: vec!["evidence://boundedness/workspace/a".to_string()],
        summary: "workspace queue exceeded boundedness budget in a fresh run".to_string(),
        recommended_action: "repair workspace queue boundedness and add replay coverage".to_string(),
        status: "open".to_string(),
    };
    let bounded_b = KernelSentinelFinding {
        id: "ks-cluster-bounded-b".to_string(),
        fingerprint: "boundedness:workspace_queue:b".to_string(),
        evidence: vec!["evidence://boundedness/workspace/b".to_string()],
        summary: "workspace queue exceeded boundedness budget in another fresh run".to_string(),
        ..bounded_a.clone()
    };
    let gateway = KernelSentinelFinding {
        schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        id: "ks-cluster-gateway".to_string(),
        severity: KernelSentinelSeverity::High,
        category: KernelSentinelFindingCategory::GatewayIsolation,
        fingerprint: "gateway:quarantine:flapping".to_string(),
        evidence: vec!["evidence://gateway/quarantine/flapping".to_string()],
        summary: "gateway quarantine flapped across fresh health snapshots".to_string(),
        recommended_action: "repair gateway quarantine routing and add replay coverage".to_string(),
        status: "open".to_string(),
    };
    fs::write(
        &findings_path,
        [bounded_a, bounded_b, gateway]
            .iter()
            .map(|finding| serde_json::to_string(finding).unwrap())
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .unwrap();

    let args = vec![
        format!("--findings-path={}", findings_path.display()),
        "--final-report-finding-limit=10".to_string(),
        "--final-report-byte-budget=16000".to_string(),
    ];
    let (report, _verdict, _exit) = build_report(&root, &args);
    let final_report = &report["final_report"];
    assert_eq!(final_report["quality_filter"]["released_finding_count"], 3);
    assert_eq!(final_report["quality_filter"]["clustered_top_finding_count"], 2);
    assert_eq!(
        final_report["quality_filter"]["clustered_duplicate_finding_count"],
        1
    );
    assert_eq!(final_report["root_cause_clustering"]["cluster_count"], 2);
    let clusters = final_report["root_cause_clusters"].as_array().unwrap();
    let bounded_cluster = clusters
        .iter()
        .find(|cluster| cluster["fingerprint_family"] == "boundedness:workspace_queue")
        .unwrap();
    assert_eq!(bounded_cluster["occurrence_count"], 2);
    assert_eq!(bounded_cluster["owner_guess"], "observability");
    assert!(bounded_cluster["sample_finding_ids"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id.as_str() == Some("ks-cluster-bounded-b")));
    assert!(final_report["top_findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|finding| finding["cluster"]["occurrence_count"] == 2));
}

#[test]
fn final_report_collapses_findings_when_budget_is_tiny() {
    let root = std::env::temp_dir().join(format!(
        "kernel-sentinel-final-report-tiny-budget-{}",
        crate::deterministic_receipt_hash(&json!({"test": "final-report-tiny-budget"}))
    ));
    fs::create_dir_all(&root).unwrap();
    let findings_path = root.join("findings.jsonl");
    fs::write(
        &findings_path,
        serde_json::to_string(&KernelSentinelFinding {
            schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
            id: "ks-budget-tiny".to_string(),
            severity: KernelSentinelSeverity::High,
            category: KernelSentinelFindingCategory::Boundedness,
            fingerprint: "boundedness:large-report".to_string(),
            evidence: vec!["evidence://stream/ref".to_string()],
            summary: "large report surface".to_string(),
            recommended_action: "keep raw evidence out of final reports".to_string(),
            status: "open".to_string(),
        })
        .unwrap(),
    )
    .unwrap();
    let args = vec![
        format!("--findings-path={}", findings_path.display()),
        "--final-report-finding-limit=1".to_string(),
        "--final-report-byte-budget=1".to_string(),
    ];
    let (report, _verdict, _exit) = build_report(&root, &args);
    let final_report = &report["final_report"];
    assert_eq!(final_report["top_findings"], Value::Array(Vec::new()));
    assert_eq!(final_report["report_budget"]["retained_top_finding_count"], 0);
    assert_eq!(final_report["report_budget"]["dropped_top_finding_count"], 1);
    assert_eq!(final_report["raw_evidence"]["embedded"], false);
}
