) -> Option<CompatApiResponse> {
    let Some(segments) = hands_segments(path_only) else {
        return None;
    };
    let mut state = load_state(root);
    let mut instances = load_instances(&state);
    let catalog_rows = catalog(root, snapshot, &state);

    if method == "GET" && segments.is_empty() {
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "hands": catalog_rows}),
        });
    }

    if method == "GET" && segments.len() == 1 && segments[0] == "active" {
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "instances": instances}),
        });
    }

    if method == "GET" && segments.len() == 1 && segments[0] != "active" {
        let hand_id = clean_id(&segments[0], 80);
        if let Some(detail) = hand_from_catalog(&catalog_rows, &hand_id) {
            return Some(CompatApiResponse {
                status: 200,
                payload: detail,
            });
        }
        return Some(CompatApiResponse {
            status: 404,
            payload: json!({"ok": false, "error": "hand_not_found"}),
        });
    }

    if method == "POST" && segments.len() == 2 {
        let hand_id = clean_id(&segments[0], 80);
        let action = clean_id(&segments[1], 40);
        if action == "check-deps" || action == "install-deps" {
            if let Some(mut detail) = hand_from_catalog(&catalog_rows, &hand_id) {
                let config = hand_config(&state, &hand_id);
                let requirements = detail
                    .get("requirements")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let (evaluated, met) = evaluate_requirements(&requirements, &config);
                if action == "check-deps" {
                    detail["requirements"] = Value::Array(evaluated.clone());
                    detail["requirements_met"] = Value::Bool(met);
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: json!({
                            "ok": true,
                            "hand_id": hand_id,
                            "requirements": evaluated,
                            "requirements_met": met
                        }),
                    });
                }
                let mut results = Vec::<Value>::new();
                for req in &evaluated {
                    let label = clean_text(
                        req.get("label")
                            .and_then(Value::as_str)
                            .unwrap_or("dependency"),
                        120,
                    );
                    let satisfied = as_bool(req.get("satisfied"), false);
                    if satisfied {
                        results.push(json!({
                            "key": clean_text(req.get("key").and_then(Value::as_str).unwrap_or(""), 120),
                            "label": label,
                            "status": "already_installed",
                            "message": "Dependency already satisfied."
                        }));
                    } else if clean_text(req.get("type").and_then(Value::as_str).unwrap_or(""), 40)
                        .eq_ignore_ascii_case("ApiKey")
                    {
                        results.push(json!({
                            "key": clean_text(req.get("key").and_then(Value::as_str).unwrap_or(""), 120),
                            "label": label,
                            "status": "error",
                            "message": "Provide API key in setup to satisfy this requirement."
                        }));
                    } else {
                        results.push(json!({
                            "key": clean_text(req.get("key").and_then(Value::as_str).unwrap_or(""), 120),
                            "label": label,
                            "status": "error",
                            "message": "Automatic install is disabled in compatibility mode. Use the install command shown in setup."
                        }));
                    }
                }
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({
                        "ok": true,
                        "hand_id": hand_id,
                        "results": results,
                        "requirements": evaluated,
                        "requirements_met": met
                    }),
                });
            }
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "hand_not_found"}),
            });
        }
        if action == "activate" {
            let request = parse_json(body);
            let config = request
                .get("config")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            if let Some(detail) = hand_from_catalog(&catalog_rows, &hand_id) {
                set_hand_config(&mut state, &hand_id, &config);
                let hand_name = clean_text(
                    detail.get("name").and_then(Value::as_str).unwrap_or("Hand"),
                    120,
                );
                let agent_name = clean_text(
                    config
                        .get("agent_name")
                        .and_then(Value::as_str)
                        .unwrap_or(&format!("{hand_name} Agent")),
                    140,
                );
                let agent_id = super::make_agent_id(root, &agent_name);
                let provider = clean_text(
                    detail
                        .pointer("/agent/provider")
                        .and_then(Value::as_str)
                        .unwrap_or("auto"),
                    80,
                );
                let model = clean_text(
                    detail
                        .pointer("/agent/model")
                        .and_then(Value::as_str)
                        .unwrap_or("auto"),
                    120,
                );
                let system_prompt = clean_text(
                    detail
                        .pointer("/agent/system_prompt")
                        .and_then(Value::as_str)
                        .unwrap_or("You are a specialized hand agent."),
                    4000,
                );
                let now = crate::now_iso();
                let _ = super::update_profile_patch(
                    root,
                    &agent_id,
                    &json!({
                        "agent_id": agent_id,
                        "name": agent_name,
                        "role": clean_text(detail.pointer("/agent/role").and_then(Value::as_str).unwrap_or("specialist"), 80),
                        "state": "Running",
                        "model_provider": provider,
                        "model_name": model,
                        "system_prompt": system_prompt,
                        "hand_id": hand_id,
                        "created_at": now,
                        "updated_at": now
                    }),
                );
                let _ = super::upsert_contract_patch(
                    root,
                    &agent_id,
                    &json!({
                        "status": "active",
                        "owner": "hands_runtime",
                        "mission": format!("Execute {} hand tasks.", hand_name),
                        "created_at": now,
                        "updated_at": now,
                        "termination_condition": "manual_or_timeout",
                        "expiry_seconds": 0,
                        "auto_terminate_allowed": false
                    }),
                );
                let _ = crate::dashboard_agent_state::memory_kv_set(
                    root,
                    &agent_id,
                    "active_hand_id",
                    &json!(hand_id),
                );
                if hand_id == "trader" {
                    let _ = crate::dashboard_agent_state::memory_kv_set(
                        root,
                        &agent_id,
                        "trader_hand_portfolio_value",
                        &json!("100000"),
                    );
                    let _ = crate::dashboard_agent_state::memory_kv_set(
                        root,
                        &agent_id,
                        "trader_hand_total_pnl",
                        &json!("0"),
                    );
                }
                super::append_turn_message(
                    root,
                    &agent_id,
                    "",
                    &format!("{hand_name} activated and linked to Rust runtime."),
                );
                let instance_id = make_id(
                    "handinst",
                    &json!({"hand_id": hand_id, "agent_id": agent_id, "ts": now}),
                );
                instances.push(normalize_instance(&json!({
                    "instance_id": instance_id,
                    "hand_id": hand_id,
                    "agent_id": agent_id,
                    "agent_name": agent_name,
                    "status": "Active",
                    "activated_at": now,
                    "updated_at": now,
                    "config": config
                })));
                set_instances(&mut state, instances.clone());
                save_state(root, state);
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({
                        "ok": true,
                        "instance_id": instances.first().and_then(|v| v.get("instance_id")).cloned().unwrap_or(Value::Null),
                        "hand_id": hand_id,
                        "agent_id": agent_id,
                        "agent_name": agent_name
                    }),
                });
            }
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "hand_not_found"}),
            });
        }
    }

    if segments.len() >= 2 && segments[0] == "instances" {
        let instance_id = clean_id(&segments[1], 120);
        if instance_id.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": "instance_id_required"}),
            });
        }
        if method == "DELETE" && segments.len() == 2 {
            if let Some(idx) = instances.iter().position(|row| {
                clean_id(
                    row.get("instance_id").and_then(Value::as_str).unwrap_or(""),
                    120,
                ) == instance_id
            }) {
                let agent_id = clean_id(
                    instances[idx]
                        .get("agent_id")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    120,
                );
                if !agent_id.is_empty() {
                    let _ = super::update_profile_patch(
                        root,
                        &agent_id,
                        &json!({"state": "Inactive", "updated_at": crate::now_iso()}),
                    );
                    let _ = super::upsert_contract_patch(
                        root,
                        &agent_id,
                        &json!({"status": "inactive", "updated_at": crate::now_iso()}),
                    );
                }
                instances.remove(idx);
                set_instances(&mut state, instances);
                save_state(root, state);
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({"ok": true, "deleted": true, "instance_id": instance_id}),
                });
            }
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "instance_not_found"}),
            });
        }
        if method == "GET" && segments.len() == 3 && segments[2] == "stats" {
            if let Some(instance) = instances.iter().find(|row| {
                clean_id(
                    row.get("instance_id").and_then(Value::as_str).unwrap_or(""),
                    120,
                ) == instance_id
            }) {
                let mut payload = stats_for_instance(instance);
                payload["instance_id"] = Value::String(instance_id);
                return Some(CompatApiResponse {
                    status: 200,
                    payload,
                });
            }
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "instance_not_found"}),
            });
        }
        if method == "GET" && segments.len() == 3 && segments[2] == "browser" {
            if let Some(instance) = instances.iter().find(|row| {
                clean_id(
                    row.get("instance_id").and_then(Value::as_str).unwrap_or(""),
                    120,
                ) == instance_id
            }) {
                let hand_id = clean_id(
                    instance
                        .get("hand_id")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    80,
                );
                if hand_id != "browser" {
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: json!({"ok": true, "active": false}),
                    });
                }
                let url = clean_text(
                    instance
                        .pointer("/config/start_url")
                        .and_then(Value::as_str)
                        .unwrap_or("https://example.com"),
                    400,
                );
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({
                        "ok": true,
                        "active": clean_text(instance.get("status").and_then(Value::as_str).unwrap_or(""), 40) == "Active",
                        "url": url,
                        "title": "Browser Hand Session",
                        "screenshot_base64": BROWSER_PLACEHOLDER_SCREENSHOT_BASE64,
                        "content": "Browser hand connected. Live browser telemetry can stream into this panel."
                    }),
                });
            }
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "instance_not_found"}),
            });
        }
        if method == "POST"
            && segments.len() == 3
            && (segments[2] == "pause" || segments[2] == "resume")
        {
            if let Some(idx) = instances.iter().position(|row| {
                clean_id(
                    row.get("instance_id").and_then(Value::as_str).unwrap_or(""),
                    120,
                ) == instance_id
            }) {
                let is_pause = segments[2] == "pause";
                instances[idx]["status"] = Value::String(if is_pause {
                    "Paused".to_string()
                } else {
                    "Active".to_string()
                });
                instances[idx]["updated_at"] = Value::String(crate::now_iso());
                let agent_id = clean_id(
                    instances[idx]
                        .get("agent_id")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    120,
                );
                if !agent_id.is_empty() {
                    let _ = super::update_profile_patch(
                        root,
                        &agent_id,
                        &json!({
                            "state": if is_pause { "Paused" } else { "Running" },
                            "updated_at": crate::now_iso()
                        }),
                    );
                }
                set_instances(&mut state, instances.clone());
                save_state(root, state);
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({"ok": true, "instance": instances[idx].clone()}),
                });
            }
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "instance_not_found"}),
            });
        }
    }

    Some(CompatApiResponse {
        status: 405,
        payload: json!({"ok": false, "error": "method_not_allowed"}),
    })
}
