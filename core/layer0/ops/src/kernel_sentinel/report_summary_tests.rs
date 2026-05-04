// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::report_summary::{build_health_report, release_blockers};
use serde_json::json;

#[test]
fn release_blockers_include_scheduler_stale_when_present() {
    let blockers = release_blockers(0, 0, true, true);
    assert_eq!(blockers, vec!["scheduler_stale"]);
}

#[test]
fn release_blockers_omit_scheduler_stale_when_fresh() {
    let blockers = release_blockers(0, 0, true, false);
    assert!(blockers.is_empty());
}

#[test]
fn build_health_report_surfaces_single_observability_snapshot() {
    let report = json!({
        "ok": true,
        "release_gate": { "pass": true },
        "operator_summary": {
            "observation_state": "healthy_observation",
            "scheduler_status": "fresh",
            "scheduler_running": false,
            "scheduler_stale": false,
            "stale_evidence": false,
            "stale_record_count": 0,
            "max_evidence_age_seconds": 12,
            "stale_evidence_seconds": 5400,
            "data_starved": false,
            "partial_evidence": false,
            "malformed_evidence": false,
            "present_source_count": 14,
            "missing_source_count": 1,
            "present_required_source_count": 14,
            "missing_required_source_count": 0,
            "present_optional_source_count": 0,
            "missing_optional_source_count": 1,
            "evidence_record_count": 120,
            "freshness_observed_record_count": 80,
            "status_counts": {"open": 2},
            "severity_counts": {"critical": 1},
            "category_counts": {"runtime_correctness": 2}
        },
        "issue_synthesis": {
            "issue_draft_count": 1,
            "active_issue_window_count": 1,
            "rate_limited_cluster_count": 3,
            "issue_quality": {
                "ok": true,
                "low_quality_issue_count": 0
            }
        },
        "maintenance_synthesis": {
            "suggestion_count": 4,
            "automation_candidate_count": 2
        },
        "guard_consistency": {
            "ok": false,
            "checked_count": 1,
            "contradiction_count": 1,
            "contradictions": [
                {
                    "record_id": "gateway-pass",
                    "matching_findings": [
                        {
                            "finding_id": "release-fail",
                            "severity": "high"
                        }
                    ]
                }
            ]
        }
    });
    let verdict = json!({
        "ok": true,
        "verdict": "allow",
        "critical_open_count": 0,
        "finding_count": 2,
        "malformed_finding_count": 0,
        "release_blockers": []
    });
    let self_study = json!({
        "trend_history_runs": 4,
        "trend_status": "improving",
        "regression_count": 0,
        "improvement_count": 2,
        "trend_delta": {
            "baseline": "previous_run",
            "regressions": [],
            "improvements": [
                {"metric": "finding_count", "before": 4, "after": 2},
                {"metric": "critical_open_count", "before": 1, "after": 0}
            ]
        },
        "rsi_readiness": {
            "ready_for_observation": true,
            "ready_for_autonomous_rsi": true
        }
    });
    let diagnostic_run = serde_json::json!({
        "authorized_probe_count": 2,
        "refused_probe_count": 1,
        "total_expected_confidence_gain": 0.54,
        "recurring_inconclusive_patterns": ["typed_probe_contract_gap"],
        "probe_requests": [
            {"selected_probe": "health://dashboard/healthz"},
            {"selected_probe": "golden://kernel_sentinel/authority_shape_residue"}
        ]
    });
    let health = build_health_report(&report, &verdict, Some(&self_study), Some(&diagnostic_run));
    assert_eq!(health["type"], "kernel_sentinel_health_report");
    assert_eq!(health["freshness"]["scheduler_status"], "fresh");
    assert_eq!(health["coverage"]["present_required_source_count"], 14);
    assert_eq!(
        health["coverage"]["source_classes"]["required"]["present_count"],
        14
    );
    assert_eq!(
        health["coverage"]["source_classes"]["required"]["missing_count"],
        0
    );
    assert_eq!(
        health["coverage"]["source_classes"]["required"]["ready"],
        true
    );
    assert_eq!(
        health["coverage"]["source_classes"]["optional"]["present_count"],
        0
    );
    assert_eq!(
        health["coverage"]["source_classes"]["optional"]["missing_count"],
        1
    );
    assert_eq!(
        health["coverage"]["source_classes"]["optional"]["fully_present"],
        false
    );
    assert_eq!(health["trend"]["status"], "improving");
    assert_eq!(health["trend"]["history_run_count"], 4);
    assert_eq!(health["trend"]["regression_count"], 0);
    assert_eq!(health["trend"]["improvement_count"], 2);
    assert_eq!(health["trend"]["delta"]["baseline"], "previous_run");
    assert_eq!(health["quality"]["finding_count"], 2);
    assert_eq!(health["issue_synthesis"]["issue_draft_count"], 1);
    assert_eq!(
        health["maintenance_synthesis"]["automation_candidate_count"],
        2
    );
    assert_eq!(health["guard_consistency"]["ok"], false);
    assert_eq!(health["guard_consistency"]["contradiction_count"], 1);
    assert_eq!(
        health["guard_consistency"]["contradictions"][0]["record_id"],
        "gateway-pass"
    );
    assert_eq!(
        health["guard_consistency"]["contradictions"][0]["matching_findings"][0]["finding_id"],
        "release-fail"
    );
    assert_eq!(health["diagnostic_report"]["probes_run"], 2);
    assert_eq!(health["diagnostic_report"]["probes_refused"], 1);
    assert_eq!(
        health["diagnostic_report"]["confidence_gain_expected_total"],
        0.54
    );
    assert_eq!(
        health["diagnostic_report"]["recurring_inconclusive_patterns"][0],
        "typed_probe_contract_gap"
    );
    assert_eq!(
        health["authority_safety"]["safe_for_observation_authority"],
        true
    );
    assert_eq!(
        health["authority_safety"]["safe_for_automation_authority"],
        true
    );
}

#[test]
fn build_health_report_keeps_automation_authority_false_without_self_study_readiness() {
    let report = json!({
        "ok": true,
        "release_gate": { "pass": true },
        "operator_summary": {
            "observation_state": "healthy_observation",
            "scheduler_status": "fresh",
            "scheduler_running": false,
            "scheduler_stale": false,
            "stale_evidence": false,
            "stale_record_count": 0,
            "max_evidence_age_seconds": 0,
            "stale_evidence_seconds": 5400,
            "data_starved": false,
            "partial_evidence": false,
            "malformed_evidence": false,
            "present_source_count": 14,
            "missing_source_count": 0,
            "present_required_source_count": 14,
            "missing_required_source_count": 0,
            "present_optional_source_count": 1,
            "missing_optional_source_count": 0,
            "evidence_record_count": 120,
            "freshness_observed_record_count": 80,
            "status_counts": {},
            "severity_counts": {},
            "category_counts": {}
        },
        "issue_synthesis": {
            "issue_draft_count": 0,
            "active_issue_window_count": 0,
            "rate_limited_cluster_count": 0,
            "issue_quality": {
                "ok": true,
                "low_quality_issue_count": 0
            }
        },
        "maintenance_synthesis": {
            "suggestion_count": 0,
            "automation_candidate_count": 0
        },
        "guard_consistency": {
            "ok": true,
            "checked_count": 0,
            "contradiction_count": 0,
            "contradictions": []
        }
    });
    let verdict = json!({
        "ok": true,
        "verdict": "allow",
        "critical_open_count": 0,
        "finding_count": 0,
        "malformed_finding_count": 0,
        "release_blockers": []
    });
    let health = build_health_report(&report, &verdict, None, None);
    assert_eq!(
        health["authority_safety"]["safe_for_observation_authority"],
        true
    );
    assert_eq!(
        health["authority_safety"]["safe_for_automation_authority"],
        false
    );
    assert_eq!(health["diagnostic_report"]["probes_run"], 0);
    assert_eq!(health["guard_consistency"]["contradiction_count"], 0);
    assert_eq!(health["trend"]["status"], "unavailable");
    assert_eq!(health["trend"]["delta"]["baseline"], "unavailable");
    assert_eq!(
        health["authority_safety"]["automation_blockers"][0],
        "self_study_observation_not_ready"
    );
    assert_eq!(
        health["authority_safety"]["automation_blockers"][1],
        "self_study_readiness_unavailable"
    );
}
