// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::collections::BTreeMap;

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

fn recurring_family_fingerprint(row: &Value) -> String {
    let family = string_field(row, "feedback_family_fingerprint");
    if family == "unknown" {
        string_field(row, "fingerprint")
    } else {
        family
    }
}

fn recurring_failure_family(row: &Value) -> Value {
    let category = string_field(row, "category");
    let family_fingerprint = recurring_family_fingerprint(row);
    let family_key = format!("{category}:{family_fingerprint}");
    let evidence_count = row
        .get("evidence")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    json!({
        "type": "kernel_sentinel_recurring_failure_family",
        "family_key": family_key,
        "family_fingerprint": family_fingerprint,
        "category": category,
        "severity": string_field(row, "severity"),
        "todo_priority": string_field(row, "todo_priority"),
        "priority_rank": usize_at(row, &["priority_rank"]),
        "operator_value_tier": string_field(row, "operator_value_tier"),
        "operator_value_rank": usize_at(row, &["operator_value_rank"]),
        "recurrence_count": usize_at(row, &["recurrence_count"]),
        "recurrence_threshold": usize_at(row, &["recurrence_threshold"]),
        "issue_candidate_ready": bool_at(row, &["issue_candidate_ready"], false),
        "exemplar_fingerprint": string_field(row, "fingerprint"),
        "exemplar_summary": string_field(row, "summary"),
        "recommended_action": string_field(row, "recommended_action"),
        "failure_level": string_field(row, "failure_level"),
        "root_frame": string_field(row, "root_frame"),
        "remediation_level": string_field(row, "remediation_level"),
        "evidence_count": evidence_count,
        "source_feedback_dedupe_keys": [string_field(row, "dedupe_key")]
    })
}

fn merge_family(existing: &mut Value, row: &Value) {
    let incoming_recurrence = usize_at(row, &["recurrence_count"]);
    let existing_recurrence = usize_at(existing, &["recurrence_count"]);
    if incoming_recurrence > existing_recurrence {
        *existing = recurring_failure_family(row);
        return;
    }
    existing["recurrence_count"] = json!(existing_recurrence.max(incoming_recurrence));
    existing["issue_candidate_ready"] = json!(
        existing["issue_candidate_ready"].as_bool().unwrap_or(false)
            || bool_at(row, &["issue_candidate_ready"], false)
    );
    if let Some(keys) = existing
        .get_mut("source_feedback_dedupe_keys")
        .and_then(Value::as_array_mut)
    {
        let key = string_field(row, "dedupe_key");
        if !keys.iter().any(|value| value.as_str() == Some(key.as_str())) {
            keys.push(json!(key));
        }
    }
}

fn recurring_failure_families(feedback_rows: &[Value]) -> Vec<Value> {
    let mut by_family = BTreeMap::<String, Value>::new();
    for row in feedback_rows {
        let recurring =
            bool_at(row, &["issue_candidate_ready"], false) || usize_at(row, &["recurrence_count"]) >= 2;
        if !recurring {
            continue;
        }
        let key = format!("{}:{}", string_field(row, "category"), recurring_family_fingerprint(row));
        match by_family.get_mut(&key) {
            Some(existing) => merge_family(existing, row),
            None => {
                by_family.insert(key, recurring_failure_family(row));
            }
        }
    }
    let mut families = by_family.into_values().collect::<Vec<_>>();
    families.sort_by(|left, right| {
        bool_at(right, &["issue_candidate_ready"], false)
            .cmp(&bool_at(left, &["issue_candidate_ready"], false))
            .then_with(|| usize_at(right, &["recurrence_count"]).cmp(&usize_at(left, &["recurrence_count"])))
            .then_with(|| usize_at(left, &["operator_value_rank"]).cmp(&usize_at(right, &["operator_value_rank"])))
            .then_with(|| usize_at(left, &["priority_rank"]).cmp(&usize_at(right, &["priority_rank"])))
            .then_with(|| string_field(left, "family_key").cmp(&string_field(right, "family_key")))
    });
    families
}

pub(super) fn top_holes(feedback_rows: &[Value], generated_at: &str) -> Value {
    let mut rows = feedback_rows.to_vec();
    rows.sort_by_key(|row| {
        (
            usize_at(row, &["operator_value_rank"]),
            usize_at(row, &["priority_rank"]),
            string_field(row, "category"),
            string_field(row, "fingerprint"),
        )
    });
    let holes = rows.into_iter().take(10).collect::<Vec<_>>();
    let recurring_families = recurring_failure_families(feedback_rows);
    let issue_candidates = holes
        .iter()
        .filter(|row| {
            bool_at(row, &["issue_candidate_ready"], false)
                || usize_at(row, &["recurrence_count"]) >= 2
        })
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
                "failure_level": string_field(row, "failure_level"),
                "root_frame": string_field(row, "root_frame"),
                "remediation_level": string_field(row, "remediation_level"),
                "recurrence_count": usize_at(row, &["recurrence_count"]),
                "recurrence_threshold": usize_at(row, &["recurrence_threshold"]),
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
            "recurring_failure_family_count": recurring_families.len(),
            "source": "kernel_sentinel_feedback_inbox",
            "candidate_contract_version": 1,
            "recurring_family_contract_version": 1,
            "raw_finding_count_is_not_family_count": true
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
        "recurring_failure_family_contract": {
            "required_fields": [
                "family_key",
                "family_fingerprint",
                "category",
                "recurrence_count",
                "exemplar_fingerprint",
                "recommended_action"
            ],
            "separate_from_raw_finding_count": true
        },
        "holes": holes,
        "recurring_failure_families": recurring_families,
        "issue_candidates": issue_candidates
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feedback_row(fingerprint: &str, recurrence_count: usize) -> Value {
        json!({
            "dedupe_key": format!("correctness:{fingerprint}"),
            "fingerprint": fingerprint,
            "feedback_family_fingerprint": fingerprint,
            "severity": "high",
            "category": "correctness",
            "failure_level": "L2_boundary_contract_breach",
            "root_frame": "cross_boundary_contract",
            "remediation_level": "boundary_repair",
            "todo_priority": "P1",
            "priority_rank": 1,
            "operator_value_tier": "correctness",
            "operator_value_rank": 0,
            "summary": format!("{fingerprint} observed"),
            "recommended_action": "inspect the repeated Sentinel failure family",
            "evidence": [format!("runtime://{fingerprint}")],
            "recurrence_count": recurrence_count,
            "recurrence_threshold": 2,
            "issue_candidate_ready": recurrence_count >= 2
        })
    }

    #[test]
    fn top_holes_keeps_singletons_advisory_until_recurrence_threshold() {
        let top = top_holes(
            &[
                feedback_row("one_off_failure", 1),
                feedback_row("repeated_failure", 2),
            ],
            "2026-04-28T00:00:00Z",
        );

        assert_eq!(top["summary"]["hole_count"], 2);
        assert_eq!(top["summary"]["issue_candidate_count"], 1);
        assert_eq!(
            top["issue_candidates"][0]["fingerprint"],
            "kernel_sentinel:correctness:repeated_failure"
        );
        assert_eq!(top["issue_candidates"][0]["recurrence_count"], 2);
        assert_eq!(
            top["issue_candidates"][0]["failure_level"],
            "L2_boundary_contract_breach"
        );
        assert_eq!(
            top["issue_candidates"][0]["root_frame"],
            "cross_boundary_contract"
        );
        assert_eq!(
            top["issue_candidates"][0]["remediation_level"],
            "boundary_repair"
        );
    }

    #[test]
    fn top_holes_surfaces_recurring_failure_families_separately() {
        let top = top_holes(
            &[
                feedback_row("one_off_failure", 1),
                feedback_row("repeated_failure", 3),
            ],
            "2026-04-29T00:00:00Z",
        );

        assert_eq!(top["summary"]["recurring_failure_family_count"], 1);
        assert_eq!(top["summary"]["raw_finding_count_is_not_family_count"], true);
        assert_eq!(
            top["recurring_failure_families"][0]["family_key"],
            "correctness:repeated_failure"
        );
        assert_eq!(top["recurring_failure_families"][0]["recurrence_count"], 3);
        assert_eq!(
            top["recurring_failure_family_contract"]["separate_from_raw_finding_count"],
            true
        );
    }
}
