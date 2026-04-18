fn build_agent_roster(root: &Path, snapshot: &Value, include_terminated: bool) -> Vec<Value> {
    let mut archived = crate::dashboard_agent_state::archived_agent_ids(root);
    let profiles = profiles_map(root);
    let contracts = contracts_map(root);
    let collab = collab_agents_map(snapshot);
    let session_summaries = session_summary_map(root, snapshot);
    let (default_provider, default_model) = effective_app_settings(root, snapshot);
    for (raw_id, profile) in &profiles {
        let profile_state = clean_text(
            profile.get("state").and_then(Value::as_str).unwrap_or(""),
            40,
        )
        .to_ascii_lowercase();
        if profile_state == "archived" {
            let id = clean_agent_id(raw_id);
            if !id.is_empty() {
                archived.insert(id);
            }
        }
    }
    let mut all_ids = HashSet::<String>::new();
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
    for (key, row) in &session_summaries {
        let id = clean_agent_id(key);
        if id.is_empty() {
            continue;
        }
        let has_session_activity = row
            .get("message_count")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            > 0
            || !clean_text(
                row.get("updated_at").and_then(Value::as_str).unwrap_or(""),
                80,
            )
            .is_empty();
        if profiles.contains_key(&id) || contracts.contains_key(&id) || has_session_activity {
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
        let session_summary = session_summaries.get(&agent_id);
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
        let termination_condition = clean_text(
            contract
                .get("termination_condition")
                .and_then(Value::as_str)
                .unwrap_or("task_or_timeout"),
            80,
        )
        .to_ascii_lowercase();
        let termination_reason = clean_text(
            contract
                .get("termination_reason")
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        )
        .to_ascii_lowercase();
        let revive_recommended = contract_terminated
            && ((termination_reason.contains("timeout")
                || termination_reason.contains("expired"))
                || ((termination_condition.starts_with("manual")
                    || termination_condition == "task_complete"
                    || (!contract
                        .get("auto_terminate_allowed")
                        .and_then(Value::as_bool)
                        .unwrap_or(true)
                        && !contract
                            .get("idle_terminate_allowed")
                            .and_then(Value::as_bool)
                            .unwrap_or(true)))
                    && termination_reason.contains("terminated")));
        let timeout_terminated = contract_terminated && (termination_reason.contains("timeout") || termination_reason.contains("expired"));
        if !include_terminated && (timeout_terminated || (contract_terminated && !revive_recommended)) { continue; }
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
                if !from_collab.is_empty() {
                    from_collab
                } else {
                    "analyst".to_string()
                }
            }
        };
        let session_updated_at = clean_text(
            session_summary
                .and_then(|row| row.get("updated_at").and_then(Value::as_str))
                .unwrap_or(""),
            80,
        );
        let session_message_count = session_summary
            .and_then(|row| row.get("message_count").and_then(Value::as_i64))
            .unwrap_or(0);
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
                if session_message_count > 0 || !session_updated_at.is_empty() {
                    "Idle".to_string()
                } else {
                    "Running".to_string()
                }
            } else if raw.eq_ignore_ascii_case("active") || raw.eq_ignore_ascii_case("running") {
                "Running".to_string()
            } else if raw.eq_ignore_ascii_case("idle") {
                "Idle".to_string()
            } else if raw.eq_ignore_ascii_case("inactive") || raw.eq_ignore_ascii_case("paused") {
                let profile_state = clean_text(
                    profile.get("state").and_then(Value::as_str).unwrap_or(""),
                    40,
                )
                .to_ascii_lowercase();
                if profile_state == "running"
                    || profile_state == "active"
                    || contract_status == "active"
                {
                    "Idle".to_string()
                } else {
                    "Inactive".to_string()
                }
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
                "emoji": profile.get("emoji").cloned().unwrap_or_else(|| json!("🧑‍💻")),
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
        let contract_total_ms = if auto_terminate_allowed { contract.get("expiry_seconds").and_then(Value::as_i64).map(|seconds| seconds.clamp(1, 31 * 24 * 60 * 60).saturating_mul(1000)) } else { None };
        let contract_remaining_ms = if auto_terminate_allowed {
            contract.get("remaining_ms").and_then(Value::as_i64)
        } else {
            None
        };
        let contract_finite_expiry =
            auto_terminate_allowed && (contract_remaining_ms.is_some() || contract_total_ms.is_some());
        let created_at = clean_text(
            profile
                .get("created_at")
                .and_then(Value::as_str)
                .or_else(|| contract.get("created_at").and_then(Value::as_str))
                .or_else(|| {
                    session_summary.and_then(|row| row.get("updated_at").and_then(Value::as_str))
                })
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
            session_updated_at.clone(),
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
            "workspace_dir": profile
                .get("workspace_dir")
                .cloned()
                .unwrap_or_else(|| json!(root.to_string_lossy().to_string())),
            "workspace_rel": profile.get("workspace_rel").cloned().unwrap_or(Value::Null),
            "git_tree_ready": profile.get("git_tree_ready").cloned().unwrap_or_else(|| json!(true)),
            "git_tree_error": profile.get("git_tree_error").cloned().unwrap_or_else(|| json!("")),
            "is_master_agent": is_master,
            "created_at": created_at,
            "updated_at": updated_at,
            "message_count": session_message_count,
            "contract": contract.clone(),
            "contract_expires_at": contract.get("expires_at").cloned().unwrap_or(Value::Null),
            "contract_total_ms": contract_total_ms.map(Value::from).unwrap_or(Value::Null),
            "contract_remaining_ms": contract_remaining_ms.map(Value::from).unwrap_or(Value::Null),
            "contract_finite_expiry": contract_finite_expiry,
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
fn archive_all_visible_agents(root: &Path, snapshot: &Value, reason: &str) -> Value {
    let archive_reason = {
        let cleaned = clean_text(reason, 120);
        if cleaned.is_empty() {
            "user_archive_all".to_string()
        } else {
            cleaned
        }
    };
    let mut archived_agent_ids = Vec::<String>::new();
    let mut failed_agent_ids = Vec::<String>::new();
    let mut skipped_agent_ids = Vec::<String>::new();
    for row in build_agent_roster(root, snapshot, false) {
        let agent_id = clean_agent_id(row.get("id").and_then(Value::as_str).unwrap_or(""));
        if agent_id.is_empty() {
            continue;
        }
        if agent_id.eq_ignore_ascii_case("system") {
            skipped_agent_ids.push(agent_id);
            continue;
        }
        let _ = update_profile_patch(
            root,
            &agent_id,
            &json!({"state": "Archived", "updated_at": crate::now_iso()}),
        );
        let _ = upsert_contract_patch(
            root,
            &agent_id,
            &json!({
                "status": "terminated",
                "termination_reason": "user_archived",
                "terminated_at": crate::now_iso(),
                "updated_at": crate::now_iso()
            }),
        );
        let archived =
            crate::dashboard_agent_state::archive_agent(root, &agent_id, &archive_reason);
        if archived.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            archived_agent_ids.push(agent_id);
        } else {
            failed_agent_ids.push(agent_id);
        }
    }
    let attempted = archived_agent_ids.len() + failed_agent_ids.len();
    json!({
        "ok": failed_agent_ids.is_empty(),
        "type": "dashboard_agent_archive_all",
        "reason": archive_reason,
        "attempted": attempted,
        "archived_count": archived_agent_ids.len(),
        "archived_agent_ids": archived_agent_ids,
        "failed_agent_ids": failed_agent_ids,
        "skipped_agent_ids": skipped_agent_ids
    })
}

fn agent_row_by_id(root: &Path, snapshot: &Value, agent_id: &str) -> Option<Value> {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return None;
    }
    build_agent_roster(root, snapshot, true)
        .into_iter()
        .find(|row| clean_agent_id(row.get("id").and_then(Value::as_str).unwrap_or("")) == id)
}
fn archived_agent_stub(root: &Path, agent_id: &str) -> Value {
    let id = clean_agent_id(agent_id);
    let profile = profiles_map(root)
        .get(&id)
        .cloned()
        .unwrap_or_else(|| json!({}));
    let name = clean_text(
        profile.get("name").and_then(Value::as_str).unwrap_or(""),
        120,
    );
    let role = clean_text(
        profile
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("analyst"),
        60,
    );
    let role_value = if role.is_empty() {
        "analyst".to_string()
    } else {
        role
    };
    json!({
        "ok": true,
        "id": id,
        "agent_id": id,
        "name": if name.is_empty() { humanize_agent_name(agent_id) } else { name },
        "role": role_value,
        "state": "inactive",
        "archived": true
    })
}
fn update_profile_patch(root: &Path, agent_id: &str, patch: &Value) -> Value {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    crate::dashboard_agent_state::upsert_profile(root, &id, patch)
}

fn upsert_contract_patch(root: &Path, agent_id: &str, patch: &Value) -> Value {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    crate::dashboard_agent_state::upsert_contract(root, &id, patch)
}
fn session_payload(root: &Path, agent_id: &str) -> Value {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let state = load_session_state(root, &id);
    let messages = session_messages(&state);
    let sessions = session_rows_payload(&state);
    json!({
        "ok": true,
        "agent_id": id,
        "active_session_id": state.get("active_session_id").cloned().unwrap_or_else(|| json!("default")),
        "messages": messages,
        "sessions": sessions,
        "session": state
    })
}
fn append_jsonl_row(path: &Path, row: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(line) = serde_json::to_string(row) {
        let _ = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| {
                std::io::Write::write_all(&mut file, format!("{line}\n").as_bytes())
            });
    }
}
fn attention_queue_fallback_path(root: &Path) -> PathBuf {
    root.join("client/runtime/local/state/attention/pending_memory_events.jsonl")
}
