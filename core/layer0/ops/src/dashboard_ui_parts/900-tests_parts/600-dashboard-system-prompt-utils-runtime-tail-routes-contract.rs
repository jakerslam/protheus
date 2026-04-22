#[test]
fn dashboard_system_prompt_utils_runtime_tail_routes_contract_wave_600() {
    let root = tempfile::tempdir().expect("tempdir");

    let model_utils = run_action(
        root.path(),
        "dashboard.prompts.system.utils.modelUtils.describe",
        &json!({"family": "gpt"}),
    );
    assert!(model_utils.ok);
    assert_eq!(
        model_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("family")
            .and_then(Value::as_str),
        Some("gpt")
    );

    let path = run_action(
        root.path(),
        "dashboard.prompts.system.utils.path.describe",
        &json!({"scope": "workspace"}),
    );
    assert!(path.ok);
    assert_eq!(
        path.payload
            .unwrap_or_else(|| json!({}))
            .get("scope")
            .and_then(Value::as_str),
        Some("workspace")
    );

    let powershell = run_action(
        root.path(),
        "dashboard.prompts.system.utils.powershell.describe",
        &json!({"mode": "compatible"}),
    );
    assert!(powershell.ok);
    assert_eq!(
        powershell
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("compatible")
    );

    let process_termination = run_action(
        root.path(),
        "dashboard.prompts.system.utils.processTermination.describe",
        &json!({"strategy": "graceful_then_force"}),
    );
    assert!(process_termination.ok);
    assert_eq!(
        process_termination
            .payload
            .unwrap_or_else(|| json!({}))
            .get("strategy")
            .and_then(Value::as_str),
        Some("graceful_then_force")
    );

    let retry = run_action(
        root.path(),
        "dashboard.prompts.system.utils.retry.describe",
        &json!({"policy": "bounded_exponential"}),
    );
    assert!(retry.ok);
    assert_eq!(
        retry
            .payload
            .unwrap_or_else(|| json!({}))
            .get("policy")
            .and_then(Value::as_str),
        Some("bounded_exponential")
    );

    let shell = run_action(
        root.path(),
        "dashboard.prompts.system.utils.shell.describe",
        &json!({"shell": "zsh"}),
    );
    assert!(shell.ok);
    assert_eq!(
        shell
            .payload
            .unwrap_or_else(|| json!({}))
            .get("shell")
            .and_then(Value::as_str),
        Some("zsh")
    );

    let storage = run_action(
        root.path(),
        "dashboard.prompts.system.utils.storage.describe",
        &json!({"backend": "sqlite"}),
    );
    assert!(storage.ok);
    assert_eq!(
        storage
            .payload
            .unwrap_or_else(|| json!({}))
            .get("backend")
            .and_then(Value::as_str),
        Some("sqlite")
    );

    let string = run_action(
        root.path(),
        "dashboard.prompts.system.utils.string.describe",
        &json!({"operation": "trim"}),
    );
    assert!(string.ok);
    assert_eq!(
        string
            .payload
            .unwrap_or_else(|| json!({}))
            .get("operation")
            .and_then(Value::as_str),
        Some("trim")
    );

    let tab_filtering = run_action(
        root.path(),
        "dashboard.prompts.system.utils.tabFiltering.describe",
        &json!({"filter": "visible"}),
    );
    assert!(tab_filtering.ok);
    assert_eq!(
        tab_filtering
            .payload
            .unwrap_or_else(|| json!({}))
            .get("filter")
            .and_then(Value::as_str),
        Some("visible")
    );

    let time = run_action(
        root.path(),
        "dashboard.prompts.system.utils.time.describe",
        &json!({"clock": "utc"}),
    );
    assert!(time.ok);
    assert_eq!(
        time.payload
            .unwrap_or_else(|| json!({}))
            .get("clock")
            .and_then(Value::as_str),
        Some("utc")
    );
}
