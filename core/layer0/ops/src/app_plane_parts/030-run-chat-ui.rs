fn run_chat_ui(root: &Path, parsed: &crate::ParsedArgs, strict: bool, action: &str) -> Value {
    let contract = load_json_or(
        root,
        CHAT_UI_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "chat_ui_contract",
            "providers": ["openai", "frontier_provider", "google", "gemini", "groq", "deepseek", "openrouter", "xai", "ollama", "claude-code"],
            "default_provider": "openai",
            "default_model": "gpt-5"
        }),
    );
    let providers = contract
        .get("providers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(str::to_string))
        .collect::<Vec<_>>();
    let default_provider = contract
        .get("default_provider")
        .and_then(Value::as_str)
        .unwrap_or("openai")
        .to_string();
    let default_model = contract
        .get("default_model")
        .and_then(Value::as_str)
        .unwrap_or("gpt-5")
        .to_string();

    let mut settings = read_json(&chat_ui_settings_path(root)).unwrap_or_else(|| {
        json!({
            "provider": default_provider,
            "model": default_model,
            "updated_at": crate::now_iso()
        })
    });
    let session_id = clean_id(
        parsed
            .flags
            .get("session-id")
            .map(String::as_str)
            .or_else(|| parsed.flags.get("session").map(String::as_str)),
        "chat-ui-default",
    );
    let path = chat_ui_session_path(root, &session_id);
    let mut session = read_json(&path).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "session_id": session_id,
            "turns": []
        })
    });
    if !session.get("turns").map(Value::is_array).unwrap_or(false) {
        session["turns"] = Value::Array(Vec::new());
    }

    if action == "switch-provider" {
        let provider = clean(
            parsed
                .flags
                .get("provider")
                .cloned()
                .or_else(|| parsed.positional.get(2).cloned())
                .unwrap_or_else(|| default_provider.clone()),
            60,
        )
        .to_ascii_lowercase();
        if strict && !providers.iter().any(|row| row == &provider) {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "app_plane_chat_ui",
                "action": action,
                "errors": ["chat_ui_provider_invalid"]
            });
        }
        let model = clean(
            parsed
                .flags
                .get("model")
                .cloned()
                .unwrap_or_else(|| format!("{}-default", provider)),
            120,
        );
        settings["provider"] = Value::String(provider.clone());
        settings["model"] = Value::String(model.clone());
        settings["updated_at"] = Value::String(crate::now_iso());
        let _ = write_json(&chat_ui_settings_path(root), &settings);
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "app_plane_chat_ui",
            "lane": "core/layer0/ops",
            "action": action,
            "provider": provider,
            "model": model,
            "artifact": {
                "path": chat_ui_settings_path(root).display().to_string(),
                "sha256": sha256_hex_str(&settings.to_string())
            },
            "claim_evidence": [
                {
                    "id": "V6-APP-007.1",
                    "claim": "chat_ui_switches_provider_and_model_with_deterministic_receipts",
                    "evidence": {
                        "provider": settings.get("provider"),
                        "model": settings.get("model")
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    if matches!(action, "history" | "status") {
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "app_plane_chat_ui",
            "lane": "core/layer0/ops",
            "action": action,
            "session_id": session_id,
            "settings": settings,
            "turn_count": session.get("turns").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
            "turns": if action == "history" { session.get("turns").cloned().unwrap_or_else(|| Value::Array(Vec::new())) } else { Value::Array(Vec::new()) },
            "claim_evidence": [
                {
                    "id": "V6-APP-007.1",
                    "claim": "chat_ui_surfaces_sidebar_history_and_provider_settings_over_core_receipts",
                    "evidence": {
                        "session_id": session_id
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    if action == "replay" {
        let turn_index = parse_u64(parsed.flags.get("turn"), 0) as usize;
        let turns = session
            .get("turns")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let selected = if turns.is_empty() {
            None
        } else if turn_index >= turns.len() {
            turns.last().cloned()
        } else {
            turns.get(turn_index).cloned()
        };
        if strict && selected.is_none() {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "app_plane_chat_ui",
                "action": "replay",
                "errors": ["chat_ui_turn_not_found"]
            });
        }
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "app_plane_chat_ui",
            "lane": "core/layer0/ops",
            "action": "replay",
            "session_id": session_id,
            "turn": selected,
            "turn_index": turn_index,
            "claim_evidence": [
                {
                    "id": "V6-APP-007.1",
                    "claim": "chat_ui_replay_supports_receipted_history_sidebar_navigation",
                    "evidence": {
                        "session_id": session_id,
                        "turn_index": turn_index
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }
    if action == "view-logs" {
        let request_id = clean(
            parsed
                .flags
                .get("request-id")
                .cloned()
                .or_else(|| parsed.flags.get("trace-id").cloned())
                .or_else(|| parsed.flags.get("call-id").cloned())
                .or_else(|| parsed.positional.get(2).cloned())
                .unwrap_or_default(),
            160,
        );
        if strict && request_id.is_empty() {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "app_plane_chat_ui",
                "action": "view-logs",
                "errors": ["chat_ui_request_id_required"]
            });
        }
        let history_path = state_root(root).join("chat_ui").join("history.jsonl");
        let rows = chat_ui_read_jsonl_rows(&history_path, 400);
        let request_key = request_id.to_ascii_lowercase();
        let mut matches = Vec::<Value>::new();
        for row in rows.into_iter().rev() {
            if request_key.is_empty() {
                matches.push(row);
            } else {
                let row_blob = clean(&row.to_string(), 20_000).to_ascii_lowercase();
                if row_blob.contains(&request_key) {
                    matches.push(row);
                }
            }
            if matches.len() >= 24 {
                break;
            }
        }
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "app_plane_chat_ui",
            "lane": "core/layer0/ops",
            "action": "view-logs",
            "session_id": session_id,
            "request_id": request_id,
            "history_path": history_path.display().to_string(),
            "match_count": matches.len(),
            "matches": matches,
            "claim_evidence": [
                {
                    "id": "V6-APP-007.1",
                    "claim": "chat_ui_supports_request_trace_debug_lookup_with_receipted_results",
                    "evidence": {
                        "session_id": session_id
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }
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
    let base_system_prompt = clean(parsed.flags.get("system").cloned().unwrap_or_else(|| "You are an Infring dashboard runtime agent. You have host-integrated access to runtime telemetry, agent session memory, and approved protheus/infring command surfaces. Never claim you lack system access; if a value is missing, request a runtime sync or the exact command needed and continue.".to_string()), 12_000);
    let tool_gate = chat_ui_turn_tool_decision_tree(&message);
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
    let requires_live_web = tool_gate
        .get("requires_live_web")
        .and_then(Value::as_bool)
        .unwrap_or_else(|| chat_ui_requests_live_web(&message));
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
    } else if requires_live_web && chat_ui_web_search_call_count(&tools) == 0 {
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
    let (assistant_initial, response_finalization_outcome) =
        finalize_chat_ui_assistant_response(&message, &assistant_raw, &tools);
    let tool_diagnostics = chat_ui_tool_diagnostics(&tools);
    let receipt_summary =
        chat_ui_semantic_receipt_summary(&tool_diagnostics, requires_live_web, &message);
    let (assistant_rewritten, rewrite_outcome) =
        rewrite_chat_ui_placeholder_with_tool_diagnostics(&assistant_initial, &tool_diagnostics);
    let mut assistant = assistant_rewritten;
    let mut hard_guard = json!({
        "applied": false
    });
    let web_search_calls = chat_ui_web_search_call_count(&tools) as i64;
    if assistant.trim().is_empty()
        || crate::tool_output_match_filter::matches_ack_placeholder(&assistant)
        || crate::tool_output_match_filter::contains_forbidden_runtime_context_markers(&assistant)
    {
        let (fallback_status, fallback_error_code) = chat_ui_fallback_status_error_for_diagnostics(
            &tool_diagnostics,
            requires_live_web,
            web_search_calls,
        );
        assistant = crate::tool_output_match_filter::canonical_tooling_fallback_copy(
            fallback_status,
            fallback_error_code,
            None,
        );
        hard_guard = json!({
            "applied": true,
            "status": fallback_status,
            "error_code": fallback_error_code
        });
        if forced_web_error_code.is_empty() {
            forced_web_error_code = fallback_error_code.to_string();
        }
    }
    let blocked_calls = tool_diagnostics
        .get("blocked_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let blocked_error_calls = tool_diagnostics
        .pointer("/error_codes/web_tool_policy_blocked")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let blocked_receipt_calls = chat_ui_receipt_status_count(&tool_diagnostics, "blocked");
    let not_found_error_calls = tool_diagnostics
        .pointer("/error_codes/web_tool_not_found")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let not_found_receipt_calls = chat_ui_receipt_status_count(&tool_diagnostics, "not_found");
    let low_signal_calls = tool_diagnostics
        .get("low_signal_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let no_result_calls = tool_diagnostics
        .get("no_result_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let low_signal_error_calls = tool_diagnostics
        .pointer("/error_codes/web_tool_low_signal")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let low_signal_receipt_calls = chat_ui_receipt_status_count(&tool_diagnostics, "low_signal");
    let surface_unavailable_calls = tool_diagnostics
        .get("surface_unavailable_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let surface_degraded_calls = tool_diagnostics
        .get("surface_degraded_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let blocked_signal = blocked_calls > 0 || blocked_error_calls > 0 || blocked_receipt_calls > 0;
    let not_found_signal = not_found_error_calls > 0 || not_found_receipt_calls > 0;
    let forced_surface_error_hint = if matches!(
        forced_web_error_code.as_str(),
        "web_tool_surface_unavailable" | "web_tool_surface_degraded"
    ) {
        Some(forced_web_error_code.clone())
    } else {
        None
    };
    let finalization_inferred_surface_error = if response_finalization_outcome
        == "tool_surface_error_fail_closed"
    {
        detected_tool_surface_error
            .clone()
            .or_else(|| forced_surface_error_hint.clone())
            .or_else(|| Some("web_tool_surface_degraded".to_string()))
    } else {
        None
    };
    let tool_surface_error_code = detected_tool_surface_error
        .clone()
        .or_else(|| forced_surface_error_hint.clone())
        .or(finalization_inferred_surface_error)
        .or_else(|| {
            if surface_unavailable_calls > 0 {
                Some("web_tool_surface_unavailable".to_string())
            } else if surface_degraded_calls > 0 {
                Some("web_tool_surface_degraded".to_string())
            } else {
                None
            }
        });
    let low_signal = low_signal_calls > 0
        || no_result_calls > 0
        || low_signal_error_calls > 0
        || low_signal_receipt_calls > 0;
    let mut web_classification = if let Some(surface_error) = tool_surface_error_code.as_deref() {
        chat_ui_tool_surface_classification(surface_error).to_string()
    } else if requires_live_web && web_search_calls == 0 {
        "tool_not_invoked".to_string()
    } else if blocked_signal {
        "policy_blocked".to_string()
    } else if not_found_signal {
        "tool_not_found".to_string()
    } else if low_signal {
        "low_signal".to_string()
    } else if requires_live_web {
        "healthy".to_string()
    } else {
        "not_required".to_string()
    };
    let expected_web_classification = chat_ui_expected_classification_from_diagnostics(
        &tool_diagnostics,
        requires_live_web,
        web_search_calls,
    );
    let classification_consistent = web_classification == expected_web_classification;
    let selected_classification_error_code =
        chat_ui_error_code_for_classification(&web_classification).to_string();
    let expected_classification_error_code =
        chat_ui_error_code_for_classification(expected_web_classification).to_string();
    let mut classification_active_error_code = if classification_consistent {
        selected_classification_error_code.clone()
    } else {
        expected_classification_error_code.clone()
    };
    let mut classification_guard = json!({
        "applied": !classification_consistent,
        "selected": web_classification.clone(),
        "expected": expected_web_classification,
        "consistent": classification_consistent,
        "mode": if classification_consistent { "none" } else { "override" },
        "selected_error_code": if selected_classification_error_code.is_empty() { Value::Null } else { json!(selected_classification_error_code) },
        "expected_error_code": if expected_classification_error_code.is_empty() { Value::Null } else { json!(expected_classification_error_code) },
        "active_error_code": Value::Null,
        "retry_recommended": false,
        "retry_strategy": "none",
        "retry_lane": "none",
        "not_invoked_fail_closed": false,
        "fail_closed": false,
        "fail_closed_class": null
    });
    if let Some(guard) = classification_guard.as_object_mut() {
        if !classification_active_error_code.is_empty() {
            guard.insert(
                "active_error_code".to_string(),
                json!(classification_active_error_code),
            );
        }
    }
    if !classification_consistent {
        web_classification = expected_web_classification.to_string();
    }
    let assistant_placeholder_like = assistant.trim().is_empty()
        || crate::tool_output_match_filter::matches_ack_placeholder(&assistant)
        || crate::tool_output_match_filter::contains_forbidden_runtime_context_markers(&assistant);
    let assistant_context_mismatch = !chat_ui_response_matches_previous_message(&message, &assistant)
        || chat_ui_contains_kernel_patch_thread_dump(&message, &assistant)
        || chat_ui_contains_role_preamble_prompt_dump(&message, &assistant)
        || chat_ui_contains_competitive_programming_dump(&message, &assistant);
    let classification_findings_available = chat_ui_tools_have_valid_findings(&tools);
    let classification_should_fail_close = requires_live_web
        && matches!(
            web_classification.as_str(),
            "tool_not_invoked"
                | "policy_blocked"
                | "tool_not_found"
                | "low_signal"
        )
        && (assistant_placeholder_like
            || !classification_findings_available
            || assistant_context_mismatch);
    if classification_should_fail_close {
        let fallback_status = chat_ui_fallback_status_for_classification(&web_classification);
        let fallback_error_code = chat_ui_error_code_for_classification(&web_classification);
        if !fallback_error_code.is_empty() {
            assistant = crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                fallback_status,
                fallback_error_code,
                None,
            );
            hard_guard = json!({
                "applied": true,
                "status": fallback_status,
                "error_code": fallback_error_code,
                "classification": web_classification,
                "source": "classification_guard"
            });
            if forced_web_error_code.is_empty() {
                forced_web_error_code = fallback_error_code.to_string();
            }
            classification_active_error_code = fallback_error_code.to_string();
            if let Some(guard) = classification_guard.as_object_mut() {
                guard.insert("applied".to_string(), json!(true));
                guard.insert("mode".to_string(), json!("fail_close"));
                guard.insert("fail_closed".to_string(), json!(true));
                guard.insert("fail_closed_class".to_string(), json!(web_classification));
                guard.insert(
                    "active_error_code".to_string(),
                    json!(classification_active_error_code),
                );
                if web_classification == "tool_not_invoked" {
                    guard.insert("not_invoked_fail_closed".to_string(), json!(true));
                }
            }
        }
    }
    let mut final_outcome = if forced_web_outcome.is_empty() {
        response_finalization_outcome.clone()
    } else {
        forced_web_outcome.clone()
    };
    let hard_guard_applied = hard_guard
        .get("applied")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let hard_guard_source = hard_guard.get("source").and_then(Value::as_str).unwrap_or("");
    if hard_guard_source == "classification_guard" {
        let classification_error_code = chat_ui_error_code_for_classification(&web_classification);
        if !classification_error_code.is_empty() {
            forced_web_error_code = classification_error_code.to_string();
            classification_active_error_code = classification_error_code.to_string();
            if let Some(guard) = classification_guard.as_object_mut() {
                guard.insert(
                    "active_error_code".to_string(),
                    json!(classification_active_error_code),
                );
            }
        }
    }
    let retry_loop_risk = tool_diagnostics
        .get("loop_risk")
        .cloned()
        .unwrap_or_else(|| chat_ui_retry_loop_risk_from_diagnostics(&tool_diagnostics));
    let (base_guard_retry_recommended, base_guard_retry_strategy, base_guard_retry_lane) =
        chat_ui_retry_profile_for_guard(&classification_active_error_code, &web_classification);
    let (guard_retry_recommended, guard_retry_strategy, guard_retry_lane) =
        chat_ui_apply_loop_risk_to_retry(
            base_guard_retry_recommended,
            base_guard_retry_strategy,
            base_guard_retry_lane,
            &retry_loop_risk,
        );
    let retry_suppressed_by_loop_risk = base_guard_retry_recommended && !guard_retry_recommended;
    let mut guard_retry_plan = chat_ui_retry_plan_for_guard(
        guard_retry_recommended,
        guard_retry_strategy,
        guard_retry_lane,
    );
    if let Some(plan) = guard_retry_plan.as_object_mut() {
        plan.insert("loop_risk".to_string(), retry_loop_risk.clone());
        plan.insert(
            "suppressed_by_loop_risk".to_string(),
            json!(retry_suppressed_by_loop_risk),
        );
    }
    if let Some(guard) = classification_guard.as_object_mut() {
        guard.insert(
            "retry_recommended".to_string(),
            json!(guard_retry_recommended),
        );
        guard.insert("retry_strategy".to_string(), json!(guard_retry_strategy));
        guard.insert("retry_lane".to_string(), json!(guard_retry_lane));
        guard.insert("retry_plan".to_string(), guard_retry_plan.clone());
        guard.insert(
            "retry_suppressed_by_loop_risk".to_string(),
            json!(retry_suppressed_by_loop_risk),
        );
        guard.insert("retry_loop_risk".to_string(), retry_loop_risk.clone());
    }
    if hard_guard_applied {
        final_outcome = if hard_guard_source == "classification_guard" {
            let guard_class = clean(
                hard_guard
                    .get("classification")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
                80,
            )
            .replace('-', "_");
            if guard_class == "tool_not_invoked" {
                "classification_guard_not_invoked_fail_closed".to_string()
            } else {
                format!("classification_guard_{guard_class}_fail_closed")
            }
        } else {
            "hard_guard_fallback".to_string()
        };
    }
    if !classification_consistent && !hard_guard_applied {
        final_outcome = "classification_guard_overrode".to_string();
    }
    if final_outcome == "tool_surface_error_fail_closed" {
        final_outcome = chat_ui_tool_surface_forced_outcome(
            tool_surface_error_code
                .as_deref()
                .unwrap_or("web_tool_surface_degraded"),
        )
        .to_string();
    }
    if forced_web_error_code.is_empty() {
        if let Some(surface_error) = tool_surface_error_code.as_deref() {
            forced_web_error_code = surface_error.to_string();
        }
    }
    if forced_web_error_code.is_empty() {
        let classification_error_code = chat_ui_error_code_for_classification(&web_classification);
        if !classification_error_code.is_empty() {
            forced_web_error_code = classification_error_code.to_string();
        }
    }
    let response_tool_surface_error_code = if matches!(
        forced_web_error_code.as_str(),
        "web_tool_surface_unavailable" | "web_tool_surface_degraded"
    ) {
        Some(forced_web_error_code.clone())
    } else {
        tool_surface_error_code.clone()
    };
    let trace_id = format!(
        "trace_{}",
        &sha256_hex_str(&format!(
            "{}:{}:{}:{}:{}",
            session_id,
            selected_provider,
            selected_model,
            message,
            crate::now_iso()
        ))[..12]
    );
    let mut discovered_tools = Vec::<String>::new();
    for row in &tools {
        let tool_name = tool_name_for_diagnostics(row);
        if !tool_name.is_empty() && !discovered_tools.iter().any(|existing| existing == &tool_name)
        {
            discovered_tools.push(tool_name);
        }
    }
    if requires_live_web && !discovered_tools.iter().any(|tool| tool == "batch_query") {
        discovered_tools.push("batch_query".to_string());
    }
    let transaction_complete = (!requires_live_web || web_classification == "healthy")
        && !hard_guard_applied;
    let transaction_status = if transaction_complete {
        "complete"
    } else if matches!(
        web_classification.as_str(),
        "low_signal" | "tool_surface_degraded"
    ) {
        "degraded"
    } else {
        "failed"
    };
    let transaction_id = format!(
        "txn_{}",
        &sha256_hex_str(&format!(
            "{}:{}:{}:{}",
            session_id, trace_id, web_classification, final_outcome
        ))[..12]
    );
    let transaction_intent = if requires_live_web {
        chat_ui_extract_web_query(&message)
    } else {
        clean(&message, 200)
    };
    let turn = json!({
        "turn_id": format!(
            "turn_{}",
            &sha256_hex_str(&format!("{}:{}:{}:{}", session_id, selected_provider, selected_model, crate::now_iso()))[..10]
        ),
        "trace_id": trace_id,
        "ts": crate::now_iso(),
        "provider": selected_provider,
        "model": selected_model,
        "user": message,
        "assistant": assistant,
        "tool_summary": receipt_summary,
        "transaction": {
            "id": transaction_id,
            "intent": transaction_intent,
            "status": transaction_status,
            "complete": transaction_complete,
            "classification": web_classification.clone(),
            "retry": {
                "recommended": guard_retry_recommended,
                "strategy": guard_retry_strategy,
                "lane": guard_retry_lane,
                "plan": guard_retry_plan.clone()
            },
            "closed_at": crate::now_iso()
        }
    });
    let mut turns = session
        .get("turns")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    turns.push(turn.clone());
    session["turns"] = Value::Array(turns);
    session["updated_at"] = Value::String(crate::now_iso());
    let _ = write_json(&path, &session);
    let _ = append_jsonl(
        &state_root(root).join("chat_ui").join("history.jsonl"),
        &json!({"action":"run","session_id":session_id,"turn":turn,"ts":crate::now_iso()}),
    );
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "app_plane_chat_ui",
        "lane": "core/layer0/ops",
        "action": "run",
        "session_id": session_id,
        "trace_id": trace_id,
        "turn": turn,
        "provider": response.get("provider").cloned().unwrap_or_else(|| json!(provider)),
        "model": response.get("model").cloned().unwrap_or_else(|| json!(model)),
        "runtime_model": response.get("runtime_model").cloned().unwrap_or_else(|| json!(selected_model)),
        "input_tokens": response.get("input_tokens").cloned().unwrap_or_else(|| json!(0)),
        "output_tokens": response.get("output_tokens").cloned().unwrap_or_else(|| json!(0)),
        "cost_usd": response.get("cost_usd").cloned().unwrap_or_else(|| json!(0.0)),
        "context_window": response.get("context_window").cloned().unwrap_or_else(|| json!(0)),
        "tools": Value::Array(tools.clone()),
        "response_finalization": {
            "applied": true,
            "outcome": final_outcome,
            "rewrite_outcome": rewrite_outcome,
            "tool_surface_error_code": response_tool_surface_error_code,
            "final_ack_only": crate::tool_output_match_filter::matches_ack_placeholder(&assistant),
            "findings_available": chat_ui_tools_have_valid_findings(&tools),
            "tool_receipt_summary": receipt_summary,
            "tool_transaction": {
                "id": transaction_id,
                "intent": transaction_intent,
                "status": transaction_status,
                "complete": transaction_complete,
                "classification": web_classification.clone(),
                "retry": {
                    "recommended": guard_retry_recommended,
                    "strategy": guard_retry_strategy,
                    "lane": guard_retry_lane,
                    "plan": guard_retry_plan,
                    "suppressed_by_loop_risk": retry_suppressed_by_loop_risk,
                    "loop_risk": retry_loop_risk
                },
                "closed_at": crate::now_iso()
            },
            "hard_guard": hard_guard,
            "classification_guard": classification_guard,
            "tool_diagnostics": tool_diagnostics,
            "tool_gate": tool_gate,
            "capability_discovery": {
                "contract": "tool_execution_receipt_v1",
                "execution_statuses": ["ok", "error", "blocked", "not_found", "low_signal", "unknown"],
                "discovered_tools": discovered_tools,
                "recommended_tool_family": clean(
                    tool_gate.get("recommended_tool_family").and_then(Value::as_str).unwrap_or("none"),
                    80
                ),
                "provider_catalog": providers,
                "selected_provider": selected_provider,
                "selected_model": selected_model
            },
            "web_invariant": {
                "requires_live_web": requires_live_web,
                "tool_attempted": web_search_calls > 0,
                "web_search_calls": web_search_calls,
                "classification": web_classification,
                "tool_surface_error_code": response_tool_surface_error_code,
                "diagnostic": "forced_live_web_invariant_from_app_plane_chat_ui"
            }
        },
        "web_tooling_fallback": forced_web_fallback,
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&session.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-APP-007.1",
                "claim": "chat_ui_runs_multi_provider_conversation_with_receipted_model_calls",
                "evidence": {
                    "provider": settings.get("provider"),
                    "model": settings.get("model"),
                    "session_id": session_id
                }
            }
        ]
    });
    if !forced_web_error_code.is_empty() {
        out["error"] = Value::String(forced_web_error_code);
    }
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn chat_ui_contains_any(lowered: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| lowered.contains(marker))
}

fn chat_ui_turn_is_meta_control_message(raw_input: &str) -> bool {
    let lowered = clean(raw_input, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    chat_ui_contains_any(
        &lowered,
        &[
            "that was just a test",
            "just a test",
            "just testing",
            "test only",
            "ignore that",
            "never mind",
            "nm",
            "thanks",
            "thank you",
            "cool",
            "sounds good",
            "did you try it",
            "did you do it",
            "what happened",
        ],
    ) && !chat_ui_contains_any(
        &lowered,
        &[
            "search",
            "web",
            "online",
            "internet",
            "file",
            "patch",
            "edit",
            "update",
            "create",
            "read",
            "memory",
            "repo",
            "codebase",
        ],
    )
}

fn chat_ui_message_is_tooling_status_check(raw_input: &str) -> bool {
    let lowered = clean(raw_input, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let status_frame = lowered.starts_with("did you")
        || lowered.starts_with("what happened")
        || lowered.starts_with("status")
        || lowered.contains("did that run")
        || lowered.contains("did it run")
        || lowered.contains("did it work")
        || lowered.contains("is it working");
    if !status_frame {
        return false;
    }
    let tooling_reference = lowered.contains("web request")
        || lowered.contains("web tooling")
        || lowered.contains("web tool")
        || lowered.contains("web search")
        || lowered.contains("search request")
        || lowered.contains("tooling workflow")
        || lowered.contains("tool workflow")
        || lowered.contains("tool call")
        || lowered.contains("tool run")
        || lowered.contains("workflow run")
        || lowered.contains("last run")
        || lowered.contains("workspace analysis")
        || lowered.contains("workspace analyze")
        || lowered.contains("batch query");
    if !tooling_reference {
        return false;
    }
    let asks_fresh_query = lowered.contains("search for ")
        || lowered.contains("look up ")
        || lowered.contains("find information")
        || lowered.contains("about ")
        || lowered.contains("latest ")
        || lowered.contains("top ")
        || lowered.contains("best ")
        || lowered.contains("read file ")
        || lowered.contains("open file ")
        || lowered.contains("analyze ");
    !asks_fresh_query
}

fn chat_ui_turn_requires_file_mutation(raw_input: &str) -> bool {
    let lowered = clean(raw_input, 1_200).to_ascii_lowercase();
    chat_ui_contains_any(
        &lowered,
        &[
            "edit file",
            "modify file",
            "update file",
            "patch",
            "write ",
            "rewrite ",
            "create file",
            "add file",
            "delete file",
            "remove file",
            "rename file",
            "refactor",
            "implement",
        ],
    )
}

fn chat_ui_turn_requires_local_lookup(raw_input: &str) -> bool {
    let lowered = clean(raw_input, 1_200).to_ascii_lowercase();
    chat_ui_contains_any(
        &lowered,
        &[
            "repo",
            "repository",
            "workspace",
            "codebase",
            "project files",
            "memory file",
            "local memory",
            "logs",
            "read file",
            "check file",
            "inspect file",
            "in this repo",
            "in our system",
        ],
    )
}

fn chat_ui_turn_tool_decision_tree(raw_input: &str) -> Value {
    let meta_control_message = chat_ui_turn_is_meta_control_message(raw_input);
    let status_check_message = if meta_control_message {
        false
    } else {
        chat_ui_message_is_tooling_status_check(raw_input)
    };
    let requires_file_mutation = if meta_control_message || status_check_message {
        false
    } else {
        chat_ui_turn_requires_file_mutation(raw_input)
    };
    let requires_live_web = if meta_control_message || status_check_message {
        false
    } else {
        chat_ui_requests_live_web(raw_input)
    };
    let requires_local_lookup = if meta_control_message || status_check_message {
        false
    } else {
        chat_ui_turn_requires_local_lookup(raw_input)
    };
    let has_sufficient_information =
        meta_control_message
            || status_check_message
            || (!requires_file_mutation && !requires_live_web && !requires_local_lookup);
    let should_call_tools =
        !has_sufficient_information && (requires_file_mutation || requires_live_web || requires_local_lookup);
    let info_source = if requires_live_web {
        "web"
    } else if requires_local_lookup || requires_file_mutation {
        "local"
    } else {
        "none"
    };
    let recommended_tool_family = if requires_file_mutation {
        "file_tools"
    } else if requires_live_web {
        "web_tools"
    } else if requires_local_lookup {
        "memory_or_workspace_tools"
    } else {
        "none"
    };
    json!({
        "contract": "tool_decision_tree_v1",
        "requires_file_mutation": requires_file_mutation,
        "requires_local_lookup": requires_local_lookup,
        "requires_live_web": requires_live_web,
        "has_sufficient_information": has_sufficient_information,
        "should_call_tools": should_call_tools,
        "info_source": info_source,
        "recommended_tool_family": recommended_tool_family,
        "meta_control_message": meta_control_message,
        "status_check_message": status_check_message
    })
}

fn chat_ui_tool_gate_system_prompt(raw_input: &str) -> String {
    let gate = chat_ui_turn_tool_decision_tree(raw_input);
    let requires_file_mutation = gate
        .get("requires_file_mutation")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let has_sufficient_information = gate
        .get("has_sufficient_information")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let status_check_message = gate
        .get("status_check_message")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let info_source = clean(
        gate.get("info_source")
            .and_then(Value::as_str)
            .unwrap_or("none"),
        40,
    );
    let should_call_tools = gate
        .get("should_call_tools")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let recommended_tool_family = clean(
        gate.get("recommended_tool_family")
            .and_then(Value::as_str)
            .unwrap_or("none"),
        80,
    );
    clean(
        &format!(
            "Deterministic tool gate for this turn: requires_file_mutation={requires_file_mutation}, has_sufficient_information={has_sufficient_information}, status_check_message={status_check_message}, info_source={info_source}, should_call_tools={should_call_tools}, recommended_tool_family={recommended_tool_family}. Decision tree: (1) If file mutation is required, use file tools. (2) If enough information is already available, answer directly with no tool calls. (3) If information is missing, use local memory/workspace tools for local facts and web tools only for online/current facts. Meta/control or tooling status-check turns are direct-answer turns and should not trigger web tools.",
        ),
        4_000,
    )
}

fn chat_ui_has_explicit_web_intent(lowered: &str) -> bool {
    lowered.contains("web search")
        || lowered.contains("websearch")
        || lowered.contains("search the web")
        || lowered.contains("search online")
        || lowered.contains("find information")
        || lowered.contains("finding information")
        || lowered.contains("look it up")
        || lowered.contains("look this up")
        || lowered.contains("search again")
        || lowered.contains("best chili recipes")
}

fn chat_ui_is_meta_diagnostic_request(lowered: &str) -> bool {
    if lowered.is_empty() {
        return false;
    }
    if chat_ui_has_explicit_web_intent(lowered) {
        return false;
    }
    if [
        "that was just a test",
        "that was a test",
        "did you do the web request",
        "did you try it",
        "where did that come from",
        "where the hell did that come from",
        "you returned no result",
        "you hallucinated",
        "answer the question",
    ]
    .iter()
    .any(|marker| lowered.contains(*marker))
    {
        return true;
    }
    let meta_hits = [
        "what happened",
        "workflow",
        "tool call",
        "web tooling",
        "hallucination",
        "hallucinated",
        "training data",
        "context issue",
        "last response",
        "previous response",
        "system issue",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    if meta_hits == 0 {
        return false;
    }
    let signal_terms = lowered
        .split_whitespace()
        .filter(|token| token.len() >= 3)
        .count();
    meta_hits >= 2 || signal_terms <= 7
}

fn chat_ui_requests_live_web(raw_input: &str) -> bool {
    if chat_ui_turn_is_meta_control_message(raw_input) {
        return false;
    }
    if chat_ui_message_is_tooling_status_check(raw_input) {
        return false;
    }
    let lowered = clean(raw_input, 2_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    if chat_ui_has_explicit_web_intent(&lowered) {
        return true;
    }
    if chat_ui_is_meta_diagnostic_request(&lowered) {
        return false;
    }
    ((lowered.contains("framework") || lowered.contains("frameworks"))
        && (lowered.contains("current")
            || lowered.contains("latest")
            || lowered.contains("top")
            || lowered.contains("best")))
        || (lowered.contains("search")
            && (lowered.contains("latest")
                || lowered.contains("current")
                || lowered.contains("framework")
                || lowered.contains("recipes")
                || lowered.contains("update")))
}

fn chat_ui_extract_web_query(raw_input: &str) -> String {
    let cleaned = clean(raw_input, 600);
    if cleaned.is_empty() {
        return "latest public web updates".to_string();
    }
    if let Some(start) = cleaned.find('"') {
        if let Some(end_rel) = cleaned[start + 1..].find('"') {
            let quoted = clean(&cleaned[start + 1..start + 1 + end_rel], 320);
            if !quoted.is_empty() {
                return quoted;
            }
        }
    }
    let lowered = cleaned.to_ascii_lowercase();
    for marker in ["about ", "for "] {
        if let Some(idx) = lowered.rfind(marker) {
            let candidate = clean(&cleaned[idx + marker.len()..], 320);
            if !candidate.is_empty() {
                return candidate;
            }
        }
    }
    cleaned
}

fn chat_ui_query_alignment_terms(text: &str, max_terms: usize) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for token in clean(text, 2_000)
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
    {
        if token.len() < 3 {
            continue;
        }
        if matches!(
            token,
            "the"
                | "and"
                | "for"
                | "with"
                | "this"
                | "that"
                | "from"
                | "into"
                | "what"
                | "when"
                | "where"
                | "why"
                | "how"
                | "about"
                | "just"
                | "again"
                | "please"
                | "best"
                | "top"
                | "give"
                | "show"
                | "find"
                | "search"
                | "web"
                | "results"
                | "result"
        ) {
            continue;
        }
        if out.iter().any(|existing| existing == token) {
            continue;
        }
        out.push(token.to_string());
        if out.len() >= max_terms {
            break;
        }
    }
    out
}

fn chat_ui_web_result_matches_query(query: &str, output: &str) -> bool {
    let query_terms = chat_ui_query_alignment_terms(query, 16);
    if query_terms.len() < 2 {
        return true;
    }
    let lowered = clean(output, 4_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let matched = query_terms
        .iter()
        .filter(|term| lowered.contains(term.as_str()))
        .count();
    let required_hits = 2.min(query_terms.len());
    if matched >= required_hits {
        return true;
    }
    let ratio = (matched as f64) / (query_terms.len() as f64);
    let ratio_floor = if query_terms.len() >= 6 { 0.40 } else { 0.34 };
    ratio >= ratio_floor
}

fn chat_ui_tool_name_is_web_search(name: &str) -> bool {
    let lowered = clean(name, 120).to_ascii_lowercase();
    lowered.contains("web_search")
        || lowered.contains("search_web")
        || lowered.contains("web_query")
        || lowered.contains("batch_query")
        || lowered == "search"
        || lowered.contains("web_fetch")
}

fn chat_ui_web_search_call_count(tools: &[Value]) -> usize {
    tools
        .iter()
        .filter(|row| {
            chat_ui_tool_name_is_web_search(
                row.get("name")
                    .or_else(|| row.get("tool"))
                    .or_else(|| row.get("type"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            )
        })
        .count()
}

fn chat_ui_read_jsonl_rows(path: &Path, max_rows: usize) -> Vec<Value> {
    let raw = fs::read_to_string(path).unwrap_or_default();
    if raw.is_empty() {
        return Vec::new();
    }
    let mut rows = Vec::<Value>::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(line) {
            rows.push(value);
        }
    }
    if rows.len() > max_rows {
        rows.split_off(rows.len().saturating_sub(max_rows))
    } else {
        rows
    }
}

fn chat_ui_semantic_receipt_summary(
    diagnostics: &Value,
    requires_live_web: bool,
    message: &str,
) -> String {
    let total_calls = diagnostics
        .get("total_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let successful_calls = diagnostics
        .get("successful_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let failed_calls = diagnostics
        .get("failed_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let blocked_calls = diagnostics
        .get("blocked_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let not_found_calls = diagnostics
        .get("not_found_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let low_signal_calls = diagnostics
        .get("low_signal_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let silent_failure_calls = diagnostics
        .get("silent_failure_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let surface_unavailable_calls = diagnostics
        .get("surface_unavailable_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let surface_degraded_calls = diagnostics
        .get("surface_degraded_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let error_codes = diagnostics
        .get("error_codes")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let has_surface_unavailable =
        surface_unavailable_calls > 0 || error_codes.contains_key("web_tool_surface_unavailable");
    let has_surface_degraded =
        surface_degraded_calls > 0 || error_codes.contains_key("web_tool_surface_degraded");
    let status = if requires_live_web && has_surface_unavailable {
        "failed"
    } else if requires_live_web && has_surface_degraded {
        "degraded"
    } else if requires_live_web && total_calls <= 0 {
        "failed"
    } else if failed_calls == 0 && silent_failure_calls == 0 {
        "complete"
    } else if successful_calls > 0 {
        "degraded"
    } else {
        "failed"
    };
    let intent = if requires_live_web {
        chat_ui_extract_web_query(message)
    } else {
        clean(message, 140)
    };
    clean(
        &format!(
            "Tool transaction {} for intent \"{}\": total={} success={} failed={} blocked={} not_found={} low_signal={} surface_unavailable={} surface_degraded={} silent_failure={}.",
            status,
            intent,
            total_calls,
            successful_calls,
            failed_calls,
            blocked_calls,
            not_found_calls,
            low_signal_calls,
            surface_unavailable_calls,
            surface_degraded_calls,
            silent_failure_calls
        ),
        600,
    )
}

#[cfg(test)]
fn scripted_batch_query_harness_response(root: &Path, query: &str) -> Option<Value> {
    let path = root.join("client/runtime/local/state/ui/infring_dashboard/test_chat_script.json");
    let mut script = read_json(&path).unwrap_or_else(|| json!({}));
    let step = script
        .get_mut("batch_query_queue")
        .and_then(Value::as_array_mut)
        .and_then(|queue| {
            if queue.is_empty() {
                None
            } else {
                Some(queue.remove(0))
            }
        });
    let mut payload = step?;
    if !payload.is_object() {
        payload = json!({});
    }
    if payload.get("type").is_none() {
        payload["type"] = json!("batch_query");
    }
    if payload.get("query").is_none() {
        payload["query"] = json!(clean(query, 320));
    }
    if let Some(obj) = script.as_object_mut() {
        let calls = obj
            .entry("batch_query_calls".to_string())
            .or_insert_with(|| json!([]));
        if let Some(rows) = calls.as_array_mut() {
            rows.push(json!({
                "query": clean(query, 320)
            }));
        }
    }
    let _ = write_json(&path, &script);
    Some(payload)
}

#[cfg(test)]
mod chat_ui_direct_path_tests {
    use super::*;
    use std::fs;

    fn write_chat_script(root: &Path, payload: &Value) {
        let path = root.join("client/runtime/local/state/ui/infring_dashboard/test_chat_script.json");
        let parent = path.parent().expect("chat script parent");
        fs::create_dir_all(parent).expect("mkdir chat script");
        fs::write(path, serde_json::to_string_pretty(payload).expect("chat script json"))
            .expect("write chat script");
    }

    #[test]
    fn direct_run_chat_ui_forces_web_tool_attempt_for_explicit_chili_prompt() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "I don't have web search capabilities.",
                        "tools": []
                    }
                ],
                "batch_query_queue": [
                    {
                        "ok": true,
                        "type": "batch_query",
                        "status": "ok",
                        "summary": "Key findings: allrecipes.com: Best Damn Chili Recipe.",
                        "evidence_refs": [
                            {
                                "locator": "https://www.allrecipes.com/recipe/233613/best-damn-chili/"
                            }
                        ]
                    }
                ]
            }),
        );

        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--app=chat-ui".to_string(),
            "--session-id=direct-web-parity".to_string(),
            "--message=well try doing a web search and returning the results. make the websearch about best chili recipes".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        let response = payload
            .pointer("/turn/assistant")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        assert!(
            response.contains("web search results for")
                && response.contains("best chili recipes"),
            "{response}"
        );
        let tools = payload
            .get("tools")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(tools.iter().any(|row| {
            clean(
                row.get("name")
                    .or_else(|| row.get("tool"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                80,
            )
            .to_ascii_lowercase()
                == "batch_query"
        }));
        let invariant = payload
            .pointer("/response_finalization/web_invariant")
            .cloned()
            .unwrap_or_else(|| json!({}));
        assert_eq!(
            invariant.get("classification").and_then(Value::as_str),
            Some("healthy")
        );
        assert_eq!(
            invariant.get("tool_attempted").and_then(Value::as_bool),
            Some(true)
        );
        assert!(
            invariant
                .get("web_search_calls")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= 1
        );
        let discovery = payload
            .pointer("/response_finalization/capability_discovery")
            .cloned()
            .unwrap_or_else(|| json!({}));
        assert_eq!(
            discovery.get("contract").and_then(Value::as_str),
            Some("tool_execution_receipt_v1")
        );
        assert!(discovery
            .get("execution_statuses")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| row.as_str() == Some("unknown")))
            .unwrap_or(false));
        let summary = payload
            .pointer("/response_finalization/tool_receipt_summary")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(summary.to_ascii_lowercase().contains("tool transaction complete"));
        let transaction = payload
            .pointer("/response_finalization/tool_transaction")
            .cloned()
            .unwrap_or_else(|| json!({}));
        assert_eq!(
            transaction.get("complete").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            transaction.get("status").and_then(Value::as_str),
            Some("complete")
        );
    }

    #[test]
    fn chat_ui_finalization_fail_closes_when_tool_surface_is_unavailable() {
        let (assistant, outcome) = finalize_chat_ui_assistant_response(
            "search current top agent frameworks",
            "I'll get you an update on the current best AI agent frameworks.",
            &[json!({
                "name": "batch_query",
                "status": "error",
                "error": "web_search_tool_surface_unavailable"
            })],
        );
        assert_eq!(outcome, "tool_surface_error_fail_closed");
        let lowered = assistant.to_ascii_lowercase();
        assert!(
            lowered.contains("web tool surface is unavailable"),
            "{assistant}"
        );
    }

    #[test]
    fn direct_run_chat_ui_surfaces_tool_surface_unavailable_error_and_classification() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "working on it",
                        "tools": [
                            {
                                "name": "batch_query",
                                "status": "error",
                                "error": "web_search_tool_surface_unavailable"
                            }
                        ]
                    }
                ]
            }),
        );

        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--app=chat-ui".to_string(),
            "--session-id=surface-unavailable".to_string(),
            "--message=try searching for current top agent frameworks".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload.get("error").and_then(Value::as_str),
            Some("web_tool_surface_unavailable")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/classification")
                .and_then(Value::as_str),
            Some("tool_surface_unavailable")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/status")
                .and_then(Value::as_str),
            Some("failed")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/outcome")
                .and_then(Value::as_str),
            Some("forced_web_tool_surface_unavailable")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/fail_closed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload
                .pointer("/web_tooling_fallback/reason")
                .and_then(Value::as_str),
            Some("detected_tool_surface_error")
        );
        let assistant = payload
            .pointer("/turn/assistant")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(
            assistant
                .to_ascii_lowercase()
                .contains("web tool surface is unavailable"),
            "{assistant}"
        );
    }

    #[test]
    fn chat_ui_finalization_fail_closes_when_tool_surface_is_degraded() {
        let (assistant, outcome) = finalize_chat_ui_assistant_response(
            "search current top agent frameworks",
            "let me check that quickly",
            &[json!({
                "name": "batch_query",
                "status": "error",
                "error": "web_search_tool_surface_degraded"
            })],
        );
        assert_eq!(outcome, "tool_surface_error_fail_closed");
        let lowered = assistant.to_ascii_lowercase();
        assert!(lowered.contains("web tool surface is degraded"), "{assistant}");
    }

    #[test]
    fn direct_run_chat_ui_surfaces_tool_surface_degraded_error_and_classification() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "working on it",
                        "tools": [
                            {
                                "name": "batch_query",
                                "status": "error",
                                "error": "web_search_tool_surface_degraded"
                            }
                        ]
                    }
                ]
            }),
        );

        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--app=chat-ui".to_string(),
            "--session-id=surface-degraded".to_string(),
            "--message=try searching for current top agent frameworks".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload.get("error").and_then(Value::as_str),
            Some("web_tool_surface_degraded")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/classification")
                .and_then(Value::as_str),
            Some("tool_surface_degraded")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/status")
                .and_then(Value::as_str),
            Some("degraded")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/outcome")
                .and_then(Value::as_str),
            Some("forced_web_tool_surface_degraded")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/fail_closed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/retry/recommended")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/retry/strategy")
                .and_then(Value::as_str),
            Some("retry_with_backoff")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/retry/plan/auto")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/retry/plan/attempts")
                .and_then(Value::as_i64),
            Some(2)
        );
        assert_eq!(
            payload
                .pointer("/web_tooling_fallback/reason")
                .and_then(Value::as_str),
            Some("detected_tool_surface_error")
        );
        let assistant = payload
            .pointer("/turn/assistant")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(
            assistant
                .to_ascii_lowercase()
                .contains("web tool surface is degraded"),
            "{assistant}"
        );
    }

    #[test]
    fn chat_ui_tool_surface_detector_prioritizes_unavailable_over_degraded() {
        let code = chat_ui_detect_tool_surface_error_code(&[
            json!({
                "name": "batch_query",
                "status": "error",
                "error": "web_search_tool_surface_degraded"
            }),
            json!({
                "name": "batch_query",
                "status": "error",
                "error": "web_search_tool_surface_unavailable"
            }),
        ]);
        assert_eq!(code, Some("web_tool_surface_unavailable"));
    }

    #[test]
    fn direct_run_chat_ui_mixed_surface_signals_report_unavailable_canonically() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "working on it",
                        "tools": [
                            {
                                "name": "batch_query",
                                "status": "error",
                                "error": "web_search_tool_surface_degraded"
                            },
                            {
                                "name": "batch_query",
                                "status": "error",
                                "error": "web_fetch_tool_surface_unavailable"
                            }
                        ]
                    }
                ]
            }),
        );
        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--app=chat-ui".to_string(),
            "--session-id=surface-mixed-priority".to_string(),
            "--message=try searching for current top agent frameworks".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload.get("error").and_then(Value::as_str),
            Some("web_tool_surface_unavailable")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_surface_error_code")
                .and_then(Value::as_str),
            Some("web_tool_surface_unavailable")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/web_invariant/tool_surface_error_code")
                .and_then(Value::as_str),
            Some("web_tool_surface_unavailable")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/classification")
                .and_then(Value::as_str),
            Some("tool_surface_unavailable")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/outcome")
                .and_then(Value::as_str),
            Some("forced_web_tool_surface_unavailable")
        );
    }

    #[test]
    fn chat_ui_tool_diagnostics_emits_explicit_execution_receipts() {
        let diagnostics = chat_ui_tool_diagnostics(&[
            json!({"name":"batch_query","status":"ok","ok":true,"result":"found docs"}),
            json!({"name":"parse_workspace","status":"failed","error":"tool not found"}),
            json!({"name":"spawn_subagents"}),
        ]);
        assert_eq!(
            diagnostics
                .get("not_found_calls")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            1
        );
        assert_eq!(
            diagnostics
                .get("silent_failure_calls")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            1
        );
        let receipts = diagnostics
            .get("execution_receipts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(receipts.len(), 3);
        assert!(receipts.iter().all(|row| {
            row.get("call_id")
                .and_then(Value::as_str)
                .map(|value| value.starts_with("toolcall_"))
                .unwrap_or(false)
        }));
        assert!(receipts.iter().any(|row| {
            row.get("tool").and_then(Value::as_str) == Some("parse_workspace")
                && row.get("status").and_then(Value::as_str) == Some("not_found")
        }));
        assert!(receipts.iter().any(|row| {
            row.get("tool").and_then(Value::as_str) == Some("spawn_subagents")
                && row.get("status").and_then(Value::as_str) == Some("unknown")
        }));
    }

    #[test]
    fn chat_ui_tool_diagnostics_preserves_surface_error_code_from_status_only_rows() {
        let diagnostics = chat_ui_tool_diagnostics(&[json!({
            "name": "batch_query",
            "status": "web_tool_surface_degraded",
            "ok": false,
            "error": ""
        })]);
        assert_eq!(
            diagnostics
                .pointer("/error_codes/web_tool_surface_degraded")
                .and_then(Value::as_i64),
            Some(1)
        );
        let receipts = diagnostics
            .get("execution_receipts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(receipts.iter().any(|row| {
            row.get("error_code").and_then(Value::as_str)
                == Some("web_tool_surface_degraded")
        }));
    }

    #[test]
    fn chat_ui_tool_diagnostics_preserves_surface_error_code_from_result_only_rows() {
        let diagnostics = chat_ui_tool_diagnostics(&[json!({
            "name": "batch_query",
            "status": "error",
            "ok": false,
            "error": "",
            "result": "provider failed: web_fetch_tool_surface_unavailable"
        })]);
        assert_eq!(
            diagnostics
                .pointer("/error_codes/web_tool_surface_unavailable")
                .and_then(Value::as_i64),
            Some(1)
        );
        let receipts = diagnostics
            .get("execution_receipts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(receipts.iter().any(|row| {
            row.get("error_code").and_then(Value::as_str)
                == Some("web_tool_surface_unavailable")
                && row.get("status").and_then(Value::as_str) == Some("error")
        }));
        assert_eq!(
            diagnostics
                .get("surface_unavailable_calls")
                .and_then(Value::as_i64),
            Some(1)
        );
    }

    #[test]
    fn chat_ui_tool_diagnostics_counts_surface_classes_separately() {
        let diagnostics = chat_ui_tool_diagnostics(&[
            json!({
                "name": "batch_query",
                "status": "web_tool_surface_unavailable",
                "ok": false
            }),
            json!({
                "name": "batch_query",
                "status": "web_tool_surface_degraded",
                "ok": false
            }),
        ]);
        assert_eq!(
            diagnostics
                .get("surface_unavailable_calls")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            diagnostics
                .get("surface_degraded_calls")
                .and_then(Value::as_i64),
            Some(1)
        );
    }

    #[test]
    fn chat_ui_tool_diagnostics_treats_policy_denied_status_as_blocked() {
        let diagnostics = chat_ui_tool_diagnostics(&[json!({
            "name": "batch_query",
            "status": "policy_denied",
            "ok": false
        })]);
        assert_eq!(
            diagnostics
                .get("blocked_calls")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            diagnostics
                .pointer("/error_codes/web_tool_policy_blocked")
                .and_then(Value::as_i64),
            Some(1)
        );
        let receipts = diagnostics
            .get("execution_receipts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(receipts.iter().any(|row| {
            row.get("status").and_then(Value::as_str) == Some("blocked")
                && row.get("error_code").and_then(Value::as_str) == Some("web_tool_policy_blocked")
        }));
    }

    #[test]
    fn direct_run_chat_ui_classifies_policy_denied_status_as_policy_blocked() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "working on it",
                        "tools": [
                            {
                                "name": "batch_query",
                                "status": "policy_denied",
                                "ok": false
                            }
                        ]
                    }
                ]
            }),
        );
        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--app=chat-ui".to_string(),
            "--session-id=policy-denied-classification".to_string(),
            "--message=search for current top agent frameworks".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/classification")
                .and_then(Value::as_str),
            Some("policy_blocked")
        );
        assert_eq!(
            payload.get("error").and_then(Value::as_str),
            Some("web_tool_policy_blocked")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_diagnostics/blocked_calls")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_diagnostics/error_codes/web_tool_policy_blocked")
                .and_then(Value::as_i64),
            Some(1)
        );
    }

    #[test]
    fn chat_ui_tool_diagnostics_treats_provider_low_signal_status_as_low_signal() {
        let diagnostics = chat_ui_tool_diagnostics(&[json!({
            "name": "batch_query",
            "status": "provider_low_signal",
            "ok": false
        })]);
        assert_eq!(
            diagnostics
                .get("low_signal_calls")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            diagnostics
                .pointer("/error_codes/web_tool_low_signal")
                .and_then(Value::as_i64),
            Some(1)
        );
        let receipts = diagnostics
            .get("execution_receipts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(receipts.iter().any(|row| {
            row.get("status").and_then(Value::as_str) == Some("low_signal")
                && row.get("error_code").and_then(Value::as_str) == Some("web_tool_low_signal")
        }));
    }

    #[test]
    fn direct_run_chat_ui_classifies_provider_low_signal_status_as_low_signal() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "working on it",
                        "tools": [
                            {
                                "name": "batch_query",
                                "status": "provider_low_signal",
                                "ok": false
                            }
                        ]
                    }
                ]
            }),
        );
        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--app=chat-ui".to_string(),
            "--session-id=provider-low-signal-classification".to_string(),
            "--message=search for current top agent frameworks".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/classification")
                .and_then(Value::as_str),
            Some("low_signal")
        );
        assert_eq!(
            payload.get("error").and_then(Value::as_str),
            Some("web_tool_low_signal")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/status")
                .and_then(Value::as_str),
            Some("degraded")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_diagnostics/low_signal_calls")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_diagnostics/error_codes/web_tool_low_signal")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/consistent")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn chat_ui_tool_diagnostics_treats_unknown_tool_not_found_result_as_not_found() {
        let diagnostics = chat_ui_tool_diagnostics(&[json!({
            "name": "batch_query",
            "status": "unknown",
            "ok": false,
            "result": "tool not found: batch_query is unavailable"
        })]);
        assert_eq!(
            diagnostics
                .get("not_found_calls")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            diagnostics
                .pointer("/error_codes/web_tool_not_found")
                .and_then(Value::as_i64),
            Some(1)
        );
        let receipts = diagnostics
            .get("execution_receipts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(receipts.iter().any(|row| {
            row.get("status").and_then(Value::as_str) == Some("not_found")
                && row.get("error_code").and_then(Value::as_str) == Some("web_tool_not_found")
        }));
    }

    #[test]
    fn direct_run_chat_ui_classifies_unknown_tool_not_found_result_as_tool_not_found() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "working on it",
                        "tools": [
                            {
                                "name": "batch_query",
                                "status": "unknown",
                                "ok": false,
                                "result": "tool not found: batch_query is unavailable"
                            }
                        ]
                    }
                ]
            }),
        );
        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--app=chat-ui".to_string(),
            "--session-id=unknown-not-found-classification".to_string(),
            "--message=search for current top agent frameworks".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/classification")
                .and_then(Value::as_str),
            Some("tool_not_found")
        );
        assert_eq!(
            payload.get("error").and_then(Value::as_str),
            Some("web_tool_not_found")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/status")
                .and_then(Value::as_str),
            Some("failed")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_diagnostics/not_found_calls")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_diagnostics/error_codes/web_tool_not_found")
                .and_then(Value::as_i64),
            Some(1)
        );
    }

    #[test]
    fn direct_run_chat_ui_not_invoked_without_findings_fail_closes_via_classification_guard() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "analysis pending",
                        "tools": [
                            {
                                "name": "parse_workspace",
                                "status": "ok",
                                "ok": true,
                                "result": ""
                            }
                        ]
                    }
                ]
            }),
        );
        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--app=chat-ui".to_string(),
            "--session-id=classification-guard-not-invoked".to_string(),
            "--message=search for current top agent frameworks".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload
                .get("error")
                .and_then(Value::as_str),
            Some("web_tool_not_invoked")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/outcome")
                .and_then(Value::as_str),
            Some("classification_guard_not_invoked_fail_closed")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/classification")
                .and_then(Value::as_str),
            Some("tool_not_invoked")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/hard_guard/source")
                .and_then(Value::as_str),
            Some("classification_guard")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/not_invoked_fail_closed")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/fail_closed")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/applied")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/fail_closed_class")
                .and_then(Value::as_str),
            Some("tool_not_invoked")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/mode")
                .and_then(Value::as_str),
            Some("fail_close")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/active_error_code")
                .and_then(Value::as_str),
            Some("web_tool_not_invoked")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_recommended")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_strategy")
                .and_then(Value::as_str),
            Some("rerun_with_tool_call")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_lane")
                .and_then(Value::as_str),
            Some("immediate")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_plan/auto")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_plan/attempts")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert!(
            payload
                .pointer("/turn/assistant")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase()
                .contains("before any search tool call was recorded")
        );
    }

    #[test]
    fn direct_run_chat_ui_low_signal_without_findings_fail_closes_via_classification_guard() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "analysis pending",
                        "tools": [
                            {
                                "name": "batch_query",
                                "status": "provider_low_signal",
                                "ok": false,
                                "result": ""
                            }
                        ]
                    }
                ]
            }),
        );
        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--app=chat-ui".to_string(),
            "--session-id=classification-guard-low-signal".to_string(),
            "--message=search for current top agent frameworks".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload.get("error").and_then(Value::as_str),
            Some("web_tool_low_signal")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/outcome")
                .and_then(Value::as_str),
            Some("classification_guard_low_signal_fail_closed")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/hard_guard/source")
                .and_then(Value::as_str),
            Some("classification_guard")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/hard_guard/status")
                .and_then(Value::as_str),
            Some("provider_low_signal")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/fail_closed")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/applied")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/fail_closed_class")
                .and_then(Value::as_str),
            Some("low_signal")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/mode")
                .and_then(Value::as_str),
            Some("fail_close")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/active_error_code")
                .and_then(Value::as_str),
            Some("web_tool_low_signal")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_recommended")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_strategy")
                .and_then(Value::as_str),
            Some("narrow_query")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_lane")
                .and_then(Value::as_str),
            Some("immediate")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_plan/auto")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_plan/attempts")
                .and_then(Value::as_i64),
            Some(1)
        );
    }

    #[test]
    fn direct_run_chat_ui_healthy_with_findings_does_not_apply_classification_guard() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "Here are current top agentic AI frameworks.",
                        "tools": [
                            {
                                "name": "batch_query",
                                "status": "ok",
                                "ok": true,
                                "result": "LangGraph docs captured; OpenAI Agents SDK docs captured"
                            }
                        ]
                    }
                ]
            }),
        );
        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--app=chat-ui".to_string(),
            "--session-id=classification-guard-healthy".to_string(),
            "--message=search for current top agent frameworks".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/classification")
                .and_then(Value::as_str),
            Some("healthy")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/applied")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/fail_closed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/mode")
                .and_then(Value::as_str),
            Some("none")
        );
        assert_eq!(
            payload.pointer("/response_finalization/classification_guard/active_error_code"),
            Some(&Value::Null)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_recommended")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_strategy")
                .and_then(Value::as_str),
            Some("none")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_lane")
                .and_then(Value::as_str),
            Some("none")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_plan/attempts")
                .and_then(Value::as_i64),
            Some(0)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/complete")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(payload.get("error"), None);
    }

    #[test]
    fn direct_run_chat_ui_placeholder_with_policy_blocked_uses_policy_guard_fallback() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "I'll get you an update on the current best AI agent frameworks.",
                        "tools": [
                            {
                                "name": "batch_query",
                                "status": "policy_denied",
                                "ok": false
                            }
                        ]
                    }
                ]
            }),
        );
        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--app=chat-ui".to_string(),
            "--session-id=placeholder-policy-blocked".to_string(),
            "--message=search for current top agent frameworks".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload.get("error").and_then(Value::as_str),
            Some("web_tool_policy_blocked")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/hard_guard/status")
                .and_then(Value::as_str),
            Some("policy_blocked")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/hard_guard/error_code")
                .and_then(Value::as_str),
            Some("web_tool_policy_blocked")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/outcome")
                .and_then(Value::as_str),
            Some("classification_guard_policy_blocked_fail_closed")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/applied")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/fail_closed")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/fail_closed_class")
                .and_then(Value::as_str),
            Some("policy_blocked")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/active_error_code")
                .and_then(Value::as_str),
            Some("web_tool_policy_blocked")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_recommended")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_strategy")
                .and_then(Value::as_str),
            Some("operator_policy_action")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_lane")
                .and_then(Value::as_str),
            Some("blocked")
        );
    }

    #[test]
    fn direct_run_chat_ui_not_found_without_findings_fail_closes_via_classification_guard() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "analysis pending",
                        "tools": [
                            {
                                "name": "batch_query",
                                "status": "unknown",
                                "ok": false,
                                "result": "tool not found: batch_query is unavailable"
                            }
                        ]
                    }
                ]
            }),
        );
        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--app=chat-ui".to_string(),
            "--session-id=classification-guard-tool-not-found".to_string(),
            "--message=search for current top agent frameworks".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload.get("error").and_then(Value::as_str),
            Some("web_tool_not_found")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/outcome")
                .and_then(Value::as_str),
            Some("classification_guard_tool_not_found_fail_closed")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/hard_guard/source")
                .and_then(Value::as_str),
            Some("classification_guard")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/hard_guard/status")
                .and_then(Value::as_str),
            Some("failed")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/applied")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/fail_closed")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/fail_closed_class")
                .and_then(Value::as_str),
            Some("tool_not_found")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/active_error_code")
                .and_then(Value::as_str),
            Some("web_tool_not_found")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_recommended")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_strategy")
                .and_then(Value::as_str),
            Some("adjust_tool_selection")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_lane")
                .and_then(Value::as_str),
            Some("blocked")
        );
    }

    #[test]
    fn chat_ui_receipt_summary_marks_missing_required_web_calls_as_failed() {
        let summary = chat_ui_semantic_receipt_summary(
            &json!({
                "total_calls": 0,
                "successful_calls": 0,
                "failed_calls": 0,
                "blocked_calls": 0,
                "not_found_calls": 0,
                "low_signal_calls": 0,
                "silent_failure_calls": 0
            }),
            true,
            "latest agent frameworks",
        );
        assert!(
            summary.to_ascii_lowercase().contains("tool transaction failed"),
            "{summary}"
        );
    }

    #[test]
    fn chat_ui_receipt_summary_marks_surface_degraded_as_degraded() {
        let summary = chat_ui_semantic_receipt_summary(
            &json!({
                "total_calls": 1,
                "successful_calls": 0,
                "failed_calls": 1,
                "blocked_calls": 0,
                "not_found_calls": 0,
                "low_signal_calls": 0,
                "silent_failure_calls": 0,
                "error_codes": {
                    "web_tool_surface_degraded": 1
                }
            }),
            true,
            "latest agent frameworks",
        );
        assert!(
            summary.to_ascii_lowercase().contains("tool transaction degraded"),
            "{summary}"
        );
    }

    #[test]
    fn chat_ui_receipt_summary_marks_surface_unavailable_as_failed() {
        let summary = chat_ui_semantic_receipt_summary(
            &json!({
                "total_calls": 1,
                "successful_calls": 0,
                "failed_calls": 1,
                "blocked_calls": 0,
                "not_found_calls": 0,
                "low_signal_calls": 0,
                "silent_failure_calls": 0,
                "error_codes": {
                    "web_tool_surface_unavailable": 1
                }
            }),
            true,
            "latest agent frameworks",
        );
        assert!(
            summary.to_ascii_lowercase().contains("tool transaction failed"),
            "{summary}"
        );
        assert!(
            summary
                .to_ascii_lowercase()
                .contains("surface_unavailable=1"),
            "{summary}"
        );
    }

    #[test]
    fn chat_ui_placeholder_rewrite_returns_canonical_error_copy() {
        let (rewritten, outcome) = rewrite_chat_ui_placeholder_with_tool_diagnostics(
            "Web search completed.",
            &json!({
                "total_calls": 1,
                "error_codes": {
                    "web_tool_auth_missing": 1
                }
            }),
        );
        assert_eq!(outcome, "placeholder_replaced_auth");
        let lowered = rewritten.to_ascii_lowercase();
        assert!(lowered.contains("web_status: auth_missing"), "{rewritten}");
        assert!(lowered.contains("error_code: web_tool_auth_missing"), "{rewritten}");
    }

    #[test]
    fn chat_ui_placeholder_rewrite_prioritizes_surface_unavailable_copy() {
        let (rewritten, outcome) = rewrite_chat_ui_placeholder_with_tool_diagnostics(
            "Web search completed.",
            &json!({
                "total_calls": 1,
                "error_codes": {
                    "web_tool_surface_unavailable": 1,
                    "web_tool_error": 1
                }
            }),
        );
        assert_eq!(outcome, "placeholder_replaced_surface_unavailable");
        assert!(
            rewritten
                .to_ascii_lowercase()
                .contains("web tool surface is unavailable"),
            "{rewritten}"
        );
    }

    #[test]
    fn chat_ui_placeholder_rewrite_prioritizes_surface_degraded_copy() {
        let (rewritten, outcome) = rewrite_chat_ui_placeholder_with_tool_diagnostics(
            "Web search completed.",
            &json!({
                "total_calls": 1,
                "error_codes": {
                    "web_tool_surface_degraded": 1
                }
            }),
        );
        assert_eq!(outcome, "placeholder_replaced_surface_degraded");
        assert!(
            rewritten
                .to_ascii_lowercase()
                .contains("web tool surface is degraded"),
            "{rewritten}"
        );
    }

    #[test]
    fn chat_ui_view_logs_returns_trace_matches_for_request_id() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "done",
                        "tools": [
                            {"name":"batch_query","status":"ok","ok":true,"result":"found"}
                        ]
                    }
                ]
            }),
        );
        let run_payload = run_chat_ui(
            root.path(),
            &crate::parse_args(&[
                "run".to_string(),
                "--app=chat-ui".to_string(),
                "--session-id=view-logs-demo".to_string(),
                "--message=search docs".to_string(),
                "--strict=1".to_string(),
            ]),
            true,
            "run",
        );
        let trace_id = run_payload
            .get("trace_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        assert!(!trace_id.is_empty());
        let logs_payload = run_chat_ui(
            root.path(),
            &crate::parse_args(&[
                "run".to_string(),
                "--app=chat-ui".to_string(),
                "--session-id=view-logs-demo".to_string(),
                format!("--request-id={trace_id}"),
                "--strict=1".to_string(),
            ]),
            true,
            "view-logs",
        );
        assert_eq!(logs_payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            logs_payload.get("match_count").and_then(Value::as_u64),
            Some(1)
        );
        assert!(logs_payload
            .get("matches")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn chat_ui_history_includes_semantic_tool_receipt_summary() {
        let session = json!({
            "turns": [
                {
                    "user": "search rust tracing",
                    "assistant": "done",
                    "tool_summary": "Tool transaction complete for intent \"search rust tracing\": total=1 success=1 failed=0 blocked=0 not_found=0 low_signal=0 silent_failure=0."
                }
            ]
        });
        let history = chat_ui_history_messages(&session);
        assert!(history.iter().any(|row| {
            row.get("role").and_then(Value::as_str) == Some("assistant")
                && row
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_ascii_lowercase()
                    .contains("tool receipt summary")
        }));
    }
}

fn tool_name_for_diagnostics(row: &Value) -> String {
    clean(
        row.get("tool")
            .or_else(|| row.get("name"))
            .or_else(|| row.get("type"))
            .or_else(|| row.pointer("/tool/name"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    )
    .to_ascii_lowercase()
}

fn tool_findings_count(row: &Value) -> usize {
    for key in ["findings", "results", "items", "citations", "sources"] {
        if let Some(count) = row
            .get(key)
            .or_else(|| row.pointer(&format!("/result/{key}")))
            .and_then(Value::as_array)
            .map(|rows| rows.len())
        {
            return count;
        }
    }
    0
}

fn chat_ui_surface_error_code_hint_from_row(row: &Value) -> Option<String> {
    let mut saw_degraded = false;
    let mut saw_unavailable = false;
    for candidate in [
        clean(row.get("status").and_then(Value::as_str).unwrap_or(""), 240),
        clean(row.get("error").and_then(Value::as_str).unwrap_or(""), 240),
        clean(row.get("result").and_then(Value::as_str).unwrap_or(""), 1_200),
        chat_ui_tool_text_blob(row),
    ] {
        let code = crate::tool_output_match_filter::normalize_web_tooling_error_code(&candidate);
        if code == "web_tool_surface_unavailable" {
            saw_unavailable = true;
        } else if code == "web_tool_surface_degraded" {
            saw_degraded = true;
        }
    }
    if saw_unavailable {
        Some("web_tool_surface_unavailable".to_string())
    } else if saw_degraded {
        Some("web_tool_surface_degraded".to_string())
    } else {
        None
    }
}

fn chat_ui_policy_blocked_hint_from_row(row: &Value) -> bool {
    for candidate in [
        clean(row.get("status").and_then(Value::as_str).unwrap_or(""), 240),
        clean(row.get("error").and_then(Value::as_str).unwrap_or(""), 240),
        clean(row.get("result").and_then(Value::as_str).unwrap_or(""), 1_200),
        chat_ui_tool_text_blob(row),
    ] {
        let code = crate::tool_output_match_filter::normalize_web_tooling_error_code(&candidate);
        if code == "web_tool_policy_blocked" {
            return true;
        }
    }
    false
}

fn chat_ui_low_signal_hint_from_row(row: &Value) -> bool {
    for candidate in [
        clean(row.get("status").and_then(Value::as_str).unwrap_or(""), 240),
        clean(row.get("error").and_then(Value::as_str).unwrap_or(""), 240),
        clean(row.get("result").and_then(Value::as_str).unwrap_or(""), 1_200),
        chat_ui_tool_text_blob(row),
    ] {
        let code = crate::tool_output_match_filter::normalize_web_tooling_error_code(&candidate);
        if code == "web_tool_low_signal" {
            return true;
        }
    }
    false
}

fn chat_ui_not_found_hint_from_row(row: &Value) -> bool {
    for candidate in [
        clean(row.get("status").and_then(Value::as_str).unwrap_or(""), 240),
        clean(row.get("error").and_then(Value::as_str).unwrap_or(""), 240),
        clean(row.get("result").and_then(Value::as_str).unwrap_or(""), 1_200),
        chat_ui_tool_text_blob(row),
    ] {
        let code = crate::tool_output_match_filter::normalize_web_tooling_error_code(&candidate);
        if code == "web_tool_not_found" {
            return true;
        }
    }
    false
}

fn chat_ui_receipt_status_count(diagnostics: &Value, status: &str) -> i64 {
    diagnostics
        .get("execution_receipts")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter(|row| {
                    clean(row.get("status").and_then(Value::as_str).unwrap_or(""), 64)
                        .eq_ignore_ascii_case(status)
                })
                .count() as i64
        })
        .unwrap_or(0)
}

fn chat_ui_receipt_has_error_code(diagnostics: &Value, error_code: &str) -> bool {
    diagnostics
        .get("execution_receipts")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter().any(|row| {
                clean(row.get("error_code").and_then(Value::as_str).unwrap_or(""), 128)
                    .eq_ignore_ascii_case(error_code)
            })
        })
        .unwrap_or(false)
}

fn chat_ui_expected_classification_from_diagnostics(
    diagnostics: &Value,
    requires_live_web: bool,
    web_search_calls: i64,
) -> &'static str {
    let error_codes = diagnostics
        .get("error_codes")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let has_surface_unavailable = error_codes.contains_key("web_tool_surface_unavailable")
        || chat_ui_receipt_has_error_code(diagnostics, "web_tool_surface_unavailable");
    if has_surface_unavailable {
        return "tool_surface_unavailable";
    }
    let has_surface_degraded = error_codes.contains_key("web_tool_surface_degraded")
        || chat_ui_receipt_has_error_code(diagnostics, "web_tool_surface_degraded");
    if has_surface_degraded {
        return "tool_surface_degraded";
    }
    if requires_live_web && web_search_calls == 0 {
        return "tool_not_invoked";
    }
    let blocked_signal = error_codes.contains_key("web_tool_policy_blocked")
        || chat_ui_receipt_status_count(diagnostics, "blocked") > 0;
    if blocked_signal {
        return "policy_blocked";
    }
    let not_found_signal = error_codes.contains_key("web_tool_not_found")
        || chat_ui_receipt_status_count(diagnostics, "not_found") > 0;
    if not_found_signal {
        return "tool_not_found";
    }
    let loop_risk_signal = diagnostics
        .pointer("/loop_risk/detected")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if loop_risk_signal {
        return "low_signal";
    }
    let low_signal_signal = error_codes.contains_key("web_tool_low_signal")
        || chat_ui_receipt_status_count(diagnostics, "low_signal") > 0
        || diagnostics
            .get("no_result_calls")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            > 0;
    if low_signal_signal {
        return "low_signal";
    }
    if requires_live_web {
        "healthy"
    } else {
        "not_required"
    }
}

fn chat_ui_fallback_status_error_for_diagnostics(
    diagnostics: &Value,
    requires_live_web: bool,
    web_search_calls: i64,
) -> (&'static str, &'static str) {
    match chat_ui_expected_classification_from_diagnostics(
        diagnostics,
        requires_live_web,
        web_search_calls,
    ) {
        "tool_surface_unavailable" => ("failed", "web_tool_surface_unavailable"),
        "tool_surface_degraded" => ("failed", "web_tool_surface_degraded"),
        "tool_not_invoked" => ("tool_not_invoked", "web_tool_not_invoked"),
        "policy_blocked" => ("policy_blocked", "web_tool_policy_blocked"),
        "tool_not_found" => ("failed", "web_tool_not_found"),
        "low_signal" => ("provider_low_signal", "web_tool_low_signal"),
        _ => ("parse_failed", "web_tool_invalid_response"),
    }
}

fn chat_ui_error_code_for_classification(classification: &str) -> &'static str {
    match classification {
        "tool_surface_unavailable" => "web_tool_surface_unavailable",
        "tool_surface_degraded" => "web_tool_surface_degraded",
        "tool_not_invoked" => "web_tool_not_invoked",
        "policy_blocked" => "web_tool_policy_blocked",
        "tool_not_found" => "web_tool_not_found",
        "low_signal" => "web_tool_low_signal",
        _ => "",
    }
}

fn chat_ui_fallback_status_for_classification(classification: &str) -> &'static str {
    match classification {
        "tool_surface_unavailable" | "tool_surface_degraded" => "failed",
        "tool_not_invoked" => "tool_not_invoked",
        "policy_blocked" => "policy_blocked",
        "tool_not_found" => "failed",
        "low_signal" => "provider_low_signal",
        _ => "parse_failed",
    }
}

fn chat_ui_retry_loop_risk_from_diagnostics(diagnostics: &Value) -> Value {
    let receipts = diagnostics
        .get("execution_receipts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let receipt_count = receipts.len() as i64;
    if receipt_count == 0 {
        return json!({
            "detected": false,
            "severity": "none",
            "receipt_count": 0,
            "max_duplicate_signature_count": 0,
            "max_consecutive_signature_streak": 0,
            "dominant_signature": Value::Null,
            "source": "execution_receipts"
        });
    }

    let mut signatures = Vec::<String>::new();
    for row in &receipts {
        let status = clean(row.get("status").and_then(Value::as_str).unwrap_or(""), 120)
            .to_ascii_lowercase();
        let error_code = clean(row.get("error_code").and_then(Value::as_str).unwrap_or(""), 120)
            .to_ascii_lowercase();
        let signature = if status.is_empty() && error_code.is_empty() {
            "unknown".to_string()
        } else if error_code.is_empty() {
            status
        } else {
            format!("{status}|{error_code}")
        };
        signatures.push(signature);
    }

    let mut max_duplicate_signature_count = 0_i64;
    let mut dominant_signature = String::new();
    for signature in &signatures {
        let duplicate_count = signatures.iter().filter(|candidate| *candidate == signature).count() as i64;
        if duplicate_count > max_duplicate_signature_count {
            max_duplicate_signature_count = duplicate_count;
            dominant_signature = signature.clone();
        }
    }

    let mut max_consecutive_signature_streak = 0_i64;
    let mut streak = 0_i64;
    let mut last_signature = String::new();
    for signature in &signatures {
        if *signature == last_signature {
            streak += 1;
        } else {
            streak = 1;
            last_signature = signature.clone();
        }
        if streak > max_consecutive_signature_streak {
            max_consecutive_signature_streak = streak;
        }
    }

    let detected = receipt_count >= 3
        && (max_duplicate_signature_count >= 3 || max_consecutive_signature_streak >= 2);
    let severity = if receipt_count >= 4
        && (max_duplicate_signature_count >= 4 || max_consecutive_signature_streak >= 3)
    {
        "high"
    } else if detected {
        "medium"
    } else {
        "none"
    };
    json!({
        "detected": detected,
        "severity": severity,
        "receipt_count": receipt_count,
        "max_duplicate_signature_count": max_duplicate_signature_count,
        "max_consecutive_signature_streak": max_consecutive_signature_streak,
        "dominant_signature": if dominant_signature.is_empty() { Value::Null } else { json!(dominant_signature) },
        "source": "execution_receipts"
    })
}

fn chat_ui_apply_loop_risk_to_retry(
    retry_recommended: bool,
    retry_strategy: &'static str,
    retry_lane: &'static str,
    loop_risk: &Value,
) -> (bool, &'static str, &'static str) {
    let loop_risk_detected = loop_risk
        .get("detected")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if loop_risk_detected && retry_recommended {
        return (false, "halt_on_loop_risk", "manual_intervention");
    }
    (retry_recommended, retry_strategy, retry_lane)
}

fn chat_ui_retry_profile_for_guard(error_code: &str, classification: &str) -> (bool, &'static str, &'static str) {
    let code = clean(error_code, 120).to_ascii_lowercase();
    let class = clean(classification, 80).to_ascii_lowercase();
    if code == "web_tool_low_signal" || class == "low_signal" {
        return (true, "narrow_query", "immediate");
    }
    if code == "web_tool_not_invoked" || class == "tool_not_invoked" {
        return (true, "rerun_with_tool_call", "immediate");
    }
    if code == "web_tool_timeout" || code == "web_tool_http_429" || code == "web_tool_surface_degraded"
    {
        return (true, "retry_with_backoff", "delayed");
    }
    if code == "web_tool_policy_blocked" || class == "policy_blocked" {
        return (false, "operator_policy_action", "blocked");
    }
    if code == "web_tool_auth_missing" {
        return (false, "provide_auth", "blocked");
    }
    if code == "web_tool_surface_unavailable" {
        return (false, "restore_tool_surface", "blocked");
    }
    if code == "web_tool_not_found" || class == "tool_not_found" {
        return (false, "adjust_tool_selection", "blocked");
    }
    (false, "none", "none")
}

fn chat_ui_retry_plan_for_guard(
    retry_recommended: bool,
    retry_strategy: &str,
    retry_lane: &str,
) -> Value {
    if !retry_recommended {
        return json!({
            "auto": false,
            "attempts": 0,
            "min_delay_ms": 0,
            "max_delay_ms": 0,
            "jitter": 0.0
        });
    }
    match (retry_strategy, retry_lane) {
        ("retry_with_backoff", "delayed") => json!({
            "auto": true,
            "attempts": 2,
            "min_delay_ms": 400,
            "max_delay_ms": 30000,
            "jitter": 0.1
        }),
        ("rerun_with_tool_call", "immediate") => json!({
            "auto": true,
            "attempts": 1,
            "min_delay_ms": 0,
            "max_delay_ms": 0,
            "jitter": 0.0
        }),
        ("narrow_query", "immediate") => json!({
            "auto": false,
            "attempts": 1,
            "min_delay_ms": 0,
            "max_delay_ms": 0,
            "jitter": 0.0
        }),
        _ => json!({
            "auto": false,
            "attempts": 1,
            "min_delay_ms": 0,
            "max_delay_ms": 0,
            "jitter": 0.0
        }),
    }
}

#[cfg(test)]
#[test]
fn chat_ui_retry_loop_risk_detects_repeated_receipts_and_suppresses_retry() {
    let loop_risk = chat_ui_retry_loop_risk_from_diagnostics(&json!({
        "execution_receipts": [
            {"status": "low_signal", "error_code": "web_tool_low_signal"},
            {"status": "low_signal", "error_code": "web_tool_low_signal"},
            {"status": "low_signal", "error_code": "web_tool_low_signal"}
        ]
    }));
    assert_eq!(
        loop_risk.get("detected").and_then(Value::as_bool),
        Some(true),
        "{loop_risk}"
    );
    assert_eq!(
        loop_risk
            .get("max_duplicate_signature_count")
            .and_then(Value::as_i64),
        Some(3),
        "{loop_risk}"
    );
    let (recommended, strategy, lane) = chat_ui_apply_loop_risk_to_retry(
        true,
        "narrow_query",
        "immediate",
        &loop_risk,
    );
    assert!(!recommended, "retry should be suppressed when loop-risk is detected");
    assert_eq!(strategy, "halt_on_loop_risk");
    assert_eq!(lane, "manual_intervention");
}

#[cfg(test)]
#[test]
fn chat_ui_retry_loop_risk_keeps_retry_when_receipts_are_diverse() {
    let loop_risk = chat_ui_retry_loop_risk_from_diagnostics(&json!({
        "execution_receipts": [
            {"status": "ok", "error_code": ""},
            {"status": "error", "error_code": "web_tool_timeout"},
            {"status": "ok", "error_code": ""}
        ]
    }));
    assert_eq!(
        loop_risk.get("detected").and_then(Value::as_bool),
        Some(false),
        "{loop_risk}"
    );
    let (recommended, strategy, lane) = chat_ui_apply_loop_risk_to_retry(
        true,
        "retry_with_backoff",
        "delayed",
        &loop_risk,
    );
    assert!(recommended, "retry should remain available when no loop-risk is detected");
    assert_eq!(strategy, "retry_with_backoff");
    assert_eq!(lane, "delayed");
}

#[cfg(test)]
#[test]
fn chat_ui_expected_classification_uses_loop_risk_signal() {
    let classification = chat_ui_expected_classification_from_diagnostics(
        &json!({
            "total_calls": 3,
            "execution_receipts": [
                {"status": "low_signal", "error_code": "web_tool_low_signal"},
                {"status": "low_signal", "error_code": "web_tool_low_signal"},
                {"status": "low_signal", "error_code": "web_tool_low_signal"}
            ],
            "loop_risk": {
                "detected": true
            }
        }),
        true,
        3,
    );
    assert_eq!(classification, "low_signal");
}

#[cfg(test)]
#[test]
fn chat_ui_tool_diagnostics_publishes_loop_risk() {
    let diagnostics = chat_ui_tool_diagnostics(&[
        json!({"tool": "batch_query", "status": "low_signal", "error": "web_tool_low_signal"}),
        json!({"tool": "batch_query", "status": "low_signal", "error": "web_tool_low_signal"}),
        json!({"tool": "batch_query", "status": "low_signal", "error": "web_tool_low_signal"}),
    ]);
    assert_eq!(
        diagnostics
            .pointer("/loop_risk/detected")
            .and_then(Value::as_bool),
        Some(true),
        "{diagnostics}"
    );
    assert_eq!(
        diagnostics
            .pointer("/loop_risk/max_duplicate_signature_count")
            .and_then(Value::as_i64),
        Some(3),
        "{diagnostics}"
    );
}

fn chat_ui_tool_diagnostics(tools: &[Value]) -> Value {
    let mut search_calls = 0_i64;
    let mut fetch_calls = 0_i64;
    let mut successful_calls = 0_i64;
    let mut failed_calls = 0_i64;
    let mut no_result_calls = 0_i64;
    let mut blocked_calls = 0_i64;
    let mut not_found_calls = 0_i64;
    let mut low_signal_calls = 0_i64;
    let mut silent_failure_calls = 0_i64;
    let mut surface_unavailable_calls = 0_i64;
    let mut surface_degraded_calls = 0_i64;
    let mut error_codes = serde_json::Map::<String, Value>::new();
    let mut execution_receipts = Vec::<Value>::new();

    for (idx, row) in tools.iter().enumerate() {
        let tool_name = tool_name_for_diagnostics(row);
        if tool_name.contains("search")
            || tool_name.contains("web_search")
            || tool_name.contains("batch_query")
        {
            search_calls += 1;
        }
        if tool_name.contains("fetch") || tool_name.contains("web_fetch") {
            fetch_calls += 1;
        }

        let findings = tool_findings_count(row) as i64;
        let ok = row
            .get("ok")
            .and_then(Value::as_bool)
            .or_else(|| row.pointer("/result/ok").and_then(Value::as_bool));
        let raw_status = clean(row.get("status").and_then(Value::as_str).unwrap_or(""), 120)
            .to_ascii_lowercase();
        let error = clean(
            row.get("error")
                .or_else(|| row.pointer("/result/error"))
                .or_else(|| row.pointer("/result/message"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            300,
        );
        let result = clean(
            row.get("result")
                .or_else(|| row.pointer("/result/summary"))
                .or_else(|| row.pointer("/result/text"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            600,
        );
        let duration_ms = row
            .get("duration_ms")
            .or_else(|| row.pointer("/telemetry/duration_ms"))
            .or_else(|| row.pointer("/result/telemetry/duration_ms"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let tokens_used = row
            .get("tokens_used")
            .or_else(|| row.pointer("/telemetry/tokens_used"))
            .or_else(|| row.pointer("/result/telemetry/tokens_used"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let call_id = format!(
            "toolcall_{}",
            &sha256_hex_str(&format!("{}:{}:{}", idx, tool_name, raw_status))[..12]
        );
        let mut status = crate::tool_output_match_filter::canonical_tool_status(
            &raw_status,
            ok,
            &error,
            findings,
            !result.is_empty(),
        );
        let surface_error_code_hint = chat_ui_surface_error_code_hint_from_row(row);
        if surface_error_code_hint.is_some() && status != "ok" {
            status = "error".to_string();
        }
        let status_hint_error_code =
            crate::tool_output_match_filter::normalize_web_tooling_error_code(&raw_status);
        let prioritized_surface_error_code = if matches!(
            status_hint_error_code.as_str(),
            "web_tool_surface_unavailable" | "web_tool_surface_degraded"
        ) {
            Some(status_hint_error_code.clone())
        } else {
            surface_error_code_hint.clone()
        };
        let policy_blocked_hint = chat_ui_policy_blocked_hint_from_row(row);
        let low_signal_hint = chat_ui_low_signal_hint_from_row(row);
        let not_found_hint = chat_ui_not_found_hint_from_row(row);
        if prioritized_surface_error_code.is_none() && status != "ok" {
            if policy_blocked_hint {
                status = "blocked".to_string();
            } else if not_found_hint {
                status = "not_found".to_string();
            } else if low_signal_hint {
                status = "low_signal".to_string();
            }
        }
        let error_code = if error.is_empty() {
            if status == "error"
                && prioritized_surface_error_code.is_some()
            {
                prioritized_surface_error_code
                    .clone()
                    .unwrap_or_else(|| "web_tool_error".to_string())
            } else {
                match status.as_str() {
                    "blocked" => "web_tool_policy_blocked".to_string(),
                    "not_found" => "web_tool_not_found".to_string(),
                    "low_signal" => "web_tool_low_signal".to_string(),
                    "unknown" => "web_tool_silent_failure".to_string(),
                    _ => "web_tool_error".to_string(),
                }
            }
        } else {
            let normalized = crate::tool_output_match_filter::normalize_web_tooling_error_code(&error);
            if normalized == "web_tool_error" {
                prioritized_surface_error_code.unwrap_or(normalized)
            } else {
                normalized
            }
        };

        match status.as_str() {
            "ok" => {
                successful_calls += 1;
                if findings == 0 {
                    no_result_calls += 1;
                }
            }
            "blocked" => {
                failed_calls += 1;
                blocked_calls += 1;
                let next = error_codes
                    .get(&error_code)
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    .saturating_add(1);
                error_codes.insert(error_code.clone(), Value::from(next));
            }
            "not_found" => {
                failed_calls += 1;
                not_found_calls += 1;
                let next = error_codes
                    .get(&error_code)
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    .saturating_add(1);
                error_codes.insert(error_code.clone(), Value::from(next));
            }
            "low_signal" => {
                low_signal_calls += 1;
                no_result_calls += 1;
                let next = error_codes
                    .get(&error_code)
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    .saturating_add(1);
                error_codes.insert(error_code.clone(), Value::from(next));
            }
            "error" => {
                failed_calls += 1;
                if error_code == "web_tool_surface_unavailable" {
                    surface_unavailable_calls += 1;
                } else if error_code == "web_tool_surface_degraded" {
                    surface_degraded_calls += 1;
                }
                let next = error_codes
                    .get(&error_code)
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    .saturating_add(1);
                error_codes.insert(error_code.clone(), Value::from(next));
            }
            _ => {
                failed_calls += 1;
                silent_failure_calls += 1;
                let code = "web_tool_silent_failure".to_string();
                let next = error_codes
                    .get(&code)
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    .saturating_add(1);
                error_codes.insert(code, Value::from(next));
            }
        }
        let mut execution_receipt = crate::tool_output_match_filter::canonical_tool_execution_receipt(
            &call_id,
            &tool_name,
            &status,
            ok,
            &error,
            findings,
            duration_ms,
            tokens_used,
            !result.is_empty(),
        );
        if let Some(obj) = execution_receipt.as_object_mut() {
            obj.insert("status".to_string(), json!(status));
            obj.insert("error_code".to_string(), json!(error_code));
        }
        execution_receipts.push(execution_receipt);
    }

    let total_calls = tools.len() as i64;
    let error_ratio = if total_calls > 0 {
        (failed_calls as f64) / (total_calls as f64)
    } else {
        0.0
    };
    let mut diagnostics = json!({
        "total_calls": total_calls,
        "search_calls": search_calls,
        "fetch_calls": fetch_calls,
        "successful_calls": successful_calls,
        "failed_calls": failed_calls,
        "no_result_calls": no_result_calls,
        "blocked_calls": blocked_calls,
        "not_found_calls": not_found_calls,
        "low_signal_calls": low_signal_calls,
        "silent_failure_calls": silent_failure_calls,
        "surface_unavailable_calls": surface_unavailable_calls,
        "surface_degraded_calls": surface_degraded_calls,
        "error_ratio": error_ratio,
        "error_codes": Value::Object(error_codes),
        "execution_receipts": execution_receipts
    });
    let loop_risk = chat_ui_retry_loop_risk_from_diagnostics(&diagnostics);
    if let Some(obj) = diagnostics.as_object_mut() {
        obj.insert("loop_risk".to_string(), loop_risk);
    }
    diagnostics
}

fn rewrite_chat_ui_placeholder_with_tool_diagnostics(
    assistant: &str,
    diagnostics: &Value,
) -> (String, String) {
    let current = clean(assistant, 16_000);
    if current.is_empty() || !crate::tool_output_match_filter::matches_ack_placeholder(&current) {
        return (current, "unchanged".to_string());
    }
    let errors = diagnostics
        .get("error_codes")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let has_error = !errors.is_empty();
    let total_calls = diagnostics
        .get("total_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let has_surface_unavailable = errors.contains_key("web_tool_surface_unavailable");
    let has_surface_degraded = errors.contains_key("web_tool_surface_degraded");
    let has_auth_missing = errors.contains_key("web_tool_auth_missing");
    let has_policy_blocked = errors.contains_key("web_tool_policy_blocked");
    let has_invalid_response = errors.contains_key("web_tool_invalid_response");
    let has_not_found = errors.contains_key("web_tool_not_found");
    let has_silent_failure = errors.contains_key("web_tool_silent_failure");

    if has_surface_unavailable {
        return (
            chat_ui_tool_surface_fail_closed_copy("web_tool_surface_unavailable").to_string(),
            "placeholder_replaced_surface_unavailable".to_string(),
        );
    }
    if has_surface_degraded {
        return (
            chat_ui_tool_surface_fail_closed_copy("web_tool_surface_degraded").to_string(),
            "placeholder_replaced_surface_degraded".to_string(),
        );
    }
    if has_auth_missing {
        return (
            crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                "auth_missing",
                "web_tool_auth_missing",
                None,
            ),
            "placeholder_replaced_auth".to_string(),
        );
    }
    if has_policy_blocked {
        return (
            crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                "policy_blocked",
                "web_tool_policy_blocked",
                None,
            ),
            "placeholder_replaced_policy".to_string(),
        );
    }
    if has_invalid_response {
        return (
            crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                "parse_failed",
                "web_tool_invalid_response",
                None,
            ),
            "placeholder_replaced_invalid_response".to_string(),
        );
    }
    if has_not_found {
        return (
            crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                "failed",
                "web_tool_not_found",
                None,
            ),
            "placeholder_replaced_not_found".to_string(),
        );
    }
    if has_silent_failure {
        return (
            crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                "failed",
                "web_tool_silent_failure",
                None,
            ),
            "placeholder_replaced_silent_failure".to_string(),
        );
    }
    if has_error {
        return (
            crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                "failed",
                "web_tool_error",
                None,
            ),
            "placeholder_replaced_error".to_string(),
        );
    }
    if total_calls > 0 {
        return (
            crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                "provider_low_signal",
                "web_tool_low_signal",
                None,
            ),
            "placeholder_replaced_low_signal".to_string(),
        );
    }
    (current, "unchanged".to_string())
}

fn ensure_file(path: &Path, content: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("missing_parent:{}", path.display()))?;
    fs::create_dir_all(parent).map_err(|e| format!("mkdir_failed:{}:{e}", parent.display()))?;
    fs::write(path, content).map_err(|e| format!("write_failed:{}:{e}", path.display()))
}

fn code_engineer_templates_path(root: &Path) -> PathBuf {
    state_root(root)
        .join("code_engineer")
        .join("builders_templates.json")
}

fn slug_from_goal(goal: &str, fallback_prefix: &str) -> String {
    let mut out = String::new();
    for ch in goal.chars() {
        if out.len() >= 48 {
            break;
        }
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_ascii_whitespace() || ch == '-' || ch == '_' {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        format!("{fallback_prefix}-{}", &sha256_hex_str("default")[..8])
    } else {
        trimmed.to_string()
    }
}

fn classify_builder_risk(goal: &str, explicit: Option<&String>) -> String {
    if let Some(raw) = explicit {
        let normalized = raw.trim().to_ascii_lowercase();
        if matches!(normalized.as_str(), "low" | "medium" | "high") {
            return normalized;
        }
    }
    let lower = goal.to_ascii_lowercase();
    let high_terms = [
        "delete",
        "drop table",
        "production",
        "payment",
        "security",
        "auth bypass",
    ];
    if high_terms.iter().any(|term| lower.contains(term)) {
        return "high".to_string();
    }
    let medium_terms = [
        "deploy",
        "migration",
        "schema",
        "customer data",
        "live traffic",
    ];
    if medium_terms.iter().any(|term| lower.contains(term)) {
        return "medium".to_string();
    }
    "low".to_string()
}

fn build_reasoning_receipt(contract: &Value, goal: &str, risk: &str, approved: bool) -> Value {
    let auto_allow = contract
        .get("reasoning_gate")
        .and_then(|v| v.get("auto_allow_risks"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![Value::String("low".to_string())]);
    let auto_allow_risks = auto_allow
        .iter()
        .filter_map(Value::as_str)
        .map(|v| v.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let requires_explicit_approval = !auto_allow_risks.iter().any(|v| v == risk);
    let continue_allowed = !requires_explicit_approval || approved;
    let mut out = json!({
        "type": "app_plane_reasoning_gate",
        "goal": clean(goal, 2000),
        "risk_class": risk,
        "approved": approved,
        "requires_explicit_approval": requires_explicit_approval,
        "continue_allowed": continue_allowed,
        "plan": [
            {"stage":"research","intent":"collect constraints and edge cases"},
            {"stage":"plan","intent":"derive execution graph and acceptance criteria"},
            {"stage":"code","intent":"materialize deterministic artifacts"},
            {"stage":"test","intent":"run bounded verification and critique loop"},
            {"stage":"package","intent":"emit delivery manifest with provenance"}
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
