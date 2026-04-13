fn turn_workflow_event(kind: &str, detail: Value) -> Value {
    json!({
        "kind": clean_text(kind, 80),
        "detail": detail
    })
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
    json!({
        "completion_state": completion_state,
        "findings_available": !findings.is_empty(),
        "final_ack_only": response_looks_like_tool_ack_without_findings(&cleaned),
        "final_no_findings": response_is_no_findings_placeholder(&cleaned),
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
    let system_prompt = clean_text(
        &format!(
            "{}\n\nHardcoded agent workflow: you are writing the final assistant response after the system collected tool outcomes and workflow events. Use the recorded evidence. If a tool failed, timed out, was blocked, or returned low-signal output, say that plainly in your own words. Never emit raw telemetry, placeholder copy, inline `<function=...>` markup, or pretend a failed tool succeeded.",
            AGENT_RUNTIME_SYSTEM_PROMPT
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
    let max_attempts = 2;
    let mut last_error = String::new();
    let mut last_invalid_excerpt = String::new();
    workflow["final_llm_response"]["attempted"] = Value::Bool(true);
    workflow["final_llm_response"]["max_attempts"] = json!(max_attempts);
    for attempt in 1..=max_attempts {
        workflow["final_llm_response"]["attempt_count"] = json!(attempt);
        let attempt_user_prompt = if attempt > 1 {
            clean_text(
                &format!(
                    "{}\n\nCorrection for attempt {} of {}: your previous answer did not complete the workflow because it tried to start another search, deferred the answer, or emitted inline tool markup. Do not ask to retry, rerun, narrow the query, fetch another source, or emit `<function=...>` calls. Using only the recorded tool outcomes and workflow events above, explain what happened in your own words and tell the user what the tool actually returned.",
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
                if retried_text.is_empty()
                    || response_is_no_findings_placeholder(&retried_text)
                    || response_looks_like_tool_ack_without_findings(&retried_text)
                    || workflow_response_requests_more_tooling(&retried_text)
                    || response_is_unrelated_context_dump(message, &retried_text)
                {
                    last_invalid_excerpt = first_sentence(&retried_text, 240);
                    continue;
                }
                workflow["final_llm_response"]["used"] = Value::Bool(true);
                workflow["final_llm_response"]["status"] =
                    Value::String("synthesized".to_string());
                set_turn_workflow_final_stage_status(&mut workflow, "synthesized");
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
    } else {
        workflow["final_llm_response"]["status"] = Value::String("invoke_failed".to_string());
        set_turn_workflow_final_stage_status(&mut workflow, "invoke_failed");
        workflow["final_llm_response"]["error"] = Value::String(last_error);
    }
    workflow
}
