fn dashboard_prompt_integrations_editor_comment_review_controller_describe(payload: &Value) -> Value {
    let review_mode = clean_text(
        payload
            .get("review_mode")
            .and_then(Value::as_str)
            .unwrap_or("inline"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_editor_comment_review_controller_describe",
        "review_mode": review_mode
    })
}

fn dashboard_prompt_integrations_editor_diff_view_provider_describe(payload: &Value) -> Value {
    let provider_mode = clean_text(
        payload
            .get("provider_mode")
            .and_then(Value::as_str)
            .unwrap_or("vscode"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_editor_diff_view_provider_describe",
        "provider_mode": provider_mode
    })
}

fn dashboard_prompt_integrations_editor_file_edit_provider_describe(payload: &Value) -> Value {
    let edit_mode = clean_text(
        payload
            .get("edit_mode")
            .and_then(Value::as_str)
            .unwrap_or("patch"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_editor_file_edit_provider_describe",
        "edit_mode": edit_mode
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
    let flavor = clean_text(
        payload
            .get("flavor")
            .and_then(Value::as_str)
            .unwrap_or("gfm"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_misc_export_markdown_describe",
        "flavor": flavor
    })
}

fn dashboard_prompt_integrations_misc_extract_file_content_describe(payload: &Value) -> Value {
    let path = clean_text(
        payload
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or(""),
        320,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_misc_extract_file_content_describe",
        "path": path
    })
}

fn dashboard_prompt_integrations_misc_extract_images_describe(payload: &Value) -> Value {
    let include_metadata = payload
        .get("include_metadata")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_misc_extract_images_describe",
        "include_metadata": include_metadata
    })
}

fn dashboard_prompt_integrations_misc_extract_text_describe(payload: &Value) -> Value {
    let source_type = clean_text(
        payload
            .get("source_type")
            .and_then(Value::as_str)
            .unwrap_or("document"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_misc_extract_text_describe",
        "source_type": source_type
    })
}

fn dashboard_prompt_integrations_misc_link_preview_describe(payload: &Value) -> Value {
    let url = clean_text(
        payload
            .get("url")
            .and_then(Value::as_str)
            .unwrap_or(""),
        320,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_misc_link_preview_describe",
        "url": url
    })
}

fn dashboard_prompt_integrations_misc_notebook_utils_describe(payload: &Value) -> Value {
    let notebook_mode = clean_text(
        payload
            .get("notebook_mode")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_misc_notebook_utils_describe",
        "notebook_mode": notebook_mode
    })
}

fn dashboard_prompt_hosts_surface_tail_integrations_editor_misc_route_extension(
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
        _ => dashboard_prompt_hosts_surface_tail_integrations_runtime_terminal_route_extension(
            root,
            normalized,
            payload,
        ),
    }
}
include!("082-dashboard-system-prompt-integrations-runtime-terminal-tail-helpers.rs");
