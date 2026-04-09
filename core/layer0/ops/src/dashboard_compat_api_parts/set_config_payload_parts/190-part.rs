// FILE_SIZE_EXCEPTION: reason=Function-scale route/tool block retained atomically during staged decomposition; owner=jay; expires=2026-04-22
pub fn handle_with_headers(
    root: &Path,
    method: &str,
    path: &str,
    body: &[u8],
    headers: &[(&str, &str)],
    snapshot: &Value,
) -> Option<CompatApiResponse> {
    let path_only = path.split('?').next().unwrap_or(path);
    let requester_agent = requester_agent_id(headers);
    let request_host = header_value(headers, "host").unwrap_or_default();
    if let Some(payload) =
        crate::dashboard_terminal_broker::handle_http(root, method, path_only, body)
    {
        return Some(CompatApiResponse {
            status: 200,
            payload,
        });
    }
    if let Some(response) = dashboard_compat_api_reference_gap_closure::handle(
        root, method, path, path_only, body, snapshot,
    ) {
        return Some(response);
    }
    if let Some(response) = dashboard_compat_api_reference_parity::handle(
        root, method, path, path_only, headers, body, snapshot,
    ) {
        return Some(response);
    }
    if let Some(response) = dashboard_compat_api_channels::handle(root, method, path_only, body) {
        return Some(compat_api_response_with_nexus(
            "dashboard_compat_api_channels",
            response,
        ));
    }
    if let Some(response) = dashboard_skills_marketplace::handle(root, method, path, snapshot, body)
    {
        return Some(response);
    }
    if let Some(response) =
        dashboard_compat_api_comms::handle(root, method, path, path_only, body, snapshot)
    {
        return Some(compat_api_response_with_nexus(
            "dashboard_compat_api_comms",
            response,
        ));
    }
    if let Some(response) =
        dashboard_compat_api_hands::handle(root, method, path_only, body, snapshot)
    {
        return Some(compat_api_response_with_nexus(
            "dashboard_compat_api_hands",
            response,
        ));
    }
    if let Some(response) =
        dashboard_compat_api_sidebar_ops::handle(root, method, path_only, body, snapshot)
    {
        return Some(compat_api_response_with_nexus(
            "dashboard_compat_api_sidebar_ops",
            response,
        ));
    }
    if let Some(response) = dashboard_compat_api_settings_ops::handle(root, method, path_only, body)
    {
        return Some(compat_api_response_with_nexus(
            "dashboard_compat_api_settings_ops",
            response,
        ));
    }

    if let Some((requested_agent_id, segments)) = parse_memory_route(path_only) {
        let agent_id = resolve_agent_id_alias(root, &requested_agent_id);
        if !requester_agent.is_empty()
            && requester_agent != agent_id
            && !actor_can_manage_target(root, snapshot, &requester_agent, &agent_id)
        {
            return Some(CompatApiResponse {
                status: 403,
                payload: json!({
                    "ok": false,
                    "error": "agent_manage_forbidden",
                    "actor_agent_id": requester_agent.clone(),
                    "target_agent_id": agent_id
                }),
            });
        }
        if segments.first().map(|v| v == "kv").unwrap_or(false) {
            if method == "GET" && segments.len() == 1 {
                let state = load_session_state(root, &agent_id);
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({
                        "ok": true,
                        "agent_id": agent_id,
                        "kv_pairs": memory_kv_pairs_from_state(&state)
                    }),
                });
            }
            if segments.len() >= 2 {
                let key = decode_path_segment(&segments[1..].join("/"));
                if method == "GET" {
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: crate::dashboard_agent_state::memory_kv_get(root, &agent_id, &key),
                    });
                }
                if method == "PUT" {
                    let request =
                        serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
                    let value = request.get("value").cloned().unwrap_or(Value::Null);
                    let payload =
                        crate::dashboard_agent_state::memory_kv_set(root, &agent_id, &key, &value);
                    return Some(CompatApiResponse {
                        status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                            200
                        } else {
                            400
                        },
                        payload,
                    });
                }
                if method == "DELETE" {
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: crate::dashboard_agent_state::memory_kv_delete(
                            root, &agent_id, &key,
                        ),
                    });
                }
            }
        }
        if segments
            .first()
            .map(|v| v == "semantic-query" || v == "semantic_query")
            .unwrap_or(false)
        {
            if method == "GET" || method == "POST" {
                let request = if method == "POST" {
                    serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}))
                } else {
                    json!({
                        "query": query_value(path, "q")
                            .or_else(|| query_value(path, "query"))
                            .unwrap_or_default(),
                        "limit": query_value(path, "limit")
                            .and_then(|raw| raw.parse::<usize>().ok())
                            .unwrap_or(8)
                    })
                };
                let query = clean_text(
                    request
                        .get("query")
                        .or_else(|| request.get("q"))
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    600,
                );
                let limit = request
                    .get("limit")
                    .and_then(Value::as_u64)
                    .map(|value| value as usize)
                    .unwrap_or(8)
                    .clamp(1, 25);
                let payload = crate::dashboard_agent_state::memory_kv_semantic_query(
                    root, &agent_id, &query, limit,
                );
                return Some(CompatApiResponse {
                    status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                        200
                    } else {
                        400
                    },
                    payload,
                });
            }
        }
    }

    if let Some((provider_id, segments)) = parse_provider_route(path_only) {
        if method == "GET" && segments.is_empty() && provider_id == "routing" {
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_provider_runtime::routing_policy_payload(root),
            });
        }
        if method == "POST" && segments.is_empty() && provider_id == "routing" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let payload = crate::dashboard_provider_runtime::update_routing_policy(root, &request);
            return Some(CompatApiResponse {
                status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    200
                } else {
                    400
                },
                payload,
            });
        }
        if method == "POST" && segments.len() == 1 && segments[0] == "key" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let key = clean_text(
                request.get("key").and_then(Value::as_str).unwrap_or(""),
                4096,
            );
            let payload =
                crate::dashboard_provider_runtime::save_provider_key(root, &provider_id, &key);
            return Some(CompatApiResponse {
                status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    200
                } else {
                    400
                },
                payload,
            });
        }
        if method == "DELETE" && segments.len() == 1 && segments[0] == "key" {
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_provider_runtime::remove_provider_key(root, &provider_id),
            });
        }
        if method == "POST" && segments.len() == 1 && segments[0] == "test" {
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_provider_runtime::test_provider(root, &provider_id),
            });
        }
        if method == "PUT" && segments.len() == 1 && segments[0] == "url" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let base_url = clean_text(
                request
                    .get("base_url")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                400,
            );
            let payload =
                crate::dashboard_provider_runtime::set_provider_url(root, &provider_id, &base_url);
            return Some(CompatApiResponse {
                status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    200
                } else {
                    400
                },
                payload,
            });
        }
    }

    if method == "GET" && path_only == "/api/virtual-keys" {
        return Some(CompatApiResponse {
            status: 200,
            payload: crate::dashboard_provider_runtime::virtual_keys_payload(root),
        });
    }

    if method == "POST" && path_only == "/api/virtual-keys" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let key_id = clean_text(
            request
                .get("key_id")
                .or_else(|| request.get("id"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let payload =
            crate::dashboard_provider_runtime::upsert_virtual_key(root, &key_id, &request);
        return Some(CompatApiResponse {
            status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                200
            } else {
                400
            },
            payload,
        });
    }

    if method == "DELETE" {
        if let Some((key_id, segments)) = parse_virtual_key_route(path_only) {
            if segments.is_empty() {
                return Some(CompatApiResponse {
                    status: 200,
                    payload: crate::dashboard_provider_runtime::remove_virtual_key(root, &key_id),
                });
            }
        }
    }

    if method == "POST" && path_only == "/api/models/discover" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let input = clean_text(
            request
                .get("input")
                .and_then(Value::as_str)
                .or_else(|| request.get("api_key").and_then(Value::as_str))
                .unwrap_or(""),
            4096,
        );
        let payload = crate::dashboard_provider_runtime::discover_models(root, &input);
        return Some(CompatApiResponse {
            status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                200
            } else {
                400
            },
            payload,
        });
    }
    if method == "POST" && path_only == "/api/models/download" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let provider = clean_text(
            request
                .get("provider")
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        );
        let model = clean_text(
            request.get("model").and_then(Value::as_str).unwrap_or(""),
            240,
        );
        let payload = crate::dashboard_provider_runtime::download_model(root, &provider, &model);
        return Some(CompatApiResponse {
            status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                200
            } else {
                400
            },
            payload,
        });
    }
    if method == "POST" && path_only == "/api/models/custom" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let provider = clean_text(
            request
                .get("provider")
                .and_then(Value::as_str)
                .unwrap_or("openrouter"),
            80,
        );
        let model = clean_text(
            request
                .get("id")
                .or_else(|| request.get("model"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            240,
        );
        let context_window = request
            .get("context_window")
            .and_then(Value::as_i64)
            .unwrap_or(128_000);
        let max_output_tokens = request
            .get("max_output_tokens")
            .and_then(Value::as_i64)
            .unwrap_or(8192);
        let payload = crate::dashboard_provider_runtime::add_custom_model(
            root,
            &provider,
            &model,
            context_window,
            max_output_tokens,
        );
        return Some(CompatApiResponse {
            status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                200
            } else {
                400
            },
            payload,
        });
    }
    if method == "DELETE" && path_only.starts_with("/api/models/custom/") {
        let model_ref = decode_path_segment(path_only.trim_start_matches("/api/models/custom/"));
        return Some(CompatApiResponse {
            status: 200,
            payload: crate::dashboard_provider_runtime::delete_custom_model(root, &model_ref),
        });
    }

    if method == "GET" && path_only == "/api/search/conversations" {
        let query = query_value(path, "q")
            .or_else(|| query_value(path, "query"))
            .unwrap_or_default();
        let limit = query_value(path, "limit")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(40);
        return Some(CompatApiResponse {
            status: 200,
            payload: crate::dashboard_internal_search::search_conversations(root, &query, limit),
        });
    }
    if method == "POST" && path_only == "/api/search/conversations" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let query = clean_text(
            request
                .get("q")
                .or_else(|| request.get("query"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            260,
        );
        let limit = request
            .get("limit")
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(40);
        return Some(CompatApiResponse {
            status: 200,
            payload: crate::dashboard_internal_search::search_conversations(root, &query, limit),
        });
    }

    if method == "GET" && path_only == "/api/agents/terminated" {
        return Some(CompatApiResponse {
            status: 200,
            payload: crate::dashboard_agent_state::terminated_entries(root),
        });
    }
    if method == "POST" && path_only.starts_with("/api/agents/") && path_only.ends_with("/revive") {
        let agent_id = path_only
            .trim_start_matches("/api/agents/")
            .trim_end_matches("/revive")
            .trim_matches('/');
        if !requester_agent.is_empty()
            && !actor_can_manage_target(root, snapshot, &requester_agent, agent_id)
        {
            return Some(CompatApiResponse {
                status: 403,
                payload: json!({
                    "ok": false,
                    "error": "agent_manage_forbidden",
                    "actor_agent_id": requester_agent.clone(),
                    "target_agent_id": clean_agent_id(agent_id)
                }),
            });
        }
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let role = request
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("analyst");
        return Some(CompatApiResponse {
            status: 200,
            payload: crate::dashboard_agent_state::revive_agent(root, agent_id, role),
        });
    }
    if method == "DELETE" && path_only == "/api/agents/terminated" {
        if !requester_agent.is_empty() {
            return Some(CompatApiResponse {
                status: 403,
                payload: json!({
                    "ok": false,
                    "error": "agent_manage_forbidden",
                    "actor_agent_id": requester_agent.clone(),
                    "target_agent_id": "terminated/*"
                }),
            });
        }
        if query_value(path, "all")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
        {
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_agent_state::delete_all_terminated(root),
            });
        }
    }
    if method == "DELETE" && path_only.starts_with("/api/agents/terminated/") {
        let agent_id = path_only
            .trim_start_matches("/api/agents/terminated/")
            .trim();
        if !requester_agent.is_empty()
            && !actor_can_manage_target(root, snapshot, &requester_agent, agent_id)
        {
            return Some(CompatApiResponse {
                status: 403,
                payload: json!({
                    "ok": false,
                    "error": "agent_manage_forbidden",
                    "actor_agent_id": requester_agent.clone(),
                    "target_agent_id": clean_agent_id(agent_id)
                }),
            });
        }
        return Some(CompatApiResponse {
            status: 200,
            payload: crate::dashboard_agent_state::delete_terminated(
                root,
                agent_id,
                query_value(path, "contract_id").as_deref(),
            ),
        });
    }

    if method == "GET" && path_only == "/api/agents" {
        let _ = crate::dashboard_agent_state::enforce_expired_contracts(root);
        let include_terminated = query_value(path, "include_terminated")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        return Some(CompatApiResponse {
            status: 200,
            payload: Value::Array(build_agent_roster(root, snapshot, include_terminated)),
        });
    }

    if method == "POST" && path_only == "/api/agents/archive-all" {
        if !requester_agent.is_empty() {
            return Some(CompatApiResponse {
                status: 403,
                payload: json!({
                    "ok": false,
                    "error": "agent_manage_forbidden",
                    "actor_agent_id": requester_agent.clone(),
                    "target_agent_id": "*"
                }),
            });
        }
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let reason = clean_text(
            request
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or("user_archive_all"),
            120,
        );
        return Some(CompatApiResponse {
            status: 200,
            payload: archive_all_visible_agents(root, snapshot, &reason),
        });
    }

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
            requester_agent.clone()
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

    if let Some((requested_agent_id, segments)) = parse_agent_route(path_only) {
        let agent_id = resolve_agent_id_alias(root, &requested_agent_id);
        if !requester_agent.is_empty()
            && method != "GET"
            && requester_agent != agent_id
            && !actor_can_manage_target(root, snapshot, &requester_agent, &agent_id)
        {
            return Some(CompatApiResponse {
                status: 403,
                payload: json!({
                    "ok": false,
                    "error": "agent_manage_forbidden",
                    "actor_agent_id": requester_agent.clone(),
                    "target_agent_id": agent_id
                }),
            });
        }
        let existing = agent_row_by_id(root, snapshot, &agent_id);
        let is_archived =
            crate::dashboard_agent_state::archived_agent_ids(root).contains(&agent_id);
        if method == "GET" && segments.is_empty() {
            if let Some(row) = existing {
                return Some(CompatApiResponse {
                    status: 200,
                    payload: row,
                });
            }
            if is_archived {
                return Some(CompatApiResponse {
                    status: 200,
                    payload: archived_agent_stub(root, &agent_id),
                });
            }
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "agent_not_found", "agent_id": agent_id}),
            });
        }

        if method == "DELETE" && segments.is_empty() {
            if existing.is_none() {
                if is_archived {
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: json!({
                            "ok": true,
                            "type": "dashboard_agent_archive",
                            "id": agent_id,
                            "agent_id": agent_id,
                            "state": "inactive",
                            "archived": true
                        }),
                    });
                }
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "agent_not_found", "agent_id": agent_id}),
                });
            }
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let requested_archive_reason = clean_text(
                request.get("reason").and_then(Value::as_str).unwrap_or(""),
                120,
            );
            let archive_reason = if requested_archive_reason.is_empty() {
                "user_archive".to_string()
            } else {
                requested_archive_reason
            };
            let requested_termination_reason = clean_text(
                request
                    .get("termination_reason")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                80,
            )
            .to_ascii_lowercase();
            let archive_reason_lower = archive_reason.to_ascii_lowercase();
            let termination_reason = if requested_termination_reason == "parent_archived"
                || archive_reason_lower == "archived by parent agent"
                || archive_reason_lower == "parent_archived"
                || archive_reason_lower == "parent_archive"
            {
                "parent_archived"
            } else {
                "user_archived"
            };
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
                    "termination_reason": termination_reason,
                    "terminated_at": crate::now_iso(),
                    "updated_at": crate::now_iso()
                }),
            );
            let _ = crate::dashboard_agent_state::archive_agent(root, &agent_id, &archive_reason);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "type": "dashboard_agent_archive",
                    "id": agent_id,
                    "agent_id": agent_id,
                    "state": "inactive",
                    "archived": true,
                    "reason": archive_reason
                }),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "stop" {
            if existing.is_none() {
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "agent_not_found", "agent_id": agent_id}),
                });
            }
            let _ = upsert_contract_patch(
                root,
                &agent_id,
                &json!({
                    "status": "terminated",
                    "termination_reason": "stopped",
                    "terminated_at": crate::now_iso(),
                    "updated_at": crate::now_iso()
                }),
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "type": "dashboard_agent_stop", "agent_id": agent_id}),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "start" {
            if existing.is_none() {
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "agent_not_found", "agent_id": agent_id}),
                });
            }
            let _ = update_profile_patch(
                root,
                &agent_id,
                &json!({
                    "state": "Running",
                    "updated_at": crate::now_iso()
                }),
            );
            let _ = upsert_contract_patch(
                root,
                &agent_id,
                &json!({
                    "status": "active",
                    "termination_reason": "",
                    "terminated_at": "",
                    "updated_at": crate::now_iso()
                }),
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "type": "dashboard_agent_start", "agent_id": agent_id}),
            });
        }

        if existing.is_none() {
            if is_archived && method == "POST" && segments.len() == 1 && segments[0] == "message" {
                return Some(CompatApiResponse {
                    status: 409,
                    payload: json!({
                        "ok": false,
                        "error": "agent_inactive",
                        "agent_id": agent_id,
                        "state": "inactive",
                        "archived": true
                    }),
                });
            }
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "agent_not_found", "agent_id": agent_id}),
            });
        }

        if method == "GET" && segments.len() == 1 && segments[0] == "session" {
            return Some(CompatApiResponse {
                status: 200,
                payload: session_payload(root, &agent_id),
            });
        }

        if method == "POST"
            && segments.len() == 2
            && segments[0] == "session"
            && segments[1] == "reset"
        {
            return Some(CompatApiResponse {
                status: 200,
                payload: reset_active_session(root, &agent_id),
            });
        }

        if method == "POST"
            && segments.len() == 2
            && segments[0] == "session"
            && segments[1] == "compact"
        {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            return Some(CompatApiResponse {
                status: 200,
                payload: compact_active_session(root, &agent_id, &request),
            });
        }

        if method == "GET" && segments.len() == 1 && segments[0] == "sessions" {
            let payload = session_payload(root, &agent_id);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "agent_id": agent_id,
                    "active_session_id": payload.get("active_session_id").cloned().unwrap_or_else(|| json!("default")),
                    "sessions": payload.get("sessions").cloned().unwrap_or_else(|| json!([]))
                }),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "sessions" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let label = clean_text(
                request
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or("Session"),
                80,
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_agent_state::create_session(root, &agent_id, &label),
            });
        }

        if method == "POST"
            && segments.len() == 3
            && segments[0] == "sessions"
            && segments[2] == "switch"
        {
            let session_id = decode_path_segment(&segments[1]);
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_agent_state::switch_session(root, &agent_id, &session_id),
            });
        }

        if method == "DELETE" && segments.len() == 1 && segments[0] == "history" {
            let mut state = load_session_state(root, &agent_id);
            if let Some(rows) = state.get_mut("sessions").and_then(Value::as_array_mut) {
                for row in rows.iter_mut() {
                    row["messages"] = Value::Array(Vec::new());
                    row["updated_at"] = Value::String(crate::now_iso());
                }
            }
            save_session_state(root, &agent_id, &state);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "type": "dashboard_agent_history_cleared", "agent_id": agent_id}),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "message" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let message = clean_text(
                request.get("message").and_then(Value::as_str).unwrap_or(""),
                8_000,
            );
            if message.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "message_required"}),
                });
            }
            let row = existing.clone().unwrap_or_else(|| json!({}));
            let lowered = message.to_ascii_lowercase();
            let contains_any = |terms: &[&str]| terms.iter().any(|term| lowered.contains(term));
            let contract_violation = (contains_any(&["ignore", "bypass", "disable", "override"])
                && contains_any(&["contract", "safety", "policy", "receipt"]))
                || contains_any(&["exfiltrate", "steal", "dump secrets", "leak", "secrets"]);
            if contract_violation {
                let _ = upsert_contract_patch(
                    root,
                    &agent_id,
                    &json!({
                        "status": "terminated",
                        "termination_reason": "contract_violation",
                        "terminated_at": crate::now_iso(),
                        "updated_at": crate::now_iso()
                    }),
                );
                return Some(CompatApiResponse {
                    status: 409,
                    payload: json!({
                        "ok": false,
                        "error": "agent_contract_terminated",
                        "agent_id": agent_id,
                        "termination_reason": "contract_violation"
                    }),
                });
            }
            let workspace_hints = workspace_file_hints_for_message(root, Some(&row), &message, 5);
            let latent_tool_candidates =
                latent_tool_candidates_for_message(&message, &workspace_hints);
            let mut resolved_tool_intent = direct_tool_intent_from_user_message(&message);
            let mut replayed_pending_confirmation = false;
            if let Some((pending_tool_name, mut pending_tool_input)) =
                pending_tool_confirmation_call(root, &agent_id)
            {
                if resolved_tool_intent.is_none() {
                    if message_is_negative_confirmation(&message) {
                        clear_pending_tool_confirmation(root, &agent_id);
                    } else if message_is_affirmative_confirmation(&message) {
                        if !pending_tool_input.is_object() {
                            pending_tool_input = json!({});
                        }
                        if !input_has_confirmation(&pending_tool_input) {
                            pending_tool_input["confirm"] = Value::Bool(true);
                        }
                        if input_approval_note(&pending_tool_input).is_empty() {
                            pending_tool_input["approval_note"] =
                                Value::String("user confirmed pending action".to_string());
                        }
                        resolved_tool_intent = Some((pending_tool_name, pending_tool_input));
                        replayed_pending_confirmation = true;
                    }
                }
            }
            if available_model_count(root, snapshot) == 0 && resolved_tool_intent.is_none() {
                return Some(CompatApiResponse {
                    status: 503,
                    payload: no_models_available_payload(&agent_id),
                });
            }
            if let Some((tool_name, tool_input)) = resolved_tool_intent {
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
                let requires_confirmation = tool_error_requires_confirmation(&tool_payload);
                if requires_confirmation {
                    store_pending_tool_confirmation(
                        root,
                        &agent_id,
                        &tool_name,
                        &tool_input,
                        "direct_message",
                    );
                } else {
                    clear_pending_tool_confirmation(root, &agent_id);
                }
                let mut response_text = summarize_tool_payload(&tool_name, &tool_payload);
                if response_text.trim().is_empty() {
                    response_text = if ok {
                        format!(
                            "I ran `{}`, but it returned no usable findings yet. Ask me to retry with a narrower input.",
                            normalize_tool_name(&tool_name)
                        )
                    } else {
                        user_facing_tool_failure_summary(&tool_name, &tool_payload).unwrap_or_else(
                            || {
                                format!(
                                    "I couldn't complete `{}` right now.",
                                    normalize_tool_name(&tool_name)
                                )
                            },
                        )
                    };
                }
                if ok && response_looks_like_tool_ack_without_findings(&response_text) {
                    response_text = format!(
                        "I ran `{}`, but it returned no usable findings yet. Ask me to retry with a narrower input.",
                        normalize_tool_name(&tool_name)
                    );
                }
                if !user_requested_internal_runtime_details(&message) {
                    response_text = abstract_runtime_mechanics_terms(&response_text);
                }
                response_text = strip_internal_cache_control_markup(&response_text);
                let tool_card = json!({
                    "id": format!("tool-direct-{}", normalize_tool_name(&tool_name)),
                    "name": normalize_tool_name(&tool_name),
                    "input": trim_text(&tool_input.to_string(), 4000),
                    "result": trim_text(&summarize_tool_payload(&tool_name, &tool_payload), 24_000),
                    "is_error": !ok
                });
                let response_tools = vec![tool_card.clone()];
                let (finalized_response, tool_completion, finalization_seed) =
                    enforce_user_facing_finalization_contract(response_text, &response_tools);
                let mut tooling_fallback_used = false;
                let mut finalized_response = finalized_response;
                let mut finalization_outcome = clean_text(&finalization_seed, 180);
                let mut tool_completion = tool_completion;
                if let Some(tooling_fallback) =
                    maybe_tooling_failure_fallback(&message, &finalized_response, "")
                {
                    finalized_response = tooling_fallback;
                    finalization_outcome =
                        format!("{finalization_outcome}+tooling_failure_fallback");
                    tooling_fallback_used = true;
                    let (contracted, report, retry_outcome) =
                        enforce_user_facing_finalization_contract(
                            finalized_response,
                            &response_tools,
                        );
                    finalized_response = contracted;
                    tool_completion = report;
                    finalization_outcome =
                        merge_response_outcomes(&finalization_outcome, &retry_outcome, 180);
                }
                tool_completion = enrich_tool_completion_receipt(tool_completion, &response_tools);
                let final_ack_only =
                    response_looks_like_tool_ack_without_findings(&finalized_response);
                response_text = finalized_response;
                let response_finalization = json!({
                    "applied": finalization_outcome != "unchanged",
                    "outcome": finalization_outcome,
                    "initial_ack_only": tool_completion
                        .get("initial_ack_only")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    "final_ack_only": final_ack_only,
                    "findings_available": tool_completion
                        .get("findings_available")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    "tool_completion": tool_completion,
                    "pending_confirmation_replayed": replayed_pending_confirmation,
                    "tooling_fallback_used": tooling_fallback_used,
                    "retry_attempted": false,
                    "retry_used": false
                });
                let turn_transaction = crate::dashboard_tool_turn_loop::turn_transaction_payload(
                    "complete", "complete", "complete", "complete",
                );
                let mut turn_receipt =
                    append_turn_message(root, &agent_id, &message, &response_text);
                turn_receipt["response_finalization"] = response_finalization.clone();
                return Some(CompatApiResponse {
                    status: if ok { 200 } else { 400 },
                    payload: json!({
                        "ok": ok,
                        "agent_id": agent_id,
                        "provider": "tool",
                        "model": "tool-router",
                        "runtime_model": "tool-router",
                        "iterations": 1,
                        "input_tokens": estimate_tokens(&message),
                        "output_tokens": estimate_tokens(&response_text),
                        "cost_usd": 0.0,
                        "response": response_text,
                        "tools": response_tools,
                        "response_finalization": response_finalization,
                        "turn_transaction": turn_transaction,
                        "workspace_hints": workspace_hints,
                        "latent_tool_candidates": latent_tool_candidates,
                        "attention_queue": turn_receipt.get("attention_queue").cloned().unwrap_or_else(|| json!({})),
                        "memory_capture": turn_receipt.get("memory_capture").cloned().unwrap_or_else(|| json!({}))
                    }),
                });
            }
            let requested_provider = clean_text(
                row.get("model_provider")
                    .and_then(Value::as_str)
                    .unwrap_or("auto"),
                80,
            );
            let requested_model = clean_text(
                row.get("model_name").and_then(Value::as_str).unwrap_or(""),
                240,
            );
            let virtual_key_id = clean_text(
                request
                    .get("virtual_key_id")
                    .or_else(|| request.get("virtual_key"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120,
            );
            let route_request = json!({
                "agent_id": agent_id,
                "message": message,
                "task_type": row.get("role").cloned().unwrap_or_else(|| json!("general")),
                "token_count": estimate_tokens(&message),
                "virtual_key_id": if virtual_key_id.is_empty() { Value::Null } else { json!(virtual_key_id.clone()) },
                "has_vision": request
                    .get("attachments")
                    .and_then(Value::as_array)
                    .map(|rows| rows.iter().any(|row| {
                        clean_text(
                            row.get("content_type")
                                .or_else(|| row.get("mime_type"))
                                .and_then(Value::as_str)
                                .unwrap_or(""),
                            120,
                        )
                        .to_ascii_lowercase()
                        .starts_with("image/")
                    }))
                    .unwrap_or(false)
            });
            let (provider, model, auto_route) =
                crate::dashboard_model_catalog::resolve_model_selection(
                    root,
                    snapshot,
                    &requested_provider,
                    &requested_model,
                    &route_request,
                );
            let mut provider = provider;
            let mut model = model;
            let mut virtual_key_gate = Value::Null;
            if !virtual_key_id.is_empty() {
                let gate = crate::dashboard_provider_runtime::reserve_virtual_key_slot(
                    root,
                    &virtual_key_id,
                );
                if !gate.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    let error_code = clean_text(
                        gate.get("error")
                            .and_then(Value::as_str)
                            .unwrap_or("virtual_key_denied"),
                        80,
                    );
                    let status = if error_code == "virtual_key_budget_exceeded" {
                        402
                    } else if error_code == "virtual_key_rate_limited" {
                        429
                    } else {
                        400
                    };
                    return Some(CompatApiResponse {
                        status,
                        payload: json!({
                            "ok": false,
                            "agent_id": agent_id,
                            "error": error_code,
                            "virtual_key_id": virtual_key_id,
                            "virtual_key": gate
                        }),
                    });
                }
                let route_hint = crate::dashboard_provider_runtime::resolve_virtual_key_route(
                    root,
                    &virtual_key_id,
                );
                let key_provider = clean_text(
                    route_hint
                        .get("provider")
                        .or_else(|| gate.get("provider"))
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    80,
                );
                let key_model = clean_text(
                    route_hint
                        .get("model")
                        .or_else(|| gate.get("model"))
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    240,
                );
                if !key_provider.is_empty() && !key_provider.eq_ignore_ascii_case("auto") {
                    provider = key_provider;
                }
                if !key_model.is_empty() && !key_model.eq_ignore_ascii_case("auto") {
                    model = key_model;
                }
                virtual_key_gate = gate;
            }
            let mut state = load_session_state(root, &agent_id);
            let sessions_total = state
                .get("sessions")
                .and_then(Value::as_array)
                .map(|rows| rows.len())
                .unwrap_or(0);
            let row_context_window = row
                .get("context_window_tokens")
                .or_else(|| row.get("context_window"))
                .and_then(Value::as_i64)
                .unwrap_or(0);
            let fallback_window = if row_context_window > 0 {
                row_context_window
            } else {
                128_000
            };
            let active_context_target_tokens = request
                .get("active_context_target_tokens")
                .or_else(|| request.get("target_context_window"))
                .and_then(Value::as_i64)
                .unwrap_or_else(|| ((fallback_window as f64) * 0.68).round() as i64)
                .clamp(4_096, 512_000);
            let active_context_min_recent = request
                .get("active_context_min_recent_messages")
                .or_else(|| request.get("min_recent_messages"))
                .and_then(Value::as_u64)
                .unwrap_or(ACTIVE_CONTEXT_MIN_RECENT_FLOOR as u64)
                .clamp(ACTIVE_CONTEXT_MIN_RECENT_FLOOR as u64, 256)
                as usize;
            let include_all_sessions_context = request
                .get("include_all_sessions_context")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let row_system_context_limit = row
                .get("system_context_tokens")
                .or_else(|| row.get("context_pool_limit_tokens"))
                .and_then(Value::as_i64)
                .unwrap_or(1_000_000);
            let row_auto_compact_threshold_ratio = row
                .get("auto_compact_threshold_ratio")
                .and_then(Value::as_f64)
                .unwrap_or(0.95);
            let row_auto_compact_target_ratio = row
                .get("auto_compact_target_ratio")
                .and_then(Value::as_f64)
                .unwrap_or(0.72);
            let context_pool_limit_tokens = request
                .get("context_pool_limit_tokens")
                .and_then(Value::as_i64)
                .unwrap_or(row_system_context_limit)
                .clamp(32_000, 2_000_000);
            let auto_compact_threshold_ratio = request
                .get("auto_compact_threshold_ratio")
                .and_then(Value::as_f64)
                .unwrap_or(row_auto_compact_threshold_ratio)
                .clamp(0.75, 0.99);
            let auto_compact_target_ratio = request
                .get("auto_compact_target_ratio")
                .and_then(Value::as_f64)
                .unwrap_or(row_auto_compact_target_ratio)
                .clamp(0.40, 0.90);
            // Conversation history is authoritative and must not be rewritten as a side effect
            // of normal message execution. Manual compaction remains available through explicit
            // compaction routes only.
            let history_trim_confirmed = false;
            let persist_system_prune = false;
            let persist_auto_compact = false;
            let mut messages = context_source_messages(&state, include_all_sessions_context);
            let all_session_history_count = context_source_messages(&state, true).len();
            let mut pooled_messages = trim_context_pool(&messages, context_pool_limit_tokens);
            let pre_generation_pruned = pooled_messages.len() != messages.len();
            if pre_generation_pruned && persist_system_prune {
                set_active_session_messages(&mut state, &pooled_messages);
                save_session_state(root, &agent_id, &state);
                state = load_session_state(root, &agent_id);
                messages = context_source_messages(&state, include_all_sessions_context);
                pooled_messages = trim_context_pool(&messages, context_pool_limit_tokens);
            }
            let (pooled_messages_with_floor, recent_floor_injected) = enforce_recent_context_floor(
                &messages,
                &pooled_messages,
                active_context_min_recent,
            );
            let recent_floor_enforced = recent_floor_injected > 0;
            pooled_messages = pooled_messages_with_floor;
            if all_session_history_count > 0 && messages.is_empty() {
                return Some(CompatApiResponse {
                    status: 503,
                    payload: crate::dashboard_tool_turn_loop::hydration_failed_payload(&agent_id),
                });
            }
            let mut active_messages = select_active_context_window(
                &pooled_messages,
                active_context_target_tokens,
                active_context_min_recent,
            );
            let mut context_pool_tokens = total_message_tokens(&pooled_messages);
            let mut context_active_tokens = total_message_tokens(&active_messages);
            let mut context_ratio = if fallback_window > 0 {
                (context_active_tokens as f64 / fallback_window as f64).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut context_pressure = context_pressure_label(context_ratio).to_string();
            let mut emergency_compact = json!({
                "triggered": false,
                "threshold_ratio": auto_compact_threshold_ratio,
                "target_ratio": auto_compact_target_ratio,
                "removed_messages": 0
            });
            if context_ratio >= auto_compact_threshold_ratio && fallback_window > 0 {
                let emergency_target_tokens =
                    ((fallback_window as f64) * auto_compact_target_ratio).round() as i64;
                let emergency_min_recent = request
                    .get("emergency_min_recent_messages")
                    .or_else(|| request.get("min_recent_messages"))
                    .and_then(Value::as_u64)
                    .unwrap_or(active_context_min_recent as u64)
                    .clamp(ACTIVE_CONTEXT_MIN_RECENT_FLOOR as u64, 256)
                    as usize;
                let emergency_messages = select_active_context_window(
                    &pooled_messages,
                    emergency_target_tokens,
                    emergency_min_recent,
                );
                let emergency_tokens = total_message_tokens(&emergency_messages);
                let removed_messages = pooled_messages
                    .len()
                    .saturating_sub(emergency_messages.len())
                    as u64;
                emergency_compact = json!({
                    "triggered": true,
                    "threshold_ratio": auto_compact_threshold_ratio,
                    "target_ratio": auto_compact_target_ratio,
                    "removed_messages": removed_messages,
                    "before_tokens": context_active_tokens,
                    "after_tokens": emergency_tokens,
                    "persisted_to_history": false
                });
                if removed_messages > 0 {
                    active_messages = emergency_messages;
                    context_pool_tokens = total_message_tokens(&pooled_messages);
                    context_active_tokens = emergency_tokens;
                    context_ratio = if fallback_window > 0 {
                        (context_active_tokens as f64 / fallback_window as f64).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    context_pressure = context_pressure_label(context_ratio).to_string();
                    if persist_auto_compact {
                        let compact_request = json!({
                            "target_context_window": fallback_window,
                            "target_ratio": auto_compact_target_ratio,
                            "min_recent_messages": emergency_min_recent,
                            "max_messages": request
                                .get("max_messages")
                                .and_then(Value::as_u64)
                                .unwrap_or(220)
                                .clamp(20, 800)
                        });
                        let compact_result =
                            compact_active_session(root, &agent_id, &compact_request);
                        emergency_compact["persisted_to_history"] = json!(true);
                        emergency_compact["persist_result"] = compact_result;
                    }
                }
            }
            let memory_kv_entries = memory_kv_pairs_from_state(&state).len();
            let memory_prompt_context = memory_kv_prompt_context(&state, 24);
            let instinct_prompt_context = agent_instinct_prompt_context(root, 6_000);
            let plugin_prompt_context =
                dashboard_skills_marketplace::skills_prompt_context(root, 12, 4_000);
            let passive_memory_context =
                passive_attention_context_for_message(root, &agent_id, &message, 6);
            let keyframe_context = context_keyframes_prompt_context(&state, 8, 2_400);
            let overflow_keyframes_context =
                historical_context_keyframes_prompt_context(&messages, &active_messages, 10, 2_400);
            let relevant_recall_context = historical_relevant_recall_prompt_context(
                &messages,
                &active_messages,
                &message,
                8,
                2_800,
            );
            let identity_hydration_prompt = agent_identity_hydration_prompt(&row);
            let custom_system_prompt = clean_text(
                row.get("system_prompt")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                12_000,
            );
            let inline_tools_allowed = inline_tool_calls_allowed_for_user_message(&message);
            let mut prompt_parts = Vec::<String>::new();
            if !identity_hydration_prompt.is_empty() {
                prompt_parts.push(identity_hydration_prompt);
            }
            prompt_parts.push(AGENT_RUNTIME_SYSTEM_PROMPT.to_string());
            if !inline_tools_allowed {
                prompt_parts.push("Direct-answer guard: default to natural conversational answers. Do not emit `<function=...>` tool calls unless the user explicitly requested web retrieval, file/terminal operations, memory operations, or agent management in this turn.".to_string());
            }
            if !instinct_prompt_context.is_empty() {
                prompt_parts.push(instinct_prompt_context);
            }
            if !plugin_prompt_context.is_empty() {
                prompt_parts.push(plugin_prompt_context);
            }
            if !passive_memory_context.is_empty() {
                prompt_parts.push(passive_memory_context);
            }
            if !keyframe_context.is_empty() {
                prompt_parts.push(keyframe_context);
            }
            if !overflow_keyframes_context.is_empty() {
                prompt_parts.push(overflow_keyframes_context);
            }
            if !relevant_recall_context.is_empty() {
                prompt_parts.push(relevant_recall_context);
            }
            if !custom_system_prompt.is_empty() {
                prompt_parts.push(custom_system_prompt);
            }
            if !memory_prompt_context.is_empty() {
                prompt_parts.push(memory_prompt_context);
            }
            let system_prompt = clean_text(&prompt_parts.join("\n\n"), 12_000);
            match crate::dashboard_provider_runtime::invoke_chat(
                root,
                &provider,
                &model,
                &system_prompt,
                &active_messages,
                &message,
            ) {
                Ok(result) => {
                    let mut response_text = clean_chat_text(
                        result.get("response").and_then(Value::as_str).unwrap_or(""),
                        32_000,
                    );
                    let response_had_context_meta =
                        internal_context_metadata_phrase(&response_text);
                    response_text = strip_internal_context_metadata_prefix(&response_text);
                    response_text = strip_internal_cache_control_markup(&response_text);
                    if response_text.is_empty() && response_had_context_meta {
                        response_text = "I have relevant prior context loaded and can keep going from here. Tell me what you want to do next.".to_string();
                    }
                    let runtime_summary = runtime_sync_summary(snapshot);
                    let runtime_probe = runtime_probe_requested(&message);
                    let runtime_denial = runtime_access_denied_phrase(&response_text);
                    if runtime_probe || runtime_denial {
                        response_text = if runtime_probe {
                            runtime_access_summary_text(&runtime_summary)
                        } else {
                            "I can access runtime telemetry, persistent memory, workspace files, channels, and approved command surfaces in this session. Tell me what you want me to check and I will run it now.".to_string()
                        };
                    }
                    if memory_recall_requested(&message)
                        || persistent_memory_denied_phrase(&response_text)
                    {
                        response_text = build_memory_recall_response(&state, &messages, &message);
                    }
                    let explicit_parallel_directive = swarm_intent_requested(&message)
                        || message.to_ascii_lowercase().contains("multi-agent")
                        || message.to_ascii_lowercase().contains("multi agent");
                    let response_denied_spawn = spawn_surface_denied_phrase(&response_text);
                    let response_has_tool_call = response_text.contains("<function=");
                    if explicit_parallel_directive
                        && (response_denied_spawn || !response_has_tool_call)
                    {
                        let auto_count = infer_subagent_count_from_message(&message);
                        let directive_hint_receipt = crate::deterministic_receipt_hash(&json!({
                            "agent_id": agent_id,
                            "message": message,
                            "requested_at": crate::now_iso()
                        }));
                        response_text = format!(
                            "<function=spawn_subagents>{}</function>",
                            json!({
                                "count": auto_count,
                                "objective": message,
                                "reason": "user_directive_parallelization",
                                "directive_receipt_hint": directive_hint_receipt,
                                "confirm": true,
                                "approval_note": "user requested parallelization in active turn"
                            })
                            .to_string()
                        );
                    }
                    let (
                        tool_adjusted_response,
                        response_tools,
                        inline_pending_confirmation,
                        inline_tools_suppressed,
                    ) = execute_inline_tool_calls(
                        root,
                        snapshot,
                        &agent_id,
                        Some(&row),
                        &response_text,
                        &message,
                        inline_tools_allowed,
                    );
                    response_text = tool_adjusted_response;
                    if inline_tools_suppressed {
                        let direct_only_prompt = clean_text(
                            &format!(
                                "{}\n\nDirect-answer guard: unless the user explicitly requested tool execution in this turn, do not emit `<function=...>` calls. Respond directly in natural language.",
                                AGENT_RUNTIME_SYSTEM_PROMPT
                            ),
                            12_000,
                        );
                        if let Ok(retried) = crate::dashboard_provider_runtime::invoke_chat(
                            root,
                            &provider,
                            &model,
                            &direct_only_prompt,
                            &active_messages,
                            &message,
                        ) {
                            let mut retried_text = clean_chat_text(
                                retried
                                    .get("response")
                                    .and_then(Value::as_str)
                                    .unwrap_or(""),
                                32_000,
                            );
                            retried_text = strip_internal_context_metadata_prefix(&retried_text);
                            retried_text = strip_internal_cache_control_markup(&retried_text);
                            let (without_inline_calls, _) =
                                extract_inline_tool_calls(&retried_text, 6);
                            let candidate = if without_inline_calls.trim().is_empty() {
                                retried_text
                            } else {
                                without_inline_calls
                            };
                            if !candidate.trim().is_empty() {
                                response_text = clean_chat_text(candidate.trim(), 32_000);
                            }
                        }
                        if response_text.trim().is_empty() {
                            response_text = "I can answer directly without tool calls. Ask your question naturally and I will respond conversationally unless you explicitly request a tool run.".to_string();
                        }
                    }
                    if response_tools.is_empty()
                        && !inline_tools_allowed
                        && (response_is_no_findings_placeholder(&response_text)
                            || response_looks_like_raw_web_artifact_dump(&response_text)
                            || response_looks_like_unsynthesized_web_snippet_dump(&response_text))
                    {
                        let no_fake_tooling_prompt = clean_text(
                            &format!(
                                "{}\n\nNo-fake-tooling guard: if no tool call executed in this turn, do not claim web retrieval/findings. Answer directly from stable context and label uncertainty when needed.",
                                AGENT_RUNTIME_SYSTEM_PROMPT
                            ),
                            12_000,
                        );
                        if let Ok(retried) = crate::dashboard_provider_runtime::invoke_chat(
                            root,
                            &provider,
                            &model,
                            &no_fake_tooling_prompt,
                            &active_messages,
                            &message,
                        ) {
                            let mut retried_text = clean_chat_text(
                                retried
                                    .get("response")
                                    .and_then(Value::as_str)
                                    .unwrap_or(""),
                                32_000,
                            );
                            retried_text = strip_internal_context_metadata_prefix(&retried_text);
                            retried_text = strip_internal_cache_control_markup(&retried_text);
                            let (without_inline_calls, _) =
                                extract_inline_tool_calls(&retried_text, 6);
                            let candidate = if without_inline_calls.trim().is_empty() {
                                retried_text
                            } else {
                                without_inline_calls
                            };
                            if !candidate.trim().is_empty() {
                                response_text = clean_chat_text(candidate.trim(), 32_000);
                            }
                        }
                        if response_text.trim().is_empty()
                            || response_is_no_findings_placeholder(&response_text)
                            || response_looks_like_raw_web_artifact_dump(&response_text)
                            || response_looks_like_unsynthesized_web_snippet_dump(&response_text)
                        {
                            response_text = "I can answer this directly without running tools. If you want live sourcing, ask me to run a web search explicitly.".to_string();
                        }
                    }
                    if let Some(pending) = inline_pending_confirmation {
                        let pending_tool = clean_text(
                            pending
                                .get("tool_name")
                                .or_else(|| pending.get("tool"))
                                .and_then(Value::as_str)
                                .unwrap_or(""),
                            120,
                        );
                        if !pending_tool.is_empty() {
                            let pending_input =
                                pending.get("input").cloned().unwrap_or_else(|| json!({}));
                            store_pending_tool_confirmation(
                                root,
                                &agent_id,
                                &pending_tool,
                                &pending_input,
                                pending
                                    .get("source")
                                    .and_then(Value::as_str)
                                    .unwrap_or("inline_tool_call"),
                            );
                        }
                    } else if !response_tools.is_empty() {
                        clear_pending_tool_confirmation(root, &agent_id);
                    } else if message_is_negative_confirmation(&message) {
                        clear_pending_tool_confirmation(root, &agent_id);
                    }
                    if !user_requested_internal_runtime_details(&message) {
                        response_text = abstract_runtime_mechanics_terms(&response_text);
                    }
                    response_text = strip_internal_cache_control_markup(&response_text);
                    if response_is_unrelated_context_dump(&message, &response_text) {
                        let strict_relevance_prompt = clean_text(
                            &format!(
                                "{}\n\nRelevance guard: answer only the latest user request. Ignore unrelated prior snippets and project templates. If the user asks for code, provide direct code first.",
                                AGENT_RUNTIME_SYSTEM_PROMPT
                            ),
                            12_000,
                        );
                        let retried = crate::dashboard_provider_runtime::invoke_chat(
                            root,
                            &provider,
                            &model,
                            &strict_relevance_prompt,
                            &[],
                            &message,
                        )
                        .ok()
                        .and_then(|value| {
                            let mut retried_text = clean_chat_text(
                                value.get("response").and_then(Value::as_str).unwrap_or(""),
                                32_000,
                            );
                            retried_text = strip_internal_context_metadata_prefix(&retried_text);
                            retried_text = strip_internal_cache_control_markup(&retried_text);
                            if !user_requested_internal_runtime_details(&message) {
                                retried_text = abstract_runtime_mechanics_terms(&retried_text);
                            }
                            if response_is_unrelated_context_dump(&message, &retried_text) {
                                None
                            } else {
                                let cleaned = retried_text.trim().to_string();
                                if cleaned.is_empty() {
                                    None
                                } else {
                                    Some(cleaned)
                                }
                            }
                        });
                        response_text = retried.unwrap_or_else(|| {
                            "I dropped an unrelated context artifact and did not return it. Please resend your request and I will answer only that prompt.".to_string()
                        });
                    }
                    let (mut finalized_response, mut tool_completion, seed_outcome) =
                        enforce_user_facing_finalization_contract(response_text, &response_tools);
                    let mut finalization_outcome = clean_text(&seed_outcome, 200);
                    let initial_ack_only = tool_completion
                        .get("initial_ack_only")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let mut retry_attempted = false;
                    let mut retry_used = false;
                    if initial_ack_only
                        && tool_completion
                            .get("final_ack_only")
                            .and_then(Value::as_bool)
                            .unwrap_or(false)
                    {
                        retry_attempted = true;
                        let strict_tool_prompt = clean_text(
                            &format!(
                                "{}\n\nOutput guard: Return synthesized findings or an explicit no-findings reason. Do not output tool status text like 'Web search completed' or 'Tool call finished'.",
                                AGENT_RUNTIME_SYSTEM_PROMPT
                            ),
                            12_000,
                        );
                        if let Ok(retried) = crate::dashboard_provider_runtime::invoke_chat(
                            root,
                            &provider,
                            &model,
                            &strict_tool_prompt,
                            &active_messages,
                            &message,
                        ) {
                            let mut retried_text = clean_chat_text(
                                retried
                                    .get("response")
                                    .and_then(Value::as_str)
                                    .unwrap_or(""),
                                32_000,
                            );
                            retried_text = strip_internal_context_metadata_prefix(&retried_text);
                            retried_text = strip_internal_cache_control_markup(&retried_text);
                            if !user_requested_internal_runtime_details(&message) {
                                retried_text = abstract_runtime_mechanics_terms(&retried_text);
                            }
                            let (retry_finalized, _retry_report, retry_outcome) =
                                enforce_user_facing_finalization_contract(
                                    retried_text,
                                    &response_tools,
                                );
                            finalized_response = retry_finalized;
                            finalization_outcome = merge_response_outcomes(
                                &finalization_outcome,
                                &format!("retry:{retry_outcome}"),
                                200,
                            );
                            retry_used = true;
                        }
                    }
                    let mut synthesis_retry_used = false;
                    if response_is_no_findings_placeholder(&finalized_response)
                        && message_requests_comparative_answer(&message)
                    {
                        let synthesis_prompt = clean_text(
                            &format!(
                                "{}\n\nFallback guard: if tool extraction failed or returned no usable findings, still answer the user directly using stable knowledge. Prioritize relevance to the latest request and return usable content in the requested format.",
                                AGENT_RUNTIME_SYSTEM_PROMPT
                            ),
                            12_000,
                        );
                        if let Ok(retried) = crate::dashboard_provider_runtime::invoke_chat(
                            root,
                            &provider,
                            &model,
                            &synthesis_prompt,
                            &active_messages,
                            &message,
                        ) {
                            let mut retried_text = clean_chat_text(
                                retried
                                    .get("response")
                                    .and_then(Value::as_str)
                                    .unwrap_or(""),
                                32_000,
                            );
                            retried_text = strip_internal_context_metadata_prefix(&retried_text);
                            retried_text = strip_internal_cache_control_markup(&retried_text);
                            if !user_requested_internal_runtime_details(&message) {
                                retried_text = abstract_runtime_mechanics_terms(&retried_text);
                            }
                            if !response_is_unrelated_context_dump(&message, &retried_text) {
                                let (retry_finalized, _retry_report, retry_outcome) =
                                    enforce_user_facing_finalization_contract(
                                        retried_text,
                                        &response_tools,
                                    );
                                if !response_is_no_findings_placeholder(&retry_finalized) {
                                    finalized_response = retry_finalized;
                                    finalization_outcome = merge_response_outcomes(
                                        &finalization_outcome,
                                        &format!("synthesis_retry:{retry_outcome}"),
                                        200,
                                    );
                                    synthesis_retry_used = true;
                                }
                            }
                        }
                    }
                    if response_is_no_findings_placeholder(&finalized_response)
                        && message_requests_comparative_answer(&message)
                    {
                        finalized_response = comparative_no_findings_fallback(&message);
                        finalization_outcome =
                            format!("{finalization_outcome}+comparative_fallback");
                    }
                    if response_tools.is_empty()
                        && !inline_tools_allowed
                        && response_is_no_findings_placeholder(&finalized_response)
                    {
                        let direct_chat_repair_prompt = clean_text(
                            &format!(
                                "{}\n\nConversational recovery: answer directly in natural language without tools. Do not mention missing findings unless the user explicitly requested a tool call.",
                                AGENT_RUNTIME_SYSTEM_PROMPT
                            ),
                            12_000,
                        );
                        if let Ok(retried) = crate::dashboard_provider_runtime::invoke_chat(
                            root,
                            &provider,
                            &model,
                            &direct_chat_repair_prompt,
                            &active_messages,
                            &message,
                        ) {
                            let mut retried_text = clean_chat_text(
                                retried
                                    .get("response")
                                    .and_then(Value::as_str)
                                    .unwrap_or(""),
                                32_000,
                            );
                            retried_text = strip_internal_context_metadata_prefix(&retried_text);
                            retried_text = strip_internal_cache_control_markup(&retried_text);
                            if !user_requested_internal_runtime_details(&message) {
                                retried_text = abstract_runtime_mechanics_terms(&retried_text);
                            }
                            if !response_is_unrelated_context_dump(&message, &retried_text) {
                                let (retry_finalized, _retry_report, retry_outcome) =
                                    enforce_user_facing_finalization_contract(
                                        retried_text,
                                        &response_tools,
                                    );
                                if !response_is_no_findings_placeholder(&retry_finalized) {
                                    finalized_response = retry_finalized;
                                    finalization_outcome = merge_response_outcomes(
                                        &finalization_outcome,
                                        &format!("conversation_retry:{retry_outcome}"),
                                        200,
                                    );
                                }
                            }
                        }
                        if response_is_no_findings_placeholder(&finalized_response) {
                            finalized_response =
                                "I can answer directly without tool calls. Ask your question naturally and I’ll respond conversationally unless you explicitly request a tool run.".to_string();
                            finalization_outcome =
                                format!("{finalization_outcome}+conversation_fallback");
                        }
                    }
                    let mut tooling_fallback_used = false;
                    if let Some(tooling_fallback) = maybe_tooling_failure_fallback(
                        &message,
                        &finalized_response,
                        &latest_assistant_message_text(&active_messages),
                    ) {
                        finalized_response = tooling_fallback;
                        finalization_outcome =
                            format!("{finalization_outcome}+tooling_failure_fallback");
                        tooling_fallback_used = true;
                    }
                    let (contract_finalized, contract_report, contract_outcome) =
                        enforce_user_facing_finalization_contract(
                            finalized_response,
                            &response_tools,
                        );
                    finalized_response = contract_finalized;
                    tool_completion = contract_report;
                    tool_completion =
                        enrich_tool_completion_receipt(tool_completion, &response_tools);
                    finalization_outcome =
                        merge_response_outcomes(&finalization_outcome, &contract_outcome, 200);
                    response_text = finalized_response;
                    if memory_recall_requested(&message)
                        && (response_is_no_findings_placeholder(&response_text)
                            || response_looks_like_tool_ack_without_findings(&response_text))
                    {
                        response_text = build_memory_recall_response(&state, &messages, &message);
                    }
                    let final_ack_only =
                        response_looks_like_tool_ack_without_findings(&response_text);
                    let response_finalization = json!({
                        "applied": finalization_outcome != "unchanged",
                        "outcome": finalization_outcome,
                        "initial_ack_only": initial_ack_only,
                        "final_ack_only": final_ack_only,
                        "findings_available": tool_completion
                            .get("findings_available")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                        "tool_completion": tool_completion,
                        "retry_attempted": retry_attempted,
                        "retry_used": retry_used,
                        "synthesis_retry_used": synthesis_retry_used,
                        "tooling_fallback_used": tooling_fallback_used
                    });
                    let turn_transaction =
                        crate::dashboard_tool_turn_loop::turn_transaction_payload(
                            "complete",
                            if response_tools.is_empty() {
                                "none"
                            } else {
                                "complete"
                            },
                            "complete",
                            "complete",
                        );
                    let mut turn_receipt =
                        append_turn_message(root, &agent_id, &message, &response_text);
                    turn_receipt["response_finalization"] = response_finalization.clone();
                    let runtime_model = clean_text(
                        result
                            .get("runtime_model")
                            .and_then(Value::as_str)
                            .unwrap_or(&model),
                        240,
                    );
                    let mut runtime_patch = json!({
                        "runtime_model": runtime_model,
                        "context_window": result.get("context_window").cloned().unwrap_or_else(|| json!(0)),
                        "context_window_tokens": result.get("context_window").cloned().unwrap_or_else(|| json!(0)),
                        "updated_at": crate::now_iso()
                    });
                    if auto_route.is_some() {
                        runtime_patch["runtime_provider"] = json!(provider.clone());
                        if !requested_provider.eq_ignore_ascii_case("auto")
                            && !requested_model.is_empty()
                            && !requested_model.eq_ignore_ascii_case("auto")
                        {
                            runtime_patch["model_provider"] = json!(provider.clone());
                            runtime_patch["model_name"] = json!(model.clone());
                            runtime_patch["model_override"] = json!(format!("{provider}/{model}"));
                        }
                    }
                    let _ = update_profile_patch(root, &agent_id, &runtime_patch);
                    let terminal_transcript = tool_terminal_transcript(&response_tools);
                    let mut payload = result.clone();
                    payload["ok"] = json!(true);
                    payload["agent_id"] = json!(agent_id);
                    payload["provider"] = json!(provider);
                    payload["model"] = json!(model);
                    payload["iterations"] = json!(1);
                    payload["response"] = json!(response_text);
                    payload["runtime_sync"] = runtime_summary;
                    payload["tools"] = Value::Array(response_tools);
                    payload["terminal_transcript"] = Value::Array(terminal_transcript);
                    payload["response_finalization"] = response_finalization;
                    payload["turn_transaction"] = turn_transaction;
                    payload["context_window"] = json!(fallback_window.max(0));
                    payload["context_tokens"] = json!(context_active_tokens.max(0));
                    payload["context_used_tokens"] = json!(context_active_tokens.max(0));
                    payload["context_ratio"] = json!(context_ratio);
                    payload["context_pressure"] = json!(context_pressure.clone());
                    payload["attention_queue"] = turn_receipt
                        .get("attention_queue")
                        .cloned()
                        .unwrap_or_else(|| json!({}));
                    payload["memory_capture"] = turn_receipt
                        .get("memory_capture")
                        .cloned()
                        .unwrap_or_else(|| json!({}));
                    payload["context_pool"] = json!({
                        "pool_limit_tokens": context_pool_limit_tokens,
                        "pool_tokens": context_pool_tokens,
                        "pool_messages": pooled_messages.len(),
                        "session_count": sessions_total,
                        "system_context_enabled": true,
                        "system_context_limit_tokens": context_pool_limit_tokens,
                        "llm_context_window_tokens": fallback_window.max(0),
                        "cross_session_memory_enabled": true,
                        "memory_kv_entries": memory_kv_entries,
                        "active_target_tokens": active_context_target_tokens,
                        "active_tokens": context_active_tokens,
                        "active_messages": active_messages.len(),
                        "min_recent_messages": active_context_min_recent,
                        "include_all_sessions_context": include_all_sessions_context,
                        "context_window": fallback_window.max(0),
                        "context_ratio": context_ratio,
                        "context_pressure": context_pressure,
                        "pre_generation_pruning_enabled": true,
                        "pre_generation_pruned": pre_generation_pruned,
                        "recent_floor_enforced": recent_floor_enforced,
                        "recent_floor_injected": recent_floor_injected,
                        "history_trim_confirmed": history_trim_confirmed,
                        "emergency_compact_enabled": true,
                        "emergency_compact": emergency_compact
                    });
                    payload["workspace_hints"] = json!(workspace_hints);
                    payload["latent_tool_candidates"] = json!(latent_tool_candidates);
                    if let Some(route) = auto_route {
                        payload["auto_route"] =
                            route.get("route").cloned().unwrap_or_else(|| route.clone());
                    }
                    if !virtual_key_id.is_empty() {
                        let spend_receipt =
                            crate::dashboard_provider_runtime::record_virtual_key_usage(
                                root,
                                &virtual_key_id,
                                payload
                                    .get("cost_usd")
                                    .and_then(Value::as_f64)
                                    .unwrap_or(0.0),
                            );
                        payload["virtual_key"] = json!({
                            "id": virtual_key_id,
                            "reservation": virtual_key_gate,
                            "spend": spend_receipt
                        });
                    }
                    return Some(CompatApiResponse {
                        status: 200,
                        payload,
                    });
                }
                Err(err) => {
                    return Some(CompatApiResponse {
                        status: 502,
                        payload: json!({
                            "ok": false,
                            "agent_id": agent_id,
                            "error": clean_text(&err, 280),
                            "provider": provider,
                            "model": model
                        }),
                    });
                }
            }
        }

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
                let tool_completion =
                    enrich_tool_completion_receipt(tool_completion, &response_tools);
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
            let explicit_role =
                clean_text(patch.get("role").and_then(Value::as_str).unwrap_or(""), 60);
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
                let preserve_default_name_for_self_named_models =
                    selected_model_supports_self_naming(
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
                    let auto_name =
                        dashboard_compat_api_agent_identity::resolve_post_init_agent_name(
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
                    let vibe =
                        clean_text(patch.get("vibe").and_then(Value::as_str).unwrap_or(""), 80);
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
            let _ = update_profile_patch(root, &agent_id, &patch);
            if patch.get("contract").map(Value::is_object).unwrap_or(false) {
                let _ = upsert_contract_patch(
                    root,
                    &agent_id,
                    patch.get("contract").unwrap_or(&json!({})),
                );
            } else if patch.get("expiry_seconds").is_some()
                || patch.get("termination_condition").is_some()
                || patch.get("auto_terminate_allowed").is_some()
                || patch.get("idle_terminate_allowed").is_some()
            {
                let _ = upsert_contract_patch(root, &agent_id, &patch);
            }
            if should_seed_intro {
                let intro_name = clean_text(
                    patch
                        .get("name")
                        .and_then(Value::as_str)
                        .or_else(|| {
                            existing
                                .as_ref()
                                .and_then(|row| row.get("name").and_then(Value::as_str))
                        })
                        .unwrap_or(&agent_id),
                    120,
                );
                let _ = crate::dashboard_agent_state::seed_intro_message(
                    root,
                    &agent_id,
                    &resolved_role,
                    &intro_name,
                );
            }
            let row = agent_row_by_id(root, snapshot, &agent_id)
                .unwrap_or_else(|| json!({"id": agent_id}));
            let mut payload = json!({"ok": true, "agent_id": agent_id, "agent": row});
            if let Some(notice) = rename_notice {
                payload["rename_notice"] = notice;
            }
            return Some(CompatApiResponse {
                status: 200,
                payload,
            });
        }

        if method == "PUT" && segments.len() == 1 && segments[0] == "model" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let requested = clean_text(
                request.get("model").and_then(Value::as_str).unwrap_or(""),
                200,
            );
            if requested.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "model_required"}),
                });
            }
            let (default_provider, default_model) = effective_app_settings(root, snapshot);
            let (provider, model) = split_model_ref(&requested, &default_provider, &default_model);
            let _ = update_profile_patch(
                root,
                &agent_id,
                &json!({
                    "model_override": format!("{provider}/{model}"),
                    "model_provider": provider,
                    "model_name": model,
                    "runtime_model": model
                }),
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "agent_id": agent_id,
                    "provider": provider,
                    "model": model,
                    "runtime_model": model
                }),
            });
        }

        if method == "PUT" && segments.len() == 1 && segments[0] == "mode" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let mode = clean_text(
                request.get("mode").and_then(Value::as_str).unwrap_or(""),
                40,
            );
            let _ = update_profile_patch(
                root,
                &agent_id,
                &json!({"mode": mode, "updated_at": crate::now_iso()}),
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": agent_id, "mode": mode}),
            });
        }

        if method == "GET" && segments.len() == 1 && segments[0] == "git-trees" {
            return Some(CompatApiResponse {
                status: 200,
                payload: git_tree_payload_for_agent(root, snapshot, &agent_id),
            });
        }

        if method == "POST"
            && segments.len() == 2
            && segments[0] == "git-tree"
            && segments[1] == "switch"
        {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let branch = clean_text(
                request.get("branch").and_then(Value::as_str).unwrap_or(""),
                180,
            );
            if branch.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "branch_required"}),
                });
            }
            let require_new = request
                .get("require_new")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let result = crate::dashboard_git_runtime::switch_agent_worktree(
                root,
                &agent_id,
                &branch,
                require_new,
            );
            let kind = clean_text(
                result
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or("isolated"),
                40,
            );
            let default_workspace_dir = root.to_string_lossy().to_string();
            let workspace_dir = clean_text(
                result
                    .get("workspace_dir")
                    .and_then(Value::as_str)
                    .unwrap_or(default_workspace_dir.as_str()),
                4000,
            );
            let workspace_rel = clean_text(
                result
                    .get("workspace_rel")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                4000,
            );
            let ready = result
                .get("ready")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let error = clean_text(
                result.get("error").and_then(Value::as_str).unwrap_or(""),
                280,
            );
            let _ = update_profile_patch(
                root,
                &agent_id,
                &json!({
                    "git_branch": clean_text(result.get("branch").and_then(Value::as_str).unwrap_or(&branch), 180),
                    "git_tree_kind": kind,
                    "workspace_dir": workspace_dir,
                    "workspace_rel": workspace_rel,
                    "git_tree_ready": ready,
                    "git_tree_error": error,
                    "updated_at": crate::now_iso()
                }),
            );
            return Some(CompatApiResponse {
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    200
                } else {
                    400
                },
                payload: git_tree_payload_for_agent(root, snapshot, &agent_id),
            });
        }

        if method == "POST" && segments.len() == 2 && segments[0] == "file" && segments[1] == "read"
        {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let requested_path = clean_text(
                request
                    .get("path")
                    .and_then(Value::as_str)
                    .or_else(|| request.get("file_path").and_then(Value::as_str))
                    .unwrap_or(""),
                4000,
            );
            if requested_path.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "path_required"}),
                });
            }
            let nexus_connection =
                match crate::dashboard_tool_turn_loop::authorize_ingress_tool_call_with_nexus(
                    "file_read",
                ) {
                    Ok(meta) => meta,
                    Err(err) => {
                        return Some(CompatApiResponse {
                            status: 403,
                            payload: json!({
                                "ok": false,
                                "error": "file_read_nexus_delivery_denied",
                                "message": "File read blocked by hierarchical nexus ingress policy.",
                                "nexus_error": clean_text(&err, 240)
                            }),
                        })
                    }
                };
            let workspace_base = workspace_base_for_agent(root, existing.as_ref());
            let target = resolve_workspace_path(&workspace_base, &requested_path);
            let Some(target_path) = target else {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "path_outside_workspace", "path": requested_path}),
                });
            };
            if !target_path.is_file() {
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({
                        "ok": false,
                        "error": "file_not_found",
                        "file": {"ok": false, "path": target_path.to_string_lossy().to_string()}
                    }),
                });
            }
            let bytes = fs::read(&target_path).unwrap_or_default();
            let full = request
                .get("full")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let allow_binary = request
                .get("allow_binary")
                .or_else(|| request.get("binary"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let max_bytes = if full {
                bytes.len().max(1)
            } else {
                request
                    .get("max_bytes")
                    .and_then(Value::as_u64)
                    .unwrap_or((256 * 1024) as u64)
                    .clamp(1, (8 * 1024 * 1024) as u64) as usize
            };
            let binary = bytes_look_binary(&bytes);
            let content_type = guess_mime_type_for_file(&target_path, &bytes);
            if binary && !allow_binary {
                return Some(CompatApiResponse {
                    status: 415,
                    payload: json!({
                        "ok": false,
                        "error": "binary_file_requires_opt_in",
                        "file": {
                            "ok": false,
                            "path": target_path.to_string_lossy().to_string(),
                            "bytes": bytes.len(),
                            "binary": true,
                            "content_type": content_type,
                            "file_name": clean_text(
                                target_path.file_name().and_then(|v| v.to_str()).unwrap_or("download.bin"),
                                180
                            )
                        }
                    }),
                });
            }
            let (content, truncated) = if binary {
                (String::new(), bytes.len() > max_bytes)
            } else {
                truncate_utf8_lossy(&bytes, max_bytes)
            };
            let content_base64 = if binary {
                use base64::engine::general_purpose::STANDARD;
                use base64::Engine;
                let slice_end = bytes.len().min(max_bytes.max(1));
                STANDARD.encode(&bytes[..slice_end])
            } else {
                String::new()
            };
            let download_url = if bytes.len() <= (2 * 1024 * 1024) {
                data_url_from_bytes(&bytes, &content_type)
            } else {
                String::new()
            };
            let file_name = clean_text(
                target_path
                    .file_name()
                    .and_then(|v| v.to_str())
                    .unwrap_or("download.txt"),
                180,
            );
            let mut payload = json!({
                "ok": true,
                "file": {
                    "ok": true,
                    "path": target_path.to_string_lossy().to_string(),
                    "content": content,
                    "content_base64": content_base64,
                    "truncated": truncated,
                    "bytes": bytes.len(),
                    "max_bytes": max_bytes,
                    "full": full,
                    "binary": binary,
                    "allow_binary": allow_binary,
                    "download_url": download_url,
                    "file_name": file_name,
                    "content_type": content_type
                }
            });
            if let Some(meta) = nexus_connection {
                payload["nexus_connection"] = meta;
            }
            let trace_id = crate::deterministic_receipt_hash(&json!({
                "agent_id": agent_id,
                "tool": "file_read",
                "path": requested_path
            }));
            let task_id = format!(
                "tool-file-read-{}",
                trace_id.chars().take(12).collect::<String>()
            );
            let pipeline = tooling_pipeline_execute(
                &trace_id,
                &task_id,
                "file_read",
                &json!({
                    "path": requested_path,
                    "full": full,
                    "allow_binary": allow_binary
                }),
                |_| Ok(payload.clone()),
            );
            if pipeline.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                attach_tool_pipeline(&mut payload, &pipeline);
            }
            return Some(CompatApiResponse {
                status: 200,
                payload,
            });
        }

        if method == "POST"
            && segments.len() == 2
            && segments[0] == "file"
            && segments[1] == "read-many"
        {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let mut paths = request
                .get("paths")
                .or_else(|| request.get("sources"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .filter_map(|row| row.as_str().map(|v| clean_text(v, 4000)))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>();
            if paths.is_empty() {
                let single = clean_text(
                    request
                        .get("path")
                        .and_then(Value::as_str)
                        .or_else(|| request.get("file_path").and_then(Value::as_str))
                        .unwrap_or(""),
                    4000,
                );
                if !single.is_empty() {
                    paths.push(single);
                }
            }
            if paths.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "paths_required"}),
                });
            }
            let nexus_connection =
                match crate::dashboard_tool_turn_loop::authorize_ingress_tool_call_with_nexus(
                    "file_read_many",
                ) {
                    Ok(meta) => meta,
                    Err(err) => {
                        return Some(CompatApiResponse {
                            status: 403,
                            payload: json!({
                                "ok": false,
                                "error": "file_read_many_nexus_delivery_denied",
                                "message": "File read-many blocked by hierarchical nexus ingress policy.",
                                "nexus_error": clean_text(&err, 240)
                            }),
                        })
                    }
                };
            let workspace_base = workspace_base_for_agent(root, existing.as_ref());
            let full = request
                .get("full")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let allow_binary = request
                .get("allow_binary")
                .or_else(|| request.get("binary"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let max_bytes = request
                .get("max_bytes")
                .and_then(Value::as_u64)
                .unwrap_or((256 * 1024) as u64)
                .clamp(1, (8 * 1024 * 1024) as u64) as usize;
            let mut files = Vec::<Value>::new();
            let mut failed = Vec::<Value>::new();
            let mut unclassified = Vec::<Value>::new();
            let mut grouped_text = Vec::<String>::new();
            let mut grouped_binary = Vec::<String>::new();
            let mut grouped_unclassified = Vec::<String>::new();
            for requested_path in &paths {
                let target = resolve_workspace_path(&workspace_base, requested_path);
                let Some(target_path) = target else {
                    failed.push(json!({
                        "path": requested_path,
                        "error": "path_outside_workspace",
                        "status": 400
                    }));
                    grouped_unclassified.push(requested_path.clone());
                    continue;
                };
                if !target_path.is_file() {
                    let rendered = target_path.to_string_lossy().to_string();
                    unclassified.push(json!({
                        "path": rendered,
                        "error": "file_not_found",
                        "status": 404
                    }));
                    grouped_unclassified.push(target_path.to_string_lossy().to_string());
                    continue;
                }
                let bytes = fs::read(&target_path).unwrap_or_default();
                let file_max_bytes = if full { bytes.len().max(1) } else { max_bytes };
                let binary = bytes_look_binary(&bytes);
                let content_type = guess_mime_type_for_file(&target_path, &bytes);
                if binary && !allow_binary {
                    failed.push(json!({
                        "path": target_path.to_string_lossy().to_string(),
                        "error": "binary_file_requires_opt_in",
                        "status": 415,
                        "binary": true,
                        "bytes": bytes.len(),
                        "content_type": content_type
                    }));
                    grouped_binary.push(target_path.to_string_lossy().to_string());
                    continue;
                }
                let (content, truncated) = if binary {
                    (String::new(), bytes.len() > file_max_bytes)
                } else {
                    truncate_utf8_lossy(&bytes, file_max_bytes)
                };
                let content_base64 = if binary {
                    use base64::engine::general_purpose::STANDARD;
                    use base64::Engine;
                    let slice_end = bytes.len().min(file_max_bytes.max(1));
                    STANDARD.encode(&bytes[..slice_end])
                } else {
                    String::new()
                };
                let download_url = if bytes.len() <= (2 * 1024 * 1024) {
                    data_url_from_bytes(&bytes, &content_type)
                } else {
                    String::new()
                };
                let file_name = clean_text(
                    target_path
                        .file_name()
                        .and_then(|v| v.to_str())
                        .unwrap_or("download.txt"),
                    180,
                );
                let rendered_path = target_path.to_string_lossy().to_string();
                if binary {
                    grouped_binary.push(rendered_path.clone());
                } else {
                    grouped_text.push(rendered_path.clone());
                }
                files.push(json!({
                    "ok": true,
                    "path": rendered_path,
                    "content": content,
                    "content_base64": content_base64,
                    "truncated": truncated,
                    "bytes": bytes.len(),
                    "max_bytes": file_max_bytes,
                    "full": full,
                    "binary": binary,
                    "allow_binary": allow_binary,
                    "download_url": download_url,
                    "file_name": file_name,
                    "content_type": content_type
                }));
            }
            let ok = !files.is_empty();
            let status = if ok {
                200
            } else {
                failed
                    .first()
                    .or_else(|| unclassified.first())
                    .and_then(|row| row.get("status").and_then(Value::as_u64))
                    .unwrap_or(400) as u16
            };
            let mut payload = json!({
                "ok": ok,
                "type": "file_read_many",
                "files": files,
                "failed": failed,
                "unclassified": unclassified,
                "partial": ok && (!failed.is_empty() || !unclassified.is_empty()),
                "groups": {
                    "text": grouped_text,
                    "binary": grouped_binary,
                    "unclassified": grouped_unclassified
                },
                "counts": {
                    "requested": paths.len(),
                    "ok": files.len(),
                    "failed": failed.len(),
                    "unclassified": unclassified.len(),
                    "text": grouped_text.len(),
                    "binary": grouped_binary.len(),
                    "group_unclassified": grouped_unclassified.len()
                }
            });
            if let Some(meta) = nexus_connection {
                payload["nexus_connection"] = meta;
            }
            let trace_id = crate::deterministic_receipt_hash(&json!({
                "agent_id": agent_id,
                "tool": "file_read_many",
                "paths": paths
            }));
            let task_id = format!(
                "tool-file-read-many-{}",
                trace_id.chars().take(12).collect::<String>()
            );
            let pipeline = tooling_pipeline_execute(
                &trace_id,
                &task_id,
                "file_read_many",
                &json!({
                    "paths": paths,
                    "full": full,
                    "allow_binary": allow_binary
                }),
                |_| Ok(payload.clone()),
            );
            if pipeline.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                attach_tool_pipeline(&mut payload, &pipeline);
            }
            return Some(CompatApiResponse { status, payload });
        }

        if method == "POST"
            && segments.len() == 2
            && segments[0] == "folder"
            && segments[1] == "export"
        {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let requested_path = clean_text(
                request
                    .get("path")
                    .and_then(Value::as_str)
                    .or_else(|| request.get("folder").and_then(Value::as_str))
                    .unwrap_or(""),
                4000,
            );
            if requested_path.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "path_required"}),
                });
            }
            let workspace_base = workspace_base_for_agent(root, existing.as_ref());
            let target = resolve_workspace_path(&workspace_base, &requested_path);
            let Some(target_path) = target else {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "path_outside_workspace", "path": requested_path}),
                });
            };
            if !target_path.is_dir() {
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({
                        "ok": false,
                        "error": "folder_not_found",
                        "folder": {"ok": false, "path": target_path.to_string_lossy().to_string()}
                    }),
                });
            }
            let full = request
                .get("full")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let max_entries = if full {
                1_000_000usize
            } else {
                request
                    .get("max_entries")
                    .and_then(Value::as_u64)
                    .unwrap_or(20_000)
                    .clamp(1, 100_000) as usize
            };
            let mut lines = Vec::<String>::new();
            let root_name = clean_text(
                target_path
                    .file_name()
                    .and_then(|v| v.to_str())
                    .unwrap_or("folder"),
                180,
            );
            lines.push(format!("[d] {root_name}"));
            let mut entries = 0usize;
            let mut truncated = false;
            for entry in WalkDir::new(&target_path)
                .follow_links(false)
                .sort_by_file_name()
            {
                let Ok(row) = entry else {
                    continue;
                };
                let path = row.path();
                if path == target_path {
                    continue;
                }
                entries += 1;
                if entries > max_entries {
                    truncated = true;
                    continue;
                }
                let rel = path.strip_prefix(&target_path).unwrap_or(path);
                let rel_name =
                    clean_text(rel.file_name().and_then(|v| v.to_str()).unwrap_or(""), 240);
                if rel_name.is_empty() {
                    continue;
                }
                let depth = rel.components().count().saturating_sub(1).min(32);
                let indent = "  ".repeat(depth + 1);
                let marker = if row.file_type().is_dir() { "[d]" } else { "-" };
                lines.push(format!("{indent}{marker} {rel_name}"));
            }
            let tree = lines.join("\n");
            let archive_name = if root_name.is_empty() {
                "folder-tree.txt".to_string()
            } else {
                format!("{root_name}-tree.txt")
            };
            let tree_bytes = tree.as_bytes().len();
            let download_url = if tree_bytes > 0 && tree_bytes <= (2 * 1024 * 1024) {
                data_url_from_bytes(tree.as_bytes(), "text/plain; charset=utf-8")
            } else {
                String::new()
            };
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "folder": {
                        "ok": true,
                        "path": target_path.to_string_lossy().to_string(),
                        "tree": tree,
                        "entries": entries,
                        "truncated": truncated,
                        "full": full,
                        "max_entries": max_entries
                    },
                    "archive": {
                        "ok": true,
                        "download_url": download_url,
                        "file_name": archive_name,
                        "bytes": tree_bytes
                    }
                }),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "terminal" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let command = clean_text(
                request
                    .get("command")
                    .and_then(Value::as_str)
                    .or_else(|| request.get("cmd").and_then(Value::as_str))
                    .unwrap_or(""),
                16_000,
            );
            if command.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "command_required"}),
                });
            }
            let workspace_base = workspace_base_for_agent(root, existing.as_ref());
            let requested_cwd = clean_text(
                request.get("cwd").and_then(Value::as_str).unwrap_or(""),
                4000,
            );
            let cwd = if requested_cwd.is_empty() {
                workspace_base.clone()
            } else {
                resolve_workspace_path(&workspace_base, &requested_cwd)
                    .unwrap_or(workspace_base.clone())
            };
            let session_id = format!("agent-{}", clean_agent_id(&agent_id));
            let _ = crate::dashboard_terminal_broker::create_session(
                root,
                &json!({
                    "id": session_id,
                    "cwd": workspace_base.to_string_lossy().to_string()
                }),
            );
            let payload = crate::dashboard_terminal_broker::exec_command(
                root,
                &json!({
                    "session_id": session_id,
                    "command": command,
                    "cwd": cwd.to_string_lossy().to_string()
                }),
            );
            let status = match payload.get("error").and_then(Value::as_str).unwrap_or("") {
                "session_id_and_command_required"
                | "session_not_found"
                | "cwd_outside_workspace" => 400,
                _ => 200,
            };
            return Some(CompatApiResponse { status, payload });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "upload" {
            let file_name = clean_text(
                header_value(headers, "X-Filename")
                    .as_deref()
                    .unwrap_or("upload.bin"),
                240,
            );
            let content_type = clean_text(
                header_value(headers, "Content-Type")
                    .as_deref()
                    .unwrap_or("application/octet-stream"),
                120,
            );
            let workspace_base = workspace_base_for_agent(root, existing.as_ref());
            let uploads_dir = workspace_base.join(".infring").join("uploads");
            let _ = fs::create_dir_all(&uploads_dir);
            let file_id = format!(
                "upload-{}",
                crate::deterministic_receipt_hash(&json!({
                    "agent_id": agent_id,
                    "filename": file_name,
                    "bytes": body.len(),
                    "ts": crate::now_iso()
                }))
                .chars()
                .take(16)
                .collect::<String>()
            );
            let ext = Path::new(&file_name)
                .extension()
                .and_then(|v| v.to_str())
                .map(|v| clean_text(v, 16))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "bin".to_string());
            let stored_name = format!("{file_id}.{ext}");
            let stored_path = uploads_dir.join(&stored_name);
            let _ = fs::write(&stored_path, body);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "file_id": file_id,
                    "filename": file_name,
                    "content_type": content_type,
                    "bytes": body.len(),
                    "stored_path": stored_path.to_string_lossy().to_string(),
                    "uploaded_at": crate::now_iso()
                }),
            });
        }

        if method == "GET" && segments.len() == 1 && segments[0] == "files" {
            let dir = agent_files_dir(root, &agent_id);
            let mut rows = Vec::<Value>::new();
            let defaults = vec!["SOUL.md".to_string(), "SYSTEM.md".to_string()];
            for name in defaults {
                let path = dir.join(&name);
                rows.push(json!({
                    "name": name,
                    "exists": path.exists(),
                    "size": fs::metadata(&path).ok().map(|m| m.len()).unwrap_or(0)
                }));
            }
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }
                    let name =
                        clean_text(path.file_name().and_then(|v| v.to_str()).unwrap_or(""), 180);
                    if name.is_empty() {
                        continue;
                    }
                    if rows
                        .iter()
                        .any(|row| row.get("name").and_then(Value::as_str) == Some(name.as_str()))
                    {
                        continue;
                    }
                    rows.push(json!({
                        "name": name,
                        "exists": true,
                        "size": fs::metadata(&path).ok().map(|m| m.len()).unwrap_or(0)
                    }));
                }
            }
            rows.sort_by(|a, b| {
                clean_text(a.get("name").and_then(Value::as_str).unwrap_or(""), 180).cmp(
                    &clean_text(b.get("name").and_then(Value::as_str).unwrap_or(""), 180),
                )
            });
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": agent_id, "files": rows}),
            });
        }

        if (method == "GET" || method == "PUT") && segments.len() >= 2 && segments[0] == "files" {
            let file_name = decode_path_segment(&segments[1..].join("/"));
            if file_name.is_empty() || file_name.contains("..") {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "invalid_file_name"}),
                });
            }
            let path = agent_files_dir(root, &agent_id).join(&file_name);
            if method == "GET" {
                if !path.exists() {
                    return Some(CompatApiResponse {
                        status: 404,
                        payload: json!({"ok": false, "error": "file_not_found"}),
                    });
                }
                let content = fs::read_to_string(&path).unwrap_or_default();
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({"ok": true, "agent_id": agent_id, "name": file_name, "content": content}),
                });
            }
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let content = request
                .get("content")
                .and_then(Value::as_str)
                .map(|v| v.to_string())
                .unwrap_or_default();
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&path, content);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": agent_id, "name": file_name}),
            });
        }

        if method == "GET" && segments.len() == 1 && segments[0] == "tools" {
            let payload = read_json_loose(&agent_tools_path(root, &agent_id))
                .unwrap_or_else(|| json!({"tool_allowlist": [], "tool_blocklist": []}));
            return Some(CompatApiResponse {
                status: 200,
                payload,
            });
        }

        if method == "PUT" && segments.len() == 1 && segments[0] == "tools" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let payload = json!({
                "tool_allowlist": request.get("tool_allowlist").cloned().unwrap_or_else(|| json!([])),
                "tool_blocklist": request.get("tool_blocklist").cloned().unwrap_or_else(|| json!([]))
            });
            write_json_pretty(&agent_tools_path(root, &agent_id), &payload);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": agent_id, "tool_filters": payload}),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "clone" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let source = existing.unwrap_or_else(|| json!({}));
            let requested_new_name = clean_text(
                request
                    .get("new_name")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120,
            );
            let source_role = clean_text(
                source
                    .get("role")
                    .and_then(Value::as_str)
                    .unwrap_or("analyst"),
                60,
            );
            let resolved_requested_name = if requested_new_name.is_empty() {
                dashboard_compat_api_agent_identity::resolve_agent_name(root, "", &source_role)
            } else {
                requested_new_name.clone()
            };
            let new_id_seed = if resolved_requested_name.is_empty() {
                "agent".to_string()
            } else {
                resolved_requested_name.clone()
            };
            let new_id = make_agent_id(root, &new_id_seed);
            let new_name = if resolved_requested_name.is_empty() {
                dashboard_compat_api_agent_identity::default_agent_name(&new_id)
            } else {
                resolved_requested_name
            };
            let mut profile_patch = source.clone();
            profile_patch["name"] = Value::String(new_name.clone());
            profile_patch["agent_id"] = Value::String(new_id.clone());
            profile_patch["parent_agent_id"] = Value::String(agent_id.clone());
            profile_patch["state"] = Value::String("Running".to_string());
            if requested_new_name.is_empty() {
                profile_patch["identity"] =
                    dashboard_compat_api_agent_identity::resolve_agent_identity(
                        root,
                        &json!({}),
                        &source_role,
                    );
            }
            profile_patch["created_at"] = Value::String(crate::now_iso());
            profile_patch["updated_at"] = Value::String(crate::now_iso());
            let _ = update_profile_patch(root, &new_id, &profile_patch);
            let _ = upsert_contract_patch(
                root,
                &new_id,
                &json!({
                    "status": "active",
                    "created_at": crate::now_iso(),
                    "updated_at": crate::now_iso(),
                    "owner": "dashboard_clone",
                    "mission": format!("Assist with assigned mission for {}.", new_id),
                    "parent_agent_id": agent_id,
                    "termination_condition": "task_or_timeout",
                    "expiry_seconds": 3600,
                    "auto_terminate_allowed": false,
                    "idle_terminate_allowed": false
                }),
            );
            append_turn_message(root, &new_id, "", "");
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": new_id, "name": new_name}),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "avatar" {
            let content_type = clean_text(
                query_value(path, "content_type").as_deref().unwrap_or(""),
                120,
            );
            let inferred = if content_type.is_empty() {
                "image/png".to_string()
            } else {
                content_type
            };
            let encoded = {
                use base64::engine::general_purpose::STANDARD;
                use base64::Engine;
                STANDARD.encode(body)
            };
            let avatar_url = format!("data:{};base64,{}", inferred, encoded);
            let _ = update_profile_patch(root, &agent_id, &json!({"avatar_url": avatar_url}));
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": agent_id, "avatar_url": avatar_url}),
            });
        }
    }

    let usage = usage_from_state(root, snapshot);
    let runtime = runtime_sync_summary(snapshot);
    let alerts_count = parse_non_negative_i64(snapshot.pointer("/health/alerts/count"), 0);
    let status =
        if snapshot.get("ok").and_then(Value::as_bool).unwrap_or(false) && alerts_count == 0 {
            "healthy"
        } else if snapshot.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            "degraded"
        } else {
            "critical"
        };

    if method == "GET" && path_only == "/api/receipts/lineage" {
        let task_id = clean_text(
            query_value(path, "task_id")
                .or_else(|| query_value(path, "taskId"))
                .as_deref()
                .unwrap_or(""),
            180,
        );
        if task_id.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({
                    "ok": false,
                    "error": "task_id_required"
                }),
            });
        }
        let trace_id = clean_text(
            query_value(path, "trace_id")
                .or_else(|| query_value(path, "traceId"))
                .as_deref()
                .unwrap_or(""),
            180,
        );
        let trace_opt = if trace_id.is_empty() {
            None
        } else {
            Some(trace_id.as_str())
        };
        let limit = query_value(path, "limit")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(4000)
            .clamp(1, 50_000);
        let scan_root = clean_text(
            query_value(path, "scan_root")
                .or_else(|| query_value(path, "scanRoot"))
                .as_deref()
                .unwrap_or(""),
            500,
        );
        let scan_root_path = if scan_root.is_empty() {
            None
        } else {
            let candidate = PathBuf::from(scan_root);
            Some(if candidate.is_absolute() {
                candidate
            } else {
                root.join(candidate)
            })
        };
        let payload = match crate::action_receipts_kernel::query_task_lineage(
            root,
            &task_id,
            trace_opt,
            limit,
            scan_root_path.as_deref(),
        ) {
            Ok(out) => out,
            Err(err) => {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({
                        "ok": false,
                        "error": clean_text(&err, 240)
                    }),
                })
            }
        };
        return Some(CompatApiResponse {
            status: 200,
            payload,
        });
    }

    if method == "GET" {
        let payload = match path_only {
            "/api/health" => json!({
                "ok": true,
                "status": status,
                "checks": snapshot.pointer("/health/checks").cloned().unwrap_or_else(|| json!({})),
                "alerts": snapshot.pointer("/health/alerts").cloned().unwrap_or_else(|| json!({"count": 0, "checks": []})),
                "dashboard_metrics": snapshot.pointer("/health/dashboard_metrics").cloned().unwrap_or_else(|| json!({})),
                "runtime_sync": runtime,
                "receipt_hash": snapshot.get("receipt_hash").cloned().unwrap_or(Value::Null),
                "ts": crate::now_iso()
            }),
            "/api/usage" => {
                json!({"ok": true, "agents": usage["agents"].clone(), "summary": usage["summary"].clone(), "by_model": usage["models"].clone(), "daily": usage["daily"].clone()})
            }
            "/api/usage/summary" => {
                let mut summary = usage["summary"].clone();
                summary["ok"] = json!(true);
                summary
            }
            "/api/usage/by-model" => json!({"ok": true, "models": usage["models"].clone()}),
            "/api/usage/daily" => json!({
                "ok": true,
                "days": usage["daily"].clone(),
                "today_cost_usd": usage["today_cost_usd"].clone(),
                "first_event_date": usage["first_event_date"].clone()
            }),
            "/api/status" => status_payload(root, snapshot, &request_host),
            "/api/web/status" => crate::web_conduit::api_status(root),
            "/api/web/receipts" => {
                let limit = query_value(path, "limit")
                    .and_then(|raw| raw.parse::<usize>().ok())
                    .unwrap_or(20)
                    .clamp(1, 200);
                crate::web_conduit::api_receipts(root, limit)
            }
            "/api/web/search" => {
                let nexus_connection =
                    match crate::dashboard_tool_turn_loop::authorize_ingress_tool_call_with_nexus(
                        "web_search",
                    ) {
                        Ok(meta) => meta,
                        Err(err) => {
                            return Some(CompatApiResponse {
                                status: 403,
                                payload: json!({
                                    "ok": false,
                                    "error": "web_search_nexus_delivery_denied",
                                    "message": "Web search blocked by hierarchical nexus ingress policy.",
                                    "nexus_error": clean_text(&err, 240)
                                }),
                            })
                        }
                    };
                let query = clean_text(
                    query_value(path, "q")
                        .or_else(|| query_value(path, "query"))
                        .as_deref()
                        .unwrap_or(""),
                    600,
                );
                let args = json!({"query": query, "summary_only": false});
                let trace_id = crate::deterministic_receipt_hash(&json!({
                    "tool": "web_search",
                    "query": args.get("query").cloned().unwrap_or(Value::Null),
                    "route": "api_web_search_get"
                }));
                let task_id = format!(
                    "tool-web-search-{}",
                    trace_id.chars().take(12).collect::<String>()
                );
                let pipeline = tooling_pipeline_execute(
                    &trace_id,
                    &task_id,
                    "web_search",
                    &args,
                    |normalized_args| Ok(crate::web_conduit::api_search(root, normalized_args)),
                );
                let mut payload = pipeline
                    .get("raw_payload")
                    .cloned()
                    .unwrap_or_else(|| json!({"ok": false, "error": "tool_pipeline_failed"}));
                if pipeline.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    attach_tool_pipeline(&mut payload, &pipeline);
                }
                if let Some(meta) = nexus_connection {
                    if let Some(obj) = payload.as_object_mut() {
                        obj.insert("nexus_connection".to_string(), meta);
                    }
                }
                payload
            }
            "/api/batch-query" => {
                let source =
                    clean_text(query_value(path, "source").as_deref().unwrap_or("web"), 40);
                let query = clean_text(
                    query_value(path, "q")
                        .or_else(|| query_value(path, "query"))
                        .as_deref()
                        .unwrap_or(""),
                    600,
                );
                let aperture = clean_text(
                    query_value(path, "aperture").as_deref().unwrap_or("medium"),
                    20,
                );
                let args = json!({
                    "source": source,
                    "query": query,
                    "aperture": aperture
                });
                let trace_id = crate::deterministic_receipt_hash(&json!({
                    "tool": "batch_query",
                    "query": args.get("query").cloned().unwrap_or(Value::Null),
                    "route": "api_batch_query_get"
                }));
                let task_id = format!(
                    "tool-batch-query-{}",
                    trace_id.chars().take(12).collect::<String>()
                );
                let pipeline = tooling_pipeline_execute(
                    &trace_id,
                    &task_id,
                    "batch_query",
                    &args,
                    |normalized_args| {
                        Ok(crate::batch_query_primitive::api_batch_query(
                            root,
                            normalized_args,
                        ))
                    },
                );
                let mut payload = pipeline
                    .get("raw_payload")
                    .cloned()
                    .unwrap_or_else(|| json!({"status":"blocked","error":"tool_pipeline_failed"}));
                if pipeline.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    attach_tool_pipeline(&mut payload, &pipeline);
                }
                payload
            }
            "/api/telemetry/alerts" => proactive_telemetry_alerts_payload(root, snapshot),
            "/api/continuity" | "/api/continuity/pending" => {
                continuity_pending_payload(root, snapshot)
            }
            "/api/config" => config_payload(root, snapshot),
            "/api/config/schema" => config_schema_payload(),
            "/api/auth/check" => auth_check_payload(),
            "/api/providers" => providers_payload(root, snapshot),
            "/api/models" => crate::dashboard_model_catalog::catalog_payload(root, snapshot),
            "/api/models/recommended" => crate::dashboard_model_catalog::route_decision_payload(
                root,
                snapshot,
                &json!({"task_type":"general","budget_mode":"balanced"}),
            ),
            "/api/route/auto" => crate::dashboard_model_catalog::route_decision_payload(
                root,
                snapshot,
                &json!({"task_type":"general","budget_mode":"balanced"}),
            ),
            "/api/route/decision" => {
                crate::dashboard_model_catalog::route_decision_payload(root, snapshot, &json!({}))
            }
            "/api/channels" => dashboard_compat_api_channels::channels_payload(root),
            "/api/audit/recent" => {
                let entries = recent_audit_entries(root, snapshot);
                let tip_hash = crate::deterministic_receipt_hash(&json!({"entries": entries}));
                json!({"ok": true, "entries": entries, "tip_hash": tip_hash})
            }
            "/api/audit/decisions" => {
                let limit = query_value(path, "limit")
                    .and_then(|raw| raw.parse::<usize>().ok())
                    .unwrap_or(20)
                    .clamp(1, 200);
                let rows = read_jsonl_loose(&tool_decision_audit_path(root), limit);
                let tip_hash = crate::deterministic_receipt_hash(&json!({"rows": rows}));
                json!({"ok": true, "type": "tool_decision_audit_rows", "rows": rows, "tip_hash": tip_hash})
            }
            "/api/audit/verify" => {
                let entries = recent_audit_entries(root, snapshot);
                let tip_hash = crate::deterministic_receipt_hash(&json!({"entries": entries}));
                json!({"ok": true, "valid": true, "entries": entries.len(), "tip_hash": tip_hash})
            }
            "/api/version" => {
                let version = read_json(&root.join("package.json"))
                    .and_then(|v| v.get("version").and_then(Value::as_str).map(str::to_string))
                    .unwrap_or_else(|| "0.0.0".to_string());
                json!({
                    "ok": true,
                    "version": version,
                    "rust_authority": "rust_core_lanes",
                    "platform": std::env::consts::OS,
                    "arch": std::env::consts::ARCH
                })
            }
            "/api/security" => json!({
                "ok": true,
                "mode": "strict",
                "fail_closed": true,
                "receipts_required": true,
                "checks": snapshot.pointer("/health/checks").cloned().unwrap_or_else(|| json!({})),
                "alerts": snapshot.pointer("/health/alerts").cloned().unwrap_or_else(|| json!({})),
                "runtime_sync": runtime
            }),
            "/api/capabilities/status" => {
                let policy = tool_governance_policy(root);
                let tiers = [
                    ("file_read", "green"),
                    ("file_read_many", "green"),
                    ("folder_export", "green"),
                    ("web_fetch", "green"),
                    ("batch_query", "green"),
                    ("web_search", "green"),
                    ("memory_kv_get", "green"),
                    ("memory_kv_list", "green"),
                    ("memory_semantic_query", "green"),
                    ("memory_kv_set", "yellow"),
                    ("cron_schedule", "yellow"),
                    ("cron_run", "yellow"),
                    ("cron_cancel", "yellow"),
                    ("manage_agent", "yellow"),
                    ("terminal_exec", "green"),
                    ("spawn_subagents", "green"),
                ];
                json!({
                    "ok": true,
                    "type": "tool_capability_tiers",
                    "policy": policy,
                    "tools": tiers.iter().map(|(tool, tier)| json!({"tool": tool, "tier": tier})).collect::<Vec<_>>()
                })
            }
            "/api/tools" => json!({
                "ok": true,
                "tools": [
                    {"name": "protheus-ops", "category": "runtime"},
                    {"name": "infringd", "category": "runtime"},
                    {"name": "web_conduit", "category": "runtime"},
                    {"name": "git", "category": "cli"},
                    {"name": "rg", "category": "cli"}
                ],
                "runtime_sync": runtime
            }),
            "/api/commands" => json!({
                "ok": true,
                "commands": [
                    {"cmd": "/status", "command": "/status", "desc": "Show runtime status and cockpit summary", "description": "Show runtime status and cockpit summary"},
                    {"cmd": "/queue", "command": "/queue", "desc": "Show current queue pressure", "description": "Show current queue pressure"},
                    {"cmd": "/context", "command": "/context", "desc": "Show context and attention state", "description": "Show context and attention state"},
                    {"cmd": "/model", "command": "/model", "desc": "Inspect or switch model (/model [name])", "description": "Inspect or switch model (/model [name])"},
                    {"cmd": "/file <path>", "command": "/file <path>", "desc": "Render full file output in chat from workspace path", "description": "Render full file output in chat from workspace path"},
                    {"cmd": "/folder <path>", "command": "/folder <path>", "desc": "Render folder tree + downloadable archive in chat", "description": "Render folder tree + downloadable archive in chat"},
                    {"cmd": "/alerts", "command": "/alerts", "desc": "Show proactive telemetry alerts", "description": "Show proactive telemetry alerts"},
                    {"cmd": "/continuity", "command": "/continuity", "desc": "Show pending actions across sessions/channels/tasks", "description": "Show pending actions across sessions/channels/tasks"},
                    {"cmd": "/browse <url>", "command": "/browse <url>", "desc": "Fetch and summarize a web URL via governed web conduit", "description": "Fetch and summarize a web URL via governed web conduit"},
                    {"cmd": "/search <query>", "command": "/search <query>", "desc": "Search the web with governed web conduit and summarize results", "description": "Search the web with governed web conduit and summarize results"},
                    {"cmd": "/batch <query>", "command": "/batch <query>", "desc": "Run governed batch query primitive (source=web, aperture=medium)", "description": "Run governed batch query primitive (source=web, aperture=medium)"},
                    {"cmd": "/cron", "command": "/cron list | /cron schedule <interval> <message> | /cron run <job_id> | /cron cancel <job_id>", "desc": "Manage agent-owned scheduled jobs", "description": "Manage agent-owned scheduled jobs"},
                    {"cmd": "/memory query <text>", "command": "/memory query <text>", "desc": "Semantic memory lookup over persisted KV entries", "description": "Semantic memory lookup over persisted KV entries"},
                    {"cmd": "/undo", "command": "/undo", "desc": "Undo the last conversational turn with receipted rollback", "description": "Undo the last conversational turn with receipted rollback"},
                    {"cmd": "/aliases", "command": "/aliases", "desc": "List active slash command aliases", "description": "List active slash command aliases"},
                    {"cmd": "/alias", "command": "/alias <shortcut> <target>", "desc": "Create a custom slash alias", "description": "Create a custom slash alias"}
                ]
            }),
            "/api/budget" => json!({
                "ok": true,
                "hourly_spend": 0,
                "daily_spend": usage.pointer("/summary/total_cost_usd").cloned().unwrap_or_else(|| json!(0)),
                "monthly_spend": usage.pointer("/summary/total_cost_usd").cloned().unwrap_or_else(|| json!(0)),
                "hourly_limit": 0,
                "daily_limit": 0,
                "monthly_limit": 0
            }),
            "/api/sessions" => {
                json!({"ok": true, "sessions": session_summary_rows(root, snapshot)})
            }
            "/api/comms/topology" => json!({
                "ok": true,
                "topology": {
                    "nodes": snapshot.pointer("/collab/dashboard/agents").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
                    "edges": 0,
                    "connected": true
                }
            }),
            "/api/comms/events" => json!({"ok": true, "events": []}),
            "/api/profiles" => json!({"ok": true, "profiles": extract_profiles(root)}),
            "/api/update/check" => crate::dashboard_release_update::check_update(root),
            "/api/templates" => json!({
                "ok": true,
                "templates": [
                    {"id": "general-assistant", "name": "General Assistant", "provider": "auto", "model": "auto"},
                    {"id": "research-analyst", "name": "Research Analyst", "provider": "openai", "model": "gpt-5"},
                    {"id": "ops-reliability", "name": "Ops Reliability", "provider": "frontier_provider", "model": "claude-opus-4-20250514"}
                ]
            }),
            _ => return None,
        };
        return Some(CompatApiResponse {
            status: 200,
            payload,
        });
    }

    if method == "POST" {
        if path_only == "/api/update/apply" {
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_release_update::apply_update(root),
            });
        }
        if path_only == "/api/config/set" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let payload = set_config_payload(root, snapshot, &request);
            return Some(CompatApiResponse {
                status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    200
                } else {
                    400
                },
                payload,
            });
        }
        if path_only == "/api/receipts/lineage" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let task_id = clean_text(
                request
                    .get("task_id")
                    .or_else(|| request.get("taskId"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                180,
            );
            if task_id.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({
                        "ok": false,
                        "error": "task_id_required"
                    }),
                });
            }
            let trace_id = clean_text(
                request
                    .get("trace_id")
                    .or_else(|| request.get("traceId"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                180,
            );
            let trace_opt = if trace_id.is_empty() {
                None
            } else {
                Some(trace_id.as_str())
            };
            let limit = request
                .get("limit")
                .and_then(Value::as_u64)
                .map(|value| value as usize)
                .unwrap_or(4000)
                .clamp(1, 50_000);
            let scan_root = clean_text(
                request
                    .get("scan_root")
                    .or_else(|| request.get("scanRoot"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                500,
            );
            let scan_root_path = if scan_root.is_empty() {
                None
            } else {
                let candidate = PathBuf::from(scan_root);
                Some(if candidate.is_absolute() {
                    candidate
                } else {
                    root.join(candidate)
                })
            };
            let payload = match crate::action_receipts_kernel::query_task_lineage(
                root,
                &task_id,
                trace_opt,
                limit,
                scan_root_path.as_deref(),
            ) {
                Ok(out) => out,
                Err(err) => {
                    return Some(CompatApiResponse {
                        status: 400,
                        payload: json!({
                            "ok": false,
                            "error": clean_text(&err, 240)
                        }),
                    })
                }
            };
            return Some(CompatApiResponse {
                status: 200,
                payload,
            });
        }
        if path_only == "/api/web/fetch" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let nexus_connection =
                match crate::dashboard_tool_turn_loop::authorize_ingress_tool_call_with_nexus(
                    "web_fetch",
                ) {
                    Ok(meta) => meta,
                    Err(err) => {
                        return Some(CompatApiResponse {
                            status: 403,
                            payload: json!({
                                "ok": false,
                                "error": "web_fetch_nexus_delivery_denied",
                                "message": "Web fetch blocked by hierarchical nexus ingress policy.",
                                "nexus_error": clean_text(&err, 240)
                            }),
                        })
                    }
                };
            let trace_id = crate::deterministic_receipt_hash(&json!({
                "tool": "web_fetch",
                "request": request,
                "route": "api_web_fetch_post"
            }));
            let task_id = format!(
                "tool-web-fetch-{}",
                trace_id.chars().take(12).collect::<String>()
            );
            let pipeline = tooling_pipeline_execute(
                &trace_id,
                &task_id,
                "web_fetch",
                &request,
                |normalized_args| Ok(crate::web_conduit::api_fetch(root, normalized_args)),
            );
            let mut payload = pipeline
                .get("raw_payload")
                .cloned()
                .unwrap_or_else(|| json!({"ok": false, "error": "tool_pipeline_failed"}));
            if pipeline.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                attach_tool_pipeline(&mut payload, &pipeline);
            }
            if let Some(meta) = nexus_connection {
                if let Some(obj) = payload.as_object_mut() {
                    obj.insert("nexus_connection".to_string(), meta);
                }
            }
            return Some(CompatApiResponse {
                status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    200
                } else {
                    400
                },
                payload,
            });
        }
        if path_only == "/api/web/search" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let nexus_connection =
                match crate::dashboard_tool_turn_loop::authorize_ingress_tool_call_with_nexus(
                    "web_search",
                ) {
                    Ok(meta) => meta,
                    Err(err) => {
                        return Some(CompatApiResponse {
                            status: 403,
                            payload: json!({
                                "ok": false,
                                "error": "web_search_nexus_delivery_denied",
                                "message": "Web search blocked by hierarchical nexus ingress policy.",
                                "nexus_error": clean_text(&err, 240)
                            }),
                        })
                    }
                };
            let trace_id = crate::deterministic_receipt_hash(&json!({
                "tool": "web_search",
                "request": request,
                "route": "api_web_search_post"
            }));
            let task_id = format!(
                "tool-web-search-{}",
                trace_id.chars().take(12).collect::<String>()
            );
            let pipeline = tooling_pipeline_execute(
                &trace_id,
                &task_id,
                "web_search",
                &request,
                |normalized_args| Ok(crate::web_conduit::api_search(root, normalized_args)),
            );
            let mut payload = pipeline
                .get("raw_payload")
                .cloned()
                .unwrap_or_else(|| json!({"ok": false, "error": "tool_pipeline_failed"}));
            if pipeline.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                attach_tool_pipeline(&mut payload, &pipeline);
            }
            if let Some(meta) = nexus_connection {
                if let Some(obj) = payload.as_object_mut() {
                    obj.insert("nexus_connection".to_string(), meta);
                }
            }
            return Some(CompatApiResponse {
                status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    200
                } else {
                    400
                },
                payload,
            });
        }
        if path_only == "/api/batch-query" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let trace_id = crate::deterministic_receipt_hash(&json!({
                "tool": "batch_query",
                "request": request,
                "route": "api_batch_query_post"
            }));
            let task_id = format!(
                "tool-batch-query-{}",
                trace_id.chars().take(12).collect::<String>()
            );
            let pipeline = tooling_pipeline_execute(
                &trace_id,
                &task_id,
                "batch_query",
                &request,
                |normalized_args| {
                    Ok(crate::batch_query_primitive::api_batch_query(
                        root,
                        normalized_args,
                    ))
                },
            );
            let mut payload = pipeline
                .get("raw_payload")
                .cloned()
                .unwrap_or_else(|| json!({"status":"blocked","error":"tool_pipeline_failed"}));
            if pipeline.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                attach_tool_pipeline(&mut payload, &pipeline);
            }
            return Some(CompatApiResponse {
                status: if payload.get("status").and_then(Value::as_str) == Some("blocked") {
                    400
                } else {
                    200
                },
                payload,
            });
        }
        if path_only == "/api/route/auto" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_model_catalog::route_decision_payload(
                    root, snapshot, &request,
                ),
            });
        }
        if path_only == "/api/route/decision" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_model_catalog::route_decision_payload(
                    root, snapshot, &request,
                ),
            });
        }
        return None;
    }

    if method == "DELETE" {
        return None;
    }

    None
}

