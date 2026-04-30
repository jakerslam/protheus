// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{normalize_key, RawEvidenceRecord};
use serde_json::Value;

fn push_unique(evidence: &mut Vec<String>, citation: String) {
    if !evidence.iter().any(|row| row == &citation) {
        evidence.push(citation);
    }
}

fn failed_status(status: &str) -> bool {
    matches!(
        normalize_key(status).as_str(),
        "fail" | "failed" | "blocked" | "invalid" | "critical" | "error"
    )
}

fn check_string(record: &RawEvidenceRecord, key: &str) -> Option<String> {
    record
        .details
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .map(ToString::to_string)
}

pub(super) fn append_failure_citations(
    source: &str,
    id: &str,
    record: &RawEvidenceRecord,
    evidence: &mut Vec<String>,
) {
    if record.ok == Some(false) {
        push_unique(evidence, format!("field://{source}/{id}/ok=false"));
    }
    if let Some(status) = record.status.as_deref().filter(|status| failed_status(status)) {
        push_unique(evidence, format!("field://{source}/{id}/status={}", normalize_key(status)));
    }
    if record.details.get("pass").and_then(Value::as_bool) == Some(false) {
        push_unique(evidence, format!("field://{source}/{id}/pass=false"));
    }
    for key in ["failing_check", "failed_check", "check_id"] {
        if let Some(check) = check_string(record, key) {
            push_unique(evidence, format!("check://{source}/{id}/{key}={check}"));
        }
    }
}
