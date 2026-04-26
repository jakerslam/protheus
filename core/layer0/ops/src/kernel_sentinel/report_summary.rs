// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    finding_lifecycle::normalize_finding_status, KernelSentinelFinding,
    KernelSentinelFindingCategory, KernelSentinelSeverity,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

pub(super) fn count_by_status(findings: &[KernelSentinelFinding]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for finding in findings {
        *counts
            .entry(normalize_finding_status(&finding.status))
            .or_insert(0) += 1;
    }
    counts
}

pub(super) fn count_by_category(findings: &[KernelSentinelFinding]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for finding in findings {
        *counts.entry(category_key(finding.category)).or_insert(0) += 1;
    }
    counts
}

pub(super) fn count_by_severity(findings: &[KernelSentinelFinding]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for finding in findings {
        *counts.entry(severity_key(finding.severity)).or_insert(0) += 1;
    }
    counts
}

pub(super) fn critical_open_count(findings: &[KernelSentinelFinding]) -> usize {
    findings
        .iter()
        .filter(|finding| {
            finding.severity == KernelSentinelSeverity::Critical
                && normalize_finding_status(&finding.status) == "open"
        })
        .count()
}

pub(super) fn release_blockers(
    critical_open_count: usize,
    malformed_count: usize,
    release_gate_pass: bool,
) -> Vec<&'static str> {
    let mut blockers = Vec::new();
    if critical_open_count > 0 {
        blockers.push("critical_open_findings");
    }
    if malformed_count > 0 {
        blockers.push("malformed_findings");
    }
    if !release_gate_pass {
        blockers.push("release_gate_failed");
    }
    blockers
}

pub(super) fn count_malformed_by_source_kind(records: &[Value]) -> BTreeMap<String, usize> {
    count_json_rows(records, |record| {
        string_field(record, "source_kind")
            .or_else(|| string_field(record, "source").map(|source| format!("evidence:{source}")))
            .unwrap_or_else(|| "unknown".to_string())
    })
}

pub(super) fn count_malformed_by_source(records: &[Value]) -> BTreeMap<String, usize> {
    count_json_rows(records, |record| {
        string_field(record, "source_path")
            .or_else(|| string_field(record, "path"))
            .or_else(|| string_field(record, "source"))
            .unwrap_or_else(|| "unknown".to_string())
    })
}

fn category_key(category: KernelSentinelFindingCategory) -> String {
    serialized_key(category)
}

fn severity_key(severity: KernelSentinelSeverity) -> String {
    serialized_key(severity)
}

fn serialized_key<T>(value: T) -> String
where
    T: Serialize + std::fmt::Debug,
{
    serde_json::to_value(&value)
        .ok()
        .and_then(|json| json.as_str().map(str::to_owned))
        .unwrap_or_else(|| format!("{value:?}").to_ascii_lowercase())
}

fn count_json_rows<F>(records: &[Value], key: F) -> BTreeMap<String, usize>
where
    F: Fn(&Value) -> String,
{
    let mut counts = BTreeMap::new();
    for record in records {
        *counts.entry(key(record)).or_insert(0) += 1;
    }
    counts
}

fn string_field(record: &Value, key: &str) -> Option<String> {
    record
        .get(key)
        .and_then(Value::as_str)
        .filter(|raw| !raw.trim().is_empty())
        .map(str::to_string)
}
