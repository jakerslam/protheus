// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    KernelSentinelFinding, KernelSentinelFindingCategory, KernelSentinelSeverity,
    KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy)]
struct CapabilityRequirement {
    execution_kind: &'static str,
    capability: &'static str,
}

fn capability_requirement(kind: &str) -> Option<CapabilityRequirement> {
    match kind {
        "workspace_read" | "file_read" | "read_file" | "workspace_read_execution" => {
            Some(CapabilityRequirement {
                execution_kind: "workspace_read",
                capability: "workspace_read",
            })
        }
        "workspace_search" | "file_search" | "repo_search" | "workspace_search_execution" => {
            Some(CapabilityRequirement {
                execution_kind: "workspace_search",
                capability: "workspace_search",
            })
        }
        "web_search" | "web_search_execution" => Some(CapabilityRequirement {
            execution_kind: "web_search",
            capability: "web_search",
        }),
        "web_fetch" | "web_fetch_execution" => Some(CapabilityRequirement {
            execution_kind: "web_fetch",
            capability: "web_fetch",
        }),
        "tool_route" | "tool_execution" | "tool_call" | "tool_route_execution" => {
            Some(CapabilityRequirement {
                execution_kind: "tool_route",
                capability: "tool_route",
            })
        }
        "state_mutation" | "state_write" | "mutate_state" | "state_mutation_committed" => {
            Some(CapabilityRequirement {
                execution_kind: "mutate_state",
                capability: "mutate_state",
            })
        }
        _ => None,
    }
}

fn value_str<'a>(value: &'a Value, key: &str) -> &'a str {
    value
        .get(key)
        .or_else(|| value.get("details").and_then(|details| details.get(key)))
        .and_then(Value::as_str)
        .unwrap_or("")
}

fn value_bool(value: &Value, key: &str) -> bool {
    value
        .get(key)
        .or_else(|| value.get("details").and_then(|details| details.get(key)))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn row_contains_token(value: &Value, token: &str) -> bool {
    !token.trim().is_empty()
        && (["id", "subject", "kind", "fingerprint", "capability", "grant_for"]
            .iter()
            .any(|key| value_str(value, key).contains(token))
            || value
                .get("evidence")
                .and_then(Value::as_array)
                .map(|rows| rows.iter().any(|row| row.as_str().unwrap_or("").contains(token)))
                .unwrap_or(false))
}

fn authoritative_grant(record: &Value, requirement: CapabilityRequirement) -> bool {
    if value_str(record, "source") != "kernel_receipt" {
        return false;
    }
    let kind = value_str(record, "kind");
    let capability = value_str(record, "capability");
    let expected_specific = format!("{}_grant", requirement.capability);
    matches!(
        kind,
        "capability_grant" | "probe_grant" | "policy_grant" | "execution_grant"
    ) || kind == expected_specific
        || capability == requirement.capability
}

fn grant_matches_execution(
    grant: &Value,
    execution: &Value,
    requirement: CapabilityRequirement,
) -> bool {
    authoritative_grant(grant, requirement)
        && (row_contains_token(grant, value_str(execution, "id"))
            || row_contains_token(grant, value_str(execution, "subject"))
            || value_str(grant, "grant_for") == requirement.execution_kind)
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

fn missing_grant_finding(
    record: &Value,
    requirement: CapabilityRequirement,
) -> KernelSentinelFinding {
    let action_id = value_str(record, "id");
    let subject = value_str(record, "subject");
    KernelSentinelFinding {
        schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        id: format!("missing_capability_grant:{action_id}"),
        severity: KernelSentinelSeverity::Critical,
        category: KernelSentinelFindingCategory::CapabilityEnforcement,
        fingerprint: format!(
            "capability_grant:{}:{}:{}",
            requirement.execution_kind, subject, requirement.capability
        ),
        evidence: evidence_refs(record),
        summary: format!(
            "{subject} attempted {} without authoritative {} grant",
            requirement.execution_kind, requirement.capability
        ),
        recommended_action: format!(
            "require a kernel_receipt capability grant for {} before executing {}",
            requirement.capability, requirement.execution_kind
        ),
        status: "open".to_string(),
    }
}

fn payload_shortcut_finding(
    record: &Value,
    requirement: CapabilityRequirement,
) -> KernelSentinelFinding {
    let action_id = value_str(record, "id");
    let subject = value_str(record, "subject");
    KernelSentinelFinding {
        schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        id: format!("payload_shortcut:{action_id}"),
        severity: KernelSentinelSeverity::Critical,
        category: KernelSentinelFindingCategory::CapabilityEnforcement,
        fingerprint: format!(
            "capability_payload_shortcut:{}:{}:{}",
            requirement.execution_kind, subject, requirement.capability
        ),
        evidence: evidence_refs(record),
        summary: format!(
            "{subject} used raw payload availability for {} instead of authoritative probe/grant",
            requirement.execution_kind
        ),
        recommended_action: format!(
            "remove payload shortcut and route {} through the kernel capability grant path",
            requirement.execution_kind
        ),
        status: "open".to_string(),
    }
}

pub(super) fn build_capability_grant_report(
    records: &[Value],
) -> (Value, Vec<KernelSentinelFinding>) {
    let grants = records
        .iter()
        .filter(|record| value_str(record, "source") == "kernel_receipt")
        .collect::<Vec<_>>();
    let mut findings = Vec::new();
    let mut checked_execution_count = 0usize;
    let mut payload_shortcut_count = 0usize;
    for record in records {
        let Some(requirement) = capability_requirement(value_str(record, "kind")) else {
            continue;
        };
        if value_str(record, "source") == "kernel_receipt" {
            continue;
        }
        checked_execution_count += 1;
        if value_bool(record, "payload_shortcut")
            || value_bool(record, "transport_available")
            || value_str(record, "grant_source") == "payload"
        {
            payload_shortcut_count += 1;
            findings.push(payload_shortcut_finding(record, requirement));
            continue;
        }
        if !grants
            .iter()
            .any(|grant| grant_matches_execution(grant, record, requirement))
        {
            findings.push(missing_grant_finding(record, requirement));
        }
    }
    let missing_grant_count = findings.len().saturating_sub(payload_shortcut_count);
    let report = json!({
        "ok": findings.is_empty(),
        "checked_capabilities": [
            "workspace_read",
            "workspace_search",
            "web_search",
            "web_fetch",
            "tool_route",
            "mutate_state"
        ],
        "checked_execution_count": checked_execution_count,
        "missing_grant_count": missing_grant_count,
        "payload_shortcut_count": payload_shortcut_count,
        "finding_count": findings.len(),
        "findings": findings
    });
    (report, findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_workspace_search_grant_opens_critical_finding() {
        let records = vec![json!({
            "source": "runtime_observation",
            "id": "exec-1",
            "subject": "workspace-query-1",
            "kind": "workspace_search",
            "evidence": ["trace://workspace-query-1/exec-1"]
        })];
        let (report, findings) = build_capability_grant_report(&records);
        assert_eq!(report["missing_grant_count"], Value::from(1));
        assert_eq!(findings[0].severity, KernelSentinelSeverity::Critical);
        assert_eq!(
            findings[0].fingerprint,
            "capability_grant:workspace_search:workspace-query-1:workspace_search"
        );
    }

    #[test]
    fn matching_kernel_grant_satisfies_execution_requirement() {
        let records = vec![
            json!({
                "source": "runtime_observation",
                "id": "exec-2",
                "subject": "web-query-2",
                "kind": "web_search",
                "evidence": ["trace://web-query-2/exec-2"]
            }),
            json!({
                "source": "kernel_receipt",
                "id": "grant-2",
                "subject": "web-query-2",
                "kind": "capability_grant",
                "capability": "web_search",
                "evidence": ["receipt://web-query-2/exec-2"]
            }),
        ];
        let (report, findings) = build_capability_grant_report(&records);
        assert!(findings.is_empty());
        assert_eq!(report["checked_execution_count"], Value::from(1));
    }

    #[test]
    fn payload_shortcut_is_always_critical_even_with_execution_hint() {
        let records = vec![json!({
            "source": "runtime_observation",
            "id": "exec-3",
            "subject": "tool-route-3",
            "kind": "tool_route",
            "details": {
                "payload_shortcut": true,
                "grant_source": "payload"
            },
            "evidence": ["trace://tool-route-3/exec-3"]
        })];
        let (report, findings) = build_capability_grant_report(&records);
        assert_eq!(report["payload_shortcut_count"], Value::from(1));
        assert_eq!(
            findings[0].fingerprint,
            "capability_payload_shortcut:tool_route:tool-route-3:tool_route"
        );
    }
}
