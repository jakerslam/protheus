fn build_sidebar_agent_roster_fast(root: &Path, snapshot: &Value, include_terminated: bool) -> Vec<Value> {
    let archived = std::collections::HashSet::<String>::new();
    let profiles = Map::<String, Value>::new();
    let contracts = Map::<String, Value>::new();
    let collab = collab_agents_map(snapshot);
    let (default_provider, default_model) = extract_app_settings(root, snapshot);
    let mut all_ids = std::collections::HashSet::<String>::new();
    for key in profiles.keys() {
        let id = clean_agent_id(key);
        if !id.is_empty() {
            all_ids.insert(id);
        }
    }
    for key in contracts.keys() {
        let id = clean_agent_id(key);
        if !id.is_empty() {
            all_ids.insert(id);
        }
    }
    for (key, row) in &collab {
        let id = clean_agent_id(key);
        if id.is_empty() {
            continue;
        }
        if profiles.contains_key(&id)
            || contracts.contains_key(&id)
            || collab_runtime_active(Some(row))
        {
            all_ids.insert(id);
        }
    }

    let mut rows = Vec::<Value>::new();
    for agent_id in all_ids {
        if archived.contains(&agent_id) {
            continue;
        }
        let profile = profiles
            .get(&agent_id)
            .cloned()
            .unwrap_or_else(|| json!({}));
        let contract_raw = contracts
            .get(&agent_id)
            .cloned()
            .unwrap_or_else(|| json!({}));
        let collab_row = collab.get(&agent_id);
        let runtime_active = collab_runtime_active(collab_row);
        let contract = contract_with_runtime_fields(&contract_raw);
        let contract_status = clean_text(
            contract
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("active"),
            40,
        )
        .to_ascii_lowercase();
        let contract_terminated = contract_status == "terminated" && !runtime_active;
        let termination_reason = clean_text(
            contract
                .get("termination_reason")
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        )
        .to_ascii_lowercase();
        let revive_recommended =
            contract_terminated && (termination_reason.contains("timeout") || termination_reason.contains("expired"));
        if !include_terminated && contract_terminated && !revive_recommended {
            continue;
        }

        let profile_name = clean_text(
            profile.get("name").and_then(Value::as_str).unwrap_or(""),
            120,
        );
        let name = if profile_name.is_empty() {
            humanize_agent_name(&agent_id)
        } else {
            profile_name
        };
        let role = {
            let from_profile = clean_text(
                profile.get("role").and_then(Value::as_str).unwrap_or(""),
                60,
            );
            if !from_profile.is_empty() {
                from_profile
            } else {
                let from_collab = first_string(collab_row, "role");
                if from_collab.is_empty() {
                    "analyst".to_string()
                } else {
                    from_collab
                }
            }
        };
        let state = if contract_terminated {
            if revive_recommended {
                "Idle".to_string()
            } else {
                "Terminated".to_string()
            }
        } else if runtime_active {
            "Running".to_string()
        } else {
            let raw = first_string(collab_row, "status");
            if raw.is_empty() {
                "Running".to_string()
            } else if raw.eq_ignore_ascii_case("active") || raw.eq_ignore_ascii_case("running") {
                "Running".to_string()
            } else if raw.eq_ignore_ascii_case("idle") {
                "Idle".to_string()
            } else {
                raw
            }
        };
        let identity = if profile
            .get("identity")
            .map(Value::is_object)
            .unwrap_or(false)
        {
            profile
                .get("identity")
                .cloned()
                .unwrap_or_else(|| json!({}))
        } else {
            json!({
                "emoji": profile.get("emoji").cloned().unwrap_or_else(|| json!("")),
                "color": profile.get("color").cloned().unwrap_or_else(|| json!("#2563EB")),
                "archetype": profile.get("archetype").cloned().unwrap_or_else(|| json!("assistant")),
                "vibe": profile.get("vibe").cloned().unwrap_or_else(|| json!(""))
            })
        };
        let model_override = clean_text(
            profile
                .get("model_override")
                .and_then(Value::as_str)
                .unwrap_or(""),
            160,
        );
        let model_ref =
            if !model_override.is_empty() && !model_override.eq_ignore_ascii_case("auto") {
                model_override
            } else {
                default_model.clone()
            };
        let (model_provider, model_name) =
            split_model_ref(&model_ref, &default_provider, &default_model);
        let runtime_model = clean_text(
            profile
                .get("runtime_model")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let model_runtime = if runtime_model.is_empty() {
            model_name.clone()
        } else {
            runtime_model
        };
        let git_branch = clean_text(
            profile
                .get("git_branch")
                .and_then(Value::as_str)
                .unwrap_or("main"),
            180,
        );
        let git_tree_kind = clean_text(
            profile
                .get("git_tree_kind")
                .and_then(Value::as_str)
                .unwrap_or("master"),
            60,
        );
        let is_master = profile
            .get("is_master_agent")
            .and_then(Value::as_bool)
            .unwrap_or_else(|| {
                let branch = git_branch.to_ascii_lowercase();
                let kind = git_tree_kind.to_ascii_lowercase();
                branch == "main" || branch == "master" || kind == "master" || kind == "main"
            });
        let auto_terminate_allowed = contract
            .get("auto_terminate_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(!is_master);
        let contract_total_ms = if auto_terminate_allowed {
            contract
                .get("expiry_seconds")
                .and_then(Value::as_i64)
                .map(|seconds| seconds.clamp(1, 31 * 24 * 60 * 60).saturating_mul(1000))
        } else {
            None
        };
        let contract_remaining_ms = if auto_terminate_allowed {
            contract.get("remaining_ms").and_then(Value::as_i64)
        } else {
            None
        };
        let created_at = clean_text(
            profile
                .get("created_at")
                .and_then(Value::as_str)
                .or_else(|| contract.get("created_at").and_then(Value::as_str))
                .unwrap_or(""),
            80,
        );
        let updated_at = latest_timestamp(&[
            clean_text(
                profile
                    .get("updated_at")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                80,
            ),
            clean_text(
                contract
                    .get("updated_at")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                80,
            ),
            clean_text(
                collab_row
                    .and_then(|v| v.get("activated_at").and_then(Value::as_str))
                    .unwrap_or(""),
                80,
            ),
        ]);
        rows.push(json!({
            "id": agent_id,
            "agent_id": agent_id,
            "name": name,
            "role": role,
            "state": state,
            "model_provider": model_provider,
            "model_provider_aliases": match model_provider.as_str() { "gemini" => json!(["google"]), "grok" => json!(["xai"]), "kimi" => json!(["moonshot"]), _ => json!([]) },
            "model_name": model_name,
            "runtime_model": model_runtime,
            "context_window": profile.get("context_window").cloned().unwrap_or(Value::Null),
            "context_window_tokens": profile.get("context_window_tokens").cloned().unwrap_or(Value::Null),
            "identity": identity,
            "avatar_url": profile.get("avatar_url").cloned().unwrap_or_else(|| json!("")),
            "system_prompt": profile.get("system_prompt").cloned().unwrap_or_else(|| json!("")),
            "fallback_models": profile.get("fallback_models").cloned().unwrap_or_else(|| json!([])),
            "git_branch": git_branch,
            "branch": git_branch,
            "git_tree_kind": git_tree_kind,
            "workspace_rel": profile.get("workspace_rel").cloned().unwrap_or(Value::Null),
            "git_tree_ready": profile.get("git_tree_ready").cloned().unwrap_or_else(|| json!(true)),
            "git_tree_error": profile.get("git_tree_error").cloned().unwrap_or_else(|| json!("")),
            "is_master_agent": is_master,
            "created_at": created_at,
            "updated_at": updated_at,
            "message_count": 0,
            "contract": contract.clone(),
            "contract_expires_at": contract.get("expires_at").cloned().unwrap_or(Value::Null),
            "contract_total_ms": contract_total_ms.map(Value::from).unwrap_or(Value::Null),
            "contract_remaining_ms": contract_remaining_ms.map(Value::from).unwrap_or(Value::Null),
            "contract_finite_expiry": auto_terminate_allowed && (contract_remaining_ms.is_some() || contract_total_ms.is_some()),
            "parent_agent_id": parent_agent_id_from_row(&json!({
                "parent_agent_id": profile.get("parent_agent_id").cloned().unwrap_or(Value::Null),
                "contract": {"parent_agent_id": contract.get("parent_agent_id").cloned().unwrap_or(Value::Null)}
            })),
            "auto_terminate_allowed": auto_terminate_allowed,
            "revive_recommended": revive_recommended
        }));
    }
    rows.sort_by(|left, right| {
        let left_updated = clean_text(
            left.get("updated_at").and_then(Value::as_str).unwrap_or(""),
            80,
        );
        let right_updated = clean_text(
            right
                .get("updated_at")
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        );
        right_updated.cmp(&left_updated).then_with(|| {
            clean_agent_id(left.get("id").and_then(Value::as_str).unwrap_or("")).cmp(
                &clean_agent_id(right.get("id").and_then(Value::as_str).unwrap_or("")),
            )
        })
    });
    rows
}
