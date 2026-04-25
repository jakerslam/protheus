fn handle_agent_scope_message_route(
    root: &Path,
    method: &str,
    segments: &[String],
    body: &[u8],
    _path: &str,
    snapshot: &Value,
    agent_id: &str,
    existing: &Option<Value>,
) -> Option<CompatApiResponse> {
    if method == "POST" && segments.len() == 1 && segments[0] == "message" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let message = clean_text(
            request.get("message").and_then(Value::as_str).unwrap_or(""),
            8_000,
        );
        if message.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": "message_required"}),
            });
        }
        let row = existing.clone().unwrap_or_else(|| json!({}));
        let lowered = message.to_ascii_lowercase();
        let contains_any = |terms: &[&str]| terms.iter().any(|term| lowered.contains(term));
        let contract_violation = (contains_any(&["ignore", "bypass", "disable", "override"])
            && contains_any(&["contract", "safety", "policy", "receipt"]))
            || contains_any(&["exfiltrate", "steal", "dump secrets", "leak", "secrets"]);
        if contract_violation {
            let _ = upsert_contract_patch(
                root,
                agent_id,
                &json!({
                    "status": "terminated",
                    "termination_reason": "contract_violation",
                    "terminated_at": crate::now_iso(),
                    "updated_at": crate::now_iso()
                }),
            );
            return Some(CompatApiResponse {
                status: 409,
                payload: json!({
                    "ok": false,
                    "error": "agent_contract_terminated",
                    "agent_id": agent_id,
                    "termination_reason": "contract_violation"
                }),
            });
        }
        let workspace_hints = workspace_file_hints_for_message(root, Some(&row), &message, 5);
        let latent_tool_candidates = latent_tool_candidates_for_message(&message, &workspace_hints);
        let workspace_hints_value = json!(workspace_hints);
        let latent_tool_candidates_value = json!(latent_tool_candidates);
        let explicit_operator_command = message.trim_start().starts_with('/');
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
            let web_tokens = ["http://", "https://", "web", "internet", "online", "browser"];
            local_tokens.iter().any(|token| lowered.contains(token))
                && !web_tokens.iter().any(|token| lowered.contains(token))
        };
        let mut resolved_tool_intent = direct_tool_intent_from_user_message(&message);
        let mut replayed_pending_confirmation = false;
        if let Some((pending_tool_name, mut pending_tool_input)) =
            pending_tool_confirmation_call(root, agent_id)
        {
            if resolved_tool_intent.is_none() {
                if message_is_negative_confirmation(&message) {
                    clear_pending_tool_confirmation(root, agent_id);
                } else if message_is_affirmative_confirmation(&message) {
                    if !pending_tool_input.is_object() {
                        pending_tool_input = json!({});
                    }
                    if !input_has_confirmation(&pending_tool_input) {
                        pending_tool_input["confirm"] = Value::Bool(true);
                    }
                    if input_approval_note(&pending_tool_input).is_empty() {
                        pending_tool_input["approval_note"] =
                            Value::String("user confirmed pending action".to_string());
                    }
                    resolved_tool_intent = Some((pending_tool_name, pending_tool_input));
                    replayed_pending_confirmation = true;
                }
            }
        }
        if local_workspace_tooling_probe_turn
            && replayed_pending_confirmation
            && !explicit_operator_command
        {
            resolved_tool_intent = None;
            replayed_pending_confirmation = false;
        }
        if resolved_tool_intent.is_some() && !explicit_operator_command && !replayed_pending_confirmation {
            resolved_tool_intent = None;
        }
        if available_model_count(root, snapshot) == 0 && resolved_tool_intent.is_none() {
            return Some(CompatApiResponse {
                status: 503,
                payload: no_models_available_payload(agent_id),
            });
        }
        if let Some((tool_name, tool_input)) = resolved_tool_intent {
            let tool_payload = execute_tool_call_with_recovery(
                root,
                snapshot,
                agent_id,
                Some(&row),
                &tool_name,
                &tool_input,
            );
            let ok = tool_payload
                .get("ok")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let requires_confirmation = tool_error_requires_confirmation(&tool_payload);
            if requires_confirmation {
                store_pending_tool_confirmation(
                    root,
                    agent_id,
                    &tool_name,
                    &tool_input,
                    "direct_message",
                );
            } else {
                clear_pending_tool_confirmation(root, agent_id);
            }
            let mut response_text = summarize_tool_payload(&tool_name, &tool_payload);
            if response_text.trim().is_empty() {
                response_text = if ok {
                    format!(
                        "I ran `{}`, but it returned no usable findings yet. Ask me to retry with a narrower input.",
                        normalize_tool_name(&tool_name)
                    )
                } else {
                    user_facing_tool_failure_summary(&tool_name, &tool_payload).unwrap_or_else(
                        || {
                            format!(
                                "I couldn't complete `{}` right now.",
                                normalize_tool_name(&tool_name)
                            )
                        },
                    )
                };
            }
            if ok && response_looks_like_tool_ack_without_findings(&response_text) {
                response_text = format!(
                    "I ran `{}`, but it returned no usable findings yet. Ask me to retry with a narrower input.",
                    normalize_tool_name(&tool_name)
                );
            }
            if !user_requested_internal_runtime_details(&message) {
                response_text = abstract_runtime_mechanics_terms(&response_text);
            }
            response_text = strip_internal_cache_control_markup(&response_text);
            let tool_card_status = tool_card_status_from_payload(&tool_payload);
            let tool_card = json!({
                "id": format!("tool-direct-{}", normalize_tool_name(&tool_name)),
                "name": normalize_tool_name(&tool_name),
                "input": trim_text(&tool_input.to_string(), 4000),
                "result": trim_text(&summarize_tool_payload(&tool_name, &tool_payload), 24_000),
                "is_error": !ok,
                "blocked": tool_card_status == "blocked" || tool_card_status == "policy_denied",
                "status": tool_card_status,
                "tool_attempt_receipt": tool_payload
                    .pointer("/tool_pipeline/tool_attempt_receipt")
                    .cloned()
                    .unwrap_or(Value::Null)
            });
            let response_tools = vec![tool_card.clone()];
            let tool_failure_reason = response_tools_failure_reason_for_user(&response_tools, 4);
            if !tool_failure_reason.is_empty()
                && (response_text.trim().is_empty()
                    || response_looks_like_tool_ack_without_findings(&response_text)
                    || response_is_no_findings_placeholder(&response_text))
            {
                response_text = tool_failure_reason;
            }
            let (finalized_response, tool_completion, finalization_seed) =
                enforce_user_facing_finalization_contract(
                    &message,
                    response_text,
                    &response_tools,
                );
            let initial_ack_only = tool_completion
                .get("initial_ack_only")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let mut tooling_fallback_used = false;
            let mut comparative_fallback_used = false;
            let mut visible_response_repaired = false;
            let mut finalized_response = finalized_response;
            let mut finalization_outcome = clean_text(&finalization_seed, 180);
            let mut tool_completion = json!({});
            let mut synthesis_provider = clean_text(
                row.get("model_provider")
                    .and_then(Value::as_str)
                    .unwrap_or("auto"),
                80,
            );
            if synthesis_provider.is_empty() {
                synthesis_provider = "auto".to_string();
            }
            let mut synthesis_model = clean_text(
                row.get("runtime_model")
                    .or_else(|| row.get("model_name"))
                    .and_then(Value::as_str)
                    .unwrap_or("auto"),
                240,
            );
            if synthesis_model.is_empty() {
                synthesis_model = "auto".to_string();
            }
            let synthesis_history = Vec::<Value>::new();
            let workflow_pending_confirmation = if requires_confirmation {
                Some(json!({
                    "tool_name": normalize_tool_name(&tool_name),
                    "source": "direct_message"
                }))
            } else {
                None
            };
            let mut response_workflow = run_turn_workflow_final_response(
                root,
                &synthesis_provider,
                &synthesis_model,
                &synthesis_history,
                &message,
                "direct_tool_route",
                &response_tools,
                &build_turn_workflow_events(
                    &response_tools,
                    workflow_pending_confirmation.as_ref(),
                    replayed_pending_confirmation,
                ),
                &finalized_response,
                "",
            );
            let workflow_status = workflow_final_response_status(&response_workflow);
            let workflow_used = workflow_final_response_used(&response_workflow);
            let workflow_fallback_allowed =
                workflow_final_response_allows_system_fallback(&response_workflow);
            if !workflow_status.is_empty() {
                finalization_outcome = merge_response_outcomes(
                    &finalization_outcome,
                    &format!("workflow:{workflow_status}"),
                    180,
                );
            }
            let initial_draft_response = finalized_response.clone();
            let workflow_system_fallback_used = false;
            if workflow_used {
                if let Some(synthesized) = response_workflow.get("response").and_then(Value::as_str)
                {
                    finalized_response = synthesized.to_string();
                }
                tool_completion = tool_completion_report_for_response(
                    &finalized_response,
                    &response_tools,
                    "workflow_authored",
                );
            } else if workflow_fallback_allowed {
                let fallback_response = initial_draft_response.clone();
                finalization_outcome = merge_response_outcomes(
                    &finalization_outcome,
                    "workflow_no_system_fallback",
                    180,
                );
                let (contracted, report, retry_outcome) =
                    enforce_user_facing_finalization_contract(
                        &message,
                        fallback_response,
                        &response_tools,
                    );
                finalized_response = contracted;
                tool_completion = report;
                finalization_outcome =
                    merge_response_outcomes(&finalization_outcome, &retry_outcome, 180);
            } else {
                finalization_outcome = merge_response_outcomes(
                    &finalization_outcome,
                    "workflow_no_system_fallback",
                    180,
                );
                let fallback_response = initial_draft_response.clone();
                let (contracted, report, retry_outcome) = enforce_user_facing_finalization_contract(
                    &message,
                    fallback_response,
                    &response_tools,
                );
                finalized_response = contracted;
                tool_completion = report;
                finalization_outcome =
                    merge_response_outcomes(&finalization_outcome, &retry_outcome, 180);
            }
            let (repaired_response, repair_outcome, repair_tooling_used, repair_comparative_used) =
                repair_visible_response_after_workflow(
                    &message,
                    &finalized_response,
                    &initial_draft_response,
                    "",
                    &response_tools,
                    true,
                    None,
                );
            if repair_outcome != "unchanged" {
                visible_response_repaired = true;
                tooling_fallback_used |= repair_tooling_used;
                comparative_fallback_used |= repair_comparative_used;
                let (contracted, report, retry_outcome) =
                    enforce_user_facing_finalization_contract(
                        &message,
                        repaired_response,
                        &response_tools,
                    );
                finalized_response = contracted;
                tool_completion = report;
                finalization_outcome =
                    merge_response_outcomes(&finalization_outcome, &repair_outcome, 180);
                finalization_outcome =
                    merge_response_outcomes(&finalization_outcome, &retry_outcome, 180);
            }
            tool_completion = enrich_tool_completion_receipt(tool_completion, &response_tools);
            let final_ack_only = response_looks_like_tool_ack_without_findings(&finalized_response);
            response_text = finalized_response;
            let mut response_finalization = json!({
                "applied": finalization_outcome != "unchanged",
                "outcome": finalization_outcome,
                "initial_ack_only": initial_ack_only,
                "final_ack_only": final_ack_only,
                "findings_available": tool_completion
                    .get("findings_available")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                "tool_completion": tool_completion,
                "tool_synthesis_retry_used": false,
                "pending_confirmation_replayed": replayed_pending_confirmation,
                "local_workspace_tooling_probe_turn": local_workspace_tooling_probe_turn,
                "tooling_fallback_used": tooling_fallback_used,
                "comparative_fallback_used": comparative_fallback_used,
                "workflow_system_fallback_used": workflow_system_fallback_used,
                "visible_response_repaired": visible_response_repaired,
                "retry_attempted": false,
                "retry_used": false
            });
            let visible_response_source = visible_response_source_for_turn(
                &response_text,
                workflow_used,
                visible_response_repaired,
                response_finalization
                    .get("outcome")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            );
            apply_visible_response_provenance(
                &mut response_workflow,
                &mut response_finalization,
                visible_response_source,
            );
            let process_summary = build_turn_process_summary(&message, &response_tools, &response_workflow, &response_finalization);
            let workflow_visibility = workflow_visibility_payload(&response_workflow, &response_finalization);
            let response_quality_telemetry = response_workflow.get("quality_telemetry").cloned().unwrap_or_else(|| json!({}));
            let terminal_transcript = tool_terminal_transcript(&response_tools);
            let turn_transaction = crate::dashboard_tool_turn_loop::turn_transaction_payload(
                "complete", "complete", "complete", "complete",
            );
            let previous_assistant =
                latest_assistant_message_text(&session_messages(&load_session_state(root, agent_id)));
            let mut turn_receipt = append_turn_message(root, agent_id, &message, &response_text);
            turn_receipt["assistant_turn_patch"] = persist_last_assistant_turn_metadata(
                root,
                agent_id,
                &response_text,
                &json!({
                    "tools": response_tools.clone(),
                    "response_workflow": response_workflow.clone(),
                    "response_finalization": response_finalization.clone(),
                    "process_summary": process_summary.clone(),
                    "workflow_visibility": workflow_visibility.clone(),
                    "response_quality_telemetry": response_quality_telemetry.clone(),
                    "terminal_transcript": terminal_transcript.clone(),
                    "turn_transaction": turn_transaction.clone()
                }),
            );
            turn_receipt["process_summary"] = process_summary.clone();
            turn_receipt["workflow_visibility"] = workflow_visibility.clone();
            turn_receipt["response_finalization"] = response_finalization.clone();
            turn_receipt["live_eval_monitor"] = live_eval_monitor_turn(
                root,
                agent_id,
                &message,
                &response_text,
                &previous_assistant,
                &response_finalization,
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": ok,
                    "agent_id": agent_id,
                    "provider": "tool",
                    "model": "tool-router",
                    "runtime_model": "tool-router",
                    "iterations": 1,
                    "input_tokens": estimate_tokens(&message),
                    "output_tokens": estimate_tokens(&response_text),
                    "cost_usd": 0.0,
                    "response": response_text,
                    "tools": response_tools,
                    "response_workflow": response_workflow,
                    "response_finalization": response_finalization,
                    "process_summary": process_summary,
                    "workflow_visibility": workflow_visibility,
                    "response_quality_telemetry": response_quality_telemetry,
                    "visible_response_source": visible_response_source,
                    "system_chat_injection_used": false,
                    "terminal_transcript": terminal_transcript,
                    "live_eval_monitor": turn_receipt.get("live_eval_monitor").cloned().unwrap_or_else(|| json!({})),
                    "turn_transaction": turn_transaction,
                    "workspace_hints": workspace_hints_value.clone(),
                    "latent_tool_candidates": latent_tool_candidates_value.clone(),
                    "attention_queue": turn_receipt.get("attention_queue").cloned().unwrap_or_else(|| json!({})),
                    "memory_capture": turn_receipt.get("memory_capture").cloned().unwrap_or_else(|| json!({}))
                }),
            });
        }
        let requested_provider = clean_text(
            row.get("model_provider")
                .and_then(Value::as_str)
                .unwrap_or("auto"),
            80,
        );
        let requested_model = clean_text(
            row.get("model_name").and_then(Value::as_str).unwrap_or(""),
            240,
        );
        let virtual_key_id = clean_text(
            request
                .get("virtual_key_id")
                .or_else(|| request.get("virtual_key"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let route_request = json!({
            "agent_id": agent_id,
            "message": message,
            "task_type": row.get("role").cloned().unwrap_or_else(|| json!("general")),
            "token_count": estimate_tokens(&message),
            "virtual_key_id": if virtual_key_id.is_empty() { Value::Null } else { json!(virtual_key_id.clone()) },
            "has_vision": request
                .get("attachments")
                .and_then(Value::as_array)
                .map(|rows| rows.iter().any(|row| {
                    clean_text(
                        row.get("content_type")
                            .or_else(|| row.get("mime_type"))
                            .and_then(Value::as_str)
                            .unwrap_or(""),
                        120,
                    )
                    .to_ascii_lowercase()
                    .starts_with("image/")
                }))
                .unwrap_or(false)
        });
        let prepared = match prepare_message_route_context(
            root,
            snapshot,
            &row,
            &request,
            &message,
            &route_request,
            &requested_provider,
            &requested_model,
            &virtual_key_id,
            agent_id,
            &workspace_hints_value,
            &latent_tool_candidates_value,
        ) {
            Ok(ctx) => ctx,
            Err(response) => return Some(response),
        };
        return handle_message_chat_response_pass(
            root, snapshot, &row, agent_id, &message, prepared,
        );
    }
    None
}
