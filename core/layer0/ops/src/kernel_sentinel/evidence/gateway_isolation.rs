// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    KernelSentinelFinding, KernelSentinelFindingCategory, KernelSentinelSeverity,
    KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy)]
struct GatewayRule {
    rule: &'static str,
    severity: KernelSentinelSeverity,
    recommended_action: &'static str,
}

const CHECKED_GATEWAY_RULES: [&str; 6] = [
    "gateway_repeated_failure",
    "gateway_missing_quarantine",
    "gateway_route_around_failure",
    "gateway_recovery_loop",
    "gateway_unsafe_mutation",
    "gateway_isolation_breach",
];

const CHECKED_GATEWAY_OUTCOMES: [&str; 4] = [
    "gateway_quarantine_event",
    "gateway_recovery_event",
    "gateway_route_around",
    "gateway_isolation_breach",
];

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
                .unwrap_or_else(|| matches!(raw.as_str().unwrap_or("").trim().to_lowercase().as_str(), "1" | "true" | "yes" | "fail" | "failed"))
        })
        .unwrap_or(false)
}

fn value_f64(record: &Value, key: &str) -> Option<f64> {
    value(record, key).and_then(|raw| {
        raw.as_f64()
            .or_else(|| raw.as_u64().map(|number| number as f64))
            .or_else(|| raw.as_i64().map(|number| number as f64))
            .or_else(|| raw.as_str().and_then(|text| text.parse::<f64>().ok()))
    })
}

fn normalize_key(raw: &str) -> String {
    raw.trim()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch.to_ascii_lowercase() } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
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

fn row_mentions(record: &Value, token: &str) -> bool {
    !token.trim().is_empty()
        && (["id", "subject", "kind", "gateway_id", "fingerprint"]
            .iter()
            .any(|key| value_str(record, key).contains(token))
            || record
                .get("evidence")
                .and_then(Value::as_array)
                .map(|rows| rows.iter().any(|row| row.as_str().unwrap_or("").contains(token)))
                .unwrap_or(false))
}

fn explicit_rule(record: &Value) -> Option<GatewayRule> {
    for raw in [
        value_str(record, "kind"),
        value_str(record, "signal"),
        value_str(record, "failure_mode"),
        value_str(record, "gateway_rule"),
    ] {
        match raw {
            "gateway_flapping" | "flapping" | "gateway_repeated_failure" | "repeated_gateway_failure" | "repeated_flapping" => {
                return Some(GatewayRule {
                    rule: "gateway_repeated_failure",
                    severity: KernelSentinelSeverity::High,
                    recommended_action:
                        "quarantine the gateway and route around it until recovery evidence is present",
                });
            }
            "gateway_missing_quarantine" | "missing_quarantine" | "quarantine_missing" => {
                return Some(GatewayRule {
                    rule: "gateway_missing_quarantine",
                    severity: KernelSentinelSeverity::Critical,
                    recommended_action:
                        "emit a gateway quarantine receipt and block further routing through this gateway",
                });
            }
            "gateway_route_around_failure" | "route_around_failure" | "routearound_failure" => {
                return Some(GatewayRule {
                    rule: "gateway_route_around_failure",
                    severity: KernelSentinelSeverity::High,
                    recommended_action:
                        "restore route-around policy and prove degraded execution avoids the failed gateway",
                });
            }
            "gateway_recovery_loop" | "gateway_recovery_storm" | "recovery_loop" | "retry_loop" => {
                return Some(GatewayRule {
                    rule: "gateway_recovery_loop",
                    severity: KernelSentinelSeverity::High,
                    recommended_action:
                        "cap recovery attempts and require backoff or human escalation before retrying",
                });
            }
            "gateway_unsafe_mutation" | "unsafe_gateway_mutation" | "gateway_mutated_state" => {
                return Some(GatewayRule {
                    rule: "gateway_unsafe_mutation",
                    severity: KernelSentinelSeverity::Critical,
                    recommended_action:
                        "remove mutation authority from the gateway and restore Kernel-owned receipts",
                });
            }
            "gateway_isolation_breach" | "gateway_boundary_escape" => {
                return Some(GatewayRule {
                    rule: "gateway_isolation_breach",
                    severity: KernelSentinelSeverity::Critical,
                    recommended_action:
                        "isolate the gateway process and enforce timeout/memory/authority boundaries",
                });
            }
            _ => {}
        }
    }
    None
}

fn gateway_outcome_kind(record: &Value) -> &'static str {
    for raw in [
        value_str(record, "kind"),
        value_str(record, "signal"),
        value_str(record, "event"),
        value_str(record, "outcome"),
    ] {
        match normalize_key(raw).as_str() {
            "gateway_quarantine_event" | "gateway_quarantine_receipt" | "gateway_quarantined"
            | "quarantine" | "quarantined" => return "gateway_quarantine_event",
            "gateway_recovery_event" | "gateway_recovered" | "recovery" | "recovered" => {
                return "gateway_recovery_event";
            }
            "gateway_route_around" | "route_around" | "routed_around" => {
                return "gateway_route_around";
            }
            "gateway_isolation_breach" | "gateway_boundary_escape" => {
                return "gateway_isolation_breach";
            }
            _ => {}
        }
    }
    "none"
}

fn has_quarantine_evidence(records: &[Value], record: &Value) -> bool {
    let subject = value_str(record, "subject");
    let gateway_id = value_str(record, "gateway_id");
    records.iter().any(|candidate| {
        matches!(
            value_str(candidate, "kind"),
            "gateway_quarantine_event" | "gateway_quarantine_receipt" | "gateway_quarantined"
        ) && (row_mentions(candidate, subject) || row_mentions(candidate, gateway_id))
    }) || value_bool(record, "quarantined")
        || value_bool(record, "quarantine_event")
        || value_bool(record, "quarantine_receipt")
}

fn metric_rules(records: &[Value], record: &Value) -> Vec<GatewayRule> {
    let mut rules = Vec::new();
    let failures = value_f64(record, "failure_count").or_else(|| value_f64(record, "flap_count"));
    let failure_budget = value_f64(record, "failure_budget")
        .or_else(|| value_f64(record, "failure_limit"))
        .unwrap_or(3.0);
    if failures.map(|count| count > failure_budget).unwrap_or(false) {
        rules.push(GatewayRule {
            rule: "gateway_repeated_failure",
            severity: KernelSentinelSeverity::High,
            recommended_action:
                "quarantine the gateway and route around it until recovery evidence is present",
        });
        if !has_quarantine_evidence(records, record) {
            rules.push(GatewayRule {
                rule: "gateway_missing_quarantine",
                severity: KernelSentinelSeverity::Critical,
                recommended_action:
                    "emit a gateway quarantine receipt and block further routing through this gateway",
            });
        }
    }
    let attempts = value_f64(record, "recovery_attempts");
    let budget = value_f64(record, "recovery_budget").unwrap_or(2.0);
    if attempts.map(|count| count > budget).unwrap_or(false) {
        rules.push(GatewayRule {
            rule: "gateway_recovery_loop",
            severity: KernelSentinelSeverity::High,
            recommended_action:
                "cap recovery attempts and require backoff or human escalation before retrying",
        });
    }
    if value_bool(record, "route_around_failed") {
        rules.push(GatewayRule {
            rule: "gateway_route_around_failure",
            severity: KernelSentinelSeverity::High,
            recommended_action:
                "restore route-around policy and prove degraded execution avoids the failed gateway",
        });
    }
    rules
}

fn gateway_finding(record: &Value, rule: GatewayRule) -> KernelSentinelFinding {
    let action_id = clean_token(value_str(record, "id"), "gateway-observation");
    let subject = clean_token(value_str(record, "subject"), "unknown_gateway");
    KernelSentinelFinding {
        schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        id: format!("gateway_isolation:{}:{action_id}", rule.rule),
        severity: rule.severity,
        category: KernelSentinelFindingCategory::GatewayIsolation,
        fingerprint: format!("gateway_isolation:{}:{}", rule.rule, subject),
        evidence: evidence_refs(record),
        summary: format!("{subject} violated {}", rule.rule),
        recommended_action: rule.recommended_action.to_string(),
        status: "open".to_string(),
    }
}

pub(super) fn build_gateway_isolation_report(
    records: &[Value],
) -> (Value, Vec<KernelSentinelFinding>) {
    let mut findings = Vec::new();
    let mut checked_gateway_count = 0usize;
    let mut missing_quarantine_count = 0usize;
    let mut recovery_loop_count = 0usize;
    let mut quarantine_event_count = 0usize;
    let mut recovery_event_count = 0usize;
    let mut route_around_count = 0usize;
    let mut isolation_breach_count = 0usize;
    for record in records {
        match gateway_outcome_kind(record) {
            "gateway_quarantine_event" => quarantine_event_count += 1,
            "gateway_recovery_event" => recovery_event_count += 1,
            "gateway_route_around" => route_around_count += 1,
            "gateway_isolation_breach" => isolation_breach_count += 1,
            _ => {}
        }
        let mut rules = Vec::new();
        if let Some(rule) = explicit_rule(record) {
            rules.push(rule);
        }
        rules.extend(metric_rules(records, record));
        if rules.is_empty() {
            continue;
        }
        checked_gateway_count += 1;
        for rule in rules {
            if rule.rule == "gateway_missing_quarantine" {
                missing_quarantine_count += 1;
            }
            if rule.rule == "gateway_recovery_loop" {
                recovery_loop_count += 1;
            }
            findings.push(gateway_finding(record, rule));
        }
    }
    let report = json!({
        "ok": findings.is_empty(),
        "checked_gateway_rules": CHECKED_GATEWAY_RULES,
        "checked_gateway_outcomes": CHECKED_GATEWAY_OUTCOMES,
        "checked_gateway_count": checked_gateway_count,
        "missing_quarantine_count": missing_quarantine_count,
        "recovery_loop_count": recovery_loop_count,
        "quarantine_event_count": quarantine_event_count,
        "recovery_event_count": recovery_event_count,
        "route_around_count": route_around_count,
        "isolation_breach_count": isolation_breach_count,
        "finding_count": findings.len(),
        "findings": findings
    });
    (report, findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flapping_gateway_without_quarantine_opens_critical_finding() {
        let records = vec![json!({
            "source": "gateway_health",
            "id": "gw-obs-1",
            "subject": "ollama-gateway",
            "kind": "gateway_health",
            "failure_count": 5,
            "failure_budget": 3,
            "evidence": ["gateway://ollama-gateway/flap"]
        })];
        let (report, findings) = build_gateway_isolation_report(&records);
        assert_eq!(report["missing_quarantine_count"], Value::from(1));
        assert!(findings.iter().any(|finding| {
            finding.severity == KernelSentinelSeverity::Critical
                && finding.fingerprint
                    == "gateway_isolation:gateway_missing_quarantine:ollama-gateway"
        }));
    }

    #[test]
    fn quarantine_receipt_satisfies_repeated_failure_containment() {
        let records = vec![
            json!({
                "source": "gateway_health",
                "id": "gw-obs-2",
                "subject": "mcp-gateway",
                "kind": "gateway_health",
                "failure_count": 5,
                "failure_budget": 3,
                "evidence": ["gateway://mcp-gateway/flap"]
            }),
            json!({
                "source": "kernel_receipt",
                "id": "gw-quarantine-2",
                "subject": "mcp-gateway",
                "kind": "gateway_quarantine_receipt",
                "evidence": ["receipt://mcp-gateway/quarantine"]
            }),
        ];
        let (report, _findings) = build_gateway_isolation_report(&records);
        assert_eq!(report["missing_quarantine_count"], Value::from(0));
    }

    #[test]
    fn gateway_quarantine_and_recovery_outcomes_are_reported() {
        let records = vec![
            json!({
                "source": "gateway_health",
                "id": "gw-quarantine-3",
                "subject": "semantic-kernel-gateway",
                "kind": "gateway_quarantine_event",
                "evidence": ["gateway://semantic-kernel/quarantine"]
            }),
            json!({
                "source": "gateway_health",
                "id": "gw-recovery-3",
                "subject": "semantic-kernel-gateway",
                "kind": "gateway_recovery_event",
                "evidence": ["gateway://semantic-kernel/recovery"]
            }),
        ];
        let (report, findings) = build_gateway_isolation_report(&records);
        assert_eq!(report["quarantine_event_count"], Value::from(1));
        assert_eq!(report["recovery_event_count"], Value::from(1));
        assert!(findings.is_empty());
    }
}
