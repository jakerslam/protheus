fn dashboard_prompt_controller_worktree_create_describe(payload: &Value) -> Value {
    let branch = clean_text(
        payload
            .get("branch")
            .and_then(Value::as_str)
            .unwrap_or("feature"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_worktree_create_describe",
        "branch": branch
    })
}

fn dashboard_prompt_controller_worktree_create_include_describe(payload: &Value) -> Value {
    let include = clean_text(
        payload
            .get("include")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_worktree_create_include_describe",
        "include": include
    })
}

fn dashboard_prompt_controller_worktree_delete_describe(payload: &Value) -> Value {
    let mode = clean_text(
        payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("safe"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_worktree_delete_describe",
        "mode": mode
    })
}

fn dashboard_prompt_controller_worktree_available_branches_describe(payload: &Value) -> Value {
    let scope = clean_text(
        payload
            .get("scope")
            .and_then(Value::as_str)
            .unwrap_or("local"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_worktree_available_branches_describe",
        "scope": scope
    })
}

fn dashboard_prompt_controller_worktree_defaults_describe(payload: &Value) -> Value {
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
        "type": "dashboard_prompts_system_controller_worktree_defaults_describe",
        "profile": profile
    })
}

fn dashboard_prompt_controller_worktree_include_status_describe(payload: &Value) -> Value {
    let include_mode = clean_text(
        payload
            .get("include_mode")
            .and_then(Value::as_str)
            .unwrap_or("tracked"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_worktree_include_status_describe",
        "include_mode": include_mode
    })
}

fn dashboard_prompt_controller_worktree_list_describe(payload: &Value) -> Value {
    let list_mode = clean_text(
        payload
            .get("list_mode")
            .and_then(Value::as_str)
            .unwrap_or("active"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_worktree_list_describe",
        "list_mode": list_mode
    })
}

fn dashboard_prompt_controller_worktree_merge_describe(payload: &Value) -> Value {
    let strategy = clean_text(
        payload
            .get("strategy")
            .and_then(Value::as_str)
            .unwrap_or("fast-forward"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_worktree_merge_describe",
        "strategy": strategy
    })
}

fn dashboard_prompt_controller_worktree_switch_describe(payload: &Value) -> Value {
    let destination = clean_text(
        payload
            .get("destination")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_worktree_switch_describe",
        "destination": destination
    })
}

fn dashboard_prompt_controller_worktree_track_view_opened_describe(payload: &Value) -> Value {
    let surface = clean_text(
        payload
            .get("surface")
            .and_then(Value::as_str)
            .unwrap_or("worktree_view"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_worktree_track_view_opened_describe",
        "surface": surface
    })
}

fn dashboard_prompt_controller_worktree_ops_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.controller.worktree.createWorktree.describe" => {
            Some(dashboard_prompt_controller_worktree_create_describe(payload))
        }
        "dashboard.prompts.system.controller.worktree.createWorktreeInclude.describe" => {
            Some(dashboard_prompt_controller_worktree_create_include_describe(payload))
        }
        "dashboard.prompts.system.controller.worktree.deleteWorktree.describe" => {
            Some(dashboard_prompt_controller_worktree_delete_describe(payload))
        }
        "dashboard.prompts.system.controller.worktree.getAvailableBranches.describe" => {
            Some(dashboard_prompt_controller_worktree_available_branches_describe(payload))
        }
        "dashboard.prompts.system.controller.worktree.getWorktreeDefaults.describe" => {
            Some(dashboard_prompt_controller_worktree_defaults_describe(payload))
        }
        "dashboard.prompts.system.controller.worktree.getWorktreeIncludeStatus.describe" => {
            Some(dashboard_prompt_controller_worktree_include_status_describe(payload))
        }
        "dashboard.prompts.system.controller.worktree.listWorktrees.describe" => {
            Some(dashboard_prompt_controller_worktree_list_describe(payload))
        }
        "dashboard.prompts.system.controller.worktree.mergeWorktree.describe" => {
            Some(dashboard_prompt_controller_worktree_merge_describe(payload))
        }
        "dashboard.prompts.system.controller.worktree.switchWorktree.describe" => {
            Some(dashboard_prompt_controller_worktree_switch_describe(payload))
        }
        "dashboard.prompts.system.controller.worktree.trackWorktreeViewOpened.describe" => {
            Some(dashboard_prompt_controller_worktree_track_view_opened_describe(payload))
        }
        _ => dashboard_prompt_hooks_tail_route_extension(root, normalized, payload),
    }
}

include!("059-dashboard-system-prompt-hooks-tail-helpers.rs");
