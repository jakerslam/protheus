fn dashboard_prompt_context_manage(root: &Path, payload: &Value) -> Value {
    let op = clean_text(
        payload
            .get("op")
            .and_then(Value::as_str)
            .or_else(|| payload.get("action").and_then(Value::as_str))
            .unwrap_or("set"),
        20,
    )
    .to_ascii_lowercase();
    let key = clean_text(
        payload
            .get("key")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    let value = payload.get("value").cloned().unwrap_or(Value::Null);
    let state = dashboard_lpp_mutate_state(root, |state| match op.as_str() {
        "delete" => {
            if let Some(ctx) = state.get_mut("prompt_context").and_then(Value::as_object_mut) {
                ctx.remove(&key);
            }
        }
        "set" => {
            state["prompt_context"][key.as_str()] = value.clone();
        }
        _ => {}
    });
    let context_count = state
        .get("prompt_context")
        .and_then(Value::as_object)
        .map(|m| m.len() as i64)
        .unwrap_or(0);
    json!({
        "ok": true,
        "type": "dashboard_prompts_context_manage",
        "op": op,
        "key": key,
        "context_count": context_count,
        "context": state.get("prompt_context").cloned().unwrap_or_else(|| json!({})),
        "state": state
    })
}

fn dashboard_prompts_load_mcp_documentation(root: &Path, payload: &Value) -> Value {
    let docs = payload
        .get("docs")
        .or_else(|| payload.get("mcp_ids"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|raw| clean_text(raw, 140)))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    let state = dashboard_lpp_mutate_state(root, |state| {
        for doc in &docs {
            state["mcp_docs"][doc.as_str()] = json!({
                "loaded": true,
                "loaded_at": crate::now_iso(),
                "source": "dashboard.prompts.mcp"
            });
        }
        state["mcp_doc_load_count"] = Value::from(
            i64_from_value(state.get("mcp_doc_load_count"), 0).saturating_add(docs.len() as i64),
        );
    });
    json!({
        "ok": true,
        "type": "dashboard_prompts_load_mcp_documentation",
        "loaded_ids": docs,
        "loaded_count": state.get("mcp_docs").and_then(Value::as_object).map(|m| m.len() as i64).unwrap_or(0),
        "state": state
    })
}

fn dashboard_prompts_response_compose(root: &Path, payload: &Value) -> Value {
    let tone = clean_text(
        payload
            .get("tone")
            .and_then(Value::as_str)
            .or_else(|| payload.get("style").and_then(Value::as_str))
            .unwrap_or("direct"),
        40,
    );
    let summary = clean_text(
        payload
            .get("summary")
            .and_then(Value::as_str)
            .or_else(|| payload.get("message").and_then(Value::as_str))
            .unwrap_or(""),
        1200,
    );
    let bullets = payload
        .get("bullets")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|raw| clean_text(raw, 200)))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    let mut response = summary.clone();
    if !bullets.is_empty() {
        let mut parts = Vec::<String>::new();
        if !summary.is_empty() {
            parts.push(summary.clone());
        }
        for row in &bullets {
            parts.push(format!("- {}", row));
        }
        response = parts.join("\n");
    }
    let state = dashboard_lpp_mutate_state(root, |state| {
        state["last_composed_response"] = json!({
            "tone": tone,
            "response": response,
            "summary": summary,
            "bullets": bullets,
            "at": crate::now_iso()
        });
    });
    json!({
        "ok": true,
        "type": "dashboard_prompts_response_compose",
        "tone": tone,
        "response": response,
        "state": state
    })
}

fn dashboard_prompt_system_component_text(component: &str, payload: &Value) -> String {
    match component {
        "act_vs_plan_mode" => {
            let mode = clean_text(
                payload
                    .get("mode")
                    .and_then(Value::as_str)
                    .unwrap_or("act"),
                20,
            )
            .to_ascii_lowercase();
            if mode == "plan" {
                "Mode: PLAN. Produce explicit plan first; do not execute until requested.".to_string()
            } else {
                "Mode: ACT. Execute scoped changes directly with clear status updates.".to_string()
            }
        }
        "capabilities" => {
            let values = payload
                .get("capabilities")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|row| row.as_str().map(|raw| clean_text(raw, 120)))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>();
            if values.is_empty() {
                "Capabilities: file read/write, patching, terminal command execution.".to_string()
            } else {
                format!("Capabilities: {}.", values.join(", "))
            }
        }
        "editing_files" => {
            let max_files = payload
                .get("max_files")
                .and_then(Value::as_i64)
                .unwrap_or(10)
                .clamp(1, 500);
            format!(
                "Editing Files: prefer minimal diffs, preserve semantics, and keep edits scoped (max_files={max_files})."
            )
        }
        "feedback" => {
            "Feedback: report concrete deltas and blockers; avoid placeholder acknowledgements.".to_string()
        }
        "mcp" => {
            let policy = clean_text(
                payload
                    .get("mcp_policy")
                    .and_then(Value::as_str)
                    .unwrap_or("use_mcp_when_available"),
                120,
            );
            format!("MCP Policy: {policy}.")
        }
        "objective" => {
            let objective = clean_text(
                payload
                    .get("objective")
                    .and_then(Value::as_str)
                    .unwrap_or("Maintain reliable, fail-closed execution."),
                600,
            );
            format!("Objective: {objective}")
        }
        "rules" => {
            let rules = payload
                .get("rules")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|row| row.as_str().map(|raw| clean_text(raw, 180)))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>();
            if rules.is_empty() {
                "Rules: fail closed; preserve authority boundaries; keep output concise.".to_string()
            } else {
                format!("Rules: {}", rules.join(" | "))
            }
        }
        _ => dashboard_prompt_system_component_text_extension(component, payload),
    }
}

fn dashboard_prompt_system_default_components() -> Vec<String> {
    vec![
        "objective".to_string(),
        "rules".to_string(),
        "capabilities".to_string(),
        "editing_files".to_string(),
        "mcp".to_string(),
        "feedback".to_string(),
        "act_vs_plan_mode".to_string(),
    ]
}

fn dashboard_prompt_system_components_from_payload(payload: &Value) -> Vec<String> {
    let components = payload
        .get("components")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|raw| clean_text(raw, 60).to_ascii_lowercase()))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    if components.is_empty() {
        dashboard_prompt_system_default_components()
    } else {
        let mut uniq = std::collections::BTreeSet::<String>::new();
        for row in components {
            uniq.insert(row);
        }
        uniq.into_iter().collect::<Vec<_>>()
    }
}

fn dashboard_prompts_system_component(payload: &Value) -> Value {
    let component = clean_text(
        payload
            .get("component")
            .and_then(Value::as_str)
            .or_else(|| payload.get("name").and_then(Value::as_str))
            .unwrap_or(""),
        60,
    )
    .to_ascii_lowercase();
    if component.is_empty() {
        return json!({
            "ok": false,
            "type": "dashboard_prompts_system_component",
            "error": "component_required"
        });
    }
    let text = dashboard_prompt_system_component_text(&component, payload);
    if text.is_empty() {
        return json!({
            "ok": false,
            "type": "dashboard_prompts_system_component",
            "error": "unknown_component",
            "component": component
        });
    }
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_component",
        "component": component,
        "text": text
    })
}

fn dashboard_prompts_system_compose(root: &Path, payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("legacy_compact"),
        80,
    )
    .to_ascii_lowercase();
    let profile_header = if profile == "gpt5" || profile == "gpt-5" {
        "System Prompt Profile: GPT-5 Next-Gen"
    } else {
        "System Prompt Profile: Legacy Compact"
    };
    let components = dashboard_prompt_system_components_from_payload(payload);
    let mut rendered = Vec::<String>::new();
    for component in &components {
        let text = dashboard_prompt_system_component_text(component, payload);
        if !text.is_empty() {
            rendered.push(format!("[{component}] {text}"));
        }
    }
    let body = rendered.join("\n");
    let prompt_text = format!("{profile_header}\n{body}");
    let state = dashboard_lpp_mutate_state(root, |state| {
        state["last_system_prompt_compose"] = json!({
            "profile": profile,
            "components": components,
            "prompt_text": prompt_text,
            "at": crate::now_iso()
        });
    });
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_compose",
        "profile": profile,
        "components": rendered.len() as i64,
        "component_keys": components,
        "prompt_text": prompt_text,
        "state": state
    })
}

fn dashboard_lock_permission_prompt_route(root: &Path, normalized: &str, payload: &Value) -> Value {
    match normalized {
        "dashboard.locks.acquire" => dashboard_lock_acquire(root, payload),
        "dashboard.locks.release" => dashboard_lock_release(root, payload),
        "dashboard.locks.status" => dashboard_lock_status(root, payload),
        "dashboard.locks.list" => dashboard_locks_list(root),
        "dashboard.mentions.extract" => dashboard_mentions_extract(root, payload),
        "dashboard.permissions.setPolicy" => dashboard_permissions_set_policy(root, payload),
        "dashboard.permissions.getPolicy" => dashboard_permissions_get_policy(root),
        "dashboard.permissions.evaluateCommand" => dashboard_permissions_evaluate_command(root, payload),
        "dashboard.prompts.context.manage" => dashboard_prompt_context_manage(root, payload),
        "dashboard.prompts.loadMcpDocumentation" => dashboard_prompts_load_mcp_documentation(root, payload),
        "dashboard.prompts.response.compose" => dashboard_prompts_response_compose(root, payload),
        "dashboard.prompts.system.component" => dashboard_prompts_system_component(payload),
        "dashboard.prompts.system.compose" => dashboard_prompts_system_compose(root, payload),
        _ => {
            if let Some(result) = dashboard_prompt_route_extension(root, normalized, payload) {
                result
            } else {
                json!({
                    "ok": false,
                    "type": "dashboard_lpp_route_error",
                    "error": format!("unsupported_lpp_action:{normalized}")
                })
            }
        }
    }
}

include!("018-dashboard-system-prompt-registry-helpers.rs");
