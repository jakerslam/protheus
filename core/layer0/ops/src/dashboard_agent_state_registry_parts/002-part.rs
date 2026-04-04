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
                } else if lifespan == "task" {
                    contract["termination_condition"] = Value::String("task_complete".to_string());
                    saw_lifecycle_patch = true;
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
            if key == "status" {
                saw_status_patch = true;
            }
            if key == "expiry_seconds" {
                contract["expiry_seconds"] = Value::from(parse_expiry_seconds(Some(value)));
            }
            if key == "auto_terminate_allowed" {
                saw_lifecycle_patch = true;
                contract["auto_terminate_allowed"] = Value::Bool(value.as_bool().unwrap_or(true));
            }
            if key == "idle_timeout_seconds" {
                saw_lifecycle_patch = true;
                contract["idle_timeout_seconds"] =
                    Value::from(parse_idle_timeout_seconds(Some(value)));
            }
            if key == "idle_terminate_allowed" {
                saw_lifecycle_patch = true;
                contract["idle_terminate_allowed"] = Value::Bool(value.as_bool().unwrap_or(true));
            }
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
    if contract.get("expiry_seconds").is_none() { contract["expiry_seconds"] = Value::from(DEFAULT_EXPIRY_SECONDS); }
    if contract.get("idle_timeout_seconds").is_none() { contract["idle_timeout_seconds"] = Value::from(DEFAULT_IDLE_TIMEOUT_SECONDS); }
    if contract.get("idle_terminate_allowed").is_none() { contract["idle_terminate_allowed"] = Value::Bool(true); }
    let termination_condition = clean_text(contract.get("termination_condition").and_then(Value::as_str).unwrap_or("task_or_timeout"), 80).to_ascii_lowercase();
    if explicit_indefinite {
        contract["termination_condition"] = Value::String("manual".to_string());
        contract["auto_terminate_allowed"] = Value::Bool(false);
        contract["idle_terminate_allowed"] = Value::Bool(false);
        contract["expires_at"] = Value::String(String::new());
        contract["indefinite"] = Value::Bool(true);
        contract["lifespan"] = Value::String("permanent".to_string());
    } else if termination_condition.starts_with("manual") || termination_condition == "task_complete" {
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
    let normalized_status = clean_text(
        contract.get("status").and_then(Value::as_str).unwrap_or("active"),
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

pub fn enforce_expired_contracts(root: &Path) -> Value {
    let mut state = load_contracts_state(root);
    let now = Utc::now();
    let mut terminated = Vec::<Value>::new();
    {
        let contracts = as_object_mut(&mut state, "contracts");
        for (agent_id, contract) in contracts.iter_mut() {
            let status = contract
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("active");
            if status != "active" {
                continue;
            }
            let auto_terminate_allowed = contract
                .get("auto_terminate_allowed")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let idle_terminate_allowed = contract
                .get("idle_terminate_allowed")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let termination_condition = clean_text(contract.get("termination_condition").and_then(Value::as_str).unwrap_or("task_or_timeout"), 80).to_ascii_lowercase();
            let lifespan = clean_text(
                contract.get("lifespan").and_then(Value::as_str).unwrap_or(""),
                40,
            )
            .to_ascii_lowercase();
            let explicit_indefinite = contract
                .get("indefinite")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || lifespan == "permanent"
                || lifespan == "indefinite";
            let non_expiring = termination_condition.starts_with("manual")
                || termination_condition == "task_complete"
                || explicit_indefinite
                || (!auto_terminate_allowed && !idle_terminate_allowed);
            let expires_at = contract
                .get("expires_at")
                .and_then(Value::as_str)
                .unwrap_or("");
            let created_at = contract
                .get("created_at")
                .and_then(Value::as_str)
                .unwrap_or("");
            let expiry_seconds = parse_expiry_seconds(contract.get("expiry_seconds"));
            let idle_timeout_seconds =
                parse_idle_timeout_seconds(contract.get("idle_timeout_seconds"));
            let created_ts = parse_ts(created_at).unwrap_or(now);
            let expiry_ts = if let Some(ts) = parse_ts(expires_at) {
                ts
            } else {
                created_ts + chrono::Duration::seconds(expiry_seconds)
            };
            let last_activity_ts = session_last_activity_ts(root, agent_id).unwrap_or(created_ts);
            let idle_deadline = last_activity_ts + chrono::Duration::seconds(idle_timeout_seconds);
            let termination_reason = if !non_expiring && auto_terminate_allowed && now >= expiry_ts {
                Some("contract_expired")
            } else if !non_expiring && idle_terminate_allowed && now >= idle_deadline {
                Some("idle_timeout")
            } else {
                None
            };
            if let Some(reason) = termination_reason {
                contract["status"] = Value::String("terminated".to_string());
                contract["termination_reason"] = Value::String(reason.to_string());
                contract["terminated_at"] = Value::String(now_iso());
                contract["updated_at"] = Value::String(now_iso());
                let row = json!({
                    "agent_id": agent_id,
                    "contract_id": contract
                        .get("contract_id")
                        .cloned()
                        .unwrap_or(Value::String(String::new())),
                    "termination_reason": reason,
                    "terminated_at": contract
                        .get("terminated_at")
                        .cloned()
                        .unwrap_or(Value::String(now_iso()))
                });
                terminated.push(row);
            }
        }
    }
    {
        let history = as_array_mut(&mut state, "terminated_history");
        for row in &terminated {
            history.push(row.clone());
        }
    }
    save_contracts_state(root, state);
    json!({"ok": true, "type": "dashboard_contract_enforcement", "terminated": terminated})
}

fn purge_agent_artifacts(root: &Path, agent_id: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    if id.is_empty() {
        return json!({
            "removed_profile": false,
            "removed_archived": false,
            "removed_session_file": false,
            "git_cleanup": json!({"ok": false, "error": "agent_id_required"})
        });
    }
    let mut profiles = load_profiles_state(root);
    let git_branch_hint = profiles
        .get("agents")
        .and_then(Value::as_object)
        .and_then(|agents| agents.get(&id))
        .and_then(|profile| profile.get("git_branch").and_then(Value::as_str))
        .map(|v| clean_text(v, 180))
        .filter(|v| !v.is_empty());
    let git_cleanup = crate::dashboard_git_runtime::cleanup_agent_git_artifacts(
        root,
        &id,
        git_branch_hint.as_deref(),
    );
    let removed_profile = {
        let agents = as_object_mut(&mut profiles, "agents");
        agents.remove(&id).is_some()
    };
    save_profiles_state(root, profiles);

    let mut archived = load_archived_state(root);
    let removed_archived = {
        let agents = as_object_mut(&mut archived, "agents");
        agents.remove(&id).is_some()
    };
    save_archived_state(root, archived);

    let removed_session_file = fs::remove_file(session_path(root, &id)).is_ok();
    json!({
        "removed_profile": removed_profile,
        "removed_archived": removed_archived,
        "removed_session_file": removed_session_file,
        "git_cleanup": git_cleanup
    })
}

pub fn terminated_entries(root: &Path) -> Value {
    let state = load_contracts_state(root);
    let mut entries = state
        .get("terminated_history")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let contracts = state
        .get("contracts")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for (agent_id, contract) in contracts {
        let status = contract
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("active");
        if status != "terminated" {
            continue;
        }
        let contract_id = clean_text(
            contract
                .get("contract_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let exists = entries.iter().any(|row| {
            normalize_agent_id(row.get("agent_id").and_then(Value::as_str).unwrap_or(""))
                == agent_id
                && clean_text(
                    row.get("contract_id").and_then(Value::as_str).unwrap_or(""),
                    120,
                ) == contract_id
        });
        if !exists {
            entries.push(json!({
                "agent_id": agent_id,
                "contract_id": contract_id,
                "termination_reason": contract.get("termination_reason").cloned().unwrap_or_else(|| Value::String("terminated".to_string())),
                "terminated_at": contract.get("terminated_at").cloned().unwrap_or_else(|| Value::String(now_iso()))
            }));
        }
    }

    let profiles = load_profiles_state(root)
        .get("agents")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let archived = load_archived_state(root)
        .get("agents")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut archived_candidates = archived.keys().cloned().collect::<Vec<_>>();
    for (agent_id, profile) in &profiles {
        let state = clean_text(profile.get("state").and_then(Value::as_str).unwrap_or(""), 40)
            .to_ascii_lowercase();
        if state == "archived" {
            archived_candidates.push(agent_id.clone());
        }
    }
    for raw_id in archived_candidates {
        let agent_id = normalize_agent_id(&raw_id);
        if agent_id.is_empty() {
            continue;
        }
        let exists = entries.iter().any(|row| {
            normalize_agent_id(row.get("agent_id").and_then(Value::as_str).unwrap_or(""))
                == agent_id
        });
        if exists {
            continue;
        }
        let archived_at = archived
            .get(&raw_id)
            .and_then(|row| row.get("archived_at").and_then(Value::as_str))
            .map(|v| clean_text(v, 80))
            .filter(|v| !v.is_empty())
            .or_else(|| {
                profiles
                    .get(&raw_id)
                    .and_then(|row| row.get("updated_at").and_then(Value::as_str))
                    .map(|v| clean_text(v, 80))
                    .filter(|v| !v.is_empty())
            })
            .unwrap_or_else(now_iso);
        entries.push(json!({
            "agent_id": agent_id,
            "contract_id": "",
            "termination_reason": "archived",
            "terminated_at": archived_at
        }));
    }
    entries = entries
        .into_iter()
        .map(|mut row| {
            let agent_id =
                normalize_agent_id(row.get("agent_id").and_then(Value::as_str).unwrap_or(""));
            let role = profiles
                .get(&agent_id)
                .and_then(|profile| profile.get("role").and_then(Value::as_str))
                .map(|v| clean_text(v, 60))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "analyst".to_string());
            row["role"] = Value::String(role);
            row
        })
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| {
        clean_text(
            b.get("terminated_at").and_then(Value::as_str).unwrap_or(""),
            80,
        )
        .cmp(&clean_text(
            a.get("terminated_at").and_then(Value::as_str).unwrap_or(""),
            80,
        ))
    });
    json!({"ok": true, "type": "dashboard_agent_terminated_entries", "entries": entries})
}

pub fn delete_terminated(root: &Path, agent_id: &str, contract_id: Option<&str>) -> Value {
    let id = normalize_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let cid = contract_id
        .map(|v| clean_text(v, 120))
        .filter(|v| !v.is_empty());

    let mut state = load_contracts_state(root);
    let removed_history_entries = {
        let history = as_array_mut(&mut state, "terminated_history");
        let before = history.len();
        history.retain(|row| {
            let row_agent =
                normalize_agent_id(row.get("agent_id").and_then(Value::as_str).unwrap_or(""));
            let row_contract = clean_text(
                row.get("contract_id").and_then(Value::as_str).unwrap_or(""),
                120,
            );
            if row_agent != id {
                return true;
            }
            if let Some(target_cid) = &cid {
                row_contract != *target_cid
            } else {
                false
            }
        });
        before.saturating_sub(history.len())
    };
    let removed_contract = {
        let contracts = as_object_mut(&mut state, "contracts");
        if let Some(contract) = contracts.get(&id) {
            let status = contract
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("active");
            let contract_match = cid.as_ref().map(|target| {
                clean_text(
                    contract
                        .get("contract_id")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    120,
                ) == *target
            });
            let should_remove = status == "terminated" && contract_match.unwrap_or(true);
            if should_remove {
                contracts.remove(&id).is_some()
            } else {
                false
            }
        } else {
            false
        }
    };
    save_contracts_state(root, state);
    let purged = purge_agent_artifacts(root, &id);
    json!({
        "ok": true,
        "type": "dashboard_agent_terminated_delete",
        "agent_id": id,
        "removed_history_entries": removed_history_entries,
        "removed_contract": removed_contract,
        "removed_profile": purged.get("removed_profile").cloned().unwrap_or(Value::Bool(false)),
        "removed_archived": purged.get("removed_archived").cloned().unwrap_or(Value::Bool(false)),
        "removed_session_file": purged.get("removed_session_file").cloned().unwrap_or(Value::Bool(false)),
        "git_cleanup": purged.get("git_cleanup").cloned().unwrap_or_else(|| json!({"ok": false, "error": "cleanup_missing"}))
    })
}
