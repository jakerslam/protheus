{
        let provider = settings
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or(default_provider.as_str())
            .to_string();
        let model = settings
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or(default_model.as_str())
            .to_string();
        let message = message_from_parsed(parsed, 2, "hello from chat ui");
        if strict && message.trim().is_empty() {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "app_plane_chat_ui",
                "action": "run",
                "errors": ["chat_ui_message_required"]
            });
        }
        let mut selected_provider = provider.clone();
        let mut selected_model = model.clone();
        let (resolved_provider, resolved_model, _) =
            crate::dashboard_model_catalog::resolve_model_selection(
                root,
                &json!({
                    "app": {
                        "settings": {
                            "provider": settings.get("provider").cloned().unwrap_or_else(|| json!(provider.clone())),
                            "model": settings.get("model").cloned().unwrap_or_else(|| json!(model.clone()))
                        }
                    }
                }),
                &selected_provider,
                &selected_model,
                &json!({
                    "task_type": "general",
                    "message": message,
                    "token_count": ((message.len() as i64) / 4).max(1)
                }),
            );
        selected_provider = resolved_provider;
        selected_model = resolved_model;
        let base_system_prompt = clean(parsed.flags.get("system").cloned().unwrap_or_else(|| "You are an Infring dashboard runtime agent. You have host-integrated access to runtime telemetry, agent session memory, and approved infring/infring command surfaces. Never claim you lack system access; if a value is missing, request a runtime sync or the exact command needed and continue.".to_string()), 12_000);
        let tool_gate = chat_ui_turn_tool_decision_tree(&message);
        let gate_decision_authority_mode = clean(
            tool_gate
                .get("decision_authority_mode")
                .and_then(Value::as_str)
                .unwrap_or("llm_controlled_advisory_v1"),
            80,
        );
        let gate_is_advisory = tool_gate
            .get("gate_is_advisory")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let gate_meta_diagnostic_request = tool_gate
            .get("meta_diagnostic_request")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let tool_gate_prompt = chat_ui_tool_gate_system_prompt(&message);
        let system_prompt = if tool_gate_prompt.is_empty() {
            base_system_prompt
        } else {
            clean(
                &format!("{base_system_prompt}\n\n{tool_gate_prompt}"),
                12_000,
            )
        };
        let history_messages = chat_ui_history_messages(&session);
        let invoke = crate::dashboard_provider_runtime::invoke_chat(
            root,
            &selected_provider,
            &selected_model,
            &system_prompt,
            &history_messages,
            &message,
        );
        let response = match invoke {
            Ok(value) => value,
            Err(err) => {
                return json!({
                    "ok": false,
                    "strict": strict,
                    "type": "app_plane_chat_ui",
                    "action": "run",
                    "provider": selected_provider,
                    "model": selected_model,
                    "errors": [clean(err, 240)]
                });
            }
        };
        let mut tools = response
            .get("tools")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if gate_meta_diagnostic_request && !tools.is_empty() {
            tools.clear();
        }
        let requires_live_web = tool_gate
            .get("requires_live_web")
            .and_then(Value::as_bool)
            .map(|value| value && !gate_meta_diagnostic_request)
            .unwrap_or_else(|| !gate_meta_diagnostic_request && chat_ui_requests_live_web(&message));
        let mut assistant_raw = clean(
            response
                .get("response")
                .and_then(Value::as_str)
                .unwrap_or(""),
            16_000,
        );
        let mut forced_web_outcome = String::new();
        let mut forced_web_error_code = String::new();
        let mut forced_web_fallback = json!({
            "applied": false
        });
        let detected_tool_surface_error = chat_ui_detect_tool_surface_error_code(&tools)
            .map(ToString::to_string);
        if requires_live_web && detected_tool_surface_error.is_some() {
            let error_code = detected_tool_surface_error
                .clone()
                .unwrap_or_else(|| "web_tool_surface_degraded".to_string());
            assistant_raw.clear();
            forced_web_outcome = chat_ui_tool_surface_forced_outcome(&error_code).to_string();
            forced_web_error_code = error_code.clone();
            forced_web_fallback = json!({
                "applied": true,
                "reason": "detected_tool_surface_error",
                "fallback_status": "surface_error",
                "error": error_code,
                "decision_authority_mode": gate_decision_authority_mode,
                "gate_is_advisory": gate_is_advisory
            });
        }

    (
        provider,
        model,
        message,
        selected_provider,
        selected_model,
        response,
        tools,
        requires_live_web,
        assistant_raw,
        forced_web_outcome,
        forced_web_error_code,
        forced_web_fallback,
        detected_tool_surface_error,
    )
}
