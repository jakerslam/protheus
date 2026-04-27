const AGENT_PERMISSION_KEYS: [&str; 10] = ["web.search.basic", "web.fetch.url", "agent.spawn", "agent.permissions.manage", "github.issue.create", "file.read.workspace", "file.write.workspace", "file.delete.workspace", "terminal.exec", "memory.write"];
const AGENT_PERMISSION_CATEGORIES: [&str; 6] = ["agent", "web", "file", "github", "terminal", "memory"];

fn parse_permissions_payload(raw: &Value) -> Option<Value> {
    if raw.is_object() {
        return Some(raw.clone());
    }
    raw.as_str()
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
        .filter(Value::is_object)
}

fn normalize_permission_trit(raw: &Value) -> &'static str {
    if let Some(value) = raw.as_str() {
        let lowered = clean_text(value, 40).to_ascii_lowercase();
        return match lowered.as_str() {
            "allow" | "true" | "1" | "+1" => "allow",
            "deny" | "false" | "-1" => "deny",
            "inherit" | "0" => "inherit",
            _ => "inherit",
        };
    }
    if let Some(value) = raw.as_i64() {
        return if value > 0 { "allow" } else if value < 0 { "deny" } else { "inherit" };
    }
    if let Some(value) = raw.as_bool() {
        return if value { "allow" } else { "deny" };
    }
    "inherit"
}

fn default_permissions_manifest() -> Value {
    json!({
        "version": 1,
        "trit": { "deny": -1, "inherit": 0, "allow": 1 },
        "category_defaults": {
            "agent": "inherit",
            "web": "inherit",
            "file": "inherit",
            "github": "inherit",
            "terminal": "inherit",
            "memory": "inherit"
        },
        "grants": {
            "web.search.basic": "allow"
        }
    })
}

fn normalize_permissions_manifest(raw: &Value) -> Value {
    let mut out = default_permissions_manifest();
    let source = parse_permissions_payload(raw).unwrap_or_else(|| json!({}));
    let source_obj = source.as_object().cloned().unwrap_or_default();
    if let Some(raw_categories) = source_obj
        .get("category_defaults")
        .or_else(|| source_obj.get("categories"))
        .and_then(Value::as_object)
    {
        for category in AGENT_PERMISSION_CATEGORIES {
            if let Some(value) = raw_categories.get(category) {
                out["category_defaults"][category] = json!(normalize_permission_trit(value));
            }
        }
    }
    if let Some(raw_grants) = source_obj.get("grants").and_then(Value::as_object) {
        for permission in AGENT_PERMISSION_KEYS {
            if let Some(value) = raw_grants.get(permission) {
                out["grants"][permission] = json!(normalize_permission_trit(value));
            }
        }
    }
    for permission in AGENT_PERMISSION_KEYS {
        if let Some(value) = source_obj.get(permission) {
            out["grants"][permission] = json!(normalize_permission_trit(value));
        }
    }
    out["grants"]["web.search.basic"] = json!("allow");
    out
}

fn permissions_manifest_from_agent_row(row: &Value) -> Option<Value> {
    let from_contract = row
        .get("contract")
        .and_then(|contract| {
            contract
                .get("permissions_manifest")
                .or_else(|| contract.get("permissions"))
        })
        .cloned();
    let from_root = row
        .get("permissions_manifest")
        .or_else(|| row.get("permissions"))
        .cloned();
    from_contract
        .or(from_root)
        .map(|raw| normalize_permissions_manifest(&raw))
}

fn permissions_manifest_allows(manifest: &Value, permission: &str) -> bool {
    let normalized = normalize_permissions_manifest(manifest);
    let grant = normalized
        .get("grants")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(permission))
        .map(normalize_permission_trit)
        .unwrap_or("inherit");
    if grant == "allow" {
        return true;
    }
    if grant == "deny" {
        return false;
    }
    let category = permission.split('.').next().unwrap_or("agent");
    let category_default = normalized
        .get("category_defaults")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(category))
        .map(normalize_permission_trit)
        .unwrap_or("inherit");
    category_default == "allow"
}

fn clamp_child_permissions_manifest(parent: &Value, requested_child: &Value) -> Value {
    let mut child = normalize_permissions_manifest(requested_child);
    for category in AGENT_PERMISSION_CATEGORIES {
        child["category_defaults"][category] = json!("inherit");
    }
    for permission in AGENT_PERMISSION_KEYS {
        if permissions_manifest_allows(&child, permission) && !permissions_manifest_allows(parent, permission) {
            child["grants"][permission] = json!("inherit");
        }
    }
    child
}

fn resolve_new_agent_permissions_manifest(
    request: &Value,
    contract_obj: &Value,
    parent_row: Option<&Value>,
) -> Value {
    let requested_raw = request
        .get("permissions")
        .or_else(|| request.get("permissions_manifest"))
        .or_else(|| contract_obj.get("permissions_manifest"))
        .or_else(|| contract_obj.get("permissions"))
        .cloned();
    let explicit_requested = requested_raw
        .as_ref()
        .and_then(parse_permissions_payload)
        .filter(|value| value.as_object().map(|rows| !rows.is_empty()).unwrap_or(false));
    let parent_manifest = parent_row.and_then(permissions_manifest_from_agent_row);
    match (explicit_requested, parent_manifest) {
        (Some(requested), Some(parent)) => clamp_child_permissions_manifest(&parent, &requested),
        (Some(requested), None) => normalize_permissions_manifest(&requested),
        (None, Some(parent)) => parent,
        (None, None) => default_permissions_manifest(),
    }
}

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
                let unsupported_features_signature = unsupported_features.join("|");
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
                        "unsupported_features_count": unsupported_features.len(),
                        "unsupported_features_signature": unsupported_features_signature,
                        "validation_contract_version": "v1",
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
        let parent_row = if parent_agent_id.is_empty() {
            None
        } else {
            agent_row_by_id(root, snapshot, &parent_agent_id)
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
                    "actor_agent_id": requester_agent,
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
        let raw_contract_obj = request
            .get("contract")
            .cloned()
            .or_else(|| request.get("initial_contract").cloned())
            .or_else(|| request.get("init_contract").cloned())
            .unwrap_or_else(|| json!({}));
        let contract_obj = if raw_contract_obj.is_object() {
            raw_contract_obj
        } else if let Some(raw_text) = raw_contract_obj.as_str() {
            serde_json::from_str::<Value>(raw_text).unwrap_or_else(|_| json!({}))
        } else {
            json!({})
        };
        let permissions_manifest = resolve_new_agent_permissions_manifest(
            &request,
            &contract_obj,
            parent_row.as_ref(),
        );
        let permissions_updated_at = crate::now_iso();
        let permissions_receipt = crate::deterministic_receipt_hash(&json!({
            "type": "agent_permissions_manifest_init",
            "agent_id": agent_id,
            "parent_agent_id": parent_agent_id,
            "permissions_manifest": permissions_manifest,
            "updated_at": permissions_updated_at
        }));
        let contract_initial_prompt = clean_text(
            contract_obj
                .get("initial_prompt")
                .and_then(Value::as_str)
                .or_else(|| contract_obj.get("initialPrompt").and_then(Value::as_str))
                .or_else(|| contract_obj.get("prompt").and_then(Value::as_str))
                .unwrap_or(""),
            12_000,
        );
        let requested_system_prompt = clean_text(
            request
                .get("system_prompt")
                .and_then(Value::as_str)
                .unwrap_or(""),
            12_000,
        );
        let resolved_system_prompt = if requested_system_prompt.is_empty() {
            contract_initial_prompt.clone()
        } else {
            requested_system_prompt
        };
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
            "system_prompt": resolved_system_prompt.clone(),
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
        let contract_lifespan = clean_text(
            contract_obj
                .get("lifespan")
                .and_then(Value::as_str)
                .or_else(|| request.get("lifespan").and_then(Value::as_str))
                .unwrap_or(""),
            40,
        )
        .to_ascii_lowercase();
        let mut termination_condition = clean_text(
            contract_obj
                .get("termination_condition")
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        );
        let explicit_indefinite = contract_obj.get("indefinite").and_then(Value::as_bool).unwrap_or(false)
            || contract_lifespan == "permanent" || contract_lifespan == "indefinite";
        if explicit_indefinite {
            termination_condition = "manual".to_string();
        } else if contract_lifespan == "task" {
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
        let persisted_initial_prompt = if contract_initial_prompt.is_empty() {
            resolved_system_prompt
        } else {
            contract_initial_prompt
        };
        let contract_patch = json!({
            "agent_id": agent_id,
            "status": "active",
            "created_at": crate::now_iso(),
            "updated_at": crate::now_iso(),
            "owner": clean_text(contract_obj.get("owner").and_then(Value::as_str).unwrap_or("dashboard_auto"), 80),
            "mission": clean_text(contract_obj.get("mission").and_then(Value::as_str).unwrap_or("Assist with assigned mission."), 200),
            "initial_prompt": persisted_initial_prompt,
            "permissions_manifest": permissions_manifest.clone(),
            "permissions_receipt": permissions_receipt.clone(),
            "permissions_revision": 1,
            "permissions_updated_at": permissions_updated_at,
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
            "budget_tokens": contract_obj.get("budget_tokens").cloned().unwrap_or(Value::Null),
            "merge_strategy": contract_obj.get("merge_strategy").cloned().unwrap_or(Value::Null),
            "context_slice": contract_obj.get("context_slice").cloned().unwrap_or(Value::Null),
            "spawn_guard": contract_obj.get("spawn_guard").cloned().unwrap_or(Value::Null),
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
                "role": row
                    .get("role")
                    .cloned()
                    .unwrap_or_else(|| json!(role.clone())),
                "state": row.get("state").cloned().unwrap_or_else(|| json!("Running")),
                "model_provider": row.get("model_provider").cloned().unwrap_or_else(|| json!(default_provider)),
                "model_name": row.get("model_name").cloned().unwrap_or_else(|| json!(default_model)),
                "runtime_model": row.get("runtime_model").cloned().unwrap_or_else(|| json!(default_model)),
                "created_at": row.get("created_at").cloned().unwrap_or_else(|| json!(crate::now_iso())),
                "avatar_url": row.get("avatar_url").cloned().unwrap_or_else(|| json!("")),
                "identity": row.get("identity").cloned().unwrap_or_else(|| identity.clone()),
                "contract": row.get("contract").cloned().unwrap_or_else(|| contract_patch.clone()),
                "sidebar_status_state": row.get("sidebar_status_state").cloned().unwrap_or_else(|| json!("active")),
                "sidebar_status_label": row.get("sidebar_status_label").cloned().unwrap_or_else(|| json!("active")),
                "sidebar_status_source": row.get("sidebar_status_source").cloned().unwrap_or_else(|| json!("")),
                "sidebar_status_source_sequence": row.get("sidebar_status_source_sequence").cloned().unwrap_or_else(|| json!("")),
                "sidebar_status_age_seconds": row.get("sidebar_status_age_seconds").cloned().unwrap_or_else(|| json!(0)),
                "sidebar_status_stale": row.get("sidebar_status_stale").cloned().unwrap_or_else(|| json!(false)),
                "permissions_manifest": permissions_manifest,
                "permissions_receipt": permissions_receipt
            }),
        });
    }

    None
}
