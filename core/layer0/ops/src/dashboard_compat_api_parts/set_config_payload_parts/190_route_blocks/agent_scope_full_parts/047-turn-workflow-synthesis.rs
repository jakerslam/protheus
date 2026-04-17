fn turn_workflow_event(kind: &str, detail: Value) -> Value {
    json!({
        "kind": clean_text(kind, 80),
        "detail": detail
    })
}

fn bump_workflow_quality_counter(workflow: &mut Value, key: &str) {
    let pointer = format!("/quality_telemetry/{key}");
    let current = workflow
        .pointer(&pointer)
        .and_then(Value::as_u64)
        .unwrap_or(0);
    workflow["quality_telemetry"][key] = json!(current + 1);
}

fn response_tool_workflow_events(response_tools: &[Value]) -> Vec<Value> {
    let mut events = Vec::<Value>::new();
    let mut seen = HashSet::<String>::new();
    for tool in response_tools.iter().take(8) {
        let tool_name = normalize_tool_name(tool.get("name").and_then(Value::as_str).unwrap_or("tool"));
        if tool_name.is_empty() {
            continue;
        }
        let status = clean_text(tool.get("status").and_then(Value::as_str).unwrap_or(""), 80)
            .to_ascii_lowercase();
        let blocked = tool.get("blocked").and_then(Value::as_bool).unwrap_or(false);
        let is_error = tool.get("is_error").and_then(Value::as_bool).unwrap_or(false);
        let result = clean_text(tool.get("result").and_then(Value::as_str).unwrap_or(""), 600);
        let attempt_reason = clean_text(
            tool.pointer("/tool_attempt_receipt/reason")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let attempt_backend = clean_text(
            tool.pointer("/tool_attempt_receipt/backend")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let low_signal = !result.is_empty()
            && (response_looks_like_tool_ack_without_findings(&result)
                || response_is_no_findings_placeholder(&result)
                || response_looks_like_unsynthesized_web_snippet_dump(&result)
                || response_looks_like_raw_web_artifact_dump(&result));
        let event_kind = if blocked || matches!(status.as_str(), "blocked" | "policy_denied") {
            "tool_blocked"
        } else if matches!(status.as_str(), "timeout") {
            "tool_timeout"
        } else if is_error || matches!(status.as_str(), "error" | "failed" | "execution_error") {
            "tool_failed"
        } else if low_signal || matches!(status.as_str(), "no_results") {
            "tool_low_signal"
        } else {
            "tool_completed"
        };
        let key = format!("{tool_name}:{event_kind}:{status}:{attempt_reason}");
        if !seen.insert(key) {
            continue;
        }
        events.push(turn_workflow_event(
            event_kind,
            json!({
                "tool_name": tool_name,
                "status": status,
                "blocked": blocked,
                "is_error": is_error,
                "reason": attempt_reason,
                "backend": attempt_backend,
                "result_excerpt": first_sentence(&result, 220)
            }),
        ));
    }
    events
}

fn build_turn_workflow_events(
    response_tools: &[Value],
    pending_confirmation: Option<&Value>,
    replayed_pending_confirmation: bool,
) -> Vec<Value> {
    let mut events = response_tool_workflow_events(response_tools);
    if let Some(pending) = pending_confirmation {
        let tool_name = clean_text(
            pending
                .get("tool_name")
                .or_else(|| pending.get("tool"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let source = clean_text(
            pending.get("source").and_then(Value::as_str).unwrap_or(""),
            80,
        );
        events.push(turn_workflow_event(
            "pending_confirmation_required",
            json!({
                "tool_name": tool_name,
                "source": source
            }),
        ));
    }
    if replayed_pending_confirmation {
        events.push(turn_workflow_event(
            "pending_confirmation_replayed",
            json!({"ok": true}),
        ));
    }
    events
}

fn workflow_final_response_status(workflow: &Value) -> String {
    clean_text(
        workflow
            .pointer("/final_llm_response/status")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    )
}

fn workflow_final_response_used(workflow: &Value) -> bool {
    workflow
        .pointer("/final_llm_response/used")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && workflow
            .get("response")
            .and_then(Value::as_str)
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
}

fn workflow_final_response_allows_system_fallback(workflow: &Value) -> bool {
    matches!(
        workflow_final_response_status(workflow).as_str(),
        "invoke_failed" | "synthesis_failed" | "skipped_missing_model" | "skipped_test"
    )
}

fn tool_completion_report_for_response(
    response_text: &str,
    response_tools: &[Value],
    outcome: &str,
) -> Value {
    let cleaned = clean_chat_text(response_text, 32_000);
    let findings = clean_text(&response_tools_summary_for_user(response_tools, 4), 4_000);
    let failure_reason = clean_text(
        &response_tools_failure_reason_for_user(response_tools, 4),
        4_000,
    );
    let reasoning_source = if !cleaned.is_empty() {
        cleaned.clone()
    } else if !failure_reason.is_empty() {
        failure_reason.clone()
    } else {
        findings.clone()
    };
    let completion_state = if response_tools.is_empty() {
        "not_applicable"
    } else if !failure_reason.is_empty() {
        "reported_reason"
    } else if !findings.is_empty() {
        "reported_findings"
    } else {
        "reported_no_findings"
    };
    let deferred_execution = response_is_deferred_execution_preamble(&cleaned)
        || response_is_deferred_retry_prompt(&cleaned);
    json!({
        "completion_state": completion_state,
        "findings_available": !findings.is_empty(),
        "final_ack_only": response_looks_like_tool_ack_without_findings(&cleaned),
        "final_no_findings": response_is_no_findings_placeholder(&cleaned),
        "final_deferred_execution": deferred_execution,
        "final_requests_more_tooling": workflow_response_requests_more_tooling(&cleaned),
        "reasoning": first_sentence(&reasoning_source, 220),
        "outcome": clean_text(outcome, 200)
    })
}

fn augment_turn_workflow_events_for_final_response(
    message: &str,
    response_tools: &[Value],
    workflow_events: &[Value],
    draft_response: &str,
    latest_assistant_text: &str,
) -> Vec<Value> {
    let mut events = workflow_events.to_vec();
    let cleaned_draft = clean_text(draft_response, 4_000);
    if response_is_no_findings_placeholder(&cleaned_draft) {
        events.push(turn_workflow_event(
            "draft_response_invalid",
            json!({
                "reason": "no_findings_placeholder",
                "draft_excerpt": first_sentence(&cleaned_draft, 220)
            }),
        ));
    } else if response_looks_like_tool_ack_without_findings(&cleaned_draft) {
        events.push(turn_workflow_event(
            "draft_response_invalid",
            json!({
                "reason": "ack_only",
                "draft_excerpt": first_sentence(&cleaned_draft, 220)
            }),
        ));
    } else if response_is_deferred_execution_preamble(&cleaned_draft)
        || response_is_deferred_retry_prompt(&cleaned_draft)
        || workflow_response_requests_more_tooling(&cleaned_draft)
    {
        events.push(turn_workflow_event(
            "draft_response_invalid",
            json!({
                "reason": "deferred_retry_prompt",
                "draft_excerpt": first_sentence(&cleaned_draft, 220)
            }),
        ));
    }
    let findings = clean_text(&response_tools_summary_for_user(response_tools, 4), 2_000);
    if !findings.is_empty() {
        events.push(turn_workflow_event(
            "tool_findings_summary",
            json!({
                "summary": findings
            }),
        ));
    }
    let failure_summary = clean_text(
        &response_tools_failure_reason_for_user(response_tools, 4),
        2_000,
    );
    if !failure_summary.is_empty() {
        events.push(turn_workflow_event(
            "tool_failure_summary",
            json!({
                "summary": failure_summary
            }),
        ));
    }
    if !response_tools.is_empty() {
        let readability_hint = clean_text(
            &ensure_tool_turn_response_text(draft_response, response_tools),
            2_000,
        );
        if !readability_hint.is_empty() && readability_hint != cleaned_draft {
            events.push(turn_workflow_event(
                "tool_response_readability_guidance",
                json!({
                    "suggested_response": readability_hint
                }),
            ));
        }
    }
    if let Some(tooling_guidance) =
        maybe_tooling_failure_fallback(message, draft_response, latest_assistant_text)
    {
        events.push(turn_workflow_event(
            "tooling_failure_guidance",
            json!({
                "suggested_response": clean_text(&tooling_guidance, 2_000)
            }),
        ));
    }
    if message_requests_comparative_answer(message) {
        events.push(turn_workflow_event(
            "comparative_answer_requested",
            json!({
                "live_web_focus": message_requests_live_web_comparison(message)
            }),
        ));
        if response_is_no_findings_placeholder(&cleaned_draft) || !failure_summary.is_empty() {
            events.push(turn_workflow_event(
                "comparative_guidance",
                json!({
                    "suggested_response": clean_text(
                        &comparative_no_findings_fallback(message),
                        2_000,
                    )
                }),
            ));
        }
    }
    events
}

#[cfg(test)]
fn workflow_test_llm_enabled(root: &Path) -> bool {
    root.join("client/runtime/local/state/ui/infring_dashboard/test_chat_script.json")
        .exists()
        || matches!(
            std::env::var("INFRING_LIVE_WEB_TOOLING_SMOKE")
                .ok()
                .as_deref()
                .map(|value| value.trim().to_ascii_lowercase()),
            Some(ref value) if value == "1" || value == "true" || value == "yes"
        )
}

#[cfg(not(test))]
fn workflow_test_llm_enabled(_root: &Path) -> bool {
    false
}

fn workflow_response_template_label(message: &str) -> &'static str {
    let lowered = clean_text(message, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return "quick_qa";
    }
    if message_is_tooling_status_check(message) || lowered.starts_with("did you") {
        return "status_check";
    }
    if lowered.contains("debug")
        || lowered.contains("root cause")
        || lowered.contains("why")
        || lowered.contains("diagnos")
    {
        return "debug_diagnosis";
    }
    if message_requests_comparative_answer(message) || lowered.contains("compare") {
        return "compare";
    }
    if lowered.contains("implement")
        || lowered.contains("patch")
        || lowered.contains("fix")
        || lowered.contains("build")
        || lowered.contains("create")
        || lowered.contains("wire")
    {
        return "implement_request";
    }
    "quick_qa"
}

fn workflow_template_instruction_for_label(label: &str) -> &'static str {
    match label {
        "status_check" => {
            "Template: Start with a direct status line in the first sentence, then explain evidence in 1-3 concise bullets."
        }
        "debug_diagnosis" => {
            "Template: First sentence should state the likely root cause. Then provide the top 1-3 fixes in priority order."
        }
        "compare" => {
            "Template: Give a concise side-by-side comparison with 2-4 bullets and call out practical tradeoffs."
        }
        "implement_request" => {
            "Template: State what was done in sentence one, then summarize impact and any important caveats."
        }
        _ => {
            "Template: Answer directly in plain language first, then add only necessary supporting detail."
        }
    }
}

fn workflow_user_prefers_deep_dive(message: &str) -> bool {
    let lowered = clean_text(message, 1_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    lowered.contains("deep dive")
        || lowered.contains("in depth")
        || lowered.contains("in-depth")
        || lowered.contains("detailed")
        || lowered.contains("thorough")
        || lowered.contains("full analysis")
        || lowered.contains("step by step")
}

fn run_turn_workflow_final_response(
    root: &Path,
    provider: &str,
    model: &str,
    active_messages: &[Value],
    message: &str,
    workflow_mode: &str,
    response_tools: &[Value],
    workflow_events: &[Value],
    draft_response: &str,
    latest_assistant_text: &str,
) -> Value {
    let enriched_workflow_events = augment_turn_workflow_events_for_final_response(
        message,
        response_tools,
        workflow_events,
        draft_response,
        latest_assistant_text,
    );
    let mut workflow = turn_workflow_metadata(
        workflow_mode,
        response_tools,
        &enriched_workflow_events,
        draft_response,
        message,
    );
    let required = workflow
        .pointer("/final_llm_response/required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !required {
        workflow["final_llm_response"]["attempted"] = Value::Bool(false); workflow["final_llm_response"]["used"] = Value::Bool(false);
        workflow["final_llm_response"]["status"] = Value::String("skipped_not_required".to_string());
        set_turn_workflow_final_stage_status(&mut workflow, "skipped_not_required");
        return workflow;
    }
    if cfg!(test) && !workflow_test_llm_enabled(root) {
        workflow["final_llm_response"]["attempted"] = Value::Bool(false); workflow["final_llm_response"]["used"] = Value::Bool(false);
        workflow["final_llm_response"]["status"] = Value::String("skipped_test".to_string());
        set_turn_workflow_final_stage_status(&mut workflow, "skipped_test");
        return workflow;
    }
    let cleaned_provider = clean_text(provider, 80);
    let cleaned_model = clean_text(model, 240);
    if cleaned_provider.is_empty() || cleaned_model.is_empty() {
        workflow["final_llm_response"]["attempted"] = Value::Bool(false); workflow["final_llm_response"]["used"] = Value::Bool(false);
        workflow["final_llm_response"]["status"] =
            Value::String("skipped_missing_model".to_string());
        set_turn_workflow_final_stage_status(&mut workflow, "skipped_missing_model");
        return workflow;
    }
    let tool_rows_json = serde_json::to_string(&tool_rows_for_llm_recovery(response_tools, 6))
        .unwrap_or_else(|_| "[]".to_string());
    let workflow_events_json = serde_json::to_string(&enriched_workflow_events)
        .unwrap_or_else(|_| "[]".to_string());
    let workflow_metadata_json =
        serde_json::to_string(&workflow).unwrap_or_else(|_| "{}".to_string());
    let template_label = workflow_response_template_label(message);
    let template_instruction = workflow_template_instruction_for_label(template_label);
    let detail_style = if workflow_user_prefers_deep_dive(message) {
        "detailed"
    } else {
        "concise"
    };
    let system_prompt = clean_text(
        &format!(
            "{}\n\nHardcoded agent workflow: you are writing the final assistant response after the system collected tool outcomes and workflow events. Use the recorded evidence. If a tool failed, timed out, was blocked, or returned low-signal output, say that plainly in your own words. Never emit raw telemetry, placeholder copy, inline `<function=...>` markup, or pretend a failed tool succeeded.\n\nFinal-answer contract (final_answer_contract_v1): (1) answer the user's request in the first 1-2 sentences, (2) do not echo/restate the user prompt as your response, (3) do not include placeholder copy, (4) include source tags for key claims using `[source:local_context]` or `[source:tool_receipt:<id>]`.\n\nResponse template class: {}. {} Style: {} by default unless user requested a deep dive.",
            AGENT_RUNTIME_SYSTEM_PROMPT, template_label, template_instruction, detail_style
        ),
        12_000,
    );
    let user_prompt = clean_text(
        &format!(
            "User request:\n{message}\n\nCurrent draft response:\n{}\n\nWorkflow metadata:\n{workflow_metadata_json}\n\nRecorded tool outcomes:\n{tool_rows_json}\n\nWorkflow events:\n{workflow_events_json}\n\nWrite the final assistant response now.",
            if clean_text(draft_response, 2_000).is_empty() {
                "(empty)"
            } else {
                draft_response
            }
        ),
        20_000,
    );
    let coherence_window_messages = 2usize;
    let recent_context = active_messages
        .iter()
        .rev()
        .take(coherence_window_messages)
        .filter_map(|row| {
            let text = clean_text(
                row.get("text")
                    .or_else(|| row.get("content"))
                    .or_else(|| row.get("message"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                320,
            );
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        })
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
    let max_attempts: u64 = 2;
    let mut last_error = String::new();
    let mut last_invalid_excerpt = String::new();
    let mut last_reject_reason = String::new();
    let has_structured_block_evidence = response_tools.iter().any(|row| {
        let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 80)
            .to_ascii_lowercase();
        let error = clean_text(row.get("error").and_then(Value::as_str).unwrap_or(""), 160)
            .to_ascii_lowercase();
        let tool_type = clean_text(row.get("type").and_then(Value::as_str).unwrap_or(""), 120)
            .to_ascii_lowercase();
        let blocked = row.get("blocked").and_then(Value::as_bool).unwrap_or(false);
        blocked
            || matches!(status.as_str(), "blocked" | "policy_denied")
            || tool_type == "tool_pre_gate_blocked"
            || error.contains("nexus_delivery_denied")
            || error.contains("tool_permission_denied")
            || row
                .get("status_code")
                .and_then(Value::as_i64)
                .or_else(|| row.get("http_status").and_then(Value::as_i64))
                .map(|code| matches!(code, 401 | 403 | 404 | 422 | 429))
                .unwrap_or(false)
    });
    workflow["quality_telemetry"] = json!({
        "off_topic_reject": 0,
        "deferred_reply_reject": 0,
        "alignment_reject": 0,
        "prompt_echo_reject": 0,
        "unsourced_claim_reject": 0,
        "direct_answer_reject": 0,
        "meta_control_tool_block": workflow
            .pointer("/tool_gate/meta_control_message")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            && response_tools.is_empty(),
        "final_fallback_used": false
    });
    workflow["final_llm_response"]["attempted"] = Value::Bool(true);
    workflow["final_llm_response"]["max_attempts"] = json!(max_attempts);
    workflow["final_llm_response"]["coherence_window_messages"] =
        json!(coherence_window_messages);
    for attempt in 1..=max_attempts {
        workflow["final_llm_response"]["attempt_count"] = json!(attempt);
        let attempt_user_prompt = if attempt > 1 {
            clean_text(
                &format!(
                    "{}\n\nCorrection for attempt {} of {}: your previous answer did not complete the workflow because it tried to start another search, deferred the answer, emitted inline tool markup, or drifted away from the latest user request. Do not ask to retry, rerun, narrow the query, fetch another source, or emit `<function=...>` calls. Keep high lexical/semantic alignment to the latest user request and recent conversation context. Using only the recorded tool outcomes and workflow events above, explain what happened in your own words and tell the user what the tool actually returned.",
                    user_prompt, attempt, max_attempts
                ),
                20_000,
            )
        } else {
            user_prompt.clone()
        };
        match crate::dashboard_provider_runtime::invoke_chat(
            root,
            &cleaned_provider,
            &cleaned_model,
            &system_prompt,
            active_messages,
            &attempt_user_prompt,
        ) {
            Ok(retried) => {
                let mut retried_text = clean_chat_text(
                    retried
                        .get("response")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    32_000,
                );
                retried_text = sanitize_workflow_final_response_candidate(
                    &strip_internal_cache_control_markup(
                        &strip_internal_context_metadata_prefix(&retried_text),
                    ),
                );
                if !user_requested_internal_runtime_details(message) {
                    retried_text = abstract_runtime_mechanics_terms(&retried_text);
                }
                let deferred_reply = response_is_deferred_execution_preamble(&retried_text)
                    || response_is_deferred_retry_prompt(&retried_text)
                    || workflow_response_requests_more_tooling(&retried_text);
                let off_topic_reply = response_is_unrelated_context_dump(message, &retried_text);
                let low_alignment_reply = response_low_alignment_with_turn_context(
                    message,
                    &recent_context,
                    &retried_text,
                );
                let prompt_echo_reply = response_prompt_echo_detected(message, &retried_text);
                let receipt_mapped_sources = response_tools
                    .iter()
                    .any(|row| !response_tool_receipt_id(row).is_empty());
                let missing_evidence_tags = !response_tools.is_empty()
                    && !receipt_mapped_sources
                    && !response_has_evidence_tags(&retried_text);
                let missing_direct_answer = !response_answers_user_early(message, &retried_text);
                let direct_answer_in_first_two_sentences = !missing_direct_answer;
                let rejects_base_contract = response_fails_base_final_answer_contract(&retried_text);
                let rejects_speculative_blocker =
                    response_contains_speculative_web_blocker_language(&retried_text)
                        && !has_structured_block_evidence;
                let reject_checks = [
                    (deferred_reply, "deferred_reply", "deferred_reply_reject"),
                    (off_topic_reply, "off_topic_reply", "off_topic_reject"),
                    (low_alignment_reply, "low_alignment_reply", "alignment_reject"),
                    (prompt_echo_reply, "prompt_echo_reply", "prompt_echo_reject"),
                    (
                        missing_direct_answer,
                        "missing_direct_answer_reply",
                        "direct_answer_reject",
                    ),
                    (retried_text.is_empty(), "empty_reply", ""),
                    (
                        response_is_no_findings_placeholder(&retried_text),
                        "placeholder_reply",
                        "",
                    ),
                    (
                        response_looks_like_tool_ack_without_findings(&retried_text),
                        "ack_only_reply",
                        "",
                    ),
                    (
                        rejects_speculative_blocker || rejects_base_contract,
                        "invalid_reply",
                        "",
                    ),
                ];
                let (reject_reason, reject_counter) = reject_checks
                    .into_iter()
                    .find(|(should_reject, _, _)| *should_reject)
                    .map(|(_, reason, counter)| (reason, counter))
                    .unwrap_or(("", ""));
                if !reject_reason.is_empty() {
                    if !reject_counter.is_empty() {
                        bump_workflow_quality_counter(&mut workflow, reject_counter);
                    }
                    last_reject_reason = reject_reason.to_string();
                    last_invalid_excerpt = first_sentence(&retried_text, 240);
                    continue;
                }
                workflow["final_llm_response"]["used"] = Value::Bool(true);
                workflow["final_llm_response"]["status"] =
                    Value::String("synthesized".to_string());
                set_turn_workflow_final_stage_status(&mut workflow, "synthesized");
                workflow["final_llm_response"]["helpfulness"] = json!({
                    "direct_answer_in_first_two_sentences": direct_answer_in_first_two_sentences,
                    "prompt_echo_detected": response_prompt_echo_detected(message, &retried_text),
                    "has_evidence_tags": response_has_evidence_tags(&retried_text)
                        || receipt_mapped_sources
                        || response_tools.is_empty(),
                    "missing_evidence_mapping": missing_evidence_tags,
                    "template_label": template_label,
                    "detail_style": detail_style
                });
                let attempt_count = attempt as f64;
                let off_topic_reject =
                    response_workflow_quality_rate(&workflow, "off_topic_reject");
                let direct_answer_rate = if direct_answer_in_first_two_sentences {
                    1.0
                } else {
                    0.0
                };
                let retry_rate = if max_attempts > 1 {
                    ((attempt.saturating_sub(1)) as f64 / (max_attempts.saturating_sub(1)) as f64)
                        .clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let off_topic_reject_rate = if attempt_count > 0.0 {
                    (off_topic_reject / attempt_count).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                workflow["quality_telemetry"]["direct_answer_rate"] = json!(direct_answer_rate);
                workflow["quality_telemetry"]["retry_rate"] = json!(retry_rate);
                workflow["quality_telemetry"]["off_topic_reject_rate"] = json!(off_topic_reject_rate);
                workflow["response"] = Value::String(retried_text);
                return workflow;
            }
            Err(err) => {
                last_error = clean_text(&err, 240);
            }
        }
    }
    workflow["final_llm_response"]["used"] = Value::Bool(false);
    if !last_invalid_excerpt.is_empty() {
        workflow["final_llm_response"]["status"] = Value::String("synthesis_failed".to_string());
        set_turn_workflow_final_stage_status(&mut workflow, "synthesis_failed");
        workflow["final_llm_response"]["error"] = Value::String(last_invalid_excerpt);
        if !last_reject_reason.is_empty() {
            workflow["final_llm_response"]["last_reject_reason"] =
                Value::String(last_reject_reason);
        }
    } else {
        workflow["final_llm_response"]["status"] = Value::String("invoke_failed".to_string());
        set_turn_workflow_final_stage_status(&mut workflow, "invoke_failed");
        workflow["final_llm_response"]["error"] = Value::String(last_error);
    }
    workflow
}
