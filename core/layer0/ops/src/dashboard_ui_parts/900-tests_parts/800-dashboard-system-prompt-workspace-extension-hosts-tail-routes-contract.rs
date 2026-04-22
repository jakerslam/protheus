#[test]
fn dashboard_system_prompt_workspace_extension_hosts_tail_routes_contract_wave_800() {
    let root = tempfile::tempdir().expect("tempdir");

    let workspace_root_manager = run_action(
        root.path(),
        "dashboard.prompts.system.workspace.workspaceRootManager.describe",
        &json!({"root_mode": "single"}),
    );
    assert!(workspace_root_manager.ok);
    assert_eq!(
        workspace_root_manager
            .payload
            .unwrap_or_else(|| json!({}))
            .get("root_mode")
            .and_then(Value::as_str),
        Some("single")
    );

    let workspace_detection = run_action(
        root.path(),
        "dashboard.prompts.system.workspace.detection.describe",
        &json!({"detection_mode": "auto"}),
    );
    assert!(workspace_detection.ok);
    assert_eq!(
        workspace_detection
            .payload
            .unwrap_or_else(|| json!({}))
            .get("detection_mode")
            .and_then(Value::as_str),
        Some("auto")
    );

    let workspace_index = run_action(
        root.path(),
        "dashboard.prompts.system.workspace.index.describe",
        &json!({"index_scope": "workspace"}),
    );
    assert!(workspace_index.ok);
    assert_eq!(
        workspace_index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("index_scope")
            .and_then(Value::as_str),
        Some("workspace")
    );

    let workspace_multi_root_utils = run_action(
        root.path(),
        "dashboard.prompts.system.workspace.multiRootUtils.describe",
        &json!({"utility": "normalize"}),
    );
    assert!(workspace_multi_root_utils.ok);
    assert_eq!(
        workspace_multi_root_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("utility")
            .and_then(Value::as_str),
        Some("normalize")
    );

    let workspace_setup = run_action(
        root.path(),
        "dashboard.prompts.system.workspace.setup.describe",
        &json!({"setup_mode": "guided"}),
    );
    assert!(workspace_setup.ok);
    assert_eq!(
        workspace_setup
            .payload
            .unwrap_or_else(|| json!({}))
            .get("setup_mode")
            .and_then(Value::as_str),
        Some("guided")
    );

    let parse_workspace_inline_path = run_action(
        root.path(),
        "dashboard.prompts.system.workspace.parseWorkspaceInlinePath.describe",
        &json!({"parse_mode": "strict"}),
    );
    assert!(parse_workspace_inline_path.ok);
    assert_eq!(
        parse_workspace_inline_path
            .payload
            .unwrap_or_else(|| json!({}))
            .get("parse_mode")
            .and_then(Value::as_str),
        Some("strict")
    );

    let workspace_detection_utils = run_action(
        root.path(),
        "dashboard.prompts.system.workspace.workspaceDetection.describe",
        &json!({"utility": "workspace_detection"}),
    );
    assert!(workspace_detection_utils.ok);
    assert_eq!(
        workspace_detection_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("utility")
            .and_then(Value::as_str),
        Some("workspace_detection")
    );

    let extension = run_action(
        root.path(),
        "dashboard.prompts.system.extension.describe",
        &json!({"extension_mode": "runtime"}),
    );
    assert!(extension.ok);
    assert_eq!(
        extension
            .payload
            .unwrap_or_else(|| json!({}))
            .get("extension_mode")
            .and_then(Value::as_str),
        Some("runtime")
    );

    let auth_handler = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.external.authHandler.describe",
        &json!({"auth_mode": "oauth"}),
    );
    assert!(auth_handler.ok);
    assert_eq!(
        auth_handler
            .payload
            .unwrap_or_else(|| json!({}))
            .get("auth_mode")
            .and_then(Value::as_str),
        Some("oauth")
    );

    let external_comment_review_controller = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.external.externalCommentReviewController.describe",
        &json!({"review_mode": "threaded"}),
    );
    assert!(external_comment_review_controller.ok);
    assert_eq!(
        external_comment_review_controller
            .payload
            .unwrap_or_else(|| json!({}))
            .get("review_mode")
            .and_then(Value::as_str),
        Some("threaded")
    );
}
