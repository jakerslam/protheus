// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    finding_lifecycle::normalize_finding_status, KernelSentinelFinding,
    KernelSentinelFindingCategory, KernelSentinelSeverity,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

pub(super) fn build_health_report(
    report: &Value,
    verdict: &Value,
    self_study_outputs: Option<&Value>,
) -> Value {
    let data_starved = report["operator_summary"]["data_starved"].as_bool().unwrap_or(true);
    let partial_evidence = report["operator_summary"]["partial_evidence"]
        .as_bool()
        .unwrap_or(true);
    let malformed_evidence = report["operator_summary"]["malformed_evidence"]
        .as_bool()
        .unwrap_or(true);
    let stale_evidence = report["operator_summary"]["stale_evidence"]
        .as_bool()
        .unwrap_or(true);
    let missing_required_source_count = report["operator_summary"]["missing_required_source_count"]
        .as_u64()
        .unwrap_or(u64::MAX);
    let scheduler_stale = report["operator_summary"]["scheduler_stale"]
        .as_bool()
        .unwrap_or(true);
    let verdict_ok = verdict["ok"].as_bool().unwrap_or(false);
    let release_gate_pass = report["release_gate"]["pass"].as_bool().unwrap_or(false);
    let issue_quality_ok = report["issue_synthesis"]["issue_quality"]["ok"]
        .as_bool()
        .unwrap_or(false);
    let mut observation_blockers = Vec::new();
    if !verdict_ok {
        observation_blockers.push("verdict_not_ok");
    }
    if data_starved {
        observation_blockers.push("data_starved");
    }
    if partial_evidence {
        observation_blockers.push("partial_evidence");
    }
    if malformed_evidence {
        observation_blockers.push("malformed_evidence");
    }
    if stale_evidence {
        observation_blockers.push("stale_evidence");
    }
    if missing_required_source_count > 0 {
        observation_blockers.push("missing_required_sources");
    }
    if scheduler_stale {
        observation_blockers.push("scheduler_stale");
    }
    let safe_for_observation_authority = observation_blockers.is_empty();

    let mut automation_blockers = observation_blockers.clone();
    if !release_gate_pass {
        automation_blockers.push("release_gate_failed");
    }
    if !issue_quality_ok {
        automation_blockers.push("issue_quality_not_ok");
    }
    let self_study_ready_for_observation = self_study_outputs
        .and_then(|outputs| outputs["rsi_readiness"]["ready_for_observation"].as_bool());
    let self_study_ready_for_autonomous_rsi = self_study_outputs
        .and_then(|outputs| outputs["rsi_readiness"]["ready_for_autonomous_rsi"].as_bool());
    let trend_history_runs = self_study_outputs
        .and_then(|outputs| outputs["trend_history_runs"].as_u64());
    let regression_count = self_study_outputs
        .and_then(|outputs| outputs["regression_count"].as_u64());
    let improvement_count = self_study_outputs
        .and_then(|outputs| outputs["improvement_count"].as_u64());
    let trend_status = self_study_outputs
        .and_then(|outputs| outputs["trend_status"].as_str())
        .unwrap_or("unavailable");
    if !self_study_ready_for_observation.unwrap_or(false) {
        automation_blockers.push("self_study_observation_not_ready");
    }
    if let Some(ready) = self_study_ready_for_autonomous_rsi {
        if !ready {
            automation_blockers.push("rsi_readiness_blocked");
        }
    } else {
        automation_blockers.push("self_study_readiness_unavailable");
    }
    let safe_for_automation_authority = automation_blockers.is_empty();

    serde_json::json!({
        "ok": report["ok"].as_bool().unwrap_or(false),
        "type": "kernel_sentinel_health_report",
        "generated_at": crate::now_iso(),
        "canonical_name": super::KERNEL_SENTINEL_NAME,
        "module_id": super::KERNEL_SENTINEL_MODULE_ID,
        "verdict": verdict["verdict"].clone(),
        "release_gate_pass": report["release_gate"]["pass"].clone(),
        "authority_safety": {
            "safe_for_observation_authority": safe_for_observation_authority,
            "safe_for_automation_authority": safe_for_automation_authority,
            "observation_blockers": observation_blockers,
            "automation_blockers": automation_blockers,
            "self_study_ready_for_observation": self_study_ready_for_observation,
            "self_study_ready_for_autonomous_rsi": self_study_ready_for_autonomous_rsi
        },
        "freshness": {
            "observation_state": report["operator_summary"]["observation_state"].clone(),
            "scheduler_status": report["operator_summary"]["scheduler_status"].clone(),
            "scheduler_running": report["operator_summary"]["scheduler_running"].clone(),
            "scheduler_stale": report["operator_summary"]["scheduler_stale"].clone(),
            "stale_evidence": report["operator_summary"]["stale_evidence"].clone(),
            "stale_record_count": report["operator_summary"]["stale_record_count"].clone(),
            "max_evidence_age_seconds": report["operator_summary"]["max_evidence_age_seconds"].clone(),
            "stale_evidence_seconds": report["operator_summary"]["stale_evidence_seconds"].clone()
        },
        "coverage": {
            "data_starved": report["operator_summary"]["data_starved"].clone(),
            "partial_evidence": report["operator_summary"]["partial_evidence"].clone(),
            "present_source_count": report["operator_summary"]["present_source_count"].clone(),
            "missing_source_count": report["operator_summary"]["missing_source_count"].clone(),
            "present_required_source_count": report["operator_summary"]["present_required_source_count"].clone(),
            "missing_required_source_count": report["operator_summary"]["missing_required_source_count"].clone(),
            "present_optional_source_count": report["operator_summary"]["present_optional_source_count"].clone(),
            "missing_optional_source_count": report["operator_summary"]["missing_optional_source_count"].clone(),
            "source_classes": {
                "required": {
                    "present_count": report["operator_summary"]["present_required_source_count"].clone(),
                    "missing_count": report["operator_summary"]["missing_required_source_count"].clone(),
                    "ready": report["operator_summary"]["missing_required_source_count"]
                        .as_u64()
                        .unwrap_or(u64::MAX)
                        == 0
                },
                "optional": {
                    "present_count": report["operator_summary"]["present_optional_source_count"].clone(),
                    "missing_count": report["operator_summary"]["missing_optional_source_count"].clone(),
                    "fully_present": report["operator_summary"]["missing_optional_source_count"]
                        .as_u64()
                        .unwrap_or(u64::MAX)
                        == 0
                }
            },
            "evidence_record_count": report["operator_summary"]["evidence_record_count"].clone(),
            "freshness_observed_record_count": report["operator_summary"]["freshness_observed_record_count"].clone()
        },
        "quality": {
            "critical_open_count": verdict["critical_open_count"].clone(),
            "finding_count": verdict["finding_count"].clone(),
            "malformed_finding_count": verdict["malformed_finding_count"].clone(),
            "release_blockers": verdict["release_blockers"].clone(),
            "status_counts": report["operator_summary"]["status_counts"].clone(),
            "severity_counts": report["operator_summary"]["severity_counts"].clone(),
            "category_counts": report["operator_summary"]["category_counts"].clone()
        },
        "issue_synthesis": {
            "issue_draft_count": report["issue_synthesis"]["issue_draft_count"].clone(),
            "active_issue_window_count": report["issue_synthesis"]["active_issue_window_count"].clone(),
            "rate_limited_cluster_count": report["issue_synthesis"]["rate_limited_cluster_count"].clone(),
            "issue_quality_ok": report["issue_synthesis"]["issue_quality"]["ok"].clone(),
            "low_quality_issue_count": report["issue_synthesis"]["issue_quality"]["low_quality_issue_count"].clone()
        },
        "maintenance_synthesis": {
            "suggestion_count": report["maintenance_synthesis"]["suggestion_count"].clone(),
            "automation_candidate_count": report["maintenance_synthesis"]["automation_candidate_count"].clone()
        },
        "trend": {
            "status": trend_status,
            "history_run_count": trend_history_runs,
            "regression_count": regression_count,
            "improvement_count": improvement_count,
            "delta": self_study_outputs
                .map(|outputs| outputs["trend_delta"].clone())
                .unwrap_or_else(|| serde_json::json!({
                    "baseline": "unavailable",
                    "regressions": [],
                    "improvements": []
                }))
        }
    })
}

pub(super) fn count_by_status(findings: &[KernelSentinelFinding]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for finding in findings {
        *counts
            .entry(normalize_finding_status(&finding.status))
            .or_insert(0) += 1;
    }
    counts
}

pub(super) fn count_by_category(findings: &[KernelSentinelFinding]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for finding in findings {
        *counts.entry(category_key(finding.category)).or_insert(0) += 1;
    }
    counts
}

pub(super) fn count_by_severity(findings: &[KernelSentinelFinding]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for finding in findings {
        *counts.entry(severity_key(finding.severity)).or_insert(0) += 1;
    }
    counts
}

pub(super) fn critical_open_count(findings: &[KernelSentinelFinding]) -> usize {
    findings
        .iter()
        .filter(|finding| {
            finding.severity == KernelSentinelSeverity::Critical
                && normalize_finding_status(&finding.status) == "open"
        })
        .count()
}

pub(super) fn release_blockers(
    critical_open_count: usize,
    malformed_count: usize,
    release_gate_pass: bool,
    scheduler_stale: bool,
) -> Vec<&'static str> {
    let mut blockers = Vec::new();
    if critical_open_count > 0 {
        blockers.push("critical_open_findings");
    }
    if malformed_count > 0 {
        blockers.push("malformed_findings");
    }
    if !release_gate_pass {
        blockers.push("release_gate_failed");
    }
    if scheduler_stale {
        blockers.push("scheduler_stale");
    }
    blockers
}

pub(super) fn count_malformed_by_source_kind(records: &[Value]) -> BTreeMap<String, usize> {
    count_json_rows(records, |record| {
        string_field(record, "source_kind")
            .or_else(|| string_field(record, "source").map(|source| format!("evidence:{source}")))
            .unwrap_or_else(|| "unknown".to_string())
    })
}

pub(super) fn count_malformed_by_source(records: &[Value]) -> BTreeMap<String, usize> {
    count_json_rows(records, |record| {
        string_field(record, "source_path")
            .or_else(|| string_field(record, "path"))
            .or_else(|| string_field(record, "source"))
            .unwrap_or_else(|| "unknown".to_string())
    })
}

fn category_key(category: KernelSentinelFindingCategory) -> String {
    serialized_key(category)
}

fn severity_key(severity: KernelSentinelSeverity) -> String {
    serialized_key(severity)
}

fn serialized_key<T>(value: T) -> String
where
    T: Serialize + std::fmt::Debug,
{
    serde_json::to_value(&value)
        .ok()
        .and_then(|json| json.as_str().map(str::to_owned))
        .unwrap_or_else(|| format!("{value:?}").to_ascii_lowercase())
}

fn count_json_rows<F>(records: &[Value], key: F) -> BTreeMap<String, usize>
where
    F: Fn(&Value) -> String,
{
    let mut counts = BTreeMap::new();
    for record in records {
        *counts.entry(key(record)).or_insert(0) += 1;
    }
    counts
}

fn string_field(record: &Value, key: &str) -> Option<String> {
    record
        .get(key)
        .and_then(Value::as_str)
        .filter(|raw| !raw.trim().is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::{build_health_report, release_blockers};
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
        let health = build_health_report(&report, &verdict, Some(&self_study));
        assert_eq!(health["type"], "kernel_sentinel_health_report");
        assert_eq!(health["freshness"]["scheduler_status"], "fresh");
        assert_eq!(health["coverage"]["present_required_source_count"], 14);
        assert_eq!(health["coverage"]["source_classes"]["required"]["present_count"], 14);
        assert_eq!(health["coverage"]["source_classes"]["required"]["missing_count"], 0);
        assert_eq!(health["coverage"]["source_classes"]["required"]["ready"], true);
        assert_eq!(health["coverage"]["source_classes"]["optional"]["present_count"], 0);
        assert_eq!(health["coverage"]["source_classes"]["optional"]["missing_count"], 1);
        assert_eq!(health["coverage"]["source_classes"]["optional"]["fully_present"], false);
        assert_eq!(health["trend"]["status"], "improving");
        assert_eq!(health["trend"]["history_run_count"], 4);
        assert_eq!(health["trend"]["regression_count"], 0);
        assert_eq!(health["trend"]["improvement_count"], 2);
        assert_eq!(health["trend"]["delta"]["baseline"], "previous_run");
        assert_eq!(health["quality"]["finding_count"], 2);
        assert_eq!(health["issue_synthesis"]["issue_draft_count"], 1);
        assert_eq!(health["maintenance_synthesis"]["automation_candidate_count"], 2);
        assert_eq!(health["authority_safety"]["safe_for_observation_authority"], true);
        assert_eq!(health["authority_safety"]["safe_for_automation_authority"], true);
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
        let health = build_health_report(&report, &verdict, None);
        assert_eq!(health["authority_safety"]["safe_for_observation_authority"], true);
        assert_eq!(health["authority_safety"]["safe_for_automation_authority"], false);
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
}
