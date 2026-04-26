// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use crate::kernel_sentinel::{
    KernelSentinelFinding, KernelSentinelFindingCategory, KernelSentinelSeverity,
    KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
};
use serde_json::{json, Value};

fn value<'a>(record: &'a Value, key: &str) -> Option<&'a Value> {
    record
        .get(key)
        .or_else(|| record.get("details").and_then(|details| details.get(key)))
        .or_else(|| {
            record
                .get("details")
                .and_then(|details| details.get("details"))
                .and_then(|details| details.get(key))
        })
}

fn value_str<'a>(record: &'a Value, key: &str) -> &'a str {
    value(record, key).and_then(Value::as_str).unwrap_or("")
}

fn value_bool(record: &Value, key: &str) -> bool {
    value(record, key)
        .map(|raw| {
            raw.as_bool()
                .unwrap_or_else(|| matches!(raw.as_str().unwrap_or("").trim().to_lowercase().as_str(), "1" | "true" | "yes"))
        })
        .unwrap_or(false)
}

fn claimed_bool(record: &Value, key: &str) -> bool {
    record
        .get("details")
        .and_then(|details| details.get(key))
        .or_else(|| {
            record
                .get("details")
                .and_then(|details| details.get("details"))
                .and_then(|details| details.get(key))
        })
        .map(|raw| {
            raw.as_bool()
                .unwrap_or_else(|| matches!(raw.as_str().unwrap_or("").trim().to_lowercase().as_str(), "1" | "true" | "yes"))
        })
        .unwrap_or(false)
}

fn claimed_authority_class(record: &Value) -> &str {
    record
        .get("details")
        .and_then(|details| details.get("authority_class"))
        .or_else(|| {
            record
                .get("details")
                .and_then(|details| details.get("details"))
                .and_then(|details| details.get("authority_class"))
        })
        .and_then(Value::as_str)
        .unwrap_or("")
}

fn source_is_control_plane_eval(record: &Value) -> bool {
    value_str(record, "source") == "control_plane_eval"
}

fn clean_token(raw: &str, fallback: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn source_reference_present(record: &Value) -> bool {
    !value_str(record, "source_reference").trim().is_empty()
        || !value_str(record, "source_ref").trim().is_empty()
        || !value_str(record, "trace_id").trim().is_empty()
        || record
            .get("evidence")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false)
}

fn claims_kernel_authority(record: &Value) -> bool {
    claimed_bool(record, "may_block_release")
        || claimed_bool(record, "may_write_verdict")
        || claimed_bool(record, "may_waive_finding")
        || matches!(
            claimed_authority_class(record),
            "deterministic_kernel_authority" | "kernel_authority"
        )
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
        .unwrap_or_else(|| vec![format!("evidence://{}", clean_token(value_str(record, "id"), "control-plane-eval"))])
}

fn advisory_finding(record: &Value, rule: &str, severity: KernelSentinelSeverity) -> KernelSentinelFinding {
    let subject = clean_token(value_str(record, "subject"), "control_plane_eval");
    let id = clean_token(value_str(record, "id"), "control-plane-eval");
    KernelSentinelFinding {
        schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        id: format!("advisory_bridge:{rule}:{id}"),
        severity,
        category: KernelSentinelFindingCategory::SecurityBoundary,
        fingerprint: format!("advisory_bridge:{rule}:{subject}"),
        evidence: evidence_refs(record),
        summary: format!("control-plane eval advisory bridge violated {rule} for {subject}"),
        recommended_action: "keep control-plane eval advisory-only; use deterministic Kernel evidence for verdicts, waivers, and release blocking".to_string(),
        status: "open".to_string(),
    }
}

pub(super) fn build_advisory_bridge_report(records: &[Value]) -> (Value, Vec<KernelSentinelFinding>) {
    let mut findings = Vec::new();
    let mut checked_count = 0usize;
    let mut authority_claim_count = 0usize;
    let mut missing_source_reference_count = 0usize;
    for record in records {
        if !source_is_control_plane_eval(record) {
            continue;
        }
        checked_count += 1;
        if claims_kernel_authority(record) {
            authority_claim_count += 1;
            findings.push(advisory_finding(
                record,
                "authority_claim",
                KernelSentinelSeverity::High,
            ));
        }
        if !source_reference_present(record) {
            missing_source_reference_count += 1;
            findings.push(advisory_finding(
                record,
                "missing_source_reference",
                KernelSentinelSeverity::Medium,
            ));
        }
    }
    let report = json!({
        "ok": findings.is_empty(),
        "checked_count": checked_count,
        "authority_claim_count": authority_claim_count,
        "missing_source_reference_count": missing_source_reference_count,
        "finding_count": findings.len(),
        "constraints": {
            "control_plane_eval_authority_class": "advisory_workflow_quality",
            "may_block_release": false,
            "may_write_verdict": false,
            "may_waive_finding": false,
            "source_reference_required": true
        },
        "findings": findings
    });
    (report, findings)
}
