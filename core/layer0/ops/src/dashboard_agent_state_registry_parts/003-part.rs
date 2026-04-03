pub fn delete_all_terminated(root: &Path) -> Value {
    let mut state = load_contracts_state(root);
    let mut ids = HashSet::<String>::new();
    {
        let history = as_array_mut(&mut state, "terminated_history");
        for row in history.iter() {
            let id = normalize_agent_id(row.get("agent_id").and_then(Value::as_str).unwrap_or(""));
            if !id.is_empty() {
                ids.insert(id);
            }
        }
    }
    {
        let contracts = as_object_mut(&mut state, "contracts");
        let terminated_ids = contracts
            .iter()
            .filter_map(|(id, row)| {
                row.get("status")
                    .and_then(Value::as_str)
                    .filter(|v| *v == "terminated")
                    .map(|_| id.clone())
            })
            .collect::<Vec<_>>();
        for id in &terminated_ids {
            contracts.remove(id);
            ids.insert(id.clone());
        }
    }
    let removed_history_entries = {
        let history = as_array_mut(&mut state, "terminated_history");
        let count = history.len();
        history.clear();
        count
    };
    save_contracts_state(root, state);

    let archived_all = load_archived_state(root)
        .get("agents")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for id in archived_all.keys() {
        let normalized = normalize_agent_id(id);
        if !normalized.is_empty() {
            ids.insert(normalized);
        }
    }

    let mut deleted_archived_agents = 0usize;
    for id in ids {
        let purged = purge_agent_artifacts(root, &id);
        if purged
            .get("removed_archived")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            deleted_archived_agents += 1;
        }
    }
    json!({
        "ok": true,
        "type": "dashboard_agent_terminated_delete_all",
        "removed_history_entries": removed_history_entries,
        "deleted_archived_agents": deleted_archived_agents
    })
}

pub fn revive_agent(root: &Path, agent_id: &str, role: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let normalized_role = clean_text(role, 60);
    let role_value = if normalized_role.is_empty() {
        "analyst".to_string()
    } else {
        normalized_role
    };
    let profile = upsert_profile(
        root,
        &id,
        &json!({
            "role": role_value,
            "state": "Running"
        }),
    );
    let _ = unarchive_agent(root, &id);

    let mut state = load_contracts_state(root);
    let mut revived_from_contract_id = state
        .get("contracts")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&id))
        .and_then(|row| row.get("contract_id").and_then(Value::as_str))
        .map(|v| clean_text(v, 120))
        .unwrap_or_default();
    {
        let history = as_array_mut(&mut state, "terminated_history");
        if revived_from_contract_id.is_empty() {
            for row in history.iter().rev() {
                if normalize_agent_id(row.get("agent_id").and_then(Value::as_str).unwrap_or(""))
                    == id
                {
                    revived_from_contract_id = clean_text(
                        row.get("contract_id").and_then(Value::as_str).unwrap_or(""),
                        120,
                    );
                    if !revived_from_contract_id.is_empty() {
                        break;
                    }
                }
            }
        }
        history.retain(|row| {
            normalize_agent_id(row.get("agent_id").and_then(Value::as_str).unwrap_or("")) != id
        });
    }
    save_contracts_state(root, state);

    let mut contract_patch = json!({
        "status": "active",
        "created_at": now_iso(),
        "termination_reason": "",
        "expiry_seconds": DEFAULT_EXPIRY_SECONDS
    });
    if !revived_from_contract_id.is_empty() {
        contract_patch["revived_from_contract_id"] = json!(revived_from_contract_id);
    }
    let contract = upsert_contract(root, &id, &contract_patch);
    json!({
        "ok": true,
        "type": "dashboard_agent_revive",
        "agent_id": id,
        "profile": profile.get("profile").cloned().unwrap_or_else(|| json!({})),
        "contract": contract.get("contract").cloned().unwrap_or_else(|| json!({}))
    })
}

#[cfg(test)]
#[path = "../dashboard_agent_state_registry_tests.rs"]
mod tests;
