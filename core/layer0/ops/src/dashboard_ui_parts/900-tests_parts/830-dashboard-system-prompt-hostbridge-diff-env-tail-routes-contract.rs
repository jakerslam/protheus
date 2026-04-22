#[test]
fn dashboard_system_prompt_hostbridge_diff_env_tail_routes_contract_wave_830() {
    let root = tempfile::tempdir().expect("tempdir");

    let save_document = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.diff.saveDocument.describe",
        &json!({"uri": "file:///tmp/a.ts", "save_mode": "write"}),
    );
    assert!(save_document.ok);
    assert_eq!(
        save_document
            .payload
            .unwrap_or_else(|| json!({}))
            .get("save_mode")
            .and_then(Value::as_str),
        Some("write")
    );

    let scroll_diff = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.diff.scrollDiff.describe",
        &json!({"direction": "down", "lines": 2}),
    );
    assert!(scroll_diff.ok);
    assert_eq!(
        scroll_diff
            .payload
            .unwrap_or_else(|| json!({}))
            .get("lines")
            .and_then(Value::as_i64),
        Some(2)
    );

    let truncate_document = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.diff.truncateDocument.describe",
        &json!({"uri": "file:///tmp/a.ts", "max_bytes": 4096}),
    );
    assert!(truncate_document.ok);
    assert_eq!(
        truncate_document
            .payload
            .unwrap_or_else(|| json!({}))
            .get("max_bytes")
            .and_then(Value::as_u64),
        Some(4096)
    );

    let clipboard_read = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.env.clipboardReadText.describe",
        &json!({"scope": "global"}),
    );
    assert!(clipboard_read.ok);
    assert_eq!(
        clipboard_read
            .payload
            .unwrap_or_else(|| json!({}))
            .get("scope")
            .and_then(Value::as_str),
        Some("global")
    );

    let clipboard_write = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.env.clipboardWriteText.describe",
        &json!({"text": "hello world"}),
    );
    assert!(clipboard_write.ok);
    assert_eq!(
        clipboard_write
            .payload
            .unwrap_or_else(|| json!({}))
            .get("text_len")
            .and_then(Value::as_i64),
        Some("hello world".len() as i64)
    );

    let debug_log = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.env.debugLog.describe",
        &json!({"level": "info"}),
    );
    assert!(debug_log.ok);
    assert_eq!(
        debug_log
            .payload
            .unwrap_or_else(|| json!({}))
            .get("level")
            .and_then(Value::as_str),
        Some("info")
    );

    let host_version_test = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.env.getHostVersionTest.describe",
        &json!({"test_profile": "contract"}),
    );
    assert!(host_version_test.ok);
    assert_eq!(
        host_version_test
            .payload
            .unwrap_or_else(|| json!({}))
            .get("test_profile")
            .and_then(Value::as_str),
        Some("contract")
    );

    let host_version = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.env.getHostVersion.describe",
        &json!({"release_channel": "stable"}),
    );
    assert!(host_version.ok);
    assert_eq!(
        host_version
            .payload
            .unwrap_or_else(|| json!({}))
            .get("release_channel")
            .and_then(Value::as_str),
        Some("stable")
    );

    let ide_redirect_uri = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.env.getIdeRedirectUri.describe",
        &json!({"provider": "default"}),
    );
    assert!(ide_redirect_uri.ok);
    assert_eq!(
        ide_redirect_uri
            .payload
            .unwrap_or_else(|| json!({}))
            .get("provider")
            .and_then(Value::as_str),
        Some("default")
    );

    let telemetry_settings = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.env.getTelemetrySettings.describe",
        &json!({"surface": "vscode"}),
    );
    assert!(telemetry_settings.ok);
    assert_eq!(
        telemetry_settings
            .payload
            .unwrap_or_else(|| json!({}))
            .get("surface")
            .and_then(Value::as_str),
        Some("vscode")
    );
}
