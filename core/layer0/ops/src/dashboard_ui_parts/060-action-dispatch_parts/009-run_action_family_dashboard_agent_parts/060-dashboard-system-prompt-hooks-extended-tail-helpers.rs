fn dashboard_prompt_hooks_test_setup_describe(payload: &Value) -> Value {
    let fixture = clean_text(
        payload
            .get("fixture")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_test_setup_describe",
        "fixture": fixture
    })
}

fn dashboard_prompt_hooks_test_shell_escape_describe(payload: &Value) -> Value {
    let shell = clean_text(
        payload
            .get("shell")
            .and_then(Value::as_str)
            .unwrap_or("zsh"),
        80,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_test_shell_escape_describe",
        "shell": shell
    })
}

fn dashboard_prompt_hooks_test_task_cancel_describe(payload: &Value) -> Value {
    let cancel_mode = clean_text(
        payload
            .get("cancel_mode")
            .and_then(Value::as_str)
            .unwrap_or("graceful"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_test_task_cancel_describe",
        "cancel_mode": cancel_mode
    })
}

fn dashboard_prompt_hooks_test_task_complete_describe(payload: &Value) -> Value {
    let completion = clean_text(
        payload
            .get("completion")
            .and_then(Value::as_str)
            .unwrap_or("success"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_test_task_complete_describe",
        "completion": completion
    })
}

fn dashboard_prompt_hooks_test_task_resume_describe(payload: &Value) -> Value {
    let resume_mode = clean_text(
        payload
            .get("resume_mode")
            .and_then(Value::as_str)
            .unwrap_or("from_checkpoint"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_test_task_resume_describe",
        "resume_mode": resume_mode
    })
}

fn dashboard_prompt_hooks_test_task_start_describe(payload: &Value) -> Value {
    let start_mode = clean_text(
        payload
            .get("start_mode")
            .and_then(Value::as_str)
            .unwrap_or("fresh"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_test_task_start_describe",
        "start_mode": start_mode
    })
}

fn dashboard_prompt_hooks_test_utils_extended_describe(payload: &Value) -> Value {
    let helper = clean_text(
        payload
            .get("helper")
            .and_then(Value::as_str)
            .unwrap_or("mock_context"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_test_utils_extended_describe",
        "helper": helper
    })
}

fn dashboard_prompt_hooks_test_user_prompt_submit_describe(payload: &Value) -> Value {
    let submit_mode = clean_text(
        payload
            .get("submit_mode")
            .and_then(Value::as_str)
            .unwrap_or("interactive"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_test_user_prompt_submit_describe",
        "submit_mode": submit_mode
    })
}

fn dashboard_prompt_hooks_executor_describe(payload: &Value) -> Value {
    let executor = clean_text(
        payload
            .get("executor")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_executor_describe",
        "executor": executor
    })
}

fn dashboard_prompt_hooks_factory_describe(payload: &Value) -> Value {
    let factory = clean_text(
        payload
            .get("factory")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_factory_describe",
        "factory": factory
    })
}

fn dashboard_prompt_hooks_extended_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.hooks.tests.setup.describe" => {
            Some(dashboard_prompt_hooks_test_setup_describe(payload))
        }
        "dashboard.prompts.system.hooks.tests.shellEscape.describe" => {
            Some(dashboard_prompt_hooks_test_shell_escape_describe(payload))
        }
        "dashboard.prompts.system.hooks.tests.taskCancel.describe" => {
            Some(dashboard_prompt_hooks_test_task_cancel_describe(payload))
        }
        "dashboard.prompts.system.hooks.tests.taskComplete.describe" => {
            Some(dashboard_prompt_hooks_test_task_complete_describe(payload))
        }
        "dashboard.prompts.system.hooks.tests.taskResume.describe" => {
            Some(dashboard_prompt_hooks_test_task_resume_describe(payload))
        }
        "dashboard.prompts.system.hooks.tests.taskStart.describe" => {
            Some(dashboard_prompt_hooks_test_task_start_describe(payload))
        }
        "dashboard.prompts.system.hooks.tests.testUtils.describe" => {
            Some(dashboard_prompt_hooks_test_utils_extended_describe(payload))
        }
        "dashboard.prompts.system.hooks.tests.userPromptSubmit.describe" => {
            Some(dashboard_prompt_hooks_test_user_prompt_submit_describe(payload))
        }
        "dashboard.prompts.system.hooks.hookExecutor.describe" => {
            Some(dashboard_prompt_hooks_executor_describe(payload))
        }
        "dashboard.prompts.system.hooks.hookFactory.describe" => {
            Some(dashboard_prompt_hooks_factory_describe(payload))
        }
        _ => dashboard_prompt_hooks_runtime_locks_tail_route_extension(root, normalized, payload),
    }
}

include!("061-dashboard-system-prompt-hooks-runtime-locks-tail-helpers.rs");
