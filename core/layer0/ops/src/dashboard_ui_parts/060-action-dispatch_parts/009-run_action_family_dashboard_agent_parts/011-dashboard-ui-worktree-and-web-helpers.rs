const DASHBOARD_WORKTREE_STATE_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/worktrees_state.json";

fn dashboard_worktree_state_path(root: &Path) -> std::path::PathBuf {
    root.join(DASHBOARD_WORKTREE_STATE_REL)
}

fn dashboard_worktree_default_state(root: &Path) -> Value {
    let root_path = clean_text(&root.to_string_lossy(), 800);
    json!({
        "type": "dashboard_worktree_state",
        "is_git_repo": true,
        "is_multi_root": false,
        "is_subfolder": false,
        "git_root_path": root_path.clone(),
        "current_branch": "main",
        "current_worktree_path": root_path,
        "worktree_auto_open_path": "",
        "worktrees": [],
        "local_branches": ["main"],
        "remote_branches": [],
        "source": "dashboard.worktree.controller",
        "source_sequence": "",
        "age_seconds": 0,
        "stale": false,
        "updated_at": ""
    })
}

fn dashboard_worktree_read_state(root: &Path) -> Value {
    let path = dashboard_worktree_state_path(root);
    let mut state = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| dashboard_worktree_default_state(root));
    if !state.is_object() {
        state = dashboard_worktree_default_state(root);
    }
    if state.get("worktrees").and_then(Value::as_array).is_none() {
        state["worktrees"] = Value::Array(Vec::new());
    }
    if state.get("local_branches").and_then(Value::as_array).is_none() {
        state["local_branches"] = json!(["main"]);
    }
    if state.get("remote_branches").and_then(Value::as_array).is_none() {
        state["remote_branches"] = Value::Array(Vec::new());
    }
    state
}

fn dashboard_worktree_write_state(root: &Path, state: &Value) {
    let path = dashboard_worktree_state_path(root);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(encoded) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(path, encoded);
    }
}

fn dashboard_worktree_mutate_state<F>(root: &Path, mutator: F) -> Value
where
    F: FnOnce(&mut Value),
{
    let mut state = dashboard_worktree_read_state(root);
    mutator(&mut state);
    state["type"] = Value::String("dashboard_worktree_state".to_string());
    state["source"] = Value::String("dashboard.worktree.controller".to_string());
    state["updated_at"] = Value::String(crate::now_iso());
    state["age_seconds"] = Value::from(0);
    state["stale"] = Value::Bool(false);
    let mut seed = state.clone();
    seed["source_sequence"] = Value::String(String::new());
    state["source_sequence"] = Value::String(crate::deterministic_receipt_hash(&seed));
    dashboard_worktree_write_state(root, &state);
    state
}

fn dashboard_worktree_sorted_unique_branch_rows(rows: Vec<String>) -> Vec<String> {
    let mut set = std::collections::BTreeSet::<String>::new();
    for row in rows {
        let cleaned = clean_text(&row, 200);
        if !cleaned.is_empty() {
            set.insert(cleaned);
        }
    }
    set.into_iter().collect::<Vec<_>>()
}

fn dashboard_worktree_extract_local_branches(state: &Value) -> Vec<String> {
    let branches = state
        .get("local_branches")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|raw| clean_text(raw, 200)))
        .collect::<Vec<_>>();
    dashboard_worktree_sorted_unique_branch_rows(branches)
}

fn dashboard_worktree_extract_remote_branches(state: &Value) -> Vec<String> {
    let branches = state
        .get("remote_branches")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|raw| clean_text(raw, 200)))
        .collect::<Vec<_>>();
    dashboard_worktree_sorted_unique_branch_rows(branches)
}

fn dashboard_worktree_list(root: &Path) -> Value {
    let state = dashboard_worktree_read_state(root);
    json!({
        "ok": true,
        "type": "dashboard_worktree_list",
        "worktrees": state.get("worktrees").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "is_git_repo": state.get("is_git_repo").and_then(Value::as_bool).unwrap_or(true),
        "is_multi_root": state.get("is_multi_root").and_then(Value::as_bool).unwrap_or(false),
        "is_subfolder": state.get("is_subfolder").and_then(Value::as_bool).unwrap_or(false),
        "git_root_path": clean_text(state.get("git_root_path").and_then(Value::as_str).unwrap_or(""), 800),
        "state": state
    })
}

fn dashboard_worktree_get_available_branches(root: &Path) -> Value {
    let state = dashboard_worktree_read_state(root);
    let current_branch = clean_text(
        state
            .get("current_branch")
            .and_then(Value::as_str)
            .unwrap_or("main"),
        200,
    );
    json!({
        "ok": true,
        "type": "dashboard_worktree_available_branches",
        "local_branches": dashboard_worktree_extract_local_branches(&state),
        "remote_branches": dashboard_worktree_extract_remote_branches(&state),
        "current_branch": current_branch,
        "state": state
    })
}

fn dashboard_worktree_create(root: &Path, payload: &Value) -> Value {
    let worktree_path = clean_text(
        payload
            .get("path")
            .and_then(Value::as_str)
            .or_else(|| payload.get("worktree_path").and_then(Value::as_str))
            .unwrap_or(""),
        800,
    );
    if worktree_path.is_empty() {
        return json!({
            "ok": false,
            "type": "dashboard_worktree_result",
            "success": false,
            "message": "worktree_path_required"
        });
    }
    let branch = clean_text(
        payload
            .get("branch")
            .and_then(Value::as_str)
            .unwrap_or(""),
        200,
    );
    let branch = if branch.is_empty() {
        let suffix = crate::deterministic_receipt_hash(&json!({
            "path": worktree_path,
            "ts": crate::now_iso()
        }));
        format!("worktree/infring-{}", &suffix[..6])
    } else {
        branch
    };
    let mut worktree_row = json!({
        "path": worktree_path,
        "branch": branch,
        "commit_hash": crate::deterministic_receipt_hash(payload).chars().take(12).collect::<String>(),
        "is_current": false,
        "is_bare": false,
        "is_detached": false,
        "is_locked": false,
        "lock_reason": ""
    });
    let state = dashboard_worktree_mutate_state(root, |state| {
        let mut rows = state
            .get("worktrees")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        rows.retain(|row| {
            clean_text(row.get("path").and_then(Value::as_str).unwrap_or(""), 800)
                != clean_text(
                    worktree_row.get("path").and_then(Value::as_str).unwrap_or(""),
                    800,
                )
        });
        rows.push(worktree_row.clone());
        state["worktrees"] = Value::Array(rows);

        let mut local = dashboard_worktree_extract_local_branches(state);
        local.push(clean_text(
            worktree_row.get("branch").and_then(Value::as_str).unwrap_or(""),
            200,
        ));
        state["local_branches"] =
            Value::Array(dashboard_worktree_sorted_unique_branch_rows(local).into_iter().map(Value::String).collect());
    });
    if let Some(rows) = state.get("worktrees").and_then(Value::as_array) {
        if let Some(found) = rows.last() {
            worktree_row = found.clone();
        }
    }
    json!({
        "ok": true,
        "type": "dashboard_worktree_result",
        "success": true,
        "message": "worktree_created",
        "worktree": worktree_row,
        "state": state
    })
}

fn dashboard_worktree_delete(root: &Path, payload: &Value) -> Value {
    let target_path = clean_text(
        payload
            .get("path")
            .and_then(Value::as_str)
            .or_else(|| payload.get("worktree_path").and_then(Value::as_str))
            .unwrap_or(""),
        800,
    );
    if target_path.is_empty() {
        return json!({
            "ok": false,
            "type": "dashboard_worktree_result",
            "success": false,
            "message": "worktree_path_required"
        });
    }
    let mut removed = false;
    let state = dashboard_worktree_mutate_state(root, |state| {
        let mut rows = state
            .get("worktrees")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let before_len = rows.len();
        rows.retain(|row| {
            clean_text(row.get("path").and_then(Value::as_str).unwrap_or(""), 800) != target_path
        });
        removed = rows.len() != before_len;
        state["worktrees"] = Value::Array(rows);
        let current_path = clean_text(
            state
                .get("current_worktree_path")
                .and_then(Value::as_str)
                .unwrap_or(""),
            800,
        );
        if current_path == target_path {
            state["current_worktree_path"] =
                Value::String(clean_text(&root.to_string_lossy(), 800));
        }
    });
    json!({
        "ok": removed,
        "type": "dashboard_worktree_result",
        "success": removed,
        "message": if removed { "worktree_deleted" } else { "worktree_not_found" },
        "state": state
    })
}

fn dashboard_worktree_switch(root: &Path, payload: &Value) -> Value {
    let target_path = clean_text(
        payload
            .get("path")
            .and_then(Value::as_str)
            .or_else(|| payload.get("worktree_path").and_then(Value::as_str))
            .unwrap_or(""),
        800,
    );
    if target_path.is_empty() {
        return json!({
            "ok": false,
            "type": "dashboard_worktree_result",
            "success": false,
            "message": "worktree_path_required"
        });
    }
    let new_window = payload
        .get("new_window")
        .and_then(Value::as_bool)
        .or_else(|| payload.get("newWindow").and_then(Value::as_bool))
        .unwrap_or(false);
    let mut switched = false;
    let state = dashboard_worktree_mutate_state(root, |state| {
        let mut rows = state
            .get("worktrees")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for row in &mut rows {
            let row_path = clean_text(row.get("path").and_then(Value::as_str).unwrap_or(""), 800);
            let is_current = row_path == target_path;
            row["is_current"] = Value::Bool(is_current);
            if is_current {
                switched = true;
                state["current_branch"] =
                    Value::String(clean_text(row.get("branch").and_then(Value::as_str).unwrap_or("main"), 200));
            }
        }
        if !switched {
            rows.push(json!({
                "path": target_path,
                "branch": clean_text(payload.get("branch").and_then(Value::as_str).unwrap_or("main"), 200),
                "commit_hash": "",
                "is_current": true,
                "is_bare": false,
                "is_detached": false,
                "is_locked": false,
                "lock_reason": ""
            }));
            switched = true;
        }
        state["worktrees"] = Value::Array(rows);
        state["current_worktree_path"] = Value::String(target_path.clone());
        state["worktree_auto_open_path"] = Value::String(target_path.clone());
        state["switch_new_window"] = Value::Bool(new_window);
    });
    json!({
        "ok": switched,
        "type": "dashboard_worktree_result",
        "success": switched,
        "message": if switched { "worktree_switched" } else { "worktree_switch_failed" },
        "state": state
    })
}

fn dashboard_web_request_url(payload: &Value) -> String {
    clean_text(
        payload
            .get("url")
            .and_then(Value::as_str)
            .or_else(|| payload.get("value").and_then(Value::as_str))
            .unwrap_or(""),
        1000,
    )
}

fn dashboard_web_domain_from_url(url: &str) -> String {
    let without_scheme = if let Some((_, rest)) = url.split_once("://") {
        rest
    } else {
        url
    };
    clean_text(without_scheme.split('/').next().unwrap_or(""), 240)
}

fn dashboard_web_check_is_image_url(payload: &Value) -> Value {
    let url = dashboard_web_request_url(payload);
    let lower = url.to_ascii_lowercase();
    let is_image = lower.starts_with("data:image/")
        || [".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp", ".svg", ".ico"]
            .iter()
            .any(|ext| {
                lower.ends_with(ext) || lower.contains(&format!("{ext}?")) || lower.contains(&format!("{ext}#"))
            });
    json!({
        "ok": true,
        "type": "dashboard_web_check_is_image_url",
        "is_image": is_image,
        "url": url
    })
}

fn dashboard_web_fetch_open_graph_data(payload: &Value) -> Value {
    let url = dashboard_web_request_url(payload);
    let domain = dashboard_web_domain_from_url(&url);
    let path_part = if let Some((_, tail)) = url.split_once(&domain) {
        clean_text(tail, 400)
    } else {
        String::new()
    };
    let title = if path_part.trim().is_empty() {
        format!("{} — link preview", domain)
    } else {
        format!("{}{}", domain, path_part)
    };
    json!({
        "ok": true,
        "type": "dashboard_web_open_graph_data",
        "url": url,
        "domain": domain,
        "title": clean_text(&title, 260),
        "description": "Runtime-synthesized OpenGraph preview fallback",
        "image_url": Value::Null
    })
}

fn dashboard_web_open_in_browser(root: &Path, payload: &Value) -> Value {
    let mut result = dashboard_ui_controller_open_url(root, payload);
    result["type"] = Value::String("dashboard_web_open_in_browser".to_string());
    result
}
