fn handle_agent_scope_suggestions_command_config_routes(
    root: &Path,
    method: &str,
    segments: &[String],
    body: &[u8],
    snapshot: &Value,
    agent_id: &str,
    existing: &Option<Value>,
) -> Option<CompatApiResponse> {
    if method == "POST" && segments.len() == 1 && segments[0] == "suggestions" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let hint = clean_text(
            request
                .get("user_hint")
                .and_then(Value::as_str)
                .or_else(|| request.get("hint").and_then(Value::as_str))
                .unwrap_or(""),
            220,
        );
        return Some(CompatApiResponse {
            status: 200,
            payload: crate::dashboard_agent_state::suggestions(root, &agent_id, &hint),
        });
    }

    if method == "POST" && segments.len() == 1 && segments[0] == "command" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let command = clean_text(
            request.get("command").and_then(Value::as_str).unwrap_or(""),
            80,
        )
        .to_ascii_lowercase();
        let silent = request
            .get("silent")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if command == "context" {
            let row = existing.clone().unwrap_or_else(|| json!({}));
            return Some(CompatApiResponse {
                status: 200,
                payload: context_command_payload(root, &agent_id, &row, &request, silent),
            });
        }
        if command == "queue" {
            let runtime = runtime_sync_summary(snapshot);
            let queue_depth = parse_non_negative_i64(runtime.get("queue_depth"), 0);
            let conduit_signals = parse_non_negative_i64(runtime.get("conduit_signals"), 0);
            let backpressure_level = clean_text(
                runtime
                    .get("backpressure_level")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
                40,
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "agent_id": agent_id,
                    "command": command,
                    "silent": silent,
                    "runtime_sync": runtime,
                    "message": format!(
                        "Queue depth: {} | Conduit signals: {} | Backpressure: {}",
                        queue_depth,
                        conduit_signals,
                        backpressure_level
                    )
                }),
            });
        }
        if command == "cron" || command == "schedule" {
            let args = clean_text(
                request
                    .get("args")
                    .and_then(Value::as_str)
                    .or_else(|| request.get("input").and_then(Value::as_str))
                    .or_else(|| request.get("query").and_then(Value::as_str))
                    .unwrap_or(""),
                1_200,
            );
            let Some((tool_name, tool_input)) = cron_tool_request_from_args(&args) else {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({
                        "ok": false,
                        "agent_id": agent_id,
                        "command": command,
                        "silent": silent,
                        "error": "cron_usage_required",
                        "usage": "/cron list | /cron schedule <interval> <message> | /cron run <job_id> | /cron cancel <job_id>"
                    }),
                });
            };
            let row = existing.clone().unwrap_or_else(|| json!({}));
            let tool_payload = execute_tool_call_with_recovery(
                root,
                snapshot,
                &agent_id,
                Some(&row),
                &tool_name,
                &tool_input,
            );
            let ok = tool_payload
                .get("ok")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let tool_summary = summarize_tool_payload(&tool_name, &tool_payload);
            let response_tools = vec![json!({
                "id": format!("tool-command-{}", normalize_tool_name(&tool_name)),
                "name": normalize_tool_name(&tool_name),
                "input": trim_text(&tool_input.to_string(), 4000),
                "result": trim_text(&tool_summary, 24_000),
                "is_error": !ok
            })];
            let (message, tool_completion) =
                enforce_tool_completion_contract(tool_summary, &response_tools);
            let tool_completion = enrich_tool_completion_receipt(tool_completion, &response_tools);
            return Some(CompatApiResponse {
                status: if ok { 200 } else { 400 },
                payload: json!({
                    "ok": ok,
                    "agent_id": agent_id,
                    "command": command,
                    "silent": silent,
                    "tool": tool_name,
                    "input": tool_input,
                    "message": if message.trim().is_empty() {
                        format!("Cron command '{}' processed.", command)
                    } else {
                        message
                    },
                    "response_finalization": {
                        "tool_completion": tool_completion
                    },
                    "tools": response_tools,
                    "result": tool_payload
                }),
            });
        }
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({
                "ok": true,
                "agent_id": agent_id,
                "command": if command.is_empty() { "unknown" } else { &command },
                "silent": silent,
                "message": format!("Command '{}' acknowledged.", if command.is_empty() { "unknown" } else { &command })
            }),
        });
    }

    if method == "PATCH" && segments.len() == 1 && segments[0] == "config" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let mut patch = request.clone();
        if !patch.is_object() {
            patch = json!({});
        }
        let should_seed_intro = patch.get("contract").is_some()
            || patch.get("system_prompt").is_some()
            || patch.get("archetype").is_some()
            || patch.get("profile").is_some();
        let explicit_role = clean_text(patch.get("role").and_then(Value::as_str).unwrap_or(""), 60);
        let existing_role = clean_text(
            existing
                .as_ref()
                .and_then(|row| row.get("role").and_then(Value::as_str))
                .unwrap_or(""),
            60,
        );
        let archetype_hint = clean_text(
            patch.get("archetype").and_then(Value::as_str).unwrap_or(""),
            80,
        )
        .to_ascii_lowercase();
        let profile_hint = clean_text(
            patch.get("profile").and_then(Value::as_str).unwrap_or(""),
            80,
        )
        .to_ascii_lowercase();
        let mut role_hint = format!("{archetype_hint} {profile_hint}");
        if role_hint.trim().is_empty() {
            role_hint = clean_text(
                patch
                    .get("system_prompt")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                200,
            )
            .to_ascii_lowercase();
        }
        let inferred_role = if !explicit_role.is_empty() {
            explicit_role.clone()
        } else if role_hint.contains("teacher")
            || role_hint.contains("tutor")
            || role_hint.contains("mentor")
            || role_hint.contains("coach")
            || role_hint.contains("instructor")
        {
            "tutor".to_string()
        } else if role_hint.contains("code")
            || role_hint.contains("coder")
            || role_hint.contains("engineer")
            || role_hint.contains("developer")
            || role_hint.contains("devops")
            || role_hint.contains("api")
            || role_hint.contains("build")
        {
            "engineer".to_string()
        } else if role_hint.contains("research") || role_hint.contains("investig") {
            "researcher".to_string()
        } else if role_hint.contains("analyst")
            || role_hint.contains("analysis")
            || role_hint.contains("data")
        {
            "analyst".to_string()
        } else if role_hint.contains("writer")
            || role_hint.contains("editor")
            || role_hint.contains("content")
        {
            "writer".to_string()
        } else if role_hint.contains("design")
            || role_hint.contains("ui")
            || role_hint.contains("ux")
        {
            "designer".to_string()
        } else if role_hint.contains("support") {
            "support".to_string()
        } else if !existing_role.is_empty() {
            existing_role.clone()
        } else {
            "analyst".to_string()
        };
        let resolved_role = if inferred_role.is_empty() {
            "analyst".to_string()
        } else {
            inferred_role
        };
        if should_seed_intro
            && explicit_role.is_empty()
            && !resolved_role.eq_ignore_ascii_case(&existing_role)
        {
            patch["role"] = Value::String(resolved_role.clone());
        }
        let mut rename_notice: Option<Value> = None;
        if patch.get("name").is_some() {
            let requested_name =
                clean_text(patch.get("name").and_then(Value::as_str).unwrap_or(""), 120);
            if requested_name.is_empty() {
                if let Some(map) = patch.as_object_mut() {
                    map.remove("name");
                }
            } else {
                let requested_default_like =
                    dashboard_compat_api_agent_identity::is_default_agent_name_for_agent(
                        &requested_name,
                        &agent_id,
                    );
                let resolved_name = dashboard_compat_api_agent_identity::resolve_agent_name(
                    root,
                    &requested_name,
                    &resolved_role,
                );
                let treat_as_blank_for_init = should_seed_intro
                    && (requested_default_like
                        || dashboard_compat_api_agent_identity::is_default_agent_name_for_agent(
                            &resolved_name,
                            &agent_id,
                        ));
                if treat_as_blank_for_init {
                    if let Some(map) = patch.as_object_mut() {
                        map.remove("name");
                    }
                } else {
                    patch["name"] = Value::String(resolved_name);
                }
            }
        }
        if should_seed_intro && patch.get("name").is_none() {
            let selected_provider_hint = clean_text(
                patch
                    .get("model_provider")
                    .or_else(|| patch.get("provider"))
                    .and_then(Value::as_str)
                    .or_else(|| {
                        existing
                            .as_ref()
                            .and_then(|row| row.get("model_provider").and_then(Value::as_str))
                    })
                    .unwrap_or("auto"),
                80,
            );
            let selected_model_hint = clean_text(
                patch
                    .get("model_override")
                    .or_else(|| patch.get("model_name"))
                    .or_else(|| patch.get("runtime_model"))
                    .or_else(|| patch.get("model"))
                    .and_then(Value::as_str)
                    .or_else(|| {
                        existing.as_ref().and_then(|row| {
                            row.get("model_override")
                                .or_else(|| row.get("model_name"))
                                .or_else(|| row.get("runtime_model"))
                                .and_then(Value::as_str)
                        })
                    })
                    .unwrap_or(""),
                200,
            );
            let preserve_default_name_for_self_named_models = selected_model_supports_self_naming(
                root,
                snapshot,
                &selected_provider_hint,
                &selected_model_hint,
            );
            let existing_name = clean_text(
                existing
                    .as_ref()
                    .and_then(|row| row.get("name").and_then(Value::as_str))
                    .unwrap_or(""),
                120,
            );
            if dashboard_compat_api_agent_identity::is_default_agent_name_for_agent(
                &existing_name,
                &agent_id,
            ) && !preserve_default_name_for_self_named_models
            {
                let previous_name = if existing_name.is_empty() {
                    dashboard_compat_api_agent_identity::default_agent_name(&agent_id)
                } else {
                    existing_name.clone()
                };
                let auto_name = dashboard_compat_api_agent_identity::resolve_post_init_agent_name(
                    root,
                    &agent_id,
                    &resolved_role,
                );
                if !auto_name.is_empty() && !auto_name.eq_ignore_ascii_case(&previous_name) {
                    patch["name"] = Value::String(auto_name.clone());
                    rename_notice = Some(json!({
                        "notice_label": format!("changed name from {previous_name} to {auto_name}"),
                        "notice_type": "info",
                        "ts": crate::now_iso(),
                        "auto_generated": true
                    }));
                }
            }
        }
        let patch_touches_identity = patch.get("identity").is_some()
            || patch.get("emoji").is_some()
            || patch.get("color").is_some()
            || patch.get("archetype").is_some()
            || patch.get("vibe").is_some();
        if patch_touches_identity {
            if !patch.get("identity").map(Value::is_object).unwrap_or(false) {
                let emoji =
                    clean_text(patch.get("emoji").and_then(Value::as_str).unwrap_or(""), 16);
                let color =
                    clean_text(patch.get("color").and_then(Value::as_str).unwrap_or(""), 32);
                let archetype = clean_text(
                    patch.get("archetype").and_then(Value::as_str).unwrap_or(""),
                    80,
                );
                let vibe = clean_text(patch.get("vibe").and_then(Value::as_str).unwrap_or(""), 80);
                if !emoji.is_empty()
                    || !color.is_empty()
                    || !archetype.is_empty()
                    || !vibe.is_empty()
                {
                    patch["identity"] = json!({
                        "emoji": emoji,
                        "color": color,
                        "archetype": archetype,
                        "vibe": vibe
                    });
                }
            }
            let mut identity_request = existing.clone().unwrap_or_else(|| json!({}));
            if !identity_request.is_object() {
                identity_request = json!({});
            }
            if let Some(identity_patch) = patch.get("identity").and_then(Value::as_object) {
                let mut merged_identity = identity_request
                    .get("identity")
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default();
                for (key, value) in identity_patch {
                    if let Some(raw) = value.as_str() {
                        if clean_text(raw, 120).is_empty() {
                            continue;
                        }
                    }
                    merged_identity.insert(key.clone(), value.clone());
                }
                identity_request["identity"] = Value::Object(merged_identity);
            }
            for key in ["emoji", "color", "archetype", "vibe"] {
                if let Some(value) = patch.get(key) {
                    if let Some(raw) = value.as_str() {
                        if clean_text(raw, 120).is_empty() {
                            continue;
                        }
                    }
                    identity_request[key] = value.clone();
                }
            }
            patch["identity"] = dashboard_compat_api_agent_identity::resolve_agent_identity(
                root,
                &identity_request,
                &resolved_role,
            );
        }
        return Some(finalize_agent_scope_config_patch(
            root,
            snapshot,
            agent_id,
            existing,
            patch,
            should_seed_intro,
            resolved_role,
            rename_notice,
        ));
    }
    handle_agent_scope_model_mode_git_routes(root, method, segments, body, snapshot, agent_id)
}
