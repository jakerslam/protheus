#[test]
fn dashboard_system_prompt_webview_auth_storage_tail_routes_contract_wave_620() {
    let root = tempfile::tempdir().expect("tempdir");

    let oca_provider = run_action(
        root.path(),
        "dashboard.prompts.system.auth.oca.provider.describe",
        &json!({"realm": "default"}),
    );
    assert!(oca_provider.ok);
    assert_eq!(
        oca_provider
            .payload
            .unwrap_or_else(|| json!({}))
            .get("realm")
            .and_then(Value::as_str),
        Some("default")
    );

    let oca_constants = run_action(
        root.path(),
        "dashboard.prompts.system.auth.oca.constants.describe",
        &json!({"profile": "base"}),
    );
    assert!(oca_constants.ok);
    assert_eq!(
        oca_constants
            .payload
            .unwrap_or_else(|| json!({}))
            .get("profile")
            .and_then(Value::as_str),
        Some("base")
    );

    let oca_types = run_action(
        root.path(),
        "dashboard.prompts.system.auth.oca.types.describe",
        &json!({"type_set": "default"}),
    );
    assert!(oca_types.ok);
    assert_eq!(
        oca_types
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type_set")
            .and_then(Value::as_str),
        Some("default")
    );

    let oca_utils = run_action(
        root.path(),
        "dashboard.prompts.system.auth.oca.utils.describe",
        &json!({"utility": "normalize"}),
    );
    assert!(oca_utils.ok);
    assert_eq!(
        oca_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("utility")
            .and_then(Value::as_str),
        Some("normalize")
    );

    let cline_provider = run_action(
        root.path(),
        "dashboard.prompts.system.auth.clineProvider.describe",
        &json!({"provider": "cline"}),
    );
    assert!(cline_provider.ok);
    assert_eq!(
        cline_provider
            .payload
            .unwrap_or_else(|| json!({}))
            .get("provider")
            .and_then(Value::as_str),
        Some("cline")
    );

    let file_storage = run_action(
        root.path(),
        "dashboard.prompts.system.shared.storage.fileStorage.describe",
        &json!({"medium": "disk"}),
    );
    assert!(file_storage.ok);
    assert_eq!(
        file_storage
            .payload
            .unwrap_or_else(|| json!({}))
            .get("medium")
            .and_then(Value::as_str),
        Some("disk")
    );

    let storage = run_action(
        root.path(),
        "dashboard.prompts.system.shared.storage.storage.describe",
        &json!({"scope": "workspace"}),
    );
    assert!(storage.ok);
    assert_eq!(
        storage
            .payload
            .unwrap_or_else(|| json!({}))
            .get("scope")
            .and_then(Value::as_str),
        Some("workspace")
    );

    let adapters = run_action(
        root.path(),
        "dashboard.prompts.system.shared.storage.adapters.describe",
        &json!({"adapter": "file"}),
    );
    assert!(adapters.ok);
    assert_eq!(
        adapters
            .payload
            .unwrap_or_else(|| json!({}))
            .get("adapter")
            .and_then(Value::as_str),
        Some("file")
    );

    let index = run_action(
        root.path(),
        "dashboard.prompts.system.shared.storage.index.describe",
        &json!({"index_mode": "provider-first"}),
    );
    assert!(index.ok);
    assert_eq!(
        index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("index_mode")
            .and_then(Value::as_str),
        Some("provider-first")
    );

    let provider_keys = run_action(
        root.path(),
        "dashboard.prompts.system.shared.storage.providerKeys.describe",
        &json!({"key_set": "default"}),
    );
    assert!(provider_keys.ok);
    assert_eq!(
        provider_keys
            .payload
            .unwrap_or_else(|| json!({}))
            .get("key_set")
            .and_then(Value::as_str),
        Some("default")
    );
}
