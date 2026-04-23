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
        let gate_should_call_tools = tool_gate
            .get("needs_tool_access")
            .and_then(Value::as_bool)
            .or_else(|| tool_gate.get("should_call_tools").and_then(Value::as_bool))
            .unwrap_or(false);
        let gate_recommended_tool_family = clean(
            tool_gate
                .get("recommended_tool_family")
                .and_then(Value::as_str)
                .unwrap_or("none"),
            80,
        );
        let gate_route = clean(
            tool_gate
                .get("workflow_route")
                .and_then(Value::as_str)
                .unwrap_or("info"),
            40,
        );
        let gate_reason_code = clean(
            tool_gate
                .get("reason_code")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            120,
        );
        let gate_auto_tools_allowed = tool_gate
            .get("automatic_tool_calls_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let gate_llm_direct_answer = tool_gate
            .get("llm_should_answer_directly")
            .and_then(Value::as_bool)
            .unwrap_or(false);
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
        let gate_allows_web_tooling =
            requires_live_web && gate_should_call_tools && gate_recommended_tool_family == "web_tools";
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
            let fail_closed = chat_ui_tool_surface_fail_closed_copy(&error_code).to_string();
            assistant_raw = clean(&fail_closed, 16_000);
            forced_web_outcome = chat_ui_tool_surface_forced_outcome(&error_code).to_string();
            forced_web_error_code = error_code.clone();
            forced_web_fallback = json!({
                "applied": true,
                "reason": "detected_tool_surface_error",
                "fallback_status": "surface_error",
                "error": error_code
            });
        } else if requires_live_web && !gate_allows_web_tooling {
            let blocked_query = chat_ui_extract_web_query(&message);
            tools.push(json!({
                "name": "batch_query",
                "status": "blocked",
                "ok": false,
                "source": "web",
                "query": blocked_query,
                "error": "workflow_gate_blocked_web_tooling",
                "gate": {
                    "route": gate_route,
                    "reason_code": gate_reason_code,
                    "should_call_tools": gate_should_call_tools,
                    "recommended_tool_family": gate_recommended_tool_family
                }
            }));
            forced_web_outcome = "workflow_gate_blocked_web_tooling".to_string();
            forced_web_error_code = "workflow_gate_blocked_web_tooling".to_string();
            forced_web_fallback = json!({
                "applied": true,
                "status": "blocked_by_workflow_gate",
                "route": gate_route,
                "reason_code": gate_reason_code,
                "llm_should_answer_directly": gate_llm_direct_answer,
                "requires_live_web": requires_live_web,
                "should_call_tools": gate_should_call_tools,
                "recommended_tool_family": gate_recommended_tool_family
            });
        } else if gate_allows_web_tooling
            && gate_auto_tools_allowed
            && chat_ui_web_search_call_count(&tools) == 0
        {
            let fallback_query = chat_ui_extract_web_query(&message);
            let fallback = {
                #[cfg(test)]
                {
                    if let Some(mock) = scripted_batch_query_harness_response(root, &fallback_query) {
                        mock
                    } else {
                        crate::batch_query_primitive::api_batch_query(
                            root,
                            &json!({
                                "source": "web",
                                "query": fallback_query,
                                "aperture": "medium"
                            }),
                        )
                    }
                }
                #[cfg(not(test))]
                {
                    crate::batch_query_primitive::api_batch_query(
                        root,
                        &json!({
                            "source": "web",
                            "query": fallback_query,
                            "aperture": "medium"
                        }),
                    )
                }
            };
            let fallback_ok = fallback.get("ok").and_then(Value::as_bool).unwrap_or(false);
            let summary = clean(
                fallback
                    .get("summary")
                    .or_else(|| fallback.get("response"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                2_000,
            );
            let query_aligned = chat_ui_web_result_matches_query(&fallback_query, &summary);
            if fallback_ok && query_aligned {
                let assistant = if summary.is_empty() {
                    format!("Web search ran for \"{fallback_query}\" and returned results.")
                } else {
                    format!("Web search results for \"{fallback_query}\": {summary}")
                };
                tools.push(json!({
                    "name": "batch_query",
                    "status": "ok",
                    "ok": true,
                    "source": "web",
                    "query": fallback_query,
                    "result": summary,
                    "evidence_refs": fallback.get("evidence_refs").cloned().unwrap_or_else(|| json!([]))
                }));
                assistant_raw = clean(&assistant, 16_000);
                forced_web_outcome = "forced_web_tool_attempt_success".to_string();
                forced_web_fallback = json!({
                    "applied": true,
                    "query": fallback_query,
                    "status": "ok",
                    "source": "batch_query"
                });
            } else {
                let mismatch_only = fallback_ok && !query_aligned;
                let (fail_closed, error_code) = if mismatch_only {
                    (
                        crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                            "provider_low_signal",
                            "web_tool_low_signal",
                            Some("query_result_mismatch"),
                        ),
                        "web_tool_low_signal",
                    )
                } else {
                    (
                        "Web tooling execution failed before any search tool call was recorded (error_code: web_tool_not_invoked). Retry lane: run `batch_query` with a narrower query or one specific source URL.".to_string(),
                        "web_tool_not_invoked",
                    )
                };
                if mismatch_only {
                    tools.push(json!({
                        "name": "batch_query",
                        "status": "low_signal",
                        "ok": false,
                        "source": "web",
                        "query": fallback_query,
                        "result": summary,
                        "error": "web_tool_low_signal"
                    }));
                }
                assistant_raw = clean(&fail_closed, 16_000);
                forced_web_outcome = if mismatch_only {
                    "forced_web_tool_low_signal".to_string()
                } else {
                    "forced_web_tool_not_invoked".to_string()
                };
                forced_web_error_code = error_code.to_string();
                forced_web_fallback = json!({
                    "applied": true,
                    "query": fallback_query,
                    "status": if mismatch_only { "mismatch" } else { "failed" },
                    "query_aligned": query_aligned,
                    "error_code": error_code
                });
            }
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
