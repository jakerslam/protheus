// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use super::cli_args::{bool_flag, option_usize, state_dir_from_args};
use super::report_summary::build_health_report;

const DEFAULT_REPORT_RUNTIME_MS: usize = 120_000;
const DEFAULT_REPORT_INDEX_BYTES: u64 = 1_048_576;
const REPORT_TIMEOUT_EXIT_CODE: i32 = 124;

struct StreamReportBundle {
    report: Value,
    final_report: Value,
    verdict: Value,
    exit_code: i32,
}

pub(super) fn should_write_full_internal_report(args: &[String]) -> bool {
    bool_flag(args, "--write-full-internal-report")
        || bool_flag(args, "--include-full-internal-report")
}

pub(super) fn bounded_report_index(
    report: &Value,
    state_dir: &Path,
    full_internal_report_written: bool,
) -> Value {
    let mut index = json!({
        "ok": report["ok"].clone(),
        "type": "kernel_sentinel_report",
        "artifact_kind": "bounded_report_index",
        "generated_at": crate::now_iso(),
        "canonical_name": super::KERNEL_SENTINEL_NAME,
        "state_dir": state_dir,
        "verdict": report["verdict"].clone(),
        "operator_summary": report["operator_summary"].clone(),
        "report_budget": report["report_budget"].clone(),
        "final_report": report["final_report"].clone(),
        "causal_calibration": report["causal_calibration"]["final_report_summary"].clone(),
        "issue_synthesis": {
            "issue_draft_count": report["issue_synthesis"]["issue_draft_count"].clone(),
            "active_issue_window_count": report["issue_synthesis"]["active_issue_window_count"].clone(),
            "rate_limited_cluster_count": report["issue_synthesis"]["rate_limited_cluster_count"].clone(),
            "issue_quality": report["issue_synthesis"]["issue_quality"].clone(),
        },
        "maintenance_synthesis": {
            "suggestion_count": report["maintenance_synthesis"]["suggestion_count"].clone(),
            "automation_candidate_count": report["maintenance_synthesis"]["automation_candidate_count"].clone(),
        },
        "raw_evidence": {
            "embedded": false,
            "reason": "raw evidence stays in append-only evidence streams and detail artifacts",
        },
        "internal_report": {
            "embedded": false,
            "default_written": false,
            "written": full_internal_report_written,
            "opt_in_flags": [
                "--write-full-internal-report=1",
                "--include-full-internal-report=1"
            ],
            "path": state_dir.join("kernel_sentinel_internal_report_current.json").display().to_string(),
        },
        "artifact_refs": {
            "report_index": state_dir.join("kernel_sentinel_report_current.json").display().to_string(),
            "final_report": state_dir.join("kernel_sentinel_final_report_current.json").display().to_string(),
            "internal_report_opt_in": state_dir.join("kernel_sentinel_internal_report_current.json").display().to_string(),
            "verdict": state_dir.join("kernel_sentinel_verdict.json").display().to_string(),
            "health": state_dir.join("kernel_sentinel_health_current.json").display().to_string(),
            "issues": state_dir.join("issues.jsonl").display().to_string(),
            "suggestions": state_dir.join("suggestions.jsonl").display().to_string(),
            "automation_candidates": state_dir.join("automation_candidates.jsonl").display().to_string(),
            "feedback_inbox": state_dir.join("feedback_inbox.jsonl").display().to_string(),
            "trend_report": state_dir.join("sentinel_trend_report_current.json").display().to_string(),
            "rsi_readiness": state_dir.join("rsi_readiness_summary_current.json").display().to_string(),
            "top_system_holes": state_dir.join("top_system_holes_current.json").display().to_string(),
            "daily_report": state_dir.join("daily_report.md").display().to_string(),
            "causal_hypothesis_ledger": state_dir.join("causal_hypothesis_ledger_current.jsonl").display().to_string(),
            "causal_pattern_scores": state_dir.join("causal_pattern_scores_current.json").display().to_string(),
        }
    });
    if !report["output_quarantine"].is_null() {
        index["output_quarantine"] = report["output_quarantine"].clone();
    }
    index["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&index));
    index
}

pub(super) fn run_report_command(root: &Path, command: &str, args: &[String]) -> i32 {
    if !matches!(command, "run" | "status" | "report") {
        eprintln!("kernel_sentinel_unknown_command: {command}");
        return 1;
    }
    let dir = state_dir_from_args(root, args);
    let quarantine = quarantine_oversized_default_report(&dir, args);
    let write_full_internal_report = should_write_full_internal_report(args);
    if command == "report" && !write_full_internal_report {
        if let Some(bundle) = stream_report_bundle(&dir, args, quarantine.clone()) {
            return write_stream_bundle_and_print(&dir, &bundle, args);
        }
    }
    let (mut report, verdict, exit) = match build_report_with_timeout(root, args) {
        Ok(output) => output,
        Err(diagnostic) => return write_timeout_diagnostic_and_exit(&dir, &diagnostic),
    };
    if let Some(row) = quarantine {
        report["output_quarantine"] = row;
    }
    let bounded_report = bounded_report_index(&report, &dir, write_full_internal_report);
    if matches!(command, "run" | "report") {
        if let Err(err) = write_built_outputs(
            &dir,
            &report,
            &verdict,
            args,
            &bounded_report,
            write_full_internal_report,
        ) {
            eprintln!("kernel_sentinel_write_outputs_failed: {err}");
            return 1;
        }
    }
    print_json(if command == "status" {
        &verdict
    } else {
        &bounded_report
    });
    exit
}

fn build_report_with_timeout(root: &Path, args: &[String]) -> Result<(Value, Value, i32), Value> {
    let max_runtime = option_usize(args, "--max-runtime-ms", DEFAULT_REPORT_RUNTIME_MS);
    if max_runtime == 0 {
        return Ok(super::build_report(root, args));
    }
    let root = root.to_path_buf();
    let timeout_root = root.clone();
    let args = args.to_vec();
    let timeout_args = args.clone();
    let started = Instant::now();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(super::build_report(&root, &args));
    });
    match rx.recv_timeout(Duration::from_millis(max_runtime as u64)) {
        Ok(output) => Ok(output),
        Err(_) => Err(timeout_diagnostic(
            &state_dir_from_args(timeout_root.as_path(), &timeout_args),
            max_runtime,
            started,
        )),
    }
}

fn timeout_diagnostic(dir: &Path, max_runtime: usize, started: Instant) -> Value {
    let mut artifact = json!({
        "ok": false,
        "type": "kernel_sentinel_report_diagnostic",
        "artifact_kind": "diagnostic_timeout",
        "failure_kind": "sentinel_report_timeout",
        "generated_at": crate::now_iso(),
        "elapsed_ms": started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        "max_runtime_ms": max_runtime,
        "state_dir": dir,
        "raw_evidence_embedded": false,
        "full_report_embedded": false,
        "operator_summary": {
            "status": "timeout",
            "stage": "report_generation",
            "next_action": "use the bounded report stream path from issues.jsonl/suggestions.jsonl, then inspect source streams before retrying full report generation"
        },
        "artifact_refs": {
            "diagnostic": dir.join("kernel_sentinel_report_diagnostic_current.json").display().to_string(),
            "issues": dir.join("issues.jsonl").display().to_string(),
            "suggestions": dir.join("suggestions.jsonl").display().to_string()
        }
    });
    artifact["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&artifact));
    artifact
}

fn write_timeout_diagnostic_and_exit(dir: &Path, diagnostic: &Value) -> i32 {
    let _ = super::write_json(
        &dir.join("kernel_sentinel_report_diagnostic_current.json"),
        diagnostic,
    );
    print_json(diagnostic);
    REPORT_TIMEOUT_EXIT_CODE
}

fn write_built_outputs(
    dir: &Path,
    report: &Value,
    verdict: &Value,
    args: &[String],
    bounded_report: &Value,
    write_full_internal_report: bool,
) -> Result<(), String> {
    super::write_json(
        &dir.join("kernel_sentinel_report_current.json"),
        bounded_report,
    )?;
    write_full_internal_report_if_requested(dir, report, write_full_internal_report)?;
    super::write_json(
        &dir.join("kernel_sentinel_final_report_current.json"),
        &report["final_report"],
    )?;
    super::write_json(&dir.join("kernel_sentinel_verdict.json"), verdict)?;
    super::causal_calibration::write_causal_calibration_artifacts(dir, report)?;
    let self_study_outputs = super::self_study::write_self_study_outputs(dir, report)?;
    super::write_json(
        &dir.join("kernel_sentinel_health_current.json"),
        &build_health_report(report, verdict, Some(&self_study_outputs), None),
    )?;
    super::issue_synthesis::write_issue_drafts_jsonl(&dir.join("issues.jsonl"), report, None)?;
    super::maintenance_synthesis::write_maintenance_jsonl(dir, report)?;
    super::boot_watch::write_watch_metadata(dir, report, args)?;
    super::waivers::write_waiver_audit(dir, report)
}

fn quarantine_oversized_default_report(dir: &Path, args: &[String]) -> Option<Value> {
    let budget = option_usize(
        args,
        "--report-index-byte-budget",
        DEFAULT_REPORT_INDEX_BYTES as usize,
    ) as u64;
    let path = dir.join("kernel_sentinel_report_current.json");
    let size = fs::metadata(&path).ok()?.len();
    if size <= budget {
        return None;
    }
    let archive_dir = dir.join("archive/noisy_reports");
    if let Err(err) = fs::create_dir_all(&archive_dir) {
        return Some(
            json!({"ok": false, "reason": "archive_create_failed", "path": path, "bytes": size, "error": err.to_string()}),
        );
    }
    let stamp = crate::now_iso().replace([':', '.'], "-");
    let archived_path = archive_dir.join(format!(
        "kernel_sentinel_report_current_{stamp}.oversized.json"
    ));
    match fs::rename(&path, &archived_path) {
        Ok(_) => Some(
            json!({"ok": true, "reason": "oversized_default_report_quarantined", "bytes": size, "budget_bytes": budget, "archived_path": archived_path}),
        ),
        Err(err) => Some(
            json!({"ok": false, "reason": "oversized_default_report_quarantine_failed", "path": path, "bytes": size, "budget_bytes": budget, "error": err.to_string()}),
        ),
    }
}

fn stream_report_bundle(
    dir: &Path,
    args: &[String],
    quarantine: Option<Value>,
) -> Option<StreamReportBundle> {
    let mut issues = read_jsonl_limited(&dir.join("issues.jsonl"), 200);
    let suggestions = read_jsonl_limited(&dir.join("suggestions.jsonl"), 200);
    if issues.is_empty() && suggestions.is_empty() {
        return None;
    }
    issues.sort_by_key(|row| (severity_sort(row), occurrence_sort(row)));
    let limit = option_usize(
        args,
        "--final-report-finding-limit",
        super::DEFAULT_FINAL_REPORT_FINDING_LIMIT,
    );
    let byte_budget = option_usize(
        args,
        "--final-report-byte-budget",
        super::DEFAULT_FINAL_REPORT_BYTE_BUDGET,
    );
    let finding_limit = if byte_budget <= 12_000 {
        limit.min(5)
    } else {
        limit
    };
    let suggestion_limit = if byte_budget <= 12_000 {
        limit.min(2)
    } else {
        limit
    };
    let top_findings = issues
        .iter()
        .take(finding_limit)
        .map(compact_issue)
        .collect::<Vec<_>>();
    let root_cause_clusters = stream_root_cause_clusters(&issues);
    let top_suggestions = suggestions
        .iter()
        .take(suggestion_limit)
        .map(compact_suggestion)
        .collect::<Vec<_>>();
    let critical_count = issues
        .iter()
        .filter(|row| text(row, "severity") == "critical")
        .count();
    let verdict_label = if critical_count > 0 {
        "release_fail"
    } else if issues.is_empty() {
        "allow"
    } else {
        "review_required"
    };
    let mut verdict = json!({
        "ok": critical_count == 0,
        "type": "kernel_sentinel_verdict",
        "verdict": verdict_label,
        "strict": bool_flag(args, "--strict"),
        "critical_open_count": critical_count,
        "finding_count": issues.len(),
        "release_blockers": if critical_count > 0 { json!(["critical_kernel_sentinel_issue_stream"]) } else { json!([]) },
        "source": "stream_compacted"
    });
    verdict["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&verdict));
    let mut final_report = json!({
        "ok": critical_count == 0,
        "type": "kernel_sentinel_final_report",
        "artifact_kind": "stream_compacted_final_report",
        "generated_at": crate::now_iso(),
        "top_findings": top_findings,
        "root_cause_clustering": {
            "type": "kernel_sentinel_stream_root_cause_cluster_summary",
            "cluster_count": root_cause_clusters.len(),
            "policy": "stream_symptoms_are_collapsed_by_owner_pattern_and_optional_fingerprint_family_before_operator_promotion"
        },
        "root_cause_clusters": root_cause_clusters,
        "top_suggestions": top_suggestions,
        "raw_evidence": {"embedded": false},
        "source_streams": {
            "issues": dir.join("issues.jsonl").display().to_string(),
            "suggestions": dir.join("suggestions.jsonl").display().to_string()
        }
    });
    let bytes = serde_json::to_vec(&final_report)
        .map(|row| row.len())
        .unwrap_or(usize::MAX);
    final_report["report_budget"] = json!({"byte_budget": byte_budget, "estimated_bytes": bytes, "within_budget": bytes <= byte_budget, "full_report_embedded": false});
    final_report["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&final_report));
    let report = synthetic_stream_report(
        dir,
        &issues,
        &suggestions,
        &verdict,
        &final_report,
        quarantine,
    );
    Some(StreamReportBundle {
        report,
        final_report,
        verdict,
        exit_code: if critical_count > 0 && bool_flag(args, "--strict") {
            2
        } else {
            0
        },
    })
}

fn synthetic_stream_report(
    dir: &Path,
    issues: &[Value],
    suggestions: &[Value],
    verdict: &Value,
    final_report: &Value,
    quarantine: Option<Value>,
) -> Value {
    let issue_count = issues.len();
    let suggestion_count = suggestions.len();
    let critical_open_count = verdict["critical_open_count"].as_u64().unwrap_or(0);
    let stream_findings = issues
        .iter()
        .map(stream_issue_as_finding)
        .collect::<Vec<_>>();
    let stream_causal_calibration = stream_causal_calibration(issues);
    let mut report = json!({
        "ok": verdict["ok"].clone(),
        "type": "kernel_sentinel_report",
        "artifact_kind": "stream_compacted_report",
        "canonical_name": super::KERNEL_SENTINEL_NAME,
        "state_dir": dir,
        "operator_summary": {
            "issue_count": issue_count,
            "suggestion_count": suggestion_count,
            "critical_open_count": critical_open_count,
            "source": "issues_and_suggestions_streams",
            "raw_evidence_embedded": false,
            "data_starved": false,
            "partial_evidence": false,
            "malformed_evidence": false,
            "stale_evidence": false,
            "missing_required_source_count": 0,
            "missing_optional_source_count": 0,
            "present_required_source_count": 1,
            "present_optional_source_count": 0,
            "present_source_count": 1,
            "missing_source_count": 0,
            "evidence_record_count": issue_count + suggestion_count,
            "freshness_observed_record_count": issue_count + suggestion_count,
            "stale_record_count": 0,
            "max_evidence_age_seconds": 0,
            "stale_evidence_seconds": 0,
            "scheduler_status": "stream_compacted",
            "scheduler_running": false,
            "scheduler_stale": false,
            "observation_state": "stream_compacted"
        },
        "report_budget": final_report["report_budget"].clone(),
        "final_report": final_report.clone(),
        "verdict": verdict.clone(),
        "release_gate": {
            "pass": critical_open_count == 0
        },
        "issue_synthesis": {
            "issue_draft_count": issue_count,
            "active_issue_window_count": issue_count,
            "rate_limited_cluster_count": 0,
            "issue_quality": {"ok": true, "source": "pre_synthesized_stream"}
        },
        "causal_calibration": stream_causal_calibration,
        "maintenance_synthesis": {
            "suggestion_count": suggestion_count,
            "automation_candidate_count": 0
        },
        "guard_consistency": {
            "ok": true,
            "checked_count": 0,
            "contradiction_count": 0,
            "contradictions": []
        },
        "findings": stream_findings,
        "malformed_findings": []
    });
    if let Some(row) = quarantine {
        report["output_quarantine"] = row;
    }
    report
}

fn write_stream_bundle_and_print(dir: &Path, bundle: &StreamReportBundle, args: &[String]) -> i32 {
    let bounded = bounded_report_index(&bundle.report, dir, false);
    let self_study_outputs = match super::self_study::write_self_study_outputs(dir, &bundle.report)
    {
        Ok(outputs) => outputs,
        Err(err) => {
            eprintln!("kernel_sentinel_write_stream_self_study_failed: {err}");
            return 1;
        }
    };
    let health = build_health_report(
        &bundle.report,
        &bundle.verdict,
        Some(&self_study_outputs),
        None,
    );
    if let Err(err) = super::write_json(&dir.join("kernel_sentinel_report_current.json"), &bounded)
        .and_then(|_| {
            super::write_json(
                &dir.join("kernel_sentinel_final_report_current.json"),
                &bundle.final_report,
            )
        })
        .and_then(|_| super::write_json(&dir.join("kernel_sentinel_verdict.json"), &bundle.verdict))
        .and_then(|_| {
            super::causal_calibration::write_causal_calibration_artifacts(dir, &bundle.report)
        })
        .and_then(|_| super::write_json(&dir.join("kernel_sentinel_health_current.json"), &health))
        .and_then(|_| super::boot_watch::write_watch_metadata(dir, &bundle.report, args))
    {
        eprintln!("kernel_sentinel_write_stream_report_failed: {err}");
        return 1;
    }
    print_json(&bounded);
    bundle.exit_code
}

fn read_jsonl_limited(path: &Path, limit: usize) -> Vec<Value> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return Vec::new(),
    };
    BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_str::<Value>(&line).ok())
        .take(limit)
        .collect()
}

fn compact_issue(row: &Value) -> Value {
    json!({
        "title": clip(text(row, "title"), 180),
        "severity": text(row, "severity"),
        "category": text(row, "category"),
        "fingerprint": text(row, "fingerprint"),
        "occurrence_count": row["occurrence_count"].clone(),
        "summary": clip(&first_present(row, &["summary", "actual_behavior", "observed_failure"]), 260),
        "owner_guess": stream_owner_guess(row),
        "likely_source_areas": stream_likely_source_areas(row),
        "causal_pattern": stream_pattern_id(row),
        "causal_cluster_key": stream_cluster_key(row),
        "root_cause_hypothesis": clip(&stream_root_cause_hypothesis(row), 240),
        "falsification_probe": stream_falsification_probe(row),
        "recommended_fix": clip(&stream_recommended_fix(row), 420),
        "acceptance_criteria": limited_array(&row["acceptance_criteria"], 3),
        "evidence": limited_array(&row["evidence"], 3)
    })
}

fn compact_suggestion(row: &Value) -> Value {
    json!({
        "severity": text(row, "severity"),
        "category": text(row, "category"),
        "fingerprint": text(row, "fingerprint"),
        "occurrence_count": row["occurrence_count"].clone(),
        "suggested_change": clip(text(row, "suggested_change"), 260),
        "evidence": limited_array(&row["evidence"], 2)
    })
}

fn stream_issue_as_finding(row: &Value) -> Value {
    json!({
        "schema_version": super::KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        "id": first_present(row, &["id", "fingerprint", "title"]),
        "severity": text(row, "severity"),
        "category": text(row, "category"),
        "fingerprint": text(row, "fingerprint"),
        "evidence": row.get("evidence").cloned().unwrap_or_else(|| json!([])),
        "summary": clip(&first_present(row, &["summary", "actual_behavior", "title"]), 260),
        "recommended_action": clip(&first_present(row, &["recommended_action", "recommended_fix", "suggested_change"]), 260),
        "owner_guess": stream_owner_guess(row),
        "root_cause_hypothesis": stream_root_cause_hypothesis(row),
        "root_frame": stream_root_frame(row),
        "causal_pattern": stream_pattern_id(row),
        "causal_cluster_key": stream_cluster_key(row),
        "likely_source_areas": stream_likely_source_areas(row),
        "status": "open",
        "occurrence_count": row.get("occurrence_count").cloned().unwrap_or_else(|| json!(1)),
        "acceptance_criteria": limited_array(&row["acceptance_criteria"], 3)
    })
}

fn stream_root_cause_hypothesis(row: &Value) -> String {
    let explicit = text(row, "root_cause_hypothesis").trim();
    if !explicit.is_empty() {
        return explicit.to_string();
    }
    let fingerprint = text(row, "fingerprint");
    let category = text(row, "category");
    let recovery = first_present(
        row,
        &["recovery_reason", "recommended_fix", "suggested_change"],
    );
    let evidence = first_present(row, &["actual_behavior", "summary", "title"]);
    if fingerprint.contains("synthetic_user_chat_harness")
        || evidence
            .to_ascii_lowercase()
            .contains("empty_assistant_response")
    {
        return "workflow finalization is likely dropping or failing to synthesize a visible assistant response after execution evidence is produced".to_string();
    }
    if fingerprint.contains("eval_agent_feedback") || fingerprint.contains("eval_learning_loop") {
        return "evaluation evidence is recurring faster than the current runtime correction loop can absorb, suggesting unresolved correctness debt in the observed execution family".to_string();
    }
    if category == "receipt_integrity"
        || fingerprint.contains("receipt")
        || fingerprint.contains("drift")
    {
        return "authoritative receipt generation or receipt-to-action linkage is likely incomplete for this path, allowing drift evidence to persist".to_string();
    }
    if recovery.contains("inspect_kernel_evidence") {
        return "the Kernel evidence stream is surfacing a recurring failure family whose governing invariant is not yet being repaired at the source".to_string();
    }
    if recovery.contains("restore_receipt") {
        return "the runtime appears to be preserving action evidence without restoring the matching authoritative receipt contract".to_string();
    }
    format!(
        "the recurring `{}` finding likely reflects unresolved `{}` debt that still needs a falsifiable source-level repair path",
        if fingerprint.is_empty() { "kernel_sentinel_stream_issue" } else { fingerprint },
        if category.is_empty() { "runtime_correctness" } else { category }
    )
}

fn stream_causal_calibration(issues: &[Value]) -> Value {
    let entries = issues
        .iter()
        .take(8)
        .map(|row| {
            json!({
                "type": "kernel_sentinel_causal_hypothesis_ledger_entry",
                "schema_version": 1,
                "generated_at": crate::now_iso(),
                "hypothesis_id": format!("stream_hypothesis:{}", text(row, "fingerprint")),
                "finding_fingerprint": text(row, "fingerprint"),
                "pattern": stream_pattern_id(row),
                "owner_guess": stream_owner_guess(row),
                "likely_source_areas": stream_likely_source_areas(row),
                "outcome_status": "unresolved",
                "calibrated_confidence_percent": stream_confidence_percent(row),
                "promotion_ready": false,
                "falsification_probe": stream_falsification_probe(row),
                "next_action": stream_recommended_fix(row),
            })
        })
        .collect::<Vec<_>>();

    let mut pattern_counts = std::collections::BTreeMap::<String, u64>::new();
    for row in issues {
        let key = stream_pattern_id(row);
        *pattern_counts.entry(key).or_insert(0) += row["occurrence_count"].as_u64().unwrap_or(1);
    }
    let pattern_scores = pattern_counts
        .into_iter()
        .map(|(pattern, occurrences)| {
            json!({
                "pattern": pattern,
                "occurrence_count": occurrences,
                "trust_score": (50 + occurrences.min(40)).min(95),
                "status": "observed_stream_pattern"
            })
        })
        .collect::<Vec<_>>();

    json!({
        "current_ledger_entries": entries,
        "pattern_scores": pattern_scores,
        "calibrated_hypothesis_count": issues.len().min(8),
        "promotion_ready_count": 0
    })
}

fn stream_root_cause_clusters(issues: &[Value]) -> Vec<Value> {
    let mut order = Vec::<String>::new();
    let mut clusters = std::collections::BTreeMap::<String, StreamRootCauseCluster>::new();
    for row in issues {
        let key = stream_cluster_key(row);
        if !clusters.contains_key(&key) {
            order.push(key.clone());
        }
        clusters
            .entry(key)
            .or_insert_with(|| StreamRootCauseCluster::new(row))
            .push(row);
    }
    order
        .into_iter()
        .filter_map(|key| clusters.remove(&key).map(|cluster| cluster.to_json()))
        .collect()
}

struct StreamRootCauseCluster {
    key: String,
    owner: String,
    pattern: String,
    root_cause_hypothesis: String,
    occurrence_count: u64,
    sample_fingerprints: Vec<String>,
    likely_source_areas: Vec<String>,
}

impl StreamRootCauseCluster {
    fn new(row: &Value) -> Self {
        Self {
            key: stream_cluster_key(row),
            owner: stream_owner_guess(row),
            pattern: stream_pattern_id(row),
            root_cause_hypothesis: stream_root_cause_hypothesis(row),
            occurrence_count: 0,
            sample_fingerprints: Vec::new(),
            likely_source_areas: Vec::new(),
        }
    }

    fn push(&mut self, row: &Value) {
        self.occurrence_count += row["occurrence_count"].as_u64().unwrap_or(1);
        push_unique(&mut self.sample_fingerprints, text(row, "fingerprint"), 5);
        for area in stream_source_area_strings(row) {
            push_unique(&mut self.likely_source_areas, &area, 5);
        }
    }

    fn to_json(self) -> Value {
        json!({
            "cluster_key": self.key,
            "owner_guess": self.owner,
            "causal_pattern": self.pattern,
            "root_cause_hypothesis": clip(&self.root_cause_hypothesis, 260),
            "occurrence_count": self.occurrence_count,
            "sample_fingerprints": self.sample_fingerprints,
            "likely_source_areas": self.likely_source_areas,
            "promotion_state": "cluster_ready_for_human_review"
        })
    }
}

fn stream_pattern_id(row: &Value) -> String {
    let fingerprint = text(row, "fingerprint");
    let actual = first_present(row, &["actual_behavior", "summary", "title"]).to_ascii_lowercase();
    if fingerprint.contains("synthetic_user_chat_harness")
        || actual.contains("empty_assistant_response")
    {
        "response_finalization_gap".to_string()
    } else if fingerprint.contains("eval_agent_feedback")
        || fingerprint.contains("eval_learning_loop")
    {
        "eval_feedback_recurrence_gap".to_string()
    } else if fingerprint.contains("receipt") || fingerprint.contains("drift") {
        "receipt_integrity_gap".to_string()
    } else {
        format!(
            "{}_stream_pattern",
            text(row, "category").replace('-', "_").replace(' ', "_")
        )
    }
}

fn stream_cluster_key(row: &Value) -> String {
    let pattern = stream_pattern_id(row);
    if matches!(
        pattern.as_str(),
        "response_finalization_gap" | "eval_feedback_recurrence_gap" | "receipt_integrity_gap"
    ) {
        return format!("{}|{}", stream_owner_guess(row), pattern);
    }
    format!(
        "{}|{}|{}",
        stream_owner_guess(row),
        pattern,
        stream_fingerprint_family(text(row, "fingerprint"))
    )
}

fn stream_fingerprint_family(fingerprint: &str) -> String {
    let mut parts = fingerprint.split(':').take(2).collect::<Vec<_>>();
    if parts.is_empty() || parts[0].is_empty() {
        "unknown".to_string()
    } else if parts.len() == 1 {
        normalize_key(parts.remove(0))
    } else {
        normalize_key(&parts.join(":"))
    }
}

fn stream_owner_guess(row: &Value) -> String {
    let fingerprint = text(row, "fingerprint");
    let category = text(row, "category");
    if fingerprint.contains("eval_agent_feedback") || fingerprint.contains("eval_learning_loop") {
        "observability".to_string()
    } else if fingerprint.contains("synthetic_user_chat_harness") {
        "kernel".to_string()
    } else {
        match category {
            "receipt_integrity"
            | "capability_enforcement"
            | "state_transition"
            | "security_boundary"
            | "runtime_correctness"
            | "self_maintenance_loop" => "kernel".to_string(),
            "queue_backpressure" | "retry_storm" | "boundedness" | "performance_regression" => {
                "observability".to_string()
            }
            "release_evidence" => "validation".to_string(),
            "automation_candidate" => "governance".to_string(),
            "gateway_isolation" => "gateways".to_string(),
            _ => "unknown".to_string(),
        }
    }
}

fn stream_root_frame(row: &Value) -> String {
    let explicit = text(row, "root_frame");
    if !explicit.trim().is_empty() {
        explicit.to_string()
    } else {
        match stream_pattern_id(row).as_str() {
            "response_finalization_gap" => "workflow_finalization",
            "eval_feedback_recurrence_gap" => "eval_feedback_absorption",
            "receipt_integrity_gap" => "receipt_integrity",
            _ => "kernel_sentinel_stream",
        }
        .to_string()
    }
}

fn stream_likely_source_areas(row: &Value) -> Value {
    Value::Array(
        stream_source_area_strings(row)
            .into_iter()
            .map(Value::String)
            .collect(),
    )
}

fn stream_source_area_strings(row: &Value) -> Vec<String> {
    match stream_pattern_id(row).as_str() {
        "response_finalization_gap" => vec![
            "core/layer0/ops/src/app_plane_parts".to_string(),
            "validation/evals".to_string(),
        ],
        "eval_feedback_recurrence_gap" => vec![
            "observability/sentinel".to_string(),
            "local/state/ops/eval_agent_feedback".to_string(),
        ],
        "receipt_integrity_gap" => vec![
            "core/layer0/ops/src/kernel_sentinel".to_string(),
            "local/state/ops/verity".to_string(),
        ],
        _ => vec!["core/layer0/ops/src/kernel_sentinel".to_string()],
    }
}

fn stream_confidence_percent(row: &Value) -> u64 {
    let severity_bonus = match text(row, "severity") {
        "critical" => 25,
        "high" => 18,
        "medium" => 10,
        "low" => 4,
        _ => 0,
    };
    (52 + severity_bonus + row["occurrence_count"].as_u64().unwrap_or(1).min(18)).min(95)
}

fn stream_falsification_probe(row: &Value) -> String {
    let fingerprint = text(row, "fingerprint");
    if fingerprint.contains("synthetic_user_chat_harness") {
        "rerun the synthetic user chat harness and verify that assistant-visible final responses are emitted for the affected scenario family".to_string()
    } else if fingerprint.contains("eval_agent_feedback")
        || fingerprint.contains("eval_learning_loop")
    {
        "rerun the eval feedback loop and confirm that the recurring evaluator signature drops for this fingerprint family".to_string()
    } else if fingerprint.contains("receipt") || fingerprint.contains("drift") {
        "replay the affected receipt path and verify that action evidence and authoritative receipts converge without drift".to_string()
    } else {
        format!(
            "rerun focused diagnostics for `{}` and confirm the failure signature no longer recurs",
            if fingerprint.is_empty() {
                "kernel_sentinel_stream_issue"
            } else {
                fingerprint
            }
        )
    }
}

fn text<'a>(row: &'a Value, key: &str) -> &'a str {
    row.get(key).and_then(Value::as_str).unwrap_or("")
}

fn first_present(row: &Value, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| row.get(*key).and_then(Value::as_str))
        .unwrap_or("")
        .to_string()
}

fn stream_recommended_fix(row: &Value) -> String {
    let raw = text(row, "recommended_fix");
    if !raw.trim().is_empty()
        && raw != "inspect deterministic kernel evidence and restore fail-closed behavior"
    {
        return raw.to_string();
    }
    let component = text(row, "component");
    let root = text(row, "root_frame");
    let recovery = text(row, "recovery_reason");
    let fingerprint = text(row, "fingerprint");
    let validation = row["validation_route"]
        .as_array()
        .and_then(|rows| rows.first())
        .and_then(|row| row["command"].as_str())
        .unwrap_or(
            "cargo test --manifest-path core/layer0/ops/Cargo.toml kernel_sentinel -- --nocapture",
        );
    format!(
        "Repair `{}` at `{}` by resolving `{}` for `{}`; rerun `{validation}` and keep this draft open until the evidence stream stops emitting it.",
        if component.is_empty() { "kernel_sentinel" } else { component },
        if root.is_empty() { "unknown_root_frame" } else { root },
        if recovery.is_empty() { "restore_receipt" } else { recovery },
        fingerprint
    )
}

fn limited_array(value: &Value, limit: usize) -> Value {
    Value::Array(
        value
            .as_array()
            .map(|rows| rows.iter().take(limit).cloned().collect())
            .unwrap_or_default(),
    )
}

fn clip(raw: &str, max: usize) -> String {
    if raw.chars().count() <= max {
        raw.to_string()
    } else {
        format!(
            "{}...",
            raw.chars().take(max.saturating_sub(3)).collect::<String>()
        )
    }
}

fn normalize_key(raw: &str) -> String {
    raw.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .take(8)
        .collect::<Vec<_>>()
        .join("_")
}

fn push_unique(values: &mut Vec<String>, raw: &str, limit: usize) {
    if values.len() >= limit {
        return;
    }
    let value = clip(raw, 160);
    if !value.is_empty() && !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn severity_sort(row: &Value) -> u8 {
    match text(row, "severity") {
        "critical" => 0,
        "high" => 1,
        "medium" => 2,
        "low" => 3,
        _ => 4,
    }
}

fn occurrence_sort(row: &Value) -> usize {
    usize::MAX.saturating_sub(row["occurrence_count"].as_u64().unwrap_or(0) as usize)
}

fn print_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

pub(super) fn write_full_internal_report_if_requested(
    state_dir: &Path,
    report: &Value,
    requested: bool,
) -> Result<(), String> {
    if requested {
        super::write_json(
            &state_dir.join("kernel_sentinel_internal_report_current.json"),
            report,
        )
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn bounded_report_index_excludes_internal_noise_by_default() {
        let report = json!({
            "ok": true,
            "operator_summary": {"reported_finding_count": 2},
            "report_budget": {"within_budget": true, "full_report_embedded": false},
            "final_report": {"type": "kernel_sentinel_final_report", "raw_evidence": {"embedded": false}},
            "verdict": {"verdict": "allow"},
            "issue_synthesis": {
                "issue_draft_count": 1,
                "active_issue_window_count": 1,
                "rate_limited_cluster_count": 0,
                "issue_quality": {"ready": true}
            },
            "maintenance_synthesis": {
                "suggestion_count": 1,
                "automation_candidate_count": 0
            },
            "evidence_ingestion": {"records": ["should_not_escape"]},
            "findings": [{"id": "too_large_for_default"}]
        });
        let index = bounded_report_index(&report, Path::new("/tmp/kernel-sentinel"), false);
        assert_eq!(index["type"], "kernel_sentinel_report");
        assert_eq!(index["artifact_kind"], "bounded_report_index");
        assert_eq!(index["raw_evidence"]["embedded"], false);
        assert_eq!(index["internal_report"]["embedded"], false);
        assert_eq!(index["internal_report"]["default_written"], false);
        assert!(index.get("evidence_ingestion").is_none());
        assert!(index.get("findings").is_none());
        assert!(index.get("malformed_findings").is_none());
        assert!(index.get("release_gate").is_none());
    }

    #[test]
    fn stream_bundle_refreshes_self_study_companions_and_watch_metadata() {
        let dir = std::env::temp_dir().join(format!(
            "kernel-sentinel-stream-bundle-{}",
            crate::deterministic_receipt_hash(&json!({
                "test": "stream-bundle-refresh",
                "nonce": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            }))
        ));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("issues.jsonl"),
            concat!(
                "{\"title\":\"Critical receipt drift\",\"severity\":\"critical\",\"category\":\"receipt_integrity\",\"fingerprint\":\"receipt:drift\",\"summary\":\"receipt drift persisted\",\"root_cause_hypothesis\":\"authority residue\",\"occurrence_count\":3,\"evidence\":[\"fixture://receipt\"]}\n"
            ),
        )
        .unwrap();
        fs::write(
            dir.join("suggestions.jsonl"),
            concat!(
                "{\"severity\":\"high\",\"category\":\"runtime_correctness\",\"fingerprint\":\"runtime:hardening\",\"occurrence_count\":2,\"suggested_change\":\"tighten runtime proof coverage\",\"evidence\":[\"fixture://runtime\"]}\n"
            ),
        )
        .unwrap();

        let args = vec!["--strict=1".to_string()];
        let bundle = stream_report_bundle(&dir, &args, None).expect("stream bundle");
        let exit = write_stream_bundle_and_print(&dir, &bundle, &args);
        assert_eq!(exit, 2);

        for name in [
            "kernel_sentinel_report_current.json",
            "kernel_sentinel_final_report_current.json",
            "kernel_sentinel_verdict.json",
            "kernel_sentinel_health_current.json",
            "feedback_inbox.jsonl",
            "sentinel_trend_report_current.json",
            "top_system_holes_current.json",
            "rsi_readiness_summary_current.json",
            "daily_report.md",
            "watch_freshness.json",
            "causal_hypothesis_ledger_current.jsonl",
            "causal_pattern_scores_current.json",
        ] {
            assert!(dir.join(name).exists(), "missing {name}");
        }

        let feedback_raw = fs::read_to_string(dir.join("feedback_inbox.jsonl")).unwrap();
        assert!(feedback_raw.contains("kernel_sentinel_feedback_item"));

        let watch: Value =
            serde_json::from_str(&fs::read_to_string(dir.join("watch_freshness.json")).unwrap())
                .unwrap();
        assert_eq!(watch["missing_required_artifact_count"], 0);
        assert_eq!(watch["stale_required_artifact_count"], 0);

        let trend: Value = serde_json::from_str(
            &fs::read_to_string(dir.join("sentinel_trend_report_current.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(trend["current"]["finding_count"], 1);
    }
}
