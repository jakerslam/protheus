// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    KernelSentinelFinding, KernelSentinelFindingCategory, KernelSentinelSeverity,
    KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy)]
struct SignalRule {
    rule: &'static str,
    category: KernelSentinelFindingCategory,
    severity: KernelSentinelSeverity,
    summary_subject: &'static str,
    recommended_action: &'static str,
}

const CHECKED_THRESHOLDS: [&str; 7] = [
    "rss_ceiling",
    "queue_depth",
    "retry_storm",
    "recovery_time",
    "stale_surface",
    "threshold_regression",
    "backpressure_failure",
];

fn value<'a>(record: &'a Value, key: &str) -> Option<&'a Value> {
    record
        .get(key)
        .or_else(|| record.get("details").and_then(|details| details.get(key)))
}

fn value_str<'a>(record: &'a Value, key: &str) -> &'a str {
    value(record, key).and_then(Value::as_str).unwrap_or("")
}

fn value_bool(record: &Value, key: &str) -> bool {
    value(record, key).and_then(Value::as_bool).unwrap_or(false)
}

fn value_f64(record: &Value, key: &str) -> Option<f64> {
    value(record, key).and_then(|raw| {
        raw.as_f64()
            .or_else(|| raw.as_u64().map(|number| number as f64))
            .or_else(|| raw.as_i64().map(|number| number as f64))
            .or_else(|| raw.as_str().and_then(|text| text.parse::<f64>().ok()))
    })
}

fn first_metric(record: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter().find_map(|key| value_f64(record, key))
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

fn clean_token(raw: &str, fallback: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn explicit_rule(record: &Value) -> Option<SignalRule> {
    let kind = value_str(record, "kind");
    let signal = value_str(record, "signal");
    let failure_mode = value_str(record, "failure_mode");
    for raw in [kind, signal, failure_mode] {
        match raw {
            "retry_storm" | "retry_loop" | "repeated_retry_loop" => {
                return Some(SignalRule {
                    rule: "retry_storm",
                    category: KernelSentinelFindingCategory::RetryStorm,
                    severity: KernelSentinelSeverity::Critical,
                    summary_subject: "retry loop",
                    recommended_action:
                        "shed, defer, or quarantine the loop and require recovery evidence before retrying",
                });
            }
            "queue_backpressure_failure" | "queue_backpressure" | "queue_saturation" => {
                return Some(SignalRule {
                    rule: "queue_depth",
                    category: KernelSentinelFindingCategory::QueueBackpressure,
                    severity: KernelSentinelSeverity::High,
                    summary_subject: "queue backpressure",
                    recommended_action:
                        "apply queue shed/defer/quarantine policy and restore bounded queue depth",
                });
            }
            "boundedness_regression" | "threshold_regression" | "boundedness_failure" => {
                return Some(SignalRule {
                    rule: "threshold_regression",
                    category: KernelSentinelFindingCategory::Boundedness,
                    severity: KernelSentinelSeverity::High,
                    summary_subject: "boundedness threshold",
                    recommended_action:
                        "restore the previous boundedness budget or update the release baseline with proof",
                });
            }
            _ => {}
        }
    }
    None
}

fn metric_rules(record: &Value) -> Vec<SignalRule> {
    let mut rules = Vec::new();
    if let (Some(rss), Some(limit)) = (
        first_metric(record, &["max_rss_mb", "rss_mb", "rss_peak_mb"]),
        first_metric(record, &["rss_ceiling_mb", "rss_budget_mb", "rss_limit_mb"]),
    ) {
        if rss > limit {
            rules.push(SignalRule {
                rule: "rss_ceiling",
                category: KernelSentinelFindingCategory::Boundedness,
                severity: if rss >= limit * 1.5 {
                    KernelSentinelSeverity::Critical
                } else {
                    KernelSentinelSeverity::High
                },
                summary_subject: "RSS boundedness",
                recommended_action:
                    "reduce resident memory growth or raise the budget only with release evidence",
            });
        }
    }
    if let (Some(depth), Some(limit)) = (
        first_metric(record, &["queue_depth_max", "queue_depth_p95", "queue_depth"]),
        first_metric(record, &["queue_depth_limit", "queue_depth_budget", "queue_limit"]),
    ) {
        if depth > limit {
            rules.push(SignalRule {
                rule: "queue_depth",
                category: KernelSentinelFindingCategory::QueueBackpressure,
                severity: if depth >= limit * 2.0 {
                    KernelSentinelSeverity::Critical
                } else {
                    KernelSentinelSeverity::High
                },
                summary_subject: "queue depth",
                recommended_action:
                    "activate backpressure policy and verify shed/defer/quarantine receipts",
            });
        }
    }
    if let (Some(retry_count), Some(limit)) = (
        first_metric(record, &["retry_count", "retry_loop_count", "repeated_retry_count"]),
        first_metric(record, &["retry_budget", "retry_limit", "retry_window_budget"]),
    ) {
        if retry_count > limit {
            rules.push(SignalRule {
                rule: "retry_storm",
                category: KernelSentinelFindingCategory::RetryStorm,
                severity: KernelSentinelSeverity::Critical,
                summary_subject: "retry count",
                recommended_action:
                    "stop the retry loop and require recovery or quarantine evidence before retrying",
            });
        }
    } else if first_metric(record, &["retry_loop_count", "repeated_retry_count"])
        .map(|count| count > 0.0)
        .unwrap_or(false)
    {
        rules.push(SignalRule {
            rule: "retry_storm",
            category: KernelSentinelFindingCategory::RetryStorm,
            severity: KernelSentinelSeverity::Critical,
            summary_subject: "retry loop",
            recommended_action:
                "stop the retry loop and require recovery or quarantine evidence before retrying",
        });
    }
    if let (Some(recovery_ms), Some(limit)) = (
        first_metric(record, &["recovery_time_ms", "recovery_time_p95_ms"]),
        first_metric(record, &["recovery_time_budget_ms", "recovery_time_limit_ms"]),
    ) {
        if recovery_ms > limit {
            rules.push(SignalRule {
                rule: "recovery_time",
                category: KernelSentinelFindingCategory::Boundedness,
                severity: KernelSentinelSeverity::High,
                summary_subject: "recovery time",
                recommended_action: "tighten recovery path or adjust boundedness budget with evidence",
            });
        }
    }
    if let (Some(stale_count), Some(limit)) = (
        first_metric(record, &["stale_surface_count", "stale_surface_incidents"]),
        first_metric(record, &["stale_surface_budget", "stale_surface_limit"]),
    ) {
        if stale_count > limit {
            rules.push(SignalRule {
                rule: "stale_surface",
                category: KernelSentinelFindingCategory::Boundedness,
                severity: KernelSentinelSeverity::High,
                summary_subject: "stale surface",
                recommended_action: "restore freshness propagation and prove dashboard truth parity",
            });
        }
    }
    if value_bool(record, "threshold_regression") || value_bool(record, "regressed") {
        rules.push(SignalRule {
            rule: "threshold_regression",
            category: KernelSentinelFindingCategory::Boundedness,
            severity: KernelSentinelSeverity::High,
            summary_subject: "threshold regression",
            recommended_action:
                "compare against the previous release baseline and restore boundedness budget",
        });
    }
    rules
}

fn boundedness_finding(record: &Value, rule: SignalRule) -> KernelSentinelFinding {
    let action_id = clean_token(value_str(record, "id"), "boundedness-observation");
    let subject = clean_token(value_str(record, "subject"), rule.summary_subject);
    KernelSentinelFinding {
        schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        id: format!("boundedness:{rule_name}:{action_id}", rule_name = rule.rule),
        severity: rule.severity,
        category: rule.category,
        fingerprint: format!("boundedness:{}:{}", rule.rule, subject),
        evidence: evidence_refs(record),
        summary: format!("{subject} exceeded {}", rule.rule),
        recommended_action: rule.recommended_action.to_string(),
        status: "open".to_string(),
    }
}

pub(super) fn build_boundedness_report(
    records: &[Value],
) -> (Value, Vec<KernelSentinelFinding>) {
    let mut findings = Vec::new();
    let mut checked_metric_count = 0usize;
    let mut retry_storm_count = 0usize;
    let mut queue_backpressure_failure_count = 0usize;
    let mut boundedness_failure_count = 0usize;
    for record in records {
        let mut rules = Vec::new();
        if let Some(rule) = explicit_rule(record) {
            rules.push(rule);
        }
        rules.extend(metric_rules(record));
        if rules.is_empty() {
            continue;
        }
        checked_metric_count += 1;
        for rule in rules {
            match rule.category {
                KernelSentinelFindingCategory::RetryStorm => retry_storm_count += 1,
                KernelSentinelFindingCategory::QueueBackpressure => {
                    queue_backpressure_failure_count += 1
                }
                KernelSentinelFindingCategory::Boundedness => boundedness_failure_count += 1,
                _ => {}
            }
            findings.push(boundedness_finding(record, rule));
        }
    }
    let report = json!({
        "ok": findings.is_empty(),
        "checked_thresholds": CHECKED_THRESHOLDS,
        "checked_metric_count": checked_metric_count,
        "retry_storm_count": retry_storm_count,
        "queue_backpressure_failure_count": queue_backpressure_failure_count,
        "boundedness_failure_count": boundedness_failure_count,
        "finding_count": findings.len(),
        "findings": findings
    });
    (report, findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_loop_over_budget_creates_critical_retry_storm_finding() {
        let records = vec![json!({
            "source": "runtime_observation",
            "id": "retry-1",
            "subject": "agent-turn-42",
            "kind": "runtime_metrics",
            "retry_count": 8,
            "retry_budget": 3,
            "evidence": ["trace://agent-turn-42/retry-budget"]
        })];
        let (report, findings) = build_boundedness_report(&records);
        assert_eq!(report["retry_storm_count"], Value::from(1));
        assert_eq!(findings[0].category, KernelSentinelFindingCategory::RetryStorm);
        assert_eq!(findings[0].severity, KernelSentinelSeverity::Critical);
    }

    #[test]
    fn queue_depth_over_limit_creates_backpressure_finding() {
        let records = vec![json!({
            "source": "queue_backpressure",
            "id": "queue-1",
            "subject": "kernel-admission-queue",
            "kind": "queue_metrics",
            "queue_depth_max": 180,
            "queue_depth_limit": 100,
            "evidence": ["queue://kernel-admission/max-depth"]
        })];
        let (report, findings) = build_boundedness_report(&records);
        assert_eq!(report["queue_backpressure_failure_count"], Value::from(1));
        assert_eq!(
            findings[0].category,
            KernelSentinelFindingCategory::QueueBackpressure
        );
    }

    #[test]
    fn threshold_regression_creates_boundedness_finding() {
        let records = vec![json!({
            "source": "release_proof_pack",
            "id": "boundedness-1",
            "subject": "rich-profile",
            "kind": "boundedness_report",
            "max_rss_mb": 900,
            "rss_ceiling_mb": 512,
            "threshold_regression": true,
            "evidence": ["proof-pack://boundedness/rich-profile"]
        })];
        let (report, findings) = build_boundedness_report(&records);
        assert_eq!(report["boundedness_failure_count"], Value::from(2));
        assert!(findings
            .iter()
            .any(|finding| finding.category == KernelSentinelFindingCategory::Boundedness));
    }

    #[test]
    fn unrelated_records_are_ignored() {
        let records = vec![json!({
            "source": "runtime_observation",
            "id": "safe-1",
            "subject": "kernel-idle",
            "kind": "runtime_metrics",
            "queue_depth_max": 10,
            "queue_depth_limit": 100
        })];
        let (report, findings) = build_boundedness_report(&records);
        assert_eq!(report["ok"], Value::Bool(true));
        assert!(findings.is_empty());
    }
}
