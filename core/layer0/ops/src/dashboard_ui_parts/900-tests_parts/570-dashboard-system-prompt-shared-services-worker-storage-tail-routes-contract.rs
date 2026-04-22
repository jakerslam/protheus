#[test]
fn dashboard_system_prompt_shared_services_worker_storage_tail_routes_contract_wave_570() {
    let root = tempfile::tempdir().expect("tempdir");

    let posthog_config = run_action(
        root.path(),
        "dashboard.prompts.system.shared.services.config.posthogConfig.describe",
        &json!({"host": "https://app.posthog.com"}),
    );
    assert!(posthog_config.ok);
    assert_eq!(
        posthog_config
            .payload
            .unwrap_or_else(|| json!({}))
            .get("host")
            .and_then(Value::as_str),
        Some("https://app.posthog.com")
    );

    let feature_flags = run_action(
        root.path(),
        "dashboard.prompts.system.shared.services.featureFlags.featureFlags.describe",
        &json!({"profile": "default"}),
    );
    assert!(feature_flags.ok);
    assert_eq!(
        feature_flags
            .payload
            .unwrap_or_else(|| json!({}))
            .get("profile")
            .and_then(Value::as_str),
        Some("default")
    );

    let worker_backfill = run_action(
        root.path(),
        "dashboard.prompts.system.shared.services.worker.backfill.describe",
        &json!({"mode": "incremental"}),
    );
    assert!(worker_backfill.ok);
    assert_eq!(
        worker_backfill
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("incremental")
    );

    let worker_queue = run_action(
        root.path(),
        "dashboard.prompts.system.shared.services.worker.queue.describe",
        &json!({"policy": "bounded"}),
    );
    assert!(worker_queue.ok);
    assert_eq!(
        worker_queue
            .payload
            .unwrap_or_else(|| json!({}))
            .get("policy")
            .and_then(Value::as_str),
        Some("bounded")
    );

    let worker_sync = run_action(
        root.path(),
        "dashboard.prompts.system.shared.services.worker.sync.describe",
        &json!({"cadence": "scheduled"}),
    );
    assert!(worker_sync.ok);
    assert_eq!(
        worker_sync
            .payload
            .unwrap_or_else(|| json!({}))
            .get("cadence")
            .and_then(Value::as_str),
        Some("scheduled")
    );

    let worker_utils = run_action(
        root.path(),
        "dashboard.prompts.system.shared.services.worker.utils.describe",
        &json!({"helper": "retry_budget"}),
    );
    assert!(worker_utils.ok);
    assert_eq!(
        worker_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("helper")
            .and_then(Value::as_str),
        Some("retry_budget")
    );

    let worker_worker = run_action(
        root.path(),
        "dashboard.prompts.system.shared.services.worker.worker.describe",
        &json!({"worker": "default"}),
    );
    assert!(worker_worker.ok);
    assert_eq!(
        worker_worker
            .payload
            .unwrap_or_else(|| json!({}))
            .get("worker")
            .and_then(Value::as_str),
        Some("default")
    );

    let skills = run_action(
        root.path(),
        "dashboard.prompts.system.shared.skills.describe",
        &json!({"pack": "core"}),
    );
    assert!(skills.ok);
    assert_eq!(
        skills
            .payload
            .unwrap_or_else(|| json!({}))
            .get("pack")
            .and_then(Value::as_str),
        Some("core")
    );

    let slash_commands = run_action(
        root.path(),
        "dashboard.prompts.system.shared.slashCommands.describe",
        &json!({"command": "/help"}),
    );
    assert!(slash_commands.ok);
    assert_eq!(
        slash_commands
            .payload
            .unwrap_or_else(|| json!({}))
            .get("command")
            .and_then(Value::as_str),
        Some("/help")
    );

    let cline_blob_storage = run_action(
        root.path(),
        "dashboard.prompts.system.shared.storage.clineBlobStorage.describe",
        &json!({"backend": "sqlite"}),
    );
    assert!(cline_blob_storage.ok);
    assert_eq!(
        cline_blob_storage
            .payload
            .unwrap_or_else(|| json!({}))
            .get("backend")
            .and_then(Value::as_str),
        Some("sqlite")
    );
}
