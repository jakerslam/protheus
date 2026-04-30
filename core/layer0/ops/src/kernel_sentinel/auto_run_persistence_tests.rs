use super::*;
use crate::kernel_sentinel::{
    validate_system_understanding_dossier, SystemUnderstandingDossier,
};
use serde_json::Value;
use std::fs;

#[test]
fn auto_run_writes_freshness_artifact_for_clean_state() {
    let root = std::env::temp_dir().join(format!(
        "kernel-sentinel-auto-clean-{}",
        crate::deterministic_receipt_hash(&serde_json::json!({
            "test": "auto-clean",
            "nonce": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        }))
    ));
    super::write_required_sentinel_evidence(&root);
    super::write_fresh_scheduler_state(&root);
    let out = root.join("auto.json");
    let args = vec![
        "--strict=1".to_string(),
        "--cadence=maintenance".to_string(),
        format!("--auto-artifact={}", out.display()),
    ];
    let exit = run_auto(&root, &args);
    assert_eq!(exit, 0);
    let artifact: Value = serde_json::from_str(&fs::read_to_string(out).unwrap()).unwrap();
    assert_eq!(artifact["type"], "kernel_sentinel_auto_run");
    assert_eq!(artifact["automatic"], true);
    assert_eq!(artifact["release_gate_contract"]["required_for_release_verdict"], true);
    assert_eq!(artifact["release_gate_contract"]["architectural_synthesis_required"], true);
    assert!(
        artifact["output_artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row.as_str() == Some("architectural_incident_report_current.json"))
    );
    assert_eq!(artifact["self_study_outputs"]["type"], "kernel_sentinel_self_study_outputs");
    assert_eq!(artifact["self_study_outputs"]["trend_history_runs"], 1);
    assert_eq!(artifact["scheduler_status"], "fresh");
    assert_eq!(artifact["ok"], true);
    let health_path = root.join("local/state/kernel_sentinel/kernel_sentinel_health_current.json");
    let health: Value = serde_json::from_str(&fs::read_to_string(health_path).unwrap()).unwrap();
    let architectural_incident_report: Value = serde_json::from_str(
        &fs::read_to_string(
            root.join("local/state/kernel_sentinel/architectural_incident_report_current.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let dossier: SystemUnderstandingDossier = serde_json::from_str(
        &fs::read_to_string(root.join("local/state/system_understanding/infring_dossier.json"))
            .unwrap(),
    )
    .unwrap();
    let internal_rsi_bundle: Value = serde_json::from_str(
        &fs::read_to_string(
            root.join("local/state/kernel_sentinel/internal_rsi_proposals_current.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let diagnostic_run: Value = serde_json::from_str(
        &fs::read_to_string(
            root.join("local/state/kernel_sentinel/kernel_sentinel_diagnostic_run_current.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let dossier_markdown = fs::read_to_string(
        root.join("docs/workspace/system_understanding/infring_dossier.md"),
    )
    .unwrap();
    assert_eq!(health["type"], "kernel_sentinel_health_report");
    assert_eq!(health["diagnostic_report"]["type"], "kernel_sentinel_diagnostic_report_section");
    assert_eq!(
        architectural_incident_report["type"],
        "kernel_sentinel_architectural_incident_report_section"
    );
    assert!(validate_system_understanding_dossier(&dossier).is_ok());
    assert_eq!(dossier.target_system, "InfRing");
    assert!(dossier_markdown.contains("# InfRing System Understanding Dossier"));
    assert!(dossier_markdown.contains("## Runtime Behavior"));
    assert!(dossier_markdown.contains("## Authority / Truth Model"));
    assert_eq!(internal_rsi_bundle["type"], "kernel_sentinel_internal_rsi_proposal_bundle");
    assert_eq!(internal_rsi_bundle["mode"], "probe_first");
    assert_eq!(diagnostic_run["type"], "kernel_sentinel_diagnostic_run");
    assert_eq!(
        artifact["diagnostic_report"]["type"],
        "kernel_sentinel_diagnostic_report_section"
    );
    assert_eq!(
        artifact["diagnostic_report"]["probes_run"],
        diagnostic_run["authorized_probe_count"]
    );
    assert_eq!(
        health["diagnostic_report"]["probes_run"],
        diagnostic_run["authorized_probe_count"]
    );
    assert!(internal_rsi_bundle["required_next_probes"].as_array().unwrap().iter().any(|row| row.as_str() == Some("accumulate_three_kernel_sentinel_trend_runs")));
    assert!(dossier.runtime_evidence.iter().any(|row| row.contains("kernel_sentinel_diagnostic_run_current.json")));
    assert!(dossier.runtime_evidence.iter().any(|row| row.contains("kernel_sentinel_health_current.json")));
    assert!(dossier.runtime_evidence.iter().any(|row| row.starts_with("scheduler_status:")));
    assert!(dossier.authority_evidence.iter().any(|row| row.contains("kernel_sentinel_diagnostic_run_current.json")));
    assert!(dossier.authority_evidence.iter().any(|row| row.contains("kernel_sentinel_verdict.json")));
    assert!(dossier.authority_evidence.iter().any(|row| row == "release_gate_pass:true"));
    assert!(dossier.evidence_index.iter().any(|row| row.contains("top_system_holes_current.json")));
    assert!(dossier.evidence_index.iter().any(|row| row.contains("feedback_inbox.jsonl")));
    assert!(dossier.evidence_index.iter().any(|row| row.contains("kernel_sentinel_diagnostic_run_current.json")));
    assert!(dossier.confidence_overall > 0.60);
    assert!(dossier.runtime_confidence >= 0.70);
    assert!(dossier.authority_confidence >= 0.80);
    assert!(dossier.capability_confidence >= 0.70);
    assert!(dossier.transfer_confidence >= 0.80);
    assert!(dossier.syntax_confidence > 0.60);
    assert_eq!(
        artifact["system_understanding_dossier_path"],
        root.join("local/state/system_understanding/infring_dossier.json")
            .display()
            .to_string()
    );
    assert!(artifact["output_artifacts"].as_array().unwrap().iter().any(|row| row.as_str() == Some("internal_rsi_proposals_current.json")));
    assert!(artifact["output_artifacts"].as_array().unwrap().iter().any(|row| row.as_str() == Some("kernel_sentinel_diagnostic_run_current.json")));
    assert!(artifact["output_artifacts"].as_array().unwrap().iter().any(|row| row.as_str() == Some("system_understanding/infring_dossier.json")));
    assert!(artifact["output_artifacts"].as_array().unwrap().iter().any(|row| row.as_str() == Some("docs/workspace/system_understanding/infring_dossier.md")));
    assert!(artifact["diagnostic_run_path"]
        .as_str()
        .unwrap_or("")
        .contains("kernel_sentinel_diagnostic_run_current.json"));
    assert_eq!(health["freshness"]["scheduler_status"], "fresh");
    assert_eq!(health["coverage"]["present_required_source_count"], 14);
    assert_eq!(health["trend"]["status"], "first_run");
    assert_eq!(health["trend"]["history_run_count"], 1);
    assert_eq!(health["trend"]["regression_count"], 0);
    assert_eq!(health["trend"]["improvement_count"], 0);
    assert_eq!(health["authority_safety"]["safe_for_observation_authority"], true);
    assert_eq!(health["authority_safety"]["safe_for_automation_authority"], false);
}
