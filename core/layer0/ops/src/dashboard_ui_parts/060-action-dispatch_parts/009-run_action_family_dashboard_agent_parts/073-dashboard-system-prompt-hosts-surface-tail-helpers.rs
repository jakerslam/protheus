include!("074-dashboard-system-prompt-hostbridge-diff-grpc-tail-helpers.rs");

fn dashboard_prompt_hosts_external_diffview_provider_describe(payload: &Value) -> Value {
    let provider_mode = clean_text(
        payload
            .get("provider_mode")
            .and_then(Value::as_str)
            .unwrap_or("external"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_external_diffview_provider_describe",
        "provider_mode": provider_mode
    })
}

fn dashboard_prompt_hosts_external_webview_provider_describe(payload: &Value) -> Value {
    let provider_mode = clean_text(
        payload
            .get("provider_mode")
            .and_then(Value::as_str)
            .unwrap_or("external"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_external_webview_provider_describe",
        "provider_mode": provider_mode
    })
}

fn dashboard_prompt_hosts_external_grpc_types_describe(payload: &Value) -> Value {
    let grpc_profile = clean_text(
        payload
            .get("grpc_profile")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_external_grpc_types_describe",
        "grpc_profile": grpc_profile
    })
}

fn dashboard_prompt_hosts_external_host_bridge_client_manager_describe(payload: &Value) -> Value {
    let bridge_mode = clean_text(
        payload
            .get("bridge_mode")
            .and_then(Value::as_str)
            .unwrap_or("managed"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_external_host_bridge_client_manager_describe",
        "bridge_mode": bridge_mode
    })
}

fn dashboard_prompt_hosts_host_provider_types_describe(payload: &Value) -> Value {
    let provider_types = clean_text(
        payload
            .get("provider_types")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_host_provider_types_describe",
        "provider_types": provider_types
    })
}

fn dashboard_prompt_hosts_host_provider_describe(payload: &Value) -> Value {
    let host_mode = clean_text(
        payload
            .get("host_mode")
            .and_then(Value::as_str)
            .unwrap_or("vscode"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_host_provider_describe",
        "host_mode": host_mode
    })
}

fn dashboard_prompt_hosts_vscode_decoration_controller_describe(payload: &Value) -> Value {
    let decoration_mode = clean_text(
        payload
            .get("decoration_mode")
            .and_then(Value::as_str)
            .unwrap_or("inline"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_vscode_decoration_controller_describe",
        "decoration_mode": decoration_mode
    })
}

fn dashboard_prompt_hosts_vscode_notebook_diffview_describe(payload: &Value) -> Value {
    let diff_mode = clean_text(
        payload
            .get("diff_mode")
            .and_then(Value::as_str)
            .unwrap_or("notebook"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_vscode_notebook_diffview_describe",
        "diff_mode": diff_mode
    })
}

fn dashboard_prompt_hosts_vscode_diffview_provider_describe(payload: &Value) -> Value {
    let provider_mode = clean_text(
        payload
            .get("provider_mode")
            .and_then(Value::as_str)
            .unwrap_or("vscode"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_vscode_diffview_provider_describe",
        "provider_mode": provider_mode
    })
}

fn dashboard_prompt_hosts_vscode_webview_provider_describe(payload: &Value) -> Value {
    let provider_mode = clean_text(
        payload
            .get("provider_mode")
            .and_then(Value::as_str)
            .unwrap_or("vscode"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_vscode_webview_provider_describe",
        "provider_mode": provider_mode
    })
}

fn dashboard_prompt_hosts_surface_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.hosts.external.externalDiffviewProvider.describe" => {
            Some(dashboard_prompt_hosts_external_diffview_provider_describe(payload))
        }
        "dashboard.prompts.system.hosts.external.externalWebviewProvider.describe" => {
            Some(dashboard_prompt_hosts_external_webview_provider_describe(payload))
        }
        "dashboard.prompts.system.hosts.external.grpcTypes.describe" => {
            Some(dashboard_prompt_hosts_external_grpc_types_describe(payload))
        }
        "dashboard.prompts.system.hosts.external.hostBridgeClientManager.describe" => {
            Some(dashboard_prompt_hosts_external_host_bridge_client_manager_describe(payload))
        }
        "dashboard.prompts.system.hosts.hostProviderTypes.describe" => {
            Some(dashboard_prompt_hosts_host_provider_types_describe(payload))
        }
        "dashboard.prompts.system.hosts.hostProvider.describe" => {
            Some(dashboard_prompt_hosts_host_provider_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.decorationController.describe" => {
            Some(dashboard_prompt_hosts_vscode_decoration_controller_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.notebookDiffView.describe" => {
            Some(dashboard_prompt_hosts_vscode_notebook_diffview_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.vscodeDiffViewProvider.describe" => {
            Some(dashboard_prompt_hosts_vscode_diffview_provider_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.vscodeWebviewProvider.describe" => {
            Some(dashboard_prompt_hosts_vscode_webview_provider_describe(payload))
        }
        _ => dashboard_prompt_hosts_surface_tail_hostbridge_diff_grpc_route_extension(
            root,
            normalized,
            payload,
        ),
    }
}
