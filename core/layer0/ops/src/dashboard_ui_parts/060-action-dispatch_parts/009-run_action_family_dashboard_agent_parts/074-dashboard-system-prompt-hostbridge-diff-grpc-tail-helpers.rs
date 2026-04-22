fn dashboard_prompt_hosts_surface_vscode_commit_message_generator_describe(payload: &Value) -> Value {
    let style = clean_text(
        payload
            .get("style")
            .and_then(Value::as_str)
            .unwrap_or("conventional"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_commit_message_generator_describe",
        "style": style
    })
}

fn dashboard_prompt_hosts_surface_vscode_hostbridge_grpc_handler_describe(payload: &Value) -> Value {
    let handler_mode = clean_text(
        payload
            .get("handler_mode")
            .and_then(Value::as_str)
            .unwrap_or("streaming"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_hostbridge_grpc_handler_describe",
        "handler_mode": handler_mode
    })
}

fn dashboard_prompt_hosts_surface_vscode_hostbridge_grpc_service_describe(payload: &Value) -> Value {
    let service_mode = clean_text(
        payload
            .get("service_mode")
            .and_then(Value::as_str)
            .unwrap_or("grpc"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_hostbridge_grpc_service_describe",
        "service_mode": service_mode
    })
}

fn dashboard_prompt_hosts_surface_vscode_host_grpc_client_base_describe(payload: &Value) -> Value {
    let transport = clean_text(
        payload
            .get("transport")
            .and_then(Value::as_str)
            .unwrap_or("grpc"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_host_grpc_client_base_describe",
        "transport": transport
    })
}

fn dashboard_prompt_hosts_surface_vscode_host_grpc_client_describe(payload: &Value) -> Value {
    let channel = clean_text(
        payload
            .get("channel")
            .and_then(Value::as_str)
            .unwrap_or("main"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_host_grpc_client_describe",
        "channel": channel
    })
}

fn dashboard_prompt_hosts_surface_vscode_diff_close_all_describe(payload: &Value) -> Value {
    let scope = clean_text(
        payload
            .get("scope")
            .and_then(Value::as_str)
            .unwrap_or("workspace"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_diff_close_all_describe",
        "scope": scope
    })
}

fn dashboard_prompt_hosts_surface_vscode_diff_get_document_text_describe(payload: &Value) -> Value {
    let uri = clean_text(
        payload
            .get("uri")
            .and_then(Value::as_str)
            .unwrap_or(""),
        300,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_diff_get_document_text_describe",
        "uri": uri
    })
}

fn dashboard_prompt_hosts_surface_vscode_diff_open_diff_describe(payload: &Value) -> Value {
    let left = clean_text(
        payload
            .get("left")
            .and_then(Value::as_str)
            .unwrap_or(""),
        220,
    );
    let right = clean_text(
        payload
            .get("right")
            .and_then(Value::as_str)
            .unwrap_or(""),
        220,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_diff_open_diff_describe",
        "left": left,
        "right": right
    })
}

fn dashboard_prompt_hosts_surface_vscode_diff_open_multi_file_describe(payload: &Value) -> Value {
    let file_count = payload
        .get("files")
        .and_then(Value::as_array)
        .map(|files| files.len())
        .unwrap_or(0);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_diff_open_multi_file_describe",
        "file_count": file_count
    })
}

fn dashboard_prompt_hosts_surface_vscode_diff_replace_text_describe(payload: &Value) -> Value {
    let uri = clean_text(
        payload
            .get("uri")
            .and_then(Value::as_str)
            .unwrap_or(""),
        220,
    );
    let replacement_len = payload
        .get("replacement")
        .and_then(Value::as_str)
        .map(str::len)
        .unwrap_or(0);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_diff_replace_text_describe",
        "uri": uri,
        "replacement_len": replacement_len
    })
}

fn dashboard_prompt_hosts_surface_tail_hostbridge_diff_grpc_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.hosts.surface.vscode.commitMessageGenerator.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_commit_message_generator_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.grpcHandler.describe" => Some(
            dashboard_prompt_hosts_surface_vscode_hostbridge_grpc_handler_describe(payload),
        ),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.grpcService.describe" => Some(
            dashboard_prompt_hosts_surface_vscode_hostbridge_grpc_service_describe(payload),
        ),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.client.hostGrpcClientBase.describe" => Some(
            dashboard_prompt_hosts_surface_vscode_host_grpc_client_base_describe(payload),
        ),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.client.hostGrpcClient.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_host_grpc_client_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.diff.closeAllDiffs.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_diff_close_all_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.diff.getDocumentText.describe" => Some(
            dashboard_prompt_hosts_surface_vscode_diff_get_document_text_describe(payload),
        ),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.diff.openDiff.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_diff_open_diff_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.diff.openMultiFileDiff.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_diff_open_multi_file_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.diff.replaceText.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_diff_replace_text_describe(payload))
        }
        _ => dashboard_prompt_hosts_surface_tail_hostbridge_diff_env_route_extension(
            root,
            normalized,
            payload,
        ),
    }
}
include!("075-dashboard-system-prompt-hostbridge-diff-env-tail-helpers.rs");
