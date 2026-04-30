// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use crate::kernel_sentinel::{KernelSentinelFindingCategory, KernelSentinelSeverity};
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

pub(super) fn normalize_key(raw: &str) -> String {
    let mut out = String::new();
    let mut previous_lower_or_digit = false;
    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_uppercase() && previous_lower_or_digit && !out.ends_with('_') {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            previous_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        } else if !out.ends_with('_') {
            out.push('_');
            previous_lower_or_digit = false;
        }
    }
    out.trim_matches('_').to_string()
}

fn category_from_str(raw: &str) -> Option<KernelSentinelFindingCategory> {
    match normalize_key(raw).as_str() {
        "receipt_integrity" => Some(KernelSentinelFindingCategory::ReceiptIntegrity),
        "capability_enforcement" => Some(KernelSentinelFindingCategory::CapabilityEnforcement),
        "state_transition" => Some(KernelSentinelFindingCategory::StateTransition),
        "nexus_boundary" => Some(KernelSentinelFindingCategory::NexusBoundary),
        "boundedness" => Some(KernelSentinelFindingCategory::Boundedness),
        "gateway_isolation" => Some(KernelSentinelFindingCategory::GatewayIsolation),
        "queue_backpressure" => Some(KernelSentinelFindingCategory::QueueBackpressure),
        "retry_storm" => Some(KernelSentinelFindingCategory::RetryStorm),
        "release_evidence" => Some(KernelSentinelFindingCategory::ReleaseEvidence),
        "self_maintenance_loop" => Some(KernelSentinelFindingCategory::SelfMaintenanceLoop),
        "security_boundary" => Some(KernelSentinelFindingCategory::SecurityBoundary),
        "runtime_correctness" => Some(KernelSentinelFindingCategory::RuntimeCorrectness),
        "performance_regression" => Some(KernelSentinelFindingCategory::PerformanceRegression),
        "automation_candidate" => Some(KernelSentinelFindingCategory::AutomationCandidate),
        _ => None,
    }
}

fn severity_from_str(raw: &str) -> Option<KernelSentinelSeverity> {
    match normalize_key(raw).as_str() {
        "critical" => Some(KernelSentinelSeverity::Critical),
        "high" => Some(KernelSentinelSeverity::High),
        "medium" => Some(KernelSentinelSeverity::Medium),
        "low" => Some(KernelSentinelSeverity::Low),
        _ => None,
    }
}

pub(super) fn deserialize_optional_category<'de, D>(
    deserializer: D,
) -> Result<Option<KernelSentinelFindingCategory>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(match value {
        Some(Value::String(raw)) => category_from_str(&raw),
        Some(other) => serde_json::from_value(other).ok(),
        None => None,
    })
}

pub(super) fn deserialize_optional_severity<'de, D>(
    deserializer: D,
) -> Result<Option<KernelSentinelSeverity>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(match value {
        Some(Value::String(raw)) => severity_from_str(&raw),
        Some(other) => serde_json::from_value(other).ok(),
        None => None,
    })
}

pub(super) fn option_path(args: &[String], name: &str, fallback: PathBuf) -> PathBuf {
    let prefix = format!("{name}=");
    args.iter()
        .find_map(|arg| arg.strip_prefix(&prefix).map(PathBuf::from))
        .unwrap_or(fallback)
}

pub(super) fn bool_flag(args: &[String], name: &str) -> bool {
    args.iter()
        .any(|arg| arg == name || arg == &format!("{name}=1") || arg == &format!("{name}=true"))
}

pub(super) fn option_u64(args: &[String], name: &str, fallback: u64) -> u64 {
    let prefix = format!("{name}=");
    args.iter()
        .find_map(|arg| arg.strip_prefix(&prefix).and_then(|raw| raw.parse::<u64>().ok()))
        .unwrap_or(fallback)
}

pub(super) fn malformed_count_by_key(records: &[Value], key: &str) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::<String, usize>::new();
    for record in records {
        let value = record
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|raw| !raw.is_empty())
            .unwrap_or("unknown")
            .to_string();
        *out.entry(value).or_insert(0) += 1;
    }
    out
}

pub(super) fn record_freshness_age_seconds(record: &Value) -> Option<u64> {
    let details = record.get("details").unwrap_or(&Value::Null);
    ["freshness_age_seconds", "age_seconds", "source_artifact_age_seconds"]
        .iter()
        .find_map(|key| {
            details
                .get(*key)
                .or_else(|| record.get(*key))
                .and_then(|raw| {
                    raw.as_u64()
                        .or_else(|| raw.as_i64().and_then(|value| u64::try_from(value).ok()))
                        .or_else(|| raw.as_str().and_then(|text| text.trim().parse::<u64>().ok()))
                })
        })
}
