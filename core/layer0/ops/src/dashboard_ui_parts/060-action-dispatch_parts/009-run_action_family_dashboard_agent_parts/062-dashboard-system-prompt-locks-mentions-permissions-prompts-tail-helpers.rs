fn dashboard_prompt_locks_types_describe(payload: &Value) -> Value {
    let lock_kind = clean_text(
        payload
            .get("lock_kind")
            .and_then(Value::as_str)
            .unwrap_or("exclusive"),
        80,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_locks_types_describe",
        "lock_kind": lock_kind
    })
}

fn dashboard_prompt_mentions_index_describe(payload: &Value) -> Value {
    let mention_scope = clean_text(
        payload
            .get("mention_scope")
            .and_then(Value::as_str)
            .unwrap_or("workspace"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_mentions_index_describe",
        "mention_scope": mention_scope
    })
}

fn dashboard_prompt_permissions_command_controller_describe(payload: &Value) -> Value {
    let decision_mode = clean_text(
        payload
            .get("decision_mode")
            .and_then(Value::as_str)
            .unwrap_or("fail_closed"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_permissions_command_controller_describe",
        "decision_mode": decision_mode
    })
}

fn dashboard_prompt_permissions_index_describe(payload: &Value) -> Value {
    let policy_view = clean_text(
        payload
            .get("policy_view")
            .and_then(Value::as_str)
            .unwrap_or("effective"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_permissions_index_describe",
        "policy_view": policy_view
    })
}

fn dashboard_prompt_permissions_types_describe(payload: &Value) -> Value {
    let permission_shape = clean_text(
        payload
            .get("permission_shape")
            .and_then(Value::as_str)
            .unwrap_or("structured"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_permissions_types_describe",
        "permission_shape": permission_shape
    })
}

fn dashboard_prompt_context_management_describe(payload: &Value) -> Value {
    let retention_mode = clean_text(
        payload
            .get("retention_mode")
            .and_then(Value::as_str)
            .unwrap_or("bounded"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_context_management_describe",
        "retention_mode": retention_mode
    })
}

fn dashboard_prompt_load_mcp_documentation_describe(payload: &Value) -> Value {
    let doc_scope = clean_text(
        payload
            .get("doc_scope")
            .and_then(Value::as_str)
            .unwrap_or("mcp"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_load_mcp_documentation_describe",
        "doc_scope": doc_scope
    })
}

fn dashboard_prompt_responses_describe(payload: &Value) -> Value {
    let response_mode = clean_text(
        payload
            .get("response_mode")
            .and_then(Value::as_str)
            .unwrap_or("structured"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_responses_describe",
        "response_mode": response_mode
    })
}

fn dashboard_prompt_legacy_local_models_compact_describe(payload: &Value) -> Value {
    let compactness = clean_text(
        payload
            .get("compactness")
            .and_then(Value::as_str)
            .unwrap_or("high"),
        80,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_legacy_local_models_compact_describe",
        "compactness": compactness
    })
}

fn dashboard_prompt_legacy_next_gen_gpt5_describe(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("gpt-5"),
        80,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_legacy_next_gen_gpt5_describe",
        "profile": profile
    })
}

fn dashboard_prompt_locks_mentions_permissions_prompts_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.locks.types.describe" => {
            Some(dashboard_prompt_locks_types_describe(payload))
        }
        "dashboard.prompts.system.mentions.index.describe" => {
            Some(dashboard_prompt_mentions_index_describe(payload))
        }
        "dashboard.prompts.system.permissions.commandPermissionController.describe" => {
            Some(dashboard_prompt_permissions_command_controller_describe(payload))
        }
        "dashboard.prompts.system.permissions.index.describe" => {
            Some(dashboard_prompt_permissions_index_describe(payload))
        }
        "dashboard.prompts.system.permissions.types.describe" => {
            Some(dashboard_prompt_permissions_types_describe(payload))
        }
        "dashboard.prompts.system.prompts.contextManagement.describe" => {
            Some(dashboard_prompt_context_management_describe(payload))
        }
        "dashboard.prompts.system.prompts.loadMcpDocumentation.describe" => {
            Some(dashboard_prompt_load_mcp_documentation_describe(payload))
        }
        "dashboard.prompts.system.prompts.responses.describe" => {
            Some(dashboard_prompt_responses_describe(payload))
        }
        "dashboard.prompts.system.prompts.legacy.localModels.compactSystemPrompt.describe" => {
            Some(dashboard_prompt_legacy_local_models_compact_describe(payload))
        }
        "dashboard.prompts.system.prompts.legacy.nextGen.gpt5.describe" => {
            Some(dashboard_prompt_legacy_next_gen_gpt5_describe(payload))
        }
        _ => dashboard_prompt_system_prompt_components_tail_route_extension(root, normalized, payload),
    }
}

include!("063-dashboard-system-prompt-components-tail-helpers.rs");
