
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
            let termination_condition = clean_text(
                contract
                    .get("termination_condition")
                    .and_then(Value::as_str)
                    .unwrap_or("task_or_timeout"),
                80,
            )
            .to_ascii_lowercase();
            let lifespan = clean_text(
                contract
                    .get("lifespan")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
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
            let termination_reason = if !non_expiring && auto_terminate_allowed && now >= expiry_ts
            {
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
