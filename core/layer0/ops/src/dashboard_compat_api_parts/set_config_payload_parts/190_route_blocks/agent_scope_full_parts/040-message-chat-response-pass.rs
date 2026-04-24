fn handle_message_chat_response_pass(
    root: &Path,
    snapshot: &Value,
    row: &Value,
    agent_id: &str,
    message: &str,
    prepared: PreparedMessageRouteContext,
) -> Option<CompatApiResponse> {
    let PreparedMessageRouteContext {
        provider,
        model,
        auto_route,
        requested_provider,
        requested_model,
        virtual_key_id,
        virtual_key_gate,
        state,
        messages,
        active_messages,
        context_pool_limit_tokens,
        context_pool_tokens,
        pooled_messages_len,
        sessions_total,
        fallback_window,
        memory_kv_entries,
        active_context_target_tokens,
        active_context_min_recent,
        include_all_sessions_context,
        context_active_tokens,
        context_ratio,
        context_pressure,
        pre_generation_pruned,
        recent_floor_enforced,
        recent_floor_injected,
        recent_floor_target,
        recent_floor_missing_before,
        recent_floor_satisfied,
        recent_floor_coverage_before,
        recent_floor_coverage_after,
        recent_floor_active_missing,
        recent_floor_active_satisfied,
        recent_floor_active_coverage,
        recent_floor_continuity_status,
        recent_floor_continuity_action,
        recent_floor_continuity_message,
        history_trim_confirmed,
        emergency_compact,
        workspace_hints,
        latent_tool_candidates,
        inline_tools_allowed,
        system_prompt,
    } = prepared;
    match crate::dashboard_provider_runtime::invoke_chat(
        root,
        &provider,
        &model,
        &system_prompt,
        &active_messages,
        message,
    ) {
        Ok(result) => {
            let mut response_text = clean_chat_text(
                result.get("response").and_then(Value::as_str).unwrap_or(""),
                32_000,
            );
            let response_had_context_meta = internal_context_metadata_phrase(&response_text);
            response_text = strip_internal_context_metadata_prefix(&response_text);
            response_text = strip_internal_cache_control_markup(&response_text);
            if response_text.is_empty() && response_had_context_meta {
                response_text = "I have relevant prior context loaded and can keep going from here. Tell me what you want to do next.".to_string();
            }
            let local_workspace_tooling_probe_turn = {
                let lowered = message.to_ascii_lowercase();
                let local_tokens = [
                    "local",
                    "workspace",
                    "directory",
                    "folder",
                    "file tooling",
                    "file tool",
                    "repo",
                    "path",
                ];
                let web_tokens = [
                    "http://", "https://", "web", "internet", "online", "browser",
                ];
                local_tokens.iter().any(|token| lowered.contains(token))
                    && !web_tokens.iter().any(|token| lowered.contains(token))
            };
            let runtime_capability_surface_template = "I can access runtime telemetry, persistent memory, workspace files, channels, and approved command surfaces in this session.";
            let runtime_summary = runtime_sync_summary(snapshot);
            let runtime_probe = runtime_probe_requested(message);
            let runtime_denial = runtime_access_denied_phrase(&response_text);
            if runtime_probe || runtime_denial {
                response_text = if runtime_probe {
                    runtime_access_summary_text(&runtime_summary)
                } else if local_workspace_tooling_probe_turn {
                    String::new()
                } else {
                    String::new()
                };
            }
            if local_workspace_tooling_probe_turn {
                let response_lowered = response_text.to_ascii_lowercase();
                let route_classification_template = response_lowered.contains("the first gate")
                    && (response_lowered.contains("workflow_route")
                        || response_lowered.contains("task_or_info_route"))
                    && (response_lowered.contains("still classifying this as an \"info\" route rather than a \"task\" route")
                        || response_lowered.contains("still classifying this as an 'info' route rather than a 'task' route")
                        || response_lowered.contains("binary classification")
                        || response_lowered.contains("automated classification based on semantic analysis")
                        || response_lowered.contains("not a true/false decision i control")
                        || response_lowered.contains("defaults to info")
                        || contains_deprecated_workflow_source_marker(&response_lowered)
                        || response_lowered.contains("explicit tool-related phrasing")
                        || response_lowered.contains("task classification path")
                        || response_lowered.contains("tool operation request")
                        || response_lowered.contains("conversation bypass mode is currently active")
                        || response_lowered.contains("restricted from running web searches")
                        || response_lowered.contains("can't autonomously decide to use web tools")
                        || response_lowered
                            .contains("requires manual step-by-step authorization for tool usage"));
                let decision_tree_autoclassifier_template = response_lowered.contains("decision tree")
                    && response_lowered.contains("automatically classifies")
                    && response_lowered.contains("\"info\"")
                    && response_lowered.contains("\"task\"")
                    && response_lowered.contains("semantic analysis");
                if response_contains_unexpected_state_retry_boilerplate(&response_text)
                    || workflow_response_repetition_breaker_active(&response_text)
                    || route_classification_template
                    || decision_tree_autoclassifier_template
                {
                    response_text.clear();
                } else if response_text.contains("originalUrl:") && response_text.contains("title:")
                {
                    response_text.clear();
                } else if response_text.contains(runtime_capability_surface_template) {
                    response_text.clear();
                }
            }
            if memory_recall_requested(message) || persistent_memory_denied_phrase(&response_text) {
                response_text = build_memory_recall_response(&state, &messages, message);
            }
            let lowered = message.to_ascii_lowercase();
            let explicit_parallel_directive = swarm_intent_requested(message)
                || lowered.contains("multi-agent")
                || lowered.contains("multi agent");
            let response_denied_spawn = spawn_surface_denied_phrase(&response_text);
            let response_has_tool_call = response_text.contains("<function=");
            if explicit_parallel_directive && (response_denied_spawn || !response_has_tool_call) {
                let auto_count = infer_subagent_count_from_message(message);
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
                mut response_tools,
                inline_pending_confirmation,
                inline_tools_suppressed,
            ) = execute_inline_tool_calls(
                root,
                snapshot,
                agent_id,
                Some(row),
                &response_text,
                message,
                inline_tools_allowed,
            );
            response_text = tool_adjusted_response;
            let allow_draft_retry_fallback = false;
            let supplemental_comparison_tools =
                if inline_tools_allowed || allow_draft_retry_fallback {
                    latent_tool_candidate_completion_cards(
                        root,
                        snapshot,
                        agent_id,
                        Some(row),
                        message,
                        &response_text,
                        allow_draft_retry_fallback,
                        &latent_tool_candidates,
                        &response_tools,
                    )
                } else {
                    Vec::new()
                };
            if !supplemental_comparison_tools.is_empty() {
                response_tools.extend(supplemental_comparison_tools);
                let supplemented_summary =
                    clean_text(&response_tools_summary_for_user(&response_tools, 4), 32_000);
                if !supplemented_summary.is_empty()
                    && message_requests_workspace_plus_web_comparison(message)
                {
                    response_text = supplemented_summary;
                }
            }
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
                    message,
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
                    let (without_inline_calls, _) = extract_inline_tool_calls(&retried_text, 6);
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
                    response_text.clear();
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
                    message,
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
                    let (without_inline_calls, _) = extract_inline_tool_calls(&retried_text, 6);
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
            if let Some(ref pending) = inline_pending_confirmation {
                let pending_tool = clean_text(
                    pending
                        .get("tool_name")
                        .or_else(|| pending.get("tool"))
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    120,
                );
                if !pending_tool.is_empty() {
                    let pending_input = pending.get("input").cloned().unwrap_or_else(|| json!({}));
                    store_pending_tool_confirmation(
                        root,
                        agent_id,
                        &pending_tool,
                        &pending_input,
                        pending
                            .get("source")
                            .and_then(Value::as_str)
                            .unwrap_or("inline_tool_call"),
                    );
                }
            } else if !response_tools.is_empty() {
                clear_pending_tool_confirmation(root, agent_id);
            } else if message_is_negative_confirmation(message) {
                clear_pending_tool_confirmation(root, agent_id);
            }
            if !user_requested_internal_runtime_details(message) {
                response_text = abstract_runtime_mechanics_terms(&response_text);
            }
            response_text = strip_internal_cache_control_markup(&response_text);
            let latest_assistant_text = active_messages
                .iter()
                .rev()
                .find_map(|row| {
                    let role =
                        clean_text(row.get("role").and_then(Value::as_str).unwrap_or(""), 24)
                            .to_ascii_lowercase();
                    if role != "assistant" && role != "agent" {
                        return None;
                    }
                    let text = clean_chat_text(
                        row.get("text")
                            .or_else(|| row.get("content"))
                            .or_else(|| row.get("message"))
                            .and_then(Value::as_str)
                            .unwrap_or(""),
                        32_000,
                    );
                    if text.trim().is_empty() {
                        None
                    } else {
                        Some(text)
                    }
                })
                .unwrap_or_default();
            if response_contains_unexpected_state_retry_boilerplate(&response_text)
                || workflow_response_repetition_breaker_active(&response_text)
            {
                // Preserve LLM-authored output only; do not replace with system fallback text.
            }
            if response_is_unrelated_context_dump(message, &response_text) {
                let strict_relevance_prompt = clean_text(
                    &format!(
                        "{}\n\nRelevance guard: answer only the latest user request. Ignore unrelated prior snippets and project templates. If the user asks for code, provide direct code first.",
                        AGENT_RUNTIME_SYSTEM_PROMPT
                    ),
                    12_000,
                );
                let relevance_retry_messages = active_messages
                    .iter()
                    .rev()
                    .take(7)
                    .cloned()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<Vec<_>>();
                let retried = crate::dashboard_provider_runtime::invoke_chat(
                    root,
                    &provider,
                    &model,
                    &strict_relevance_prompt,
                    &relevance_retry_messages,
                    message,
                )
                .ok()
                .and_then(|value| {
                    let mut retried_text = clean_chat_text(
                        value.get("response").and_then(Value::as_str).unwrap_or(""),
                        32_000,
                    );
                    retried_text = strip_internal_context_metadata_prefix(&retried_text);
                    retried_text = strip_internal_cache_control_markup(&retried_text);
                    if !user_requested_internal_runtime_details(message) {
                        retried_text = abstract_runtime_mechanics_terms(&retried_text);
                    }
                    if response_is_unrelated_context_dump(message, &retried_text) {
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
                response_text = retried.unwrap_or_default();
            }
            let conversation_bypass_control = workflow_conversation_bypass_control_for_turn(
                message,
                &active_messages,
                inline_tools_allowed,
            );
            let conversation_bypass_active = conversation_bypass_control
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let conversation_bypass_mode_override = clean_text(
                conversation_bypass_control
                    .get("workflow_mode_override")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120,
            );
            let mut workflow_mode = if response_tools.is_empty() {
                "model_direct_answer".to_string()
            } else {
                "model_inline_tool_execution".to_string()
            };
            if conversation_bypass_active
                && response_tools.is_empty()
                && !conversation_bypass_mode_override.is_empty()
            {
                workflow_mode = conversation_bypass_mode_override;
            }
            let mut workflow_system_events = build_turn_workflow_events(
                &response_tools,
                inline_pending_confirmation.as_ref(),
                false,
            );
            if conversation_bypass_control
                .get("should_emit_event")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                workflow_system_events.push(turn_workflow_event(
                    "conversation_bypass_control",
                    conversation_bypass_control,
                ));
            }
            Some(finalize_message_finalization_and_payload(
                root,
                agent_id,
                message,
                &result,
                response_text,
                response_tools,
                workflow_mode,
                workflow_system_events,
                runtime_summary,
                state,
                messages,
                active_messages,
                provider,
                model,
                requested_provider,
                requested_model,
                auto_route,
                virtual_key_id,
                virtual_key_gate,
                fallback_window,
                context_active_tokens,
                context_ratio,
                context_pressure,
                context_pool_limit_tokens,
                context_pool_tokens,
                pooled_messages_len,
                sessions_total,
                memory_kv_entries,
                active_context_target_tokens,
                active_context_min_recent,
                include_all_sessions_context,
                pre_generation_pruned,
                recent_floor_enforced,
                recent_floor_injected,
                history_trim_confirmed,
                emergency_compact,
                workspace_hints,
                latent_tool_candidates,
                inline_tools_allowed,
            ))
        }
        Err(err) => Some(finalize_message_invoke_failure_and_payload(
            root,
            agent_id,
            message,
            &provider,
            &model,
            &err,
            &active_messages,
            workspace_hints,
            latent_tool_candidates,
        )),
    }
}
