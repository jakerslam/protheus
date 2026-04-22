fn dashboard_prompt_variant_profile_class(profile: &str) -> &'static str {
    match profile {
        "native-gpt-5-1" | "native-gpt-5" | "native-next-gen" => "native",
        "next-gen" | "gpt-5" | "gemini-3" | "glm" | "devstral" | "hermes" => "next_gen",
        "trinity" => "trinity",
        "xs" => "xs",
        "generic" => "generic",
        _ => "unknown",
    }
}

fn dashboard_prompt_variant_classify_profile(payload: &Value) -> Value {
    let profile = clean_text(
        payload.get("profile").and_then(Value::as_str).unwrap_or(""),
        80,
    )
    .to_ascii_lowercase();
    let class_name = dashboard_prompt_variant_profile_class(&profile);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_variant_classify_profile",
        "profile": profile,
        "profile_class": class_name,
        "known": class_name != "unknown"
    })
}

fn dashboard_prompt_variant_template_render_strict(payload: &Value) -> Value {
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
        payload.get("mode").and_then(Value::as_str).unwrap_or("act"),
        20,
    )
    .to_ascii_lowercase();
    let template = clean_text(
        payload
            .get("template")
            .and_then(Value::as_str)
            .unwrap_or(""),
        2400,
    );
    let has_tokens = template.contains("{{profile}}")
        || template.contains("{{variant}}")
        || template.contains("{{mode}}");
    if !has_tokens {
        return json!({
            "ok": false,
            "type": "dashboard_prompts_system_variant_template_render_strict",
            "error": "strict_template_tokens_required",
            "profile": profile,
            "variant": variant
        });
    }
    let mut rendered = template.replace("{{profile}}", &profile);
    rendered = rendered.replace("{{variant}}", &variant);
    rendered = rendered.replace("{{mode}}", &mode);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_variant_template_render_strict",
        "profile": profile,
        "variant": variant,
        "mode": mode,
        "rendered_text": rendered
    })
}

fn dashboard_prompt_variant_merge_overrides(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("generic"),
        80,
    )
    .to_ascii_lowercase();
    let base = dashboard_prompt_variant_defaults_for_profile(&profile);
    let mut merged = base
        .get("overrides")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let incoming = payload
        .get("overrides")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for (key, value) in incoming {
        merged[key.as_str()] = value;
    }
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_variant_overrides_merge",
        "profile": profile,
        "merged_overrides": merged
    })
}

fn dashboard_prompt_variant_validator_audit_matrix(payload: &Value) -> Value {
    let rows = payload
        .get("rows")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut audit = Vec::<Value>::new();
    let mut invalid_count = 0_i64;
    for row in rows {
        let profile = clean_text(row.get("profile").and_then(Value::as_str).unwrap_or(""), 80)
            .to_ascii_lowercase();
        let variant = clean_text(row.get("variant").and_then(Value::as_str).unwrap_or(""), 80)
            .to_ascii_lowercase();
        let template = clean_text(row.get("template").and_then(Value::as_str).unwrap_or(""), 2400);
        let profile_class = dashboard_prompt_variant_profile_class(&profile);
        let known = profile_class != "unknown";
        let template_valid = template.is_empty()
            || template.contains("{{profile}}")
            || template.contains("{{variant}}")
            || template.contains("{{mode}}");
        let valid = known && !variant.is_empty() && template_valid;
        if !valid {
            invalid_count = invalid_count.saturating_add(1);
        }
        audit.push(json!({
            "profile": profile,
            "variant": variant,
            "profile_class": profile_class,
            "known_profile": known,
            "template_valid": template_valid,
            "valid": valid
        }));
    }
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_variant_validator_audit_matrix",
        "rows": audit,
        "invalid_count": invalid_count,
        "valid": invalid_count == 0
    })
}

fn dashboard_prompt_storage_snapshot(root: &Path) -> Value {
    let state = dashboard_lpp_read_state(root);
    let registry = state
        .get("prompt_registry")
        .and_then(Value::as_object)
        .map(|m| m.len() as i64)
        .unwrap_or(0);
    let variants = state
        .get("prompt_variants")
        .and_then(Value::as_object)
        .map(|m| m.len() as i64)
        .unwrap_or(0);
    let toolsets = state
        .get("prompt_toolsets")
        .and_then(Value::as_object)
        .map(|m| m.len() as i64)
        .unwrap_or(0);
    let bytes = serde_json::to_vec(&state)
        .map(|buf| buf.len() as i64)
        .unwrap_or(0);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_storage_snapshot",
        "storage": {
            "prompt_registry": registry,
            "prompt_variants": variants,
            "prompt_toolsets": toolsets,
            "state_bytes": bytes
        },
        "state": state
    })
}

fn dashboard_prompt_storage_error_message(payload: &Value) -> Value {
    let code = clean_text(
        payload.get("code").and_then(Value::as_str).unwrap_or("unknown"),
        120,
    )
    .to_ascii_lowercase();
    let message = match code.as_str() {
        "remote_fetch_failed" => "Remote config fetch failed; using local fail-closed defaults.",
        "storage_missing" => "Prompt storage is missing required structures; reinitialize registry and variants.",
        "template_invalid" => "Variant template is invalid under strict token policy.",
        _ => "Unknown prompt storage error.",
    };
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_storage_error_message",
        "code": code,
        "message": message
    })
}

fn dashboard_prompt_storage_remote_seed(root: &Path, payload: &Value) -> Value {
    let source = clean_text(
        payload
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("manual"),
        120,
    );
    let etag = clean_text(payload.get("etag").and_then(Value::as_str).unwrap_or(""), 120);
    let config = payload.get("config").cloned().unwrap_or_else(|| json!({}));
    let state = dashboard_lpp_mutate_state(root, |state| {
        state["remote_prompt_config"] = json!({
            "source": source,
            "etag": etag,
            "config": config,
            "seeded_at": crate::now_iso()
        });
    });
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_storage_remote_config_seed",
        "state": state
    })
}

fn dashboard_prompt_storage_remote_fetch(root: &Path) -> Value {
    let state = dashboard_lpp_read_state(root);
    let remote = state
        .get("remote_prompt_config")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let has_remote = remote.is_object() && !remote.as_object().unwrap_or(&serde_json::Map::new()).is_empty();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_storage_remote_config_fetch",
        "has_remote": has_remote,
        "remote": remote,
        "state": state
    })
}

fn dashboard_prompt_variant_tooling_storage_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.variant.classifyProfile" => {
            Some(dashboard_prompt_variant_classify_profile(payload))
        }
        "dashboard.prompts.system.variant.template.renderStrict" => {
            Some(dashboard_prompt_variant_template_render_strict(payload))
        }
        "dashboard.prompts.system.variant.overrides.merge" => {
            Some(dashboard_prompt_variant_merge_overrides(payload))
        }
        "dashboard.prompts.system.variant.validator.auditMatrix" => {
            Some(dashboard_prompt_variant_validator_audit_matrix(payload))
        }
        "dashboard.prompts.system.storage.snapshot" => {
            Some(dashboard_prompt_storage_snapshot(root))
        }
        "dashboard.prompts.system.storage.errorMessage" => {
            Some(dashboard_prompt_storage_error_message(payload))
        }
        "dashboard.prompts.system.storage.remoteConfig.seed" => {
            Some(dashboard_prompt_storage_remote_seed(root, payload))
        }
        "dashboard.prompts.system.storage.remoteConfig.fetch" => {
            Some(dashboard_prompt_storage_remote_fetch(root))
        }
        _ => dashboard_prompt_storage_task_route_extension(root, normalized, payload),
    }
}

include!("022-dashboard-system-prompt-storage-task-helpers.rs");
