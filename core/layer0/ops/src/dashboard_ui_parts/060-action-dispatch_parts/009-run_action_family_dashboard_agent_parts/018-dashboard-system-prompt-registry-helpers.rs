fn dashboard_prompt_system_component_text_extension(component: &str, payload: &Value) -> String {
    match component {
        "skills" => {
            let skills = payload
                .get("skills")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|row| row.as_str().map(|raw| clean_text(raw, 120)))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>();
            if skills.is_empty() {
                "Skills: use available tools conservatively and prefer deterministic outputs."
                    .to_string()
            } else {
                format!("Skills: {}.", skills.join(", "))
            }
        }
        "system_info" => {
            let os = clean_text(payload.get("os").and_then(Value::as_str).unwrap_or("unknown"), 60);
            let timezone = clean_text(
                payload
                    .get("timezone")
                    .and_then(Value::as_str)
                    .unwrap_or("UTC"),
                60,
            );
            let model = clean_text(
                payload
                    .get("model")
                    .and_then(Value::as_str)
                    .unwrap_or("runtime-default"),
                120,
            );
            format!("System Info: os={os}; timezone={timezone}; model={model}.")
        }
        "task_progress" => {
            let completed = payload
                .get("completed")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .max(0);
            let total = payload
                .get("total")
                .and_then(Value::as_i64)
                .unwrap_or(completed)
                .max(1);
            let percent = ((completed as f64 / total as f64) * 100.0).round() as i64;
            format!(
                "Task Progress: {}/{} completed ({}%).",
                completed.min(total),
                total,
                percent.clamp(0, 100)
            )
        }
        "user_instructions" => {
            let text = clean_text(
                payload
                    .get("user_instructions")
                    .and_then(Value::as_str)
                    .or_else(|| payload.get("instructions").and_then(Value::as_str))
                    .unwrap_or(""),
                1200,
            );
            if text.is_empty() {
                "User Instructions: honor explicit constraints and avoid implicit assumptions."
                    .to_string()
            } else {
                format!("User Instructions: {text}")
            }
        }
        "constants" => {
            let mut rows = payload
                .get("constants")
                .and_then(Value::as_object)
                .map(|map| {
                    map.iter()
                        .map(|(k, v)| {
                            let key = clean_text(k, 80).to_ascii_uppercase();
                            let value = clean_text(v.as_str().unwrap_or(""), 120);
                            format!("{key}={value}")
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            rows.sort();
            if rows.is_empty() {
                "Constants: none.".to_string()
            } else {
                format!("Constants: {}.", rows.join("; "))
            }
        }
        "index" => {
            let mut components = dashboard_prompt_system_default_components();
            components.extend(
                [
                    "skills",
                    "system_info",
                    "task_progress",
                    "user_instructions",
                    "constants",
                    "index",
                ]
                .iter()
                .map(|s| (*s).to_string()),
            );
            components.sort();
            components.dedup();
            format!("Component Index: {}.", components.join(", "))
        }
        "spec" => {
            "Spec: required fields are profile + components; unknown components are rejected."
                .to_string()
        }
        _ => String::new(),
    }
}

fn dashboard_prompt_registry_rows(state: &Value) -> Vec<Value> {
    let mut rows = state
        .get("prompt_registry")
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .map(|(key, row)| {
                    let mut out = row.clone();
                    if !out.get("registry_key").map(Value::is_string).unwrap_or(false) {
                        out["registry_key"] = Value::String(clean_text(key, 120));
                    }
                    out
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    rows.sort_by(|a, b| {
        clean_text(
            a.get("registry_key").and_then(Value::as_str).unwrap_or(""),
            120,
        )
        .cmp(&clean_text(
            b.get("registry_key").and_then(Value::as_str).unwrap_or(""),
            120,
        ))
    });
    rows
}

fn dashboard_prompt_registry_upsert(root: &Path, payload: &Value) -> Value {
    let key = clean_text(
        payload
            .get("registry_key")
            .and_then(Value::as_str)
            .or_else(|| payload.get("key").and_then(Value::as_str))
            .or_else(|| payload.get("profile").and_then(Value::as_str))
            .unwrap_or(""),
        120,
    )
    .to_ascii_lowercase();
    if key.is_empty() {
        return json!({
            "ok": false,
            "type": "dashboard_prompts_system_registry_upsert",
            "error": "registry_key_required"
        });
    }
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("legacy_compact"),
        80,
    )
    .to_ascii_lowercase();
    let components = dashboard_prompt_system_components_from_payload(payload);
    let objective = clean_text(
        payload
            .get("objective")
            .and_then(Value::as_str)
            .unwrap_or(""),
        600,
    );
    let rules = payload
        .get("rules")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mcp_policy = clean_text(
        payload
            .get("mcp_policy")
            .and_then(Value::as_str)
            .unwrap_or("use_mcp_when_available"),
        140,
    );

    let row = json!({
        "registry_key": key,
        "profile": profile,
        "components": components,
        "objective": objective,
        "rules": rules,
        "mcp_policy": mcp_policy,
        "updated_at": crate::now_iso()
    });
    let state = dashboard_lpp_mutate_state(root, |state| {
        if !state
            .get("prompt_registry")
            .map(Value::is_object)
            .unwrap_or(false)
        {
            state["prompt_registry"] = json!({});
        }
        state["prompt_registry"][key.as_str()] = row.clone();
    });
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_registry_upsert",
        "registry_key": key,
        "entry": row,
        "state": state
    })
}

fn dashboard_prompt_registry_list(root: &Path) -> Value {
    let state = dashboard_lpp_read_state(root);
    let rows = dashboard_prompt_registry_rows(&state);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_registry_list",
        "entries": rows.clone(),
        "count": rows.len() as i64,
        "state": state
    })
}

fn dashboard_prompt_registry_upsert_toolset(root: &Path, payload: &Value) -> Value {
    let toolset_id = clean_text(
        payload
            .get("toolset_id")
            .and_then(Value::as_str)
            .or_else(|| payload.get("id").and_then(Value::as_str))
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    let tools = payload
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|raw| clean_text(raw, 120)))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    let state = dashboard_lpp_mutate_state(root, |state| {
        if !state.get("prompt_toolsets").map(Value::is_object).unwrap_or(false) {
            state["prompt_toolsets"] = json!({});
        }
        state["prompt_toolsets"][toolset_id.as_str()] = json!({
            "toolset_id": toolset_id,
            "tools": tools,
            "updated_at": crate::now_iso()
        });
    });
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_registry_upsert_toolset",
        "toolset_id": toolset_id,
        "state": state
    })
}

fn dashboard_prompt_registry_build(root: &Path, payload: &Value) -> Value {
    let key = clean_text(
        payload
            .get("registry_key")
            .and_then(Value::as_str)
            .or_else(|| payload.get("key").and_then(Value::as_str))
            .unwrap_or(""),
        120,
    )
    .to_ascii_lowercase();
    if key.is_empty() {
        return json!({
            "ok": false,
            "type": "dashboard_prompts_system_registry_build",
            "error": "registry_key_required"
        });
    }
    let state = dashboard_lpp_read_state(root);
    let entry = state
        .get("prompt_registry")
        .and_then(Value::as_object)
        .and_then(|map| map.get(&key).cloned())
        .unwrap_or_else(|| json!({}));
    let compose_payload = json!({
        "profile": payload.get("profile").cloned().or_else(|| entry.get("profile").cloned()).unwrap_or_else(|| json!("legacy_compact")),
        "components": payload.get("components").cloned().or_else(|| entry.get("components").cloned()).unwrap_or_else(|| json!(dashboard_prompt_system_default_components())),
        "objective": payload.get("objective").cloned().or_else(|| entry.get("objective").cloned()).unwrap_or_else(|| json!("Maintain deterministic output quality.")),
        "rules": payload.get("rules").cloned().or_else(|| entry.get("rules").cloned()).unwrap_or_else(|| json!(["fail closed"])),
        "mcp_policy": payload.get("mcp_policy").cloned().or_else(|| entry.get("mcp_policy").cloned()).unwrap_or_else(|| json!("use_mcp_when_available")),
        "mode": payload.get("mode").cloned().unwrap_or_else(|| json!("act"))
    });
    let composed = dashboard_prompts_system_compose(root, &compose_payload);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_registry_build",
        "registry_key": key,
        "compose": composed
    })
}

fn dashboard_prompts_system_spec_validate(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    let components = dashboard_prompt_system_components_from_payload(payload);
    let mut unknown = Vec::<String>::new();
    for component in &components {
        let rendered = dashboard_prompt_system_component_text(component, payload);
        if rendered.is_empty() {
            unknown.push(component.clone());
        }
    }
    let valid = !profile.is_empty() && !components.is_empty() && unknown.is_empty();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_spec_validate",
        "valid": valid,
        "profile": profile,
        "component_count": components.len() as i64,
        "unknown_components": unknown
    })
}

fn dashboard_prompts_system_index(payload: &Value) -> Value {
    let mut components = dashboard_prompt_system_default_components();
    components.extend(
        [
            "skills",
            "system_info",
            "task_progress",
            "user_instructions",
            "constants",
            "index",
            "spec",
        ]
        .iter()
        .map(|s| (*s).to_string()),
    );
    components.sort();
    components.dedup();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_index",
        "components": components,
        "requested_profile": clean_text(payload.get("profile").and_then(Value::as_str).unwrap_or(""), 80)
    })
}

fn dashboard_prompt_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.index" => Some(dashboard_prompts_system_index(payload)),
        "dashboard.prompts.system.registry.upsert" => {
            Some(dashboard_prompt_registry_upsert(root, payload))
        }
        "dashboard.prompts.system.registry.list" => Some(dashboard_prompt_registry_list(root)),
        "dashboard.prompts.system.registry.build" => {
            Some(dashboard_prompt_registry_build(root, payload))
        }
        "dashboard.prompts.system.registry.upsertToolSet" => {
            Some(dashboard_prompt_registry_upsert_toolset(root, payload))
        }
        "dashboard.prompts.system.spec.validate" => {
            Some(dashboard_prompts_system_spec_validate(payload))
        }
        _ => dashboard_prompt_variant_route_extension(root, normalized, payload),
    }
}

include!("019-dashboard-system-prompt-variant-helpers.rs");
