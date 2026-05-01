// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};

const MAX_PROMOTION_CANDIDATES: usize = 8;
const MAX_PROMOTION_TEXT: usize = 240;

pub(super) fn build_promotion_lane(
    top_findings: &[Value],
    root_cause_clusters: &[Value],
    triage_findings: &[Value],
) -> Value {
    let promotion_candidates = root_cause_clusters
        .iter()
        .take(MAX_PROMOTION_CANDIDATES)
        .enumerate()
        .map(|(index, cluster)| promotion_candidate(index, cluster, top_findings))
        .collect::<Vec<_>>();
    let triage_candidates = triage_findings
        .iter()
        .take(MAX_PROMOTION_CANDIDATES.saturating_sub(promotion_candidates.len()))
        .enumerate()
        .map(|(index, finding)| triage_candidate(index, finding))
        .collect::<Vec<_>>();
    let stale_candidate_count = triage_candidates
        .iter()
        .filter(|candidate| candidate["promotion_state"] == "stale_do_not_use")
        .count();
    let needs_triage_count = triage_candidates.len().saturating_sub(stale_candidate_count);

    json!({
        "type": "kernel_sentinel_human_review_promotion_lane",
        "mode": "draft_only",
        "human_review_required": true,
        "safe_to_mutate_todo": false,
        "safe_to_file_github_issue": false,
        "safe_to_auto_apply_patch": false,
        "approval_gate": "codex_or_human_review_required_before_todo_or_github_mutation",
        "candidate_state_counts": {
            "todo_ready": promotion_candidates.len(),
            "issue_ready": promotion_candidates.len(),
            "needs_triage": needs_triage_count,
            "stale_do_not_use": stale_candidate_count,
        },
        "promotion_candidates": promotion_candidates,
        "triage_candidates": triage_candidates,
        "policy": "sentinel_may_draft_candidates_but_must_not_mutate_todo_or_github_without_review"
    })
}

fn promotion_candidate(index: usize, cluster: &Value, top_findings: &[Value]) -> Value {
    let cluster_key = cluster["cluster_key"].as_str().unwrap_or("unknown");
    let exemplar = top_findings
        .iter()
        .find(|finding| finding["cluster"]["cluster_key"].as_str() == Some(cluster_key));
    let title = format!(
        "Sentinel: {} {}",
        compact_text(cluster["owner_guess"].as_str().unwrap_or("unknown")),
        compact_text(cluster["fingerprint_family"].as_str().unwrap_or("finding"))
    );
    json!({
        "candidate_id": format!("ksent-promotion-{index}"),
        "source_cluster_key": compact_text(cluster_key),
        "source_exemplar_id": cluster["exemplar_id"].clone(),
        "promotion_state": "human_review_required",
        "todo_state": "todo_ready",
        "issue_state": "issue_ready",
        "owner_guess": cluster["owner_guess"].clone(),
        "category": cluster["category"].clone(),
        "failure_level": cluster["failure_level"].clone(),
        "failure_class": cluster["failure_class"].clone(),
        "remediation_level": cluster["remediation_level"].clone(),
        "review_depth": cluster["review_depth"].clone(),
        "title": compact_text(&title),
        "root_cause_hypothesis": compact_text(cluster["root_cause_hypothesis"].as_str().unwrap_or("")),
        "observed_failure": exemplar
            .and_then(|finding| finding["summary"].as_str())
            .map(compact_text)
            .unwrap_or_else(|| "clustered Sentinel finding".to_string()),
        "recommended_action": compact_text(cluster["recommended_next_action"].as_str().unwrap_or("")),
        "acceptance_criteria": [
            "human_or_codex_reviewer_confirms_evidence_refs",
            "owner_and_repair_path_are_confirmed",
            "todo_or_issue_is_created_by_reviewed_promotion_not_sentinel_auto_mutation"
        ],
        "evidence_refs": compact_array(&cluster["evidence_refs"], 3),
        "occurrence_count": cluster["occurrence_count"].clone(),
    })
}

fn triage_candidate(index: usize, finding: &Value) -> Value {
    let stale = finding["actionability_state"].as_str() == Some("stale_do_not_use")
        || finding["quality"]["stale_do_not_use"].as_bool().unwrap_or(false);
    json!({
        "candidate_id": format!("ksent-triage-{index}"),
        "source_finding_id": finding["id"].clone(),
        "promotion_state": if stale { "stale_do_not_use" } else { "needs_triage" },
        "todo_state": if stale { "do_not_promote" } else { "triage_to_todo" },
        "issue_state": if stale { "do_not_file" } else { "needs_root_cause_synthesis" },
        "category": finding["category"].clone(),
        "failure_level": finding["failure_level"].clone(),
        "failure_class": finding["failure_class"].clone(),
        "remediation_level": finding["remediation_level"].clone(),
        "review_depth": finding["review_depth"].clone(),
        "summary": compact_text(finding["summary"].as_str().unwrap_or("")),
        "missing_requirements": finding["quality"]["missing_requirements"].clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn promotion_lane_preserves_failure_level_before_recommending_work() {
        let findings = vec![json!({
            "id": "ksent-1",
            "summary": "shell authority ghost",
            "cluster": {"cluster_key": "shell|runtime|authority"}
        })];
        let clusters = vec![json!({
            "cluster_key": "shell|runtime|authority",
            "exemplar_id": "ksent-1",
            "owner_guess": "shell",
            "category": "runtime_correctness",
            "failure_level": "L3_policy_truth_failure",
            "failure_class": "policy_truth",
            "remediation_level": "policy_realignment",
            "review_depth": "policy_truth_review",
            "fingerprint_family": "authority:ghost",
            "root_cause_hypothesis": "authority ghost survived syntax cleanup",
            "recommended_next_action": "restore canonical ownership before local patching",
            "evidence_refs": ["evidence://authority/ghost"],
            "occurrence_count": 2
        })];
        let lane = build_promotion_lane(&findings, &clusters, &[]);
        let candidate = &lane["promotion_candidates"][0];
        assert_eq!(candidate["failure_level"], "L3_policy_truth_failure");
        assert_eq!(candidate["failure_class"], "policy_truth");
        assert_eq!(candidate["review_depth"], "policy_truth_review");
        assert_eq!(
            candidate["recommended_action"],
            "restore canonical ownership before local patching"
        );
    }
}

fn compact_array(value: &Value, limit: usize) -> Vec<String> {
    value
        .as_array()
        .map(|rows| rows.iter().filter_map(Value::as_str).take(limit).map(compact_text).collect())
        .unwrap_or_default()
}

fn compact_text(raw: &str) -> String {
    let mut out = raw.trim().chars().take(MAX_PROMOTION_TEXT).collect::<String>();
    if raw.trim().chars().count() > MAX_PROMOTION_TEXT {
        out.push_str("...");
    }
    out
}
