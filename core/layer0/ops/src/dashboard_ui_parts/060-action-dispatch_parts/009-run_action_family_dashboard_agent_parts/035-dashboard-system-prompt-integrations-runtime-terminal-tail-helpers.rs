fn dashboard_prompt_integrations_misc_open_file_describe(payload: &Value) -> Value {
    let path = clean_text(payload.get("path").and_then(Value::as_str).unwrap_or(""), 1200);
    let focus = payload.get("focus").and_then(Value::as_bool).unwrap_or(true);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_misc_open_file_describe",
        "path": path,
        "focus": focus
    })
}

fn dashboard_prompt_integrations_misc_process_files_describe(payload: &Value) -> Value {
    let mode = clean_text(
        payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("batch"),
        120,
    )
    .to_ascii_lowercase();
    let file_count = payload
        .get("file_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_misc_process_files_describe",
        "mode": mode,
        "file_count": file_count
    })
}

fn dashboard_prompt_integrations_notifications_index_describe(payload: &Value) -> Value {
    let channel = clean_text(
        payload
            .get("channel")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_notifications_index_describe",
        "channel": channel
    })
}

fn dashboard_prompt_integrations_openai_codex_oauth_describe(payload: &Value) -> Value {
    let provider = clean_text(
        payload
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or("openai"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_openai_codex_oauth_describe",
        "provider": provider
    })
}

fn dashboard_prompt_integrations_terminal_command_executor_describe(payload: &Value) -> Value {
    let command = clean_text(payload.get("command").and_then(Value::as_str).unwrap_or(""), 2400);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_terminal_command_executor_describe",
        "command": command
    })
}

fn dashboard_prompt_integrations_terminal_command_orchestrator_describe(payload: &Value) -> Value {
    let strategy = clean_text(
        payload
            .get("strategy")
            .and_then(Value::as_str)
            .unwrap_or("serial"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_terminal_command_orchestrator_describe",
        "strategy": strategy
    })
}

fn dashboard_prompt_integrations_terminal_constants_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_terminal_constants_describe",
        "contracts": ["max_buffer_chars", "default_timeout_ms"]
    })
}

fn dashboard_prompt_integrations_terminal_index_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_terminal_index_describe",
        "exports": ["command_executor", "command_orchestrator", "standalone"]
    })
}

fn dashboard_prompt_integrations_terminal_standalone_terminal_describe(payload: &Value) -> Value {
    let terminal_id = clean_text(
        payload
            .get("terminal_id")
            .and_then(Value::as_str)
            .unwrap_or("standalone"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_terminal_standalone_terminal_describe",
        "terminal_id": terminal_id
    })
}

fn dashboard_prompt_integrations_terminal_standalone_manager_describe(payload: &Value) -> Value {
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
        "type": "dashboard_prompts_system_integrations_terminal_standalone_manager_describe",
        "scope": scope
    })
}

fn dashboard_prompt_integrations_runtime_terminal_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.integrations.misc.openFile.describe" => {
            Some(dashboard_prompt_integrations_misc_open_file_describe(payload))
        }
        "dashboard.prompts.system.integrations.misc.processFiles.describe" => {
            Some(dashboard_prompt_integrations_misc_process_files_describe(payload))
        }
        "dashboard.prompts.system.integrations.notifications.index.describe" => {
            Some(dashboard_prompt_integrations_notifications_index_describe(payload))
        }
        "dashboard.prompts.system.integrations.openaiCodex.oauth.describe" => {
            Some(dashboard_prompt_integrations_openai_codex_oauth_describe(payload))
        }
        "dashboard.prompts.system.integrations.terminal.commandExecutor.describe" => {
            Some(dashboard_prompt_integrations_terminal_command_executor_describe(payload))
        }
        "dashboard.prompts.system.integrations.terminal.commandOrchestrator.describe" => {
            Some(dashboard_prompt_integrations_terminal_command_orchestrator_describe(payload))
        }
        "dashboard.prompts.system.integrations.terminal.constants.describe" => {
            Some(dashboard_prompt_integrations_terminal_constants_describe())
        }
        "dashboard.prompts.system.integrations.terminal.index.describe" => {
            Some(dashboard_prompt_integrations_terminal_index_describe())
        }
        "dashboard.prompts.system.integrations.terminal.standaloneTerminal.describe" => {
            Some(dashboard_prompt_integrations_terminal_standalone_terminal_describe(payload))
        }
        "dashboard.prompts.system.integrations.terminal.standaloneTerminalManager.describe" => {
            Some(dashboard_prompt_integrations_terminal_standalone_manager_describe(payload))
        }
        _ => dashboard_prompt_integrations_terminal_package_services_tail_route_extension(
            root, normalized, payload,
        ),
    }
}

include!("036-dashboard-system-prompt-integrations-terminal-package-services-tail-helpers.rs");
