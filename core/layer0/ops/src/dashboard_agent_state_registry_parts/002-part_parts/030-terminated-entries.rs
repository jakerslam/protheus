
pub fn terminated_entries(root: &Path) -> Value {
    fn archive_reason_to_termination_reason(reason: &str) -> String {
        let cleaned = clean_text(reason, 120);
        if cleaned.is_empty() {
            return "archived".to_string();
        }
        let normalized = cleaned.to_ascii_lowercase();
        if normalized == "archived by parent agent"
            || normalized == "parent_archived"
            || normalized == "parent_archive"
        {
            return "parent_archived".to_string();
        }
        if normalized == "user_archive" || normalized == "user_archived" {
            return "user_archived".to_string();
        }
        cleaned
            .replace(' ', "_")
            .replace('-', "_")
            .to_ascii_lowercase()
            .trim_matches('_')
            .to_string()
    }

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
        let state = clean_text(
            profile.get("state").and_then(Value::as_str).unwrap_or(""),
            40,
        )
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
        let archive_reason = archived
            .get(&raw_id)
            .and_then(|row| row.get("reason").and_then(Value::as_str))
            .map(|v| clean_text(v, 120))
            .unwrap_or_default();
        let termination_reason = archive_reason_to_termination_reason(&archive_reason);
        entries.push(json!({
            "agent_id": agent_id,
            "contract_id": "",
            "termination_reason": termination_reason,
            "reason": termination_reason,
            "archive_reason": archive_reason,
            "terminated_at": archived_at
        }));
    }
    entries = entries
        .into_iter()
        .map(|mut row| {
            let agent_id =
                normalize_agent_id(row.get("agent_id").and_then(Value::as_str).unwrap_or(""));
            let termination_reason = clean_text(
                row.get("termination_reason")
                    .and_then(Value::as_str)
                    .or_else(|| row.get("reason").and_then(Value::as_str))
                    .unwrap_or("terminated"),
                120,
            );
            row["termination_reason"] = Value::String(termination_reason.clone());
            row["reason"] = Value::String(termination_reason);
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
