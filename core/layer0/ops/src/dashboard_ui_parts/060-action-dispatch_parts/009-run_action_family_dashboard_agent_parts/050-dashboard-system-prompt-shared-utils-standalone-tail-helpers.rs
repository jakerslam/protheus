fn dashboard_prompt_shared_string_describe(payload: &Value) -> Value {
    let op = clean_text(
        payload
            .get("operation")
            .and_then(Value::as_str)
            .unwrap_or("normalize"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_string_describe",
        "operation": op
    })
}

fn dashboard_prompt_shared_tools_describe(payload: &Value) -> Value {
    let lane = clean_text(
        payload
            .get("lane")
            .and_then(Value::as_str)
            .unwrap_or("tooling"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_tools_describe",
        "lane": lane
    })
}

fn dashboard_prompt_shared_utils_model_filters_describe(payload: &Value) -> Value {
    let filter = clean_text(
        payload
            .get("filter")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_utils_model_filters_describe",
        "filter": filter
    })
}

fn dashboard_prompt_shared_utils_reasoning_support_describe(payload: &Value) -> Value {
    let mode = clean_text(
        payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("balanced"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_utils_reasoning_support_describe",
        "mode": mode
    })
}

fn dashboard_prompt_shared_vs_code_selector_utils_describe(payload: &Value) -> Value {
    let selector = clean_text(
        payload
            .get("selector")
            .and_then(Value::as_str)
            .unwrap_or("active_editor"),
        180,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_vs_code_selector_utils_describe",
        "selector": selector
    })
}

fn dashboard_prompt_standalone_cline_core_describe(payload: &Value) -> Value {
    let runtime = clean_text(
        payload
            .get("runtime")
            .and_then(Value::as_str)
            .unwrap_or("resident_ipc"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_standalone_cline_core_describe",
        "runtime": runtime
    })
}

fn dashboard_prompt_standalone_hostbridge_client_describe(payload: &Value) -> Value {
    let channel = clean_text(
        payload
            .get("channel")
            .and_then(Value::as_str)
            .unwrap_or("grpc"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_standalone_hostbridge_client_describe",
        "channel": channel
    })
}

fn dashboard_prompt_standalone_lock_manager_describe(payload: &Value) -> Value {
    let policy = clean_text(
        payload
            .get("policy")
            .and_then(Value::as_str)
            .unwrap_or("fail_closed"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_standalone_lock_manager_describe",
        "policy": policy
    })
}

fn dashboard_prompt_standalone_protobus_service_describe(payload: &Value) -> Value {
    let bus = clean_text(
        payload
            .get("bus")
            .and_then(Value::as_str)
            .unwrap_or("primary"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_standalone_protobus_service_describe",
        "bus": bus
    })
}

fn dashboard_prompt_standalone_utils_describe(payload: &Value) -> Value {
    let helper = clean_text(
        payload
            .get("helper")
            .and_then(Value::as_str)
            .unwrap_or("path_resolution"),
        180,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_standalone_utils_describe",
        "helper": helper
    })
}

fn dashboard_prompt_shared_utils_standalone_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.shared.string.describe" => {
            Some(dashboard_prompt_shared_string_describe(payload))
        }
        "dashboard.prompts.system.shared.tools.describe" => {
            Some(dashboard_prompt_shared_tools_describe(payload))
        }
        "dashboard.prompts.system.shared.utils.modelFilters.describe" => {
            Some(dashboard_prompt_shared_utils_model_filters_describe(payload))
        }
        "dashboard.prompts.system.shared.utils.reasoningSupport.describe" => {
            Some(dashboard_prompt_shared_utils_reasoning_support_describe(payload))
        }
        "dashboard.prompts.system.shared.vsCodeSelectorUtils.describe" => {
            Some(dashboard_prompt_shared_vs_code_selector_utils_describe(payload))
        }
        "dashboard.prompts.system.standalone.clineCore.describe" => {
            Some(dashboard_prompt_standalone_cline_core_describe(payload))
        }
        "dashboard.prompts.system.standalone.hostbridgeClient.describe" => {
            Some(dashboard_prompt_standalone_hostbridge_client_describe(payload))
        }
        "dashboard.prompts.system.standalone.lockManager.describe" => {
            Some(dashboard_prompt_standalone_lock_manager_describe(payload))
        }
        "dashboard.prompts.system.standalone.protobusService.describe" => {
            Some(dashboard_prompt_standalone_protobus_service_describe(payload))
        }
        "dashboard.prompts.system.standalone.utils.describe" => {
            Some(dashboard_prompt_standalone_utils_describe(payload))
        }
        _ => dashboard_prompt_utils_core_tail_route_extension(root, normalized, payload),
    }
}

include!("051-dashboard-system-prompt-utils-core-tail-helpers.rs");
