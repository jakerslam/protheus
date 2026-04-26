// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use super::write_json;

const FEEDBACK_INBOX: &str = "feedback_inbox.jsonl";
const TREND_HISTORY: &str = "trend_history.jsonl";
const TREND_REPORT: &str = "sentinel_trend_report_current.json";
const DAILY_REPORT: &str = "daily_report.md";
const TOP_HOLES: &str = "top_system_holes_current.json";
const RSI_READINESS: &str = "rsi_readiness_summary_current.json";

fn string_field(row: &Value, key: &str) -> String {
    row.get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unknown")
        .to_string()
}

fn usize_at(row: &Value, path: &[&str]) -> usize {
    let mut current = row;
    for key in path {
        current = current.get(*key).unwrap_or(&Value::Null);
    }
    current.as_u64().unwrap_or(0) as usize
}

fn bool_at(row: &Value, path: &[&str], fallback: bool) -> bool {
    let mut current = row;
    for key in path {
        current = current.get(*key).unwrap_or(&Value::Null);
    }
    current.as_bool().unwrap_or(fallback)
}

fn severity_priority(severity: &str) -> u8 {
    match severity {
        "critical" => 0,
        "high" => 1,
        "medium" => 2,
        "low" => 3,
        _ => 4,
    }
}

fn todo_priority(severity: &str, category: &str) -> &'static str {
    match (severity, category) {
        ("critical", _) => "P0",
        ("high", "security_boundary" | "capability_enforcement" | "receipt_integrity") => "P0",
        ("high", _) => "P1",
        ("medium", _) => "P2",
        _ => "P3",
    }
}

fn feedback_item(finding: &Value, generated_at: &str) -> Value {
    let severity = string_field(finding, "severity");
    let category = string_field(finding, "category");
    let fingerprint = string_field(finding, "fingerprint");
    json!({
        "type": "kernel_sentinel_feedback_item",
        "source": "kernel_sentinel",
        "generated_at": generated_at,
        "status": string_field(finding, "status"),
        "fingerprint": fingerprint,
        "dedupe_key": format!("{category}:{fingerprint}"),
        "severity": severity,
        "category": category,
        "todo_priority": todo_priority(&severity, &category),
        "priority_rank": severity_priority(&severity),
        "summary": string_field(finding, "summary"),
        "recommended_action": string_field(finding, "recommended_action"),
        "evidence": finding.get("evidence").cloned().unwrap_or_else(|| json!([])),
        "preservation_policy": "preserve_until_resolved_or_waived_by_kernel_receipt"
    })
}

fn build_feedback_inbox(report: &Value, generated_at: &str) -> Vec<Value> {
    let mut by_key: BTreeMap<String, Value> = BTreeMap::new();
    for finding in report
        .get("findings")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if string_field(finding, "status") != "open" {
            continue;
        }
        let item = feedback_item(finding, generated_at);
        let key = string_field(&item, "dedupe_key");
        match by_key.get(&key) {
            Some(existing)
                if usize_at(existing, &["priority_rank"]) <= usize_at(&item, &["priority_rank"]) => {}
            _ => {
                by_key.insert(key, item);
            }
        }
    }
    by_key.into_values().collect()
}

fn trend_summary(report: &Value, generated_at: &str) -> Value {
    json!({
        "type": "kernel_sentinel_trend_summary",
        "generated_at": generated_at,
        "ok": report["ok"].as_bool().unwrap_or(false),
        "finding_count": usize_at(report, &["verdict", "finding_count"]),
        "critical_open_count": usize_at(report, &["operator_summary", "critical_open_count"]),
        "malformed_finding_count": usize_at(report, &["operator_summary", "malformed_finding_count"]),
        "release_gate_pass": bool_at(report, &["operator_summary", "release_gate_pass"], false),
        "severity_counts": report["operator_summary"]["severity_counts"].clone(),
        "category_counts": report["operator_summary"]["category_counts"].clone()
    })
}

fn read_history(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .ok()
        .map(|body| {
            body.lines()
                .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                .collect()
        })
        .unwrap_or_default()
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| err.to_string())?;
    let body = serde_json::to_string(row).map_err(|err| err.to_string())?;
    writeln!(file, "{body}").map_err(|err| err.to_string())
}

fn overwrite_jsonl(path: &Path, rows: &[Value]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let mut body = String::new();
    for row in rows {
        body.push_str(&serde_json::to_string(row).map_err(|err| err.to_string())?);
        body.push('\n');
    }
    fs::write(path, body).map_err(|err| err.to_string())
}

fn trend_delta(previous: Option<&Value>, current: &Value) -> Value {
    let Some(previous) = previous else {
        return json!({
            "baseline": "first_run",
            "regressions": [],
            "improvements": []
        });
    };
    let mut regressions = Vec::new();
    let mut improvements = Vec::new();
    for key in ["finding_count", "critical_open_count", "malformed_finding_count"] {
        let before = usize_at(previous, &[key]);
        let after = usize_at(current, &[key]);
        if after > before {
            regressions.push(json!({"metric": key, "before": before, "after": after}));
        } else if after < before {
            improvements.push(json!({"metric": key, "before": before, "after": after}));
        }
    }
    let previous_gate = bool_at(previous, &["release_gate_pass"], false);
    let current_gate = bool_at(current, &["release_gate_pass"], false);
    if previous_gate && !current_gate {
        regressions.push(json!({"metric": "release_gate_pass", "before": true, "after": false}));
    } else if !previous_gate && current_gate {
        improvements.push(json!({"metric": "release_gate_pass", "before": false, "after": true}));
    }
    json!({
        "baseline": "previous_run",
        "regressions": regressions,
        "improvements": improvements
    })
}

fn top_holes(feedback_rows: &[Value], generated_at: &str) -> Value {
    let mut rows = feedback_rows.to_vec();
    rows.sort_by_key(|row| {
        (
            usize_at(row, &["priority_rank"]),
            string_field(row, "category"),
            string_field(row, "fingerprint"),
        )
    });
    let holes = rows.into_iter().take(10).collect::<Vec<_>>();
    let issue_candidates = holes
        .iter()
        .map(|row| {
            let category = string_field(row, "category");
            let fingerprint = string_field(row, "fingerprint");
            json!({
                "type": "kernel_sentinel_issue_candidate",
                "schema_version": 1,
                "generated_at": generated_at,
                "status": "candidate",
                "source": "kernel_sentinel_feedback_inbox",
                "fingerprint": format!("kernel_sentinel:{category}:{fingerprint}"),
                "dedupe_key": format!("kernel_sentinel:{category}:{fingerprint}"),
                "owner": "core/layer0/kernel_sentinel",
                "route_to": "kernel_sentinel_issue_backlog",
                "labels": ["kernel-sentinel", "self-study", category.clone()],
                "title": string_field(row, "summary"),
                "severity": string_field(row, "severity"),
                "priority_rank": usize_at(row, &["priority_rank"]),
                "todo_priority": string_field(row, "todo_priority"),
                "category": category.clone(),
                "recommended_action": string_field(row, "recommended_action"),
                "evidence": row.get("evidence").cloned().unwrap_or_else(|| json!([])),
                "source_artifacts": [
                    "local/state/kernel_sentinel/feedback_inbox.jsonl",
                    "local/state/kernel_sentinel/top_system_holes_current.json"
                ],
                "source_feedback_dedupe_key": string_field(row, "dedupe_key"),
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
                "acceptance_criteria": [
                    "finding is resolved or explicitly waived by Kernel Sentinel policy",
                    "feedback inbox no longer contains this dedupe key",
                    "Kernel Sentinel release gate remains passing after resolution"
                ]
            })
        })
        .collect::<Vec<_>>();
    json!({
        "type": "kernel_sentinel_top_system_holes",
        "summary": {
            "hole_count": holes.len(),
            "issue_candidate_count": issue_candidates.len(),
            "source": "kernel_sentinel_feedback_inbox",
            "candidate_contract_version": 1
        },
        "issue_candidate_contract": {
            "required_fields": [
                "fingerprint",
                "dedupe_key",
                "owner",
                "route_to",
                "severity",
                "recommended_action",
                "acceptance_criteria"
            ],
            "safe_to_auto_file_issue": true,
            "safe_to_auto_apply_patch": false
        },
        "holes": holes,
        "issue_candidates": issue_candidates
    })
}

fn rsi_missing_condition_action(condition: &str) -> &'static str {
    match condition {
        "needs_nonzero_runtime_evidence" => {
            "feed Kernel Sentinel runtime evidence from local/state/kernel_sentinel/evidence/*.jsonl before promoting RSI readiness"
        }
        "needs_at_least_three_trend_runs" => {
            "allow at least three automatic Sentinel runs so trend history can identify stability or regression"
        }
        "findings_not_projected_to_feedback_inbox" => {
            "repair Sentinel feedback projection so every open finding creates a deduped feedback item"
        }
        "release_gate_not_passing" => {
            "resolve active Kernel Sentinel release-gate blockers before promoting autonomous RSI"
        }
        "active_regressions_need_operator_review" => {
            "review Sentinel trend regressions and convert stable failures into issues or waivers"
        }
        _ => "review the missing Sentinel readiness condition and add a concrete remediation action",
    }
}

fn rsi_readiness(report: &Value, history_len: usize, feedback_len: usize, trend_delta: &Value) -> Value {
    let mut missing = Vec::new();
    let evidence_record_count = usize_at(report, &["evidence_ingestion", "normalized_record_count"]);
    let evidence_ready = evidence_record_count > 0;
    if !evidence_ready {
        missing.push("needs_nonzero_runtime_evidence");
    }
    if history_len < 3 {
        missing.push("needs_at_least_three_trend_runs");
    }
    if feedback_len == 0 && usize_at(report, &["verdict", "finding_count"]) > 0 {
        missing.push("findings_not_projected_to_feedback_inbox");
    }
    if !bool_at(report, &["release_gate", "pass"], false) {
        missing.push("release_gate_not_passing");
    }
    let regression_count = trend_delta
        .get("regressions")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    if regression_count > 0 {
        missing.push("active_regressions_need_operator_review");
    }
    let next_actions: Vec<Value> = missing
        .iter()
        .map(|condition| {
            json!({
                "condition": condition,
                "action": rsi_missing_condition_action(condition),
            })
        })
        .collect();
    json!({
        "type": "kernel_sentinel_rsi_readiness_summary",
        "generated_at": crate::now_iso(),
        "ready_for_observation": true,
        "ready_for_autonomous_rsi": missing.is_empty(),
        "trend_history_runs": history_len,
        "feedback_item_count": feedback_len,
        "evidence_record_count": evidence_record_count,
        "evidence_ready": evidence_ready,
        "evidence_contract": {
            "minimum_runtime_evidence_records": 1,
            "empty_evidence_is_observation_only": true
        },
        "operator_summary": {
            "status": if missing.is_empty() { "ready" } else { "blocked" },
            "blocker_count": missing.len(),
            "primary_blocker": missing.first().copied().unwrap_or("none"),
            "evidence_record_count": evidence_record_count,
            "trend_history_runs": history_len,
            "feedback_item_count": feedback_len
        },
        "missing_conditions": missing,
        "next_actions": next_actions,
        "safety_posture": {
            "failure_priority": 1,
            "optimization_priority": 2,
            "automation_priority": 3,
            "self_modification_requires_kernel_sentinel_verdict": true
        }
    })
}

fn write_daily_report(path: &Path, report: &Value, trend: &Value, top_holes: &Value, readiness: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let mut body = String::new();
    body.push_str("# Kernel Sentinel Daily Self-Study Report\n\n");
    body.push_str(&format!("- generated_at: {}\n", crate::now_iso()));
    body.push_str(&format!("- ok: {}\n", report["ok"].as_bool().unwrap_or(false)));
    body.push_str(&format!(
        "- critical_open_count: {}\n",
        usize_at(report, &["operator_summary", "critical_open_count"])
    ));
    body.push_str(&format!(
        "- malformed_finding_count: {}\n",
        usize_at(report, &["operator_summary", "malformed_finding_count"])
    ));
    body.push_str(&format!(
        "- release_gate_pass: {}\n\n",
        bool_at(report, &["operator_summary", "release_gate_pass"], false)
    ));
    body.push_str(&format!(
        "- evidence_record_count: {}\n",
        usize_at(report, &["evidence_ingestion", "normalized_record_count"])
    ));
    body.push_str(&format!(
        "- evidence_ready_for_rsi: {}\n\n",
        usize_at(report, &["evidence_ingestion", "normalized_record_count"]) > 0
    ));
    body.push_str("## Trend\n\n");
    body.push_str(&format!(
        "- regressions: {}\n",
        trend.get("regressions").and_then(Value::as_array).map(Vec::len).unwrap_or(0)
    ));
    body.push_str(&format!(
        "- improvements: {}\n\n",
        trend.get("improvements").and_then(Value::as_array).map(Vec::len).unwrap_or(0)
    ));
    body.push_str("## RSI Readiness\n\n");
    body.push_str(&format!(
        "- ready_for_autonomous_rsi: {}\n",
        readiness["ready_for_autonomous_rsi"].as_bool().unwrap_or(false)
    ));
    body.push_str("- missing_conditions:\n");
    for condition in readiness
        .get("missing_conditions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        body.push_str(&format!("- {}\n", condition.as_str().unwrap_or("unknown")));
    }
    body.push_str("- next_actions:\n");
    for row in readiness
        .get("next_actions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        body.push_str(&format!(
            "- {}: {}\n",
            string_field(row, "condition"),
            string_field(row, "action")
        ));
    }
    body.push('\n');
    body.push_str("## Top System Holes\n\n");
    for row in top_holes
        .get("holes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        body.push_str(&format!(
            "- [{}] {} — {}\n",
            string_field(row, "todo_priority"),
            string_field(row, "category"),
            string_field(row, "summary")
        ));
    }
    fs::write(path, body).map_err(|err| err.to_string())
}

pub(super) fn write_self_study_outputs(dir: &Path, report: &Value) -> Result<Value, String> {
    let generated_at = crate::now_iso();
    let feedback_path = dir.join(FEEDBACK_INBOX);
    let history_path = dir.join(TREND_HISTORY);
    let trend_report_path = dir.join(TREND_REPORT);
    let daily_report_path = dir.join(DAILY_REPORT);
    let top_holes_path = dir.join(TOP_HOLES);
    let rsi_path = dir.join(RSI_READINESS);

    let previous_history = read_history(&history_path);
    let current_summary = trend_summary(report, &generated_at);
    let delta = trend_delta(previous_history.last(), &current_summary);
    append_jsonl(&history_path, &current_summary)?;
    let history_len = previous_history.len() + 1;

    let feedback_rows = build_feedback_inbox(report, &generated_at);
    overwrite_jsonl(&feedback_path, &feedback_rows)?;
    let top_holes = top_holes(&feedback_rows, &generated_at);
    let readiness = rsi_readiness(report, history_len, feedback_rows.len(), &delta);
    let trend_report = json!({
        "type": "kernel_sentinel_trend_report",
        "generated_at": generated_at,
        "history_path": history_path,
        "current": current_summary,
        "delta": delta,
        "history_run_count": history_len
    });

    write_json(&trend_report_path, &trend_report)?;
    write_json(&top_holes_path, &top_holes)?;
    write_json(&rsi_path, &readiness)?;
    write_daily_report(&daily_report_path, report, &trend_report["delta"], &top_holes, &readiness)?;

    let mut manifest = json!({
        "type": "kernel_sentinel_self_study_outputs",
        "feedback_inbox_path": feedback_path,
        "trend_history_path": history_path,
        "trend_report_path": trend_report_path,
        "daily_report_path": daily_report_path,
        "top_system_holes_path": top_holes_path,
        "rsi_readiness_path": rsi_path,
        "feedback_item_count": feedback_rows.len(),
        "trend_history_runs": history_len,
        "regression_count": trend_report["delta"]["regressions"].as_array().map(Vec::len).unwrap_or(0),
        "improvement_count": trend_report["delta"]["improvements"].as_array().map(Vec::len).unwrap_or(0),
        "issue_candidate_count": top_holes["summary"]["issue_candidate_count"].as_u64().unwrap_or(0),
        "rsi_operator_summary": readiness["operator_summary"].clone(),
        "rsi_next_action_count": readiness["next_actions"].as_array().map(Vec::len).unwrap_or(0),
        "rsi_readiness": readiness
    });
    manifest["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&manifest));
    Ok(manifest)
}
