fn dashboard_prompt_shared_messages_metrics_describe(payload: &Value) -> Value {
    let metric = clean_text(
        payload
            .get("metric")
            .and_then(Value::as_str)
            .unwrap_or("token_usage"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_messages_metrics_describe",
        "metric": metric
    })
}

fn dashboard_prompt_shared_multi_root_types_describe(payload: &Value) -> Value {
    let workspace_mode = clean_text(
        payload
            .get("workspace_mode")
            .and_then(Value::as_str)
            .unwrap_or("single_root"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_multi_root_types_describe",
        "workspace_mode": workspace_mode
    })
}

fn dashboard_prompt_shared_net_describe(payload: &Value) -> Value {
    let transport = clean_text(
        payload
            .get("transport")
            .and_then(Value::as_str)
            .unwrap_or("https"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_net_describe",
        "transport": transport
    })
}

fn dashboard_prompt_shared_prompts_describe(payload: &Value) -> Value {
    let prompt_kind = clean_text(
        payload
            .get("prompt_kind")
            .and_then(Value::as_str)
            .unwrap_or("system"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_prompts_describe",
        "prompt_kind": prompt_kind
    })
}

fn dashboard_prompt_shared_proto_cline_message_describe(payload: &Value) -> Value {
    let direction = clean_text(
        payload
            .get("direction")
            .and_then(Value::as_str)
            .unwrap_or("to_proto"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_proto_cline_message_describe",
        "direction": direction
    })
}

fn dashboard_prompt_shared_proto_file_search_result_describe(payload: &Value) -> Value {
    let file = clean_text(
        payload.get("file").and_then(Value::as_str).unwrap_or(""),
        500,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_proto_file_search_result_describe",
        "file": file
    })
}

fn dashboard_prompt_shared_proto_mcp_server_describe(payload: &Value) -> Value {
    let server = clean_text(
        payload
            .get("server")
            .and_then(Value::as_str)
            .unwrap_or("local"),
        180,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_proto_mcp_server_describe",
        "server": server
    })
}

fn dashboard_prompt_shared_proto_model_api_configuration_describe(payload: &Value) -> Value {
    let model_family = clean_text(
        payload
            .get("model_family")
            .and_then(Value::as_str)
            .unwrap_or("gpt"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_proto_model_api_configuration_describe",
        "model_family": model_family
    })
}

fn dashboard_prompt_shared_proto_model_type_conversion_describe(payload: &Value) -> Value {
    let conversion = clean_text(
        payload
            .get("conversion")
            .and_then(Value::as_str)
            .unwrap_or("strict"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_proto_model_type_conversion_describe",
        "conversion": conversion
    })
}

fn dashboard_prompt_shared_proto_model_vscode_lm_models_describe(payload: &Value) -> Value {
    let target = clean_text(
        payload
            .get("target")
            .and_then(Value::as_str)
            .unwrap_or("chat"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_proto_model_vscode_lm_models_describe",
        "target": target
    })
}

fn dashboard_prompt_shared_proto_conversions_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.shared.messages.metrics.describe" => {
            Some(dashboard_prompt_shared_messages_metrics_describe(payload))
        }
        "dashboard.prompts.system.shared.multiRoot.types.describe" => {
            Some(dashboard_prompt_shared_multi_root_types_describe(payload))
        }
        "dashboard.prompts.system.shared.net.describe" => {
            Some(dashboard_prompt_shared_net_describe(payload))
        }
        "dashboard.prompts.system.shared.prompts.describe" => {
            Some(dashboard_prompt_shared_prompts_describe(payload))
        }
        "dashboard.prompts.system.shared.protoConversions.clineMessage.describe" => {
            Some(dashboard_prompt_shared_proto_cline_message_describe(payload))
        }
        "dashboard.prompts.system.shared.protoConversions.file.searchResultConversion.describe" => {
            Some(dashboard_prompt_shared_proto_file_search_result_describe(payload))
        }
        "dashboard.prompts.system.shared.protoConversions.mcp.mcpServerConversion.describe" => {
            Some(dashboard_prompt_shared_proto_mcp_server_describe(payload))
        }
        "dashboard.prompts.system.shared.protoConversions.models.apiConfigurationConversion.describe" => {
            Some(dashboard_prompt_shared_proto_model_api_configuration_describe(payload))
        }
        "dashboard.prompts.system.shared.protoConversions.models.typeConversion.describe" => {
            Some(dashboard_prompt_shared_proto_model_type_conversion_describe(payload))
        }
        "dashboard.prompts.system.shared.protoConversions.models.vscodeLmModelsConversion.describe" => {
            Some(dashboard_prompt_shared_proto_model_vscode_lm_models_describe(payload))
        }
        _ => dashboard_prompt_shared_provider_remote_services_tail_route_extension(root, normalized, payload),
    }
}

include!("048-dashboard-system-prompt-shared-provider-remote-services-tail-helpers.rs");
