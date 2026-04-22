fn dashboard_prompt_host_external_diffview_provider_describe(payload: &Value) -> Value {
    let lane = clean_text(
        payload
            .get("lane")
            .and_then(Value::as_str)
            .unwrap_or("external_diff"),
        80,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_external_diffview_provider_describe",
        "lane": lane,
        "capabilities": ["open_diff", "close_diff", "refresh_diff"]
    })
}

fn dashboard_prompt_host_external_webview_provider_describe(payload: &Value) -> Value {
    let host = clean_text(
        payload
            .get("host")
            .and_then(Value::as_str)
            .unwrap_or("external"),
        80,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_external_webview_provider_describe",
        "host": host,
        "capabilities": ["render", "post_message", "open_url"]
    })
}

fn dashboard_prompt_host_external_grpc_types_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_external_grpc_types_describe",
        "types": ["HostRequest", "HostResponse", "DiffEvent", "AuthState"]
    })
}

fn dashboard_prompt_host_external_bridge_client_manager_describe(payload: &Value) -> Value {
    let strategy = clean_text(
        payload
            .get("strategy")
            .and_then(Value::as_str)
            .unwrap_or("reuse"),
        80,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_external_bridge_client_manager_describe",
        "strategy": strategy,
        "controls": ["connect", "reuse", "reconnect", "shutdown"]
    })
}

fn dashboard_prompt_host_provider_types_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_provider_types_describe",
        "provider_types": ["vscode", "external", "headless"]
    })
}

fn dashboard_prompt_host_provider_describe(payload: &Value) -> Value {
    let selected = clean_text(
        payload
            .get("selected")
            .and_then(Value::as_str)
            .unwrap_or("vscode"),
        80,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_provider_describe",
        "selected": selected,
        "supports_fallback": true
    })
}

fn dashboard_prompt_host_vscode_decoration_controller_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_decoration_controller_describe",
        "controls": ["highlight_ranges", "clear_ranges", "sync_theme"]
    })
}

fn dashboard_prompt_host_vscode_notebook_diff_view_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_notebook_diff_view_describe",
        "capabilities": ["open_notebook_diff", "scroll_to_cell", "close_notebook_diff"]
    })
}

fn dashboard_prompt_host_vscode_diffview_provider_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_diffview_provider_describe",
        "capabilities": ["open_diff", "open_multi_file_diff", "replace_text"]
    })
}

fn dashboard_prompt_host_vscode_webview_provider_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_webview_provider_describe",
        "capabilities": ["render_html", "post_message", "subscribe_events"]
    })
}

fn dashboard_prompt_host_bridge_vscode_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.hosts.external.diffviewProvider.describe" => {
            Some(dashboard_prompt_host_external_diffview_provider_describe(payload))
        }
        "dashboard.prompts.system.hosts.external.webviewProvider.describe" => {
            Some(dashboard_prompt_host_external_webview_provider_describe(payload))
        }
        "dashboard.prompts.system.hosts.external.grpcTypes.describe" => {
            Some(dashboard_prompt_host_external_grpc_types_describe())
        }
        "dashboard.prompts.system.hosts.external.hostBridgeClientManager.describe" => {
            Some(dashboard_prompt_host_external_bridge_client_manager_describe(payload))
        }
        "dashboard.prompts.system.hosts.providerTypes.describe" => {
            Some(dashboard_prompt_host_provider_types_describe())
        }
        "dashboard.prompts.system.hosts.provider.describe" => {
            Some(dashboard_prompt_host_provider_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.decorationController.describe" => {
            Some(dashboard_prompt_host_vscode_decoration_controller_describe())
        }
        "dashboard.prompts.system.hosts.vscode.notebookDiffView.describe" => {
            Some(dashboard_prompt_host_vscode_notebook_diff_view_describe())
        }
        "dashboard.prompts.system.hosts.vscode.diffviewProvider.describe" => {
            Some(dashboard_prompt_host_vscode_diffview_provider_describe())
        }
        "dashboard.prompts.system.hosts.vscode.webviewProvider.describe" => {
            Some(dashboard_prompt_host_vscode_webview_provider_describe())
        }
        _ => dashboard_prompt_hostbridge_grpc_and_diff_ops_route_extension(root, normalized, payload),
    }
}

include!("027-dashboard-system-prompt-hostbridge-grpc-and-diff-ops-helpers.rs");
