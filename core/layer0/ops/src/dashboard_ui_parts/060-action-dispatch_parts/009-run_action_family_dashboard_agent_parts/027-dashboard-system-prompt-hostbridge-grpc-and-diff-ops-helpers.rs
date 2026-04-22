fn dashboard_prompt_host_vscode_commit_message_generator_describe(payload: &Value) -> Value {
    let style = clean_text(
        payload
            .get("style")
            .and_then(Value::as_str)
            .unwrap_or("conventional"),
        80,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_commit_message_generator_describe",
        "style": style,
        "supports_scopes": true
    })
}

fn dashboard_prompt_host_vscode_hostbridge_grpc_handler_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_hostbridge_grpc_handler_describe",
        "ops": ["handle_request", "validate_payload", "emit_response"]
    })
}

fn dashboard_prompt_host_vscode_hostbridge_grpc_service_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_hostbridge_grpc_service_describe",
        "services": ["HostBridgeService", "DiffService", "EnvService"]
    })
}

fn dashboard_prompt_host_vscode_host_grpc_client_base_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_host_grpc_client_base_describe",
        "capabilities": ["connect", "call", "retry", "shutdown"]
    })
}

fn dashboard_prompt_host_vscode_host_grpc_client_describe(payload: &Value) -> Value {
    let channel = clean_text(
        payload
            .get("channel")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_host_grpc_client_describe",
        "channel": channel,
        "supports_streaming": true
    })
}

fn dashboard_prompt_host_vscode_diff_close_all_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_diff_close_all_describe",
        "operation": "close_all_diffs"
    })
}

fn dashboard_prompt_host_vscode_diff_get_document_text_describe(payload: &Value) -> Value {
    let uri = clean_text(payload.get("uri").and_then(Value::as_str).unwrap_or(""), 600);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_diff_get_document_text_describe",
        "uri": uri
    })
}

fn dashboard_prompt_host_vscode_diff_open_diff_describe(payload: &Value) -> Value {
    let left = clean_text(payload.get("left").and_then(Value::as_str).unwrap_or(""), 600);
    let right = clean_text(payload.get("right").and_then(Value::as_str).unwrap_or(""), 600);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_diff_open_diff_describe",
        "left": left,
        "right": right
    })
}

fn dashboard_prompt_host_vscode_diff_open_multi_file_describe(payload: &Value) -> Value {
    let files = payload
        .get("files")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|raw| clean_text(raw, 600)))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_diff_open_multi_file_describe",
        "file_count": files.len() as i64,
        "files": files
    })
}

fn dashboard_prompt_host_vscode_diff_replace_text_describe(payload: &Value) -> Value {
    let uri = clean_text(payload.get("uri").and_then(Value::as_str).unwrap_or(""), 600);
    let replacement_len = clean_text(
        payload
            .get("replacement")
            .and_then(Value::as_str)
            .unwrap_or(""),
        4000,
    )
    .len() as i64;
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_diff_replace_text_describe",
        "uri": uri,
        "replacement_len": replacement_len
    })
}

fn dashboard_prompt_hostbridge_grpc_and_diff_ops_route_extension(
    _root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.hosts.vscode.commitMessageGenerator.describe" => {
            Some(dashboard_prompt_host_vscode_commit_message_generator_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.grpcHandler.describe" => {
            Some(dashboard_prompt_host_vscode_hostbridge_grpc_handler_describe())
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.grpcService.describe" => {
            Some(dashboard_prompt_host_vscode_hostbridge_grpc_service_describe())
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.client.hostGrpcClientBase.describe" => {
            Some(dashboard_prompt_host_vscode_host_grpc_client_base_describe())
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.client.hostGrpcClient.describe" => {
            Some(dashboard_prompt_host_vscode_host_grpc_client_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.diff.closeAllDiffs.describe" => {
            Some(dashboard_prompt_host_vscode_diff_close_all_describe())
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.diff.getDocumentText.describe" => {
            Some(dashboard_prompt_host_vscode_diff_get_document_text_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.diff.openDiff.describe" => {
            Some(dashboard_prompt_host_vscode_diff_open_diff_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.diff.openMultiFileDiff.describe" => {
            Some(dashboard_prompt_host_vscode_diff_open_multi_file_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.diff.replaceText.describe" => {
            Some(dashboard_prompt_host_vscode_diff_replace_text_describe(payload))
        }
        _ => dashboard_prompt_hostbridge_diff_env_tail_route_extension(_root, normalized, payload),
    }
}

include!("028-dashboard-system-prompt-hostbridge-diff-env-tail-helpers.rs");
