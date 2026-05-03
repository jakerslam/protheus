// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn stream_anti_entropy_posture(
    issues: &[Value],
    problem_reliability: &Value,
    trend_report: &Value,
) -> Value {
    let occurrence_total = issues
        .iter()
        .map(|row| row["occurrence_count"].as_u64().unwrap_or(1))
        .sum::<u64>();
    let pattern_count = issue_patterns(issues).len() as u64;
    let owner_count = issue_owners(issues).len() as u64;
    let architecture = &problem_reliability["architecture_pattern_detection"];
    let architecture_pattern_count = architecture["pattern_count"].as_u64().unwrap_or(0);
    let local_ticketing_gate = architecture["local_ticketing_gate"]
        .as_str()
        .unwrap_or("unknown");
    let ticketing_paused = local_ticketing_gate == "pause_for_structural_diagnosis";
    let entropy_score = entropy_score(
        occurrence_total,
        pattern_count,
        owner_count,
        architecture_pattern_count,
        ticketing_paused,
    );
    let posture = posture_for_score(entropy_score);
    let trend_tracking = anti_entropy_trend_tracking(trend_report);
    let ownership_summary = entropy_owner_summaries(issues);
    let drivers = entropy_drivers(
        occurrence_total,
        owner_count,
        architecture_pattern_count,
        ticketing_paused,
    );
    let promotion_review = promotion_review_gate(
        entropy_score,
        architecture_pattern_count,
        ticketing_paused,
        &trend_tracking,
    );
    let next_actions =
        anti_entropy_next_actions(entropy_score, ticketing_paused, architecture_pattern_count);
    let operator_digest = operator_digest(posture, entropy_score, &drivers, &next_actions);
    json!({
        "type": "kernel_sentinel_anti_entropy_posture",
        "schema_version": 1,
        "mission": "anti_entropy_first",
        "trajectory_guidance_enabled": false,
        "posture": posture,
        "entropy_score": entropy_score,
        "trend_tracking": trend_tracking,
        "ownership_summary": ownership_summary,
        "promotion_review": promotion_review,
        "operator_digest": operator_digest,
        "inputs": {
            "issue_count": issues.len(),
            "occurrence_total": occurrence_total,
            "problem_pattern_count": pattern_count,
            "owner_count": owner_count,
            "architecture_pattern_count": architecture_pattern_count,
            "local_ticketing_gate": local_ticketing_gate
        },
        "drivers": entropy_drivers(occurrence_total, owner_count, architecture_pattern_count, ticketing_paused),
        "operating_bounds": {
            "optimize_trajectory": false,
            "propose_strategy": false,
            "mutate_todo": false,
            "file_issue": false,
            "apply_patch": false,
            "allowed_focus": [
                "recurring_failure_reduction",
                "source_of_truth_drift",
                "observability_noise_reduction",
                "evidence_absorption_backlog",
                "architecture_boundary_erosion"
            ],
            "deferred_focus": [
                "long_horizon_strategy",
                "autonomous_roadmap_changes",
                "feature_expansion"
            ]
        },
        "drivers": drivers,
        "next_actions": next_actions
    })
}

fn entropy_score(
    occurrence_total: u64,
    pattern_count: u64,
    owner_count: u64,
    architecture_pattern_count: u64,
    ticketing_paused: bool,
) -> u64 {
    let mut score = occurrence_total.min(45);
    score += pattern_count.saturating_mul(6).min(18);
    score += owner_count.saturating_sub(1).saturating_mul(7).min(14);
    score += architecture_pattern_count.saturating_mul(8).min(24);
    if ticketing_paused {
        score += 16;
    }
    score.min(100)
}

fn posture_for_score(score: u64) -> &'static str {
    match score {
        70..=100 => "stabilize_before_expansion",
        40..=69 => "reduce_recurring_entropy",
        1..=39 => "monitor_for_entropy",
        _ => "quiet",
    }
}

fn entropy_drivers(
    occurrence_total: u64,
    owner_count: u64,
    architecture_pattern_count: u64,
    ticketing_paused: bool,
) -> Vec<Value> {
    let mut drivers = Vec::new();
    if occurrence_total >= 20 {
        drivers.push(json!({
            "driver": "recurring_failure_load",
            "signal": occurrence_total,
            "interpretation": "failures are recurring enough to prefer reduction over expansion"
        }));
    }
    if owner_count >= 2 {
        drivers.push(json!({
            "driver": "cross_owner_drift",
            "signal": owner_count,
            "interpretation": "symptoms span owners, so local fixes may hide a boundary problem"
        }));
    }
    if architecture_pattern_count > 0 {
        drivers.push(json!({
            "driver": "structural_pattern_pressure",
            "signal": architecture_pattern_count,
            "interpretation": "Sentinel sees architecture-level patterns above local symptom level"
        }));
    }
    if ticketing_paused {
        drivers.push(json!({
            "driver": "local_ticketing_pause",
            "signal": "pause_for_structural_diagnosis",
            "interpretation": "new local tickets should wait until structural root cause is checked"
        }));
    }
    drivers
}

fn anti_entropy_trend_tracking(trend_report: &Value) -> Value {
    let history_run_count = trend_report["history_run_count"].as_u64().unwrap_or(0);
    let delta = &trend_report["delta"];
    let regressions = delta["regressions"].as_array().map(Vec::len).unwrap_or(0);
    let improvements = delta["improvements"].as_array().map(Vec::len).unwrap_or(0);
    let baseline = delta["baseline"].as_str().unwrap_or("unavailable");
    let state = if history_run_count == 0 {
        "history_unavailable"
    } else if history_run_count < 3 {
        "history_warming_up"
    } else if regressions > 0 {
        "regressing_entropy"
    } else if improvements > 0 {
        "improving_entropy"
    } else {
        "stable_entropy"
    };
    json!({
        "state": state,
        "history_run_count": history_run_count,
        "baseline": baseline,
        "regression_count": regressions,
        "improvement_count": improvements,
        "interpretation": trend_interpretation(state)
    })
}

fn trend_interpretation(state: &str) -> &'static str {
    match state {
        "history_unavailable" => {
            "current run is useful, but historical entropy direction is not yet available"
        }
        "history_warming_up" => {
            "collect at least three Sentinel trend runs before treating direction as reliable"
        }
        "regressing_entropy" => "current entropy appears worse than the previous tracked run",
        "improving_entropy" => {
            "entropy appears to be improving, so avoid reopening already-improving symptoms"
        }
        _ => "entropy appears stable across tracked runs",
    }
}

fn entropy_owner_summaries(issues: &[Value]) -> Vec<Value> {
    let mut owners = BTreeMap::<String, (u64, u64)>::new();
    for issue in issues {
        let owner = super::report_output::stream_owner_guess(issue);
        let owner = if owner.is_empty() { "unknown" } else { &owner };
        let occurrences = issue["occurrence_count"].as_u64().unwrap_or(1);
        let entry = owners.entry(owner.to_string()).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += occurrences;
    }
    owners
        .into_iter()
        .map(|(owner, (finding_count, occurrence_total))| {
            json!({
                "owner": owner,
                "finding_count": finding_count,
                "occurrence_total": occurrence_total,
                "routing": owner_routing(&owner)
            })
        })
        .collect()
}

fn owner_routing(owner: &str) -> &'static str {
    match owner {
        "kernel" => "Kernel authority/stability review",
        "observability" => "Observability evidence-quality review",
        "validation" => "Validation gate/eval review",
        "gateway" => "Gateway membrane review",
        "shell" => "Shell projection review only after explicit Shell lane approval",
        "governance" => "Governance/debt-policy review",
        _ => "needs owner triage before local ticketing",
    }
}

fn promotion_review_gate(
    entropy_score: u64,
    architecture_pattern_count: u64,
    ticketing_paused: bool,
    trend_tracking: &Value,
) -> Value {
    let trend_regressing = trend_tracking["state"].as_str() == Some("regressing_entropy");
    let required = ticketing_paused
        || architecture_pattern_count > 0
        || entropy_score >= 70
        || trend_regressing;
    json!({
        "required_before_todo_or_issue": required,
        "mode": if required { "human_review_required" } else { "draft_lane_allowed" },
        "blocks": if required { json!(["todo_candidate", "github_issue_candidate"]) } else { json!([]) },
        "reason": promotion_review_reason(required, ticketing_paused, architecture_pattern_count, trend_regressing),
        "safe_to_auto_file_issue": false,
        "safe_to_mutate_todo": false
    })
}

fn promotion_review_reason(
    required: bool,
    ticketing_paused: bool,
    architecture_pattern_count: u64,
    trend_regressing: bool,
) -> &'static str {
    if ticketing_paused {
        "local ticketing is paused until structural diagnosis completes"
    } else if architecture_pattern_count > 0 {
        "architecture-level patterns are active, so local symptom promotion needs review"
    } else if trend_regressing {
        "historical trend indicates regression, so promotion needs anti-entropy review"
    } else if required {
        "entropy score is high enough to require review before promotion"
    } else {
        "no structural anti-entropy blocker is active"
    }
}

fn operator_digest(
    posture: &str,
    entropy_score: u64,
    drivers: &[Value],
    next_actions: &[Value],
) -> Value {
    let top_driver = drivers
        .first()
        .and_then(|row| row["driver"].as_str())
        .unwrap_or("none");
    let next_action = next_actions
        .first()
        .and_then(|row| row["action"].as_str())
        .unwrap_or("continue_monitoring");
    json!({
        "type": "kernel_sentinel_operator_anti_entropy_digest",
        "posture": posture,
        "entropy_score": entropy_score,
        "top_driver": top_driver,
        "next_stabilizing_action": next_action,
        "summary": format!("posture={posture}; entropy_score={entropy_score}; top_driver={top_driver}; next={next_action}")
    })
}

fn anti_entropy_next_actions(
    entropy_score: u64,
    ticketing_paused: bool,
    architecture_pattern_count: u64,
) -> Vec<Value> {
    let mut actions = Vec::new();
    if ticketing_paused {
        actions.push(json!({
            "action": "review_structural_patterns_before_ticketing",
            "reason": "local ticketing is paused by Sentinel reliability analysis",
            "owner": "human_or_codex_review"
        }));
    }
    if architecture_pattern_count > 0 {
        actions.push(json!({
            "action": "run_falsification_probe_for_top_architecture_pattern",
            "reason": "architecture-level patterns require confirmation before promotion",
            "owner": "observability"
        }));
    }
    if entropy_score >= 70 {
        actions.push(json!({
            "action": "prefer_deletion_boundary_repair_or_noise_reduction_over_feature_work",
            "reason": "entropy pressure is high enough that expansion may amplify drift",
            "owner": "governance_review"
        }));
    }
    if actions.is_empty() {
        actions.push(json!({
            "action": "continue_monitoring",
            "reason": "entropy pressure is currently below intervention threshold",
            "owner": "sentinel"
        }));
    }
    actions
}

fn issue_patterns(issues: &[Value]) -> BTreeSet<String> {
    issues
        .iter()
        .map(super::report_output::stream_pattern_id)
        .collect()
}

fn issue_owners(issues: &[Value]) -> BTreeSet<String> {
    issues
        .iter()
        .map(super::report_output::stream_owner_guess)
        .filter(|owner| !owner.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn issue(fingerprint: &str, occurrences: u64) -> Value {
        json!({
            "title": "Synthetic user harness failure: empty_assistant_response",
            "severity": "high",
            "category": "runtime_correctness",
            "fingerprint": fingerprint,
            "occurrence_count": occurrences,
            "summary": "empty_assistant_response"
        })
    }

    #[test]
    fn high_entropy_pauses_expansion_and_trajectory_guidance() {
        let reliability = json!({
            "architecture_pattern_detection": {
                "pattern_count": 2,
                "local_ticketing_gate": "pause_for_structural_diagnosis"
            }
        });
        let posture = stream_anti_entropy_posture(
            &[
                issue("synthetic_user_chat_harness:failures", 25),
                issue("eval_agent_feedback:agent.attention", 30),
            ],
            &reliability,
            &json!({
                "history_run_count": 3,
                "delta": {
                    "baseline": "previous_run",
                    "regressions": [{"metric": "finding_count"}],
                    "improvements": []
                }
            }),
        );
        assert_eq!(posture["mission"].as_str(), Some("anti_entropy_first"));
        assert_eq!(
            posture["trajectory_guidance_enabled"].as_bool(),
            Some(false)
        );
        assert_eq!(
            posture["posture"].as_str(),
            Some("stabilize_before_expansion")
        );
        assert_eq!(
            posture["inputs"]["local_ticketing_gate"].as_str(),
            Some("pause_for_structural_diagnosis")
        );
        assert!(!posture["drivers"].as_array().unwrap().is_empty());
        assert_eq!(
            posture["trend_tracking"]["state"].as_str(),
            Some("regressing_entropy")
        );
        assert_eq!(
            posture["promotion_review"]["required_before_todo_or_issue"].as_bool(),
            Some(true)
        );
        assert_eq!(
            posture["operator_digest"]["type"].as_str(),
            Some("kernel_sentinel_operator_anti_entropy_digest")
        );
    }

    #[test]
    fn quiet_state_monitors_without_promotion_authority() {
        let reliability = json!({
            "architecture_pattern_detection": {
                "pattern_count": 0,
                "local_ticketing_gate": "local_ticketing_allowed"
            }
        });
        let posture = stream_anti_entropy_posture(&[], &reliability, &json!({}));
        assert_eq!(posture["posture"].as_str(), Some("quiet"));
        assert_eq!(
            posture["operating_bounds"]["mutate_todo"].as_bool(),
            Some(false)
        );
        assert_eq!(
            posture["next_actions"][0]["action"].as_str(),
            Some("continue_monitoring")
        );
        assert_eq!(
            posture["trend_tracking"]["state"].as_str(),
            Some("history_unavailable")
        );
    }

    #[test]
    fn improving_history_allows_draft_lane_without_authority() {
        let reliability = json!({
            "architecture_pattern_detection": {
                "pattern_count": 0,
                "local_ticketing_gate": "local_ticketing_allowed"
            }
        });
        let posture = stream_anti_entropy_posture(
            &[issue("validation:evaluation_noise", 2)],
            &reliability,
            &json!({
                "history_run_count": 4,
                "delta": {
                    "baseline": "previous_run",
                    "regressions": [],
                    "improvements": [{"metric": "finding_count"}]
                }
            }),
        );
        assert_eq!(
            posture["trend_tracking"]["state"].as_str(),
            Some("improving_entropy")
        );
        assert_eq!(
            posture["promotion_review"]["safe_to_mutate_todo"].as_bool(),
            Some(false)
        );
        assert!(!posture["ownership_summary"].as_array().unwrap().is_empty());
    }
}
