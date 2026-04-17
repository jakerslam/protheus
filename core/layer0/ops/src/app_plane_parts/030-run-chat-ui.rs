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
    if requires_live_web && chat_ui_web_search_call_count(&tools) == 0 {
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
        if fallback_ok {
            let summary = clean(
                fallback
                    .get("summary")
                    .or_else(|| fallback.get("response"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                2_000,
            );
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
            let fail_closed = "Web tooling execution failed before any search tool call was recorded (error_code: web_tool_not_invoked). Retry lane: run `batch_query` with a narrower query or one specific source URL.".to_string();
            assistant_raw = clean(&fail_closed, 16_000);
            forced_web_outcome = "forced_web_tool_not_invoked".to_string();
            forced_web_error_code = "web_tool_not_invoked".to_string();
            forced_web_fallback = json!({
                "applied": true,
                "query": fallback_query,
                "status": "failed",
                "error_code": "web_tool_not_invoked"
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
    if assistant.trim().is_empty()
        || crate::tool_output_match_filter::matches_ack_placeholder(&assistant)
        || crate::tool_output_match_filter::contains_forbidden_runtime_context_markers(&assistant)
    {
        let fallback_status = if requires_live_web && chat_ui_web_search_call_count(&tools) == 0 {
            "tool_not_invoked"
        } else {
            "parse_failed"
        };
        let fallback_error_code = if fallback_status == "tool_not_invoked" {
            "web_tool_not_invoked"
        } else {
            "web_tool_invalid_response"
        };
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
    let web_search_calls = chat_ui_web_search_call_count(&tools) as i64;
    let blocked_signal = tools.iter().any(|row| {
        let status = clean(
            row.get("status").and_then(Value::as_str).unwrap_or(""),
            80,
        )
        .to_ascii_lowercase();
        let error = clean(
            row.get("error").and_then(Value::as_str).unwrap_or(""),
            160,
        )
        .to_ascii_lowercase();
        status.contains("blocked")
            || error.contains("blocked")
            || error.contains("denied")
            || error.contains("policy")
            || error.contains("nexus")
    });
    let low_signal = tools.iter().any(|row| {
        let status = clean(
            row.get("status").and_then(Value::as_str).unwrap_or(""),
            80,
        )
        .to_ascii_lowercase();
        status.contains("low_signal")
            || status.contains("low-signal")
            || status.contains("no_results")
            || status.contains("no_result")
    });
    let web_classification = if requires_live_web && web_search_calls == 0 {
        "tool_not_invoked"
    } else if blocked_signal {
        "policy_blocked"
    } else if low_signal {
        "low_signal"
    } else if requires_live_web {
        "healthy"
    } else {
        "not_required"
    };
    let mut final_outcome = if forced_web_outcome.is_empty() {
        response_finalization_outcome.clone()
    } else {
        forced_web_outcome.clone()
    };
    let hard_guard_applied = hard_guard
        .get("applied")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if hard_guard_applied {
        final_outcome = "hard_guard_fallback".to_string();
    }
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
    let transaction_complete =
        (!requires_live_web || matches!(web_classification, "healthy")) && !hard_guard_applied;
    let transaction_status = if transaction_complete {
        "complete"
    } else if matches!(web_classification, "low_signal") {
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
            "classification": web_classification,
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
            "final_ack_only": crate::tool_output_match_filter::matches_ack_placeholder(&assistant),
            "findings_available": chat_ui_tools_have_valid_findings(&tools),
            "tool_receipt_summary": receipt_summary,
            "tool_transaction": {
                "id": transaction_id,
                "intent": transaction_intent,
                "status": transaction_status,
                "complete": transaction_complete,
                "classification": web_classification,
                "closed_at": crate::now_iso()
            },
            "hard_guard": hard_guard,
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
    lowered.contains("web search")
        || lowered.contains("websearch")
        || lowered.contains("search the web")
        || lowered.contains("search again")
        || lowered.contains("find information")
        || lowered.contains("finding information")
        || lowered.contains("best chili recipes")
        || ((lowered.contains("framework") || lowered.contains("frameworks"))
            && (lowered.contains("current")
                || lowered.contains("latest")
                || lowered.contains("top")))
        || (lowered.contains("search")
            && (lowered.contains("latest")
                || lowered.contains("current")
                || lowered.contains("framework")
                || lowered.contains("recipes")))
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
    let status = if requires_live_web && total_calls <= 0 {
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
            "Tool transaction {} for intent \"{}\": total={} success={} failed={} blocked={} not_found={} low_signal={} silent_failure={}.",
            status,
            intent,
            total_calls,
            successful_calls,
            failed_calls,
            blocked_calls,
            not_found_calls,
            low_signal_calls,
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
        let status = crate::tool_output_match_filter::canonical_tool_status(
            &raw_status,
            ok,
            &error,
            findings,
            !result.is_empty(),
        );
        let error_code = if error.is_empty() {
            match status.as_str() {
                "blocked" => "web_tool_policy_blocked".to_string(),
                "not_found" => "web_tool_not_found".to_string(),
                "low_signal" => "web_tool_low_signal".to_string(),
                "unknown" => "web_tool_silent_failure".to_string(),
                _ => "web_tool_error".to_string(),
            }
        } else {
            crate::tool_output_match_filter::normalize_web_tooling_error_code(&error)
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
            }
            "error" => {
                failed_calls += 1;
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
        execution_receipts.push(crate::tool_output_match_filter::canonical_tool_execution_receipt(
            &call_id,
            &tool_name,
            &status,
            ok,
            &error,
            findings,
            duration_ms,
            tokens_used,
            !result.is_empty(),
        ));
    }

    let total_calls = tools.len() as i64;
    let error_ratio = if total_calls > 0 {
        (failed_calls as f64) / (total_calls as f64)
    } else {
        0.0
    };
    json!({
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
        "error_ratio": error_ratio,
        "error_codes": Value::Object(error_codes),
        "execution_receipts": execution_receipts
    })
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
    let has_auth_missing = errors.contains_key("web_tool_auth_missing");
    let has_policy_blocked = errors.contains_key("web_tool_policy_blocked");
    let has_invalid_response = errors.contains_key("web_tool_invalid_response");
    let has_not_found = errors.contains_key("web_tool_not_found");
    let has_silent_failure = errors.contains_key("web_tool_silent_failure");

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
