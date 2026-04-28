// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;

use super::ProducerSpec;

fn error_class(error: &str) -> &'static str {
    let lower = error.to_ascii_lowercase();
    if lower == "read_failed" {
        "read_failed"
    } else if lower.contains("eof") {
        "truncated_json"
    } else if lower.contains("trailing") {
        "trailing_characters"
    } else if lower.contains("expected")
        || lower.contains("key must be")
        || lower.contains("invalid")
    {
        "invalid_json_syntax"
    } else {
        "json_parse_error"
    }
}

pub(super) fn row(path: &Path, spec: &ProducerSpec, line: Option<usize>, error: &str) -> Value {
    json!({
        "path": path,
        "file_name": path.file_name().and_then(|value| value.to_str()).unwrap_or("unknown"),
        "producer_id": spec.id,
        "target_stream": spec.target_stream,
        "authority_class": spec.authority_class,
        "kind": spec.kind,
        "line": line,
        "error": error,
        "error_class": error_class(error)
    })
}

pub(super) fn count_by_key(records: &[Value], key: &str) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for record in records {
        if let Some(value) = record.get(key).and_then(Value::as_str) {
            *counts.entry(value.to_string()).or_insert(0usize) += 1;
        }
    }
    counts
}

fn deterministic_records(records: &[Value]) -> Vec<Value> {
    records
        .iter()
        .filter(|record| {
            record
                .get("authority_class")
                .and_then(Value::as_str)
                == Some("deterministic_kernel_authority")
        })
        .cloned()
        .collect()
}

pub(super) fn deterministic_record_count(records: &[Value]) -> usize {
    deterministic_records(records).len()
}

pub(super) fn threshold_guard(records: &[Value], threshold: usize) -> Value {
    let deterministic = deterministic_records(records);
    let count = deterministic.len();
    let ok = count <= threshold;
    json!({
        "ok": ok,
        "threshold": threshold,
        "malformed_deterministic_record_count": count,
        "threshold_exceeded": !ok,
        "failure_reason": if ok { Value::Null } else { Value::from("malformed_deterministic_evidence_exceeds_threshold") },
        "by_producer": count_by_key(&deterministic, "producer_id"),
        "by_file_name": count_by_key(&deterministic, "file_name"),
        "by_error_class": count_by_key(&deterministic, "error_class"),
        "recommended_action": if ok {
            "continue monitoring deterministic Sentinel evidence producers"
        } else {
            "repair malformed deterministic Sentinel evidence producers before trusting health or release readiness"
        }
    })
}

fn str_field<'a>(record: &'a Value, key: &str, fallback: &'a str) -> &'a str {
    record.get(key).and_then(Value::as_str).unwrap_or(fallback)
}

fn remediation_action(error_class: &str, receipt_producer: bool) -> &'static str {
    match (error_class, receipt_producer) {
        ("truncated_json", true) => {
            "repair receipt producer writes so each JSONL receipt is flushed atomically as one complete object per line"
        }
        ("invalid_json_syntax", true) => {
            "validate receipt JSON before append and strip log prefixes, partial metadata prefixes, or non-JSON noise from receipt lines"
        }
        ("trailing_characters", true) => {
            "remove trailing commas or appended log text after each receipt JSON object before Sentinel collection"
        }
        ("read_failed", true) => {
            "restore readable receipt producer paths and permissions before trusting Sentinel collection"
        }
        (_, true) => {
            "repair malformed receipt producer output and rerun Kernel Sentinel collection before trusting release evidence"
        }
        ("read_failed", false) => {
            "restore readable producer paths and permissions before trusting Sentinel collection"
        }
        _ => "repair malformed producer JSONL rows before trusting Sentinel collection",
    }
}

pub(super) fn remediation_hints(records: &[Value]) -> Vec<Value> {
    let mut hints = BTreeMap::new();
    for record in records {
        let producer_id = str_field(record, "producer_id", "unknown");
        let target_stream = str_field(record, "target_stream", "unknown");
        let file_name = str_field(record, "file_name", "unknown");
        let error_class = str_field(record, "error_class", "json_parse_error");
        let authority_class = str_field(record, "authority_class", "unknown");
        let receipt_producer = target_stream.contains("receipt")
            || producer_id.contains("receipt")
            || producer_id.contains("verity");
        let key = format!("{producer_id}:{file_name}:{error_class}");
        hints.entry(key).or_insert_with(|| {
            json!({
                "producer_id": producer_id,
                "target_stream": target_stream,
                "file_name": file_name,
                "error_class": error_class,
                "receipt_producer": receipt_producer,
                "priority": if authority_class == "deterministic_kernel_authority" { "blocking" } else { "advisory" },
                "recommended_action": remediation_action(error_class, receipt_producer),
                "rerun": "infring kernel-sentinel collect --json"
            })
        });
    }
    hints.into_values().collect()
}
