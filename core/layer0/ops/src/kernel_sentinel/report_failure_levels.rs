// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::collections::BTreeMap;

const ORDER: [&str; 6] = [
    "L0_local_defect",
    "L1_component_regression",
    "L2_boundary_contract_breach",
    "L3_policy_truth_failure",
    "L4_architectural_misalignment",
    "L5_self_model_failure",
];

pub(super) fn build_failure_level_summary(
    top_findings: &[Value],
    root_cause_clusters: &[Value],
    triage_findings: &[Value],
) -> Value {
    let mut counts = ORDER
        .iter()
        .map(|level| ((*level).to_string(), 0usize))
        .collect::<BTreeMap<_, _>>();
    for item in top_findings.iter().chain(root_cause_clusters).chain(triage_findings) {
        let level = level_code(item);
        *counts.entry(level).or_insert(0) += 1;
    }
    let highest = counts
        .iter()
        .filter(|(_, count)| **count > 0)
        .max_by_key(|(level, _)| level_rank(level))
        .map(|(level, _)| level.as_str())
        .unwrap_or("none");
    json!({
        "type": "kernel_sentinel_failure_level_summary",
        "policy": "operators must review the highest failure class before accepting remediation or promotion",
        "classification_order": ORDER,
        "counts": counts,
        "highest_failure_level": highest,
        "highest_failure_class": failure_class(highest),
        "highest_remediation_level": remediation_level(highest),
        "highest_review_depth": review_depth(highest),
        "requires_architecture_review": level_rank(highest) >= 4,
        "requires_policy_truth_review": level_rank(highest) >= 3,
        "requires_self_model_review": highest == "L5_self_model_failure",
    })
}

fn level_code(value: &Value) -> String {
    value["failure_level"]
        .as_str()
        .filter(|level| !level.trim().is_empty())
        .unwrap_or("L0_local_defect")
        .to_string()
}

fn level_rank(level: &str) -> usize {
    ORDER.iter().position(|candidate| *candidate == level).unwrap_or(0)
}

pub(super) fn failure_class(level: &str) -> &'static str {
    match level {
        "L1_component_regression" => "component",
        "L2_boundary_contract_breach" => "boundary",
        "L3_policy_truth_failure" => "policy_truth",
        "L4_architectural_misalignment" => "architectural",
        "L5_self_model_failure" => "self_model",
        _ => "symptom",
    }
}

pub(super) fn remediation_level(level: &str) -> &'static str {
    match level {
        "L1_component_regression" => "component_fix",
        "L2_boundary_contract_breach" => "boundary_repair",
        "L3_policy_truth_failure" => "policy_realignment",
        "L4_architectural_misalignment" => "architectural_refactor",
        "L5_self_model_failure" => "self_model_repair",
        _ => "symptom_patch",
    }
}

pub(super) fn review_depth(level: &str) -> &'static str {
    match level {
        "L1_component_regression" => "component_owner_review",
        "L2_boundary_contract_breach" => "cross_boundary_contract_review",
        "L3_policy_truth_failure" => "policy_truth_review",
        "L4_architectural_misalignment" => "architecture_review",
        "L5_self_model_failure" => "self_model_review",
        _ => "local_symptom_review",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_report_failure_summary_promotes_highest_review_depth() {
        let findings = vec![json!({
            "failure_level": "L1_component_regression",
            "summary": "boundedness regression"
        })];
        let clusters = vec![json!({
            "failure_level": "L4_architectural_misalignment",
            "summary": "shell mini OS authority shape"
        })];
        let summary = build_failure_level_summary(&findings, &clusters, &[]);
        assert_eq!(summary["highest_failure_level"], "L4_architectural_misalignment");
        assert_eq!(summary["highest_failure_class"], "architectural");
        assert_eq!(summary["highest_review_depth"], "architecture_review");
        assert_eq!(summary["requires_architecture_review"], true);
        assert_eq!(summary["counts"]["L4_architectural_misalignment"], Value::from(1));
    }
}
