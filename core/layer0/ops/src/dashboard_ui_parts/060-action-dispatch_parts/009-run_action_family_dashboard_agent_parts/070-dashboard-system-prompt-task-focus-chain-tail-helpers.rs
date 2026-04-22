fn dashboard_prompt_task_focus_chain_file_utils_describe(payload: &Value) -> Value {
    let utility = clean_text(
        payload
            .get("utility")
            .and_then(Value::as_str)
            .unwrap_or("path_normalize"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_focus_chain_file_utils_describe",
        "utility": utility
    })
}

fn dashboard_prompt_task_focus_chain_index_describe(payload: &Value) -> Value {
    let index_scope = clean_text(
        payload
            .get("index_scope")
            .and_then(Value::as_str)
            .unwrap_or("focus_chain"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_focus_chain_index_describe",
        "index_scope": index_scope
    })
}

fn dashboard_prompt_task_focus_chain_prompts_describe(payload: &Value) -> Value {
    let prompt_profile = clean_text(
        payload
            .get("prompt_profile")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_focus_chain_prompts_describe",
        "prompt_profile": prompt_profile
    })
}

fn dashboard_prompt_task_focus_chain_utils_describe(payload: &Value) -> Value {
    let helper = clean_text(
        payload
            .get("helper")
            .and_then(Value::as_str)
            .unwrap_or("focus_chain_utils"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_focus_chain_utils_describe",
        "helper": helper
    })
}

fn dashboard_prompt_task_index_describe(payload: &Value) -> Value {
    let index_scope = clean_text(
        payload
            .get("index_scope")
            .and_then(Value::as_str)
            .unwrap_or("task"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_index_describe",
        "index_scope": index_scope
    })
}

fn dashboard_prompt_task_latency_describe(payload: &Value) -> Value {
    let latency_profile = clean_text(
        payload
            .get("latency_profile")
            .and_then(Value::as_str)
            .unwrap_or("balanced"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_latency_describe",
        "latency_profile": latency_profile
    })
}

fn dashboard_prompt_task_loop_detection_describe(payload: &Value) -> Value {
    let detection_mode = clean_text(
        payload
            .get("detection_mode")
            .and_then(Value::as_str)
            .unwrap_or("strict"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_loop_detection_describe",
        "detection_mode": detection_mode
    })
}

fn dashboard_prompt_task_message_state_describe(payload: &Value) -> Value {
    let message_state = clean_text(
        payload
            .get("message_state")
            .and_then(Value::as_str)
            .unwrap_or("active"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_message_state_describe",
        "message_state": message_state
    })
}

fn dashboard_prompt_task_multifile_diff_describe(payload: &Value) -> Value {
    let diff_mode = clean_text(
        payload
            .get("diff_mode")
            .and_then(Value::as_str)
            .unwrap_or("summary"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_multifile_diff_describe",
        "diff_mode": diff_mode
    })
}

fn dashboard_prompt_task_presentation_types_describe(payload: &Value) -> Value {
    let presentation_type = clean_text(
        payload
            .get("presentation_type")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_presentation_types_describe",
        "presentation_type": presentation_type
    })
}

fn dashboard_prompt_task_focus_chain_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.task.focusChain.fileUtils.describe" => {
            Some(dashboard_prompt_task_focus_chain_file_utils_describe(payload))
        }
        "dashboard.prompts.system.task.focusChain.index.describe" => {
            Some(dashboard_prompt_task_focus_chain_index_describe(payload))
        }
        "dashboard.prompts.system.task.focusChain.prompts.describe" => {
            Some(dashboard_prompt_task_focus_chain_prompts_describe(payload))
        }
        "dashboard.prompts.system.task.focusChain.utils.describe" => {
            Some(dashboard_prompt_task_focus_chain_utils_describe(payload))
        }
        "dashboard.prompts.system.task.index.describe" => Some(dashboard_prompt_task_index_describe(payload)),
        "dashboard.prompts.system.task.latency.describe" => {
            Some(dashboard_prompt_task_latency_describe(payload))
        }
        "dashboard.prompts.system.task.loopDetection.describe" => {
            Some(dashboard_prompt_task_loop_detection_describe(payload))
        }
        "dashboard.prompts.system.task.messageState.describe" => {
            Some(dashboard_prompt_task_message_state_describe(payload))
        }
        "dashboard.prompts.system.task.multifileDiff.describe" => {
            Some(dashboard_prompt_task_multifile_diff_describe(payload))
        }
        "dashboard.prompts.system.task.presentationTypes.describe" => {
            Some(dashboard_prompt_task_presentation_types_describe(payload))
        }
        _ => dashboard_prompt_task_webview_workspace_tail_route_extension(root, normalized, payload),
    }
}

include!("071-dashboard-system-prompt-task-webview-workspace-tail-helpers.rs");
