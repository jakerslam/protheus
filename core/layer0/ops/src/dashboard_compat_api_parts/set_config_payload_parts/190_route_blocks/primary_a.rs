fn handle_primary_dashboard_routes_a(
    root: &Path,
    method: &str,
    path: &str,
    path_only: &str,
    body: &[u8],
    headers: &[(&str, &str)],
    snapshot: &Value,
    requester_agent: &str,
) -> Option<CompatApiResponse> {
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

    None
}
