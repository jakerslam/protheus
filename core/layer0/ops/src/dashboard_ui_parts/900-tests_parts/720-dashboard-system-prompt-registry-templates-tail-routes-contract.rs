#[test]
fn dashboard_system_prompt_registry_templates_tail_routes_contract_wave_720() {
    let root = tempfile::tempdir().expect("tempdir");

    let task_progress = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.components.taskProgress.describe",
        &json!({"progress_mode": "incremental"}),
    );
    assert!(task_progress.ok);
    assert_eq!(
        task_progress
            .payload
            .unwrap_or_else(|| json!({}))
            .get("progress_mode")
            .and_then(Value::as_str),
        Some("incremental")
    );

    let user_instructions = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.components.userInstructions.describe",
        &json!({"instruction_mode": "strict"}),
    );
    assert!(user_instructions.ok);
    assert_eq!(
        user_instructions
            .payload
            .unwrap_or_else(|| json!({}))
            .get("instruction_mode")
            .and_then(Value::as_str),
        Some("strict")
    );

    let constants = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.constants.describe",
        &json!({"constants_scope": "core"}),
    );
    assert!(constants.ok);
    assert_eq!(
        constants
            .payload
            .unwrap_or_else(|| json!({}))
            .get("constants_scope")
            .and_then(Value::as_str),
        Some("core")
    );

    let index = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.index.describe",
        &json!({"index_scope": "system_prompt"}),
    );
    assert!(index.ok);
    assert_eq!(
        index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("index_scope")
            .and_then(Value::as_str),
        Some("system_prompt")
    );

    let cline_toolset = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.registry.clineToolSet.describe",
        &json!({"toolset_mode": "default"}),
    );
    assert!(cline_toolset.ok);
    assert_eq!(
        cline_toolset
            .payload
            .unwrap_or_else(|| json!({}))
            .get("toolset_mode")
            .and_then(Value::as_str),
        Some("default")
    );

    let prompt_builder = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.registry.promptBuilder.describe",
        &json!({"builder_mode": "composed"}),
    );
    assert!(prompt_builder.ok);
    assert_eq!(
        prompt_builder
            .payload
            .unwrap_or_else(|| json!({}))
            .get("builder_mode")
            .and_then(Value::as_str),
        Some("composed")
    );

    let prompt_registry = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.registry.promptRegistry.describe",
        &json!({"registry_mode": "canonical"}),
    );
    assert!(prompt_registry.ok);
    assert_eq!(
        prompt_registry
            .payload
            .unwrap_or_else(|| json!({}))
            .get("registry_mode")
            .and_then(Value::as_str),
        Some("canonical")
    );

    let spec = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.spec.describe",
        &json!({"spec_profile": "v1"}),
    );
    assert!(spec.ok);
    assert_eq!(
        spec.payload
            .unwrap_or_else(|| json!({}))
            .get("spec_profile")
            .and_then(Value::as_str),
        Some("v1")
    );

    let template_engine = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.templates.templateEngine.describe",
        &json!({"engine_mode": "deterministic"}),
    );
    assert!(template_engine.ok);
    assert_eq!(
        template_engine
            .payload
            .unwrap_or_else(|| json!({}))
            .get("engine_mode")
            .and_then(Value::as_str),
        Some("deterministic")
    );

    let placeholders = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.templates.placeholders.describe",
        &json!({"placeholder_policy": "strict"}),
    );
    assert!(placeholders.ok);
    assert_eq!(
        placeholders
            .payload
            .unwrap_or_else(|| json!({}))
            .get("placeholder_policy")
            .and_then(Value::as_str),
        Some("strict")
    );
}
