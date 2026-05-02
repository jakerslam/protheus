// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::path::Path;

use super::cli_args::bool_flag;

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
        }
    });
    index["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&index));
    index
}

pub(super) fn write_full_internal_report_if_requested(
    state_dir: &Path,
    report: &Value,
    requested: bool,
) -> Result<(), String> {
    if requested {
        super::write_json(&state_dir.join("kernel_sentinel_internal_report_current.json"), report)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
