fn handle_primary_dashboard_routes_c(
    root: &Path,
    method: &str,
    path_only: &str,
    body: &[u8],
    snapshot: &Value,
    requester_agent: &str,
) -> Option<CompatApiResponse> {
    if method == "POST" && path_only == "/api/agents" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        if request_mode_is_cua(&request) {
            let unsupported_features = cua_unsupported_features(&request);
            if !unsupported_features.is_empty() {
                let joined = unsupported_features.join(", ");
                let plurality = if unsupported_features.len() == 1 {
                    "is"
                } else {
                    "are"
                };
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({
                        "ok": false,
                        "type": "dashboard_agent_create_validation",
                        "error": "cua_unsupported_features",
                        "mode": "cua",
                        "unsupported_features": unsupported_features,
                        "message": format!("{joined} {plurality} not supported with CUA (Computer Use Agent) mode.")
                    }),
                });
            }
        }
        let requested_parent = clean_agent_id(
            request
                .get("parent_agent_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
        );
        let parent_agent_id = if requested_parent.is_empty() {
            requester_agent.to_string()
        } else {
            requested_parent
        };
        if !requester_agent.is_empty()
            && !parent_agent_id.is_empty()
            && parent_agent_id != requester_agent
            && !actor_can_manage_target(root, snapshot, &requester_agent, &parent_agent_id)
        {
            return Some(CompatApiResponse {
                status: 403,
                payload: json!({
                    "ok": false,
                    "error": "agent_manage_forbidden",
                    "actor_agent_id": requester_agent.clone(),
                    "target_agent_id": parent_agent_id
                }),
            });
        }
        let manifest = clean_text(
            request
                .get("manifest_toml")
                .and_then(Value::as_str)
                .unwrap_or(""),
            20_000,
        );
        let manifest_fields = parse_manifest_fields(&manifest);
        let requested_name = clean_text(
            request
                .get("name")
                .and_then(Value::as_str)
                .or_else(|| manifest_fields.get("name").map(|v| v.as_str()))
                .unwrap_or(""),
            120,
        );
        let requested_role = clean_text(
            request
                .get("role")
                .and_then(Value::as_str)
                .or_else(|| manifest_fields.get("role").map(|v| v.as_str()))
                .unwrap_or("analyst"),
            60,
        );
        let role = if requested_role.is_empty() {
            "analyst".to_string()
        } else {
            requested_role
        };
        let resolved_requested_name =
            dashboard_compat_api_agent_identity::resolve_agent_name(root, &requested_name, &role);
        let agent_id_seed = if resolved_requested_name.is_empty() {
            "agent".to_string()
        } else {
            resolved_requested_name.clone()
        };
        let agent_id = make_agent_id(root, &agent_id_seed);
        let name = if resolved_requested_name.is_empty() {
            dashboard_compat_api_agent_identity::default_agent_name(&agent_id)
        } else {
            resolved_requested_name
        };
        let (default_provider, default_model) = effective_app_settings(root, snapshot);
        let model_provider = clean_text(
            request
                .get("provider")
                .and_then(Value::as_str)
                .or_else(|| manifest_fields.get("provider").map(|v| v.as_str()))
                .unwrap_or(&default_provider),
            80,
        );
        let model_name = clean_text(
            request
                .get("model")
                .and_then(Value::as_str)
                .or_else(|| manifest_fields.get("model").map(|v| v.as_str()))
                .unwrap_or(&default_model),
            120,
        );
        let model_override = if model_provider.is_empty() || model_name.is_empty() {
            "auto".to_string()
        } else {
            format!("{model_provider}/{model_name}")
        };
        let identity =
            dashboard_compat_api_agent_identity::resolve_agent_identity(root, &request, &role);
        let profile_patch = json!({
            "agent_id": agent_id,
            "name": name,
            "role": role,
            "state": "Running",
            "parent_agent_id": if parent_agent_id.is_empty() { Value::Null } else { Value::String(parent_agent_id.clone()) },
            "model_override": model_override,
            "model_provider": model_provider,
            "model_name": model_name,
            "runtime_model": model_name,
            "system_prompt": request.get("system_prompt").cloned().unwrap_or_else(|| json!("")),
            "identity": identity,
            "fallback_models": request.get("fallback_models").cloned().unwrap_or_else(|| json!([])),
            "git_tree_kind": "master",
            "git_branch": "main",
            "workspace_dir": root.to_string_lossy().to_string(),
            "workspace_rel": "",
            "git_tree_ready": true,
            "git_tree_error": "",
            "is_master_agent": true
        });
        let _ = update_profile_patch(root, &agent_id, &profile_patch);
        let contract_obj = request
            .get("contract")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let contract_lifespan = clean_text(
            contract_obj
                .get("lifespan")
                .and_then(Value::as_str)
                .unwrap_or(""),
            40,
        )
        .to_ascii_lowercase();
        let mut termination_condition = clean_text(
            contract_obj
                .get("termination_condition")
                .and_then(Value::as_str)
                .unwrap_or("task_or_timeout"),
            80,
        );
        let explicit_indefinite = contract_obj
            .get("indefinite")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || contract_lifespan == "permanent"
            || contract_lifespan == "indefinite";
        if explicit_indefinite {
            termination_condition = "manual".to_string();
        } else if contract_lifespan == "task" && termination_condition.is_empty() {
            termination_condition = "task_complete".to_string();
        }
        if termination_condition.is_empty() {
            termination_condition = "task_or_timeout".to_string();
        }
        let non_expiring_termination = matches!(
            termination_condition.to_ascii_lowercase().as_str(),
            "manual" | "task_complete"
        );
        let expiry_seconds = contract_obj
            .get("expiry_seconds")
            .and_then(Value::as_i64)
            .unwrap_or(3600)
            .clamp(1, 31 * 24 * 60 * 60);
        let auto_terminate_allowed = contract_obj
            .get("auto_terminate_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(!non_expiring_termination)
            && !non_expiring_termination;
        let idle_terminate_allowed = contract_obj
            .get("idle_terminate_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(!non_expiring_termination)
            && !non_expiring_termination;
        let contract_patch = json!({
            "agent_id": agent_id,
            "status": "active",
            "created_at": crate::now_iso(),
            "updated_at": crate::now_iso(),
            "owner": clean_text(contract_obj.get("owner").and_then(Value::as_str).unwrap_or("dashboard_auto"), 80),
            "mission": clean_text(contract_obj.get("mission").and_then(Value::as_str).unwrap_or("Assist with assigned mission."), 200),
            "termination_condition": termination_condition,
            "expiry_seconds": expiry_seconds,
            "auto_terminate_allowed": auto_terminate_allowed,
            "idle_terminate_allowed": idle_terminate_allowed,
            "parent_agent_id": if parent_agent_id.is_empty() { Value::Null } else { Value::String(parent_agent_id) },
            "conversation_hold": contract_obj.get("conversation_hold").and_then(Value::as_bool).unwrap_or(false),
            "indefinite": explicit_indefinite,
            "lifespan": if explicit_indefinite {
                "permanent"
            } else if termination_condition.eq_ignore_ascii_case("task_complete") {
                "task"
            } else {
                "ephemeral"
            },
            "expires_at": clean_text(contract_obj.get("expires_at").and_then(Value::as_str).unwrap_or(""), 80),
            "source_user_directive": clean_text(contract_obj.get("source_user_directive").and_then(Value::as_str).unwrap_or(""), 800),
            "source_user_directive_receipt": clean_text(contract_obj.get("source_user_directive_receipt").and_then(Value::as_str).unwrap_or(""), 120)
        });
        let _ = upsert_contract_patch(root, &agent_id, &contract_patch);
        append_turn_message(root, &agent_id, "", "");
        let row = agent_row_by_id(root, snapshot, &agent_id).unwrap_or_else(|| {
            json!({
                "id": agent_id,
                "name": name,
                "role": role,
                "state": "Running",
                "model_provider": model_provider,
                "model_name": model_name
            })
        });
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({
                "ok": true,
                "id": row.get("id").cloned().unwrap_or_else(|| json!("")),
                "agent_id": row.get("id").cloned().unwrap_or_else(|| json!("")),
                "name": row
                    .get("name")
                    .cloned()
                    .unwrap_or_else(|| json!(name.clone())),
                "state": row.get("state").cloned().unwrap_or_else(|| json!("Running")),
                "model_provider": row.get("model_provider").cloned().unwrap_or_else(|| json!(default_provider)),
                "model_name": row.get("model_name").cloned().unwrap_or_else(|| json!(default_model)),
                "runtime_model": row.get("runtime_model").cloned().unwrap_or_else(|| json!(default_model)),
                "created_at": row.get("created_at").cloned().unwrap_or_else(|| json!(crate::now_iso()))
            }),
        });
    }

    None
}
