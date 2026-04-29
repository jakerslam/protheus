// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::*;

fn finding_with(category: KernelSentinelFindingCategory, fingerprint: &str) -> KernelSentinelFinding {
    KernelSentinelFinding {
        schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        id: "finding-1".to_string(),
        severity: KernelSentinelSeverity::Critical,
        category,
        fingerprint: fingerprint.to_string(),
        evidence: vec!["receipt://one".to_string()],
        summary: fingerprint.to_string(),
        recommended_action: "restore Kernel invariant".to_string(),
        status: "open".to_string(),
    }
}

#[test]
fn hard_fail_preflight_detects_required_proof_pack_gaps() {
    let evidence_report = json!({"normalized_records": [{
        "source": "release_proof_pack",
        "subject": "rc-pack",
        "kind": "proof_pack",
        "evidence": ["proof://rc-pack"],
        "details": {"required_missing": 1}
    }]});
    let (report, findings) = build_governance_preflight(&[], &evidence_report, &[]);
    assert_eq!(report["hard_fail_invariant_count"], Value::from(1));
    assert!(findings.iter().any(|f| f.fingerprint == "hard_fail:missing_proof_pack_required_artifact:rc-pack"));
}

#[test]
fn stale_freshness_record_creates_release_blocking_finding() {
    let evidence_report = json!({"normalized_records": [{
        "source": "runtime_observation",
        "subject": "watch",
        "kind": "background_watch",
        "evidence": ["freshness://watch"],
        "details": {"freshness_age_seconds": 7200}
    }]});
    let args = vec!["--freshness-window-seconds=60".to_string()];
    let (report, findings) = build_governance_preflight(&[], &evidence_report, &args);
    assert_eq!(report["freshness_stale_count"], Value::from(1));
    assert!(findings.iter().any(|f| f.fingerprint == "sentinel_freshness_stale:background_watch:watch"));
}

#[test]
fn release_gate_fails_on_critical_and_passes_on_clean_inputs() {
    let critical = finding_with(KernelSentinelFindingCategory::ReceiptIntegrity, "receipt_forgery:demo");
    let issue = json!({"issue_drafts": []});
    let maintenance = json!({"suggestions": [], "automation_candidates": []});
    let governance = json!({"hard_fail_invariant_count": 0, "freshness_stale_count": 0});
    let evidence = json!({"normalized_record_count": 1, "data_starved": false, "observation_state": "healthy_observation"});
    let architectural_report = json!({
        "multi_layer_incident_count": 0,
        "synthesis_guard": {
            "pass": true,
            "missing_architectural_synthesis_count": 0,
            "missing_remediation_classification_count": 0
        }
    });
    let failed = build_release_gate(&[critical], &[], &architectural_report, &issue, &maintenance, &governance, &evidence);
    assert_eq!(failed["pass"], false);
    let passed = build_release_gate(&[], &[], &architectural_report, &issue, &maintenance, &governance, &evidence);
    assert_eq!(passed["pass"], true);
    assert!(
        passed["required_artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row.as_str() == Some("architectural_incident_report_current.json"))
    );
    assert!(
        passed["proof_pack_manifest_required_artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row.as_str() == Some("architectural_incident_report_current.json"))
    );
}
