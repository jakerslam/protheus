fn finalize_message_finalization_and_payload(
    root: &Path,
    agent_id: &str,
    message: &str,
    result: &Value,
    response_text: String,
    mut response_tools: Vec<Value>,
    workflow_mode: String,
    mut workflow_system_events: Vec<Value>,
    runtime_summary: Value,
    state: Value,
    messages: Vec<Value>,
    active_messages: Vec<Value>,
    provider: String,
    model: String,
    requested_provider: String,
    requested_model: String,
    auto_route: Option<Value>,
    virtual_key_id: String,
    virtual_key_gate: Value,
    fallback_window: i64,
    context_active_tokens: i64,
    context_ratio: f64,
    context_pressure: String,
    context_pool_limit_tokens: i64,
    context_pool_tokens: i64,
    pooled_messages_len: usize,
    sessions_total: usize,
    memory_kv_entries: usize,
    active_context_target_tokens: i64,
    active_context_min_recent: usize,
    include_all_sessions_context: bool,
    pre_generation_pruned: bool,
    recent_floor_enforced: bool,
    recent_floor_injected: usize,
    history_trim_confirmed: bool,
    emergency_compact: Value,
    workspace_hints: Value,
    latent_tool_candidates: Value,
    inline_tools_allowed: bool,
) -> CompatApiResponse {
    let initial_draft_response = clean_chat_text(&response_text, 32_000);
    let initial_ack_only = response_looks_like_tool_ack_without_findings(&initial_draft_response)
        || response_is_no_findings_placeholder(&initial_draft_response);
    let web_intent = natural_web_intent_from_user_message(message);
    let finalization_tool_gate = workflow_turn_tool_decision_tree(message);
    let automatic_web_fallback_enabled = finalization_tool_gate
        .get("automatic_tool_calls_allowed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let finalization_meta_control = finalization_tool_gate
        .get("meta_control_message")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let finalization_status_check = finalization_tool_gate
        .get("status_check_message")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let finalization_requires_live_web = finalization_tool_gate
        .get("requires_live_web")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let finalization_should_call_tools = finalization_tool_gate
        .get("should_call_tools")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let draft_retry_web_signal = automatic_web_fallback_enabled
        && draft_response_implies_retryable_web_failure(&initial_draft_response)
        && finalization_requires_live_web
        && finalization_should_call_tools
        && !finalization_meta_control
        && !finalization_status_check
        && !message_explicitly_disallows_tool_calls(message);
    let web_intent_route = web_intent
        .as_ref()
        .map(|(tool, _)| clean_text(tool, 80))
        .unwrap_or_default();
    let web_intent_detected = web_intent.is_some() || draft_retry_web_signal;
    let web_intent_source = if web_intent.is_some() {
        "message"
    } else if draft_retry_web_signal {
        "draft_retry_signal"
    } else {
        "none"
    };
    let web_intent_confidence = if web_intent.is_some() {
        0.92
    } else if draft_retry_web_signal {
        0.64
    } else {
        0.0
    };
    let mut web_forced_fallback_attempted = false;
    if automatic_web_fallback_enabled
        && web_intent_detected
        && !response_tools_include_web_attempt(&response_tools)
    {
        let fallback_query = web_intent
            .as_ref()
            .and_then(|(_, payload)| {
                payload
                    .get("query")
                    .or_else(|| payload.get("q"))
                    .and_then(Value::as_str)
                    .map(|raw| clean_text(raw, 600))
            })
            .filter(|query| !query.is_empty())
            .unwrap_or_else(|| {
                fallback_live_web_query_from_failed_draft(message, &initial_draft_response)
            });
        if !fallback_query.is_empty() {
            let forced_payload = execute_tool_call_with_recovery(
                root,
                &state,
                agent_id,
                None,
                "batch_query",
                &json!({
                    "source": "web",
                    "query": fallback_query.clone(),
                    "aperture": "medium",
                    "diagnostic": "forced_live_web_invariant"
                }),
            );
            let ok = forced_payload
                .get("ok")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let result_text = summarize_tool_payload("batch_query", &forced_payload);
            let status = tool_card_status_from_payload(&forced_payload);
            response_tools.push(json!({
                "id": format!("tool-batch_query-forced-{}", response_tools.len()),
                "name": "batch_query",
                "input": trim_text(
                    &json!({
                        "source": "web",
                        "query": fallback_query.clone(),
                        "aperture": "medium",
                        "diagnostic": "forced_live_web_invariant"
                    }).to_string(),
                    4000
                ),
                "result": trim_text(&result_text, 24_000),
                "is_error": !ok,
                "blocked": status == "blocked" || status == "policy_denied",
                "status": status,
                "tool_attempt_receipt": forced_payload
                    .pointer("/tool_pipeline/tool_attempt_receipt")
                    .cloned()
                    .unwrap_or(Value::Null)
            }));
            web_forced_fallback_attempted = true;
        }
    }
    let memory_fallback = if memory_recall_requested(message) {
        Some(build_memory_recall_response(&state, &messages, message))
    } else {
        None
    };
    let latest_assistant_text = latest_assistant_message_text(&active_messages);
    if response_tools.is_empty() && !message_explicitly_disallows_tool_calls(message) {
        if let Some(candidates) = latent_tool_candidates.as_array() {
            if !candidates.is_empty() {
                workflow_system_events.push(turn_workflow_event(
                    "manual_toolbox_candidate_menu",
                    json!({
                        "selection_authority": "llm_only",
                        "automatic_execution_allowed": false,
                        "candidate_count": candidates.len(),
                        "candidates": candidates.iter().take(4).cloned().collect::<Vec<_>>()
                    }),
                ));
            }
        }
    }
    let mut response_workflow = run_turn_workflow_final_response(
        root,
        &provider,
        &model,
        &active_messages,
        message,
        &workflow_mode,
        &response_tools,
        &workflow_system_events,
        &response_text,
        &latest_assistant_text,
    );
    let mut response_text = response_workflow
        .get("response")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_default();
    let mut finalized_response = clean_chat_text(&response_text, 32_000);
    let mut tool_completion = json!({});
    let workflow_status = workflow_final_response_status(&response_workflow);
    let mut workflow_used = workflow_final_response_used(&response_workflow);
    let workflow_fallback_allowed = workflow_final_response_allows_system_fallback(&response_workflow);
    let mut finalization_outcome = if workflow_used {
        "workflow_authored".to_string()
    } else {
        "workflow_llm_unavailable".to_string()
    };
    if !workflow_status.is_empty() {
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            &format!("workflow:{workflow_status}"),
            200,
        );
    }
    let mut tooling_fallback_used = false;
    let mut comparative_fallback_used = false;
    let workflow_system_fallback_used = false;
    let mut visible_response_repaired = false;
    let mut final_fallback_used = false;
    if workflow_used {
        tool_completion = tool_completion_report_for_response(
            &finalized_response,
            &response_tools,
            "workflow_authored",
        );
    } else if workflow_fallback_allowed {
        // Policy: never inject system-authored fallback text into chat.
        let llm_only_candidate = initial_draft_response.clone();
        let (contract_finalized, contract_report, contract_outcome) =
            enforce_user_facing_finalization_contract(message, llm_only_candidate, &response_tools);
        finalized_response = contract_finalized;
        tool_completion = contract_report;
        finalization_outcome =
            merge_response_outcomes(&finalization_outcome, "workflow_no_system_fallback", 200);
        finalization_outcome =
            merge_response_outcomes(&finalization_outcome, &contract_outcome, 200);
    } else {
        // Keep chat output LLM-authored only, even when workflow final synthesis is unavailable.
        let llm_only_candidate = initial_draft_response.clone();
        let (contract_finalized, contract_report, contract_outcome) =
            enforce_user_facing_finalization_contract(message, llm_only_candidate, &response_tools);
        finalized_response = contract_finalized;
        tool_completion = contract_report;
        finalization_outcome =
            merge_response_outcomes(&finalization_outcome, "workflow_no_system_fallback", 200);
        finalization_outcome =
            merge_response_outcomes(&finalization_outcome, &contract_outcome, 200);
    }
    let repair_candidate_contamination = response_contains_stale_code_context_dump(message, &finalized_response)
        || response_contains_unrequested_content_without_tool_evidence(
            message,
            &finalized_response,
            &response_tools,
        );
    let (repaired_response, repair_outcome, repair_tooling_used, repair_comparative_used) =
        repair_visible_response_after_workflow(
            message,
            &finalized_response,
            &initial_draft_response,
            &latest_assistant_text,
            &response_tools,
            inline_tools_allowed,
            memory_fallback.as_deref(),
        );
    if repair_outcome != "unchanged" {
        visible_response_repaired = true;
        final_fallback_used = true;
        tooling_fallback_used |= repair_tooling_used;
        comparative_fallback_used |= repair_comparative_used;
        let (contract_finalized, contract_report, contract_outcome) =
            enforce_user_facing_finalization_contract(message, repaired_response, &response_tools);
        finalized_response = contract_finalized;
        tool_completion = contract_report;
        finalization_outcome = merge_response_outcomes(&finalization_outcome, &repair_outcome, 200);
        finalization_outcome =
            merge_response_outcomes(&finalization_outcome, &contract_outcome, 200);
    }
    tool_completion = enrich_tool_completion_receipt(tool_completion, &response_tools);
    response_text = finalized_response;
    if response_text.trim().is_empty() {
        let prior_manual_toolbox_pending_tool_request = response_workflow
            .get("manual_toolbox_pending_tool_request")
            .filter(|value| value.is_object())
            .cloned();
        let mut recovery_events = workflow_system_events.clone();
        recovery_events.push(turn_workflow_event(
            "empty_final_response_menu_recovery",
            json!({
                "selection_authority": "llm_only",
                "automatic_execution_allowed": false,
                "prior_finalization_outcome": finalization_outcome.clone(),
                "latent_tool_candidates": latent_tool_candidates.clone()
            }),
        ));
        let mut recovered_workflow = run_turn_workflow_final_response(
            root,
            &provider,
            &model,
            &active_messages,
            message,
            &workflow_mode,
            &response_tools,
            &recovery_events,
            "",
            &latest_assistant_text,
        );
        if recovered_workflow
            .get("manual_toolbox_pending_tool_request")
            .filter(|value| value.is_object())
            .is_none()
        {
            if let Some(pending_request) = prior_manual_toolbox_pending_tool_request {
                recovered_workflow["manual_toolbox_pending_tool_request"] = pending_request;
            }
        }
        let recovered_text = clean_chat_text(
            recovered_workflow
                .get("response")
                .and_then(Value::as_str)
                .unwrap_or(""),
            32_000,
        );
        if workflow_final_response_used(&recovered_workflow) && !recovered_text.trim().is_empty() {
            let (contract_finalized, contract_report, contract_outcome) =
                enforce_user_facing_finalization_contract(message, recovered_text, &response_tools);
            if !contract_finalized.trim().is_empty() {
                response_workflow = recovered_workflow;
                response_text = contract_finalized;
                tool_completion = enrich_tool_completion_receipt(contract_report, &response_tools);
                workflow_used = true;
                finalization_outcome = merge_response_outcomes(
                    &finalization_outcome,
                    "empty_final_response_recovered_by_llm_menu",
                    220,
                );
                finalization_outcome =
                    merge_response_outcomes(&finalization_outcome, &contract_outcome, 220);
            }
        }
    }
    let manual_toolbox_pending_tool_request = response_workflow
        .get("manual_toolbox_pending_tool_request")
        .filter(|value| value.is_object())
        .cloned();
    if let Some(pending_request) = manual_toolbox_pending_tool_request.as_ref() {
        let pending_tool = clean_text(
            pending_request
                .get("tool_name")
                .or_else(|| pending_request.get("tool"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        if !pending_tool.is_empty() {
            let pending_input = pending_request
                .get("input")
                .cloned()
                .unwrap_or_else(|| json!({}));
            store_pending_tool_confirmation(
                root,
                agent_id,
                &pending_tool,
                &pending_input,
                pending_request
                    .get("source")
                    .and_then(Value::as_str)
                    .unwrap_or("manual_toolbox_gate"),
            );
            response_workflow["pending_tool_request"] = pending_request.clone();
            finalization_outcome = merge_response_outcomes(
                &finalization_outcome,
                "manual_toolbox_pending_tool_request",
                200,
            );
        }
    }
    let web_tool_attempted = response_tools_include_web_attempt(&response_tools);
    let web_tool_blocked = response_tools_web_blocked(&response_tools);
    let web_tool_low_signal = response_tools_web_low_signal(&response_tools);
    let web_turn_classification = classify_web_turn_state(
        web_intent_detected,
        web_tool_attempted,
        web_tool_blocked,
        web_tool_low_signal,
    );
    let mut web_failure_code = web_failure_code_from_response_tools(&response_tools);
    if web_intent_detected && !web_tool_attempted {
        web_failure_code = "web_route_parse_failed".to_string();
    } else if web_failure_code.is_empty() && web_tool_low_signal {
        web_failure_code = "web_tool_low_signal".to_string();
    }
    let tooling_attempted = !response_tools.is_empty();
    let tooling_blocked = response_tools_any_blocked(&response_tools);
    let tooling_low_signal = response_tools_any_low_signal(&response_tools);
    let tooling_failure_code = tool_failure_code_from_response_tools(&response_tools);
    let tooling_turn_classification = if !tooling_attempted {
        "not_attempted".to_string()
    } else if tooling_blocked {
        "policy_blocked".to_string()
    } else if tooling_low_signal {
        "low_signal".to_string()
    } else if !tooling_failure_code.is_empty() {
        "failed".to_string()
    } else {
        "healthy".to_string()
    };
    let mut tooling_invariant_repair_used = false;
    let mut web_invariant_repair_used = false;
    if web_intent_detected && !web_tool_attempted {
        web_invariant_repair_used = true;
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            "web_invariant_missing_tool_attempt",
            200,
        );
    } else if web_tool_attempted
        && (web_tool_blocked || web_tool_low_signal || !web_failure_code.is_empty())
    {
        web_invariant_repair_used = true;
        finalization_outcome =
            merge_response_outcomes(&finalization_outcome, "web_failure_code_appended", 200);
    }
    if tooling_attempted {
        tooling_invariant_repair_used = true;
        finalization_outcome = merge_response_outcomes(&finalization_outcome, "tooling_failure_code_appended", 200);
    }
    let mut response_guard =
        final_response_guard_report(message, &response_text, &response_tools, repair_candidate_contamination);
    if response_guard_bool(&response_guard, "final_contract_violation") {
        // Do not synthesize deterministic system fallback text in chat.
        response_text.clear();
        if response_guard_bool(&response_guard, "final_contamination_violation") {
            bump_workflow_quality_counter(&mut response_workflow, "contamination_reject");
        }
        if response_guard_bool(&response_guard, "current_turn_dominance_violation") {
            bump_workflow_quality_counter(&mut response_workflow, "current_turn_dominance_reject");
        }
        if response_guard_bool(&response_guard, "unsupported_tool_success_claim") {
            bump_workflow_quality_counter(
                &mut response_workflow,
                "unsupported_tool_success_claim_reject",
            );
        }
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            final_response_guard_outcome(&response_guard),
            200,
        );
        let mut guard_recovery_events = workflow_system_events.clone();
        guard_recovery_events.push(turn_workflow_event(
            "final_response_guard_recovery",
            json!({
                "selection_authority": "llm_only",
                "automatic_execution_allowed": false,
                "guard_outcome": final_response_guard_outcome(&response_guard),
                "visible_gate_choice_leakage": response_guard_bool(&response_guard, "visible_gate_choice_leakage"),
                "unsupported_tool_success_claim": response_guard_bool(&response_guard, "unsupported_tool_success_claim")
            }),
        ));
        let recovered_workflow = run_turn_workflow_final_response(
            root,
            &provider,
            &model,
            &active_messages,
            message,
            &workflow_mode,
            &response_tools,
            &guard_recovery_events,
            "",
            &latest_assistant_text,
        );
        let recovered_text = clean_chat_text(
            recovered_workflow
                .get("response")
                .and_then(Value::as_str)
                .unwrap_or(""),
            32_000,
        );
        if workflow_final_response_used(&recovered_workflow) && !recovered_text.trim().is_empty() {
            let recovered_guard =
                final_response_guard_report(message, &recovered_text, &response_tools, false);
            if !response_guard_bool(&recovered_guard, "final_contract_violation") {
                let (contract_finalized, contract_report, contract_outcome) =
                    enforce_user_facing_finalization_contract(message, recovered_text, &response_tools);
                if !contract_finalized.trim().is_empty() {
                    response_workflow = recovered_workflow;
                    response_text = contract_finalized;
                    tool_completion = enrich_tool_completion_receipt(contract_report, &response_tools);
                    workflow_used = true;
                    finalization_outcome = merge_response_outcomes(
                        &finalization_outcome,
                        "final_response_guard_recovered_by_llm",
                        220,
                    );
                    finalization_outcome =
                        merge_response_outcomes(&finalization_outcome, &contract_outcome, 220);
                    response_guard = final_response_guard_report(
                        message,
                        &response_text,
                        &response_tools,
                        false,
                    );
                }
            }
        }
    }
    let tool_gate_should_call_tools = response_workflow
        .pointer("/tool_gate/should_call_tools")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let direct_answer_rate = response_workflow_quality_rate(&response_workflow, "direct_answer_rate");
    let retry_rate = response_workflow_quality_rate(&response_workflow, "retry_rate");
    let off_topic_reject_rate =
        response_workflow_quality_rate(&response_workflow, "off_topic_reject_rate");
    let tool_overcall_rate = if !tool_gate_should_call_tools && tooling_attempted {
        1.0
    } else {
        0.0
    };
    response_workflow["quality_telemetry"]["final_fallback_used"] =
        Value::Bool(final_fallback_used);
    let final_ack_only = response_looks_like_tool_ack_without_findings(&response_text);
    let response_quality_telemetry = build_response_quality_telemetry_payload(
        &response_workflow,
        final_fallback_used,
        tooling_invariant_repair_used,
        &tooling_failure_code,
        direct_answer_rate,
        retry_rate,
        tool_overcall_rate,
        off_topic_reject_rate,
    );
    let tooling_invariant = json!({
        "tool_attempted": tooling_attempted,
        "tool_blocked": tooling_blocked,
        "low_signal": tooling_low_signal,
        "classification": tooling_turn_classification,
        "failure_code": tooling_failure_code,
        "invariant_repair_used": tooling_invariant_repair_used
    });
    let web_invariant = json!({
        "requires_live_web": web_intent_detected,
        "intent_source": web_intent_source,
        "intent_confidence": web_intent_confidence,
        "selected_route": web_intent_route.clone(),
        "tool_attempted": web_tool_attempted,
        "tool_blocked": web_tool_blocked,
        "low_signal": web_tool_low_signal,
        "classification": web_turn_classification,
        "failure_code": web_failure_code,
        "forced_fallback_attempted": web_forced_fallback_attempted,
        "invariant_repair_used": web_invariant_repair_used
    });
    let mut response_finalization = build_response_finalization_payload(
        &finalization_outcome,
        initial_ack_only,
        final_ack_only,
        &tool_completion,
        tooling_fallback_used,
        comparative_fallback_used,
        workflow_system_fallback_used,
        visible_response_repaired,
        &response_quality_telemetry,
        &tooling_invariant,
        &web_invariant,
    );
    let workflow_direct_response_path = response_workflow
        .pointer("/workflow_control/direct_response_path")
        .and_then(Value::as_str)
        .unwrap_or(if manual_toolbox_pending_tool_request.is_some() {
            "gate_1_yes_pending_tool_confirmation"
        } else {
            "gate_1_no"
        });
    response_finalization["workflow_control"] = json!({
        "mode": "tool_menu_interface_v1",
        "direct_response_path": workflow_direct_response_path
    });
    apply_response_guard_payloads(&mut response_finalization, &response_guard);
    if let Some(pending_request) = manual_toolbox_pending_tool_request.as_ref() {
        response_finalization["pending_tool_request"] = pending_request.clone();
    }
    let visible_response_source = apply_visible_response_provenance_for_turn(
        &mut response_workflow,
        &mut response_finalization,
        &response_text,
        workflow_used,
        visible_response_repaired,
        &finalization_outcome,
    );
    let process_summary = build_turn_process_summary(
        message,
        &response_tools,
        &response_workflow,
        &response_finalization,
    );
    let workflow_visibility = workflow_visibility_payload(&response_workflow, &response_finalization);
    let turn_transaction = crate::dashboard_tool_turn_loop::turn_transaction_payload(
        "complete",
        if response_tools.is_empty() {
            "none"
        } else {
            "complete"
        },
        "complete",
        "complete",
    );
    let terminal_transcript = tool_terminal_transcript(&response_tools);
    let empty_response_has_guard_diagnostics =
        response_guard_bool(&response_guard, "final_contamination_violation")
            || response_guard_bool(&response_guard, "current_turn_dominance_violation")
            || response_guard_bool(&response_guard, "unsupported_tool_success_claim")
            || response_guard_bool(&response_guard, "visible_gate_choice_leakage");
    if response_text.trim().is_empty() && !empty_response_has_guard_diagnostics {
        return final_response_empty_message_response(
            root,
            agent_id,
            message,
            &provider,
            &model,
            workspace_hints,
            latent_tool_candidates,
        );
    }
    let turn_receipt = append_turn_receipt_with_metadata(
        root,
        agent_id,
        message,
        &response_text,
        Value::Array(response_tools.clone()),
        &response_workflow,
        &response_finalization,
        &process_summary,
        &turn_transaction,
        &terminal_transcript,
    );
    let agent_health_snapshot = persist_agent_control_plane_health_snapshot_for_turn(root, agent_id, message, &response_text, &response_workflow, &response_finalization, &process_summary, &turn_receipt);
    let runtime_provider = clean_text(
        response_workflow
            .pointer("/final_llm_response/provider")
            .or_else(|| response_workflow.get("provider"))
            .and_then(Value::as_str)
            .unwrap_or(&provider),
        80,
    );
    let runtime_model = clean_text(
        response_workflow
            .pointer("/final_llm_response/runtime_model")
            .or_else(|| response_workflow.get("runtime_model"))
            .or_else(|| result.get("runtime_model"))
            .or_else(|| result.get("model"))
            .and_then(Value::as_str)
            .unwrap_or(&model),
        240,
    );
    let mut runtime_patch = json!({
        "runtime_model": runtime_model,
        "context_window": result.get("context_window").cloned().unwrap_or_else(|| json!(0)),
        "context_window_tokens": result.get("context_window").cloned().unwrap_or_else(|| json!(0)),
        "updated_at": crate::now_iso()
    });
    if auto_route.is_some() {
        runtime_patch["runtime_provider"] = json!(runtime_provider.clone());
        if !requested_provider.eq_ignore_ascii_case("auto")
            && !requested_model.is_empty()
            && !requested_model.eq_ignore_ascii_case("auto")
        {
            runtime_patch["model_provider"] = json!(runtime_provider.clone());
            runtime_patch["model_name"] = json!(runtime_model.clone());
            runtime_patch["model_override"] = json!(format!("{runtime_provider}/{runtime_model}"));
        }
    }
    let _ = update_profile_patch(root, agent_id, &runtime_patch);
    let mut payload = result.clone();
    payload["ok"] = json!(true);
    payload["agent_id"] = json!(agent_id);
    payload["provider"] = json!(runtime_provider);
    payload["model"] = json!(runtime_model.clone());
    payload["runtime_model"] = json!(runtime_model);
    payload["iterations"] = json!(1);
    payload["response"] = json!(response_text);
    payload["runtime_sync"] = runtime_summary;
    payload["tools"] = Value::Array(response_tools);
    payload["response_workflow"] = response_workflow;
    payload["terminal_transcript"] = Value::Array(terminal_transcript);
    payload["response_finalization"] = response_finalization;
    if let Some(pending_request) = manual_toolbox_pending_tool_request {
        payload["pending_tool_request"] = pending_request;
    }
    payload["process_summary"] = process_summary;
    payload["workflow_visibility"] = workflow_visibility;
    payload["response_quality_telemetry"] = response_quality_telemetry;
    payload["visible_response_source"] = json!(visible_response_source);
    payload["system_chat_injection_used"] = json!(false);
    payload["web_intent"] = json!({
        "detected": web_intent_detected,
        "source": web_intent_source,
        "confidence": web_intent_confidence,
        "selected_route": web_intent_route
    });
    payload["turn_transaction"] = turn_transaction;
    payload["context_window"] = json!(fallback_window.max(0));
    payload["context_tokens"] = json!(context_active_tokens.max(0));
    payload["context_used_tokens"] = json!(context_active_tokens.max(0));
    payload["context_ratio"] = json!(context_ratio);
    payload["context_pressure"] = json!(context_pressure.clone());
    payload["attention_queue"] = turn_receipt.get("attention_queue").cloned().unwrap_or_else(|| json!({}));
    payload["live_eval_monitor"] = turn_receipt.get("live_eval_monitor").cloned().unwrap_or_else(|| json!({}));
    payload["dashboard_health_indicator"] = agent_health_snapshot.get("dashboard_health_indicator").cloned().unwrap_or_else(|| json!({}));
    payload["agent_health_snapshot"] = agent_health_snapshot;
    payload["memory_capture"] = turn_receipt
        .get("memory_capture")
        .cloned()
        .unwrap_or_else(|| json!({}));
    payload["context_pool"] = json!({
        "pool_limit_tokens": context_pool_limit_tokens,
        "pool_tokens": context_pool_tokens,
        "pool_messages": pooled_messages_len,
        "session_count": sessions_total,
        "system_context_enabled": true,
        "system_context_limit_tokens": context_pool_limit_tokens,
        "llm_context_window_tokens": fallback_window.max(0),
        "cross_session_memory_enabled": true,
        "memory_kv_entries": memory_kv_entries,
        "active_target_tokens": active_context_target_tokens,
        "active_tokens": context_active_tokens,
        "active_messages": active_messages.len(),
        "min_recent_messages": active_context_min_recent,
        "include_all_sessions_context": include_all_sessions_context,
        "context_window": fallback_window.max(0),
        "context_ratio": context_ratio,
        "context_pressure": context_pressure,
        "pre_generation_pruning_enabled": true,
        "pre_generation_pruned": pre_generation_pruned,
        "recent_floor_enforced": recent_floor_enforced,
        "recent_floor_injected": recent_floor_injected,
        "history_trim_confirmed": history_trim_confirmed,
        "emergency_compact_enabled": true,
        "emergency_compact": emergency_compact
    });
    payload["workspace_hints"] = workspace_hints;
    payload["latent_tool_candidates"] = latent_tool_candidates;
    if let Some(route) = auto_route {
        payload["auto_route"] = route.get("route").cloned().unwrap_or_else(|| route.clone());
    }
    if !virtual_key_id.is_empty() {
        let spend_receipt = crate::dashboard_provider_runtime::record_virtual_key_usage(
            root,
            &virtual_key_id,
            payload
                .get("cost_usd")
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
        );
        payload["virtual_key"] = json!({
            "id": virtual_key_id,
            "reservation": virtual_key_gate,
            "spend": spend_receipt
        });
    }
    CompatApiResponse {
        status: 200,
        payload,
    }
}
