fn dashboard_prompt_system_prompt_variants_glm_config_describe(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("glm"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_glm_config_describe",
        "profile": profile
    })
}

fn dashboard_prompt_system_prompt_variants_glm_overrides_describe(payload: &Value) -> Value {
    let override_mode = clean_text(
        payload
            .get("override_mode")
            .and_then(Value::as_str)
            .unwrap_or("balanced"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_glm_overrides_describe",
        "override_mode": override_mode
    })
}

fn dashboard_prompt_system_prompt_variants_glm_template_describe(payload: &Value) -> Value {
    let template_mode = clean_text(
        payload
            .get("template_mode")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_glm_template_describe",
        "template_mode": template_mode
    })
}

fn dashboard_prompt_system_prompt_variants_gpt5_config_describe(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("gpt5"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_gpt5_config_describe",
        "profile": profile
    })
}

fn dashboard_prompt_system_prompt_variants_gpt5_template_describe(payload: &Value) -> Value {
    let template_mode = clean_text(
        payload
            .get("template_mode")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_gpt5_template_describe",
        "template_mode": template_mode
    })
}

fn dashboard_prompt_system_prompt_variants_hermes_config_describe(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("hermes"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_hermes_config_describe",
        "profile": profile
    })
}

fn dashboard_prompt_system_prompt_variants_hermes_overrides_describe(payload: &Value) -> Value {
    let override_mode = clean_text(
        payload
            .get("override_mode")
            .and_then(Value::as_str)
            .unwrap_or("balanced"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_hermes_overrides_describe",
        "override_mode": override_mode
    })
}

fn dashboard_prompt_system_prompt_variants_hermes_template_describe(payload: &Value) -> Value {
    let template_mode = clean_text(
        payload
            .get("template_mode")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_hermes_template_describe",
        "template_mode": template_mode
    })
}

fn dashboard_prompt_system_prompt_variants_index_describe(payload: &Value) -> Value {
    let index_scope = clean_text(
        payload
            .get("index_scope")
            .and_then(Value::as_str)
            .unwrap_or("all"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_index_describe",
        "index_scope": index_scope
    })
}

fn dashboard_prompt_system_prompt_variants_native_gpt51_config_describe(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("native-gpt-5-1"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_native_gpt51_config_describe",
        "profile": profile
    })
}

fn dashboard_prompt_system_prompt_variants_family_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.systemPrompt.variants.glm.config.describe" => {
            Some(dashboard_prompt_system_prompt_variants_glm_config_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.glm.overrides.describe" => {
            Some(dashboard_prompt_system_prompt_variants_glm_overrides_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.glm.template.describe" => {
            Some(dashboard_prompt_system_prompt_variants_glm_template_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.gpt5.config.describe" => {
            Some(dashboard_prompt_system_prompt_variants_gpt5_config_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.gpt5.template.describe" => {
            Some(dashboard_prompt_system_prompt_variants_gpt5_template_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.hermes.config.describe" => {
            Some(dashboard_prompt_system_prompt_variants_hermes_config_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.hermes.overrides.describe" => {
            Some(dashboard_prompt_system_prompt_variants_hermes_overrides_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.hermes.template.describe" => {
            Some(dashboard_prompt_system_prompt_variants_hermes_template_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.index.describe" => {
            Some(dashboard_prompt_system_prompt_variants_index_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.nativeGpt51.config.describe" => {
            Some(dashboard_prompt_system_prompt_variants_native_gpt51_config_describe(payload))
        }
        _ => dashboard_prompt_system_prompt_variants_native_tail_route_extension(
            root,
            normalized,
            payload,
        ),
    }
}

include!("067-dashboard-system-prompt-variants-native-tail-helpers.rs");
