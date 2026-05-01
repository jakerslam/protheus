// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::collections::BTreeMap;

use super::{contains_any, feedback_search_text, string_field, usize_at};

fn symptom_surface_family(item: &Value) -> &'static str {
    let text = feedback_search_text(item);
    if contains_any(
        &text,
        &[
            "shell",
            "chat",
            "browser",
            "dashboard",
            "frontend",
            "svelte",
            "alpine",
            "ui/",
        ],
    ) {
        return "shell_projection_runtime";
    }
    if contains_any(
        &text,
        &[
            "tool",
            "workspace_read",
            "workspace_search",
            "web_search",
            "web_fetch",
            "mcp",
            "route",
        ],
    ) {
        return "tool_routing_runtime";
    }
    if contains_any(
        &text,
        &[
            "eval",
            "validation",
            "benchmark",
            "scorecard",
            "regression",
            "release_gate",
        ],
    ) {
        return "assurance_feedback_runtime";
    }
    if contains_any(&text, &["gateway", "adapter", "ingress", "egress"]) {
        return "gateway_boundary_runtime";
    }
    if contains_any(
        &text,
        &[
            "orchestration",
            "planner",
            "workflow",
            "decomposition",
            "finalization",
        ],
    ) {
        return "orchestration_control_runtime";
    }
    if contains_any(&text, &["kernel", "receipt", "capability", "policy"]) {
        return "kernel_authority_runtime";
    }
    "general_runtime"
}

fn root_cause_symptom_cluster_key(item: &Value) -> String {
    let surface_family = symptom_surface_family(item);
    let root_frame = match surface_family {
        "shell_projection_runtime" => "shell_projection_boundary".to_string(),
        "tool_routing_runtime" => "tool_routing_boundary".to_string(),
        "assurance_feedback_runtime" => "assurance_feedback_loop".to_string(),
        _ => string_field(item, "root_frame"),
    };
    let remediation_level = match surface_family {
        "shell_projection_runtime" => "shell_projection_repair".to_string(),
        "tool_routing_runtime" => "tool_routing_repair".to_string(),
        "assurance_feedback_runtime" => "assurance_feedback_repair".to_string(),
        _ => string_field(item, "remediation_level"),
    };
    format!(
        "surface_family={}|root_frame={}|remediation_level={}",
        surface_family, root_frame, remediation_level
    )
}

fn symptom_member_summary(item: &Value) -> Value {
    json!({
        "dedupe_key": string_field(item, "dedupe_key"),
        "fingerprint": string_field(item, "fingerprint"),
        "severity": string_field(item, "severity"),
        "category": string_field(item, "category"),
        "summary": string_field(item, "summary"),
        "recurrence_count": usize_at(item, &["recurrence_count"])
    })
}

pub(super) fn annotate_root_cause_symptom_clusters(rows: &mut [Value]) {
    let mut indexes_by_key = BTreeMap::<String, Vec<usize>>::new();
    let mut members_by_key = BTreeMap::<String, Vec<Value>>::new();
    for (index, row) in rows.iter().enumerate() {
        let key = root_cause_symptom_cluster_key(row);
        indexes_by_key.entry(key.clone()).or_default().push(index);
        members_by_key
            .entry(key)
            .or_default()
            .push(symptom_member_summary(row));
    }

    for (key, indexes) in indexes_by_key {
        let member_count = indexes.len();
        let repeated = member_count >= 2;
        let members = members_by_key.remove(&key).unwrap_or_default();
        let total_recurrence: usize = indexes
            .iter()
            .map(|index| usize_at(&rows[*index], &["recurrence_count"]))
            .sum();
        for index in indexes {
            let row = &mut rows[index];
            row["root_cause_cluster_key"] = json!(key);
            row["root_cause_cluster_member_count"] = json!(member_count);
            row["root_cause_cluster_repeated"] = json!(repeated);
            row["root_cause_cluster_total_recurrence"] = json!(total_recurrence);
            row["symptom_surface_family"] = json!(symptom_surface_family(row));
            row["root_cause_cluster"] = json!({
                "type": "kernel_sentinel_root_cause_symptom_cluster",
                "key": row["root_cause_cluster_key"].clone(),
                "surface_family": row["symptom_surface_family"].clone(),
                "member_count": member_count,
                "total_recurrence": total_recurrence,
                "repeated": repeated,
                "members": members,
                "policy": "repeated_symptoms_must_be_triaged_as_one_structural_failure_family_before_opening_separate_local_tickets"
            });
            if repeated {
                row["quality_signals"]["root_cause_cluster_repeated"] = json!(true);
                row["todo_actionability"]["root_cause_cluster_ready"] = json!(true);
            }
        }
    }
}
