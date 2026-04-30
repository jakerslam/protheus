use super::self_dossier::build_infring_self_dossier;
use super::SystemUnderstandingDossier;
use serde_json::{json, Value};
use std::path::Path;

fn sample_inputs(
    missing_required_source_count: u64,
    scheduler_stale: bool,
    release_gate_pass: bool,
    trend_history_runs: u64,
) -> (Value, Value, Value) {
    let report = json!({
        "contract_version": 1,
        "generated_at": "2026-04-29T00:00:00Z",
        "operator_summary": {
            "release_gate_pass": release_gate_pass,
            "scheduler_stale": scheduler_stale,
            "scheduler_status": if scheduler_stale { "stale" } else { "fresh" },
            "observation_state": "healthy_observation",
            "missing_required_source_count": missing_required_source_count,
            "release_blockers": [],
        },
        "architectural_incident_report": {
            "incidents": []
        }
    });
    let verdict = json!({
        "verdict": if release_gate_pass { "allow" } else { "release_fail" }
    });
    let self_study_outputs = json!({
        "trend_history_runs": trend_history_runs,
        "top_holes_path": "local/state/kernel_sentinel/top_system_holes_current.json",
        "rsi_readiness_path": "local/state/kernel_sentinel/rsi_readiness_summary_current.json",
        "trend_report_path": "local/state/kernel_sentinel/sentinel_trend_report_current.json",
        "feedback_inbox_path": "local/state/kernel_sentinel/feedback_inbox.jsonl",
        "daily_report_path": "local/state/kernel_sentinel/daily_report.md"
    });
    (report, verdict, self_study_outputs)
}

#[test]
fn low_dossier_confidence_blocks_structural_recommendations_and_emits_probes() {
    let (report, verdict, self_study_outputs) = sample_inputs(2, true, false, 1);
    let diagnostic_run = json!({
        "type": "kernel_sentinel_diagnostic_run",
        "diagnostic_follow_up_request_count": 1,
        "authorized_probe_count": 1
    });
    let dossier: SystemUnderstandingDossier = serde_json::from_value(
        build_infring_self_dossier(
            Path::new("."),
            &report,
            &verdict,
            &self_study_outputs,
            &diagnostic_run,
        )
        .unwrap(),
    )
    .unwrap();
    assert!(dossier.implementation_items.is_empty());
    assert!(dossier
        .required_next_probes
        .iter()
        .any(|row| row == "fill_missing_required_sentinel_sources"));
    assert!(dossier
        .required_next_probes
        .iter()
        .any(|row| row == "raise_runtime_dossier_confidence"));
    assert!(dossier.blocking_unknowns.iter().any(|row| {
        row == "structural_recommendations_blocked_until_dossier_confidence_recovers"
    }));
}

#[test]
fn confident_dossier_keeps_structural_recommendations_available() {
    let (report, verdict, self_study_outputs) = sample_inputs(0, false, true, 3);
    let diagnostic_run = json!({
        "type": "kernel_sentinel_diagnostic_run",
        "diagnostic_follow_up_request_count": 1,
        "authorized_probe_count": 1
    });
    let dossier: SystemUnderstandingDossier = serde_json::from_value(
        build_infring_self_dossier(
            Path::new("."),
            &report,
            &verdict,
            &self_study_outputs,
            &diagnostic_run,
        )
        .unwrap(),
    )
    .unwrap();
    assert!(!dossier.implementation_items.is_empty());
    assert!(!dossier.blocking_unknowns.iter().any(|row| {
        row == "structural_recommendations_blocked_until_dossier_confidence_recovers"
    }));
    assert!(!dossier
        .required_next_probes
        .iter()
        .any(|row| row.starts_with("raise_")));
}
