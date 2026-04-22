#[test]
fn dashboard_system_prompt_integrations_editor_misc_tail_routes_contract_wave_420() {
    let root = tempfile::tempdir().expect("tempdir");

    let comment_review = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.editor.commentReviewController.describe",
        &json!({"thread_id": "thr-1"}),
    );
    assert!(comment_review.ok);
    assert_eq!(
        comment_review
            .payload
            .unwrap_or_else(|| json!({}))
            .get("thread_id")
            .and_then(Value::as_str),
        Some("thr-1")
    );

    let diff_view = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.editor.diffViewProvider.describe",
        &json!({"file_path": "/tmp/a.rs"}),
    );
    assert!(diff_view.ok);
    assert_eq!(
        diff_view
            .payload
            .unwrap_or_else(|| json!({}))
            .get("file_path")
            .and_then(Value::as_str),
        Some("/tmp/a.rs")
    );

    let file_edit = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.editor.fileEditProvider.describe",
        &json!({"file_path": "/tmp/b.rs", "edit_count": 2}),
    );
    assert!(file_edit.ok);
    let file_edit_payload = file_edit.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        file_edit_payload.get("file_path").and_then(Value::as_str),
        Some("/tmp/b.rs")
    );
    assert_eq!(
        file_edit_payload.get("edit_count").and_then(Value::as_u64),
        Some(2)
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
        &json!({"destination": "workspace"}),
    );
    assert!(export_markdown.ok);
    assert_eq!(
        export_markdown
            .payload
            .unwrap_or_else(|| json!({}))
            .get("destination")
            .and_then(Value::as_str),
        Some("workspace")
    );

    let extract_file_content = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.misc.extractFileContent.describe",
        &json!({"path": "/tmp/c.md", "max_chars": 1200}),
    );
    assert!(extract_file_content.ok);
    let extract_file_content_payload = extract_file_content.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        extract_file_content_payload
            .get("path")
            .and_then(Value::as_str),
        Some("/tmp/c.md")
    );
    assert_eq!(
        extract_file_content_payload
            .get("max_chars")
            .and_then(Value::as_u64),
        Some(1200)
    );

    let extract_images = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.misc.extractImages.describe",
        &json!({"path": "/tmp/d.ipynb"}),
    );
    assert!(extract_images.ok);
    assert_eq!(
        extract_images
            .payload
            .unwrap_or_else(|| json!({}))
            .get("path")
            .and_then(Value::as_str),
        Some("/tmp/d.ipynb")
    );

    let extract_text = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.misc.extractText.describe",
        &json!({"path": "/tmp/e.pdf"}),
    );
    assert!(extract_text.ok);
    assert_eq!(
        extract_text
            .payload
            .unwrap_or_else(|| json!({}))
            .get("path")
            .and_then(Value::as_str),
        Some("/tmp/e.pdf")
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
        &json!({"notebook_path": "/tmp/f.ipynb"}),
    );
    assert!(notebook_utils.ok);
    assert_eq!(
        notebook_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("notebook_path")
            .and_then(Value::as_str),
        Some("/tmp/f.ipynb")
    );
}
