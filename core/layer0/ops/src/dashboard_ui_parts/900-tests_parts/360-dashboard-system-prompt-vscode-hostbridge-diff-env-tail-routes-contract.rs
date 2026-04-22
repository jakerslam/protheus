#[test]
fn dashboard_system_prompt_vscode_hostbridge_diff_tail_routes_contract_wave_360() {
    let root = tempfile::tempdir().expect("tempdir");

    let save = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.diff.saveDocument.describe",
        &json!({"uri": "file:///tmp/a.ts"}),
    );
    assert!(save.ok);
    assert_eq!(
        save.payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_host_vscode_diff_save_document_describe")
    );

    let scroll = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.diff.scrollDiff.describe",
        &json!({"direction": "up", "lines": 3}),
    );
    assert!(scroll.ok);
    assert_eq!(
        scroll
            .payload
            .unwrap_or_else(|| json!({}))
            .get("lines")
            .and_then(Value::as_i64),
        Some(3)
    );

    let truncate = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.diff.truncateDocument.describe",
        &json!({"uri": "file:///tmp/a.ts", "max_bytes": 2048}),
    );
    assert!(truncate.ok);
    assert_eq!(
        truncate
            .payload
            .unwrap_or_else(|| json!({}))
            .get("max_bytes")
            .and_then(Value::as_i64),
        Some(2048)
    );
}

#[test]
fn dashboard_system_prompt_vscode_hostbridge_env_tail_routes_contract_wave_360() {
    let root = tempfile::tempdir().expect("tempdir");

    let read = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.env.clipboardReadText.describe",
        &json!({}),
    );
    assert!(read.ok);
    assert_eq!(
        read.payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_host_vscode_env_clipboard_read_text_describe")
    );

    let write = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.env.clipboardWriteText.describe",
        &json!({"text": "hello"}),
    );
    assert!(write.ok);
    assert_eq!(
        write
            .payload
            .unwrap_or_else(|| json!({}))
            .get("text_len")
            .and_then(Value::as_i64),
        Some(5)
    );

    let debug = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.env.debugLog.describe",
        &json!({"level": "warn"}),
    );
    assert!(debug.ok);
    assert_eq!(
        debug
            .payload
            .unwrap_or_else(|| json!({}))
            .get("level")
            .and_then(Value::as_str),
        Some("warn")
    );

    let host_version_test = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.env.getHostVersionTest.describe",
        &json!({}),
    );
    assert!(host_version_test.ok);
    assert_eq!(
        host_version_test
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("test_fixture")
    );

    let host_version = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.env.getHostVersion.describe",
        &json!({"expected": "1.91.0"}),
    );
    assert!(host_version.ok);
    assert_eq!(
        host_version
            .payload
            .unwrap_or_else(|| json!({}))
            .get("expected")
            .and_then(Value::as_str),
        Some("1.91.0")
    );

    let redirect = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.env.getIdeRedirectUri.describe",
        &json!({"ide": "vscode"}),
    );
    assert!(redirect.ok);
    assert_eq!(
        redirect
            .payload
            .unwrap_or_else(|| json!({}))
            .get("ide")
            .and_then(Value::as_str),
        Some("vscode")
    );

    let telemetry = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.env.getTelemetrySettings.describe",
        &json!({}),
    );
    assert!(telemetry.ok);
    assert!(
        telemetry
            .payload
            .unwrap_or_else(|| json!({}))
            .get("fields")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false)
    );
}
