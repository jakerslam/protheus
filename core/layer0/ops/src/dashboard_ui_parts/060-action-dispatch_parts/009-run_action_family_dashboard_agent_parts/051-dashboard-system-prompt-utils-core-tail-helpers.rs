fn dashboard_prompt_utils_announcements_describe(payload: &Value) -> Value {
    let channel = clean_text(
        payload
            .get("channel")
            .and_then(Value::as_str)
            .unwrap_or("release"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_utils_announcements_describe",
        "channel": channel
    })
}

fn dashboard_prompt_utils_cli_detector_describe(payload: &Value) -> Value {
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
        "type": "dashboard_prompts_system_utils_cli_detector_describe",
        "shell": shell
    })
}

fn dashboard_prompt_utils_cost_describe(payload: &Value) -> Value {
    let model = clean_text(
        payload
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("gpt-5"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_utils_cost_describe",
        "model": model
    })
}

fn dashboard_prompt_utils_env_describe(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_utils_env_describe",
        "profile": profile
    })
}

fn dashboard_prompt_utils_env_expansion_describe(payload: &Value) -> Value {
    let pattern = clean_text(
        payload
            .get("pattern")
            .and_then(Value::as_str)
            .unwrap_or("${HOME}"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_utils_env_expansion_describe",
        "pattern": pattern
    })
}

fn dashboard_prompt_utils_fs_describe(payload: &Value) -> Value {
    let op = clean_text(
        payload
            .get("operation")
            .and_then(Value::as_str)
            .unwrap_or("read"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_utils_fs_describe",
        "operation": op
    })
}

fn dashboard_prompt_utils_git_worktree_describe(payload: &Value) -> Value {
    let action = clean_text(
        payload
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or("attach"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_utils_git_worktree_describe",
        "action": action
    })
}

fn dashboard_prompt_utils_git_describe(payload: &Value) -> Value {
    let verb = clean_text(
        payload
            .get("verb")
            .and_then(Value::as_str)
            .unwrap_or("status"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_utils_git_describe",
        "verb": verb
    })
}

fn dashboard_prompt_utils_github_url_utils_describe(payload: &Value) -> Value {
    let host = clean_text(
        payload
            .get("host")
            .and_then(Value::as_str)
            .unwrap_or("github.com"),
        160,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_utils_github_url_utils_describe",
        "host": host
    })
}

fn dashboard_prompt_utils_mcp_auth_describe(payload: &Value) -> Value {
    let flow = clean_text(
        payload
            .get("flow")
            .and_then(Value::as_str)
            .unwrap_or("device_code"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_utils_mcp_auth_describe",
        "flow": flow
    })
}

fn dashboard_prompt_utils_core_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.utils.announcements.describe" => {
            Some(dashboard_prompt_utils_announcements_describe(payload))
        }
        "dashboard.prompts.system.utils.cliDetector.describe" => {
            Some(dashboard_prompt_utils_cli_detector_describe(payload))
        }
        "dashboard.prompts.system.utils.cost.describe" => {
            Some(dashboard_prompt_utils_cost_describe(payload))
        }
        "dashboard.prompts.system.utils.env.describe" => {
            Some(dashboard_prompt_utils_env_describe(payload))
        }
        "dashboard.prompts.system.utils.envExpansion.describe" => {
            Some(dashboard_prompt_utils_env_expansion_describe(payload))
        }
        "dashboard.prompts.system.utils.fs.describe" => {
            Some(dashboard_prompt_utils_fs_describe(payload))
        }
        "dashboard.prompts.system.utils.gitWorktree.describe" => {
            Some(dashboard_prompt_utils_git_worktree_describe(payload))
        }
        "dashboard.prompts.system.utils.git.describe" => {
            Some(dashboard_prompt_utils_git_describe(payload))
        }
        "dashboard.prompts.system.utils.githubUrlUtils.describe" => {
            Some(dashboard_prompt_utils_github_url_utils_describe(payload))
        }
        "dashboard.prompts.system.utils.mcpAuth.describe" => {
            Some(dashboard_prompt_utils_mcp_auth_describe(payload))
        }
        _ => dashboard_prompt_utils_runtime_tail_route_extension(root, normalized, payload),
    }
}

include!("052-dashboard-system-prompt-utils-runtime-tail-helpers.rs");
