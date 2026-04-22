fn dashboard_prompt_task_focus_chain_file_utils_inspect(payload: &Value) -> Value {
    let files = payload
        .get("files")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|raw| clean_text(raw, 240)))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    let mut ext_counts = std::collections::BTreeMap::<String, i64>::new();
    for file in &files {
        let ext = file
            .rsplit_once('.')
            .map(|(_, e)| e.to_ascii_lowercase())
            .unwrap_or_else(|| "no_ext".to_string());
        *ext_counts.entry(ext).or_insert(0) += 1;
    }
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_focus_chain_file_utils_inspect",
        "file_count": files.len() as i64,
        "extension_counts": ext_counts
    })
}

fn dashboard_prompt_task_focus_chain_index_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_focus_chain_index_describe",
        "modules": [
            "file_utils",
            "prompts",
            "utils",
            "index"
        ],
        "contracts": [
            "normalize_focus_chain",
            "compose_focus_chain_prompt",
            "inspect_focus_chain_files"
        ]
    })
}

fn dashboard_prompt_task_focus_chain_prompts_compose(payload: &Value) -> Value {
    let objective = clean_text(
        payload
            .get("objective")
            .and_then(Value::as_str)
            .unwrap_or("Maintain deterministic tool routing."),
        600,
    );
    let constraints = payload
        .get("constraints")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|raw| clean_text(raw, 180)))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    let mut lines = vec![format!("Objective: {objective}")];
    if constraints.is_empty() {
        lines.push("Constraints: fail closed; preserve authority boundaries.".to_string());
    } else {
        lines.push(format!("Constraints: {}", constraints.join(" | ")));
    }
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_focus_chain_prompts_compose",
        "prompt_text": lines.join("\n")
    })
}

fn dashboard_prompt_task_focus_chain_utils_normalize(payload: &Value) -> Value {
    let mut chain = payload
        .get("chain")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|raw| clean_text(raw, 120).to_ascii_lowercase()))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    chain.sort();
    chain.dedup();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_focus_chain_utils_normalize",
        "normalized_chain": chain
    })
}

fn dashboard_prompt_task_index() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_index",
        "surfaces": [
            "focus_chain",
            "latency",
            "loop_detection",
            "message_state",
            "multifile_diff",
            "presentation_types",
            "stream",
            "tool_executor"
        ]
    })
}

fn dashboard_prompt_task_latency_estimate(payload: &Value) -> Value {
    let model_ms = payload
        .get("model_ms")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let tool_ms = payload
        .get("tool_ms")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let steps = payload
        .get("steps")
        .and_then(Value::as_i64)
        .unwrap_or(1)
        .clamp(1, 200);
    let estimated_total_ms = steps.saturating_mul(model_ms.saturating_add(tool_ms));
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_latency_estimate",
        "steps": steps,
        "estimated_total_ms": estimated_total_ms,
        "estimated_step_ms": model_ms.saturating_add(tool_ms)
    })
}

fn dashboard_prompt_task_loop_detection_analyze(payload: &Value) -> Value {
    let sequence = payload
        .get("sequence")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|raw| clean_text(raw, 120)))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    let mut repeated_edges = 0_i64;
    for pair in sequence.windows(2) {
        if pair[0] == pair[1] {
            repeated_edges = repeated_edges.saturating_add(1);
        }
    }
    let has_loop_signal = repeated_edges > 0;
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_loop_detection_analyze",
        "sequence_length": sequence.len() as i64,
        "repeated_edges": repeated_edges,
        "has_loop_signal": has_loop_signal
    })
}

fn dashboard_prompt_task_message_state_snapshot(root: &Path) -> Value {
    let state = dashboard_lpp_read_state(root);
    let message_count = state
        .get("messages")
        .and_then(Value::as_array)
        .map(|rows| rows.len() as i64)
        .unwrap_or(0);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_message_state_snapshot",
        "message_count": message_count,
        "state": state
    })
}

fn dashboard_prompt_task_multifile_diff_plan(payload: &Value) -> Value {
    let rows = payload
        .get("files")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let plan = rows
        .into_iter()
        .enumerate()
        .map(|(idx, row)| {
            let path = clean_text(row.get("path").and_then(Value::as_str).unwrap_or(""), 240);
            let change = clean_text(
                row.get("change").and_then(Value::as_str).unwrap_or("modify"),
                80,
            );
            json!({
                "order": (idx as i64) + 1,
                "path": path,
                "change": change
            })
        })
        .collect::<Vec<_>>();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_multifile_diff_plan",
        "plan": plan,
        "count": plan.len() as i64
    })
}

fn dashboard_prompt_task_presentation_types_describe(payload: &Value) -> Value {
    let requested = clean_text(
        payload
            .get("presentation_type")
            .and_then(Value::as_str)
            .unwrap_or("summary"),
        80,
    )
    .to_ascii_lowercase();
    let known = ["summary", "timeline", "diff", "checklist"];
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_presentation_types_describe",
        "requested": requested,
        "known_types": known,
        "known": known.contains(&requested.as_str())
    })
}

fn dashboard_prompt_task_focus_chain_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.task.focusChain.fileUtils.inspect" => {
            Some(dashboard_prompt_task_focus_chain_file_utils_inspect(payload))
        }
        "dashboard.prompts.system.task.focusChain.index.describe" => {
            Some(dashboard_prompt_task_focus_chain_index_describe())
        }
        "dashboard.prompts.system.task.focusChain.prompts.compose" => {
            Some(dashboard_prompt_task_focus_chain_prompts_compose(payload))
        }
        "dashboard.prompts.system.task.focusChain.utils.normalize" => {
            Some(dashboard_prompt_task_focus_chain_utils_normalize(payload))
        }
        "dashboard.prompts.system.task.index" => Some(dashboard_prompt_task_index()),
        "dashboard.prompts.system.task.latency.estimate" => {
            Some(dashboard_prompt_task_latency_estimate(payload))
        }
        "dashboard.prompts.system.task.loopDetection.analyze" => {
            Some(dashboard_prompt_task_loop_detection_analyze(payload))
        }
        "dashboard.prompts.system.task.messageState.snapshot" => {
            Some(dashboard_prompt_task_message_state_snapshot(root))
        }
        "dashboard.prompts.system.task.multifileDiff.plan" => {
            Some(dashboard_prompt_task_multifile_diff_plan(payload))
        }
        "dashboard.prompts.system.task.presentationTypes.describe" => {
            Some(dashboard_prompt_task_presentation_types_describe(payload))
        }
        _ => dashboard_prompt_task_webview_workspace_route_extension(root, normalized, payload),
    }
}

include!("024-dashboard-system-prompt-task-webview-workspace-helpers.rs");
