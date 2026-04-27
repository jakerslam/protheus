// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

use super::cli_args::{bool_flag, option_path, option_usize, state_dir_from_args};
use super::{boot_watch, build_report, issue_synthesis, maintenance_synthesis, self_study, waivers, write_json};

const DEFAULT_AUTO_ARTIFACT: &str = "core/local/artifacts/kernel_sentinel_auto_run_current.json";
const DEFAULT_STALE_MINUTES: usize = 90;

fn has_option(args: &[String], name: &str) -> bool {
    args.iter()
        .any(|arg| arg == name || arg.starts_with(&format!("{name}=")))
}

fn option_string(args: &[String], name: &str, fallback: &str) -> String {
    let prefix = format!("{name}=");
    args.iter()
        .find_map(|arg| arg.strip_prefix(&prefix).map(str::to_string))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn effective_args(args: &[String]) -> Vec<String> {
    let mut out = args.to_vec();
    if !has_option(&out, "--strict") {
        out.push("--strict=1".to_string());
    }
    out
}

fn auto_artifact_path(root: &Path, args: &[String]) -> PathBuf {
    option_path(args, "--auto-artifact", root.join(DEFAULT_AUTO_ARTIFACT))
}

fn persist_run_outputs(dir: &Path, report: &Value, verdict: &Value, args: &[String]) -> Result<Value, String> {
    write_json(&dir.join("kernel_sentinel_report_current.json"), report)?;
    write_json(&dir.join("kernel_sentinel_verdict.json"), verdict)?;
    issue_synthesis::write_issue_drafts_jsonl(&dir.join("issues.jsonl"), report)?;
    maintenance_synthesis::write_maintenance_jsonl(dir, report)?;
    boot_watch::write_watch_metadata(dir, report, args)?;
    waivers::write_waiver_audit(dir, report)?;
    self_study::write_self_study_outputs(dir, report)
}

pub fn build_auto_run_artifact(
    root: &Path,
    args: &[String],
    report: &Value,
    verdict: &Value,
    exit_code: i32,
    self_study_outputs: &Value,
) -> Value {
    let dir = state_dir_from_args(root, args);
    let evidence_dir = option_path(args, "--evidence-dir", dir.join("evidence"));
    let cadence = option_string(args, "--cadence", "maintenance");
    let max_stale_minutes = option_usize(args, "--max-stale-minutes", DEFAULT_STALE_MINUTES);
    let report_path = dir.join("kernel_sentinel_report_current.json");
    let verdict_path = dir.join("kernel_sentinel_verdict.json");
    let rsi_ready = self_study_outputs["rsi_readiness"]["ready_for_autonomous_rsi"]
        .as_bool()
        .unwrap_or(false);
    let evidence_ready = self_study_outputs["rsi_readiness"]["evidence_ready"]
        .as_bool()
        .unwrap_or(false);
    let blocker_count = self_study_outputs["rsi_readiness"]["operator_summary"]["blocker_count"]
        .as_u64()
        .unwrap_or(0);
    let primary_blocker = self_study_outputs["rsi_readiness"]["operator_summary"]["primary_blocker"]
        .as_str()
        .unwrap_or("none")
        .to_string();
    let next_actions = self_study_outputs["rsi_readiness"]["next_actions"].clone();
    let issue_candidate = if rsi_ready {
        Value::Null
    } else {
        json!({
            "type": "kernel_sentinel_rsi_readiness_issue_candidate",
            "schema_version": 1,
            "generated_at": crate::now_iso(),
            "status": "candidate",
            "source": "kernel_sentinel_auto_run",
            "fingerprint": format!("kernel_sentinel_rsi_readiness:{primary_blocker}"),
            "dedupe_key": format!("kernel_sentinel_rsi_readiness:{primary_blocker}"),
            "owner": "core/layer0/kernel_sentinel",
            "route_to": "kernel_sentinel_issue_backlog",
            "labels": ["kernel-sentinel", "rsi-readiness", "release-gate"],
            "title": "Kernel Sentinel RSI readiness is blocked",
            "severity": if evidence_ready { "medium" } else { "high" },
            "primary_blocker": primary_blocker,
            "blocker_count": blocker_count,
            "source_artifacts": [
                report_path.clone(),
                verdict_path.clone(),
                dir.join("rsi_readiness_summary_current.json"),
                dir.join("top_system_holes_current.json"),
                dir.join("feedback_inbox.jsonl"),
                dir.join("trend_history.jsonl")
            ],
            "triage": {
                "state": "ready_for_issue_synthesis",
                "safe_to_auto_file_issue": true,
                "safe_to_auto_apply_patch": false,
                "requires_kernel_receipt_to_close": true
            },
            "automation_policy": {
                "mode": "proposal_only",
                "failure_priority": 1,
                "optimization_priority": 2,
                "automation_priority": 3,
                "requires_operator_or_kernel_receipt_before_apply": true
            },
            "next_actions": next_actions.clone(),
            "acceptance_criteria": [
                "Kernel Sentinel has nonzero runtime evidence",
                "Kernel Sentinel has at least three trend runs",
                "Kernel Sentinel release gate is passing",
                "Kernel Sentinel reports no active RSI readiness blockers"
            ]
        })
    };
    let mut artifact = json!({
        "ok": verdict["ok"].as_bool().unwrap_or(false),
        "type": "kernel_sentinel_auto_run",
        "automatic": true,
        "canonical_name": super::KERNEL_SENTINEL_NAME,
        "module_id": super::KERNEL_SENTINEL_MODULE_ID,
        "cadence": cadence,
        "generated_at": crate::now_iso(),
        "max_stale_minutes": max_stale_minutes,
        "stale_after_minutes": max_stale_minutes,
        "strict": verdict["strict"].as_bool().unwrap_or(false),
        "exit_code": exit_code,
        "state_dir": dir,
        "evidence_dir": evidence_dir,
        "report_path": report_path,
        "verdict_path": verdict_path,
        "output_artifacts": [
            "kernel_sentinel_report_current.json",
            "kernel_sentinel_verdict.json",
            "kernel_sentinel_auto_run_current.json",
            "issues.jsonl",
            "suggestions.jsonl",
            "automation_candidates.jsonl",
            "feedback_inbox.jsonl",
            "trend_history.jsonl",
            "sentinel_trend_report_current.json",
            "daily_report.md",
            "top_system_holes_current.json",
            "rsi_readiness_summary_current.json"
        ],
        "self_study_outputs": self_study_outputs,
        "verdict": verdict,
        "operator_summary": report["operator_summary"],
        "rsi_preparation_role": {
            "failure_detection_priority": 1,
            "optimization_priority": 2,
            "automation_priority": 3,
            "purpose": "automatic_kernel_self_understanding_feedback_loop",
            "ready_for_observation": self_study_outputs["rsi_readiness"]["ready_for_observation"].as_bool().unwrap_or(false),
            "ready_for_autonomous_rsi": rsi_ready,
            "evidence_ready": evidence_ready,
            "action_required": !rsi_ready,
            "blocker_count": blocker_count,
            "primary_blocker": primary_blocker,
            "next_actions": next_actions
        },
        "issue_candidate": issue_candidate,
        "issue_candidate_contract": {
            "candidate_schema_version": 1,
            "safe_to_auto_file_issue": true,
            "safe_to_auto_apply_patch": false,
            "kernel_receipt_required_to_close": true
        },
        "release_gate_contract": {
            "required_for_release_verdict": true,
            "active_kernel_findings_block_release": true,
            "nonzero_runtime_evidence_required": true,
            "required_sentinel_source_coverage_required": true,
            "missing_optional_shell_telemetry_is_context_only": true,
            "feedback_outputs_required": true,
            "trend_outputs_required": true,
            "control_plane_eval_can_only_advise": true
        }
    });
    artifact["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&artifact));
    artifact
}

pub fn run_auto(root: &Path, args: &[String]) -> i32 {
    let effective = effective_args(args);
    let (report, verdict, exit) = build_report(root, &effective);
    let dir = state_dir_from_args(root, &effective);
    let self_study_outputs = match persist_run_outputs(&dir, &report, &verdict, &effective) {
        Ok(outputs) => outputs,
        Err(err) => {
        eprintln!("kernel_sentinel_auto_persist_failed: {err}");
        return 1;
        }
    };
    let artifact = build_auto_run_artifact(root, &effective, &report, &verdict, exit, &self_study_outputs);
    let out = auto_artifact_path(root, &effective);
    if let Err(err) = write_json(&out, &artifact) {
        eprintln!("kernel_sentinel_auto_write_artifact_failed: {err}");
        return 1;
    }
    if !(bool_flag(&effective, "--quiet-success") && exit == 0) {
        println!(
            "{}",
            serde_json::to_string_pretty(&artifact)
                .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
        );
    }
    exit
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(artifact["self_study_outputs"]["type"], "kernel_sentinel_self_study_outputs");
        assert_eq!(artifact["self_study_outputs"]["trend_history_runs"], 1);
        assert_eq!(artifact["ok"], true);
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
        assert_eq!(artifact["self_study_outputs"]["feedback_item_count"], 1);
        assert_eq!(artifact["self_study_outputs"]["rsi_readiness"]["ready_for_observation"], true);
    }
}
