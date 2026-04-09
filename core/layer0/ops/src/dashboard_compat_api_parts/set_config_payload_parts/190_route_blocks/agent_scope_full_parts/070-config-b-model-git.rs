fn finalize_agent_scope_config_patch(
    root: &Path,
    snapshot: &Value,
    agent_id: &str,
    existing: &Option<Value>,
    patch: Value,
    should_seed_intro: bool,
    resolved_role: String,
    rename_notice: Option<Value>,
) -> CompatApiResponse {
    let _ = update_profile_patch(root, agent_id, &patch);
    if patch.get("contract").map(Value::is_object).unwrap_or(false) {
        let _ = upsert_contract_patch(root, agent_id, patch.get("contract").unwrap_or(&json!({})));
    } else if patch.get("expiry_seconds").is_some()
        || patch.get("termination_condition").is_some()
        || patch.get("auto_terminate_allowed").is_some()
        || patch.get("idle_terminate_allowed").is_some()
    {
        let _ = upsert_contract_patch(root, agent_id, &patch);
    }
    if should_seed_intro {
        let intro_name = clean_text(
            patch
                .get("name")
                .and_then(Value::as_str)
                .or_else(|| {
                    existing
                        .as_ref()
                        .and_then(|row| row.get("name").and_then(Value::as_str))
                })
                .unwrap_or(agent_id),
            120,
        );
        let _ = crate::dashboard_agent_state::seed_intro_message(
            root,
            agent_id,
            &resolved_role,
            &intro_name,
        );
    }
    let row = agent_row_by_id(root, snapshot, agent_id).unwrap_or_else(|| json!({"id": agent_id}));
    let mut payload = json!({"ok": true, "agent_id": agent_id, "agent": row});
    if let Some(notice) = rename_notice {
        payload["rename_notice"] = notice;
    }
    CompatApiResponse {
        status: 200,
        payload,
    }
}

fn handle_agent_scope_model_mode_git_routes(
    root: &Path,
    method: &str,
    segments: &[String],
    body: &[u8],
    snapshot: &Value,
    agent_id: &str,
) -> Option<CompatApiResponse> {
    if method == "PUT" && segments.len() == 1 && segments[0] == "model" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let requested = clean_text(
            request.get("model").and_then(Value::as_str).unwrap_or(""),
            200,
        );
        if requested.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": "model_required"}),
            });
        }
        let (default_provider, default_model) = effective_app_settings(root, snapshot);
        let (provider, model) = split_model_ref(&requested, &default_provider, &default_model);
        let _ = update_profile_patch(
            root,
            agent_id,
            &json!({
                "model_override": format!("{provider}/{model}"),
                "model_provider": provider,
                "model_name": model,
                "runtime_model": model
            }),
        );
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({
                "ok": true,
                "agent_id": agent_id,
                "provider": provider,
                "model": model,
                "runtime_model": model
            }),
        });
    }
    if method == "PUT" && segments.len() == 1 && segments[0] == "mode" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let mode = clean_text(
            request.get("mode").and_then(Value::as_str).unwrap_or(""),
            40,
        );
        let _ = update_profile_patch(
            root,
            agent_id,
            &json!({"mode": mode, "updated_at": crate::now_iso()}),
        );
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "agent_id": agent_id, "mode": mode}),
        });
    }
    if method == "GET" && segments.len() == 1 && segments[0] == "git-trees" {
        return Some(CompatApiResponse {
            status: 200,
            payload: git_tree_payload_for_agent(root, snapshot, agent_id),
        });
    }
    if method == "POST"
        && segments.len() == 2
        && segments[0] == "git-tree"
        && segments[1] == "switch"
    {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let branch = clean_text(
            request.get("branch").and_then(Value::as_str).unwrap_or(""),
            180,
        );
        if branch.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": "branch_required"}),
            });
        }
        let require_new = request
            .get("require_new")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let result = crate::dashboard_git_runtime::switch_agent_worktree(
            root,
            agent_id,
            &branch,
            require_new,
        );
        let kind = clean_text(
            result
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("isolated"),
            40,
        );
        let default_workspace_dir = root.to_string_lossy().to_string();
        let workspace_dir = clean_text(
            result
                .get("workspace_dir")
                .and_then(Value::as_str)
                .unwrap_or(default_workspace_dir.as_str()),
            4000,
        );
        let workspace_rel = clean_text(
            result
                .get("workspace_rel")
                .and_then(Value::as_str)
                .unwrap_or(""),
            4000,
        );
        let ready = result
            .get("ready")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let error = clean_text(
            result.get("error").and_then(Value::as_str).unwrap_or(""),
            280,
        );
        let _ = update_profile_patch(
            root,
            agent_id,
            &json!({
                "git_branch": clean_text(result.get("branch").and_then(Value::as_str).unwrap_or(&branch), 180),
                "git_tree_kind": kind,
                "workspace_dir": workspace_dir,
                "workspace_rel": workspace_rel,
                "git_tree_ready": ready,
                "git_tree_error": error,
                "updated_at": crate::now_iso()
            }),
        );
        return Some(CompatApiResponse {
            status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                200
            } else {
                400
            },
            payload: git_tree_payload_for_agent(root, snapshot, agent_id),
        });
    }
    None
}
