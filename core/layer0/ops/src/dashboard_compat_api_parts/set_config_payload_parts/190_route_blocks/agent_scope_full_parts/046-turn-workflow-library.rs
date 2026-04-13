fn turn_workflow_library_catalog() -> Vec<Value> {
    vec![
        json!({
            "name": "complex_prompt_chain_v1",
            "workflow_type": "hard_agent_workflow",
            "default": true,
            "description": "Model-first workflow: the LLM interprets the user prompt, decides whether tools are needed, the system collects tool and workflow outputs, and the final user-facing reply is LLM-authored when the model is online.",
            "stages": [
                "workflow_gate",
                "initial_model_interpretation",
                "tool_and_system_collection",
                "final_llm_response"
            ],
            "final_response_policy": "llm_authored_when_online"
        }),
        json!({
            "name": "simple_conversation_v1",
            "workflow_type": "hard_agent_workflow",
            "default": false,
            "description": "Reserved lightweight workflow slot for direct conversation. It still passes through the workflow gate so turn control remains centralized.",
            "stages": [
                "workflow_gate",
                "initial_model_interpretation",
                "final_llm_response"
            ],
            "final_response_policy": "llm_authored_when_online"
        }),
    ]
}

fn default_turn_workflow_name() -> &'static str {
    "complex_prompt_chain_v1"
}

fn selected_turn_workflow(workflow_mode: &str) -> Value {
    json!({
        "name": default_turn_workflow_name(),
        "workflow_type": "hard_agent_workflow",
        "mode": clean_text(workflow_mode, 80),
        "selection_reason": "default_library_workflow",
        "final_response_policy": "llm_authored_when_online"
    })
}

fn workflow_library_prompt_context(latent_tool_candidates: &[Value]) -> String {
    let broker = protheus_tooling_core_v1::ToolBroker::default();
    let grouped_catalog = broker.grouped_capability_catalog();
    let mut lines = vec![
        format!(
            "Workflow library gate: every chat turn must pass through `{}`. The default selected workflow is `{}`.",
            "agent_workflow_library_v1",
            default_turn_workflow_name()
        ),
        "Default workflow contract: read the user request, decide whether tools are needed, emit inline `<function=...>{...}</function>` calls only when justified, wait for tool/system results, and write the final answer using the recorded evidence.".to_string(),
        "Chat operator syntax such as `tool::...` or slash tool requests are workflow hints, not pre-executed results. You still must decide whether to call the hinted tool.".to_string(),
    ];
    if !grouped_catalog.is_empty() {
        lines.push("Modular tool catalog by domain:".to_string());
        for group in grouped_catalog.iter().take(6) {
            let domain = serde_json::to_value(group.domain)
                .ok()
                .and_then(|value| value.as_str().map(|row| row.to_string()))
                .unwrap_or_else(|| "unknown".to_string());
            let tool_names = group
                .tools
                .iter()
                .filter(|row| row.discoverable)
                .take(6)
                .map(|row| clean_text(&row.tool_name, 80))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>();
            if tool_names.is_empty() {
                lines.push(format!(
                    "- {}: {}",
                    clean_text(&domain, 40),
                    clean_text(&group.description, 180)
                ));
            } else {
                lines.push(format!(
                    "- {}: {} Available tools: {}.",
                    clean_text(&domain, 40),
                    clean_text(&group.description, 180),
                    clean_text(&tool_names.join(", "), 240)
                ));
            }
        }
    }
    if !latent_tool_candidates.is_empty() {
        lines.push("Strong workflow hints for this turn (not yet executed):".to_string());
        for row in latent_tool_candidates.iter().take(4) {
            let tool = clean_text(row.get("tool").and_then(Value::as_str).unwrap_or(""), 80);
            let reason = clean_text(row.get("reason").and_then(Value::as_str).unwrap_or(""), 220);
            let label = clean_text(row.get("label").and_then(Value::as_str).unwrap_or(""), 80);
            let workflow_only = row
                .get("workflow_only")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let detail = if workflow_only {
                let message = clean_text(
                    row.pointer("/proposed_input/message")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    220,
                );
                if message.is_empty() {
                    format!("- {}: {}.", tool, reason)
                } else {
                    format!("- {}: {} Guidance: {}.", tool, reason, message)
                }
            } else if label.is_empty() {
                format!("- {}: {}.", tool, reason)
            } else {
                format!("- {} ({}): {}.", tool, label, reason)
            };
            lines.push(clean_text(&detail, 360));
        }
    }
    clean_text(&lines.join("\n"), 12_000)
}

fn turn_workflow_requires_final_llm(response_tools: &[Value], workflow_events: &[Value]) -> bool {
    let _ = response_tools;
    let _ = workflow_events;
    true
}

fn turn_workflow_stage_rows(
    workflow_mode: &str,
    response_tools: &[Value],
    workflow_events: &[Value],
    draft_response: &str,
) -> Vec<Value> {
    let requires_final_llm = turn_workflow_requires_final_llm(response_tools, workflow_events);
    let _ = workflow_mode;
    let cleaned_draft = clean_text(draft_response, 2_000);
    let final_stage_status = if requires_final_llm {
        "pending_final_llm"
    } else {
        "no_post_synthesis_required"
    };
    vec![
        json!({
            "stage": "workflow_gate",
            "status": "enforced"
        }),
        json!({
            "stage": "initial_model_interpretation",
            "status": if cleaned_draft.is_empty() {
                "completed_empty"
            } else {
                "completed"
            },
            "draft_response_state": if cleaned_draft.is_empty() {
                "empty"
            } else if response_is_no_findings_placeholder(&cleaned_draft) {
                "no_findings"
            } else if response_looks_like_tool_ack_without_findings(&cleaned_draft) {
                "ack_only"
            } else {
                "present"
            }
        }),
        json!({
            "stage": "tool_and_system_collection",
            "status": if response_tools.is_empty() && workflow_events.is_empty() {
                "no_external_events"
            } else {
                "collected"
            },
            "tool_count": response_tools.len(),
            "system_event_count": workflow_events.len()
        }),
        json!({
            "stage": "final_llm_response",
            "required": requires_final_llm,
            "status": final_stage_status
        }),
    ]
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
        "stage_statuses": turn_workflow_stage_rows(workflow_mode, response_tools, workflow_events, draft_response),
        "final_llm_response": {
            "required": requires_final_llm,
            "source": "workflow_post_synthesis"
        }
    })
}

fn set_turn_workflow_final_stage_status(workflow: &mut Value, status: &str) {
    if let Some(rows) = workflow.get_mut("stage_statuses").and_then(Value::as_array_mut) {
        for row in rows.iter_mut() {
            if row
                .get("stage")
                .and_then(Value::as_str)
                .map(|value| value == "final_llm_response")
                .unwrap_or(false)
            {
                row["status"] = Value::String(clean_text(status, 80));
            }
        }
    }
}

fn workflow_response_requests_more_tooling(response: &str) -> bool {
    let lowered = clean_text(response, 800).to_ascii_lowercase();
    !lowered.is_empty()
        && [
            "let me search",
            "i'll search",
            "i will search",
            "would you like me to search",
            "would you like me to fetch",
            "search for more",
            "rerun with",
            "retry with",
            "narrower query",
            "specific source url",
            "need to search",
            "need targeted web research",
            "need more specific",
            "let me try",
            "i'll try",
            "i will try",
            "if you'd like, i can search",
            "if you would like, i can search",
            "if you'd like, i can fetch",
            "if you would like, i can fetch",
            "if you'd like, i can look deeper",
            "if you would like, i can look deeper",
            "more targeted approach",
            "another search",
            "technical documentation",
            "architecture details to enable",
        ]
        .iter()
        .any(|marker| lowered.contains(marker))
}

fn sanitize_workflow_final_response_candidate(response: &str) -> String {
    let (without_inline_calls, inline_calls) = extract_inline_tool_calls(response, 6);
    let mut cleaned = clean_chat_text(
        if inline_calls.is_empty() || without_inline_calls.trim().is_empty() {
            response
        } else {
            without_inline_calls.trim()
        },
        32_000,
    );
    let lowered = cleaned.to_ascii_lowercase();
    let cutoff = [
        "let me try",
        "i'll try",
        "i will try",
        "let me search",
        "i'll search",
        "i will search",
        "would you like me to search",
        "would you like me to fetch",
        "if you'd like, i can search",
        "if you would like, i can search",
        "if you'd like, i can fetch",
        "if you would like, i can fetch",
        "if you'd like, i can look deeper",
        "if you would like, i can look deeper",
    ]
        .iter()
        .filter_map(|marker| lowered.find(marker))
        .min();
    if let Some(idx) = cutoff {
        cleaned = cleaned[..idx].trim().trim_end_matches(&['\n', ' ', '-', ':'][..]).to_string();
    }
    clean_chat_text(cleaned.trim(), 32_000)
}
