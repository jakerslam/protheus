fn workflow_trace_string(
    value: &Value,
    paths: &[&[&str]],
    default: &str,
    max_len: usize,
) -> String {
    for path in paths {
        let mut cursor = value;
        let mut found = true;
        for segment in *path {
            if let Some(next) = cursor.get(*segment) {
                cursor = next;
            } else {
                found = false;
                break;
            }
        }
        if found {
            let raw = match cursor {
                Value::Bool(true) => "Yes",
                Value::Bool(false) => "No",
                _ => cursor.as_str().unwrap_or(""),
            };
            let cleaned = clean_text(raw, max_len);
            if !cleaned.is_empty() {
                return cleaned;
            }
        }
    }
    clean_text(default, max_len)
}

fn workflow_visibility_trace_payload(
    response_workflow: &Value,
    response_finalization: &Value,
) -> Value {
    let pending = response_finalization
        .get("pending_tool_request")
        .or_else(|| response_workflow.get("pending_tool_request"))
        .or_else(|| response_workflow.get("manual_toolbox_pending_tool_request"));
    let confirmation_state = pending
        .and_then(|row| row.get("status").and_then(Value::as_str))
        .unwrap_or_else(|| {
            if pending.is_some() {
                "pending_confirmation"
            } else {
                "none"
            }
        });
    let mut selected_option = workflow_trace_string(
        response_workflow,
        &[
            &["tool_gate", "gate_submission", "llm_submission"],
            &["tool_gate", "needs_tool_access"],
        ],
        "",
        80,
    );
    if selected_option.is_empty()
        && response_workflow
            .pointer("/final_llm_response/status")
            .and_then(Value::as_str)
            == Some("skipped_not_required")
    {
        selected_option = "No".to_string();
    }
    json!({
        "gate_id": workflow_trace_string(response_workflow, &[
            &["tool_gate", "gate_submission", "gate_id"],
            &["current_stage"],
        ], "gate_1_need_tool_access_menu", 120),
        "input_kind": workflow_trace_string(response_workflow, &[
            &["tool_gate", "gate_submission", "input_shape", "type"],
            &["tool_gate", "gate_1_question_type"],
        ], "multiple_choice", 80),
        "selected_option": selected_option,
        "tool_name": clean_text(
            pending
                .and_then(|row| row.get("tool_name").or_else(|| row.get("tool")).and_then(Value::as_str))
                .unwrap_or(""),
            120,
        ),
        "confirmation_state": clean_text(confirmation_state, 80),
        "final_authority": "llm_only"
    })
}

fn workflow_visibility_payload(response_workflow: &Value, response_finalization: &Value) -> Value {
    let visibility = response_workflow
        .get("visibility")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let read_visibility = |key: &str, fallback: &str, max_len: usize| -> String {
        let value = visibility
            .get(key)
            .or_else(|| response_workflow.get(key))
            .and_then(Value::as_str)
            .unwrap_or(fallback);
        clean_text(value, max_len)
    };
    let current_stage = {
        let stage = read_visibility("current_stage", "final_response", 80);
        if stage.is_empty() {
            "final_response".to_string()
        } else {
            stage
        }
    };
    let current_stage_status = read_visibility("current_stage_status", "complete", 80);
    let ui_status = read_visibility("ui_status", "Workflow status available.", 180);
    let agent_process_status = read_visibility(
        "agent_process_status",
        "Workflow diagnostics available in payload.",
        220,
    );
    let debug_status = read_visibility("debug_status", "", 320);
    let formats = visibility
        .get("formats")
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or_else(|| {
            json!({
                "ui": ui_status,
                "agent_process": agent_process_status,
                "debug": debug_status
            })
        });
    let visible_response_source = clean_text(
        response_finalization
            .get("visible_response_source")
            .or_else(|| response_workflow.get("visible_response_source"))
            .and_then(Value::as_str)
            .unwrap_or("none"),
        80,
    );
    let system_chat_injection_used = response_finalization
        .get("system_chat_injection_used")
        .or_else(|| response_workflow.get("system_chat_injection_used"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    json!({
        "contract": "workflow_visibility_payload_v1",
        "current_stage": current_stage,
        "current_stage_status": current_stage_status,
        "ui_status": ui_status,
        "agent_process_status": agent_process_status,
        "debug_status": debug_status,
        "formats": formats,
        "workflow_trace": workflow_visibility_trace_payload(response_workflow, response_finalization),
        "selected_workflow_id": clean_text(response_workflow.pointer("/selected_workflow/name").and_then(Value::as_str).unwrap_or(""), 80),
        "stage_count": response_workflow.get("stage_statuses").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
        "finalization_status": clean_text(response_finalization.get("outcome").and_then(Value::as_str).unwrap_or(""), 180),
        "diagnostics_only": true,
        "final_chat_authority": "llm_only",
        "visible_chat_text_authority": "llm_only",
        "chat_injection_allowed": false,
        "system_injected_chat_text_allowed": false,
        "system_chat_injection_used": system_chat_injection_used,
        "visible_response_source": visible_response_source
    })
}
