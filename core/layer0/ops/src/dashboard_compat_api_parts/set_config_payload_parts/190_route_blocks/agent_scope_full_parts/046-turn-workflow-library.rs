fn turn_workflow_library_catalog() -> Vec<Value> {
    vec![json!({
        "name": "complex_prompt_chain_v1",
        "workflow_type": "hard_agent_workflow",
        "default": true,
        "description": "Model-first workflow: the LLM interprets the user prompt, decides whether tools are needed, the system collects tool and workflow outputs, and the final user-facing reply is LLM-authored when the model is online.",
        "stages": [
            "workflow_gate",
            "tool_and_system_collection",
            "final_llm_response"
        ],
        "final_response_policy": "llm_authored_when_online"
    })]
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

fn turn_workflow_requires_final_llm(response_tools: &[Value], workflow_events: &[Value]) -> bool {
    !response_tools.is_empty() || !workflow_events.is_empty()
}

fn turn_workflow_stage_rows(
    workflow_mode: &str,
    response_tools: &[Value],
    workflow_events: &[Value],
    draft_response: &str,
) -> Vec<Value> {
    let requires_final_llm = turn_workflow_requires_final_llm(response_tools, workflow_events);
    let draft_present = !clean_text(draft_response, 4_000).is_empty();
    let final_stage_status = if requires_final_llm {
        "pending_final_llm"
    } else if workflow_mode == "model_direct_answer" && draft_present {
        "accepted_initial_model_response"
    } else if workflow_mode == "direct_tool_route" && draft_present {
        "accepted_operator_route_response"
    } else {
        "no_post_synthesis_required"
    };
    vec![
        json!({
            "stage": "workflow_gate",
            "status": "enforced"
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
