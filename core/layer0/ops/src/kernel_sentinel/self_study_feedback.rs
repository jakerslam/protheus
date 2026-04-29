// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

use crate::kernel_sentinel::kernel_sentinel_semantic_frame_for_parts;

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

fn severity_priority(severity: &str) -> u8 {
    match severity {
        "critical" => 0,
        "high" => 1,
        "medium" => 2,
        "low" => 3,
        _ => 4,
    }
}

fn todo_priority(severity: &str, category: &str) -> &'static str {
    match (severity, category) {
        ("critical", _) => "P0",
        ("high", "security_boundary" | "capability_enforcement" | "receipt_integrity") => "P0",
        ("high", _) => "P1",
        ("medium", _) => "P2",
        _ => "P3",
    }
}

fn synthetic_harness_failure_family(fingerprint: &str) -> Option<&'static str> {
    const MISTY_ROUND_PREFIX: &str = "misty_simulated_round";
    let normalized = fingerprint.to_ascii_lowercase();
    let prefix_index = normalized.find(MISTY_ROUND_PREFIX)?;
    let suffix_index = prefix_index + MISTY_ROUND_PREFIX.len();
    let rest = &normalized[suffix_index..];
    let digit_count = rest.chars().take_while(|ch| ch.is_ascii_digit()).count();
    if digit_count == 0 {
        return None;
    }
    let after_digits = &rest[digit_count..];
    if matches!(
        after_digits,
        "_failure" | "_failures" | "-failure" | "-failures" | ":failure" | ":failures"
    ) {
        Some("synthetic_user_chat_harness:misty_simulated_failures")
    } else {
        None
    }
}

fn feedback_family_fingerprint(fingerprint: &str) -> String {
    synthetic_harness_failure_family(fingerprint)
        .map(str::to_string)
        .unwrap_or_else(|| fingerprint.to_string())
}

fn feedback_item(finding: &Value, generated_at: &str) -> Value {
    let severity = string_field(finding, "severity");
    let category = string_field(finding, "category");
    let fingerprint = string_field(finding, "fingerprint");
    let family_fingerprint = feedback_family_fingerprint(&fingerprint);
    let evidence = finding.get("evidence").cloned().unwrap_or_else(|| json!([]));
    let semantic_frame = kernel_sentinel_semantic_frame_for_parts(
        &category,
        &severity,
        &fingerprint,
        &string_field(finding, "summary"),
        &string_field(finding, "recommended_action"),
    );
    json!({
        "type": "kernel_sentinel_feedback_item",
        "source": "kernel_sentinel",
        "generated_at": generated_at,
        "status": string_field(finding, "status"),
        "fingerprint": fingerprint,
        "feedback_family_fingerprint": family_fingerprint,
        "dedupe_key": format!("{category}:{family_fingerprint}"),
        "severity": severity,
        "category": category,
        "failure_level": semantic_frame["failure_level"].clone(),
        "root_frame": semantic_frame["root_frame"].clone(),
        "remediation_level": semantic_frame["remediation_level"].clone(),
        "todo_priority": todo_priority(&severity, &category),
        "priority_rank": severity_priority(&severity),
        "summary": string_field(finding, "summary"),
        "recommended_action": string_field(finding, "recommended_action"),
        "evidence": evidence.clone(),
        "per_run_evidence": [{
            "fingerprint": fingerprint,
            "evidence": evidence
        }],
        "recurrence_count": 1,
        "recurrence_threshold": 2,
        "issue_candidate_ready": false,
        "preservation_policy": "preserve_until_resolved_or_waived_by_kernel_receipt"
    })
}

fn merge_feedback_evidence(target: &mut Value, incoming: &Value) {
    let mut evidence_seen = BTreeSet::<String>::new();
    if let Some(rows) = target.get_mut("evidence").and_then(Value::as_array_mut) {
        for row in rows.iter() {
            evidence_seen.insert(row.to_string());
        }
        for row in incoming
            .get("evidence")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if evidence_seen.insert(row.to_string()) {
                rows.push(row.clone());
            }
        }
    }

    let mut run_seen = BTreeSet::<String>::new();
    if let Some(rows) = target.get_mut("per_run_evidence").and_then(Value::as_array_mut) {
        for row in rows.iter() {
            run_seen.insert(row.to_string());
        }
        for row in incoming
            .get("per_run_evidence")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if run_seen.insert(row.to_string()) {
                rows.push(row.clone());
            }
        }
        let recurrence_count = rows.len();
        target["recurrence_count"] = json!(recurrence_count);
        target["recurrence_threshold"] = json!(2);
        target["issue_candidate_ready"] = json!(recurrence_count >= 2);
    }
}

pub(super) fn build_feedback_inbox(report: &Value, generated_at: &str) -> Vec<Value> {
    let mut by_key: BTreeMap<String, Value> = BTreeMap::new();
    for finding in report
        .get("findings")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if string_field(finding, "status") != "open" {
            continue;
        }
        let item = feedback_item(finding, generated_at);
        let key = string_field(&item, "dedupe_key");
        match by_key.get_mut(&key) {
            Some(existing)
                if usize_at(existing, &["priority_rank"]) <= usize_at(&item, &["priority_rank"]) =>
            {
                merge_feedback_evidence(existing, &item);
            }
            Some(existing) => {
                let mut replacement = item;
                merge_feedback_evidence(&mut replacement, existing);
                *existing = replacement;
            }
            None => {
                by_key.insert(key, item);
            }
        }
    }
    by_key.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_round_failures_collapse_to_one_feedback_item() {
        let report = json!({
            "findings": [
                {
                    "status": "open",
                    "severity": "medium",
                    "category": "runtime_correctness",
                    "fingerprint": "synthetic_user_chat_harness:misty_simulated_round01_failures",
                    "summary": "round 01 synthetic chat failure",
                    "recommended_action": "inspect synthetic chat harness output",
                    "evidence": ["synthetic://misty/round01"]
                },
                {
                    "status": "open",
                    "severity": "high",
                    "category": "runtime_correctness",
                    "fingerprint": "synthetic_user_chat_harness:misty_simulated_round02_failures",
                    "summary": "round 02 synthetic chat failure",
                    "recommended_action": "inspect synthetic chat harness output",
                    "evidence": ["synthetic://misty/round02"]
                }
            ]
        });

        let rows = build_feedback_inbox(&report, "2026-04-28T00:00:00Z");

        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0]["feedback_family_fingerprint"],
            "synthetic_user_chat_harness:misty_simulated_failures"
        );
        assert_eq!(
            rows[0]["dedupe_key"],
            "runtime_correctness:synthetic_user_chat_harness:misty_simulated_failures"
        );
        assert_eq!(rows[0]["severity"], "high");
        assert_eq!(rows[0]["failure_level"], "L2_boundary_contract_breach");
        assert_eq!(rows[0]["root_frame"], "cross_boundary_contract");
        assert_eq!(rows[0]["remediation_level"], "boundary_repair");
        assert_eq!(
            rows[0]["fingerprint"],
            "synthetic_user_chat_harness:misty_simulated_round02_failures"
        );
        assert_eq!(
            rows[0]["evidence"],
            json!(["synthetic://misty/round02", "synthetic://misty/round01"])
        );
        assert_eq!(rows[0]["per_run_evidence"].as_array().unwrap().len(), 2);
        assert_eq!(rows[0]["recurrence_count"], 2);
        assert_eq!(rows[0]["recurrence_threshold"], 2);
        assert_eq!(rows[0]["issue_candidate_ready"], true);
    }
}
