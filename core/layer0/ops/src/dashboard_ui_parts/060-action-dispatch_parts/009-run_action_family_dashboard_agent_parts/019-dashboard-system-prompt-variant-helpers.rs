fn dashboard_prompt_variant_row_from_payload(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("generic"),
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
    let template = clean_text(
        payload
            .get("template")
            .and_then(Value::as_str)
            .unwrap_or(""),
        2400,
    );
    let overrides = payload
        .get("overrides")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let config = payload.get("config").cloned().unwrap_or_else(|| json!({}));
    json!({
        "profile": profile,
        "variant": variant,
        "template": template,
        "overrides": overrides,
        "config": config,
        "updated_at": crate::now_iso()
    })
}

fn dashboard_prompt_variant_key(profile: &str, variant: &str) -> String {
    format!("{profile}:{variant}")
}

fn dashboard_prompt_variant_upsert(root: &Path, payload: &Value) -> Value {
    let row = dashboard_prompt_variant_row_from_payload(payload);
    let profile = clean_text(row.get("profile").and_then(Value::as_str).unwrap_or(""), 80);
    let variant = clean_text(row.get("variant").and_then(Value::as_str).unwrap_or(""), 80);
    if profile.is_empty() || variant.is_empty() {
        return json!({
            "ok": false,
            "type": "dashboard_prompts_system_variant_upsert",
            "error": "profile_and_variant_required"
        });
    }
    let key = dashboard_prompt_variant_key(&profile, &variant);
    let state = dashboard_lpp_mutate_state(root, |state| {
        if !state
            .get("prompt_variants")
            .map(Value::is_object)
            .unwrap_or(false)
        {
            state["prompt_variants"] = json!({});
        }
        state["prompt_variants"][key.as_str()] = row.clone();
    });
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_variant_upsert",
        "key": key,
        "row": row,
        "state": state
    })
}

fn dashboard_prompt_variant_list(root: &Path, payload: &Value) -> Value {
    let filter_profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    )
    .to_ascii_lowercase();
    let state = dashboard_lpp_read_state(root);
    let mut rows = state
        .get("prompt_variants")
        .and_then(Value::as_object)
        .map(|map| {
            map.values()
                .filter(|row| {
                    if filter_profile.is_empty() {
                        true
                    } else {
                        clean_text(row.get("profile").and_then(Value::as_str).unwrap_or(""), 80)
                            .eq(&filter_profile)
                    }
                })
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    rows.sort_by(|a, b| {
        let left = dashboard_prompt_variant_key(
            &clean_text(a.get("profile").and_then(Value::as_str).unwrap_or(""), 80),
            &clean_text(a.get("variant").and_then(Value::as_str).unwrap_or(""), 80),
        );
        let right = dashboard_prompt_variant_key(
            &clean_text(b.get("profile").and_then(Value::as_str).unwrap_or(""), 80),
            &clean_text(b.get("variant").and_then(Value::as_str).unwrap_or(""), 80),
        );
        left.cmp(&right)
    });
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_variant_list",
        "count": rows.len() as i64,
        "rows": rows,
        "state": state
    })
}

fn dashboard_prompt_variant_resolve(root: &Path, payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("generic"),
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
    let key = dashboard_prompt_variant_key(&profile, &variant);
    let state = dashboard_lpp_read_state(root);
    let row = state
        .get("prompt_variants")
        .and_then(Value::as_object)
        .and_then(|map| map.get(&key).cloned())
        .unwrap_or_else(|| json!({}));
    let found = row.is_object() && !row.as_object().unwrap_or(&serde_json::Map::new()).is_empty();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_variant_resolve",
        "profile": profile,
        "variant": variant,
        "found": found,
        "row": row,
        "state": state
    })
}

fn dashboard_prompt_variant_render(root: &Path, payload: &Value) -> Value {
    let resolved = dashboard_prompt_variant_resolve(root, payload);
    let found = resolved.get("found").and_then(Value::as_bool).unwrap_or(false);
    if !found {
        return json!({
            "ok": false,
            "type": "dashboard_prompts_system_variant_render",
            "error": "variant_not_found",
            "profile": clean_text(payload.get("profile").and_then(Value::as_str).unwrap_or(""), 80),
            "variant": clean_text(payload.get("variant").and_then(Value::as_str).unwrap_or(""), 80)
        });
    }
    let row = resolved.get("row").cloned().unwrap_or_else(|| json!({}));
    let profile = clean_text(
        row.get("profile")
            .and_then(Value::as_str)
            .or_else(|| payload.get("profile").and_then(Value::as_str))
            .unwrap_or("generic"),
        80,
    );
    let variant = clean_text(
        row.get("variant")
            .and_then(Value::as_str)
            .or_else(|| payload.get("variant").and_then(Value::as_str))
            .unwrap_or("default"),
        80,
    );
    let template = clean_text(row.get("template").and_then(Value::as_str).unwrap_or(""), 2400);
    let context = payload.get("context").cloned().unwrap_or_else(|| json!({}));
    let context_mode = clean_text(context.get("mode").and_then(Value::as_str).unwrap_or("act"), 20);
    let rendered = if template.is_empty() {
        format!("Variant<{profile}:{variant}> mode={context_mode}")
    } else {
        let mut out = template.replace("{{profile}}", &profile);
        out = out.replace("{{variant}}", &variant);
        out = out.replace("{{mode}}", &context_mode);
        out
    };
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_variant_render",
        "profile": profile,
        "variant": variant,
        "rendered_text": rendered,
        "row": row
    })
}

fn dashboard_prompt_variant_validate(payload: &Value) -> Value {
    let row = dashboard_prompt_variant_row_from_payload(payload);
    let profile = clean_text(row.get("profile").and_then(Value::as_str).unwrap_or(""), 80);
    let variant = clean_text(row.get("variant").and_then(Value::as_str).unwrap_or(""), 80);
    let template = clean_text(row.get("template").and_then(Value::as_str).unwrap_or(""), 2400);
    let valid = !profile.is_empty()
        && !variant.is_empty()
        && (template.is_empty()
            || (template.contains("{{profile}}")
                || template.contains("{{variant}}")
                || template.contains("{{mode}}")));
    let mut reasons = Vec::<String>::new();
    if profile.is_empty() {
        reasons.push("profile_required".to_string());
    }
    if variant.is_empty() {
        reasons.push("variant_required".to_string());
    }
    if !template.is_empty()
        && !(template.contains("{{profile}}")
            || template.contains("{{variant}}")
            || template.contains("{{mode}}"))
    {
        reasons.push("template_requires_known_tokens".to_string());
    }
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_variant_validate",
        "valid": valid,
        "reasons": reasons,
        "profile": profile,
        "variant": variant
    })
}

fn dashboard_prompt_variant_defaults_for_profile(profile: &str) -> Value {
    match profile {
        "devstral" => json!({
            "template": "You are {{profile}}::{{variant}} mode={{mode}}. Prioritize deterministic tooling synthesis.",
            "config": {"temperature": 0.2, "top_p": 0.9},
            "overrides": {"max_output_tokens": 700}
        }),
        "gemini-3" => json!({
            "template": "You are {{profile}} {{variant}} mode={{mode}}. Keep responses grounded and compact.",
            "config": {"temperature": 0.3, "top_p": 0.95},
            "overrides": {"max_output_tokens": 800}
        }),
        "glm" => json!({
            "template": "You are {{profile}}/{{variant}} in {{mode}} mode. Use explicit evidence-first synthesis.",
            "config": {"temperature": 0.25, "top_p": 0.9},
            "overrides": {"max_output_tokens": 900}
        }),
        "gpt-5" => json!({
            "template": "You are {{profile}} variant={{variant}} mode={{mode}}. Enforce fail-closed policy contracts.",
            "config": {"temperature": 0.1, "top_p": 0.85},
            "overrides": {"max_output_tokens": 1000}
        }),
        "hermes" => json!({
            "template": "You are {{profile}} {{variant}} mode={{mode}}. Favor concise structure and stable tool routing.",
            "config": {"temperature": 0.35, "top_p": 0.95},
            "overrides": {"max_output_tokens": 850}
        }),
        _ => json!({
            "template": "You are {{profile}}::{{variant}} mode={{mode}}.",
            "config": {"temperature": 0.2, "top_p": 0.9},
            "overrides": {"max_output_tokens": 800}
        }),
    }
}

fn dashboard_prompt_variant_profile_defaults(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("generic"),
        80,
    )
    .to_ascii_lowercase();
    let defaults = dashboard_prompt_variant_defaults_for_profile(&profile);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_variant_profile_defaults",
        "profile": profile,
        "defaults": defaults
    })
}

fn dashboard_prompt_variant_compose_from_profile(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("generic"),
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
            .or_else(|| payload.pointer("/context/mode").and_then(Value::as_str))
            .unwrap_or("act"),
        20,
    )
    .to_ascii_lowercase();
    let defaults = dashboard_prompt_variant_defaults_for_profile(&profile);
    let template = clean_text(
        payload
            .get("template")
            .and_then(Value::as_str)
            .or_else(|| defaults.get("template").and_then(Value::as_str))
            .unwrap_or("You are {{profile}} {{variant}} mode={{mode}}."),
        2400,
    );
    let mut rendered = template.replace("{{profile}}", &profile);
    rendered = rendered.replace("{{variant}}", &variant);
    rendered = rendered.replace("{{mode}}", &mode);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_variant_compose_from_profile",
        "profile": profile,
        "variant": variant,
        "mode": mode,
        "rendered_text": rendered,
        "defaults": defaults
    })
}

fn dashboard_prompt_variant_builder_preview(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("generic"),
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
    let components = payload
        .get("components")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|raw| clean_text(raw, 80).to_ascii_lowercase()))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    let component_lines = if components.is_empty() {
        "components: objective,rules".to_string()
    } else {
        format!("components: {}", components.join(","))
    };
    let template = format!(
        "You are {{profile}}::{{variant}} mode={{mode}}.\n{component_lines}\nsource=variant_builder"
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_variant_builder_preview",
        "profile": profile,
        "variant": variant,
        "components": components,
        "template": template
    })
}

fn dashboard_prompt_variant_builder_validate(payload: &Value) -> Value {
    let preview = dashboard_prompt_variant_builder_preview(payload);
    let components = preview
        .get("components")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let valid = !components.is_empty();
    let reasons = if valid {
        Vec::<String>::new()
    } else {
        vec!["components_required".to_string()]
    };
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_variant_builder_validate",
        "valid": valid,
        "reasons": reasons,
        "preview": preview
    })
}

fn dashboard_prompt_variant_index(payload: &Value) -> Value {
    let requested_profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    )
    .to_ascii_lowercase();
    let profiles = vec![
        "devstral".to_string(),
        "gemini-3".to_string(),
        "glm".to_string(),
        "gpt-5".to_string(),
        "hermes".to_string(),
        "generic".to_string(),
    ];
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_variant_index",
        "profiles": profiles,
        "requested_profile": requested_profile
    })
}

fn dashboard_prompt_variant_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.variant.index" => Some(dashboard_prompt_variant_index(payload)),
        "dashboard.prompts.system.variant.profileDefaults" => {
            Some(dashboard_prompt_variant_profile_defaults(payload))
        }
        "dashboard.prompts.system.variant.composeFromProfile" => {
            Some(dashboard_prompt_variant_compose_from_profile(payload))
        }
        "dashboard.prompts.system.variant.builder.preview" => {
            Some(dashboard_prompt_variant_builder_preview(payload))
        }
        "dashboard.prompts.system.variant.builder.validate" => {
            Some(dashboard_prompt_variant_builder_validate(payload))
        }
        "dashboard.prompts.system.variant.upsert" => Some(dashboard_prompt_variant_upsert(root, payload)),
        "dashboard.prompts.system.variant.list" => Some(dashboard_prompt_variant_list(root, payload)),
        "dashboard.prompts.system.variant.resolve" => Some(dashboard_prompt_variant_resolve(root, payload)),
        "dashboard.prompts.system.variant.render" => Some(dashboard_prompt_variant_render(root, payload)),
        "dashboard.prompts.system.variant.validate" => Some(dashboard_prompt_variant_validate(payload)),
        _ => dashboard_prompt_variant_native_route_extension(root, normalized, payload),
    }
}

fn dashboard_prompt_route_supported(normalized: &str) -> bool {
    matches!(
        normalized,
        "dashboard.locks.acquire"
            | "dashboard.locks.release"
            | "dashboard.locks.status"
            | "dashboard.locks.list"
            | "dashboard.mentions.extract"
            | "dashboard.permissions.setPolicy"
            | "dashboard.permissions.getPolicy"
            | "dashboard.permissions.evaluateCommand"
            | "dashboard.prompts.context.manage"
            | "dashboard.prompts.loadMcpDocumentation"
            | "dashboard.prompts.response.compose"
    ) || normalized.starts_with("dashboard.prompts.system.")
}

include!("020-dashboard-system-prompt-native-variant-helpers.rs");
