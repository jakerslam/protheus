// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::Value;
use std::path::Path;

pub(super) fn repair_jsonl_line(
    producer_id: &str,
    path: &Path,
    line: &str,
) -> Option<(Value, &'static str)> {
    let file_name = path.file_name().and_then(|value| value.to_str())?;
    if !file_name.contains("receipt") {
        return None;
    }
    let trimmed = line.trim_start_matches('\u{feff}').trim();
    let mut candidates = Vec::new();
    if producer_id == "verity_receipts" && trimmed.starts_with("a\":{") {
        candidates.push((
            "verity_receipts_missing_metadata_prefix",
            format!("{{\"metadat{trimmed}"),
        ));
    }
    if trimmed.starts_with("\"metadata\":") {
        candidates.push((
            "verity_receipts_missing_object_open",
            format!("{{{trimmed}"),
        ));
    }
    if trimmed.ends_with("},") {
        candidates.push((
            "receipt_line_trailing_comma",
            trimmed.trim_end_matches(',').to_string(),
        ));
    }
    if !trimmed.starts_with('{') {
        if let Some(offset) = trimmed.find('{') {
            candidates.push((
                "receipt_line_leading_noise_before_object",
                trimmed[offset..].to_string(),
            ));
        }
    }
    if trimmed.starts_with('{') && !trimmed.ends_with('}') {
        candidates.push((
            "receipt_line_missing_closing_brace",
            format!("{trimmed}}}"),
        ));
    }
    for (repair_id, candidate) in candidates {
        if let Ok(value) = serde_json::from_str::<Value>(&candidate) {
            if !looks_like_receipt_row(&value) {
                continue;
            }
            return Some((value, repair_id));
        }
    }
    None
}

fn looks_like_receipt_row(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    [
        "receipt_hash",
        "operation_hash",
        "operation_type",
        "subject",
        "id",
        "ok",
        "type",
    ]
    .iter()
    .any(|key| object.contains_key(*key))
}
