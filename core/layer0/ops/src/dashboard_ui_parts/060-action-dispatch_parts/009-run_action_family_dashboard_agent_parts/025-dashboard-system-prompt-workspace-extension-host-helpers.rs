fn dashboard_prompt_workspace_root_manager_describe(payload: &Value) -> Value {
    let roots = payload
        .get("roots")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|raw| clean_text(raw, 600)))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_workspace_root_manager_describe",
        "root_count": roots.len() as i64,
        "roots": roots
    })
}

fn dashboard_prompt_workspace_detection_inspect(payload: &Value) -> Value {
    let cwd = clean_text(payload.get("cwd").and_then(Value::as_str).unwrap_or(""), 600);
    let indicators = payload
        .get("indicators")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|raw| clean_text(raw, 120)))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_workspace_detection_inspect",
        "cwd": cwd,
        "indicator_count": indicators.len() as i64,
        "indicators": indicators
    })
}

fn dashboard_prompt_workspace_index_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_workspace_index_describe",
        "surfaces": [
            "root_manager",
            "detection",
            "multi_root_utils",
            "setup",
            "inline_path",
            "workspace_detection"
        ]
    })
}

fn dashboard_prompt_workspace_multi_root_utils_normalize(payload: &Value) -> Value {
    let mut roots = payload
        .get("roots")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|raw| clean_text(raw, 600)))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    roots.sort();
    roots.dedup();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_workspace_multi_root_utils_normalize",
        "roots": roots,
        "count": roots.len() as i64
    })
}

fn dashboard_prompt_workspace_setup_plan(payload: &Value) -> Value {
    let workspace = clean_text(
        payload
            .get("workspace")
            .and_then(Value::as_str)
            .unwrap_or("."),
        600,
    );
    let defaults = payload
        .get("defaults")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_workspace_setup_plan",
        "workspace": workspace,
        "defaults": defaults,
        "steps": [
            "detect_roots",
            "apply_workspace_defaults",
            "persist_workspace_state"
        ]
    })
}

fn dashboard_prompt_workspace_parse_inline_path(payload: &Value) -> Value {
    let inline = clean_text(payload.get("inline_path").and_then(Value::as_str).unwrap_or(""), 1200);
    let parsed = inline
        .split(':')
        .next()
        .map(|raw| clean_text(raw, 600))
        .unwrap_or_default();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_workspace_parse_inline_path",
        "inline_path": inline,
        "path": parsed
    })
}

fn dashboard_prompt_workspace_detection_summarize(payload: &Value) -> Value {
    let has_workspace_file = payload
        .get("has_workspace_file")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let has_git = payload
        .get("has_git")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let confidence = if has_workspace_file && has_git {
        "high"
    } else if has_workspace_file || has_git {
        "medium"
    } else {
        "low"
    };
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_workspace_detection_summarize",
        "confidence": confidence,
        "has_workspace_file": has_workspace_file,
        "has_git": has_git
    })
}

fn dashboard_prompt_extension_bootstrap_describe(payload: &Value) -> Value {
    let host = clean_text(payload.get("host").and_then(Value::as_str).unwrap_or("vscode"), 120)
        .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_extension_bootstrap_describe",
        "host": host,
        "phases": ["activate", "register_routes", "ready"]
    })
}

fn dashboard_prompt_host_external_auth_describe(payload: &Value) -> Value {
    let provider = clean_text(
        payload
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or("external"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_external_auth_describe",
        "provider": provider,
        "flows": ["oauth", "token_refresh", "session_validate"]
    })
}

fn dashboard_prompt_host_external_comment_review_describe(payload: &Value) -> Value {
    let review_mode = clean_text(
        payload
            .get("review_mode")
            .and_then(Value::as_str)
            .unwrap_or("inline"),
        80,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_external_comment_review_describe",
        "review_mode": review_mode,
        "capabilities": ["list_threads", "resolve_thread", "post_reply"]
    })
}

fn dashboard_prompt_workspace_extension_host_route_extension(
    _root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.workspace.rootManager.describe" => {
            Some(dashboard_prompt_workspace_root_manager_describe(payload))
        }
        "dashboard.prompts.system.workspace.detection.inspect" => {
            Some(dashboard_prompt_workspace_detection_inspect(payload))
        }
        "dashboard.prompts.system.workspace.index.describe" => {
            Some(dashboard_prompt_workspace_index_describe())
        }
        "dashboard.prompts.system.workspace.multiRootUtils.normalize" => {
            Some(dashboard_prompt_workspace_multi_root_utils_normalize(payload))
        }
        "dashboard.prompts.system.workspace.setup.plan" => {
            Some(dashboard_prompt_workspace_setup_plan(payload))
        }
        "dashboard.prompts.system.workspace.parseInlinePath" => {
            Some(dashboard_prompt_workspace_parse_inline_path(payload))
        }
        "dashboard.prompts.system.workspace.workspaceDetection.summarize" => {
            Some(dashboard_prompt_workspace_detection_summarize(payload))
        }
        "dashboard.prompts.system.extension.bootstrap.describe" => {
            Some(dashboard_prompt_extension_bootstrap_describe(payload))
        }
        "dashboard.prompts.system.hosts.external.auth.describe" => {
            Some(dashboard_prompt_host_external_auth_describe(payload))
        }
        "dashboard.prompts.system.hosts.external.commentReview.describe" => {
            Some(dashboard_prompt_host_external_comment_review_describe(payload))
        }
        _ => dashboard_prompt_host_bridge_vscode_route_extension(_root, normalized, payload),
    }
}

include!("026-dashboard-system-prompt-host-bridge-vscode-helpers.rs");
