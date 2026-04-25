// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    KernelSentinelFinding, KernelSentinelFindingCategory, KernelSentinelSeverity,
    KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
};
use serde_json::{json, Value};

fn value_str<'a>(value: &'a Value, key: &str) -> &'a str {
    value
        .get(key)
        .or_else(|| value.get("details").and_then(|details| details.get(key)))
        .and_then(Value::as_str)
        .unwrap_or("")
}

fn transition_kind(kind: &str) -> bool {
    matches!(
        kind,
        "state_transition"
            | "task_transition"
            | "task_state_transition"
            | "session_transition"
            | "runtime_state_transition"
            | "retry"
            | "reopen"
    )
}

fn state_pair(record: &Value) -> Option<(&str, &str)> {
    let from = [
        "from_state",
        "previous_state",
        "old_state",
        "source_state",
        "from",
    ]
    .iter()
    .find_map(|key| {
        let value = value_str(record, key);
        (!value.is_empty()).then_some(value)
    })?;
    let to = ["to_state", "next_state", "new_state", "target_state", "to"]
        .iter()
        .find_map(|key| {
            let value = value_str(record, key);
            (!value.is_empty()).then_some(value)
        })?;
    Some((from, to))
}

fn canonical_state(raw: &str) -> String {
    raw.trim()
        .to_ascii_lowercase()
        .replace([' ', '-'], "_")
        .trim_matches('_')
        .to_string()
}

fn terminal_state(state: &str) -> bool {
    matches!(
        canonical_state(state).as_str(),
        "failed" | "cancelled" | "canceled" | "complete" | "completed" | "succeeded" | "success"
    )
}

fn non_terminal_state(state: &str) -> bool {
    !terminal_state(state)
        || matches!(
            canonical_state(state).as_str(),
            "queued" | "pending" | "ready" | "running" | "in_progress" | "retrying" | "reopened"
        )
}

fn row_contains_token(value: &Value, token: &str) -> bool {
    !token.trim().is_empty()
        && (["id", "subject", "kind", "fingerprint", "receipt_for"]
            .iter()
            .any(|key| value_str(value, key).contains(token))
            || value
                .get("evidence")
                .and_then(Value::as_array)
                .map(|rows| rows.iter().any(|row| row.as_str().unwrap_or("").contains(token)))
                .unwrap_or(false))
}

fn rollback_receipt_matches(receipt: &Value, transition: &Value) -> bool {
    if value_str(receipt, "source") != "kernel_receipt" {
        return false;
    }
    let kind = value_str(receipt, "kind");
    let rollback_receipt = matches!(
        kind,
        "rollback_receipt" | "state_transition_rollback_receipt" | "terminal_reopen_receipt"
    );
    rollback_receipt
        && (row_contains_token(receipt, value_str(transition, "id"))
            || row_contains_token(receipt, value_str(transition, "subject")))
}

fn evidence_refs(record: &Value) -> Vec<String> {
    record
        .get("evidence")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| vec![format!("evidence://{}", value_str(record, "id"))])
}

fn illegal_terminal_reopen(record: &Value) -> Option<(String, String)> {
    if !transition_kind(value_str(record, "kind")) && state_pair(record).is_none() {
        return None;
    }
    let (from, to) = state_pair(record)?;
    if terminal_state(from) && non_terminal_state(to) {
        Some((canonical_state(from), canonical_state(to)))
    } else {
        None
    }
}

fn transition_finding(record: &Value, from: &str, to: &str) -> KernelSentinelFinding {
    let action_id = value_str(record, "id");
    let subject = value_str(record, "subject");
    KernelSentinelFinding {
        schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        id: format!("illegal_state_transition:{action_id}"),
        severity: KernelSentinelSeverity::Critical,
        category: KernelSentinelFindingCategory::StateTransition,
        fingerprint: format!("state_transition:illegal_reopen:{subject}:{from}->{to}"),
        evidence: evidence_refs(record),
        summary: format!(
            "{subject} attempted illegal terminal-state transition {from}->{to} without rollback receipt"
        ),
        recommended_action: format!(
            "require an explicit rollback_receipt before reopening terminal state {from}"
        ),
        status: "open".to_string(),
    }
}

pub(super) fn build_state_transition_report(
    records: &[Value],
) -> (Value, Vec<KernelSentinelFinding>) {
    let receipts = records
        .iter()
        .filter(|record| value_str(record, "source") == "kernel_receipt")
        .collect::<Vec<_>>();
    let mut checked_transition_count = 0usize;
    let mut terminal_reopen_attempt_count = 0usize;
    let mut findings = Vec::new();
    for record in records {
        if value_str(record, "source") == "kernel_receipt" {
            continue;
        }
        let Some((from, to)) = illegal_terminal_reopen(record) else {
            if transition_kind(value_str(record, "kind")) || state_pair(record).is_some() {
                checked_transition_count += 1;
            }
            continue;
        };
        checked_transition_count += 1;
        terminal_reopen_attempt_count += 1;
        if !receipts
            .iter()
            .any(|receipt| rollback_receipt_matches(receipt, record))
        {
            findings.push(transition_finding(record, &from, &to));
        }
    }
    let report = json!({
        "ok": findings.is_empty(),
        "checked_invariants": [
            "terminal_state_immutability",
            "rollback_receipt_required",
            "retry_reopen_legality",
            "cancelled_failed_task_behavior"
        ],
        "checked_transition_count": checked_transition_count,
        "terminal_reopen_attempt_count": terminal_reopen_attempt_count,
        "illegal_transition_count": findings.len(),
        "findings": findings
    });
    (report, findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_task_reopen_without_rollback_receipt_is_critical() {
        let records = vec![json!({
            "source": "runtime_observation",
            "id": "transition-1",
            "subject": "task-1",
            "kind": "task_state_transition",
            "details": {
                "from_state": "Failed",
                "to_state": "Running"
            },
            "evidence": ["trace://task-1/transition-1"]
        })];
        let (report, findings) = build_state_transition_report(&records);
        assert_eq!(report["illegal_transition_count"], Value::from(1));
        assert_eq!(findings[0].severity, KernelSentinelSeverity::Critical);
        assert_eq!(
            findings[0].fingerprint,
            "state_transition:illegal_reopen:task-1:failed->running"
        );
    }

    #[test]
    fn rollback_receipt_allows_terminal_state_reopen() {
        let records = vec![
            json!({
                "source": "runtime_observation",
                "id": "transition-2",
                "subject": "task-2",
                "kind": "task_state_transition",
                "details": {
                    "from_state": "Cancelled",
                    "to_state": "Retrying"
                },
                "evidence": ["trace://task-2/transition-2"]
            }),
            json!({
                "source": "kernel_receipt",
                "id": "rollback-2",
                "subject": "task-2",
                "kind": "rollback_receipt",
                "evidence": ["receipt://task-2/transition-2"]
            }),
        ];
        let (report, findings) = build_state_transition_report(&records);
        assert!(findings.is_empty());
        assert_eq!(report["terminal_reopen_attempt_count"], Value::from(1));
    }
}
