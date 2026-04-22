fn dashboard_prompt_integrations_editor_comment_review_controller_describe(payload: &Value) -> Value {
    let thread_id = clean_text(
        payload
            .get("thread_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_editor_comment_review_controller_describe",
        "thread_id": thread_id
    })
}

fn dashboard_prompt_integrations_editor_diff_view_provider_describe(payload: &Value) -> Value {
    let file_path = clean_text(payload.get("file_path").and_then(Value::as_str).unwrap_or(""), 1200);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_editor_diff_view_provider_describe",
        "file_path": file_path
    })
}

fn dashboard_prompt_integrations_editor_file_edit_provider_describe(payload: &Value) -> Value {
    let file_path = clean_text(payload.get("file_path").and_then(Value::as_str).unwrap_or(""), 1200);
    let edit_count = payload
        .get("edit_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_editor_file_edit_provider_describe",
        "file_path": file_path,
        "edit_count": edit_count
    })
}

fn dashboard_prompt_integrations_editor_detect_omission_describe(payload: &Value) -> Value {
    let strategy = clean_text(
        payload
            .get("strategy")
            .and_then(Value::as_str)
            .unwrap_or("token_window"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_editor_detect_omission_describe",
        "strategy": strategy
    })
}

fn dashboard_prompt_integrations_misc_export_markdown_describe(payload: &Value) -> Value {
    let destination = clean_text(
        payload
            .get("destination")
            .and_then(Value::as_str)
            .unwrap_or("workspace"),
        240,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_misc_export_markdown_describe",
        "destination": destination
    })
}

fn dashboard_prompt_integrations_misc_extract_file_content_describe(payload: &Value) -> Value {
    let path = clean_text(payload.get("path").and_then(Value::as_str).unwrap_or(""), 1200);
    let max_chars = payload
        .get("max_chars")
        .and_then(Value::as_u64)
        .unwrap_or(4000);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_misc_extract_file_content_describe",
        "path": path,
        "max_chars": max_chars
    })
}

fn dashboard_prompt_integrations_misc_extract_images_describe(payload: &Value) -> Value {
    let path = clean_text(payload.get("path").and_then(Value::as_str).unwrap_or(""), 1200);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_misc_extract_images_describe",
        "path": path
    })
}

fn dashboard_prompt_integrations_misc_extract_text_describe(payload: &Value) -> Value {
    let path = clean_text(payload.get("path").and_then(Value::as_str).unwrap_or(""), 1200);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_misc_extract_text_describe",
        "path": path
    })
}

fn dashboard_prompt_integrations_misc_link_preview_describe(payload: &Value) -> Value {
    let url = clean_text(payload.get("url").and_then(Value::as_str).unwrap_or(""), 1200);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_misc_link_preview_describe",
        "url": url
    })
}

fn dashboard_prompt_integrations_misc_notebook_utils_describe(payload: &Value) -> Value {
    let notebook_path = clean_text(
        payload
            .get("notebook_path")
            .and_then(Value::as_str)
            .unwrap_or(""),
        1200,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_misc_notebook_utils_describe",
        "notebook_path": notebook_path
    })
}

fn dashboard_prompt_integrations_editor_misc_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.integrations.editor.commentReviewController.describe" => {
            Some(dashboard_prompt_integrations_editor_comment_review_controller_describe(payload))
        }
        "dashboard.prompts.system.integrations.editor.diffViewProvider.describe" => {
            Some(dashboard_prompt_integrations_editor_diff_view_provider_describe(payload))
        }
        "dashboard.prompts.system.integrations.editor.fileEditProvider.describe" => {
            Some(dashboard_prompt_integrations_editor_file_edit_provider_describe(payload))
        }
        "dashboard.prompts.system.integrations.editor.detectOmission.describe" => {
            Some(dashboard_prompt_integrations_editor_detect_omission_describe(payload))
        }
        "dashboard.prompts.system.integrations.misc.exportMarkdown.describe" => {
            Some(dashboard_prompt_integrations_misc_export_markdown_describe(payload))
        }
        "dashboard.prompts.system.integrations.misc.extractFileContent.describe" => {
            Some(dashboard_prompt_integrations_misc_extract_file_content_describe(payload))
        }
        "dashboard.prompts.system.integrations.misc.extractImages.describe" => {
            Some(dashboard_prompt_integrations_misc_extract_images_describe(payload))
        }
        "dashboard.prompts.system.integrations.misc.extractText.describe" => {
            Some(dashboard_prompt_integrations_misc_extract_text_describe(payload))
        }
        "dashboard.prompts.system.integrations.misc.linkPreview.describe" => {
            Some(dashboard_prompt_integrations_misc_link_preview_describe(payload))
        }
        "dashboard.prompts.system.integrations.misc.notebookUtils.describe" => {
            Some(dashboard_prompt_integrations_misc_notebook_utils_describe(payload))
        }
        _ => dashboard_prompt_integrations_runtime_terminal_tail_route_extension(root, normalized, payload),
    }
}

include!("035-dashboard-system-prompt-integrations-runtime-terminal-tail-helpers.rs");
