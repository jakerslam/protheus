fn handle_primary_dashboard_routes_b(
    root: &Path,
    method: &str,
    path: &str,
    path_only: &str,
    body: &[u8],
    snapshot: &Value,
    requester_agent: &str,
) -> Option<CompatApiResponse> {
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
        let payload = crate::dashboard_sidebar_view_model::augment_conversation_search_payload(
            crate::dashboard_internal_search::search_conversations(root, &query, limit),
            &query,
        );
        let payload = crate::dashboard_sidebar_preview_model::augment_search_payload_with_previews(
            root, payload,
        );
        return Some(CompatApiResponse {
            status: 200,
            payload,
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
        let payload = crate::dashboard_sidebar_view_model::augment_conversation_search_payload(
            crate::dashboard_internal_search::search_conversations(root, &query, limit),
            &query,
        );
        let payload = crate::dashboard_sidebar_preview_model::augment_search_payload_with_previews(
            root, payload,
        );
        return Some(CompatApiResponse {
            status: 200,
            payload,
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
                    "actor_agent_id": requester_agent,
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
                    "actor_agent_id": requester_agent,
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
                    "actor_agent_id": requester_agent,
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
        let rows = crate::dashboard_sidebar_view_model::augment_agent_roster_rows(
            build_agent_roster(root, snapshot, include_terminated),
        );
        let rows =
            crate::dashboard_sidebar_preview_model::augment_agent_roster_with_previews(root, rows);
        return Some(CompatApiResponse {
            status: 200,
            payload: Value::Array(rows),
        });
    }

    if method == "POST" && path_only == "/api/agents/archive-all" {
        if !requester_agent.is_empty() {
            return Some(CompatApiResponse {
                status: 403,
                payload: json!({
                    "ok": false,
                    "error": "agent_manage_forbidden",
                    "actor_agent_id": requester_agent,
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
        let include_permanent = request
            .get("include_permanent")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        return Some(CompatApiResponse {
            status: 200,
            payload: archive_all_visible_agents(root, snapshot, &reason, include_permanent),
        });
    }

    None
}
