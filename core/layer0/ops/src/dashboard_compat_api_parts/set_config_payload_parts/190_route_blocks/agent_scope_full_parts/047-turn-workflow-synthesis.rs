fn turn_workflow_event(kind: &str, detail: Value) -> Value {
    json!({
        "kind": clean_text(kind, 80),
        "detail": detail
    })
}

fn build_turn_workflow_events(
    pending_confirmation: Option<&Value>,
    replayed_pending_confirmation: bool,
) -> Vec<Value> {
    let mut events = Vec::<Value>::new();
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

fn turn_workflow_metadata(
    workflow_mode: &str,
    response_tools: &[Value],
    workflow_events: &[Value],
    draft_response: &str,
) -> Value {
    let cleaned_draft = clean_text(draft_response, 4_000);
    let draft_response_state = if cleaned_draft.is_empty() {
        "empty"
    } else if response_is_no_findings_placeholder(&cleaned_draft) {
        "no_findings"
    } else if response_looks_like_tool_ack_without_findings(&cleaned_draft) {
        "ack_only"
    } else {
        "present"
    };
    let requires_final_llm = turn_workflow_requires_final_llm(response_tools, workflow_events);
    json!({
        "contract": "agent_workflow_library_v1",
        "workflow_gate": {
            "required": true,
            "status": "enforced"
        },
        "library": {
            "default_workflow": default_turn_workflow_name(),
            "available_workflows": turn_workflow_library_catalog()
        },
        "selected_workflow": selected_turn_workflow(workflow_mode),
        "tool_count": response_tools.len(),
        "system_event_count": workflow_events.len(),
        "draft_response_state": draft_response_state,
        "findings_summary": clean_text(&response_tools_summary_for_user(response_tools, 4), 2_000),
        "failure_summary": clean_text(&response_tools_failure_reason_for_user(response_tools, 4), 2_000),
        "system_events": workflow_events,
        "stage_statuses": turn_workflow_stage_rows(
            workflow_mode,
            response_tools,
            workflow_events,
            draft_response,
        ),
        "final_llm_response": {
            "required": requires_final_llm,
            "source": if requires_final_llm {
                "workflow_post_synthesis"
            } else if workflow_mode == "model_direct_answer" {
                "initial_model_response"
            } else {
                "workflow_gate_only"
            }
        }
    })
}

fn set_turn_workflow_final_stage_status(workflow: &mut Value, status: &str) {
    if let Some(rows) = workflow
        .get_mut("stage_statuses")
        .and_then(Value::as_array_mut)
    {
        for row in rows.iter_mut() {
            let is_final_stage = row
                .get("stage")
                .and_then(Value::as_str)
                .map(|value| value == "final_llm_response")
                .unwrap_or(false);
            if is_final_stage {
                row["status"] = Value::String(clean_text(status, 80));
            }
        }
    }
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
) -> Value {
    let mut workflow = turn_workflow_metadata(
        workflow_mode,
        response_tools,
        workflow_events,
        draft_response,
    );
    let required = workflow
        .pointer("/final_llm_response/required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !required {
        workflow["final_llm_response"]["attempted"] = Value::Bool(false);
        workflow["final_llm_response"]["used"] = Value::Bool(false);
        let fallback_status = if workflow_mode == "model_direct_answer"
            && !clean_text(draft_response, 4_000).is_empty()
        {
            "accepted_initial_model_response"
        } else if workflow_mode == "direct_tool_route"
            && !clean_text(draft_response, 4_000).is_empty()
        {
            "accepted_operator_route_response"
        } else {
            "skipped_not_required"
        };
        workflow["final_llm_response"]["status"] = Value::String(fallback_status.to_string());
        set_turn_workflow_final_stage_status(&mut workflow, fallback_status);
        return workflow;
    }
    if cfg!(test) {
        workflow["final_llm_response"]["attempted"] = Value::Bool(false);
        workflow["final_llm_response"]["used"] = Value::Bool(false);
        workflow["final_llm_response"]["status"] = Value::String("skipped_test".to_string());
        set_turn_workflow_final_stage_status(&mut workflow, "skipped_test");
        return workflow;
    }
    let cleaned_provider = clean_text(provider, 80);
    let cleaned_model = clean_text(model, 240);
    if cleaned_provider.is_empty() || cleaned_model.is_empty() {
        workflow["final_llm_response"]["attempted"] = Value::Bool(false);
        workflow["final_llm_response"]["used"] = Value::Bool(false);
        workflow["final_llm_response"]["status"] =
            Value::String("skipped_missing_model".to_string());
        set_turn_workflow_final_stage_status(&mut workflow, "skipped_missing_model");
        return workflow;
    }
    let tool_rows_json = serde_json::to_string(&tool_rows_for_llm_recovery(response_tools, 6))
        .unwrap_or_else(|_| "[]".to_string());
    let workflow_events_json =
        serde_json::to_string(workflow_events).unwrap_or_else(|_| "[]".to_string());
    let workflow_metadata_json =
        serde_json::to_string(&workflow).unwrap_or_else(|_| "{}".to_string());
    let system_prompt = clean_text(
        &format!(
            "{}\n\nHardcoded agent workflow: you are writing the final assistant response after the system collected tool outcomes and workflow events. Use the recorded evidence. If a tool failed, timed out, was blocked, or returned low-signal output, say that plainly in your own words. Never emit raw telemetry, placeholder copy, or pretend a failed tool succeeded.",
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
    workflow["final_llm_response"]["attempted"] = Value::Bool(true);
    match crate::dashboard_provider_runtime::invoke_chat(
        root,
        &cleaned_provider,
        &cleaned_model,
        &system_prompt,
        active_messages,
        &user_prompt,
    ) {
        Ok(retried) => {
            let mut retried_text = clean_chat_text(
                retried
                    .get("response")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                32_000,
            );
            retried_text = strip_internal_context_metadata_prefix(&retried_text);
            retried_text = strip_internal_cache_control_markup(&retried_text);
            if !user_requested_internal_runtime_details(message) {
                retried_text = abstract_runtime_mechanics_terms(&retried_text);
            }
            if retried_text.is_empty()
                || response_is_unrelated_context_dump(message, &retried_text)
            {
                workflow["final_llm_response"]["used"] = Value::Bool(false);
                workflow["final_llm_response"]["status"] =
                    Value::String("empty_or_unrelated".to_string());
                set_turn_workflow_final_stage_status(&mut workflow, "empty_or_unrelated");
                workflow
            } else {
                workflow["final_llm_response"]["used"] = Value::Bool(true);
                workflow["final_llm_response"]["status"] =
                    Value::String("synthesized".to_string());
                set_turn_workflow_final_stage_status(&mut workflow, "synthesized");
                workflow["response"] = Value::String(retried_text);
                workflow
            }
        }
        Err(err) => {
            workflow["final_llm_response"]["used"] = Value::Bool(false);
            workflow["final_llm_response"]["status"] =
                Value::String("invoke_failed".to_string());
            set_turn_workflow_final_stage_status(&mut workflow, "invoke_failed");
            workflow["final_llm_response"]["error"] = Value::String(clean_text(&err, 240));
            workflow
        }
    }
}
