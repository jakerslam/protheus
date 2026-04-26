// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use crate::kernel_sentinel::{
    KernelSentinelFinding, KernelSentinelFindingCategory, KernelSentinelSeverity,
    KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
};
use serde_json::{json, Value};

const ORDERED_PHASES: [&str; 9] = [
    "input",
    "routing",
    "capability_check",
    "tool_start",
    "tool_result",
    "state_mutation",
    "receipt",
    "finalization",
    "recovery",
];

#[derive(Debug, Clone)]
struct Phase {
    name: String,
    status: String,
}

fn text(value: &Value, key: &str, fallback: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn evidence(record: &Value, fallback: &str) -> Vec<String> {
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
        .unwrap_or_else(|| vec![fallback.to_string()])
}

fn details(record: &Value) -> &Value {
    record.get("details").unwrap_or(&Value::Null)
}

fn phase_status_ok(status: &str) -> bool {
    matches!(
        status,
        "ok" | "pass" | "passed" | "allow" | "allowed" | "complete" | "completed" | "synthesized" | "skipped"
    )
}

fn normalized_phase_name(raw: &str) -> String {
    let lowered = raw.trim().to_lowercase().replace('-', "_");
    let lowered = match lowered.as_str() {
        "capability" | "precondition" | "preconditions" | "probe" | "probes" => "capability_check",
        "tool" | "tool_call" | "tool_invocation" | "tool_execute" | "tool_execution" => "tool_start",
        "tool_done" | "tool_complete" | "tool_completion" | "tool_observation" | "tool_output" => "tool_result",
        "final" | "response" | "assistant_response" | "llm_final" | "llm_finalization" => "finalization",
        "recover" | "retry" | "fallback" | "escalation" => "recovery",
        _ => lowered.as_str(),
    }
    .to_string();
    if ORDERED_PHASES.contains(&lowered.as_str()) {
        lowered
    } else {
        "unknown".to_string()
    }
}

fn phase_order(name: &str) -> usize {
    ORDERED_PHASES
        .iter()
        .position(|candidate| *candidate == name)
        .unwrap_or(ORDERED_PHASES.len())
}

fn parse_phase(value: &Value) -> Option<Phase> {
    if let Some(name) = value.as_str() {
        return Some(Phase {
            name: normalized_phase_name(name),
            status: "ok".to_string(),
        });
    }
    let name = text(value, "name", "");
    if name.is_empty() {
        return None;
    }
    Some(Phase {
        name: normalized_phase_name(&name),
        status: text(value, "status", "ok").to_lowercase(),
    })
}

fn phases_from_record(record: &Value) -> Vec<Phase> {
    let details = details(record);
    let mut phases = details
        .get("phases")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().filter_map(parse_phase).collect::<Vec<_>>())
        .unwrap_or_default();
    if phases.is_empty() {
        if let Some(rows) = details.get("workflow_phases").and_then(Value::as_array) {
            phases = rows.iter().filter_map(parse_phase).collect::<Vec<_>>();
        }
    }
    if phases.is_empty() {
        let phase = text(details, "phase", "");
        if !phase.is_empty() {
            phases.push(Phase {
                name: normalized_phase_name(&phase),
                status: text(details, "phase_status", "ok").to_lowercase(),
            });
        }
    }
    phases.sort_by_key(|phase| phase_order(&phase.name));
    phases
}

fn first_failed_phase(phases: &[Phase]) -> Option<&Phase> {
    phases.iter().find(|phase| !phase_status_ok(&phase.status))
}

fn compact_timeline(phases: &[Phase]) -> String {
    phases
        .iter()
        .map(|phase| format!("{}:{}", phase.name, phase.status))
        .collect::<Vec<_>>()
        .join(">")
}

fn trajectory_subject(record: &Value) -> String {
    text(record, "subject", "unknown_trajectory")
}

fn trajectory_record(record: &Value, phases: &[Phase], first_failed: Option<&Phase>) -> Value {
    let subject = trajectory_subject(record);
    json!({
        "subject": subject,
        "source": text(record, "source", "unknown_source"),
        "kind": text(record, "kind", "operation_trace"),
        "phase_count": phases.len(),
        "first_failed_phase": first_failed.map(|phase| phase.name.clone()),
        "timeline": compact_timeline(phases),
        "ordered_phase_model": ORDERED_PHASES
    })
}

fn finding_for_failed_trajectory(record: &Value, phase: &Phase, phases: &[Phase]) -> KernelSentinelFinding {
    let subject = trajectory_subject(record);
    let timeline = compact_timeline(phases);
    let mut evidence = evidence(record, &format!("trajectory://{subject}"));
    evidence.push(format!(
        "trajectory://{subject};first_failed_phase={};timeline={timeline}",
        phase.name
    ));
    KernelSentinelFinding {
        schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        id: format!("trajectory:{subject}:{}", phase.name),
        severity: KernelSentinelSeverity::High,
        category: KernelSentinelFindingCategory::RuntimeCorrectness,
        fingerprint: format!("trajectory:{}:{}", subject, phase.name),
        evidence,
        summary: format!("{subject} first failed during `{}` phase", phase.name),
        recommended_action: "replay the ordered trajectory and repair the first failed phase before retrying".to_string(),
        status: "open".to_string(),
    }
}

pub fn build_trajectory_report(records: &[Value]) -> (Value, Vec<KernelSentinelFinding>) {
    let mut trajectories = Vec::new();
    let mut findings = Vec::new();
    for record in records {
        let phases = phases_from_record(record);
        if phases.is_empty() {
            continue;
        }
        let first_failed = first_failed_phase(&phases);
        trajectories.push(trajectory_record(record, &phases, first_failed));
        if let Some(phase) = first_failed {
            findings.push(finding_for_failed_trajectory(record, phase, &phases));
        }
    }
    let failed_trajectory_count = findings.len();
    (
        json!({
            "ok": failed_trajectory_count == 0,
            "type": "kernel_sentinel_trajectory_report",
            "ordered_phase_model": ORDERED_PHASES,
            "trajectory_count": trajectories.len(),
            "failed_trajectory_count": failed_trajectory_count,
            "trajectories": trajectories
        }),
        findings,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_failed_phase_becomes_trajectory_finding() {
        let records = vec![json!({
            "source": "runtime_observation",
            "subject": "turn-7",
            "kind": "operation_trace",
            "evidence": ["trace://turn-7"],
            "details": {
                "phases": [
                    {"name": "input", "status": "ok"},
                    {"name": "routing", "status": "ok"},
                    {"name": "tool_result", "status": "failed"},
                    {"name": "finalization", "status": "skipped"}
                ]
            }
        })];
        let (report, findings) = build_trajectory_report(&records);
        assert_eq!(report["failed_trajectory_count"], Value::from(1));
        assert_eq!(findings[0].fingerprint, "trajectory:turn-7:tool_result");
        assert!(findings[0].evidence.iter().any(|row| row.contains("first_failed_phase=tool_result")));
    }

    #[test]
    fn successful_trajectory_is_reported_without_finding() {
        let records = vec![json!({
            "subject": "turn-8",
            "details": {"phases": [
                {"name": "input", "status": "ok"},
                {"name": "routing", "status": "ok"},
                {"name": "finalization", "status": "complete"}
            ]}
        })];
        let (report, findings) = build_trajectory_report(&records);
        assert_eq!(report["trajectory_count"], Value::from(1));
        assert_eq!(findings.len(), 0);
    }
}
