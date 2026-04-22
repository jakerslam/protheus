#[test]
fn dashboard_system_prompt_integrations_editor_misc_tail_routes_contract_wave_890() {
    let root = tempfile::tempdir().expect("tempdir");

    let comment_review_controller = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.editor.commentReviewController.describe",
        &json!({"review_mode": "inline"}),
    );
    assert!(comment_review_controller.ok);
    assert_eq!(
        comment_review_controller
            .payload
            .unwrap_or_else(|| json!({}))
            .get("review_mode")
            .and_then(Value::as_str),
        Some("inline")
    );

    let diff_view_provider = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.editor.diffViewProvider.describe",
        &json!({"provider_mode": "vscode"}),
    );
    assert!(diff_view_provider.ok);
    assert_eq!(
        diff_view_provider
            .payload
            .unwrap_or_else(|| json!({}))
            .get("provider_mode")
            .and_then(Value::as_str),
        Some("vscode")
    );

    let file_edit_provider = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.editor.fileEditProvider.describe",
        &json!({"edit_mode": "patch"}),
    );
    assert!(file_edit_provider.ok);
    assert_eq!(
        file_edit_provider
            .payload
            .unwrap_or_else(|| json!({}))
            .get("edit_mode")
            .and_then(Value::as_str),
        Some("patch")
    );

    let detect_omission = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.editor.detectOmission.describe",
        &json!({"strategy": "token_window"}),
    );
    assert!(detect_omission.ok);
    assert_eq!(
        detect_omission
            .payload
            .unwrap_or_else(|| json!({}))
            .get("strategy")
            .and_then(Value::as_str),
        Some("token_window")
    );

    let export_markdown = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.misc.exportMarkdown.describe",
        &json!({"flavor": "gfm"}),
    );
    assert!(export_markdown.ok);
    assert_eq!(
        export_markdown
            .payload
            .unwrap_or_else(|| json!({}))
            .get("flavor")
            .and_then(Value::as_str),
        Some("gfm")
    );

    let extract_file_content = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.misc.extractFileContent.describe",
        &json!({"path": "/tmp/a.txt"}),
    );
    assert!(extract_file_content.ok);
    assert_eq!(
        extract_file_content
            .payload
            .unwrap_or_else(|| json!({}))
            .get("path")
            .and_then(Value::as_str),
        Some("/tmp/a.txt")
    );

    let extract_images = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.misc.extractImages.describe",
        &json!({"include_metadata": true}),
    );
    assert!(extract_images.ok);
    assert_eq!(
        extract_images
            .payload
            .unwrap_or_else(|| json!({}))
            .get("include_metadata")
            .and_then(Value::as_bool),
        Some(true)
    );

    let extract_text = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.misc.extractText.describe",
        &json!({"source_type": "document"}),
    );
    assert!(extract_text.ok);
    assert_eq!(
        extract_text
            .payload
            .unwrap_or_else(|| json!({}))
            .get("source_type")
            .and_then(Value::as_str),
        Some("document")
    );

    let link_preview = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.misc.linkPreview.describe",
        &json!({"url": "https://example.com"}),
    );
    assert!(link_preview.ok);
    assert_eq!(
        link_preview
            .payload
            .unwrap_or_else(|| json!({}))
            .get("url")
            .and_then(Value::as_str),
        Some("https://example.com")
    );

    let notebook_utils = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.misc.notebookUtils.describe",
        &json!({"notebook_mode": "default"}),
    );
    assert!(notebook_utils.ok);
    assert_eq!(
        notebook_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("notebook_mode")
            .and_then(Value::as_str),
        Some("default")
    );
}
