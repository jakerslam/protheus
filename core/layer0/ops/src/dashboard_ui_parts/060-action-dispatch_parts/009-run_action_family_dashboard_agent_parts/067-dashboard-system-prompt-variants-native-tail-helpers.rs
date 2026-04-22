fn dashboard_prompt_system_prompt_variants_native_gpt51_overrides_describe(payload: &Value) -> Value {
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
        "type": "dashboard_prompts_system_system_prompt_variants_native_gpt51_overrides_describe",
        "override_mode": override_mode
    })
}

fn dashboard_prompt_system_prompt_variants_native_gpt51_template_describe(payload: &Value) -> Value {
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
        "type": "dashboard_prompts_system_system_prompt_variants_native_gpt51_template_describe",
        "template_mode": template_mode
    })
}

fn dashboard_prompt_system_prompt_variants_native_gpt5_config_describe(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("native-gpt-5"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_native_gpt5_config_describe",
        "profile": profile
    })
}

fn dashboard_prompt_system_prompt_variants_native_gpt5_template_describe(payload: &Value) -> Value {
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
        "type": "dashboard_prompts_system_system_prompt_variants_native_gpt5_template_describe",
        "template_mode": template_mode
    })
}

fn dashboard_prompt_system_prompt_variants_native_next_gen_config_describe(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("native-next-gen"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_native_next_gen_config_describe",
        "profile": profile
    })
}

fn dashboard_prompt_system_prompt_variants_native_next_gen_template_describe(payload: &Value) -> Value {
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
        "type": "dashboard_prompts_system_system_prompt_variants_native_next_gen_template_describe",
        "template_mode": template_mode
    })
}

fn dashboard_prompt_system_prompt_variants_next_gen_config_describe(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("next-gen"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_next_gen_config_describe",
        "profile": profile
    })
}

fn dashboard_prompt_system_prompt_variants_next_gen_template_describe(payload: &Value) -> Value {
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
        "type": "dashboard_prompts_system_system_prompt_variants_next_gen_template_describe",
        "template_mode": template_mode
    })
}

fn dashboard_prompt_system_prompt_variants_trinity_config_describe(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("trinity"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_trinity_config_describe",
        "profile": profile
    })
}

fn dashboard_prompt_system_prompt_variants_trinity_overrides_describe(payload: &Value) -> Value {
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
        "type": "dashboard_prompts_system_system_prompt_variants_trinity_overrides_describe",
        "override_mode": override_mode
    })
}

fn dashboard_prompt_system_prompt_variants_native_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.systemPrompt.variants.nativeGpt51.overrides.describe" => {
            Some(dashboard_prompt_system_prompt_variants_native_gpt51_overrides_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.nativeGpt51.template.describe" => {
            Some(dashboard_prompt_system_prompt_variants_native_gpt51_template_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.nativeGpt5.config.describe" => {
            Some(dashboard_prompt_system_prompt_variants_native_gpt5_config_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.nativeGpt5.template.describe" => {
            Some(dashboard_prompt_system_prompt_variants_native_gpt5_template_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.nativeNextGen.config.describe" => {
            Some(dashboard_prompt_system_prompt_variants_native_next_gen_config_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.nativeNextGen.template.describe" => {
            Some(dashboard_prompt_system_prompt_variants_native_next_gen_template_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.nextGen.config.describe" => {
            Some(dashboard_prompt_system_prompt_variants_next_gen_config_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.nextGen.template.describe" => {
            Some(dashboard_prompt_system_prompt_variants_next_gen_template_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.trinity.config.describe" => {
            Some(dashboard_prompt_system_prompt_variants_trinity_config_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.trinity.overrides.describe" => {
            Some(dashboard_prompt_system_prompt_variants_trinity_overrides_describe(payload))
        }
        _ => dashboard_prompt_system_prompt_variants_storage_tail_route_extension(
            root,
            normalized,
            payload,
        ),
    }
}

include!("068-dashboard-system-prompt-variants-storage-tail-helpers.rs");
