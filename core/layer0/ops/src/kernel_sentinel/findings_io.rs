// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{validate_finding, KernelSentinelFinding};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub(super) fn read_jsonl_findings(path: &Path) -> (Vec<KernelSentinelFinding>, Vec<Value>) {
    let raw = fs::read_to_string(path).unwrap_or_default();
    let source_path = path.display().to_string();
    let mut findings = Vec::new();
    let mut malformed = Vec::new();
    for (idx, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<KernelSentinelFinding>(trimmed) {
            Ok(finding) if validate_finding(&finding).is_ok() => findings.push(finding),
            Ok(finding) => malformed.push(malformed_row(idx + 1, Some(finding.id), &source_path, "invalid_finding")),
            Err(err) => malformed.push(malformed_row(idx + 1, None, &source_path, &err.to_string())),
        }
    }
    (findings, malformed)
}

fn malformed_row(line: usize, id: Option<String>, source_path: &str, error: &str) -> Value {
    let mut row = json!({
        "line": line,
        "source_path": source_path,
        "source_kind": "kernel_sentinel_findings",
        "error": error
    });
    if let Some(id) = id {
        row["id"] = Value::String(id);
    }
    row
}
