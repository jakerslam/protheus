fn dashboard_prompt_component_act_vs_plan_mode_describe(payload: &Value) -> Value {
    let mode = clean_text(
        payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("act"),
        80,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_component_act_vs_plan_mode_describe",
        "mode": mode
    })
}

fn dashboard_prompt_component_capabilities_describe(payload: &Value) -> Value {
    let capability_scope = clean_text(
        payload
            .get("capability_scope")
            .and_then(Value::as_str)
            .unwrap_or("core"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_component_capabilities_describe",
        "capability_scope": capability_scope
    })
}

fn dashboard_prompt_component_editing_files_describe(payload: &Value) -> Value {
    let edit_policy = clean_text(
        payload
            .get("edit_policy")
            .and_then(Value::as_str)
            .unwrap_or("safe"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_component_editing_files_describe",
        "edit_policy": edit_policy
    })
}

fn dashboard_prompt_component_feedback_describe(payload: &Value) -> Value {
    let feedback_tone = clean_text(
        payload
            .get("feedback_tone")
            .and_then(Value::as_str)
            .unwrap_or("direct"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_component_feedback_describe",
        "feedback_tone": feedback_tone
    })
}

fn dashboard_prompt_component_index_describe(payload: &Value) -> Value {
    let index_scope = clean_text(
        payload
            .get("index_scope")
            .and_then(Value::as_str)
            .unwrap_or("components"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_component_index_describe",
        "index_scope": index_scope
    })
}

fn dashboard_prompt_component_mcp_describe(payload: &Value) -> Value {
    let mcp_mode = clean_text(
        payload
            .get("mcp_mode")
            .and_then(Value::as_str)
            .unwrap_or("connected"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_component_mcp_describe",
        "mcp_mode": mcp_mode
    })
}

fn dashboard_prompt_component_objective_describe(payload: &Value) -> Value {
    let objective_mode = clean_text(
        payload
            .get("objective_mode")
            .and_then(Value::as_str)
            .unwrap_or("deliverable"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_component_objective_describe",
        "objective_mode": objective_mode
    })
}

fn dashboard_prompt_component_rules_describe(payload: &Value) -> Value {
    let ruleset = clean_text(
        payload
            .get("ruleset")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_component_rules_describe",
        "ruleset": ruleset
    })
}

fn dashboard_prompt_component_skills_describe(payload: &Value) -> Value {
    let skill_scope = clean_text(
        payload
            .get("skill_scope")
            .and_then(Value::as_str)
            .unwrap_or("active"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_component_skills_describe",
        "skill_scope": skill_scope
    })
}

fn dashboard_prompt_component_system_info_describe(payload: &Value) -> Value {
    let info_scope = clean_text(
        payload
            .get("info_scope")
            .and_then(Value::as_str)
            .unwrap_or("runtime"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_component_system_info_describe",
        "info_scope": info_scope
    })
}

fn dashboard_prompt_system_prompt_components_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.components.actVsPlanMode.describe" => {
            Some(dashboard_prompt_component_act_vs_plan_mode_describe(payload))
        }
        "dashboard.prompts.system.components.capabilities.describe" => {
            Some(dashboard_prompt_component_capabilities_describe(payload))
        }
        "dashboard.prompts.system.components.editingFiles.describe" => {
            Some(dashboard_prompt_component_editing_files_describe(payload))
        }
        "dashboard.prompts.system.components.feedback.describe" => {
            Some(dashboard_prompt_component_feedback_describe(payload))
        }
        "dashboard.prompts.system.components.index.describe" => {
            Some(dashboard_prompt_component_index_describe(payload))
        }
        "dashboard.prompts.system.components.mcp.describe" => {
            Some(dashboard_prompt_component_mcp_describe(payload))
        }
        "dashboard.prompts.system.components.objective.describe" => {
            Some(dashboard_prompt_component_objective_describe(payload))
        }
        "dashboard.prompts.system.components.rules.describe" => {
            Some(dashboard_prompt_component_rules_describe(payload))
        }
        "dashboard.prompts.system.components.skills.describe" => {
            Some(dashboard_prompt_component_skills_describe(payload))
        }
        "dashboard.prompts.system.components.systemInfo.describe" => {
            Some(dashboard_prompt_component_system_info_describe(payload))
        }
        _ => dashboard_prompt_system_prompt_registry_templates_tail_route_extension(
            root,
            normalized,
            payload,
        ),
    }
}

include!("064-dashboard-system-prompt-registry-templates-tail-helpers.rs");
