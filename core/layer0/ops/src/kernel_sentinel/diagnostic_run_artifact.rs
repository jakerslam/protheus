// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};

pub const KERNEL_SENTINEL_DIAGNOSTIC_RUN_ARTIFACT_NAME: &str =
    "kernel_sentinel_diagnostic_run_current.json";

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| row.as_str().map(str::to_string))
        .collect()
}

fn intersects(left: &[String], right: &[String]) -> bool {
    left.iter().any(|lhs| right.iter().any(|rhs| rhs == lhs))
}

pub fn build_kernel_sentinel_diagnostic_run_artifact(report: &Value) -> Value {
    let probe_requests = report["architectural_incident_report"]["incidents"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|incident| {
            let request = incident.get("diagnostic_follow_up_request")?;
            Some(json!({
                "incident_cluster_key": incident.get("cluster_key").cloned().unwrap_or(Value::Null),
                "root_frame": incident.get("root_frame").cloned().or_else(|| incident.get("likely_root_frame").cloned()).unwrap_or(Value::Null),
                "failure_level": incident.get("failure_level").cloned().unwrap_or(Value::Null),
                "violated_invariants": incident.get("violated_invariants").cloned().unwrap_or_else(|| json!([])),
                "evidence_refs": incident.get("evidence_refs").cloned().unwrap_or_else(|| json!([])),
                "probe_class": request.get("probe_class").cloned().unwrap_or(Value::Null),
                "failure_signature": request.get("failure_signature").cloned().unwrap_or(Value::Null),
                "selected_probe": request.get("selected_probe").cloned().unwrap_or(Value::Null),
                "expected_confidence_gain": request.get("expected_confidence_gain").cloned().unwrap_or(Value::Null),
                "authorization_state": "authorized"
            }))
        })
        .collect::<Vec<_>>();
    let total_expected_confidence_gain = probe_requests
        .iter()
        .filter_map(|row| row["expected_confidence_gain"].as_f64())
        .sum::<f64>();
    json!({
        "ok": true,
        "type": "kernel_sentinel_diagnostic_run",
        "generated_at": crate::now_iso(),
        "diagnostic_follow_up_request_count": probe_requests.len(),
        "authorized_probe_count": probe_requests.len(),
        "refused_probe_count": 0,
        "total_expected_confidence_gain": total_expected_confidence_gain,
        "recurring_inconclusive_patterns": [],
        "probe_requests": probe_requests
    })
}

pub fn build_kernel_sentinel_diagnostic_report_section(diagnostic_run: &Value) -> Value {
    json!({
        "type": "kernel_sentinel_diagnostic_report_section",
        "probes_run": diagnostic_run["authorized_probe_count"].clone(),
        "probes_refused": diagnostic_run["refused_probe_count"].clone(),
        "confidence_gain_expected_total": diagnostic_run["total_expected_confidence_gain"].clone(),
        "recurring_inconclusive_patterns": diagnostic_run["recurring_inconclusive_patterns"].clone(),
        "probe_requests": diagnostic_run["probe_requests"].clone()
    })
}

pub fn attach_diagnostic_context_to_issue_draft(draft: &Value, diagnostic_run: &Value) -> Value {
    let root_frame = draft["root_frame"].as_str().unwrap_or("");
    let violated_invariants = strings(&draft["violated_invariants"]);
    let matching = diagnostic_run["probe_requests"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|row| {
            let probe_root_frame = row["root_frame"].as_str().unwrap_or("");
            let probe_invariants = strings(&row["violated_invariants"]);
            (!root_frame.is_empty() && probe_root_frame == root_frame)
                || (!violated_invariants.is_empty()
                    && intersects(&violated_invariants, &probe_invariants))
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut enriched = draft.clone();
    enriched["diagnostic_run_artifact_path"] = Value::String(format!(
        "local/state/kernel_sentinel/{KERNEL_SENTINEL_DIAGNOSTIC_RUN_ARTIFACT_NAME}"
    ));
    enriched["diagnostic_evidence"] = Value::Array(matching);
    enriched
}
