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
            let tool_card = json!({
                "id": format!("tool-direct-{}", normalize_tool_name(&tool_name)),
                "name": normalize_tool_name(&tool_name),
                "input": trim_text(&tool_input.to_string(), 4000),
                "result": trim_text(&summarize_tool_payload(&tool_name, &tool_payload), 24_000),
                "is_error": !ok,
                "blocked": tool_payload
                    .pointer("/tool_pipeline/tool_attempt_receipt/status")
                    .and_then(Value::as_str)
                    .map(|status| status == "blocked" || status == "policy_denied")
                    .unwrap_or(false),
                "status": tool_payload
                    .pointer("/tool_pipeline/tool_attempt_receipt/status")
                    .cloned()
                    .unwrap_or_else(|| json!(if ok { "ok" } else { "error" })),
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
                enforce_user_facing_finalization_contract(response_text, &response_tools);
            let mut tooling_fallback_used = false;
            let mut comparative_fallback_used = false;
            let mut tool_synthesis_retry_used = false;
            let mut finalized_response = finalized_response;
            let mut finalization_outcome = clean_text(&finalization_seed, 180);
            let mut tool_completion = tool_completion;
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
            if let Some(synthesized) = maybe_synthesize_tool_turn_response(
                root,
                &synthesis_provider,
                &synthesis_model,
                &synthesis_history,
                &message,
                &response_tools,
                &finalized_response,
            ) {
                let (contracted, report, retry_outcome) =
                    enforce_user_facing_finalization_contract(synthesized, &response_tools);
                finalized_response = contracted;
                tool_completion = report;
                finalization_outcome = merge_response_outcomes(
                    &finalization_outcome,
                    &format!("tool_synthesis_retry:{retry_outcome}"),
                    180,
                );
                tool_synthesis_retry_used = true;
            }
            if let Some(tooling_fallback) =
                maybe_tooling_failure_fallback(&message, &finalized_response, "")
            {
                finalized_response = tooling_fallback;
                finalization_outcome = format!("{finalization_outcome}+tooling_failure_fallback");
                tooling_fallback_used = true;
                let (contracted, report, retry_outcome) =
                    enforce_user_facing_finalization_contract(finalized_response, &response_tools);
                finalized_response = contracted;
                tool_completion = report;
                finalization_outcome =
                    merge_response_outcomes(&finalization_outcome, &retry_outcome, 180);
            }
            if response_is_no_findings_placeholder(&finalized_response)
                && message_requests_live_web_comparison(&message)
            {
                comparative_fallback_used = true;
                finalized_response = comparative_no_findings_fallback(&message);
                finalization_outcome =
                    merge_response_outcomes(&finalization_outcome, "comparative_fallback", 180);
                let (contracted, report, retry_outcome) =
                    enforce_user_facing_finalization_contract(finalized_response, &response_tools);
                finalized_response = contracted;
                tool_completion = report;
                finalization_outcome =
                    merge_response_outcomes(&finalization_outcome, &retry_outcome, 180);
            }
            tool_completion = enrich_tool_completion_receipt(tool_completion, &response_tools);
            finalized_response = ensure_tool_turn_response_text(&finalized_response, &response_tools);
            let final_ack_only = response_looks_like_tool_ack_without_findings(&finalized_response);
            response_text = finalized_response;
            let response_finalization = json!({
                "applied": finalization_outcome != "unchanged",
                "outcome": finalization_outcome,
                "initial_ack_only": tool_completion
                    .get("initial_ack_only")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                "final_ack_only": final_ack_only,
                "findings_available": tool_completion
                    .get("findings_available")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                "tool_completion": tool_completion,
                "tool_synthesis_retry_used": tool_synthesis_retry_used,
                "pending_confirmation_replayed": replayed_pending_confirmation,
                "tooling_fallback_used": tooling_fallback_used,
                "comparative_fallback_used": comparative_fallback_used,
                "retry_attempted": false,
                "retry_used": false
            });
            let turn_transaction = crate::dashboard_tool_turn_loop::turn_transaction_payload(
                "complete", "complete", "complete", "complete",
            );
            let mut turn_receipt = append_turn_message(root, agent_id, &message, &response_text);
            turn_receipt["assistant_turn_patch"] = persist_last_assistant_turn_metadata(
                root,
                agent_id,
                &response_text,
                &json!({
                    "tools": response_tools.clone(),
                    "response_finalization": response_finalization.clone(),
                    "turn_transaction": turn_transaction.clone()
                }),
            );
            turn_receipt["response_finalization"] = response_finalization.clone();
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
                    "response_finalization": response_finalization,
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
