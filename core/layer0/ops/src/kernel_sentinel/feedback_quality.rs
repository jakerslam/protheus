// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const KERNEL_SENTINEL_FEEDBACK_QUALITY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelSentinelFeedbackReviewStatus {
    Accepted,
    Rejected,
    Actionable,
    Resolved,
    SymptomPatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelSentinelFeedbackReviewInput {
    pub finding_id: String,
    pub status: KernelSentinelFeedbackReviewStatus,
    pub reviewer: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub root_cause_hypothesis: String,
    #[serde(default)]
    pub concrete_next_action: String,
    #[serde(default)]
    pub resolution_ref: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KernelSentinelFeedbackQualityReview {
    pub schema_version: u32,
    pub finding_id: String,
    pub status: KernelSentinelFeedbackReviewStatus,
    pub reviewer: String,
    pub evidence_count: usize,
    pub quality_score: f64,
    pub accepted_for_learning: bool,
    pub strengthens_sentinel: bool,
    pub symptom_patch_risk: bool,
    pub required_follow_up: String,
}

fn clean(value: &str, max: usize) -> String {
    value.trim().chars().take(max).collect()
}

fn has_required_actionability(input: &KernelSentinelFeedbackReviewInput) -> bool {
    !clean(&input.root_cause_hypothesis, 400).is_empty()
        && !clean(&input.concrete_next_action, 400).is_empty()
        && !input.evidence_refs.is_empty()
}

pub fn review_kernel_sentinel_feedback_quality(
    input: KernelSentinelFeedbackReviewInput,
) -> KernelSentinelFeedbackQualityReview {
    let finding_id = clean(&input.finding_id, 160);
    let reviewer = clean(&input.reviewer, 120);
    let has_actionability = has_required_actionability(&input);
    let resolved = input.status == KernelSentinelFeedbackReviewStatus::Resolved
        && !clean(&input.resolution_ref, 300).is_empty();
    let accepted_for_learning = matches!(
        input.status,
        KernelSentinelFeedbackReviewStatus::Accepted
            | KernelSentinelFeedbackReviewStatus::Actionable
            | KernelSentinelFeedbackReviewStatus::Resolved
    ) && has_actionability;
    let symptom_patch_risk = input.status == KernelSentinelFeedbackReviewStatus::SymptomPatch
        || (!has_actionability && input.status != KernelSentinelFeedbackReviewStatus::Rejected);
    let quality_score = match input.status {
        KernelSentinelFeedbackReviewStatus::Rejected => {
            if input.evidence_refs.is_empty() {
                0.2
            } else {
                0.35
            }
        }
        KernelSentinelFeedbackReviewStatus::SymptomPatch => 0.25,
        KernelSentinelFeedbackReviewStatus::Actionable => {
            if has_actionability {
                0.75
            } else {
                0.45
            }
        }
        KernelSentinelFeedbackReviewStatus::Accepted => {
            if has_actionability {
                0.82
            } else {
                0.5
            }
        }
        KernelSentinelFeedbackReviewStatus::Resolved => {
            if resolved && has_actionability {
                0.95
            } else {
                0.6
            }
        }
    };
    let required_follow_up = if symptom_patch_risk {
        "escalate_root_cause_review"
    } else if accepted_for_learning {
        "feed_learning_ledger"
    } else {
        "retain_for_audit_only"
    };

    KernelSentinelFeedbackQualityReview {
        schema_version: KERNEL_SENTINEL_FEEDBACK_QUALITY_SCHEMA_VERSION,
        finding_id,
        status: input.status,
        reviewer,
        evidence_count: input.evidence_refs.len(),
        quality_score,
        accepted_for_learning,
        strengthens_sentinel: accepted_for_learning && !symptom_patch_risk,
        symptom_patch_risk,
        required_follow_up: required_follow_up.to_string(),
    }
}

pub fn kernel_sentinel_feedback_quality_model() -> Value {
    json!({
        "schema_version": KERNEL_SENTINEL_FEEDBACK_QUALITY_SCHEMA_VERSION,
        "purpose": "Track whether Sentinel findings were accepted, rejected, actionable, resolved, or symptom patches so feedback quality improves over time.",
        "statuses": [
            "accepted",
            "rejected",
            "actionable",
            "resolved",
            "symptom_patch"
        ],
        "required_for_learning": [
            "evidence_refs",
            "root_cause_hypothesis",
            "concrete_next_action"
        ],
        "symptom_patch_follow_up": "escalate_root_cause_review"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(status: KernelSentinelFeedbackReviewStatus) -> KernelSentinelFeedbackReviewInput {
        KernelSentinelFeedbackReviewInput {
            finding_id: "ksent.finding.demo".to_string(),
            status,
            reviewer: "codex".to_string(),
            evidence_refs: vec!["local/state/kernel_sentinel/report.json".to_string()],
            root_cause_hypothesis: "The finding maps to a boundary ownership gap.".to_string(),
            concrete_next_action: "Move the repair into the owning domain and add a replay guard.".to_string(),
            resolution_ref: "commit:demo".to_string(),
        }
    }

    #[test]
    fn actionable_feedback_is_accepted_for_learning() {
        let review = review_kernel_sentinel_feedback_quality(input(
            KernelSentinelFeedbackReviewStatus::Actionable,
        ));
        assert!(review.accepted_for_learning);
        assert!(review.strengthens_sentinel);
        assert!(!review.symptom_patch_risk);
        assert_eq!(review.required_follow_up, "feed_learning_ledger");
    }

    #[test]
    fn symptom_patch_feedback_requires_root_cause_review() {
        let review = review_kernel_sentinel_feedback_quality(input(
            KernelSentinelFeedbackReviewStatus::SymptomPatch,
        ));
        assert!(!review.accepted_for_learning);
        assert!(review.symptom_patch_risk);
        assert_eq!(review.required_follow_up, "escalate_root_cause_review");
    }

    #[test]
    fn resolved_feedback_scores_high_when_resolution_ref_exists() {
        let review = review_kernel_sentinel_feedback_quality(input(
            KernelSentinelFeedbackReviewStatus::Resolved,
        ));
        assert!(review.quality_score >= 0.95);
        assert!(review.accepted_for_learning);
    }

    #[test]
    fn accepted_feedback_without_actionability_is_not_learning_input() {
        let mut weak = input(KernelSentinelFeedbackReviewStatus::Accepted);
        weak.evidence_refs.clear();
        weak.root_cause_hypothesis.clear();
        let review = review_kernel_sentinel_feedback_quality(weak);
        assert!(!review.accepted_for_learning);
        assert!(review.symptom_patch_risk);
    }
}
