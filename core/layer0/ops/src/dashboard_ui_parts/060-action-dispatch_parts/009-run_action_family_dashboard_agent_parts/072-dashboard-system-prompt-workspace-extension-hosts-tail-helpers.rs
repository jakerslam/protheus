fn dashboard_prompt_workspace_root_manager_describe(payload: &Value) -> Value {
    let root_mode = clean_text(
        payload
            .get("root_mode")
            .and_then(Value::as_str)
            .unwrap_or("single"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_workspace_root_manager_describe",
        "root_mode": root_mode
    })
}

fn dashboard_prompt_workspace_detection_describe(payload: &Value) -> Value {
    let detection_mode = clean_text(
        payload
            .get("detection_mode")
            .and_then(Value::as_str)
            .unwrap_or("auto"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_workspace_detection_describe",
        "detection_mode": detection_mode
    })
}

fn dashboard_prompt_workspace_index_describe(payload: &Value) -> Value {
    let index_scope = clean_text(
        payload
            .get("index_scope")
            .and_then(Value::as_str)
            .unwrap_or("workspace"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_workspace_index_describe",
        "index_scope": index_scope
    })
}

fn dashboard_prompt_workspace_multi_root_utils_describe(payload: &Value) -> Value {
    let utility = clean_text(
        payload
            .get("utility")
            .and_then(Value::as_str)
            .unwrap_or("normalize"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_workspace_multi_root_utils_describe",
        "utility": utility
    })
}

fn dashboard_prompt_workspace_setup_describe(payload: &Value) -> Value {
    let setup_mode = clean_text(
        payload
            .get("setup_mode")
            .and_then(Value::as_str)
            .unwrap_or("guided"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_workspace_setup_describe",
        "setup_mode": setup_mode
    })
}

fn dashboard_prompt_workspace_parse_inline_path_describe(payload: &Value) -> Value {
    let parse_mode = clean_text(
        payload
            .get("parse_mode")
            .and_then(Value::as_str)
            .unwrap_or("strict"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_workspace_parse_inline_path_describe",
        "parse_mode": parse_mode
    })
}

fn dashboard_prompt_workspace_detection_utils_describe(payload: &Value) -> Value {
    let utility = clean_text(
        payload
            .get("utility")
            .and_then(Value::as_str)
            .unwrap_or("workspace_detection"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_workspace_detection_utils_describe",
        "utility": utility
    })
}

fn dashboard_prompt_extension_describe(payload: &Value) -> Value {
    let extension_mode = clean_text(
        payload
            .get("extension_mode")
            .and_then(Value::as_str)
            .unwrap_or("runtime"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_extension_describe",
        "extension_mode": extension_mode
    })
}

fn dashboard_prompt_hosts_external_auth_handler_describe(payload: &Value) -> Value {
    let auth_mode = clean_text(
        payload
            .get("auth_mode")
            .and_then(Value::as_str)
            .unwrap_or("oauth"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_external_auth_handler_describe",
        "auth_mode": auth_mode
    })
}

fn dashboard_prompt_hosts_external_comment_review_controller_describe(payload: &Value) -> Value {
    let review_mode = clean_text(
        payload
            .get("review_mode")
            .and_then(Value::as_str)
            .unwrap_or("threaded"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_external_comment_review_controller_describe",
        "review_mode": review_mode
    })
}

fn dashboard_prompt_workspace_extension_hosts_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.workspace.workspaceRootManager.describe" => {
            Some(dashboard_prompt_workspace_root_manager_describe(payload))
        }
        "dashboard.prompts.system.workspace.detection.describe" => {
            Some(dashboard_prompt_workspace_detection_describe(payload))
        }
        "dashboard.prompts.system.workspace.index.describe" => {
            Some(dashboard_prompt_workspace_index_describe(payload))
        }
        "dashboard.prompts.system.workspace.multiRootUtils.describe" => {
            Some(dashboard_prompt_workspace_multi_root_utils_describe(payload))
        }
        "dashboard.prompts.system.workspace.setup.describe" => {
            Some(dashboard_prompt_workspace_setup_describe(payload))
        }
        "dashboard.prompts.system.workspace.parseWorkspaceInlinePath.describe" => {
            Some(dashboard_prompt_workspace_parse_inline_path_describe(payload))
        }
        "dashboard.prompts.system.workspace.workspaceDetection.describe" => {
            Some(dashboard_prompt_workspace_detection_utils_describe(payload))
        }
        "dashboard.prompts.system.extension.describe" => {
            Some(dashboard_prompt_extension_describe(payload))
        }
        "dashboard.prompts.system.hosts.external.authHandler.describe" => {
            Some(dashboard_prompt_hosts_external_auth_handler_describe(payload))
        }
        "dashboard.prompts.system.hosts.external.externalCommentReviewController.describe" => {
            Some(dashboard_prompt_hosts_external_comment_review_controller_describe(payload))
        }
        _ => dashboard_prompt_hosts_surface_tail_route_extension(root, normalized, payload),
    }
}

include!("073-dashboard-system-prompt-hosts-surface-tail-helpers.rs");
