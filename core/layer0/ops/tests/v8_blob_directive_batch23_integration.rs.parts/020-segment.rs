#[test]
fn v8_batch40_blob_migrate_and_status_emit_dashboard_metrics() {
    let _guard = env_guard();
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    std::env::remove_var("BINARY_BLOB_RUNTIME_STATE_ROOT");
    std::env::remove_var("DIRECTIVE_KERNEL_STATE_ROOT");
    std::env::set_var("DIRECTIVE_KERNEL_SIGNING_KEY", "batch40-signing-key");
    std::env::set_var("BINARY_BLOB_VAULT_SIGNING_KEY", "batch40-blob-key");

    allow(root, "allow:blob:*");
    let module_path = root.join("demo_module.rs");
    fs::write(&module_path, "pub fn demo() -> u64 { 99 }\n").expect("write module");

    assert_eq!(
        binary_blob_runtime::run(
            root,
            &[
                "migrate".to_string(),
                "--modules=demo".to_string(),
                format!("--module-path={}", module_path.display()),
                "--apply=1".to_string(),
            ],
        ),
        0
    );
    let migrate_latest = latest("binary_blob_runtime", root);
    assert_eq!(
        migrate_latest.get("type").and_then(Value::as_str),
        Some("binary_blob_runtime_migrate")
    );
    assert!(has_claim(&migrate_latest, "V8-BINARY-BLOB-001.6"));
    assert!(migrate_latest
        .get("dashboard")
        .and_then(|v| v.get("blob_health"))
        .and_then(|v| v.get("healthy_modules"))
        .and_then(Value::as_u64)
        .map(|v| v >= 1)
        .unwrap_or(false));

    assert_eq!(
        binary_blob_runtime::run(root, &["dashboard".to_string()]),
        0
    );
    let status_latest = latest("binary_blob_runtime", root);
    assert_eq!(
        status_latest.get("type").and_then(Value::as_str),
        Some("binary_blob_runtime_status")
    );
    assert!(has_claim(&status_latest, "V8-BINARY-BLOB-001.6"));
    assert!(status_latest
        .get("dashboard")
        .and_then(|v| v.get("memory_savings"))
        .and_then(|v| v.get("source_bytes_total"))
        .and_then(Value::as_u64)
        .map(|v| v > 0)
        .unwrap_or(false));
    assert_eq!(
        status_latest
            .get("dashboard")
            .and_then(|v| v.get("directive_compliance"))
            .and_then(|v| v.get("directive_integrity_ok"))
            .and_then(Value::as_bool),
        Some(true)
    );

    std::env::remove_var("DIRECTIVE_KERNEL_SIGNING_KEY");
    std::env::remove_var("BINARY_BLOB_VAULT_SIGNING_KEY");
    std::env::remove_var("BINARY_BLOB_RUNTIME_STATE_ROOT");
    std::env::remove_var("DIRECTIVE_KERNEL_STATE_ROOT");
}

#[test]
fn v8_batch23_mutation_probe_and_debug_paths_are_runtime_enforced() {
    let _guard = env_guard();
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    std::env::set_var("DIRECTIVE_KERNEL_SIGNING_KEY", "batch23-signing-key");
    std::env::set_var("BINARY_BLOB_VAULT_SIGNING_KEY", "batch23-blob-key");
    std::env::set_var("SOUL_TOKEN_GUARD_KEY", "batch23-soul-key");

    allow(root, "allow:blob:*");
    allow(root, "allow:blob:mutate");
    allow(root, "allow:blob_mutate:*");

    let module_path = root.join("demo_module.rs");
    fs::write(&module_path, "pub fn demo() -> u64 { 55 }\n").expect("write module");
    assert_eq!(
        binary_blob_runtime::run(
            root,
            &[
                "settle".to_string(),
                "--module=demo".to_string(),
                format!("--module-path={}", module_path.display()),
                "--apply=1".to_string(),
            ],
        ),
        0
    );

    assert_eq!(
        binary_blob_runtime::run(
            root,
            &[
                "mutate".to_string(),
                "--module=demo".to_string(),
                "--proposal=optimize_demo".to_string(),
                "--canary-pass=0".to_string(),
                "--sim-regression=0.02".to_string(),
                "--apply=1".to_string(),
            ],
        ),
        2
    );
    let mutate_denied = latest("binary_blob_runtime", root);
    assert!(has_claim(&mutate_denied, "V8-BINARY-BLOB-001.3"));
    let mutation_history = read_jsonl(
        &root
            .join("core")
            .join("local")
            .join("state")
            .join("ops")
            .join("binary_blob_runtime")
            .join("mutation_history.jsonl"),
    );
    assert!(mutation_history
        .iter()
        .any(|row| { row.get("type").and_then(Value::as_str) == Some("rollback_triggered") }));

    std::env::set_var("BITNET_TERNARY_AVAILABLE", "1");
    assert_eq!(
        binary_blob_runtime::run(
            root,
            &[
                "substrate-probe".to_string(),
                "--prefer=ternary".to_string()
            ],
        ),
        0
    );
    let substrate_latest = latest("binary_blob_runtime", root);
    assert_eq!(
        substrate_latest.get("selected").and_then(Value::as_str),
        Some("ternary")
    );
    assert!(has_claim(&substrate_latest, "V8-BINARY-BLOB-001.4"));
    std::env::remove_var("BITNET_TERNARY_AVAILABLE");

    let _ = fs::remove_file(
        root.join("client")
            .join("local")
            .join("state")
            .join("security")
            .join("soul_token_guard.json"),
    );
    assert_eq!(
        binary_blob_runtime::run(
            root,
            &[
                "debug-access".to_string(),
                "--module=demo".to_string(),
                "--tamper=0".to_string(),
                "--apply=1".to_string(),
            ],
        ),
        2
    );
    let debug_denied = latest("binary_blob_runtime", root);
    assert!(has_claim(&debug_denied, "V8-BINARY-BLOB-001.5"));

    assert_eq!(
        security_plane::run(
            root,
            &[
                "soul-token-guard".to_string(),
                "issue".to_string(),
                "--instance-id=batch23".to_string(),
                "--approval-note=test".to_string(),
            ],
        ),
        0
    );
    assert_eq!(
        binary_blob_runtime::run(
            root,
            &[
                "debug-access".to_string(),
                "--module=demo".to_string(),
                "--tamper=0".to_string(),
                "--apply=1".to_string(),
            ],
        ),
        0
    );
    let debug_allowed = latest("binary_blob_runtime", root);
    assert!(has_claim(&debug_allowed, "V8-BINARY-BLOB-001.5"));
    assert_eq!(
        binary_blob_runtime::run(
            root,
            &[
                "debug-access".to_string(),
                "--module=demo".to_string(),
                "--tamper=1".to_string(),
                "--apply=1".to_string(),
            ],
        ),
        2
    );
    let debug_tamper = latest("binary_blob_runtime", root);
    assert!(has_claim(&debug_tamper, "V8-BINARY-BLOB-001.5"));
    let mutation_history_after = read_jsonl(
        &root
            .join("core")
            .join("local")
            .join("state")
            .join("ops")
            .join("binary_blob_runtime")
            .join("mutation_history.jsonl"),
    );
    assert!(mutation_history_after
        .iter()
        .any(|row| { row.get("type").and_then(Value::as_str) == Some("anti_tamper_dissolution") }));

    std::env::remove_var("DIRECTIVE_KERNEL_SIGNING_KEY");
    std::env::remove_var("BINARY_BLOB_VAULT_SIGNING_KEY");
    std::env::remove_var("SOUL_TOKEN_GUARD_KEY");
}

#[test]
fn v8_batch23_compliance_and_rsi_bridge_emit_denial_trace_and_rollback() {
    let _guard = env_guard();
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    std::env::set_var("DIRECTIVE_KERNEL_SIGNING_KEY", "batch23-signing-key");

    allow(root, "allow:rsi:*");
    assert_eq!(
        directive_kernel::run(
            root,
            &[
                "derive".to_string(),
                "--parent=allow:rsi:*".to_string(),
                "--directive=deny:rsi:unsafe".to_string(),
                "--signer=tester".to_string(),
            ],
        ),
        0
    );
    assert!(has_claim(
        &latest("directive_kernel", root),
        "V8-DIRECTIVES-001.2"
    ));

    assert_eq!(
        directive_kernel::run(
            root,
            &[
                "compliance-check".to_string(),
                "--action=rsi:unsafe".to_string(),
            ],
        ),
        2
    );
    let compliance_latest = latest("directive_kernel", root);
    assert_eq!(
        compliance_latest.get("type").and_then(Value::as_str),
        Some("directive_kernel_compliance_check")
    );
    assert_eq!(
        compliance_latest
            .get("evaluation")
            .and_then(|v| v.get("deny_hits"))
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty()),
        Some(true)
    );
    assert!(has_claim(&compliance_latest, "V8-DIRECTIVES-001.3"));

    assert_eq!(
        directive_kernel::run(
            root,
            &[
                "bridge-rsi".to_string(),
                "--proposal=unsafe".to_string(),
                "--apply=1".to_string(),
            ],
        ),
        2
    );
    let bridge_latest = latest("directive_kernel", root);
    assert_eq!(
        bridge_latest.get("type").and_then(Value::as_str),
        Some("directive_kernel_rsi_bridge")
    );
    assert_eq!(
        bridge_latest.get("allowed").and_then(Value::as_bool),
        Some(false)
    );
    assert!(has_claim(&bridge_latest, "V8-DIRECTIVES-001.4"));

    let history = read_jsonl(
        &root
            .join("core")
            .join("local")
            .join("state")
            .join("ops")
            .join("directive_kernel")
            .join("history.jsonl"),
    );
    assert!(history.iter().any(|row| {
        row.get("type").and_then(Value::as_str) == Some("directive_kernel_rsi_bridge_rollback")
    }));

    std::env::remove_var("DIRECTIVE_KERNEL_SIGNING_KEY");
}
