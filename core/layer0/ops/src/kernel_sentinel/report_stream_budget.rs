// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::path::Path;

pub(super) fn enforce_stream_final_report_budget(
    final_report: Value,
    byte_budget: usize,
    state_dir: &Path,
) -> Value {
    let initial_bytes = serialized_len(&final_report);
    if initial_bytes <= byte_budget {
        return with_budget(
            final_report,
            byte_budget,
            initial_bytes,
            false,
            initial_bytes,
        );
    }
    let digest = digest_only_report(&final_report, state_dir, byte_budget, initial_bytes);
    let digest_bytes = serialized_len(&digest);
    if digest_bytes <= byte_budget {
        with_budget(digest, byte_budget, digest_bytes, true, initial_bytes)
    } else {
        with_budget(
            minimal_digest_report(&final_report, state_dir, byte_budget, initial_bytes),
            byte_budget,
            0,
            true,
            initial_bytes,
        )
    }
}

fn digest_only_report(
    report: &Value,
    state_dir: &Path,
    byte_budget: usize,
    initial_bytes: usize,
) -> Value {
    json!({
        "ok": report["ok"].clone(),
        "type": "kernel_sentinel_final_report",
        "trace_id": report["trace_id"].clone(),
        "parent_span_id": report["parent_span_id"].clone(),
        "artifact_kind": "stream_digest_only_final_report",
        "generated_at": report["generated_at"].clone(),
        "anti_entropy_digest": report["anti_entropy"]["operator_digest"].clone(),
        "anti_entropy": {
            "mission": report["anti_entropy"]["mission"].clone(),
            "posture": report["anti_entropy"]["posture"].clone(),
            "entropy_score": report["anti_entropy"]["entropy_score"].clone(),
            "trend_tracking": report["anti_entropy"]["trend_tracking"].clone(),
            "promotion_review": report["anti_entropy"]["promotion_review"].clone()
        },
        "root_cause_clustering": report["root_cause_clustering"].clone(),
        "problem_finding_reliability": {
            "quality_gate": report["problem_finding_reliability"]["quality_gate"].clone(),
            "architecture_pattern_detection": report["problem_finding_reliability"]["architecture_pattern_detection"].clone()
        },
        "raw_evidence": {"embedded": false, "reason": "digest_only_mode_keeps_raw_evidence_in_source_streams"},
        "source_streams": report["source_streams"].clone(),
        "detail_refs": detail_refs(state_dir),
        "report_budget": budget(byte_budget, initial_bytes, false, true, initial_bytes)
    })
}

fn minimal_digest_report(
    report: &Value,
    state_dir: &Path,
    byte_budget: usize,
    initial_bytes: usize,
) -> Value {
    json!({
        "ok": report["ok"].clone(),
        "type": "kernel_sentinel_final_report",
        "trace_id": report["trace_id"].clone(),
        "parent_span_id": report["parent_span_id"].clone(),
        "artifact_kind": "stream_minimal_digest_final_report",
        "anti_entropy_digest": report["anti_entropy"]["operator_digest"].clone(),
        "raw_evidence": {"embedded": false},
        "detail_refs": detail_refs(state_dir),
        "report_budget": budget(byte_budget, 0, false, true, initial_bytes)
    })
}

fn with_budget(
    mut report: Value,
    byte_budget: usize,
    estimated_bytes: usize,
    digest_only: bool,
    pre_fallback_bytes: usize,
) -> Value {
    let final_bytes = if estimated_bytes == 0 {
        serialized_len(&report)
    } else {
        estimated_bytes
    };
    report["report_budget"] = budget(
        byte_budget,
        final_bytes,
        final_bytes <= byte_budget,
        digest_only,
        pre_fallback_bytes,
    );
    report
}

fn budget(
    byte_budget: usize,
    estimated_bytes: usize,
    within_budget: bool,
    digest_only: bool,
    pre_fallback_bytes: usize,
) -> Value {
    json!({
        "byte_budget": byte_budget,
        "estimated_bytes": estimated_bytes,
        "within_budget": within_budget,
        "full_report_embedded": false,
        "digest_only_mode": digest_only,
        "pre_fallback_estimated_bytes": pre_fallback_bytes,
        "fallback_reason": if digest_only { "stream_final_report_exceeded_byte_budget" } else { "within_budget" }
    })
}

fn detail_refs(state_dir: &Path) -> Value {
    json!({
        "final_report": state_dir.join("kernel_sentinel_final_report_current.json").display().to_string(),
        "issues": state_dir.join("issues.jsonl").display().to_string(),
        "suggestions": state_dir.join("suggestions.jsonl").display().to_string(),
        "trend_report": state_dir.join("sentinel_trend_report_current.json").display().to_string()
    })
}

fn serialized_len(value: &Value) -> usize {
    serde_json::to_vec(value)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bulky_report() -> Value {
        json!({
            "ok": false,
            "type": "kernel_sentinel_final_report",
            "artifact_kind": "stream_compacted_final_report",
            "generated_at": "2026-05-03T00:00:00Z",
            "top_findings": [{"summary": "x".repeat(12_000)}],
            "root_cause_clustering": {"cluster_count": 8},
            "root_cause_clusters": [{"summary": "y".repeat(12_000)}],
            "problem_finding_reliability": {
                "quality_gate": {"ok": false},
                "architecture_pattern_detection": {"pattern_count": 1}
            },
            "anti_entropy": {
                "mission": "anti_entropy_first",
                "posture": "stabilize_before_expansion",
                "entropy_score": 100,
                "trend_tracking": {"state": "stable_entropy", "history_run_count": 20},
                "promotion_review": {"required_before_todo_or_issue": true, "safe_to_mutate_todo": false},
                "operator_digest": {"summary": "posture=stabilize_before_expansion"}
            },
            "top_suggestions": [{"suggested_change": "z".repeat(12_000)}],
            "raw_evidence": {"embedded": false},
            "source_streams": {"issues": "issues.jsonl", "suggestions": "suggestions.jsonl"}
        })
    }

    #[test]
    fn oversized_stream_report_falls_back_to_digest_only() {
        let report =
            enforce_stream_final_report_budget(bulky_report(), 6_000, Path::new("/tmp/sentinel"));
        assert_eq!(report["artifact_kind"], "stream_digest_only_final_report");
        assert_eq!(report["report_budget"]["within_budget"], true);
        assert_eq!(report["report_budget"]["digest_only_mode"], true);
        assert_eq!(report["raw_evidence"]["embedded"], false);
        assert_eq!(
            report["anti_entropy"]["promotion_review"]["safe_to_mutate_todo"],
            false
        );
        assert!(report.get("top_findings").is_none());
    }

    #[test]
    fn small_stream_report_stays_compacted() {
        let mut report = bulky_report();
        report["top_findings"] = json!([]);
        report["root_cause_clusters"] = json!([]);
        report["top_suggestions"] = json!([]);
        let report = enforce_stream_final_report_budget(report, 20_000, Path::new("/tmp/sentinel"));
        assert_eq!(report["artifact_kind"], "stream_compacted_final_report");
        assert_eq!(report["report_budget"]["within_budget"], true);
        assert_eq!(report["report_budget"]["digest_only_mode"], false);
    }
}
