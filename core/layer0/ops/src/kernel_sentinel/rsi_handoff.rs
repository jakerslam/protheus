// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    KernelSentinelFinding, KernelSentinelFindingCategory, KernelSentinelSeverity,
    KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
};
use serde_json::{json, Value};

fn text(value: &Value, key: &str, fallback: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn detail<'a>(record: &'a Value, key: &str) -> Option<&'a Value> {
    record
        .get("details")
        .and_then(|details| details.get(key))
        .or_else(|| record.get(key))
        .or_else(|| {
            record
                .get("details")
                .and_then(|details| details.get("details"))
                .and_then(|details| details.get(key))
        })
}

fn detail_bool(record: &Value, key: &str) -> bool {
    detail(record, key).and_then(Value::as_bool).unwrap_or(false)
}

fn detail_text(record: &Value, key: &str) -> String {
    detail(record, key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .unwrap_or("")
        .to_string()
}

fn detail_array_nonempty(record: &Value, key: &str) -> bool {
    detail(record, key)
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false)
}

fn is_self_modification(record: &Value) -> bool {
    matches!(
        text(record, "kind", "").as_str(),
        "self_modification_proposal" | "rsi_safety_handoff" | "self_modification"
    )
        || detail_bool(record, "rsi_safety_handoff")
        || detail_bool(record, "self_modification_proposal")
}

fn wants_advance(record: &Value) -> bool {
    detail_bool(record, "advance_requested")
        || detail_bool(record, "apply_requested")
        || detail_bool(record, "monitor_requested")
        || matches!(
            detail_text(record, "requested_stage").as_str(),
            "apply" | "advance" | "monitor" | "rollback"
        )
        || matches!(
            detail_text(record, "stage").as_str(),
            "apply" | "advance" | "monitor" | "rollback"
        )
}

fn evidence(record: &Value, fallback: &str) -> Vec<String> {
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
        .unwrap_or_else(|| vec![fallback.to_string()])
}

fn missing_fields(record: &Value) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if detail_text(record, "sentinel_verdict").is_empty() {
        missing.push("sentinel_verdict");
    }
    if !detail_array_nonempty(record, "sentinel_evidence_refs") {
        missing.push("sentinel_evidence_refs");
    }
    if detail_text(record, "rollback_plan").is_empty() {
        missing.push("rollback_plan");
    }
    if detail_text(record, "post_apply_monitoring_criteria").is_empty()
        && !detail_array_nonempty(record, "post_apply_monitoring_criteria")
    {
        missing.push("post_apply_monitoring_criteria");
    }
    missing
}

fn handoff_finding(record: &Value, missing: &[&str]) -> KernelSentinelFinding {
    let subject = text(record, "subject", "self_modification");
    KernelSentinelFinding {
        schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        id: format!("rsi_handoff:missing_contract:{subject}"),
        severity: KernelSentinelSeverity::Critical,
        category: KernelSentinelFindingCategory::SecurityBoundary,
        fingerprint: format!("rsi_handoff:missing_contract:{subject}"),
        evidence: evidence(record, &format!("rsi://handoff/{subject}")),
        summary: format!(
            "{subject} attempted RSI/self-modification advance without required Sentinel handoff fields: {}",
            missing.join(",")
        ),
        recommended_action: "require Sentinel verdict, deterministic evidence refs, rollback plan, and post-apply monitoring before advance".to_string(),
        status: "open".to_string(),
    }
}

pub fn build_rsi_handoff_report(records: &[Value]) -> (Value, Vec<KernelSentinelFinding>) {
    let mut checked = Vec::new();
    let mut findings = Vec::new();
    for record in records {
        if !is_self_modification(record) {
            continue;
        }
        let subject = text(record, "subject", "self_modification");
        let advance_requested = wants_advance(record);
        let missing = if advance_requested {
            missing_fields(record)
        } else {
            Vec::new()
        };
        if !missing.is_empty() {
            findings.push(handoff_finding(record, &missing));
        }
        checked.push(json!({
            "subject": subject,
            "advance_requested": advance_requested,
            "missing_fields": missing,
            "ok": missing.is_empty()
        }));
    }
    (
        json!({
            "ok": findings.is_empty(),
            "type": "kernel_sentinel_rsi_safety_handoff",
            "checked_count": checked.len(),
            "blocking_failure_count": findings.len(),
            "required_fields": [
                "sentinel_verdict",
                "sentinel_evidence_refs",
                "rollback_plan",
                "post_apply_monitoring_criteria"
            ],
            "checked": checked
        }),
        findings,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_modification_cannot_advance_without_sentinel_contract() {
        let records = vec![json!({
            "subject": "patch-loop-1",
            "kind": "self_modification_proposal",
            "evidence": ["proposal://patch-loop-1"],
            "details": {"advance_requested": true, "sentinel_verdict": "allow"}
        })];
        let (report, findings) = build_rsi_handoff_report(&records);
        assert_eq!(report["blocking_failure_count"], Value::from(1));
        assert_eq!(findings[0].fingerprint, "rsi_handoff:missing_contract:patch-loop-1");
        assert!(findings[0].summary.contains("rollback_plan"));
    }

    #[test]
    fn complete_handoff_passes() {
        let records = vec![json!({
            "subject": "patch-loop-2",
            "kind": "self_modification_proposal",
            "details": {
                "advance_requested": true,
                "sentinel_verdict": "allow",
                "sentinel_evidence_refs": ["receipt://ok"],
                "rollback_plan": "restore previous artifact",
                "post_apply_monitoring_criteria": "no new critical Sentinel findings"
            }
        })];
        let (report, findings) = build_rsi_handoff_report(&records);
        assert!(findings.is_empty());
        assert_eq!(report["checked"][0]["ok"], true);
    }
}
