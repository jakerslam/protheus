fn dashboard_prompt_variant_native_defaults(profile: &str) -> Option<Value> {
    match profile {
        "native-gpt-5-1" => Some(json!({
            "profile": "native-gpt-5-1",
            "template": "Native {{profile}}/{{variant}} mode={{mode}} with strict synthesis controls.",
            "config": {"temperature": 0.08, "top_p": 0.82},
            "overrides": {"max_output_tokens": 1200, "reasoning_effort": "high"}
        })),
        "native-gpt-5" => Some(json!({
            "profile": "native-gpt-5",
            "template": "Native {{profile}} {{variant}} mode={{mode}}. Preserve policy and lane authority.",
            "config": {"temperature": 0.1, "top_p": 0.85},
            "overrides": {"max_output_tokens": 1100}
        })),
        "native-next-gen" => Some(json!({
            "profile": "native-next-gen",
            "template": "Native {{profile}} {{variant}} mode={{mode}}. Minimize orchestration ambiguity.",
            "config": {"temperature": 0.12, "top_p": 0.88},
            "overrides": {"max_output_tokens": 1050}
        })),
        "next-gen" => Some(json!({
            "profile": "next-gen",
            "template": "Next-gen {{profile}} {{variant}} mode={{mode}} with explicit contract output.",
            "config": {"temperature": 0.15, "top_p": 0.9},
            "overrides": {"max_output_tokens": 1000}
        })),
        "trinity" => Some(json!({
            "profile": "trinity",
            "template": "Trinity {{profile}} {{variant}} mode={{mode}}. Balance reasoning with concise synthesis.",
            "config": {"temperature": 0.2, "top_p": 0.92},
            "overrides": {"max_output_tokens": 950}
        })),
        _ => None,
    }
}

fn dashboard_prompt_variant_native_catalog() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_variant_native_catalog",
        "profiles": [
            "native-gpt-5-1",
            "native-gpt-5",
            "native-next-gen",
            "next-gen",
            "trinity"
        ]
    })
}

fn dashboard_prompt_variant_native_resolve(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    )
    .to_ascii_lowercase();
    let found = dashboard_prompt_variant_native_defaults(&profile);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_variant_native_resolve",
        "profile": profile,
        "found": found.is_some(),
        "row": found.unwrap_or_else(|| json!({}))
    })
}

fn dashboard_prompt_variant_native_render(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    )
    .to_ascii_lowercase();
    let variant = clean_text(
        payload
            .get("variant")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        80,
    )
    .to_ascii_lowercase();
    let mode = clean_text(
        payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("act"),
        20,
    )
    .to_ascii_lowercase();
    let defaults = match dashboard_prompt_variant_native_defaults(&profile) {
        Some(v) => v,
        None => {
            return json!({
                "ok": false,
                "type": "dashboard_prompts_system_variant_native_render",
                "error": "native_variant_profile_not_found",
                "profile": profile
            });
        }
    };
    let template = clean_text(
        defaults
            .get("template")
            .and_then(Value::as_str)
            .unwrap_or("Native {{profile}} {{variant}} mode={{mode}}."),
        2400,
    );
    let mut rendered = template.replace("{{profile}}", &profile);
    rendered = rendered.replace("{{variant}}", &variant);
    rendered = rendered.replace("{{mode}}", &mode);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_variant_native_render",
        "profile": profile,
        "variant": variant,
        "mode": mode,
        "rendered_text": rendered,
        "row": defaults
    })
}

fn dashboard_prompt_variant_native_validate(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    )
    .to_ascii_lowercase();
    let row = dashboard_prompt_variant_native_defaults(&profile);
    let valid = row.is_some();
    let reasons = if valid {
        Vec::<String>::new()
    } else {
        vec!["native_profile_unknown".to_string()]
    };
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_variant_native_validate",
        "profile": profile,
        "valid": valid,
        "reasons": reasons
    })
}

fn dashboard_prompt_variant_native_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.variant.native.catalog" => {
            Some(dashboard_prompt_variant_native_catalog())
        }
        "dashboard.prompts.system.variant.native.resolve" => {
            Some(dashboard_prompt_variant_native_resolve(payload))
        }
        "dashboard.prompts.system.variant.native.render" => {
            Some(dashboard_prompt_variant_native_render(payload))
        }
        "dashboard.prompts.system.variant.native.validate" => {
            Some(dashboard_prompt_variant_native_validate(payload))
        }
        _ => dashboard_prompt_variant_tooling_storage_route_extension(root, normalized, payload),
    }
}

include!("021-dashboard-system-prompt-variant-tooling-storage-helpers.rs");
