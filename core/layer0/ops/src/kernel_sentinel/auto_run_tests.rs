use super::*;
use crate::kernel_sentinel::{
    validate_system_understanding_dossier, SystemUnderstandingDossier,
};
use crate::kernel_sentinel::{
    KernelSentinelFinding, KernelSentinelFindingCategory, KernelSentinelSeverity,
    KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
};
use std::fs;

fn write_required_sentinel_evidence(root: &std::path::Path) {
    let evidence_dir = root.join("local/state/kernel_sentinel/evidence");
    fs::create_dir_all(&evidence_dir).unwrap();
    for (file_name, subject, category) in [
        ("kernel_receipts.jsonl", "receipt-1", "ReceiptIntegrity"),
        ("runtime_observations.jsonl", "runtime-1", "RuntimeCorrectness"),
        ("state_mutations.jsonl", "mutation-1", "StateTransition"),
        ("scheduler_admission.jsonl", "admission-1", "CapabilityEnforcement"),
        ("live_recovery.jsonl", "recovery-1", "RuntimeCorrectness"),
        ("boundedness_observations.jsonl", "boundedness-1", "Boundedness"),
        ("release_proof_packs.jsonl", "proof-pack-1", "ReleaseEvidence"),
        ("release_repairs.jsonl", "repair-1", "ReleaseEvidence"),
        ("gateway_health.jsonl", "gateway-1", "GatewayIsolation"),
        ("gateway_quarantine.jsonl", "quarantine-1", "GatewayIsolation"),
        ("gateway_recovery.jsonl", "gateway-recovery-1", "GatewayIsolation"),
        ("gateway_isolation.jsonl", "gateway-isolation-1", "GatewayIsolation"),
        ("queue_backpressure.jsonl", "queue-1", "QueueBackpressure"),
        ("control_plane_eval.jsonl", "eval-1", "RuntimeCorrectness"),
    ] {
        fs::write(
            evidence_dir.join(file_name),
            format!(
                "{{\"id\":\"{subject}\",\"ok\":true,\"subject\":\"{subject}\",\"kind\":\"required_stream_regression\",\"category\":\"{category}\",\"evidence\":[\"fixture://{subject}\"],\"details\":{{\"source_artifact\":\"fixture://{subject}\",\"freshness_age_seconds\":0}}}}"
            ),
        )
        .unwrap();
    }
}

fn write_collector_backed_required_inputs(root: &std::path::Path) {
    let verity = root.join("local/state/ops/verity");
    fs::create_dir_all(&verity).unwrap();
    fs::write(
        verity.join("receipt.jsonl"),
        r#"{"id":"receipt-1","ok":true,"subject":"receipt-1","kind":"receipt_check","summary":"receipt observed","evidence":["receipt://receipt-1"]}"#,
    )
    .unwrap();

    let runtime = root.join("local/state/ops/system_health_audit");
    fs::create_dir_all(&runtime).unwrap();
    fs::write(
        runtime.join("runtime.jsonl"),
        r#"{"id":"runtime-1","ok":true,"subject":"runtime-1","kind":"runtime_health","summary":"runtime healthy","evidence":["runtime://runtime-1"]}"#,
    )
    .unwrap();

    let eval = root.join("local/state/ops/eval_agent_feedback");
    fs::create_dir_all(&eval).unwrap();
    fs::write(
        eval.join("eval.jsonl"),
        r#"{"id":"eval-1","ok":true,"subject":"eval-1","kind":"eval_feedback","summary":"eval feedback observed","evidence":["control_plane_eval://eval-1"]}"#,
    )
    .unwrap();

    let eval_learning = root.join("local/state/ops/eval_learning_loop");
    fs::create_dir_all(&eval_learning).unwrap();
    fs::write(
        eval_learning.join("learning.jsonl"),
        r#"{"id":"eval-learning-1","ok":true,"subject":"eval-learning-1","kind":"eval_learning_feedback","summary":"eval learning feedback observed","evidence":["control_plane_eval://eval-learning-1"]}"#,
    )
    .unwrap();

    let synthetic = root.join("local/state/ops/synthetic_user_chat_harness");
    fs::create_dir_all(&synthetic).unwrap();
    fs::write(
        synthetic.join("runtime.jsonl"),
        r#"{"id":"synthetic-runtime-1","ok":true,"subject":"synthetic-runtime-1","kind":"synthetic_runtime_observation","summary":"synthetic runtime observation observed","evidence":["runtime://synthetic-runtime-1"]}"#,
    )
    .unwrap();

    let artifacts = root.join("core/local/artifacts");
    fs::create_dir_all(&artifacts).unwrap();
    for (file_name, subject) in [
        ("release_proof_pack_current.json", "proof-pack-1"),
        ("repair_fallback_current.json", "repair-1"),
        ("stateful_upgrade_rollback_gate_current.json", "state-mutation-1"),
        ("agent_surface_status_guard_current.json", "scheduler-1"),
        ("workflow_failure_recovery_current.json", "recovery-1"),
        ("gateway_health_current.json", "gateway-health-1"),
        ("gateway_flapping_quarantine_current.json", "gateway-quarantine-1"),
        ("gateway_auto_heal_recovery_current.json", "gateway-recovery-1"),
        ("gateway_boundary_guard_current.json", "gateway-isolation-1"),
        ("queue_backpressure_current.json", "queue-1"),
        ("boundedness_report_current.json", "boundedness-1"),
    ] {
        fs::write(
            artifacts.join(file_name),
            format!(
                "{{\"id\":\"{subject}\",\"ok\":true,\"subject\":\"{subject}\",\"summary\":\"{subject} observed\",\"evidence\":[\"artifact://{subject}\"]}}"
            ),
        )
        .unwrap();
    }
}

fn write_fresh_scheduler_state(root: &std::path::Path) {
    let state_dir = root.join("local/state/kernel_sentinel");
    fs::create_dir_all(&state_dir).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    for file_name in [
        "kernel_sentinel_schedule_state.json",
        "kernel_sentinel_heartbeat_state.json",
    ] {
        fs::write(
            state_dir.join(file_name),
            serde_json::to_string_pretty(&json!({
                "type": "kernel_sentinel_schedule_state",
                "generated_at": crate::now_iso(),
                "last_attempt_epoch_secs": now,
                "last_success_epoch_secs": now,
                "last_exit_code": 0,
                "stale": false
            }))
            .unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn auto_run_writes_freshness_artifact_for_clean_state() {
    let root = std::env::temp_dir().join(format!(
        "kernel-sentinel-auto-clean-{}",
        crate::deterministic_receipt_hash(&json!({
            "test": "auto-clean",
            "nonce": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        }))
    ));
    write_required_sentinel_evidence(&root);
    write_fresh_scheduler_state(&root);
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
    let dossier_markdown = fs::read_to_string(
        root.join("docs/workspace/system_understanding/infring_dossier.md"),
    )
    .unwrap();
    assert_eq!(health["type"], "kernel_sentinel_health_report");
    assert_eq!(
        architectural_incident_report["type"],
        "kernel_sentinel_architectural_incident_report_section"
    );
    assert!(validate_system_understanding_dossier(&dossier).is_ok());
    assert_eq!(dossier.target_system, "InfRing");
    assert!(dossier_markdown.contains("# InfRing System Understanding Dossier"));
    assert!(dossier_markdown.contains("## Runtime Behavior"));
    assert!(dossier_markdown.contains("## Authority / Truth Model"));
    assert_eq!(
        artifact["system_understanding_dossier_path"],
        root.join("local/state/system_understanding/infring_dossier.json")
            .display()
            .to_string()
    );
    assert!(
        artifact["output_artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row.as_str() == Some("system_understanding/infring_dossier.json"))
    );
    assert!(
        artifact["output_artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row.as_str() == Some("docs/workspace/system_understanding/infring_dossier.md"))
    );
    assert_eq!(health["freshness"]["scheduler_status"], "fresh");
    assert_eq!(health["coverage"]["present_required_source_count"], 14);
    assert_eq!(health["trend"]["status"], "first_run");
    assert_eq!(health["trend"]["history_run_count"], 1);
    assert_eq!(health["trend"]["regression_count"], 0);
    assert_eq!(health["trend"]["improvement_count"], 0);
    assert_eq!(health["authority_safety"]["safe_for_observation_authority"], true);
    assert_eq!(health["authority_safety"]["safe_for_automation_authority"], false);
}

#[test]
fn auto_run_strict_fails_closed_on_open_critical_findings() {
    let root = std::env::temp_dir().join(format!(
        "kernel-sentinel-auto-critical-{}",
        crate::deterministic_receipt_hash(&json!({
            "test": "auto-critical",
            "nonce": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        }))
    ));
    let state_dir = root.join("state/kernel_sentinel");
    fs::create_dir_all(&state_dir).unwrap();
    fs::write(
        state_dir.join("findings.jsonl"),
        serde_json::to_string(&KernelSentinelFinding {
            schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
            id: "ks-auto-critical".to_string(),
            severity: KernelSentinelSeverity::Critical,
            category: KernelSentinelFindingCategory::RuntimeCorrectness,
            fingerprint: "auto:critical:runtime".to_string(),
            evidence: vec!["kernel://auto-critical".to_string()],
            summary: "automatic sentinel run found critical runtime correctness issue".to_string(),
            recommended_action: "block release until the runtime issue is fixed".to_string(),
            status: "open".to_string(),
        })
        .unwrap(),
    )
    .unwrap();
    let out = root.join("auto-critical.json");
    let args = vec![
        "--strict=1".to_string(),
        "--cadence=release".to_string(),
        format!("--state-dir={}", state_dir.display()),
        format!("--auto-artifact={}", out.display()),
    ];
    let exit = run_auto(&root, &args);
    assert_eq!(exit, 2);
    let artifact: Value = serde_json::from_str(&fs::read_to_string(out).unwrap()).unwrap();
    assert_eq!(artifact["ok"], false);
    assert_eq!(artifact["verdict"]["verdict"], "release_fail");
    assert_eq!(artifact["operator_summary"]["critical_open_count"], 1);
    assert_eq!(artifact["scheduler_status"], "unconfigured");
    assert_eq!(artifact["self_study_outputs"]["feedback_item_count"], 1);
    assert_eq!(artifact["self_study_outputs"]["rsi_readiness"]["ready_for_observation"], true);
    assert_eq!(artifact["issue_candidate"]["failure_level"], "L5_self_model_failure");
    assert_eq!(artifact["issue_candidate"]["root_frame"], "system_self_model");
    assert_eq!(
        artifact["issue_candidate"]["remediation_level"],
        "self_model_repair"
    );
}

#[test]
fn auto_run_end_to_end_outputs_stay_consistent() {
    let root = std::env::temp_dir().join(format!(
        "kernel-sentinel-auto-e2e-{}",
        crate::deterministic_receipt_hash(&json!({
            "test": "auto-e2e",
            "nonce": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        }))
    ));
    write_required_sentinel_evidence(&root);
    write_fresh_scheduler_state(&root);
    let state_dir = root.join("local/state/kernel_sentinel");
    fs::write(
        state_dir.join("findings.jsonl"),
        serde_json::to_string(&KernelSentinelFinding {
            schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
            id: "ks-auto-e2e".to_string(),
            severity: KernelSentinelSeverity::High,
            category: KernelSentinelFindingCategory::RuntimeCorrectness,
            fingerprint: "auto:e2e:runtime".to_string(),
            evidence: vec!["kernel://auto-e2e".to_string()],
            summary: "automatic sentinel end-to-end finding".to_string(),
            recommended_action: "preserve feedback, issue, and maintenance synthesis".to_string(),
            status: "open".to_string(),
        })
        .unwrap(),
    )
    .unwrap();

    let out = root.join("auto-e2e.json");
    let args = vec![
        "--strict=1".to_string(),
        "--cadence=maintenance".to_string(),
        format!("--auto-artifact={}", out.display()),
    ];
    let exit = run_auto(&root, &args);
    assert_eq!(exit, 0);

    let artifact: Value = serde_json::from_str(&fs::read_to_string(&out).unwrap()).unwrap();
    let report: Value = serde_json::from_str(
        &fs::read_to_string(state_dir.join("kernel_sentinel_report_current.json")).unwrap(),
    )
    .unwrap();
    let verdict: Value =
        serde_json::from_str(&fs::read_to_string(state_dir.join("kernel_sentinel_verdict.json")).unwrap()).unwrap();
    let health: Value = serde_json::from_str(
        &fs::read_to_string(state_dir.join("kernel_sentinel_health_current.json")).unwrap(),
    )
    .unwrap();
    let trend: Value = serde_json::from_str(
        &fs::read_to_string(state_dir.join("sentinel_trend_report_current.json")).unwrap(),
    )
    .unwrap();
    let readiness: Value = serde_json::from_str(
        &fs::read_to_string(state_dir.join("rsi_readiness_summary_current.json")).unwrap(),
    )
    .unwrap();
    let issues_body = fs::read_to_string(state_dir.join("issues.jsonl")).unwrap();
    let feedback_body = fs::read_to_string(state_dir.join("feedback_inbox.jsonl")).unwrap();
    let suggestions_body = fs::read_to_string(state_dir.join("suggestions.jsonl")).unwrap();
    let automation_body = fs::read_to_string(state_dir.join("automation_candidates.jsonl")).unwrap();
    let daily_report = fs::read_to_string(state_dir.join("daily_report.md")).unwrap();

    assert_eq!(artifact["type"], "kernel_sentinel_auto_run");
    assert_eq!(artifact["verdict"]["verdict"], verdict["verdict"]);
    assert_eq!(report["verdict"]["verdict"], verdict["verdict"]);
    assert_eq!(verdict["verdict"], "allow");
    assert_eq!(health["quality"]["finding_count"], verdict["finding_count"]);
    assert_eq!(
        health["issue_synthesis"]["issue_draft_count"],
        report["issue_synthesis"]["issue_draft_count"]
    );
    assert_eq!(
        health["maintenance_synthesis"]["suggestion_count"],
        report["maintenance_synthesis"]["suggestion_count"]
    );
    assert_eq!(
        health["maintenance_synthesis"]["automation_candidate_count"],
        report["maintenance_synthesis"]["automation_candidate_count"]
    );
    assert_eq!(
        health["trend"]["regression_count"],
        artifact["self_study_outputs"]["regression_count"]
    );
    assert_eq!(
        health["trend"]["improvement_count"],
        artifact["self_study_outputs"]["improvement_count"]
    );
    assert_eq!(health["trend"]["delta"]["baseline"], trend["delta"]["baseline"]);
    assert_eq!(
        artifact["self_study_outputs"]["rsi_readiness"]["operator_summary"]["status"],
        readiness["operator_summary"]["status"]
    );
    let issue_draft_count = report["issue_synthesis"]["issue_draft_count"]
        .as_u64()
        .unwrap_or(0);
    if issue_draft_count > 0 {
        assert!(issues_body.contains("kernel_sentinel_issue_draft"));
    } else {
        assert!(issues_body.trim().is_empty());
    }
    assert!(feedback_body.contains("kernel_sentinel_feedback_item"));
    let suggestion_count = report["maintenance_synthesis"]["suggestion_count"]
        .as_u64()
        .unwrap_or(0);
    if suggestion_count > 0 {
        assert!(suggestions_body.contains("kernel_sentinel_maintenance_suggestion"));
    } else {
        assert!(suggestions_body.trim().is_empty());
    }
    let automation_candidate_count = report["maintenance_synthesis"]["automation_candidate_count"]
        .as_u64()
        .unwrap_or(0);
    if automation_candidate_count > 0 {
        assert!(automation_body.contains("kernel_sentinel_automation_candidate"));
    } else {
        assert!(automation_body.trim().is_empty());
    }
    assert!(daily_report.contains("Kernel Sentinel Daily Self-Study Report"));
    assert!(daily_report.contains("Top System Holes"));
}

#[test]
fn collector_then_auto_run_outputs_stay_consistent() {
    let root = std::env::temp_dir().join(format!(
        "kernel-sentinel-collector-auto-e2e-{}",
        crate::deterministic_receipt_hash(&json!({
            "test": "collector-auto-e2e",
            "nonce": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        }))
    ));
    write_collector_backed_required_inputs(&root);
    write_fresh_scheduler_state(&root);

    let collector_out = root.join("collector.json");
    let collect_args = vec![format!("--collector-artifact={}", collector_out.display())];
    let collect_exit = super::super::collector::run_collect(&root, &collect_args);
    assert_eq!(collect_exit, 0);

    let collector_report: Value =
        serde_json::from_str(&fs::read_to_string(&collector_out).unwrap()).unwrap();
    assert_eq!(collector_report["type"], "kernel_sentinel_collector_run");
    assert_eq!(collector_report["coverage"]["required_observation_ready"], true);
    assert_eq!(collector_report["coverage"]["missing_required_source_count"], 0);
    assert_eq!(collector_report["coverage"]["present_required_source_count"], 16);
    assert_eq!(collector_report["malformed_record_count"], 0);
    assert!(collector_report["records_written"].as_u64().unwrap_or(0) >= 16);

    let state_dir = root.join("local/state/kernel_sentinel");
    let repeated_finding = KernelSentinelFinding {
        schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        id: "ks-collector-auto-e2e".to_string(),
        severity: KernelSentinelSeverity::High,
        category: KernelSentinelFindingCategory::GatewayIsolation,
        fingerprint: "gateway_isolation:collector:auto:e2e".to_string(),
        evidence: vec![
            "gateway://collector-auto-e2e;session=collector;surface=gateway;receipt_type=quarantine;recovery_reason=route_around".to_string(),
        ],
        summary: "collector-fed gateway isolation failure repeated across the same session".to_string(),
        recommended_action: "keep quarantine and recovery behavior receipted and replayable".to_string(),
        status: "open".to_string(),
    };
    fs::write(
        state_dir.join("findings.jsonl"),
        format!(
            "{}\n{}\n",
            serde_json::to_string(&repeated_finding).unwrap(),
            serde_json::to_string(&repeated_finding).unwrap()
        ),
    )
    .unwrap();

    let out = root.join("collector-auto.json");
    let auto_args = vec![
        "--strict=1".to_string(),
        "--cadence=maintenance".to_string(),
        format!("--auto-artifact={}", out.display()),
    ];
    let auto_exit = run_auto(&root, &auto_args);
    assert_eq!(auto_exit, 0);

    let artifact: Value = serde_json::from_str(&fs::read_to_string(&out).unwrap()).unwrap();
    let report: Value = serde_json::from_str(
        &fs::read_to_string(state_dir.join("kernel_sentinel_report_current.json")).unwrap(),
    )
    .unwrap();
    let issues_body = fs::read_to_string(state_dir.join("issues.jsonl")).unwrap();
    let feedback_body = fs::read_to_string(state_dir.join("feedback_inbox.jsonl")).unwrap();

    assert_eq!(artifact["type"], "kernel_sentinel_auto_run");
    assert_eq!(artifact["verdict"]["verdict"], "allow");
    assert_eq!(artifact["operator_summary"]["present_required_source_count"], 14);
    assert_eq!(report["issue_synthesis"]["issue_draft_count"], 1);
    assert_eq!(report["issue_synthesis"]["active_issue_window_count"], 1);
    assert!(issues_body.contains("kernel_sentinel_issue_draft"));
    assert!(issues_body.contains("gateway_isolation:collector:auto:e2e"));
    assert!(feedback_body.contains("kernel_sentinel_feedback_item"));
}
