fn dashboard_prompt_task_hook_execution_describe(payload: &Value) -> Value {
    let hook_name = clean_text(
        payload
            .get("hook_name")
            .and_then(Value::as_str)
            .or_else(|| payload.get("name").and_then(Value::as_str))
            .unwrap_or("unknown_hook"),
        120,
    );
    let phase = clean_text(
        payload
            .get("phase")
            .and_then(Value::as_str)
            .unwrap_or("run"),
        80,
    )
    .to_ascii_lowercase();
    let blocking = payload
        .get("blocking")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_hook_execution_describe",
        "hook_name": hook_name,
        "phase": phase,
        "blocking": blocking
    })
}

fn dashboard_prompt_task_utils_normalize(payload: &Value) -> Value {
    let text = clean_text(payload.get("text").and_then(Value::as_str).unwrap_or(""), 4000);
    let normalized = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_utils_normalize",
        "normalized_text": normalized
    })
}

fn dashboard_prompt_task_utils_build_user_feedback_content(payload: &Value) -> Value {
    let summary = clean_text(
        payload
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or(""),
        1200,
    );
    let bullets = payload
        .get("bullets")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|raw| clean_text(raw, 220)))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    let mut lines = Vec::<String>::new();
    if !summary.is_empty() {
        lines.push(summary);
    }
    for bullet in &bullets {
        lines.push(format!("- {bullet}"));
    }
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_utils_build_user_feedback_content",
        "content": lines.join("\n"),
        "bullet_count": bullets.len() as i64
    })
}

fn dashboard_prompt_task_utils_extract_user_prompt_from_content(payload: &Value) -> Value {
    let content = clean_text(
        payload
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or(""),
        6000,
    );
    let explicit = clean_text(
        payload
            .get("user_prompt")
            .and_then(Value::as_str)
            .unwrap_or(""),
        2000,
    );
    let extracted = if !explicit.is_empty() {
        explicit
    } else {
        content
            .lines()
            .rev()
            .find_map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    None
                } else if let Some(rest) = trimmed.strip_prefix("User:") {
                    Some(clean_text(rest.trim(), 2000))
                } else {
                    Some(clean_text(trimmed, 2000))
                }
            })
            .unwrap_or_default()
    };
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_utils_extract_user_prompt_from_content",
        "user_prompt": extracted
    })
}

fn dashboard_prompt_webview_provider_describe(payload: &Value) -> Value {
    let provider = clean_text(
        payload
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or("dashboard_webview_provider"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_webview_provider_describe",
        "provider": provider,
        "capabilities": ["render_html", "open_url", "track_events"]
    })
}

fn dashboard_prompt_webview_nonce(payload: &Value) -> Value {
    let seed = clean_text(payload.get("seed").and_then(Value::as_str).unwrap_or(""), 200);
    let value = if seed.is_empty() {
        format!("nonce-{}", crate::now_iso().replace([':', '-', 'T', 'Z'], ""))
    } else {
        let checksum = seed
            .bytes()
            .fold(0_u64, |acc, b| acc.wrapping_mul(131).wrapping_add(b as u64));
        format!("nonce-{checksum:x}")
    };
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_webview_get_nonce",
        "nonce": value
    })
}

fn dashboard_prompt_webview_index() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_webview_index",
        "routes": [
            "dashboard.prompts.system.webview.provider.describe",
            "dashboard.prompts.system.webview.getNonce",
            "dashboard.prompts.system.webview.index"
        ]
    })
}

fn dashboard_prompt_workspace_migration_reporter_report(payload: &Value) -> Value {
    let entries = payload
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let total = entries.len() as i64;
    let failed = entries
        .iter()
        .filter(|row| row.get("status").and_then(Value::as_str) == Some("failed"))
        .count() as i64;
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_workspace_migration_reporter_report",
        "total": total,
        "failed": failed,
        "succeeded": total.saturating_sub(failed)
    })
}

fn dashboard_prompt_workspace_path_adapter_resolve(payload: &Value) -> Value {
    let root = clean_text(
        payload
            .get("workspace_root")
            .and_then(Value::as_str)
            .unwrap_or("."),
        600,
    );
    let path = clean_text(payload.get("path").and_then(Value::as_str).unwrap_or(""), 600);
    let resolved = if path.is_empty() {
        root.clone()
    } else if std::path::Path::new(&path).is_absolute() {
        path.clone()
    } else {
        std::path::Path::new(&root)
            .join(path)
            .to_string_lossy()
            .to_string()
    };
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_workspace_path_adapter_resolve",
        "resolved_path": resolved
    })
}

fn dashboard_prompt_workspace_resolver_resolve(payload: &Value) -> Value {
    let candidates = payload
        .get("candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|raw| clean_text(raw, 600)))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    let selected = candidates.first().cloned().unwrap_or_default();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_workspace_resolver_resolve",
        "selected": selected,
        "candidate_count": candidates.len() as i64
    })
}

fn dashboard_prompt_task_webview_workspace_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.task.hookExecution.describe" => {
            Some(dashboard_prompt_task_hook_execution_describe(payload))
        }
        "dashboard.prompts.system.task.utils.normalize" => {
            Some(dashboard_prompt_task_utils_normalize(payload))
        }
        "dashboard.prompts.system.task.utils.buildUserFeedbackContent" => {
            Some(dashboard_prompt_task_utils_build_user_feedback_content(payload))
        }
        "dashboard.prompts.system.task.utils.extractUserPromptFromContent" => {
            Some(dashboard_prompt_task_utils_extract_user_prompt_from_content(payload))
        }
        "dashboard.prompts.system.webview.provider.describe" => {
            Some(dashboard_prompt_webview_provider_describe(payload))
        }
        "dashboard.prompts.system.webview.getNonce" => Some(dashboard_prompt_webview_nonce(payload)),
        "dashboard.prompts.system.webview.index" => Some(dashboard_prompt_webview_index()),
        "dashboard.prompts.system.workspace.migrationReporter.report" => {
            Some(dashboard_prompt_workspace_migration_reporter_report(payload))
        }
        "dashboard.prompts.system.workspace.pathAdapter.resolve" => {
            Some(dashboard_prompt_workspace_path_adapter_resolve(payload))
        }
        "dashboard.prompts.system.workspace.resolver.resolve" => {
            Some(dashboard_prompt_workspace_resolver_resolve(payload))
        }
        _ => dashboard_prompt_workspace_extension_host_route_extension(root, normalized, payload),
    }
}

include!("025-dashboard-system-prompt-workspace-extension-host-helpers.rs");
