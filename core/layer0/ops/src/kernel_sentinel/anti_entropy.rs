// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::collections::BTreeSet;

pub(super) fn stream_anti_entropy_posture(issues: &[Value], problem_reliability: &Value) -> Value {
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
    json!({
        "type": "kernel_sentinel_anti_entropy_posture",
        "schema_version": 1,
        "mission": "anti_entropy_first",
        "trajectory_guidance_enabled": false,
        "posture": posture_for_score(entropy_score),
        "entropy_score": entropy_score,
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
        "next_actions": anti_entropy_next_actions(entropy_score, ticketing_paused, architecture_pattern_count)
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
    }

    #[test]
    fn quiet_state_monitors_without_promotion_authority() {
        let reliability = json!({
            "architecture_pattern_detection": {
                "pattern_count": 0,
                "local_ticketing_gate": "local_ticketing_allowed"
            }
        });
        let posture = stream_anti_entropy_posture(&[], &reliability);
        assert_eq!(posture["posture"].as_str(), Some("quiet"));
        assert_eq!(
            posture["operating_bounds"]["mutate_todo"].as_bool(),
            Some(false)
        );
        assert_eq!(
            posture["next_actions"][0]["action"].as_str(),
            Some("continue_monitoring")
        );
    }
}
