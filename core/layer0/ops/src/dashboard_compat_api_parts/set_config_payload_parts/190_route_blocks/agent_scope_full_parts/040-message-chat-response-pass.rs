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
        recent_floor_target: _,
        recent_floor_missing_before: _,
        recent_floor_satisfied: _,
        recent_floor_coverage_before: _,
        recent_floor_coverage_after: _,
        recent_floor_active_missing: _,
        recent_floor_active_satisfied: _,
        recent_floor_active_coverage: _,
        recent_floor_continuity_status: _,
        recent_floor_continuity_action: _,
        recent_floor_continuity_message: _,
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
            let turn_provider = clean_text(
                result
                    .get("provider")
                    .and_then(Value::as_str)
                    .unwrap_or(&provider),
                80,
            );
            let turn_model = clean_text(
                result
                    .get("runtime_model")
                    .or_else(|| result.get("model"))
                    .and_then(Value::as_str)
                    .unwrap_or(&model),
                240,
            );
            let mut response_text = clean_chat_text(
                result.get("response").and_then(Value::as_str).unwrap_or(""),
                32_000,
            );
            response_text = strip_internal_context_metadata_prefix(&response_text);
            response_text = strip_internal_cache_control_markup(&response_text);
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
                // Diagnostic-only. The runtime must not replace LLM-authored
                // chat text with a system-authored capability summary.
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
                    // Diagnostic-only: do not erase LLM-authored chat text.
                } else if response_text.contains("originalUrl:") && response_text.contains("title:")
                {
                    // Diagnostic-only: raw-looking output must be evaluated, not hidden.
                } else if response_text.contains(runtime_capability_surface_template) {
                    // Diagnostic-only: preserve the model output for trace/eval.
                }
            }
            let _memory_recall_diagnostic =
                memory_recall_requested(message) || persistent_memory_denied_phrase(&response_text);
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
                let _ = (auto_count, directive_hint_receipt);
                // Diagnostic-only. Do not manufacture a tool call when the LLM
                // did not choose one through the workflow gate.
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
            let supplemental_comparison_tools =
                if inline_tools_allowed {
                    latent_tool_candidate_completion_cards(
                        root,
                        snapshot,
                        agent_id,
                        Some(row),
                        message,
                        &response_text,
                        false,
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
                if !supplemented_summary.is_empty() {
                    let _ = supplemented_summary;
                    // Diagnostic-only. Tool summaries are observations for the
                    // model/eval path, not system-authored chat replacements.
                }
            }
            let _inline_tool_suppression_diagnostic = inline_tools_suppressed && response_tools.is_empty();
            if response_tools.is_empty()
                && !inline_tools_allowed
                && (response_is_no_findings_placeholder(&response_text)
                    || response_looks_like_raw_web_artifact_dump(&response_text)
                    || response_looks_like_unsynthesized_web_snippet_dump(&response_text))
            {
                // Diagnostic-only: surface the LLM text and let eval flag the bad shape.
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
            if response_contains_unexpected_state_retry_boilerplate(&response_text)
                || workflow_response_repetition_breaker_active(&response_text)
            {
                // Preserve LLM-authored output only; do not replace with system fallback text.
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
                turn_provider,
                turn_model,
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
