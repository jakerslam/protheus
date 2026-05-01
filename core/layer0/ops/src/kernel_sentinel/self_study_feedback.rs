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

fn evidence_signal_counts(item: &Value) -> (usize, usize, usize) {
    let evidence = item
        .get("evidence")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let field_citations = evidence.iter().filter(|row| row.starts_with("field://")).count();
    let check_citations = evidence.iter().filter(|row| row.starts_with("check://")).count();
    (evidence.len(), field_citations, check_citations)
}

fn concrete_feedback_action(action: &str) -> bool {
    action.trim() != "unknown" && action.split_whitespace().count() >= 4 && action.chars().count() >= 16
}

fn feedback_todo_actionability(item: &Value) -> Value {
    let (evidence_count, field_citations, check_citations) = evidence_signal_counts(item);
    let recurrence_count = usize_at(item, &["recurrence_count"]);
    let recurrence_threshold = usize_at(item, &["recurrence_threshold"]).max(1);
    let evidence_present = evidence_count > 0;
    let recurrence_or_freshness_support = recurrence_count >= recurrence_threshold
        || field_citations > 0
        || check_citations > 0
        || item.get("generated_at").is_some();
    let semantic_frame_present = string_field(item, "failure_level") != "unknown"
        && string_field(item, "root_frame") != "unknown"
        && string_field(item, "remediation_level") != "unknown";
    let concrete_next_action = concrete_feedback_action(&string_field(item, "recommended_action"));
    let dedupe_key_present = string_field(item, "dedupe_key") != "unknown";
    let mut missing = Vec::new();
    if !evidence_present {
        missing.push("evidence");
    }
    if !semantic_frame_present {
        missing.push("semantic_root_frame");
    }
    if !concrete_next_action {
        missing.push("concrete_next_action");
    }
    if !dedupe_key_present {
        missing.push("dedupe_key");
    }
    if !recurrence_or_freshness_support {
        missing.push("recurrence_or_freshness_support");
    }
    let state = if missing.is_empty() && recurrence_count >= recurrence_threshold {
        "todo_ready"
    } else if !evidence_present || !semantic_frame_present || !concrete_next_action {
        "needs_root_cause_synthesis"
    } else {
        "triage_to_todo"
    };
    json!({
        "type": "kernel_sentinel_feedback_to_todo_actionability",
        "state": state,
        "allowed_states": ["todo_ready", "triage_to_todo", "needs_root_cause_synthesis"],
        "requirements": {
            "evidence_present": evidence_present,
            "recurrence_or_freshness_support": recurrence_or_freshness_support,
            "semantic_frame_present": semantic_frame_present,
            "concrete_next_action": concrete_next_action,
            "dedupe_key_present": dedupe_key_present,
        },
        "missing_requirements": missing,
        "human_review_required": true,
        "safe_to_mutate_todo": false,
        "policy": "sentinel_feedback_may_draft_todo_candidates_but_must_not_mutate_todo_without_review"
    })
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn operator_value_tier(item: &Value) -> &'static str {
    let category = string_field(item, "category").to_ascii_lowercase();
    if contains_any(
        &category,
        &[
            "correctness",
            "receipt_integrity",
            "invariant",
            "truth",
            "contract",
        ],
    ) {
        return "correctness";
    }
    if contains_any(
        &category,
        &[
            "security",
            "capability",
            "permission",
            "auth",
            "sandbox",
            "secret",
        ],
    ) {
        return "security";
    }
    if contains_any(&category, &["release", "gate", "blocking", "proof", "ci"]) {
        return "release_blocking";
    }
    if contains_any(
        &category,
        &["optimization", "performance", "latency", "cleanup", "churn"],
    ) {
        return "optimization";
    }

    let semantic_context = format!(
        "{} {} {} {}",
        string_field(item, "root_frame"),
        string_field(item, "failure_level"),
        string_field(item, "summary"),
        string_field(item, "recommended_action")
    )
    .to_ascii_lowercase();
    if contains_any(
        &semantic_context,
        &["correctness", "invariant", "truth", "contract", "receipt"],
    ) {
        "correctness"
    } else if contains_any(
        &semantic_context,
        &["security", "capability", "permission", "auth", "sandbox", "secret"],
    ) {
        "security"
    } else if contains_any(
        &semantic_context,
        &["release", "gate", "blocking", "proof", "ci"],
    ) {
        "release_blocking"
    } else if contains_any(
        &semantic_context,
        &["optimization", "performance", "latency", "cleanup", "churn"],
    ) {
        "optimization"
    } else {
        "general"
    }
}

fn operator_value_priority(tier: &str) -> u8 {
    match tier {
        "correctness" => 0,
        "security" => 1,
        "release_blocking" => 2,
        "optimization" => 3,
        _ => 4,
    }
}

fn feedback_search_text(item: &Value) -> String {
    let mut parts = vec![
        string_field(item, "category"),
        string_field(item, "fingerprint"),
        string_field(item, "feedback_family_fingerprint"),
        string_field(item, "summary"),
        string_field(item, "recommended_action"),
    ];
    if let Some(evidence) = item.get("evidence").and_then(Value::as_array) {
        parts.extend(evidence.iter().filter_map(Value::as_str).map(str::to_string));
    }
    parts.join(" ").to_ascii_lowercase()
}

fn is_empty_response_variant(item: &Value) -> bool {
    contains_any(
        &feedback_search_text(item),
        &[
            "empty response",
            "empty_response",
            "blank response",
            "blank_response",
            "no response",
            "no_response",
            "non-response",
            "zero response",
            "final_response=empty",
            "final_response_length=0",
        ],
    )
}

fn item_issue_candidate_ready(item: &Value) -> bool {
    item["issue_candidate_ready"].as_bool().unwrap_or(false)
}

fn annotate_empty_response_parent_downrank(rows: &mut [Value]) {
    let parent_exists = rows
        .iter()
        .any(|row| is_empty_response_variant(row) && item_issue_candidate_ready(row));
    for row in rows {
        let is_empty_variant = is_empty_response_variant(row);
        let downrank = parent_exists && is_empty_variant && !item_issue_candidate_ready(row);
        row["empty_response_variant"] = json!(is_empty_variant);
        row["empty_response_parent_issue_exists"] = json!(parent_exists && is_empty_variant);
        row["downranked_by_parent_issue"] = json!(downrank);
        row["duplicate_family_rank"] = json!(usize::from(downrank));
    }
}

fn feedback_quality_score(item: &Value) -> usize {
    let (evidence_count, field_citations, check_citations) = evidence_signal_counts(item);
    let recurrence_bonus = usize_at(item, &["recurrence_count"]).saturating_sub(1).min(5) * 3;
    let actionable_bonus = usize::from(string_field(item, "recommended_action") != "unknown") * 5;
    let semantic_bonus = usize::from(string_field(item, "failure_level") != "unknown") * 5;
    evidence_count.min(6) * 2
        + field_citations * 4
        + check_citations * 5
        + recurrence_bonus
        + actionable_bonus
        + semantic_bonus
        + usize::from(item["issue_candidate_ready"].as_bool().unwrap_or(false)) * 4
}

fn refresh_feedback_quality(item: &mut Value) {
    let (evidence_count, field_citations, check_citations) = evidence_signal_counts(item);
    let operator_value_tier = operator_value_tier(item);
    let todo_actionability = feedback_todo_actionability(item);
    let todo_actionability_state = string_field(&todo_actionability, "state");
    item["operator_value_tier"] = json!(operator_value_tier);
    item["operator_value_rank"] = json!(operator_value_priority(operator_value_tier));
    item["feedback_quality_score"] = json!(feedback_quality_score(item));
    item["todo_actionability_state"] = json!(todo_actionability_state);
    item["todo_ready"] = json!(todo_actionability["state"] == "todo_ready");
    item["todo_actionability"] = todo_actionability;
    item["quality_signals"] = json!({
        "evidence_count": evidence_count,
        "field_citation_count": field_citations,
        "check_citation_count": check_citations,
        "operator_value_tier": operator_value_tier,
        "operator_value_rank": operator_value_priority(operator_value_tier),
        "recurrence_count": usize_at(item, &["recurrence_count"]),
        "actionable_recommendation": string_field(item, "recommended_action") != "unknown",
        "todo_actionability_state": item["todo_actionability_state"].clone(),
        "semantic_frame_present": string_field(item, "failure_level") != "unknown"
    });
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
    let mut item = json!({
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
    });
    refresh_feedback_quality(&mut item);
    item
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
        refresh_feedback_quality(target);
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
    let mut rows = by_key.into_values().collect::<Vec<_>>();
    for row in &mut rows {
        refresh_feedback_quality(row);
    }
    annotate_empty_response_parent_downrank(&mut rows);
    rows.sort_by(|left, right| {
        usize_at(left, &["duplicate_family_rank"])
            .cmp(&usize_at(right, &["duplicate_family_rank"]))
            .then_with(|| {
                usize_at(left, &["operator_value_rank"])
            .cmp(&usize_at(right, &["operator_value_rank"]))
            })
            .then_with(|| {
                usize_at(left, &["priority_rank"]).cmp(&usize_at(right, &["priority_rank"]))
            })
            .then_with(|| {
                usize_at(right, &["feedback_quality_score"])
                    .cmp(&usize_at(left, &["feedback_quality_score"]))
            })
            .then_with(|| string_field(left, "dedupe_key").cmp(&string_field(right, "dedupe_key")))
    });
    for (index, row) in rows.iter_mut().enumerate() {
        row["feedback_quality_rank"] = json!(index + 1);
    }
    rows
}

#[cfg(test)]
#[path = "self_study_feedback_tests.rs"]
mod self_study_feedback_tests;
