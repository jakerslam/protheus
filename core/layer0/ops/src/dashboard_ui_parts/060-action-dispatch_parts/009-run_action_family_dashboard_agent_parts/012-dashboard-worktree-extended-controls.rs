fn dashboard_worktree_checkout_branch(root: &Path, payload: &Value) -> Value {
    let branch = clean_text(
        payload
            .get("branch")
            .and_then(Value::as_str)
            .or_else(|| payload.get("name").and_then(Value::as_str))
            .or_else(|| payload.get("value").and_then(Value::as_str))
            .unwrap_or(""),
        200,
    );
    if branch.is_empty() {
        return json!({
            "ok": false,
            "type": "dashboard_worktree_checkout_branch",
            "error": "branch_required"
        });
    }
    let state = dashboard_worktree_mutate_state(root, |state| {
        let mut local = dashboard_worktree_extract_local_branches(state);
        local.push(branch.clone());
        state["local_branches"] = Value::Array(
            dashboard_worktree_sorted_unique_branch_rows(local)
                .into_iter()
                .map(Value::String)
                .collect(),
        );
        state["current_branch"] = Value::String(branch.clone());
        state["last_checkout_branch"] = Value::String(branch.clone());
        state["last_checkout_branch_at"] = Value::String(crate::now_iso());
    });
    json!({
        "ok": true,
        "type": "dashboard_worktree_checkout_branch",
        "branch": branch,
        "state": state
    })
}

fn dashboard_worktree_create_include(root: &Path, payload: &Value) -> Value {
    let include_path = clean_text(
        payload
            .get("path")
            .and_then(Value::as_str)
            .or_else(|| payload.get("worktree_path").and_then(Value::as_str))
            .or_else(|| payload.get("value").and_then(Value::as_str))
            .unwrap_or(""),
        800,
    );
    if include_path.is_empty() {
        return json!({
            "ok": false,
            "type": "dashboard_worktree_create_include",
            "error": "worktree_path_required"
        });
    }
    let state = dashboard_worktree_mutate_state(root, |state| {
        state["worktree_include_enabled"] = Value::Bool(true);
        state["worktree_auto_open_path"] = Value::String(include_path.clone());
        state["last_worktree_include_path"] = Value::String(include_path.clone());
        state["last_worktree_include_at"] = Value::String(crate::now_iso());
        state["worktree_include_count"] = Value::from(
            i64_from_value(state.get("worktree_include_count"), 0).saturating_add(1),
        );
    });
    json!({
        "ok": true,
        "type": "dashboard_worktree_create_include",
        "include_path": include_path,
        "include_directive": format!("worktree.include={}", include_path),
        "state": state
    })
}

fn dashboard_worktree_get_defaults(root: &Path) -> Value {
    let state = dashboard_worktree_read_state(root);
    let root_path = clean_text(&root.to_string_lossy(), 800);
    json!({
        "ok": true,
        "type": "dashboard_worktree_defaults",
        "defaults": {
            "git_root_path": root_path,
            "base_branch": clean_text(state.get("current_branch").and_then(Value::as_str).unwrap_or("main"), 200),
            "new_window_default": false,
            "auto_open_worktree": true,
            "default_timeout_seconds": 300
        },
        "state": state
    })
}

fn dashboard_worktree_get_include_status(root: &Path) -> Value {
    let state = dashboard_worktree_read_state(root);
    let include_path = clean_text(
        state
            .get("worktree_auto_open_path")
            .and_then(Value::as_str)
            .unwrap_or(""),
        800,
    );
    let include_enabled = state
        .get("worktree_include_enabled")
        .and_then(Value::as_bool)
        .unwrap_or(!include_path.is_empty());
    json!({
        "ok": true,
        "type": "dashboard_worktree_include_status",
        "include_enabled": include_enabled,
        "include_path": include_path,
        "last_worktree_include_at": clean_text(state.get("last_worktree_include_at").and_then(Value::as_str).unwrap_or(""), 80),
        "state": state
    })
}

fn dashboard_worktree_merge(root: &Path, payload: &Value) -> Value {
    let source_branch = clean_text(
        payload
            .get("source_branch")
            .and_then(Value::as_str)
            .or_else(|| payload.get("branch").and_then(Value::as_str))
            .or_else(|| payload.get("value").and_then(Value::as_str))
            .unwrap_or(""),
        200,
    );
    if source_branch.is_empty() {
        return json!({
            "ok": false,
            "type": "dashboard_worktree_merge_result",
            "error": "source_branch_required"
        });
    }
    let target_branch = clean_text(
        payload
            .get("target_branch")
            .and_then(Value::as_str)
            .or_else(|| payload.get("into").and_then(Value::as_str))
            .unwrap_or("main"),
        200,
    );
    let state = dashboard_worktree_mutate_state(root, |state| {
        state["current_branch"] = Value::String(target_branch.clone());
        state["last_merge_source_branch"] = Value::String(source_branch.clone());
        state["last_merge_target_branch"] = Value::String(target_branch.clone());
        state["last_merge_at"] = Value::String(crate::now_iso());
        state["merge_count"] =
            Value::from(i64_from_value(state.get("merge_count"), 0).saturating_add(1));
    });
    json!({
        "ok": true,
        "type": "dashboard_worktree_merge_result",
        "merged": true,
        "source_branch": source_branch,
        "target_branch": target_branch,
        "state": state
    })
}

fn dashboard_worktree_track_view_opened(root: &Path, payload: &Value) -> Value {
    let view = clean_text(
        payload
            .get("view")
            .and_then(Value::as_str)
            .or_else(|| payload.get("value").and_then(Value::as_str))
            .unwrap_or("worktrees"),
        120,
    );
    let state = dashboard_worktree_mutate_state(root, |state| {
        state["last_view_opened"] = Value::String(view.clone());
        state["last_view_opened_at"] = Value::String(crate::now_iso());
        state["view_opened_count"] =
            Value::from(i64_from_value(state.get("view_opened_count"), 0).saturating_add(1));
    });
    json!({
        "ok": true,
        "type": "dashboard_worktree_view_opened",
        "view": view,
        "state": state
    })
}
