fn dashboard_prompt_system_prompt_component_task_progress_describe(payload: &Value) -> Value {
    let progress_mode = clean_text(
        payload
            .get("progress_mode")
            .and_then(Value::as_str)
            .unwrap_or("incremental"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_component_task_progress_describe",
        "progress_mode": progress_mode
    })
}

fn dashboard_prompt_system_prompt_component_user_instructions_describe(payload: &Value) -> Value {
    let instruction_mode = clean_text(
        payload
            .get("instruction_mode")
            .and_then(Value::as_str)
            .unwrap_or("strict"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_component_user_instructions_describe",
        "instruction_mode": instruction_mode
    })
}

fn dashboard_prompt_system_prompt_constants_describe(payload: &Value) -> Value {
    let constants_scope = clean_text(
        payload
            .get("constants_scope")
            .and_then(Value::as_str)
            .unwrap_or("core"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_constants_describe",
        "constants_scope": constants_scope
    })
}

fn dashboard_prompt_system_prompt_index_describe(payload: &Value) -> Value {
    let index_scope = clean_text(
        payload
            .get("index_scope")
            .and_then(Value::as_str)
            .unwrap_or("system_prompt"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_index_describe",
        "index_scope": index_scope
    })
}

fn dashboard_prompt_system_prompt_registry_cline_toolset_describe(payload: &Value) -> Value {
    let toolset_mode = clean_text(
        payload
            .get("toolset_mode")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_registry_cline_toolset_describe",
        "toolset_mode": toolset_mode
    })
}

fn dashboard_prompt_system_prompt_registry_prompt_builder_describe(payload: &Value) -> Value {
    let builder_mode = clean_text(
        payload
            .get("builder_mode")
            .and_then(Value::as_str)
            .unwrap_or("composed"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_registry_prompt_builder_describe",
        "builder_mode": builder_mode
    })
}

fn dashboard_prompt_system_prompt_registry_prompt_registry_describe(payload: &Value) -> Value {
    let registry_mode = clean_text(
        payload
            .get("registry_mode")
            .and_then(Value::as_str)
            .unwrap_or("canonical"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_registry_prompt_registry_describe",
        "registry_mode": registry_mode
    })
}

fn dashboard_prompt_system_prompt_spec_describe(payload: &Value) -> Value {
    let spec_profile = clean_text(
        payload
            .get("spec_profile")
            .and_then(Value::as_str)
            .unwrap_or("v1"),
        80,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_spec_describe",
        "spec_profile": spec_profile
    })
}

fn dashboard_prompt_system_prompt_template_engine_describe(payload: &Value) -> Value {
    let engine_mode = clean_text(
        payload
            .get("engine_mode")
            .and_then(Value::as_str)
            .unwrap_or("deterministic"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_template_engine_describe",
        "engine_mode": engine_mode
    })
}

fn dashboard_prompt_system_prompt_placeholders_describe(payload: &Value) -> Value {
    let placeholder_policy = clean_text(
        payload
            .get("placeholder_policy")
            .and_then(Value::as_str)
            .unwrap_or("strict"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_placeholders_describe",
        "placeholder_policy": placeholder_policy
    })
}

fn dashboard_prompt_system_prompt_registry_templates_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.systemPrompt.components.taskProgress.describe" => {
            Some(dashboard_prompt_system_prompt_component_task_progress_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.components.userInstructions.describe" => {
            Some(dashboard_prompt_system_prompt_component_user_instructions_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.constants.describe" => {
            Some(dashboard_prompt_system_prompt_constants_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.index.describe" => {
            Some(dashboard_prompt_system_prompt_index_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.registry.clineToolSet.describe" => {
            Some(dashboard_prompt_system_prompt_registry_cline_toolset_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.registry.promptBuilder.describe" => {
            Some(dashboard_prompt_system_prompt_registry_prompt_builder_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.registry.promptRegistry.describe" => {
            Some(dashboard_prompt_system_prompt_registry_prompt_registry_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.spec.describe" => {
            Some(dashboard_prompt_system_prompt_spec_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.templates.templateEngine.describe" => {
            Some(dashboard_prompt_system_prompt_template_engine_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.templates.placeholders.describe" => {
            Some(dashboard_prompt_system_prompt_placeholders_describe(payload))
        }
        _ => dashboard_prompt_system_prompt_variants_tail_route_extension(root, normalized, payload),
    }
}

include!("065-dashboard-system-prompt-variants-tail-helpers.rs");
