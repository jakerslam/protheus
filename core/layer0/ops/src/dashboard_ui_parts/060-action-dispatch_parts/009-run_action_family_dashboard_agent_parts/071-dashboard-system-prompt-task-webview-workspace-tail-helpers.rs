fn dashboard_prompt_task_hook_execution_type_describe(payload: &Value) -> Value {
    let hook_mode = clean_text(
        payload
            .get("hook_mode")
            .and_then(Value::as_str)
            .unwrap_or("pre_tool_use"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_hook_execution_type_describe",
        "hook_mode": hook_mode
    })
}

fn dashboard_prompt_task_utils_describe(payload: &Value) -> Value {
    let utility = clean_text(
        payload
            .get("utility")
            .and_then(Value::as_str)
            .unwrap_or("task_utils"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_utils_describe",
        "utility": utility
    })
}

fn dashboard_prompt_task_build_user_feedback_content_describe(payload: &Value) -> Value {
    let feedback_profile = clean_text(
        payload
            .get("feedback_profile")
            .and_then(Value::as_str)
            .unwrap_or("concise"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_build_user_feedback_content_describe",
        "feedback_profile": feedback_profile
    })
}

fn dashboard_prompt_task_extract_user_prompt_from_content_describe(payload: &Value) -> Value {
    let extraction_mode = clean_text(
        payload
            .get("extraction_mode")
            .and_then(Value::as_str)
            .unwrap_or("strict"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_extract_user_prompt_from_content_describe",
        "extraction_mode": extraction_mode
    })
}

fn dashboard_prompt_webview_provider_describe(payload: &Value) -> Value {
    let provider_mode = clean_text(
        payload
            .get("provider_mode")
            .and_then(Value::as_str)
            .unwrap_or("embedded"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_webview_provider_describe",
        "provider_mode": provider_mode
    })
}

fn dashboard_prompt_webview_nonce_describe(payload: &Value) -> Value {
    let nonce_policy = clean_text(
        payload
            .get("nonce_policy")
            .and_then(Value::as_str)
            .unwrap_or("per_render"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_webview_nonce_describe",
        "nonce_policy": nonce_policy
    })
}

fn dashboard_prompt_webview_index_describe(payload: &Value) -> Value {
    let index_scope = clean_text(
        payload
            .get("index_scope")
            .and_then(Value::as_str)
            .unwrap_or("webview"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_webview_index_describe",
        "index_scope": index_scope
    })
}

fn dashboard_prompt_workspace_migration_reporter_describe(payload: &Value) -> Value {
    let report_mode = clean_text(
        payload
            .get("report_mode")
            .and_then(Value::as_str)
            .unwrap_or("summary"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_workspace_migration_reporter_describe",
        "report_mode": report_mode
    })
}

fn dashboard_prompt_workspace_path_adapter_describe(payload: &Value) -> Value {
    let adapter_mode = clean_text(
        payload
            .get("adapter_mode")
            .and_then(Value::as_str)
            .unwrap_or("workspace_relative"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_workspace_path_adapter_describe",
        "adapter_mode": adapter_mode
    })
}

fn dashboard_prompt_workspace_resolver_describe(payload: &Value) -> Value {
    let resolver_mode = clean_text(
        payload
            .get("resolver_mode")
            .and_then(Value::as_str)
            .unwrap_or("strict"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_workspace_resolver_describe",
        "resolver_mode": resolver_mode
    })
}

fn dashboard_prompt_task_webview_workspace_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.task.types.hookExecution.describe" => {
            Some(dashboard_prompt_task_hook_execution_type_describe(payload))
        }
        "dashboard.prompts.system.task.utils.describe" => {
            Some(dashboard_prompt_task_utils_describe(payload))
        }
        "dashboard.prompts.system.task.utils.buildUserFeedbackContent.describe" => {
            Some(dashboard_prompt_task_build_user_feedback_content_describe(payload))
        }
        "dashboard.prompts.system.task.utils.extractUserPromptFromContent.describe" => {
            Some(dashboard_prompt_task_extract_user_prompt_from_content_describe(payload))
        }
        "dashboard.prompts.system.webview.webviewProvider.describe" => {
            Some(dashboard_prompt_webview_provider_describe(payload))
        }
        "dashboard.prompts.system.webview.getNonce.describe" => {
            Some(dashboard_prompt_webview_nonce_describe(payload))
        }
        "dashboard.prompts.system.webview.index.describe" => {
            Some(dashboard_prompt_webview_index_describe(payload))
        }
        "dashboard.prompts.system.workspace.migrationReporter.describe" => {
            Some(dashboard_prompt_workspace_migration_reporter_describe(payload))
        }
        "dashboard.prompts.system.workspace.workspacePathAdapter.describe" => {
            Some(dashboard_prompt_workspace_path_adapter_describe(payload))
        }
        "dashboard.prompts.system.workspace.workspaceResolver.describe" => {
            Some(dashboard_prompt_workspace_resolver_describe(payload))
        }
        _ => dashboard_prompt_workspace_extension_hosts_tail_route_extension(root, normalized, payload),
    }
}

include!("072-dashboard-system-prompt-workspace-extension-hosts-tail-helpers.rs");
