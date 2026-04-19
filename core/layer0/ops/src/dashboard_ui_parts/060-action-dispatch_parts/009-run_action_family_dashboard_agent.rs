fn run_action_family_dashboard_agent(root: &Path, normalized: &str, payload: &Value) -> LaneResult {
    match normalized {
        "dashboard.agent.upsertProfile" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let result = dashboard_agent_state::upsert_profile(root, &agent_id, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.upsertProfile".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.archive" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let reason = payload
                .get("reason")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_agent_state::archive_agent(root, &agent_id, &reason);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.archive".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.unarchive" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let result = dashboard_agent_state::unarchive_agent(root, &agent_id);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.unarchive".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.upsertContract" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let result = dashboard_agent_state::upsert_contract(root, &agent_id, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.upsertContract".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.enforceContracts" => {
            let result = dashboard_agent_state::enforce_expired_contracts(root);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: 0,
                argv: vec!["dashboard.agent.enforceContracts".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.session.get" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let result = dashboard_agent_state::load_session(root, &agent_id);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.session.get".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.session.create" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let label = payload
                .get("label")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 80))
                .unwrap_or_default();
            let result = dashboard_agent_state::create_session(root, &agent_id, &label);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.session.create".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.session.switch" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let session_id = payload
                .get("session_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("sessionId").and_then(Value::as_str))
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_agent_state::switch_session(root, &agent_id, &session_id);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.session.switch".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.session.delete" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let session_id = payload
                .get("session_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("sessionId").and_then(Value::as_str))
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_agent_state::delete_session(root, &agent_id, &session_id);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.session.delete".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.session.appendTurn" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let user_text = payload
                .get("user")
                .and_then(Value::as_str)
                .or_else(|| payload.get("input").and_then(Value::as_str))
                .map(|v| clean_chat_text_preserve_layout(v, 2000))
                .unwrap_or_default();
            let assistant_text = payload
                .get("assistant")
                .and_then(Value::as_str)
                .or_else(|| payload.get("response").and_then(Value::as_str))
                .map(|v| clean_chat_text_preserve_layout(v, 4000))
                .unwrap_or_default();
            let result =
                dashboard_agent_state::append_turn(root, &agent_id, &user_text, &assistant_text);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.session.appendTurn".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.memoryKv.set" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let key = payload
                .get("key")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let value = payload.get("value").cloned().unwrap_or(Value::Null);
            let result = dashboard_agent_state::memory_kv_set(root, &agent_id, &key, &value);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.memoryKv.set".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.memoryKv.get" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let key = payload
                .get("key")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_agent_state::memory_kv_get(root, &agent_id, &key);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.memoryKv.get".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.memoryKv.delete" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let key = payload
                .get("key")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_agent_state::memory_kv_delete(root, &agent_id, &key);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.memoryKv.delete".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.suggestions" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let user_hint = payload
                .get("user_hint")
                .and_then(Value::as_str)
                .or_else(|| payload.get("hint").and_then(Value::as_str))
                .map(|v| clean_text(v, 220))
                .unwrap_or_default();
            let result = dashboard_agent_state::suggestions(root, &agent_id, &user_hint);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.suggestions".to_string()],
                payload: Some(result),
            }
        }
        _ => LaneResult {
            ok: false,
            status: 2,
            argv: Vec::new(),
            payload: Some(json!({
                "ok": false,
                "type": "infring_dashboard_action_error",
                "error": format!("unsupported_action:{normalized}")
            })),
        },
    }
}
