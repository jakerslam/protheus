fn workflow_gate_stability_contract(workflow: &Value) -> Value {
    workflow
        .pointer("/selected_workflow/tool_menu_interface_contract/gate_stability_contract")
        .filter(|value| value.is_object())
        .cloned()
        .or_else(|| {
            default_workflow_tool_menu_contract()
                .get("gate_stability_contract")
                .filter(|value| value.is_object())
                .cloned()
        })
        .unwrap_or_else(|| {
            json!({
                "version": "gate_stability_v1",
                "event_stream": "local/state/ops/workflow_gate_stability/events.jsonl",
                "latest_summary": "local/state/ops/workflow_gate_stability/latest.json",
                "versions_ring": "local/state/ops/workflow_gate_stability/versions_ring.json",
                "workflow_snapshots_dir": "local/state/ops/workflow_gate_stability/workflow_versions",
                "versions_ring_size": 3,
                "tracked_gates": [
                    "gate_1_work_category_menu",
                    "gate_2_tool_family_menu",
                    "gate_3_tool_menu",
                    "gate_4_request_payload_input",
                    "gate_5_post_tool_menu",
                    "gate_6_llm_final_output"
                ],
                "required_artifacts": {},
                "failure_classes": [
                    "empty_response",
                    "invalid_artifact",
                    "missing_transition",
                    "retry_exhausted",
                    "not_applicable"
                ]
            })
        })
}

fn workflow_stage_status_for_gate(workflow: &Value, gate: &str) -> String {
    workflow
        .get("stage_statuses")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter().find_map(|row| {
                (row.get("stage").and_then(Value::as_str) == Some(gate)).then(|| {
                    clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 120)
                })
            })
        })
        .unwrap_or_default()
}

fn workflow_gate_required_artifact_present(workflow: &Value, artifact: &str) -> bool {
    match clean_text(artifact, 120).as_str() {
        "workflow_category_or_direct_final_answer" => {
            workflow
                .pointer("/tool_gate/selected_work_category")
                .and_then(Value::as_str)
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false)
                || workflow
                    .get("response")
                    .and_then(Value::as_str)
                    .map(|value| !value.trim().is_empty())
                    .unwrap_or(false)
                || workflow
                    .pointer("/workflow_control/direct_response_path")
                    .and_then(Value::as_str)
                    .map(|value| value != "first_gate_unresolved")
                    .unwrap_or(false)
        }
        "tool_request" => workflow
            .get("manual_toolbox_pending_tool_request")
            .filter(|value| value.is_object())
            .is_some(),
        "tool_family_selection" => workflow
            .pointer("/tool_gate/selected_tool_family")
            .and_then(Value::as_str)
            .or_else(|| {
                workflow
                    .pointer("/manual_toolbox_pending_tool_request/selected_tool_family")
                    .and_then(Value::as_str)
            })
            .map(|value| !value.trim().is_empty() && value != "none")
            .unwrap_or(false),
        "tool_selection" => workflow
            .pointer("/tool_gate/selected_tool")
            .and_then(Value::as_str)
            .or_else(|| {
                workflow
                    .pointer("/tool_gate/selected_tool_label")
                    .and_then(Value::as_str)
            })
            .or_else(|| {
                workflow
                    .pointer("/manual_toolbox_pending_tool_request/tool_name")
                    .and_then(Value::as_str)
            })
            .or_else(|| {
                workflow
                    .pointer("/manual_toolbox_pending_tool_request/selected_tool_label")
                    .and_then(Value::as_str)
            })
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false),
        "request_payload" => {
            workflow
                .pointer("/tool_gate/request_payload")
                .filter(|value| value.is_object())
                .is_some()
                || workflow
                    .pointer("/manual_toolbox_pending_tool_request/input")
                    .filter(|value| value.is_object())
                    .is_some()
        }
        "tool_result" => {
            workflow
                .get("tool_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                > 0
        }
        "final_response" => {
            let has_visible_response = workflow
                .get("response")
                .and_then(Value::as_str)
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false);
            let final_status_is_user_facing = workflow
                .pointer("/final_llm_response/status")
                .and_then(Value::as_str)
                .map(|status| {
                    matches!(
                        status,
                        "synthesized"
                            | "direct_llm_response"
                            | "no_post_synthesis_required"
                            | "skipped_not_required"
                    )
                })
                .unwrap_or(false);
            let final_used = workflow
                .pointer("/final_llm_response/used")
                .and_then(Value::as_bool)
                .unwrap_or(final_status_is_user_facing);
            has_visible_response && final_status_is_user_facing && final_used
        }
        _ => false,
    }
}

fn workflow_gate_is_applicable(workflow: &Value, gate: &str) -> bool {
    let direct_path = workflow
        .pointer("/workflow_control/direct_response_path")
        .and_then(Value::as_str)
        .unwrap_or("");
    let selected_category = workflow
        .pointer("/tool_gate/selected_work_category")
        .and_then(Value::as_str)
        .map(|value| normalized_workflow_token(value))
        .unwrap_or_default();
    let selected_category_uses_tools = !selected_category.is_empty()
        && !matches!(
            selected_category.as_str(),
            "respond directly"
                | "respond_directly"
                | "planning current context"
                | "planning_current_context"
        );
    match gate {
        "gate_2_tool_family_menu" | "gate_3_tool_menu" | "gate_4_request_payload_input" => {
            selected_category_uses_tools
                || workflow
                    .get("manual_toolbox_pending_tool_request")
                    .filter(|value| value.is_object())
                    .is_some()
                || direct_path.contains("gate_2")
                || direct_path.contains("gate_3")
                || direct_path.contains("gate_4")
                || direct_path == "first_gate_pending_tool_confirmation"
        }
        "gate_5_post_tool_menu" => {
            workflow
                .get("tool_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                > 0
        }
        "gate_6_llm_final_output" => {
            workflow
                .pointer("/final_llm_response/required")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || workflow
                    .get("response")
                    .and_then(Value::as_str)
                    .map(|value| !value.trim().is_empty())
                    .unwrap_or(false)
        }
        _ => true,
    }
}

fn workflow_gate_stability_rows(workflow: &Value) -> Vec<Value> {
    let contract = workflow_gate_stability_contract(workflow);
    let required = contract
        .get("required_artifacts")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    contract
        .get("tracked_gates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|gate_value| {
            let gate = clean_text(gate_value.as_str().unwrap_or(""), 120);
            if gate.is_empty() {
                return None;
            }
            let stage_status = workflow_stage_status_for_gate(workflow, &gate);
            let required_artifacts = required
                .get(&gate)
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|value| value.as_str().map(|raw| clean_text(raw, 120)))
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>();
            let missing_artifacts = required_artifacts
                .iter()
                .filter(|artifact| !workflow_gate_required_artifact_present(workflow, artifact))
                .cloned()
                .collect::<Vec<_>>();
            let applicable = workflow_gate_is_applicable(workflow, &gate);
            let status = if !applicable {
                "not_applicable"
            } else if missing_artifacts.is_empty() {
                "passed"
            } else if stage_status.contains("pending") || stage_status == "presented" {
                "pending"
            } else {
                "failed"
            };
            let failure_class = if status == "failed" {
                if missing_artifacts
                    .iter()
                    .any(|artifact| artifact == "final_response")
                {
                    "empty_response"
                } else {
                    "invalid_artifact"
                }
            } else if status == "not_applicable" {
                "not_applicable"
            } else {
                ""
            };
            Some(json!({
                "gate": gate,
                "status": status,
                "stage_status": stage_status,
                "required_artifacts": required_artifacts,
                "missing_artifacts": missing_artifacts,
                "failure_class": failure_class
            }))
        })
        .collect()
}

fn workflow_gate_stability_summary(rows: &[Value]) -> Value {
    let passed = rows
        .iter()
        .filter(|row| row.get("status").and_then(Value::as_str) == Some("passed"))
        .count();
    let failed = rows
        .iter()
        .filter(|row| row.get("status").and_then(Value::as_str) == Some("failed"))
        .count();
    let observed = passed + failed;
    let success_rate = if observed == 0 {
        1.0
    } else {
        (passed as f64 / observed as f64).clamp(0.0, 1.0)
    };
    json!({
        "passed": passed,
        "failed": failed,
        "observed": observed,
        "success_rate": success_rate
    })
}

fn workflow_gate_stability_latest_rollup(root: &Path, workflow_id: &str) -> Value {
    let path = root.join("local/state/ops/workflow_gate_stability/events.jsonl");
    let mut counts = HashMap::<String, (usize, usize, usize)>::new();
    let events = read_jsonl_loose(&path, 400);
    for event in &events {
        if event.get("workflow_id").and_then(Value::as_str) != Some(workflow_id) {
            continue;
        }
        let gate = clean_text(event.get("gate").and_then(Value::as_str).unwrap_or(""), 120);
        if gate.is_empty() {
            continue;
        }
        let entry = counts.entry(gate).or_insert((0, 0, 0));
        match event.get("status").and_then(Value::as_str).unwrap_or("") {
            "passed" => entry.0 += 1,
            "failed" => entry.1 += 1,
            _ => entry.2 += 1,
        }
    }
    let mut per_gate = workflow_gate_stability_per_gate_rollup_from_counts(counts);
    per_gate.sort_by(|a, b| {
        a.get("gate")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(b.get("gate").and_then(Value::as_str).unwrap_or(""))
    });
    json!({
        "workflow_id": workflow_id,
        "window_event_count": events.len(),
        "per_gate": per_gate
    })
}

fn workflow_gate_stability_current_workflow_snapshot(workflow_id: &str, contract: &Value) -> Value {
    default_workflow_definition()
        .map(|workflow| workflow_definition_to_json(&workflow))
        .unwrap_or_else(|| {
            json!({
                "name": workflow_id,
                "tool_menu_interface_contract": contract
            })
        })
}

fn workflow_gate_stability_version_hash(workflow_id: &str, workflow_snapshot: &Value) -> String {
    crate::deterministic_receipt_hash(&json!({
        "type": "workflow_gate_stability_workflow_version",
        "workflow_id": workflow_id,
        "workflow_json": workflow_snapshot
    }))
}

fn workflow_gate_stability_per_gate_rollup_from_counts(
    counts: HashMap<String, (usize, usize, usize)>,
) -> Vec<Value> {
    let mut per_gate = counts
        .into_iter()
        .map(|(gate, (passed, failed, other))| {
            let observed = passed + failed;
            let success_rate = if observed == 0 {
                1.0
            } else {
                (passed as f64 / observed as f64).clamp(0.0, 1.0)
            };
            json!({
                "gate": gate,
                "passed": passed,
                "failed": failed,
                "other": other,
                "observed": observed,
                "success_rate": success_rate
            })
        })
        .collect::<Vec<_>>();
    per_gate.sort_by(|a, b| {
        a.get("gate")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(b.get("gate").and_then(Value::as_str).unwrap_or(""))
    });
    per_gate
}
