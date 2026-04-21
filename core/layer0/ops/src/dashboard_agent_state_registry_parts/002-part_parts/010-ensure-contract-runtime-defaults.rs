fn ensure_contract_runtime_defaults(contract: &mut Value) {
    if contract.get("expiry_seconds").is_none() {
        contract["expiry_seconds"] = Value::from(DEFAULT_EXPIRY_SECONDS);
    }
    if contract.get("idle_timeout_seconds").is_none() {
        contract["idle_timeout_seconds"] = Value::from(DEFAULT_IDLE_TIMEOUT_SECONDS);
    }
    if contract.get("idle_terminate_allowed").is_none() {
        contract["idle_terminate_allowed"] = Value::Bool(true);
    }
}

fn apply_lifecycle_contract_rules(
    contract: &mut Value,
    explicit_indefinite: bool,
    termination_condition: &str,
) {
    if explicit_indefinite {
        contract["termination_condition"] = Value::String("manual".to_string());
        contract["auto_terminate_allowed"] = Value::Bool(false);
        contract["idle_terminate_allowed"] = Value::Bool(false);
        contract["expires_at"] = Value::String(String::new());
        contract["indefinite"] = Value::Bool(true);
        contract["lifespan"] = Value::String("permanent".to_string());
        return;
    }
    if termination_condition.starts_with("manual") || termination_condition == "task_complete" {
        contract["auto_terminate_allowed"] = Value::Bool(false);
        contract["idle_terminate_allowed"] = Value::Bool(false);
        if termination_condition == "task_complete" {
            contract["lifespan"] = Value::String("task".to_string());
        } else if contract
            .get("lifespan")
            .and_then(Value::as_str)
            .map(|v| v.trim().is_empty())
            .unwrap_or(true)
        {
            contract["lifespan"] = Value::String("permanent".to_string());
        }
    }
}

pub fn upsert_contract(root: &Path, agent_id: &str, patch: &Value) -> Value {
    let id = normalize_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let mut state = load_contracts_state(root);
    let contracts = as_object_mut(&mut state, "contracts");
    let mut contract = contracts
        .get(&id)
        .cloned()
        .unwrap_or_else(|| default_contract(&id));
    let mut saw_status_patch = false;
    let mut saw_lifecycle_patch = false;
    let mut explicit_indefinite = false;
    if let Some(obj) = patch.as_object() {
        for (key, value) in obj {
            if key == "indefinite" && value.as_bool().unwrap_or(false) {
                explicit_indefinite = true;
                saw_lifecycle_patch = true;
            }
            if key == "lifespan" {
                let lifespan = clean_text(value.as_str().unwrap_or(""), 40).to_ascii_lowercase();
                if lifespan == "permanent" {
                    explicit_indefinite = true;
                    saw_lifecycle_patch = true;
                    contract["lifespan"] = Value::String("permanent".to_string());
                } else if lifespan == "task" {
                    contract["termination_condition"] = Value::String("task_complete".to_string());
                    saw_lifecycle_patch = true;
                    contract["lifespan"] = Value::String("task".to_string());
                } else if lifespan == "ephemeral" {
                    saw_lifecycle_patch = true;
                    contract["lifespan"] = Value::String("ephemeral".to_string());
                }
            }
            if matches!(
                key.as_str(),
                "mission"
                    | "owner"
                    | "termination_condition"
                    | "expires_at"
                    | "expiry_seconds"
                    | "idle_timeout_seconds"
                    | "idle_terminate_allowed"
                    | "parent_agent_id"
            ) {
                saw_lifecycle_patch = true;
            }
            if matches!(
                key.as_str(),
                "mission"
                    | "owner"
                    | "initial_prompt"
                    | "permissions_manifest" | "permissions_receipt" | "permissions_revision" | "permissions_updated_at"
                    | "termination_condition"
                    | "expires_at"
                    | "parent_agent_id"
                    | "status"
                    | "termination_reason"
                    | "created_at"
                    | "revived_from_contract_id"
                    | "idle_terminate_allowed"
            ) {
                contract[key] = value.clone();
            }
            if key == "status" { saw_status_patch = true; }
            if key == "expiry_seconds" {
                contract["expiry_seconds"] = Value::from(parse_expiry_seconds(Some(value)));
            }
            if key == "auto_terminate_allowed" {
                saw_lifecycle_patch = true;
                contract["auto_terminate_allowed"] = Value::Bool(value.as_bool().unwrap_or(true));
            }
            if key == "idle_timeout_seconds" {
                saw_lifecycle_patch = true;
                contract["idle_timeout_seconds"] = Value::from(parse_idle_timeout_seconds(Some(value)));
            }
            if key == "idle_terminate_allowed" {
                saw_lifecycle_patch = true;
                contract["idle_terminate_allowed"] = Value::Bool(value.as_bool().unwrap_or(true));
            }
            if key == "permissions_revision" { contract["permissions_revision"] = Value::from(value.as_i64().unwrap_or(1).max(1)); }
        }
    }
    let existing_status = contract
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("active");
    if !saw_status_patch && saw_lifecycle_patch && existing_status == "terminated" {
        contract["status"] = Value::String("active".to_string());
        contract["created_at"] = Value::String(now_iso());
        contract["updated_at"] = Value::String(now_iso());
        if !patch
            .as_object()
            .map(|obj| obj.contains_key("expires_at"))
            .unwrap_or(false)
        {
            contract["expires_at"] = Value::String(String::new());
        }
        if let Some(obj) = contract.as_object_mut() {
            obj.remove("terminated_at");
            obj.remove("termination_reason");
        }
    }
    ensure_contract_runtime_defaults(&mut contract);
    let termination_condition = clean_text(
        contract
            .get("termination_condition")
            .and_then(Value::as_str)
            .unwrap_or("task_or_timeout"),
        80,
    )
    .to_ascii_lowercase();
    apply_lifecycle_contract_rules(&mut contract, explicit_indefinite, &termination_condition);
    let normalized_status = clean_text(
        contract
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("active"),
        40,
    )
    .to_ascii_lowercase();
    if normalized_status == "active" {
        if let Some(obj) = contract.as_object_mut() {
            obj.remove("terminated_at");
            if clean_text(
                obj.get("termination_reason")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                80,
            )
            .is_empty()
            {
                obj.remove("termination_reason");
            }
        }
    }
    if contract
        .get("created_at")
        .and_then(Value::as_str)
        .map(|v| v.is_empty())
        .unwrap_or(true)
    {
        contract["created_at"] = Value::String(now_iso());
    }
    contract["updated_at"] = Value::String(now_iso());
    contracts.insert(id.clone(), contract.clone());
    save_contracts_state(root, state);
    json!({"ok": true, "type": "dashboard_agent_contract", "agent_id": id, "contract": contract})
}
