fn dashboard_prompt_system_prompt_types_describe(payload: &Value) -> Value {
    let type_mode = clean_text(
        payload
            .get("type_mode")
            .and_then(Value::as_str)
            .unwrap_or("strict"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_types_describe",
        "type_mode": type_mode
    })
}

fn dashboard_prompt_system_prompt_variants_config_template_describe(payload: &Value) -> Value {
    let template_profile = clean_text(
        payload
            .get("template_profile")
            .and_then(Value::as_str)
            .unwrap_or("base"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_config_template_describe",
        "template_profile": template_profile
    })
}

fn dashboard_prompt_system_prompt_variants_devstral_config_describe(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("devstral"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_devstral_config_describe",
        "profile": profile
    })
}

fn dashboard_prompt_system_prompt_variants_devstral_overrides_describe(payload: &Value) -> Value {
    let override_mode = clean_text(
        payload
            .get("override_mode")
            .and_then(Value::as_str)
            .unwrap_or("safe"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_devstral_overrides_describe",
        "override_mode": override_mode
    })
}

fn dashboard_prompt_system_prompt_variants_devstral_template_describe(payload: &Value) -> Value {
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
        "type": "dashboard_prompts_system_system_prompt_variants_devstral_template_describe",
        "template_mode": template_mode
    })
}

fn dashboard_prompt_system_prompt_variants_gemini3_config_describe(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("gemini3"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_gemini3_config_describe",
        "profile": profile
    })
}

fn dashboard_prompt_system_prompt_variants_gemini3_overrides_describe(payload: &Value) -> Value {
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
        "type": "dashboard_prompts_system_system_prompt_variants_gemini3_overrides_describe",
        "override_mode": override_mode
    })
}

fn dashboard_prompt_system_prompt_variants_gemini3_template_describe(payload: &Value) -> Value {
    let template_mode = clean_text(
        payload
            .get("template_mode")
            .and_then(Value::as_str)
            .unwrap_or("concise"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_gemini3_template_describe",
        "template_mode": template_mode
    })
}

fn dashboard_prompt_system_prompt_variants_generic_config_describe(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("generic"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_generic_config_describe",
        "profile": profile
    })
}

fn dashboard_prompt_system_prompt_variants_generic_template_describe(payload: &Value) -> Value {
    let template_mode = clean_text(
        payload
            .get("template_mode")
            .and_then(Value::as_str)
            .unwrap_or("generic"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_generic_template_describe",
        "template_mode": template_mode
    })
}

fn dashboard_prompt_system_prompt_variants_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.systemPrompt.types.describe" => {
            Some(dashboard_prompt_system_prompt_types_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.configTemplate.describe" => {
            Some(dashboard_prompt_system_prompt_variants_config_template_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.devstral.config.describe" => {
            Some(dashboard_prompt_system_prompt_variants_devstral_config_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.devstral.overrides.describe" => {
            Some(dashboard_prompt_system_prompt_variants_devstral_overrides_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.devstral.template.describe" => {
            Some(dashboard_prompt_system_prompt_variants_devstral_template_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.gemini3.config.describe" => {
            Some(dashboard_prompt_system_prompt_variants_gemini3_config_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.gemini3.overrides.describe" => {
            Some(dashboard_prompt_system_prompt_variants_gemini3_overrides_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.gemini3.template.describe" => {
            Some(dashboard_prompt_system_prompt_variants_gemini3_template_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.generic.config.describe" => {
            Some(dashboard_prompt_system_prompt_variants_generic_config_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.generic.template.describe" => {
            Some(dashboard_prompt_system_prompt_variants_generic_template_describe(payload))
        }
        _ => dashboard_prompt_system_prompt_variants_family_tail_route_extension(
            root,
            normalized,
            payload,
        ),
    }
}

include!("066-dashboard-system-prompt-variants-family-tail-helpers.rs");
