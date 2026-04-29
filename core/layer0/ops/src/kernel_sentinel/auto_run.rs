// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

use super::cli_args::{bool_flag, option_path, option_usize, state_dir_from_args};
use super::report_summary::build_health_report;
use super::self_dossier::build_infring_self_dossier;
use super::self_dossier_markdown::render_infring_self_dossier_markdown;
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

fn persist_run_outputs(
    root: &Path,
    dir: &Path,
    report: &Value,
    verdict: &Value,
    args: &[String],
) -> Result<Value, String> {
    write_json(&dir.join("kernel_sentinel_report_current.json"), report)?;
    write_json(
        &dir.join("architectural_incident_report_current.json"),
        &report["architectural_incident_report"],
    )?;
    write_json(&dir.join("kernel_sentinel_verdict.json"), verdict)?;
    let self_study_outputs = self_study::write_self_study_outputs(dir, report)?;
    let dossier = build_infring_self_dossier(root, report, verdict, &self_study_outputs)?;
    write_json(
        &root.join("local/state/system_understanding/infring_dossier.json"),
        &dossier,
    )?;
    let dossier_markdown = serde_json::from_value(dossier.clone())
        .map(|parsed| render_infring_self_dossier_markdown(&parsed))
        .map_err(|err| err.to_string())?;
    let markdown_path = root.join("docs/workspace/system_understanding/infring_dossier.md");
    if let Some(parent) = markdown_path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::write(&markdown_path, dossier_markdown).map_err(|err| err.to_string())?;
    write_json(
        &dir.join("kernel_sentinel_health_current.json"),
        &build_health_report(report, verdict, Some(&self_study_outputs)),
    )?;
    issue_synthesis::write_issue_drafts_jsonl(&dir.join("issues.jsonl"), report)?;
    maintenance_synthesis::write_maintenance_jsonl(dir, report)?;
    boot_watch::write_watch_metadata(dir, report, args)?;
    waivers::write_waiver_audit(dir, report)?;
    Ok(self_study_outputs)
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
    let system_understanding_dossier_path = root.join("local/state/system_understanding/infring_dossier.json");
    let cadence = option_string(args, "--cadence", "maintenance");
    let max_stale_minutes = option_usize(args, "--max-stale-minutes", DEFAULT_STALE_MINUTES);
    let report_path = dir.join("kernel_sentinel_report_current.json");
    let architectural_incident_report_path = dir.join("architectural_incident_report_current.json");
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
            "failure_level": "L5_self_model_failure",
            "root_frame": "system_self_model",
            "remediation_level": "self_model_repair",
            "primary_blocker": primary_blocker,
            "blocker_count": blocker_count,
            "source_artifacts": [
                report_path.clone(),
                architectural_incident_report_path.clone(),
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
        "scheduler_status": report["operator_summary"]["scheduler_status"],
        "scheduler_running": report["operator_summary"]["scheduler_running"],
        "scheduler_stale": report["operator_summary"]["scheduler_stale"],
        "state_dir": dir,
        "evidence_dir": evidence_dir,
        "report_path": report_path,
        "verdict_path": verdict_path,
        "output_artifacts": [
            "kernel_sentinel_report_current.json",
            "architectural_incident_report_current.json",
            "kernel_sentinel_verdict.json",
            "kernel_sentinel_health_current.json",
            "system_understanding/infring_dossier.json",
            "docs/workspace/system_understanding/infring_dossier.md",
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
        "system_understanding_dossier_path": system_understanding_dossier_path,
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
            "architectural_synthesis_required": true,
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
    let self_study_outputs = match persist_run_outputs(root, &dir, &report, &verdict, &effective) {
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
#[path = "auto_run_tests.rs"]
mod tests;
