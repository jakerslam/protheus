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
            let runtime_summary = runtime_sync_summary(snapshot);
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
            let workflow_mode = if response_tools.is_empty() {
                "model_direct_answer".to_string()
            } else {
                "model_inline_tool_execution".to_string()
            };
            let workflow_system_events = build_turn_workflow_events(
                &response_tools,
                inline_pending_confirmation.as_ref(),
                false,
            );
            Some(finalize_message_finalization_and_payload(
                root,
                agent_id,
                snapshot,
                row,
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
